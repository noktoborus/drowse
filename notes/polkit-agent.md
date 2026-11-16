# The agent of polkit on a terminal

## Question

From ssh, polkit asks the password of an admin for an inhibitor of
`sleep` or `shutdown` (notes/logind-power-rights.md). Who asks it on a
terminal, and how does the password reach the agent? Read on 2026-11-16
in systemd 261.2 and polkit, branch main, commit `4c2422f` of
2026-11-09 (github.com/polkit-org/polkit).

## systemd-inhibit

- It starts `pkttyagent --notify-fd <fd> --fallback` when the process
  has a controlling terminal (`shall_fork_agent()`,
  `src/shared/exec-util.c:555`; `polkit_agent_open()`,
  `src/shared/polkit-agent.c:20`), not as root (`geteuid() == 0`).
- The agent takes `/dev/tty` for stdin, stdout and stderr when they are
  not a terminal (`_fork_agent()`, `src/shared/exec-util.c:572`).
- `--no-ask-password` starts no agent and also calls with no interactive
  authorization (`sd_bus_set_allow_interactive_authorization()`,
  `src/login/inhibit.c:284`): nobody is asked.
- In a process group of its own in the background of the terminal, the
  agent shows its prompt but gets no input: the terminal gives input to
  its foreground group only. This was the fault of drowse-inhibit with
  `process_group(0)`.

## pkttyagent and polkit

- With no `--process`, pkttyagent is the agent of its parent process
  (`src/programs/pkttyagent.c:186`); `--notify-fd` is closed once it is
  registered (`:258`).
- It reads the terminal of `ctermid()`, not its stdin
  (`src/polkitagent/polkitagenttextlistener.c:251`).
- polkit finds the agent of a subject first by the subject, and for a
  name on the system bus by its process too
  (`get_authentication_agent_for_subject()`,
  `src/polkitbackend/polkitbackendinteractiveauthority.c`). logind gives
  its caller as `system-bus-name` or `unix-process` with a pidfd
  (`src/shared/bus-polkit.c:92`, `:711`).
- So an agent started by the caller itself (pkttyagent with no
  `--process`) is found for the call of that caller.

## For drowse-inhibit

- The process that calls `Inhibit` starts `pkttyagent` itself, in the
  foreground group of the terminal, then calls with interactive
  authorization; root is never asked and starts no agent (`agent.rs`).
- The holder of `on` leaves the terminal by `setsid()` only after it has
  the inhibitor (`hold.rs`).
