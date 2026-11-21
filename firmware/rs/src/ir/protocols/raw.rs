use crate::ir::signal::{Signal, FRAME_GAP_US};

pub fn encode(durations: &[u16]) -> Signal {
    let mut signal: Signal = durations
        .chunks(2)
        .map(|pair| (pair[0], pair.get(1).copied().unwrap_or(FRAME_GAP_US)))
        .collect();
    if let Some(last) = signal.last_mut() {
        last.1 = last.1.max(FRAME_GAP_US);
    }
    signal
}
