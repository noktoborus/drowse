# Requirements

While the system is idle, the service does the actions of its stages,
one after another: a command and its undo, an action of power. It takes
the idle state and the inhibitors from logind, and knows the power
source.

## Terms

- **Idle**: every session of logind is idle, and no inhibitor of logind
  of `idle`, `sleep` or `shutdown` (`block` or `block-weak`) is held,
  whatever `Action` is. Such an inhibitor counts as activity. Sessions
  of all kinds count alike: a graphical one, a console, ssh. A session
  in the state `closing` is left out: after a logout nobody updates its
  idle state (notes/session-end.md). A session of a class not in
  `Classes=` of `[Sessions]` is left out (see "Settings"). The service works this out itself from the
  sessions (`ListSessionsEx()`, the state of each), not from `IdleHint`
  of the Manager, which counts `closing` sessions too.
- **The start of the idle state**: the latest start of idle of the
  sessions counted. It starts no sooner than the last of these moments,
  each counted as activity:
  - a session that was not idle leaves the count (`closing`, removed),
    when the service sees it;
  - the last inhibitor of `idle`, `sleep` or `shutdown` is let go, of
    any kind alike; the service does not count on the session to see
    an inhibitor of `idle`;
  - the power source changes;
  - the service starts (see "Sleep").
- **The move to `closing`** sends no signal on D-Bus. The service learns
  of it by the sd-login monitor (`sd_login_monitor_new("session")`),
  which fires when logind writes the state of a session again: at once
  when the leader exits, at most 20 s after the logout
  (notes/session-end.md).
- **Inhibitor of screen and system**: a Wayland idle inhibitor, set by
  hand or by a program (a video player, a browser). The session does not
  become idle, and no action comes. In sway it holds only while its
  surface is shown: not while the screen is locked, nor while the output
  is off, nor while its window is hidden (notes/sway-inhibitors.md). This
  is meant so: it holds while the screen is used. To keep the system up
  with the screen off or locked, the inhibitor of system is for that.
- **Inhibitor of system**: an inhibitor of logind, `block` or
  `block-weak`, of `idle`, `sleep` or `shutdown`, taken by hand or by a
  program. No action comes. It holds with the screen locked or off. An
  inhibitor of `idle` also stops swayidle (no screen off, no lock); one of
  `sleep` and `shutdown` does not (notes/sway-inhibitors.md).

## Cases

| Case | The action |
|---|---|
| Work in any session (sway, console, ssh) | no |
| All sessions idle, on battery | by the stages of `[IdleBattery]` |
| All sessions idle, on AC | by the stages of `[IdleAC]` |
| An ssh connection is open (with a terminal or not) | no |
| Inhibitor of screen and system | no |
| Inhibitor of system | no |
| The charger is put in or taken out while idle | a new count from that moment |
| The inhibitor of system is taken off, every session idle | a new count from that moment |
| The charge is low or critical | by UPower |

An ssh session is never idle for logind, with a terminal or not
(notes/session-types.md). So it keeps the system up while it is open.

## The power source

- The service takes it from UPower: `OnBattery` and its changes
  (notes/power-source.md).
- With no UPower (upowerd not running, no answer), the service takes the
  power to be AC. So nothing goes down by the time on battery.

## The part in the session

Each graphical session tells logind when it becomes idle and when it is
active again (`SetIdleHint`). Only this is asked of it. With swayidle it
is the line `idlehint <seconds>` (notes/swayidle.md).

Everything else of the idle program of the session is for the user: the
times of the screen off and of the lock, on battery or on AC.

## The action

The actions come in stages, one after another, while the system stays
idle. A stage has a count of idle (`IdleTime`), an announce
(`Countdown`), and may have an action (`Action`) and its undo
(`Resume`). The first stage is `[IdleBattery]` or `[IdleAC]`, by the
power source; the next ones are `[IdleBattery-1]`, `[IdleBattery-2]`, …
(see "Settings").

