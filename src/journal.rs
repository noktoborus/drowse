//! The journal: one line on stderr with the prefix of its level, which
//! journald reads for a unit of systemd (`sd-daemon(3)`).

use crate::engine::Level;

/// The prefix of a level for journald.
fn prefix(level: Level) -> &'static str {
    match level {
        Level::Info => "<6>",
        Level::Notice => "<5>",
        Level::Warning => "<4>",
    }
}

/// Writes one line to the journal.
pub fn write(level: Level, text: &str) {
    eprintln!("{}{}", prefix(level), text.replace('\n', " "));
}

/// Writes an error and the errors under it, on one line, at the level
/// `err`.
pub fn error(error: &dyn std::error::Error) {
    let mut line = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        line.push_str(": ");
        line.push_str(&cause.to_string());
        source = cause.source();
    }
    eprintln!("<3>{}", line.replace('\n', " "));
}
