use std::{collections::BTreeSet, sync::Arc};

use sim_kernel::{
    Args, Cx, DefaultFactory, Expr, NoopEvalPolicy, Symbol, Value, browse_internal_capability,
};

use crate::runtime::install_core_runtime;

use super::support::table_value;

#[test]
fn root_catalog_lists_b6_surface_cards() {
    let mut cx = test_cx();
    let neighbors = call(
        &mut cx,
        Symbol::qualified("core", "browse-neighbors"),
        Vec::new(),
    );
    let symbols = symbol_set(&expr(&mut cx, &neighbors));

    for symbol in [
        Symbol::qualified("server", "server"),
        Symbol::qualified("agent", "tool-surface"),
        Symbol::qualified("stream", "stream"),
        Symbol::qualified("stream", "data"),
        Symbol::qualified("stream", "queue-policy"),
        Symbol::qualified("stream/data", "model-event"),
        Symbol::qualified("stream/data", "rank-frontier"),
        Symbol::qualified("rank", "space"),
        Symbol::qualified("standard", "profile"),
    ] {
        assert!(symbols.contains(&symbol), "root should link {symbol}");
    }

    assert_card_kind(
        &mut cx,
        Symbol::qualified("server", "server"),
        Symbol::qualified("server", "server"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("agent", "model-runner"),
        Symbol::qualified("agent", "model-runner"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("stream", "stream"),
        Symbol::qualified("core", "stream"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("stream", "data"),
        Symbol::qualified("stream", "data"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("stream", "queue-policy"),
        Symbol::qualified("stream", "queue-policy"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("stream/data", "model-event"),
        Symbol::qualified("stream", "model-event"),
    );
    assert_card_kind(
        &mut cx,
        Symbol::qualified("stream/data", "rank-frontier"),
        Symbol::qualified("stream", "rank-frontier"),
    );
}

#[test]
fn server_metrics_facet_redacts_host_details_without_internal_capability() {
    let mut cx = test_cx();
    let card = browse_symbol(&mut cx, Symbol::qualified("server", "server"));
    let card = expr(&mut cx, &card);
    let metrics = facet_by_name(&card, Symbol::qualified("server", "metrics"));

    assert_eq!(
        table_value(metrics, &field("visibility")),
        Some(&Expr::Symbol(Symbol::new("private")))
    );
    assert_list_contains_symbol(
        table_value(metrics, &field("requires")).expect("requires"),
        Symbol::qualified("capability", "browse.internal"),
    );
    let redaction = table_value(metrics, &field("value")).expect("redaction");
    assert_eq!(
        table_value(redaction, &field("reason")),
        Some(&Expr::Symbol(Symbol::new("capability-required")))
    );

    cx.grant(browse_internal_capability());
    let card = browse_symbol(&mut cx, Symbol::qualified("server", "server"));
    let card = expr(&mut cx, &card);
    let metrics = facet_by_name(&card, Symbol::qualified("server", "metrics"));
    let payload = table_value(metrics, &field("value")).expect("metrics payload");

    assert!(table_value(payload, &field("messages-sent")).is_some());
    assert!(table_value(payload, &field("host-addresses")).is_some());
}

#[test]
fn server_and_agent_facets_publish_stream_compatibility() {
    let mut cx = test_cx();

    let server = browse_symbol(&mut cx, Symbol::qualified("server", "server"));
    let server = expr(&mut cx, &server);
    let transport = facet_by_name(&server, Symbol::qualified("server", "transport"));
    let transport_value = table_value(transport, &field("value")).expect("transport payload");
    assert_list_contains_symbol(
        table_value(transport_value, &field("stream-frame-kinds")).expect("stream frames"),
        Symbol::qualified("server", "stream-chunk"),
    );
    assert_list_contains_symbol(
        table_value(transport_value, &field("data-stream-kinds")).expect("data stream kinds"),
        Symbol::qualified("stream/data", "model-event"),
    );
    assert_list_contains_symbol(
        table_value(transport_value, &field("data-stream-kinds")).expect("data stream kinds"),
        Symbol::qualified("stream/data", "rank-frontier"),
    );

    let agent = browse_symbol(&mut cx, Symbol::qualified("agent", "model-runner"));
    let agent = expr(&mut cx, &agent);
    let runner = facet_by_name(&agent, Symbol::qualified("agent", "model-runner"));
    let runner_value = table_value(runner, &field("value")).expect("runner payload");
    assert_list_contains_symbol(
        table_value(runner_value, &field("stream-data-kinds")).expect("stream data kinds"),
        Symbol::qualified("stream/data", "model-event"),
    );
}

#[cfg(feature = "codec-lisp")]
#[test]
fn codec_card_publishes_positions_extensions_and_roundtrip_facets() {
    let mut cx = test_cx();
    let codec_id = cx.registry_mut().fresh_codec_id();
    let lib = crate::codec_lisp::LispCodecLib::new(codec_id).unwrap();
    cx.load_lib(&lib).unwrap();

    let card = browse_symbol(&mut cx, Symbol::qualified("codec", "lisp"));
    let card = expr(&mut cx, &card);
    assert_eq!(
        table_value(&card, &field("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "codec")))
    );

    let positions = facet_by_name(&card, Symbol::qualified("codec", "positions"));
    let positions_value = table_value(positions, &field("value")).expect("positions payload");
    assert_eq!(
        table_value(positions_value, &field("default-decode")),
        Some(&Expr::String("term-in-eval-datum-otherwise".to_owned()))
    );
    assert_position_target(
        table_value(positions_value, &field("positions")).expect("position rows"),
        Symbol::new("eval"),
        Symbol::qualified("core", "Term"),
    );

    let extensions = facet_by_name(&card, Symbol::qualified("codec", "extensions"));
    let extensions_value = table_value(extensions, &field("value")).expect("extensions payload");
    assert_list_contains_string(
        table_value(extensions_value, &field("extensions")).expect("extensions"),
        ".siml",
    );
    assert!(find_facet(&card, Symbol::qualified("codec", "roundtrip-examples")).is_some());
}

#[cfg(feature = "numbers-f64")]
#[test]
fn number_domain_card_publishes_shape_and_dispatch_facets() {
    let mut cx = test_cx();
    let card = browse_symbol(&mut cx, Symbol::qualified("numbers", "f64"));
    let card = expr(&mut cx, &card);
    assert_eq!(
        table_value(&card, &field("kind")),
        Some(&Expr::Symbol(Symbol::qualified("core", "number-domain")))
    );

    let domain = facet_by_name(&card, Symbol::qualified("numbers", "domain"));
    let domain_value = table_value(domain, &field("value")).expect("domain payload");
    assert!(table_value(domain_value, &field("value-shape")).is_some());

    let dispatch = facet_by_name(&card, Symbol::qualified("numbers", "dispatch"));
    let dispatch_value = table_value(dispatch, &field("value")).expect("dispatch payload");
    let rows = list_items(dispatch_value);
    assert!(rows.iter().any(|row| {
        table_value(row, &field("operator"))
            == Some(&Expr::Symbol(Symbol::qualified("math", "add")))
    }));
}

#[test]
fn stream_rank_and_standard_cards_publish_k6_facets() {
    let mut cx = test_cx();

    for (subject, kind, facet) in [
        (
            Symbol::qualified("stream", "stream"),
            Symbol::qualified("core", "stream"),
            Symbol::qualified("stream", "events"),
        ),
        (
            Symbol::qualified("rank", "space"),
            Symbol::qualified("rank", "space"),
            Symbol::qualified("rank", "space"),
        ),
        (
            Symbol::qualified("standard", "profile"),
            Symbol::qualified("standard", "profile"),
            Symbol::qualified("standard", "fidelity"),
        ),
    ] {
        let card = browse_symbol(&mut cx, subject.clone());
        let card = expr(&mut cx, &card);
        assert_eq!(
            table_value(&card, &field("kind")),
            Some(&Expr::Symbol(kind)),
            "{subject} kind"
        );
        assert!(
            find_facet(&card, facet.clone()).is_some(),
            "{subject} facet {facet}"
        );
    }

    let stream_card = browse_symbol(&mut cx, Symbol::qualified("stream", "stream"));
    let stream = expr(&mut cx, &stream_card);
    let stream_facet = facet_by_name(&stream, Symbol::qualified("stream", "events"));
    let stream_value = table_value(stream_facet, &field("value")).expect("stream facet payload");
    assert_list_contains_symbol(
        table_value(stream_value, &field("metadata-predicates")).expect("metadata predicates"),
        Symbol::qualified("stream", "clock"),
    );
    assert_list_contains_symbol(
        table_value(stream_value, &field("metadata-predicates")).expect("metadata predicates"),
        Symbol::qualified("stream", "codec"),
    );
    assert_list_contains_symbol(
        table_value(stream_value, &field("media")).expect("stream media"),
        Symbol::qualified("stream/media", "data"),
    );

    let data_facet = facet_by_name(&stream, Symbol::qualified("stream", "data"));
    let data_value = table_value(data_facet, &field("value")).expect("stream data payload");
    assert_eq!(
        table_value(data_value, &field("packet-kind")),
        Some(&Expr::Symbol(Symbol::qualified("stream/packet", "data")))
    );
    assert_list_contains_symbol(
        table_value(data_value, &field("data-kinds")).expect("data kinds"),
        Symbol::qualified("stream/data", "model-event"),
    );
    assert_list_contains_symbol(
        table_value(data_value, &field("data-kinds")).expect("data kinds"),
        Symbol::qualified("stream/data", "rank-frontier"),
    );

    let queue_facet = facet_by_name(&stream, Symbol::qualified("stream", "queue-policy"));
    let queue_value = table_value(queue_facet, &field("value")).expect("queue payload");
    assert_eq!(
        table_value(queue_value, &field("bounded")),
        Some(&Expr::Bool(true))
    );
    assert_list_contains_symbol(
        table_value(queue_value, &field("overflow-policies")).expect("overflow policies"),
        Symbol::qualified("stream/overflow", "drop-newest"),
    );
    let queue_card = browse_symbol(&mut cx, Symbol::qualified("stream", "queue-policy"));
    let queue = expr(&mut cx, &queue_card);
    let queue_surface_facet = facet_by_name(&queue, Symbol::qualified("stream", "queue-policy"));
    let queue_surface_value =
        table_value(queue_surface_facet, &field("value")).expect("queue surface payload");
    assert_eq!(
        table_value(queue_surface_value, &field("bounded")),
        Some(&Expr::Bool(true))
    );

    let model_facet = facet_by_name(&stream, Symbol::qualified("stream/data", "model-event"));
    let model_value = table_value(model_facet, &field("value")).expect("model event payload");
    assert_eq!(
        table_value(model_value, &field("data-kind")),
        Some(&Expr::Symbol(Symbol::qualified(
            "stream/data",
            "model-event"
        )))
    );
    assert_eq!(
        table_value(model_value, &field("compatible-surface")),
        Some(&Expr::Symbol(Symbol::qualified("agent", "model-runner")))
    );

    let frontier_facet = facet_by_name(&stream, Symbol::qualified("stream/data", "rank-frontier"));
    let frontier_value =
        table_value(frontier_facet, &field("value")).expect("rank frontier payload");
    assert_eq!(
        table_value(frontier_value, &field("data-kind")),
        Some(&Expr::Symbol(Symbol::qualified(
            "stream/data",
            "rank-frontier"
        )))
    );
    assert_eq!(
        table_value(frontier_value, &field("compatible-surface")),
        Some(&Expr::Symbol(Symbol::qualified("rank", "space")))
    );

    let rank_card = browse_symbol(&mut cx, Symbol::qualified("rank", "space"));
    let rank = expr(&mut cx, &rank_card);
    let rank_facet = facet_by_name(&rank, Symbol::qualified("rank", "space"));
    let rank_value = table_value(rank_facet, &field("value")).expect("rank facet payload");
    assert_list_contains_symbol(
        table_value(rank_value, &field("predicates")).expect("rank predicates"),
        Symbol::qualified("rank", "ordinal"),
    );

    let standard_card = browse_symbol(&mut cx, Symbol::qualified("standard", "profile"));
    let standard = expr(&mut cx, &standard_card);
    let standard_facet = facet_by_name(&standard, Symbol::qualified("standard", "fidelity"));
    let standard_value =
        table_value(standard_facet, &field("value")).expect("standard facet payload");
    assert_list_contains_symbol(
        table_value(standard_value, &field("predicates")).expect("standard predicates"),
        Symbol::qualified("standard", "fidelity"),
    );
}

#[test]
fn root_graph_reaches_unified_stream_data_contracts() {
    let mut cx = test_cx();

    for (target, kind) in [
        (
            Symbol::qualified("stream", "data"),
            Symbol::qualified("stream", "data"),
        ),
        (
            Symbol::qualified("stream/data", "model-event"),
            Symbol::qualified("stream", "model-event"),
        ),
        (
            Symbol::qualified("stream/data", "rank-frontier"),
            Symbol::qualified("stream", "rank-frontier"),
        ),
    ] {
        let root = cx
            .factory()
            .symbol(Symbol::qualified("browse", "catalog"))
            .unwrap();
        let target_value = cx.factory().symbol(target.clone()).unwrap();
        let path = call(
            &mut cx,
            Symbol::qualified("core", "browse-path"),
            vec![root, target_value],
        );
        let path = expr(&mut cx, &path);
        let Expr::List(items) = path else {
            panic!("path to {target} should be visible");
        };
        assert_eq!(items.last(), Some(&Expr::Symbol(target.clone())));
        assert_card_kind(&mut cx, target, kind);
    }
}

fn test_cx() -> Cx {
    let mut cx = Cx::new(Arc::new(NoopEvalPolicy), Arc::new(DefaultFactory));
    install_core_runtime(&mut cx);
    cx
}

fn call(cx: &mut Cx, symbol: Symbol, args: Vec<Value>) -> Value {
    cx.call_function(&symbol, Args::new(args))
        .unwrap_or_else(|err| panic!("{symbol} failed: {err}"))
}

fn browse_symbol(cx: &mut Cx, symbol: Symbol) -> Value {
    let value = cx.factory().symbol(symbol).unwrap();
    call(cx, Symbol::qualified("core", "browse"), vec![value])
}

fn expr(cx: &mut Cx, value: &Value) -> Expr {
    value.object().as_expr(cx).unwrap()
}

fn field(name: &str) -> Symbol {
    Symbol::new(name.to_owned())
}

fn symbol_set(expr: &Expr) -> BTreeSet<Symbol> {
    let Expr::List(items) = expr else {
        panic!("expected list");
    };
    items
        .iter()
        .filter_map(|item| match item {
            Expr::Symbol(symbol) => Some(symbol.clone()),
            _ => None,
        })
        .collect()
}

fn assert_card_kind(cx: &mut Cx, subject: Symbol, expected: Symbol) {
    let card = browse_symbol(cx, subject.clone());
    let card = expr(cx, &card);
    assert_eq!(
        table_value(&card, &field("kind")),
        Some(&Expr::Symbol(expected)),
        "{subject} should browse with the expected kind"
    );
}

fn facet_by_name(card: &Expr, name: Symbol) -> &Expr {
    find_facet(card, name.clone()).unwrap_or_else(|| panic!("missing facet {name}"))
}

fn find_facet(card: &Expr, name: Symbol) -> Option<&Expr> {
    let facets = table_value(card, &field("facets"))?;
    list_items(facets)
        .iter()
        .find(|facet| table_value(facet, &field("name")) == Some(&Expr::Symbol(name.clone())))
}

fn list_items(expr: &Expr) -> &[Expr] {
    match expr {
        Expr::List(items) => items,
        _ => &[],
    }
}

fn assert_list_contains_symbol(expr: &Expr, expected: Symbol) {
    let contains = list_items(expr)
        .iter()
        .any(|item| item == &Expr::Symbol(expected.clone()));
    assert!(contains, "expected list to contain {expected}");
}

fn assert_list_contains_string(expr: &Expr, expected: &str) {
    let contains = list_items(expr)
        .iter()
        .any(|item| item == &Expr::String(expected.to_owned()));
    assert!(contains, "expected list to contain {expected}");
}

fn assert_position_target(expr: &Expr, position: Symbol, target: Symbol) {
    let rows = list_items(expr);
    assert!(rows.iter().any(|row| {
        table_value(row, &field("position")) == Some(&Expr::Symbol(position.clone()))
            && table_value(row, &field("default-target")) == Some(&Expr::Symbol(target.clone()))
    }));
}
