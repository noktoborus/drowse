# What a rule of polkit knows of the caller

## Question

In a rule of polkit, what does `subject` hold for a process in no
session, as a unit of the user manager? Read on 2026-11-14 in polkit 126
(github.com/polkit-org/polkit, tag `126`), the version of the image.

## The fields

`src/polkitbackend/polkitbackendduktapeauthority.c`, from line 531:

```c
set_property_int32 (cx, "pid", pid_early);
set_property_str (cx, "user", user_name);
set_property_strv (cx, "groups", groups);
set_property_str (cx, "seat", seat_str);
set_property_str (cx, "session", session_str);
set_property_str (cx, "system_unit", system_unit);
...
set_property_bool (cx, "local", subject_is_local);
set_property_bool (cx, "active", subject_is_active);
```

- `session` comes from `sd_pidfd_get_session()` or `sd_pid_get_session()`
  (lines 418, 421); `seat` from that session. A process in no session
  gets no session: `session_str` stays `NULL`, and `duk_push_string()`
  of `NULL` gives `undefined` in the rule (by the API of Duktape). So
  `!subject.session` is true
  for it.
- `local` and `active` come from the session too; with none, both are
  false. So the defaults of an action give it `allow_any`.
- `system_unit`: the unit of the system manager of the process, asked
  by its pidfd (`polkit_backend_common_pidfd_to_systemd_unit()`, only
  when the pidfd came from the bus and is safe). For a process of the user
  manager it is `user@<uid>.service` (not checked on a device).
- `groups`: the groups from D-Bus, else from the user.

## So

A rule can give a right to a process of a user with no session by
`!subject.session`, and narrow it to the user manager by
`subject.system_unit == "user@" + <uid> + ".service"`. It cannot tell one
unit of the user manager from another.
