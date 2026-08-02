use std::collections::BTreeMap;
use std::sync::Arc;

use sim::discrete_search::{NeverInterrupt, SearchControl};
use sim::lib_music_core::{Articulation, Channel, Note, ObjectId, Pitch, StaffNote, Time};
use sim::lib_pitch_core::PitchClass;
use sim::serial_music::{
    adaptation::default_realizer_registry,
    completion::{
        CompletionCandidate, CompletionRequest, NoteAddition, SerialCompletionAllowances,
        SerialCompletionRequest, complete_serial,
    },
    practice::{
        BuiltInPracticeRule, DeclaredWaivers, EventPlacement, OrdinalRef, PlannedSerialEvent,
        PracticeId, PracticeRuleId, RowInstanceId, SerialEventId, SerialOrigin, SerialPlan,
        SerialPractice, SerialRole, StrictEventSpec, StrictRealizationContext, StructuralLicense,
        StructuralReadingId, realize_strict,
    },
    theory::{
        ROW_MATRIX_SIZE, RowFamily, RowFamilySet, RowLabelConvention, RowMatrix, RowOperation,
        ToneRow,
    },
};

use crate::support::CONFORMANCE_CONTRACT;

pub(crate) const SERIAL_MUSIC_PATH: &str = "crates/sim-conformance/tests/spec/serial_music.rs";

const ROW_MATRIX_RECIPE: &str =
    include_str!("../../../../recipes/serial-music/row-matrix-analysis/setup.rs");
const MODAL_RECIPE: &str =
    include_str!("../../../../recipes/serial-music/modal-realization/setup.rs");
const COMPLETION_RECIPE: &str =
    include_str!("../../../../recipes/serial-music/reversible-completion/setup.rs");
const SDK_FEATURES: &str = include_str!("../../../../features.toml");

#[test]
fn serial_music_facade_claims_reversibility_parity_and_discovery()
-> Result<(), Box<dyn std::error::Error>> {
    assert!(CONFORMANCE_CONTRACT.contains(SERIAL_MUSIC_PATH));
    assert!(SDK_FEATURES.contains("feature/sim-sdk/serial-music-composition"));
    assert!(SDK_FEATURES.contains("route/compose-serial-music-from-sdk"));

    let row = op25_row()?;
    let family = RowFamilySet::of(&row);
    assert_eq!(family.aliases().len(), 48);
    let matrix = RowMatrix::new(&row, RowLabelConvention::FirstLastPitch);
    assert_eq!(
        matrix.render_data().cells().len(),
        ROW_MATRIX_SIZE * ROW_MATRIX_SIZE
    );

    for snippet in [
        "sim::serial_music::theory",
        "RowFamilySet",
        "RowMatrix",
        "default_realizer_registry",
        "complete_serial",
    ] {
        assert!(
            ROW_MATRIX_RECIPE.contains(snippet)
                || MODAL_RECIPE.contains(snippet)
                || COMPLETION_RECIPE.contains(snippet)
        );
    }

    let plan = serial_plan()?;
    let specs = strict_specs()?;
    let realization = realize_strict(&plan, &StrictRealizationContext::new(specs))?;
    let practice = SerialPractice::new(
        PracticeId::new("practice/sdk-conformance-serial")?,
        vec![
            Arc::new(BuiltInPracticeRule::aggregate(PracticeRuleId::new(
                "rule/aggregate",
            )?)),
            Arc::new(BuiltInPracticeRule::order(PracticeRuleId::new(
                "rule/order",
            )?)),
            Arc::new(BuiltInPracticeRule::repeats(PracticeRuleId::new(
                "rule/repeats",
            )?)),
        ],
    );
    let result = complete_serial(
        &realization,
        &practice,
        &DeclaredWaivers::default(),
        &SerialCompletionRequest {
            completion: CompletionRequest {
                candidates: vec![CompletionCandidate::Note(NoteAddition {
                    note: note(
                        "voice/high",
                        "added-e",
                        64,
                        Time::from_integer(0),
                        quarter(),
                    ),
                })],
                min_candidates: 1,
                max_candidates: Some(1),
                pitch_ranges: Vec::new(),
            },
            allowances: SerialCompletionAllowances::default(),
        },
        SearchControl::default(),
        &NeverInterrupt,
    )?;
    assert_eq!(result.structural_plan, plan);
    assert_eq!(result.structural_before, result.structural_after);
    assert!(
        result
            .sounding_after
            .entries()
            .iter()
            .any(|entry| entry.rule_id.as_str() == "rule/repeats")
    );

    let mut context = StrictRealizationContext::new(strict_specs()?);
    context.modal_scale = Some(sim::lib_pitch_scale::PlayerScale::from_scale(
        sim::lib_pitch_scale::Scale::dorian(PitchClass::C),
    ));
    let registry = default_realizer_registry();
    let modal = registry.realize_named("realizer/modal-degree-cycle", &plan, &context)?;
    assert_eq!(modal.plan(), &plan);
    assert!(modal.spine_report().is_some());

    let missing = registry.realize_named("realizer/sdk-missing", &plan, &context);
    let detail = format!("{missing:?}");
    assert!(detail.contains("sdk-missing"));
    Ok(())
}