A stage is entered and left:

- A stage is entered when its time comes: its `IdleTime` and its
  `Countdown` after the start of the idle state (the first stage), or
  after the entry into the stage before (the next ones).
- To enter a stage is to do its `Action`, and to wait for its end. A
  stage with no `Action` is entered with nothing done. An `Action` that
  fails or is killed is a failed entry: the stage has no `Resume` to run.
  The next stage counts its time from that entry all the same.
- To leave a stage is to run its `Resume`, when the stage was entered
  and has one, and to wait for its end. A stage is left before the next
  one is entered, and on activity. A `Resume` that fails or is killed is
  a warning in the journal; the next stage is entered all the same.
- One stage at most is entered at a time, and one job runs at a time: no
  `Action` starts while an `Action` or a `Resume` runs. A job that
  started runs to its end.
- The last stage stays entered while the system is idle; it is left on
  activity.

1. The system becomes idle. The service publishes the action of the
   first stage with its time.
2. The system has been idle for `IdleTime` of the stage. The service
   announces: "action X in `Countdown`". The action comes when the
   countdown ends: `Countdown` after the moment the announce really
   started. So the announce runs in full even when the service was
   stopped for a while (`SIGSTOP`, a freeze of its cgroup) and starts
   late. With `Countdown=Never` there is no announce. The stage entered
   before stays entered during the announce.
3. Any activity before that time cancels the entry: the stage entered is
   left, and the count starts anew at the first stage.
4. Else the stage entered before is left, and the service waits for its
   `Resume`; the state stays `announced` meanwhile. Then the service
   reads logind again: with no activity since the time of the entry, it
   enters the stage, and the next stage counts its `IdleTime` from this
   entry. Activity in that wait cancels the entry, as in 3.
5. After the last stage the service does nothing more until the system
   is active and idle again.

### An inhibitor of system

- An inhibitor of `idle`, `sleep` or `shutdown` makes the state
  `active`: no count, no announce, no action. One taken during the
  announce cancels it. The journal gets one line: the inhibitor, who,
  why.
- When the last one is let go, that moment is activity: the count starts
  anew from it, at the first stage.
- When logind refuses the action as inhibited (`BlockedByInhibitorLock`:
  an inhibitor taken just before the call), the state is `active`, as
  for any inhibitor.

### The power source changes

- A change of the power source is activity: a running announce is
  cancelled, the stage entered is left, and the count
  starts anew from that moment by the stages of the other power source.
  The power that goes and comes back again and again (a bad cable) has
  no rule of its own: each change is activity.
- The journal gets one line: the power source, and that the times moved.

### The action itself

- `Action` is an action of logind or a command. `ActionHint` is the name
  of the action that the bus and the panel show; with none, they show
  `Action` as it is.
- A stage has no default action: with no `Action` it is entered with
  nothing done.
- Just before the action, the service reads the sessions and the
  inhibitors from logind again (`ListSessionsEx()`, `BlockInhibited`,
  `BlockWeakInhibited`), not from what it keeps from the signals.
- The action is inhibited when `idle`, `sleep` or `shutdown` is in
  `BlockInhibited` or `BlockWeakInhibited` of logind, whatever `Action`
  is: a command too. So a program that holds only `sleep` holds a
  `poweroff` too.
- An action of logind is a name of logind: `poweroff`, `halt`,
  `reboot`, `soft-reboot`, `kexec`, `suspend`, `hibernate`,
  `hybrid-sleep`, `suspend-then-hibernate`, `sleep`, `lock`, `ignore`.
  The service calls it through logind (`*WithFlags`,
  `SD_LOGIND_ROOT_CHECK_INHIBITORS`). It has no `Resume`; a `Resume` set for it is passed over, with a
  warning in the journal.
