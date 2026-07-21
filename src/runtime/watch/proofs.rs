use std::rc::Rc;

use sim_kernel::{Error, Expr, Result, Symbol};
use sim_lib_stream_device::{DeviceSample, ModeledSource, seq_is_monotone};
use sim_lib_stream_wrist::{ModeledHeartRateSource, WornEvent, WornSensor};
use sim_lib_view_device::{
    DeviceProfile, DeviceProfileParts, DeviceSampleStore, EdgeId, EncodedScene, FrameClock,
    LocalAdapter, RateClass, StoreKey,
};
use sim_lib_view_wrist::{
    FleetSensorQuorum, FleetSensorSample, WatchCommand, WristSide, fleet_sensor_quorum, offer_worn,
    store_worn_sample, sweep_watch_privacy, tick_worn, watch_adapter_loop, watch_frame_clock_at,
    watch_glance_adapter, worn_state_from,
};
use sim_value::build;

/// Result of the hardware-free glance pager proof.
#[derive(Clone, Debug, PartialEq)]
pub struct GlancePagerProof {
    /// Whether the source is deterministic and monotone.
    pub modeled_source_monotone: bool,
    /// Whether the adapted glance becomes a watch notification command.
    pub notification_sent: bool,
    /// Modeled worn sample sequence.
    pub sample_seq: u64,
    /// Compact watch adapter cell budget.
    pub adapter_cells: u8,
    /// Number of notification body lines.
    pub notification_lines: usize,
    /// Encoded notification command.
    pub notification: Expr,
}

impl GlancePagerProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("watch/sdk", "glance-pager-proof")),
            (
                "modeled-source-monotone",
                Expr::Bool(self.modeled_source_monotone),
            ),
            ("notification-sent", Expr::Bool(self.notification_sent)),
            ("sample-seq", build::uint(self.sample_seq)),
            ("adapter-cells", build::uint(u64::from(self.adapter_cells))),
            (
                "notification-lines",
                build::uint(self.notification_lines as u64),
            ),
            ("notification", self.notification.clone()),
        ])
    }
}

/// Runs the modeled source -> shared glance adapter -> notification proof.
pub fn prove_glance_pager() -> Result<GlancePagerProof> {
    let source = ModeledHeartRateSource;
    let sample = source.at(14);
    let profile = watch_profile();
    let card = glance_card("Wrist", "HR", format!("{} bpm", heart_rate_bpm(&sample)?));
    let encoded = EncodedScene::new(card);
    let adapted =
        watch_glance_adapter(false).adapt(&encoded, &worn_state_from(&sample), &profile)?;
    let command = WatchCommand::notify_from_glance(adapted.as_ref())?;
    let (notification_sent, notification_lines) = match &command {
        WatchCommand::Notify { lines, .. } => (true, lines.len()),
        _ => (false, 0),
    };
    Ok(GlancePagerProof {
        modeled_source_monotone: seq_is_monotone(&source, 0, 4),
        notification_sent,
        sample_seq: sample.seq(),
        adapter_cells: watch_glance_adapter(false).budget.cells,
        notification_lines,
        notification: command.to_expr(),
    })
}

/// Result of the hardware-free hold-last proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldLastProof {
    /// Whether a stale frame reuses the last emitted card.
    pub held_last: bool,
    /// Number of coalesced worn updates reported as drops.
    pub dropped: u32,
    /// Whether the final frame is marked stale.
    pub stale: bool,
    /// Modeled sequence of the held frame.
    pub held_seq: u64,
}

impl HoldLastProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("watch/sdk", "hold-last-proof")),
            ("held-last", Expr::Bool(self.held_last)),
            ("dropped", build::uint(u64::from(self.dropped))),
            ("stale", Expr::Bool(self.stale)),
            ("held-seq", build::uint(self.held_seq)),
        ])
    }
}

