use crate::ir::signal::Signal;

pub struct PulseDistance {
    pub header_mark_us: u16,
    pub header_space_us: u16,
    pub bit_mark_us: u16,
    pub zero_space_us: u16,
    pub one_space_us: u16,
}

impl PulseDistance {
    pub fn frame(&self, bits: impl Iterator<Item = bool>, gap_us: u16) -> Signal {
        let mut frame = vec![(self.header_mark_us, self.header_space_us)];
        frame.extend(bits.map(|bit| (self.bit_mark_us, self.space_for(bit))));
        frame.push((self.bit_mark_us, gap_us));
        frame
    }

    fn space_for(&self, bit: bool) -> u16 {
        if bit {
            self.one_space_us
        } else {
            self.zero_space_us
        }
    }
}
