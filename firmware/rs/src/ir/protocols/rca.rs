use super::bits::{pack, BYTE_BITS};
use super::pulse_distance::PulseDistance;
use crate::ir::signal::{Signal, FRAME_GAP_US};

pub const CARRIER_HZ: u32 = 38_000;

const TIMING: PulseDistance = PulseDistance {
    header_mark_us: 4000,
    header_space_us: 4000,
    bit_mark_us: 500,
    zero_space_us: 1000,
    one_space_us: 2000,
};
const ADDRESS_BITS: usize = 4;

pub fn encode(address: u32, command: u32) -> Signal {
    let payload = pack(&[
        (address, ADDRESS_BITS),
        (command, BYTE_BITS),
        (!address, ADDRESS_BITS),
        (!command, BYTE_BITS),
    ]);
    TIMING.frame(payload.lsb_first(), FRAME_GAP_US)
}
