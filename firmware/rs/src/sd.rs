use std::ffi::CStr;

use esp_idf_svc::fs::fatfs::Fatfs;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::sd::mmc::{SdMmcHostConfiguration, SdMmcHostDriver};
use esp_idf_svc::hal::sd::{SdCardConfiguration, SdCardDriver};
use esp_idf_svc::io::vfs::MountedFatfs;
use esp_idf_svc::sys::{self, EspError, FRESULT, MKFS_PARM};

use crate::board;

pub const MOUNT_POINT: &str = "/sdcard";
const DRIVE: u8 = 0;
const DRIVE_PATH: &CStr = c"0:";
const MAX_OPEN_FILES: usize = 4;
const NO_FILESYSTEM_ERRNO: i32 = sys::ENODEV as i32;
const FAT_COPIES: u8 = 1;
const CLUSTER_BYTES: u32 = 32 * 1024;
const FORMAT_WORK_BUFFER_BYTES: usize = 16 * 1024;

type Mounted = MountedFatfs<Fatfs<SdCardDriver<SdMmcHostDriver<'static>>>>;

#[derive(Clone, Copy, PartialEq)]
pub enum CardState {
    Ready,
    NoCard,
    NoFilesystem,
}

#[derive(Debug)]
pub enum FormatError {
    NoCard,
    FatFs(FRESULT),
    OutOfMemory,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::NoCard => write!(f, "no card"),
            FormatError::FatFs(code) => write!(f, "format failed ({code})"),
            FormatError::OutOfMemory => write!(f, "out of memory"),
        }
    }
}

pub struct SdCard {
    mounted: Option<Mounted>,
}

impl SdCard {
    pub fn new() -> Self {
        Self { mounted: None }
    }

    pub fn ensure_mounted(&mut self) -> CardState {
        if self.mounted.is_none() {
            self.mounted = mount().inspect_err(|e| log::warn!("sd card mount failed: {e}")).ok();
        }
        if self.mounted.is_none() {
            return CardState::NoCard;
        }
        self.check_filesystem()
    }

    pub fn format_fat32(&mut self) -> Result<(), FormatError> {
        if self.ensure_mounted() == CardState::NoCard {
            return Err(FormatError::NoCard);
        }
        make_fat32()?;
        log::info!("sd card formatted as FAT32");
        Ok(())
    }

    fn check_filesystem(&mut self) -> CardState {
        match std::fs::read_dir(MOUNT_POINT) {
            Ok(_) => CardState::Ready,
            Err(e) if e.raw_os_error() == Some(NO_FILESYSTEM_ERRNO) => CardState::NoFilesystem,
            Err(e) => {
                log::warn!("sd card lost: {e}");
                self.mounted = None;
                CardState::NoCard
            }
        }
    }
}

fn mount() -> Result<Mounted, EspError> {
    let pins = unsafe { board::steal_sd_pins() };
    let host = SdMmcHostDriver::new_4bits(
        pins.slot,
        pins.cmd,
        pins.clk,
        pins.d0,
        pins.d1,
        pins.d2,
        pins.d3,
        None::<AnyIOPin>,
        None::<AnyIOPin>,
        &SdMmcHostConfiguration::new(),
    )?;
    let card = SdCardDriver::new_mmc(host, &SdCardConfiguration::new())?;
    MountedFatfs::mount(Fatfs::new_sdcard(DRIVE, card)?, MOUNT_POINT, MAX_OPEN_FILES)
}

fn make_fat32() -> Result<(), FormatError> {
    let params = MKFS_PARM { fmt: sys::FM_FAT32 as u8, n_fat: FAT_COPIES, align: 0, n_root: 0, au_size: CLUSTER_BYTES };
    let mut work = Vec::new();
    work.try_reserve_exact(FORMAT_WORK_BUFFER_BYTES).map_err(|_| FormatError::OutOfMemory)?;
    work.resize(FORMAT_WORK_BUFFER_BYTES, 0u8);
    let result = unsafe { sys::f_mkfs(DRIVE_PATH.as_ptr(), &params, work.as_mut_ptr().cast(), work.len() as u32) };
    match result {
        sys::FRESULT_FR_OK => Ok(()),
        code => Err(FormatError::FatFs(code)),
    }
}
