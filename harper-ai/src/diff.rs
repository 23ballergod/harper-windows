//! Turns a model's corrected sentence into small, reviewable Harper lints.
//!
//! Language models tend to return a whole rewritten sentence. Users want Grammarly-style
//! underlines on just the words that changed, so we diff the original against the correction at
//! the token level and keep each contiguous change as its own lint.

use harper_core::{
    Span,
    linting::{Lint, LintKind, Suggestion},
};
use similar::{Algorithm, DiffOp, capture_diff_slices};

/// One contiguous change the model proposed, in character offsets relative to the sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub span: Span<char>,
    pub original: String,
    pub replacement: String,
}

/// The largest share of a sentence's words the model may change before we treat the output as an
/// unreliable rewrite instead of a correction.
const MAX_CHANGED_WORD_RATIO: f32 = 0.5;

#[derive(Debug, Clone)]
struct Token {
    text: String,
    start: usize,
    end: usize,
}

impl Token {
    fn is_whitespace(&self) -> bool {
        self.text.chars().all(char::is_whitespace)
    }

    fn is_word(&self) -> bool {
        self.text.chars().any(char::is_alphanumeric)
    }
}

/// Splits text into words (letters, digits, apostrophes, inner hyphens), single punctuation marks
/// and whitespace runs, tracking character offsets.
fn tokenize(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let start = i;
        let c = chars[i];

        if c.is_whitespace() {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
        } else if c.is_alphanumeric() {
            while i < chars.len() {
                let ch = chars[i];
                let joins_word = matches!(ch, '\'' | '’' | '-')
                    && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
                if ch.is_alphanumeric() || joins_word {
                    i += 1;
                } else {
                    break;
                }
            }
        } else {
            i += 1;
        }

        tokens.push(Token {
            text: chars[start..i].iter().collect(),
            start,
            end: i,
        });
    }

    tokens
}

fn joined(tokens: &[Token]) -> String {
    tokens.iter().map(|t| t.text.as_str()).collect()
}

/// Normalizes the typographic differences a model introduces without the user asking, so that
/// they never show up as suggestions.
fn normalize(text: &str) -> String {
    text.replace(['’', '‘'], "'").replace(['“', '”'], "\"")
}