- A command runs in the unit of its stage, `drowsed-stage@.service`
  (instance `battery-0`, `battery-1`, `ac-0`, …), which runs
  `drowsed stage action <instance>`; its `Resume` is the stop of the
  unit, `drowsed stage resume <instance>`. `drowsed stage` reads the
  line from `/etc/drowse.conf`, splits it as a shell does (no shell
  runs: no `|`, `&&`, `$VAR`) and runs it by `exec`; the path is looked
  up in `PATH` of the unit.
- The service does not wait for the end of a command. Its exit code
  gives nothing; its output is in the journal of the unit.
- A command has 5 s to run (`TimeoutStartSec=5s`), its `Resume` 5 s
  (`TimeoutStopSec=5s`). Past it systemd kills it: a warning in the
  journal; an `Action` killed so has no `Resume`. A long task does not
  belong in a stage.
- A stage with no `Action` but a `Resume` has a unit too: its start
  runs nothing, its stop the `Resume`.
- `Resume` runs always when the stage was entered: before the next stage
  is entered, on activity, an inhibitor let go, a change of the power
  source, a stop of the service, a sleep. The service keeps a queue of
  jobs for each unit: a start, then a stop, one at a time, the next when
  the one before has ended (`JobRemoved` of systemd). So a stop never
  cuts a start that runs. An entry waits until no job of any unit runs.
  A stop that waits for a command that still runs is a warning in the
  journal.
- When logind refuses the action, or the unit of a command cannot be
  started, the service writes the refusal to the journal, with the
  answer (for a sleep, the error `SleepVerbNotSupported` and its text:
  disabled by the settings, not in the kernel, no place to resume from).
  It is a failed entry, not a stop and not a state: the next stage
  comes as if the action had run.
- At its stop (a sleep, `systemctl stop`, a restart) the service waits
  for the commands that run, stops every unit of a stage that ran (so
  their `Resume` runs), then exits: at most 10 s
  (`TimeoutStopSec=10s`). At its start it stops every unit of a stage
  that is left from before.

### Sleep

- systemd stops the service before any sleep and starts it after the
  wake; the service has no code of its own for a sleep and does not
  follow `PrepareForSleep`.
- Before a sleep: `drowsed.service` has `Conflicts=sleep.target` and
  `Before=sleep.target`. The units of the stages that ran stop with it
  (`PartOf=`), so each `Resume` runs before the sleep.
- After the wake: `drowsed-resume.service`, wanted by `suspend.target`,
  `hibernate.target`, `hybrid-sleep.target`,
  `suspend-then-hibernate.target` and ordered after them, starts the
  service again. A start is a new count.
- A sleep that does not go through the targets of systemd (a write to
  `/sys/power/state`) is not seen.
- A limit: nothing goes through a sleep. While the system sleeps the
  service is not there; after the wake it starts from the first stage. So
  a stage after a sleep (`suspend`, then `poweroff` some hours later) does
  not come: only systemd can wake the system for it. For a sleep, then
  hibernation, systemd has `suspend-then-hibernate`; a sleep, then a
  power off, the project does not do.

## The state on the bus

The service publishes its state on the system bus, with
`PropertiesChanged` on each change. A program that shows it (the tool of
the user, a panel) reads it there and does not work it out again.

| Property | Value |
|---|---|
| `State` | `s`: `active` (not idle: a session in use or an inhibitor), `idle` (the time runs), `announced` (the warning runs) |
| `ScheduledAction` | `(st)`: the action of the next stage (its `ActionHint`, or `Action` as it is), and when it comes; `("", 0)` when none comes and in `active` |
| `HeldBySessions` | `a(sussss)`: the sessions that hold the system up; on the object `/io/github/noktoborus/Drowse/HeldBy` |
| `HeldByInhibitors` | `a(ssssuu)`: the inhibitors that hold the system up, in the form of `ListInhibitors()` of logind: what, who, why, mode, uid, pid; on the object `/io/github/noktoborus/Drowse/HeldBy` |
| `PowerSource` | `s`: `battery`, `ac`, or `unknown` when UPower does not answer (the power is then taken as AC) |

