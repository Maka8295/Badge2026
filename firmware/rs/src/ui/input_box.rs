use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::font::BODY;
use super::layout::{BORDER_THICKNESS, FOOTER_HEIGHT, MARGIN, SCREEN_HEIGHT, SCREEN_WIDTH};
use super::theme::{ACCENT, BORDER, FG, SURFACE};
use super::widgets::{fill, text_on_background};

const BOX_W: i32 = SCREEN_WIDTH - 2 * MARGIN;
const BOX_H: i32 = 22;
const BOX_BOTTOM_GAP: i32 = 4;
const BOX_TOP: i32 = SCREEN_HEIGHT - FOOTER_HEIGHT - BOX_H - BOX_BOTTOM_GAP;
const BOX_BASELINE: i32 = 17;
const PROMPT_X: i32 = 18;
const TEXT_X: i32 = 34;
const PROMPT_SPACE: i32 = 30;
const CURSOR: &str = "_";
const VISIBLE_CHARS: usize = ((BOX_W - PROMPT_SPACE) / BODY.character_size.width as i32) as usize - CURSOR.len();

pub fn input_box<D: DrawTarget<Color = Rgb565>>(d: &mut D, text: &str) {
    let inner = BORDER_THICKNESS;
    fill(d, MARGIN, BOX_TOP, BOX_W as u32, BOX_H as u32, BORDER);
    fill(d, MARGIN + inner, BOX_TOP + inner, (BOX_W - 2 * inner) as u32, (BOX_H - 2 * inner) as u32, SURFACE);
    let hidden = text.chars().count().saturating_sub(VISIBLE_CHARS);
    let shown: String = text.chars().skip(hidden).collect::<String>() + CURSOR;
    text_on_background(d, ">", PROMPT_X, BOX_TOP + BOX_BASELINE, &BODY, ACCENT, SURFACE);
    text_on_background(d, &shown, TEXT_X, BOX_TOP + BOX_BASELINE, &BODY, FG, SURFACE);
}
