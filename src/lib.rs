#![warn(missing_debug_implementations, unreachable_pub)]

//! A Rust library for parsing datamined game files from Arknights and
//! exposing them as easy to understand Rust structures.

extern crate base64;
extern crate chrono;
#[macro_use]
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate thiserror;
extern crate tokio;
extern crate uord;
pub extern crate octocrab;

mod format;
pub mod game_data;
pub mod options;

pub use crate::game_data::GameData;
pub use crate::options::{Options, Region};

#[derive(Debug, Error)]
pub enum Error {
  #[error(transparent)]
  Base64Error(#[from] base64::DecodeError),
  #[error(transparent)]
  OctocrabError(#[from] octocrab::Error),
  #[error("invalid request contents")]
  InvalidResponseContents,
  #[error(transparent)]
  JsonError(#[from] serde_json::Error),
  #[error(transparent)]
  IoError(#[from] std::io::Error),
  #[error("cannot find update time")]
  /// Returned when `ak-data` cannot find a commit entry with
  /// a valid date within the first request page from GitHub.
  CannotFindUpdateTime
}
