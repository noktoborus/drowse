# How a process sees the end of its session

## Question

Can a process of its own in the background (not a unit) see that its
session has ended, to let its inhibitor go? Read on 2026-11-14 in systemd
261.2 (tag `v261.2`).

## What happens at the logout

- The session is stopped (`session_stop()`, `src/login/logind-session.c`
  from line 1002): `s->stopping = true` (line 1027), and the state file is
  written again (`session_save()`, line 1031).
- `session_get_state()`, line 1377: `stopping` gives the state `closing`.
- With `KillUserProcesses=no` (the default) the processes left in the
  scope are not killed. The session stays `closing` until they are gone;
  `SessionRemoved` comes only then. A process that waits for
  `SessionRemoved` waits for itself.

## The ways to see it

| Way | Seen | Note |
|---|---|---|
| sd-login: `sd_login_monitor_new("session")`, then `sd_session_get_state()` | `closing` | reads `/run/systemd/sessions/<id>` (`STATE=`, written at line 366) with inotify; no D-Bus |
| D-Bus: `State` of `org.freedesktop.login1.Session` | `closing` on a read | declared `EMITS_CHANGE` (`src/login/logind-session-dbus.c:994`), but `session_stop()` sends no `PropertiesChanged` for it: no call with "State" found. So a read by a timer, not a signal |
| D-Bus: `SessionRemoved` of the Manager | too late | only when no process is left |
| The leader: `Leader` of the Session (`logind-session-dbus.c:988`), `pidfd_open()` and poll | the leader exits | the leader of sway is the worker of greetd; of ssh, the monitor of sshd-session. Its exit starts the stop of the session |

## So

Yes, a process of its own can see the end: by the sd-login monitor
(`closing`), or by a pidfd on the leader. Both are code that a unit of the
user manager does not need: the manager stops after the last session of
the user (with no linger), and its units with it.

## The sd-login monitor and a pidfd on the leader

| The moment | A pidfd on the leader | The sd-login monitor |
|---|---|---|
| The leader exits after PAM is closed (the usual logout) | fires | fires: logind writes the file of the session then |
| The leader dies or is killed | fires | fires, as above |
| `loginctl terminate-session` | fires (the leader is killed) | fires |
| PAM closed, the leader lives on | never | fires after 20 s, by the release timer |
| A new session | no | fires (D-Bus also sends `SessionNew`) |
| `ReleaseSession` itself (the start of `closing`) | no | no: logind writes no file then |

A pidfd sees the death of one process; the state of the session is read
from logind after it anyway. The monitor sees logind write the state, so
it sees a session closed by the timer as well. A pidfd needs one fd for
each session, kept on `SessionNew` and `SessionRemoved`, and a check
against `LeaderPIDFDId`. Taken: the monitor (REQUIREMENTS.md).

## A leaving session and the start of the idle state

Example: on AC, `main.ac[0].after` = 35 min. Session B is idle
since 12:00.
Session A (sway) is in use; at 12:30 its user logs out and leaves tmux;
A goes `closing` with its idle hint "not idle".

- Left out by the start of idle of B only: the idle state would start at
  12:00, and the action would come at 12:35, five minutes after the
  logout.
- The logout is the last activity. So a session that was not idle and
  leaves the count counts as activity at the moment it is seen: the idle
  state starts at 12:30, the action comes at 13:05 (REQUIREMENTS.md).

With the monitor, A is seen `closing` at 12:30 (its leader, the worker of
greetd, exits then). With no monitor:

- `KillUserProcesses=yes`: logind kills tmux at 12:30:20 and removes A;
  `SessionRemoved` comes; the idle state starts at 12:30:20.
- `KillUserProcesses=no`, tmux alive: no signal comes; a service with no
  monitor keeps A as "not idle" until any other event.

## What leaving out `closing` costs

- Work left running after a logout (a build, a download, tmux) does not
  hold the system up; the counting of logind would have held it, by the
  stale hint. It is held by an inhibitor (README.md).
- The idle state of the service and `IdleHint` of logind differ while a
  `closing` session with a stale hint is there.

## The states

### Of a session

`session_get_state()`, `src/login/logind-session.c:1377`, in this order:

```c
if (s->stopping || s->timer_event_source)
        return SESSION_CLOSING;
if (s->scope_job || !pidref_is_set(&s->leader))
        return SESSION_OPENING;
if (session_is_active(s))
        return SESSION_ACTIVE;
return SESSION_ONLINE;
```

