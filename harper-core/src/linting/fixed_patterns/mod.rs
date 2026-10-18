//! Narrow, high-precision rules for common fixed-pattern mistakes that need more context than a
//! Weir rule can express (clause-start anchoring, look-behind, or word-specific rewrites).
//!
//! Every rule here works on one chunk (a clause ending at a comma or sentence terminator) at a
//! time and only fires when the surrounding words make the mistake unambiguous.

use crate::{Document, IrregularVerbs, Lint, Span, Token, TokenStringExt};

use super::{LintKind, Linter, Suggestion};

#[cfg(test)]
mod tests;

/// The non-whitespace tokens of a chunk, with lowercase text for easy matching.
struct View<'a> {
    toks: Vec<&'a Token>,
    low: Vec<String>,
    src: &'a [char],
}

impl<'a> View<'a> {
    fn new(chunk: &'a [Token], src: &'a [char]) -> Self {
        let toks: Vec<&Token> = chunk.iter().filter(|t| !t.kind.is_whitespace()).collect();
        let low = toks
            .iter()
            .map(|t| {
                t.get_ch(src)
                    .iter()
                    .map(|c| if *c == '’' { '\'' } else { *c })
                    .collect::<String>()
                    .to_lowercase()
            })
            .collect();
        Self { toks, low, src }
    }

    fn len(&self) -> usize {
        self.toks.len()
    }

    /// Lowercase text of item `i`, or "" when out of range.
    fn w(&self, i: usize) -> &str {
        self.low.get(i).map(String::as_str).unwrap_or("")
    }

    fn is(&self, i: usize, options: &[&str]) -> bool {
        i < self.len() && options.contains(&self.w(i))
    }

    fn is_word(&self, i: usize) -> bool {
        self.toks.get(i).is_some_and(|t| t.kind.is_word())
    }

    /// True when item `i` is past the end of the chunk or is a clause-ending mark.
    fn is_end(&self, i: usize) -> bool {
        match self.toks.get(i) {
            None => true,
            Some(t) => {
                matches!(self.w(i), "." | "!" | "?" | "," | ";" | ":") || t.kind.is_newline()
            }
        }
    }

    fn is_hyphen(&self, i: usize) -> bool {
        self.toks.get(i).is_some_and(|t| t.kind.is_hyphen())
    }

    fn span(&self, from: usize, to: usize) -> Span<char> {
        Span::new(self.toks[from].span.start, self.toks[to].span.end)
    }

    fn text(&self, from: usize, to: usize) -> &'a [char] {
        self.span(from, to).get_content(self.src)
    }
}

fn lint(span: Span<char>, kind: LintKind, suggestion: Suggestion, message: &str) -> Lint {
    Lint {
        span,
        lint_kind: kind,
        suggestions: vec![suggestion],
        message: message.to_owned(),
        priority: 31,
    }
}

/// Replace items `from..=to` with `value`, copying the case of the original text.
fn replace(v: &View, from: usize, to: usize, value: &str, kind: LintKind, message: &str) -> Lint {
    lint(
        v.span(from, to),
        kind,
        Suggestion::replace_with_match_case_str(value, v.text(from, to)),
        message,
    )
}

macro_rules! chunk_rule {
    ($name:ident, $desc:literal, $func:ident) => {
        #[derive(Default)]
        pub struct $name;

        impl Linter for $name {
            fn lint(&mut self, document: &Document) -> Vec<Lint> {
                let src = document.get_source();
                document
                    .iter_chunks()
                    .flat_map(|chunk| $func(&View::new(chunk, src)))
                    .collect()
            }

            fn description(&self) -> &str {
                $desc
            }
        }
    };
}

/// Words that commonly introduce a clause, so the next word acts like a clause start.
const CLAUSE_LEADS: &[&str] = &[
    "think", "know", "hope", "guess", "bet", "if", "when", "because", "but", "and", "so", "maybe",
    "sure", "now", "okay", "ok", "well", "oh", "hey",
];

