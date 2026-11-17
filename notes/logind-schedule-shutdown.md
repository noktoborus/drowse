# The standard way to tell of an action to come: ScheduleShutdown

## Question

Does logind have a way to tell all users that the system goes down in N
minutes? Can the service use it for its announce? Read in the sources of
systemd 261.2.

## What it does

`ScheduleShutdown(s type, t usec)` of the Manager
(`method_schedule_shutdown()`, `src/login/logind-dbus.c:2644`). It is
what `shutdown -h +5` and `systemctl poweroff --when=` call.

- It publishes `ScheduledShutdown (st)`: the action and its time, with
  `PropertiesChanged` (`logind-dbus.c:2709`, `:3842`). The time is
  `CLOCK_REALTIME`.
- It writes `wall` to the terminals, more often as the time comes near:
  at 180, 150, 130, 100, 70, 55, 40, 25, 10 to 1 minutes before
  (`when_wall()`, `src/login/logind-wall.c:16`). `EnableWallMessages`
  turns it off.
- It makes `/run/nologin` 5 minutes before the action
  (`nologin_timeout_usec()`, `logind-dbus.c:2438`). From then on only
  root can log in. When less than 5 minutes are left, the file comes at
  once.
- logind does the action itself at the time
  (`manager_scheduled_shutdown_handler()`, `logind-dbus.c:2493`).
- `CancelScheduledShutdown()` cancels it (`logind-dbus.c:2714`).
- A type with the prefix `dry-` (`dry-poweroff`) does all of this but the
  action (`logind-dbus.c:2659`, `:2513`). `/run/nologin` still comes.

## Why it does not fit the service

1. Only the actions of shutdown: `poweroff`, `reboot`, `halt`, `kexec`,
   `soft-reboot` (`HANDLE_ACTION_IS_SHUTDOWN`,
   `src/login/logind-action.h:48`). `suspend` and `hibernate` are
   refused: "Unsupported shutdown type".
2. `/run/nologin` stops a cancel. With an announce of less than 5
   minutes, nobody but root can log in during it. A new session is the
   way a user cancels the action (REQUIREMENTS.md).
3. A `block` inhibitor is checked when the action is scheduled, not when
   it runs. At the time the handler only waits for `delay` inhibitors
   (`bus_manager_shutdown_or_sleep_now_or_later()`). An inhibitor taken
   during the count, by `drowse-inhibit` for one, does not stop it.
4. logind does the action, not the service: no chain, no `leave`.
5. `wall` is on by default; the service writes no `wall`.

## What there is not

A system service has no standard way to show a notification to a user.
Notifications are `org.freedesktop.Notifications` on the session bus of
each user. A program in the session reads the state on the system bus
and shows it.

## So

The service publishes its own state. `ScheduledAction (st)` has the form
of `ScheduledShutdown`: the action and its time.
