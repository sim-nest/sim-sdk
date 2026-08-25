; Data-only authority lattice. A citation can yield a recommendation while it
; is fresh; stale evidence and absent forecasts yield questions, never work.
(define (consider-guidance guidance weather as-of)
  (cond
    ((time/after? as-of (guidance/expires-at guidance))
     (next-question 'expired-guidance (guidance/id guidance) as-of))
    ((or (missing? weather) (not (weather/fresh? weather as-of)))
     (next-question 'missing-forecast (guidance/id guidance) as-of))
    (else
     (recommendation (guidance/id guidance) (guidance/recommendation guidance)))))

; Projection follows an already chosen action. It cannot choose, command, or
; mutate a calendar, and absence of a calendar leaves the action intact.
(define (project-chosen-action action calendar)
  (if (missing? calendar)
      action
      (calendar-projection (action/id action)
                           (action/chosen-for action)
                           (calendar/reference calendar))))

; Kitchen receives only a copied value claim from a fresh confirmed harvest.
(define (copy-harvest harvest)
  (if (and (= (harvest/truth harvest) 'confirmed)
           (harvest/fresh? harvest))
      (confirmed-harvest-availability
        (harvest/claim harvest) (harvest/source harvest)
        (harvest/observed-at harvest) (harvest/fresh-until harvest)
        (harvest/quantity harvest))
      ()))
