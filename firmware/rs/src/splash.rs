// Sir Jaffalot full screen, the About page easter egg.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::ui::{self, ACCENT, BG, FG, MUTED, SCREEN_HEIGHT, SCREEN_WIDTH, SMALL};

const JAFFALOT_TITLE_TOP: i32 = 6;
const JAFFALOT_ART_TOP: i32 = 48;
// In the empty corner left of the snail, which starts at x = 98 (SMALL is 7 px wide, so 13 characters fit).
const JAFFALOT_NOTE: [&str; 4] = ["this was hard", "to make, i", "hope you like", "it :)"];
const JAFFALOT_NOTE_X: i32 = 6;
const JAFFALOT_NOTE_FIRST_BASELINE: i32 = 159;
const JAFFALOT_NOTE_LINE: i32 = 17;

pub fn jaffalot_page<D: DrawTarget<Color = Rgb565>>(d: &mut D, hint: &str) {
    ui::fill(d, 0, 0, SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, BG);
    jaffalot(d);
    ui::footer(d, hint);
}

fn centered(width: i32) -> i32 {
    (SCREEN_WIDTH - width) / 2
}

fn jaffalot<D: DrawTarget<Color = Rgb565>>(d: &mut D) {
    let title = &ui::JAFFALOT_TITLE;
    ui::draw_bitmap(d, title, Point::new(centered(title.width as i32), JAFFALOT_TITLE_TOP), ACCENT);
    let art = &ui::JAFFALOT;
    ui::draw_bitmap(d, art, Point::new(centered(art.width as i32), JAFFALOT_ART_TOP), FG);
    for (i, line) in JAFFALOT_NOTE.iter().enumerate() {
        let y = JAFFALOT_NOTE_FIRST_BASELINE + i as i32 * JAFFALOT_NOTE_LINE;
        ui::text(d, line, JAFFALOT_NOTE_X, y, &SMALL, MUTED);
    }
}
