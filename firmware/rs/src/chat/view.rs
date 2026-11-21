use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::emote::{Emote, Picture};
use super::settings::{Config, FIELDS};
use super::Msg;
use crate::ui::{
    self, ACCENT, BG, BODY, BORDER, BORDER_THICKNESS, CONTENT_HEIGHT, FG, FOOTER_HEIGHT, HEADER_HEIGHT, MARGIN, MUTED,
    SCREEN_HEIGHT, SCREEN_WIDTH, SMALL, SURFACE,
};

const CONTENT_W: i32 = SCREEN_WIDTH - 2 * MARGIN;
const SMALL_CHAR_W: i32 = SMALL.character_size.width as i32;
const BODY_CHAR_W: i32 = BODY.character_size.width as i32;

const MSG_TOP: i32 = 46;
const MSG_LINE_H: i32 = 18;
const MSG_BASELINE: i32 = 13;
pub const MSG_LINES: usize = 8;
const MSG_COLS: usize = (CONTENT_W / SMALL_CHAR_W) as usize;

const INPUT_TOP: i32 = 192;
const INPUT_H: i32 = 22;
const INPUT_BASELINE: i32 = 17;
const INPUT_PROMPT_X: i32 = 18;
const INPUT_TEXT_X: i32 = 34;
const INPUT_PROMPT_SPACE: i32 = 30;
const INPUT_VISIBLE_CHARS: usize = ((CONTENT_W - INPUT_PROMPT_SPACE) / BODY_CHAR_W) as usize;
const INPUT_TEXT_CHARS: usize = INPUT_VISIBLE_CHARS - CURSOR.len();
const CURSOR: &str = "_";
const PASSWORD_MASK: &str = "*";
const PASSWORD_HINT: &str = "OK/A confirm   B cancel";

const FIELD_TOP: i32 = 50;
const FIELD_H: i32 = 23;
const FIELD_BASELINE: i32 = 16;
const FIELD_HIGHLIGHT_INSET: i32 = 1;
const FIELD_ACCENT_BAR_W: u32 = 3;
const FIELD_LABEL_X: i32 = 24;
const FIELD_VALUE_RIGHT_MARGIN: i32 = 20;
const FIELDS_BOTTOM: i32 = FIELD_TOP + FIELDS.len() as i32 * FIELD_H;

#[derive(Clone, Default)]
struct Line {
    nick: String,
    text: String,
    color: Rgb565,
    // An emote spans several lines; each line draws its own band of the picture, so it scrolls off
    // the top one line at a time like text. (emote, band index, x where the picture starts)
    emote: Option<(Emote, u32, i32)>,
}

// `scroll` is how many lines up from the newest the view is; 0 shows the newest messages.
pub fn chat_screen<D: DrawTarget<Color = Rgb565>>(d: &mut D, msgs: &[Msg], scroll: usize, input_text: &str, status_text: &str) {
    ui::header(d, "// lora chat");
    // Plain fill, not ui::background: message rows repaint their own cells, which would cut the grid into dashes.
    ui::fill(d, 0, HEADER_HEIGHT, SCREEN_WIDTH as u32, CONTENT_HEIGHT as u32, BG);
    messages(d, msgs, scroll);
    input(d, input_text);
    status(d, status_text, scroll);
}

pub fn messages<D: DrawTarget<Color = Rgb565>>(d: &mut D, msgs: &[Msg], scroll: usize) {
    for (slot, line) in visible(msgs, scroll).iter().enumerate() {
        message_line(d, slot, line);
    }
}

// Redraws only the picture bands of animated emotes on screen.
pub fn animate<D: DrawTarget<Color = Rgb565>>(d: &mut D, msgs: &[Msg], scroll: usize) {
    for (slot, line) in visible(msgs, scroll).iter().enumerate() {
        if let Some((emote, band, x)) = line.emote {
            if let Picture::Animated(animation) = emote.picture() {
                draw_band(d, animation, band, x, slot_top(slot));
            }
        }
    }
}

