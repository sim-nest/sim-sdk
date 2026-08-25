(garden/rebuild-season
  (season "spring-2026" "Spring 2026" "2026-03-01" "2026-05-31")
  (beds
    (bed "south-bed" "South bed" "south-wall")
    (bed "herb-bed" "Herb bed" "kitchen-door"))
  (crops
    (crop "pea" "Pea" "Kelvedon Wonder")
    (crop "dill" "Dill" "Bouquet"))
  (guidance
    (cited-guidance "pea-sowing" "source:local-extension-sheet"
      "SE-AB" "2026-03-01T08:00:00Z" "2026-03-15T08:00:00Z"
      "Consider sowing after checking current conditions")
    (cited-guidance "old-dill-note" "source:local-extension-sheet"
      "SE-AB" "2025-03-01T08:00:00Z" "2025-03-15T08:00:00Z"
      "Consider direct sowing"))
  (chosen-actions
    (care-action "water-pea" "south-bed" "pea" "water"
      "mia" "2026-03-10T09:00:00Z"))
  (observations
    (harvest "claim:pea-harvest" "south-bed" "pea"
      (exact-quantity 250 1 "g") 'confirmed
      "2026-05-20T10:00:00Z" "2026-05-22T10:00:00Z" "garden:observation-copy")))
