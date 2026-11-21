use super::database::{Code, Protocol};

pub const NAME_KEY: &str = "name";

const UNMODULATED_HZ: u32 = 0;
const CARRIER_RANGE_HZ: std::ops::RangeInclusive<u32> = 10_000..=500_000;
const SEPARATOR: char = '#';
const FIELD_SEPARATOR: char = ':';
const HEX_RADIX: u32 = 16;
const BITS_PER_BYTE: usize = 8;
const MAX_VALUE_BYTES: usize = 4;
const PARSED_TYPE: &str = "parsed";
const RAW_TYPE: &str = "raw";

const PROTOCOL_NAMES: [(&str, Protocol); 13] = [
    ("NEC", Protocol::Nec),
    ("NECext", Protocol::NecExt),
    ("NEC42", Protocol::Nec42),
    ("Samsung32", Protocol::Samsung32),
    ("SIRC", Protocol::Sirc12),
    ("SIRC15", Protocol::Sirc15),
    ("SIRC20", Protocol::Sirc20),
    ("RC5", Protocol::Rc5),
    ("RC5X", Protocol::Rc5x),
    ("RC6", Protocol::Rc6),
    ("Kaseikyo", Protocol::Kaseikyo),
    ("RCA", Protocol::Rca),
    ("Pioneer", Protocol::Pioneer),
];

pub enum Record {
    Parsed(Protocol, u32, u32),
    Raw(u32, Vec<u16>),
}

impl Record {
    pub fn code(&self) -> Code<'_> {
        match self {
            Record::Parsed(protocol, address, command) => Code::Parsed(*protocol, *address, *command),
            Record::Raw(carrier_hz, durations) => Code::Raw(*carrier_hz, durations),
        }
    }
}

#[derive(Default)]
struct Fields {
    kind: String,
    protocol: String,
    address: String,
    command: String,
    frequency: String,
    data: String,
}

impl Fields {
    fn set(&mut self, key: &str, value: &str) {
        let slot = match key {
            "type" => &mut self.kind,
            "protocol" => &mut self.protocol,
            "address" => &mut self.address,
            "command" => &mut self.command,
            "frequency" => &mut self.frequency,
            "data" => &mut self.data,
            _ => return,
        };
        *slot = value.to_string();
    }

    fn record(&self) -> Option<Record> {
        match self.kind.as_str() {
            PARSED_TYPE => self.parsed(),
            RAW_TYPE => self.raw(),
            _ => None,
        }
    }

    fn parsed(&self) -> Option<Record> {
        let protocol = protocol_from_name(&self.protocol)?;
        Some(Record::Parsed(
            protocol,
            little_endian_hex(&self.address)?,
            little_endian_hex(&self.command)?,
        ))
    }

    fn raw(&self) -> Option<Record> {
        let carrier_hz = self.frequency.parse::<u32>().ok()?;
        let supported = carrier_hz == UNMODULATED_HZ || CARRIER_RANGE_HZ.contains(&carrier_hz);
        supported.then_some(())?;
        Some(Record::Raw(carrier_hz, durations(&self.data)?))
    }
}

pub fn is_separator(line: &str) -> bool {
    line.trim_start().starts_with(SEPARATOR)
}

pub fn field(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(FIELD_SEPARATOR)?;
    Some((key.trim(), value.trim()))
}

pub fn parse_record(lines: impl Iterator<Item = impl AsRef<str>>) -> Option<Record> {
    let mut fields = Fields::default();
    for line in lines {
        let line = line.as_ref();
        if is_separator(line) {
            break;
        }
        if let Some((key, value)) = field(line) {
            fields.set(key, value);
        }
    }
    fields.record()
}

fn protocol_from_name(name: &str) -> Option<Protocol> {
    PROTOCOL_NAMES.iter().find(|(known, _)| *known == name).map(|&(_, protocol)| protocol)
}

fn little_endian_hex(text: &str) -> Option<u32> {
    let bytes: Vec<u8> = text
        .split_whitespace()
        .map(|byte| u8::from_str_radix(byte, HEX_RADIX).ok())
        .collect::<Option<_>>()?;
    if bytes.len() > MAX_VALUE_BYTES {
        return None;
    }
    Some(bytes.iter().enumerate().fold(0, |value, (i, &byte)| value | (byte as u32) << (i * BITS_PER_BYTE)))
}

fn durations(text: &str) -> Option<Vec<u16>> {
    text.split_whitespace()
        .map(|value| value.parse::<u32>().ok().map(|us| u16::try_from(us).unwrap_or(u16::MAX)))
        .collect()
}
