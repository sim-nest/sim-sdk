#[cfg(feature = "numbers-prelude")]
use std::collections::{BTreeSet, VecDeque};

#[cfg(feature = "numbers-prelude")]
use sim_kernel::Symbol;

#[cfg(feature = "numbers-prelude")]
use crate::{numbers_core::domains, numbers_prelude::NumbersPreludeLib};

#[cfg(feature = "numbers-prelude")]
use super::support::eval_cx;

#[cfg(feature = "numbers-prelude")]
#[test]
fn number_promotion_lattice_reaches_default_scalar_sinks() {
    let mut cx = eval_cx();
    NumbersPreludeLib::new().install_all(&mut cx).unwrap();

    let literal_edges = cx
        .registry()
        .promotion_rules()
        .iter()
        .map(|rule| (rule.from_domain.clone(), rule.to_domain.clone()))
        .collect::<BTreeSet<_>>();
    let value_edges = cx
        .registry()
        .value_promotion_rules()
        .iter()
        .map(|rule| (rule.from_domain.clone(), rule.to_domain.clone()))
        .collect::<BTreeSet<_>>();
    for edge in &literal_edges {
        assert!(
            value_edges.contains(edge),
            "literal promotion edge {}/{} has no value-promotion mirror",
            edge.0,
            edge.1
        );
    }

    let mut graph = literal_edges;
    graph.extend(value_edges);
    for integer in domains::integer_domains() {
        assert_reaches(&graph, integer.clone(), domains::f64());
        assert_reaches(&graph, integer, domains::rational());
    }
    for scalar in [
        domains::bool(),
        domains::f32(),
        domains::f64(),
        domains::i64(),
        domains::bigint(),
        domains::rational(),
    ] {
        assert_reaches(&graph, scalar.clone(), domains::complex());
        assert_reaches(&graph, scalar, domains::cas());
    }
    assert_reaches(&graph, domains::complex(), domains::cas());
    let scalar_domains = [
        domains::bool(),
        domains::f32(),
        domains::f64(),
        domains::i64(),
        domains::bigint(),
        domains::rational(),
    ];
    assert!(
        graph
            .iter()
            .filter(|(from, _)| from == &domains::complex())
            .all(|(_, to)| !scalar_domains.contains(to)),
        "complex should not promote back into narrower scalar domains"
    );
}

#[cfg(feature = "numbers-prelude")]
fn assert_reaches(graph: &BTreeSet<(Symbol, Symbol)>, from: Symbol, to: Symbol) {
    assert!(
        reaches(graph, &from, &to),
        "promotion lattice should connect {from} -> {to}"
    );
}

#[cfg(feature = "numbers-prelude")]
fn reaches(graph: &BTreeSet<(Symbol, Symbol)>, from: &Symbol, to: &Symbol) -> bool {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([from.clone()]);
    while let Some(domain) = queue.pop_front() {
        if &domain == to {
            return true;
        }
        if !seen.insert(domain.clone()) {
            continue;
        }
        for (edge_from, edge_to) in graph {
            if edge_from == &domain {
                queue.push_back(edge_to.clone());
            }
        }
    }
    false
}
