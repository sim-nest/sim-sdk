# sim

In one line: the single starting point a developer adds to reach every part of the SIM runtime.

## What it gives you

This is the one-stop crate you depend on to work with SIM. Instead of hunting down and wiring together the kernel, the codecs, the number domains, the list and table backends, and the behavior libraries one by one, you add this single package and get them all through a tidy set of named modules. It ships the installer that boots a working runtime and the authoring helpers for writing functions, classes, macros, and shapes. You pick how much of the system you want through named feature groups, starting from a small default and widening as your needs grow. It is the front door: the place newcomers begin and the surface everything else hangs behind.

## Why you will be glad

- You start from one dependency instead of assembling a dozen pieces by hand.
- A handful of lines boots a real runtime you can evaluate against.
- You turn features on as you need them, keeping small projects small.
- Stable module names shield your code from how the pieces are split up underneath.

## Where it fits

This crate is the umbrella and entry point of the SIM constellation. The real implementation lives in sibling packages for the kernel, syntax, codecs, and libraries; this crate gathers them under one roof, re-exports them by steady names, and hands you the core runtime installer. When someone new asks where to begin with SIM, the answer is here. Everything else in the system is reached through the doorway this crate opens.
