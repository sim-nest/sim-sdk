macro_rules! cookbook_directory_glasses {
    ($m:ident) => {
        $m!(
            "glasses/sdk",
            "Glasses SDK",
            "glasses-modeled",
            Some(crate::runtime::glasses::RECIPES),
            || Box::new(crate::runtime::glasses::GlassesStackLib)
        );
        $m!(
            "stream-xr",
            "XR Stream",
            "glasses-modeled",
            Some(crate::lib_stream_xr::RECIPES),
            || Box::new(crate::lib_stream_xr::XrStreamLib)
        );
    };
}
