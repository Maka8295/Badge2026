// Gradient palettes in the style of FastLED's CRGBPalette16: a few colour stops, blended smoothly.
// Positions are 0.0..1.0 and wrap around, so a palette can scroll along the strip forever.

use super::Rgb;

pub type Color = [f32; 3];

pub struct Palette {
    pub name: &'static str,
    stops: &'static [Rgb],
}

impl Palette {
    pub fn at(&self, position: f32) -> Color {
        let count = self.stops.len();
        let scaled = position.rem_euclid(1.0) * count as f32;
        let index = scaled as usize % count;
        let blend = scaled.fract();
        mix(to_color(self.stops[index]), to_color(self.stops[(index + 1) % count]), blend)
    }

    // No wrap: 0.0 is the first stop and 1.0 the last. Used for heat maps.
    pub fn along(&self, position: f32) -> Color {
        let last = self.stops.len() - 1;
        let scaled = position.clamp(0.0, 1.0) * last as f32;
        let index = (scaled as usize).min(last.saturating_sub(1));
        mix(to_color(self.stops[index]), to_color(self.stops[(index + 1).min(last)]), scaled - index as f32)
    }
}

pub fn to_color((r, g, b): Rgb) -> Color {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

pub fn scale(c: Color, by: f32) -> Color {
    [c[0] * by, c[1] * by, c[2] * by]
}

pub const BLACK: Color = [0.0; 3];
pub const WHITE: Color = [1.0; 3];

const fn palette(name: &'static str, stops: &'static [Rgb]) -> Palette {
    Palette { name, stops }
}

pub static NEON: Palette = palette("Neon", &[(255, 0, 140), (140, 0, 255), (0, 170, 255), (0, 255, 210), (140, 0, 255)]);
pub static MATRIX: Palette = palette("Matrix", &[(0, 255, 0), (0, 200, 4), (0, 255, 12), (0, 130, 0)]);
pub static VAPORWAVE: Palette = palette("Vaporwave", &[(255, 60, 200), (150, 40, 255), (0, 220, 255), (255, 120, 180)]);
pub static RAINBOW: Palette = palette(
    "Rainbow",
    &[(255, 0, 0), (255, 110, 0), (255, 230, 0), (0, 255, 0), (0, 255, 230), (0, 40, 255), (140, 0, 255), (255, 0, 140)],
);
pub static PARTY: Palette = palette(
    "Party",
    &[(85, 0, 171), (181, 0, 75), (232, 23, 0), (171, 85, 0), (171, 171, 0), (221, 34, 0), (143, 0, 113), (0, 7, 249)],
);
pub static OCEAN: Palette = palette("Ocean", &[(0, 0, 120), (0, 30, 255), (0, 140, 255), (0, 255, 200), (0, 60, 160)]);
pub static LAVA: Palette = palette("Lava", &[(60, 0, 0), (160, 0, 0), (255, 20, 0), (255, 100, 0), (255, 170, 0), (140, 0, 0)]);
pub static FOREST: Palette = palette("Forest", &[(0, 100, 0), (34, 139, 34), (110, 160, 20), (0, 160, 60), (60, 120, 0)]);
pub static ICE: Palette = palette("Ice", &[(0, 40, 255), (0, 160, 255), (170, 220, 255), (0, 90, 255)]);
pub static SUNSET: Palette = palette("Sunset", &[(255, 40, 0), (255, 110, 0), (255, 0, 90), (120, 0, 160)]);
pub static TOXIC: Palette = palette("Toxic", &[(80, 255, 0), (200, 255, 0), (0, 255, 60), (140, 255, 0)]);
pub static BLOOD: Palette = palette("Blood", &[(255, 0, 0), (110, 0, 0), (255, 10, 40), (180, 0, 0)]);
// Black body: dark red to yellow to white. Read with `along`, not `at`.
pub static HEAT: Palette = palette("Heat", &[(0, 0, 0), (120, 0, 0), (255, 0, 0), (255, 90, 0), (255, 200, 0), (255, 255, 160)]);
pub static WHITE_ONLY: Palette = palette("White", &[(255, 255, 255)]);
pub static RED: Palette = palette("Red", &[(255, 0, 0)]);
pub static ORANGE: Palette = palette("Orange", &[(255, 90, 0)]);
pub static YELLOW: Palette = palette("Yellow", &[(255, 200, 0)]);
pub static GREEN: Palette = palette("Green", &[(0, 255, 0)]);
pub static CYAN: Palette = palette("Cyan", &[(0, 255, 255)]);
pub static BLUE: Palette = palette("Blue", &[(0, 0, 255)]);
pub static PURPLE: Palette = palette("Purple", &[(140, 0, 255)]);
pub static MAGENTA: Palette = palette("Magenta", &[(255, 0, 200)]);
pub static PINK: Palette = palette("Pink", &[(255, 60, 120)]);

// Choices in the LEDs app "Colour" row. Index 0 means "use the theme's own palette".
pub static CHOICES: [&Palette; 23] = [
    &NEON, &MATRIX, &VAPORWAVE, &RAINBOW, &PARTY, &OCEAN, &LAVA, &FOREST, &ICE, &SUNSET, &TOXIC, &BLOOD, &HEAT,
    &WHITE_ONLY, &RED, &ORANGE, &YELLOW, &GREEN, &CYAN, &BLUE, &PURPLE, &MAGENTA, &PINK,
];
