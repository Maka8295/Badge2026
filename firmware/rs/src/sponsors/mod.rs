mod list;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::keys::{self, Key};
use crate::ui::{self, BODY, FG, MARGIN, SCREEN_WIDTH, TITLE};
use list::{Sponsor, SPONSORS};

const SUBTITLE: &str = "// sponsors";
const HINT: &str = "LEFT/RIGHT browse   B back";
const LOGO_TOP: i32 = 50;
const LOGO_BOX_HEIGHT: i32 = 128;
const NAMED_LOGO_BOX_HEIGHT: i32 = 104;
const NAME_BASELINE: i32 = 174;
const CAPTION_BASELINE: i32 = 206;
const CAPTION_MAX_WIDTH: i32 = SCREEN_WIDTH - 2 * MARGIN;

pub struct Sponsors {
    shown: usize,
}

impl Sponsors {
    pub fn new() -> Self {
        Self { shown: 0 }
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        ui::screen(d, SUBTITLE, &[]);
        draw_sponsor(d, &SPONSORS[self.shown]);
        ui::footer(d, &format!("{HINT}   {}/{}", self.shown + 1, SPONSORS.len()));
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) -> bool {
        match k {
            keys::LEFT | keys::RIGHT => {
                self.shown = step(self.shown, k == keys::RIGHT);
                self.draw(d);
                false
            }
            keys::B | keys::ESC => true,
            _ => false,
        }
    }
}

fn step(shown: usize, forward: bool) -> usize {
    let count = SPONSORS.len();
    if forward { (shown + 1) % count } else { (shown + count - 1) % count }
}

fn draw_sponsor<D: DrawTarget<Color = Rgb565>>(d: &mut D, sponsor: &Sponsor) {
    let box_height = if sponsor.name.is_some() { NAMED_LOGO_BOX_HEIGHT } else { LOGO_BOX_HEIGHT };
    match sponsor.logo {
        Some(logo) => ui::draw_color_image(d, logo, logo_top_left(logo.width, logo.height, box_height)),
        None => ui::draw_bitmap(d, &ui::TENGU_LOGO, logo_top_left(ui::TENGU_LOGO.width, ui::TENGU_LOGO.height, box_height), FG),
    }
    if let Some(name) = sponsor.name {
        ui::text(d, name, centered_x(ui::text_width(name, &BODY)), NAME_BASELINE, &BODY, FG);
    }
    draw_caption(d, sponsor.caption);
}

fn logo_top_left(width: u32, height: u32, box_height: i32) -> Point {
    Point::new(centered_x(width as i32), LOGO_TOP + (box_height - height as i32) / 2)
}

fn draw_caption<D: DrawTarget<Color = Rgb565>>(d: &mut D, caption: &str) {
    let font = if ui::text_width(caption, &TITLE) <= CAPTION_MAX_WIDTH { &TITLE } else { &BODY };
    ui::text(d, caption, centered_x(ui::text_width(caption, font)), CAPTION_BASELINE, font, FG);
}

fn centered_x(width: i32) -> i32 {
    (SCREEN_WIDTH - width) / 2
}

