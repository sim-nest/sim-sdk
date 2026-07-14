//! sim-nest's product cookbook loadable-lib directory.
//!
//! `sim-lib-cookbook` keeps a small standalone fixture directory. The umbrella
//! crate owns the product directory because only this crate sees the
//! constellation feature graph without adding back-edges to the cookbook lib.

use std::sync::Arc;

use sim_cookbook::EmbeddedDir;
use sim_kernel::{CodecId, Lib};
use sim_lib_cookbook::{
    ConfigProvider, CookbookConfig, LoadableLibConfig, LoadableLibList, LoadableLibResolver,
    ResolvedLoadable,
};

const PRODUCT_CODEC_BASE: u32 = 10_000;

#[macro_use]
mod audio_stream;
#[macro_use]
mod codecs;
#[macro_use]
mod data;
#[macro_use]
mod femm;
#[macro_use]
mod music;
#[macro_use]
mod numbers;
#[macro_use]
mod runtime_libs;

macro_rules! loadable_libs {
    ($m:ident) => {
        cookbook_directory_codecs!($m);
        cookbook_directory_numbers!($m);
        cookbook_directory_runtime_libs!($m);
        cookbook_directory_femm!($m);
        cookbook_directory_music!($m);
        cookbook_directory_audio_stream!($m);
        cookbook_directory_data!($m);
    };
}

/// Product default cookbook config for the enabled sim-nest feature set.
pub fn default_cookbook_config() -> CookbookConfig {
    CookbookConfig {
        minimum_loaded: vec!["codec/lisp".to_owned(), "core".to_owned()],
        loadable_libs: loadable_rows(),
    }
}

/// Resolve the product default directory for the enabled sim-nest feature set.
pub fn default_loadable_libs() -> (LoadableLibList, Vec<String>) {
    ConfigProvider::new(default_cookbook_config(), &SimNestCookbookResolver).loadable_libs()
}

/// Loadable-lib ids compiled into a `cookbook-all` build.
#[cfg(feature = "cookbook-all")]
pub fn cookbook_all_lib_ids() -> Vec<&'static str> {
    let mut ids = Vec::new();
    macro_rules! push_id {
        ($id:literal, $title:literal, $feature:literal, $recipes:expr, $make:expr) => {
            #[cfg(feature = $feature)]
            {
                let _ = $title;
                ids.push($id);
            }
        };
    }
    loadable_libs!(push_id);
    ids
}

fn loadable_rows() -> Vec<LoadableLibConfig> {
    let mut rows = Vec::new();
    macro_rules! push_row {
        ($id:literal, $title:literal, $feature:literal, $recipes:expr, $make:expr) => {
            #[cfg(feature = $feature)]
            {
                let _ = $title;
                rows.push(row($id));
            }
        };
    }
    loadable_libs!(push_row);
    rows
}

fn row(id: &str) -> LoadableLibConfig {
    LoadableLibConfig {
        id: id.to_owned(),
        source: format!("sim-nest:{id}"),
    }
}

/// Resolver over the loadable libs linked into this sim-nest build.
pub struct SimNestCookbookResolver;

impl LoadableLibResolver for SimNestCookbookResolver {
    fn resolve(&self, source: &str, id: &str) -> Option<ResolvedLoadable> {
        if source != format!("sim-nest:{id}") {
            return None;
        }

        macro_rules! resolve_if {
            ($row_id:literal, $title:literal, $feature:literal, $recipes:expr, $make:expr) => {
                #[cfg(feature = $feature)]
                if id == $row_id {
                    return Some(resolved($title, $recipes, $make));
                }
            };
        }
        loadable_libs!(resolve_if);
        None
    }
}

fn codec_id(offset: u32) -> CodecId {
    CodecId(PRODUCT_CODEC_BASE + offset)
}

fn resolved<F>(title: &str, recipes: Option<EmbeddedDir>, make: F) -> ResolvedLoadable
where
    F: Fn() -> Box<dyn Lib + Send + Sync> + Send + Sync + 'static,
{
    ResolvedLoadable {
        title: title.to_owned(),
        recipes,
        factory: Arc::new(make),
    }
}

#[cfg(all(test, feature = "cookbook-all"))]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn every_loadable_lib_has_a_directory_row() {
        let cfg = super::default_cookbook_config();
        assert_eq!(cfg.minimum_loaded, ["codec/lisp", "core"]);

        let ids = super::cookbook_all_lib_ids();
        let mut counts = BTreeMap::new();
        for row in &cfg.loadable_libs {
            *counts.entry(row.id.as_str()).or_insert(0usize) += 1;
            assert_eq!(row.source, format!("sim-nest:{}", row.id));
        }
        for id in &ids {
            assert_eq!(counts.get(id).copied(), Some(1), "{id} row count");
        }
        assert_eq!(counts.len(), ids.len());

        let (dir, diags) = super::default_loadable_libs();
        assert!(diags.is_empty(), "unresolved rows: {diags:?}");
        let resolved = dir
            .entries()
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<BTreeSet<_>>();
        for id in &ids {
            assert!(
                dir.entry(id).is_some(),
                "loadable lib `{id}` missing a directory row"
            );
            assert!(
                resolved.contains(id),
                "loadable lib `{id}` missing a factory"
            );
        }
        assert_eq!(dir.entries().len(), ids.len());
    }

    #[test]
    fn cookbook_all_feature_matches_directory_features() {
        let cargo_toml = include_str!("../../Cargo.toml");
        let cookbook_features = parse_feature(cargo_toml, "cookbook-all");
        let mut row_features = BTreeSet::new();
        macro_rules! push_feature {
            ($id:literal, $title:literal, $feature:literal, $recipes:expr, $make:expr) => {
                #[cfg(feature = $feature)]
                {
                    let _ = ($id, $title);
                    row_features.insert($feature);
                }
            };
        }
        loadable_libs!(push_feature);

        for feature in &row_features {
            assert!(
                cookbook_features.contains(*feature),
                "`cookbook-all` does not enable `{feature}`"
            );
        }

        let no_directory_rows = BTreeSet::from(["cookbook", "shape", "discrete-rank"]);
        for feature in cookbook_features {
            assert!(
                row_features.contains(feature) || no_directory_rows.contains(feature),
                "`cookbook-all` enables `{feature}` without a loadable-lib directory row"
            );
        }
    }

    fn parse_feature<'a>(cargo_toml: &'a str, feature: &str) -> BTreeSet<&'a str> {
        let prefix = format!("{feature} = [");
        let line = cargo_toml
            .lines()
            .find(|line| line.starts_with(&prefix))
            .expect("feature line");
        line[prefix.len()..]
            .trim_end_matches(']')
            .split(',')
            .map(str::trim)
            .filter_map(|part| part.strip_prefix('"')?.strip_suffix('"'))
            .collect()
    }
}
