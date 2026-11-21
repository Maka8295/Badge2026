use std::io;

use embedded_graphics::mono_font::ascii::FONT_8X13;
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::library::Text;
use super::pages::{Pages, MAX_LINE_BYTES, MAX_VIEW_LINES};
use crate::keys::{self, Key};
use crate::ui::{self, FOOTER_HEIGHT, HEADER_HEIGHT, SCREEN_HEIGHT, SCREEN_WIDTH};

const FONT: MonoFont = FONT_8X13;
const CONTENT_HEIGHT: i32 = SCREEN_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT;
const SIDE_MARGIN: i32 = 0;
const TOP_PADDING: i32 = 2;
const LINE_HEIGHT: i32 = FONT.character_size.height as i32;
const CHAR_WIDTH: i32 = FONT.character_size.width as i32;
const BASELINE: i32 = FONT.baseline as i32;
const ROWS: usize = ((CONTENT_HEIGHT - TOP_PADDING) / LINE_HEIGHT) as usize;
const COLUMNS: usize = ((SCREEN_WIDTH - 2 * SIDE_MARGIN) / CHAR_WIDTH) as usize;
const PAN_COLUMNS: usize = COLUMNS / 2;
const SCROLL_LINES: usize = 10;
const MAX_LEFT: usize = MAX_LINE_BYTES - COLUMNS;
const HINT: &str = "UP/DOWN LEFT/RIGHT   B back";
const PERCENT: usize = 100;

const _: () = assert!(ROWS <= MAX_VIEW_LINES);

pub struct Reader {
    title: String,
    pages: Pages,
    lines: Vec<String>,
    top: usize,
    left: usize,
}

impl Reader {
    pub fn open(text: &Text, title: String) -> io::Result<Self> {
        let mut pages = Pages::open(text.open()?)?;
        let lines = pages.lines(0, ROWS)?;
        Ok(Self { title, pages, lines, top: 0, left: 0 })
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        ui::header(d, &self.title);
        ui::background(d, 0, HEADER_HEIGHT, SCREEN_WIDTH, CONTENT_HEIGHT);
        self.draw_text(d);
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) -> bool {
        if matches!(k, keys::B | keys::ESC) {
            return true;
        }
        let step = if shift { ROWS } else { SCROLL_LINES };
        let (top, left) = match k {
            keys::UP => (self.top.saturating_sub(step), self.left),
            keys::DOWN => ((self.top + step).min(self.last_top()), self.left),
            keys::LEFT => (self.top, self.left.saturating_sub(PAN_COLUMNS)),
            keys::RIGHT => (self.top, (self.left + PAN_COLUMNS).min(MAX_LEFT)),
            _ => return false,
        };
        self.move_to(top, left, d);
        false
    }

    fn move_to<D: DrawTarget<Color = Rgb565>>(&mut self, top: usize, left: usize, d: &mut D) {
        if (top, left) == (self.top, self.left) {
            return;
        }
        if top != self.top {
            match self.pages.lines(top, ROWS) {
                Ok(lines) => self.lines = lines,
                Err(e) => self.lines = vec![format!("read failed: {e}")],
            }
        }
        self.top = top;
        self.left = left;
        self.draw_text(d);
    }

    fn last_top(&self) -> usize {
        self.pages.line_count().saturating_sub(ROWS)
    }

    fn draw_text<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        for row in 0..ROWS {
            let line = self.lines.get(row).map_or("", String::as_str);
            let shown: String = line.chars().skip(self.left).take(COLUMNS).collect();
            let y = HEADER_HEIGHT + TOP_PADDING + row as i32 * LINE_HEIGHT + BASELINE;
            ui::text_on_background(d, &format!("{shown:<COLUMNS$}"), SIDE_MARGIN, y, &FONT, ui::FG, ui::BG);
        }
        ui::footer(d, &format!("{HINT}   {}%", self.percent()));
    }

    fn percent(&self) -> usize {
        match self.last_top() {
            0 => PERCENT,
            last => self.top * PERCENT / last,
        }
    }
}
