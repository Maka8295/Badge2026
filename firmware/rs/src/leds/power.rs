// Current estimate for the SK6803MINI-E chain, the same idea as FastLED's setMaxPowerInVoltsAndMilliamps.
// SK6803MINI-E datasheet: IOUT = 5 mA per colour channel at full PWM, about 1 mA static per LED.
// So 10 LEDs at full white draw about 160 mA from the "5V" rail.
//
// HARDWARE: the "5V" rail is the TPS61023 boost from the battery (see HARDWARE.md "Power"). On USB the
// MCP73871 is set for 500 mA input, shared with charging (~210 mA), the ESP32 (Wi-Fi peaks ~350 mA at 3.3 V)
// and the LCD backlight (~85 mA). The LED budget below keeps the strip from eating that margin.

use super::palettes::Color;
use super::{Rgb, LED_COUNT};

pub const MA_PER_CHANNEL: f32 = 5.0;
pub const STATIC_MA_PER_LED: f32 = 1.0;
// Total for all ten LEDs. The theme strip is scaled down to stay under it; the morse and activity
// LEDs are never scaled.
pub const BUDGET_MA: f32 = 90.0;
// Above this brightness the LEDs app shows a warning and the log says so.
pub const WARN_LEVEL: u8 = 128;

pub fn channel_ma(value: f32) -> f32 {
    value / 255.0 * MA_PER_CHANNEL
}

pub fn rgb_ma(&(r, g, b): &Rgb) -> f32 {
    channel_ma(r as f32) + channel_ma(g as f32) + channel_ma(b as f32)
}

pub fn color_ma(c: &Color) -> f32 {
    channel_ma(c[0] + c[1] + c[2])
}

// Worst case at a brightness level: every LED full white, before the limiter.
pub fn worst_case_ma(level: u8) -> f32 {
    LED_COUNT as f32 * (3.0 * channel_ma(level as f32) + STATIC_MA_PER_LED)
}

// Factor to multiply the strip by so strip + fixed LEDs stay under the budget.
pub fn limit(strip_ma: f32, fixed_ma: f32) -> f32 {
    let room = BUDGET_MA - LED_COUNT as f32 * STATIC_MA_PER_LED - fixed_ma;
    if strip_ma <= room || strip_ma <= 0.0 { 1.0 } else { (room / strip_ma).max(0.0) }
}
