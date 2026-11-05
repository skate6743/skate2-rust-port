//! Wipeout300's material corrections and collision-policy requests82D3EC08.
use crate::skate3_native::physics::skeleton_controller::SkeletonControllerState;
use skate3_core::{
    physics::skeleton_body::{
        SkeletonBody, SkeletonCollisionFeedback, SkeletonCollisionMode, SkeletonJoints,
    },
    player::wipeout_state::{
        State,
        contact_response::{ContactResponse, ContactResponseInput},
    },
};

pub(crate) fn update(
    state: &mut State,
    response: &mut ContactResponse,
    body: &mut SkeletonBody,
    feedback: &SkeletonCollisionFeedback,
    controller: &mut SkeletonControllerState,
    collision: &mut SkeletonCollisionMode,
    joints: &mut SkeletonJoints,
    ragdoll: &super::ragdoll::RagdollSetup,
    contact: bool,
    com_velocity: [f32; 4],
    effective_axis: [f32; 4],
    controls: [f32; 2],
) -> Result<(), String> {
    let output = response.update(
        body,
        ContactResponseInput {
            com_velocity_608: com_velocity,
            material10_normal: feedback
                .flags
                .material_10
                .then_some(feedback.material_normals[0]),
            material11_normal: feedback
                .flags
                .material_11
                .then_some(feedback.material_normals[1]),
            effective_axis_224: effective_axis,
            wipeout_control_2824_2828: controls,
        },
    );
    state.material_ten_response = output.material10_finished_77;
    // The original comparison rejects <=0.3; unordered values retain true.
    let air_mode = !contact && !(state.predicted_time <= 0.3);
    let air_changed = state.air_collision_mode != air_mode;
    state.air_collision_mode = air_mode;
    let material_changed = state.material_eleven_response != output.material11_active_73;
    state.material_eleven_response = output.material11_active_73;
    if !state.special_surface && (air_changed || material_changed) {
        let requested = if state.material_eleven_response {
            9
        } else if air_mode {
            11
        } else {
            8
        };
        ragdoll.request(controller, requested, body, joints, collision)?;
    }
    Ok(())
}
