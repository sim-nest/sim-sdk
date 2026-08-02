use std::collections::BTreeMap;

use sim::lib_music_core::{Articulation, Channel, ObjectId, Time};
use sim::lib_pitch_core::PitchClass;
use sim::lib_pitch_scale::{PlayerScale, Scale};
use sim::serial_music::theory::{RowFamily, RowFamilySet, RowOperation, ToneRow};
use sim::serial_music::{
    adaptation::default_realizer_registry,
    practice::{
        EventPlacement, OrdinalRef, PlannedSerialEvent, RowInstanceId, SerialEventId, SerialOrigin,
        SerialPlan, SerialRole, StrictEventSpec, StrictRealizationContext, StructuralLicense,
        StructuralReadingId,
    },
};

pub fn modal_realization() -> Result<(), Box<dyn std::error::Error>> {
    let source_row = ToneRow::try_from_classes([
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
    ])?;
    let row = source_row.apply(RowOperation::new(RowFamily::P, 0));
    let row_id = RowInstanceId::new("row/op25/p0")?;
    let mut rows = BTreeMap::new();
    rows.insert(row_id.clone(), row.clone());
    let license = StructuralLicense::new(
        StructuralReadingId::new("reading/sdk-modal")?,
        "sdk modal recipe reading",
    )?;
    let event = |id: &str,
                 ordinals: &[usize],
                 voice: &str|
     -> Result<PlannedSerialEvent, Box<dyn std::error::Error>> {
        Ok(PlannedSerialEvent {
            id: SerialEventId::new(id)?,
            ordinals: ordinals
                .iter()
                .copied()
                .map(|ordinal| OrdinalRef::new(row_id.clone(), ordinal))
                .collect(),
            role: SerialRole::Structural,
            origin: SerialOrigin::Structural {
                rationale: "sdk modal statement".to_owned(),
            },
            voice: ObjectId::new(voice)?,
            placement: EventPlacement::independent(),
            parents: vec![],
            licenses: vec![license.clone()],
        })
    };
    let plan = SerialPlan::try_new(
        rows,
        [
            event("event/a", &[0, 1], "voice/high")?,
            event("event/b", &[2, 3], "voice/low")?,
            event("event/c", &[4, 5], "voice/high")?,
            event("event/d", &[6, 7], "voice/low")?,
            event("event/e", &[8, 9], "voice/high")?,
            event("event/f", &[10, 11], "voice/low")?,
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
        ]
        .into_iter()
        .map(|(before, after)| Ok((SerialEventId::new(before)?, SerialEventId::new(after)?)))
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
    )?;

    let channel = Channel::new(0)?;
    let quarter = Time::new(1, 4);
    let specs = [
        "event/a", "event/b", "event/c", "event/d", "event/e", "event/f",
    ]
    .into_iter()
    .map(|id| {
        Ok((
            SerialEventId::new(id)?,
            StrictEventSpec::notes(4, quarter, 96, channel, Articulation::Normal),
        ))
    })
    .collect::<Result<BTreeMap<_, _>, Box<dyn std::error::Error>>>()?;

    let mut context = StrictRealizationContext::new(specs);
    context.modal_scale = Some(PlayerScale::from_scale(Scale::dorian(PitchClass::C)));

    let registry = default_realizer_registry();
    let realization = registry.realize_named("realizer/modal-degree-cycle", &plan, &context)?;
    assert_eq!(realization.plan(), &plan);
    assert_eq!(realization.events().len(), 6);
    assert!(realization.spine_report().is_some());
    assert_eq!(RowFamilySet::of(&source_row).aliases().len(), 48);
    Ok(())
}
