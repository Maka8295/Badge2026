use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::marker::PhantomData;
use std::path::Path;
use std::ptr;

use esp_idf_svc::sys::{
    esp_elf_deinit, esp_elf_init, esp_elf_relocate, esp_elf_request, esp_elf_t, heap_caps_get_free_size,
    heap_caps_get_largest_free_block, EINVAL, EIO, ENOMEM, ENOSYS, MALLOC_CAP_8BIT,
};

const ELF_MAGIC: &[u8] = b"\x7fELF";
const ELF_HEADER_SIZE: usize = 52;
const MACHINE_OFFSET: usize = 18;
const MACHINE_XTENSA: u16 = 94;
const COPIES_IN_RAM: usize = 2;
const KIB: usize = 1024;
const HEAP_RESERVE: usize = 16 * KIB;
const REQUEST_OPTIONS: i32 = 0;
const NO_ARGUMENTS: i32 = 0;
const OK: i32 = 0;

pub enum Error {
    TooBig { size: usize, free: usize },
    Read(io::Error),
    NotElf,
    WrongMachine,
    Relocate(i32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::TooBig { size, free } => write!(f, "too big: {}K, free {}K", size / KIB, free / KIB),
            Error::Read(e) => write!(f, "read failed: {e}"),
            Error::NotElf => write!(f, "not an ELF file"),
            Error::WrongMachine => write!(f, "not built for Xtensa"),
            Error::Relocate(code) => write!(f, "{} ({code})", relocate_failure(*code)),
        }
    }
}

fn relocate_failure(code: i32) -> &'static str {
    match code.unsigned_abs() {
        ENOSYS => "unknown symbol",
        ENOMEM => "out of memory",
        EINVAL | EIO => "bad ELF layout",
        _ => "relocate failed",
    }
}

pub fn read(path: &Path) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path).map_err(Error::Read)?;
    let size = file.metadata().map_err(Error::Read)?.len() as usize;
    check_fits_in_ram(size)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| too_big(size))?;
    file.read_to_end(&mut bytes).map_err(Error::Read)?;
    check_header(&bytes)?;
    Ok(bytes)
}

fn check_fits_in_ram(size: usize) -> Result<(), Error> {
    let largest_block = unsafe { heap_caps_get_largest_free_block(MALLOC_CAP_8BIT) };
    let room_per_copy = free_heap().saturating_sub(HEAP_RESERVE) / COPIES_IN_RAM;
    if size > largest_block.min(room_per_copy) {
        return Err(too_big(size));
    }
    Ok(())
}

fn too_big(size: usize) -> Error {
    Error::TooBig { size, free: free_heap() }
}

fn free_heap() -> usize {
    unsafe { heap_caps_get_free_size(MALLOC_CAP_8BIT) }
}

fn check_header(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() < ELF_HEADER_SIZE || !bytes.starts_with(ELF_MAGIC) {
        return Err(Error::NotElf);
    }
    let machine = u16::from_le_bytes([bytes[MACHINE_OFFSET], bytes[MACHINE_OFFSET + 1]]);
    if machine != MACHINE_XTENSA {
        return Err(Error::WrongMachine);
    }
    Ok(())
}

pub struct Program<'a> {
    elf: esp_elf_t,
    _file: PhantomData<&'a [u8]>,
}

impl<'a> Program<'a> {
    pub fn load(file: &'a [u8]) -> Result<Self, Error> {
        let mut elf = esp_elf_t::default();
        unsafe { esp_elf_init(&mut elf) };
        let code = unsafe { esp_elf_relocate(&mut elf, file.as_ptr()) };
        if code != OK {
            release_after_failed_relocate(&mut elf, code);
            return Err(Error::Relocate(code));
        }
        Ok(Self { elf, _file: PhantomData })
    }

    pub fn start(&mut self) {
        unsafe { esp_elf_request(&mut self.elf, REQUEST_OPTIONS, NO_ARGUMENTS, ptr::null_mut()) };
    }
}

impl Drop for Program<'_> {
    fn drop(&mut self) {
        unsafe { esp_elf_deinit(&mut self.elf) };
    }
}

fn release_after_failed_relocate(elf: &mut esp_elf_t, code: i32) {
    let loader_already_freed_sections = code == -(ENOMEM as i32);
    if !loader_already_freed_sections {
        unsafe { esp_elf_deinit(elf) };
    }
}
