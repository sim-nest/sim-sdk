use sim::kernel::Symbol;
use sim::lib_lang_javascript::{Completion, JavascriptState, JavascriptValue};
use sim::lib_lang_typescript::{
    AnnotationMetadata, AnnotationProvenance, TypeScriptNotation, TypeScriptProgram,
    project_annotation, typescript_gap_manifest, typescript_notation_profile,
};

pub fn admitted_typescript_notation() -> Result<(), Box<dyn std::error::Error>> {
    let source = "let answer: number = 40 + 2; answer;";
    let tree = sim::codec_typescript::parse_module(source)?;
    let lowered = sim::codec_typescript::lower_typescript(&tree)?;
    let type_start = source.find("number").expect("fixture annotation");
    let annotations = lowered
        .annotations
        .iter()
        .map(|annotation| {
            AnnotationMetadata {
                provenance: AnnotationProvenance {
                    source: "number".to_owned(),
                    span: type_start..type_start + "number".len(),
                    context: annotation.context.clone(),
                    origins: vec!["typescript".to_owned(), "javascript".to_owned()],
                },
                projected: project_annotation("number"),
            }
        })
        .collect::<Vec<_>>();
    let program = TypeScriptProgram {
        javascript: lowered.javascript,
        annotations,
    };

    let result = TypeScriptNotation::new(128)?
        .eval(&program, &mut JavascriptState::default())?;
    assert_eq!(result, Completion::Normal(JavascriptValue::Number(42.0)));

    let projected = program.annotations[0]
        .projected
        .as_ref()
        .expect("number has faithful browse metadata");
    let _browsable_shape = projected.shape_ref(Symbol::qualified("typescript", "answer"));
    assert_eq!(
        typescript_notation_profile().symbol,
        Symbol::qualified("language", "typescript-notation")
    );

    assert!(project_annotation("T extends U ? X : Y").is_none());
    assert!(typescript_gap_manifest().contains(&"inference"));
    assert!(typescript_gap_manifest().contains(&"compiler-diagnostics"));
    Ok(())
}
