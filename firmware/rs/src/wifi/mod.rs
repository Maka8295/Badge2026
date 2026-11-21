mod radio;

use std::net::UdpSocket;
use std::time::{Duration, Instant};

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::keys::{self, Key};
use crate::ui;
use radio::{Network, Radio};

const TITLE: &str = "// wi-fi";
const LIST_HINT: &str = "UP/DOWN move   A join   B back";
const PASSWORD_HINT: &str = "OK/A join   B back";
const SSID_HINT: &str = "OK/A next   B back";
const SSID_PROMPT: &str = "network name (SSID):";
const HIDDEN_ITEM: &str = "hidden network...";
const SSID_MAX: usize = 32;
const LISTEN_HINT: &str = "B leave";
const WAIT_HINT: &str = "please wait";
const SCANNING: &str = "scanning...";
const PASSWORD_MAX: usize = 63;
const MESSAGE_MAX: usize = 256;
const ANY_ADDRESS: &str = "0.0.0.0";
const MESSAGE_LINES: usize = 6;
const LISTEN_PORT: u16 = 31337;
const SIGNAL_REFRESH: Duration = Duration::from_secs(1);

struct Listen {
    socket: Option<UdpSocket>,
    message: String,
    rssi_dbm: Option<i8>,
    refresh_at: Instant,
}

enum Screen {
    Message(String),
    List,
    Ssid,
    Password,
    Listen(Listen),
}

pub struct WifiApp {
    radio: Option<Radio>,
    networks: Vec<Network>,
    selected: usize,
    ssid: String,
    password: String,
    screen: Screen,
}

impl WifiApp {
    pub fn new() -> Self {
        WifiApp {
            radio: None,
            networks: Vec::new(),
            selected: 0,
            ssid: String::new(),
            password: String::new(),
            screen: Screen::List,
        }
    }

