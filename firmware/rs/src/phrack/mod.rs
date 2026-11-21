mod extract;
mod library;
mod pages;
mod reader;
mod tar;

use std::fs::File;
use std::io;
use std::path::Path;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use crate::keys::{self, is_back, is_go, Key};
use crate::sd::{CardState, SdCard, MOUNT_POINT};
use crate::ui;
use library::{Archive, Article};
use reader::Reader;

const TITLE: &str = "// phrack";
const LIST_HINT: &str = "UP/DOWN move   A open   B back";
const EXTRACTING_LINES: [&str; 2] = ["extracting...", "(only the first time)"];
const EXTRACTING_HINT: &str = "please wait";
const LABEL_MAX_CHARS: usize = 26;
const SUBTITLE_MAX_CHARS: usize = 20;

enum Screen {
    Archives,
    Articles,
    Reader(Reader),
    Failed(Vec<String>),
}

pub struct Phrack {
    screen: Screen,
    archives: Vec<Archive>,
    archive_sel: usize,
    articles: Vec<Article>,
    article_sel: usize,
}

impl Phrack {
    pub fn new() -> Self {
        Self { screen: Screen::Archives, archives: Vec::new(), archive_sel: 0, articles: Vec::new(), article_sel: 0 }
    }

    pub fn open<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D, sd: &mut SdCard) {
        let card = (sd.ensure_mounted() == CardState::Ready).then_some(Path::new(MOUNT_POINT));
        self.archives = library::archives(card);
        self.archive_sel = 0;
        self.show(Screen::Archives, d);
    }

    pub fn key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, shift: bool, d: &mut D) -> bool {
        match &mut self.screen {
            Screen::Archives => return self.archives_key(k, d),
            Screen::Articles => self.articles_key(k, d),
            Screen::Reader(reader) => {
                if reader.key(k, shift, d) {
                    self.show(Screen::Articles, d);
                }
            }
            Screen::Failed(_) if is_back(k) => self.show(Screen::Archives, d),
            Screen::Failed(_) => {}
        }
        false
    }

    fn archives_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) -> bool {
        if let Some(new) = keys::wrap_selection(k, self.archive_sel, self.archives.len()) {
            let old = std::mem::replace(&mut self.archive_sel, new);
            ui::menu_select(d, &ui::as_strs(&self.archive_labels()), old, new, LIST_HINT);
        } else if is_go(k) {
            self.open_archive(d);
        } else if is_back(k) {
            return true;
        }
        false
    }

    fn articles_key<D: DrawTarget<Color = Rgb565>>(&mut self, k: Key, d: &mut D) {
        if let Some(new) = keys::wrap_selection(k, self.article_sel, self.articles.len()) {
            let old = std::mem::replace(&mut self.article_sel, new);
            ui::menu_select(d, &ui::as_strs(&self.article_labels()), old, new, LIST_HINT);
        } else if is_go(k) {
            self.open_article(d);
        } else if is_back(k) {
            self.show(Screen::Archives, d);
        }
    }

    fn open_archive<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        let loaded = match &self.archives[self.archive_sel] {
            Archive::Builtin => library::builtin_articles(),
            Archive::Card(path) => card_articles(path, d),
        };
        self.article_sel = 0;
        let screen = match loaded {
            Ok(articles) if articles.is_empty() => failed("archive is empty", ""),
            Ok(articles) => {
                self.articles = articles;
                Screen::Articles
            }
            Err(e) => failed("cannot open archive", &e.to_string()),
        };
        self.show(screen, d);
    }

    fn open_article<D: DrawTarget<Color = Rgb565>>(&mut self, d: &mut D) {
        let article = &self.articles[self.article_sel];
        let screen = match Reader::open(&article.text, cut(&article.title, SUBTITLE_MAX_CHARS)) {
            Ok(reader) => Screen::Reader(reader),
            Err(e) => failed("cannot open article", &e.to_string()),
        };
        self.show(screen, d);
    }

    fn show<D: DrawTarget<Color = Rgb565>>(&mut self, screen: Screen, d: &mut D) {
        self.screen = screen;
        self.draw(d);
    }

    fn archive_labels(&self) -> Vec<String> {
        self.archives.iter().map(|a| cut(&a.name(), LABEL_MAX_CHARS)).collect()
    }

    fn article_labels(&self) -> Vec<String> {
        self.articles.iter().enumerate().map(|(i, a)| cut(&format!("{}. {}", i + 1, a.title), LABEL_MAX_CHARS)).collect()
    }

    fn draw<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) {
        match &self.screen {
            Screen::Archives => ui::menu(d, TITLE, &ui::as_strs(&self.archive_labels()), self.archive_sel, LIST_HINT),
            Screen::Articles => ui::menu(d, TITLE, &ui::as_strs(&self.article_labels()), self.article_sel, LIST_HINT),
            Screen::Reader(reader) => reader.draw(d),
            Screen::Failed(lines) => ui::screen(d, TITLE, &ui::as_strs(lines)),
        }
    }
}

fn card_articles<D: DrawTarget<Color = Rgb565>>(archive: &Path, d: &mut D) -> io::Result<Vec<Article>> {
    let dir = library::cache_dir(Path::new(MOUNT_POINT), archive).ok_or(io::ErrorKind::InvalidInput)?;
    if !library::is_extracted(&dir) {
        ui::screen(d, TITLE, &EXTRACTING_LINES);
        ui::footer(d, EXTRACTING_HINT);
        let files = extract::extract(File::open(archive)?, &dir)?;
        log::info!("extracted {files} files from {}", archive.display());
    }
    Ok(library::card_articles(&dir))
}

fn failed(what: &str, why: &str) -> Screen {
    Screen::Failed(vec![what.into(), cut(why, LABEL_MAX_CHARS)])
}

fn cut(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
