//! Actual hand-controller lifecycle and board contact producer binding.
use skate3_core::physics::board_ground::BoardGroundState;
pub(crate) use skate3_core::physics::skateboard_controller::SkateboardController;

pub(crate) fn partial_request(
    controller: &SkateboardController,
    ground: &BoardGroundState,
) -> bool {
    //82C08450..8494 counts Board844..850 into byte868. This is not a
    //geometric overlap estimate or the separate four-wheel count869.
    controller.request_partial_ragdoll(ground.part_contact_count)
}
