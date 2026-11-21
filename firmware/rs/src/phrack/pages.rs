use std::io::{self, Read, Seek, SeekFrom};

use super::library::Source;

pub const MAX_VIEW_LINES: usize = 32;
const LINES_PER_CHECKPOINT: usize = 32;
pub const MAX_LINE_BYTES: usize = 200;
const BLOCK_BYTES: u64 = ((LINES_PER_CHECKPOINT + MAX_VIEW_LINES) * MAX_LINE_BYTES) as u64;
const INDEX_CHUNK_BYTES: usize = 4096;
const TAB_STOP: usize = 8;
const NEWLINE: u8 = b'\n';
const UNPRINTABLE: char = '?';

#[derive(Default)]
struct LineBreaker {
    len: usize,
}

impl LineBreaker {
    fn ends_line(&mut self, b: u8) -> bool {
        if b == NEWLINE || self.len + 1 == MAX_LINE_BYTES {
            self.len = 0;
            return true;
        }
        self.len += 1;
        false
    }
}

pub struct Pages {
    source: Box<dyn Source>,
    checkpoints: Vec<u64>,
    line_count: usize,
    text_bytes: u64,
    block: Vec<u8>,
    block_checkpoint: Option<usize>,
}

impl Pages {
    pub fn open(mut source: Box<dyn Source>) -> io::Result<Self> {
        let (checkpoints, line_count, text_bytes) = index(&mut source)?;
        Ok(Self { source, checkpoints, line_count, text_bytes, block: Vec::new(), block_checkpoint: None })
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn lines(&mut self, first: usize, count: usize) -> io::Result<Vec<String>> {
        let count = count.min(MAX_VIEW_LINES);
        let checkpoint = first / LINES_PER_CHECKPOINT;
        self.load_block(checkpoint)?;
        let lines = split_lines(&self.block, self.block_reaches_end());
        Ok(lines.into_iter().skip(first % LINES_PER_CHECKPOINT).take(count).map(shown_line).collect())
    }

    fn load_block(&mut self, checkpoint: usize) -> io::Result<()> {
        if self.block_checkpoint == Some(checkpoint) {
            return Ok(());
        }
        let start = self.checkpoints.get(checkpoint).copied().unwrap_or(self.text_bytes);
        self.source.seek(SeekFrom::Start(start))?;
        self.block.clear();
        (&mut self.source).take(BLOCK_BYTES).read_to_end(&mut self.block)?;
        self.block_checkpoint = Some(checkpoint);
        Ok(())
    }

    fn block_reaches_end(&self) -> bool {
        (self.block.len() as u64) < BLOCK_BYTES
    }
}

fn index(source: &mut impl Read) -> io::Result<(Vec<u64>, usize, u64)> {
    let mut breaker = LineBreaker::default();
    let mut checkpoints = vec![0];
    let mut lines = 0;
    let mut offset = 0u64;
    let mut chunk = vec![0u8; INDEX_CHUNK_BYTES];
    loop {
        let n = source.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        for &b in &chunk[..n] {
            offset += 1;
            if breaker.ends_line(b) {
                lines += 1;
                if lines % LINES_PER_CHECKPOINT == 0 {
                    checkpoints.push(offset);
                }
            }
        }
    }
    let unfinished_last_line = usize::from(breaker.len > 0);
    Ok((checkpoints, lines + unfinished_last_line, offset))
}

fn split_lines(block: &[u8], reaches_end: bool) -> Vec<&[u8]> {
    let mut breaker = LineBreaker::default();
    let mut lines = Vec::new();
    let mut start = 0;
    for (i, &b) in block.iter().enumerate() {
        if breaker.ends_line(b) {
            lines.push(&block[start..=i]);
            start = i + 1;
        }
    }
    if reaches_end && start < block.len() {
        lines.push(&block[start..]);
    }
    lines
}

fn shown_line(raw: &[u8]) -> String {
    let mut line = String::new();
    for c in String::from_utf8_lossy(raw).chars() {
        match c {
            '\t' => line.extend(std::iter::repeat_n(' ', TAB_STOP - line.len() % TAB_STOP)),
            '\r' | '\n' => {}
            ' '..='~' => line.push(c),
            _ => line.push(UNPRINTABLE),
        }
    }
    line
}
