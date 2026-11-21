pub const BYTE_BITS: usize = 8;

#[derive(Clone, Copy)]
pub struct Bits {
    pub value: u64,
    pub count: usize,
}

pub fn pack(fields: &[(u32, usize)]) -> Bits {
    let mut value = 0;
    let mut count = 0;
    for &(field, width) in fields {
        value |= (field as u64 & ((1 << width) - 1)) << count;
        count += width;
    }
    Bits { value, count }
}

impl Bits {
    pub fn lsb_first(self) -> impl Iterator<Item = bool> {
        (0..self.count).map(move |i| self.value >> i & 1 == 1)
    }

    pub fn msb_first(self) -> impl Iterator<Item = bool> {
        (0..self.count).rev().map(move |i| self.value >> i & 1 == 1)
    }
}