// Furthest the view can scroll up.
pub fn max_scroll(msgs: &[Msg]) -> usize {
    wrap(msgs).len().saturating_sub(MSG_LINES)
}

pub fn line_count(m: &Msg) -> usize {
    let mut lines = Vec::new();
    wrap_message(m, &mut lines);
    lines.len()
}

fn visible(msgs: &[Msg], scroll: usize) -> Vec<Line> {
    let mut lines = wrap(msgs);
    let end = lines.len().saturating_sub(scroll);
    let first = end.saturating_sub(MSG_LINES);
    lines.truncate(end);
    lines.drain(..first);
    lines.resize(MSG_LINES, Line::default());
    lines
}

fn slot_top(slot: usize) -> i32 {
    MSG_TOP + slot as i32 * MSG_LINE_H
}

fn band_rows(band: u32, height: u32) -> std::ops::Range<u32> {
    let first = band * MSG_LINE_H as u32;
    first..(first + MSG_LINE_H as u32).min(height)
}

fn draw_band<D: DrawTarget<Color = Rgb565>>(d: &mut D, animation: &ui::Animation, band: u32, x: i32, top: i32) {
    let rows = band_rows(band, animation.height);
    let origin = Point::new(x, top - rows.start as i32);
    ui::draw_animation_rows(d, animation, animation.frame_now(), origin, rows);
}

fn wrap(msgs: &[Msg]) -> Vec<Line> {
    let mut lines = Vec::new();
    for m in msgs {
        wrap_message(m, &mut lines);
    }
    lines
}

fn wrap_message(m: &Msg, lines: &mut Vec<Line>) {
    let mut nick = format!("{}: ", m.nick);
    let color = if m.mine { ACCENT } else { FG };
    if let Some(emote) = Emote::from_text(&m.text) {
        let x = MARGIN + SMALL_CHAR_W * nick.len() as i32;
        let bands = emote.height().div_ceil(MSG_LINE_H as u32);
        for band in 0..bands {
            let nick = if band == 0 { std::mem::take(&mut nick) } else { String::new() };
            lines.push(Line { nick, text: String::new(), color, emote: Some((emote, band, x)) });
        }
        return;
    }
    let chars: Vec<char> = m.text.chars().collect();
    let mut rest = &chars[..];
    loop {
        let room = MSG_COLS.saturating_sub(nick.len()).max(1);
        let (now, later) = rest.split_at(room.min(rest.len()));
        lines.push(Line { nick: std::mem::take(&mut nick), text: now.iter().collect(), color, emote: None });
        rest = later;
        if rest.is_empty() {
            return;
        }
    }
}

fn message_line<D: DrawTarget<Color = Rgb565>>(d: &mut D, slot: usize, line: &Line) {
    let top = slot_top(slot);
    let baseline = top + MSG_BASELINE;
    if let Some((emote, band, x)) = line.emote {
        ui::fill(d, MARGIN, top, CONTENT_W as u32, MSG_LINE_H as u32, BG);
        ui::text(d, &line.nick, MARGIN, baseline, &SMALL, line.color);
        match emote.picture() {
            Picture::Mono(bitmap) => {
                let rows = band_rows(band, bitmap.height);
                ui::draw_bitmap_rows(d, bitmap, Point::new(x, top - rows.start as i32), FG, rows);
            }
            Picture::Animated(animation) => draw_band(d, animation, band, x, top),
        }
        return;
    }
    // Text cells are one row shorter than the line; clear that row too, or a scrolled emote leaves a trace.
    ui::fill(d, MARGIN, top, CONTENT_W as u32, 1, BG);
    let text_x = MARGIN + SMALL_CHAR_W * line.nick.len() as i32;
    let padded = format!("{:<w$}", line.text, w = MSG_COLS.saturating_sub(line.nick.len()));
    ui::text_on_background(d, &line.nick, MARGIN, baseline, &SMALL, line.color, BG);
    ui::text_on_background(d, &padded, text_x, baseline, &SMALL, FG, BG);
}

