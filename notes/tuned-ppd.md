# tuned-ppd and a profile set by a stage

## Question

A section sets a profile of TuneD for the idle state and gives it back at
its `leave`. How does tuned-ppd take a profile set from outside, and how
is the profile the user chose given back? Which profiles touch the power
save of Wi-Fi? Read on 2026-11-16 in tuned, branch main, commit
`94f85fe` of 2026-11-15 (github.com/redhat-performance/tuned).

## tuned-ppd

- tuned-ppd serves the API of power-profiles-daemon on the bus. A
  profile of PPD (`power-saver`, `balanced`, `performance`) maps to a
  profile of TuneD by `/etc/tuned/ppd.conf`, with a section `[battery]`
  for the profiles on battery (`balanced=balanced-battery`)
  (`tuned/ppd/ppd.conf`).
- It keeps a base profile, the one the user chose, in
  `/etc/tuned/ppd_base_profile` (`PPD_BASE_PROFILE_FILE`,
  `tuned/consts.py:109`).

### A profile set from outside

`_tuned_profile_changed()`, `tuned/ppd/controller.py:248`:

- A profile of TuneD that maps to one of PPD (`tuned-adm profile
  powersave`): the holds are cleared, and the profile becomes the base
  profile, written to the file. The choice of the user is lost.
- A profile of TuneD that maps to none: `ActiveProfile` is `unknown`,
  the holds are cleared, the base profile stays, in memory and in the
  file.
- On a change of the power source, tuned-ppd sets its active profile
  again (`_upower_changed()`, `:240`); with `unknown` the lookup of the
  map fails.

### The profile given back

- At its start tuned-ppd reads the base profile from its file and sets
  it, with the map of the power source now (`initialize()`, `:379`). So
  `systemctl try-restart tuned-ppd.service` gives back the profile the
  user chose, after a profile that maps to none.
- With tuned and no tuned-ppd, nothing gives the profile back: TuneD
  keeps no profile before; `tuned-adm auto_profile` sets the one it
  recommends.

### Holds

- `HoldProfile(profile, reason, app_id)` gives a cookie; only
  `power-saver` and `performance` can be held (`HoldProfile()`, `:445`).
  A hold of `power-saver` wins over one of `performance`
  (`_effective_hold_profile()`, `:149`).
- `ReleaseProfile(cookie)`, or the caller leaving the bus
  (`watch_name_owner`, `:175`), releases it; with no hold left the base
  profile comes back (`remove()`, `:195`). A hold does not change the
  base profile.
- A profile set by the user clears every hold
  (`set_active_profile()`, `:474`).
- polkit: `hold-profile` and `switch-profile` give `allow_active=yes`,
  `allow_any=no` (`tuned/ppd/tuned-ppd.policy`).

## The power save of Wi-Fi

- Of the profiles tuned-ppd maps, only `powersave` has a script:
  `enable_wifi_powersave` at its start, `disable_wifi_powersave` at its
  stop (`profiles/powersave/script.sh`). `balanced`,
  `balanced-battery`, `throughput-performance` have no `[script]` and no
  `[net]` of Wi-Fi.
- `disable_wifi_powersave` sets the level 0: `iw dev … set power_save
  off`, not the value before (`_wifi_set_power_level()`,
  `profiles/functions`). With no `iw` it does nothing: it falls back to
  `iwpriv` and to `/sys/bus/pci/devices/*/power_level` only.
- So once `iw` is installed, a section with a profile that includes
  `powersave` turns the power save of Wi-Fi off at its `leave`, until a
  reboot (the kernel keeps the value: the note `wifi-cm5.md` of the
  research of the uConsole kernel).
- `USB_AUTOSUSPEND` is not set by default, so the script of `powersave`
  does not touch USB.

## For the service

- A section with a profile that maps to none of PPD, and `leave` a restart
  of tuned-ppd, gives back the profile the user chose (the example
  `examples/tuned/cm5-powersave`) by the source. On the uConsole
  (2026-11-19) the profile was not given back at a stop of drowsed.
  So the package gives `drowsed-tuned`: it keeps the profile before
  itself and gives it back with `tuned-adm`, with no restart of
  tuned-ppd. The restart stays in README.md as another way.
- The script of `powersave`, included by such a profile, runs too; the
  branch `plans` holds the question of the power save of Wi-Fi.
