(private-observation/enter
  (opt-in '(declared-product dose-event meal-note))
  (declared-product "Mia's entered label" "package transcription")
  (dose-event "event:morning" "Mia's entered label"
    "2026-08-25T07:30:00Z" (exact-quantity 1 2 "tablet"))
  (meal-note "meal:breakfast" "2026-08-25T08:00:00Z" "Mia's own note"))

(private-observation/preview-export
  '(declared-product dose-event)
  '(event:morning)
  'locally-encrypted)
