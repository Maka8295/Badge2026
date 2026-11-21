use std::fs;
use std::path::Path;

const PARENT: &str = "..";
const DIR_MARK: char = '/';
const SIZE_GAP: &str = "  ";
const LABEL_MAX_CHARS: usize = 26;
const UNITS: [char; 4] = ['B', 'K', 'M', 'G'];
const UNIT_STEP: f32 = 1024.0;

#[derive(Clone)]
pub enum Kind {
    Parent,
    Dir,
    File(u64),
}

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub kind: Kind,
}

impl Entry {
    pub fn label(&self) -> String {
        match self.kind {
            Kind::Parent => PARENT.into(),
            Kind::Dir => format!("{}{DIR_MARK}", cut(&self.name, LABEL_MAX_CHARS - 1)),
            Kind::File(size) => file_label(&self.name, &human_size(size)),
        }
    }
}

pub fn read(dir: &Path, at_root: bool) -> Vec<Entry> {
    let mut entries = children(dir);
    entries.sort_by_key(|e| (!matches!(e.kind, Kind::Dir), e.name.to_lowercase()));
    if !at_root {
        entries.insert(0, Entry { name: PARENT.into(), kind: Kind::Parent });
    }
    entries
}

fn children(dir: &Path) -> Vec<Entry> {
    match fs::read_dir(dir) {
        Ok(list) => list.filter_map(Result::ok).filter_map(|e| entry(&e)).collect(),
        Err(e) => {
            log::warn!("cannot list {}: {e}", dir.display());
            Vec::new()
        }
    }
}

fn entry(e: &fs::DirEntry) -> Option<Entry> {
    let meta = e.metadata().ok()?;
    let kind = if meta.is_dir() { Kind::Dir } else { Kind::File(meta.len()) };
    Some(Entry { name: e.file_name().to_string_lossy().into_owned(), kind })
}

fn file_label(name: &str, size: &str) -> String {
    let room = LABEL_MAX_CHARS - SIZE_GAP.len() - size.len();
    format!("{}{SIZE_GAP}{size}", cut(name, room))
}

fn cut(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

fn human_size(bytes: u64) -> String {
    let mut size = bytes as f32;
    let mut unit = 0;
    while size >= UNIT_STEP && unit + 1 < UNITS.len() {
        size /= UNIT_STEP;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}{}", UNITS[0])
    } else {
        format!("{size:.1}{}", UNITS[unit])
    }
}
