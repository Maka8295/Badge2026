mod board;
mod board_name;
mod chat;
mod elf;
mod hunt;
mod explorer;
mod intro;
mod ir;
mod irapp;
mod irdb;
mod keys;
mod ledapp;
mod leds;
mod matrix;
mod phrack;
mod sd;
mod rmt_padding;
mod splash;
mod sponsors;
mod sx126x;
mod nametag;
mod ui;
mod wifi;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::DrawTarget;
use esp_idf_svc::hal::delay::{Delay, FreeRtos};
use esp_idf_svc::hal::gpio::{InputPin, OutputPin, PinDriver, Pull};
use esp_idf_svc::hal::spi::config::{Config, DriverConfig};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use mipidsi::interface::SpiInterface;
use mipidsi::models::ILI9341Rgb565;
use mipidsi::options::{ColorOrder, Orientation, Rotation};
use mipidsi::Builder;

use board::{Board, SharedSpi};
use keys::{Key, Keys};

const DISPLAY_SPI_MHZ: u32 = 40;
const LORA_SPI_MHZ: u32 = 8;
const DISPLAY_BUFFER_SIZE: usize = 4096;
const DISPLAY_ORIENTATION: Orientation = Orientation::new().rotate(Rotation::Deg90).flip_horizontal();
const LOOP_DELAY_MS: u32 = 10;
const NVS_NAMESPACE: &str = "chat";

const MENU_HINT: &str = "UP/DOWN move   A open";
const MENU_ITEMS: [&str; 11] =
    ["Chat", "IR blaster", "Nametag", "Badge swap", "LEDs", "Phrack", "Matrix", "Files", "Wi-Fi", "Sponsors", "About"];
const MENU_APPS: [App; 11] = [
    App::Chat,
    App::IrBlaster,
    App::Nametag,
    App::Swap,
    App::Leds,
    App::Phrack,
    App::Matrix,
    App::Files,
    App::Wifi,
    App::Sponsors,
    App::About,
];
const ABOUT_TITLE: &str = "// about";
// Pressing A this many times in a row on the About page shows Sir Jaffalot.
const JAFFALOT_PRESSES: u8 = 5;
const ABOUT_LINES: [&str; 10] = [
    "Tengucon, by tripp/cicada & good vibes :)",
    "Hardware by Jonathan Black",
    "  (github.com/Maka8295)",
    "Firmware by djnn (https://djnn.sh),",
    "  Claude and Jonathan",
    "Assembly by Jonathan and Marika",
    "Special thanks to Damien, Lily, Jojo,",
    "Amanda, James, Sheldon and Caleb!",
    "Tengu never dies",
    "Want a black badge? Type TENGU in menu.",
];

const IR_SELF_CHECK: [(ir::Code, &str); 4] = [
    (ir::Code::Parsed(ir::Protocol::Nec, 0x04, 0x08), "NEC 04:08"),
    (ir::Code::Parsed(ir::Protocol::NecExt, 0xdf00, 0x1c), "NEC 00DF:1C00"),
    (ir::Code::Parsed(ir::Protocol::Samsung32, 0x07, 0x02), "Samsung 07:02"),
    (ir::Code::Parsed(ir::Protocol::NecExt, 0x6f74, 0x656b), "NEC 746F:6B65"),
];

#[derive(Clone, Copy, PartialEq)]
enum App {
    Hunt,
    Chat,
    IrBlaster,
    Wifi,
    Swap,
    Files,
    Phrack,
    Leds,
    Matrix,
    Nametag,
    Sponsors,
    About,
}

