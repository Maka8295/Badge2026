// Animations for the 8-LED strip on the left edge (index 0 = bottom, 7 = top). Ideas from FastLED's
// examples (Fire2012, TwinkleFOX, Cylon, ColorPalette) and WLED effects, sized for 8 pixels.
// Every effect gets its clock and frame time already multiplied by the speed setting.

use std::f32::consts::TAU;

use esp_idf_svc::sys::esp_random;

use super::palettes::{self, mix, scale, Color, Palette, BLACK, HEAT, WHITE};

pub const STRIP: usize = 8;
const TOP: f32 = (STRIP - 1) as f32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Cyberpunk,
    Rain,
    Glitch,
    Flow,
    Breathe,
    Scanner,
    Comet,
    Twinkle,
    Fire,
    Heartbeat,
    Chase,
    Static,
    Off,
}

#[derive(Clone, Copy, Default)]
struct Twinkle {
    position: f32,
    length: f32,
    rest: f32,
    elapsed: f32,
}

#[derive(Default)]
struct Glitch {
    left: f32,
    next_in: f32,
    mask: u8,
    kind: u8,
}

#[derive(Default)]
pub struct State {
    trail: [Color; STRIP],
    heat: [f32; STRIP],
    twinkles: [Twinkle; STRIP],
    drops: Vec<(f32, Color)>,
    glitch: Glitch,
    fire_due: f32,
    packet: Option<f32>,
}

impl State {
    pub fn reset(&mut self) {
        *self = State::default();
    }
}

// FastLED fadeToBlackBy, per second instead of per frame so it does not depend on the frame rate.
fn fade(trail: &mut [Color; STRIP], per_second: f32, dt: f32) {
    let keep = (-per_second * dt).exp();
    for c in trail.iter_mut() {
        *c = scale(*c, keep);
    }
}

fn add(a: Color, b: Color) -> Color {
    [(a[0] + b[0]).min(1.0), (a[1] + b[1]).min(1.0), (a[2] + b[2]).min(1.0)]
}

// Lights the two pixels around a fractional position, so moving dots glide instead of jumping.
fn splat(trail: &mut [Color; STRIP], position: f32, color: Color) {
    let low = position.floor();
    let blend = position - low;
    for (pixel, weight) in [(low, 1.0 - blend), (low + 1.0, blend)] {
        if (0.0..=TOP).contains(&pixel) {
            let i = pixel as usize;
            trail[i] = add(trail[i], scale(color, weight));
        }
    }
}

fn triangle(t: f32) -> f32 {
    let x = t.rem_euclid(2.0);
    if x < 1.0 { x } else { 2.0 - x }
}

fn smooth_pulse(x: f32, width: f32) -> f32 {
    (-(x / width).powi(2)).exp()
}

fn along(i: usize) -> f32 {
    i as f32 / STRIP as f32
}

pub fn random() -> f32 {
    // SAFETY: esp_random has no preconditions.
    (unsafe { esp_random() } >> 8) as f32 / (1 << 24) as f32
}

fn chance(per_second: f32, dt: f32) -> bool {
    random() < per_second * dt
}

pub fn render(effect: Effect, palette: &Palette, own_heat: bool, s: &mut State, t: f32, dt: f32) -> [Color; STRIP] {
    match effect {
        Effect::Off => [BLACK; STRIP],
        Effect::Static => std::array::from_fn(|i| palette.at(along(i) * 0.5)),
        Effect::Flow => std::array::from_fn(|i| palette.at(along(i) * 0.5 + t * 0.12)),
        Effect::Breathe => {
            let level = 0.08 + 0.92 * super::breath_level((t / 4.0).fract());
            std::array::from_fn(|i| scale(palette.at(along(i) * 0.3 + t * 0.02), level))
        }
        Effect::Scanner => scanner(palette, s, t, dt),
        Effect::Comet => comet(palette, s, t, dt),
        Effect::Twinkle => twinkle(palette, s, dt),
        Effect::Fire => fire(palette, own_heat, s, dt),
        Effect::Rain => rain(palette, s, dt),
        Effect::Glitch => {
            let base: [Color; STRIP] = std::array::from_fn(|i| scale(palette.at(along(i) * 0.4 + t * 0.03), 0.7));
            glitch(base, palette, s, dt, 1.2)
        }
        Effect::Heartbeat => heartbeat(palette, t),
        Effect::Chase => chase(palette, t),
        Effect::Cyberpunk => cyberpunk(palette, s, t, dt),
    }
}

