# The Wayland inhibitors in sway, and the inhibitors of logind in swayidle

## Question

When does a Wayland idle inhibitor (waybar, a video player) hold back the
idle state of sway? Does swayidle look at the inhibitors of logind?

Read on 2026-11-14 in the versions of the uConsole image:

- sway 1.12 (github.com/swaywm/sway, tag `1.12`)
- wlroots 0.20.2 (gitlab.freedesktop.org/wlroots/wlroots, tag `0.20.2`)
- swayidle 1.9.0 (github.com/swaywm/swayidle, tag `v1.9.0`)

## swayidle takes the notifications that obey inhibitors

`register_timeout()`, swayidle `main.c:575`: every timeout and `idlehint`
is registered with `obey_inhibitors = true` (`main.c:612`, `main.c:653`),
so with `get_idle_notification`, the kind that obeys the inhibitors. Only
SIGUSR1 ("idle now") uses the kind that does not (`main.c:908`).

## wlroots: what an inhibitor does

`wlr_idle_notifier_v1_set_inhibited()`, wlroots
`types/wlr_idle_notify_v1.c:239`, and `notification_reset_timer()`,
`:84`:

- Inhibited: each notification that obeys inhibitors leaves the idle state
  (`resumed` is sent) and its timer stops. So swayidle calls
  `SetIdleHint(false)`.
- No more inhibited: the timers start again from that moment.
- While inhibited, input is not counted for these notifications (`:259`).

## sway: when an inhibitor is in force

`sway_idle_inhibit_v1_is_active()`, sway
`sway/desktop/idle_inhibit_v1.c:114`. The state is checked again on a new
or gone inhibitor, a lock or unlock (`sway/lock.c:239`, `:306`), an
`inhibit_idle` command, and each transaction of the layout
(`sway/desktop/transaction.c:770`).

- **The session is locked** (swaylock): only an inhibitor of an
  application on a surface of the lock screen counts. Every other
  inhibitor (waybar, a video, `inhibit_idle`) is ignored.
- **A layer surface** (waybar): in force while it is mapped and its output
  is enabled.
- **A window of an application**: in force while the window is visible.
- `inhibit_idle` of sway: `focus`, `fullscreen`, `open`, `visible`
  (`sway/commands/inhibit_idle.c`).

`output * power off` disables the output (`queue_output_config()`,
sway `sway/config/output.c:478`, `wlr_output_state_set_enabled(pending,
false)`).

## So: the inhibitor of screen and system is lost

The inhibitor of waybar (and of a video) does not hold when:

- the screen is locked, by hand or by the power key
  (`uconsole-screen-toggle lock`);
- the output is turned off (`swaymsg "output * power off"`, the power key
  with no lock);
- the window with the video is not visible.

Then the session goes idle after the time of `idlehint`, logind sees it,
and the action comes after its time. Example: the user has set the
inhibitor of waybar by hand and gives no input; then turns the screen off
by the power key: the system goes down after `IdleTime` and `Countdown`
of `[IdleBattery]`, though the user set the inhibitor. What the system does meanwhile does not count:
only the inhibitor set by hand stood for "keep the system up".

To keep the system up with the screen off or locked, an inhibitor of
logind is needed (the inhibitor of system).

## swayidle looks at the inhibitors of logind

swayidle `main.c:252` (`get_logind_idle_inhibit()`) and `main.c:364`:

- On `PropertiesChanged` of `BlockInhibited` of the Manager of logind,
  swayidle reads it. With `idle` in it, it turns off all its timeouts
  (`disable_timeouts()`, `main.c:617`): no lock, no screen off, and
  `SetIdleHint(false)`. With no `idle`, it turns them on again.
- It reads `BlockInhibited` only, not `BlockWeakInhibited`.
- It reads it only on a change. An inhibitor taken before swayidle starts
  is not seen until the next change (`main.c:1150` turns the timeouts on
  with no check).

So an inhibitor of logind with `idle` also keeps the screen on and the
session unlocked, when swayidle is the idle program. To keep the system up
and still let the screen go off, the inhibitor of system takes `sleep` and
`shutdown`, not `idle`:

```
systemd-inhibit --what=sleep:shutdown --why=... sleep infinity
```

## swayidle and sleep, lock

- After a sleep (`PrepareForSleep(false)`), swayidle calls
  `SetIdleHint(false)` (`main.c:298`).
- On `Unlock` of logind, `SetIdleHint(false)` (`main.c:329`).
