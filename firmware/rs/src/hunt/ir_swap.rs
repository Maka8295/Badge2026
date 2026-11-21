use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use esp_idf_svc::sys::esp_random;

use crate::chat::NICK_MAX;
use crate::ir::Ir;

const CARRIER_HZ: u32 = 38_000;
const HEADER_MARK_US: u16 = 3000;
const HEADER_SPACE_US: u16 = 3000;
const BIT_MARK_US: u16 = 500;
const SPACE_STEP_US: u16 = 500;
const HEADER_MIN_US: u16 = 2500;
const END_SPACE_US: u16 = 20_000;
const MAGIC: u8 = 0xB7;
const BITS_PER_PULSE: usize = 2;
const PULSES_PER_BYTE: usize = 4;
const PULSE_VALUE_MASK: u8 = 0b11;
const SEND_EVERY_MIN_MS: u32 = 700;
const SEND_JITTER_MS: u32 = 900;
const SEEN_MAX: usize = 8;

pub struct IrSwap {
    own_name: String,
    seen: BTreeSet<String>,
    next_send: Instant,
}

impl IrSwap {
    pub fn new(own_name: &str) -> Self {
        IrSwap { own_name: own_name.into(), seen: BTreeSet::new(), next_send: Instant::now() }
    }

    pub fn seen(&self) -> &BTreeSet<String> {
        &self.seen
    }

    pub fn tick(&mut self, ir: &mut Ir) -> bool {
        if Instant::now() >= self.next_send {
            ir.send(&encode(&self.own_name), CARRIER_HZ);
            self.next_send = Instant::now() + jitter();
            return false;
        }
        let Some(name) = ir.try_receive().and_then(|signal| decode(&signal)) else { return false };
        name != self.own_name && self.seen.len() < SEEN_MAX && self.seen.insert(name)
    }
}

fn jitter() -> Duration {
    // SAFETY: esp_random has no preconditions.
    let random = unsafe { esp_random() };
    Duration::from_millis((SEND_EVERY_MIN_MS + random % SEND_JITTER_MS) as u64)
}

fn encode(name: &str) -> Vec<(u16, u16)> {
    let name = &name.as_bytes()[..name.len().min(NICK_MAX)];
    let mut bytes = vec![MAGIC, name.len() as u8];
    bytes.extend_from_slice(name);
    bytes.push(checksum(&bytes));
    let mut signal = vec![(HEADER_MARK_US, HEADER_SPACE_US)];
    signal.extend(bytes.iter().flat_map(|&b| (0..PULSES_PER_BYTE).map(move |i| pulse(b >> (i * BITS_PER_PULSE)))));
    signal.push((BIT_MARK_US, END_SPACE_US));
    signal
}

fn pulse(value: u8) -> (u16, u16) {
    (BIT_MARK_US, SPACE_STEP_US * (1 + (value & PULSE_VALUE_MASK) as u16))
}

fn pulse_value(space_us: u16) -> u8 {
    let steps = (space_us + SPACE_STEP_US / 2) / SPACE_STEP_US;
    (steps.clamp(1, 4) - 1) as u8
}

fn decode(signal: &[(u16, u16)]) -> Option<String> {
    let (&(mark, space), bits) = signal.split_first()?;
    if mark < HEADER_MIN_US || space < HEADER_MIN_US {
        return None;
    }
    let bytes: Vec<u8> = bits.chunks_exact(PULSES_PER_BYTE).map(byte_from_pulses).collect();
    let (&magic, rest) = bytes.split_first()?;
    let (&len, rest) = rest.split_first()?;
    let name = rest.get(..len as usize)?;
    let sum = *rest.get(len as usize)?;
    let valid = magic == MAGIC && sum == checksum(&bytes[..2 + len as usize]);
    valid.then(|| String::from_utf8_lossy(name).into_owned())
}

fn byte_from_pulses(pulses: &[(u16, u16)]) -> u8 {
    pulses.iter().enumerate().fold(0, |b, (i, &(_, space))| b | pulse_value(space) << (i * BITS_PER_PULSE))
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, b| sum.wrapping_add(*b))
}
