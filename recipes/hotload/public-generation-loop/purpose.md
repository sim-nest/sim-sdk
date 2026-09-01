# Public hot-generation loop

This checked specimen uses only `sim::hotload` lifecycle records. It builds two
sealed source generations, admits and atomically activates them, retains a
callable from generation A across activation of B, exercises closed refusal
legs, releases A, and replays the completed journal into a fresh image.

The tiny `generation-a` and `generation-b` directories are input fixtures, not
host loader helpers. Both are built through `NativeBuilder`; the deterministic
test capsule supplies the sandbox, storage, loader, and journal ports.
