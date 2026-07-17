use sim::{
    kernel::{
        AbiVersion, CatalogSource, ClaimPattern, Cx, DefaultFactory, Dependency, EagerPolicy,
        Export, ExportKind, ExportRecord, Lib, LibId, LibLoader, LibManifest, LibSource,
        LibSourceSpec, LibTarget, Linker, LoadCx, LoaderRegistry, Ref, RegistryBootState, Symbol,
        Version,
    },
    lib_standard_core::{LanguageProfile, ProfileRegistry, language_profile_lib_symbol},
};

use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct LifecycleSnapshot {
    libs: Vec<(Symbol, Vec<ExportRecord>)>,
    export_symbols: Vec<(ExportKind, Symbol, String)>,
    claims: Vec<Vec<u8>>,
    effect_records: Vec<String>,
}

#[derive(Clone)]
struct FixtureValueLib {
    id: Symbol,
    export: Symbol,
    value: bool,
    requires: Vec<Dependency>,
}

struct StandardProfileCase {
    name: &'static str,
    install: fn(&mut Cx, &mut ProfileRegistry) -> sim::kernel::Result<LanguageProfile>,
}

struct ReceiptFixtureLoader;

#[test]
fn load_unload_restores_observable_registry_snapshot() {
    let mut cx = conformance_cx();
    let before = lifecycle_snapshot(&cx);
    let lib = fixture_lib("roundtrip");

    let lib_id = cx.load_lib(&lib).unwrap();
    assert!(cx.registry().value_by_symbol(&lib.export).is_some());
    cx.unload_lib(lib_id).unwrap();

    assert_eq!(lifecycle_snapshot(&cx), before);
}

#[test]
fn reload_matches_single_load_and_absent_unload_is_noop() {
    let lib = fixture_lib("reload");
    let mut single = conformance_cx();
    let single_id = single.load_lib(&lib).unwrap();
    let single_loaded = lifecycle_snapshot(&single);

    let mut roundtrip = conformance_cx();
    let first_id = roundtrip.load_lib(&lib).unwrap();
    roundtrip.unload_lib(first_id).unwrap();
    let after_unload = lifecycle_snapshot(&roundtrip);
    assert_eq!(roundtrip.unload_lib(LibId(999_999)).unwrap(), Vec::new());
    assert_eq!(lifecycle_snapshot(&roundtrip), after_unload);

    let second_id = roundtrip.load_lib(&lib).unwrap();
    assert_eq!(second_id, single_id);
    assert_eq!(lifecycle_snapshot(&roundtrip), single_loaded);
}

#[test]
fn depended_on_lib_refuses_bare_unload_and_cascade_unloads_dependents_first() {
    let dep = fixture_lib("dep");
    let user = fixture_lib("user").requiring(dep.id.clone());
    let mut cx = conformance_cx();

    let dep_id = cx.load_lib(&dep).unwrap();
    let user_id = cx.load_lib(&user).unwrap();
    let err = cx.unload_lib(dep_id).unwrap_err();
    assert!(matches!(
        err,
        sim::kernel::Error::LibHasDependents { lib, dependents }
            if lib == dep.id && dependents == vec![user.id.clone()]
    ));
    assert!(cx.registry().lib(&dep.id).is_some());
    assert!(cx.registry().lib(&user.id).is_some());

    assert_eq!(
        cx.unload_lib_cascade(dep_id).unwrap(),
        vec![user_id, dep_id]
    );
    assert!(cx.registry().lib(&dep.id).is_none());
    assert!(cx.registry().lib(&user.id).is_none());
}

#[test]
fn live_value_survives_unload_but_fresh_resolution_fails() {
    let mut cx = conformance_cx();
    let lib = fixture_lib("live");
    let lib_id = cx.load_lib(&lib).unwrap();
    let live = cx.registry().value_by_symbol(&lib.export).unwrap().clone();

    cx.unload_lib(lib_id).unwrap();

    assert!(cx.registry().value_by_symbol(&lib.export).is_none());
    assert_eq!(
        live.object().as_expr(&mut cx).unwrap(),
        sim::kernel::Expr::Bool(true)
    );
}