fn op25_row() -> Result<ToneRow, Box<dyn std::error::Error>> {
    Ok(ToneRow::try_from_classes([
        PitchClass::E,
        PitchClass::F,
        PitchClass::G,
        PitchClass::CS,
        PitchClass::FS,
        PitchClass::DS,
        PitchClass::GS,
        PitchClass::D,
        PitchClass::B,
        PitchClass::C,
        PitchClass::A,
        PitchClass::AS,
    ])?)
}

fn serial_plan() -> Result<SerialPlan, Box<dyn std::error::Error>> {
    let row = op25_row()?.apply(RowOperation::new(RowFamily::P, 0));
    let row_id = RowInstanceId::new("row/op25/p0")?;
    let license = StructuralLicense::new(
        StructuralReadingId::new("reading/sdk-conformance-serial")?,
        "sdk serial conformance reading",
    )?;
    let event =
        |id: &str, ordinal: usize| -> Result<PlannedSerialEvent, Box<dyn std::error::Error>> {
            Ok(PlannedSerialEvent {
                id: SerialEventId::new(id)?,
                ordinals: vec![OrdinalRef::new(row_id.clone(), ordinal)],
                role: SerialRole::Structural,
                origin: SerialOrigin::Structural {
                    rationale: "sdk serial conformance statement".to_owned(),
                },
                voice: ObjectId::new("voice/high")?,
                placement: EventPlacement::independent(),
                parents: Vec::new(),
                licenses: vec![license.clone()],
            })
        };
    SerialPlan::try_new(
        [(row_id.clone(), row)].into_iter().collect(),
        [
            event("event/a", 0)?,
            event("event/b", 1)?,
            event("event/c", 2)?,
            event("event/d", 3)?,
            event("event/e", 4)?,
            event("event/f", 5)?,
            event("event/g", 6)?,
            event("event/h", 7)?,
            event("event/i", 8)?,
            event("event/j", 9)?,
            event("event/k", 10)?,
            event("event/l", 11)?,
        ]
        .into_iter()
        .map(|event| (event.id.clone(), event))
        .collect(),
        [
            ("event/a", "event/b"),
            ("event/b", "event/c"),
            ("event/c", "event/d"),
            ("event/d", "event/e"),
            ("event/e", "event/f"),
            ("event/f", "event/g"),
            ("event/g", "event/h"),
            ("event/h", "event/i"),
            ("event/i", "event/j"),
            ("event/j", "event/k"),
            ("event/k", "event/l"),
        ]
        .into_iter()
        .map(|(before, after)| Ok((SerialEventId::new(before)?, SerialEventId::new(after)?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
    )
    .map_err(Into::into)
}

fn strict_specs() -> Result<BTreeMap<SerialEventId, StrictEventSpec>, Box<dyn std::error::Error>> {
    let channel = Channel::new(0)?;
    [
        "event/a", "event/b", "event/c", "event/d", "event/e", "event/f", "event/g", "event/h",
        "event/i", "event/j", "event/k", "event/l",
    ]
    .into_iter()
    .map(|id| {
        Ok((
            SerialEventId::new(id)?,
            StrictEventSpec::notes(4, quarter(), 96, channel, Articulation::Normal),
        ))
    })
    .collect()
}

fn quarter() -> Time {
    Time::new(1, 4)
}

fn note(voice_id: &str, event: &str, pitch: u8, onset: Time, duration: Time) -> StaffNote {
    StaffNote {
        voice_id: ObjectId::new(voice_id).expect("voice id"),
        note_id: ObjectId::new(format!("note/{event}")).expect("note id"),
        event_id: ObjectId::new(format!("event/{event}")).expect("event id"),
        onset,
        note: Note::new(
            duration,
            Pitch::from_midi(pitch),
            96,
            Channel::new(0).expect("channel"),
            Articulation::Normal,
        )
        .expect("note"),
    }
}
