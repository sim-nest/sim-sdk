use std::sync::Arc;

use sim::{
    interference_compute::{
        InterferenceComputeLib, TensorStudyConfig, TensorStudySolver,
        interference_compute_lib_symbol,
    },
    interference_core::{
        Emitter, FieldAmplitude, Hertz, InterferenceProblem, MetresPerSecond, NepersPerMetre,
        Point3M, PositiveMetres, Radians, SamplingPlane, ScalarMedium, SourceSet, UnitVector3,
    },
    interference_runtime::{
        PlaneDescriptor, ProblemDescriptor, ScalarProjectionDescriptor, StudyDescriptor,
        solve_function_symbol,
    },
    kernel::{
        Args, CapabilityName, Consistency, Cx, Error, EvalFabric, EvalMode, EvalReply, EvalRequest,
        Expr, NumberLiteral, ObjectCompat, Result, Symbol, Value,
    },
    lib_intent::{Origin, intent},
    lib_view::{Operation, SurfaceCodec, surface},
    view_interference::{
        INTERFERENCE_PROJECT_CAPABILITY, INTERFERENCE_SOLVE_CAPABILITY, InterferenceSurfaceCodec,
    },
};

use crate::support::{CONFORMANCE_CONTRACT, grant_capabilities, seated_cx};

pub(crate) const INTERFERENCE_PATH: &str = "crates/sim-conformance/tests/spec/interference.rs";

#[test]
fn interference_modeled_workflow_and_site_swap_compose_through_the_sdk() {
    assert!(CONFORMANCE_CONTRACT.contains(INTERFERENCE_PATH));

    let (mut cx, seat) = seated_cx();
    grant_capabilities(
        &seat,
        &mut cx,
        [
            CapabilityName::new(INTERFERENCE_PROJECT_CAPABILITY),
            CapabilityName::new(INTERFERENCE_SOLVE_CAPABILITY),
        ],
    );
    install_ci_sized_compute_provider(&mut cx);
    cx.load_lib(&sim::compute_model::ComputeModelLib::new(
        sim::compute_model::ModeledComputeProfile {
            max_queue_depth: 64,
            auto_flush_batches: true,
            ..sim::compute_model::ModeledComputeProfile::default()
        },
    ))
    .unwrap();

    let solve = bound_solve_expression(&mut cx, 8, 8);
    let solve_key = solve.canonical_key();
    let modeled = solve_at_modeled_site(&mut cx, solve.clone());
    assert_eq!(
        modeled.evidence.provider,
        Symbol::qualified("compute", "executor/model")
    );

    let codec = InterferenceSurfaceCodec::new();
    let modeled_expr = modeled.as_expr(&mut cx).unwrap();
    let scene = codec
        .encode(&mut cx, &modeled_expr, &surface::preset("desktop").unwrap())
        .unwrap();
    assert!(sim::lib_scene::node_kind(&scene).is_some());

    let project = checked_operation(
        &codec,
        &mut cx,
        &modeled_expr,
        edit(
            &modeled_expr,
            &["observable"],
            Expr::Symbol(Symbol::new("phase")),
        ),
    );
    let projection = realize_operation(&mut cx, &project)
        .object()
        .downcast_ref::<ScalarProjectionDescriptor>()
        .cloned()
        .expect("project edit realizes the canonical projection record");
    assert_eq!((projection.rows, projection.columns), (8, 8));

    let model = checked_operation(
        &codec,
        &mut cx,
        &modeled_expr,
        edit(&modeled_expr, &["frequency"], number(686.0)),
    );
    let refreshed = realize_operation(&mut cx, &model)
        .object()
        .downcast_ref::<StudyDescriptor>()
        .cloned()
        .expect("model edit realizes a re-solved canonical Study");
    assert_eq!(refreshed.problem.frequency_hz, 686.0);
    let refreshed_expr = refreshed.as_expr(&mut cx).unwrap();
    let refreshed_scene = codec
        .encode(
            &mut cx,
            &refreshed_expr,
            &surface::preset("desktop").unwrap(),
        )
        .unwrap();
    assert!(sim::lib_scene::node_kind(&refreshed_scene).is_some());

    let local = cx
        .eval_expr(solve.clone())
        .unwrap()
        .object()
        .downcast_ref::<StudyDescriptor>()
        .cloned()
        .expect("local placement returns a canonical Study");
    assert_eq!(solve.canonical_key(), solve_key);
    assert_ne!(local.evidence.provider, modeled.evidence.provider);
}

