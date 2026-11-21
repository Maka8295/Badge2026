// Saved signals <-> SD card, as Flipper Zero .ir files.
// Export writes every filled slot as a raw button named Slot_N into a new file under /sdcard/ir
// (never overwriting an earlier export). Import reads any .ir file, raw or parsed, into empty slots
// only: Slot_N goes back to slot N when that slot is free, everything else fills the first free slot.
// Slots always replay at 38 kHz, so parsed codes for other carriers (RC5 36 kHz, SIRC 40 kHz) are
// sent slightly off-carrier; most receivers still take them.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use esp_idf_svc::nvs::{EspNvs, NvsDefault};

use super::slots::{self, SLOT_COUNT};
use super::{IrApp, REPLAY_CARRIER_HZ};
use crate::ir::flipper_file::{self, NAME_KEY};
use crate::ir::{self, Signal, MAX_PAIRS};
use crate::sd::{CardState, SdCard, MOUNT_POINT};

const EXPORT_DIR: &str = "ir";
const EXPORT_STEM: &str = "learned";
const EXPORT_EXTENSION: &str = "ir";
const MAX_EXPORT_FILES: usize = 99;
const FILE_HEADER: &str = "Filetype: IR signals file\nVersion: 1\n";
const DUTY_CYCLE: &str = "0.330000";
const SLOT_PREFIX: &str = "slot";

pub struct ImportResult {
    pub imported: usize,
    // No free slot, or NVS full.
    pub no_room: usize,
    // Records the badge can't turn into a signal (unknown protocol, too long).
    pub unusable: usize,
}

pub enum ExportError {
    Card(&'static str),
    NothingSaved,
    Io(io::Error),
}

pub fn card_problem(sd: &mut SdCard) -> Option<&'static str> {
    match sd.ensure_mounted() {
        CardState::Ready => None,
        CardState::NoCard => Some("No SD card"),
        CardState::NoFilesystem => Some("Card not FAT32, format it in Files"),
    }
}

impl IrApp {
    // Returns the written path relative to the card, like "ir/learned-2.ir", and the signal count.
    pub(super) fn export_slots(&self, sd: &mut SdCard) -> Result<(String, usize), ExportError> {
        if let Some(problem) = card_problem(sd) {
            return Err(ExportError::Card(problem));
        }
        let filled: Vec<(usize, &Signal)> =
            self.slots.iter().enumerate().filter_map(|(i, slot)| slot.as_ref().map(|s| (i, s))).collect();
        if filled.is_empty() {
            return Err(ExportError::NothingSaved);
        }
        let dir = Path::new(MOUNT_POINT).join(EXPORT_DIR);
        fs::create_dir_all(&dir).map_err(ExportError::Io)?;
        let path = free_export_path(&dir).ok_or_else(|| ExportError::Io(io::ErrorKind::AlreadyExists.into()))?;
        let mut file = File::create(&path).map_err(ExportError::Io)?;
        file.write_all(export_text(&filled).as_bytes()).map_err(ExportError::Io)?;
        let relative = path.strip_prefix(MOUNT_POINT).unwrap_or(&path).to_string_lossy().trim_start_matches('/').to_string();
        log::info!("ir: exported {} signals to {relative}", filled.len());
        Ok((relative, filled.len()))
    }

    pub(super) fn import_slots(&mut self, path: &Path, nvs: &EspNvs<NvsDefault>) -> io::Result<ImportResult> {
        let mut result = ImportResult { imported: 0, no_room: 0, unusable: 0 };
        for (name, lines) in records(path)? {
            let Some(signal) = signal_from(&lines) else {
                result.unusable += 1;
                continue;
            };
            let Some(slot) = self.free_slot_for(&name) else {
                result.no_room += 1;
                continue;
            };
            if !slots::save(nvs, slot, &signal) {
                result.no_room += 1;
                continue;
            }
            self.slots[slot] = Some(signal);
            result.imported += 1;
        }
        log::info!(
            "ir: imported {} from {} ({} no room, {} unusable)",
            result.imported,
            path.display(),
            result.no_room,
            result.unusable
        );
        Ok(result)
    }

    fn free_slot_for(&self, name: &str) -> Option<usize> {
        let preferred = slot_number(name).filter(|&slot| self.slots[slot].is_none());
        preferred.or_else(|| (0..SLOT_COUNT).find(|&slot| self.slots[slot].is_none()))
    }
}

fn free_export_path(dir: &Path) -> Option<PathBuf> {
    (1..=MAX_EXPORT_FILES)
        .map(|n| match n {
            1 => dir.join(format!("{EXPORT_STEM}.{EXPORT_EXTENSION}")),
            n => dir.join(format!("{EXPORT_STEM}-{n}.{EXPORT_EXTENSION}")),
        })
        .find(|path| !path.exists())
}

fn export_text(filled: &[(usize, &Signal)]) -> String {
    let mut text = FILE_HEADER.to_string();
    for (slot, signal) in filled {
        // Flipper raw data is mark space mark ... ending on a mark; the trailing gap is implied.
        let mut durations: Vec<u16> = signal.iter().flat_map(|&(mark, space)| [mark, space]).collect();
        durations.pop();
        let data: Vec<String> = durations.iter().map(u16::to_string).collect();
        text += &format!(
            "# \n{NAME_KEY}: Slot_{}\ntype: raw\nfrequency: {REPLAY_CARRIER_HZ}\nduty_cycle: {DUTY_CYCLE}\ndata: {}\n",
            slot + 1,
            data.join(" ")
        );
    }
    text
}

// Each record's name and its lines, in file order. Records start after a "#" line.
fn records(path: &Path) -> io::Result<Vec<(String, Vec<String>)>> {
    let mut out = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut finish = |lines: &mut Vec<String>| {
        let name = lines.iter().find_map(|line| flipper_file::field(line).filter(|(k, _)| *k == NAME_KEY).map(|(_, v)| v.to_string()));
        if let Some(name) = name {
            out.push((name, std::mem::take(lines)));
        }
        lines.clear();
    };
    for line in BufReader::new(File::open(path)?).lines() {
        let line = line?;
        if flipper_file::is_separator(&line) {
            finish(&mut lines);
        } else {
            lines.push(line);
        }
    }
    finish(&mut lines);
    Ok(out)
}

fn signal_from(lines: &[String]) -> Option<Signal> {
    let record = flipper_file::parse_record(lines.iter())?;
    let (signal, _carrier_hz) = ir::encode(&record.code());
    (!signal.is_empty() && signal.len() <= MAX_PAIRS).then_some(signal)
}

// "Slot_7", "slot 7" or "SLOT7" -> slot index 6.
fn slot_number(name: &str) -> Option<usize> {
    let lower = name.to_lowercase();
    let digits = lower.strip_prefix(SLOT_PREFIX)?.trim_start_matches(['_', ' ', '-']);
    let n: usize = digits.parse().ok()?;
    (1..=SLOT_COUNT).contains(&n).then(|| n - 1)
}
