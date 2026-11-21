use std::io;
use std::ops::Range;
use std::path::Path;

pub const BLOCK_BYTES: usize = 512;
const NAME: Range<usize> = 0..100;
const SIZE: Range<usize> = 124..136;
const TYPE: usize = 156;
const REGULAR_FILE_TYPES: [u8; 2] = [b'0', 0];
const OCTAL: u32 = 8;
const HIDDEN_PREFIX: char = '.';

pub struct Entry {
    pub file_name: Option<String>,
    pub size: u64,
}

pub fn parse(header: &[u8]) -> io::Result<Option<Entry>> {
    if header.len() < BLOCK_BYTES {
        return Err(bad_header());
    }
    if header.iter().all(|&b| b == 0) {
        return Ok(None);
    }
    let file_name = file_name(header).filter(|_| REGULAR_FILE_TYPES.contains(&header[TYPE]));
    Ok(Some(Entry { file_name, size: entry_size(header)? }))
}

pub fn padded(size: u64) -> u64 {
    size.next_multiple_of(BLOCK_BYTES as u64)
}

pub fn files(tar: &[u8]) -> io::Result<Vec<(String, &[u8])>> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(entry) = parse(tar.get(at..).unwrap_or_default())? {
        let data = at + BLOCK_BYTES;
        let end = data.checked_add(entry.size as usize).filter(|&end| end <= tar.len()).ok_or_else(bad_header)?;
        if let Some(name) = entry.file_name {
            found.push((name, &tar[data..end]));
        }
        at = data + padded(entry.size) as usize;
    }
    Ok(found)
}

fn entry_size(header: &[u8]) -> io::Result<u64> {
    let field = std::str::from_utf8(&header[SIZE]).map_err(|_| bad_header())?;
    u64::from_str_radix(field.trim_matches([' ', '\0']), OCTAL).map_err(|_| bad_header())
}

fn file_name(header: &[u8]) -> Option<String> {
    let raw = header[NAME].split(|&b| b == 0).next()?;
    let name = Path::new(std::str::from_utf8(raw).ok()?).file_name()?.to_str()?;
    (!name.starts_with(HIDDEN_PREFIX)).then(|| name.to_string())
}

fn bad_header() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "bad tar header")
}
