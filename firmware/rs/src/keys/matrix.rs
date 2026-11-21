use esp_idf_svc::hal::delay::Ets;
use esp_idf_svc::hal::gpio::{AnyOutputPin, Input, Output, PinDriver, Pull};

use crate::board::{KeyMatrixPins, MUX_SELECT_PINS};

pub const ROWS: usize = 7;
pub const COLS: usize = 14;
const COLS_PER_MUX: usize = 7;
const FIRST_MUX_ADDRESS: u8 = 1;
const MUX_SETTLE_US: u32 = 5;

pub type KeyStates = [[bool; COLS]; ROWS];

struct Mux([PinDriver<'static, Output>; MUX_SELECT_PINS]);

impl Mux {
    fn new(select_pins: [AnyOutputPin<'static>; MUX_SELECT_PINS]) -> Self {
        Self(select_pins.map(|pin| PinDriver::output(pin).unwrap()))
    }

    fn select(&mut self, line: usize) {
        let address = FIRST_MUX_ADDRESS + line as u8;
        for (bit, pin) in self.0.iter_mut().enumerate() {
            pin.set_level(((address >> bit) & 1 != 0).into()).unwrap();
        }
    }
}

pub struct Matrix {
    _row_out: PinDriver<'static, Output>,
    rows: Mux,
    cols_1_to_7: Mux,
    cols_8_to_14: Mux,
    cols_1_to_7_out: PinDriver<'static, Input>,
    cols_8_to_14_out: PinDriver<'static, Input>,
}

impl Matrix {
    pub fn new(pins: KeyMatrixPins) -> Self {
        let mut row_out = PinDriver::output(pins.row_out).unwrap();
        row_out.set_low().unwrap();
        Self {
            _row_out: row_out,
            rows: Mux::new(pins.row_select),
            cols_1_to_7: Mux::new(pins.cols_1_to_7_select),
            cols_8_to_14: Mux::new(pins.cols_8_to_14_select),
            cols_1_to_7_out: PinDriver::input(pins.cols_1_to_7_out, Pull::Up).unwrap(),
            cols_8_to_14_out: PinDriver::input(pins.cols_8_to_14_out, Pull::Up).unwrap(),
        }
    }

    pub fn scan(&mut self) -> KeyStates {
        let mut states = [[false; COLS]; ROWS];
        for (row, row_states) in states.iter_mut().enumerate() {
            self.rows.select(row);
            for col in 0..COLS_PER_MUX {
                self.select_column(col);
                row_states[col] = self.cols_1_to_7_out.is_low();
                row_states[col + COLS_PER_MUX] = self.cols_8_to_14_out.is_low();
            }
        }
        states
    }

    fn select_column(&mut self, col: usize) {
        self.cols_1_to_7.select(col);
        self.cols_8_to_14.select(col);
        Ets::delay_us(MUX_SETTLE_US);
    }
}
