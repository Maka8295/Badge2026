// Nametag: a name badge on screen (name, company, where you are from) and its editor.
// Two names, edited separately:
// - Name: shown big on the nametag. Empty means "use the LoRa name".
// - LoRa name: chat nick, main menu title, and badge_name() for ELF programs. Kept under the old
//   "username" key so names set before the Nametag app carry over. Empty means the board ID.
// The scavenger hunt and badge swap keep using the fixed board ID from board_name.rs: the hunt derives
// codes and NVS keys from it, and swap counts unique badges by it, so renaming must not change them.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use crate::chat::NICK_MAX;
use crate::keys::{self, Key};
use crate::ui::{self, ACCENT, BODY, FG, HEADER_HEIGHT, MARGIN, MUTED, SCREEN_WIDTH, SMALL, TITLE};

const SUBTITLE: &str = "// nametag";
const EDIT_SUBTITLE: &str = "// edit nametag";
const VIEW_HINT: &str = "A edit   B back";
const EDIT_HINT: &str = "OK save   DEL erase   B cancel";
const GREETING: &str = "HELLO, MY NAME IS";
const EMPTY_HINT: &str = "A: add your company and where you're from";
const FROM_PREFIX: &str = "from ";

// BODY is 10 px wide: 28 characters plus "from " still fit inside the margins.
const DETAIL_MAX: usize = 28;
// TITLE is 14 px wide: 21 characters fit inside the margins.
const NAME_MAX: usize = 20;

// View layout, baselines inside the content area (y 44 to 216).
const GREETING_BASELINE: i32 = 72;
const NAME_BASELINE: i32 = 114;
const RULE_TOP: i32 = 126;
const RULE_HEIGHT: u32 = 2;
const RULE_MIN_WIDTH: i32 = 120;
const COMPANY_BASELINE: i32 = 160;
const FROM_BASELINE: i32 = 188;

// Edit layout: rows of the SMALL text page (see ui::text_page).
const ROW_TOP: i32 = HEADER_HEIGHT + 2;
const ROW_HEIGHT: i32 = SMALL.character_size.height as i32;
const ROW_BASELINE: i32 = SMALL.baseline as i32;
const LABEL_WIDTH: usize = 9;

#[derive(Clone, Copy)]
struct Field {
    label: &'static str,
    key: &'static str,
    max: usize,
}

const NAME: usize = 0;
const LORA: usize = 3;
const FIELDS: [Field; 4] = [
    Field { label: "Name", key: "nt_name", max: NAME_MAX },
    Field { label: "Company", key: "nt_company", max: DETAIL_MAX },
    Field { label: "From", key: "nt_from", max: DETAIL_MAX },
    Field { label: "LoRa name", key: "username", max: NICK_MAX },
];

struct Edit {
    values: [String; 4],
    field: usize,
}

pub struct NametagApp {
    board_id: String,
    // Saved values. values[LORA] is never empty (it falls back to the board ID); values[NAME] may be.
    values: [String; 4],
    edit: Option<Edit>,
    renamed: Option<String>,
}

impl NametagApp {
    pub fn load(nvs: &EspNvs<NvsDefault>, board_id: &str) -> Self {
        let values = FIELDS.map(|f| read(nvs, f));
        let mut app = Self { board_id: board_id.into(), values, edit: None, renamed: None };
        if app.values[LORA].is_empty() {
            app.values[LORA] = app.board_id.clone();
        }
        app
    }

    // Chat nick, menu title and ELF badge_name().
    pub fn lora_name(&self) -> &str {
        &self.values[LORA]
    }

    fn nametag_name(&self) -> &str {
        if self.values[NAME].is_empty() { &self.values[LORA] } else { &self.values[NAME] }
    }

    // The new LoRa name after a save that changed it, once.
    pub fn take_renamed(&mut self) -> Option<String> {
        self.renamed.take()
    }

