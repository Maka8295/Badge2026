use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use super::ir_swap::IrSwap;
use super::stage::SwapCode;
use crate::ir::Ir;
use crate::keys::{self, Key};
use crate::ui;

const TITLE: &str = "// badge swap";
const NAMES_SHOWN: usize = 7;

pub struct SwapApp {
    swap: IrSwap,
    code: Option<SwapCode>,
}

impl SwapApp {
    pub fn new(board_name: &str) -> Self {
        SwapApp { swap: IrSwap::new(board_name), code: None }
    }

    pub fn unlock(&mut self, code: SwapCode) {
        self.code = Some(code);
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        ui::text_page(d, TITLE, &ui::as_strs(&self.lines()), ui::BACK_HINT);
    }

    pub fn tick<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, ir: &mut Ir) {
        if self.swap.tick(ir) {
            self.draw(d);
        }
    }

    pub fn key(&self, k: Key) -> bool {
        keys::is_back(k)
    }

    fn lines(&self) -> Vec<String> {
        let found = self.swap.seen().len();
        let mut lines = vec![match &self.code {
            Some(code) => format!("badges found: {}/{}", found.min(code.badges), code.badges),
            None => format!("badges found: {found}"),
        }];
        lines.extend(self.swap.seen().iter().take(NAMES_SHOWN).cloned());
        if let Some(code) = self.code.as_ref().filter(|code| found >= code.badges) {
            lines.push(format!("code: {}", code.secret));
        }
        lines
    }
}