#[test]
fn standard_control_lib_uses_recorded_claim_receipts() {
    let mut cx = conformance_cx();
    let before = lifecycle_snapshot(&cx);

    sim::lib_control::install_control_lib(&mut cx).unwrap();
    let lib_id = cx
        .registry()
        .lib(&sim::lib_control::manifest_name())
        .unwrap()
        .id;
    assert!(
        cx.resolve_function(&sim::lib_control::prompt_symbol())
            .is_ok()
    );
    assert_ne!(lifecycle_snapshot(&cx), before);

    cx.unload_lib(lib_id).unwrap();

    assert_eq!(lifecycle_snapshot(&cx), before);
    assert!(
        cx.resolve_function(&sim::lib_control::prompt_symbol())
            .is_err()
    );
}

#[test]
fn standard_profile_installers_unload_profile_owned_claims_with_profile_lib() {
    for case in standard_profile_cases() {
        let mut cx = conformance_cx();
        let mut registry = ProfileRegistry::new();

        let profile = (case.install)(&mut cx, &mut registry)
            .unwrap_or_else(|err| panic!("{} profile install failed: {err:?}", case.name));
        let lib_symbol = language_profile_lib_symbol(&profile.symbol);
        let lib_id = cx
            .registry()
            .lib(&lib_symbol)
            .unwrap_or_else(|| panic!("{} did not load {lib_symbol}", case.name))
            .id;
        assert!(
            !profile_claims(&cx, &profile.symbol).is_empty(),
            "{} did not publish visible profile claims",
            case.name
        );

        cx.unload_lib(lib_id).unwrap();

        assert!(
            cx.registry().lib(&lib_symbol).is_none(),
            "{} did not unload its profile receipt",
            case.name
        );
        assert!(
            profile_claims(&cx, &profile.symbol).is_empty(),
            "{} left profile claims behind after unloading the profile receipt",
            case.name
        );
    }
}

#[test]
fn boot_receipts_encode_decode_and_replay_registry_surface() {
    let dep_source = Symbol::qualified("lifecycle", "boot-dep-source");
    let user_source = Symbol::qualified("lifecycle", "boot-user-source");
    let loaders = LoaderRegistry::new()
        .with_loader(ReceiptFixtureLoader)
        .with_source(dep_source.clone(), CatalogSource::Bytes(b"dep".to_vec()))
        .with_source(user_source.clone(), CatalogSource::Bytes(b"user".to_vec()));
    let mut recorded = conformance_cx();

    let dep_receipt = loaders
        .load_and_register_with_receipt(&mut recorded, LibSourceSpec::Symbol(dep_source))
        .unwrap();
    let user_receipt = loaders
        .load_and_register_with_receipt(&mut recorded, LibSourceSpec::Symbol(user_source))
        .unwrap();
    assert_eq!(
        user_receipt.dependencies,
        vec![sim::kernel::LibBootDependency {
            lib_id: dep_receipt.lib_id,
            symbol: dep_receipt.manifest.id.clone(),
        }]
    );

    let state = RegistryBootState::new(vec![dep_receipt, user_receipt]);
    let encoded = state.to_datum();
    encoded.canonical_bytes().unwrap();
    let decoded = RegistryBootState::from_datum(&encoded).unwrap();
    assert_eq!(decoded, state);

    let mut replayed = conformance_cx();
    assert_eq!(
        loaders.replay_boot_state(&mut replayed, &decoded).unwrap(),
        state.receipts
    );
    assert_eq!(lifecycle_snapshot(&replayed), lifecycle_snapshot(&recorded));
}

fn conformance_cx() -> Cx {
    Cx::new(Arc::new(EagerPolicy), Arc::new(DefaultFactory))
}

fn fixture_lib(name: &str) -> FixtureValueLib {
    FixtureValueLib {
        id: Symbol::qualified("lifecycle", format!("{name}-lib")),
        export: Symbol::qualified("lifecycle", format!("{name}-value")),
        value: true,
        requires: Vec::new(),
    }
}

