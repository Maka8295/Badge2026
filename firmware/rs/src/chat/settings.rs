use std::ops::RangeInclusive;

use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use crate::sx126x::{RadioConfig, BW_KHZ, CHANNEL_STEP_KHZ, CR_RANGE, FREQ_KHZ_RANGE, POWER_DBM_RANGE, SF_RANGE};

const KEY_FREQ: &str = "freq";
const KEY_SF: &str = "sf";
const KEY_BW: &str = "bw";
const KEY_CR: &str = "cr";
const KEY_POWER: &str = "pwr";
const KEY_SYNC: &str = "sync";

const DEFAULT_FREQ_KHZ: u32 = 923_200;
const DEFAULT_SF: u32 = 9;
const DEFAULT_BANDWIDTH_INDEX: u32 = 1;
const DEFAULT_CR: u32 = 5;
const DEFAULT_POWER_DBM: i32 = 10;
const DEFAULT_SYNC: u32 = 0x12;

const FREQ_STEP_KHZ: i32 = CHANNEL_STEP_KHZ;
const FREQ_BIG_STEP_KHZ: i32 = 5 * CHANNEL_STEP_KHZ;
const SYNC_STEP: i32 = 1;
const SYNC_BIG_STEP: i32 = 16;
const BANDWIDTH_INDEX_RANGE: RangeInclusive<i32> = 0..=BW_KHZ.len() as i32 - 1;

const KHZ_PER_MHZ: u32 = 1000;

#[derive(Clone, Copy, PartialEq)]
pub enum Field {
    Frequency,
    SpreadingFactor,
    Bandwidth,
    CodingRate,
    TxPower,
    SyncWord,
    TxPassword,
}

pub const FIELDS: [Field; 7] = [
    Field::Frequency,
    Field::SpreadingFactor,
    Field::Bandwidth,
    Field::CodingRate,
    Field::TxPower,
    Field::SyncWord,
    Field::TxPassword,
];

impl Field {
    pub fn label(self) -> &'static str {
        match self {
            Field::Frequency => "Freq MHz",
            Field::SpreadingFactor => "Spreading",
            Field::Bandwidth => "Bandwidth",
            Field::CodingRate => "Coding",
            Field::TxPower => "TX power",
            Field::SyncWord => "Sync word",
            Field::TxPassword => "TX password",
        }
    }
}

pub struct Config {
    pub nick: String,
    pub radio: RadioConfig,
}

impl Config {
    pub fn load(nvs: &EspNvs<NvsDefault>, board_name: String) -> Self {
        let u32_or = |key: &str, default: u32| nvs.get_u32(key).ok().flatten().unwrap_or(default);
        let saved = RadioConfig {
            freq_khz: u32_or(KEY_FREQ, DEFAULT_FREQ_KHZ),
            spreading_factor: u32_or(KEY_SF, DEFAULT_SF) as u8,
            bandwidth_index: u32_or(KEY_BW, DEFAULT_BANDWIDTH_INDEX) as u8,
            coding_rate: u32_or(KEY_CR, DEFAULT_CR) as u8,
            power_dbm: u32_or(KEY_POWER, DEFAULT_POWER_DBM as u32) as i32 as i8,
            sync: u32_or(KEY_SYNC, DEFAULT_SYNC) as u8,
        };
        // Settings saved by older firmware (433 MHz, wide bandwidth, high power) are not legal in Japan.
        let radio = if saved.is_valid() {
            saved
        } else {
            log::warn!("saved radio config outside the Japan 920 MHz rules, using defaults");
            default_radio(saved.sync)
        };
        Config { nick: board_name, radio }
    }

    pub fn save(&self, nvs: &EspNvs<NvsDefault>) {
        let r = &self.radio;
        let result = nvs
            .set_u32(KEY_FREQ, r.freq_khz)
            .and(nvs.set_u32(KEY_SF, r.spreading_factor as u32))
            .and(nvs.set_u32(KEY_BW, r.bandwidth_index as u32))
            .and(nvs.set_u32(KEY_CR, r.coding_rate as u32))
            .and(nvs.set_u32(KEY_POWER, r.power_dbm as i32 as u32))
            .and(nvs.set_u32(KEY_SYNC, r.sync as u32));
        if let Err(e) = result {
            log::error!("saving chat config: {e}");
        }
    }

    pub fn summary(&self) -> String {
        let r = &self.radio;
        format!("{} SF{}/{}k {}dBm", format_mhz(r.freq_khz), r.spreading_factor, r.bandwidth_khz(), r.power_dbm)
    }

    pub fn value(&self, field: Field) -> String {
        let r = &self.radio;
        match field {
            Field::Frequency => format_mhz(r.freq_khz),
            Field::SpreadingFactor => format!("SF{}", r.spreading_factor),
            Field::Bandwidth => format!("{} kHz", r.bandwidth_khz()),
            Field::CodingRate => format!("4/{}", r.coding_rate),
            Field::TxPower => format!("{} dBm", r.power_dbm),
            Field::SyncWord => format!("0x{:02X}", r.sync),
            Field::TxPassword => "OK to change".into(),
        }
    }

    pub fn adjust(&mut self, field: Field, direction: i32, big_step: bool) {
        let r = &mut self.radio;
        match field {
            Field::Frequency => {
                let step = if big_step { FREQ_BIG_STEP_KHZ } else { FREQ_STEP_KHZ };
                r.freq_khz = step_within(r.freq_khz as i32, direction * step, FREQ_KHZ_RANGE) as u32;
            }
            Field::SpreadingFactor => r.spreading_factor = step_within(r.spreading_factor as i32, direction, SF_RANGE) as u8,
            Field::Bandwidth => r.bandwidth_index = step_within(r.bandwidth_index as i32, direction, BANDWIDTH_INDEX_RANGE) as u8,
            Field::CodingRate => r.coding_rate = step_within(r.coding_rate as i32, direction, CR_RANGE) as u8,
            Field::TxPower => r.power_dbm = step_within(r.power_dbm as i32, direction, POWER_DBM_RANGE) as i8,
            Field::SyncWord => {
                let step = if big_step { SYNC_BIG_STEP } else { SYNC_STEP };
                r.sync = r.sync.wrapping_add((direction * step) as u8);
            }
            Field::TxPassword => {}
        }
    }
}

fn default_radio(sync: u8) -> RadioConfig {
    RadioConfig {
        freq_khz: DEFAULT_FREQ_KHZ,
        spreading_factor: DEFAULT_SF as u8,
        bandwidth_index: DEFAULT_BANDWIDTH_INDEX as u8,
        coding_rate: DEFAULT_CR as u8,
        power_dbm: DEFAULT_POWER_DBM as i8,
        sync,
    }
}

fn step_within(value: i32, delta: i32, range: RangeInclusive<i32>) -> i32 {
    (value + delta).clamp(*range.start(), *range.end())
}

fn format_mhz(khz: u32) -> String {
    format!("{}.{:03}", khz / KHZ_PER_MHZ, khz % KHZ_PER_MHZ)
}