    pub fn open<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        ui::text_page(d, TITLE, &[SCANNING], WAIT_HINT);
        self.screen = match self.start_and_scan() {
            Ok(()) => Screen::List,
            Err(e) => Screen::Message(format!("wifi: {e}")),
        };
        self.draw(d);
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        match &self.screen {
            Screen::Message(text) => ui::text_page(d, TITLE, &[text.as_str()], ui::BACK_HINT),
            Screen::List => ui::menu(d, TITLE, &ui::as_strs(&self.labels()), self.selected, LIST_HINT),
            Screen::Ssid => draw_entry(d, SSID_PROMPT, &self.ssid, SSID_HINT),
            Screen::Password => draw_entry(d, &format!("password for {}:", self.ssid), &self.password, PASSWORD_HINT),
            Screen::Listen(listen) => ui::text_page(d, TITLE, &ui::as_strs(&self.listen_lines(listen)), LISTEN_HINT),
        }
    }

    pub fn tick<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        let Screen::Listen(listen) = &mut self.screen else { return };
        let mut changed = receive_newest(listen);
        if Instant::now() >= listen.refresh_at {
            let rssi_dbm = self.radio.as_ref().and_then(Radio::rssi_dbm);
            changed |= rssi_dbm != listen.rssi_dbm;
            listen.rssi_dbm = rssi_dbm;
            listen.refresh_at = Instant::now() + SIGNAL_REFRESH;
        }
        if changed {
            self.draw(d);
        }
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) -> bool {
        match self.screen {
            Screen::List => return self.list_key(k, d),
            Screen::Ssid => self.ssid_key(k, shift, d),
            Screen::Password => self.password_key(k, shift, d),
            Screen::Listen(_) if keys::is_back(k) => self.leave_network(d),
            Screen::Message(_) if keys::is_back(k) && self.radio.is_some() => self.show(Screen::List, d),
            Screen::Message(_) if keys::is_back(k) => return self.close(),
            _ => {}
        }
        false
    }

    fn start_and_scan(&mut self) -> Result<(), esp_idf_svc::sys::EspError> {
        let radio = match self.radio.as_mut() {
            Some(radio) => radio,
            None => self.radio.insert(Radio::start()?),
        };
        self.networks = radio.scan()?;
        self.selected = 0;
        Ok(())
    }

    fn close(&mut self) -> bool {
        self.radio = None;
        self.screen = Screen::List;
        true
    }

    fn labels(&self) -> Vec<String> {
        let networks = self.networks.iter().map(|n| format!("{}  {} dBm{}", n.ssid, n.rssi_dbm, if n.open { "" } else { " *" }));
        networks.chain([HIDDEN_ITEM.to_string()]).collect()
    }

    fn list_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) -> bool {
        if keys::is_back(k) {
            return self.close();
        }
        if let Some(next) = keys::wrap_selection(k, self.selected, self.networks.len() + 1) {
            ui::menu_select(d, &ui::as_strs(&self.labels()), self.selected, next, LIST_HINT);
            self.selected = next;
        } else if keys::is_go(k) {
            self.pick(d);
        }
        false
    }

    fn pick<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        self.password.clear();
        match self.networks.get(self.selected) {
            None => {
                self.ssid.clear();
                self.show(Screen::Ssid, d);
            }
            Some(network) if network.open => {
                self.ssid = network.ssid.clone();
                self.join(d);
            }
            Some(network) => {
                self.ssid = network.ssid.clone();
                self.show(Screen::Password, d);
            }
        }
    }

    fn ssid_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) {
        match k {
            keys::B | keys::ESC => self.show(Screen::List, d),
            k if keys::is_submit(k) && !self.ssid.is_empty() => self.show(Screen::Password, d),
            _ => edit(&mut self.ssid, k, shift, SSID_MAX, d),
        }
    }

    fn password_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) {
        match k {
            keys::B | keys::ESC => self.show(Screen::List, d),
            k if keys::is_submit(k) => self.join(d),
            _ => edit(&mut self.password, k, shift, PASSWORD_MAX, d),
        }
    }

    fn join<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        ui::text_page(d, TITLE, &[&format!("joining {}...", self.ssid)], WAIT_HINT);
        let Some(radio) = self.radio.as_mut() else { return };
        let screen = match radio.connect(&self.ssid, &self.password) {
            Ok(()) => Screen::Listen(Listen { socket: bind(LISTEN_PORT), message: String::new(), rssi_dbm: None, refresh_at: Instant::now() }),
            Err(e) => Screen::Message(format!("could not join: {e}")),
        };
        self.show(screen, d);
    }

    fn leave_network<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        if let Some(radio) = self.radio.as_mut() {
            radio.disconnect();
        }
        self.show(Screen::List, d);
    }

    fn show<D: DrawTarget<Color = Rgb565>>(&mut self, screen: Screen, d: &mut D) {
        self.screen = screen;
        self.draw(d);
    }

    fn listen_lines(&self, listen: &Listen) -> Vec<String> {
        let ip = self.radio.as_ref().and_then(Radio::ip).map_or("?".into(), |ip| ip.to_string());
        let signal = listen.rssi_dbm.map_or("?".into(), |rssi| rssi.to_string());
        let mut lines = vec![
            format!("on {}", self.ssid),
            format!("my ip: {ip}"),
            format!("signal: {signal} dBm"),
            format!("udp {LISTEN_PORT}:"),
        ];
        let chars: Vec<char> = listen.message.chars().collect();
        lines.extend(chars.chunks(ui::TEXT_COLUMNS).take(MESSAGE_LINES).map(|chunk| chunk.iter().collect::<String>()));
        lines
    }
}

fn draw_entry<D: DrawTarget<Color = Rgb565>>(d: &mut D, prompt: &str, text: &str, hint: &str) {
    ui::text_page(d, TITLE, &[prompt], hint);
    ui::input_box(d, text);
}

fn edit<D: DrawTarget<Color = Rgb565>>(text: &mut String, k: Key, shift: bool, max: usize, d: &mut D) {
    if k == keys::DEL {
        text.pop();
    } else if let Some(c) = keys::char_for(k, shift).filter(|_| text.len() < max) {
        text.push(c);
    } else {
        return;
    }
    ui::input_box(d, text);
}

fn bind(port: u16) -> Option<UdpSocket> {
    let socket = UdpSocket::bind((ANY_ADDRESS, port)).and_then(|s| s.set_nonblocking(true).map(|_| s));
    socket.inspect_err(|e| log::error!("udp bind: {e}")).ok()
}

fn receive_newest(listen: &mut Listen) -> bool {
    let Some(socket) = &listen.socket else { return false };
    let mut buf = [0u8; MESSAGE_MAX];
    let mut changed = false;
    while let Ok((len, sender)) = socket.recv_from(&mut buf) {
        let message = format!("{}: {}", sender.ip(), String::from_utf8_lossy(&buf[..len]).trim_end());
        changed |= message != listen.message;
        listen.message = message;
    }
    changed
}
