use embedded_graphics::pixelcolor::Rgb565;

const RED_BLUE_DROPPED_BITS: u32 = 8 - 5;
const GREEN_DROPPED_BITS: u32 = 8 - 6;

const fn from_hex(rgb: u32) -> Rgb565 {
    let [_, red, green, blue] = rgb.to_be_bytes();
    Rgb565::new(red >> RED_BLUE_DROPPED_BITS, green >> GREEN_DROPPED_BITS, blue >> RED_BLUE_DROPPED_BITS)
}

pub const BG: Rgb565 = from_hex(0x090909);
pub const FG: Rgb565 = from_hex(0xe6e9eb);
pub const MUTED: Rgb565 = from_hex(0xc5c5c5);
pub const SURFACE: Rgb565 = from_hex(0x212121);
pub const BORDER: Rgb565 = from_hex(0x333333);
pub const GRID: Rgb565 = from_hex(0x161616);
pub const ACCENT: Rgb565 = from_hex(0xed252d);
