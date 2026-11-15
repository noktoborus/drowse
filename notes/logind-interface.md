# What the service takes from logind, and what logind lacks

## Question

What does the service read from logind and call in it? What does logind
not give? Does turning the screen off need a part in the session of a
user? Read in the source of systemd 261.2 on 2026-11-14. See also
notes/logind-idle.md.

## What logind gives

`org.freedesktop.login1.Manager` on the system bus:

| Name | Kind | For |
|---|---|---|
| `IdleHint`, `IdleSinceHint`, `IdleSinceHintMonotonic` | properties | the idle state of the system and its start |
| `BlockInhibited`, `BlockWeakInhibited`, `DelayInhibited` | properties | which kinds are inhibited (`shutdown`, `sleep`, `idle`, ...) |
| `ListInhibitors()` | method | who holds an inhibitor and why, for the journal |
| `CanSuspend()`, `CanHibernate()`, `CanPowerOff()`, ... | methods | the chain: can the action be done now |
| `SuspendWithFlags()`, `HibernateWithFlags()`, `PowerOffWithFlags()`, ... | methods | the action |
| `PrepareForSleep`, `PrepareForShutdown` | signals | the system goes down or to sleep by another way |
| `ListSessions()`, `Session.SetIdleHint()` | method | the sessions; a graphical session sets its idle hint |

## The signals of logind

From the tables of the objects of systemd 261.2 (`SD_BUS_SIGNAL` and
the properties marked `EMITS_CHANGE`).

`org.freedesktop.login1.Manager` (`src/login/logind-dbus.c:4209`):

- signals: `SessionNew`, `SessionRemoved`, `UserNew`, `UserRemoved`,
  `SeatNew`, `SeatRemoved`, `PrepareForShutdown`,
  `PrepareForShutdownWithMetadata`, `PrepareForSleep`,
  `SecureAttentionKey`;
- `PropertiesChanged` of: `IdleHint`, `IdleSinceHint`,
  `IdleSinceHintMonotonic`, `BlockInhibited`, `BlockWeakInhibited`,
  `DelayInhibited`, `NCurrentInhibitors`, `ScheduledShutdown`,
  `LidClosed`.

`org.freedesktop.login1.Session` (`src/login/logind-session-dbus.c:1088`):

- signals: `PauseDevice`, `ResumeDevice`, `Lock`, `Unlock`;
- `PropertiesChanged` of: `TTY`, `Display`, `Type`, `Class`, `Active`,
  `State`, `IdleHint`, `IdleSinceHint`, `IdleSinceHintMonotonic`,
  `LockedHint`.

`org.freedesktop.login1.User` (`src/login/logind-user-dbus.c`):
`PropertiesChanged` of `Display`, `IdleHint`, `IdleSinceHint`,
`IdleSinceHintMonotonic`, `Linger`.

### Checked in all the code of logind

Every change sent by logind goes through `session_send_changed()`,
`seat_send_changed()`, `user_send_changed()`, `manager_send_changed()`,
each with the names of the properties given by the caller; there is no
send of all properties at once (`sd_bus_emit_properties_changed_strv`
only inside these, `src/login/logind-*-dbus.c`). All calls in
`src/login/*.c`:

- `IdleHint`, `IdleSinceHint`, `IdleSinceHintMonotonic`: only in
  `session_set_idle_hint()` (`logind-session.c:1233-1239`), for the
  session, its seat, its user and the Manager at once. Nowhere else: a
  console that becomes idle or active, an inhibitor of `idle` taken or let
  go, a session that goes or comes, send no change of the idle hint.
- Inhibitors: `bus_manager_send_inhibited_change()`
  (`logind-inhibit.c:151`): `BlockInhibited` (for `block` and
  `block-weak`) or `DelayInhibited`, and `NCurrentInhibitors`.
- `State` of a session: never sent. `Active` of a session: sent by the
  seat when the session in front changes (`logind-seat.c:231`, `:444`).
- Others: `Display` of a user, `ActiveSession` and `CanGraphical` of a
  seat, `LockedHint`, `Type`, `Class`, `Display`, `TTY` of a session,
  `Linger`, `LidClosed`, `ScheduledShutdown`.

### The documentation

`man/org.freedesktop.login1.xml` of systemd 261.2:

- "Whenever the inhibition state or idle hint changes,
  `PropertyChanged` signals are sent out to clients which have
  subscribed." The code sends the idle hint only on `SetIdleHint`: the
  idle hint of the Manager also changes with a console and with an
  inhibitor of `idle`, and that is not sent. The text says more than the
  code does.
- `State` of a session is listed with no `EmitsChangedSignal`
  annotation, so by the rule of D-Bus it emits changes; the code never
  sends it. `State` of a user is marked `EmitsChangedSignal("false")`.

A property marked so is sent only where the code calls
`*_send_changed()` for it. `State` is marked, but the start of `closing`
(`session_release()`) and `session_stop()` send nothing
(notes/session-end.md). `IdleHint` is sent only on `SetIdleHint` of a
graphical session; a console is worked out on each read.

## The inhibitors at the action

`manager_verify_shutdown_creds()`, `src/login/logind-shutdown.c:139`.

