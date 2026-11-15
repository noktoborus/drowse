# hypridle, Hyprland and niri: the idle hint and the inhibitors

## Question

Can hypridle be the part in the session (the idle hint for logind)?
Which inhibitors does it obey? When does a Wayland idle inhibitor hold in
Hyprland and in niri: with the screen locked, with the output off?

Read on 2026-11-14 in the last releases:

- hypridle v0.1.8 (github.com/hyprwm/hypridle, 2026-07-26)
- Hyprland v0.57.2 (github.com/hyprwm/Hyprland, 2026-11-09)
- niri v26.04 (github.com/YaLTeR/niri, 2026-04-25)

None of them is in the image of the uConsole. Whether Gentoo has them was
not checked.

## hypridle

### No idle hint

No `SetIdleHint` and no `IdleHint` in `src/`. hypridle never tells logind
that the session is idle. With hypridle alone, a graphical session is
never idle for logind.

It can still be done by a listener of hypridle:

```
on-timeout = busctl call org.freedesktop.login1 /org/freedesktop/login1/session/auto org.freedesktop.login1.Session SetIdleHint b true
on-resume  = busctl call org.freedesktop.login1 /org/freedesktop/login1/session/auto org.freedesktop.login1.Session SetIdleHint b false
```

logind lets the owner of the session do it (systemd 261.2,
`src/login/logind-session-dbus.c:290`):

```c
if (uid != 0 && uid != s->user->user_record->uid)
        return sd_bus_error_set(error, SD_BUS_ERROR_ACCESS_DENIED, "Only owner of session may set idle hint");
```

Unlike swayidle `idlehint`, nothing sets the hint back to false at the
start, at unlock or after a sleep.

### The inhibitors it obeys

`src/core/Hypridle.cpp`. Each kind can be turned off in `general`:

- **Wayland** (`ignore_wayland_inhibit`): the notifications that obey
  inhibitors, unless turned off (line 74):

  ```cpp
  if (*IGNOREWAYLANDINHIBIT || r.ignoreInhibit)
      ... sendGetInputIdleNotification(...)
  else
      ... sendGetIdleNotification(...)
  ```

- **logind** (`ignore_systemd_inhibit`): reads `BlockInhibited` at the
  start (line 621) and on each `PropertiesChanged` (line 537):

  ```cpp
  auto inhibits_ = ":" + inhibits + ":";
  if (inhibits_.contains(":idle:")) {
      ...
      g_pHypridle->onInhibit(true);
  ```

  Only `BlockInhibited`, so only `block`, not `block-weak`. Only `idle`.
  An inhibitor of logind with `idle` stops every listener of hypridle,
  as with swayidle; one of `sleep` or `shutdown` does not.
- **D-Bus ScreenSaver** (`ignore_dbus_inhibit`): hypridle takes the name
  `org.freedesktop.ScreenSaver` on the session bus and serves `Inhibit`
  and `UnInhibit` (from line 626).
- A listener with `ignore_inhibit` obeys none (line 301):

  ```cpp
  if (g_pHypridle->m_iInhibitLocks > 0 && !pListener->ignoreInhibit) {
  ```

### Sleep

`inhibit_sleep` (default 2): hypridle holds a `delay` inhibitor of
`sleep` so the lock comes before the sleep (lines 109-143).

## Hyprland: when a Wayland inhibitor holds

`src/managers/input/IdleInhibitor.cpp`, `recheckIdleInhibitorStatus()`:

```cpp
if (ii->nonDesktop) {
    PROTO::idle->setInhibit(true);
    return;
}
...
if (VIEW->mapped() && VIEW->acceptsInput() && (!ALPHAMODIFIER || ALPHAMODIFIER->alphaNonZero())) {
    PROTO::idle->setInhibit(true);
```

- An inhibitor holds while its surface is mapped, takes input and is not
  fully clear. A surface with no view of Hyprland ("non-desktop") is
  taken as shown.
- The lock of the session and the output off are not looked at here. So
  in Hyprland an inhibitor (of waybar, of a video) keeps holding with the
  screen locked or off. sway does the other way
  (notes/sway-inhibitors.md).
- Window rules add inhibitors by hand: `idleinhibit always`, `focus`,
  `fullscreen` (`isWindowInhibiting()`).
- Hyprland itself does not serve `org.freedesktop.ScreenSaver`;
  hypridle does.

## niri: when a Wayland inhibitor holds

`src/niri.rs:3993`, `refresh_idle_inhibit()`:

```rust
let is_inhibited = self.is_fdo_idle_inhibited.load(Ordering::SeqCst)
    || self.idle_inhibiting_surfaces.iter().any(|surface| {
        with_states(surface, |states| {
            surface_primary_scanout_output(surface, states).is_some()
        })
    });
self.idle_notifier_state.set_is_inhibited(is_inhibited);
```

- A Wayland inhibitor holds while its surface is shown on an output (it
  has a primary scanout output). A surface behind the lock screen or on
  an output turned off is not shown, so the inhibitor does not hold then,
  as in sway. This is a conclusion from the name and use of
  `surface_primary_scanout_output`; the code of smithay behind it was not
  read.
- niri itself serves `org.freedesktop.ScreenSaver`
  (`src/dbus/freedesktop_screensaver.rs`). An inhibit by D-Bus (a
  browser, a player) holds always, shown or not.
- niri does not read the inhibitors of logind and does not set the idle
  hint (no `IdleHint`, no `BlockInhibited` in `src/`).

## So

| | Idle hint for logind | Wayland inhibitor with the screen locked or off | Inhibitor of logind `idle` | D-Bus ScreenSaver |
|---|---|---|---|---|
| sway + swayidle | `idlehint` | lost | swayidle stops (`block` only) | not served |
| Hyprland + hypridle | only by `busctl` in a listener | holds | hypridle stops (`block` only) | served by hypridle |
| niri + swayidle | `idlehint` | lost (from the code, not run) | swayidle stops (`block` only) | served by niri, holds always |

- For the service, any of them works once the session sets its idle
  hint. With hypridle it takes the two `busctl` lines, and the hint is not
  set back to false after a sleep or an unlock by itself.
- The inhibitor of system of the tool (`block-weak`, `sleep:shutdown`) is
  seen by none of the idle programs: the screen goes off and the lock
  comes, in all three. Only the service keeps to it.
