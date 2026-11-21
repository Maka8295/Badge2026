use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use crate::keys::{self, Key};
use crate::leds::effects::{DEFAULT_SPEED, SPEEDS, THEMES};
use crate::leds::palettes::CHOICES;
use crate::leds::power::{self, BUDGET_MA, WARN_LEVEL};
use crate::leds::{self, DEFAULT_LEVEL, FULL};
use crate::ui::{self, ACCENT, MARGIN, MUTED, SMALL};

const SUBTITLE: &str = "// leds";
const HINT: &str = "UP/DOWN row   L/R change   B back";
const KEY_LEVEL: &str = "led_level";
const KEY_THEME: &str = "led_theme";
const KEY_COLOUR: &str = "led_colour";
const KEY_SPEED: &str = "led_speed";
const STEP: u8 = 15;
const FINE_STEP: u8 = 1;
const BAR_CELLS: usize = 36;
const THEME_COLOURS: &str = "as per style";
// Below the four menu rows (ui::menu rows are 26 px from y = 52).
const INFO_BASELINE: i32 = 52 + 4 * 26 + 18;
const INFO_LINE: i32 = 17;

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Theme,
    Colour,
    Speed,
    Brightness,
}

const ROWS: [Row; 4] = [Row::Theme, Row::Colour, Row::Speed, Row::Brightness];

pub struct LedApp {
    level: u8,
    theme: usize,
    // 0 = the theme's own palette, n = palettes::CHOICES[n - 1].
    colour: usize,
    speed: usize,
    row: usize,
}

impl LedApp {
    pub fn new(nvs: &EspNvs<NvsDefault>) -> Self {
        let load = |key| nvs.get_u8(key).ok().flatten();
        let app = Self {
            level: load(KEY_LEVEL).unwrap_or(DEFAULT_LEVEL),
            theme: load(KEY_THEME).map_or(0, usize::from).min(THEMES.len() - 1),
            colour: load(KEY_COLOUR).map_or(0, usize::from).min(CHOICES.len()),
            speed: load(KEY_SPEED).map_or(DEFAULT_SPEED, usize::from).min(SPEEDS.len() - 1),
            row: 0,
        };
        leds::set_level(app.level);
        leds::set_theme(app.theme);
        leds::set_palette(app.colour.checked_sub(1));
        leds::set_speed(app.speed);
        app
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        let colour = match self.colour {
            0 => THEME_COLOURS,
            n => CHOICES[n - 1].name,
        };
        let items = [
            format!("Blink style {}", THEMES[self.theme].name),
            format!("Colour      {colour}"),
            format!("Speed       {}", SPEEDS[self.speed].name),
            format!("Brightness  {}/{FULL}", self.level),
        ];
        ui::menu(d, SUBTITLE, &ui::as_strs(&items), self.row, HINT);
        self.draw_power(d);
    }

    fn draw_power<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        let filled = self.level as usize * BAR_CELLS / FULL as usize;
        let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(BAR_CELLS - filled));
        let high = self.level > WARN_LEVEL;
        let worst = power::worst_case_ma(self.level);
        let draw = if worst > BUDGET_MA {
            format!("up to {worst:.0} mA, capped at {BUDGET_MA:.0} mA")
        } else {
            format!("up to {worst:.0} mA")
        };
        ui::text(d, &bar, MARGIN, INFO_BASELINE, &SMALL, if high { ACCENT } else { MUTED });
        ui::text(d, &format!("LED current {draw}"), MARGIN, INFO_BASELINE + INFO_LINE, &SMALL, MUTED);
        if high {
            let warning = "! HIGH: drains battery fast, may brown out";
            ui::text(d, warning, MARGIN, INFO_BASELINE + 2 * INFO_LINE, &SMALL, ACCENT);
        }
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D, nvs: &EspNvs<NvsDefault>) -> bool {
        match k {
            keys::UP | keys::DOWN => self.row = keys::wrap_selection(k, self.row, ROWS.len()).unwrap(),
            keys::LEFT | keys::RIGHT => self.change(ROWS[self.row], k == keys::RIGHT, shift, nvs),
            keys::B | keys::ESC => return true,
            _ => return false,
        }
        self.draw(d);
        false
    }

    fn change(&mut self, row: Row, up: bool, fine: bool, nvs: &EspNvs<NvsDefault>) {
        match row {
            Row::Theme => {
                self.theme = cycle(self.theme, THEMES.len(), up);
                leds::set_theme(self.theme);
                save(nvs, KEY_THEME, self.theme as u8);
            }
            Row::Colour => {
                self.colour = cycle(self.colour, CHOICES.len() + 1, up);
                leds::set_palette(self.colour.checked_sub(1));
                save(nvs, KEY_COLOUR, self.colour as u8);
            }
            Row::Speed => {
                self.speed = cycle(self.speed, SPEEDS.len(), up);
                leds::set_speed(self.speed);
                save(nvs, KEY_SPEED, self.speed as u8);
            }
            Row::Brightness => {
                let step = if fine { FINE_STEP } else { STEP };
                self.level = if up { self.level.saturating_add(step) } else { self.level.saturating_sub(step) };
                leds::set_level(self.level);
                save(nvs, KEY_LEVEL, self.level);
            }
        }
    }
}

fn cycle(value: usize, len: usize, up: bool) -> usize {
    if up { (value + 1) % len } else { (value + len - 1) % len }
}

fn save(nvs: &EspNvs<NvsDefault>, key: &str, value: u8) {
    if let Err(e) = nvs.set_u8(key, value) {
        log::error!("ledapp: saving {key}: {e}");
    }
}
