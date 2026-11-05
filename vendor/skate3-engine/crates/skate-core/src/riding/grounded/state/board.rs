//! Branch composition of `PhysicsGround::UpdateSkateboard` (`82D38800`).

use crate::riding::{speed_wobble, steering};

use super::{
    board_types::{
        GroundBoardComponents, GroundBoardError, GroundBoardInput, GroundBoardServices,
        GroundBoardSettings, GroundBoardStage,
    },
    data::PhysicsGroundState,
    ordinary,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundBoardOutcome {
    Animated,
    Collision { tag_15_queued: bool },
    Ordinary(ordinary::OrdinaryGroundResult),
}

/// Runs the native steering/contact branch prefix, then exactly one of the
/// animated, collision-response, or ordinary grounded paths.
pub fn update<S: GroundBoardServices>(
    state: &mut PhysicsGroundState,
    components: GroundBoardComponents<'_, '_>,
    settings: GroundBoardSettings<'_>,
    mut input: GroundBoardInput,
    services: &mut S,
) -> Result<GroundBoardOutcome, GroundBoardError<S::BoardError>> {
    let tilt = steering::calculate_tilt(
        settings.steering,
        input.steering,
        Some(&mut state.steering_push_scalar_2640),
        Some(&mut state.steering_damped_turn_2644),
    );
    input.speed_wobble.tilt = tilt;
    input.speed_wobble.center_of_mass_height = service(
        GroundBoardStage::CenterOfMassHeight,
        services.center_of_mass_height_82d38838(),
    )?;
    let tilt = speed_wobble::calculate(
        components.speed_wobble,
        settings.speed_wobble,
        input.speed_wobble,
    );
    components.truck_steering.update(
        tilt,
        settings.steering.tilt_blending,
        input.truck_flags_2468,
        input.truck_flags_2472,
    );

    service(
        GroundBoardStage::SetWheelMaterials,
        services.set_contact_wheel_materials(),
    )?;
    let contact = service(
        GroundBoardStage::ContactResponse,
        services.contact_response_82d93df0(input.contact, state.vector_2688),
    )?;
    state.flag_2731 = contact.active_2731;
    state.vector_2688 = contact.vector_2688;
    state.scalar_2704 = contact.scalar_2704;
    state.flag_2708 = contact.animated_board_2708;
    state.flag_2720 = false;

    service(
        GroundBoardStage::UpdateBodyAccumulator,
        services.update_body_accumulator_82d389dc(),
    )?;
    if state.flag_2708 {
        return animated(state, components, services);
    }

    let collision = service(
        GroundBoardStage::CollisionForce,
        services.collision_force_82d944e8(input.ground_vector_1216),
    )?;
    if let Some(collision) = collision {
        state.collision_force_2528 = collision.force_2528;
        state.collision_point_2544 = collision.point_2544;
        state.vector_2592 = collision.vector_2592;
        state.collision_countdown_2652 = settings.collision_response_duration;
        service(
            GroundBoardStage::ApplyCollisionVector,
            services.apply_vector_82c07000(collision.vector_2592),
        )?;
        components.speed_model.flags_1360 |= 0x8000_0000;
        let queued = components.force_queue.append(collision.tagged_force());
        let projection = service(
            GroundBoardStage::CollisionProjection,
            services.collision_force_dot_velocity_82d38e18(
                collision.force_2528,
                input.ground_force.velocity_400,
            ),
        )?;
        state.flag_2722 = projection < -0.75;
        return Ok(GroundBoardOutcome::Collision {
            tag_15_queued: queued,
        });
    }

    let result = ordinary::update(
        state,
        components,
        settings,
        input,
        contact.tag_16_force,
        services,
    )?;
    Ok(GroundBoardOutcome::Ordinary(result))
}

fn animated<S: GroundBoardServices>(
    state: &mut PhysicsGroundState,
    components: GroundBoardComponents<'_, '_>,
    services: &mut S,
) -> Result<GroundBoardOutcome, GroundBoardError<S::BoardError>> {
    service(
        GroundBoardStage::SetAnimatedVelocity,
        services.set_animated_velocity_82c04168(state.vector_2688),
    )?;
    service(
        GroundBoardStage::WriteProcessedVelocity,
        services.write_processed_velocity_400(state.vector_2688),
    )?;
    components
        .inertias
        .set_linear_drag(0.0, crate::riding::grounded::drag::DragSelection::AllParts)
        .map_err(GroundBoardError::Drag)?;
    let pose = service(
        GroundBoardStage::BuildAnimatedPose,
        services.build_animated_pose_82d33448(),
    )?;
    service(
        GroundBoardStage::PublishAnimatedPose,
        services.publish_animated_pose_82be33d0(&pose),
    )?;
    service(
        GroundBoardStage::UpdateExternalPlayer,
        services.update_external_player_82d67848(&pose, state.vector_2688),
    )?;
    service(
        GroundBoardStage::CommitExternalPlayer,
        services.commit_external_player_82d68800(),
    )?;
    service(
        GroundBoardStage::FinalizeAnimatedBoard,
        services.finalize_animated_board_sk83_na_f_01a4(),
    )?;
    state.flag_2720 = true;
    Ok(GroundBoardOutcome::Animated)
}

pub(super) fn service<T, E>(
    stage: GroundBoardStage,
    result: Result<T, E>,
) -> Result<T, GroundBoardError<E>> {
    result.map_err(|source| GroundBoardError::Service { stage, source })
}
