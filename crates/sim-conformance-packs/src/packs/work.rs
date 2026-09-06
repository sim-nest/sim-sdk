//! Pure bounded implementation-packet conformance scenarios.

use sim_conformance_core::{
    ConformanceError, DependencyUseSet, FakeInputPort, IdKind, InputPort, OwnerBinding,
    OwnerBindingId, OwnerCommand, SemanticId, SurfaceKey, SurfaceStatus, SurfaceUse,
    SurfaceUseRole,
};
use sim_kernel::{Datum, Symbol};
use sim_work_core::{
    DescentCertificate, ImplementationPacket, IndexImpact, InputBudget, PacketBuilder, PacketDraft,
    PacketInputSpec, StopCondition, SurfaceEvidence, SurfaceEvidenceState, WorkError,
    decode_proposal,
};

use crate::{CheckObservation, PackFailure, PackRequest, PackVerdict, harness};

/// Public seam used to judge packet builders supplied by another crate.
pub trait PacketLaw {
    /// Builds one packet using only its declared input port.
    fn build(
        &self,
        draft: PacketDraft,
        binding: &OwnerBinding,
        uses: &DependencyUseSet,
        evidence: &[SurfaceEvidence],
        inputs: Vec<PacketInputSpec>,
        port: &mut dyn InputPort,
    ) -> Result<ImplementationPacket, WorkError>;
}

/// The released `sim-work-core` packet builder.
#[derive(Clone, Copy, Debug, Default)]
pub struct CanonicalPacketLaw;

impl PacketLaw for CanonicalPacketLaw {
    fn build(
        &self,
        draft: PacketDraft,
        binding: &OwnerBinding,
        uses: &DependencyUseSet,
        evidence: &[SurfaceEvidence],
        inputs: Vec<PacketInputSpec>,
        port: &mut dyn InputPort,
    ) -> Result<ImplementationPacket, WorkError> {
        PacketBuilder::build(draft, binding, uses, evidence, inputs, port)
    }
}

/// Checks the exact requested work scope.
pub fn check(request: &PackRequest<'_>) -> PackVerdict {
    harness::check_registered("checker/c-work", request, |_| {
        check_implementation(&CanonicalPacketLaw)
    })
}

/// Runs the complete bootstrap packet scenario set through a public trait.
pub fn check_implementation(law: &dyn PacketLaw) -> Result<Vec<CheckObservation>, PackFailure> {
    deterministic_and_planned(law)?;
    source_bounds(law)?;
    dependency_refusals(law)?;
    hostile_return_bounds(law)?;
    Ok(vec![CheckObservation {
        key: "work.packet-pure-scenarios".into(),
        value: "deterministic,planned,undeclared,oversize,dependency,phase,owner,malformed".into(),
    }])
}

fn deterministic_and_planned(law: &dyn PacketLaw) -> Result<(), PackFailure> {
    let binding = binding()?;
    let uses = uses(binding.id().clone())?;
    let budget = budget();
    let mut first_port = FakeInputPort::default();
    let first_input = input(
        &mut first_port,
        b"exact source".to_vec(),
        "api/local-adapter",
        2,
    )?;
    let first = law
        .build(
            draft(binding.id().clone(), budget)?,
            &binding,
            &uses,
            &evidence(binding.id(), &uses)?,
            vec![first_input],
            &mut first_port,
        )
        .map_err(work_failure)?;
    let mut second_port = FakeInputPort::default();
    let second_input = input(
        &mut second_port,
        b"exact source".to_vec(),
        "api/local-adapter",
        2,
    )?;
    let second = law
        .build(
            draft(binding.id().clone(), budget)?,
            &binding,
            &uses,
            &evidence(binding.id(), &uses)?,
            vec![second_input],
            &mut second_port,
        )
        .map_err(work_failure)?;
    if first.id() != second.id()
        || first.funded_targets() != [SurfaceKey::new("api/projection").map_err(core_failure)?]
    {
        return Err(PackFailure {
            code: "nondeterministic-packet",
            detail: "equal semantic inputs did not produce the same planned-target packet".into(),
        });
    }
    Ok(())
}

