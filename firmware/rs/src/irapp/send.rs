use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::ir::{Ir, Signal};
use crate::ui;

pub fn send_codes<D: DrawTarget<Color = Rgb565>>(
    d: &mut D,
    ir: &mut Ir,
    name: &str,
    total: usize,
    mut encoded: impl FnMut(usize) -> Option<(Signal, u32)>,
    abort: &mut dyn FnMut() -> bool,
) {
    let mut skipped = 0;
    for i in 0..total {
        if abort() {
            ui::footer(d, &format!("{name}: stopped at {i}/{total}"));
            return;
        }
        ui::footer(d, &format!("{name} {}/{total}   B stop", i + 1));
        match encoded(i) {
            Some((signal, carrier_hz)) => ir.send(&signal, carrier_hz),
            None => skipped += 1,
        }
    }
    ui::footer(d, &done_message(name, total - skipped, skipped));
}

fn done_message(name: &str, sent: usize, skipped: usize) -> String {
    match skipped {
        0 => format!("{name}: sent {sent} codes"),
        _ => format!("{name}: sent {sent} codes, {skipped} skipped"),
    }
}