/// Computes the minimal edits that turn `original` into `corrected`.
///
/// Returns an empty list when the sentences match, and also when the model changed so much of the
/// sentence that the result is more likely a rewrite (or a hallucination) than a correction.
pub fn compute_edits(original: &str, corrected: &str) -> Vec<Edit> {
    let corrected = normalize(corrected.trim());
    let original_norm = normalize(original);

    if corrected.is_empty() || corrected == original_norm.trim() {
        return Vec::new();
    }

    let old = tokenize(&original_norm);
    let new = tokenize(&corrected);

    // Leading and trailing whitespace of the original is not something the model can see clearly,
    // so pin it in place.
    let old_texts: Vec<&str> = old.iter().map(|t| t.text.as_str()).collect();
    let new_texts: Vec<&str> = new.iter().map(|t| t.text.as_str()).collect();

    let ops = capture_diff_slices(Algorithm::Patience, &old_texts, &new_texts);

    let word_count = old.iter().filter(|t| t.is_word()).count().max(1);
    let mut changed_words = 0;
    let mut edits: Vec<Edit> = Vec::new();

    for op in ops {
        let (old_range, new_range) = match op {
            DiffOp::Equal { .. } => continue,
            DiffOp::Delete {
                old_index,
                old_len,
                new_index,
            } => (old_index..old_index + old_len, new_index..new_index),
            DiffOp::Insert {
                old_index,
                new_index,
                new_len,
            } => (old_index..old_index, new_index..new_index + new_len),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => (
                old_index..old_index + old_len,
                new_index..new_index + new_len,
            ),
        };

        let mut old_tokens = &old[old_range.clone()];
        let new_tokens = &new[new_range];

        // Pure whitespace changes are noise (double spaces are Harper's job).
        if old_tokens.iter().all(Token::is_whitespace)
            && new_tokens.iter().all(Token::is_whitespace)
        {
            continue;
        }

        changed_words += old_tokens
            .iter()
            .filter(|t| t.is_word())
            .count()
            .max(new_tokens.iter().filter(|t| t.is_word()).count());

        let mut replacement = joined(new_tokens);

        // An insertion has no characters to underline, so anchor it on the preceding word (or
        // the following one at the start of a sentence) and fold that word into the replacement.
        let anchor_storage;
        if old_tokens.is_empty() {
            if let Some(prev) = old_range
                .start
                .checked_sub(1)
                .and_then(|i| old[..=i].iter().rposition(|t| !t.is_whitespace()))
            {
                anchor_storage = old[prev..old_range.start].to_vec();
                replacement = format!("{}{}", joined(&anchor_storage), replacement);
            } else if let Some(next) = old[old_range.start..]
                .iter()
                .position(|t| !t.is_whitespace())
            {
                let next = old_range.start + next;
                anchor_storage = old[old_range.start..=next].to_vec();
                replacement = format!("{}{}", replacement, joined(&anchor_storage));
            } else {
                continue;
            }
            old_tokens = &anchor_storage;
        }

        // Trim shared whitespace at either end so the underline hugs the actual words.
        let mut first = 0;
        let mut last = old_tokens.len();
        while first < last
            && old_tokens[first].is_whitespace()
            && replacement.starts_with(&old_tokens[first].text)
        {
            replacement.drain(..old_tokens[first].text.len());
            first += 1;
        }
        while last > first
            && old_tokens[last - 1].is_whitespace()
            && replacement.ends_with(&old_tokens[last - 1].text)
        {
            let new_len = replacement.len() - old_tokens[last - 1].text.len();
            replacement.truncate(new_len);
            last -= 1;
        }
        let old_tokens = &old_tokens[first..last];
        if old_tokens.is_empty() {
            continue;
        }

        let span = Span::new(old_tokens[0].start, old_tokens[old_tokens.len() - 1].end);
        let original_text: String = original.chars().skip(span.start).take(span.len()).collect();

        if normalize(&original_text) == replacement
            || expands_contraction(&original_text, &replacement)
        {
            continue;
        }

        // Merge with the previous edit when only whitespace separates them, which reads as one fix.
        if let Some(prev) = edits.last_mut() {
            let gap: String = original
                .chars()
                .skip(prev.span.end)
                .take(span.start.saturating_sub(prev.span.end))
                .collect();
            if span.start >= prev.span.end && gap.chars().all(char::is_whitespace) && gap.len() <= 1
            {
                prev.replacement = format!("{}{}{}", prev.replacement, gap, replacement);
                prev.span.end = span.end;
                prev.original = original
                    .chars()
                    .skip(prev.span.start)
                    .take(prev.span.len())
                    .collect();
                continue;
            }
        }

        edits.push(Edit {
            span,
            original: original_text,
            replacement,
        });
    }

    if changed_words as f32 / word_count as f32 > MAX_CHANGED_WORD_RATIO {
        return Vec::new();
    }

    edits
}

const NOT_CONTRACTIONS: &[(&str, &str)] = &[
    ("don't", "do not"),
    ("doesn't", "does not"),
    ("didn't", "did not"),
    ("can't", "cannot"),
    ("can't", "can not"),
    ("won't", "will not"),
    ("isn't", "is not"),
    ("aren't", "are not"),
    ("wasn't", "was not"),
    ("weren't", "were not"),
    ("haven't", "have not"),
    ("hasn't", "has not"),
    ("hadn't", "had not"),
    ("wouldn't", "would not"),
    ("shouldn't", "should not"),
    ("couldn't", "could not"),
];

const CONTRACTION_SUFFIXES: &[(&str, &[&str])] = &[
    ("'s", &["is", "has"]),
    ("'re", &["are"]),
    ("'ve", &["have"]),
    ("'ll", &["will"]),
    ("'d", &["would", "had"]),
    ("'m", &["am"]),
];

/// True when the model only spelled out a contraction the writer used on purpose ("He's" → "He
/// is"). That changes the writer's tone without fixing anything.
fn expands_contraction(original: &str, replacement: &str) -> bool {
    let original = normalize(original).to_lowercase();
    let replacement = replacement.to_lowercase();
    let original = original.trim();
    let replacement = replacement.trim();

    if NOT_CONTRACTIONS
        .iter()
        .any(|(short, long)| original == *short && replacement == *long)
    {
        return true;
    }

    CONTRACTION_SUFFIXES.iter().any(|(suffix, expansions)| {
        original.strip_suffix(suffix).is_some_and(|stem| {
            expansions
                .iter()
                .any(|word| replacement == format!("{stem} {word}"))
        })
    })
}

