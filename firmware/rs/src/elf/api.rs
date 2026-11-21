use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::sync::Mutex;

use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::sys::{esp_elf_register_symbol, esp_elfsym, esp_timer_get_time};

use super::context::with_devices;
use crate::keys::{self, Key};
use crate::ui;

const NO_KEY: c_int = -1;
const KEY_CODE_ROW_STRIDE: c_int = 16;
const MICROS_PER_MILLI: i64 = 1000;
const FONTS: [&MonoFont; 3] = [&ui::SMALL, &ui::BODY, &ui::TITLE];
const SYMBOL_COUNT: usize = 11;

// The username. Only changed from the main loop, never while a program runs, so the pointer
// badge_name() hands out stays valid for the whole run.
static BOARD_NAME: Mutex<Option<CString>> = Mutex::new(None);

struct SymbolTable([esp_elfsym; SYMBOL_COUNT]);

unsafe impl Sync for SymbolTable {}

const fn symbol(name: &'static CStr, function: *const c_void) -> esp_elfsym {
    esp_elfsym { name: name.as_ptr(), sym: function }
}

static SYMBOLS: SymbolTable = SymbolTable([
    symbol(c"badge_clear", badge_clear as *const c_void),
    symbol(c"badge_fill_rect", badge_fill_rect as *const c_void),
    symbol(c"badge_draw_text", badge_draw_text as *const c_void),
    symbol(c"badge_screen_width", badge_screen_width as *const c_void),
    symbol(c"badge_screen_height", badge_screen_height as *const c_void),
    symbol(c"badge_key_pressed", badge_key_pressed as *const c_void),
    symbol(c"badge_key_held", badge_key_held as *const c_void),
    symbol(c"badge_millis", badge_millis as *const c_void),
    symbol(c"badge_delay_ms", badge_delay_ms as *const c_void),
    symbol(c"badge_name", badge_name as *const c_void),
    esp_elfsym { name: ptr::null(), sym: ptr::null() },
]);

pub fn set_name(name: &str) {
    if let Ok(mut stored) = BOARD_NAME.lock() {
        *stored = Some(CString::new(name).unwrap_or_default());
    }
}

pub fn register(name: &str) {
    set_name(name);
    let code = unsafe { esp_elf_register_symbol(SYMBOLS.0.as_ptr()) };
    if code != 0 {
        log::error!("cannot register badge api for programs: {code}");
    }
}

fn color(rgb565: u16) -> Rgb565 {
    RawU16::new(rgb565).into()
}

fn code_of(k: Key) -> c_int {
    c_int::from(k.0) * KEY_CODE_ROW_STRIDE + c_int::from(k.1)
}

fn key_from_code(code: c_int) -> Option<Key> {
    let (row, col) = (code / KEY_CODE_ROW_STRIDE, code % KEY_CODE_ROW_STRIDE);
    let valid = (1..=keys::ROWS as c_int).contains(&row) && (1..=keys::COLS as c_int).contains(&col);
    valid.then_some((row as u8, col as u8))
}

extern "C" fn badge_clear(rgb565: u16) {
    with_devices(|d| d.screen.clear_to(color(rgb565)));
}

extern "C" fn badge_fill_rect(x: c_int, y: c_int, w: c_int, h: c_int, rgb565: u16) {
    let (w, h) = (w.max(0) as u32, h.max(0) as u32);
    with_devices(|d| d.screen.fill_rect(x, y, w, h, color(rgb565)));
}

extern "C" fn badge_draw_text(x: c_int, y: c_int, text: *const c_char, rgb565: u16, size: c_int) {
    if text.is_null() {
        return;
    }
    let text = unsafe { CStr::from_ptr(text) }.to_string_lossy();
    let font = FONTS[(size.max(0) as usize).min(FONTS.len() - 1)];
    with_devices(|d| d.screen.draw_text(x, y, &text, color(rgb565), font));
}

extern "C" fn badge_screen_width() -> c_int {
    ui::SCREEN_WIDTH
}

extern "C" fn badge_screen_height() -> c_int {
    ui::SCREEN_HEIGHT
}

extern "C" fn badge_key_pressed() -> c_int {
    let next = with_devices(|mut d| {
        if d.pending_keys.is_empty() {
            d.scan_keys();
        }
        d.pending_keys.pop_front()
    });
    next.flatten().map_or(NO_KEY, code_of)
}

extern "C" fn badge_key_held(code: c_int) -> bool {
    let Some(k) = key_from_code(code) else {
        return false;
    };
    with_devices(|mut d| {
        d.scan_keys();
        d.keys.is_held(k)
    })
    .unwrap_or(false)
}

extern "C" fn badge_millis() -> u32 {
    (unsafe { esp_timer_get_time() } / MICROS_PER_MILLI) as u32
}

extern "C" fn badge_delay_ms(ms: u32) {
    FreeRtos::delay_ms(ms);
}

extern "C" fn badge_name() -> *const c_char {
    let stored = BOARD_NAME.lock();
    stored.ok().and_then(|name| name.as_ref().map(|n| n.as_ptr())).unwrap_or(c"".as_ptr())
}
