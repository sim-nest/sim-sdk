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

const EXPECTED_IDS: [&str; 5] = [
    "atelier-radar-standard-crate",
    "atelier-runtime-operation",
    "atelier-codec-roundtrip",
    "atelier-guideline-firewall",
    "atelier-change-capsule",
];

#[derive(Clone, Debug, Default)]
struct RecipeDoc {
    strings: BTreeMap<String, String>,
    bools: BTreeMap<String, bool>,
    arrays: BTreeMap<String, Vec<String>>,
    expect_results: Vec<String>,
}

#[test]
#[ignore = "requires sibling repository recipe corpora and recipe-runtime parity"]
fn atelier_self_hosting_recipes_are_offline_and_codec_evaluated() {
    let manifests = collect_atelier_manifests();
    assert_eq!(manifests.len(), EXPECTED_IDS.len(), "{manifests:?}");

    let mut seen = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    let mut cx = build_decode_cx();
    for manifest in manifests {
        let doc = parse_recipe_doc(&manifest);
        let id = required_string(&doc, &manifest, "id");
        assert!(
            EXPECTED_IDS.contains(&id.as_str()),
            "unexpected recipe id {id}"
        );
        assert!(seen.insert(id.clone()), "duplicate recipe id {id}");
        assert_eq!(id, recipe_dir_name(&manifest));

        let runner = required_string(&doc, &manifest, "runner_mode");
        assert!(
            runner == "fake" || runner == "cassette",
            "{}: unsupported runner {runner}",
            manifest.display()
        );
        assert!(!required_bool(&doc, &manifest, "live_model"));
        assert!(!required_bool(&doc, &manifest, "network"));

        let events = required_array(&doc, &manifest, "cassette_events");
        assert!(!events.is_empty(), "{}: empty cassette", manifest.display());

        for tag in required_array(&doc, &manifest, "tags") {
            evidence.insert(tag);
        }
        for item in required_array(&doc, &manifest, "requires") {
            evidence.insert(item);
        }
        assert_setup_evaluates_to_expected(&mut cx, &manifest, &doc);
    }

    assert_eq!(seen, EXPECTED_IDS.into_iter().map(str::to_owned).collect());
    for required in [
        "radar",
        "codec-prism",
        "guideline-firewall",
        "validation",
        "pin-plan",
        "change-capsule",
        "cassette-hash",
    ] {
        assert!(evidence.contains(required), "missing {required}");
    }
}

fn collect_atelier_manifests() -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();
    if let Some(packages_root) = find_meta_packages_root() {
        for entry in fs::read_dir(&packages_root)
            .unwrap_or_else(|err| panic!("read {}: {err}", packages_root.display()))
        {
            let entry =
                entry.unwrap_or_else(|err| panic!("read {}: {err}", packages_root.display()));
            let chapter = entry.path().join("recipes/40-atelier");
            if chapter.is_dir() {
                collect_recipe_manifests(&chapter, &mut paths);
            }
        }
    } else {
        let chapter =
            find_projects_root().join("sim-agent-net/crates/sim-lib-agent/recipes/40-atelier");
        collect_recipe_manifests(&chapter, &mut paths);
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
    assert!(text.is_ascii(), "{} must be ASCII", path.display());
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
                doc.expect_results.push(parse_string(value, path, key));
            }
        } else if value.starts_with('"') {
            doc.strings
                .insert(key.to_owned(), parse_string(value, path, key));
        } else if value.starts_with('[') {
            doc.arrays
                .insert(key.to_owned(), parse_array(value, path, key));
        } else if let Ok(value) = value.parse::<bool>() {
            doc.bools.insert(key.to_owned(), value);
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

fn required_string(doc: &RecipeDoc, path: &Path, key: &str) -> String {
    doc.strings
        .get(key)
        .unwrap_or_else(|| panic!("{}: missing `{key}`", path.display()))
        .to_owned()
}

fn required_bool(doc: &RecipeDoc, path: &Path, key: &str) -> bool {
    *doc.bools
        .get(key)
        .unwrap_or_else(|| panic!("{}: missing `{key}`", path.display()))
}

fn required_array(doc: &RecipeDoc, path: &Path, key: &str) -> Vec<String> {
    doc.arrays
        .get(key)
        .unwrap_or_else(|| panic!("{}: missing `{key}`", path.display()))
        .to_owned()
}

fn recipe_dir_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn assert_setup_evaluates_to_expected(cx: &mut Cx, path: &Path, doc: &RecipeDoc) {
    let recipe_dir = path
        .parent()
        .unwrap_or_else(|| panic!("{} has no parent", path.display()));

    // Decode the setup form through the lisp codec, accepting either the direct
    // setup expression or the older quoted wrapper, then compare semantically.
    let setup_path = recipe_dir.join(required_string(doc, path, "setup"));
    let setup_text = read_ascii(&setup_path);
    let evaluated = evaluate_setup(cx, &setup_text, &setup_path);

    let expected_path = recipe_dir.join(required_string(doc, path, "expected"));
    let expected_text = read_ascii(&expected_path);
    let expected = decode_lisp(cx, expected_text.trim(), &expected_path);
    assert_eq!(
        evaluated,
        expected,
        "{}: setup must evaluate to the expected form",
        path.display()
    );

    assert!(
        doc.expect_results
            .iter()
            .any(|result| decode_lisp(cx, result.trim(), path) == evaluated),
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
        Expr::List(items) if items.len() > 1 => {
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

fn read_ascii(path: &Path) -> String {
    let text =
        fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    assert!(text.is_ascii(), "{} must be ASCII", path.display());
    text
}
