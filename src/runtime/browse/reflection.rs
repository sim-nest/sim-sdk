use std::sync::Arc;

use sim_kernel::{Cx, Expr, QuoteMode, Result, Symbol};

use crate::runtime::{SimTest, TestExpected};

#[path = "reflection/shape_help.rs"]
mod shape_help;

#[derive(Clone, Copy)]
pub(crate) struct AuthoredHelp {
    pub(crate) kind: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) detail: &'static str,
    see_also: &'static [(&'static str, &'static str)],
}

impl AuthoredHelp {
    pub(crate) fn see_also_symbols(self) -> Vec<Symbol> {
        self.see_also
            .iter()
            .map(|(namespace, name)| Symbol::qualified(*namespace, *name))
            .collect()
    }
}

struct ReflectionSubject {
    namespace: &'static str,
    name: &'static str,
    example: Option<&'static str>,
    help: AuthoredHelp,
}

const CARD_SEE_ALSO: &[(&str, &str)] = &[
    ("browse", "Help"),
    ("browse", "Test"),
    ("browse", "Coverage"),
    ("browse", "Facet"),
    ("browse", "Redaction"),
    ("browse", "TestReport"),
];

const FACET_SEE_ALSO: &[(&str, &str)] = &[("browse", "Redaction"), ("core", "Card")];

const TEST_SEE_ALSO: &[(&str, &str)] = &[("browse", "Coverage"), ("browse", "TestReport")];

const BROWSE_SEE_ALSO: &[(&str, &str)] = &[
    ("core", "help"),
    ("core", "args"),
    ("core", "result"),
    ("core", "tests"),
    ("core", "examples"),
    ("core", "coverage"),
    ("core", "facets"),
    ("core", "help-object"),
    ("core", "browse-neighbors"),
    ("core", "browse-path"),
];

