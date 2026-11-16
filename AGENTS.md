# AGENTS

LANG   this file uses the notation below. keep writing it in this notation.
FORM   RULE <topic> = <value>        hard rule
       RULE <topic> : <predicate>    hard rule, predicate form
       WHY  <reason>                 why the rule exists
       THEN <action>                 what to do when the rule triggers
       EX+  <good example>
       EX-  <bad example>
       REF  <path>                   where to look
       EXC  <exception>              where a rule is broken, and on whose word
READ   RULE is binding. WHY is context, not negotiable. EX is literal.

## Project

RULE product = drowse: `drowsed`, a system service of systemd that does
     the actions of its stages (an action of power, a command) while every
     session is idle; `drowse-inhibit` and `drowse-status`, the tools of
     the user
RULE names : no prefix `systemd-` in the names of the programs, the units,
     the files
WHY  they are not a part of systemd
RULE names.service : what runs for the service has the prefix `drowsed`;
     a tool of the user has the prefix `drowse-`
EX+  drowsed-stage@.service, drowsed-resume.service, `drowsed stage`
EX+  drowse-inhibit, drowse-status
RULE requirements = REQUIREMENTS.md, what the service does
THEN change it with the service, in the same commit
RULE device = the first device is the ClockworkPi uConsole with a CM5
RULE lang = rust, edition 2024: the service and the tools of the user

RULE settings : the service has system settings only; a user has none
WHY  with nobody logged in, or with two users, there is no one user to
     take the settings from
RULE idle.source = the sessions of logind and its inhibitors of `idle`,
     `sleep` and `shutdown`, whatever the action is; a session in the
     state `closing` is left out; a session of a class not in `Classes=`
     is left out; the service counts no activity of its own
WHY  logind counts every session alike: sway, a console, ssh; but it also
     counts a `closing` session, whose idle state nobody updates
REF  notes/logind-idle.md, notes/session-end.md
RULE state.bus : the service publishes its state on the system bus; a
     program that shows it reads it there, and does not work it out again
WHY  one rule, in one place
RULE control : the service takes no commands of its own; the bus is for
     reading only; it is driven only by the mechanisms of the system:
     inhibitors of logind, activity in sessions, its settings file,
     systemctl
WHY  those check the rights themselves; a way of its own would need its
     own rights
THEN anyone reads the state; only the group `wheel` reads the details,
     what holds the system up: one rule of the policy of D-Bus
EX-  a method `Postpone()` or `Cancel()` on the bus

RULE scope : the service decides, announces and acts; it shows nothing
THEN warnings, notifications, sound, the screen on at a warning and the
     state of the inhibitors are the interface, not the service
RULE battery.low : a low or critical charge is for UPower, not for the
     service
RULE session.part : a graphical session gives only `SetIdleHint` on idle
     and on activity; the screen and the lock are set by the user in the
     idle program of the session
EX+  swayidle with `idlehint 1`, its other timeouts as the user likes

## Code

REF  Cargo.toml                      edition, lints (`unsafe_code`,
                                     `missing_docs`), fat LTO
RULE file.max_lines = 1000
THEN split into modules
RULE module.subject : one module is one subject
EX-  util.rs, helpers.rs, misc.rs
RULE comment.place : a comment documents an item, by `///` and `//!`;
     none in the body of a function
THEN a line that needs an explanation gets a name: a value or a function
RULE comment.lang = simplified english (`text.lang`)
RULE api.inspect : the API of a crate is looked up, not guessed
THEN cargo brief, ruskel, docs.rs

## Unsafe

RULE ffi : as little `unsafe` and FFI as can be; what a crate does, a crate
     does; never an `extern "C"` block of its own
RULE login.unsafe : the calls of libsystemd-sys are in `src/login.rs`
     only, behind a safe type; it alone has `#![allow(unsafe_code)]`
RULE unix.calls = `rustix`; never `libc` itself, never `nix`
THEN a call `rustix` lacks is taken from std or tokio
EX+  rustix::process::kill_process_group(pgid, Signal::TERM)
EX+  `tokio::signal` for SIGTERM
EX+  rustix::process::setsid() for the holder of `drowse-inhibit on`
EX-  nix::unistd::fork(), unsafe { libc::kill(pid, libc::SIGTERM) }

## Errors

RULE error.enum : a crate has one error enum in error.rs and a
     `Result<T>` alias
RULE error.shape : the enum is closed, one variant per failure, with what
     is needed to act on it
RULE error.source : an error of a lower layer is kept in a variant of its
     own; `source()` of `std::error::Error` returns it
RULE error.impl : `Display` and `std::error::Error` are written by hand;
     no crate for them
WHY  the errors are few
EX-  Err(format!("{error}"))
RULE error.message : an error message is english and for the journal

## Dependencies

RULE dep.search : before code for a general problem, look for a crate
     that solves it
RULE dep.version = the latest release on crates.io
EX-  a git or pre-release dependency

## Checks

RULE checks : all are clean before the work is reported done
THEN cargo fmt --all
     cargo clippy --all-targets
     cargo build
     cargo test
     cargo test -- --ignored        with dbus-daemon: the programs on a
                                    bus of their own, with mocks
     ! cargo tree -e normal,build --depth 1 --prefix none \
         | grep -E '^(libc|nix) v'
RULE format.tool = rustfmt with its defaults, no `rustfmt.toml`

## Commits

RULE commit : one commit per subject; the subject line says what the work
     does, the body what it does it for
RULE commit.clean : a message carries nothing of the machine it was
     written on
EX-  a session identifier, a password, a token, the name of a local user,
     a path under a home directory, the address of a machine

## Layout

RULE layout.bins = src/bin/drowsed.rs, src/bin/drowse-inhibit.rs,
     src/bin/drowse-status.rs: one package, three programs
RULE layout.lib : the code is a library, src/lib.rs; a program only reads
     its arguments and runs the library
WHY  the tests run the service as a library, with mocks of the world
     outside
RULE layout.docs = README.md, for the one who sets up the system: what
     the system needs, and why
RULE layout.notes = notes/<question>.md, one file per question; a fact is
     cited with its source
RULE layout.system : the files of the system are in a directory by
     where they go: `dbus/` the policy of D-Bus, `systemd/` the units,
     `preset/` the preset, `etc/` the settings file, `waybar/` the
     module of waybar; each file named as it is installed
RULE settings.file : etc/drowse.conf is every key with its default, in
     comments; a test checks it gives the defaults
THEN a change of a default changes etc/drowse.conf in the same commit
EX+  dbus/io.github.noktoborus.Drowse.conf, systemd/drowsed.service,
     preset/50-drowse.preset
RULE license = MIT (LICENSE); every crate it takes allows it
RULE layout.plans : the work put off is on the branch `plans`, a history
     of its own; one file a subject; main carries no plan
RULE plans.questions : an open question in `plans` has a number `Q<n>`,
     one for the whole project; a number is never given again
THEN the next number is one more than the greatest in `plans`
THEN a question answered leaves `plans`; its number is not reused

## Text

RULE text.lang = simplified english: short sentences, common words, one
     idea per sentence
