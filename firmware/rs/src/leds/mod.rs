use std::f32::consts::TAU;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::gpio::OutputPin;
use esp_idf_svc::hal::rmt::config::{MemoryAccess, TransmitConfig, TxChannelConfig};
use esp_idf_svc::hal::rmt::encoder::CopyEncoder;
use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, Symbol, TxChannelDriver};
use esp_idf_svc::hal::units::FromValueType;

use crate::rmt_padding;

pub mod effects;
pub mod palettes;
pub mod power;

use effects::{Effect, State, SPEEDS, STRIP, THEMES};
use palettes::{Color, CHOICES};

pub const LED_COUNT: usize = 10;
// Chain index 0-7: left edge strip, bottom to top, driven by the theme.
// 8: top of the left edge, the hunt's morse LED. Never touched by themes, breathing or the limiter.
// 9: top right, LoRa/IR activity.
pub const MORSE_LED_INDEX: usize = STRIP;
pub const ACTIVITY_LED_INDEX: usize = 9;
const BITS_PER_LED: usize = 24;
const TICK_RATE_MHZ: u32 = 10;
const ZERO_HIGH_TICKS: u16 = 3;
const ZERO_LOW_TICKS: u16 = 9;
const ONE_HIGH_TICKS: u16 = 6;
const ONE_LOW_TICKS: u16 = 6;
const DMA_BUFFER_SYMBOLS: usize = 256;
const RESET_LOW_TICKS: u16 = 500;
const ACTIVITY_BREATH: Duration = Duration::from_millis(1000);
const ACTIVITY_BREATHS: u32 = 2;
const ACTIVITY_PULSE: Duration = Duration::from_millis(70);
const ACTIVITY_PULSES: u32 = 4;
const TICK_INTERVAL: Duration = Duration::from_millis(10);
const TICK_THREAD_STACK_BYTES: usize = 6144;
// HARDWARE: keep this refresh. Do not switch to sending only on change.
// D69 (first in the chain) gets a 3.3 V data high, but the SK6803 datasheet only guarantees a "1" at
// VIH >= 0.7 * VDD (3.47 V on the 4.95 V rail, about 3.6 V on USB). It works on the boards we have,
// but a bit can be misread, so the frame is re-sent regularly and any wrong colour clears itself.
// See HARDWARE.md "LEDs".
const REFRESH_INTERVAL: Duration = Duration::from_millis(500);

const BREATH_FRAME: Duration = Duration::from_millis(20);
// Longest frame step fed to effects, so a stalled thread does not make them jump.
const MAX_FRAME_SECONDS: f32 = 0.1;
// Gamma on each colour's brightness (its largest channel), so fades look even.
const GAMMA: f32 = 2.2;
// Milder gamma on each channel's share of the colour. Palettes are written in screen terms, where a small
// channel barely shows; sent linearly to the LED it is real light and washes colours towards white
// ((10, 120, 25) came out mint). The full 2.2 per channel overcorrects: orange (255, 90, 0) turns red
// and pinks go magenta.
const CHROMA_GAMMA: f32 = 1.6;
// Squaring the raised cosine roughly corrects for the eye's response, so the fade looks even.
const BREATH_GAMMA: i32 = 2;

pub const FULL: u8 = 255;
pub const DEFAULT_LEVEL: u8 = 40;
pub const OFF: Rgb = (0, 0, 0);

pub type Rgb = (u8, u8, u8);

pub const LORA_COLOR: Rgb = (0, 0, FULL);
pub const IR_COLOR: Rgb = (FULL, 0, 0);

#[derive(Clone, Copy)]
struct Activity {
    color: Rgb,
    started: Instant,
    breath: bool,
}

impl Activity {
    fn length(&self) -> Duration {
        if self.breath { ACTIVITY_BREATH * ACTIVITY_BREATHS } else { ACTIVITY_PULSE * (2 * ACTIVITY_PULSES - 1) }
    }

    fn level(&self) -> f32 {
        let elapsed = self.started.elapsed();
        if self.breath {
            breath_level((elapsed.as_secs_f32() / ACTIVITY_BREATH.as_secs_f32()).fract())
        } else if (elapsed.as_millis() / ACTIVITY_PULSE.as_millis()) % 2 == 0 {
            1.0
        } else {
            0.0
        }
    }
}

struct Led {
    tx: TxChannelDriver<'static>,
    morse: Rgb,
    activity: Option<Activity>,
    sent: Instant,
    level: u8,
    theme: usize,
    palette: Option<usize>,
    speed: usize,
    state: State,
    clock: f32,
    last_frame: Instant,
}

