//! Scores the app's real checking pipeline on `tests/data/tricky.jsonl`: the mistakes Grammarly is
//! known for catching, plus already-correct sentences that should be left alone.
//!
//! Runs Harper's rules alone, the AI model alone, and both the way the app combines them: each
//! checks the original text and only one suggestion is kept where they overlap. A sentence counts as fixed when accepting every suggestion produces one of
//! the reference answers exactly.
//!
//! Usage: `cargo run -p harper-ai --release --example benchmark -- <model dir> [Fast|Accurate | <file.gguf> <tokenizer.json>] [limit]`

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

/// A replacement of `chars[start..end]`, in characters of the original text.
type CharEdit = (usize, usize, Vec<char>);

/// Harper's first suggestion for every lint on the original text, as the app shows them.
fn harper_edits(linter: &mut LintGroup, text: &str) -> Vec<CharEdit> {
    let chars: Vec<char> = text.chars().collect();
    let doc = Document::new_plain_english_curated(text);
    linter
        .organized_lints(&doc)
        .into_values()
        .flatten()
        .filter(|l| !l.suggestions.is_empty())
        .filter(|l| is_probably_helpful(&chars, l))
        .map(|l| match &l.suggestions[0] {
            Suggestion::ReplaceWith(with) => (l.span.start, l.span.end, with.clone()),
            Suggestion::InsertAfter(with) => (l.span.end, l.span.end, with.clone()),
            Suggestion::Remove => (l.span.start, l.span.end, Vec::new()),
        })
        .collect()
}

/// Runs the model sentence by sentence on the original text and keeps only the small edits the
/// app would show.
fn ai_edits(corrector: &mut QwenCorrector, text: &str) -> Vec<CharEdit> {
    let mut out = Vec::new();
    for sentence in split_sentences(text) {
        let Ok(corrected) = corrector.correct(&sentence.text) else {
            continue;
        };
        for edit in compute_edits(&sentence.text, &corrected) {
            out.push((
                sentence.span.start + edit.span.start,
                sentence.span.start + edit.span.end,
                edit.replacement.chars().collect(),
            ));
        }
    }
    out
}

fn overlaps(a: &CharEdit, b: &CharEdit) -> bool {
    a.0 < b.1.max(b.0 + 1) && b.0 < a.1.max(a.0 + 1)
}

/// Applies every `first` edit, then each `second` edit that doesn't overlap one already taken.
/// This is how the app merges the two checkers: the first one wins where both flag the same words.
fn merge_and_apply(text: &str, first: &[CharEdit], second: &[CharEdit]) -> String {
    let mut taken: Vec<&CharEdit> = Vec::new();
    for edit in first.iter().chain(second) {
        if !taken.iter().any(|t| overlaps(t, edit)) {
            taken.push(edit);
        }
    }
    taken.sort_by_key(|e| (e.0, e.1));
    let mut chars: Vec<char> = text.chars().collect();
    for (start, end, with) in taken.into_iter().rev() {
        chars.splice(*start..*end, with.iter().copied());
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
    // Either one of the app's models by name, or any GGUF file followed by its tokenizer, to try
    // out candidates before they ship.
    let model_arg = args.next().unwrap_or_default();
    let (name, weights, tokenizer) = if model_arg.ends_with(".gguf") {
        let tokenizer = args.next().expect("pass the tokenizer after the GGUF file");
        (model_arg.clone(), dir.join(&model_arg), dir.join(tokenizer))
    } else {
        let model = match model_arg.as_str() {
            "Accurate" => AiModel::Accurate,
            _ => AiModel::Fast,
        };
        let weights = if model.is_downloaded(&dir) {
            model.weights_path(&dir)
        } else {
            dir.join("missing.gguf")
        };
        (
            model.display_name().to_string(),
            weights,
            model.tokenizer_path(&dir),
        )
    };
    let limit = args.next().and_then(|s| s.parse().ok());

    let cases = load_cases(limit);
    let mut linter = LintGroup::new_curated(FstDictionary::curated(), Dialect::American)
        .with_lint_config(harper_core::linting::FlatConfig::new_curated());
    // Without a downloaded model, only Harper's rules are scored.
    let mut corrector = weights
        .is_file()
        .then(|| QwenCorrector::load(&weights, &tokenizer).expect("failed to load model"));

    let mut harper = Score::default();
    let mut ai = Score::default();
    let mut both = Score::default();
    let mut ai_first = Score::default();
    let mut misses = Vec::new();

    for case in &cases {
        let start = Instant::now();
        let h = apply_harper(&mut linter, &case.source);
        harper.record(case, &h, start.elapsed());

        let start = Instant::now();
        let h_edits = harper_edits(&mut linter, &case.source);
        let harper_time = start.elapsed();

        let b = match corrector.as_mut() {
            Some(corrector) => {
                let start = Instant::now();
                let a_edits = ai_edits(corrector, &case.source);
                let ai_time = start.elapsed();
                ai.record(case, &merge_and_apply(&case.source, &a_edits, &[]), ai_time);

                let b = merge_and_apply(&case.source, &h_edits, &a_edits);
                both.record(case, &b, harper_time + ai_time);
                let a = merge_and_apply(&case.source, &a_edits, &h_edits);
                ai_first.record(case, &a, harper_time + ai_time);
                a
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

    println!("## {} on {} sentences\n", name, cases.len());
    println!("| System | Fixed | Left correct text alone | Time per sentence |");
    println!("|---|---|---|---|");
    println!("{}", harper.row("Harper rules", cases.len()));
    println!("{}", ai.row("AI model", cases.len()));
    println!(
        "{}",
        both.row("Harper + AI (Harper wins overlaps)", cases.len())
    );
    println!(
        "{}",
        ai_first.row("Harper + AI (the app: AI wins overlaps)", cases.len())
    );

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
            pct(&ai_first)
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
