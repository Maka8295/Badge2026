use crate::ir::signal::{Signal, FRAME_GAP_US};

pub type Level = (bool, u16);

pub fn one_as_space_then_mark(bits: impl Iterator<Item = bool>, half_bit_us: u16) -> impl Iterator<Item = Level> {
    bits.flat_map(move |bit| [(!bit, half_bit_us), (bit, half_bit_us)])
}

pub fn one_as_mark_then_space(bits: impl Iterator<Item = bool>, half_bit_us: u16) -> impl Iterator<Item = Level> {
    bits.flat_map(move |bit| [(bit, half_bit_us), (!bit, half_bit_us)])
}

pub fn levels_to_signal(levels: &[Level]) -> Signal {
    let mut signal = Signal::new();
    for &(on, us) in levels.iter().skip_while(|(on, _)| !on) {
        add_level(&mut signal, on, us);
    }
    if let Some(last) = signal.last_mut() {
        last.1 = FRAME_GAP_US;
    }
    signal
}

fn add_level(signal: &mut Signal, on: bool, us: u16) {
    match (on, signal.last_mut()) {
        (true, Some(last)) if last.1 == 0 => last.0 += us,
        (true, _) => signal.push((us, 0)),
        (false, Some(last)) => last.1 += us,
        (false, None) => {}
    }
}
