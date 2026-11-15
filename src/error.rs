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
    /// A call on the system bus failed.
    Bus {
        /// What the call was for.
        what: &'static str,
        /// The error of the bus.
        source: zbus::Error,
    },
    /// The monitor of sd-login cannot start.
    SessionMonitor {
        /// The error of the system.
        source: std::io::Error,
    },
    /// The signal `SIGTERM` cannot be followed.
    Signal {
        /// The error of the system.
        source: std::io::Error,
    },
    /// logind left the bus: the service exits, and systemd starts it again.
    LogindGone,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadSettings { path, .. } => {
                write!(f, "cannot read the settings file {}", path.display())
            }
            Self::Bus { what, .. } => write!(f, "cannot {what}"),
            Self::SessionMonitor { .. } => f.write_str("cannot start the monitor of sd-login"),
            Self::Signal { .. } => f.write_str("cannot follow SIGTERM"),
            Self::LogindGone => f.write_str("logind left the bus"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadSettings { source, .. } => Some(source),
            Self::Bus { source, .. } => Some(source),
            Self::SessionMonitor { source } | Self::Signal { source } => Some(source),
            Self::LogindGone => None,
        }
    }
}

/// The result of the crate.
pub type Result<T> = std::result::Result<T, Error>;
