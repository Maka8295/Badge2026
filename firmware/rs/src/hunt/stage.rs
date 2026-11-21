const FIELD_HEADER_LEN: usize = 5;
const FIELD_CLUE_LINE: u8 = 1;
const FIELD_SERIAL: u8 = 2;
const FIELD_MORSE: u8 = 3;
const FIELD_NVS: u8 = 4;
const FIELD_PROGRAM: u8 = 5;
const FIELD_IR_SWAP: u8 = 7;
const FIELD_IR_REMOTE: u8 = 8;
const FIELD_FINISH: u8 = 9;

pub enum Action {
    Serial(String),
    Morse(String),
    Nvs(Vec<u8>),
    Program(Vec<u8>),
    IrSwap(SwapCode),
    IrRemote { name: String, text: String },
    Finish,
}

pub struct SwapCode {
    pub badges: usize,
    pub secret: String,
}

#[derive(Default)]
pub struct Stage {
    pub clue: Vec<String>,
    pub actions: Vec<Action>,
}

impl Stage {
    pub fn unlocks(&self) -> bool {
        self.actions.iter().any(|a| matches!(a, Action::IrSwap(_) | Action::IrRemote { .. }))
    }

    pub fn parse(mut data: &[u8]) -> Self {
        let mut stage = Stage::default();
        while data.len() >= FIELD_HEADER_LEN {
            let kind = data[0];
            let len = u32::from_le_bytes(data[1..FIELD_HEADER_LEN].try_into().unwrap()) as usize;
            let (value, rest) = data[FIELD_HEADER_LEN..].split_at(len);
            stage.add(kind, value);
            data = rest;
        }
        stage
    }

    fn add(&mut self, kind: u8, value: &[u8]) {
        let text = || String::from_utf8_lossy(value).into_owned();
        let action = match kind {
            FIELD_CLUE_LINE => return self.clue.push(text()),
            FIELD_SERIAL => Action::Serial(text()),
            FIELD_MORSE => Action::Morse(text()),
            FIELD_NVS => Action::Nvs(value.to_vec()),
            FIELD_PROGRAM => Action::Program(value.to_vec()),
            FIELD_IR_SWAP => ir_swap(&text()),
            FIELD_IR_REMOTE => ir_remote(&text()),
            FIELD_FINISH => Action::Finish,
            _ => return log::warn!("hunt: unknown field {kind}"),
        };
        self.actions.push(action);
    }
}

fn two_lines(text: &str) -> (&str, &str) {
    text.split_once('\n').unwrap_or((text, ""))
}

fn ir_swap(text: &str) -> Action {
    let (badges, secret) = two_lines(text);
    Action::IrSwap(SwapCode { badges: badges.parse().unwrap_or_default(), secret: secret.into() })
}

fn ir_remote(text: &str) -> Action {
    let (name, text) = two_lines(text);
    Action::IrRemote { name: name.into(), text: text.into() }
}