fn source_bounds(law: &dyn PacketLaw) -> Result<(), PackFailure> {
    let binding = binding()?;
    let uses = uses(binding.id().clone())?;
    let mut port = FakeInputPort::default();
    let oversized = input(
        &mut port,
        b"source larger than budget".to_vec(),
        "api/local-adapter",
        1,
    )?;
    let result = law.build(
        draft(
            binding.id().clone(),
            InputBudget {
                bytes: 3,
                ..budget()
            },
        )?,
        &binding,
        &uses,
        &evidence(binding.id(), &uses)?,
        vec![oversized],
        &mut port,
    );
    if result != Err(WorkError::ByteBudget) {
        return Err(PackFailure {
            code: "oversize-input-accepted",
            detail: format!("observed {result:?}"),
        });
    }

    let mut port = FakeInputPort::default();
    let undeclared = input(&mut port, b"x".to_vec(), "api/not-declared", 1)?;
    if !matches!(
        law.build(
            draft(binding.id().clone(), budget())?,
            &binding,
            &uses,
            &evidence(binding.id(), &uses)?,
            vec![undeclared],
            &mut port,
        ),
        Err(WorkError::UndeclaredInput(_))
    ) {
        return Err(PackFailure {
            code: "undeclared-input-accepted",
            detail: "packet read a surface absent from its dependency-use set".into(),
        });
    }
    Ok(())
}

fn dependency_refusals(law: &dyn PacketLaw) -> Result<(), PackFailure> {
    let binding = binding()?;
    let uses = uses(binding.id().clone())?;
    let cases = [
        (
            "unimplemented-dependency",
            vec![
                SurfaceEvidence {
                    key: SurfaceKey::new("api/local-adapter").map_err(core_failure)?,
                    owner: sid("owner/adapter")?,
                    state: SurfaceEvidenceState::Planned {
                        producing_phase: "NV12.05".into(),
                    },
                },
                planned_target(binding.id().clone(), "NV12.06")?,
            ],
        ),
        (
            "wrong-target-phase",
            vec![
                released_dependency(&uses)?,
                planned_target(binding.id().clone(), "NV12.07")?,
            ],
        ),
        (
            "wrong-target-owner",
            vec![
                released_dependency(&uses)?,
                planned_target(sid("owner/other")?, "NV12.06")?,
            ],
        ),
    ];
    for (name, evidence) in cases {
        let mut port = FakeInputPort::default();
        let source = input(&mut port, b"source".to_vec(), "api/local-adapter", 1)?;
        if !matches!(
            law.build(
                draft(binding.id().clone(), budget())?,
                &binding,
                &uses,
                &evidence,
                vec![source],
                &mut port,
            ),
            Err(WorkError::Qualification(_))
        ) {
            return Err(PackFailure {
                code: "dependency-admission-mutant",
                detail: name.into(),
            });
        }
    }
    Ok(())
}

fn hostile_return_bounds(law: &dyn PacketLaw) -> Result<(), PackFailure> {
    let binding = binding()?;
    let uses = uses(binding.id().clone())?;
    let mut port = FakeInputPort::default();
    let source = input(&mut port, b"source".to_vec(), "api/local-adapter", 1)?;
    let packet = law
        .build(
            draft(binding.id().clone(), budget())?,
            &binding,
            &uses,
            &evidence(binding.id(), &uses)?,
            vec![source],
            &mut port,
        )
        .map_err(work_failure)?;
    let malformed = decode_proposal(
        &packet,
        b"not-a-packet",
        |_| Err("syntax".into()),
        |_, _| Ok(()),
        sid("facet/plan")?,
        vec![],
    );
    if !matches!(malformed, Err(WorkError::MalformedReturn(_))) {
        return Err(PackFailure {
            code: "malformed-output-accepted",
            detail: format!("observed {malformed:?}"),
        });
    }
    let oversized = vec![b'x'; budget().output_bytes as usize + 1];
    if decode_proposal(
        &packet,
        &oversized,
        |_| Ok(Datum::Nil),
        |_, _| Ok(()),
        sid("facet/plan")?,
        vec![],
    ) != Err(WorkError::OutputBudget)
    {
        return Err(PackFailure {
            code: "oversize-output-accepted",
            detail: "hostile return crossed its byte ceiling".into(),
        });
    }
    Ok(())
}

