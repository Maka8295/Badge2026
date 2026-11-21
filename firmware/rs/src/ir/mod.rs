mod database;
mod decode;
pub mod flipper_file;
pub mod protocols;
mod rmt;
mod signal;

pub use database::{encode, Button, Code, Protocol, Remote};
pub use decode::describe;
pub use rmt::Ir;
pub use signal::{from_bytes, to_bytes, Signal, BYTES_PER_PAIR, MAX_PAIRS};
