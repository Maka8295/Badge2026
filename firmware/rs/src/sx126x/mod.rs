mod airtime;
mod commands;
mod config;

use std::time::{Duration, Instant};

use esp_idf_svc::hal::delay::{Ets, FreeRtos};
use esp_idf_svc::hal::gpio::{Input, Output, PinDriver};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};
use esp_idf_svc::sys::esp_random;

use crate::leds;
use airtime::Airtime;
use commands::*;
pub use config::{RadioConfig, BW_KHZ, CHANNEL_STEP_KHZ, CR_RANGE, FREQ_KHZ_RANGE, MAX_TX_MS, POWER_DBM_RANGE, SF_RANGE};

const WAIT_INSTEAD_OF_BUSY_PIN_US: u32 = 1000;
const CALIBRATION_WAIT_MS: u32 = 50;
// The radio's own timer stops the PA here if the firmware ever hangs, keeping every packet under 4 s.
const TX_CUTOFF_MS: u32 = 3950;
const TX_CUTOFF_TICKS_PER_MS: u32 = 64;
const TX_FIRMWARE_DEADLINE: Duration = Duration::from_millis(TX_CUTOFF_MS as u64 + 10);
const TX_POLL_INTERVAL_MS: u32 = 10;
const RSSI_STEPS_PER_DB: i16 = 2;
const SNR_STEPS_PER_DB: i8 = 4;
const STATUS_BYTE_LEN: usize = 1;

// ARIB STD-T108 listen-before-talk: sense the channel for at least 5 ms, must not send at -80 dBm or above
// at the antenna, and rest at least 50 ms between transmissions. The threshold allows for the switch and
// filter loss between the antenna and the chip.
const CARRIER_SENSE: Duration = Duration::from_millis(6);
const CARRIER_BUSY_DBM: i16 = -83;
const CARRIER_SENSE_ATTEMPTS: u32 = 10;
const BACKOFF_MIN_MS: u32 = 20;
const BACKOFF_SPREAD_MS: u32 = 100;
const TX_REST: Duration = Duration::from_millis(50);

pub enum TxError {
    NoConfig,
    TooLong,
    HourlyLimit,
    ChannelBusy,
    Timeout,
}

pub struct Packet {
    pub data: Vec<u8>,
    pub rssi_dbm: i16,
    pub snr_db: i8,
}

pub struct Radio<'d> {
    spi: SpiDeviceDriver<'d, &'d SpiDriver<'d>>,
    rf_switch_line: PinDriver<'d, Input>,
    rx_enable: PinDriver<'d, Output>,
    config: Option<RadioConfig>,
    last_tx_end: Option<Instant>,
    airtime: Airtime,
    powered_on: Instant,
}

impl<'d> Radio<'d> {
    pub fn new(
        spi: SpiDeviceDriver<'d, &'d SpiDriver<'d>>,
        rf_switch_line: PinDriver<'d, Input>,
        rx_enable: PinDriver<'d, Output>,
    ) -> Self {
        Self {
            spi,
            rf_switch_line,
            rx_enable,
            config: None,
            last_tx_end: None,
            airtime: Airtime::new(),
            powered_on: Instant::now(),
        }
    }

    pub fn detect(&mut self) -> bool {
        let sync_word = self.read_register(REG_LORA_SYNC_WORD, 2);
        log::info!("sx126x sync word reg: {:02x?}", sync_word);
        sync_word == LORA_SYNC_WORD_AFTER_RESET
    }

