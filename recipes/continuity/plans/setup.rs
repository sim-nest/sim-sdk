use sim::continuity::{ContinuityPlan, NetworkPolicy, RoleDemand};
use sim::kernel::Symbol;

fn main() {
    let minimum = ContinuityPlan {
        plan_id: Symbol::qualified("continuity", "carrier-only"),
        roles: vec![RoleDemand {
            role: Symbol::new("android-root"),
            root: true,
            required_services: [
                "phone-review", "lifecycle", "mounts", "capture", "render", "stop",
                "journal-append",
            ].into_iter().map(Symbol::new).collect(),
            fallbacks: vec![],
        }],
        available_services: [
            "phone-review", "lifecycle", "mounts", "capture", "render", "stop",
            "journal-append",
        ].into_iter().map(Symbol::new).collect(),
        network: NetworkPolicy::Offline,
        ..ContinuityPlan::default()
    };
    minimum.validate().expect("the satellite-free plan is complete");
    println!("{}", minimum.plan_id);
}