impl FixtureValueLib {
    fn requiring(mut self, id: Symbol) -> Self {
        self.requires.push(Dependency {
            id,
            minimum_version: None,
        });
        self
    }
}

impl Lib for FixtureValueLib {
    fn manifest(&self) -> LibManifest {
        LibManifest {
            id: self.id.clone(),
            version: Version(env!("CARGO_PKG_VERSION").to_owned()),
            abi: AbiVersion { major: 0, minor: 1 },
            target: LibTarget::HostRegistered,
            requires: self.requires.clone(),
            capabilities: Vec::new(),
            exports: vec![Export::Value {
                symbol: self.export.clone(),
            }],
        }
    }

    fn load(&self, cx: &mut LoadCx, linker: &mut Linker<'_>) -> sim::kernel::Result<()> {
        linker.value(self.export.clone(), cx.factory().bool(self.value).unwrap())
    }
}

impl LibLoader for ReceiptFixtureLoader {
    fn can_load(&self, source: &LibSource) -> bool {
        matches!(source, LibSource::Bytes(bytes) if matches!(bytes.as_slice(), b"dep" | b"user"))
    }

    fn load(&self, _cx: &mut Cx, source: LibSource) -> sim::kernel::Result<Box<dyn Lib>> {
        match source {
            LibSource::Bytes(bytes) if bytes == b"dep" => Ok(Box::new(fixture_lib("boot-dep"))),
            LibSource::Bytes(bytes) if bytes == b"user" => {
                let dep = fixture_lib("boot-dep");
                Ok(Box::new(fixture_lib("boot-user").requiring(dep.id)))
            }
            _ => Err(sim::kernel::Error::HostError(
                "unsupported receipt fixture source".to_owned(),
            )),
        }
    }
}

fn lifecycle_snapshot(cx: &Cx) -> LifecycleSnapshot {
    let libs = cx
        .registry()
        .libs()
        .iter()
        .map(|loaded| (loaded.manifest.id.clone(), loaded.exports.clone()))
        .collect();
    let export_symbols = cx
        .registry()
        .export_symbols()
        .iter()
        .flat_map(|(kind, symbols)| {
            symbols
                .iter()
                .map(move |(symbol, id)| (kind.clone(), symbol.clone(), format!("{id:?}")))
        })
        .collect();
    let mut claims = cx
        .query_facts(ClaimPattern::any().include_revoked())
        .unwrap()
        .into_iter()
        .map(|claim| claim.canonical_datum().canonical_bytes().unwrap())
        .collect::<Vec<_>>();
    claims.sort();
    LifecycleSnapshot {
        libs,
        export_symbols,
        claims,
        effect_records: cx
            .effect_ledger()
            .records()
            .iter()
            .map(|record| format!("{record:?}"))
            .collect(),
    }
}

fn profile_claims(cx: &Cx, profile: &Symbol) -> Vec<sim::kernel::Claim> {
    cx.query_facts(ClaimPattern {
        subject: Some(Ref::Symbol(profile.clone())),
        ..ClaimPattern::any()
    })
    .unwrap()
}

fn standard_profile_cases() -> [StandardProfileCase; 8] {
    [
        StandardProfileCase {
            name: "scheme",
            install: sim::lib_lang_scheme::install_r7rs_small_profile,
        },
        StandardProfileCase {
            name: "common-lisp",
            install: sim::lib_lang_cl::install_cl_lite_profile,
        },
        StandardProfileCase {
            name: "clojure",
            install: sim::lib_lang_clojure::install_clojure_core_profile,
        },
        StandardProfileCase {
            name: "islisp",
            install: sim::lib_lang_islisp::install_islisp_profile,
        },
        StandardProfileCase {
            name: "julia",
            install: sim::lib_lang_julia::install_julia_core_profile,
        },
        StandardProfileCase {
            name: "lua",
            install: sim::lib_lang_lua::install_lua_core_profile,
        },
        StandardProfileCase {
            name: "ruby",
            install: sim::lib_lang_ruby::install_ruby_dsl_profile,
        },
        StandardProfileCase {
            name: "typed-lazy",
            install: sim::lib_lang_typed_lazy::install_typed_lazy_profile,
        },
    ]
}
