use std::convert::Infallible;
use std::fmt;

use sim_kernel::{Diagnostic, Severity};
use sim_lib_midi_core::{
    MemoryMidiSource, MetaEvent, MidiPayload, PumpError, TrackedMidiEvent, pump,
};
use sim_lib_midi_smf::SmfFile;
use sim_lib_music_core::{MusicObject, Score, Time};
use sim_lib_music_lower::{LowerError, LowerOpts, lower_score};
use sim_lib_pitch_core::Pitch;
use sim_lib_sound_bridge::{
    BridgeOptions, MidiToSoundBridge, ScheduledTone, SoundBridgeError, TimbreBank,
};
use sim_lib_sound_core::Frequency;
use sim_lib_sound_render::{PcmRenderer, SoundRenderError};
use sim_lib_sound_tuning::{PitchClassN, SoundTuningError, Tuning};

/// Options controlling how a score is lowered to MIDI and bridged to sound.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MusicStackRenderOpts {
    /// Options for lowering a score to a MIDI file.
    pub lower: LowerOpts,
    /// Options for bridging MIDI events to scheduled sound tones.
    pub bridge: BridgeOptions,
}

/// Result of rendering a score: the lowered MIDI, scheduled tones, PCM samples,
/// and any diagnostics collected along the way.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderScoreReport {
    /// MIDI file the score lowered to.
    pub smf: SmfFile,
    /// Tones scheduled from the MIDI events.
    pub tones: Vec<ScheduledTone>,
    /// Rendered PCM audio samples.
    pub samples: Vec<f32>,
    /// Diagnostics gathered during rendering.
    pub diagnostics: Vec<Diagnostic>,
}

/// Error raised at one of the stages of the music rendering stack.
#[derive(Debug)]
pub enum MusicStackError {
    /// The score failed preflight: some notes cannot lower to MIDI.
    Preflight {
        /// Diagnostics explaining why preflight failed.
        diagnostics: Vec<Diagnostic>,
    },
    /// Lowering the score to MIDI failed.
    Lower(LowerError),
    /// Pumping MIDI events through the sound bridge failed.
    Pump(PumpError<Infallible, SoundBridgeError>),
    /// The MIDI-to-sound bridge failed.
    Bridge(SoundBridgeError),
    /// Rendering scheduled tones to PCM failed.
    Render(SoundRenderError),
}

