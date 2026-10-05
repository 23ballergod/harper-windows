//! Background checking: the model runs on its own thread so typing never waits on it.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

use harper_core::linting::Lint;

use crate::diff::{Edit, compute_edits, edit_to_lint};
use crate::engine::Corrector;
use crate::sentences::split_sentences;

/// Sentences longer than this are skipped: they are slow on a CPU and models tend to rewrite them.
const MAX_SENTENCE_CHARS: usize = 400;
/// Bounds memory use; the cache is cleared when it grows past this many sentences.
const MAX_CACHE_ENTRIES: usize = 4_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckerStatus {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Default)]
struct Shared {
    status: Option<CheckerStatus>,
    cache: HashMap<String, Vec<Edit>>,
    queue: VecDeque<String>,
    shutdown: bool,
    /// Increments every time a sentence finishes, so callers can tell when to redraw.
    generation: u64,
}

/// Checks sentences with a [`Corrector`] on a background thread and caches the results.
pub struct AiChecker {
    shared: Arc<(Mutex<Shared>, Condvar)>,
}

impl AiChecker {
    /// Starts the worker thread. `load` runs on that thread, so a slow model load never blocks the
    /// caller.
    pub fn spawn<C, F>(load: F) -> Self
    where
        C: Corrector + 'static,
        F: FnOnce() -> Result<C, String> + Send + 'static,
    {
        let shared = Arc::new((
            Mutex::new(Shared {
                status: Some(CheckerStatus::Loading),
                ..Default::default()
            }),
            Condvar::new(),
        ));

        let worker_shared = shared.clone();
        std::thread::Builder::new()
            .name("harper-ai".into())
            .spawn(move || run_worker(worker_shared, load))
            .expect("failed to spawn AI checker thread");

        Self { shared }
    }

    pub fn status(&self) -> CheckerStatus {
        let guard = self.shared.0.lock().unwrap();
        guard.status.clone().unwrap_or(CheckerStatus::Loading)
    }

    pub fn generation(&self) -> u64 {
        self.shared.0.lock().unwrap().generation
    }

    /// Returns AI lints for every sentence already checked, and queues the rest.
    ///
    /// Each call replaces the queue, so sentences the user has since edited away are never
    /// checked. Finished sentences are queued before the one still being typed.
    pub fn lints(&self, text: &str) -> Vec<Lint> {
        let (lock, condvar) = &*self.shared;
        let mut shared = lock.lock().unwrap();

        if matches!(shared.status, Some(CheckerStatus::Failed(_))) {
            return Vec::new();
        }

        let mut lints = Vec::new();
        let mut finished = VecDeque::new();
        let mut unfinished = VecDeque::new();

        for sentence in split_sentences(text) {
            let words = sentence.text.split_whitespace().count();
            if words < 3 || sentence.text.chars().count() > MAX_SENTENCE_CHARS {
                continue;
            }

            match shared.cache.get(&sentence.text) {
                Some(edits) => {
                    lints.extend(edits.iter().map(|e| edit_to_lint(e, sentence.span.start)))
                }
                None => {
                    let ends_sentence = sentence.text.ends_with(['.', '!', '?', '"', '”', ')']);
                    if ends_sentence {
                        finished.push_back(sentence.text);
                    } else {
                        unfinished.push_back(sentence.text);
                    }
                }
            }
        }

        finished.extend(unfinished);
        shared.queue = finished;
        condvar.notify_all();

        lints
    }
}

impl Drop for AiChecker {
    fn drop(&mut self) {
        let (lock, condvar) = &*self.shared;
        lock.lock().unwrap().shutdown = true;
        condvar.notify_all();
    }
}

fn run_worker<C, F>(shared: Arc<(Mutex<Shared>, Condvar)>, load: F)
where
    C: Corrector,
    F: FnOnce() -> Result<C, String>,
{
    let (lock, condvar) = &*shared;

    let mut corrector = match load() {
        Ok(c) => {
            lock.lock().unwrap().status = Some(CheckerStatus::Ready);
            c
        }
        Err(error) => {
            lock.lock().unwrap().status = Some(CheckerStatus::Failed(error));
            return;
        }
    };

    loop {
        let sentence = {
            let mut guard = lock.lock().unwrap();
            loop {
                if guard.shutdown {
                    return;
                }
                if let Some(sentence) = guard.queue.pop_front() {
                    if guard.cache.contains_key(&sentence) {
                        continue;
                    }
                    break sentence;
                }
                guard = condvar.wait(guard).unwrap();
            }
        };

        let edits = match corrector.correct(&sentence) {
            Ok(corrected) => compute_edits(&sentence, &corrected),
            Err(error) => {
                eprintln!("AI grammar check failed: {error}");
                Vec::new()
            }
        };

        let mut guard = lock.lock().unwrap();
        if guard.cache.len() >= MAX_CACHE_ENTRIES {
            guard.cache.clear();
        }
        guard.cache.insert(sentence, edits);
        guard.generation += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    struct FakeCorrector;

    impl Corrector for FakeCorrector {
        fn correct(&mut self, sentence: &str) -> Result<String, String> {
            Ok(sentence
                .replace("go to", "goes to")
                .replace("dont", "doesn't"))
        }
    }

    fn wait_for(checker: &AiChecker, text: &str, count: usize) -> Vec<Lint> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let lints = checker.lints(text);
            if lints.len() >= count || Instant::now() > deadline {
                return lints;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn finds_errors_in_background_and_offsets_them() {
        let checker = AiChecker::spawn(|| Ok(FakeCorrector));
        let text = "Everything here is fine. She go to school daily. He dont care at all.";
        let lints = wait_for(&checker, text, 2);
        assert_eq!(lints.len(), 2);

        let chars: Vec<char> = text.chars().collect();
        let first: String = chars[lints[0].span.start..lints[0].span.end]
            .iter()
            .collect();
        assert_eq!(first, "go");
        assert_eq!(checker.status(), CheckerStatus::Ready);
    }

    #[test]
    fn load_failures_are_reported() {
        let checker = AiChecker::spawn(|| Err::<FakeCorrector, _>("missing model".to_string()));
        let deadline = Instant::now() + Duration::from_secs(5);
        while checker.status() == CheckerStatus::Loading && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            checker.status(),
            CheckerStatus::Failed("missing model".into())
        );
        assert!(checker.lints("She go to school every day.").is_empty());
    }
}
