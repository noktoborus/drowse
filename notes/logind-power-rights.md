# logind: rights of the power actions, inhibitors for root, sleep

## Question

What may a user or root do with the power actions and the inhibitors?
What does `CanPowerOff()` say to root while an inhibitor is held? What does
logind do with the idle state around a sleep?

Read on 2026-11-14 in systemd 261.2 (github.com/systemd/systemd, tag
`v261.2`), the version of the uConsole image.

## The rights by default

`src/login/org.freedesktop.login1.policy`, as
`allow_any` / `allow_inactive` / `allow_active`:

| Action | any | inactive | active |
|---|---|---|---|
| `power-off`, `suspend`, `hibernate` | auth_admin_keep | auth_admin_keep | yes |
| `*-multiple-sessions` of them | auth_admin_keep | auth_admin_keep | yes |
| `*-ignore-inhibit` of them | auth_admin_keep | auth_admin_keep | auth_admin_keep |
| `inhibit-block-idle` | yes | yes | yes |
| `inhibit-block-sleep`, `inhibit-block-shutdown` | auth_admin_keep | yes | yes |
| `inhibit-delay-sleep`, `inhibit-delay-shutdown` | yes | yes | yes |

- "any" is a session that is not local, as ssh. There a user may take an
  inhibitor of `idle` freely, but one of `sleep` or `shutdown` only with
  the password of an admin. In a local session (active or not) it is
  free.
- For a service of its own user later: a rule of polkit for
  `power-off`, `suspend`, `hibernate` and their `-multiple-sessions`; no
  `-ignore-inhibit`.

## The rights to take an inhibitor

`method_inhibit()`, `src/login/logind-dbus.c:3715`:

```c
v == INHIBIT_SHUTDOWN ? (IN_SET(mm, INHIBIT_BLOCK, INHIBIT_BLOCK_WEAK) ? "org.freedesktop.login1.inhibit-block-shutdown" : "org.freedesktop.login1.inhibit-delay-shutdown") :
v == INHIBIT_SLEEP    ? (IN_SET(mm, INHIBIT_BLOCK, INHIBIT_BLOCK_WEAK) ? "org.freedesktop.login1.inhibit-block-sleep"    : "org.freedesktop.login1.inhibit-delay-sleep") :
```

- `block-weak` asks the same right of polkit as `block`. So an inhibitor
  of `sleep` or `shutdown`, weak or not, is free in a local session and
  needs the password of an admin in a session that is not local (ssh).
- polkit judges by the session of the caller. A process in no session
  (a unit of the user manager, `systemd-run --user`) is not in a local
  session; by the rule of polkit it gets `allow_any`, as ssh does
  (from the rules of polkit; its source was not read).

## Root and the inhibitors

- `bus_test_polkit()`, `src/shared/bus-polkit.c:109`: a privileged caller
  (`sd_bus_query_sender_privilege()`, line 128) passes every test of
  polkit.
- `method_can_shutdown_or_sleep()`, `src/login/logind-dbus.c:2770`: with
  an inhibitor held, it tests `*-ignore-inhibit`; root passes it. So
  `CanPowerOff()`, `CanSuspend()` say `yes` to root while a block
  inhibitor is held. `inhibitor-blocked` is never said to root.
- `manager_verify_shutdown_creds()`, `src/login/logind-shutdown.c:139`:
  the action itself refuses a `block` inhibitor for root too, unless
  `SD_LOGIND_SKIP_INHIBITORS`; a `block-weak` one is passed by root unless
  `SD_LOGIND_ROOT_CHECK_INHIBITORS` (notes/logind-interface.md).
- So the service, as root, finds an inhibited action by `BlockInhibited`
  and `BlockWeakInhibited` of the Manager, not by `Can*()`.

## Which kind of inhibitor holds which action

`handle_action_data_table`, `src/login/logind-action.c:28`, field
`inhibit_what`:

