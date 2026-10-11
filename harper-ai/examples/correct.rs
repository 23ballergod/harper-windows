//! Loads a downloaded model and runs it over a small set of sentences, printing what it suggests.
//!
//! Usage: `cargo run -p harper-ai --release --example correct -- <model dir> [Fast|Accurate]`

use std::time::Instant;

use harper_ai::{AiModel, Corrector, QwenCorrector, compute_edits};

/// Sentences with a mistake, paired with the fix a good checker should make.
const BROKEN: &[(&str, &str)] = &[
    ("She go to school every day.", "goes"),
    ("I could of been a contender.", "could have"),
    ("Their going to the park later.", "They're"),
    ("He don't like apples very much.", "doesn't"),
    ("We was late for the meeting yesterday.", "were"),
    ("I have less friends than my brother.", "fewer"),
    ("Me and him went to the store.", "He and I"),
    ("The data shows that alot of people agree.", "a lot"),
    ("Its a beautiful day outside today.", "It's"),
    (
        "Everyone should bring their own lunch tomorrow, they will need it.",
        ";",
    ),
];

/// Correct sentences that should come back unchanged.
const CLEAN: &[&str] = &[
    "The meeting has been moved to Thursday afternoon.",
    "I think we should try the new restaurant downtown.",
    "Please send me the report by the end of the day.",
    "She has lived in Chicago for ten years.",
];

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("pass the model directory"));
    let model = match args.next().as_deref() {
        Some("Accurate") => AiModel::Accurate,
        _ => AiModel::Fast,
    };

    let start = Instant::now();
    let mut corrector = QwenCorrector::load(&model.weights_path(&dir), &model.tokenizer_path(&dir))
        .expect("failed to load model");
    println!("Loaded {} in {:.1?}", model.display_name(), start.elapsed());

    let mut caught = 0;
    let mut total_time = std::time::Duration::ZERO;

    for (sentence, expected) in BROKEN {
        let start = Instant::now();
        let corrected = corrector.correct(sentence).expect("inference failed");
        let elapsed = start.elapsed();
        total_time += elapsed;
        let edits = compute_edits(sentence, &corrected);
        let hit = !edits.is_empty() && corrected.contains(expected);
        caught += hit as usize;
        println!(
            "[{}] {:.1?}\n  in:  {sentence}\n  out: {corrected}\n  edits: {:?}",
            if hit { "ok" } else { "MISS" },
            elapsed,
            edits
                .iter()
                .map(|e| format!("{} -> {}", e.original, e.replacement))
                .collect::<Vec<_>>()
        );
    }

    let mut false_alarms = 0;
    for sentence in CLEAN {
        let start = Instant::now();
        let corrected = corrector.correct(sentence).expect("inference failed");
        total_time += start.elapsed();
        let edits = compute_edits(sentence, &corrected);
        if !edits.is_empty() {
            false_alarms += 1;
            println!("[FALSE ALARM]\n  in:  {sentence}\n  out: {corrected}");
        }
    }

    let runs = BROKEN.len() + CLEAN.len();
    println!(
        "\nCaught {caught}/{} mistakes, {false_alarms}/{} false alarms, {:.0?} per sentence on average",
        BROKEN.len(),
        CLEAN.len(),
        total_time / runs as u32
    );

    assert!(
        caught * 2 >= BROKEN.len(),
        "the model caught fewer than half of the known mistakes"
    );
}
