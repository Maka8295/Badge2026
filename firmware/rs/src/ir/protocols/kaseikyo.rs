use super::bits::{pack, BYTE_BITS};
use super::pulse_distance::PulseDistance;
use crate::ir::signal::{Signal, FRAME_GAP_US};

pub const CARRIER_HZ: u32 = 38_000;

const TIMING: PulseDistance = PulseDistance {
    header_mark_us: 3456,
    header_space_us: 1728,
    bit_mark_us: 432,
    zero_space_us: 432,
    one_space_us: 1296,
};
const FRAME_BYTES: usize = 6;
const NIBBLE_BITS: u32 = 4;
const NIBBLE_MASK: u32 = 0xf;
const ID_SHIFT: u32 = 24;
const ID_MASK: u32 = 0x3;
const ID_POSITION: u32 = 6;
const VENDOR_SHIFT: u32 = 8;
const VENDOR_MASK: u32 = 0xffff;

pub fn encode(address: u32, command: u32) -> Signal {
    let fields = frame_bytes(address, command).map(|byte| (byte as u32, BYTE_BITS));
    TIMING.frame(pack(&fields).lsb_first(), FRAME_GAP_US)
}

fn frame_bytes(address: u32, command: u32) -> [u8; FRAME_BYTES] {
    let id = (address >> ID_SHIFT & ID_MASK) as u8;
    let [vendor_low, vendor_high] = ((address >> VENDOR_SHIFT & VENDOR_MASK) as u16).to_le_bytes();
    let genre1 = (address >> NIBBLE_BITS & NIBBLE_MASK) as u8;
    let genre2 = (address & NIBBLE_MASK) as u8;
    let vendor_parity = nibble_parity(vendor_low ^ vendor_high);
    let byte2 = vendor_parity | genre1 << NIBBLE_BITS;
    let byte3 = genre2 | ((command & NIBBLE_MASK) as u8) << NIBBLE_BITS;
    let byte4 = id << ID_POSITION | (command >> NIBBLE_BITS) as u8;
    [vendor_low, vendor_high, byte2, byte3, byte4, byte2 ^ byte3 ^ byte4]
}

fn nibble_parity(byte: u8) -> u8 {
    (byte & NIBBLE_MASK as u8) ^ (byte >> NIBBLE_BITS)
}
