mod effects;
mod ir_swap;
mod lights;
mod morse;
mod stage;
mod swap_app;
mod vault;
mod view;

use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use crate::keys::{self, Key};
use crate::ui;
use lights::Lights;
use stage::{Action, Stage, SwapCode};
use vault::Vault;

pub use swap_app::SwapApp;

pub const SECRET_WORD: &str = "tengu";
const ANSWER_MAX: usize = 64;
const SERIAL_EVERY: Duration = Duration::from_secs(10);
const CODE_HINT: &str = "OK/A check   B back";
const PROMPT_HINT: &str = "A back";
const CHECKING: &str = "checking...";
const WRONG: &str = "not a code";
const WAIT: &str = "wait a few seconds";
const WRONG_WAIT: Duration = Duration::from_secs(5);

enum Screen {
    Code,
    Prompt,
}

pub struct Hunt {
    vault: Vault,
    stage: Stage,
    answer: String,
    screen: Screen,
    lights: Lights,
    serial: Option<String>,
    serial_at: Instant,
    board_name: String,
    nvs_partition: EspDefaultNvsPartition,
    unlocked: Vec<String>,
    swap: Option<SwapCode>,
    retry_at: Instant,
}

impl Hunt {
    pub fn new(board_name: &str, nvs_partition: &EspDefaultNvsPartition) -> Self {
        let vault = Vault::load();
        let badge = Stage::parse(&vault.public());
        let mut hunt = Hunt {
            vault,
            stage: Stage::default(),
            answer: String::new(),
            screen: Screen::Code,
            lights: Lights::new(),
            serial: None,
            serial_at: Instant::now(),
            board_name: board_name.into(),
            nvs_partition: nvs_partition.clone(),
            unlocked: effects::load_unlocked(nvs_partition, board_name),
            swap: None,
            retry_at: Instant::now(),
        };
        hunt.apply(badge.actions);
        hunt.reopen_unlocked();
        hunt
    }

    pub fn take_swap(&mut self) -> Option<SwapCode> {
        self.swap.take()
    }

    pub fn background_tick(&mut self) {
        self.lights.tick();
        if let Some(serial) = self.serial.as_ref().filter(|_| Instant::now() >= self.serial_at) {
            println!("{serial}");
            self.serial_at = Instant::now() + SERIAL_EVERY;
        }
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        match self.screen {
            Screen::Code => view::code_screen(d, &self.answer, CODE_HINT),
            Screen::Prompt => view::lines_screen(d, &self.stage.clue, PROMPT_HINT),
        }
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) -> bool {
        match self.screen {
            Screen::Code => return self.code_key(k, shift, d),
            Screen::Prompt if matches!(k, keys::A | keys::OK | keys::B | keys::ESC) => {
                self.stage = Stage::default();
                self.show(Screen::Code, d);
            }
            Screen::Prompt => {}
        }
        false
    }

    fn apply(&mut self, actions: Vec<Action>) {
        for action in actions {
            match action {
                Action::Serial(text) => self.serial = Some(text),
                Action::Morse(text) => self.lights.start_morse(&text),
                Action::Nvs(flag) => effects::write_nvs_flag(&self.nvs_partition, &flag, &self.board_name),
                Action::Program(program) => effects::store_program(&self.nvs_partition, &program),
                Action::IrSwap(code) => self.swap = Some(code),
                Action::IrRemote { name, text } => effects::add_text_remote(&name, &text),
                Action::Finish => {}
            }
        }
    }

    fn reopen_unlocked(&mut self) {
        for answer in self.unlocked.clone() {
            if let Some(plain) = self.vault.open(&answer) {
                self.apply(Stage::parse(&plain).actions);
            }
        }
    }

    fn remember_unlock(&mut self, answer: &str) {
        let answer = vault::normalize(answer);
        if !self.unlocked.contains(&answer) {
            self.unlocked.push(answer);
            effects::save_unlocked(&self.nvs_partition, &self.unlocked, &self.board_name);
        }
    }

    fn code_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) -> bool {
        match k {
            keys::B | keys::ESC => return true,
            k if keys::is_submit(k) && Instant::now() < self.retry_at => ui::footer(d, WAIT),
            k if keys::is_submit(k) && !self.answer.is_empty() => self.check_answer(d),
            keys::DEL => {
                self.answer.pop();
                ui::input_box(d, &self.answer);
            }
            _ => self.type_char(k, shift, d),
        }
        false
    }

    fn show<D: DrawTarget<Color = Rgb565>>(&mut self, screen: Screen, d: &mut D) {
        self.screen = screen;
        self.draw(d);
    }

    fn type_char<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) {
        if let Some(c) = keys::char_for(k, shift).filter(|_| self.answer.len() < ANSWER_MAX) {
            self.answer.push(c);
            ui::input_box(d, &self.answer);
        }
    }

    fn check_answer<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        ui::footer(d, CHECKING);
        let answer = std::mem::take(&mut self.answer);
        let Some(plain) = self.vault.open(&answer) else {
            self.retry_at = Instant::now() + WRONG_WAIT;
            ui::input_box(d, &self.answer);
            return ui::footer(d, WRONG);
        };
        self.stage = self.accept(&answer, Stage::parse(&plain));
        self.show(Screen::Prompt, d);
    }

    fn accept(&mut self, answer: &str, mut stage: Stage) -> Stage {
        if stage.unlocks() {
            self.remember_unlock(answer);
        }
        if stage.actions.iter().any(|a| matches!(a, Action::Finish)) {
            let code = vault::finish_code(answer, &self.board_name);
            stage.clue.extend([String::new(), format!("badge: {}", self.board_name), format!("code:  {code}")]);
        }
        self.apply(std::mem::take(&mut stage.actions));
        stage
    }
}
