; Pure projection: only fresh confirmed claims reduce a need. Stale and unknown
; observations become questions. Units remain labels until a reviewed exact
; rational conversion rule is supplied by the input data.
(define (project-need need observation reviewed-conversion)
  (cond
    ((or (= (claim/truth observation) 'unknown)
         (= (claim/truth observation) 'stale))
     (question (need/item need) (claim/truth observation) (need/key need)))
    ((and (= (claim/truth observation) 'confirmed)
          (claim/fresh? observation)
          (or (= (quantity/unit (need/quantity need))
                 (quantity/unit (claim/quantity observation)))
              (conversion/reviewed-exact? reviewed-conversion)))
     (exact/subtract (need/quantity need) (claim/quantity observation)))
    (else (draft-line need))))

