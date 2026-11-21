use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::keys::{self, is_back, Key};
use crate::ui;

const MAX_BYTES: u64 = 4096;
const HINT: &str = "UP/DOWN scroll   B back";
const UNPRINTABLE: char = '.';

pub struct Viewer {
    title: String,
    lines: Vec<String>,
    top: usize,
}

impl Viewer {
    pub fn open(path: &Path, title: String) -> Self {
        let lines = match read_start(path) {
            Ok(bytes) => text_lines(&bytes),
            Err(e) => vec![format!("read failed: {e}")],
        };
        Self { title, lines, top: 0 }
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        ui::text_page(d, &self.title, &self.visible(), HINT);
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) -> bool {
        if is_back(k) {
            return true;
        }
        if let Some(top) = self.scrolled(k) {
            self.top = top;
            ui::text_lines(d, &self.visible());
        }
        false
    }

    fn scrolled(&self, k: Key) -> Option<usize> {
        let last_top = self.lines.len().saturating_sub(ui::TEXT_ROWS);
        let top = match k {
            keys::UP => self.top.saturating_sub(1),
            keys::DOWN => (self.top + 1).min(last_top),
            _ => return None,
        };
        (top != self.top).then_some(top)
    }

    fn visible(&self) -> Vec<&str> {
        self.lines.iter().skip(self.top).take(ui::TEXT_ROWS).map(String::as_str).collect()
    }
}

fn read_start(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_BYTES).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn text_lines(bytes: &[u8]) -> Vec<String> {
    bytes.split(|&b| b == b'\n').flat_map(|line| wrap(&printable(line))).collect()
}

fn printable(line: &[u8]) -> Vec<char> {
    line.iter().filter(|&&b| b != b'\r').map(|&b| shown_as(b)).collect()
}

fn shown_as(b: u8) -> char {
    match b {
        b'\t' => ' ',
        b' '..=b'~' => b as char,
        _ => UNPRINTABLE,
    }
}

fn wrap(line: &[char]) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    line.chunks(ui::TEXT_COLUMNS).map(|chunk| chunk.iter().collect()).collect()
}
