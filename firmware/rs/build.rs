use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use flate2::read::GzDecoder;

const PHRACK_FOLDER: &str = "phrack";
const ARCHIVE_SUFFIX: &str = ".tar.gz";
const EMBEDDED_TAR: &str = "phrack.tar";
const HUNT_BIN: &str = "hunt.bin";
// Everything `make hunt` reads. Not the generated crackme_secret.h, or every build would rerun.
const HUNT_INPUTS: &[&str] = &[
    "../hunt/hunt.toml",
    "../hunt/build_hunt.py",
    "../hunt/crackme_secret.py",
    "../apps/crackme/CMakeLists.txt",
    "../apps/crackme/sdkconfig.defaults",
    "../apps/crackme/main/CMakeLists.txt",
    "../apps/crackme/main/crackme.c",
    "../apps/crackme/main/machine.h",
    "../apps/crackme/main/idf_component.yml",
];

fn main() {
    embuild::espidf::sysenv::output();
    embed_phrack();
    build_hunt();
}

/// `src/hunt/vault.rs` embeds `hunt.bin`, which is gitignored (it is built from the hunt secrets).
/// Run `make hunt` when it is missing or older than its inputs, so plain `cargo build` works.
fn build_hunt() {
    for input in HUNT_INPUTS {
        println!("cargo:rerun-if-changed={input}");
    }
    let built = modified(Path::new(HUNT_BIN));
    let newest_input = HUNT_INPUTS.iter().filter_map(|p| modified(Path::new(p))).max();
    let stale = match (built, newest_input) {
        (None, _) => true,
        (Some(built), Some(input)) => input > built,
        (Some(_), None) => false,
    };
    if !stale {
        return;
    }
    let status = Command::new("make")
        .args(["-C", "..", "hunt"])
        .status()
        .expect("could not run make (needed to build rs/hunt.bin)");
    assert!(status.success(), "`make hunt` failed, see output above");
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn embed_phrack() {
    println!("cargo:rerun-if-changed={PHRACK_FOLDER}");
    let archive = single_archive(Path::new(PHRACK_FOLDER));
    let name = archive.file_name().unwrap().to_str().unwrap().trim_end_matches(ARCHIVE_SUFFIX);
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(EMBEDDED_TAR);
    let mut tar = GzDecoder::new(File::open(&archive).unwrap());
    io::copy(&mut tar, &mut File::create(out).unwrap()).unwrap();
    println!("cargo:rustc-env=PHRACK_BUILTIN={name}");
}

fn single_archive(dir: &Path) -> PathBuf {
    let archives: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_str().is_some_and(|s| s.ends_with(ARCHIVE_SUFFIX)))
        .collect();
    assert_eq!(archives.len(), 1, "put exactly one phrackNN.tar.gz in rs/{PHRACK_FOLDER}/");
    archives.into_iter().next().unwrap()
}
