use sim::kernel::{Cx, GrantSeat};
use sim_lib_cookbook::CookbookCapabilityProfile;

pub(crate) fn seat_cookbook_capabilities(seat: &GrantSeat, cx: &mut Cx) {
    CookbookCapabilityProfile::seat(seat, cx).unwrap();
}
