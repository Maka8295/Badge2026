mod listing;
mod viewer;

use std::path::{Path, PathBuf};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::elf;
use crate::keys::{self, is_back, is_go, Key, Keys};
use crate::sd::{CardState, SdCard, MOUNT_POINT};
use crate::ui;
use listing::{Entry, Kind};
use viewer::Viewer;

const TITLE: &str = "// files";
const LIST_HINT: &str = "UP/DOWN move   A open   B back";
const PROGRAM_HINT: &str = "UP/DOWN move   A run   B back";
const PROGRAM_EXTENSION: &str = "elf";
const RUNNING_HINT: &str = "loading";
const NO_CARD_LINES: [&str; 1] = ["No SD card"];
const NO_CARD_HINT: &str = "A retry   B back";
const NO_FILESYSTEM_LINES: [&str; 2] = ["Card is not FAT32", "(exFAT or not formatted)"];
const NO_FILESYSTEM_HINT: &str = "A format   B back";
const CONFIRM_FORMAT_LINES: [&str; 3] = ["Erase the whole card", "and format it as FAT32?", "Everything on it is lost."];
const CONFIRM_FORMAT_HINT: &str = "OK erase   B cancel";
const FORMATTING_LINES: [&str; 1] = ["formatting..."];
const FORMATTING_HINT: &str = "please wait";
const FORMAT_FAILED: &str = "format failed";
const EMPTY_LABEL: &str = "(empty)";
const SUBTITLE_MAX_CHARS: usize = 20;
const CUT_MARK: &str = "...";

enum Screen {
    NoCard,
    NoFilesystem,
    ConfirmFormat,
    List,
    Viewer(Viewer),
    Failed(Vec<String>),
}

pub struct Explorer {
    screen: Screen,
    dir: PathBuf,
    entries: Vec<Entry>,
    sel: usize,
}

impl Explorer {
    pub fn new() -> Self {
        Self { screen: Screen::NoCard, dir: PathBuf::new(), entries: Vec::new(), sel: 0 }
    }

