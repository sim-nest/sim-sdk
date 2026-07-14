use sim_kernel::{Cx, Export};

pub(super) fn install_optional_runtime_libs(cx: &mut Cx) {
    let _ = cx;
    #[cfg(feature = "list-cell")]
    {
        crate::list_cell::install_cons_list_lib(cx)
            .expect("core runtime should install the cons list backend");
    }

    #[cfg(feature = "list-lazy")]
    {
        crate::list_lazy::install_lazy_list_lib(cx)
            .expect("core runtime should install the lazy list backend");
    }

    #[cfg(feature = "table-hash")]
    {
        crate::table_hash::install_hash_table_lib(cx)
            .expect("core runtime should install the hash table backend");
    }

    #[cfg(feature = "table-override")]
    {
        crate::table_override::install_override_table_lib(cx)
            .expect("core runtime should install the override table class");
    }

    #[cfg(feature = "table-lazy")]
    {
        crate::table_lazy::install_lazy_table_lib(cx)
            .expect("core runtime should install the lazy table backend");
    }

    #[cfg(feature = "logic-core")]
    {
        crate::lib_logic::install_logic_lib(cx)
            .expect("core runtime should install the logic runtime library");
    }

    #[cfg(feature = "rank")]
    {
        crate::lib_rank::install_rank_lib(cx)
            .expect("core runtime should install the rank runtime library");
    }

    #[cfg(feature = "numbers-stats")]
    {
        cx.load_lib(&crate::lib_numbers_stats::StatsNumbersLib::new())
            .expect("core runtime should install the statistics runtime library");
    }

    #[cfg(feature = "discrete-runtime")]
    {
        sim_lib_discrete::install_discrete_lib(cx)
            .expect("core runtime should install the discrete runtime library");
    }

    #[cfg(feature = "cookbook")]
    {
        sim_lib_cookbook::install_seeded_cookbook_lib(cx)
            .expect("core runtime should install the seeded cookbook runtime library");
    }

    #[cfg(feature = "audio-graph-live")]
    {
        crate::lib_audio_graph_live::install_audio_graph_live_lib(cx)
            .expect("core runtime should install live audio graph browse surfaces");
    }

    #[cfg(feature = "audio-dsp")]
    {
        crate::lib_audio_dsp::install_audio_dsp_lib(cx)
            .expect("core runtime should install audio DSP browse surfaces");
    }

    #[cfg(feature = "music-synth")]
    {
        crate::lib_music_synth::install_audio_synth_lib(cx)
            .expect("core runtime should install audio synth browse surfaces");
    }

    #[cfg(feature = "plugin-core")]
    {
        crate::lib_plugin_core::install_plugin_core_lib(cx)
            .expect("core runtime should install plugin core browse surfaces");
    }

    #[cfg(feature = "plugin-clap")]
    {
        crate::lib_plugin_clap::install_clap_plugin_lib(cx)
            .expect("core runtime should install CLAP plugin browse surfaces");
    }

    #[cfg(feature = "plugin-lv2")]
    {
        crate::lib_plugin_lv2::install_lv2_plugin_lib(cx)
            .expect("core runtime should install LV2 plugin browse surfaces");
    }

    #[cfg(feature = "plugin-vst3")]
    {
        crate::lib_plugin_vst3::install_vst3_plugin_lib(cx)
            .expect("core runtime should install VST3 plugin browse surfaces");
    }

    #[cfg(feature = "daw-session")]
    {
        crate::lib_daw_session::install_daw_session_lib(cx)
            .expect("core runtime should install DAW session browse surfaces");
    }

    #[cfg(feature = "topology-core")]
    {
        crate::lib_topology::install_topology_lib(cx)
            .expect("core runtime should install topology runtime functions");
    }

    #[cfg(feature = "stream-portaudio")]
    {
        crate::lib_stream_portaudio::install_stream_portaudio_lib(cx)
            .expect("core runtime should install PortAudio browse surfaces");
    }

    #[cfg(feature = "stream-alsa")]
    {
        crate::lib_stream_alsa::install_stream_alsa_lib(cx)
            .expect("core runtime should install ALSA browse surfaces");
    }

    #[cfg(feature = "stream-pipewire")]
    {
        crate::lib_stream_pipewire::install_stream_pipewire_lib(cx)
            .expect("core runtime should install PipeWire browse surfaces");
    }

    #[cfg(feature = "stream-jack")]
    {
        crate::lib_stream_jack::install_stream_jack_lib(cx)
            .expect("core runtime should install JACK browse surfaces");
    }

    #[cfg(feature = "stream-asio")]
    {
        crate::lib_stream_asio::install_stream_asio_lib(cx)
            .expect("core runtime should install ASIO browse surfaces");
    }

    #[cfg(feature = "stream-coreaudio")]
    {
        crate::lib_stream_coreaudio::install_stream_coreaudio_lib(cx)
            .expect("core runtime should install CoreAudio browse surfaces");
    }

    #[cfg(feature = "pitch-namer")]
    {
        crate::lib_pitch_namer::install_pitch_namer_lib(cx)
            .expect("core runtime should install pitch namer browse surfaces");
    }

    #[cfg(feature = "pitch-dissonance")]
    {
        crate::lib_pitch_dissonance::install_pitch_dissonance_lib(cx)
            .expect("core runtime should install pitch dissonance browse surfaces");
    }

    #[cfg(feature = "pitch-shapes")]
    {
        crate::lib_pitch_shapes::install_pitch_shapes_lib(cx)
            .expect("core runtime should install pitch shape docs");
    }

    #[cfg(feature = "midi-core")]
    {
        crate::lib_midi_core::install_midi_io_lib(cx)
            .expect("core runtime should install MIDI I/O browse surfaces");
    }

    #[cfg(feature = "midi-live")]
    {
        crate::lib_midi_live::install_midi_live_lib(cx)
            .expect("core runtime should install MIDI live browse surfaces");
    }

    #[cfg(feature = "midi-rtmidi")]
    {
        crate::lib_midi_rtmidi::install_midi_rtmidi_lib(cx)
            .expect("core runtime should install RtMidi browse surfaces");
    }

    #[cfg(feature = "midi-ble")]
    {
        crate::lib_midi_ble::install_midi_ble_lib(cx)
            .expect("core runtime should install BLE-MIDI browse surfaces");
    }

    #[cfg(feature = "midi-shapes")]
    {
        crate::lib_midi_shapes::install_midi_shapes_lib(cx)
            .expect("core runtime should install MIDI shape docs");
    }

    #[cfg(feature = "music-lift")]
    {
        crate::lib_music_lift::install_music_lift_lib(cx)
            .expect("core runtime should install MIDI lifter browse surfaces");
    }

    #[cfg(feature = "music-notation")]
    {
        crate::lib_music_notation::install_music_notation_lib(cx)
            .expect("core runtime should install notation codec browse surfaces");
    }

    #[cfg(feature = "music-shapes")]
    {
        crate::lib_music_shapes::install_music_shapes_lib(cx)
            .expect("core runtime should install music shape docs");
    }

    #[cfg(feature = "sound-timbre")]
    {
        crate::lib_sound_timbre::install_sound_timbre_lib(cx)
            .expect("core runtime should install timbre browse surfaces");
    }

    #[cfg(feature = "sound-tuning")]
    {
        crate::lib_sound_tuning::install_sound_tuning_lib(cx)
            .expect("core runtime should install tuning browse surfaces");
    }

    #[cfg(feature = "sound-dissonance")]
    {
        crate::lib_sound_dissonance::install_sound_dissonance_lib(cx)
            .expect("core runtime should install sound dissonance browse surfaces");
    }

    #[cfg(feature = "sound-bridge")]
    {
        crate::lib_sound_bridge::install_sound_bridge_lib(cx)
            .expect("core runtime should install sound bridge browse surfaces");
    }

    #[cfg(feature = "sound-render")]
    {
        crate::lib_sound_render::install_sound_render_lib(cx)
            .expect("core runtime should install renderer browse surfaces");
    }

    #[cfg(feature = "sound-audio-lift")]
    {
        crate::lib_sound_audio_lift::install_sound_audio_lift_lib(cx)
            .expect("core runtime should install audio lifter browse surfaces");
    }

    #[cfg(feature = "sound-shapes")]
    {
        crate::lib_sound_shapes::install_sound_shapes_lib(cx)
            .expect("core runtime should install sound shape docs");
    }
}

