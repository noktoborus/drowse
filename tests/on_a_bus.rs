//! drowsed and drowse-status on a bus of their own, with mocks of logind,
//! UPower and systemd. It needs `dbus-daemon`: `cargo test -- --ignored`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use zbus::{Connection, OwnedObjectPath, connection::Builder, interface, message::Header};

/// An inhibitor taken of the mock: what, who, why, mode, uid, pid, and
/// the end of a socket that ends when the holder lets it go.
struct Taken {
    row: (String, String, String, String, u32, u32),
    end: UnixStream,
}

impl Taken {
    /// Whether the holder still holds it.
    fn held(&self) -> bool {
        let mut byte = [0];
        self.end.set_nonblocking(true).unwrap();
        matches!((&self.end).read(&mut byte), Err(error) if error.kind() == ErrorKind::WouldBlock)
    }
}

/// The inhibitors taken of the mock, shared with the test.
type Inhibitors = Arc<Mutex<Vec<Taken>>>;

/// The mock of the manager of logind: one idle session; the inhibitors
/// taken of it.
struct Logind {
    inhibitors: Inhibitors,
}

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
        let inhibitors = self.inhibitors.lock().unwrap();
        inhibitors
            .iter()
            .filter(|taken| taken.held())
            .map(|taken| taken.row.clone())
            .collect()
    }

    async fn inhibit(
        &self,
        what: String,
        who: String,
        why: String,
        mode: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::OwnedFd {
        let dbus = zbus::fdo::DBusProxy::new(connection).await.unwrap();
        let sender = header.sender().unwrap().to_owned();
        let pid = dbus
            .get_connection_unix_process_id(sender.clone().into())
            .await
            .unwrap();
        let uid = dbus.get_connection_unix_user(sender.into()).await.unwrap();
        let (end, given) = UnixStream::pair().unwrap();
        self.inhibitors.lock().unwrap().push(Taken {
            row: (what, who, why, mode, uid, pid),
            end,
        });
        OwnedFd::from(given).into()
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

async fn serve(address: &str, inhibitors: &Inhibitors) -> Vec<Connection> {
    let at = |name: &'static str| Builder::address(address).name(name);
    let logind = Logind {
        inhibitors: Arc::clone(inhibitors),
    };
    vec![
        at("org.freedesktop.login1")
            .serve_at("/org/freedesktop/login1", logind)
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
    let _mocks = serve(&daemon.address, &Inhibitors::default()).await;
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
    assert!(line.contains("\"alt\":\"idle\""), "{line}");
    let _ = Command::new("kill")
        .args(["-TERM", &drowsed.id().to_string()])
        .status();
    let ended = tokio::task::spawn_blocking(move || drowsed.wait().unwrap())
        .await
        .unwrap();
    assert!(ended.success());
}

/// Runs drowse-inhibit on the bus of the daemon, with no terminal.
async fn drowse_inhibit(address: &str, args: &[&str]) -> std::process::Output {
    let address = address.to_owned();
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_drowse-inhibit"))
            .args(&args)
            .env("DBUS_SYSTEM_BUS_ADDRESS", address)
            .env("LANG", "C")
            .stdin(Stdio::null())
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

/// Whether some inhibitor of this `who` is held, after a moment for the
/// holder to end.
async fn held(inhibitors: &Inhibitors, who: &str) -> bool {
    for _ in 0..20 {
        let any = inhibitors
            .lock()
            .unwrap()
            .iter()
            .any(|taken| taken.row.1 == who && taken.held());
        if !any {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    true
}

#[tokio::test]
#[ignore = "needs dbus-daemon"]
async fn drowse_inhibit_on_a_bus() {
    let daemon = daemon();
    let inhibitors = Inhibitors::default();
    let _mocks = serve(&daemon.address, &inhibitors).await;

    let ran = drowse_inhibit(&daemon.address, &["--", "sh", "-c", "sleep 3 & exit 3"]).await;
    assert_eq!(ran.status.code(), Some(3), "{ran:?}");
    {
        let taken = inhibitors.lock().unwrap();
        let row = &taken[0].row;
        assert_eq!(
            (
                row.0.as_str(),
                row.1.as_str(),
                row.2.as_str(),
                row.3.as_str()
            ),
            (
                "idle",
                "until sh -c 'sleep 3 & exit 3'",
                "held by hand",
                "block-weak"
            )
        );
    }
    assert!(
        !held(&inhibitors, "until sh -c 'sleep 3 & exit 3'").await,
        "the inhibitor outlived the command: the fd went to its child"
    );

    let on = drowse_inhibit(&daemon.address, &["on", "--why", "a test"]).await;
    let said = String::from_utf8(on.stdout).unwrap();
    assert!(on.status.success(), "{said}");
    assert_eq!(said, "drowse-inhibit: the system is held (a test)\n");
    assert!(
        held(&inhibitors, "drowse-inhibit").await,
        "the holder let go"
    );
    let pid = inhibitors.lock().unwrap()[1].row.5;
    assert_ne!(pid, 0);

    let again = drowse_inhibit(&daemon.address, &["on"]).await;
    assert!(again.status.success());
    tokio::time::sleep(Duration::from_millis(300)).await;
    {
        let taken = inhibitors.lock().unwrap();
        assert!(!taken[1].held(), "the holder before is still there");
        assert!(taken[2].held(), "the new holder let go");
    }

    let off = drowse_inhibit(&daemon.address, &["off"]).await;
    assert!(off.status.success());
    assert!(
        !held(&inhibitors, "drowse-inhibit").await,
        "off let nothing go"
    );
    let off = drowse_inhibit(&daemon.address, &["off"]).await;
    assert_eq!(
        String::from_utf8(off.stdout).unwrap(),
        "drowse-inhibit: nothing to let go\n"
    );

    let both = drowse_inhibit(&daemon.address, &["--all", "--", "true"]).await;
    assert_eq!(both.status.code(), Some(0), "{both:?}");
    let taken = inhibitors.lock().unwrap();
    assert!(taken.iter().any(|item| item.row.0 == "idle"));
    assert!(taken.iter().any(|item| item.row.0 == "sleep:shutdown"));
}
