#![cfg(feature = "relation-sqlite")]

use sim::{
    kernel::{Datum, Symbol},
    relation::{core, plan, schema, site, sqlite},
};
use std::sync::{Arc, Mutex};

use site::Driver as _;

#[derive(Default)]
struct RecordingDriver(Arc<Mutex<Vec<String>>>);

struct RecordingSession(Arc<Mutex<Vec<String>>>);

impl site::Driver for RecordingDriver {
    fn connect(
        &self,
        _: &Datum,
        _: &site::Limits,
    ) -> Result<Box<dyn site::Session>, site::SiteError> {
        Ok(Box::new(RecordingSession(self.0.clone())))
    }
}

impl site::Session for RecordingSession {
    fn query(
        &mut self,
        checked: &plan::CheckedQuery,
        _: &site::Bindings,
        _: &site::Limits,
        sink: &mut dyn site::RowSink,
    ) -> Result<site::ProviderStats, site::SiteError> {
        self.0
            .lock()
            .unwrap()
            .push(format!("{:?}", checked.plan_id()));
        let plan::Rel::Values { rows, .. } = checked.plan() else {
            return Err(site::SiteError::Provider);
        };
        for row in rows {
            sink.push(row.clone())?;
        }
        Ok(site::ProviderStats {
            work: rows.len() as u64,
            affected: 0,
        })
    }
    fn mutate(
        &mut self,
        _: &plan::CheckedMutation,
        _: &site::Bindings,
        _: &site::Limits,
        _: &mut dyn site::RowSink,
    ) -> Result<site::ProviderStats, site::SiteError> {
        Err(site::SiteError::Provider)
    }
    fn migrate(
        &mut self,
        _: &sim::relation::migrate::CheckedProgram,
        _: &site::Limits,
    ) -> Result<site::ProviderStats, site::SiteError> {
        Err(site::SiteError::Provider)
    }
    fn schema(
        &mut self,
        _: &sim::relation::migrate::CheckedProgram,
        _: &site::Limits,
    ) -> Result<site::ProviderStats, site::SiteError> {
        Err(site::SiteError::Provider)
    }
    fn transaction(
        &mut self,
        _: &mut dyn FnMut(&mut dyn site::Transaction) -> Result<(), site::SiteError>,
    ) -> Result<(), site::SiteError> {
        Err(site::SiteError::Provider)
    }
    fn attach(
        &mut self,
        _: &Datum,
        _: &site::Limits,
    ) -> Result<site::ProviderStats, site::SiteError> {
        Err(site::SiteError::Provider)
    }
}

#[derive(Default)]
struct Rows(Vec<core::Row>);
impl site::RowSink for Rows {
    fn push(&mut self, row: core::Row) -> Result<(), site::SiteError> {
        self.0.push(row);
        Ok(())
    }
}

fn admitted_request() -> (core::DomainCatalog, plan::CheckedQuery, site::Bindings) {
    let domains = core::DomainCatalog::new([core::BaseDomain::Text.spec()]).unwrap();
    let row_type = core::RowType::new([core::FieldType {
        name: core::FieldName::new(Symbol::new("message")).unwrap(),
        domain: core::BaseDomain::Text.id(),
        nullable: false,
    }])
    .unwrap();
    let row = core::Row::new(
        row_type.clone(),
        [core::Cell::new(
            core::BaseDomain::Text.id(),
            Some(Datum::String("same plan".into())),
        )],
    )
    .unwrap();
    let logical = plan::Rel::Values {
        bind: core::BindingName::new(Symbol::new("input")).unwrap(),
        row_type,
        rows: vec![row],
    };
    let schema = schema::SchemaBuilder::new(core::SchemaName::new(Symbol::new("empty")).unwrap())
        .build(&domains, &schema::AcceptAllValues)
        .unwrap();
    let empty = core::RowType::new([]).unwrap();
    let checked = plan::admit_query(
        logical,
        &schema,
        &domains,
        empty.clone(),
        plan::AdmissionLimits::default(),
    )
    .unwrap();
    let bindings = site::Bindings::new(&empty, []).unwrap();
    (domains, checked, bindings)
}

#[test]
fn one_admitted_request_substitutes_recording_and_sqlite_sites() {
    let (domains, checked, bindings) = admitted_request();
    let limits = site::Limits::new(8, 8, 1024, 64).unwrap();
    let locator = Datum::Node {
        tag: Symbol::qualified("relation", "memory"),
        fields: vec![],
    };

    let recording = RecordingDriver::default();
    let mut logical = recording.connect(&locator, &limits).unwrap();
    let mut logical_rows = Rows::default();
    logical
        .query(&checked, &bindings, &limits, &mut logical_rows)
        .unwrap();

    let sqlite = sqlite::SqliteDriver::new(domains, sqlite::PreopenedStores::default());
    let mut physical = sqlite.connect(&locator, &limits).unwrap();
    let mut sqlite_rows = Rows::default();
    physical
        .query(&checked, &bindings, &limits, &mut sqlite_rows)
        .unwrap();

    assert_eq!(logical_rows.0, sqlite_rows.0);
}

#[test]
fn sdk_surface_keeps_seals_and_provider_artifacts_at_their_owners() {
    fn checked_views(query: &plan::CheckedQuery) {
        let _ = (
            query.schema_id(),
            query.catalog_id(),
            query.parameters(),
            query.output(),
            query.plan_id(),
            query.plan(),
        );
    }
    let (_, checked, _) = admitted_request();
    checked_views(&checked);
    let _table_path = sim::relation::table::TablePath::parse_absolute("/relation").unwrap();
    let _mounted_type = std::any::TypeId::of::<sim::relation::mount::MountedDir>();
    let _relation_dir_type = std::any::TypeId::of::<sim::relation::table_relation::RelationDir>();
}
// conformance: SDK relation exports preserve the intended facade boundary.