// Cylon / KITT: one eye bouncing end to end with a fading tail.
fn scanner(palette: &Palette, s: &mut State, t: f32, dt: f32) -> [Color; STRIP] {
    fade(&mut s.trail, 7.0, dt);
    splat(&mut s.trail, triangle(t * 0.7) * TOP, palette.at(t * 0.05));
    s.trail
}

// A meteor shooting up the strip, then a short gap.
fn comet(palette: &Palette, s: &mut State, t: f32, dt: f32) -> [Color; STRIP] {
    const LAP: f32 = TOP + 5.0;
    fade(&mut s.trail, 5.0, dt);
    let head = (t * 7.0).rem_euclid(LAP);
    if head <= TOP {
        splat(&mut s.trail, head, mix(palette.at(t * 0.04), WHITE, 0.25));
    }
    s.trail
}

// TwinkleFOX-style: each LED slowly glows up and down in its own random palette colour.
fn twinkle(palette: &Palette, s: &mut State, dt: f32) -> [Color; STRIP] {
    std::array::from_fn(|i| {
        let tw = &mut s.twinkles[i];
        tw.elapsed += dt;
        if tw.length == 0.0 || tw.elapsed >= tw.length + tw.rest {
            *tw = Twinkle { position: random(), length: 2.5 + 2.0 * random(), rest: 1.5 * random(), elapsed: 0.0 };
        }
        scale(palette.at(tw.position), super::breath_level(tw.elapsed / tw.length))
    })
}

// Fire2012 (Mark Kriegsman, FastLED), run at 30 steps a second and scaled down to 8 cells.
fn fire(palette: &Palette, own_heat: bool, s: &mut State, dt: f32) -> [Color; STRIP] {
    const STEP: f32 = 1.0 / 30.0;
    const COOLING: f32 = 0.3;
    const SPARKING: f32 = 0.5;
    s.fire_due += dt;
    while s.fire_due >= STEP {
        s.fire_due -= STEP;
        for h in s.heat.iter_mut() {
            *h = (*h - random() * COOLING).max(0.0);
        }
        for k in (2..STRIP).rev() {
            s.heat[k] = (s.heat[k - 1] + 2.0 * s.heat[k - 2]) / 3.0;
        }
        if random() < SPARKING {
            let y = (random() * 2.0) as usize;
            s.heat[y] = (s.heat[y] + 0.6 + 0.4 * random()).min(1.0);
        }
    }
    std::array::from_fn(|i| {
        let h = s.heat[i];
        if own_heat { HEAT.along(h) } else { scale(palette.at(h * 0.5), h) }
    })
}

// Matrix code rain: heads falling from the top, leaving fading trails. Heads stay the palette colour:
// mixing in white washed the dark Matrix greens out to mint.
fn rain(palette: &Palette, s: &mut State, dt: f32) -> [Color; STRIP] {
    fade(&mut s.trail, 3.5, dt);
    if chance(1.6, dt) {
        s.drops.push((TOP + 0.99, palette.at(random())));
    }
    for (position, color) in s.drops.iter_mut() {
        *position -= dt * 9.0;
        if *position >= 0.0 {
            let i = (position.round() as usize).min(STRIP - 1);
            s.trail[i] = *color;
        }
    }
    s.drops.retain(|(position, _)| *position >= 0.0);
    s.trail
}

// Short random bursts: dropouts, colour inversion, a palette jump, or white sparks.
fn glitch(mut frame: [Color; STRIP], palette: &Palette, s: &mut State, dt: f32, rate: f32) -> [Color; STRIP] {
    let g = &mut s.glitch;
    if g.left > 0.0 {
        g.left -= dt;
    } else {
        g.next_in -= dt;
        if g.next_in <= 0.0 {
            g.left = 0.05 + 0.2 * random();
            g.next_in = (0.4 + 2.5 * random()) / rate;
            g.mask = (random() * 255.0) as u8 | 1 << (random() * 8.0) as u8;
            g.kind = (random() * 4.0) as u8;
        }
    }
    if g.left <= 0.0 {
        return frame;
    }
    let jump = random();
    for (i, c) in frame.iter_mut().enumerate().filter(|(i, _)| g.mask >> *i & 1 == 1) {
        *c = match g.kind {
            0 => BLACK,
            1 => [1.0 - c[0], 1.0 - c[1], 1.0 - c[2]],
            2 => palette.at(along(i) + jump),
            _ => WHITE,
        };
    }
    frame
}

