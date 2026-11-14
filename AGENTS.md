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

RULE product = a system service of systemd: an action of power (suspend,
     hibernate, power off) when every session is idle
RULE requirements = REQUIREMENTS.md, what the service does
THEN change it with the service, in the same commit
RULE device = the first device is the ClockworkPi uConsole with a CM5
REF  the research of the kernel

RULE settings : the service has system settings only; a user has none
WHY  with nobody logged in, or with two users, there is no one user to
     take the settings from
RULE idle.source = the idle counter of logind; the service counts no
     activity of its own
WHY  logind counts every session alike: sway, a console, ssh
REF  notes/logind-idle.md

RULE scope : the service decides, announces and acts; it shows nothing
THEN warnings, notifications, sound, the screen on at a warning and the
     state of the inhibitors are the interface, not the service
RULE battery.low : a low or critical charge is for UPower, not for the
     service

## Layout

RULE layout.notes = notes/<question>.md, one file per question; a fact is
     cited with its source
RULE layout.plans : the work put off is on the branch `plans`
     (`plans.project` of the rules of the workspace)

## Text

RULE text.lang = simplified english (`text.lang` of the rules of the workspace)
