use std::time::{Duration, Instant};

use esp_idf_svc::sys::esp_random;

use super::morse::{self, DASH_UNITS, DOT_UNITS, LETTER_GAP_UNITS, SYMBOL_GAP_UNITS, WORD_GAP_UNITS};
use crate::leds::{self, Rgb, FULL, OFF};

// Drives only the morse LED (chain index 8, top of the left edge). The other LEDs belong to the
// LED theme in leds/effects.rs (the old random breathing is the "Fireflies" theme).
const UNIT: Duration = Duration::from_millis(200);
const ON_UNITS: [u32; 2] = [DOT_UNITS, DASH_UNITS];
const OFF_UNITS: [u32; 4] = [SYMBOL_GAP_UNITS, SYMBOL_GAP_UNITS, LETTER_GAP_UNITS, WORD_GAP_UNITS];
const MORSE_COLOR: Rgb = (FULL, FULL, FULL);

struct Morse {
    steps: Vec<(bool, u32)>,
    index: usize,
}

pub struct Lights {
    lit: Rgb,
    next_morse_change: Instant,
    morse: Option<Morse>,
}

impl Lights {
    pub fn new() -> Self {
        Lights { lit: OFF, next_morse_change: Instant::now(), morse: None }
    }

    pub fn start_morse(&mut self, text: &str) {
        self.morse = Some(Morse { steps: morse::steps(text), index: 0 });
        self.next_morse_change = Instant::now();
    }

    pub fn tick(&mut self) {
        if Instant::now() < self.next_morse_change {
            return;
        }
        let (color, units) = match self.morse.as_mut() {
            Some(morse) => next_morse_step(morse),
            None => next_decoy_step(self.lit, MORSE_COLOR),
        };
        self.lit = color;
        leds::set_morse(color);
        self.next_morse_change = Instant::now() + UNIT * units;
    }
}

fn next_morse_step(morse: &mut Morse) -> (Rgb, u32) {
    let (lit, units) = morse.steps[morse.index];
    morse.index = (morse.index + 1) % morse.steps.len();
    (if lit { MORSE_COLOR } else { OFF }, units)
}

fn next_decoy_step(current: Rgb, color: Rgb) -> (Rgb, u32) {
    if current == OFF {
        (color, ON_UNITS[random_below(ON_UNITS.len())])
    } else {
        (OFF, OFF_UNITS[random_below(OFF_UNITS.len())])
    }
}

fn random_below(limit: usize) -> usize {
    // SAFETY: esp_random has no preconditions.
    unsafe { esp_random() as usize % limit }
}
