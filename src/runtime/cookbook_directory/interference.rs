macro_rules! cookbook_directory_interference {
    ($m:ident) => {
        $m!(
            "interference/records",
            "Interference records and Shapes",
            "interference-runtime",
            None,
            || Box::new(crate::interference_runtime::InterferenceRecordsLib)
        );
        $m!(
            "interference/runtime",
            "Interference runtime",
            "interference-runtime",
            Some(crate::interference_runtime::RECIPES),
            || Box::new(crate::interference_runtime::InterferenceLib)
        );
        $m!(
            "interference/compute",
            "Interference Tensor provider",
            "interference-compute",
            Some(crate::interference_compute::RECIPES),
            || Box::new(crate::interference_compute::InterferenceComputeLib::default())
        );
    };
}