static LED: Mutex<Option<Led>> = Mutex::new(None);

pub fn init(pin: impl OutputPin + 'static) {
    let config = TxChannelConfig {
        resolution: TICK_RATE_MHZ.MHz().into(),
        memory_access: MemoryAccess::Direct { memory_block_symbols: DMA_BUFFER_SYMBOLS },
        ..Default::default()
    };
    let mut led = Led {
        tx: TxChannelDriver::new(pin, &config).unwrap(),
        morse: OFF,
        activity: None,
        sent: Instant::now(),
        level: DEFAULT_LEVEL,
        theme: 0,
        palette: None,
        speed: effects::DEFAULT_SPEED,
        state: State::default(),
        clock: 0.0,
        last_frame: Instant::now(),
    };
    led.show();
    *LED.lock().unwrap() = Some(led);
    // Own thread: screen redraws and radio waits block the main loop for long enough to freeze a pulse.
    thread::Builder::new().stack_size(TICK_THREAD_STACK_BYTES).spawn(tick_forever).unwrap();
}

// Transmit: rapid pulses on the activity LED. Repeat sends during the pulses join them instead of restarting.
pub fn flash(color: Rgb) {
    with_led(|led| {
        let now = Instant::now();
        let started = led.activity.filter(|a| !a.breath).map_or(now, |a| a.started);
        led.activity = Some(Activity { color, started, breath: false });
        led.show();
    });
}

// Receive: two slow breaths on the activity LED. Skipped during transmit pulses, so a badge hearing its own
// IR does not replace them.
pub fn breathe(color: Rgb) {
    with_led(|led| {
        if led.activity.is_some_and(|a| !a.breath) {
            return;
        }
        let now = Instant::now();
        let started = led.activity.filter(|a| a.breath).map_or(now, |a| rising_start(a.started, now));
        led.activity = Some(Activity { color, started, breath: true });
        led.show();
    });
}

// A new event during a fade keeps the current brightness and starts rising through both breaths again,
// so bursts never blink to dark.
fn rising_start(started: Instant, now: Instant) -> Instant {
    let cycle_ms = ACTIVITY_BREATH.as_millis() as u64;
    let into_breath_ms = now.saturating_duration_since(started).as_millis() as u64 % cycle_ms;
    let rising_ms = into_breath_ms.min(cycle_ms - into_breath_ms);
    now - Duration::from_millis(rising_ms)
}

pub fn breath_level(progress: f32) -> f32 {
    if progress >= 1.0 {
        return 0.0;
    }
    ((1.0 - (TAU * progress).cos()) / 2.0).powi(BREATH_GAMMA)
}

// The hunt's morse LED. Shown as given, scaled only by the brightness level.
pub fn set_morse(color: Rgb) {
    with_led(|led| {
        if led.morse != color {
            led.morse = color;
            led.show();
        }
    });
}

pub fn set_theme(theme: usize) {
    with_led(|led| {
        led.theme = theme.min(THEMES.len() - 1);
        led.state.reset();
        led.show();
    });
}

// None: the theme's own palette.
pub fn set_palette(palette: Option<usize>) {
    with_led(|led| {
        led.palette = palette.filter(|&p| p < CHOICES.len());
        led.show();
    });
}

pub fn set_speed(speed: usize) {
    with_led(|led| led.speed = speed.min(SPEEDS.len() - 1));
}

pub fn set_level(level: u8) {
    with_led(|led| {
        if level > power::WARN_LEVEL && led.level <= power::WARN_LEVEL {
            log::warn!(
                "leds: brightness {level}/{FULL} is high, up to {:.0} mA before the {:.0} mA limit",
                power::worst_case_ma(level),
                power::BUDGET_MA
            );
        }
        led.level = level;
        led.show();
    });
}

fn tick_forever() {
    loop {
        tick();
        thread::sleep(TICK_INTERVAL);
    }
}

fn tick() {
    with_led(|led| {
        let now = Instant::now();
        if led.activity.is_some_and(|a| now >= a.started + a.length()) {
            led.activity = None;
            led.show();
        } else if now >= led.sent + led.refresh_interval() {
            led.show();
        }
    });
}

fn with_led(f: impl FnOnce(&mut Led)) {
    if let Some(led) = LED.lock().unwrap().as_mut() {
        f(led);
    }
}

