# Requirements

The service does an action of power when the system is idle for a time.
It takes the idle state from logind, the power source from the kernel and
the inhibitors from logind and from the compositor.

## Terms

- **Idle**: logind says that every session is idle and that no block
  inhibitor of `idle` is held. Sessions of all kinds count alike: a
  graphical one, a console, ssh.
- **Inhibitor of screen and system**: a Wayland idle inhibitor, set by
  hand or by a program (a video player, a browser). The session does not
  become idle: the screen stays on, and no action comes.
- **Inhibitor of system**: a block inhibitor of logind (`idle`, `sleep`,
  `shutdown`), taken by hand or by a program. The screen goes off; no
  action comes.

## Cases

| Case | The screen goes off | The action |
|---|---|---|
| Work in any session (sway, console, ssh) | no; yes when the work is by ssh only | no |
| All sessions idle, on battery | yes, after `screen_battery` | yes, after `idle_battery` |
| All sessions idle, on mains | yes, after `screen_mains` | after `idle_mains`, or never |
| A connection with no terminal is open (sftp, a tunnel) | yes | no |
| Inhibitor of screen and system | no | no |
| Inhibitor of system | yes | no |
| The charger is taken out while idle | yes | by `after_change` |
| The inhibitor of system is taken off while idle | yes | by `after_change` |
| An external monitor | as the built in screen | as usual |
| The charge is low or critical | by UPower | by UPower |

A connection with no terminal is never idle for logind
(notes/logind-idle.md). So it keeps the system up while it is open.

## The action

1. The time of a timeout has come. The service picks the action by the
   chain.
2. The service announces: "action X in `warning` seconds".
3. Any activity in any session before that time cancels the action. The
   service announces the cancel.
4. Else the service does the action through logind.

### The chain

- The action is the first one of the chain that logind can do.
- The default chain: `suspend`, `hibernate`, `poweroff`.
- The service asks logind (`CanSuspend`, `CanHibernate`, `CanPowerOff`)
  at the time of the action, not at its start.
- When no action of the chain can be done, the service does nothing and
  writes an error to the journal.
- An action is a name of logind: `poweroff`, `halt`, `reboot`,
  `soft-reboot`, `kexec`, `suspend`, `hibernate`, `hybrid-sleep`,
  `suspend-then-hibernate`, `sleep`, `lock`, `ignore`.

## Settings

System settings only. A user has no settings of their own.

| Setting | Meaning |
|---|---|
| `screen_battery` | idle time before the screen goes off, on battery |
| `screen_mains` | idle time before the screen goes off, on mains |
| `idle_battery` | idle time before the action, on battery |
| `idle_mains` | idle time before the action, on mains; or `never` |
| `warning` | time from the announce to the action |
| `after_change` | `now`: the action comes at once when the time has passed; `restart`: the time is counted again from the change |
| `chain` | the list of actions, in order |

## Not the service

- Low and critical charge: UPower (`CriticalPowerAction=`).
- The interface: warnings, notifications, sound, the screen on at a
  warning, the state of the inhibitors, the lifetime of the inhibitors set
  by hand. See the branch `plans`, `interface.md`.
