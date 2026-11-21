// Full-screen digital rain, cmatrix style. No header or footer: the rain covers the whole screen.
// Each column has a falling head (pale), a bright neck, a green body and a dark tail. Only the cells
// whose colour changes are redrawn each step, so it stays fast on the SPI display.

use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::sys::esp_random;

use crate::keys::{self, Key};
use crate::ui::{self, SCREEN_HEIGHT, SCREEN_WIDTH, SMALL};

const CELL_W: i32 = SMALL.character_size.width as i32;
const CELL_H: i32 = SMALL.character_size.height as i32;
const COLS: usize = (SCREEN_WIDTH / CELL_W) as usize;
const ROWS: i32 = SCREEN_HEIGHT / CELL_H;
// Centre the grid; the few spare pixels at the edges stay black.
const LEFT: i32 = (SCREEN_WIDTH - COLS as i32 * CELL_W) / 2;
const TOP: i32 = (SCREEN_HEIGHT - ROWS * CELL_H) / 2;
const STEP: Duration = Duration::from_millis(45);
// A column moves every 1-3 steps, so heads fall at about 7-22 rows a second.
const SLOWEST: u8 = 3;
const TRAIL_MIN: i32 = 6;
const TRAIL_MAX: i32 = ROWS + 6;
const NECK: i32 = 2;
const TAIL: i32 = 3;
const GLITCHES_PER_STEP: usize = 6;
// Printable ASCII without space (the font has no katakana).
const GLYPHS: std::ops::RangeInclusive<u8> = b'!'..=b'~';

const BLACK: Rgb565 = Rgb565::BLACK;
const HEAD: Rgb565 = Rgb565::new(22, 63, 22);
const NECK_GREEN: Rgb565 = Rgb565::new(0, 63, 6);
const BODY_GREEN: Rgb565 = Rgb565::new(0, 44, 3);
const TAIL_GREEN: Rgb565 = Rgb565::new(0, 20, 1);

struct Column {
    // Row of the head; negative while waiting to start.
    head: i32,
    trail: i32,
    every: u8,
    wait: u8,
}

pub struct Matrix {
    columns: Vec<Column>,
    glyphs: Vec<u8>,
    next_step: Instant,
}

impl Matrix {
    pub fn new() -> Self {
        Self { columns: Vec::new(), glyphs: vec![b' '; COLS * ROWS as usize], next_step: Instant::now() }
    }

    pub fn open<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        ui::fill(d, 0, 0, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, BLACK);
        self.columns = (0..COLS).map(|_| new_column(ROWS)).collect();
        self.glyphs.iter_mut().for_each(|g| *g = random_glyph());
        self.next_step = Instant::now();
    }

    // True when the app should close.
    pub fn key(&mut self, k: Key) -> bool {
        matches!(k, keys::B | keys::ESC)
    }

    pub fn tick<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        let now = Instant::now();
        if now < self.next_step {
            return;
        }
        self.next_step = now + STEP;
        for col in 0..COLS {
            self.step_column(d, col);
        }
        for _ in 0..GLITCHES_PER_STEP {
            self.glitch(d);
        }
    }

    fn step_column<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, col: usize) {
        let column = &mut self.columns[col];
        if column.wait > 1 {
            column.wait -= 1;
            return;
        }
        column.wait = column.every;
        column.head += 1;
        if column.head - column.trail >= ROWS {
            self.columns[col] = new_column(ROWS);
            return;
        }
        let (head, trail) = (column.head, column.trail);
        // A fresh glyph for the new head, then redraw the cells whose colour band changed.
        if (0..ROWS).contains(&head) {
            self.glyphs[index(col, head)] = random_glyph();
        }
        for distance in [0, 1, NECK + 1, trail - TAIL, trail] {
            self.draw_cell(d, col, head - distance, distance, trail);
        }
    }

    fn glitch<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        let col = random_below(COLS as u32) as usize;
        let Column { head, trail, .. } = self.columns[col];
        let distance = 1 + random_below(trail.max(2) as u32 - 1) as i32;
        let row = head - distance;
        if (0..ROWS).contains(&row) && distance < trail {
            self.glyphs[index(col, row)] = random_glyph();
            self.draw_cell(d, col, row, distance, trail);
        }
    }

    fn draw_cell<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, col: usize, row: i32, distance: i32, trail: i32) {
        if !(0..ROWS).contains(&row) || distance < 0 {
            return;
        }
        let x = LEFT + col as i32 * CELL_W;
        let y = TOP + row * CELL_H;
        let color = match distance {
            0 => HEAD,
            n if n <= NECK => NECK_GREEN,
            n if n >= trail => return ui::fill(d, x, y, CELL_W as u32, CELL_H as u32, BLACK),
            n if n >= trail - TAIL => TAIL_GREEN,
            _ => BODY_GREEN,
        };
        let glyph = [self.glyphs[index(col, row)]];
        let glyph = std::str::from_utf8(&glyph).unwrap_or("0");
        ui::text_on_background(d, glyph, x, y + SMALL.baseline as i32, &SMALL, color, BLACK);
    }
}

fn index(col: usize, row: i32) -> usize {
    row as usize * COLS + col
}

fn new_column(rows: i32) -> Column {
    let every = 1 + random_below(SLOWEST as u32) as u8;
    Column {
        // Start above the screen by a random amount, so the columns don't fall in step.
        head: -1 - random_below(rows as u32 * 2) as i32,
        trail: TRAIL_MIN + random_below((TRAIL_MAX - TRAIL_MIN + 1) as u32) as i32,
        every,
        wait: every,
    }
}

fn random_glyph() -> u8 {
    GLYPHS.start() + random_below((GLYPHS.end() - GLYPHS.start() + 1) as u32) as u8
}

fn random_below(limit: u32) -> u32 {
    // SAFETY: esp_random has no preconditions.
    unsafe { esp_random() % limit.max(1) }
}
