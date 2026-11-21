use super::animation::Animation;
use super::bitmap::Bitmap;

pub static TENGU_LOGO: Bitmap = Bitmap { width: 127, height: 128, bits: include_bytes!("../../images/tengu.raw") };
// Sir Jaffalot, from the back silkscreen of shuriken2 (tools/images/jaffalot*.png).
pub static JAFFALOT: Bitmap = Bitmap { width: 248, height: 166, bits: include_bytes!("../../images/jaffalot.raw") };
pub static JAFFALOT_TITLE: Bitmap =
    Bitmap { width: 200, height: 37, bits: include_bytes!("../../images/jaffalot_title.raw") };
// Chat emotes, 72 px tall (4 chat lines).
pub static JAFFALOT_EMOTE: Bitmap =
    Bitmap { width: 108, height: 72, bits: include_bytes!("../../images/jaffalot_emote.raw") };
pub static FIH_EMOTE: Bitmap = Bitmap { width: 108, height: 72, bits: include_bytes!("../../images/fih_emote.raw") };
// Party kakapo, from tools/images/kakapo.gif: 10 frames of 50 ms.
pub static KAKAPO_EMOTE: Animation =
    Animation { width: 96, height: 72, frames: 10, frame_ms: 50, pixels: include_bytes!("../../images/kakapo.raw") };
pub static TENGU_EMOTE: Bitmap = Bitmap { width: 72, height: 72, bits: include_bytes!("../../images/tengu_emote.raw") };
pub static TENGU_ICON: Bitmap = Bitmap { width: 24, height: 24, bits: include_bytes!("../../images/tengu_24.raw") };
