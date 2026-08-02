use std::sync::Arc;

use sim::{
    codec::{DecodePosition, DecodedForm, Input, decode_default_with_codec},
    expr_tree::{
        RECIPES, expr_tree_calculate_capability, expr_tree_mount_capability,
        expr_tree_read_capability, expr_tree_write_capability, install_expr_tree_lib,
    },
    expr_tree_server::{ExpressionTreeServer, SessionId},
    kernel::{
        Consistency, Cx, EvalFabric, EvalMode, EvalRequest, Expr, NumberLiteral, ReadPolicy,
        Symbol, Value,
    },
    lib_intent::{Origin, intent},
    lib_server::{
        DeterministicWallClock, EvalSite, ServerAddress, register_loopback_transport_endpoint,
    },
    lib_view::LensRegistry,
    lib_web_bridge::{PhoneHost, RemoteTransport},
    view_expr_tree::{
        expression_tree_surface_codec_symbol, register_expression_tree_surface_codec,
    },
};

use crate::support::{CONFORMANCE_CONTRACT, grant_capabilities, seated_cx};

pub(crate) const EXPR_TREE_PATH: &str = "crates/sim-conformance/tests/spec/expr_tree.rs";
const ADDRESS_THREAD: u64 = 17_029;

#[test]
fn expr_tree_recipe_restart_view_and_authority_compose_through_the_sdk() {
    assert!(CONFORMANCE_CONTRACT.contains(EXPR_TREE_PATH));

    let mut recipe_cx = full_runtime_cx();
    let finite = eval_lisp(
        &mut recipe_cx,
        embedded_recipe("01-basics/finite-tree/setup.siml"),
    );
    let Expr::List(root_entries) = finite.object().as_expr(&mut recipe_cx).unwrap() else {
        panic!("finite-tree recipe returns its bounded mixed-backend root");
    };
    assert_eq!(root_entries.len(), 3);
    let finite_tree = eval_lisp(&mut recipe_cx, "(expr-tree/open \"recipe-finite-tree\")");
    bind(&mut recipe_cx, "sdk-finite-tree", finite_tree);
    assert_eq!(
        eval_expr(
            &mut recipe_cx,
            "(expr-tree/ref sdk-finite-tree \"/measurements/trial-0001\")",
        ),
        Expr::String("measured-value".to_owned())
    );

    let explanation = eval_lisp(
        &mut recipe_cx,
        embedded_recipe("02-calculation/automatic-and-directed/setup.siml"),
    );
    assert_eq!(
        table_field_expr(&mut recipe_cx, &explanation, "status"),
        Expr::Symbol(Symbol::qualified("expr-tree/status", "fresh"))
    );
    let calculation_tree = eval_lisp(
        &mut recipe_cx,
        "(expr-tree/open \"recipe-automatic-and-directed\")",
    );
    bind(&mut recipe_cx, "sdk-calculation-tree", calculation_tree);
    assert_eq!(
        eval_expr(
            &mut recipe_cx,
            "(expr-tree/ref sdk-calculation-tree \"/manual\")",
        ),
        Expr::String("automatic-value".to_owned())
    );
    assert_eq!(
        eval_expr(
            &mut recipe_cx,
            "(expr-tree/ref sdk-calculation-tree \"/cycle-a\")",
        ),
        Expr::String("recovered".to_owned())
    );

    let address = ServerAddress::InProcess {
        thread: ADDRESS_THREAD,
    };
    let active_server = Arc::new(new_server(address.clone(), 10));
    let active_site: Arc<dyn EvalSite> = active_server.clone();
    let endpoint = register_loopback_transport_endpoint(address.clone(), active_site).unwrap();
    let mut creator = full_runtime_cx();
    let session = active_server
        .create_session(&mut creator, "sdk-expression-tree")
        .unwrap();
    assert_eq!(
        realize_expr(
            &active_server,
            &mut creator,
            runtime_call(
                &session,
                "new-cell",
                vec![
                    Expr::String("/".to_owned()),
                    Expr::String("answer".to_owned()),
                    Expr::String("42".to_owned()),
                ],
            ),
        ),
        Expr::String("/answer".to_owned())
    );

    let registry = view_registry();
    let mut viewer_cx = read_runtime_cx();
    let viewer_transport = connect(&mut viewer_cx, &address);
    let mut viewer =
        PhoneHost::with_surface_codec(viewer_transport, expression_tree_surface_codec_symbol());
    let scene = viewer
        .open(&mut viewer_cx, &registry, session.resource())
        .unwrap();
    sim::lib_scene::validate_scene(&scene).unwrap();

    let revision = active_server.revision(&session).unwrap();
    let denied = viewer
        .submit(
            &mut viewer_cx,
            &registry,
            edit_source(&session, revision, "/answer", "forbidden"),
        )
        .unwrap_err();
    assert!(
        denied.to_string().contains("authority-denied"),
        "server-backed view must preserve diminished read-only authority: {denied}"
    );
    assert_eq!(
        realize_expr(
            &active_server,
            &mut creator,
            runtime_call(&session, "ref", vec![Expr::String("/answer".to_owned())],),
        ),
        Expr::String("42".to_owned())
    );

    drop(viewer);
    drop(endpoint);
    drop(active_server);

    let restarted = Arc::new(new_server(address.clone(), 100));
    let restarted_site: Arc<dyn EvalSite> = restarted.clone();
    let _restarted_endpoint =
        register_loopback_transport_endpoint(address.clone(), restarted_site).unwrap();
    let mut restarted_cx = full_runtime_cx();
    let stale = realize_expr(
        &restarted,
        &mut restarted_cx,
        runtime_call(&session, "list", vec![Expr::String("/".to_owned())]),
    );
    assert_eq!(error_code(&stale).as_deref(), Some("unknown-session"));

    let fresh = restarted
        .create_session(&mut restarted_cx, "sdk-expression-tree-restarted")
        .unwrap();
    let mut reconnected_cx = read_runtime_cx();
    let reconnected_transport = connect(&mut reconnected_cx, &address);
    let mut reconnected = PhoneHost::with_surface_codec(
        reconnected_transport,
        expression_tree_surface_codec_symbol(),
    );
    let fresh_scene = reconnected
        .open(&mut reconnected_cx, &registry, fresh.resource())
        .unwrap();
    sim::lib_scene::validate_scene(&fresh_scene).unwrap();
}

