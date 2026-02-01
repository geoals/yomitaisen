pub mod core;
pub mod duel;

pub use core::WordRepository;
pub use duel::{DuelState, handle_connection, messages};
