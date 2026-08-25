use sim::continuity::{ContinuityEvent, ContinuityJournal, ContinuityState, MemoryJournal};
use sim::kernel::{Expr, Symbol};

mod shared { include!("../shared.rs"); }

fn main() {
    let plan = shared::phone_plan(Symbol::qualified("synthetic", "thought"));
    let event = ContinuityEvent {
        event_id: Symbol::qualified("content", "sha256-8f7d-synthetic-thought"),
        kind: Symbol::new("capture"), role: Symbol::new("phone"),
        payload: Expr::String("pending-honestly".into()), ..Default::default()
    };
    let mut journal = MemoryJournal::default();
    journal.accept(&plan, &ContinuityState::default(), event)
        .expect("offline phone capture is complete");
    println!("{}", journal.turns()[0].event_id);
}