- An inhibitor of mode `block` refuses the action outright, for root too,
  unless the call has `SD_LOGIND_SKIP_INHIBITORS`.
- An inhibitor of mode `block-weak` is passed by root, unless the call has
  `SD_LOGIND_ROOT_CHECK_INHIBITORS`.
- So the service calls `*WithFlags` with `SD_LOGIND_ROOT_CHECK_INHIBITORS`
  and never with `SD_LOGIND_SKIP_INHIBITORS`. A refusal
  (`BlockedByInhibitorLock`) is an inhibitor of the system.
- `CanPowerOff()` and the like say `yes` to root while an inhibitor is
  held. The service finds an inhibited action by `BlockInhibited` and
  `BlockWeakInhibited` (notes/logind-power-rights.md).

## The sessions one by one

`method_list_sessions_ex()`, `src/login/logind-dbus.c:704`:

- `ListSessionsEx()` gives for each session: id, uid, user, seat, leader,
  class, tty, idle hint, idle since (monotonic), object path. The idle
  hint is worked out as for `IdleHint` (`session_get_idle_hint()`), so a
  console is right at the time of the call.
- The state (`online`, `active`, `closing`) is not in it: it is the
  property `State` of each session, or `sd_session_get_state()` of
  sd-login.
- The move to `closing` sends no signal on D-Bus
  (notes/session-end.md). It is seen by the sd-login monitor (inotify on
  `/run/systemd/sessions`).

So a watcher can work out the idle state of the system with the sessions
in `closing` left out: the sessions of the classes that count, not
`closing`, all idle, and no inhibitor of `idle` (`block` or
`block-weak`, from `BlockInhibited` and `BlockWeakInhibited`).

## Who sees which inhibitors

`method_list_inhibitors()`, `src/login/logind-dbus.c:801`:

- `ListInhibitors()` gives every inhibitor of every user: what, who,
  why, mode, uid, pid. No filter by the caller, no test of polkit; the
  method is `SD_BUS_VTABLE_UNPRIVILEGED` (line 3906), and the D-Bus policy
  lets anyone call it (`org.freedesktop.login1.conf`, `ListInhibitors`).
- So any user, and the service, sees the inhibitors of all users.
- The time since an inhibitor is held is not in the answer. Only the
  kinds held, joined, are in `BlockInhibited` and the like.
- Each start and stop of an inhibitor sends `PropertiesChanged` of the
  Manager with `BlockInhibited` (for `block` and `block-weak`) or
  `DelayInhibited` (for `delay`), and `NCurrentInhibitors`
  (`bus_manager_send_inhibited_change()`, `src/login/logind-inhibit.c:151`,
  called at lines 176 and 197). The signal comes even when the joined
  kinds stay the same. So a watcher reads `ListInhibitors()` again on it.

## What logind lacks

1. **The power source.** logind does not know AC from battery. It
   comes from UPower (`OnBattery`) or from `/sys/class/power_supply`
   (`systemd-ac-power`).
2. **A signal when a tty session becomes idle.** The `IdleHint` of the
   Manager is sent as changed only when a graphical session calls
   `SetIdleHint` (`src/login/logind-session.c:1239`). The idle state of a
   tty is worked out on each read from the atime of the tty. A watcher
   sees a console change only when it reads again: on another signal, at
   a time of its own, or by a timer. A new ssh session comes by
   `SessionNew`; an open one is never idle (notes/session-types.md).
3. **The idle state of a graphical session.** logind does not measure it.
   A part of the session must call `SetIdleHint`. With no such part, the
   session is never idle, and so the system is never idle.
4. **The Wayland inhibitors.** logind does not see them. They are seen
   only in the compositor: a session with one does not set its idle
   hint.
5. **The screen.** logind has no call to turn an output off.
6. **The announce.** logind has no signal "action X in N seconds".
7. **Times by power source.** `IdleAction` has one time and one action.

## The screen and the part in the session

A part in each graphical session is needed whatever is done with the
screen (point 3): it measures the idle state through the compositor
(`ext-idle-notify-v1`, which keeps the Wayland inhibitors) and calls
`SetIdleHint`. It runs in the session of the user. The login screen is a
task of its own: the branch `plans`, login-screen.md.

Who turns the screen off:

| Way | By | Good | Bad |
|---|---|---|---|
| The output off through the compositor (`wlr-output-power-management-v1`, `swaymsg output * power off`) | the part in the session | the compositor knows the screen is off and stops drawing | needs the part in the session |
| The backlight off in sysfs (`/sys/class/backlight/*/bl_power`) | the system service | no part in the session for it | the panel and the compositor keep working; the compositor does not know |
| DPMS of DRM | nobody | — | only the DRM master, the compositor, may set it |

The part in the session exists anyway, so the screen is there too. The
service asks of it only `SetIdleHint` (REQUIREMENTS.md). The screen and
its times are for the user, in the idle program of the session: swayidle
with `idlehint` and any other timeouts (notes/swayidle.md).

## So

- A system service: reads logind and the power source, counts the time
  of the action, announces, does the action through logind.
- A part in each graphical session: the idle hint for logind. The screen
  is set there by the user.
