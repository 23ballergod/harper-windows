//! A small sentence splitter.
//!
//! The model sees one sentence at a time: it keeps prompts short (fast on a CPU) and lets us cache
//! results so only the sentence being typed is re-checked.

use harper_core::Span;

/// Common abbreviations that end in a period without ending the sentence.
const ABBREVIATIONS: &[&str] = &[
    "mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "e.g", "i.e", "inc", "ltd",
    "co", "no", "fig", "approx", "dept", "est", "mt", "u.s",
];

/// A sentence and its location (in characters) within the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    pub span: Span<char>,
    pub text: String,
}

/// Splits `text` into sentences, trimming surrounding whitespace from each.
///
/// Line breaks always end a sentence, since in chat apps and forms they usually separate thoughts.
pub fn split_sentences(text: &str) -> Vec<Sentence> {
    let chars: Vec<char> = text.chars().collect();
    let mut sentences = Vec::new();
    let mut start = 0;
    let mut i = 0;

    let push = |from: usize, to: usize, sentences: &mut Vec<Sentence>| {
        let mut s = from;
        let mut e = to;
        while s < e && chars[s].is_whitespace() {
            s += 1;
        }
        while e > s && chars[e - 1].is_whitespace() {
            e -= 1;
        }
        if s < e {
            sentences.push(Sentence {
                span: Span::new(s, e),
                text: chars[s..e].iter().collect(),
            });
        }
    };

    while i < chars.len() {
        let c = chars[i];

        if c == '\n' {
            push(start, i, &mut sentences);
            start = i + 1;
        } else if matches!(c, '.' | '!' | '?') {
            // Swallow runs like "?!" or "..." and closing quotes or brackets.
            let mut end = i + 1;
            while end < chars.len()
                && matches!(chars[end], '.' | '!' | '?' | '"' | '\'' | '”' | '’' | ')')
            {
                end += 1;
            }

            let at_boundary = end >= chars.len() || chars[end].is_whitespace();

            if at_boundary && !(c == '.' && is_abbreviation(&chars[start..i])) {
                push(start, end, &mut sentences);
                start = end;
            }
            i = end;
            continue;
        }

        i += 1;
    }

    push(start, chars.len(), &mut sentences);
    sentences
}

fn is_abbreviation(before: &[char]) -> bool {
    let word: String = before
        .iter()
        .rev()
        .take_while(|c| c.is_alphanumeric() || **c == '.')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>()
        .to_lowercase();

    // Single letters are initials ("J. R. R. Tolkien").
    word.chars().count() == 1 && word.chars().all(char::is_alphabetic)
        || ABBREVIATIONS.contains(&word.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(text: &str) -> Vec<String> {
        split_sentences(text).into_iter().map(|s| s.text).collect()
    }

    #[test]
    fn splits_on_terminal_punctuation() {
        assert_eq!(
            texts("Hello there. How are you? I'm great!"),
            vec!["Hello there.", "How are you?", "I'm great!"]
        );
    }

    #[test]
    fn keeps_abbreviations_together() {
        assert_eq!(
            texts("I met Dr. Smith today. He was nice."),
            vec!["I met Dr. Smith today.", "He was nice."]
        );
    }

    #[test]
    fn line_breaks_end_sentences() {
        assert_eq!(texts("hey\nwhats up"), vec!["hey", "whats up"]);
    }

    #[test]
    fn spans_point_into_source() {
        let text = "  One.  Two words.";
        for s in split_sentences(text) {
            let slice: String = text.chars().skip(s.span.start).take(s.span.len()).collect();
            assert_eq!(slice, s.text);
        }
    }

    #[test]
    fn decimals_do_not_split() {
        assert_eq!(
            texts("It costs 3.50 dollars."),
            vec!["It costs 3.50 dollars."]
        );
    }
}