| Kind | Actions |
|---|---|
| `sleep` | `suspend`, `hibernate`, `hybrid-sleep`, `suspend-then-hibernate` (lines 88-118) |
| `shutdown` | `poweroff`, `reboot`, `halt`, `kexec`, `soft-reboot` (lines 28-79) |
| none | `factory-reset` (line 124) |

There is no kind of its own for hibernation: the kinds are `shutdown`,
`sleep`, `idle` and the `handle-*` keys (`inhibit_what_from_string()`,
`src/login/logind-inhibit.c:511`). `handle-hibernate-key` holds only the
handling of the key by logind, not the action.

## block-weak

- Modes of an inhibitor: `block`, `block-weak`, `delay`
  (`inhibit_mode_table`, `src/login/logind-inhibit.c:595`;
  `systemd-inhibit --mode=`, `src/login/inhibit.c:251`).
- A `block-weak` inhibitor is shown in `BlockWeakInhibited`, apart from
  `BlockInhibited` (`src/login/logind-dbus.c:325`).
- `manager_get_idle_hint()` calls `manager_is_inhibited(m, INHIBIT_IDLE,
  NULL, 0, ...)` (`src/login/logind-core.c:461`). With flags 0,
  `manager_is_inhibited()` (`src/login/logind-inhibit.c:400`) counts every
  inhibitor that is not `delay` (line 423): `block` and `block-weak`
  alike. So a `block-weak` inhibitor of `idle` keeps the system not idle.
- A `block-weak` inhibitor is not counted for the user who holds it, when
  that user asks for the action (line 430).

## Around a sleep

- `IdleAction` of logind does not act again at once after its own
  action: `idle_action_not_before_usec`, set at the start and after each
  action (`src/login/logind.c:72`, `:1159`); the next action waits a new
  `IdleActionSec` (`:1142`). logind gives no such field on the bus: the
  service keeps its own rule.
- `PrepareForSleep(true)` before, `PrepareForSleep(false)` after
  (`send_prepare_for()`, `src/login/logind-dbus.c:1930`).
- swayidle calls `SetIdleHint(false)` after a sleep
  (notes/sway-inhibitors.md), so the idle state starts again.

## The atime of a tty

The kernel of the image, raspberrypi/linux commit `e4d5c8fd` (7.3.1),
`drivers/tty/tty_io.c`:

- `tty_read()` calls `tty_update_time(tty, false)`: a read of the tty sets
  its atime; a write sets the mtime.
- `tty_update_time()`, line 800: the time is changed only when it differs
  in more than the low three bits of the seconds: a step of 8 s.
- So the idle state of a console is known to within 8 s, plus
  `IdleActionSec`.

## systemctl and the inhibitors

`src/systemctl/systemctl-logind.c` and `src/basic/login-util.h` of
systemd 261.2:

```c
#define SD_LOGIND_ROOT_CHECK_INHIBITORS           (UINT64_C(1) << 0)
#define SD_LOGIND_SKIP_INHIBITORS                 (UINT64_C(1) << 4)
```

```c
SET_FLAG(flags, SD_LOGIND_ROOT_CHECK_INHIBITORS, arg_check_inhibitors > 0);
SET_FLAG(flags, SD_LOGIND_SKIP_INHIBITORS, arg_check_inhibitors == 0);
```

- `systemctl --check-inhibitors=yes suspend` asks with
  `SD_LOGIND_ROOT_CHECK_INHIBITORS`: as root it keeps to `block-weak`
  too, as the service does.
- `systemctl suspend` (the default, `auto`) sets neither flag: as root it
  passes `block-weak`; a `block` one still refuses it.
- `systemctl --check-inhibitors=no` (or `-i`) sets
  `SD_LOGIND_SKIP_INHIBITORS`: it passes every inhibitor.
- The same by `busctl`:
  `busctl call org.freedesktop.login1 /org/freedesktop/login1 org.freedesktop.login1.Manager SuspendWithFlags t 1`.
- Writing to `/sys/power/state` passes logind: no inhibitor, and no
  `PrepareForSleep` (no lock before the sleep).
