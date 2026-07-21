macro_rules! cookbook_directory_watch {
    ($m:ident) => {
        $m!(
            "watch/sdk",
            "Watch SDK",
            "watch-modeled",
            Some(crate::runtime::watch::RECIPES),
            || Box::new(crate::runtime::watch::WatchStackLib)
        );
        $m!(
            "stream-wrist",
            "Worn Stream",
            "watch-modeled",
            Some(crate::lib_stream_wrist::RECIPES),
            || Box::new(crate::lib_stream_wrist::WristStreamLib)
        );
    };
}
