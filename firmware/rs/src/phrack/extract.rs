use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use flate2::bufread::GzDecoder;

use super::tar::{self, BLOCK_BYTES};

pub const DONE_MARK: &str = ".done";
const INPUT_BUFFER_BYTES: usize = 4096;
const COPY_CHUNK_BYTES: usize = 4096;

pub fn extract(archive: impl Read, dir: &Path) -> io::Result<usize> {
    fs::create_dir_all(dir)?;
    let mut stream = GzDecoder::new(BufReader::with_capacity(INPUT_BUFFER_BYTES, archive));
    let mut header = [0u8; BLOCK_BYTES];
    let mut files = 0;
    loop {
        stream.read_exact(&mut header)?;
        let Some(entry) = tar::parse(&header)? else { break };
        match entry.file_name {
            Some(name) => {
                copy_exact(&mut stream, entry.size, &mut File::create(dir.join(name))?)?;
                files += 1;
            }
            None => copy_exact(&mut stream, entry.size, &mut io::sink())?,
        }
        copy_exact(&mut stream, tar::padded(entry.size) - entry.size, &mut io::sink())?;
    }
    File::create(dir.join(DONE_MARK))?;
    Ok(files)
}

fn copy_exact(from: &mut impl Read, bytes: u64, to: &mut impl Write) -> io::Result<()> {
    let mut chunk = vec![0u8; COPY_CHUNK_BYTES];
    let mut left = bytes;
    while left > 0 {
        let len = left.min(COPY_CHUNK_BYTES as u64) as usize;
        from.read_exact(&mut chunk[..len])?;
        to.write_all(&chunk[..len])?;
        left -= len as u64;
    }
    Ok(())
}
