use super::protocols::{kaseikyo, nec, pioneer, raw, rc5, rc6, rca, samsung, sirc};
use super::signal::Signal;

#[derive(Clone, Copy, Debug)]
pub enum Protocol {
    Nec,
    NecExt,
    Nec42,
    Samsung32,
    Sirc12,
    Sirc15,
    Sirc20,
    Rc5,
    Rc5x,
    Rc6,
    Kaseikyo,
    Rca,
    Pioneer,
}

pub enum Code<'a> {
    Parsed(Protocol, u32, u32),
    Raw(u32, &'a [u16]),
}

pub struct Button {
    pub name: &'static str,
    pub codes: &'static [Code<'static>],
}

pub struct Remote {
    pub name: &'static str,
    pub buttons: &'static [Button],
}

pub fn encode(code: &Code<'_>) -> (Signal, u32) {
    match *code {
        Code::Raw(carrier_hz, durations) => (raw::encode(durations), carrier_hz),
        Code::Parsed(proto, address, command) => (proto.encode(address, command), proto.carrier_hz()),
    }
}

impl Protocol {
    fn encode(self, address: u32, command: u32) -> Signal {
        match self {
            Protocol::Nec => nec::standard(address, command),
            Protocol::NecExt => nec::extended(address, command),
            Protocol::Nec42 => nec::nec42(address, command),
            Protocol::Samsung32 => samsung::encode(address, command),
            Protocol::Sirc12 => sirc::sirc12(address, command),
            Protocol::Sirc15 => sirc::sirc15(address, command),
            Protocol::Sirc20 => sirc::sirc20(address, command),
            Protocol::Rc5 => rc5::standard(address, command),
            Protocol::Rc5x => rc5::extended(address, command),
            Protocol::Rc6 => rc6::encode(address, command),
            Protocol::Kaseikyo => kaseikyo::encode(address, command),
            Protocol::Rca => rca::encode(address, command),
            Protocol::Pioneer => pioneer::encode(address, command),
        }
    }

    fn carrier_hz(self) -> u32 {
        match self {
            Protocol::Nec | Protocol::NecExt | Protocol::Nec42 => nec::CARRIER_HZ,
            Protocol::Samsung32 => samsung::CARRIER_HZ,
            Protocol::Sirc12 | Protocol::Sirc15 | Protocol::Sirc20 => sirc::CARRIER_HZ,
            Protocol::Rc5 | Protocol::Rc5x => rc5::CARRIER_HZ,
            Protocol::Rc6 => rc6::CARRIER_HZ,
            Protocol::Kaseikyo => kaseikyo::CARRIER_HZ,
            Protocol::Rca => rca::CARRIER_HZ,
            Protocol::Pioneer => pioneer::CARRIER_HZ,
        }
    }
}