| State | Means |
|---|---|
| `opening` | the scope is being made, or no leader yet |
| `online` | logged in, not in the front of its seat (a VT not shown, ssh) |
| `active` | logged in and in the front of its seat |
| `closing` | being stopped: the logout came, processes may be left |

The timer of `closing` (`session_release()`, line 1097,
`RELEASE_USEC` = 20 s at line 55):

- At the logout `pam_systemd` (in `pam_close_session`) calls
  `ReleaseSession` (`src/login/pam_systemd.c:1910`; the method at
  `src/login/logind-dbus.c:1373`). logind arms a timer of 20 s; from
  that moment the session is `closing`.
- When it fires, `release_timeout_callback()` (line 1088) calls
  `session_stop(s, false)`: `stopping` is set; with
  `KillUserProcesses=yes` the processes are killed then.
- So the processes have 20 s to end by themselves. The session is
  `closing` both in those 20 s and after them, while processes are left.

All four count in the idle state of logind (`manager_get_idle_hint()`
looks only at the class).

### Of a user

`user_get_state()`, `src/login/logind-user.c:863`: `offline`, `opening`,
`lingering`, `online`, `active`, `closing`.

- `closing`: the user is being stopped, or all its sessions are
  `closing` (line 888).
- `lingering`: no session, linger on, the user manager runs.
- `active`: a session of it is `active`; `online`: it has sessions, none
  `active`.

"No session left" for a user is a state of `closing`, `lingering` or
`offline`.

### The idle state of a user

`user_get_idle_hint()`, `src/login/logind-user.c:683`: the same rule as
for the system, over the sessions of the user; only the class is looked
at, so a `closing` session counts here too. The state of the user is
`closing` only when all its sessions are `closing`.

So the state of the user does not leave out a stale `closing` session
when the user has another session: after a logout and a new login, the
old session (`closing`, hint "active") and the new one (idle) give the
user the state `active` or `online` and the idle hint "not idle". Only
the state of each session tells the stale one.

### Linger and the session of the user manager

- Linger by itself makes no login session. It keeps the user manager
  (`user@<uid>.service`) running with no session of the user.
- The user manager has a session of its own in logind: `pam_systemd`, in
  the PAM stack `systemd-user`, gives it the class `manager`
  (`manager-early` for a system user) and the type `unspecified`
  (`src/login/pam_systemd.c:928`).
- It does not count in the idle state: `SESSION_CLASS_CAN_IDLE` has no
  `manager` (`src/login/logind-session.h:51`).
- It does not keep the user `online` or `active`: it does not pin the
  user (`src/login/logind-session.h:48`):

  ```c
  #define SESSION_CLASS_PIN_USER(class) (!IN_SET((class), SESSION_MANAGER, SESSION_MANAGER_EARLY, SESSION_NONE))
  ```

  With linger and no other session, the user is `lingering`.

## When the sd-login monitor sees `closing`

`sd_login_monitor_new()`, `src/libsystemd/sd-login/sd-login.c:1183`:
an inotify on `/run/systemd/sessions/` (for `session`) or
`/run/systemd/users/` (for `uid`), with `IN_MOVED_TO|IN_DELETE`. It
fires when logind writes the file of a session again (`session_save()`
writes a new file and moves it in place).

`session_release()` (the start of `closing`, `logind-session.c:1097`)
does not call `session_save()`. The file is written again by
`session_stop()` (line 1031). That comes either when the release timer
fires (20 s), or at once when the leader exits: logind watches the
leader by a pidfd (since v258) and stops the session then
(`session_dispatch_leader_pidfd()`, line 98):

```c
static int session_dispatch_leader_pidfd(sd_event_source *es, int fd, uint32_t revents, void *userdata) {
        Session *s = ASSERT_PTR(userdata);
        ...
        session_stop(s, /* force= */ false);
```

At the logout of sway:

1. sway ends: logind drops its controller (`session_drop_controller()`,
   line 1650) and writes the file: the monitor fires; the state is still
   `active`.
2. greetd closes the PAM session: `ReleaseSession`; the state on D-Bus
   is `closing`; the file is not written yet.
3. The worker of greetd, the leader, exits just after: logind stops the
   session and writes `STATE=closing`; the monitor fires.

So the sd-login monitor sees `closing` when the leader exits, which is at
once in the usual case; only a leader that lives on after
`ReleaseSession` makes it wait for the timer of 20 s.
`sd_session_get_state()` reads the file, so it says `closing` from the
same moment; `State` on D-Bus says it from step 2.

