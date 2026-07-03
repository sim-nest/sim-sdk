use std::sync::Arc;

use sim_kernel::{Linker, Result, Symbol};
use sim_shape::{
    AnyShape, ExprKind, ExprKindShape, FieldShape, FieldSpec, NumberValueShape, OneOfShape,
};

use crate::{
    runtime::browse::schema::{
        BROWSE_TEST_FIELDS, CARD_V2_FIELDS, COVERAGE_FIELDS, FACET_FIELDS, HELP_FIELDS,
        REDACTION_FIELDS, TEST_REPORT_FIELDS,
    },
    shapes::{DocumentedShape as RuntimeDocumentedShape, shape_value},
};

use super::register::CoreBuildCx;

type BrowseSchemaShape = (
    Symbol,
    &'static str,
    &'static [&'static str],
    Arc<dyn sim_shape::Shape>,
);

pub(super) fn register_core_shapes(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
) -> Result<()> {
    link_shape(
        cx,
        linker,
        Symbol::qualified("core", "Any"),
        Arc::new(AnyShape),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("core", "Number"),
        Arc::new(RuntimeDocumentedShape::new(
            "Number",
            [
                "accepts any value that participates in numeric dispatch",
                "literal Expr::Number syntax remains the expression-only number shape",
            ],
            Arc::new(NumberValueShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("core", "Expr"),
        Arc::new(RuntimeDocumentedShape::new(
            "Expr",
            [
                "canonical shared expression graph",
                "accepts every current Expr variant",
            ],
            Arc::new(OneOfShape::new(vec![
                Arc::new(ExprKindShape::new(ExprKind::Nil)),
                Arc::new(ExprKindShape::new(ExprKind::Bool)),
                Arc::new(ExprKindShape::new(ExprKind::Number)),
                Arc::new(ExprKindShape::new(ExprKind::Symbol)),
                Arc::new(ExprKindShape::new(ExprKind::String)),
                Arc::new(ExprKindShape::new(ExprKind::Bytes)),
                Arc::new(ExprKindShape::new(ExprKind::List)),
                Arc::new(ExprKindShape::new(ExprKind::Vector)),
                Arc::new(ExprKindShape::new(ExprKind::Map)),
                Arc::new(ExprKindShape::new(ExprKind::Set)),
                Arc::new(ExprKindShape::new(ExprKind::Call)),
                Arc::new(ExprKindShape::new(ExprKind::Infix)),
                Arc::new(ExprKindShape::new(ExprKind::Prefix)),
                Arc::new(ExprKindShape::new(ExprKind::Postfix)),
                Arc::new(ExprKindShape::new(ExprKind::Block)),
                Arc::new(ExprKindShape::new(ExprKind::Quote)),
                Arc::new(ExprKindShape::new(ExprKind::Annotated)),
                Arc::new(ExprKindShape::new(ExprKind::Extension)),
            ])),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("core", "EncodeOptions"),
        Arc::new(RuntimeDocumentedShape::new(
            "EncodeOptions",
            [
                "position: eval|quote|data|pattern",
                "canonical: canonical|preserve-input",
                "lossless-origin: retain origin/trivia when supported",
                "read-construct: allow|forbid constructor surface forms",
                "read-eval: forbid|allow-broad read-eval surface forms",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("core", "MacroSyntax"),
        Arc::new(RuntimeDocumentedShape::new(
            "MacroSyntax",
            [
                "macro syntax is checked by a Shape before expansion",
                "captures are passed to the macro as Expr bindings",
                "expansion phase is controlled by the active EvalPolicy",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    register_codec_shapes(cx, linker)?;
    register_browse_schema_shapes(cx, linker)?;
    Ok(())
}

fn register_codec_shapes(cx: &mut impl CoreBuildCx, linker: &mut Linker<'_>) -> Result<()> {
    link_shape(
        cx,
        linker,
        Symbol::qualified("codec", "LispSurface"),
        Arc::new(RuntimeDocumentedShape::new(
            "LispSurface",
            [
                "lists, vectors, blocks, quotes, symbols, literals",
                "#(...) uses read-construct capability and decodes constructor args as data",
                "#eval(...) and #. use read-eval capability",
                "lossless origin/trivia is best-effort and not a round-trip contract",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("codec", "JsonTaggedExpr"),
        Arc::new(RuntimeDocumentedShape::new(
            "JsonTaggedExpr",
            [
                "canonical tagged JSON object with $expr discriminator",
                "encodes the full shared Expr graph",
                "can carry LocatedExpr origin metadata through helper APIs",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("codec", "BinaryFrame"),
        Arc::new(RuntimeDocumentedShape::new(
            "BinaryFrame",
            [
                "SLB8 magic, versioned canonical frame",
                "interned symbol and number-domain tables",
                "optional origin payload can ride in the frame flags",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("codec", "BinaryBase64Text"),
        Arc::new(RuntimeDocumentedShape::new(
            "BinaryBase64Text",
            [
                "standard padded Base64 text for an SLB8 binary frame",
                "unwrapped ASCII output, optional ASCII whitespace on decode",
                "semantics are identical to codec:binary after decoding",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("codec", "ChatTranscript"),
        Arc::new(RuntimeDocumentedShape::new(
            "ChatTranscript",
            [
                "provider-neutral model transcript map",
                "top-level marker is model-request, model-response, model-event, or model-card",
                "nested payloads remain ordinary Expr values",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    link_shape(
        cx,
        linker,
        Symbol::qualified("codec", "AlgolSurface"),
        Arc::new(RuntimeDocumentedShape::new(
            "AlgolSurface",
            [
                "ordinary arithmetic and call-shaped surface",
                "unsupported forms escape through expr.lisp(...)",
                "origin/trivia is not preserved by the surface codec",
            ],
            Arc::new(AnyShape),
        )),
    )?;
    Ok(())
}

fn register_browse_schema_shapes(cx: &mut impl CoreBuildCx, linker: &mut Linker<'_>) -> Result<()> {
    let shapes: Vec<BrowseSchemaShape> = vec![
        (
            Symbol::qualified("core", "Card"),
            "Card",
            &[
                "browse Card data with the fixed fields first",
                "B6 appends facets, coverage, provenance, and freshness",
            ][..],
            browse_fields(CARD_V2_FIELDS),
        ),
        (
            Symbol::qualified("browse", "Help"),
            "Help",
            &[
                "fixed-field help document for a browsable subject",
                "missing optional data is represented by nil or an empty list",
            ][..],
            browse_fields(HELP_FIELDS),
        ),
        (
            Symbol::qualified("browse", "Test"),
            "Test",
            &[
                "fixed-field executable test or worked example",
                "browse describes tests without running them",
            ][..],
            browse_fields(BROWSE_TEST_FIELDS),
        ),
        (
            Symbol::qualified("browse", "Coverage"),
            "Coverage",
            &[
                "test and example coverage summary",
                "run result fields stay nil until a visible run exists",
            ][..],
            browse_fields(COVERAGE_FIELDS),
        ),
        (
            Symbol::qualified("browse", "Facet"),
            "Facet",
            &[
                "typed extension payload attached to a Card",
                "hidden facet values remain present as redactions",
            ][..],
            browse_fields(FACET_FIELDS),
        ),
        (
            Symbol::qualified("browse", "Redaction"),
            "Redaction",
            &[
                "placeholder for data that exists but is not visible",
                "capability requirements are data, not display text",
            ][..],
            browse_fields(REDACTION_FIELDS),
        ),
        (
            Symbol::qualified("browse", "TestReport"),
            "TestReport",
            &[
                "visible result of an explicit test run",
                "shape-mode reports may carry a ShapeReport value",
            ][..],
            browse_fields(TEST_REPORT_FIELDS),
        ),
    ];

    for (symbol, name, details, shape) in shapes {
        link_shape(
            cx,
            linker,
            symbol,
            Arc::new(RuntimeDocumentedShape::new(
                name,
                details.iter().copied(),
                shape,
            )),
        )?;
    }
    Ok(())
}

fn browse_fields(fields: &[&str]) -> Arc<dyn sim_shape::Shape> {
    Arc::new(FieldShape::anonymous(
        fields
            .iter()
            .map(|field| FieldSpec::required(Symbol::new(*field), Arc::new(AnyShape)))
            .collect(),
    ))
}

fn link_shape(
    cx: &mut impl CoreBuildCx,
    linker: &mut Linker<'_>,
    symbol: Symbol,
    shape: Arc<dyn sim_shape::Shape>,
) -> Result<()> {
    linker.shape_value(symbol.clone(), shape_value(symbol, shape))?;
    let _ = cx;
    Ok(())
}
