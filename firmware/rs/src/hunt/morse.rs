pub const DOT_UNITS: u32 = 1;
pub const DASH_UNITS: u32 = 3;
pub const SYMBOL_GAP_UNITS: u32 = 1;
pub const LETTER_GAP_UNITS: u32 = 3;
pub const WORD_GAP_UNITS: u32 = 7;
const REPEAT_GAP_UNITS: u32 = 20;

const CODES: [(char, &str); 37] = [
    ('A', ".-"), ('B', "-..."), ('C', "-.-."), ('D', "-.."), ('E', "."), ('F', "..-."),
    ('G', "--."), ('H', "...."), ('I', ".."), ('J', ".---"), ('K', "-.-"), ('L', ".-.."),
    ('M', "--"), ('N', "-."), ('O', "---"), ('P', ".--."), ('Q', "--.-"), ('R', ".-."),
    ('S', "..."), ('T', "-"), ('U', "..-"), ('V', "...-"), ('W', ".--"), ('X', "-..-"),
    ('Y', "-.--"), ('Z', "--.."), ('0', "-----"), ('1', ".----"), ('2', "..---"),
    ('3', "...--"), ('4', "....-"), ('5', "....."), ('6', "-...."), ('7', "--..."),
    ('8', "---.."), ('9', "----."), ('-', "-....-"),
];

pub fn steps(text: &str) -> Vec<(bool, u32)> {
    let mut steps = Vec::new();
    for c in text.to_uppercase().chars() {
        match CODES.iter().find(|(letter, _)| *letter == c) {
            Some((_, code)) => push_letter(&mut steps, code),
            None => steps.push((false, WORD_GAP_UNITS - LETTER_GAP_UNITS)),
        }
    }
    steps.push((false, REPEAT_GAP_UNITS));
    steps
}

fn push_letter(steps: &mut Vec<(bool, u32)>, code: &str) {
    for symbol in code.chars() {
        steps.push((true, if symbol == '-' { DASH_UNITS } else { DOT_UNITS }));
        steps.push((false, SYMBOL_GAP_UNITS));
    }
    steps.push((false, LETTER_GAP_UNITS - SYMBOL_GAP_UNITS));
}
