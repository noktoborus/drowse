//! The settings of the service, read from `/etc/drowse.yaml`.

pub mod action;
pub mod sessions;
pub mod time;
pub mod yaml;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub use action::{Command, LogindAction};
pub use time::Span;
pub use yaml::{Section, SectionKind};

use crate::error::{Error, Result};

/// The path of the settings file.
pub const PATH: &str = "/etc/drowse.yaml";

/// The chain the service runs.
pub const MAIN: &str = "main";

/// The name of a section, as it stands in the file: the chain, the power
/// source, the number from 1. The journal, the unit and the errors of the
/// settings give it alike.
pub fn section_name(power: Power, index: usize) -> String {
    format!("{MAIN}.{}.{}", power.instance(), index + 1)
}

/// The power source and the index of a section, from its name.
pub fn parse_section_name(name: &str) -> Option<(Power, usize)> {
    let rest = name.strip_prefix(MAIN)?.strip_prefix('.')?;
    let (power, number) = rest.split_once('.')?;
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let power = Power::ALL
        .into_iter()
        .find(|known| known.instance() == power)?;
    number
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
        .map(|index| (power, index))
}

/// A power source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Power {
    /// On battery.
    Battery,
    /// On AC, or the power source is not known.
    Ac,
}

impl Power {
    /// Both power sources.
    pub const ALL: [Self; 2] = [Self::Battery, Self::Ac];

    /// The name in the file and in the name of a section: `battery`, `ac`.
    pub fn instance(self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::Ac => "ac",
        }
    }
}

/// The settings used by the running service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    battery: Vec<Section>,
    ac: Vec<Section>,
    classes: BTreeSet<String>,
}

impl Settings {
    /// Whether a session of this class counts in the idle state.
    pub fn counts_class(&self, class: &str) -> bool {
        self.classes.contains(class)
    }

    /// The sections of a power source, the first one first.
    pub fn sections(&self, power: Power) -> &[Section] {
        match power {
            Power::Battery => &self.battery,
            Power::Ac => &self.ac,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            battery: Vec::new(),
            ac: Vec::new(),
            classes: sessions::default_classes(),
        }
    }
}

/// Reads settings. No file or an empty file gives empty chains.
pub fn load(path: &Path) -> Result<Settings> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(source) => Err(Error::ReadSettings {
            path: PathBuf::from(path),
            source,
        }),
    }
}

/// Reads and validates a settings file.
pub fn parse(text: &str) -> Result<Settings> {
    let file = yaml::parse(text)?;
    Ok(Settings {
        battery: file.main.battery,
        ac: file.main.ac,
        classes: file.sessions.classes.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_section_has_one_name_in_the_file_the_unit_and_the_errors() {
        assert_eq!(section_name(Power::Battery, 0), "main.battery.1");
        assert_eq!(section_name(Power::Ac, 2), "main.ac.3");
        for power in Power::ALL {
            for index in [0, 1, 9] {
                let name = section_name(power, index);
                assert_eq!(parse_section_name(&name), Some((power, index)));
            }
        }
        assert_eq!(
            crate::engine::unit_name(Power::Battery, 1),
            "drowsed-stage@main.battery.2.service"
        );
        let error = parse("main:\n  battery: [{after: 1m}, {after: 1m, action: nap}]\n")
            .unwrap_err()
            .to_string();
        assert!(error.contains(&section_name(Power::Battery, 1)), "{error}");
    }

    #[test]
    fn no_file_is_empty() {
        let path = std::env::temp_dir().join(format!("drowse-no-file-{}", std::process::id()));
        assert_eq!(load(&path).unwrap(), Settings::default());
    }

    #[test]
    fn package_settings_are_valid_and_keep_the_main_chain() {
        let settings = parse(include_str!("../../etc/drowse.yaml")).unwrap();
        let battery = settings.sections(Power::Battery);
        assert_eq!(battery.len(), 2);
        assert_eq!(battery[0].after, Span::Time(Duration::from_secs(600)));
        assert_eq!(battery[0].countdown, Span::Never);
        assert_eq!(battery[0].hint.as_deref(), Some("power save"));
        assert!(matches!(
            &battery[0].kind,
            SectionKind::Command {
                enter: Some(command),
                leave: Some(_),
            } if command.line() == "/usr/libexec/drowse/drowsed-tuned enter cm5-powersave"
        ));
        assert_eq!(battery[1].after, Span::Time(Duration::from_secs(38 * 60)));
        assert_eq!(battery[1].countdown, Span::Time(Duration::from_secs(120)));
        assert!(matches!(
            battery[1].kind,
            SectionKind::Action(LogindAction::Poweroff)
        ));
        assert_eq!(settings.sections(Power::Ac).len(), 0);
        assert!(settings.counts_class("greeter"));
        assert!(!settings.counts_class("background"));

        let file = yaml::parse(include_str!("../../etc/drowse.yaml")).unwrap();
        assert_eq!(file.sections.len(), 1);
        assert!(file.lid.is_none());
    }
}
