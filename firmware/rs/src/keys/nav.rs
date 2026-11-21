use super::keymap::{Key, A, B, DOWN, ESC, LEFT, OK, RIGHT, UP};

pub fn wrap_selection(k: Key, sel: usize, len: usize) -> Option<usize> {
    match k {
        UP => Some((sel + len - 1) % len),
        DOWN => Some((sel + 1) % len),
        _ => None,
    }
}

pub fn is_back(k: Key) -> bool {
    matches!(k, B | ESC | LEFT)
}

pub fn is_submit(k: Key) -> bool {
    matches!(k, A | OK)
}

pub fn is_go(k: Key) -> bool {
    matches!(k, A | OK | RIGHT)
}
