use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};

use crate::ir::{Button, Code, Protocol, Remote};
use crate::irapp;

const NVS_NAMESPACE: &str = "hunt";
const NVS_KEY: &str = "flag";
const NVS_PROGRAM_KEY: &str = "gift";
const NVS_UNLOCKED_KEY: &str = "unlocked";
const UNLOCKED_MAX_BYTES: usize = 512;
const UNLOCKED_SEPARATOR: char = '\n';
const BYTES_PER_CODE: usize = 4;
const PADDING: u8 = b' ';

pub fn write_nvs_flag(partition: &EspDefaultNvsPartition, flag: &[u8], board_name: &str) {
    save_blob(partition, NVS_KEY, &xor_with_name(flag, board_name));
}

fn xor_with_name(data: &[u8], board_name: &str) -> Vec<u8> {
    data.iter().zip(board_name.bytes().cycle()).map(|(d, k)| d ^ k).collect()
}

pub fn store_program(partition: &EspDefaultNvsPartition, program: &[u8]) {
    save_blob(partition, NVS_PROGRAM_KEY, program);
}

pub fn load_unlocked(partition: &EspDefaultNvsPartition, board_name: &str) -> Vec<String> {
    let mut buf = [0u8; UNLOCKED_MAX_BYTES];
    let nvs = EspNvs::new(partition.clone(), NVS_NAMESPACE, true);
    let Ok(Some(blob)) = nvs.and_then(|nvs| nvs.get_blob(NVS_UNLOCKED_KEY, &mut buf)) else { return Vec::new() };
    let text = String::from_utf8_lossy(&xor_with_name(blob, board_name)).into_owned();
    text.split(UNLOCKED_SEPARATOR).filter(|a| !a.is_empty()).map(String::from).collect()
}

pub fn save_unlocked(partition: &EspDefaultNvsPartition, answers: &[String], board_name: &str) {
    let joined = answers.join(&UNLOCKED_SEPARATOR.to_string());
    if joined.len() > UNLOCKED_MAX_BYTES {
        return log::error!("hunt: unlocked codes do not fit in {UNLOCKED_MAX_BYTES} bytes");
    }
    save_blob(partition, NVS_UNLOCKED_KEY, &xor_with_name(joined.as_bytes(), board_name));
}

fn save_blob(partition: &EspDefaultNvsPartition, key: &str, data: &[u8]) {
    let saved = EspNvs::new(partition.clone(), NVS_NAMESPACE, true).and_then(|nvs| nvs.set_blob(key, data));
    if let Err(e) = saved {
        log::error!("hunt: saving {key}: {e}");
    }
}

pub fn add_text_remote(name: &str, text: &str) {
    irapp::add_remote(|| {
        let buttons: Vec<Button> = text_chunks(text).iter().enumerate().map(|(i, chunk)| button(i, chunk)).collect();
        Remote { name: leak_str(name), buttons: Box::leak(buttons.into_boxed_slice()) }
    });
}

fn text_chunks(text: &str) -> Vec<[u8; BYTES_PER_CODE]> {
    text.as_bytes()
        .chunks(BYTES_PER_CODE)
        .map(|chunk| std::array::from_fn(|i| chunk.get(i).copied().unwrap_or(PADDING)))
        .collect()
}

fn button(index: usize, chunk: &[u8; BYTES_PER_CODE]) -> Button {
    let address = u16::from_le_bytes([chunk[0], chunk[1]]) as u32;
    let command = u16::from_le_bytes([chunk[2], chunk[3]]) as u32;
    let codes = Box::leak(Box::new([Code::Parsed(Protocol::NecExt, address, command)]));
    Button { name: leak_str(&format!("{:02}", index + 1)), codes }
}

fn leak_str(text: &str) -> &'static str {
    Box::leak(text.to_owned().into_boxed_str())
}
