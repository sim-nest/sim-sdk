(interference-study
  (solve
    (realize
      (interference/solve problem plane
        {sampling annotate work-budget default})
      :fabric site/compute/model
      :result interference/Study))
  (view
    (project-edit observable phase)
    (model-edit frequency-hz 686.0))
  (site-swap
    (from site/compute/model)
    (to core/local-fabric)
    (solve-expression unchanged)))
