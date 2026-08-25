(physics-study
  (load lib/physics)
  (boundary (port source voltage current :orientation into-system)
            (event switch-open :at 0.5))
  (solve :method injected :split-at switch-open)
  (audit :energy stored :work signed :uncertainty independent)
  (verdict :certified-or unresolved)
  (finding (append immutable) (browse projection)))
