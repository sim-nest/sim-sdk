; One normal observation covers the flock. Missing capture remains unknown and
; is never expanded into inferred per-bird facts.
(define (capture-common-care flock care observation supplies captured-at)
  (if (missing? observation)
      (flock-state (flock/id flock) 'unknown captured-at)
      (daily-capture (common-care-check (flock/id flock) care captured-at)
                     observation supplies)))

; A deviation produces attention plus an exact factual handoff. Interpretation
; and treatment remain with the reviewing human or professional.
(define (prepare-handoff deviation current-care current-supplies)
  (factual-handoff
    (deviation/bird deviation)
    (deviation/observed-facts deviation)
    (deviation/observed-at deviation)
    current-care current-supplies
    (deviation/source-refs deviation)
    (attention 'human-review-required (deviation/bird deviation)
               'observed-deviation (deviation/observed-at deviation))))

; Kitchen receives at most one copied value claim, without flock or bird data.
(define (copy-egg-availability observation)
  (if (and (= (observation/truth observation) 'confirmed)
           (observation/fresh? observation))
      (confirmed-egg-availability
        (observation/claim observation) (observation/source observation)
        (observation/observed-at observation)
        (observation/fresh-until observation)
        (observation/quantity observation) 'confirmed)
      ()))

