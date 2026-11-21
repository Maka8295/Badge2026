use std::fs;
use std::path::{Path, PathBuf};

const IR_EXTENSION: &str = "ir";
const HIDDEN_PREFIX: char = '.';
const MAX_DEPTH: usize = 3;

pub fn find_ir_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    search(root, MAX_DEPTH, &mut found);
    let mut relative: Vec<String> = found
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    relative.sort();
    relative
}

fn search(dir: &Path, depth_left: usize, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let (path, Ok(kind)) = (entry.path(), entry.file_type()) else { continue };
        if is_hidden(&path) {
            continue;
        }
        if kind.is_dir() && depth_left > 0 {
            search(&path, depth_left - 1, found);
        } else if kind.is_file() && is_ir_file(&path) {
            found.push(path);
        }
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(HIDDEN_PREFIX))
}

fn is_ir_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(IR_EXTENSION))
}