fn classify(edit: &Edit) -> LintKind {
    let is_punct = |s: &str| s.chars().all(|c| !c.is_alphanumeric());
    let strip = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };

    if edit.original.to_lowercase() == edit.replacement.to_lowercase() {
        LintKind::Capitalization
    } else if strip(&edit.original) == strip(&edit.replacement)
        || (is_punct(&edit.original) && is_punct(&edit.replacement))
    {
        LintKind::Punctuation
    } else if edit.replacement.trim().is_empty() {
        LintKind::Redundancy
    } else {
        LintKind::Grammar
    }
}

/// The rule name AI lints are reported under, so users can disable them like any other rule.
pub const AI_RULE_NAME: &str = "AIGrammar";

/// Converts an edit into a Harper lint, shifting it by `offset` characters to place the sentence
/// within the full document.
pub fn edit_to_lint(edit: &Edit, offset: usize) -> Lint {
    let kind = classify(edit);
    let message = if edit.replacement.trim().is_empty() {
        format!("Consider removing “{}”.", edit.original.trim())
    } else {
        format!(
            "Consider “{}” instead of “{}”.",
            edit.replacement.trim(),
            edit.original.trim()
        )
    };

    Lint {
        span: Span::new(edit.span.start + offset, edit.span.end + offset),
        lint_kind: kind,
        suggestions: vec![if edit.replacement.is_empty() {
            Suggestion::Remove
        } else {
            Suggestion::ReplaceWith(edit.replacement.chars().collect())
        }],
        message,
        priority: 96,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(original: &str, edits: &[Edit]) -> String {
        let mut chars: Vec<char> = original.chars().collect();
        for edit in edits.iter().rev() {
            chars.splice(edit.span.start..edit.span.end, edit.replacement.chars());
        }
        chars.into_iter().collect()
    }

    #[test]
    fn identical_sentences_have_no_edits() {
        assert!(compute_edits("This is fine.", "This is fine.").is_empty());
    }

    #[test]
    fn single_word_replacement() {
        let edits = compute_edits(
            "She go to school every day.",
            "She goes to school every day.",
        );
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].original, "go");
        assert_eq!(edits[0].replacement, "goes");
    }

    #[test]
    fn insertion_is_anchored_to_previous_word() {
        let original = "I went to store yesterday.";
        let edits = compute_edits(original, "I went to the store yesterday.");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].original, "to");
        assert_eq!(edits[0].replacement, "to the");
        assert_eq!(apply(original, &edits), "I went to the store yesterday.");
    }

    #[test]
    fn insertion_at_start_anchors_forward() {
        let original = "dog barked loudly at night.";
        let edits = compute_edits(original, "The dog barked loudly at night.");
        assert_eq!(apply(original, &edits), "The dog barked loudly at night.");
    }

    #[test]
    fn adjacent_words_merge_into_one_edit() {
        let original = "Their going to the park later today with friends.";
        let corrected = "They're going to the park later today with friends.";
        let edits = compute_edits(original, corrected);
        assert_eq!(edits.len(), 1);
        assert_eq!(apply(original, &edits), corrected);
    }

    #[test]
    fn multiple_separate_edits_round_trip() {
        let original = "He dont know where the keys is, but he will looks for them.";
        let corrected = "He doesn't know where the keys are, but he will look for them.";
        let edits = compute_edits(original, corrected);
        assert_eq!(edits.len(), 3);
        assert_eq!(apply(original, &edits), corrected);
    }

    #[test]
    fn heavy_rewrites_are_rejected() {
        let edits = compute_edits(
            "The cat sat on the mat.",
            "A large dog was sleeping peacefully outside.",
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn smart_quotes_are_not_suggestions() {
        assert!(compute_edits("It's my friend's car.", "It’s my friend’s car.").is_empty());
    }

    #[test]
    fn expanded_contractions_are_not_suggestions() {
        assert!(
            compute_edits("He's a member of the club.", "He is a member of the club.").is_empty()
        );
        assert!(compute_edits("I don't know why.", "I do not know why.").is_empty());
        let edits = compute_edits("He's a memeber of the club.", "He is a member of the club.");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].replacement, "member");
    }

    #[test]
    fn punctuation_only_change_is_classified() {
        let edits = compute_edits("Hello how are you today?", "Hello, how are you today?");
        assert_eq!(edits.len(), 1);
        assert_eq!(classify(&edits[0]), LintKind::Punctuation);
    }

    #[test]
    fn lints_are_offset_into_the_document() {
        let edits = compute_edits("She go home.", "She goes home.");
        let lint = edit_to_lint(&edits[0], 10);
        assert_eq!(lint.span, Span::new(14, 16));
    }
}
