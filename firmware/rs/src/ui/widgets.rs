use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle, Rectangle};
use embedded_graphics::text::Text;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use super::bitmap::draw_bitmap;
use super::images::TENGU_ICON;
use super::font::{text_width, BODY, SMALL, TITLE};
use super::layout::*;
use super::theme::*;

const ICON_TOP: i32 = 10;
const ICON_TEXT_GAP: i32 = 6;
const LOGO_START: &str = "TENGU";
const LOGO_END: &str = "SEC";
const LOGO_BASELINE: i32 = 31;
const SUBTITLE_BASELINE: i32 = 29;
const FOOTER_BASELINE: i32 = SCREEN_HEIGHT - 7;
const SCREEN_FIRST_BASELINE: i32 = 76;
const SCREEN_LINE_HEIGHT: i32 = 24;
pub const BACK_HINT: &str = "B back";
const GRID_LINE_WIDTH: u32 = 1;
const CAPS_TAG: &str = "CAPS";

static CAPS_LOCK: AtomicBool = AtomicBool::new(false);
static FOOTER_HINT: Mutex<String> = Mutex::new(String::new());

pub fn text<D: DrawTarget<Color = Rgb565>>(d: &mut D, s: &str, x: i32, y: i32, f: &MonoFont, c: Rgb565) {
    Text::new(s, Point::new(x, y), MonoTextStyle::new(f, c)).draw(d).ok();
}

pub fn text_on_background<D: DrawTarget<Color = Rgb565>>(d: &mut D, s: &str, x: i32, y: i32, f: &MonoFont, c: Rgb565, bg: Rgb565) {
    let mut style = MonoTextStyle::new(f, c);
    style.background_color = Some(bg);
    Text::new(s, Point::new(x, y), style).draw(d).ok();
}

pub fn fill<D: DrawTarget<Color = Rgb565>>(d: &mut D, x: i32, y: i32, w: u32, h: u32, c: Rgb565) {
    Rectangle::new(Point::new(x, y), Size::new(w, h))
        .into_styled(PrimitiveStyle::with_fill(c))
        .draw(d)
        .ok();
}

pub fn background<D: DrawTarget<Color = Rgb565>>(d: &mut D, x: i32, y: i32, w: i32, h: i32) {
    fill(d, x, y, w as u32, h as u32, BG);
    for grid_x in grid_positions(x, w) {
        grid_line(d, Point::new(grid_x, y), Point::new(grid_x, y + h - 1));
    }
    for grid_y in grid_positions(y, h) {
        grid_line(d, Point::new(x, grid_y), Point::new(x + w - 1, grid_y));
    }
}

fn grid_positions(start: i32, len: i32) -> impl Iterator<Item = i32> {
    (start..start + len).filter(|v| v % GRID_CELL == 0)
}

fn grid_line<D: DrawTarget<Color = Rgb565>>(d: &mut D, from: Point, to: Point) {
    Line::new(from, to)
        .into_styled(PrimitiveStyle::with_stroke(GRID, GRID_LINE_WIDTH))
        .draw(d)
        .ok();
}

fn border_line<D: DrawTarget<Color = Rgb565>>(d: &mut D, y: i32) {
    fill(d, 0, y, SCREEN_WIDTH as u32, BORDER_THICKNESS as u32, BORDER);
}

pub fn header<D: DrawTarget<Color = Rgb565>>(d: &mut D, subtitle: &str) {
    background(d, 0, 0, SCREEN_WIDTH, HEADER_HEIGHT);
    draw_bitmap(d, &TENGU_ICON, Point::new(MARGIN, ICON_TOP), FG);
    let logo_start_x = MARGIN + TENGU_ICON.width as i32 + ICON_TEXT_GAP;
    text(d, LOGO_START, logo_start_x, LOGO_BASELINE, &TITLE, FG);
    let logo_end_x = logo_start_x + text_width(LOGO_START, &TITLE);
    text(d, LOGO_END, logo_end_x, LOGO_BASELINE, &TITLE, ACCENT);
    let subtitle_x = SCREEN_WIDTH - MARGIN - text_width(subtitle, &SMALL);
    text(d, subtitle, subtitle_x, SUBTITLE_BASELINE, &SMALL, MUTED);
    border_line(d, HEADER_HEIGHT - BORDER_THICKNESS);
}

pub fn footer<D: DrawTarget<Color = Rgb565>>(d: &mut D, hint: &str) {
    border_line(d, FOOTER_TOP);
    let inside_height = (FOOTER_HEIGHT - BORDER_THICKNESS) as u32;
    fill(d, 0, FOOTER_TOP + BORDER_THICKNESS, SCREEN_WIDTH as u32, inside_height, BG);
    text(d, hint, MARGIN, FOOTER_BASELINE, &SMALL, MUTED);
    if CAPS_LOCK.load(Ordering::Relaxed) {
        let x = SCREEN_WIDTH - MARGIN - text_width(CAPS_TAG, &SMALL);
        text_on_background(d, CAPS_TAG, x, FOOTER_BASELINE, &SMALL, ACCENT, BG);
    }
    if let Ok(mut last) = FOOTER_HINT.lock() {
        last.clear();
        last.push_str(hint);
    }
}

/// Records caps lock without drawing, for full-screen apps that have no footer.
pub fn remember_caps_lock(on: bool) {
    CAPS_LOCK.store(on, Ordering::Relaxed);
}

/// Shows or hides the CAPS tag by redrawing the footer that is on screen now.
pub fn set_caps_lock<D: DrawTarget<Color = Rgb565>>(d: &mut D, on: bool) {
    CAPS_LOCK.store(on, Ordering::Relaxed);
    let hint = FOOTER_HINT.lock().map(|h| h.clone()).unwrap_or_default();
    footer(d, &hint);
}

pub fn screen<D: DrawTarget<Color = Rgb565>>(d: &mut D, title: &str, lines: &[&str]) {
    header(d, title);
    background(d, 0, HEADER_HEIGHT, SCREEN_WIDTH, CONTENT_HEIGHT);
    for (i, line) in lines.iter().enumerate() {
        let y = SCREEN_FIRST_BASELINE + i as i32 * SCREEN_LINE_HEIGHT;
        text(d, line, MARGIN, y, &BODY, FG);
    }
    footer(d, BACK_HINT);
}