fn clause_start(v: &View, i: usize) -> bool {
    i == 0 || (v.is_word(i - 1) && v.is(i - 1, CLAUSE_LEADS))
}

// ---------------------------------------------------------------------------------------------

chunk_rule!(
    YourYoure,
    "Corrects `your` to `you're` at the start of a clause when it is followed by words like `welcome to`, `going to` or `kidding`.",
    your_youre
);

fn your_youre(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        if v.w(i) != "your" || !clause_start(v, i) {
            continue;
        }
        let n = i + 1;
        let hit = match v.w(n) {
            "welcome" => v.is(n + 1, &["to", "back"]) || v.is_end(n + 1),
            "going" => v.is(n + 1, &["to"]),
            "gonna" | "kidding" | "joking" => true,
            "not" => v.is(
                n + 1,
                &[
                    "going",
                    "gonna",
                    "supposed",
                    "allowed",
                    "alone",
                    "ready",
                    "sure",
                    "welcome",
                    "invited",
                    "listening",
                    "kidding",
                    "joking",
                ],
            ),
            "so" => !v.is_hyphen(n + 1) && v.toks.get(n + 1).is_some_and(|t| t.kind.is_adjective()),
            "right" | "wrong" => v.is_end(n + 1),
            _ => false,
        };
        if hit {
            out.push(replace(
                v,
                i,
                i,
                "you're",
                LintKind::WordChoice,
                "Use `you're` (you are) here; `your` shows possession.",
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------

chunk_rule!(
    WereWeAre,
    "Corrects `Were going to` at the start of a clause to `We're going to`.",
    were_we_are
);

fn were_we_are(v: &View) -> Vec<Lint> {
    if v.w(0) == "were" && v.is(1, &["going", "gonna", "trying", "leaving", "coming"]) {
        // "Were going concerns..." style noun uses are vanishingly rare, but a following
        // hyphen is a sign of a compound.
        if v.is_hyphen(2) {
            return vec![];
        }
        return vec![replace(
            v,
            0,
            0,
            "we're",
            LintKind::WordChoice,
            "Use `we're` (we are) here; `were` is a past-tense verb.",
        )];
    }
    vec![]
}

// ---------------------------------------------------------------------------------------------

const LETS_VERBS: &[&str] = &[
    "go",
    "get",
    "see",
    "do",
    "make",
    "try",
    "start",
    "eat",
    "talk",
    "play",
    "meet",
    "find",
    "take",
    "move",
    "keep",
    "have",
    "begin",
    "hope",
    "celebrate",
    "grab",
    "check",
    "focus",
    "look",
    "think",
    "work",
    "discuss",
    "review",
    "be",
    "not",
    "just",
    "all",
    "head",
    "watch",
    "call",
    "say",
    "put",
    "stay",
    "wait",
    "dance",
    "sing",
    "ask",
    "help",
    "pretend",
    "agree",
    "build",
    "fix",
    "finish",
    "order",
    "plan",
    "leave",
    "stop",
    "walk",
    "run",
    "sit",
    "hang",
    "dive",
    "jump",
    "figure",
    "give",
    "keep",
    "write",
    "read",
    "use",
    "add",
    "create",
];

chunk_rule!(
    LetsLetUs,
    "Corrects `Lets go` at the start of a clause to `Let's go`.",
    lets_let_us
);

fn lets_let_us(v: &View) -> Vec<Lint> {
    let i = if v.w(0) == "lets" {
        0
    } else if v.w(1) == "lets"
        && v.is(
            0,
            &[
                "so", "now", "ok", "okay", "come", "then", "well", "oh", "hey",
            ],
        )
    {
        1
    } else {
        return vec![];
    };
    if v.is(i + 1, LETS_VERBS) && !v.is_hyphen(i + 2) {
        return vec![replace(
            v,
            i,
            i,
            "let's",
            LintKind::Punctuation,
            "Use `let's` (let us) to make a suggestion; `lets` means `allows`.",
        )];
    }
    vec![]
}

// ---------------------------------------------------------------------------------------------

chunk_rule!(
    ComeTooEos,
    "Corrects `I want to come to.` to `I want to come too.`",
    come_too_eos
);

fn come_too_eos(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        let lead = if v.is(i, &["want", "wants", "wanted", "like", "love"]) && v.w(i + 1) == "to" {
            i + 2
        } else if v.w(i) == "wanna" {
            i + 1
        } else {
            continue;
        };
        if v.is(lead, &["come", "go"])
            && v.w(lead + 1) == "to"
            && (lead + 2 >= v.len() || v.is(lead + 2, &[".", "!"]))
        {
            out.push(replace(
                v,
                lead + 1,
                lead + 1,
                "too",
                LintKind::WordChoice,
                "Use `too` to mean `also`.",
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------

const LIE_AFTER: &[&str] = &[
    "for",
    "on",
    "and",
    "now",
    "here",
    "again",
    "after",
    "until",
    "in",
    "flat",
    "quietly",
    "awhile",
    "there",
    "beside",
    "next",
    "somewhere",
    "when",
    "while",
    "all",
    "because",
];

chunk_rule!(
    LayDownLieDown,
    "Corrects `lay down` used without an object (`I need to lay down`) to `lie down`.",
    lay_down_lie_down
);

fn lay_down_lie_down(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        if v.w(i + 1) != "down" {
            continue;
        }
        let tail_ok = v.is_end(i + 2) || v.is(i + 2, LIE_AFTER);
        if !tail_ok {
            continue;
        }
        let (ok, fix) = match v.w(i) {
            "lay" => (
                (i == 0 && v.is_end(i + 2))
                    || (i > 0
                        && v.is(
                            i - 1,
                            &[
                                "to", "gonna", "wanna", "i'll", "we'll", "you'll", "they'll",
                                "let's", "should", "must", "can", "could", "will", "would",
                                "please", "just", "might",
                            ],
                        )),
                "lie",
            ),
            "laying" => (
                i > 0
                    && v.is(
                        i - 1,
                        &[
                            "am", "is", "are", "was", "were", "been", "be", "i'm", "you're",
                            "we're", "they're", "he's", "she's",
                        ],
                    ),
                "lying",
            ),
            "lays" => (i > 0 && v.is(i - 1, &["he", "she", "it"]), "lies"),
            _ => (false, ""),
        };
        if ok {
            out.push(replace(
                v,
                i,
                i,
                fix,
                LintKind::WordChoice,
                "Without an object, use `lie down` (to recline); `lay` needs something being put down.",
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------

chunk_rule!(
    TastesGood,
    "Corrects `tastes well` and `smells well` to `tastes good` and `smells good`.",
    tastes_good
);

fn tastes_good(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        if v.is(i, &["tastes", "tasted", "smells"])
            && v.w(i + 1) == "well"
            && !v.is_hyphen(i + 2)
            && !v.is(i + 2, &["as", "enough"])
        {
            out.push(replace(
                v,
                i + 1,
                i + 1,
                "good",
                LintKind::Grammar,
                "After a linking verb like `tastes` or `smells`, use the adjective `good`, not the adverb `well`.",
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------

const NOT_PLURAL: &[&str] = &[
    "news",
    "series",
    "species",
    "means",
    "physics",
    "mathematics",
    "economics",
    "politics",
    "ethics",
    "athletics",
    "gymnastics",
    "measles",
    "headquarters",
    "crossroads",
    "lens",
];

chunk_rule!(
    HereIsPlural,
    "Corrects `here is the documents` to `here are the documents`.",
    here_is_plural
);

fn here_is_plural(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        let (be_from, be_to, det, fix) = if v.w(i) == "here" && v.w(i + 1) == "is" {
            (i + 1, i + 1, i + 2, "are")
        } else if v.w(i) == "here's" {
            (i, i, i + 1, "here are")
        } else {
            continue;
        };
        if !v.is(
            det,
            &[
                "the", "my", "your", "our", "his", "her", "their", "these", "those", "all",
            ],
        ) {
            continue;
        }
        let n = det + 1;
        let Some(noun) = v.toks.get(n) else { continue };
        if !(noun.kind.is_plural_noun() && !noun.kind.is_singular_noun()) || v.is(n, NOT_PLURAL) {
            continue;
        }
        let tail_ok = v.is_end(n + 1)
            || v.is(
                n + 1,
                &[
                    "for", "from", "that", "you", "i", "we", "they", "which", "to", "again", "now",
                    "here", "with",
                ],
            );
        if tail_ok {
            out.push(replace(
                v,
                be_from,
                be_to,
                fix,
                LintKind::Agreement,
                "The noun is plural, so use `are`.",
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------

const COUNT_NOUNS: &[&str] = &[
    "people",
    "students",
    "cars",
    "things",
    "items",
    "mistakes",
    "errors",
    "words",
    "problems",
    "books",
    "friends",
    "questions",
    "options",
    "choices",
    "calories",
    "cookies",
    "apples",
    "kids",
    "children",
    "employees",
    "members",
    "users",
    "customers",
    "players",
    "bugs",
    "files",
    "steps",
    "jobs",
    "ideas",
    "features",
    "lines",
    "pages",
    "emails",
    "messages",
    "tickets",
    "chances",
    "reasons",
    "opportunities",
    "countries",
    "cities",
    "houses",
    "animals",
    "dogs",
    "cats",
    "trees",
    "games",
    "songs",
    "movies",
    "teachers",
    "workers",
    "parents",
    "visitors",
    "guests",
    "tourists",
    "fans",
    "voters",
    "patients",
    "doctors",
    "accidents",
    "cases",
    "issues",
    "changes",
    "tasks",
    "meetings",
    "classes",
    "courses",
    "tests",
    "points",
    "goals",
    "products",
    "jobs",
    "shops",
    "stores",
    "restaurants",
    "hotels",
    "rooms",
    "seats",
    "tables",
    "chairs",
    "computers",
    "phones",
    "bottles",
    "boxes",
    "bags",
    "dollars",
    "coins",
    "eggs",
    "potatoes",
    "vegetables",
    "fruits",
];

chunk_rule!(
    FewerMany,
    "Corrects `less` and `much` before a plural countable noun (`less people`, `much people`) to `fewer` and `many`.",
    fewer_many
);

fn fewer_many(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        let fix = match v.w(i) {
            "less" => "fewer",
            "much" => "many",
            _ => continue,
        };
        if !v.is(i + 1, COUNT_NOUNS) || v.is_hyphen(i + 2) || v.is(i + 2, &["'s"]) {
            continue;
        }
        if i > 0 && v.is(i - 1, &["or", "very", "so", "thanks", "thank", "you"]) {
            continue;
        }
        out.push(replace(
            v,
            i,
            i,
            fix,
            LintKind::Agreement,
            if fix == "fewer" {
                "Use `fewer` with plural nouns you can count; `less` is for amounts."
            } else {
                "Use `many` with plural nouns you can count; `much` is for amounts."
            },
        ));
    }
    out
}

// ---------------------------------------------------------------------------------------------

const SHORT_ADJ: &[(&str, &str)] = &[
    ("cheap", "cheaper"),
    ("big", "bigger"),
    ("fast", "faster"),
    ("small", "smaller"),
    ("tall", "taller"),
    ("short", "shorter"),
    ("long", "longer"),
    ("old", "older"),
    ("young", "younger"),
    ("high", "higher"),
    ("low", "lower"),
    ("easy", "easier"),
    ("hard", "harder"),
    ("happy", "happier"),
    ("nice", "nicer"),
    ("large", "larger"),
    ("strong", "stronger"),
    ("weak", "weaker"),
    ("smart", "smarter"),
    ("quick", "quicker"),
    ("slow", "slower"),
    ("warm", "warmer"),
    ("cold", "colder"),
    ("hot", "hotter"),
    ("rich", "richer"),
    ("poor", "poorer"),
    ("safe", "safer"),
    ("busy", "busier"),
    ("heavy", "heavier"),
    ("dark", "darker"),
    ("bright", "brighter"),
    ("clean", "cleaner"),
    ("deep", "deeper"),
    ("thin", "thinner"),
    ("wide", "wider"),
    ("loud", "louder"),
    ("sweet", "sweeter"),
    ("tough", "tougher"),
    ("good", "better"),
    ("bad", "worse"),
];

chunk_rule!(
    MoreShortAdjective,
    "Corrects `more` before a short adjective (`more cheap`) to its `-er` form (`cheaper`).",
    more_short_adjective
);

fn more_short_adjective(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        if v.w(i) != "more" || (i > 0 && v.is(i - 1, &["the", "no", "any", "once"])) {
            continue;
        }
        let Some((_, comp)) = SHORT_ADJ.iter().find(|(a, _)| *a == v.w(i + 1)) else {
            continue;
        };
        let tail_ok = if v.w(i + 2) == "than" {
            // "more lucky than smart" compares two qualities and is correct.
            !v.toks.get(i + 3).is_some_and(|t| t.kind.is_adjective())
        } else {
            v.is_end(i + 2) && !v.is_hyphen(i + 2)
        };
        if tail_ok {
            out.push(replace(
                v,
                i,
                i + 1,
                comp,
                LintKind::Grammar,
                "Short adjectives take `-er` instead of `more`.",
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------

const GERUNDS: &[(&str, &str)] = &[
    ("pay", "paying"),
    ("go", "going"),
    ("come", "coming"),
    ("help", "helping"),
    ("see", "seeing"),
    ("do", "doing"),
    ("stay", "staying"),
    ("drive", "driving"),
    ("take", "taking"),
    ("buy", "buying"),
    ("leave", "leaving"),
    ("make", "making"),
    ("have", "having"),
    ("speak", "speaking"),
    ("talk", "talking"),
    ("use", "using"),
    ("get", "getting"),
    ("walk", "walking"),
    ("cook", "cooking"),
    ("carry", "carrying"),
    ("wait", "waiting"),
    ("join", "joining"),
    ("sit", "sitting"),
    ("meet", "meeting"),
    ("eat", "eating"),
    ("write", "writing"),
    ("call", "calling"),
    ("check", "checking"),
    ("keep", "keeping"),
    ("give", "giving"),
    ("work", "working"),
    ("attend", "attending"),
    ("bring", "bringing"),
    ("try", "trying"),
];

chunk_rule!(
    InsistOn,
    "Corrects `insisted to pay` to `insisted on paying`.",
    insist_on
);

fn insist_on(v: &View) -> Vec<Lint> {
    let mut out = Vec::new();
    for i in 0..v.len() {
        if !v.is(i, &["insist", "insists", "insisted", "insisting"]) || v.w(i + 1) != "to" {
            continue;
        }
        let Some((_, ger)) = GERUNDS.iter().find(|(b, _)| *b == v.w(i + 2)) else {
            continue;
        };
        out.push(replace(
            v,
            i + 1,
            i + 2,
            &format!("on {ger}"),
            LintKind::Usage,
            "`Insist` takes `on` followed by an `-ing` verb.",
        ));
    }
    out
}

// ---------------------------------------------------------------------------------------------

const OBJ_TO_SUBJ: &[(&str, &str)] = &[
    ("me", "I"),
    ("him", "he"),
    ("her", "she"),
    ("them", "they"),
    ("us", "we"),
];

const SUBJ: &[&str] = &["i", "he", "she", "they", "we"];

/// Words that show the pronoun pair is the subject of a following verb.
const VERBISH: &[&str] = &[
    "will", "would", "can", "could", "should", "shall", "may", "might", "must", "are", "were",
    "was", "is", "am", "have", "has", "had", "do", "did", "went", "go", "goes", "need", "want",
    "both", "always", "never", "also", "often", "just", "love", "like", "met", "saw", "ate",
    "played", "made", "got", "plan", "planned", "decided", "live", "lived", "work", "worked",
    "came", "left", "stayed", "tried", "started", "took", "wrote", "became", "agreed", "sat",
    "ran", "walked", "talked", "spent", "bought", "watched", "enjoyed", "used", "don't", "didn't",
    "won't", "can't", "aren't", "weren't", "wasn't", "haven't", "hadn't", "usually",
];

fn subj_of(word: &str) -> Option<&'static str> {
    OBJ_TO_SUBJ
        .iter()
        .find(|(o, _)| *o == word)
        .map(|(_, s)| *s)
}

fn capitalize_like(original: &[char], value: &str) -> Vec<char> {
    let mut chars: Vec<char> = value.chars().collect();
    if original.first().is_some_and(|c| c.is_uppercase())
        && let Some(first) = chars.first_mut()
    {
        *first = first.to_ascii_uppercase();
    }
    chars
}

chunk_rule!(
    CompoundSubjectPronoun,
    "Corrects object pronouns in a compound subject at the start of a clause, such as `Me and him went` to `He and I went`.",
    compound_subject_pronoun
);

fn compound_subject_pronoun(v: &View) -> Vec<Lint> {
    let msg =
        "Use subject pronouns (`I`, `he`, `she`, `we`, `they`) for the people doing the action.";

    // "Us students need ..." -> "We students need ..."
    if v.w(0) == "us"
        && v.toks
            .get(1)
            .is_some_and(|t| t.kind.is_plural_noun() && !t.kind.is_singular_noun())
        && v.is(2, VERBISH)
    {
        return vec![replace(v, 0, 0, "we", LintKind::Grammar, msg)];
    }

    if v.w(1) != "and" || !v.is(3, VERBISH) || v.len() < 4 {
        return vec![];
    }
    let (a, b) = (v.w(0), v.w(2));
    let a_word = v.is_word(0);
    let first_is_name = v.toks[0].kind.is_proper_noun() && !v.toks[0].kind.is_pronoun();
    let second_is_name = v.toks[2].kind.is_proper_noun() && !v.toks[2].kind.is_pronoun();

    let fixed: Option<String> = if a == "myself" && second_is_name {
        Some(format!("{} and I", v.text(2, 2).iter().collect::<String>()))
    } else if b == "myself" && first_is_name {
        Some(format!("{} and I", v.text(0, 0).iter().collect::<String>()))
    } else if first_is_name && subj_of(b).is_some() {
        Some(format!(
            "{} and {}",
            v.text(0, 0).iter().collect::<String>(),
            subj_of(b).unwrap()
        ))
    } else if let Some(sa) = subj_of(a) {
        // Object pronoun first: "Me and him", "Her and I", "Him and her".
        let sb = if b == "i" {
            Some("I")
        } else {
            subj_of(b).or(SUBJ.contains(&b).then_some(b))
        };
        match sb {
            Some(sb) if sa != sb => {
                if sa == "I" {
                    Some(format!("{sb} and I"))
                } else {
                    Some(format!("{sa} and {sb}"))
                }
            }
            _ => None,
        }
    } else if SUBJ.contains(&a) && a != "i" && a_word {
        // "He and me went", "She and him are"
        subj_of(b).filter(|sb| *sb != a).map(|sb| {
            if sb == "I" {
                format!("{a} and I")
            } else {
                format!("{a} and {sb}")
            }
        })
    } else {
        None
    };

    let Some(fixed) = fixed else { return vec![] };
    let original = v.text(0, 2);
    let value = capitalize_like(original, &fixed);
    if value == original {
        return vec![];
    }
    vec![lint(
        v.span(0, 2),
        LintKind::Grammar,
        Suggestion::ReplaceWith(value),
        msg,
    )]
}

// ---------------------------------------------------------------------------------------------

const PAST_UNITS: &[&str] = &[
    "week",
    "month",
    "year",
    "night",
    "weekend",
    "summer",
    "winter",
    "spring",
    "fall",
    "autumn",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "semester",
];

const AGO_UNITS: &[&str] = &[
    "seconds", "minutes", "hours", "days", "weeks", "months", "years", "decades", "second",
    "minute", "hour", "day", "week", "month", "year", "decade", "while",
];

/// Words that tie the time phrase to something other than the perfect verb.
const PERFECT_BLOCKERS: &[&str] = &[
    "since", "before", "until", "till", "after", "by", "from", "for", "when", "ever", "which",
    "who", "because", "while", "and", "but", "or", "than", "where", "what",
];

chunk_rule!(
    PerfectWithPastTime,
    "Flags the present perfect (`have seen`) used with a finished past time (`last week`, `yesterday`, `ago`) and suggests the simple past.",
    perfect_with_past_time
);

fn preterite_for(participle: &str) -> Option<String> {
    if participle == "been" {
        return None;
    }
    if let Some(pt) = IrregularVerbs::curated().get_preterite_for_past_participle(participle) {
        return Some(pt.to_owned());
    }
    participle.ends_with("ed").then(|| participle.to_owned())
}

fn perfect_with_past_time(v: &View) -> Vec<Lint> {
    for i in 0..v.len() {
        let w = v.w(i);
        let (prefix, is_aux) = match w {
            "have" | "has" => (None, true),
            "i've" | "we've" | "you've" | "they've" => (Some(&w[..w.len() - 3]), true),
            _ => (None, false),
        };
        if !is_aux {
            continue;
        }
        if i > 0
            && v.is(
                i - 1,
                &[
                    "would", "could", "should", "must", "might", "may", "will", "to", "shall",
                    "i'd", "you'd", "we'd", "they'd", "he'd", "she'd",
                ],
            )
        {
            continue;
        }
        // Question order ("Have you seen ...") is not handled.
        let mut p = i + 1;
        let adverb = if v.is(p, &["already", "just", "also", "finally", "only"]) {
            p += 1;
            Some(v.w(p - 1).to_owned())
        } else {
            None
        };
        let Some(part) = v.toks.get(p) else { continue };
        if !(part.kind.is_verb_past_participle_form()
            || (part.kind.is_verb_past_form() && v.w(p).ends_with("ed")))
        {
            continue;
        }
        let Some(pret) = preterite_for(v.w(p)) else {
            continue;
        };

        // Find a past-time phrase that ends the clause.
        let mut found = None;
        for k in p + 1..v.len() {
            let end = if v.w(k) == "yesterday" {
                Some(k)
            } else if v.w(k) == "last" && v.is(k + 1, PAST_UNITS) {
                Some(k + 1)
            } else if v.w(k) == "ago" && k > p + 1 && v.is(k - 1, AGO_UNITS) {
                Some(k)
            } else {
                None
            };
            if let Some(end) = end {
                found = Some((k, end));
                break;
            }
        }
        let Some((start_k, end_k)) = found else {
            continue;
        };
        if !v.is_end(end_k + 1) {
            continue;
        }
        let between = p + 1..start_k;
        let blocked = between.clone().any(|j| {
            v.is(j, PERFECT_BLOCKERS)
                || v.is(j, &["i", "you", "he", "she", "we", "they"])
                || v.toks[j].kind.is_verb_simple_past_form()
                || v.toks[j].kind.is_verb()
                    && !v.toks[j].kind.is_noun()
                    && !v.toks[j].kind.is_adjective()
                    && !v.toks[j].kind.is_determiner()
                    && !v.toks[j].kind.is_pronoun()
        });
        if blocked {
            continue;
        }

        let mut value = String::new();
        if let Some(prefix) = prefix {
            value.push_str(
                &v.text(i, i)
                    .iter()
                    .take(prefix.chars().count())
                    .collect::<String>(),
            );
            value.push(' ');
        }
        if let Some(adverb) = adverb {
            value.push_str(&adverb);
            value.push(' ');
        }
        value.push_str(&pret);

        let template = v.text(i, p);
        let suggestion = if prefix.is_some() {
            Suggestion::ReplaceWith(value.chars().collect())
        } else {
            Suggestion::replace_with_match_case(value.chars().collect(), template)
        };
        return vec![lint(
            v.span(i, p),
            LintKind::Grammar,
            suggestion,
            "A finished past time like `yesterday` or `last week` takes the simple past, not `have` + past participle.",
        )];
    }
    vec![]
}
