//! Scores the app's real checking pipeline on `tests/data/tricky.jsonl`: the mistakes Grammarly is
//! known for catching, plus already-correct sentences that should be left alone.
//!
//! Runs three systems: Harper's rules alone, the AI model alone, and both (Harper first, then the
//! AI, as the app does). A sentence counts as fixed when accepting every suggestion produces one of
//! the reference answers exactly.
//!
//! Usage: `cargo run -p harper-ai --release --example benchmark -- <model dir> [Fast|Accurate] [limit]`

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use harper_ai::{
    AiModel, Corrector, QwenCorrector, compute_edits, is_probably_helpful, split_sentences,
};
use harper_core::linting::{LintGroup, Suggestion};
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};

struct Case {
    category: String,
    source: String,
    targets: Vec<String>,
    has_error: bool,
}

fn load_cases(limit: Option<usize>) -> Vec<Case> {
    let data = include_str!("../tests/data/tricky.jsonl");
    data.lines()
        .filter(|l| !l.trim().is_empty())
        .take(limit.unwrap_or(usize::MAX))
        .map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).expect("bad jsonl line");
            Case {
                category: v["category"].as_str().unwrap_or("").to_string(),
                source: v["source"].as_str().unwrap().to_string(),
                targets: v["targets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| t.as_str().unwrap().to_string())
                    .collect(),
                has_error: v["has_error"].as_bool().unwrap_or(true),
            }
        })
        .collect()
}

/// Accepts the first suggestion of every non-overlapping Harper lint, a few passes deep.
fn apply_harper(linter: &mut LintGroup, text: &str) -> String {
    let mut text = text.to_string();
    for _ in 0..3 {
        let doc = Document::new_plain_english_curated(&text);
        let mut lints: Vec<_> = linter
            .organized_lints(&doc)
            .into_values()
            .flatten()
            .filter(|l| !l.suggestions.is_empty())
            .filter(|l| is_probably_helpful(&text.chars().collect::<Vec<_>>(), l))
            .collect();
        if lints.is_empty() {
            break;
        }
        lints.sort_by_key(|l| (l.span.start, l.priority));

        let mut chars: Vec<char> = text.chars().collect();
        let mut taken: Vec<(usize, usize)> = Vec::new();
        for lint in lints.iter().rev() {
            if taken
                .iter()
                .any(|(s, e)| lint.span.start < *e && *s < lint.span.end.max(lint.span.start + 1))
            {
                continue;
            }
            taken.push((lint.span.start, lint.span.end.max(lint.span.start + 1)));
            apply_suggestion(
                &mut chars,
                lint.span.start,
                lint.span.end,
                &lint.suggestions[0],
            );
        }
        let next: String = chars.into_iter().collect();
        if next == text {
            break;
        }
        text = next;
    }
    text
}

fn apply_suggestion(chars: &mut Vec<char>, start: usize, end: usize, suggestion: &Suggestion) {
    match suggestion {
        Suggestion::ReplaceWith(with) => {
            chars.splice(start..end, with.iter().copied());
        }
        Suggestion::InsertAfter(with) => {
            chars.splice(end..end, with.iter().copied());
        }
        Suggestion::Remove => {
            chars.drain(start..end);
        }
    }
}

/// Runs the model sentence by sentence and applies only the small edits the app would show.
fn apply_ai(corrector: &mut QwenCorrector, text: &str) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    for sentence in split_sentences(text).into_iter().rev() {
        let Ok(corrected) = corrector.correct(&sentence.text) else {
            continue;
        };
        let edits = compute_edits(&sentence.text, &corrected);
        for edit in edits.iter().rev() {
            let start = sentence.span.start + edit.span.start;
            let end = sentence.span.start + edit.span.end;
            chars.splice(start..end, edit.replacement.chars());
        }
    }
    chars.into_iter().collect()
}

