#![cfg(feature = "music-stack")]

use sim_kernel::Severity;
use sim_lib_midi_core::{Channel, MetaBucket, MetaEvent, MidiPayload};
use sim_lib_music_core::{
    Articulation, Chord, Counterpoint, Melody, MelodyItem, Music, MusicObject, Note, Score, Time,
};
use sim_lib_music_lift::{
    LabelStrategy, ProgressionLiftOpts, lift_to_piano_roll_report, lift_to_progression_report,
};
use sim_lib_music_lower::{LowerOpts, TrackSplit, lower_score};
use sim_lib_pitch_namer::{LabelContext, NamerRegistry, NamingSchool};
use sim_lib_pitch_scale::{Key, Mode};
use sim_lib_pitch_set::PitchClassMask;
use sim_lib_sound_bridge::BridgeOptions;
use sim_lib_sound_gm::general_midi_bank;
use sim_lib_sound_render::{PcmRenderer, RendererOptions};
use sim_lib_sound_tuning::TuningDescriptor;

use crate::music_stack::{
    MusicStackRenderOpts, render_score, render_score_report, render_smf_with_opts_report,
};

fn quarter() -> Time {
    Time::new(1, 4)
}

fn note(midi: u8, channel: u8) -> Note {
    Note::new(
        quarter(),
        sim_lib_pitch_core::Pitch::from_midi(midi),
        110,
        Channel::new(channel).expect("valid channel"),
        Articulation::Normal,
    )
    .expect("valid note")
}

fn melody_fixture() -> Score {
    let melody = Melody::new(vec![
        MelodyItem::Note(note(60, 0)),
        MelodyItem::Note(note(62, 0)),
        MelodyItem::Rest(sim_lib_music_core::Rest::new(quarter()).expect("valid rest")),
        MelodyItem::Note(note(64, 0)),
    ])
    .expect("valid melody");
    Score::new(120, (4, 4), Some("C".to_owned()), Music::Melody(melody)).expect("valid score")
}

fn progression_fixture() -> Score {
    let channel = Channel::new(0).expect("valid channel");
    let chords = vec![
        Chord::new(
            quarter(),
            "Cmaj".to_owned(),
            vec![
                sim_lib_pitch_core::Pitch::from_midi(60),
                sim_lib_pitch_core::Pitch::from_midi(64),
                sim_lib_pitch_core::Pitch::from_midi(67),
            ],
            112,
            channel,
        )
        .expect("valid chord"),
        Chord::new(
            quarter(),
            "Fmaj".to_owned(),
            vec![
                sim_lib_pitch_core::Pitch::from_midi(65),
                sim_lib_pitch_core::Pitch::from_midi(69),
                sim_lib_pitch_core::Pitch::from_midi(72),
            ],
            112,
            channel,
        )
        .expect("valid chord"),
        Chord::new(
            quarter(),
            "Gmaj".to_owned(),
            vec![
                sim_lib_pitch_core::Pitch::from_midi(67),
                sim_lib_pitch_core::Pitch::from_midi(71),
                sim_lib_pitch_core::Pitch::from_midi(74),
            ],
            112,
            channel,
        )
        .expect("valid chord"),
    ];
    Score::new(
        108,
        (4, 4),
        Some("C".to_owned()),
        Music::Progression(
            sim_lib_music_core::Progression::new(Some("C-major".to_owned()), chords)
                .expect("valid progression"),
        ),
    )
    .expect("valid score")
}

fn counterpoint_fixture() -> Score {
    let soprano = Melody::new(vec![
        MelodyItem::Note(note(72, 0)),
        MelodyItem::Note(note(74, 0)),
        MelodyItem::Note(note(76, 0)),
        MelodyItem::Note(note(77, 0)),
    ])
    .expect("valid melody");
    let alto = Melody::new(vec![
        MelodyItem::Note(note(67, 1)),
        MelodyItem::Note(note(69, 1)),
        MelodyItem::Note(note(71, 1)),
        MelodyItem::Note(note(72, 1)),
    ])
    .expect("valid melody");
    let tenor = Melody::new(vec![
        MelodyItem::Note(note(60, 2)),
        MelodyItem::Note(note(62, 2)),
        MelodyItem::Note(note(64, 2)),
        MelodyItem::Note(note(65, 2)),
    ])
    .expect("valid melody");
    let bass = Melody::new(vec![
        MelodyItem::Note(note(48, 3)),
        MelodyItem::Note(note(50, 3)),
        MelodyItem::Note(note(52, 3)),
        MelodyItem::Note(note(53, 3)),
    ])
    .expect("valid melody");
    let counterpoint = Counterpoint::new(
        vec![soprano, alto, tenor, bass],
        vec![
            "Soprano".to_owned(),
            "Alto".to_owned(),
            "Tenor".to_owned(),
            "Bass".to_owned(),
        ],
    )
    .expect("valid counterpoint");
    Score::new(
        96,
        (4, 4),
        Some("C".to_owned()),
        Music::Counterpoint(counterpoint),
    )
    .expect("valid score")
}

