use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::font::SMALL;
use super::layout::*;
use super::theme::FG;
use super::widgets::{background, footer, header, text};

const TOP_PADDING: i32 = 2;
const LINE_HEIGHT: i32 = SMALL.character_size.height as i32;
const CHAR_WIDTH: i32 = SMALL.character_size.width as i32;
const BASELINE: i32 = SMALL.baseline as i32;

pub const TEXT_ROWS: usize = ((CONTENT_HEIGHT - TOP_PADDING) / LINE_HEIGHT) as usize;
pub const TEXT_COLUMNS: usize = ((SCREEN_WIDTH - 2 * MARGIN) / CHAR_WIDTH) as usize;

pub fn text_page<D: DrawTarget<Color = Rgb565>>(d: &mut D, subtitle: &str, lines: &[&str], hint: &str) {
    header(d, subtitle);
    text_lines(d, lines);
    footer(d, hint);
}

pub fn text_lines<D: DrawTarget<Color = Rgb565>>(d: &mut D, lines: &[&str]) {
    background(d, 0, HEADER_HEIGHT, SCREEN_WIDTH, CONTENT_HEIGHT);
    for (i, line) in lines.iter().enumerate() {
        let y = HEADER_HEIGHT + TOP_PADDING + i as i32 * LINE_HEIGHT + BASELINE;
        text(d, line, MARGIN, y, &SMALL, FG);
    }
}
