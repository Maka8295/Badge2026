use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

const BITS_PER_BYTE: u32 = 8;
const LEFTMOST_BIT: u8 = 0x80;

pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub bits: &'static [u8],
}

impl Bitmap {
    pub fn is_set(&self, x: u32, y: u32) -> bool {
        let bytes_per_row = self.width.div_ceil(BITS_PER_BYTE);
        let byte = self.bits[(y * bytes_per_row + x / BITS_PER_BYTE) as usize];
        byte & (LEFTMOST_BIT >> (x % BITS_PER_BYTE)) != 0
    }

    fn set_runs(&self, y: u32) -> impl Iterator<Item = (u32, u32)> + '_ {
        let mut x = 0;
        std::iter::from_fn(move || {
            while x < self.width && !self.is_set(x, y) {
                x += 1;
            }
            let start = x;
            while x < self.width && self.is_set(x, y) {
                x += 1;
            }
            (start < x).then_some((start, x - start))
        })
    }
}

pub fn draw_bitmap<D: DrawTarget<Color = Rgb565>>(d: &mut D, bitmap: &Bitmap, top_left: Point, color: Rgb565) {
    draw_bitmap_rows(d, bitmap, top_left, color, 0..bitmap.height);
}

pub fn draw_bitmap_rows<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    bitmap: &Bitmap,
    top_left: Point,
    color: Rgb565,
    rows: std::ops::Range<u32>,
) {
    for y in rows {
        for (x, length) in bitmap.set_runs(y) {
            let run = Rectangle::new(top_left + Point::new(x as i32, y as i32), Size::new(length, 1));
            d.fill_solid(&run, color).ok();
        }
    }
}
