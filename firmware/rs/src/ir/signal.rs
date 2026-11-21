pub type Signal = Vec<(u16, u16)>;

pub const MAX_PAIRS: usize = 192;
pub const BYTES_PER_PAIR: usize = 4;
pub const FRAME_GAP_US: u16 = 40_000;

pub fn to_bytes(signal: &[(u16, u16)]) -> Vec<u8> {
    signal
        .iter()
        .flat_map(|&(mark, space)| [mark.to_le_bytes(), space.to_le_bytes()].concat())
        .collect()
}

pub fn from_bytes(bytes: &[u8]) -> Signal {
    bytes
        .chunks_exact(BYTES_PER_PAIR)
        .map(|chunk| (u16::from_le_bytes([chunk[0], chunk[1]]), u16::from_le_bytes([chunk[2], chunk[3]])))
        .collect()
}
