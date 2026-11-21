use esp_idf_svc::hal::gpio::*;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::sd::mmc::SDMMC1;
use esp_idf_svc::hal::spi::SPI2;

pub const MUX_SELECT_PINS: usize = 3;

pub struct SharedSpi {
    pub bus: SPI2<'static>,
    pub sck: Gpio41<'static>,
    pub mosi: Gpio39<'static>,
    pub miso: Gpio40<'static>,
}

pub struct DisplayPins {
    pub cs: Gpio2<'static>,
    pub data_command: Gpio1<'static>,
}

pub struct LoraPins {
    pub cs: Gpio38<'static>,
    pub rf_switch_line: Gpio42<'static>,
    pub rx_enable: Gpio46<'static>,
}

pub struct IrPins {
    pub tx: Gpio44<'static>,
    pub rx: Gpio43<'static>,
}

pub struct KeyMatrixPins {
    pub row_out: Gpio21<'static>,
    pub row_select: [AnyOutputPin<'static>; MUX_SELECT_PINS],
    pub cols_1_to_7_select: [AnyOutputPin<'static>; MUX_SELECT_PINS],
    pub cols_8_to_14_select: [AnyOutputPin<'static>; MUX_SELECT_PINS],
    pub cols_1_to_7_out: Gpio47<'static>,
    pub cols_8_to_14_out: Gpio48<'static>,
}

pub struct SdPins {
    pub slot: SDMMC1<'static>,
    pub clk: Gpio6<'static>,
    pub cmd: Gpio7<'static>,
    pub d0: Gpio5<'static>,
    pub d1: Gpio4<'static>,
    pub d2: Gpio16<'static>,
    pub d3: Gpio15<'static>,
}

pub struct Board {
    pub spi: SharedSpi,
    pub display: DisplayPins,
    pub lora: LoraPins,
    pub ir: IrPins,
    pub keys: KeyMatrixPins,
    pub leds: Gpio3<'static>,
}

impl Board {
    pub fn take() -> Self {
        let peripherals = Peripherals::take().unwrap();
        let p = peripherals.pins;
        Self {
            spi: SharedSpi { bus: peripherals.spi2, sck: p.gpio41, mosi: p.gpio39, miso: p.gpio40 },
            display: DisplayPins { cs: p.gpio2, data_command: p.gpio1 },
            lora: LoraPins { cs: p.gpio38, rf_switch_line: p.gpio42, rx_enable: p.gpio46 },
            ir: IrPins { tx: p.gpio44, rx: p.gpio43 },
            keys: KeyMatrixPins {
                row_out: p.gpio21,
                row_select: [p.gpio17.degrade_output(), p.gpio18.degrade_output(), p.gpio8.degrade_output()],
                cols_1_to_7_select: [p.gpio9.degrade_output(), p.gpio10.degrade_output(), p.gpio11.degrade_output()],
                cols_8_to_14_select: [p.gpio12.degrade_output(), p.gpio13.degrade_output(), p.gpio14.degrade_output()],
                cols_1_to_7_out: p.gpio47,
                cols_8_to_14_out: p.gpio48,
            },
            leds: p.gpio3,
        }
    }
}

pub unsafe fn steal_sd_pins() -> SdPins {
    SdPins {
        slot: SDMMC1::steal(),
        clk: Gpio6::steal(),
        cmd: Gpio7::steal(),
        d0: Gpio5::steal(),
        d1: Gpio4::steal(),
        d2: Gpio16::steal(),
        d3: Gpio15::steal(),
    }
}