    pub fn configure(&mut self, c: &RadioConfig) {
        self.config = Some(*c);
        self.standby();
        self.calibrate();
        self.write(SET_DIO2_AS_RF_SWITCH, &[DIO2_DRIVES_RF_SWITCH]);
        self.apply_datasheet_tuning();
        self.write(SET_RF_FREQUENCY, &c.frequency_steps().to_be_bytes());
        self.set_tx_power(c.power_dbm);
        self.write(SET_BUFFER_BASE_ADDRESS, &[BUFFER_BASE_ADDRESS, BUFFER_BASE_ADDRESS]);
        self.write(SET_MODULATION_PARAMS, &c.modulation_params());
        self.set_payload_len(MAX_PAYLOAD_LEN);
        self.set_sync_word(c.sync);
        self.enable_irqs();
        self.clear_irqs();
        self.start_rx();
        self.log_chip_state();
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TxError> {
        let config = self.config.ok_or(TxError::NoConfig)?;
        let on_air_ms = config.time_on_air_ms(data.len());
        if on_air_ms > MAX_TX_MS {
            return Err(TxError::TooLong);
        }
        if !self.airtime.allows(self.powered_on.elapsed().as_secs(), on_air_ms.ceil() as u32) {
            return Err(TxError::HourlyLimit);
        }
        self.wait_for_tx_rest();
        self.standby();
        self.write_buffer(data);
        self.set_payload_len(data.len() as u8);
        let result = self.wait_for_clear_channel(&config).and_then(|()| self.transmit());
        self.clear_irqs();
        self.set_payload_len(MAX_PAYLOAD_LEN);
        self.start_rx();
        result
    }

    fn transmit(&mut self) -> Result<(), TxError> {
        leds::flash(leds::LORA_COLOR);
        self.clear_irqs();
        self.rx_enable.set_low().unwrap();
        let started = Instant::now();
        let cutoff_ticks = (TX_CUTOFF_MS * TX_CUTOFF_TICKS_PER_MS).to_be_bytes();
        self.write(SET_TX, &cutoff_ticks[1..]);
        log::info!("tx: rf switch line {}", if self.rf_switch_line.is_high() { "high" } else { "LOW, check module R8" });
        let done = self.wait_for_tx_done();
        self.last_tx_end = Some(Instant::now());
        let on_air_ms = started.elapsed().as_millis() as u32;
        self.airtime.record(self.powered_on.elapsed().as_secs(), on_air_ms);
        if done { Ok(()) } else { Err(TxError::Timeout) }
    }

    fn wait_for_tx_rest(&mut self) {
        if let Some(end) = self.last_tx_end {
            let rested = end.elapsed();
            if rested < TX_REST {
                FreeRtos::delay_ms((TX_REST - rested).as_millis() as u32);
            }
        }
    }

    fn wait_for_clear_channel(&mut self, c: &RadioConfig) -> Result<(), TxError> {
        for _ in 0..CARRIER_SENSE_ATTEMPTS {
            if self.channel_clear(c) {
                return Ok(());
            }
            FreeRtos::delay_ms(BACKOFF_MIN_MS + unsafe { esp_random() } % BACKOFF_SPREAD_MS);
        }
        Err(TxError::ChannelBusy)
    }

    fn channel_clear(&mut self, c: &RadioConfig) -> bool {
        self.write(SET_MODULATION_PARAMS, &c.carrier_sense_params());
        self.start_rx();
        let start = Instant::now();
        let mut clear = true;
        while clear && start.elapsed() < CARRIER_SENSE {
            let rssi_dbm = self.instant_rssi_dbm();
            if rssi_dbm >= CARRIER_BUSY_DBM {
                log::info!("carrier sense: channel busy ({rssi_dbm} dBm)");
                clear = false;
            }
        }
        self.standby();
        self.write(SET_MODULATION_PARAMS, &c.modulation_params());
        clear
    }

    fn instant_rssi_dbm(&mut self) -> i16 {
        -(self.read(&[GET_RSSI_INST], 1)[0] as i16) / RSSI_STEPS_PER_DB
    }

    pub fn try_receive(&mut self) -> Option<Packet> {
        let irq = self.irq_status();
        if irq & IRQ_RX_DONE == 0 {
            if irq & IRQ_HEADER_ERR != 0 {
                self.clear_irqs();
            }
            return None;
        }
        self.clear_irqs();
        if irq & IRQ_CRC_ERR != 0 {
            log::warn!("rx crc error");
            return None;
        }
        leds::breathe(leds::LORA_COLOR);
        Some(self.read_packet())
    }

    fn calibrate(&mut self) {
        self.write(SET_REGULATOR_MODE, &[REGULATOR_DC_DC]);
        self.write(SET_PACKET_TYPE, &[PACKET_TYPE_LORA]);
        self.write(CALIBRATE, &[CALIBRATE_ALL_BLOCKS]);
        FreeRtos::delay_ms(CALIBRATION_WAIT_MS);
        self.write(CALIBRATE_IMAGE, &IMAGE_CALIBRATION_902_928_MHZ);
        FreeRtos::delay_ms(CALIBRATION_WAIT_MS);
    }

    // SX1262 datasheet: boosted Rx gain (9.6) and the antenna mismatch fix (15.2).
    fn apply_datasheet_tuning(&mut self) {
        self.write_register(REG_RX_GAIN, &[RX_GAIN_BOOSTED]);
        let clamp = self.read_register(REG_TX_CLAMP_CONFIG, 1)[0];
        self.write_register(REG_TX_CLAMP_CONFIG, &[clamp | TX_CLAMP_MISMATCH_FIX]);
    }

    fn set_tx_power(&mut self, power_dbm: i8) {
        self.write(SET_PA_CONFIG, &PA_CONFIG_HIGH_POWER);
        let capped_dbm = power_dbm.min(*POWER_DBM_RANGE.end() as i8);
        self.write(SET_TX_PARAMS, &[capped_dbm as u8, RAMP_200_US]);
    }

    fn set_sync_word(&mut self, sync: u8) {
        let high = (sync & HIGH_NIBBLE_MASK) | LORA_SYNC_WORD_LOW_BITS;
        let low = (sync << NIBBLE_BITS) | LORA_SYNC_WORD_LOW_BITS;
        self.write_register(REG_LORA_SYNC_WORD, &[high, low]);
    }

    fn enable_irqs(&mut self) {
        let mut args = IRQ_USED.to_be_bytes().to_vec();
        args.extend_from_slice(&IRQ_NOT_ROUTED_TO_DIO_PINS);
        self.write(SET_DIO_IRQ_PARAMS, &args);
    }

    fn set_payload_len(&mut self, len: u8) {
        let [preamble_high, preamble_low] = PREAMBLE_SYMBOLS.to_be_bytes();
        self.write(SET_PACKET_PARAMS, &[preamble_high, preamble_low, HEADER_EXPLICIT, len, CRC_ON, IQ_STANDARD]);
    }

    fn standby(&mut self) {
        self.write(SET_STANDBY, &[STANDBY_RC]);
    }

    fn start_rx(&mut self) {
        self.rx_enable.set_high().unwrap();
        self.write(SET_RX, &RX_CONTINUOUS);
    }

    fn wait_for_tx_done(&mut self) -> bool {
        let start = Instant::now();
        while start.elapsed() < TX_FIRMWARE_DEADLINE {
            let irq = self.irq_status();
            if irq & IRQ_TX_DONE != 0 {
                return true;
            }
            if irq & IRQ_TIMEOUT != 0 {
                return false;
            }
            FreeRtos::delay_ms(TX_POLL_INTERVAL_MS);
        }
        self.standby();
        false
    }

    fn read_packet(&mut self) -> Packet {
        let buffer_status = self.read(&[GET_RX_BUFFER_STATUS], 2);
        let (len, start) = (buffer_status[0] as usize, buffer_status[1]);
        let data = self.read(&[READ_BUFFER, start], len);
        let packet_status = self.read(&[GET_PACKET_STATUS], 3);
        Packet {
            data,
            rssi_dbm: -(packet_status[0] as i16) / RSSI_STEPS_PER_DB,
            snr_db: (packet_status[1] as i8) / SNR_STEPS_PER_DB,
        }
    }

    fn log_chip_state(&mut self) {
        let errors = self.read_u16(GET_DEVICE_ERRORS);
        let mode = (self.chip_status() >> CHIP_MODE_SHIFT) & CHIP_MODE_MASK;
        log::info!("sx126x errors 0x{:04x}, mode {}", errors, mode);
    }

    fn irq_status(&mut self) -> u16 {
        self.read_u16(GET_IRQ_STATUS)
    }

    fn clear_irqs(&mut self) {
        self.write(CLEAR_IRQ_STATUS, &IRQ_ALL.to_be_bytes());
    }

    fn transfer(&mut self, buf: &mut [u8]) {
        self.spi.transfer_in_place(buf).unwrap();
        wait_until_not_busy();
    }

    fn write(&mut self, opcode: u8, args: &[u8]) {
        let mut buf = vec![opcode];
        buf.extend_from_slice(args);
        self.transfer(&mut buf);
    }

    fn read(&mut self, command: &[u8], len: usize) -> Vec<u8> {
        let mut buf = command.to_vec();
        buf.resize(command.len() + STATUS_BYTE_LEN + len, NOP);
        self.transfer(&mut buf);
        buf.split_off(command.len() + STATUS_BYTE_LEN)
    }

    fn read_u16(&mut self, opcode: u8) -> u16 {
        let response = self.read(&[opcode], 2);
        u16::from_be_bytes([response[0], response[1]])
    }

    fn chip_status(&mut self) -> u8 {
        let mut buf = [GET_STATUS, NOP];
        self.transfer(&mut buf);
        buf[1]
    }

    fn write_register(&mut self, addr: u16, data: &[u8]) {
        let mut args = addr.to_be_bytes().to_vec();
        args.extend_from_slice(data);
        self.write(WRITE_REGISTER, &args);
    }

    fn read_register(&mut self, addr: u16, len: usize) -> Vec<u8> {
        let [addr_high, addr_low] = addr.to_be_bytes();
        self.read(&[READ_REGISTER, addr_high, addr_low], len)
    }

    fn write_buffer(&mut self, data: &[u8]) {
        let mut args = vec![BUFFER_BASE_ADDRESS];
        args.extend_from_slice(data);
        self.write(WRITE_BUFFER, &args);
    }
}

fn wait_until_not_busy() {
    Ets::delay_us(WAIT_INSTEAD_OF_BUSY_PIN_US);
}