fn install_ci_sized_compute_provider(cx: &mut Cx) {
    let default_provider = cx
        .registry()
        .libs()
        .iter()
        .find(|loaded| loaded.manifest.id == interference_compute_lib_symbol())
        .map(|loaded| loaded.id)
        .expect("standard install registered the interference compute library");
    cx.unload_lib(default_provider).unwrap();
    cx.load_lib(&InterferenceComputeLib::new(TensorStudySolver::new(
        TensorStudyConfig {
            min_accelerated_cells: 1,
            ..TensorStudyConfig::default()
        },
    )))
    .unwrap();
}

fn bound_solve_expression(cx: &mut Cx, rows: usize, columns: usize) -> Expr {
    let problem = InterferenceProblem::new(
        Hertz::new(343.0).unwrap(),
        ScalarMedium::new(
            MetresPerSecond::new(343.0).unwrap(),
            NepersPerMetre::new(0.01).unwrap(),
        ),
        SourceSet::new(vec![Emitter::Point {
            id: "sdk-source".to_owned(),
            position: Point3M::from_metres(0.0, 0.0, 0.0).unwrap(),
            amplitude_at_reference: FieldAmplitude::new(1.0).unwrap(),
            phase: Radians::new(0.25).unwrap(),
        }])
        .unwrap(),
        PositiveMetres::new(0.01).unwrap(),
    );
    let plane = SamplingPlane::new(
        Point3M::from_metres(1.0, -0.125, -0.125).unwrap(),
        UnitVector3::new(0.0, 1.0, 0.0).unwrap(),
        UnitVector3::new(0.0, 0.0, 1.0).unwrap(),
        PositiveMetres::new(0.25).unwrap(),
        PositiveMetres::new(0.25).unwrap(),
        rows,
        columns,
    )
    .unwrap();
    let problem_symbol = Symbol::qualified("sdk-conformance", "problem");
    let plane_symbol = Symbol::qualified("sdk-conformance", "plane");
    let problem_value = cx
        .factory()
        .opaque(Arc::new(ProblemDescriptor::from_problem(&problem)))
        .unwrap();
    let plane_value = cx
        .factory()
        .opaque(Arc::new(PlaneDescriptor::from_plane(plane)))
        .unwrap();
    cx.env_mut().define(problem_symbol.clone(), problem_value);
    cx.env_mut().define(plane_symbol.clone(), plane_value);

    Expr::Call {
        operator: Box::new(Expr::Symbol(solve_function_symbol())),
        args: vec![
            Expr::Symbol(problem_symbol),
            Expr::Symbol(plane_symbol),
            Expr::Map(vec![
                (
                    Expr::Symbol(Symbol::new("sampling")),
                    Expr::Symbol(Symbol::new("annotate")),
                ),
                (
                    Expr::Symbol(Symbol::new("work-budget")),
                    Expr::Symbol(Symbol::new("default")),
                ),
            ]),
        ],
    }
}

fn solve_at_modeled_site(cx: &mut Cx, expr: Expr) -> StudyDescriptor {
    let site = cx
        .registry()
        .site_by_symbol(&sim::compute_model::compute_model_site_symbol())
        .cloned()
        .expect("modeled compute site is registered");
    site.object()
        .as_eval_fabric()
        .expect("modeled compute Site implements EvalFabric")
        .realize(cx, eval_request(expr))
        .unwrap()
        .value
        .object()
        .downcast_ref::<StudyDescriptor>()
        .cloned()
        .expect("modeled placement returns a canonical Study")
}

