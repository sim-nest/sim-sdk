macro_rules! cookbook_directory_music {
    ($m:ident) => {
        $m!(
            "midi-io",
            "MIDI IO",
            "midi-core",
            Some(crate::lib_midi_core::RECIPES),
            || Box::new(crate::lib_midi_core::MidiIoLib)
        );
        $m!(
            "midi/digest",
            "MIDI digest",
            "midi-core",
            Some(crate::lib_midi_core::RECIPES),
            || Box::new(crate::lib_midi_core::MidiDigestLib)
        );
        $m!(
            "midi-live",
            "MIDI live",
            "midi-live",
            Some(crate::lib_midi_live::RECIPES),
            || Box::new(crate::lib_midi_live::MidiLiveLib)
        );
        $m!(
            "midi-ble",
            "MIDI BLE",
            "midi-ble",
            Some(crate::lib_midi_ble::RECIPES),
            || Box::new(crate::lib_midi_ble::MidiBleLib)
        );
        $m!(
            "midi-rtmidi",
            "MIDI RtMidi",
            "midi-rtmidi",
            Some(crate::lib_midi_rtmidi::RECIPES),
            || Box::new(crate::lib_midi_rtmidi::MidiRtmidiLib)
        );
        $m!(
            "midi-shapes",
            "MIDI shapes",
            "midi-shapes",
            Some(crate::lib_midi_shapes::RECIPES),
            || Box::new(crate::lib_midi_shapes::MidiShapesLib)
        );
        $m!(
            "music-lift",
            "Music lift",
            "music-lift",
            Some(crate::lib_music_lift::RECIPES),
            || Box::new(crate::lib_music_lift::MusicLiftLib)
        );
        $m!(
            "music-notation",
            "Music notation",
            "music-notation",
            Some(crate::lib_music_notation::RECIPES),
            || Box::new(crate::lib_music_notation::MusicNotationLib)
        );
        $m!(
            "music-shapes",
            "Music shapes",
            "music-shapes",
            Some(crate::lib_music_shapes::RECIPES),
            || Box::new(crate::lib_music_shapes::MusicShapesLib)
        );
        $m!(
            "audio-synth",
            "Audio synth",
            "music-synth",
            Some(crate::lib_music_synth::RECIPES),
            || Box::new(crate::lib_music_synth::AudioSynthLib)
        );
        $m!(
            "pitch-dissonance",
            "Pitch dissonance",
            "pitch-dissonance",
            Some(crate::lib_pitch_dissonance::RECIPES),
            || Box::new(crate::lib_pitch_dissonance::PitchDissonanceLib)
        );
        $m!(
            "pitch-namer",
            "Pitch namer",
            "pitch-namer",
            Some(crate::lib_pitch_namer::RECIPES),
            || Box::new(crate::lib_pitch_namer::PitchNamerLib)
        );
        $m!(
            "pitch-shapes",
            "Pitch shapes",
            "pitch-shapes",
            Some(crate::lib_pitch_shapes::RECIPES),
            || Box::new(crate::lib_pitch_shapes::PitchShapesLib)
        );
        $m!(
            "sound-audio-lift",
            "Sound audio lift",
            "sound-audio-lift",
            Some(crate::lib_sound_audio_lift::RECIPES),
            || Box::new(crate::lib_sound_audio_lift::SoundAudioLiftLib)
        );
        $m!(
            "sound-bridge",
            "Sound bridge",
            "sound-bridge",
            Some(crate::lib_sound_bridge::RECIPES),
            || Box::new(crate::lib_sound_bridge::SoundBridgeLib)
        );
        $m!(
            "sound-dissonance",
            "Sound dissonance",
            "sound-dissonance",
            Some(crate::lib_sound_dissonance::RECIPES),
            || Box::new(crate::lib_sound_dissonance::SoundDissonanceLib)
        );
        $m!(
            "sound-render",
            "Sound render",
            "sound-render",
            Some(crate::lib_sound_render::RECIPES),
            || Box::new(crate::lib_sound_render::SoundRenderLib)
        );
        $m!(
            "sound-shapes",
            "Sound shapes",
            "sound-shapes",
            Some(crate::lib_sound_shapes::RECIPES),
            || Box::new(crate::lib_sound_shapes::SoundShapesLib)
        );
        $m!(
            "sound-timbre",
            "Sound timbre",
            "sound-timbre",
            Some(crate::lib_sound_timbre::RECIPES),
            || Box::new(crate::lib_sound_timbre::SoundTimbreLib)
        );
        $m!(
            "sound-tuning",
            "Sound tuning",
            "sound-tuning",
            Some(crate::lib_sound_tuning::RECIPES),
            || Box::new(crate::lib_sound_tuning::SoundTuningLib)
        );
    };
}
