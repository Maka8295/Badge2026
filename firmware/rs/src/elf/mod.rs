mod api;
mod context;
mod loader;

use std::path::Path;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::DrawTarget;

use crate::keys::Keys;

pub use api::{register, set_name};
pub use loader::Error;

pub fn run<D: DrawTarget<Color = Rgb565>>(path: &Path, display: &mut D, keys: &mut Keys) -> Result<(), Error> {
    let file = loader::read(path)?;
    let mut program = loader::Program::load(&file)?;
    context::while_running(display, keys, || program.start());
    Ok(())
}
