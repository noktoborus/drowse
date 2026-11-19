//! libexec/drowsed-tuned with a stub of `tuned-adm`: the stub keeps the
//! profile and the mode in files and writes down each call. It needs `sh`
//! and `sed` only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The stub of `tuned-adm`, in the form of the output of TuneD.
const STUB: &str = r#"#!/bin/sh
d=$TUNED_STUB
echo "$*" >> "$d/calls"
case $1 in
	active)
		p=$(cat "$d/profile")
		if [ -n "$p" ]; then echo "Current active profile: $p"; else echo "No current active profile."; fi
		;;
	profile_mode) echo "Profile selection mode: $(cat "$d/mode")" ;;
	profile)
		shift
		if [ -e "$d/fail" ]; then echo "no such profile: $*" >&2; exit 1; fi
		echo "$*" > "$d/profile"
		echo manual > "$d/mode"
		;;
	auto_profile) echo recommended > "$d/profile"; echo auto > "$d/mode" ;;
	off) : > "$d/profile"; echo manual > "$d/mode" ;;
esac
"#;

/// A place of its own for a test: the stub, its state and the directory
/// of the unit.
struct Bench {
    root: PathBuf,
}

impl Bench {
    /// TuneD with this profile (empty: none) in this mode.
    fn new(name: &str, profile: &str, mode: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("drowsed-tuned-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["bin", "unit"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        let stub = root.join("bin/tuned-adm");
        fs::write(&stub, STUB).unwrap();
        Command::new("chmod").arg("+x").arg(&stub).status().unwrap();
        fs::write(root.join("profile"), format!("{profile}\n")).unwrap();
        fs::write(root.join("mode"), format!("{mode}\n")).unwrap();
        fs::write(root.join("calls"), "").unwrap();
        Self { root }
    }

    /// Runs the wrapper with these arguments.
    fn run(&self, args: &[&str]) -> Output {
        let wrapper = Path::new(env!("CARGO_MANIFEST_DIR")).join("libexec/drowsed-tuned");
        let path = format!(
            "{}:{}",
            self.root.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new("sh")
            .arg(wrapper)
            .args(args)
            .env("PATH", path)
            .env("TUNED_STUB", &self.root)
            .env("RUNTIME_DIRECTORY", self.root.join("unit"))
            .output()
            .unwrap()
    }

    /// Runs the wrapper; it must end well. Gives its stdout.
    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(out.status.success(), "{args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap()
    }

    /// The profile and the mode of the stub now.
    fn tuned(&self) -> (String, String) {
        let read = |file| fs::read_to_string(self.root.join(file)).unwrap();
        (
            read("profile").trim().to_owned(),
            read("mode").trim().to_owned(),
        )
    }

    /// The profile and the mode change, as by the user.
    fn set_by_hand(&self, profile: &str, mode: &str) {
        fs::write(self.root.join("profile"), format!("{profile}\n")).unwrap();
        fs::write(self.root.join("mode"), format!("{mode}\n")).unwrap();
    }

    /// The calls of `tuned-adm` that change it.
    fn changes(&self) -> Vec<String> {
        fs::read_to_string(self.root.join("calls"))
            .unwrap()
            .lines()
            .filter(|line| !matches!(*line, "active" | "profile_mode"))
            .map(str::to_owned)
            .collect()
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn leave_gives_back_the_profile_before() {
    let bench = Bench::new("back", "balanced-battery", "manual");
    let said = bench.ok(&["enter", "cm5-powersave"]);
    assert_eq!(
        said,
        "tuned: cm5-powersave set; before: balanced-battery (manual)\n"
    );
    assert_eq!(bench.tuned().0, "cm5-powersave");
    let said = bench.ok(&["leave"]);
    assert_eq!(said, "tuned: balanced-battery given back\n");
    assert_eq!(
        bench.tuned(),
        ("balanced-battery".to_owned(), "manual".to_owned())
    );
    assert_eq!(
        bench.changes(),
        ["profile cm5-powersave", "profile balanced-battery"]
    );
}

#[test]
fn the_mode_auto_is_given_back_as_auto() {
    let bench = Bench::new("auto", "recommended", "auto");
    bench.ok(&["enter", "cm5-powersave"]);
    assert_eq!(bench.ok(&["leave"]), "tuned: the auto profile given back\n");
    assert_eq!(bench.tuned().1, "auto");
    assert_eq!(bench.changes(), ["profile cm5-powersave", "auto_profile"]);
}

#[test]
fn no_profile_before_is_given_back_as_none() {
    let bench = Bench::new("none", "", "manual");
    bench.ok(&["enter", "cm5-powersave"]);
    assert_eq!(bench.ok(&["leave"]), "tuned: no profile given back\n");
    assert_eq!(bench.changes(), ["profile cm5-powersave", "off"]);
}

#[test]
fn a_profile_changed_in_between_stays() {
    let bench = Bench::new("changed", "balanced-battery", "manual");
    bench.ok(&["enter", "cm5-powersave"]);
    bench.set_by_hand("performance", "manual");
    assert_eq!(
        bench.ok(&["leave"]),
        "tuned: the profile is performance (manual), not cm5-powersave as set; \
         balanced-battery is not given back\n"
    );
    assert_eq!(bench.tuned().0, "performance");
    assert_eq!(bench.changes(), ["profile cm5-powersave"]);
}

#[test]
fn the_same_profile_in_the_mode_auto_stays() {
    let bench = Bench::new("auto-after", "balanced", "manual");
    bench.ok(&["enter", "cm5-powersave"]);
    bench.set_by_hand("cm5-powersave", "auto");
    let said = bench.ok(&["leave"]);
    assert!(said.contains("not given back"), "{said}");
    assert_eq!(bench.changes(), ["profile cm5-powersave"]);
}

#[test]
fn a_merged_profile_goes_and_comes_back_whole() {
    let bench = Bench::new("merged", "balanced usb-off", "manual");
    bench.ok(&["enter", "cm5-powersave", "no-radio"]);
    assert_eq!(bench.tuned().0, "cm5-powersave no-radio");
    assert_eq!(bench.ok(&["leave"]), "tuned: balanced usb-off given back\n");
    assert_eq!(bench.tuned().0, "balanced usb-off");
}

#[test]
fn leave_with_no_enter_does_nothing() {
    let bench = Bench::new("nothing", "balanced", "manual");
    assert_eq!(bench.ok(&["leave"]), "tuned: nothing to give back\n");
    assert!(bench.changes().is_empty());
}

#[test]
fn a_failed_enter_keeps_nothing() {
    let bench = Bench::new("fail", "balanced", "manual");
    fs::write(bench.root.join("fail"), "").unwrap();
    let out = bench.run(&["enter", "cm5-powersave"]);
    assert!(!out.status.success());
    fs::remove_file(bench.root.join("fail")).unwrap();
    assert_eq!(bench.ok(&["leave"]), "tuned: nothing to give back\n");
}

#[test]
fn a_wrong_command_line_is_code_2() {
    let bench = Bench::new("usage", "balanced", "manual");
    assert_eq!(bench.run(&["enter"]).status.code(), Some(2));
    assert_eq!(bench.run(&["off"]).status.code(), Some(2));
}