fn checked_operation(
    codec: &InterferenceSurfaceCodec,
    cx: &mut Cx,
    base: &Expr,
    submitted: Expr,
) -> Operation {
    let draft = codec.decode(cx, base, &submitted).unwrap();
    assert!(draft.committable, "submitted interference edit is valid");
    codec.commit(cx, &draft).unwrap()
}

fn realize_operation(cx: &mut Cx, operation: &Operation) -> Value {
    OperationFabric
        .realize(
            cx,
            EvalRequest {
                expr: operation.form.clone(),
                result_shape: operation.result_shape.clone(),
                required_capabilities: operation.required_capabilities.clone(),
                ..eval_request(Expr::Nil)
            },
        )
        .unwrap()
        .value
}

struct OperationFabric;

impl EvalFabric for OperationFabric {
    fn realize(&self, cx: &mut Cx, request: EvalRequest) -> Result<EvalReply> {
        for capability in &request.required_capabilities {
            cx.require(capability)?;
        }
        let value = evaluate_operation(cx, request.expr)?;
        if let Some(shape) = request.result_shape {
            let matched = shape
                .object()
                .as_shape()
                .expect("Operation result shape")
                .check_value(cx, value.clone())?;
            assert!(matched.accepted, "Operation result satisfies its Shape");
        }
        Ok(EvalReply {
            value,
            diagnostics: cx.take_diagnostics(),
            trace: None,
        })
    }
}

fn evaluate_operation(cx: &mut Cx, form: Expr) -> Result<Value> {
    let Expr::Call { operator, args } = form else {
        return Err(Error::Eval(
            "interference Operation must be a call".to_owned(),
        ));
    };
    let Expr::Symbol(function) = operator.as_ref() else {
        return Err(Error::Eval(
            "interference Operation requires a symbol operator".to_owned(),
        ));
    };
    let args = args
        .iter()
        .map(|arg| operation_argument(cx, arg))
        .collect::<Result<Vec<_>>>()?;
    cx.call_function(function, Args::new(args))
}

fn operation_argument(cx: &mut Cx, expr: &Expr) -> Result<Value> {
    let Expr::Extension { tag, payload } = expr else {
        return cx.eval_expr(expr.clone());
    };
    if *tag != Symbol::qualified("citizen", "read-construct") {
        return cx.eval_expr(expr.clone());
    }
    let Expr::Vector(parts) = payload.as_ref() else {
        return Err(Error::Eval(
            "citizen read-construct payload must be a vector".to_owned(),
        ));
    };
    let Some((Expr::Symbol(class), args)) = parts.split_first() else {
        return Err(Error::Eval(
            "citizen read-construct must begin with a class symbol".to_owned(),
        ));
    };
    let args = args
        .iter()
        .map(|arg| sim::citizen::value_from_expr(cx, arg))
        .collect::<Result<Vec<_>>>()?;
    cx.read_construct(class, args)
}

fn edit(base: &Expr, path: &[&str], value: Expr) -> Expr {
    intent(
        "edit-field",
        Origin::human(17),
        vec![
            ("target", base.clone()),
            (
                "path",
                Expr::List(
                    path.iter()
                        .map(|segment| Expr::Symbol(Symbol::new(*segment)))
                        .collect(),
                ),
            ),
            ("value", value),
        ],
    )
}

fn number(value: f64) -> Expr {
    Expr::Number(NumberLiteral {
        domain: Symbol::new("f64"),
        canonical: value.to_string(),
    })
}

fn eval_request(expr: Expr) -> EvalRequest {
    EvalRequest {
        expr,
        result_shape: None,
        required_capabilities: Vec::new(),
        deadline: None,
        consistency: Consistency::LocalFirst,
        mode: EvalMode::Eval,
        answer_limit: None,
        stream_buffer: None,
        stream: false,
        trace: false,
    }
}