// Lub-dub, once every 1.2 s at normal speed.
fn heartbeat(palette: &Palette, t: f32) -> [Color; STRIP] {
    let x = t.rem_euclid(1.2);
    let beat = smooth_pulse(x - 0.1, 0.06) + 0.6 * smooth_pulse(x - 0.38, 0.07);
    let level = 0.05 + 0.95 * beat.min(1.0);
    std::array::from_fn(|i| scale(palette.at(along(i) * 0.25), level))
}

// Theater chase: every third LED lit, stepping upward with a crossfade.
fn chase(palette: &Palette, t: f32) -> [Color; STRIP] {
    let step = t * 5.0;
    std::array::from_fn(|i| {
        let distance = (i as f32 - step).rem_euclid(3.0);
        let level = (1.0 - distance.min(3.0 - distance)).max(0.0);
        scale(palette.at(along(i) * 0.5 + t * 0.05), level)
    })
}

// Default theme: dim neon gradient drifting with a slow scanline, data packets racing up the strip,
// and the odd glitch.
fn cyberpunk(palette: &Palette, s: &mut State, t: f32, dt: f32) -> [Color; STRIP] {
    let scanline = triangle(t * 0.25) * TOP;
    let mut frame: [Color; STRIP] = std::array::from_fn(|i| {
        let glow = 0.22 + 0.35 * smooth_pulse(i as f32 - scanline, 1.3);
        let shimmer = 0.9 + 0.1 * (TAU * (t * 1.7 + along(i))).sin();
        scale(palette.at(along(i) * 0.6 + t * 0.04), glow * shimmer)
    });
    fade(&mut s.trail, 9.0, dt);
    match s.packet {
        Some(position) if position > TOP + 1.0 => s.packet = None,
        Some(position) => {
            splat(&mut s.trail, position, mix(palette.at(0.6), WHITE, 0.5));
            s.packet = Some(position + dt * 16.0);
        }
        None if chance(0.5, dt) => s.packet = Some(0.0),
        None => {}
    }
    for (c, packet) in frame.iter_mut().zip(s.trail) {
        *c = add(*c, packet);
    }
    glitch(frame, palette, s, dt, 0.35)
}

pub struct Theme {
    pub name: &'static str,
    pub effect: Effect,
    pub palette: &'static Palette,
}

const fn theme(name: &'static str, effect: Effect, palette: &'static Palette) -> Theme {
    Theme { name, effect, palette }
}

// Index 0 is the default.
pub static THEMES: [Theme; 18] = [
    theme("Cyberpunk", Effect::Cyberpunk, &palettes::NEON),
    theme("Matrix rain", Effect::Rain, &palettes::MATRIX),
    theme("Glitch", Effect::Glitch, &palettes::VAPORWAVE),
    theme("Neon flow", Effect::Flow, &palettes::NEON),
    theme("Vaporwave", Effect::Breathe, &palettes::VAPORWAVE),
    theme("Scanner", Effect::Scanner, &palettes::RED),
    theme("Comet", Effect::Comet, &palettes::ICE),
    theme("Fireflies", Effect::Twinkle, &palettes::PARTY),
    theme("Fire", Effect::Fire, &palettes::HEAT),
    theme("Toxic", Effect::Rain, &palettes::TOXIC),
    theme("Rainbow", Effect::Flow, &palettes::RAINBOW),
    theme("Ocean", Effect::Flow, &palettes::OCEAN),
    theme("Lava", Effect::Flow, &palettes::LAVA),
    theme("Forest", Effect::Twinkle, &palettes::FOREST),
    theme("Heartbeat", Effect::Heartbeat, &palettes::BLOOD),
    theme("Chase", Effect::Chase, &palettes::PARTY),
    theme("Static", Effect::Static, &palettes::NEON),
    theme("Off", Effect::Off, &palettes::NEON),
];

pub struct Speed {
    pub name: &'static str,
    pub factor: f32,
}

pub static SPEEDS: [Speed; 5] = [
    Speed { name: "Chill", factor: 0.35 },
    Speed { name: "Slow", factor: 0.6 },
    Speed { name: "Normal", factor: 1.0 },
    Speed { name: "Fast", factor: 1.8 },
    Speed { name: "Overclock", factor: 3.2 },
];
pub const DEFAULT_SPEED: usize = 2;