/// Runs the modeled hold-last staleness proof.
pub fn prove_hold_last() -> Result<HoldLastProof> {
    let profile = watch_profile();
    let sample = ModeledHeartRateSource.at(0);
    let encoded = EncodedScene::new(glance_card("Wrist", "HR", "58 bpm"));
    let mut loop_ = watch_adapter_loop(&profile);

    offer_worn(&mut loop_, &sample);
    let fresh = tick_worn(
        &mut loop_,
        &watch_frame_clock_at(&profile, sample.seq()),
        &encoded,
        1,
        &sample,
        &profile,
    )?;

    for _ in 0..3 {
        offer_worn(&mut loop_, &sample);
    }
    let stale = tick_worn(
        &mut loop_,
        &watch_frame_clock_at(&profile, 10),
        &encoded,
        1,
        &sample,
        &profile,
    )?;

    Ok(HoldLastProof {
        held_last: Rc::ptr_eq(&fresh.out, &stale.out),
        dropped: stale.dropped,
        stale: stale.stale,
        held_seq: stale.seq,
    })
}

/// Result of the hardware-free privacy reaper proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivacyReaperProof {
    /// Whether the HR sample is evicted after the privacy window.
    pub hr_evicted: bool,
    /// Whether the location sample is evicted after the privacy window.
    pub location_evicted: bool,
    /// Whether referenced content is evicted with the sensitive samples.
    pub content_evicted: bool,
    /// Number of records evicted by the final sweep.
    pub evicted: usize,
}

impl PrivacyReaperProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("watch/sdk", "privacy-reaper-proof")),
            ("hr-evicted", Expr::Bool(self.hr_evicted)),
            ("location-evicted", Expr::Bool(self.location_evicted)),
            ("content-evicted", Expr::Bool(self.content_evicted)),
            ("evicted", build::uint(self.evicted as u64)),
        ])
    }
}

/// Runs the modeled privacy-window retention proof.
pub fn prove_privacy_reaper() -> Result<PrivacyReaperProof> {
    let profile = watch_profile();
    let receipt = WatchCommand::PrivacyMode {
        enabled: true,
        window_ms: 1_000,
    }
    .privacy_consent_receipt(EdgeId::named("watch-sdk-modeled"), 21)
    .ok_or_else(|| Error::Eval("enabled privacy command must yield a receipt".to_owned()))?;
    let mut store = DeviceSampleStore::new();
    let hr_content = StoreKey::named("watch-hr-content");
    let location_content = StoreKey::named("watch-location-content");
    store.insert_content(hr_content.clone(), build::text("heart-rate payload"));
    store.insert_content(location_content.clone(), build::text("location payload"));

    let hr = WornEvent::heart_rate(0, 72)?.to_expr();
    let location = WornEvent::gps(1, 59_329_300, 18_068_600, 450)?.to_expr();
    let hr_key = store_worn_sample(
        &mut store,
        &hr,
        &receipt,
        FrameClock::new(0, profile.rate),
        vec![hr_content.clone()],
    )?;
    let location_key = store_worn_sample(
        &mut store,
        &location,
        &receipt,
        FrameClock::new(0, profile.rate),
        vec![location_content.clone()],
    )?;

    let _kept = sweep_watch_privacy(
        &mut store,
        std::slice::from_ref(&receipt),
        FrameClock::new(0, profile.rate),
    );
    let evicted = sweep_watch_privacy(
        &mut store,
        std::slice::from_ref(&receipt),
        FrameClock::new(2, profile.rate),
    );

    Ok(PrivacyReaperProof {
        hr_evicted: !store.contains_sample(&hr_key),
        location_evicted: !store.contains_sample(&location_key),
        content_evicted: !store.contains_content(&hr_content)
            && !store.contains_content(&location_content),
        evicted: evicted.len(),
    })
}

