use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use sim::{
    codec::{Input, decode_with_codec},
    kernel::{Cx, DefaultFactory, EagerPolicy, Expr, QuoteMode, ReadPolicy, Symbol},
};

#[path = "conformance_support/mod.rs"]
mod conformance_support;

const EXPECTED_RECIPE_IDS: [&str; 30] = [
    "a30-001-autonomous-decision",
    "a30-002-planning",
    "a30-003-memory-augmented",
    "a30-004-knowledge-retrieval",
    "a30-005-document-intelligence",
    "a30-006-scientific-research",
    "a30-007-tool-using",
    "a30-008-chain-orchestrator",
    "a30-009-agentic-workflow",
    "a30-010-data-analysis",
    "a30-011-verification-validation",
    "a30-012-general-problem-solver",
    "a30-013-code-generation",
    "a30-014-security-hardened",
    "a30-015-self-improving",
    "a30-016-conversational",
    "a30-017-content-creation",
    "a30-018-recommendation",
    "a30-019-vision-language",
    "a30-020-audio-processing",
    "a30-021-physical-sensing",
    "a30-022-ethical-reasoning",
    "a30-023-explainable",
    "a30-024-healthcare-intelligence",
    "a30-025-scientific-discovery",
    "a30-026-financial-advisory",
    "a30-027-legal-intelligence",
    "a30-028-education-intelligence",
    "a30-029-collective-intelligence",
    "a30-030-embodied-intelligence",
];

const EXPECTED_SOURCE_CHAPTERS: [i64; 30] = [
    5, 5, 5, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 14, 14,
    15, 15, 16,
];

#[derive(Clone, Debug, Default)]
struct RecipeDoc {
    strings: BTreeMap<String, String>,
    ints: BTreeMap<String, i64>,
    arrays: BTreeMap<String, Vec<String>>,
    expect_results: Vec<String>,
}

#[test]
#[ignore = "requires sibling repository recipe corpora and recipe-runtime parity"]
fn agents30_numbered_recipes_are_deterministic_offline_and_metadata_complete() {
    let manifests = collect_agent30_manifests();
    assert!(
        !manifests.is_empty(),
        "no recipes/30-agents recipe.toml files found"
    );

    let mut numbered = BTreeMap::new();
    for manifest_path in manifests {
        let doc = parse_recipe_doc(&manifest_path);
        if let Some(number) = doc.ints.get("recipe_number").copied() {
            let id = required_string(&doc, &manifest_path, "id");
            assert!(
                (1..=30).contains(&number),
                "{}: recipe_number {number} outside 1..=30",
                manifest_path.display()
            );
            assert!(
                numbered.insert(number, (manifest_path, doc)).is_none(),
                "duplicate recipe_number {number} for {id}"
            );
        }
    }

    assert_eq!(
        numbered.len(),
        EXPECTED_RECIPE_IDS.len(),
        "numbered 30-agents recipe count mismatch"
    );

    let mut cx = build_decode_cx();
    for (index, expected_id) in EXPECTED_RECIPE_IDS.iter().enumerate() {
        let number = i64::try_from(index + 1).unwrap();
        let Some((manifest_path, doc)) = numbered.get(&number) else {
            panic!("missing 30-agents recipe number {number}");
        };

        assert_recipe_identity(manifest_path, doc, number, expected_id);
        assert_metadata(manifest_path, doc, number, index);
        assert_no_network_capability(manifest_path, doc);
        assert_no_copied_external_material(manifest_path);
        assert_deterministic_fixture_run(&mut cx, manifest_path, doc);
    }
}

fn collect_agent30_manifests() -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();

    if let Some(packages_root) = find_meta_packages_root() {
        for entry in fs::read_dir(&packages_root)
            .unwrap_or_else(|err| panic!("read {}: {err}", packages_root.display()))
        {
            let entry =
                entry.unwrap_or_else(|err| panic!("read {}: {err}", packages_root.display()));
            let chapter = entry.path().join("recipes/30-agents");
            if chapter.is_dir() {
                collect_recipe_manifests(&chapter, &mut paths);
            }
        }
    } else {
        let projects_root = find_projects_root();
        for entry in fs::read_dir(&projects_root)
            .unwrap_or_else(|err| panic!("read {}: {err}", projects_root.display()))
        {
            let entry =
                entry.unwrap_or_else(|err| panic!("read {}: {err}", projects_root.display()));
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with("sim-") && entry.path().is_dir() {
                collect_agent30_chapters(&entry.path(), &mut paths);
            }
        }
    }

    paths.into_iter().collect()
}