fn tuning() -> Box<dyn sim_lib_sound_tuning::Tuning> {
    TuningDescriptor::EqualTemperament {
        divisions: 12,
        reference_midi: 69,
        reference_hz: 440.0,
    }
    .to_tuning()
    .expect("valid tuning")
}

fn renderer() -> PcmRenderer {
    PcmRenderer::new(RendererOptions::new(22_050, 2).expect("valid renderer options"))
        .expect("valid renderer")
}

fn note_onsets(object: &dyn MusicObject) -> Vec<Time> {
    let mut atoms = Vec::new();
    object.voices(Time::from_integer(0), &mut atoms);
    let mut onsets = atoms
        .into_iter()
        .filter_map(|timed| match timed.atom {
            sim_lib_music_core::AtomRef::Note(_) => Some(timed.onset),
            _ => None,
        })
        .collect::<Vec<_>>();
    onsets.sort();
    onsets
}

fn abs_time_delta(left: Time, right: Time) -> Time {
    if left >= right {
        left - right
    } else {
        right - left
    }
}

#[test]
fn score_to_smf_to_score_preserves_note_onsets_within_grid() {
    let score = melody_fixture();
    let grid = Time::new(1, 16);
    let smf = lower_score(&score, &LowerOpts::default()).expect("lower");
    let report = lift_to_piano_roll_report(&smf).expect("lift");
    let expected = note_onsets(&score.body);
    let actual = report
        .value
        .items
        .iter()
        .map(|item| item.onset)
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), actual.len());
    for (left, right) in expected.into_iter().zip(actual) {
        assert!(abs_time_delta(left, right) <= grid);
    }
}

#[test]
fn score_to_smf_to_pcm_produces_non_silent_audio() {
    let score = progression_fixture();
    let audio =
        render_score(&score, &general_midi_bank(), tuning().as_ref(), &renderer()).expect("render");
    assert!(audio.iter().any(|sample| sample.abs() > 1.0e-4));
}

#[test]
fn midi_fixture_to_progression_labels_through_all_pitch_namers() {
    let score = progression_fixture();
    let smf = lower_score(&score, &LowerOpts::default()).expect("lower");
    let report = lift_to_progression_report(
        &smf,
        ProgressionLiftOpts {
            key_hint: Some(Key {
                tonic: sim_lib_pitch_core::PitchClass::C,
                mode: Mode::Major,
            }),
            label_strategy: LabelStrategy::JazzChord,
            ..ProgressionLiftOpts::default()
        },
    )
    .expect("lift progression");
    let chord = report.value.chords.first().expect("progression chord");
    let pitch_classes = chord
        .pitches
        .iter()
        .map(|pitch| pitch.class)
        .collect::<Vec<_>>();
    let mask = PitchClassMask::from_pitch_classes(&pitch_classes);
    let labels = NamerRegistry::new_with_builtins().label_all(
        mask,
        &LabelContext {
            root: Some(chord.pitches[0].class),
            key: Some(Key {
                tonic: sim_lib_pitch_core::PitchClass::C,
                mode: Mode::Major,
            }),
        },
    );
    for school in [
        NamingSchool::Forte,
        NamingSchool::FunctionalRoman,
        NamingSchool::SetTheory,
        NamingSchool::Riemannian,
        NamingSchool::Jazz,
    ] {
        let label = labels
            .iter()
            .find(|label| label.school == school)
            .expect("label");
        assert!(!label.text.is_empty());
    }
}

#[test]
fn multi_track_smf_fixture_emits_cross_layer_diagnostics() {
    let score = counterpoint_fixture();
    let smf = lower_score(
        &score,
        &LowerOpts {
            track_split: TrackSplit::CounterpointVoices,
            ..LowerOpts::default()
        },
    )
    .expect("lower");
    let mut smf = smf;
    smf.tracks[0].events.push(sim_lib_midi_core::MidiEvent {
        time: sim_lib_midi_core::TickTime::new(0, smf.tpq + 1).expect("tick time"),
        origin: sim_lib_midi_core::synthetic_origin(),
        payload: MidiPayload::Meta(MetaEvent::Other(MetaBucket {
            type_byte: 0x09,
            data: b"marker".to_vec(),
        })),
    });
    let report = render_smf_with_opts_report(
        &smf,
        &general_midi_bank(),
        tuning().as_ref(),
        &renderer(),
        &BridgeOptions::new(1, 200.0).expect("bridge options"),
    )
    .expect("render report");
    assert!(report.samples.iter().any(|sample| sample.abs() > 1.0e-4));
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown MIDI meta"))
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("tick quantization"))
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("voice stealing"))
    );
}

#[test]
fn clipping_diagnostic_is_reported_for_hot_render() {
    let mut score = progression_fixture();
    if let Music::Progression(prog) = &mut score.body {
        for chord in &mut prog.chords {
            chord.velocity = 127;
        }
    }
    let report = render_score_report(&score, &general_midi_bank(), tuning().as_ref(), &renderer())
        .expect("render report");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Warning
                && diag.message.contains("audio clipping"))
    );
}

#[test]
fn sound_music_integration_remains_feature_gated_at_root() {
    let _ = MusicStackRenderOpts::default();
}
