use sim::kernel::{Args, Cx, Expr, Symbol, Value};

use crate::support::{cx, q};

/// Browses real runtime subjects through the public `core/browse` facade and
/// asserts the returned Card records expose the reflection surfaces SIM
/// advertises (subject, kind, help, tests, see-also, coverage, provenance).
///
/// This exercises the browse/reflection runtime and inspects two distinct cards
/// so the check depends on runtime behavior.
#[test]
fn core_browse_reflects_distinct_subjects_into_schema_cards() {
    let mut cx = cx();

    let add_card = browse(&mut cx, q("math", "add"));
    let browse_card = browse(&mut cx, q("core", "browse"));

    // Both are Card maps exposing the reflection surfaces the runtime claims.
    for (label, card) in [("math/add", &add_card), ("core/browse", &browse_card)] {
        assert!(
            matches!(card, Expr::Map(_)),
            "{label} card must be a map, got {card:?}"
        );
        for surface in [
            "subject",
            "kind",
            "help",
            "tests",
            "see-also",
            "coverage",
            "provenance",
        ] {
            assert!(
                card_field(card, surface).is_some(),
                "{label} browse card is missing the `{surface}` surface"
            );
        }
        assert!(
            !matches!(card_field(card, "subject"), Some(Expr::Nil) | None),
            "{label} card subject surface must identify the browsed subject"
        );
    }

    // Reflection actually depends on the browsed subject: distinct subjects yield
    // distinct subject surfaces (this is what makes the check non-tautological).
    assert_ne!(
        card_field(&add_card, "subject"),
        card_field(&browse_card, "subject"),
        "browse returned the same subject surface for two different subjects"
    );
    assert_ne!(
        add_card, browse_card,
        "browse returned identical cards for two different subjects"
    );
}

fn browse(cx: &mut Cx, subject: Symbol) -> Expr {
    let value: Value = cx.factory().symbol(subject.clone()).unwrap();
    let card = cx
        .call_function(&q("core", "browse"), Args::new(vec![value]))
        .unwrap_or_else(|err| panic!("core/browse failed for {subject}: {err:?}"));
    card.object().as_expr(cx).unwrap()
}

fn card_field(card: &Expr, name: &str) -> Option<Expr> {
    let Expr::Map(entries) = card else {
        return None;
    };
    entries.iter().find_map(|(key, value)| match key {
        Expr::Symbol(symbol) if symbol.namespace.is_none() && symbol.name.as_ref() == name => {
            Some(value.clone())
        }
        _ => None,
    })
}
