// Chat emotes: a message that is exactly one of these words is drawn as a picture in the chat log,
// on every badge that receives it. The word itself is still what goes over the air.

use crate::ui::{self, Animation, Bitmap};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Emote {
    Jaffalot,
    Tengu,
    Fih,
    Kakapo,
}

pub enum Picture {
    Mono(&'static Bitmap),
    Animated(&'static Animation),
}

#[derive(Clone, Copy)]
enum Match {
    // Ignores case and surrounding spaces.
    Loose,
    // The whole message must be exactly the word: no spaces, no capitals, nothing else.
    Exact,
}

const WORDS: [(&str, Match, Emote); 4] = [
    ("jaffalicious", Match::Loose, Emote::Jaffalot),
    ("longnose", Match::Loose, Emote::Tengu),
    ("fih", Match::Exact, Emote::Fih),
    ("marikapo", Match::Loose, Emote::Kakapo),
];

impl Emote {
    pub fn from_text(text: &str) -> Option<Emote> {
        let matches = |word: &str, rule: Match| match rule {
            Match::Loose => text.trim().eq_ignore_ascii_case(word),
            Match::Exact => text == word,
        };
        WORDS.iter().find(|&&(word, rule, _)| matches(word, rule)).map(|&(_, _, emote)| emote)
    }

    pub fn picture(self) -> Picture {
        match self {
            Emote::Jaffalot => Picture::Mono(&ui::JAFFALOT_EMOTE),
            Emote::Tengu => Picture::Mono(&ui::TENGU_EMOTE),
            // Original drawing of the "fih" meme idea (tools/images/fih.png).
            Emote::Fih => Picture::Mono(&ui::FIH_EMOTE),
            Emote::Kakapo => Picture::Animated(&ui::KAKAPO_EMOTE),
        }
    }

    pub fn height(self) -> u32 {
        match self.picture() {
            Picture::Mono(bitmap) => bitmap.height,
            Picture::Animated(animation) => animation.height,
        }
    }
}
