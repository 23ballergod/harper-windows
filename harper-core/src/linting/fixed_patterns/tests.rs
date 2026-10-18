use super::*;
use crate::linting::tests::{assert_no_lints, assert_suggestion_result};

macro_rules! fixes {
    ($name:ident, $linter:expr, [$(($bad:literal, $good:literal)),* $(,)?]) => {
        #[test]
        fn $name() {
            $( assert_suggestion_result($bad, $linter, $good); )*
        }
    };
}

macro_rules! allows {
    ($name:ident, $linter:expr, [$($ok:literal),* $(,)?]) => {
        #[test]
        fn $name() {
            $( assert_no_lints($ok, $linter); )*
        }
    };
}

fixes!(
    your_youre_fixes,
    YourYoure,
    [
        ("Your welcome to join us.", "You're welcome to join us."),
        ("Your going to love it.", "You're going to love it."),
        ("Your gonna regret that.", "You're gonna regret that."),
        ("Your welcome!", "You're welcome!"),
        ("Your kidding me.", "You're kidding me."),
        ("I think your going to win.", "I think you're going to win."),
        (
            "Your not supposed to be here.",
            "You're not supposed to be here."
        ),
        ("Your so kind.", "You're so kind."),
        ("Your right.", "You're right."),
        (
            "Thanks, your welcome back anytime.",
            "Thanks, you're welcome back anytime."
        ),
    ]
);

allows!(
    your_youre_allows,
    YourYoure,
    [
        "Your welcome mat is dirty.",
        "You're welcome to join us.",
        "Your going-away party was fun.",
        "Your right hand is shaking.",
        "Your so-called friend left.",
        "Your very own copy is here.",
        "Your welcome speech was great.",
        "I appreciate your going to the trouble.",
        "Your wrong answer cost us.",
        "Your not knowing hurt us.",
    ]
);

fixes!(
    were_we_are_fixes,
    WereWeAre,
    [
        ("Were going to be late.", "We're going to be late."),
        ("Were gonna miss the bus!", "We're gonna miss the bus!"),
        (
            "Hurry, were going to be late.",
            "Hurry, we're going to be late."
        ),
        ("Were leaving at noon.", "We're leaving at noon."),
        ("Were trying our best.", "We're trying our best."),
    ]
);

allows!(
    were_we_are_allows,
    WereWeAre,
    [
        "We were going to be late.",
        "Were you going to call?",
        "Were they coming with us?",
        "They were going home.",
        "If it were going well, we'd know.",
        "Were it not for you, I'd be lost.",
        "We're going to be late.",
    ]
);

fixes!(
    lets_fixes,
    LetsLetUs,
    [
        ("Lets go to the beach.", "Let's go to the beach."),
        ("Lets get started.", "Let's get started."),
        ("Okay, lets do it.", "Okay, let's do it."),
        ("So lets try again.", "So let's try again."),
        ("Lets see what happens.", "Let's see what happens."),
        ("Lets not forget the keys.", "Let's not forget the keys."),
        ("LETS GO!", "LET'S GO!"),
    ]
);

allows!(
    lets_allows,
    LetsLetUs,
    [
        "Let's go to the beach.",
        "He lets go of the rope.",
        "She lets me borrow her car.",
        "The app lets you edit files.",
        "Lets are short-term rentals.",
        "My mom lets us stay up late.",
        "It lets go when you press the button.",
    ]
);

fixes!(
    come_too_fixes,
    ComeTooEos,
    [
        ("I want to come to.", "I want to come too."),
        ("I want to go to!", "I want to go too!"),
        ("She wants to come to.", "She wants to come too."),
        ("We would love to come to.", "We would love to come too."),
        ("I'd like to come to.", "I'd like to come too."),
        ("I wanna come to.", "I wanna come too."),
        (
            "Can I come? I want to go to.",
            "Can I come? I want to go too."
        ),
    ]
);

