macro_rules! cookbook_directory_audio_stream {
    ($m:ident) => {
        $m!(
            "audio-dsp",
            "Audio DSP",
            "audio-dsp",
            Some(crate::lib_audio_dsp::RECIPES),
            || Box::new(crate::lib_audio_dsp::AudioDspLib)
        );
        $m!(
            "audio-graph-live",
            "Audio graph live",
            "audio-graph-live",
            Some(crate::lib_audio_graph_live::RECIPES),
            || Box::new(crate::lib_audio_graph_live::AudioGraphLiveLib)
        );
        $m!(
            "daw-session",
            "DAW session",
            "daw-session",
            Some(crate::lib_daw_session::RECIPES),
            || Box::new(crate::lib_daw_session::DawSessionLib)
        );
        $m!(
            "plugin-clap",
            "CLAP plugin",
            "plugin-clap",
            Some(crate::lib_plugin_clap::RECIPES),
            || Box::new(crate::lib_plugin_clap::ClapPluginLib)
        );
        $m!(
            "plugin-vst3",
            "VST3 plugin",
            "plugin-vst3",
            Some(crate::lib_plugin_vst3::RECIPES),
            || Box::new(crate::lib_plugin_vst3::Vst3PluginLib)
        );
        $m!(
            "stream-alsa",
            "ALSA stream backend",
            "stream-alsa",
            None,
            || Box::new(crate::lib_stream_alsa::AlsaLib)
        );
        $m!(
            "stream-asio",
            "ASIO stream backend",
            "stream-asio",
            None,
            || Box::new(crate::lib_stream_asio::AsioLib)
        );
        $m!(
            "stream-coreaudio",
            "CoreAudio stream backend",
            "stream-coreaudio",
            None,
            || Box::new(crate::lib_stream_coreaudio::CoreAudioLib)
        );
        $m!(
            "stream-jack",
            "JACK stream backend",
            "stream-jack",
            None,
            || Box::new(crate::lib_stream_jack::JackLib)
        );
        $m!(
            "stream-pipewire",
            "PipeWire stream backend",
            "stream-pipewire",
            None,
            || Box::new(crate::lib_stream_pipewire::PipeWireLib)
        );
        $m!(
            "stream-portaudio",
            "PortAudio stream backend",
            "stream-portaudio",
            None,
            || Box::new(crate::lib_stream_portaudio::PortAudioLib)
        );
        $m!(
            "stream-bridge",
            "Stream bridge",
            "stream-bridge",
            Some(crate::lib_stream_bridge::RECIPES),
            || Box::new(crate::lib_stream_bridge::StreamBridgeLib)
        );
        $m!(
            "stream-core-shapes",
            "Stream core shapes",
            "stream-core",
            Some(crate::lib_stream_core::RECIPES),
            || Box::new(crate::lib_stream_core::StreamCoreShapesLib)
        );
        $m!(
            "stream-prelude",
            "Stream prelude",
            "stream-prelude",
            Some(crate::lib_stream_prelude::RECIPES),
            || Box::new(crate::lib_stream_prelude::StreamPreludeLib)
        );
        $m!(
            "topology",
            "Topology runtime",
            "topology-core",
            Some(crate::lib_topology::RECIPES),
            || Box::new(crate::lib_topology::TopologyLib::new())
        );
    };
}
