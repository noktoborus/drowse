# The power source

## Question

How does the service tell AC from battery, and when does it learn of a
change? Read on 2026-11-14 in UPower 1.90.9 (gitlab.freedesktop.org,
tag `v1.90.9`) and systemd 261.2 (tag `v261.2`), the versions of the
uConsole image.

## The answer

The service takes `OnBattery` of `org.freedesktop.UPower` on the system
bus, and learns of a change by `PropertiesChanged`. With no UPower it
takes AC (REQUIREMENTS.md).

logind knows no power source, and systemd sends no event of it on D-Bus
(`ConditionACPower=` is checked only when a unit starts).

## How UPower works out OnBattery

`src/up-daemon.c:783`:

```c
ret = (up_daemon_get_on_battery_local (daemon) && !up_daemon_get_on_ac_local (daemon, NULL));
up_daemon_set_on_battery (daemon, ret);
```

- `OnBattery` is true when the batteries that power the system discharge
  and no line power is online.
- A battery of state `unknown` with no line power online is taken as
  discharging (`src/up-daemon.c:285`). `systemd-ac-power` takes that case
  as AC (`on_ac_power()`, `src/shared/battery-util.c:108`).
- No battery: `OnBattery` is false.

## When UPower sees a change

`src/up-constants.h`, `src/up-device-battery.c:185`:

- A line power: by its uevent only; it is not polled.
- After a change of a line power: the batteries are read at once, then
  every 1 s for 5 s.
- A battery: every 30 s, or by its uevent.

## The critical action of UPower and the inhibitors

`up_backend_take_action()`, `src/linux/up-backend.c:621`:

```c
g_dbus_proxy_call (backend->priv->logind_proxy,
                   method,
                   g_variant_new ("(b)", FALSE),
                   G_DBUS_CALL_FLAGS_NONE,
                   G_MAXINT,
                   NULL,
                   NULL,
                   NULL);
```

- The old method of logind (`PowerOff(false)`), as root, with no flags,
  and no look at the answer.
- So a `block` inhibitor of `shutdown` refuses it, and nobody sees the
  refusal; a `block-weak` one lets it through
  (notes/logind-power-rights.md).

## On the uConsole

The supplies: `axp22x-ac` (type `Mains`, a uevent on each plug and
unplug) and the battery of `axp22x_soft_battery`
(notes/uconsole-power.md). To check on the device: `upower -d` with the
charger in and out.