struct Badge<'a, D> {
    display: D,
    radio: sx126x::Radio<'a>,
    radio_ok: bool,
    chat: chat::Chat,
    ir: ir::Ir<'a>,
    irapp: irapp::IrApp,
    sd: sd::SdCard,
    explorer: explorer::Explorer,
    phrack: phrack::Phrack,
    ledapp: ledapp::LedApp,
    matrix: matrix::Matrix,
    nametag: nametag::NametagApp,
    sponsors: sponsors::Sponsors,
    hunt: hunt::Hunt,
    wifi: wifi::WifiApp,
    swap: hunt::SwapApp,
    typed: String,
    keys: Keys,
    nvs: EspNvs<NvsDefault>,
    open: Option<App>,
    selected: usize,
    name: String,
    about_presses: u8,
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let board = Board::take();
    // HARDWARE: IR must be the first peripheral set up. Do not move it below anything else.
    // Until this runs, GPIO43 (IR-IN) is still the boot ROM's UART0 TX output and fights the TSOP38238
    // output, and GPIO44 (IR-OUT) sits on its weak reset pull-up, which half-turns-on Q1 so the IR LEDs glow.
    // See HARDWARE.md "IR".
    let ir = ir::Ir::new(board.ir.tx, board.ir.rx);
    leds::init(board.leds);
    let nvs_partition = EspDefaultNvsPartition::take().unwrap();
    let nvs = EspNvs::new(nvs_partition.clone(), NVS_NAMESPACE, true).unwrap();
    let spi = init_spi(board.spi);
    let display_spi = spi_device(&spi, board.display.cs, DISPLAY_SPI_MHZ);
    let lora_spi = spi_device(&spi, board.lora.cs, LORA_SPI_MHZ);
    let mut display_buffer = [0u8; DISPLAY_BUFFER_SIZE];
    let display = init_display(display_spi, board.display.data_command, &mut display_buffer);
    let (mut radio, radio_ok) = init_radio(lora_spi, board.lora.rf_switch_line, board.lora.rx_enable);
    // board_id is fixed (from the MAC) and keys the hunt and badge swap; name is the editable nametag name.
    let board_id = board_name::read();
    let nametag = nametag::NametagApp::load(&nvs, &board_id);
    let name = nametag.lora_name().to_string();
    log::info!("board id: {board_id}, username: {name}");
    elf::register(&name);
    let chat = init_chat(&nvs, &mut radio, radio_ok, &name);
    let hunt = hunt::Hunt::new(&board_id, &nvs_partition);
    let mut badge = Badge {
        display,
        radio,
        radio_ok,
        chat,
        ir,
        irapp: irapp::IrApp::new(&nvs),
        sd: sd::SdCard::new(),
        explorer: explorer::Explorer::new(),
        phrack: phrack::Phrack::new(),
        ledapp: ledapp::LedApp::new(&nvs),
        matrix: matrix::Matrix::new(),
        nametag,
        sponsors: sponsors::Sponsors::new(),
        hunt,
        wifi: wifi::WifiApp::new(),
        swap: hunt::SwapApp::new(&board_id),
        typed: String::new(),
        keys: Keys::new(board.keys),
        nvs,
        open: None,
        selected: 0,
        name,
        about_presses: 0,
    };
    badge.take_swap_unlock();
    log_ir_self_check();
    intro::play(&mut badge.display, &mut badge.keys);
    badge.run();
}

fn init_spi(pins: SharedSpi) -> SpiDriver<'static> {
    SpiDriver::new(pins.bus, pins.sck, pins.mosi, Some(pins.miso), &DriverConfig::new()).unwrap()
}

fn spi_device<'a>(spi: &'a SpiDriver<'a>, cs: impl OutputPin + 'a, mhz: u32) -> SpiDeviceDriver<'a, &'a SpiDriver<'a>> {
    SpiDeviceDriver::new(spi, Some(cs), &Config::new().baudrate(mhz.MHz().into())).unwrap()
}