allows!(
    come_too_allows,
    ComeTooEos,
    [
        "I want to come too.",
        "I want to come to the party.",
        "Where do you want to go to?",
        "She was starting to come to.",
        "I want to go to school.",
        "I want to come to terms with it.",
        "He came to.",
    ]
);

fixes!(
    lie_down_fixes,
    LayDownLieDown,
    [
        ("I'm going to lay down.", "I'm going to lie down."),
        (
            "I need to lay down for a bit.",
            "I need to lie down for a bit."
        ),
        (
            "You should lay down on the couch.",
            "You should lie down on the couch."
        ),
        ("Let's lay down and rest.", "Let's lie down and rest."),
        ("I'm gonna lay down in bed.", "I'm gonna lie down in bed."),
        (
            "I was laying down when you called.",
            "I was lying down when you called."
        ),
        (
            "She's laying down on the sofa.",
            "She's lying down on the sofa."
        ),
        ("He lays down after lunch.", "He lies down after lunch."),
        ("Lay down!", "Lie down!"),
        ("You can lay down here.", "You can lie down here."),
    ]
);

allows!(
    lie_down_allows,
    LayDownLieDown,
    [
        "I'm going to lie down.",
        "Lay down the law.",
        "You need to lay down some ground rules.",
        "They lay down their weapons.",
        "Please lay down your cards.",
        "She lay down on the bed yesterday.",
        "We lay down on the grass after the race.",
        "They are laying down new tiles.",
        "We need to lay down a foundation.",
        "She was laying down the law.",
        "Lay the baby down gently.",
    ]
);

fixes!(
    tastes_good_fixes,
    TastesGood,
    [
        ("The food tastes well.", "The food tastes good."),
        (
            "This soup tastes well with bread.",
            "This soup tastes good with bread."
        ),
        ("The cake tasted well.", "The cake tasted good."),
        ("Your perfume smells well.", "Your perfume smells good."),
        ("It tastes well, thanks.", "It tastes good, thanks."),
        (
            "Everything tasted well at the party.",
            "Everything tasted good at the party."
        ),
        ("THE FOOD TASTES WELL.", "THE FOOD TASTES GOOD."),
    ]
);

allows!(
    tastes_good_allows,
    TastesGood,
    [
        "The food tastes good.",
        "The wine tastes well-balanced.",
        "It tastes sweet and smells good as well.",
        "Dogs can smell well.",
        "I can't taste well with this cold.",
        "It tastes as well as it looks.",
        "It smells well enough to eat.",
        "She cooks well.",
    ]
);

fixes!(
    here_is_plural_fixes,
    HereIsPlural,
    [
        ("Here is the documents.", "Here are the documents."),
        (
            "Here is the documents you asked for.",
            "Here are the documents you asked for."
        ),
        ("Here's the files.", "Here are the files."),
        (
            "Here is my notes from the meeting.",
            "Here are my notes from the meeting."
        ),
        ("Here is the results.", "Here are the results."),
        ("OK, here is the keys.", "OK, here are the keys."),
    ]
);

allows!(
    here_is_plural_allows,
    HereIsPlural,
    [
        "Here are the documents.",
        "Here is the news.",
        "Here is the document.",
        "Here is the documents folder.",
        "Here is the series finale.",
        "Here is the results page.",
        "Here's the thing.",
        "Here is my analysis.",
    ]
);

fixes!(
    fewer_many_fixes,
    FewerMany,
    [
        (
            "Less people came than expected.",
            "Fewer people came than expected."
        ),
        ("Much people came.", "Many people came."),
        (
            "There were less cars on the road.",
            "There were fewer cars on the road."
        ),
        (
            "I made less mistakes this time.",
            "I made fewer mistakes this time."
        ),
        ("How much people live here?", "How many people live here?"),
        ("Eat less calories.", "Eat fewer calories."),
        ("We have less options now.", "We have fewer options now."),
    ]
);

allows!(
    fewer_many_allows,
    FewerMany,
    [
        "Fewer people came.",
        "Less water is better.",
        "Much time was wasted.",
        "There were more or less people than we planned for.",
        "Thank you so much people!",
        "It costs much less.",
        "Less people-oriented companies struggle.",
        "We need less money.",
    ]
);

