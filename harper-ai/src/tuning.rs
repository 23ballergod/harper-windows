//! Removes rule-based suggestions that are usually wrong in everyday writing.
//!
//! Measured on the tricky test set, most of Harper's false alarms come from a few patterns:
//! people's and places' names "corrected" into dictionary words, chat shorthand like "lol", and
//! doubled words that are actually grammatical ("I had had enough").

use harper_core::linting::{Lint, LintKind};

/// Informal words people type on purpose in chats and messages.
const CHAT_WORDS: &[&str] = &[
    "ok", "okay", "lol", "lmao", "lmk", "btw", "idk", "omg", "tbh", "imo", "imho", "brb", "thx",
    "pls", "plz", "ty", "np", "fyi", "asap", "gonna", "wanna", "gotta", "kinda", "sorta", "yeah",
    "yep", "nope", "haha", "hahaha", "hmm", "ugh", "nvm", "rn", "irl", "ngl", "smh", "tho", "ya",
    "yall", "y'all", "u", "ur", "k", "kk",
];

/// Words that are correct when doubled ("had had", "that that").
const LEGITIMATE_DOUBLES: &[&str] = &["had", "that", "is", "do"];

fn is_sentence_start(chars: &[char], index: usize) -> bool {
    let before = chars[..index]
        .iter()
        .rev()
        .find(|c| !c.is_whitespace() && !matches!(c, '"' | '“' | '(' | '\''));
    match before {
        None => true,
        Some(c) => matches!(c, '.' | '!' | '?' | '\n' | ':'),
    }
}

fn starts_new_line(chars: &[char], index: usize) -> bool {
    chars[..index]
        .iter()
        .rev()
        .take_while(|c| **c != '\n')
        .all(|c| c.is_whitespace())
}

/// Returns `false` for a lint that is very likely a false alarm.
pub fn is_probably_helpful(chars: &[char], lint: &Lint) -> bool {
    let end = lint.span.end.min(chars.len());
    if lint.span.start >= end {
        return true;
    }
    let text: String = chars[lint.span.start..end].iter().collect();
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.len() == 1
        && CHAT_WORDS.contains(&words[0].trim_matches(|c: char| !c.is_alphanumeric() && c != '\''))
    {
        return false;
    }

    if lint.lint_kind == LintKind::Spelling {
        // Numbers and decades ("'90s", "4th").
        if text.chars().any(|c| c.is_ascii_digit()) {
            return false;
        }
        if lint.span.start > 0 && chars[lint.span.start - 1] == '\'' {
            return false;
        }
        // A capitalized word mid-sentence is almost always a name the dictionary doesn't know.
        let capitalized = text.chars().next().is_some_and(char::is_uppercase);
        if capitalized
            && !is_sentence_start(chars, lint.span.start)
            && !starts_new_line(chars, lint.span.start)
        {
            return false;
        }
    }

    if lint.lint_kind == LintKind::Repetition
        && words.len() == 2
        && words[0] == words[1]
        && LEGITIMATE_DOUBLES.contains(&words[0])
    {
        return false;
    }

    true
}

/// Drops likely false alarms from every rule's lints.
pub fn remove_false_alarms<'a>(chars: &[char], lints: impl IntoIterator<Item = &'a mut Vec<Lint>>) {
    for list in lints {
        list.retain(|lint| is_probably_helpful(chars, lint));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harper_core::Span;

    fn lint(text: &str, word: &str, kind: LintKind) -> (Vec<char>, Lint) {
        let start = text.find(word).unwrap();
        let start = text[..start].chars().count();
        let chars: Vec<char> = text.chars().collect();
        (
            chars,
            Lint {
                span: Span::new(start, start + word.chars().count()),
                lint_kind: kind,
                ..Default::default()
            },
        )
    }

    #[test]
    fn names_mid_sentence_are_not_misspellings() {
        let (chars, l) = lint(
            "My friend Priyanka lives here.",
            "Priyanka",
            LintKind::Spelling,
        );
        assert!(!is_probably_helpful(&chars, &l));
    }

    #[test]
    fn misspellings_at_sentence_start_still_count() {
        let (chars, l) = lint(
            "Teh cat sat. Becuase it was tired.",
            "Becuase",
            LintKind::Spelling,
        );
        assert!(is_probably_helpful(&chars, &l));
    }

    #[test]
    fn lowercase_misspellings_still_count() {
        let (chars, l) = lint("I recieved it.", "recieved", LintKind::Spelling);
        assert!(is_probably_helpful(&chars, &l));
    }

    #[test]
    fn chat_shorthand_is_left_alone() {
        let (chars, l) = lint("lol that's hilarious", "lol", LintKind::Spelling);
        assert!(!is_probably_helpful(&chars, &l));
        let (chars, l) = lint("ok", "ok", LintKind::Style);
        assert!(!is_probably_helpful(&chars, &l));
    }

    #[test]
    fn grammatical_doubles_are_left_alone() {
        let (chars, l) = lint("I had had enough by then.", "had had", LintKind::Repetition);
        assert!(!is_probably_helpful(&chars, &l));
        let (chars, l) = lint("I went to to the store.", "to to", LintKind::Repetition);
        assert!(is_probably_helpful(&chars, &l));
    }

    #[test]
    fn decades_are_not_misspellings() {
        let (chars, l) = lint("The '90s were fun.", "90s", LintKind::Spelling);
        assert!(!is_probably_helpful(&chars, &l));
    }
}
