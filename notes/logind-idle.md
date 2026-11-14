# The idle state of logind

## Question

How does logind tell that the system is idle? Are a session of ssh and a
local session the same for it? Read in the source of systemd 261.2, the
version of the uConsole image, on 2026-11-14.

## The system

`manager_get_idle_hint()`, `src/login/logind-core.c:450`.

- The system is idle when no block inhibitor of `idle` is held and every
  session of the classes `user`, `user-early`, `user-light`,
  `user-early-light` and `greeter` is idle (`SESSION_CLASS_CAN_IDLE`,
  `src/login/logind-session.h:51`).
- The time of the idle state (`IdleSinceHint`) is the time when the last
  of the sessions became idle.
- An inhibitor of `idle` makes the system not idle, whatever the
  sessions say.

## One session

`session_get_idle_hint()`, `src/login/logind-session.c:1160`.

| Session | Idle when |
|---|---|
| Graphical (wayland, x11, mir) | the session says so itself, by `SetIdleHint`. logind does not measure it |
| tty: a console, ssh with a pty | the last access to the tty (its atime) + `IdleActionSec` <= now |
| Anything else: ssh with no pty (sftp, a tunnel, `ssh host command`) | never |

- For a tty, the delay is `IdleActionSec` (default 30 min,
  `src/login/logind-core.c:72`), or `StopIdleSessionSec` when it is
  smaller. `IdleActionSec` counts even with `IdleAction=ignore`.
- The atime of a tty changes on input. Output changes the mtime and does
  not count.
- Only a graphical session can set its idle hint
  (`session_set_idle_hint()`, `src/login/logind-session.c:1219`). The
  time of the hint is the time it was set, not the time of the last input.

## The action of logind

`manager_dispatch_idle_action()`, `src/login/logind.c:1118`.

- `IdleAction=` runs one action after `IdleActionSec` of idle state. It
  knows no power source and gives no warning.
- It goes through `manager_handle_action()` with the inhibitors checked.

## The actions

`handle_action_table`, `src/login/logind-action.c:422`: `ignore`,
`poweroff`, `reboot`, `halt`, `kexec`, `soft-reboot`, `suspend`,
`hibernate`, `hybrid-sleep`, `suspend-then-hibernate`, `sleep`,
`factory-reset`, `lock`, `secure-attention-key`.

## For the service

- The idle state of the service is that of logind.
- A graphical session must set its idle hint. swayidle does it with
  `idlehint <timeout>`; with no such line, sway is never idle for logind.
  The sway of the login screen too.
- `IdleActionSec` must be small (1 min, say): it is added to the idle time
  of every tty session. The time of the action is counted by the service.
- A tty session that waits at a prompt is idle. A connection with no pty
  keeps the system up while it is open.

## On the uConsole

- `bcm2712_defconfig`: `# CONFIG_SUSPEND is not set`, no
  `CONFIG_HIBERNATION`. Of the actions only `poweroff`, `halt`,
  `reboot`, `lock` work.
- `halt` does not cut the power: the AXP228 keeps the board on.
- Suspend to RAM: `uconsole/linux/notes/suspend-cm5.md`.
