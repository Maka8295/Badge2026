use std::ops::RangeInclusive;

use super::commands::PREAMBLE_SYMBOLS;

#[derive(Clone, Copy)]
pub struct RadioConfig {
    pub freq_khz: u32,
    pub spreading_factor: u8,
    pub bandwidth_index: u8,
    pub coding_rate: u8,
    pub power_dbm: i8,
    pub sync: u8,
}

// Japan 920 MHz band (ARIB STD-T108 Part 2, 20 mW): channels 24..38 on a 200 kHz grid. These are the only
// channels allowing 4 s packets; 923.6 MHz and up cap packets below 400 ms. One channel per packet, so the
// bandwidth must fit inside 200 kHz.
pub const BW_KHZ: [f32; 2] = [62.5, 125.0];
const BW_CODE: [u8; 2] = [0x03, 0x04];
const BW_CODE_CARRIER_SENSE: u8 = 0x05;

pub const FREQ_KHZ_RANGE: RangeInclusive<i32> = 920_600..=923_400;
pub const CHANNEL_STEP_KHZ: i32 = 200;
pub const SF_RANGE: RangeInclusive<i32> = 7..=12;
pub const CR_RANGE: RangeInclusive<i32> = 5..=8;
pub const POWER_DBM_RANGE: RangeInclusive<i32> = -9..=13;
// The rule is "less than 4 s". The margin covers clock error and the PA ramps.
pub const MAX_TX_MS: f32 = 3900.0;

const CODING_RATE_DENOMINATOR_OFFSET: u8 = 4;
const LOW_DATA_RATE_MIN_SYMBOL_MS: f32 = 16.0;
const PREAMBLE_EXTRA_SYMBOLS: f32 = 4.25;
const PAYLOAD_FIXED_SYMBOLS: f32 = 8.0;
const PAYLOAD_BITS_OFFSET: f32 = 44.0;
const BITS_PER_BYTE: f32 = 8.0;

const HZ_PER_KHZ: u64 = 1000;

const CRYSTAL_HZ: u64 = 32_000_000;
const FREQ_STEP_SHIFT: u32 = 25;


impl RadioConfig {
    pub fn bandwidth_khz(&self) -> f32 {
        BW_KHZ[self.bandwidth_index as usize]
    }

    pub(super) fn modulation_params(&self) -> [u8; 4] {
        let cr_code = self.coding_rate - CODING_RATE_DENOMINATOR_OFFSET;
        [self.spreading_factor, BW_CODE[self.bandwidth_index as usize], cr_code, self.low_data_rate_optimize() as u8]
    }

    // Carrier sense must cover the whole 200 kHz channel, wider than the 125 kHz receive filter.
    pub(super) fn carrier_sense_params(&self) -> [u8; 4] {
        let mut params = self.modulation_params();
        params[1] = BW_CODE_CARRIER_SENSE;
        params
    }

    pub fn is_valid(&self) -> bool {
        let freq = self.freq_khz as i32;
        FREQ_KHZ_RANGE.contains(&freq)
            && (freq - FREQ_KHZ_RANGE.start()) % CHANNEL_STEP_KHZ == 0
            && SF_RANGE.contains(&(self.spreading_factor as i32))
            && (self.bandwidth_index as usize) < BW_KHZ.len()
            && CR_RANGE.contains(&(self.coding_rate as i32))
            && POWER_DBM_RANGE.contains(&(self.power_dbm as i32))
    }

    // Semtech LoRa time-on-air formula, explicit header, CRC on.
    pub fn time_on_air_ms(&self, payload_len: usize) -> f32 {
        let sf = self.spreading_factor as f32;
        let bits = BITS_PER_BYTE * payload_len as f32 - 4.0 * sf + PAYLOAD_BITS_OFFSET;
        let bits_per_symbol_group = 4.0 * (sf - 2.0 * self.low_data_rate_optimize() as u8 as f32);
        let payload_symbols = PAYLOAD_FIXED_SYMBOLS + ((bits / bits_per_symbol_group).ceil() * self.coding_rate as f32).max(0.0);
        (PREAMBLE_SYMBOLS as f32 + PREAMBLE_EXTRA_SYMBOLS + payload_symbols) * self.symbol_ms()
    }

    fn symbol_ms(&self) -> f32 {
        (1u32 << self.spreading_factor) as f32 / self.bandwidth_khz()
    }

    fn low_data_rate_optimize(&self) -> bool {
        self.symbol_ms() >= LOW_DATA_RATE_MIN_SYMBOL_MS
    }

    pub(super) fn frequency_steps(&self) -> u32 {
        let hz = self.freq_khz as u64 * HZ_PER_KHZ;
        ((hz << FREQ_STEP_SHIFT) / CRYSTAL_HZ) as u32
    }

}
