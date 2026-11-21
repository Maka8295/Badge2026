mod keymap;
mod matrix;
mod nav;

pub use keymap::{char_for, Key, A, B, DEL, DOWN, ESC, LEFT, OK, RIGHT, SHIFT, TAB, UP};
pub use matrix::{COLS, ROWS};
pub use nav::{is_back, is_go, is_submit, wrap_selection};

use crate::board::KeyMatrixPins;
use matrix::{KeyStates, Matrix};

pub struct Keys {
    matrix: Matrix,
    held: KeyStates,
    caps_lock: bool,
}

impl Keys {
    pub fn new(pins: KeyMatrixPins) -> Self {
        Self { matrix: Matrix::new(pins), held: [[false; COLS]; ROWS], caps_lock: false }
    }

    pub fn pressed(&mut self) -> Vec<Key> {
        let now = self.matrix.scan();
        let pressed = newly_pressed(&self.held, &now);
        self.held = now;
        if pressed.contains(&SHIFT) {
            self.caps_lock = !self.caps_lock;
        }
        pressed
    }

    /// SHIFT latches instead of being held: each press toggles this.
    pub fn caps_lock(&self) -> bool {
        self.caps_lock
    }

    pub fn is_held(&self, k: Key) -> bool {
        let (row, col) = index_of(k);
        self.held[row][col]
    }
}

fn newly_pressed(before: &KeyStates, now: &KeyStates) -> Vec<Key> {
    let mut pressed = Vec::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            if now[row][col] && !before[row][col] {
                pressed.push(key_at(row, col));
            }
        }
    }
    pressed
}

fn key_at(row: usize, col: usize) -> Key {
    (row as u8 + 1, col as u8 + 1)
}

fn index_of(k: Key) -> (usize, usize) {
    (usize::from(k.0) - 1, usize::from(k.1) - 1)
}