const SCHEMA_SUBJECTS: &[ReflectionSubject] = &[
    ReflectionSubject {
        namespace: "core",
        name: "Card",
        example: Some("core-card"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "Card fixed-field browse envelope",
            detail: "Card keeps the fixed fields first and appends facets, coverage, provenance, and freshness so agents can inspect one stable table.",
            see_also: CARD_SEE_ALSO,
        },
    },
    ReflectionSubject {
        namespace: "browse",
        name: "Help",
        example: Some("browse-help"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "fixed-field help document embedded in a Card",
            detail: "Help values describe one subject with summary text, exported-by metadata, stability, capabilities, demand, and see-also refs.",
            see_also: &[("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "browse",
        name: "Test",
        example: Some("browse-test"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "fixed-field executable test or worked example",
            detail: "Test values describe an expression, expected result mode, codecs, subjects, capabilities, and whether the test is an example.",
            see_also: TEST_SEE_ALSO,
        },
    },
    ReflectionSubject {
        namespace: "browse",
        name: "Coverage",
        example: Some("browse-coverage"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "test and example coverage summary for one Card",
            detail: "Coverage counts visible tests, examples, runnable tests, latest pass/fail/skip totals, last run ref, and stale state.",
            see_also: TEST_SEE_ALSO,
        },
    },
    ReflectionSubject {
        namespace: "browse",
        name: "Facet",
        example: Some("browse-facet"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "typed extension payload attached to a Card",
            detail: "Facet values name a payload, version, kind, shape, visibility, requirements, and evidence while preserving redacted payload presence.",
            see_also: FACET_SEE_ALSO,
        },
    },
    ReflectionSubject {
        namespace: "browse",
        name: "Redaction",
        example: Some("browse-redaction"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "placeholder for hidden but known browse data",
            detail: "Redaction values state why payload data is hidden, which capabilities reveal it, and a stable summary safe for public Cards.",
            see_also: FACET_SEE_ALSO,
        },
    },
    ReflectionSubject {
        namespace: "browse",
        name: "TestReport",
        example: Some("browse-test-report"),
        help: AuthoredHelp {
            kind: "shape",
            summary: "fixed-field result of an explicit test run",
            detail: "TestReport values expose pass state, mode, detail, event refs, effect ref, and optional shape-report data.",
            see_also: TEST_SEE_ALSO,
        },
    },
];

const BROWSE_FUNCTIONS: &[ReflectionSubject] = &[
    ReflectionSubject {
        namespace: "core",
        name: "browse",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "returns a Card for a subject or the root browse catalog",
            detail: "core/browse returns the root catalog Card with no arguments. With a subject ref, symbol, or string, it returns the Card for that subject.",
            see_also: BROWSE_SEE_ALSO,
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "help",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "returns and publishes authored Help for one subject",
            detail: "core/help resolves a subject symbol or string, builds a fixed browse/Help value, publishes it as a help-doc claim, and returns a compatibility Card.",
            see_also: &[("browse", "Help"), ("core", "help-object")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "args",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects one stable Card field for a subject",
            detail: "core/args returns the args field from the subject Card, normally the argument shape ref.",
            see_also: &[("core", "browse"), ("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "result",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects one stable Card field for a subject",
            detail: "core/result returns the result field from the subject Card, normally the result shape ref.",
            see_also: &[("core", "browse"), ("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "tests",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects one stable Card field for a subject",
            detail: "core/tests with one subject returns the subject tests field. With no arguments it lists all registered tests.",
            see_also: &[("core", "browse"), ("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "examples",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects example browse/Test values for one subject",
            detail: "core/examples reads the subject Card tests field and returns only tests marked with example true.",
            see_also: &[("browse", "Test"), ("core", "tests")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "coverage",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects one stable Card field for a subject",
            detail: "core/coverage returns the coverage field from the subject Card as a browse/Coverage value.",
            see_also: &[("core", "browse"), ("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "facets",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects one stable Card field for a subject",
            detail: "core/facets returns the facets field from the subject Card as a list of browse/Facet values.",
            see_also: &[("core", "browse"), ("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "help-object",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "projects one stable Card field for a subject",
            detail: "core/help-object returns the help field from the subject Card as a browse/Help value when authored help is available.",
            see_also: &[("core", "browse"), ("core", "Card")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "browse-neighbors",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "returns visible graph refs directly reachable from one Card",
            detail: "core/browse-neighbors returns refs extracted from a subject Card, including args, result, tests, facets, coverage, provenance, Help links, and catalog entries.",
            see_also: &[("core", "browse"), ("core", "browse-path")],
        },
    },
    ReflectionSubject {
        namespace: "core",
        name: "browse-path",
        example: None,
        help: AuthoredHelp {
            kind: "function",
            summary: "returns the shortest visible browse graph path between refs",
            detail: "core/browse-path walks visible browse-neighbors edges from a start ref to a target ref and returns a list of refs, or nil when no path is visible.",
            see_also: &[("core", "browse"), ("core", "browse-neighbors")],
        },
    },
];

pub(crate) fn authored_help(subject: &Symbol) -> Option<AuthoredHelp> {
    SCHEMA_SUBJECTS
        .iter()
        .chain(BROWSE_FUNCTIONS.iter())
        .find(|item| item.matches(subject))
        .map(|item| item.help)
        .or_else(|| shape_help::authored_shape_help(subject))
}

pub(crate) fn install_schema_examples(cx: &mut Cx) -> Result<()> {
    for item in SCHEMA_SUBJECTS {
        let Some(test_name) = item.example else {
            continue;
        };
        let name = Symbol::qualified("browse-example", test_name);
        if cx.registry().registered_test(&name).is_some() {
            continue;
        }

        let subject = item.symbol();
        let subjects = vec![subject.clone()];
        let test = SimTest::new(
            name.clone(),
            Symbol::new("core"),
            browse_call_expr(subject),
            TestExpected::Truthy,
            subjects.clone(),
        )
        .as_example();
        cx.registry_mut()
            .register_test(name, Symbol::new("core"), Arc::new(test), subjects)?;
    }
    Ok(())
}

impl ReflectionSubject {
    fn symbol(&self) -> Symbol {
        Symbol::qualified(self.namespace, self.name)
    }

    fn matches(&self, subject: &Symbol) -> bool {
        subject == &self.symbol()
    }
}

fn browse_call_expr(subject: Symbol) -> Expr {
    Expr::Call {
        operator: Box::new(Expr::Symbol(Symbol::qualified("core", "browse"))),
        args: vec![Expr::Quote {
            mode: QuoteMode::Quote,
            expr: Box::new(Expr::Symbol(subject)),
        }],
    }
}
