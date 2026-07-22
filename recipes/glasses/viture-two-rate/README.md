# Reproject A Modeled Viture Pose Track

This recipe encodes one spatial Scene, coalesces a modeled Viture pose track at
device rate, and reports dropped poses. It then clamps one stale prediction and
holds the resulting frame when pose age moves beyond the clamp.
