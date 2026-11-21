pub const CLEAR_IRQ_STATUS: u8 = 0x02;
pub const SET_DIO_IRQ_PARAMS: u8 = 0x08;
pub const WRITE_REGISTER: u8 = 0x0D;
pub const WRITE_BUFFER: u8 = 0x0E;
pub const GET_IRQ_STATUS: u8 = 0x12;
pub const GET_RX_BUFFER_STATUS: u8 = 0x13;
pub const GET_PACKET_STATUS: u8 = 0x14;
pub const GET_RSSI_INST: u8 = 0x15;
pub const GET_DEVICE_ERRORS: u8 = 0x17;
pub const READ_REGISTER: u8 = 0x1D;
pub const READ_BUFFER: u8 = 0x1E;
pub const SET_STANDBY: u8 = 0x80;
pub const SET_RX: u8 = 0x82;
pub const SET_TX: u8 = 0x83;
pub const SET_RF_FREQUENCY: u8 = 0x86;
pub const CALIBRATE: u8 = 0x89;
pub const SET_PACKET_TYPE: u8 = 0x8A;
pub const SET_MODULATION_PARAMS: u8 = 0x8B;
pub const SET_PACKET_PARAMS: u8 = 0x8C;
pub const SET_TX_PARAMS: u8 = 0x8E;
pub const SET_BUFFER_BASE_ADDRESS: u8 = 0x8F;
pub const SET_PA_CONFIG: u8 = 0x95;
pub const SET_REGULATOR_MODE: u8 = 0x96;
pub const CALIBRATE_IMAGE: u8 = 0x98;
pub const SET_DIO2_AS_RF_SWITCH: u8 = 0x9D;
pub const GET_STATUS: u8 = 0xC0;

pub const NOP: u8 = 0x00;

pub const REG_LORA_SYNC_WORD: u16 = 0x0740;
pub const LORA_SYNC_WORD_AFTER_RESET: [u8; 2] = [0x14, 0x24];
pub const REG_RX_GAIN: u16 = 0x08AC;
pub const RX_GAIN_BOOSTED: u8 = 0x96;
pub const REG_TX_CLAMP_CONFIG: u16 = 0x08D8;
pub const TX_CLAMP_MISMATCH_FIX: u8 = 0x1E;
pub const LORA_SYNC_WORD_LOW_BITS: u8 = 0x04;
pub const HIGH_NIBBLE_MASK: u8 = 0xF0;
pub const NIBBLE_BITS: u8 = 4;

pub const IRQ_TX_DONE: u16 = 1 << 0;
pub const IRQ_RX_DONE: u16 = 1 << 1;
pub const IRQ_HEADER_ERR: u16 = 1 << 5;
pub const IRQ_CRC_ERR: u16 = 1 << 6;
pub const IRQ_TIMEOUT: u16 = 1 << 9;
pub const IRQ_USED: u16 = IRQ_TX_DONE | IRQ_RX_DONE | IRQ_HEADER_ERR | IRQ_CRC_ERR | IRQ_TIMEOUT;
pub const IRQ_ALL: u16 = 0xFFFF;
pub const IRQ_NOT_ROUTED_TO_DIO_PINS: [u8; 6] = [0; 6];

pub const STANDBY_RC: u8 = 0x00;
pub const REGULATOR_DC_DC: u8 = 0x01;
pub const PACKET_TYPE_LORA: u8 = 0x01;
pub const CALIBRATE_ALL_BLOCKS: u8 = 0x7F;
pub const IMAGE_CALIBRATION_902_928_MHZ: [u8; 2] = [0xE1, 0xE9];
pub const DIO2_DRIVES_RF_SWITCH: u8 = 0x01;
pub const PA_CONFIG_HIGH_POWER: [u8; 4] = [0x04, 0x07, 0x00, 0x01];
pub const RAMP_200_US: u8 = 0x04;
pub const BUFFER_BASE_ADDRESS: u8 = 0x00;
pub const RX_CONTINUOUS: [u8; 3] = [0xFF, 0xFF, 0xFF];

pub const PREAMBLE_SYMBOLS: u16 = 8;
pub const HEADER_EXPLICIT: u8 = 0x00;
pub const CRC_ON: u8 = 0x01;
pub const IQ_STANDARD: u8 = 0x00;
pub const MAX_PAYLOAD_LEN: u8 = 255;

pub const CHIP_MODE_SHIFT: u8 = 4;
pub const CHIP_MODE_MASK: u8 = 0x07;
