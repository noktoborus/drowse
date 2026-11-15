# The kinds of the sessions and who tells their idle state

## Question

Which kind (type) does logind give to the session of sway, of a console,
of ssh? Who tells logind that each is idle? Read in the sources of the
versions of the uConsole image on 2026-11-14: systemd 261.2, greetd
0.10.3, gtkgreet 0.8, sway 1.12, wlroots 0.20.2, seatd 0.9.3 (libseat,
`USE="systemd -builtin -server"`: the logind backend), OpenSSH 10.5p1.

## How logind takes the kind

`pam_systemd`, `src/login/pam_systemd.c`:

- The kind is `XDG_SESSION_TYPE` of the PAM environment
  (`pam_systemd.c:1791`). With none: `x11` when there is a display, `tty`
  when there is a tty, else `unspecified` (`pam_systemd.c:986`).
- `PAM_TTY` = `ssh`: the kind is `tty`, the class `user`, and the tty is
  dropped (`pam_systemd.c:955`).
- `unspecified` with no class given makes the class `background`
  (`pam_systemd.c:999`). A session of class `background` does not count
  in the idle state at all.
- The kind and the class are put in the environment of the session
  (`pam_systemd.c:1310`).
- The controller of a session may change its kind later by `SetType`
  (`method_set_type()`, `src/login/logind-session-dbus.c:421`).
- Only a graphical session may call `SetIdleHint`; for others it is
  `ENOTTY` (`session_set_idle_hint()`, `src/login/logind-session.c:1219`).

## sway through greetd

- greetd sets `PAM_TTY=tty<vt>` and `XDG_VTNR`, and `XDG_SESSION_CLASS`;
  it sets no `XDG_SESSION_TYPE` (`greetd/src/session/worker.rs:173`,
  `:207`).
- gtkgreet sends only the command of the session, no environment
  (`gtkgreet/proto.c:159`).
- So pam_systemd makes the session `tty` on tty1.
- wlroots sets `XDG_SESSION_TYPE=wayland` before it opens the seat
  (`backend/session/session.c:88`). libseat, after `TakeControl`, calls
  `SetType` with it (`libseat/backend/logind.c:701`).
- So the session of sway becomes `wayland`: graphical. logind then takes
  its idle state only from `SetIdleHint`.
- sway, wlroots and libseat never call `SetIdleHint` (no `IdleHint` in
  their sources). With no swayidle `idlehint`, the session of sway is
  never idle.

## A console (agetty, login)

- `PAM_TTY=tty<n>`: kind `tty`. logind reads the atime of `/dev/tty<n>`:
  idle when no input for `IdleActionSec`.
- Nobody sets an idle hint. `SetIdleHint` would be refused.

## ssh

- sshd of the image opens a PAM session: `net-misc/openssh-10.5_p1` with
  `USE="pam seccomp ssl"` in the image. With `pam` the ebuild
  writes `UsePAM yes` to `/etc/ssh/sshd_config.d/9999999gentoo-pam.conf`
  (`openssh-10.5_p1.ebuild:272`, gentoo master on 2026-11-14).
  `sys-auth/pambase-20251104-r1` is built with `USE="systemd"`, so the PAM stack has
  `pam_systemd`, and logind gets the session.
- On Linux OpenSSH sets `PAM_TTY` to `ssh` (`PAM_TTY_KLUDGE`,
  `configure.ac:978`, `auth-pam.c:769`).
- So pam_systemd makes the session `tty` with no tty. logind then reads
  the controlling tty of the leader of the session
  (`session_get_idle_hint()`, `src/login/logind-session.c:1192`).
- The leader is the process that opened the PAM session: the privileged
  monitor of sshd-session (`do_pam_session()` at `sshd-session.c:1283`,
  before the fork of `privsep_postauth()`). The pty becomes the
  controlling tty only of the child of the session, after `setsid()`
  (`session.c:584`, `sshpty.c:96`).
- The monitor has no controlling tty. So logind finds no atime and says
  "not idle".
- So an ssh session is never idle: with a pty or not, typed in or not.
  It keeps the system up while it is open.
- Nobody sets an idle hint for it. `SetIdleHint` would be refused.

## So

| Session | Kind | Idle by |
|---|---|---|
| sway (greetd, wlroots, libseat) | `wayland` | `SetIdleHint` only: swayidle `idlehint`; with none, never idle |
| A console | `tty` | atime of the tty + `IdleActionSec` |
| ssh, with a pty or not | `tty`, no tty | never idle |

To check on the device: `loginctl show-session <id> -p Type -p IdleHint`
for each session.
