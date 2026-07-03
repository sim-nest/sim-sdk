use sim_kernel::Symbol;

use super::AuthoredHelp;

type HelpTuple = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const SHAPE_ALGEBRA_SEE_ALSO: &[(&str, &str)] = &[
    ("shape", "and"),
    ("shape", "or"),
    ("shape", "not"),
    ("shape", "table"),
    ("shape", "repeat"),
];

const SHAPE_COMPARE_SEE_ALSO: &[(&str, &str)] = &[
    ("shape", "compare"),
    ("shape", "compare-with"),
    ("shape", "venn"),
];

const SHAPE_HOOK_SEE_ALSO: &[(&str, &str)] = &[
    ("shape", "hook"),
    ("shape", "hook-trace"),
    ("shape", "hook-score-floor"),
];

const SHAPE_FUNCTIONS: &[HelpTuple] = &[
    (
        "shape",
        "and",
        "builds a shape accepted by every child shape",
        "shape:and takes a list of shapes and returns an AndShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "all",
        "alias for building an all-of shape",
        "shape:all is the shape:and alias for an AndShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "or",
        "builds a shape accepted by any child shape",
        "shape:or takes a list of shapes and returns an OrShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "any",
        "alias for building an any-of shape",
        "shape:any is the shape:or alias for an OrShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "not",
        "builds a shape that accepts when another shape rejects",
        "shape:not wraps one shape in a NotShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "none",
        "alias for building a negated shape",
        "shape:none is the shape:not alias for a NotShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "without",
        "builds a shape accepted by the left shape but not the right",
        "shape:without combines AndShape and NotShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "list",
        "builds a fixed-prefix list shape",
        "shape:list takes a list of item shapes and returns a ListShape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "list-rest",
        "builds a variadic list shape",
        "shape:list-rest takes prefix shapes and a rest shape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "table",
        "builds a table shape with one required field",
        "shape:table takes a key and shape for a required field.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "table-required",
        "builds an open table shape from required fields",
        "shape:table-required accepts field specs and allows extra keys.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "table-open",
        "builds an open table shape",
        "shape:table-open checks listed fields and allows extra keys.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "table-closed",
        "builds a closed table shape",
        "shape:table-closed rejects keys not listed in its field specs.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "repeat",
        "builds an unbounded repeated-item shape",
        "shape:repeat checks every list item with the body shape.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "repeat-bounds",
        "builds a repeated-item shape with count bounds",
        "shape:repeat-bounds checks every item and enforces min and max.",
        SHAPE_ALGEBRA_SEE_ALSO,
    ),
    (
        "shape",
        "compare",
        "compares two shapes conservatively",
        "shape:compare returns a relation table without probes.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "compare-with",
        "compares two shapes with explicit probes",
        "shape:compare-with returns a relation table with witness data.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "venn",
        "builds a named set of shapes for Venn regions",
        "shape:venn returns a VennShapeSet runtime value.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "venn-union",
        "builds the union region of a Venn shape set",
        "shape:venn-union returns a shape accepted by any member.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "venn-intersection",
        "builds the intersection region of a Venn shape set",
        "shape:venn-intersection returns a shape accepted by every member.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "venn-only",
        "builds a region accepted only by one Venn member",
        "shape:venn-only excludes all other members.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "venn-outside",
        "builds the outside region of a Venn shape set",
        "shape:venn-outside rejects the union of all members.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "venn-exactly",
        "builds a region accepted by exactly selected Venn members",
        "shape:venn-exactly includes selected names and excludes others.",
        SHAPE_COMPARE_SEE_ALSO,
    ),
    (
        "shape",
        "hook",
        "wraps a shape with match hooks",
        "shape:hook takes an inner shape and a list of hook values.",
        SHAPE_HOOK_SEE_ALSO,
    ),
    (
        "shape",
        "hook-trace",
        "returns a mark hook that traces shape matching",
        "shape:hook-trace creates a TraceMarkHook value.",
        SHAPE_HOOK_SEE_ALSO,
    ),
    (
        "shape",
        "hook-score-floor",
        "returns an annotate hook that raises accepted scores",
        "shape:hook-score-floor creates a ScoreFloorHook value.",
        SHAPE_HOOK_SEE_ALSO,
    ),
    (
        "shape",
        "hook-accept-on-no-diagnostics",
        "returns an accept hook for quiet rejections",
        "shape:hook-accept-on-no-diagnostics creates an accept hook value.",
        SHAPE_HOOK_SEE_ALSO,
    ),
    (
        "shape",
        "hook-discard-on-diagnostic-prefix",
        "returns a discard hook keyed by diagnostic prefix",
        "shape:hook-discard-on-diagnostic-prefix creates a discard hook value.",
        SHAPE_HOOK_SEE_ALSO,
    ),
];

pub(super) fn authored_shape_help(subject: &Symbol) -> Option<AuthoredHelp> {
    SHAPE_FUNCTIONS
        .iter()
        .find(|(namespace, name, _, _, _)| {
            subject.namespace.as_deref() == Some(*namespace) && subject.name.as_ref() == *name
        })
        .map(|(_, _, summary, detail, see_also)| AuthoredHelp {
            kind: "function",
            summary,
            detail,
            see_also,
        })
}
