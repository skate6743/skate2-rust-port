//! Ground Post82D32AA0 and complete typed Fill82D32D38/82D785F8.
//! Parent publishes every returned field through the shared output records.
use super::Skate3ControllerState;
use skate3_core::player::offboard::{
    board_possession::manager::State as FeetState,
    ground_lifecycle::{self as core, CollisionInput, CollisionSettings, Publication},
};
use skate_data::collections::Collections;

/// Apply these AFTER the shared state template/reset, not before it.
#[must_use = "Ground State36/40/86 must survive the shared output reset"]
pub(crate) struct StatePublication {
    pub word_36: u32,
    pub word_40: u32,
    pub flag_86: bool,
}

/// Apply every represented record field and return state-template writes for
/// the parent's common publication reset/reconstruction.
/// The actual shared record type is PhysicalPlayerInput, not PhysicsOutputFields.
pub(crate) fn publish_fields(
    p: Publication,
    physical: &mut skate3_core::player::input_phase::PhysicalPlayerInput,
) -> StatePublication {
    let o = &mut physical.off_board;
    o.flag_304 = u8::from(p.offboard_flag_304);
    o.kind_88 = p.offboard_kind_88;
    o.scalar_112 = p.offboard_scalar_112;
    o.flag_329 = u8::from(p.offboard_flag_329);
    o.flag_330 = u8::from(p.offboard_flag_330);
    o.distance_116 = p.offboard_distance_116;
    o.flag_334 = u8::from(p.offboard_flag_334);
    o.scalar_32 = p.offboard_scalar_32;
    o.flag_328 = u8::from(p.offboard_flag_328);
    o.flags_306_307 = p.offboard_hand_flags_306_307.map(u8::from);
    physical.reckoning.vector_144 = p.animation_vector_144.map(f32::to_bits);
    physical.reckoning.flag_164 = u8::from(p.animation_flag_164);
    physical.state.counter_36 = p.physics_counter_36;
    physical.state.skitch_value_40 = p.physics_counter_40;
    StatePublication {
        word_36: p.physics_counter_36,
        word_40: p.physics_counter_40,
        flag_86: p.physics_flag_86,
    }
}

pub(super) fn load_collision_settings(data: &Collections) -> Result<CollisionSettings, String> {
    let value = |name| data.float("physics_wipeout", "default", name);
    Ok(CollisionSettings {
        vehicle_scalar: value("Wipeout_OB_VehicleScalar")?,
        vehicle_contact: value("Wipeout_OB_VehicleContact")?,
        maximum_displacement: value("Wipeout_OB_SkeletonMaxDisp")?,
        maximum_arm_contact: value("Wipeout_OB_SkeletonMaxContactArms")?,
        maximum_body_contact: value("Wipeout_OB_SkeletonMaxContact")?,
        minimum_speed: value("Wipeout_OB_MinSpeed")?,
        maximum_squash: value("Wipeout_OB_MaxSquash")?,
        special_scalar: value("Hash_472174920C68FBE3")?,
    })
}

/// Called AFTER skeleton feedback, before shared FootIK Post consumes requests.
/// `collision` borrows the completed canonical Frame and real Skeleton16336 /
/// SkeletonCollision4072/4056; there is no pre-solve/default observation path.
pub(crate) fn post(skater: &mut Skate3ControllerState, collision: CollisionInput<'_>) {
    let p = &skater.player_input.processed;
    let owner = &mut skater.biped_ground;
    core::post_physics(
        &mut owner.ground,
        &mut skater.wipeout.state,
        &owner.collision_settings,
        &core::PostInput {
            collision,
            processed_flags_2484: p.flags_2484,
            processed_velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
            skeleton_displacement_16288: skater.collision_extra_errors[0],
            skeleton_displacement_16304: skater.collision_extra_errors[1],
            ground_kind_356: owner.contact.kind_164,
        },
    );
}

/// No output is silently dropped: the full packet includes State36/40/86,
/// OffBoard88/112/116/304/306/307/328/329/330/334, and Reckoning144/164.
/// Borrow the ONE state56 FeetIK owner (its historical type calls feet hands).
pub(crate) fn fill(skater: &Skate3ControllerState, feet: &FeetState) -> Result<Publication, String> {
    let owner = &skater.biped_ground;
    let motion = owner
        .result
        .ok_or("Ground Fill requires completed Ground motion")?;
    let geometry = owner
        .geometry_adjustment
        .ok_or("Ground Fill requires this tick's real geometry consumption")?;
    let position = geometry.frame_768.position;
    Ok(core::publish(
        &owner.ground,
        &core::PublicationInput {
            ground_flags_752_to_754: [geometry.state_752, geometry.state_753, geometry.state_754],
            contact_flags_368: owner.contact.flags_176,
            contact_position_192: owner.contact.position,
            query_position_816: [position.x, position.y, position.z, 0.],
            processed_flags_2476: skater.player_input.processed.flags_2476,
            ground_kind_356: owner.contact.kind_164,
            ground_scalar_360: owner.contact.distance_168,
            motion_vector_1040: motion.velocity,
            motion_up_864: motion.physical_frame[1],
            hand_flags: feet.hands.map(|foot| {
                [
                    foot.flags_104_to_107[1],
                    foot.flags_104_to_107[2],
                    foot.flags_104_to_107[3],
                ]
            }),
        },
    ))
}