#[derive(Default)]
struct Score {
    fixed: usize,
    wrong: usize,
    left_alone: usize,
    clean: usize,
    time: Duration,
    by_category: BTreeMap<String, (usize, usize)>,
}

impl Score {
    fn record(&mut self, case: &Case, output: &str, time: Duration) {
        self.time += time;
        let output = output.trim();
        let entry = self.by_category.entry(case.category.clone()).or_default();
        entry.1 += 1;
        if case.has_error {
            self.wrong += 1;
            if case.targets.iter().any(|t| t.trim() == output) {
                self.fixed += 1;
                entry.0 += 1;
            }
        } else {
            self.clean += 1;
            if output == case.source.trim() {
                self.left_alone += 1;
                entry.0 += 1;
            }
        }
    }

    fn row(&self, name: &str, total: usize) -> String {
        format!(
            "| {name} | {:.0}% ({}/{}) | {:.0}% ({}/{}) | {:.0?} |",
            100.0 * self.fixed as f64 / self.wrong.max(1) as f64,
            self.fixed,
            self.wrong,
            100.0 * self.left_alone as f64 / self.clean.max(1) as f64,
            self.left_alone,
            self.clean,
            self.time / total.max(1) as u32,
        )
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("pass the model directory"));
    let model = match args.next().as_deref() {
        Some("Accurate") => AiModel::Accurate,
        _ => AiModel::Fast,
    };
    let limit = args.next().and_then(|s| s.parse().ok());

    let cases = load_cases(limit);
    let mut linter = LintGroup::new_curated(FstDictionary::curated(), Dialect::American)
        .with_lint_config(harper_core::linting::FlatConfig::new_curated());
    // Without a downloaded model, only Harper's rules are scored.
    let mut corrector = model.is_downloaded(&dir).then(|| {
        QwenCorrector::load(&model.weights_path(&dir), &model.tokenizer_path(&dir))
            .expect("failed to load model")
    });

    let mut harper = Score::default();
    let mut ai = Score::default();
    let mut both = Score::default();
    let mut misses = Vec::new();

    for case in &cases {
        let start = Instant::now();
        let h = apply_harper(&mut linter, &case.source);
        let harper_time = start.elapsed();
        harper.record(case, &h, harper_time);

        let b = match corrector.as_mut() {
            Some(corrector) => {
                let start = Instant::now();
                let a = apply_ai(corrector, &case.source);
                ai.record(case, &a, start.elapsed());

                let start = Instant::now();
                let b = apply_ai(corrector, &h);
                both.record(case, &b, harper_time + start.elapsed());
                b
            }
            None => h,
        };

        let ok = if case.has_error {
            case.targets.iter().any(|t| t.trim() == b.trim())
        } else {
            b.trim() == case.source.trim()
        };
        if !ok {
            misses.push(format!(
                "- [{}] `{}` → `{}`",
                case.category,
                case.source,
                b.trim()
            ));
        }
    }

    println!("## {} on {} sentences\n", model.display_name(), cases.len());
    println!("| System | Fixed | Left correct text alone | Time per sentence |");
    println!("|---|---|---|---|");
    println!("{}", harper.row("Harper rules", cases.len()));
    println!("{}", ai.row("AI model", cases.len()));
    println!("{}", both.row("Harper + AI (the app)", cases.len()));

    println!("\n| Category | Harper | AI | Harper + AI |");
    println!("|---|---|---|---|");
    for (category, (_, total)) in &harper.by_category {
        let pct = |s: &Score| {
            let (ok, _) = s.by_category.get(category).copied().unwrap_or_default();
            format!("{:.0}%", 100.0 * ok as f64 / (*total).max(1) as f64)
        };
        println!(
            "| {category} | {} | {} | {} |",
            pct(&harper),
            pct(&ai),
            pct(&both)
        );
    }

    println!(
        "\n<details><summary>Sentences the app gets wrong ({})</summary>\n",
        misses.len()
    );
    for miss in &misses {
        println!("{miss}");
    }
    println!("\n</details>");
}