fn init_display<'a>(
    spi: SpiDeviceDriver<'a, &'a SpiDriver<'a>>,
    data_command: impl OutputPin + 'a,
    buffer: &'a mut [u8],
) -> impl DrawTarget<Color = Rgb565> + 'a {
    let data_command = PinDriver::output(data_command).unwrap();
    Builder::new(ILI9341Rgb565, SpiInterface::new(spi, data_command, buffer))
        .color_order(ColorOrder::Bgr)
        .orientation(DISPLAY_ORIENTATION)
        .init(&mut Delay::new_default())
        .unwrap()
}

fn init_radio<'a>(
    spi: SpiDeviceDriver<'a, &'a SpiDriver<'a>>,
    rf_switch_line: impl InputPin + 'a,
    rx_enable: impl OutputPin + 'a,
) -> (sx126x::Radio<'a>, bool) {
    // HARDWARE: never drive this as an output. Module pin 5 is tied to the SX1262's DIO2 output (see HARDWARE.md).
    let rf_switch_line = PinDriver::input(rf_switch_line, Pull::Floating).unwrap();
    let mut radio = sx126x::Radio::new(spi, rf_switch_line, PinDriver::output(rx_enable).unwrap());
    let found = radio.detect();
    (radio, found)
}

fn init_chat(nvs: &EspNvs<NvsDefault>, radio: &mut sx126x::Radio, radio_ok: bool, name: &str) -> chat::Chat {
    let chat = chat::Chat::new(chat::Config::load(nvs, name.to_string()), radio_ok, nvs);
    if radio_ok {
        radio.configure(&chat.config.radio);
    }
    chat
}

fn log_ir_self_check() {
    for (code, expected) in IR_SELF_CHECK {
        log::info!("ir self-check: {} (expected {expected})", ir::describe(&ir::encode(&code).0));
    }
}

