use crate::chat::NICK_MAX;

const ADJECTIVES: [&str; 32] = [
    "Neon", "Lazy", "Swift", "Shy", "Bold", "Dark", "Wild", "Sly", "Calm", "Grim", "Hazy", "Lucky", "Rusty", "Quiet",
    "Red", "Odd", "Brave", "Fuzzy", "Tiny", "Giant", "Retro", "Turbo", "Crazy", "Happy", "Salty", "Spicy", "Sharp",
    "Cyber", "Rogue", "Hyper", "Mega", "Noisy",
];

const NOUNS: [&str; 32] = [
    "Tengu", "Oni", "Fox", "Ninja", "Ronin", "Kappa", "Neko", "Koi", "Ramen", "Sushi", "Mochi", "Yokai", "Robot",
    "Ghost", "Byte", "Shell", "Root", "Bit", "Pixel", "Crab", "Owl", "Crow", "Wolf", "Shark", "Moth", "Bug", "Gecko",
    "Panda", "Raven", "Comet", "Kaiju", "Mecha",
];

const WORD_INDEX_MASK: u8 = ADJECTIVES.len() as u8 - 1;
const HEX_SUFFIX_LEN: usize = 2;

const fn longest(words: &[&str]) -> usize {
    let mut longest = 0;
    let mut i = 0;
    while i < words.len() {
        if words[i].len() > longest {
            longest = words[i].len();
        }
        i += 1;
    }
    longest
}

const _: () = assert!(longest(&ADJECTIVES) + longest(&NOUNS) + HEX_SUFFIX_LEN <= NICK_MAX);
const _: () = assert!(ADJECTIVES.len() == NOUNS.len() && ADJECTIVES.len().is_power_of_two());

fn from_mac(mac: [u8; 6]) -> String {
    let adjective = ADJECTIVES[(mac[3] & WORD_INDEX_MASK) as usize];
    let noun = NOUNS[(mac[4] & WORD_INDEX_MASK) as usize];
    format!("{adjective}{noun}{:02x}", mac[5])
}

pub fn read() -> String {
    let mut mac = [0u8; 6];
    unsafe { esp_idf_svc::sys::esp_efuse_mac_get_default(mac.as_mut_ptr()) };
    from_mac(mac)
}
