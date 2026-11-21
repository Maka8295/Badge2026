use std::cell::RefCell;
use std::collections::VecDeque;

use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};

use crate::keys::{Key, Keys};
use crate::ui;

const MAX_PENDING_KEYS: usize = 16;

pub trait Screen {
    fn clear_to(&mut self, color: Rgb565);
    fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Rgb565);
    fn draw_text(&mut self, x: i32, y: i32, text: &str, color: Rgb565, font: &MonoFont);
}

impl<D: DrawTarget<Color = Rgb565>> Screen for D {
    fn clear_to(&mut self, color: Rgb565) {
        self.clear(color).ok();
    }

    fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Rgb565) {
        ui::fill(self, x, y, w, h, color);
    }

    fn draw_text(&mut self, x: i32, y: i32, text: &str, color: Rgb565, font: &MonoFont) {
        let style = MonoTextStyle::new(font, color);
        Text::with_baseline(text, Point::new(x, y), style, Baseline::Top).draw(self).ok();
    }
}

pub struct Devices<'a> {
    pub screen: &'a mut dyn Screen,
    pub keys: &'a mut Keys,
    pub pending_keys: &'a mut VecDeque<Key>,
}

impl Devices<'_> {
    pub fn scan_keys(&mut self) {
        self.pending_keys.extend(self.keys.pressed());
        let extra = self.pending_keys.len().saturating_sub(MAX_PENDING_KEYS);
        self.pending_keys.drain(..extra);
    }
}

struct Context {
    screen: *mut (dyn Screen + 'static),
    keys: *mut Keys,
    pending_keys: VecDeque<Key>,
}

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) };
}

pub fn while_running<R>(screen: &mut dyn Screen, keys: &mut Keys, run: impl FnOnce() -> R) -> R {
    let context = Context { screen: forget_lifetime(screen), keys, pending_keys: VecDeque::new() };
    CONTEXT.with_borrow_mut(|slot| *slot = Some(context));
    let result = run();
    CONTEXT.with_borrow_mut(|slot| *slot = None);
    result
}

pub fn with_devices<R>(f: impl FnOnce(Devices) -> R) -> Option<R> {
    CONTEXT.with_borrow_mut(|slot| {
        let context = slot.as_mut()?;
        // SAFETY: both pointers come from `&mut` borrows held by `while_running`, which clears the slot before returning.
        let devices = unsafe {
            Devices { screen: &mut *context.screen, keys: &mut *context.keys, pending_keys: &mut context.pending_keys }
        };
        Some(f(devices))
    })
}

fn forget_lifetime(screen: &mut dyn Screen) -> *mut (dyn Screen + 'static) {
    let screen: *mut (dyn Screen + '_) = screen;
    unsafe { std::mem::transmute(screen) }
}
