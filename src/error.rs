//! The errors of the crate.

use std::fmt;
use std::path::PathBuf;

/// A failure the program cannot go on from.
#[derive(Debug)]
pub enum Error {
    /// The settings file is there but cannot be read.
    ReadSettings {
        /// The path of the settings file.
        path: PathBuf,
        /// The error of the system.
        source: std::io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadSettings { path, .. } => {
                write!(f, "cannot read the settings file {}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadSettings { source, .. } => Some(source),
        }
    }
}

/// The result of the crate.
pub type Result<T> = std::result::Result<T, Error>;