fn binding() -> Result<OwnerBinding, PackFailure> {
    OwnerBinding::new(
        "law/projection".into(),
        sim_conformance_core::OwnerDisposition::ExtractReusable,
        vec!["sim-projection-core".into()],
        vec![
            sim_conformance_core::BoundSurface {
                key: SurfaceKey::new("api/local-adapter").map_err(core_failure)?,
                public_name: "adapter::Local".into(),
                status: SurfaceStatus::Existing,
            },
            sim_conformance_core::BoundSurface {
                key: SurfaceKey::new("api/projection").map_err(core_failure)?,
                public_name: "projection::Projection".into(),
                status: SurfaceStatus::Planned {
                    producing_phase: "NV12.06".into(),
                },
            },
        ],
        vec!["sim-world".into(), "sim-prove".into()],
        "products -> projection -> kernel".into(),
        SurfaceKey::new("route/projection").map_err(core_failure)?,
        SurfaceKey::new("specimen/projection").map_err(core_failure)?,
        command("validate")?,
        command("docs")?,
    )
    .map_err(core_failure)
}

fn uses(owner: OwnerBindingId) -> Result<DependencyUseSet, PackFailure> {
    DependencyUseSet::new(
        "NV12.06".into(),
        owner,
        vec![
            SurfaceUse {
                surface: SurfaceKey::new("api/local-adapter").map_err(core_failure)?,
                role: SurfaceUseRole::ReleasedDependency,
            },
            SurfaceUse {
                surface: SurfaceKey::new("api/projection").map_err(core_failure)?,
                role: SurfaceUseRole::FundedTarget,
            },
        ],
    )
    .map_err(core_failure)
}

fn draft(owner: OwnerBindingId, input_budget: InputBudget) -> Result<PacketDraft, PackFailure> {
    Ok(PacketDraft {
        phase: "NV12.06".into(),
        owner,
        behavior: sid("behavior/projection")?,
        falsifier: sid("falsifier/map-order")?,
        allowed_api: vec![sid("type/datum")?],
        input_budget,
        output_contract: sid("shape/proposal")?,
        forbidden_edges: vec![sim_work_core::ForbiddenEdge {
            from: "projection".into(),
            to: "host-process".into(),
        }],
        tests_first: vec![sid("proof/map-order")?],
        validation: sid("command/validate")?,
        docs: sid("command/docs")?,
        index_impact: IndexImpact::SourceFacts,
        descent: DescentCertificate {
            measure: Symbol::qualified("work", "unknowns"),
            before: 2,
            after: 1,
        },
        stop: StopCondition {
            condition: Symbol::qualified("work", "gate-green"),
            max_attempts: 2,
        },
        commit_subject: Some("Implement projection law".into()),
    })
}

fn evidence(
    owner: &OwnerBindingId,
    uses: &DependencyUseSet,
) -> Result<Vec<SurfaceEvidence>, PackFailure> {
    Ok(vec![
        released_dependency(uses)?,
        planned_target(owner.clone(), "NV12.06")?,
    ])
}

fn released_dependency(uses: &DependencyUseSet) -> Result<SurfaceEvidence, PackFailure> {
    Ok(SurfaceEvidence {
        key: SurfaceKey::new("api/local-adapter").map_err(core_failure)?,
        owner: sid("owner/adapter")?,
        state: SurfaceEvidenceState::Released {
            dependency_uses: uses.id().clone(),
        },
    })
}

fn planned_target(owner: OwnerBindingId, phase: &str) -> Result<SurfaceEvidence, PackFailure> {
    Ok(SurfaceEvidence {
        key: SurfaceKey::new("api/projection").map_err(core_failure)?,
        owner,
        state: SurfaceEvidenceState::Planned {
            producing_phase: phase.into(),
        },
    })
}

fn input(
    port: &mut FakeInputPort,
    bytes: Vec<u8>,
    surface: &str,
    tokens: u64,
) -> Result<PacketInputSpec, PackFailure> {
    Ok(PacketInputSpec {
        surface: SurfaceKey::new(surface).map_err(core_failure)?,
        location: port.insert(bytes),
        tokens,
    })
}

fn budget() -> InputBudget {
    InputBudget {
        bytes: 32,
        files: 1,
        tokens: 4,
        output_bytes: 64,
    }
}

fn command(value: &str) -> Result<OwnerCommand, PackFailure> {
    OwnerCommand::new("repo".into(), vec![value.into()], "sealed".into()).map_err(core_failure)
}

fn sid<K: IdKind>(value: &str) -> Result<SemanticId<K>, PackFailure> {
    SemanticId::from_text(value).map_err(core_failure)
}

fn core_failure(error: ConformanceError) -> PackFailure {
    PackFailure {
        code: "packet-fixture",
        detail: error.to_string(),
    }
}

fn work_failure(error: WorkError) -> PackFailure {
    PackFailure {
        code: "packet-law-refusal",
        detail: error.to_string(),
    }
}
