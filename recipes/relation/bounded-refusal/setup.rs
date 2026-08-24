use sim::relation::{plan::{AdmissionError, AdmissionLimits}, site::{LimitKind, Limits, SiteError}};
fn main() { let _ = (AdmissionLimits::default(), Limits::new(1, 1, 1, 1).unwrap(), std::any::TypeId::of::<AdmissionError>(), std::any::TypeId::of::<LimitKind>(), std::any::TypeId::of::<SiteError>()); }
