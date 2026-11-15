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
