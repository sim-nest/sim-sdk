; Pure projection over explicitly opted-in, human-entered records. Missing data
; remains missing; competing edits remain adjacent until Mia reviews them.
(define (observation/timeline records opted-fields reviewed-conversions)
  (timeline
    (stable-sort entered-at/event-id
      (filter (lambda (record)
                (and (human-entered? record)
                     (field-opted-in? opted-fields (record/field record))))
              records))
    (missing-fields records opted-fields)
    (concurrent-changes records)))

(define (observation/exact-totals records opted-fields reviewed-conversions)
  (group-and-sum-exact-by-unit
    (filter (lambda (event)
              (and (dose-event? event)
                   (human-entered? event)
                   (field-opted-in? opted-fields 'dose-event)))
            records)
    reviewed-conversions))
