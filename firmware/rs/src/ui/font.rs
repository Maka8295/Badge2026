use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::mapping::ASCII;
use embedded_graphics::mono_font::{DecorationDimensions, MonoFont};
use embedded_graphics::prelude::Size;

const GLYPHS_PER_ATLAS_ROW: u32 = 16;

struct Glyph {
    width: u32,
    height: u32,
    baseline: u32,
}

const fn font(atlas: &'static [u8], glyph: Glyph) -> MonoFont<'static> {
    MonoFont {
        image: ImageRaw::new(atlas, GLYPHS_PER_ATLAS_ROW * glyph.width),
        glyph_mapping: &ASCII,
        character_size: Size::new(glyph.width, glyph.height),
        character_spacing: 0,
        baseline: glyph.baseline,
        strikethrough: DecorationDimensions::default_strikethrough(glyph.height),
        underline: DecorationDimensions::default_underline(glyph.height),
    }
}

pub const SMALL: MonoFont = font(
    include_bytes!("../../fonts/jbm12.raw"),
    Glyph { width: 7, height: 17, baseline: 12 },
);
pub const BODY: MonoFont = font(
    include_bytes!("../../fonts/jbm16.raw"),
    Glyph { width: 10, height: 22, baseline: 16 },
);
pub const TITLE: MonoFont = font(
    include_bytes!("../../fonts/jbm24b.raw"),
    Glyph { width: 14, height: 33, baseline: 24 },
);

pub fn text_width(s: &str, font: &MonoFont) -> i32 {
    s.len() as i32 * font.character_size.width as i32
}