- A time is in microseconds of `CLOCK_MONOTONIC`.
- A session in `HeldBySessions` has the fields of logind as they are:
  id, uid, user, type, tty, remote host. The service gives no kind of
  its own.
- What they hold by the state: in `active`, the sessions that are not
  idle and the inhibitors that hold (see "Idle"); in `idle` and
  `announced`, nothing. Only the modes `block` and `block-weak`, never
  `delay`.
- A console that becomes idle sends no signal (notes/logind-interface.md).
  So a console can stay in `HeldBySessions` after it is idle, up to the
  next signal or time of the service.
- One change of the state sends one `PropertiesChanged` per object, with
  every property of it that changed.
- When the service is not on the bus (a restart), a reader shows the
  state as not known. It follows the name by `NameOwnerChanged`.
- The name and the interface are `io.github.noktoborus.Drowse`. Two
  objects:
  - `/io/github/noktoborus/Drowse`: `State`, `ScheduledAction`,
    `PowerSource`.
    Anyone may read it: every user can learn of an action to come.
  - `/io/github/noktoborus/Drowse/HeldBy`: `HeldBySessions`,
    `HeldByInhibitors`. Only the group `wheel` may read it.
- The rights are set by the policy of D-Bus of the package, by the path
  of the object (`send_path`, `receive_path`), not by the service. The
  policy of D-Bus filters by the object, not by the name of a property.
- logind still shows its sessions and inhibitors to anyone
  (`ListInhibitors()`, `loginctl`). To close them is a setting of the
  system, not of drowse.

The interface on the bus is for reading only: properties and their
changes, no methods. Anyone reads the state; only the group `wheel`
reads the details (the object `HeldBy`), by one rule of the policy of
D-Bus.

**No way round the system.** The service takes no commands of its own:
no method on the bus, no socket, no file to write to, no signal of its
own. Nothing can hold, hurry, cancel or change what it does except by the
mechanisms of the system, which check the rights themselves:

- to hold the action: an inhibitor of logind (`systemd-inhibit`,
  `drowse-inhibit`);
- to cancel an announce: any activity in a session;
- to change the settings: the settings file, then a restart of the
  unit (`systemctl`).

A reader learns of the announce by `State`: it becomes `announced`. It
does not work out the time of the announce. With `Countdown=Never`,
`State` is never `announced`.

The service writes no `wall`: while an ssh session is open the system is
never idle, so no announce comes; a console sees it only when its text
screen is shown.

`ScheduleShutdown` of logind is not used: it has no sleep, it makes
`/run/nologin` and so stops a cancel by a new login, and it does not
check a `block` inhibitor at the time of the action
(notes/logind-schedule-shutdown.md).

## The journal

The service writes to the journal, short and clear, one line for one
change. A state that has not changed is not written again: a read by the
timer that finds the same is silent.

It writes:

- **A change of state**: idle or active; the power source; an inhibitor
  taken or let go, with who holds it and why; the time of the action set,
  and when it is.
- **The announce and the action**: the action announced; the action
  done.
- **Why the action is cancelled**: activity in a graphical
  session or a console (its id, user); a new session, ssh too (its id,
  user, kind); an inhibitor (who, why); a change of the power source.
- **A source that is not there**: logind not on the bus; UPower not
  there, so the power is taken as AC; an action that logind refuses,
  with its answer.
- **A command of a stage**: started; its `Resume`; a stop that waits for
  a command that still runs; a command killed past its 5 s.

A source that comes back is written once, too.

Examples:

```
idle, on battery: poweroff in 17 min
announce: poweroff in 2 min
action: poweroff
cancel: session 4 (alice) in use
cancel: inhibitor of sleep:shutdown by "drowse-inhibit" (uid 1000): "emerge"
inhibitors let go: a new count
power: AC; the times moved
idle, on AC: no action
upower: no answer; the power is taken as AC
stage 1 (radio off): started
drowsed-stage@battery-1.service: the command still runs after 4 s; its Resume waits
drowsed-stage@battery-1.service: the command ended; its Resume runs late
suspend refused by logind: org.freedesktop.login1.SleepVerbNotSupported: Sleep verb 'suspend' is not configured or configuration is not supported by kernel
```

