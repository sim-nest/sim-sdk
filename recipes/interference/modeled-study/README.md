# Explore A Modeled Interference Study

This facade recipe keeps the physical problem and `interference/solve`
expression independent of placement. The modeled compute site produces a
certified Study, the interference surface projects it without re-solving, and
a model edit produces a new solve operation. Swapping to another Tensor site
changes only the `realize` placement.
