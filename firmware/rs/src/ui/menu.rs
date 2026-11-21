use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::font::BODY;
use super::layout::*;
use super::theme::*;
use super::widgets::{background, fill, footer, header, text};

const ROWS_PER_PAGE: usize = 6;
const LIST_TOP: i32 = 52;
const ROW_HEIGHT: i32 = 26;
const LIST_BOTTOM: i32 = LIST_TOP + ROWS_PER_PAGE as i32 * ROW_HEIGHT;
const HIGHLIGHT_INSET: i32 = 1;
const HIGHLIGHT_HEIGHT: u32 = (ROW_HEIGHT - 2 * HIGHLIGHT_INSET) as u32;
const HIGHLIGHT_WIDTH: u32 = (SCREEN_WIDTH - 2 * MARGIN) as u32;
const SELECTION_BAR_WIDTH: u32 = 3;
const CURSOR: &str = ">";
const CURSOR_X: i32 = 22;
const LABEL_X: i32 = 40;
const LABEL_BASELINE: i32 = 19;
const PAGE_GAP: &str = "   ";

fn row_top(slot: usize) -> i32 {
    LIST_TOP + slot as i32 * ROW_HEIGHT
}

fn page_of(index: usize) -> usize {
    index / ROWS_PER_PAGE
}

fn row<D: DrawTarget<Color = Rgb565>>(d: &mut D, slot: usize, label: &str, selected: bool) {
    let y = row_top(slot);
    background(d, 0, y, SCREEN_WIDTH, ROW_HEIGHT);
    if selected {
        selected_row(d, y, label);
    } else {
        text(d, label, LABEL_X, y + LABEL_BASELINE, &BODY, MUTED);
    }
}

fn selected_row<D: DrawTarget<Color = Rgb565>>(d: &mut D, y: i32, label: &str) {
    let highlight_y = y + HIGHLIGHT_INSET;
    fill(d, MARGIN, highlight_y, HIGHLIGHT_WIDTH, HIGHLIGHT_HEIGHT, SURFACE);
    fill(d, MARGIN, highlight_y, SELECTION_BAR_WIDTH, HIGHLIGHT_HEIGHT, ACCENT);
    text(d, CURSOR, CURSOR_X, y + LABEL_BASELINE, &BODY, ACCENT);
    text(d, label, LABEL_X, y + LABEL_BASELINE, &BODY, FG);
}

fn rows<D: DrawTarget<Color = Rgb565>>(d: &mut D, items: &[&str], sel: usize) {
    let start = page_of(sel) * ROWS_PER_PAGE;
    for slot in 0..ROWS_PER_PAGE {
        match items.get(start + slot) {
            Some(label) => row(d, slot, label, start + slot == sel),
            None => background(d, 0, row_top(slot), SCREEN_WIDTH, ROW_HEIGHT),
        }
    }
}

fn menu_footer<D: DrawTarget<Color = Rgb565>>(d: &mut D, hint: &str, sel: usize, len: usize) {
    if len > ROWS_PER_PAGE {
        let pages = len.div_ceil(ROWS_PER_PAGE);
        footer(d, &format!("{hint}{PAGE_GAP}{}/{pages}", page_of(sel) + 1));
    } else {
        footer(d, hint);
    }
}

pub fn menu<D: DrawTarget<Color = Rgb565>>(d: &mut D, subtitle: &str, items: &[&str], sel: usize, hint: &str) {
    header(d, subtitle);
    background(d, 0, HEADER_HEIGHT, SCREEN_WIDTH, LIST_TOP - HEADER_HEIGHT);
    rows(d, items, sel);
    background(d, 0, LIST_BOTTOM, SCREEN_WIDTH, FOOTER_TOP - LIST_BOTTOM);
    menu_footer(d, hint, sel, items.len());
}

pub fn menu_select<D: DrawTarget<Color = Rgb565>>(d: &mut D, items: &[&str], old: usize, new: usize, hint: &str) {
    if page_of(old) == page_of(new) {
        row(d, old % ROWS_PER_PAGE, items[old], false);
        row(d, new % ROWS_PER_PAGE, items[new], true);
    } else {
        rows(d, items, new);
    }
    menu_footer(d, hint, new, items.len());
}

pub fn as_strs(items: &[String]) -> Vec<&str> {
    items.iter().map(String::as_str).collect()
}