    pub fn open<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        self.edit = None;
        self.draw_view(d);
    }

    // True when the app should close.
    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D, nvs: &EspNvs<NvsDefault>) -> bool {
        if self.edit.is_some() {
            self.edit_key(k, shift, d, nvs);
            return false;
        }
        match k {
            keys::A | keys::OK => {
                self.edit = Some(Edit { values: self.values.clone(), field: NAME });
                self.draw_edit(d);
                false
            }
            k => keys::is_back(k),
        }
    }

    fn edit_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D, nvs: &EspNvs<NvsDefault>) {
        let Some(edit) = self.edit.as_mut() else { return };
        match k {
            keys::B | keys::ESC => {
                self.edit = None;
                return self.draw_view(d);
            }
            keys::OK | keys::A => {
                self.save(nvs);
                return self.draw_view(d);
            }
            keys::UP | keys::DOWN => {
                edit.field = keys::wrap_selection(k, edit.field, FIELDS.len()).unwrap();
                return self.draw_edit(d);
            }
            keys::TAB => {
                edit.field = (edit.field + 1) % FIELDS.len();
                return self.draw_edit(d);
            }
            keys::DEL => {
                edit.values[edit.field].pop();
            }
            _ => match keys::char_for(k, shift) {
                Some(c) if edit.values[edit.field].len() < FIELDS[edit.field].max => edit.values[edit.field].push(c),
                _ => return,
            },
        }
        let field = edit.field;
        self.draw_row(d, field);
        if let Some(edit) = &self.edit {
            ui::input_box(d, &edit.values[field]);
        }
    }

    fn save(&mut self, nvs: &EspNvs<NvsDefault>) {
        let Some(edit) = self.edit.take() else { return };
        for (field, value) in FIELDS.iter().zip(&edit.values) {
            write(nvs, field, value.trim());
        }
        let mut values = edit.values.map(|v| v.trim().to_string());
        if values[LORA].is_empty() {
            values[LORA] = self.board_id.clone();
        }
        if values[LORA] != self.values[LORA] {
            log::info!("nametag: lora name {}", values[LORA]);
            self.renamed = Some(values[LORA].clone());
        }
        self.values = values;
    }

    fn draw_view<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        let [_, company, from, _] = &self.values;
        let name = self.nametag_name();
        ui::header(d, SUBTITLE);
        ui::background(d, 0, HEADER_HEIGHT, SCREEN_WIDTH, ui::CONTENT_HEIGHT);
        centered(d, GREETING, GREETING_BASELINE, &SMALL, ACCENT);
        centered(d, name, NAME_BASELINE, &TITLE, FG);
        let rule = ui::text_width(name, &TITLE).max(RULE_MIN_WIDTH);
        ui::fill(d, (SCREEN_WIDTH - rule) / 2, RULE_TOP, rule as u32, RULE_HEIGHT, ACCENT);
        if company.is_empty() && from.is_empty() {
            centered(d, EMPTY_HINT, COMPANY_BASELINE, &SMALL, MUTED);
        }
        if !company.is_empty() {
            centered(d, company, COMPANY_BASELINE, &BODY, FG);
        }
        if !from.is_empty() {
            // Moves up into the company line when there is no company.
            let y = if company.is_empty() { COMPANY_BASELINE } else { FROM_BASELINE };
            centered(d, &format!("{FROM_PREFIX}{from}"), y, &BODY, MUTED);
        }
        ui::footer(d, VIEW_HINT);
    }

    fn draw_edit<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        let Some(edit) = &self.edit else { return };
        // Rows 0-3 are the fields, drawn by draw_row.
        let lines = [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "UP/DOWN or TAB: pick a line. Type to edit.".to_string(),
            "Empty name uses the LoRa name (chat).".to_string(),
            format!("Empty LoRa name goes back to {}.", self.board_id),
        ];
        ui::text_page(d, EDIT_SUBTITLE, &ui::as_strs(&lines), EDIT_HINT);
        for field in 0..FIELDS.len() {
            self.draw_row(d, field);
        }
        ui::input_box(d, &edit.values[edit.field]);
    }

    fn draw_row<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D, field: usize) {
        let Some(edit) = &self.edit else { return };
        let top = ROW_TOP + field as i32 * ROW_HEIGHT;
        let selected = field == edit.field;
        let marker = if selected { ">" } else { " " };
        let line = format!("{marker} {:<LABEL_WIDTH$} {}", FIELDS[field].label, edit.values[field]);
        ui::background(d, 0, top, SCREEN_WIDTH, ROW_HEIGHT);
        ui::text(d, &line, MARGIN, top + ROW_BASELINE, &SMALL, if selected { FG } else { MUTED });
    }
}

fn centered<D: DrawTarget<Color = Rgb565>>(d: &mut D, s: &str, baseline: i32, font: &embedded_graphics::mono_font::MonoFont, c: Rgb565) {
    ui::text(d, s, (SCREEN_WIDTH - ui::text_width(s, font)) / 2, baseline, font, c);
}

fn read(nvs: &EspNvs<NvsDefault>, field: Field) -> String {
    // NVS strings need room for the terminator.
    let mut buf = [0u8; DETAIL_MAX + 1];
    match nvs.get_str(field.key, &mut buf[..field.max + 1]) {
        Ok(Some(value)) => value.trim().to_string(),
        _ => String::new(),
    }
}

fn write(nvs: &EspNvs<NvsDefault>, field: &Field, value: &str) {
    let result = if value.is_empty() { nvs.remove(field.key).map(|_| ()) } else { nvs.set_str(field.key, value) };
    if let Err(e) = result {
        log::error!("nametag: saving {}: {e}", field.key);
    }
}
