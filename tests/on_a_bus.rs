//! drowsed and drowse-status on a bus of their own, with mocks of logind,
//! UPower and systemd. It needs `dbus-daemon`: `cargo test -- --ignored`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use zbus::{Connection, OwnedObjectPath, connection::Builder, interface};

/// The mock of the manager of logind: one idle session, no inhibitor.
struct Logind;

#[interface(name = "org.freedesktop.login1.Manager")]
impl Logind {
    #[allow(clippy::type_complexity)]
    fn list_sessions_ex(
        &self,
    ) -> Vec<(
        String,
        u32,
        String,
        String,
        u32,
        String,
        String,
        bool,
        u64,
        OwnedObjectPath,
    )> {
        vec![(
            "1".to_owned(),
            1000,
            "alice".to_owned(),
            "seat0".to_owned(),
            1,
            "user".to_owned(),
            "tty1".to_owned(),
            true,
            0,
            OwnedObjectPath::try_from("/org/freedesktop/login1/session/_31").unwrap(),
        )]
    }

    fn list_inhibitors(&self) -> Vec<(String, String, String, String, u32, u32)> {
        Vec::new()
    }

    fn power_off_with_flags(&self, _flags: u64) {}
}

/// The mock of a session of logind.
struct LoginSession;

#[interface(name = "org.freedesktop.login1.Session")]
impl LoginSession {
    #[zbus(property)]
    fn state(&self) -> String {
        "active".to_owned()
    }

    #[zbus(property, name = "Type")]
    fn kind(&self) -> String {
        "wayland".to_owned()
    }

    #[zbus(property, name = "TTY")]
    fn tty(&self) -> String {
        "tty1".to_owned()
    }

    #[zbus(property)]
    fn remote_host(&self) -> String {
        String::new()
    }
}

/// The mock of UPower: on battery.
struct UPower;

#[interface(name = "org.freedesktop.UPower")]
impl UPower {
    #[zbus(property)]
    fn on_battery(&self) -> bool {
        true
    }
}

/// The mock of the manager of systemd: no unit of a stage.
struct Systemd;

#[interface(name = "org.freedesktop.systemd1.Manager")]
impl Systemd {
    fn subscribe(&self) {}

    #[allow(clippy::type_complexity)]
    fn list_units_by_patterns(
        &self,
        _states: Vec<String>,
        _patterns: Vec<String>,
    ) -> Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        OwnedObjectPath,
        u32,
        String,
        OwnedObjectPath,
    )> {
        Vec::new()
    }
}

/// A `dbus-daemon` of its own; it ends with the test.
struct Daemon {
    child: Child,
    address: String,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn daemon() -> Daemon {
    let dir = std::env::temp_dir().join(format!("drowse-bus-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("bus.conf");
    std::fs::write(
        &config,
        format!(
            "<busconfig><type>system</type><listen>unix:dir={}</listen>\
             <auth>EXTERNAL</auth><policy context=\"default\"><allow user=\"*\"/>\
             <allow own=\"*\"/><allow send_type=\"method_call\"/>\
             <allow send_destination=\"*\"/><allow receive_sender=\"*\"/>\
             <allow eavesdrop=\"true\"/></policy></busconfig>",
            dir.display()
        ),
    )
    .unwrap();
    let mut child = Command::new("dbus-daemon")
        .arg(format!("--config-file={}", config.display()))
        .args(["--nofork", "--print-address"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("dbus-daemon");
    let mut address = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut address)
        .unwrap();
    Daemon {
        child,
        address: address.trim().to_owned(),
    }
}

async fn serve(address: &str) -> Vec<Connection> {
    let at = |name: &'static str| Builder::address(address).name(name);
    vec![
        at("org.freedesktop.login1")
            .serve_at("/org/freedesktop/login1", Logind)
            .serve_at("/org/freedesktop/login1/session/_31", LoginSession)
            .build()
            .await
            .unwrap(),
        at("org.freedesktop.UPower")
            .serve_at("/org/freedesktop/UPower", UPower)
            .build()
            .await
            .unwrap(),
        at("org.freedesktop.systemd1")
            .serve_at("/org/freedesktop/systemd1", Systemd)
            .build()
            .await
            .unwrap(),
    ]
}

#[tokio::test]
#[ignore = "needs dbus-daemon"]
async fn drowsed_and_drowse_status_on_a_bus() {
    let daemon = daemon();
    let _mocks = serve(&daemon.address).await;
    let mut drowsed = Command::new(env!("CARGO_BIN_EXE_drowsed"))
        .env("DBUS_SYSTEM_BUS_ADDRESS", &daemon.address)
        .spawn()
        .unwrap();
    let client = Builder::address(daemon.address.as_str())
        .build()
        .await
        .unwrap();
    let dbus = zbus::fdo::DBusProxy::new(&client).await.unwrap();
    let mut up = false;
    for _ in 0..50 {
        if dbus
            .name_has_owner("io.github.noktoborus.Drowse".try_into().unwrap())
            .await
            .unwrap()
        {
            up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(up, "drowsed did not take its name");
    let properties = zbus::fdo::PropertiesProxy::builder(&client)
        .destination("io.github.noktoborus.Drowse")
        .path("/io/github/noktoborus/Drowse")
        .build()
        .await
        .unwrap();
    let all: HashMap<String, zbus::OwnedVariant> = properties
        .get_all(zbus::names::InterfaceName::from_static_str_unchecked(
            "io.github.noktoborus.Drowse",
        ))
        .await
        .unwrap();
    assert_eq!(String::try_from(all["State"].clone()).unwrap(), "idle");
    assert_eq!(
        String::try_from(all["PowerSource"].clone()).unwrap(),
        "battery"
    );
    let address = daemon.address.clone();
    let status = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_drowse-status"))
            .arg("--waybar")
            .env("DBUS_SYSTEM_BUS_ADDRESS", address)
            .env("LANG", "C")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    let line = String::from_utf8(status.stdout).unwrap();
    assert!(status.status.success(), "{line}");
    assert!(
        line.starts_with("{\"text\":\"17 min\",\"alt\":\"idle\""),
        "{line}"
    );
    let _ = Command::new("kill")
        .args(["-TERM", &drowsed.id().to_string()])
        .status();
    let ended = tokio::task::spawn_blocking(move || drowsed.wait().unwrap())
        .await
        .unwrap();
    assert!(ended.success());
}
