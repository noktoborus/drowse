# swayidle as the part in the session

## Question

Can swayidle be the part in the session: the idle hint for logind and the
screen off? Is `idlehint` on by default? Read in the source of swayidle
1.9.0 (tag `v1.9.0`), the version of the uConsole image (built with
`USE="systemd"`), on 2026-11-14.

## idlehint

- Off by default. `state.logind_idlehint` is set only by the command
  `idlehint <seconds>` (`parse_idlehint()`, `main.c:807`). With no such
  command swayidle never calls `SetIdleHint`.
- Only one `idlehint` is allowed (`main.c:813`).
- Built with neither systemd nor elogind, `idlehint` stops swayidle with an
  error (`main.c:808`).
- With `idlehint`: `SetIdleHint(false)` at the start (`main.c:1141`);
  `SetIdleHint(true)` on `idled` of its timeout (`handle_idled()`,
  `main.c:639`); `SetIdleHint(false)` on `resumed` (`main.c:656`), and on
  unlock and after a sleep (`main.c:298`, `main.c:329`, `main.c:628`).
- An `idlehint` event runs no command: the timeout is used for the hint
  only (`main.c:639`).
- The man page: "If built with systemd support, set IdleHint to indicate
  an idle logind/elogind session after <timeout> seconds"
  (`swayidle.1.scd:67`).

## The idle state

- swayidle 1.9.0 uses `ext-idle-notify-v1`, the kind that obeys
  inhibitors. The compositor sends `idled` after the timeout with no
  input, and `resumed` on input.
- A Wayland idle inhibitor holds back `idled` only while its surface is
  shown: in sway not with the screen locked or the output off
  (notes/sway-inhibitors.md).
- swayidle reads `BlockInhibited` of logind on each change. With `idle`
  in it, it stops all its timeouts and sets the hint to "active". It does
  not read `BlockWeakInhibited`, and does not see an inhibitor taken
  before it starts (notes/sway-inhibitors.md).

## Idle now, and the other way of a hint

Read on 2026-11-16 in swayidle 1.9.0.

- `SIGUSR1`: idle at once. Every timeout is registered again with the
  time 0, not obeying inhibitors (`handle_signal()`, `main.c:905`); with
  `idlehint`, the hint is true at once. Input sends `resumed` again.
- A hint set by commands instead of `idlehint`
  (`timeout N 'busctl … SetIdleHint b true' resume 'busctl … b false'`)
  works on idle and on input (`handle_idled()`, `handle_resumed()`,
  `main.c:634-663`), but loses what `idlehint` does: false at the start
  (`main.c:1142`), at an unlock (`main.c:299`), after a sleep
  (`main.c:330`), and when an inhibitor of `idle` turns the timeouts off:
  `disable_timeouts()` (`main.c:617`) drops the timers with no `resume`
  command, so such a hint stays true.
- `before-sleep` or `after-resume` makes swayidle hold an inhibitor of
  `sleep`, mode `delay` (`main.c:488`, `main.c:294`), "Swayidle is
  preventing sleep": it runs the command (`swaylock` of the image) before
  the sleep (`main.c:304-309`). A `delay` inhibitor does not hold the
  idle state of the service.

## The screen

- `timeout <s> <command> resume <command>`: one time per timeout, no power
  source. Two timeouts with a check of `systemd-ac-power` give one time
  on battery and one on AC. A change of the power source while idle is
  then late: up to the larger time.

## On the uConsole

- The configuration of sway of the image runs swayidle with no
  `idlehint`:

  ```
  exec swayidle -w \
      timeout 300 'swaylock -f -c 000000' \
      timeout 600 'swaymsg "output * power off"' resume 'swaymsg "output * power on"' \
      before-sleep 'swaylock -f -c 000000'
  ```

  So the session of sway is never idle for logind.
- sway, wlroots and libseat never call `SetIdleHint`
  (notes/session-types.md). The session is `wayland`, so logind takes its
  idle state from `SetIdleHint` only.

## Sources

- https://github.com/swaywm/swayidle/blob/v1.9.0/main.c
- https://github.com/swaywm/swayidle/blob/v1.9.0/swayidle.1.scd