A time in the journal is the time left (`in 17 min`); the times on the
bus are of `CLOCK_MONOTONIC`.

The levels: a change of state is `info`; a cancel is `notice`; a source
not there, a refusal, a command that waits or is killed is `warning`.

## Settings

System settings only. A user has no settings of their own in the service.

The settings are in sections, a section a stage (see "The action"). The
first stage on battery is `[IdleBattery]`, the same as `[IdleBattery-0]`;
the next ones are `[IdleBattery-1]`, `[IdleBattery-2]`, …; the same for
`[IdleAC]`. The power source now picks the stages. The numbers go one by
one from 1; a gap ends the stages there, with a warning in the journal.
When both `[IdleBattery]` and `[IdleBattery-0]` are there, the service
writes a warning and goes on: they are one section, the keys of both
apply, the later one wins.

Each key may be in each section; a key that is not there takes its
default, the same in every stage. `Action` has no default: a stage with
none is entered with nothing done. The default `IdleTime` is `Never`:
with no settings file the service does nothing.

| Key | Meaning | Default in `[IdleBattery]` | Default in `[IdleAC]` |
|---|---|---|---|
| `IdleTime` | idle time before the announce: a time or `Never` (no announce, no entry). For a further stage, counted from the entry into the stage before it | `Never` | `Never` |
| `Countdown` | how long the announce runs before the action: a time or `Never` (no announce: the action comes at `IdleTime`). It is added to `IdleTime`: with `IdleTime=15m` and `Countdown=2m` the announce starts at 15 min of idle, the action comes at 17 min | `2m` | `2m` |
| `Action` | an action of logind, or a command; with none, the stage is entered with nothing done | none | none |
| `ActionHint` | the name of the action that the bus and the panel show; with none, they show `Action` as it is | none | none |
| `Resume` | a command that undoes `Action` when the system is active again; for a command only | none | none |

Example:

```ini
[IdleBattery]
IdleTime=0
Countdown=Never
Action=tuned-adm profile powersave
ActionHint=power save
Resume=tuned-adm auto_profile

[IdleBattery-1]
IdleTime=5m
Countdown=Never
Action=nmcli radio all off
ActionHint=radio off
Resume=nmcli radio all on

[IdleBattery-2]
IdleTime=30m
Countdown=2m
Action=poweroff

[IdleAC]
IdleTime=Never
```

On battery: at once the profile of tuned is power save; at 5 min tuned
picks its profile again (the power save is left), then the radio is off;
30 min later the announce, and 2 min after it the radio is on again and
the system powers off. When the user comes back while a stage is
entered, only that stage is left: the profile of tuned comes back, or
the radio is on. On AC: nothing.

`tuned-adm` has no command to go back to the profile before;
`auto_profile` goes back to the profile it recommends. A user who picks
the profile by hand names it: `Resume=tuned-adm profile balanced`.

### The sessions that count

The section `[Sessions]`, one key:

| Key | Meaning | Default |
|---|---|---|
| `Classes` | the classes of logind of the sessions that count in the idle state, split by spaces; a session of another class is left out; empty leaves out all | `user user-early user-light user-early-light greeter` |

- A name is compared with the class of a session as it is. Any name is
  taken, unchecked, so a class of a newer logind can be named.
- The default is the classes logind counts itself
  (`SESSION_CLASS_CAN_IDLE`, notes/logind-idle.md).
- A login screen (`greeter`) that sets no idle hint is never idle and
  keeps the system up while it is shown. `Classes=user user-early
  user-light user-early-light` leaves it out: with nobody logged in, the
  system is then idle.
