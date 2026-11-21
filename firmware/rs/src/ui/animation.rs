// Colour animation: RGB565 frames back to back (tools/mkanimation.py). Frames are fully opaque, so a
// new frame paints over the old one with no clearing and no flicker.

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;

const BYTES_PER_PIXEL: usize = 2;

pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub frames: u32,
    pub frame_ms: u32,
    pub pixels: &'static [u8],
}

impl Animation {
    // The frame to show now; every copy on screen stays in step.
    pub fn frame_now(&self) -> u32 {
        // SAFETY: esp_timer_get_time has no preconditions.
        let ms = unsafe { esp_idf_svc::sys::esp_timer_get_time() } / 1000;
        (ms / i64::from(self.frame_ms)) as u32 % self.frames
    }
}

pub fn draw_animation_rows<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    animation: &Animation,
    frame: u32,
    top_left: Point,
    rows: std::ops::Range<u32>,
) {
    let (width, height) = (animation.width, animation.height);
    let rows = rows.start.min(height)..rows.end.min(height);
    let frame_start = (frame * width * height) as usize * BYTES_PER_PIXEL;
    let first = frame_start + (rows.start * width) as usize * BYTES_PER_PIXEL;
    let last = frame_start + (rows.end * width) as usize * BYTES_PER_PIXEL;
    let colors = animation.pixels[first..last]
        .chunks_exact(BYTES_PER_PIXEL)
        .map(|p| Rgb565::from(RawU16::new(u16::from_be_bytes([p[0], p[1]]))));
    let area = Rectangle::new(top_left + Point::new(0, rows.start as i32), Size::new(width, rows.end - rows.start));
    d.fill_contiguous(&area, colors).ok();
}
