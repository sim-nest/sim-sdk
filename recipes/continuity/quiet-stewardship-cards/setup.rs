use sim::continuity::{ContinuityEvent, ContinuityJournal, ContinuityState, MemoryJournal};
use sim::kernel::{Expr, Symbol};

mod shared { include!("../shared.rs"); }

fn main() {
    let plan = shared::phone_plan(Symbol::qualified("synthetic", "cards"));
    let event = ContinuityEvent {
        event_id: Symbol::qualified("cards", "acknowledge-one"),
        kind: Symbol::new("acknowledge"), role: Symbol::new("phone"),
        payload: Expr::String("low-disclosure".into()), ..Default::default()
    };
    let mut journal = MemoryJournal::default();
    journal.accept(&plan, &ContinuityState::default(), event)
        .expect("bounded phone acknowledgement is complete");
    println!("{}", journal.turns().len());
}
