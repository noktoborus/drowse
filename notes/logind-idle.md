# The idle state of logind

## Question

How does logind tell that the system is idle? Are a session of ssh and a
local session the same for it? Read in the source of systemd 261.2, the
version of the uConsole image, on 2026-11-14.

## The system

`manager_get_idle_hint()`, `src/login/logind-core.c:450`.

- The system is idle when no inhibitor of `idle`, of mode `block` or
  `block-weak`, is held (`manager_is_inhibited()` with flags 0 counts all
  but `delay`, notes/logind-power-rights.md), and every
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
| tty with a tty: a console | the last access to the tty (its atime) + `IdleActionSec` <= now |
| tty with no tty: ssh (notes/session-types.md) | the controlling tty of the leader is read; none found: never |
| Anything else | never |

- For a tty, the delay is `IdleActionSec` (default 30 min,
  `src/login/logind-core.c:72`), or `StopIdleSessionSec` when it is
  smaller. `IdleActionSec` counts even with `IdleAction=ignore`.
- The atime of a tty changes on input. Output changes the mtime and does
  not count.
- Only a graphical session can set its idle hint
  (`session_set_idle_hint()`, `src/login/logind-session.c:1219`). The
  time of the hint is the time it was set, not the time of the last input.

## When a console will become idle

`session_get_idle_hint()`, `src/login/logind-session.c:1203`
(`found_atime`): for a console the time given back (`IdleSinceHint`, and
`since` of `ListSessionsEx()`) is the atime of the tty, the last input,
whether the console is idle yet or not:

```c
found_atime:
        if (ret_timestamp)
                dual_timestamp_from_realtime(ret_timestamp, atime);
```

The delay is `IdleActionUSec` of the Manager
(`src/login/logind-dbus.c:3838`, constant). So a watcher can work out
when a console will become idle: `since + IdleActionUSec`, and read
again then, in place of polling. That it is active again cannot be
known ahead: only a read shows it.

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

## The idle hint of a graphical session

Read on 2026-11-16 in systemd 261.2.

- Only the owner of the session or root sets it: "Only owner of session
  may set idle hint" (`method_set_idle_hint()`,
  `src/login/logind-session-dbus.c:290`).
- Only a graphical session of a class that can idle takes it; else
  `ENOTTY`, on the bus "Idle hint control is not supported on
  non-graphical and non-user sessions" (`session_set_idle_hint()`,
  `src/login/logind-session.c:1219`). A console or ssh cannot set it.
- The time of the hint (`IdleSinceHint`) changes only when the value
  changes: `if (s->idle_hint == b) return 0;` (the same function). A
  session starts with the hint false and no time, and the first
  `SetIdleHint(false)` of swayidle changes nothing.
- So a graphical session with the hint false and `IdleSinceHint` 0 has
  never been idle: no helper sets its hint, or nobody left it idle since
  the login. Once idle once, a session with a helper has a time. This
  tells a session with no helper apart, after its first idle only
  (`session_get_idle_hint()`, `src/login/logind-session.c:1160`).
- By hand, from the session itself:
  `busctl call org.freedesktop.login1 /org/freedesktop/login1/session/auto
  org.freedesktop.login1.Session SetIdleHint b true`; for another
  session, its path (`…/session/_31` for the id 1).

## For the service

- The idle state of the service is that of logind.
- A graphical session must set its idle hint. swayidle does it with
  `idlehint <timeout>`; with no such line, sway is never idle for logind.
  The login screen: the branch `plans`, login-screen.md.
- `IdleActionSec` is added to the idle time of a console (default 30
  min). A small one makes a console idle sooner; the service takes a
  console as logind says (the branch `plans`, later-consoles.md).
- A console that waits at a prompt is idle. An ssh session is never idle,
  with a pty or not: it keeps the system up while it is open
  (notes/session-types.md).

## On the uConsole

- `bcm2712_defconfig`: `# CONFIG_SUSPEND is not set`, no
  `CONFIG_HIBERNATION`. Of the actions only `poweroff`, `halt`,
  `reboot`, `lock` work.
- `halt` does not cut the power: the AXP228 keeps the board on.
- Suspend to RAM on the CM5 needs a bootloader with S3 and kernel patches
  that are not released (Raspberry Pi, pull request 7514 of
  raspberrypi/linux, open on 2026-10-24).
