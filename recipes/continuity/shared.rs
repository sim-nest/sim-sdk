use sim::continuity::{ContinuityPlan, NetworkPolicy, RoleDemand};
use sim::kernel::Symbol;

pub fn phone_plan(plan_id: Symbol) -> ContinuityPlan {
    let services = ["phone-review", "lifecycle", "capture", "render", "stop", "journal-append"]
        .into_iter().map(Symbol::new).collect::<Vec<_>>();
    ContinuityPlan {
        plan_id,
        roles: vec![RoleDemand { role: Symbol::new("phone"), root: true,
            required_services: services.clone(), fallbacks: vec![] }],
        available_services: services,
        retention_turns: 8,
        network: NetworkPolicy::Offline,
        ..ContinuityPlan::default()
    }
}