fn embedded_recipe(path: &str) -> &'static str {
    RECIPES
        .iter()
        .find_map(|(candidate, bytes)| {
            (*candidate == path).then(|| {
                std::str::from_utf8(bytes)
                    .unwrap_or_else(|error| panic!("embedded recipe {path} is not UTF-8: {error}"))
            })
        })
        .unwrap_or_else(|| panic!("missing embedded expression-tree recipe {path}"))
}

fn full_runtime_cx() -> Cx {
    runtime_cx(true)
}

fn read_runtime_cx() -> Cx {
    runtime_cx(false)
}

fn runtime_cx(writable: bool) -> Cx {
    let (mut cx, seat) = seated_cx();
    let mut capabilities = vec![expr_tree_read_capability()];
    if writable {
        capabilities.extend([
            expr_tree_write_capability(),
            expr_tree_calculate_capability(),
            expr_tree_mount_capability(),
        ]);
    }
    grant_capabilities(&seat, &mut cx, capabilities);
    install_expr_tree_lib(&mut cx).unwrap();
    cx
}

fn eval_lisp(cx: &mut Cx, source: &str) -> Value {
    let decoded = decode_default_with_codec(
        cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(source.to_owned()),
        ReadPolicy::default(),
        DecodePosition::Eval,
    )
    .unwrap();
    let expression = match decoded {
        DecodedForm::Term(term) => Expr::from(term),
        DecodedForm::Datum(datum) => Expr::from(datum),
    };
    cx.eval_expr(expression).unwrap()
}

fn eval_expr(cx: &mut Cx, source: &str) -> Expr {
    eval_lisp(cx, source).object().as_expr(cx).unwrap()
}

fn bind(cx: &mut Cx, name: &str, value: Value) {
    cx.env_mut().define(Symbol::new(name), value);
}

fn table_field_expr(cx: &mut Cx, table: &Value, name: &str) -> Expr {
    table
        .object()
        .as_table_impl()
        .expect("recipe explanation is a table")
        .get(cx, Symbol::new(name))
        .unwrap()
        .object()
        .as_expr(cx)
        .unwrap()
}

fn new_server(address: ServerAddress, wall_start: u64) -> ExpressionTreeServer {
    ExpressionTreeServer::new(
        address,
        vec![Symbol::qualified("codec", "lisp")],
        Arc::new(DeterministicWallClock::new(wall_start, 1)),
        Default::default(),
    )
    .unwrap()
}

fn request(expr: Expr) -> EvalRequest {
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

fn realize_expr(server: &ExpressionTreeServer, cx: &mut Cx, expr: Expr) -> Expr {
    server
        .realize(cx, request(expr))
        .unwrap()
        .value
        .object()
        .as_expr(cx)
        .unwrap()
}

fn runtime_call(session: &SessionId, name: &str, args: Vec<Expr>) -> Expr {
    let mut all = vec![Expr::Symbol(session.resource())];
    all.extend(args);
    Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::qualified("expr-tree", name))),
        args: all,
    }
}

fn view_registry() -> LensRegistry {
    let mut registry = LensRegistry::new();
    register_expression_tree_surface_codec(&mut registry);
    registry
}

fn connect(cx: &mut Cx, address: &ServerAddress) -> RemoteTransport {
    let mut transport = RemoteTransport::local_server_address(
        format!("in-process:{ADDRESS_THREAD}"),
        address.clone(),
    )
    .with_offered_codecs(vec![Symbol::qualified("codec", "lisp")]);
    transport.connect(cx).unwrap();
    transport
}

fn edit_source(session: &SessionId, revision: u64, path: &str, source: &str) -> Expr {
    intent(
        "edit-field",
        Origin::human(revision),
        vec![
            ("target", target(session, revision, path)),
            ("path", Expr::List(vec![Expr::String("source".to_owned())])),
            ("value", Expr::String(source.to_owned())),
        ],
    )
}

fn target(session: &SessionId, revision: u64, path: &str) -> Expr {
    Expr::Map(vec![
        (
            Expr::Symbol(Symbol::new("tree")),
            Expr::Symbol(session.resource()),
        ),
        (
            Expr::Symbol(Symbol::new("revision")),
            Expr::Number(NumberLiteral {
                domain: Symbol::new("u64"),
                canonical: revision.to_string(),
            }),
        ),
        (
            Expr::Symbol(Symbol::new("path")),
            Expr::String(path.to_owned()),
        ),
    ])
}

fn error_code(expr: &Expr) -> Option<String> {
    let Expr::Map(entries) = expr else {
        return None;
    };
    entries.iter().find_map(|(key, value)| match (key, value) {
        (Expr::Symbol(key), Expr::Symbol(value)) if key.as_qualified_str() == "error" => {
            Some(value.name.to_string())
        }
        _ => None,
    })
}
