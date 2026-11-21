use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::ir::flipper_file::{self, Record, NAME_KEY};

pub struct IndexedButton {
    pub name: String,
    pub offsets: Vec<u32>,
}

pub struct IrFile {
    pub path: PathBuf,
    pub buttons: Vec<IndexedButton>,
}

#[derive(Default)]
struct Index {
    buttons: Vec<IndexedButton>,
    record_start: u32,
    record_name: Option<String>,
}

impl Index {
    fn add_line(&mut self, line: &str, next_line_offset: u32) {
        if flipper_file::is_separator(line) {
            self.end_record();
            self.record_start = next_line_offset;
        } else if let Some((key, name)) = flipper_file::field(line) {
            if key == NAME_KEY {
                self.record_name = Some(name.to_string());
            }
        }
    }

    fn end_record(&mut self) {
        let Some(name) = self.record_name.take() else { return };
        match self.buttons.iter_mut().find(|button| button.name == name) {
            Some(button) => button.offsets.push(self.record_start),
            None => self.buttons.push(IndexedButton { name, offsets: vec![self.record_start] }),
        }
    }

    fn finish(mut self) -> Vec<IndexedButton> {
        self.end_record();
        self.buttons
    }
}

pub fn index(path: &Path) -> io::Result<IrFile> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut index = Index::default();
    let mut line = String::new();
    let mut offset = 0u32;
    while reader.read_line(&mut line)? > 0 {
        offset += line.len() as u32;
        index.add_line(&line, offset);
        line.clear();
    }
    Ok(IrFile { path: path.to_path_buf(), buttons: index.finish() })
}

pub fn read_record(reader: &mut BufReader<File>, offset: u32) -> io::Result<Option<Record>> {
    reader.seek(SeekFrom::Start(offset.into()))?;
    Ok(flipper_file::parse_record(reader.lines().map_while(Result::ok)))
}
