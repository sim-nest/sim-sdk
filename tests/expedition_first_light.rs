#![cfg(feature = "continuity")]

use sim_citizen::CitizenField;
use sim_kernel::{Expr, Symbol};
use sim_lib_capability_pack::{CapabilityPack, ContentId, PackDir, resolve};
use sim_lib_continuity::{
    ContinuityEvent, ContinuityJournal, ContinuityPlan, ContinuityState, MemoryJournal,
    NetworkPolicy, RoleDemand,
};
use sim_lib_expedition_book::{EvidenceRef, ExpeditionBook, codec as book_codec};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    time::{Duration, Instant},
};

fn id(c: char) -> ContentId {
    ContentId::parse(format!("sha256:{}", c.to_string().repeat(64))).unwrap()
}
fn syms(v: &[&str]) -> BTreeSet<Symbol> {
    v.iter().map(|s| Symbol::new(*s)).collect()
}
fn import(content: &ContentId) -> Expr {
    Expr::List(vec![
        Symbol::new("organ").encode_field(),
        content.to_string().encode_field(),
        vec![Symbol::new("read")].encode_field(),
    ])
}
fn pack(content: &ContentId, imports: Vec<Expr>) -> CapabilityPack {
    CapabilityPack {
        content: content.to_string(),
        imports,
        ..Default::default()
    }
}

#[derive(Default)]
struct Dir(BTreeMap<ContentId, CapabilityPack>);
impl PackDir for Dir {
    fn get(&self, id: &ContentId) -> Option<(ContentId, CapabilityPack)> {
        self.0.get(id).cloned().map(|p| (id.clone(), p))
    }
}

fn plan() -> ContinuityPlan {
    let services = [
        "phone-review",
        "lifecycle",
        "capture",
        "render",
        "stop",
        "journal-append",
    ]
    .map(Symbol::new)
    .to_vec();
    ContinuityPlan {
        plan_id: Symbol::qualified("expedition", "first-light"),
        roles: vec![RoleDemand {
            role: Symbol::new("phone"),
            root: true,
            required_services: services.clone(),
            fallbacks: vec![],
        }],
        available_services: services,
        retention_turns: 16,
        network: NetworkPolicy::Offline,
        ..Default::default()
    }
}

fn record(
    journal: &mut MemoryJournal,
    state: &ContinuityState,
    n: usize,
    kind: &str,
    condition: &str,
) -> ContinuityState {
    journal
        .accept(
            &plan(),
            state,
            ContinuityEvent {
                event_id: Symbol::qualified("first-light", format!("{n:02}-{kind}")),
                sequence: (n - 1) as u64,
                logical_time: n as u64,
                kind: Symbol::new(kind),
                role: Symbol::new("phone"),
                payload: Expr::String(condition.into()),
                ..Default::default()
            },
        )
        .unwrap()
}

#[test]
fn post_deletion_offline_replay_is_byte_stable_at_every_content_boundary() {
    let started = Instant::now();
    let shared = id('a');
    let root = id('b');
    let mut dir = Dir::default();
    dir.0.insert(shared.clone(), pack(&shared, vec![]));
    dir.0
        .insert(root.clone(), pack(&root, vec![import(&shared)]));
    let closure = resolve(&dir, root.clone(), syms(&["read"])).unwrap();
    assert_eq!(
        closure
            .packs
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>(),
        vec![shared.clone(), root.clone()]
    );

    let mut book = ExpeditionBook::new("first-light", "choose the durable offline route");
    book.branch(
        book.revision,
        "root",
        "offline",
        "adopt canonical offline organs",
    )
    .unwrap();
    book.reference(
        book.revision,
        "offline",
        [
            EvidenceRef::new("artifact", root.to_string()).unwrap(),
            EvidenceRef::new("source", shared.to_string()).unwrap(),
        ],
    )
    .unwrap();
    book.set_next_play(book.revision, "offline", "export and close")
        .unwrap();
    book.choose(book.revision, "offline").unwrap();
    book.seal(book.revision, "offline").unwrap();
    let book_bytes = book_codec::encode(&book);

    let mut journal = MemoryJournal::default();
    let mut state = ContinuityState::default();
    for (n, kind, condition) in [
        (1, "open", "book-opened"),
        (2, "resolve", "pack-closure-resolved"),
        (3, "condition", "model-endpoint-absent"),
        (4, "condition", "device-absent"),
        (5, "unsupported-route", "preserved:route/model-live"),
        (6, "refused-effect", "preserved:effect/device-output"),
        (7, "adopt", "offline-choice-adopted"),
        (8, "export", "canonical-bytes-exported"),
        (9, "close", "expedition-closed"),
    ] {
        state = record(&mut journal, &state, n, kind, condition);
    }
    let journal_identity = journal
        .turns()
        .iter()
        .map(|t| (t.event_id.to_string(), t.event.payload.clone()))
        .collect::<Vec<_>>();

    let scratch = std::env::temp_dir().join(format!("sim-first-light-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    for name in ["views", "chat", "model-state", "caches", "scratch"] {
        fs::create_dir(scratch.join(name)).unwrap();
        fs::write(scratch.join(name).join("derived"), b"disposable").unwrap();
    }
    fs::write(scratch.join("book.export"), &book_bytes).unwrap();
    let durable_book = fs::read_to_string(scratch.join("book.export")).unwrap();
    for name in ["views", "chat", "model-state", "caches", "scratch"] {
        fs::remove_dir_all(scratch.join(name)).unwrap();
    }
    let replayed = book_codec::decode(&durable_book).unwrap();
    assert_eq!(book_codec::encode(&replayed), book_bytes);
    assert_eq!(replayed.choice.as_deref(), Some("offline"));
    assert_eq!(
        replayed.branches["offline"]
            .evidence
            .iter()
            .map(|r| r.content_key.clone())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([root.to_string(), shared.to_string()])
    );
    let replayed_closure = resolve(&dir, root, syms(&["read"])).unwrap();
    assert_eq!(replayed_closure.packs, closure.packs);
    assert!(
        journal_identity
            .iter()
            .any(|(_, p)| p == &Expr::String("preserved:route/model-live".into()))
    );
    assert!(
        journal_identity
            .iter()
            .any(|(_, p)| p == &Expr::String("preserved:effect/device-output".into()))
    );
    assert_eq!(journal_identity.len(), 9);
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "bounded offline setup/replay exceeded two seconds"
    );
    fs::remove_dir_all(&scratch).unwrap();
}
