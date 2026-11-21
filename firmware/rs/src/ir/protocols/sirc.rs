use super::bits::pack;
use crate::ir::signal::Signal;

pub const CARRIER_HZ: u32 = 40_000;

const START_MARK_US: u16 = 2400;
const ONE_MARK_US: u16 = 1200;
const ZERO_MARK_US: u16 = 600;
const SPACE_US: u16 = 600;
const FRAME_PERIOD_US: u32 = 45_000;
const REPEATS: usize = 3;
const COMMAND_BITS: usize = 7;
const SIRC12_ADDRESS_BITS: usize = 5;
const SIRC15_ADDRESS_BITS: usize = 8;
const SIRC20_ADDRESS_BITS: usize = 13;

pub fn sirc12(address: u32, command: u32) -> Signal {
    encode(address, command, SIRC12_ADDRESS_BITS)
}

pub fn sirc15(address: u32, command: u32) -> Signal {
    encode(address, command, SIRC15_ADDRESS_BITS)
}

pub fn sirc20(address: u32, command: u32) -> Signal {
    encode(address, command, SIRC20_ADDRESS_BITS)
}

fn encode(address: u32, command: u32, address_bits: usize) -> Signal {
    let payload = pack(&[(command, COMMAND_BITS), (address, address_bits)]);
    let mut frame = vec![(START_MARK_US, SPACE_US)];
    frame.extend(payload.lsb_first().map(|bit| (mark_for(bit), SPACE_US)));
    stretch_to_frame_period(&mut frame);
    frame.repeat(REPEATS)
}

fn mark_for(bit: bool) -> u16 {
    if bit {
        ONE_MARK_US
    } else {
        ZERO_MARK_US
    }
}

fn stretch_to_frame_period(frame: &mut Signal) {
    let length: u32 = frame.iter().map(|&(mark, space)| mark as u32 + space as u32).sum();
    frame.last_mut().unwrap().1 += (FRAME_PERIOD_US - length) as u16;
}
