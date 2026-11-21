use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

const BYTES_PER_PIXEL: usize = 2;
const TRANSPARENT: u16 = 0xF81F;

pub struct ColorImage {
    pub width: u32,
    pub height: u32,
    pub pixels: &'static [u8],
}

impl ColorImage {
    fn pixel(&self, x: u32, y: u32) -> u16 {
        let i = (y * self.width + x) as usize * BYTES_PER_PIXEL;
        u16::from_be_bytes([self.pixels[i], self.pixels[i + 1]])
    }

    fn opaque_runs(&self, y: u32) -> impl Iterator<Item = (u32, u32)> + '_ {
        let mut x = 0;
        std::iter::from_fn(move || {
            while x < self.width && self.pixel(x, y) == TRANSPARENT {
                x += 1;
            }
            let start = x;
            while x < self.width && self.pixel(x, y) != TRANSPARENT {
                x += 1;
            }
            (start < x).then_some((start, x - start))
        })
    }
}

pub fn draw_color_image<D: DrawTarget<Color = Rgb565>>(d: &mut D, image: &ColorImage, top_left: Point) {
    for y in 0..image.height {
        for (x, length) in image.opaque_runs(y) {
            let run = Rectangle::new(top_left + Point::new(x as i32, y as i32), Size::new(length, 1));
            let colors = (x..x + length).map(|px| Rgb565::from(RawU16::new(image.pixel(px, y))));
            d.fill_contiguous(&run, colors).ok();
        }
    }
}
