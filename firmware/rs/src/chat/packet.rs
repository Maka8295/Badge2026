use std::ops::Range;

const MAGIC: [u8; 2] = [0xB2, 0x26];
const NICK_LEN_INDEX: usize = MAGIC.len();
const HEADER_LEN: usize = NICK_LEN_INDEX + 1;
pub const NICK_MAX: usize = 12;

const PRINTABLE_ASCII: Range<u8> = 0x20..0x7f;
const UNPRINTABLE_REPLACEMENT: char = '?';

pub fn encode(nick: &str, text: &str) -> Vec<u8> {
    let mut packet = MAGIC.to_vec();
    packet.push(nick.len() as u8);
    packet.extend_from_slice(nick.as_bytes());
    packet.extend_from_slice(text.as_bytes());
    packet
}

pub fn decode(data: &[u8]) -> Option<(String, String)> {
    if data.len() < HEADER_LEN || data[..NICK_LEN_INDEX] != MAGIC {
        return None;
    }
    let nick_len = data[NICK_LEN_INDEX] as usize;
    let text_start = HEADER_LEN + nick_len;
    if data.len() < text_start {
        return None;
    }
    let nick = &data[HEADER_LEN..HEADER_LEN + nick_len.min(NICK_MAX)];
    Some((printable(nick), printable(&data[text_start..])))
}

fn printable(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if PRINTABLE_ASCII.contains(&b) { b as char } else { UNPRINTABLE_REPLACEMENT })
        .collect()
}