fn find_meta_packages_root() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        let packages = ancestor.join("packages");
        if packages.is_dir() && ancestor.join("Cargo.toml").is_file() {
            return Some(packages);
        }
    }
    None
}

fn find_projects_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest_dir.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "sim-sdk") {
            return ancestor
                .parent()
                .unwrap_or_else(|| panic!("{} has no parent", ancestor.display()))
                .to_path_buf();
        }
    }
    panic!(
        "cannot locate meta-workspace packages or sim-sdk parent from {}",
        manifest_dir.display()
    );
}

fn collect_agent30_chapters(root: &Path, paths: &mut BTreeSet<PathBuf>) {
    let file_name = root.file_name().and_then(|name| name.to_str());
    if matches!(
        file_name,
        Some(
            ".git" | ".meta-workspace" | "target" | "docs" | "generated-reports" | "split-reports"
        )
    ) {
        return;
    }

    if root.ends_with("recipes/30-agents") {
        collect_recipe_manifests(root, paths);
        return;
    }

    for entry in fs::read_dir(root).unwrap_or_else(|err| panic!("read {}: {err}", root.display())) {
        let entry = entry.unwrap_or_else(|err| panic!("read {}: {err}", root.display()));
        if entry.path().is_dir() {
            collect_agent30_chapters(&entry.path(), paths);
        }
    }
}

fn collect_recipe_manifests(root: &Path, paths: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|err| panic!("read {}: {err}", root.display())) {
        let entry = entry.unwrap_or_else(|err| panic!("read {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_recipe_manifests(&path, paths);
        } else if entry.file_name() == "recipe.toml" {
            paths.insert(path);
        }
    }
}

fn parse_recipe_doc(path: &Path) -> RecipeDoc {
    let text =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    let mut doc = RecipeDoc::default();
    let mut in_expect = false;

    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[expect]]" {
            in_expect = true;
            continue;
        }

        let (key, value) = line
            .split_once(" = ")
            .unwrap_or_else(|| panic!("{}: invalid TOML line `{line}`", path.display()));
        if in_expect {
            if key == "result" {
                doc.expect_results
                    .push(parse_string(value, path, key).trim_end().to_owned());
            }
            continue;
        }

        if value.starts_with('"') {
            doc.strings
                .insert(key.to_owned(), parse_string(value, path, key));
        } else if value.starts_with('[') {
            doc.arrays
                .insert(key.to_owned(), parse_array(value, path, key));
        } else if let Ok(number) = value.parse::<i64>() {
            doc.ints.insert(key.to_owned(), number);
        }
    }

    doc
}

fn parse_string(value: &str, path: &Path, key: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or_else(|| panic!("{}: `{key}` must be a string", path.display()))
        .to_owned()
}

fn parse_array(value: &str, path: &Path, key: &str) -> Vec<String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or_else(|| panic!("{}: `{key}` must be an array", path.display()));
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|entry| parse_string(entry.trim(), path, key))
        .collect()
}

fn assert_recipe_identity(path: &Path, doc: &RecipeDoc, number: i64, expected_id: &str) {
    let id = required_string(doc, path, "id");
    assert_eq!(id, expected_id, "{}: id mismatch", path.display());

    let dir_id = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    assert_eq!(
        id,
        dir_id,
        "{}: id must match recipe directory",
        path.display()
    );

    let order = required_int(doc, path, "order");
    assert_eq!(order, number, "{}: order mismatch", path.display());
}

fn assert_metadata(path: &Path, doc: &RecipeDoc, number: i64, index: usize) {
    let source_chapter = required_int(doc, path, "source_chapter");
    assert_eq!(
        source_chapter,
        EXPECTED_SOURCE_CHAPTERS[index],
        "{}: source_chapter mismatch",
        path.display()
    );

    let architecture_family = required_string(doc, path, "architecture_family");
    let runner_mode = required_string(doc, path, "runner_mode");
    let safety_posture = required_string(doc, path, "safety_posture");
    let tags = required_array(doc, path, "tags");
    let capabilities = required_array(doc, path, "capabilities");
    let requires = required_array(doc, path, "requires");

    assert_eq!(
        runner_mode,
        "fake",
        "{}: runner_mode must be deterministic fake",
        path.display()
    );
    assert!(
        safety_posture.starts_with("offline"),
        "{}: safety_posture must be offline, got {safety_posture}",
        path.display()
    );
    assert!(
        !capabilities.is_empty(),
        "{}: capabilities must not be empty",
        path.display()
    );
    assert!(
        !requires.is_empty(),
        "{}: requires must not be empty",
        path.display()
    );
    assert!(
        tags.contains(&"30-agents".to_owned()),
        "{}: missing 30-agents tag",
        path.display()
    );
    assert!(
        tags.contains(&format!("chapter-{source_chapter:02}")),
        "{}: missing chapter tag",
        path.display()
    );
    assert!(
        tags.contains(&architecture_family),
        "{}: missing architecture family tag `{architecture_family}`",
        path.display()
    );
    assert!(
        tags.contains(&"deterministic".to_owned()),
        "{}: missing deterministic tag",
        path.display()
    );
    assert!(
        number > 0,
        "{}: recipe number must be positive",
        path.display()
    );
}

