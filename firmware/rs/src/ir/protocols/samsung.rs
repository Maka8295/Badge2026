use super::bits::{pack, BYTE_BITS};
use super::pulse_distance::PulseDistance;
use crate::ir::signal::{Signal, FRAME_GAP_US};

pub const CARRIER_HZ: u32 = 38_000;

pub const TIMING: PulseDistance = PulseDistance {
    header_mark_us: 4500,
    header_space_us: 4500,
    bit_mark_us: 550,
    zero_space_us: 550,
    one_space_us: 1650,
};

pub fn encode(address: u32, command: u32) -> Signal {
    let payload = pack(&[(address, BYTE_BITS), (address, BYTE_BITS), (command, BYTE_BITS), (!command, BYTE_BITS)]);
    TIMING.frame(payload.lsb_first(), FRAME_GAP_US)
}