impl<D: DrawTarget<Color = Rgb565>> Badge<'_, D> {
    fn run(&mut self) -> ! {
        self.show_menu();
        loop {
            self.step();
            FreeRtos::delay_ms(LOOP_DELAY_MS);
        }
    }

    fn step(&mut self) {
        self.hunt.background_tick();
        self.poll_radio();
        match self.open {
            Some(App::IrBlaster) => self.irapp.tick(&mut self.display, &mut self.ir),
            Some(App::Wifi) => self.wifi.tick(&mut self.display),
            Some(App::Swap) => self.swap.tick(&mut self.display, &mut self.ir),
            Some(App::Chat) => self.chat.tick(&mut self.display),
            Some(App::Matrix) => self.matrix.tick(&mut self.display),
            _ => {}
        }
        let caps_before = self.keys.caps_lock();
        for k in self.keys.pressed() {
            let shift = self.keys.caps_lock();
            self.handle_key(k, shift);
        }
        let caps = self.keys.caps_lock();
        if caps != caps_before {
            if self.open == Some(App::Matrix) {
                ui::remember_caps_lock(caps);
            } else {
                ui::set_caps_lock(&mut self.display, caps);
            }
        }
    }

    fn poll_radio(&mut self) {
        if !self.radio_ok {
            return;
        }
        if let Some(packet) = self.radio.try_receive() {
            if self.chat.receive(packet) && self.open == Some(App::Chat) {
                self.chat.redraw_messages(&mut self.display);
            }
        }
    }

    fn handle_key(&mut self, k: Key, shift: bool) {
        let leave = match self.open {
            None => {
                self.menu_key(k, shift);
                false
            }
            Some(App::Hunt) => {
                let leave = self.hunt.key(k, shift, &mut self.display);
                self.take_swap_unlock();
                leave
            }
            Some(App::Wifi) => self.wifi.key(k, shift, &mut self.display),
            Some(App::Swap) => self.swap.key(k),
            Some(App::Chat) => self.chat.key(k, shift, &mut self.display, &mut self.radio, &self.nvs),
            Some(App::IrBlaster) => self.ir_key(k),
            Some(App::Files) => self.explorer.key(k, &mut self.display, &mut self.sd, &mut self.keys),
            Some(App::Phrack) => self.phrack.key(k, shift, &mut self.display),
            Some(App::Leds) => self.ledapp.key(k, shift, &mut self.display, &self.nvs),
            Some(App::Matrix) => self.matrix.key(k),
            Some(App::Nametag) => {
                let leave = self.nametag.key(k, shift, &mut self.display, &self.nvs);
                self.take_rename();
                leave
            }
            Some(App::Sponsors) => self.sponsors.key(k, &mut self.display),
            Some(App::About) => self.about_key(k),
        };
        if leave {
            self.show_menu();
        }
    }

    fn take_rename(&mut self) {
        if let Some(name) = self.nametag.take_renamed() {
            self.chat.set_nick(&name);
            elf::set_name(&name);
            self.name = name;
        }
    }

    fn take_swap_unlock(&mut self) {
        if let Some(code) = self.hunt.take_swap() {
            self.swap.unlock(code);
        }
    }

    fn ir_key(&mut self, k: Key) -> bool {
        let keys = &mut self.keys;
        let mut abort = || keys.pressed().iter().any(|k| matches!(*k, keys::B | keys::ESC));
        self.irapp.key(k, &mut self.display, &mut self.ir, &self.nvs, &mut self.sd, &mut abort)
    }

    fn menu_key(&mut self, k: Key, shift: bool) {
        match k {
            keys::UP | keys::DOWN => self.move_selection(k),
            k if keys::is_go(k) => self.open_app(MENU_APPS[self.selected]),
            _ => self.type_secret(k, shift),
        }
    }

    fn type_secret(&mut self, k: Key, shift: bool) {
        let Some(c) = keys::char_for(k, shift) else { return };
        self.typed.push(c.to_ascii_lowercase());
        let keep_from = self.typed.len().saturating_sub(hunt::SECRET_WORD.len());
        self.typed.drain(..keep_from);
        if self.typed == hunt::SECRET_WORD {
            self.typed.clear();
            self.open_app(App::Hunt);
        }
    }

    fn move_selection(&mut self, k: Key) {
        let old = self.selected;
        self.selected = keys::wrap_selection(k, old, MENU_ITEMS.len()).unwrap();
        ui::menu_select(&mut self.display, &MENU_ITEMS, old, self.selected, MENU_HINT);
    }

    fn open_app(&mut self, app: App) {
        self.open = Some(app);
        match app {
            App::Hunt => self.hunt.draw(&mut self.display),
            App::Chat => self.chat.draw(&mut self.display),
            App::IrBlaster => self.irapp.draw(&mut self.display),
            App::Wifi => self.wifi.open(&mut self.display),
            App::Swap => self.swap.draw(&mut self.display),
            App::Files => self.explorer.open(&mut self.display, &mut self.sd),
            App::Phrack => self.phrack.open(&mut self.display, &mut self.sd),
            App::Leds => self.ledapp.draw(&mut self.display),
            App::Matrix => self.matrix.open(&mut self.display),
            App::Nametag => self.nametag.open(&mut self.display),
            App::Sponsors => self.sponsors.draw(&mut self.display),
            App::About => self.show_about(),
        }
    }

    fn show_menu(&mut self) {
        self.open = None;
        ui::menu(&mut self.display, &format!("// {}", self.name), &MENU_ITEMS, self.selected, MENU_HINT);
    }

    fn about_key(&mut self, k: Key) -> bool {
        if k != keys::A {
            self.about_presses = 0;
            return keys::is_back(k);
        }
        self.about_presses = self.about_presses.saturating_add(1);
        if self.about_presses == JAFFALOT_PRESSES {
            self.show_jaffalot();
        }
        false
    }

    fn show_jaffalot(&mut self) {
        splash::jaffalot_page(&mut self.display, ui::BACK_HINT);
    }

    fn show_about(&mut self) {
        self.about_presses = 0;
        ui::text_page(&mut self.display, ABOUT_TITLE, &ABOUT_LINES, ui::BACK_HINT);
    }
}