fixes!(
    more_short_adjective_fixes,
    MoreShortAdjective,
    [
        ("This one is more cheap.", "This one is cheaper."),
        (
            "This one is more cheap than that one.",
            "This one is cheaper than that one."
        ),
        ("My car is more fast.", "My car is faster."),
        ("The new house is more big!", "The new house is bigger!"),
        (
            "This test was more easy than the last.",
            "This test was easier than the last."
        ),
        ("It's more hot.", "It's hotter."),
    ]
);

allows!(
    more_short_adjective_allows,
    MoreShortAdjective,
    [
        "This one is cheaper.",
        "We need more cheap labor.",
        "He is more lucky than smart.",
        "The more cheap the better.",
        "There are no more big ideas.",
        "It is more fun.",
        "This is more fast-paced.",
        "I want more good news.",
    ]
);

fixes!(
    insist_on_fixes,
    InsistOn,
    [
        ("He insisted to pay.", "He insisted on paying."),
        (
            "She insists to come with us.",
            "She insists on coming with us."
        ),
        (
            "They insisted to see the manager.",
            "They insisted on seeing the manager."
        ),
        ("My dad insisted to drive.", "My dad insisted on driving."),
        (
            "He keeps insisting to help.",
            "He keeps insisting on helping."
        ),
        ("I insist to stay.", "I insist on staying."),
    ]
);

allows!(
    insist_on_allows,
    InsistOn,
    [
        "He insisted on paying.",
        "He insisted to me that he was right.",
        "She insisted to the court that she was innocent.",
        "I insist that you stay.",
        "They insisted upon going.",
    ]
);

fixes!(
    compound_subject_fixes,
    CompoundSubjectPronoun,
    [
        ("Me and him went home.", "He and I went home."),
        ("Her and I are friends.", "She and I are friends."),
        ("Us students need help.", "We students need help."),
        ("Myself and Sarah will go.", "Sarah and I will go."),
        ("Him and her were late.", "He and she were late."),
        ("Sarah and me went shopping.", "Sarah and I went shopping."),
        ("He and me are cousins.", "He and I are cousins."),
        ("Them and us always fight.", "They and we always fight."),
        (
            "Sarah and myself will handle it.",
            "Sarah and I will handle it."
        ),
    ]
);

allows!(
    compound_subject_allows,
    CompoundSubjectPronoun,
    [
        "He and I went home.",
        "She and I are friends.",
        "We students need help.",
        "Her and her sister went home.",
        "Give it to me and him.",
        "He invited everyone, me and him included.",
        "Me and him.",
        "Us versus them is a bad mindset.",
        "Sarah and I will go.",
        "Her dog and I went for a walk.",
    ]
);

fixes!(
    perfect_past_fixes,
    PerfectWithPastTime,
    [
        (
            "I have seen that movie last week.",
            "I saw that movie last week."
        ),
        (
            "She has finished the report yesterday.",
            "She finished the report yesterday."
        ),
        (
            "We have visited Paris two years ago.",
            "We visited Paris two years ago."
        ),
        ("I've lost my keys yesterday.", "I lost my keys yesterday."),
        (
            "He has already left yesterday.",
            "He already left yesterday."
        ),
        (
            "They have bought a house last year.",
            "They bought a house last year."
        ),
        ("I have met him last Monday.", "I met him last Monday."),
    ]
);

allows!(
    perfect_past_allows,
    PerfectWithPastTime,
    [
        "I saw that movie last week.",
        "I have seen it before last week ended.",
        "I have lived here since last year.",
        "I would have gone last week.",
        "It seems to have happened yesterday.",
        "I have read the book you recommended last week.",
        "I have been sick since yesterday.",
        "I have been there last year.",
        "Have you seen it yesterday?",
        "I have seen it many times, and I saw it again yesterday.",
        "She has worked here for two years.",
        "I have finished the report that was due last week.",
    ]
);