impl Led {
    fn show(&mut self) {
        let mut frame = [OFF; LED_COUNT];
        frame[MORSE_LED_INDEX] = morse_dim(self.morse, self.level_scale());
        if let Some(activity) = self.activity {
            frame[ACTIVITY_LED_INDEX] = dim(activity.color, activity.level() * self.level_scale());
        }
        let fixed_ma = power::rgb_ma(&frame[MORSE_LED_INDEX]) + power::rgb_ma(&frame[ACTIVITY_LED_INDEX]);
        let strip = self.render_strip();
        let limit = power::limit(strip.iter().map(power::color_ma).sum(), fixed_ma);
        for (i, color) in strip.iter().enumerate() {
            frame[i] = to_rgb(palettes::scale(*color, limit));
        }
        let mut symbols = chain_symbols(&frame);
        rmt_padding::avoid_exact_fill(&mut symbols, DMA_BUFFER_SYMBOLS);
        if let Err(e) = self.tx.send_and_wait(CopyEncoder::new().unwrap(), &symbols, &TransmitConfig::default()) {
            log::error!("leds: {e}");
        }
        self.sent = Instant::now();
    }

    fn refresh_interval(&self) -> Duration {
        if self.activity.is_some() || self.animated() { BREATH_FRAME } else { REFRESH_INTERVAL }
    }

    fn animated(&self) -> bool {
        THEMES[self.theme].effect != Effect::Off
    }

    // The theme's strip colours in 0-255 units, after gamma and brightness.
    fn render_strip(&mut self) -> [Color; STRIP] {
        let now = Instant::now();
        let real_dt = now.saturating_duration_since(self.last_frame).as_secs_f32().min(MAX_FRAME_SECONDS);
        self.last_frame = now;
        let dt = real_dt * SPEEDS[self.speed].factor;
        self.clock += dt;
        let theme = &THEMES[self.theme];
        let palette = self.palette.map_or(theme.palette, |p| CHOICES[p]);
        let own_heat = self.palette.is_none();
        let colors = effects::render(theme.effect, palette, own_heat, &mut self.state, self.clock, dt);
        let level = self.level as f32;
        colors.map(|c| {
            let c = c.map(|v| v.clamp(0.0, 1.0));
            let peak = c[0].max(c[1]).max(c[2]);
            if peak <= 0.0 {
                return [0.0; 3];
            }
            let brightness = peak.powf(GAMMA) * level;
            c.map(|v| (v / peak).powf(CHROMA_GAMMA) * brightness)
        })
    }

    fn level_scale(&self) -> f32 {
        self.level as f32 / FULL as f32
    }
}

// No temporal dithering: at this frame rate it shows as flicker on dim channels (a 1 <-> 2 step is
// a 2x brightness jump). FastLED only hides it at refresh rates in the hundreds of Hz.
fn to_rgb(c: Color) -> Rgb {
    let channel = |v: f32| v.round().clamp(0.0, FULL as f32) as u8;
    (channel(c[0]), channel(c[1]), channel(c[2]))
}

// The morse LED is white: three channels at the menu level put out about three times the light of a
// theme LED at full colour. Share one channel's worth between the lit channels so it matches the strip,
// but never round a lit channel down to off.
fn morse_dim(color: Rgb, scale: f32) -> Rgb {
    let (r, g, b) = color;
    let lit = [r, g, b].iter().filter(|&&c| c > 0).count();
    if lit == 0 || scale <= 0.0 {
        return OFF;
    }
    let channel = |c: u8| if c == 0 { 0 } else { ((c as f32 * scale / lit as f32).round() as u8).max(1) };
    (channel(r), channel(g), channel(b))
}

fn dim((r, g, b): Rgb, scale: f32) -> Rgb {
    let channel = |c: u8| (c as f32 * scale).round() as u8;
    (channel(r), channel(g), channel(b))
}

fn chain_symbols(colors: &[Rgb; LED_COUNT]) -> Vec<Symbol> {
    let mut symbols = Vec::with_capacity(LED_COUNT * BITS_PER_LED + 1);
    for &(r, g, b) in colors {
        let grb = u32::from_be_bytes([0, g, r, b]);
        symbols.extend((0..BITS_PER_LED).rev().map(|bit| bit_symbol(grb >> bit & 1 == 1)));
    }
    symbols.push(Symbol::new(pulse(PinState::Low, RESET_LOW_TICKS), pulse(PinState::Low, RESET_LOW_TICKS)));
    symbols
}

fn bit_symbol(one: bool) -> Symbol {
    let (high, low) = if one { (ONE_HIGH_TICKS, ONE_LOW_TICKS) } else { (ZERO_HIGH_TICKS, ZERO_LOW_TICKS) };
    Symbol::new(pulse(PinState::High, high), pulse(PinState::Low, low))
}

fn pulse(state: PinState, ticks: u16) -> Pulse {
    Pulse::new(state, PulseTicks::new(ticks).unwrap())
}
