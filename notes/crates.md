# The crates

## Question

Which crates of Rust fit drowsed and drowse-inhibit: D-Bus (logind,
UPower, the state of its own), the sd-login monitor, the event loop?
Looked on 2026-11-14 on crates.io (its API) and in the sources of the
crates; the compiler of the workspace is rustc 1.99.1.

## What was found

| Crate | Last version | Updated | MSRV | License | Fits |
|---|---|---|---|---|---|
| `zbus` | 6.3.0 | 2026-11-13 | 1.89 | MIT | yes |
| `tokio` | 1.54.1 | 2026-10-21 | 1.85 | MIT | yes |
| `logind-zbus` | 5.3.2 | 2025-05-20 | — | Apache-2.0 or MIT | no: needs `zbus ^5.7.0`, a second zbus beside 6 |
| `upower_dbus` | 0.3.2 | 2023-01-09 | — | — | no: old, few users |
| `systemd` (rust-systemd) | 0.11.1 | 2026-10-28 | — | LGPL-2.1+ with GCC exception | no: its `login` has no monitor and no state of a session or a user |
| `libsystemd-sys` | 0.10.0 | 2026-10-28 | — | LGPL-2.1+ with GCC exception | can: raw FFI with the monitor; finds libsystemd by `pkg-config` |
| `libsystemd` | 0.7.2 | 2025-04-30 | 1.69 | MIT or Apache-2.0 | no: pure Rust, notify and journal, no sd-login |
| `dbus` (dbus-rs) | 0.9.12 | 2026-07-03 | — | Apache-2.0 or MIT | can, but libdbus in C and more code |
| `sd-notify` | 0.5.0 | 2026-03-09 | 1.82 | MIT or Apache-2.0 | not needed: no reload, `Type=simple` |

### zbus 6.3.0

Features: `default` = `default-rt`, `proxy`, `service`,
`object-manager`, `tracing`. Also `tokio`, `tokio-multithread`.

- `proxy`: proxies to logind and UPower by `#[proxy]`, written in the
  project (a few methods and properties each).
- `service`: the object of its own with properties and
  `PropertiesChanged`.
- `tokio`: the runtime of tokio in place of its own (`default-rt`).
- Pure Rust: no library of C for D-Bus, so the cross build to aarch64
  needs nothing more.

### sd-login

`libsystemd-sys` 0.10.0, `src/login.rs`, has the functions:

```rust
pub fn sd_uid_get_state(uid: uid_t, state: *mut *mut c_char) -> c_int;
pub fn sd_session_get_state(session: *const c_char, state: *mut *mut c_char) -> c_int;
pub fn sd_login_monitor_new(category: *const c_char, ret: *mut *mut sd_login_monitor) -> c_int;
pub fn sd_login_monitor_flush(m: *mut sd_login_monitor) -> c_int;
pub fn sd_login_monitor_get_fd(m: *mut sd_login_monitor) -> c_int;
```

Its build runs `pkg-config` for libsystemd; a cross build needs a
`pkg-config` of the target. The same five functions in an `extern "C"`
block of its own, with `#[link(name = "systemd")]`, need no build
script.

The crate `systemd` (above `libsystemd-sys`) has in `login.rs` the
session, seat, tty, type of a session, but no monitor and no state.

## So

- D-Bus: `zbus` 6 with the feature `tokio`, no default features beyond
  `proxy` and `service`. The proxies to logind and UPower are written in
  the project.
- sd-login: `libsystemd-sys`, its build script with `pkg-config`
  (AGENTS.md, `ffi`). A cross build needs a `pkg-config` of the target.
  Only the monitor is taken from it (`sd_login_monitor_*`): the strings
  of `sd_session_get_*` are made by `malloc` and must be freed by `free`
  of C, which the crate does not give, and `libc` is not used (AGENTS.md,
  `unix.calls`). The state, the type, the tty and the remote host of a
  session are read on D-Bus, as properties of the session.
- A call of the system, if one is needed: `rustix`; zbus takes it in
  already.
- The loop: `tokio`, one thread (`current_thread`): the signals of
  D-Bus, the fd of the monitor (`AsyncFd`), the timers, `SIGTERM`.
- No crate for the journal (stderr with the prefixes `<6>`, `<5>`, `<4>`),
  for the settings file (its own parse of the form of systemd), for the
  times (its own parse of the form of systemd).
