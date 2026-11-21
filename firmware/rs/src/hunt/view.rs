use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::ui;

const TITLE: &str = "// hunt";
const ENTER_CODE: &str = "Enter code:";

pub fn code_screen<D: DrawTarget<Color = Rgb565>>(d: &mut D, answer: &str, hint: &str) {
    lines_screen(d, &[ENTER_CODE.into()], hint);
    ui::input_box(d, answer);
}

pub fn lines_screen<D: DrawTarget<Color = Rgb565>>(d: &mut D, lines: &[String], hint: &str) {
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    ui::text_page(d, TITLE, &lines, hint);
}
