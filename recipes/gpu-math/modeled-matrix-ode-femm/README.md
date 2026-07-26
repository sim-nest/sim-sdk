# Compose Modeled GPU Math

This recipe selects the SDK `gpu-math` feature set, places Tensor execution at
the modeled compute site, and keeps the matrix, ODE, and FEMM solver body
provider-neutral. Swapping to `site/compute/auto`, wgpu, CUDA, or ROCm changes
only placement metadata.