/// Result of the hardware-free dual-watch quorum proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DualQuorumProof {
    /// Whether the divergent pair lowers confidence.
    pub low_confidence: bool,
    /// Quorum confidence in ten-thousandths.
    pub confidence: u16,
    /// Preferred side after scoring.
    pub prefer: Symbol,
    /// Absolute heart-rate disagreement.
    pub delta_bpm: u64,
}

impl DualQuorumProof {
    /// Encodes the proof as expression data for cookbook recipes.
    pub fn to_expr(&self) -> Expr {
        build::map(vec![
            ("kind", build::qsym("watch/sdk", "dual-quorum-proof")),
            ("low-confidence", Expr::Bool(self.low_confidence)),
            ("confidence", build::uint(u64::from(self.confidence))),
            ("prefer", Expr::Symbol(self.prefer.clone())),
            ("delta-bpm", build::uint(self.delta_bpm)),
        ])
    }
}

/// Runs the modeled dual-watch heart-rate quorum proof.
pub fn prove_dual_quorum() -> Result<DualQuorumProof> {
    let sensor = WornSensor::HeartRate.symbol();
    let left = FleetSensorSample::new(WristSide::Left, sensor.clone(), 72, 9_600)?;
    let right = FleetSensorSample::new(WristSide::Right, sensor, 94, 8_800)?;
    match fleet_sensor_quorum(&left, &right, 5)? {
        FleetSensorQuorum::LowConfidence {
            prefer,
            delta,
            confidence,
            ..
        } => Ok(DualQuorumProof {
            low_confidence: true,
            confidence,
            prefer: side_symbol(prefer),
            delta_bpm: delta,
        }),
        FleetSensorQuorum::Agree { confidence, .. } => Ok(DualQuorumProof {
            low_confidence: false,
            confidence,
            prefer: Symbol::qualified("watch/side", "none"),
            delta_bpm: 0,
        }),
    }
}

fn watch_profile() -> DeviceProfile {
    DeviceProfile::new(DeviceProfileParts {
        kind: Symbol::qualified("device", "watch-glance"),
        display: symbols(&["round"]),
        input: symbols(&["tap"]),
        output: symbols(&["haptic", "notification"]),
        links: symbols(&["modeled"]),
        streams: vec![WornSensor::HeartRate.symbol(), WornSensor::Gps.symbol()],
        rate: RateClass::watch(),
        policy: build::map(vec![
            ("consent", build::sym("visible")),
            ("retention-ms", build::uint(1_000)),
        ]),
    })
}

fn glance_card(title: &str, label: &str, value: impl Into<String>) -> Expr {
    build::map(vec![
        ("kind", build::qsym("scene", "glance")),
        ("title", build::text(title)),
        ("urgency", build::sym("info")),
        ("cells", build::uint(3)),
        ("bypass-budget", Expr::Bool(false)),
        (
            "metric",
            build::map(vec![
                ("label", build::text(label)),
                ("value", build::text(value)),
            ]),
        ),
        (
            "action",
            build::map(vec![
                ("label", build::text("ack")),
                ("target", build::qsym("watch/action", "ack")),
            ]),
        ),
    ])
}

fn heart_rate_bpm(event: &WornEvent) -> Result<u16> {
    let value = sim_value::access::required(
        event.payload(),
        "beats-per-minute",
        "watch heart-rate payload",
    )?;
    let Expr::Number(number) = value else {
        return Err(Error::TypeMismatch {
            expected: "heart-rate number",
            found: "non-number",
        });
    };
    number
        .canonical
        .parse()
        .map_err(|err| Error::Eval(format!("invalid modeled heart rate: {err}")))
}

fn symbols(names: &[&str]) -> Vec<Symbol> {
    names.iter().map(|name| Symbol::new(*name)).collect()
}

fn side_symbol(side: WristSide) -> Symbol {
    match side {
        WristSide::Left => Symbol::qualified("watch/side", "left"),
        WristSide::Right => Symbol::qualified("watch/side", "right"),
    }
}
