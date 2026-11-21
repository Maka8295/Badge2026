use super::bits::Bits;
use super::nec::standard_payload;
use super::pulse_distance::PulseDistance;
use crate::ir::signal::Signal;

pub const CARRIER_HZ: u32 = 40_000;

const TIMING: PulseDistance = PulseDistance {
    header_mark_us: 8500,
    header_space_us: 4225,
    bit_mark_us: 500,
    zero_space_us: 500,
    one_space_us: 1500,
};
const BIT_COUNT: usize = 33;
const GAP_US: u16 = 26_000;
const REPEATS: usize = 2;

pub fn encode(address: u32, command: u32) -> Signal {
    let payload = Bits { count: BIT_COUNT, ..standard_payload(address, command) };
    TIMING.frame(payload.lsb_first(), GAP_US).repeat(REPEATS)
}
