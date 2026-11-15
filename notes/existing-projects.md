# Existing projects

## Question

Is there a project that does what REQUIREMENTS.md asks? Looked on
2026-11-14: web search, the READMEs of stasis and pwr-tray, the docs of
autosuspend. The docs site of stasis and the page of pwr-tray on PyPI
could not be read; their READMEs on GitHub were read.

## What the requirements need

1. A system service: it works with ssh only and with nobody logged in.
2. The idle state of logind: every session, ssh too.
3. Other times of the action on battery and on AC.
4. A Wayland inhibitor stops all; an inhibitor of logind of
   `sleep:shutdown` stops the action only.
5. A change while idle (the charger out, an inhibitor off): `now` or
   `restart`.
6. An announce before the action; activity cancels it.
7. A chain of actions: the first one logind can do.
8. System settings only.

## The projects

| Project | Kind | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|
| logind `IdleAction` | system | yes | yes | no | logind only | — | no | no | yes |
| swayidle | Wayland client of a user | no | no; sets the hint of its session (`idlehint`) | no | Wayland, logind `idle` (`block` only) stop all | no | no | no | no |
| hypridle | Wayland client of a user | no | no hint | no | Wayland, logind `idle` (`block` only), D-Bus ScreenSaver stop all | no | no | no | no |
| stasis | Wayland client of a user | no | no | yes | partly: logind `idle` stops the suspend only | `restart` only | yes | no | no |
| pwr-tray | tray of a user, drives swayidle | no | no | yes | ? | ? | no | no | no |
| autosuspend | system | yes | partly | by a command | partly | ? | partly | no | yes |
| cosmic-idle | Wayland client of COSMIC | no | no | yes | ? | ? | ? | no | no |
| GNOME, KDE, Xfce power managers | part of their desktop | no | no | yes | ? | ? | yes | no | no |
| ChromeOS powerd | system, ChromeOS only | yes | own | yes | own | ? | yes | no | yes |

"?" is not checked. The power managers of the desktops were not looked
into further: each works only in its own desktop.

The rows of swayidle and hypridle: notes/sway-inhibitors.md,
notes/hypridle-hyprland-niri.md.

## stasis

- github.com/saltnpepper97/stasis, Rust, GPL-3.0. A Wayland idle manager
  of a user.
- Plans of steps with timeouts; `ac:` and `battery:` blocks in a plan
  (examples/lid-grace-period.rune).
- Read in the source of v1.7.0 (2026, tag `v1.7.0`):
  - The power source: it reads `/sys/class/power_supply/*` every 5 s, a
    supply of type `Mains` with `online` 1 (`src/core/utils.rs:39`,
    `src/services/power.rs`). No UPower.
  - On a change of the power source it starts the idle cycle again
    (`state.reset_idle_cycle(now_ms)`, `src/core/manager/engine.rs:386`):
    a new count from the change.
  - A notification before a step: `notify_before_action` and, per step,
    `notification` and `notify_seconds_before`
    (`src/core/manager/engine.rs:989`).
- Honors Wayland inhibitors; D-Bus `org.freedesktop.ScreenSaver`,
  `org.gnome.SessionManager`, `org.freedesktop.portal.Inhibit`.
- Polls `ListInhibitors` of login1 for block inhibitors of `idle` and maps
  them to "suspend only": lock and screen off go on, the suspend waits.
  This is requirement 4 as it is.
- Media and audio by PipeWire (`pw-dump`) or `pactl`; games; gamepads.
- Inhibitors by hand from the CLI and a JSON for waybar.
- Notifications by libnotify. A warning before a step: not checked.
- Not a system service: it does not see ssh, and does not run at the login
  screen.

## pwr-tray

- github.com/joedefen/pwr-tray, Python, PyQt5. A tray applet of a user; on
  sway it starts and drives swayidle.
- Modes; lock and sleep timeouts; settings for `HiBattery` and
  `LoBattery` (`lo_battery_pct`), else inherited.
- Sees systemd inhibitors (an icon) and playing media by `playerctl`.
- `power_down = True`: power off in place of suspend.
- An interface more than a service.

## autosuspend

- github.com/languitar/autosuspend, Python. A system service: suspends
  when no check finds activity for `idle_time`.
- `suspend_cmd` is any command, so `systemctl poweroff` works.
- `LogindSessionsIdle`: activity while a logind session of the given
  types (default tty, x11, wayland) has `IdleHint=no`. It looks at
  sessions, not at the `IdleHint` of the Manager, so it misses inhibitors
  of `idle` and sessions with no type.
- `ExternalCommand`: activity while a command exits 0. With
  `systemd-ac-power` it keeps a system on AC up. One `idle_time`, so
  no other time on AC.
- `notify_cmd_wakeup`, `notify_cmd_no_wakeup`: a command before the
  suspend. No time of warning that activity cancels: not checked.
- Wakeups: it can set an RTC wake. Not needed here.
- In Gentoo: not checked.

## cosmic-idle

- The idle daemon of COSMIC. Screen off and suspend; suspend times for AC
  and battery apart (a pull request of tiiuae/ghaf, 2026-01). Set from
  COSMIC Settings. Bound to COSMIC; not checked on sway.

## The power managers of desktops

- gnome-settings-daemon: `sleep-inactive-ac-timeout`,
  `sleep-inactive-battery-timeout`, and the type of each.
- KDE PowerDevil: profiles for AC, battery, low battery.
- xfce4-power-manager: times for AC and battery; on Wayland only in part.
- Each works only in its own desktop.

## ChromeOS powerd

- chromium.googlesource.com, platform2/power_manager. The nearest to the
  requirements: delays plugged and unplugged, screen dim and off, the idle
  action, a warning, audio as activity, policy from the browser.
- ChromeOS only: no logind, its own policy from Chrome.

## Result

No project does all of it. The nearest:

- **autosuspend**: the shape is right (a system service, checks,
  any command). Missing: other times on AC, the screen, the
  inhibitors of logind as the Manager sees them, a warning that activity
  cancels, the chain.
- **stasis**: the policy is right (AC and battery, inhibitors of logind
  for the suspend only). Missing: it is a client of a user, so no ssh and
  nothing with nobody logged in; no chain.

## Sources

- https://github.com/saltnpepper97/stasis (README, examples/lid-grace-period.rune)
- https://github.com/joedefen/pwr-tray (README)
- https://autosuspend.readthedocs.io/en/latest/available_checks.html
- https://autosuspend.readthedocs.io/en/3.0/configuration_file.html
- https://github.com/tiiuae/ghaf/pull/1693 (cosmic-idle)
- https://chromium.googlesource.com/chromiumos/platform2/+/master/power_manager/docs/inactivity_delays.md
- https://fostips.com/automatic-suspend-idle-time-less-15-min-ubuntu-fedora-debian/ (GNOME keys)
- https://aur.archlinux.org/packages/wayland-power-manager-git (a GTK front of swayidle and logind for labwc; upstream not known)
