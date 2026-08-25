;; Pure data: the host resolves these already-delivered contracts.
'(stewardship-adoption
  :brief "bridge-brief/reviewed"
  :preview "composition.toml#preview"
  :transaction "adoption/atomic-with-rollback"
  :rebuild "journal+book+sealed-source"
  :subtract "composition.toml#subtraction"
  :porch "declined-success")