impl fmt::Display for MusicStackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preflight { .. } => f.write_str("score contains notes that cannot lower to MIDI"),
            Self::Lower(error) => error.fmt(f),
            Self::Pump(PumpError::Source(_)) => {
                f.write_str("unexpected in-memory MIDI source failure")
            }
            Self::Pump(PumpError::Sink(error)) => error.fmt(f),
            Self::Bridge(error) => error.fmt(f),
            Self::Render(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for MusicStackError {}

impl From<LowerError> for MusicStackError {
    fn from(value: LowerError) -> Self {
        Self::Lower(value)
    }
}

impl From<PumpError<Infallible, SoundBridgeError>> for MusicStackError {
    fn from(value: PumpError<Infallible, SoundBridgeError>) -> Self {
        Self::Pump(value)
    }
}

impl From<SoundBridgeError> for MusicStackError {
    fn from(value: SoundBridgeError) -> Self {
        Self::Bridge(value)
    }
}

impl From<SoundRenderError> for MusicStackError {
    fn from(value: SoundRenderError) -> Self {
        Self::Render(value)
    }
}

/// Renders a score to PCM samples with default options.
pub fn render_score(
    score: &Score,
    bank: &TimbreBank,
    tuning: &dyn Tuning,
    pcm: &PcmRenderer,
) -> Result<Vec<f32>, MusicStackError> {
    Ok(render_score_report(score, bank, tuning, pcm)?.samples)
}

/// Renders a score with default options, returning the full render report.
pub fn render_score_report(
    score: &Score,
    bank: &TimbreBank,
    tuning: &dyn Tuning,
    pcm: &PcmRenderer,
) -> Result<RenderScoreReport, MusicStackError> {
    render_score_with_opts_report(score, bank, tuning, pcm, &MusicStackRenderOpts::default())
}

/// Renders a score with explicit options, returning the full render report.
pub fn render_score_with_opts_report(
    score: &Score,
    bank: &TimbreBank,
    tuning: &dyn Tuning,
    pcm: &PcmRenderer,
    opts: &MusicStackRenderOpts,
) -> Result<RenderScoreReport, MusicStackError> {
    let diagnostics = preflight_score(score);
    if diagnostics
        .iter()
        .any(|diag| diag.severity == Severity::Error)
    {
        return Err(MusicStackError::Preflight { diagnostics });
    }
    let smf = lower_score(score, &opts.lower)?;
    render_smf_with_opts_report(&smf, bank, tuning, pcm, &opts.bridge)
}

/// Renders an already-lowered MIDI file to PCM with explicit bridge options.
pub fn render_smf_with_opts_report(
    smf: &SmfFile,
    bank: &TimbreBank,
    tuning: &dyn Tuning,
    pcm: &PcmRenderer,
    bridge_opts: &BridgeOptions,
) -> Result<RenderScoreReport, MusicStackError> {
    let merged = smf.merged_events();
    let mut diagnostics = collect_midi_diagnostics(smf, &merged);
    let events = merged.into_iter().map(|tracked| tracked.event).collect();
    let mut source = MemoryMidiSource::new(smf.tpq, events);
    let mut bridge = MidiToSoundBridge::new(
        smf.tpq,
        bank.clone(),
        Box::new(FrozenTuning::from_tuning(tuning)),
        bridge_opts.clone(),
    )?;
    let _ = pump(&mut source, &mut bridge)?;
    if bridge.stolen_voice_count() > 0 {
        diagnostics.push(warning(format!(
            "voice stealing: {} voice(s) were stolen by the bridge polyphony limit",
            bridge.stolen_voice_count()
        )));
    }
    let tones = bridge.drain_tones();
    let samples = pcm.render_mix(&tones);
    if let Some(peak) = peak_abs_sample(&samples)
        && peak > 1.0
    {
        diagnostics.push(warning(format!(
            "audio clipping: render peak {:.3} exceeds PCM full scale",
            peak
        )));
    }
    Ok(RenderScoreReport {
        smf: smf.clone(),
        tones,
        samples,
        diagnostics,
    })
}

fn preflight_score(score: &Score) -> Vec<Diagnostic> {
    let mut atoms = Vec::new();
    score.body.voices(Time::from_integer(0), &mut atoms);
    atoms
        .into_iter()
        .filter_map(|timed| match timed.atom {
            sim_lib_music_core::AtomRef::Note(note) if note.pitch.to_midi().is_none() => {
                Some(Diagnostic::error(format!(
                    "pitch clipping: {:?} at onset {} cannot lower to MIDI",
                    note.pitch, timed.onset
                )))
            }
            _ => None,
        })
        .collect()
}

fn collect_midi_diagnostics(smf: &SmfFile, merged: &[TrackedMidiEvent]) -> Vec<Diagnostic> {
    let unknown_meta = merged
        .iter()
        .filter(|tracked| {
            matches!(
                tracked.event.payload,
                MidiPayload::Meta(MetaEvent::Other(_))
            )
        })
        .count();
    let quantized = merged
        .iter()
        .filter(|tracked| tracked.event.time.tpq != smf.tpq)
        .count();
    let mut diagnostics = Vec::new();
    if unknown_meta > 0 {
        diagnostics.push(warning(format!(
            "unknown MIDI meta: {} event(s) preserved but ignored by the sound bridge",
            unknown_meta
        )));
    }
    if quantized > 0 {
        diagnostics.push(warning(format!(
            "tick quantization: {} event(s) required TPQ rebasing into {} TPQ",
            quantized, smf.tpq
        )));
    }
    diagnostics
}

fn peak_abs_sample(samples: &[f32]) -> Option<f32> {
    samples
        .iter()
        .map(|sample| sample.abs())
        .max_by(|left, right| left.total_cmp(right))
}

fn warning(message: String) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        message,
        source: None,
        span: None,
        code: None,
        related: Vec::new(),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FrozenTuning {
    name: &'static str,
    reference: (Pitch, Frequency),
    divisions: u32,
    midi_frequencies: [Frequency; 128],
}

impl FrozenTuning {
    fn from_tuning(tuning: &dyn Tuning) -> Self {
        let mut midi_frequencies = [Frequency(440.0); 128];
        for (midi, slot) in midi_frequencies.iter_mut().enumerate() {
            *slot = tuning.frequency_of(Pitch::from_midi(midi as u8));
        }
        Self {
            name: tuning.name(),
            reference: tuning.reference(),
            divisions: tuning.divisions(),
            midi_frequencies,
        }
    }
}

impl Tuning for FrozenTuning {
    fn name(&self) -> &'static str {
        self.name
    }

    fn reference(&self) -> (Pitch, Frequency) {
        self.reference
    }

    fn frequency_of(&self, pitch: Pitch) -> Frequency {
        match pitch.to_midi() {
            Some(midi) => self.midi_frequencies[midi as usize],
            None => self
                .reference
                .1
                .shift_cents(f64::from(pitch.semitone() - self.reference.0.semitone()) * 100.0),
        }
    }

    fn pitch_of(&self, frequency: Frequency) -> Pitch {
        self.midi_frequencies
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                frequency
                    .cents_above(**left)
                    .abs()
                    .total_cmp(&frequency.cents_above(**right).abs())
            })
            .map(|(midi, _)| Pitch::from_midi(midi as u8))
            .unwrap_or(self.reference.0)
    }

    fn divisions(&self) -> u32 {
        self.divisions
    }

    fn frequency_of_degree(
        &self,
        degree: PitchClassN,
        octave: i16,
    ) -> Result<Frequency, SoundTuningError> {
        let pitch = self.pitch_from_degree(degree, octave)?;
        Ok(self.frequency_of(pitch))
    }
}
