macro_rules! cookbook_directory_compute {
    ($m:ident) => {
        $m!(
            "compute/model",
            "Modeled compute provider",
            "compute-model",
            Some(crate::compute_model::RECIPES),
            || Box::new(crate::compute_model::ComputeModelLib::new(
                crate::compute_model::ModeledComputeProfile::default(),
            ))
        );
        $m!(
            "compute/auto",
            "Automatic compute provider",
            "compute-auto",
            Some(crate::compute_auto::RECIPES),
            || Box::new(crate::compute_auto::ComputeAutoLib::default())
        );
        $m!(
            "compute/cli",
            "Compute command",
            "compute-cli",
            Some(crate::compute_cli::RECIPES),
            || Box::new(crate::compute_cli::ComputeCliLib::new())
        );
        $m!(
            "compute/femm",
            "Resident FEMM compute solver",
            "compute-femm",
            Some(crate::compute_femm::RECIPES),
            || Box::new(crate::compute_femm::ComputeFemmLib::new(
                crate::compute_femm::ResidentCsrConfig::default(),
            ))
        );
        $m!(
            "compute/wgpu",
            "wgpu compute provider",
            "compute-wgpu",
            Some(crate::compute_wgpu::RECIPES),
            || Box::new(crate::compute_wgpu::ComputeWgpuLib::from_discovery(
                crate::compute_wgpu::WgpuDiscovery::from_probes(Vec::new(), Vec::new()),
            ))
        );
        $m!(
            "compute/cuda",
            "CUDA compute provider",
            "compute-cuda",
            Some(crate::compute_cuda::RECIPES),
            || Box::new(crate::compute_cuda::ComputeCudaLib::from_probe(
                crate::compute_cuda::CudaRuntimeProbe {
                    runtime: None,
                    evidence: None,
                    diagnostics: Vec::new(),
                },
            ))
        );
        $m!(
            "compute/rocm",
            "ROCm compute provider",
            "compute-rocm",
            Some(crate::compute_rocm::RECIPES),
            || Box::new(crate::compute_rocm::ComputeRocmLib::from_probe(
                crate::compute_rocm::RocmRuntimeProbe {
                    runtime: None,
                    evidence: None,
                    diagnostics: Vec::new(),
                },
            ))
        );
    };
}