- The settings file says how to list the classes logind knows
  (`varlinkctl introspect`) and the class of each session
  (`loginctl list-sessions`).

### A change of the settings

- The service does not read its settings again while it runs; it is
  restarted. It keeps no state that matters: the idle state, the
  sessions, the inhibitors and the power source come again from logind
  and UPower at the start.
- At the start the count starts anew: the idle state starts no sooner
  than the start. An announce that ran before the restart is gone.
- The settings are in one file, `/etc/drowse.conf`, in the form of the
  files of systemd: `[Sessions]` and the sections of the stages
  (`[IdleBattery]`, `[IdleBattery-1]`, …, `[IdleAC]`, …), then
  `Key=value`. There are no drop-ins. With no file, the defaults apply:
  nothing is done.
- The package installs the file with its stages: on battery
  `tuned-adm profile cm5-powersave` at 10 min of idle (no
  announce, `Resume=systemctl try-restart tuned-ppd.service`), then
  `poweroff` 38 min later with an announce of 2 min, at 50 min of idle;
  on AC nothing. Every other key is there with its default in a comment.
  With no tuned the first command fails, and the `poweroff` still comes.
- The package gives the profile of TuneD `cm5-powersave` as an
  example (`examples/tuned/`, installed with the documentation): for
  the BCM2712, `powersave`, the CPU at 800 MHz at most, USB autosuspend
  on every device but those of `KEEP` of `usb-autosuspend.sh` (the
  keyboard `434b:5543` and its bootloader `1eaf:*`) and the hubs they
  hang on, which never suspend, the dirty pages written once a minute,
  runtime PM of every PCI device, and printing and the timers of
  maintenance stopped. It gives also `cm5-powersave-no-radio`, a deeper
  step: `cm5-powersave`, and every radio off by its soft block of
  rfkill, the values before written back at its end; the file of the
  package does not use it. The user copies them to
  `/etc/tuned/profiles/`, unless `tuned-adm list` shows them already.
  Neither is in
  `ppd.conf` of tuned-ppd: tuned-ppd then keeps its base profile, the
  one the user chose, and sets it again at its restart
  (`initialize()` of `tuned/ppd/controller.py`).
- A unit `drowsed-config.path` watches `/etc/drowse.conf`; on a change
  it runs `systemctl try-restart drowsed.service`.
- `drowsed.service`, `drowsed-config.path` and `drowsed-resume.service`
  are on by default: a preset file of the package enables them
  (`/usr/lib/systemd/system-preset/50-drowse.preset`). The files of the
  system are in `systemd/`, `preset/` and `dbus/`.

### Times

`IdleTime=0`: the announce comes as soon as the system is idle.
`IdleTime=Never`: no announce, no action. `Countdown=0`: the announce is
published and the action comes at once. `Countdown=Never`: no announce;
the action comes at `IdleTime`.

A time in the settings is one or more parts, each a number and a unit.
The parts are added. Spaces between the parts and between a number and
its unit may be there or not.

| Unit | Means |
|---|---|
| `s`, `sec` | seconds |
| `m`, `min` | minutes |
| `h`, `hr` | hours |

- A number with no unit is seconds.
- The form is that of a time span of systemd (`systemd.time(7)`), so a
  time reads the same as in a unit file.

Examples: `2h`; `1m 120s`, `1m120s` (both 3 minutes); `90` (90 seconds).

## Not the service

- What a logout leaves running. The service leaves `closing` sessions
  out; the documentation (README.md) still recommends
  `KillUserProcesses=yes`, with why.
- The screen and the lock: the idle program of the session, set by the
  user.
- Low and critical charge: UPower (`CriticalPowerAction=`).
- The interface: warnings, notifications, sound, the screen on at a
  warning, the state of the inhibitors, the lifetime of the inhibitors set
  by hand. See the branch `plans`: `cli.md` (the tool and its module of
  waybar), `notify-daemon.md` (the daemon of the interface).