A pidfd of the service on the leader fires at the same moment as the
monitor. logind has done that watch already.

## Who is the leader

- `pam_systemd`, in `pam_open_session`, calls `CreateSessionWithPIDFD`
  with a pidfd of its own process (`pidfd_open(getpid_cached(), 0)`,
  `src/login/pam_systemd.c:852`): the leader is the process that opens
  the PAM session.
- So the leader is the program of the login: the worker of greetd,
  `login` of a console, the privileged monitor of sshd-session, the user
  manager for its own session. They open the PAM session, start the
  session in a child, and wait for it.
- `Leader` (pid) and `LeaderPIDFDId` (the id of the pidfd) are
  properties of each session (`src/login/logind-session-dbus.c:988`).
  A pidfd opened by the service can be checked against `LeaderPIDFDId`,
  so the race between the pid and `pidfd_open()` is closed.

## When the leader exits, and when it does not

greetd 0.10.3, `greetd/src/session/worker.rs:290`: the worker waits for
the session command, then closes the PAM session, then returns:

```rust
loop {
    match waitpid(child, None) {
        Err(nix::errno::Errno::EINTR) => continue,
        ...
        Ok(_) => break,
    }
}
pam.close_session(PamFlag::NONE)?;
pam.setcred(PamFlag::DELETE_CRED)?;
pam.end()?;
```

`login` and sshd-session do the same: they wait for the session (the
shell, the connection), close PAM, exit. The worker of greetd also gets
`SIGTERM` if greetd itself dies (`PR_SET_PDEATHSIG`, line 288).

| Case | The state | The sd-login monitor | A pidfd on the leader |
|---|---|---|---|
| The session runs (sway, the shell, the connection) | `active` or `online` | — (nothing to see) | — |
| The session command ended, PAM closed, the leader exits | `closing` at once | fires at once | fires at once |
| PAM closed, the leader lives on | `closing` on D-Bus at once | fires after 20 s (the release timer) | does not fire while the leader lives |
| The leader dies with no `ReleaseSession` (a crash, a kill) | `closing` at once (logind sees the pidfd) | fires at once | fires at once |
| The session command ended, but the leader neither closes PAM nor exits | stays `active` or `online` | — | — |

The last case is a fault of the program of the login; logind cannot
tell it from a session in use, and so neither can the service. A pidfd of
the service on the leader would be worse in the third case: it waits for
the leader for good, where the monitor waits 20 s.

### PAM closed, the leader alive

How it comes:

- The usual gap: the program of the login closes PAM and exits a moment
  later (greetd: `pam.end()`, then return). Milliseconds.
- A program that does more work after `pam_close_session`: logs, waits,
  cleans up; or hangs in it (a module of PAM that waits, as an unmount
  of a busy home by `pam_mount`).
- A leader stopped or stuck (`SIGSTOP`, a hung NFS).

What follows, `session_stop_scope()`, `src/login/logind-session.c:963`:

- At once: the state is `closing`; the release timer of 20 s runs.
- After 20 s (or at the exit of the leader, if sooner): `session_stop()`.
  The scope of the session is abandoned first ("we are not interested
  anymore"), then:
  - with `KillUserProcesses=yes`: the scope is stopped, and every process
    left in it is killed, the leader too (it is in the scope: the scope is
    made with the pid of the leader);
  - with `no`: nothing is killed; "this session is allowed to persist in
    'closing' state indefinitely" (line 989), until its last process
    ends.
- When its last process ends: the session is removed (`SessionRemoved`).
  If it was the last session of the user, the user goes `closing`, and
  its user manager stops after `UserStopDelaySec` (10 s by default)
  unless linger is on.
- greetd shows the login screen again only when its worker returns. A
  worker stuck after PAM leaves the VT with no greeter.

### PAM not closed, the leader waits

For logind the session is alive, and it counts as one:

- The session runs (sway, a shell): as usual.
- ssh that does not close: a process left in the background still holds
  the channel (`nohup cmd &` with its output on the terminal); the
  client is gone, sshd-session waits for the output to close. The session
  stays `online` and, as ssh, is never idle: it holds the system up while
  the process lives. drowsed shows it in `HeldBy`.
- The session command ended, but the program of the login did not see
  it (a fault of that program): the session looks alive; the idle hint of
  a graphical session stays at its last value, as its idle program is
  gone; greetd shows no login screen.

Neither logind, nor the service, nor the sd-login monitor can tell such a
session from a live one. It ends by `loginctl terminate-session` or when
its processes end.
