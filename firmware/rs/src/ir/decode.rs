use super::protocols::{nec, samsung};

const TOLERANCE_DIVISOR: i32 = 4;
const TOLERANCE_MARGIN_US: i32 = 100;
const DATA_BITS: usize = 32;

const EMPTY: &str = "empty";
const NEC_NAME: &str = "NEC";
const SAMSUNG_NAME: &str = "Samsung";

pub fn describe(signal: &[(u16, u16)]) -> String {
    let Some(&first) = signal.first() else {
        return EMPTY.into();
    };
    let Some(name) = protocol_name(first) else {
        return format!("raw {} pulses", signal.len());
    };
    if signal.len() <= DATA_BITS {
        return format!("{name}? {} pulses", signal.len());
    }
    match read_bits(&signal[1..=DATA_BITS]) {
        Ok(value) => format_code(name, value),
        Err(bad_bit) => format!("{name}? bad bit {bad_bit}"),
    }
}

fn protocol_name((mark, space): (u16, u16)) -> Option<&'static str> {
    if near(mark, nec::TIMING.header_mark_us) && near(space, nec::TIMING.header_space_us) {
        Some(NEC_NAME)
    } else if near(mark, samsung::TIMING.header_mark_us) && near(space, samsung::TIMING.header_space_us) {
        Some(SAMSUNG_NAME)
    } else {
        None
    }
}

fn read_bits(bits: &[(u16, u16)]) -> Result<u32, usize> {
    let mut value = 0u32;
    for (i, &(mark, space)) in bits.iter().enumerate() {
        if !near(mark, nec::TIMING.bit_mark_us) {
            return Err(i);
        }
        value |= (near(space, nec::TIMING.one_space_us) as u32) << i;
    }
    Ok(value)
}

fn format_code(name: &str, value: u32) -> String {
    let [address_low, address_high, command_low, command_high] = value.to_le_bytes();
    let address_repeated = name == SAMSUNG_NAME && address_high == address_low;
    let address = field_hex(address_low, address_high, address_repeated);
    format!("{name} {address}:{}", field_hex(command_low, command_high, false))
}

fn field_hex(low: u8, high: u8, repeated: bool) -> String {
    if high == !low || repeated {
        format!("{low:02X}")
    } else {
        format!("{low:02X}{high:02X}")
    }
}

fn near(value: u16, target: u16) -> bool {
    let (value, target) = (value as i32, target as i32);
    (value - target).abs() <= target / TOLERANCE_DIVISOR + TOLERANCE_MARGIN_US
}
