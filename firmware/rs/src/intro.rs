use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::hal::delay::{Ets, FreeRtos};

use crate::keys::Keys;
use crate::ui::{self, ACCENT, BG, BODY, FG, MUTED, SCREEN_HEIGHT, SCREEN_WIDTH, TENGU_LOGO, TITLE};

const LOGO_TOP: i32 = 18;
const REVEAL_ROWS_PER_STEP: u32 = 4;
const REVEAL_STEP_MS: u32 = 20;
const SCAN_LINE_HEIGHT: u32 = 2;

const GLITCH_OFFSETS: [i32; 3] = [6, -4, 3];
const GLITCH_CLEAR_MARGIN: i32 = 8;
const GLITCH_MS: u32 = 7;

const TITLE_START: &str = "TENGUCON";
const TITLE_END: &str = " 3.0";
const TITLE_BASELINE: i32 = 184;

const PAUSE_BEFORE_TAGLINE_MS: u32 = 250;
const TAGLINE: &str = "tengu never dies";
const TAGLINE_BASELINE: i32 = 214;
const TYPE_MS: u32 = 55;
const CURSOR: &str = "_";
const CURSOR_OFF: &str = " ";
const CURSOR_BLINKS: u32 = 3;
const CURSOR_BLINK_MS: u32 = 220;

const KEY_POLL_MS: u32 = 10;
const MICROS_PER_MILLI: u32 = 1000;

struct Skipped;
type Step = Result<(), Skipped>;

pub fn play<D: DrawTarget<Color = Rgb565>>(d: &mut D, keys: &mut Keys) {
    let _ = run(d, keys);
}

fn run<D: DrawTarget<Color = Rgb565>>(d: &mut D, keys: &mut Keys) -> Step {
    ui::fill(d, 0, 0, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, BG);
    reveal_logo(d, keys)?;
    glitch_logo(d, keys)?;
    draw_title(d);
    wait(keys, PAUSE_BEFORE_TAGLINE_MS)?;
    type_tagline(d, keys)?;
    blink_cursor(d, keys)
}

fn wait(keys: &mut Keys, ms: u32) -> Step {
    for _ in 0..ms.div_ceil(KEY_POLL_MS) {
        if !keys.pressed().is_empty() {
            return Err(Skipped);
        }
        FreeRtos::delay_ms(KEY_POLL_MS);
    }
    Ok(())
}

fn wait_shorter_than_a_tick(keys: &mut Keys, ms: u32) -> Step {
    if !keys.pressed().is_empty() {
        return Err(Skipped);
    }
    Ets::delay_us(ms * MICROS_PER_MILLI);
    Ok(())
}

fn logo_origin() -> Point {
    Point::new((SCREEN_WIDTH - TENGU_LOGO.width as i32) / 2, LOGO_TOP)
}

fn reveal_logo<D: DrawTarget<Color = Rgb565>>(d: &mut D, keys: &mut Keys) -> Step {
    let origin = logo_origin();
    for start in (0..TENGU_LOGO.height).step_by(REVEAL_ROWS_PER_STEP as usize) {
        let end = (start + REVEAL_ROWS_PER_STEP).min(TENGU_LOGO.height);
        scan_line(d, origin, start, BG);
        ui::draw_bitmap_rows(d, &TENGU_LOGO, origin, FG, start..end);
        scan_line(d, origin, end, ACCENT);
        wait(keys, REVEAL_STEP_MS)?;
    }
    scan_line(d, origin, TENGU_LOGO.height, BG);
    Ok(())
}

fn scan_line<D: DrawTarget<Color = Rgb565>>(d: &mut D, origin: Point, row: u32, color: Rgb565) {
    ui::fill(d, origin.x, origin.y + row as i32, TENGU_LOGO.width, SCAN_LINE_HEIGHT, color);
}

fn glitch_logo<D: DrawTarget<Color = Rgb565>>(d: &mut D, keys: &mut Keys) -> Step {
    for offset in GLITCH_OFFSETS {
        redraw_logo(d, offset, ACCENT);
        wait_shorter_than_a_tick(keys, GLITCH_MS)?;
        redraw_logo(d, 0, FG);
        wait_shorter_than_a_tick(keys, GLITCH_MS)?;
    }
    Ok(())
}

fn redraw_logo<D: DrawTarget<Color = Rgb565>>(d: &mut D, offset: i32, color: Rgb565) {
    let origin = logo_origin();
    let clear_width = TENGU_LOGO.width + 2 * GLITCH_CLEAR_MARGIN as u32;
    ui::fill(d, origin.x - GLITCH_CLEAR_MARGIN, origin.y, clear_width, TENGU_LOGO.height, BG);
    ui::draw_bitmap(d, &TENGU_LOGO, origin + Point::new(offset, 0), color);
}

fn draw_title<D: DrawTarget<Color = Rgb565>>(d: &mut D) {
    let start_width = ui::text_width(TITLE_START, &TITLE);
    let x = centered_x(start_width + ui::text_width(TITLE_END, &TITLE));
    ui::text(d, TITLE_START, x, TITLE_BASELINE, &TITLE, FG);
    ui::text(d, TITLE_END, x + start_width, TITLE_BASELINE, &TITLE, ACCENT);
}

fn tagline_x() -> i32 {
    centered_x(ui::text_width(TAGLINE, &BODY))
}

fn char_x(index: usize) -> i32 {
    tagline_x() + index as i32 * BODY.character_size.width as i32
}

fn type_tagline<D: DrawTarget<Color = Rgb565>>(d: &mut D, keys: &mut Keys) -> Step {
    for (index, letter) in TAGLINE.char_indices() {
        let text = &TAGLINE[index..index + letter.len_utf8()];
        ui::text_on_background(d, text, char_x(index), TAGLINE_BASELINE, &BODY, MUTED, BG);
        draw_cursor(d, index + letter.len_utf8(), CURSOR);
        wait(keys, TYPE_MS)?;
    }
    Ok(())
}

fn draw_cursor<D: DrawTarget<Color = Rgb565>>(d: &mut D, index: usize, cursor: &str) {
    ui::text_on_background(d, cursor, char_x(index), TAGLINE_BASELINE, &BODY, ACCENT, BG);
}

fn blink_cursor<D: DrawTarget<Color = Rgb565>>(d: &mut D, keys: &mut Keys) -> Step {
    for _ in 0..CURSOR_BLINKS {
        draw_cursor(d, TAGLINE.len(), CURSOR_OFF);
        wait(keys, CURSOR_BLINK_MS)?;
        draw_cursor(d, TAGLINE.len(), CURSOR);
        wait(keys, CURSOR_BLINK_MS)?;
    }
    Ok(())
}

fn centered_x(width: i32) -> i32 {
    (SCREEN_WIDTH - width) / 2
}