pub(super) fn append_optional_core_function_exports(exports: &mut Vec<Export>) {
    let _ = exports;
    #[cfg(feature = "table-hash")]
    {
        exports.push(Export::Function {
            symbol: sim_kernel::Symbol::qualified("table", "hash"),
            function_id: None,
        });
    }

    #[cfg(feature = "table-lazy")]
    {
        exports.push(Export::Function {
            symbol: sim_kernel::Symbol::qualified("table", "lazy"),
            function_id: None,
        });
    }

    #[cfg(feature = "table-fs")]
    {
        exports.push(Export::Function {
            symbol: sim_kernel::Symbol::qualified("table", "fs"),
            function_id: None,
        });
    }

    #[cfg(feature = "table-db")]
    {
        exports.push(Export::Function {
            symbol: sim_kernel::Symbol::qualified("table", "db"),
            function_id: None,
        });
    }

    #[cfg(feature = "table-remote")]
    {
        exports.push(Export::Function {
            symbol: sim_kernel::Symbol::qualified("table", "remote"),
            function_id: None,
        });
    }

    #[cfg(feature = "logic-core")]
    {
        for symbol in [
            sim_kernel::Symbol::qualified("logic", "config"),
            sim_kernel::Symbol::qualified("logic", "assert!"),
            sim_kernel::Symbol::qualified("logic", "retract!"),
            sim_kernel::Symbol::qualified("logic", "facts"),
            sim_kernel::Symbol::qualified("logic", "consult"),
            sim_kernel::Symbol::qualified("logic", "consult!"),
            sim_kernel::Symbol::qualified("logic", "query"),
            sim_kernel::Symbol::qualified("logic", "query/one"),
            sim_kernel::Symbol::qualified("logic", "query/all"),
            sim_kernel::Symbol::qualified("logic", "query?"),
            sim_kernel::Symbol::qualified("logic", "predicate?"),
            sim_kernel::Symbol::qualified("logic", "stream-next"),
            sim_kernel::Symbol::qualified("logic", "stream-close"),
        ] {
            exports.push(Export::Function {
                symbol,
                function_id: None,
            });
        }
    }
}
