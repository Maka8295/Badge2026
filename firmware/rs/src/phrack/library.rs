use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek};
use std::path::{Path, PathBuf};

use super::extract::DONE_MARK;
use super::tar;

pub const BUILTIN_NAME: &str = env!("PHRACK_BUILTIN");
static BUILTIN_TAR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/phrack.tar"));
const FOLDER: &str = "phrack";
const CACHE_FOLDER: &str = ".cache";
const ARCHIVE_SUFFIXES: [&str; 2] = [".tar.gz", ".tgz"];
const FRONT_MATTER_FENCE: &str = "---";
const TITLE_KEY: &str = "title:";
const TITLE_READ_BYTES: usize = 512;

pub trait Source: Read + Seek {}
impl<T: Read + Seek> Source for T {}

pub enum Archive {
    Builtin,
    Card(PathBuf),
}

pub enum Text {
    Builtin(&'static [u8]),
    Card(PathBuf),
}

pub struct Article {
    pub text: Text,
    pub title: String,
}

impl Archive {
    pub fn name(&self) -> String {
        match self {
            Archive::Builtin => format!("{BUILTIN_NAME} (built in)"),
            Archive::Card(path) => path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        }
    }
}

impl Text {
    pub fn open(&self) -> io::Result<Box<dyn Source>> {
        Ok(match self {
            Text::Builtin(bytes) => Box::new(Cursor::new(*bytes)),
            Text::Card(path) => Box::new(File::open(path)?),
        })
    }

    fn start(&self) -> Vec<u8> {
        let mut start = Vec::new();
        if let Ok(source) = self.open() {
            source.take(TITLE_READ_BYTES as u64).read_to_end(&mut start).ok();
        }
        start
    }
}

pub fn archives(card: Option<&Path>) -> Vec<Archive> {
    let mut found: Vec<PathBuf> = card
        .map(|card| [card.to_path_buf(), card.join(FOLDER)])
        .iter()
        .flatten()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|list| list.filter_map(Result::ok).map(|e| e.path()))
        .filter(|path| archive_stem(path).is_some())
        .collect();
    found.sort();
    std::iter::once(Archive::Builtin).chain(found.into_iter().map(Archive::Card)).collect()
}

pub fn cache_dir(card: &Path, archive: &Path) -> Option<PathBuf> {
    Some(card.join(FOLDER).join(CACHE_FOLDER).join(archive_stem(archive)?))
}

pub fn is_extracted(dir: &Path) -> bool {
    dir.join(DONE_MARK).exists()
}

pub fn builtin_articles() -> io::Result<Vec<Article>> {
    let files = tar::files(BUILTIN_TAR)?;
    Ok(sorted(files.into_iter().map(|(name, bytes)| (name, Text::Builtin(bytes))).collect()))
}

pub fn card_articles(dir: &Path) -> Vec<Article> {
    let paths = fs::read_dir(dir).map(|list| list.filter_map(Result::ok).map(|e| e.path()).collect()).unwrap_or(Vec::<PathBuf>::new());
    let named = paths.into_iter().filter_map(|p| Some((p.file_name()?.to_str()?.to_string(), Text::Card(p))));
    sorted(named.filter(|(name, _)| name != DONE_MARK).collect())
}

fn archive_stem(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    let lower = name.to_ascii_lowercase();
    let suffix = ARCHIVE_SUFFIXES.iter().find(|s| lower.ends_with(*s))?;
    Some(&name[..name.len() - suffix.len()])
}

fn sorted(mut named: Vec<(String, Text)>) -> Vec<Article> {
    named.sort_by_key(|(name, _)| (stem_number(name), name.clone()));
    named.into_iter().map(|(name, text)| Article { title: title(&name, &text), text }).collect()
}

fn stem_number(name: &str) -> u32 {
    Path::new(name).file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse().ok()).unwrap_or(u32::MAX)
}

fn title(name: &str, text: &Text) -> String {
    front_matter_title(&text.start()).unwrap_or_else(|| name.to_string())
}

fn front_matter_title(start: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(start);
    let mut lines = text.lines();
    if lines.next()? != FRONT_MATTER_FENCE {
        return None;
    }
    let line = lines.take_while(|l| *l != FRONT_MATTER_FENCE).find(|l| l.starts_with(TITLE_KEY))?;
    Some(line[TITLE_KEY.len()..].trim().to_string())
}