    pub fn open<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, sd: &mut SdCard) {
        self.dir = PathBuf::new();
        self.mount_and_list(d, sd);
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D, sd: &mut SdCard, keys: &mut Keys) -> bool {
        let back_to_list = match &mut self.screen {
            Screen::NoCard => return self.no_card_key(k, d, sd),
            Screen::NoFilesystem => return self.no_filesystem_key(k, d),
            Screen::ConfirmFormat => return self.confirm_format_key(k, d, sd),
            Screen::List => return self.list_key(k, d, keys),
            Screen::Viewer(viewer) => viewer.key(k, d),
            Screen::Failed(_) => is_back(k),
        };
        if back_to_list {
            self.screen = Screen::List;
            self.draw(d);
        }
        false
    }

    fn mount_and_list<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, sd: &mut SdCard) {
        self.screen = match sd.ensure_mounted() {
            CardState::Ready => {
                self.load();
                Screen::List
            }
            CardState::NoCard => Screen::NoCard,
            CardState::NoFilesystem => Screen::NoFilesystem,
        };
        self.draw(d);
    }

    fn no_card_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D, sd: &mut SdCard) -> bool {
        if matches!(k, keys::A | keys::OK) {
            self.mount_and_list(d, sd);
            return false;
        }
        is_back(k)
    }

    fn no_filesystem_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) -> bool {
        if matches!(k, keys::A | keys::OK) {
            self.screen = Screen::ConfirmFormat;
            self.draw(d);
            return false;
        }
        is_back(k)
    }

    fn confirm_format_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D, sd: &mut SdCard) -> bool {
        match k {
            keys::OK => self.format(d, sd),
            _ if is_back(k) => {
                self.screen = Screen::NoFilesystem;
                self.draw(d);
            }
            _ => {}
        }
        false
    }

    fn format<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, sd: &mut SdCard) {
        message(d, &FORMATTING_LINES, FORMATTING_HINT);
        match sd.format_fat32() {
            Ok(()) => self.mount_and_list(d, sd),
            Err(e) => {
                self.screen = Screen::Failed(vec![FORMAT_FAILED.into(), e.to_string()]);
                self.draw(d);
            }
        }
    }

    fn list_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D, keys: &mut Keys) -> bool {
        let labels = self.labels();
        if let Some(new) = keys::wrap_selection(k, self.sel, labels.len()) {
            let old = self.sel;
            self.sel = new;
            ui::menu_select(d, &ui::as_strs(&labels), old, new, self.hint());
        } else if is_go(k) {
            self.open_selected(d, keys);
        } else if is_back(k) {
            return self.leave_dir(d);
        }
        false
    }

    fn open_selected<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, keys: &mut Keys) {
        let Some(entry) = self.entries.get(self.sel).cloned() else {
            return;
        };
        match entry.kind {
            Kind::Parent => self.go_up(),
            Kind::Dir => self.enter(&entry.name),
            Kind::File(_) if is_program(&entry.name) => self.run(&entry.name, d, keys),
            Kind::File(_) => self.view(&entry.name),
        }
        self.draw(d);
    }

    fn run<D: DrawTarget<Color = Rgb565>>(&mut self, name: &str, d: &mut D, keys: &mut Keys) {
        let path = self.dir.join(name);
        ui::text_page(d, &subtitle(&path), &[&format!("running {name}...")], RUNNING_HINT);
        if let Err(e) = elf::run(&card_path(&path), d, keys) {
            log::warn!("cannot run {}: {e}", path.display());
            self.screen = Screen::Failed(vec![format!("cannot run {name}"), e.to_string()]);
        }
    }

    fn leave_dir<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) -> bool {
        if self.at_root() {
            return true;
        }
        self.go_up();
        self.draw(d);
        false
    }

    fn enter(&mut self, name: &str) {
        self.dir.push(name);
        self.load();
    }

    fn go_up(&mut self) {
        let left = self.dir.file_name().map(|name| name.to_string_lossy().into_owned());
        self.dir.pop();
        self.load();
        self.sel = self.entries.iter().position(|e| Some(&e.name) == left.as_ref()).unwrap_or(0);
    }

    fn view(&mut self, name: &str) {
        let path = self.dir.join(name);
        let viewer = Viewer::open(&card_path(&path), subtitle(&path));
        self.screen = Screen::Viewer(viewer);
    }

    fn load(&mut self) {
        self.entries = listing::read(&card_path(&self.dir), self.at_root());
        self.sel = 0;
    }

    fn at_root(&self) -> bool {
        self.dir.as_os_str().is_empty()
    }

    fn hint(&self) -> &'static str {
        match self.entries.get(self.sel) {
            Some(Entry { name, kind: Kind::File(_) }) if is_program(name) => PROGRAM_HINT,
            _ => LIST_HINT,
        }
    }

    fn labels(&self) -> Vec<String> {
        if self.entries.is_empty() {
            return vec![EMPTY_LABEL.into()];
        }
        self.entries.iter().map(Entry::label).collect()
    }

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        match &self.screen {
            Screen::NoCard => message(d, &NO_CARD_LINES, NO_CARD_HINT),
            Screen::NoFilesystem => message(d, &NO_FILESYSTEM_LINES, NO_FILESYSTEM_HINT),
            Screen::ConfirmFormat => message(d, &CONFIRM_FORMAT_LINES, CONFIRM_FORMAT_HINT),
            Screen::List => ui::menu(d, &subtitle(&self.dir), &ui::as_strs(&self.labels()), self.sel, self.hint()),
            Screen::Viewer(viewer) => viewer.draw(d),
            Screen::Failed(lines) => ui::screen(d, TITLE, &ui::as_strs(lines)),
        }
    }
}

fn message<D: DrawTarget<Color = Rgb565>>(d: &mut D, lines: &[&str], hint: &str) {
    ui::screen(d, TITLE, lines);
    ui::footer(d, hint);
}

fn card_path(relative: &Path) -> PathBuf {
    Path::new(MOUNT_POINT).join(relative)
}

fn subtitle(relative: &Path) -> String {
    let path: Vec<char> = format!("/{}", relative.display()).chars().collect();
    if path.len() <= SUBTITLE_MAX_CHARS {
        return path.into_iter().collect();
    }
    let kept = SUBTITLE_MAX_CHARS - CUT_MARK.len();
    CUT_MARK.chars().chain(path[path.len() - kept..].iter().copied()).collect()
}


fn is_program(name: &str) -> bool {
    Path::new(name).extension().is_some_and(|e| e.eq_ignore_ascii_case(PROGRAM_EXTENSION))
}
