use std::time::Duration;

use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
use esp_idf_svc::hal::rmt::config::{
    CarrierConfig, DutyPercent, MemoryAccess, ReceiveConfig, RxChannelConfig, TransmitConfig, TxChannelConfig,
};
use esp_idf_svc::hal::rmt::encoder::CopyEncoder;
use esp_idf_svc::hal::rmt::{PinState, Pulse, PulseTicks, RmtChannel, RxChannelDriver, Symbol, TxChannelDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys;

use super::signal::{Signal, MAX_PAIRS};
use crate::{leds, rmt_padding};

const RMT_MAX_TICKS: u16 = 32_767;
const MEMORY_BLOCK_SYMBOLS: usize = 96;
const CARRIER_DUTY_CYCLE: f32 = 0.33;
const SEND_TIMEOUT: Duration = Duration::from_secs(2);
const RECEIVE_MIN_PULSE: Duration = Duration::from_nanos(1_250);
const RECEIVE_IDLE_END: Duration = Duration::from_millis(12);
const MIN_RECEIVED_PAIRS: usize = 4;

pub struct Ir<'d> {
    tx: TxChannelDriver<'d>,
    rx: RxChannelDriver<'d>,
    rx_buf: Vec<Symbol>,
}

impl<'d> Ir<'d> {
    pub fn new(tx_pin: impl OutputPin + 'd, rx_pin: impl InputPin + 'd) -> Self {
        let memory_access = MemoryAccess::Indirect {
            memory_block_symbols: MEMORY_BLOCK_SYMBOLS,
        };
        // HARDWARE: keep the TX output non-inverted (idle low). GPIO44 high switches Q1 on and runs
        // D67/D68 at ~60 mA each. Creating this channel is what drives the pin low at boot.
        let tx_config = TxChannelConfig {
            memory_access,
            ..Default::default()
        };
        let rx_config = RxChannelConfig {
            memory_access,
            invert_in: true,
            ..Default::default()
        };
        let rx_gpio = rx_pin.pin();
        let ir = Ir {
            tx: TxChannelDriver::new(tx_pin, &tx_config).unwrap(),
            rx: RxChannelDriver::new(rx_pin, &rx_config).unwrap(),
            rx_buf: vec![Symbol::default(); MAX_PAIRS],
        };
        // HARDWARE: do not remove. GPIO43 (IR-IN) is UART0 TX in the boot ROM, which leaves its output on.
        // The RMT RX driver switches the pin to GPIO input but never disables that output (ESP-IDF v6.0
        // rmt_rx.c), so the ESP32 would keep driving against the TSOP38238 output (5 mA abs max).
        // See HARDWARE.md "IR".
        sys::EspError::convert(unsafe { sys::gpio_set_direction(rx_gpio as _, sys::gpio_mode_t_GPIO_MODE_INPUT) })
            .unwrap();
        ir
    }

    pub fn send(&mut self, signal: &[(u16, u16)], carrier_hz: u32) {
        leds::flash(leds::IR_COLOR);
        self.tx.apply_carrier(Some(&carrier(carrier_hz))).unwrap();
        let mut symbols = to_symbols(signal);
        rmt_padding::avoid_exact_fill(&mut symbols, MEMORY_BLOCK_SYMBOLS);
        if let Err(e) = self
            .tx
            .send_and_wait(CopyEncoder::new().unwrap(), &symbols, &TransmitConfig::default())
        {
            log::error!("ir send: {e}");
        }
        self.tx.wait_all_done(Some(SEND_TIMEOUT)).ok();
    }

    pub fn try_receive(&mut self) -> Option<Signal> {
        let count = self.rx.receive(&mut self.rx_buf, &receive_config()).ok()?;
        let signal: Signal = self.rx_buf[..count].iter().map(symbol_to_pair).collect();
        let received = signal.len() >= MIN_RECEIVED_PAIRS;
        if received {
            leds::breathe(leds::IR_COLOR);
        }
        received.then_some(signal)
    }
}

fn carrier(frequency_hz: u32) -> CarrierConfig {
    CarrierConfig {
        frequency: frequency_hz.Hz(),
        duty_cycle: DutyPercent::new(CARRIER_DUTY_CYCLE).unwrap(),
        ..Default::default()
    }
}

fn receive_config() -> ReceiveConfig {
    ReceiveConfig {
        signal_range_min: RECEIVE_MIN_PULSE,
        signal_range_max: RECEIVE_IDLE_END,
        timeout: Some(0),
        ..Default::default()
    }
}

fn symbol_to_pair(symbol: &Symbol) -> (u16, u16) {
    (symbol.level0().ticks.ticks(), symbol.level1().ticks.ticks())
}

fn to_symbols(signal: &[(u16, u16)]) -> Vec<Symbol> {
    let mut symbols = Vec::with_capacity(signal.len());
    for &(mark, space) in signal {
        symbols.push(Symbol::new(
            pulse(PinState::High, mark),
            pulse(PinState::Low, space.min(RMT_MAX_TICKS)),
        ));
        push_long_space_rest(&mut symbols, space.saturating_sub(RMT_MAX_TICKS));
    }
    symbols
}

fn push_long_space_rest(symbols: &mut Vec<Symbol>, mut rest: u16) {
    while rest > 0 {
        let first = rest.min(RMT_MAX_TICKS);
        rest -= first;
        let second = rest.min(RMT_MAX_TICKS);
        rest -= second;
        symbols.push(Symbol::new(pulse(PinState::Low, first), pulse(PinState::Low, second)));
    }
}

fn pulse(state: PinState, ticks: u16) -> Pulse {
    Pulse::new(state, PulseTicks::new(ticks.clamp(1, RMT_MAX_TICKS)).unwrap())
}