fn assert_no_network_capability(path: &Path, doc: &RecipeDoc) {
    for field in ["tags", "requires", "capabilities"] {
        for item in required_array(doc, path, field) {
            let item = item.to_ascii_lowercase();
            assert!(
                !item.contains("network")
                    && !item.contains("http")
                    && !item.contains("egress")
                    && !item.contains("api-key"),
                "{}: `{field}` contains network-like capability `{item}`",
                path.display()
            );
        }
    }
}

fn assert_no_copied_external_material(path: &Path) {
    let recipe_dir = path
        .parent()
        .unwrap_or_else(|| panic!("{} has no parent", path.display()));
    let mut files = Vec::new();
    collect_files(recipe_dir, &mut files);
    for file in files {
        let relative = file
            .strip_prefix(recipe_dir)
            .unwrap_or_else(|err| panic!("{}: {err}", file.display()));
        assert!(
            matches!(
                relative.to_str(),
                Some("recipe.toml" | "setup.siml" | "purpose.md" | "expected.txt")
            ),
            "{}: unexpected external material file {}",
            path.display(),
            relative.display()
        );
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("read {}: {err}", file.display()));
        assert!(text.is_ascii(), "{} must be ASCII", file.display());

        let lower = text.to_ascii_lowercase();
        for marker in [
            "http://",
            "https://",
            "book pdf",
            "companion repo",
            "notebook",
            "copied yes",
            "copied-yes",
            "copied: yes",
            "from the book",
        ] {
            assert!(
                !lower.contains(marker),
                "{} contains copied-source marker `{marker}`",
                file.display()
            );
        }
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap_or_else(|err| panic!("read {}: {err}", root.display())) {
        let entry = entry.unwrap_or_else(|err| panic!("read {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

fn assert_deterministic_fixture_run(cx: &mut Cx, path: &Path, doc: &RecipeDoc) {
    // Decode the setup form through the lisp codec, accepting either the direct
    // setup expression or the older quoted wrapper. Decoding twice must produce
    // the same value: this is the deterministic-runtime claim, exercised for real.
    let setup_path = recipe_sibling(path, &required_string(doc, path, "setup"));
    let setup_text = read_ascii(&setup_path);
    let first = evaluate_setup(cx, &setup_text, &setup_path);
    let second = evaluate_setup(cx, &setup_text, &setup_path);
    assert_eq!(
        first,
        second,
        "{}: setup evaluation is not deterministic",
        setup_path.display()
    );

    // The evaluated value matches the recipe's expected form, decoded through the
    // same codec -- a semantic comparison over `Expr`, not a string compare.
    let expected_path = recipe_sibling(path, &required_string(doc, path, "expected"));
    let expected_text = read_ascii(&expected_path);
    let expected = decode_lisp(cx, expected_text.trim(), &expected_path);
    assert_eq!(
        first,
        expected,
        "{}: setup must evaluate to the expected form",
        path.display()
    );

    // ...and matches one of the declared [[expect]].result forms.
    assert!(
        doc.expect_results
            .iter()
            .any(|result| decode_lisp(cx, result.trim(), path) == first),
        "{}: evaluated output must match a declared [[expect]].result",
        path.display()
    );
}

fn build_decode_cx() -> Cx {
    let (mut cx, seat) = Cx::new_seated(Arc::new(EagerPolicy), Arc::new(DefaultFactory));
    conformance_support::seat_cookbook_capabilities(&seat, &mut cx);
    sim::runtime::install_core_runtime(&mut cx);
    sim::numbers_prelude::NumbersPreludeLib::new()
        .install_all(&mut cx)
        .unwrap();
    let lisp = sim::codec_lisp::LispCodecLib::new(cx.registry_mut().fresh_codec_id()).unwrap();
    cx.load_lib(&lisp).unwrap();
    cx.load_lib(&sim::lib_control::ControlLib).unwrap();
    sim::install_agent_lib(&mut cx).unwrap();
    cx
}

fn decode_lisp(cx: &mut Cx, text: &str, source: &Path) -> Expr {
    decode_with_codec(
        cx,
        &Symbol::qualified("codec", "lisp"),
        Input::Text(text.to_owned()),
        ReadPolicy::default(),
    )
    .unwrap_or_else(|err| panic!("{}: lisp decode failed: {err:?}", source.display()))
}

fn evaluate_setup(cx: &mut Cx, setup_text: &str, source: &Path) -> Expr {
    let form = decode_lisp(cx, setup_text.trim(), source);
    if let Some(expr) = unwrap_quote(&form) {
        return expr;
    }

    let value = cx
        .eval_expr(lower_lisp_eval_surface(form))
        .unwrap_or_else(|err| panic!("{}: setup evaluation failed: {err:?}", source.display()));
    value.object().as_expr(cx).unwrap_or_else(|err| {
        panic!(
            "{}: setup result is not an expression: {err:?}",
            source.display()
        )
    })
}

fn unwrap_quote(form: &Expr) -> Option<Expr> {
    match form {
        Expr::Quote {
            mode: QuoteMode::Quote,
            expr,
        } => Some(expr.as_ref().clone()),
        Expr::List(items) if items.len() == 2 && is_quote_head(&items[0]) => Some(items[1].clone()),
        Expr::Call { operator, args } if is_quote_head(operator.as_ref()) && args.len() == 1 => {
            Some(args[0].clone())
        }
        _ => None,
    }
}

fn lower_lisp_eval_surface(expr: Expr) -> Expr {
    match expr {
        Expr::List(items) if !items.is_empty() => {
            let mut items = items
                .into_iter()
                .map(lower_lisp_eval_surface)
                .collect::<Vec<_>>();
            let operator = Box::new(items.remove(0));
            Expr::Call {
                operator,
                args: items,
            }
        }
        Expr::List(items) => Expr::List(items.into_iter().map(lower_lisp_eval_surface).collect()),
        Expr::Vector(items) => {
            Expr::Vector(items.into_iter().map(lower_lisp_eval_surface).collect())
        }
        Expr::Map(entries) => Expr::Map(
            entries
                .into_iter()
                .map(|(key, value)| (lower_lisp_eval_surface(key), lower_lisp_eval_surface(value)))
                .collect(),
        ),
        Expr::Set(items) => Expr::Set(items.into_iter().map(lower_lisp_eval_surface).collect()),
        Expr::Block(items) => Expr::Block(items.into_iter().map(lower_lisp_eval_surface).collect()),
        Expr::Annotated { expr, annotations } => Expr::Annotated {
            expr: Box::new(lower_lisp_eval_surface(*expr)),
            annotations: annotations
                .into_iter()
                .map(|(name, value)| (name, lower_lisp_eval_surface(value)))
                .collect(),
        },
        Expr::Extension { tag, payload } => Expr::Extension {
            tag,
            payload: Box::new(lower_lisp_eval_surface(*payload)),
        },
        other => other,
    }
}

fn is_quote_head(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Symbol(symbol) if symbol.namespace.is_none() && symbol.name.as_ref() == "quote"
    )
}

fn recipe_sibling(path: &Path, name: &str) -> PathBuf {
    path.parent()
        .unwrap_or_else(|| panic!("{} has no parent", path.display()))
        .join(name)
}

fn read_ascii(path: &Path) -> String {
    let text =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    assert!(text.is_ascii(), "{} must be ASCII", path.display());
    text
}

fn required_string(doc: &RecipeDoc, path: &Path, key: &str) -> String {
    doc.strings
        .get(key)
        .unwrap_or_else(|| panic!("{}: missing `{key}`", path.display()))
        .to_owned()
}

fn required_int(doc: &RecipeDoc, path: &Path, key: &str) -> i64 {
    *doc.ints
        .get(key)
        .unwrap_or_else(|| panic!("{}: missing `{key}`", path.display()))
}

fn required_array(doc: &RecipeDoc, path: &Path, key: &str) -> Vec<String> {
    doc.arrays
        .get(key)
        .unwrap_or_else(|| panic!("{}: missing `{key}`", path.display()))
        .to_owned()
}