pub fn input<D: DrawTarget<Color = Rgb565>>(d: &mut D, text: &str) {
    let inner = BORDER_THICKNESS;
    ui::fill(d, MARGIN, INPUT_TOP, CONTENT_W as u32, INPUT_H as u32, BORDER);
    ui::fill(d, MARGIN + inner, INPUT_TOP + inner, (CONTENT_W - 2 * inner) as u32, (INPUT_H - 2 * inner) as u32, SURFACE);
    let hidden = text.chars().count().saturating_sub(INPUT_TEXT_CHARS);
    let tail: String = text.chars().skip(hidden).collect();
    let shown = format!("{:<w$}", tail + CURSOR, w = INPUT_VISIBLE_CHARS);
    ui::text_on_background(d, ">", INPUT_PROMPT_X, INPUT_TOP + INPUT_BASELINE, &BODY, ACCENT, SURFACE);
    ui::text_on_background(d, &shown, INPUT_TEXT_X, INPUT_TOP + INPUT_BASELINE, &BODY, FG, SURFACE);
}

pub fn password_screen<D: DrawTarget<Color = Rgb565>>(d: &mut D, title: &str, lines: &[&str], typed: &str) {
    ui::text_page(d, title, lines, PASSWORD_HINT);
    password(d, typed);
}

pub fn password<D: DrawTarget<Color = Rgb565>>(d: &mut D, typed: &str) {
    input(d, &PASSWORD_MASK.repeat(typed.chars().count()));
}

pub fn status<D: DrawTarget<Color = Rgb565>>(d: &mut D, text: &str, scroll: usize) {
    if scroll > 0 {
        ui::footer(d, &format!("{scroll} newer lines below, DOWN to read"));
    } else {
        ui::footer(d, &format!("{text}  tab=config"));
    }
}

pub fn settings<D: DrawTarget<Color = Rgb565>>(d: &mut D, config: &Config, selected: usize) {
    ui::header(d, "// chat settings");
    ui::background(d, 0, HEADER_HEIGHT, SCREEN_WIDTH, FIELD_TOP - HEADER_HEIGHT);
    for i in 0..FIELDS.len() {
        field(d, config, i, i == selected);
    }
    ui::background(d, 0, FIELDS_BOTTOM, SCREEN_WIDTH, SCREEN_HEIGHT - FOOTER_HEIGHT - FIELDS_BOTTOM);
    ui::footer(d, "up/dn field  l/r change  B save");
}

pub fn field<D: DrawTarget<Color = Rgb565>>(d: &mut D, config: &Config, index: usize, selected: bool) {
    let top = FIELD_TOP + index as i32 * FIELD_H;
    let baseline = top + FIELD_BASELINE;
    ui::background(d, 0, top, SCREEN_WIDTH, FIELD_H);
    if selected {
        highlight_field(d, top);
    }
    let field = FIELDS[index];
    let value = field_value_text(config.value(field), selected);
    let value_x = SCREEN_WIDTH - FIELD_VALUE_RIGHT_MARGIN - SMALL_CHAR_W * value.len() as i32;
    ui::text(d, field.label(), FIELD_LABEL_X, baseline, &SMALL, if selected { FG } else { MUTED });
    ui::text(d, &value, value_x, baseline, &SMALL, if selected { ACCENT } else { FG });
}

fn highlight_field<D: DrawTarget<Color = Rgb565>>(d: &mut D, top: i32) {
    let y = top + FIELD_HIGHLIGHT_INSET;
    let h = (FIELD_H - 2 * FIELD_HIGHLIGHT_INSET) as u32;
    ui::fill(d, MARGIN, y, CONTENT_W as u32, h, SURFACE);
    ui::fill(d, MARGIN, y, FIELD_ACCENT_BAR_W, h, ACCENT);
}

fn field_value_text(value: String, selected: bool) -> String {
    if selected {
        format!("< {value} >")
    } else {
        value
    }
}
