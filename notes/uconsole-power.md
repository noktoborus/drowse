# The power supplies of the uConsole with a CM5

## Question

Which `power_supply` devices does the uConsole with a CM5 have? Do they
send uevents? What states does the battery give to UPower?

Read on 2026-11-14 in the kernel of the image (raspberrypi/linux, branch
`rpi-7.3.y`, commit `e4d5c8fdad5e5a5bffe85bb1a755feb1a65eed1b`, 7.3.1,
with the patches of ClockworkPi for the uConsole) and in the driver
`axp22x_soft_battery` 0.1.0 of the image.

## The devices

The overlay of the uConsole with a CM5, the node of the AXP228:

```dts
battery_power_supply: battery-power-supply {
	compatible = "x-powers,axp221-battery-power-supply";
	monitored-battery = <&battery>;
};
ac_power_supply: ac_power_supply {
	compatible = "x-powers,axp221-ac-power-supply";
};
usb_power_supply: usb-power {
	compatible = "x-powers,axp223-usb-power-supply";
	status = "disabled";
};
```

So two devices: `axp22x-ac` (type `Mains`) and the battery of
`axp22x_soft_battery` (type `Battery`). The USB supply is off.

## AC: axp22x-ac

`drivers/power/supply/axp20x_ac_power.c` at that commit:

```c
static const struct power_supply_desc axp22x_ac_power_desc = {
	.name = "axp22x-ac",
	.type = POWER_SUPPLY_TYPE_MAINS,
```

```c
static const char * const axp20x_irq_names[] = {
	"ACIN_PLUGIN",
	"ACIN_REMOVAL",
};
```

The handler of these IRQs, `axp20x_ac_power_irq()`, calls
`power_supply_changed()` (line 55): a uevent on each plug and unplug.
`online` is the bit `ACIN_AVAIL` of the power status (line 94). The
patches of the uConsole add two writes to `AXP20X_VBUS_IPSOUT_MGMT` in
that handler, before `power_supply_changed()`.

## The battery: axp22x_soft_battery

It takes the cell of the battery in place of `axp20x_battery`, which is
blacklisted and must not be built in:

```c
#if IS_BUILTIN(CONFIG_BATTERY_AXP20X)
#error "CONFIG_BATTERY_AXP20X=y: axp20x_battery must be a module (m) or off (n)"
#endif
```

```
blacklist axp20x_battery
```

Its type and scope:

```c
const struct power_supply_desc sb_desc = {
	.name = "axp22x-soft-battery",
	.type = POWER_SUPPLY_TYPE_BATTERY,
```

```c
case POWER_SUPPLY_PROP_SCOPE:
	val->intval = POWER_SUPPLY_SCOPE_SYSTEM;
```

So UPower takes it as a battery that powers the system.

Its status:

```c
if (!supplied) {
	*status = POWER_SUPPLY_STATUS_DISCHARGING;
	return 0;
}
```

then `Charging`, `Full` or `Not charging`. With no valid sample, or while
the AXP does not answer, the status is not given:

```c
if (!sb->snap.valid || sb->snap.silent)
	return -ENODATA;
```

UPower then has the state `unknown`. The driver sends a uevent
(`power_supply_changed()`) from its periodic work when the status, the
level or the supply changes, and when the AXP stops answering.

## So, for UPower

- Charger in: `axp22x-ac` is online, so `OnBattery` is false at once (by
  the uevent of the IRQ).
- Charger out: `axp22x-ac` goes offline at once; the battery says
  `Discharging` at its next work. UPower polls the battery every 1 s for
  5 s after a change of a line power (notes/power-source.md), so
  `OnBattery` becomes true within some seconds.
- The battery with no data (`unknown`) and no AC: UPower takes it as
  discharging, so `OnBattery` is true.
