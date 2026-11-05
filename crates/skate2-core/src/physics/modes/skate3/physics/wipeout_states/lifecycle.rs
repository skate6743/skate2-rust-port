//! Original Wipeout Enter82D3B5E8 and Exit82D3B990.
use super::*;
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    animation::foot_ik::status::Mode,
    math::Vector3,
    physics::{board::BodyId, contact::RetailContactMaterial},
    player::wipeout_state::{body, math},
};

pub(crate) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.ground_lifecycle.skeleton_elapsed_16505 = false;
    let runtime = &mut skater.wipeout_state;
    runtime.state = State::default();
    runtime.contact.reset();
    runtime.prediction.reset();
    physics.board_wiping_out = true;
    physics.board.set_collision_group(7);
    physics.board.bodies_mut()[BodyId::Deck.index()]
        .inertia
        .angular_drag = 0.0;
    let material = RetailContactMaterial {
        static_friction: runtime.settings.board_friction,
        dynamic_friction: runtime.settings.board_friction,
        restitution: runtime.settings.board_restitution,
    };
    physics.settings.wheel_material = material;
    physics.settings.truck_material = material;
    physics.settings.deck_material = material;
    skater.ground.steering.targets = [0.0; 2];
    let drive = &mut physics.board.hook_mut().drive;
    drive.disable_angular();
    drive.disable_linear();
    //Do not reset the target history or the other limb ownership flags.
    for limb in &mut skater.foot_ik.state.limbs {
        limb.board_blend = 0.0;
        limb.external_blend = 0.0;
        limb.mode = Mode::Disabled;
    }
    skater.foot_ik.state.enable_feet(false);
    drives::set_angular_root(&mut skater.skeleton_drives, &runtime.drives, 0.5);
    drives::set_linear_root(&mut skater.skeleton_drives, &runtime.drives, 0.0);
    let p = &skater.player_input.processed;
    if p.flags_2488 & 0x0020_0000 != 0 {
        body::blend_velocity(
            &mut skater.skeleton,
            p.vectors_880_896_912_928_944[4].map(f32::from_bits),
        );
    }
    if skater.wipeout.state.reasons[6] {
        let normal = p.vectors_544_560_592_608[0].map(f32::from_bits);
        body::remove_normal_velocity(&mut skater.skeleton, normal);
        for part in physics.board.bodies_mut() {
            let v = part.rates.linear_velocity;
            let next = math::sub(
                [v.x, v.y, v.z, 0.0],
                math::scale(normal, math::dot(normal, [v.x, v.y, v.z, 0.0])),
            );
            part.rates.linear_velocity = Vector3::new(next[0], next[1], next[2]);
        }
        physics.board.bodies_mut()[BodyId::Deck.index()]
            .rates
            .angular_velocity = Vector3::ZERO;
    }
    body::limit_velocity(&mut skater.skeleton, 20.0);
    if p.flags_2468 & 4 != 0 {
        skater.skeleton.apply_part_displacement(
            12,
            math::scale(
                p.vector_1520.map(f32::from_bits),
                runtime.settings.push_force,
            ),
        );
    }
    if p.probe_1792.byte_104 != 0 {
        body::add_velocity(
            &mut skater.skeleton,
            p.probe_1792.vector_80.map(f32::from_bits),
        );
    }
    runtime.state.special_surface = p.flags_2488 & 0x4000_0000 != 0;
    runtime.state.surface_height = p.collision_scalar_2924;
    runtime.ragdoll.request(
        &mut skater.ground_lifecycle.skeleton_controller,
        if runtime.state.special_surface { 10 } else { 8 },
        &mut skater.skeleton,
        &mut skater.skeleton_joints,
        &mut skater.skeleton_collision,
    )?;
    for part in skater.skeleton.bodies_mut() {
        part.inertia.linear_drag = 0.0;
    }
    runtime.state.move_board = p.flags_2472 & 0x800 != 0;
    runtime.state.board_offset = p.vectors_720_784_800_816_832_864[3].map(f32::from_bits);
    runtime.state.board_move_frames = 0;
    runtime.state.predicted_time = f32::MAX;
    runtime.state.airborne_frames = 15;
    if p.flags_2468 & 8 != 0 {
        body::set_velocity(
            &mut skater.skeleton,
            p.vectors_544_560_592_608[3].map(f32::from_bits),
        );
    }
    runtime.state.velocity = skater.animated_skeleton.board_frames.com_velocity;
    Ok(())
}

pub(crate) fn exit(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    physics.board_wiping_out = false;
    let runtime = &mut skater.wipeout_state;
    physics.board.bodies_mut()[BodyId::Deck.index()]
        .inertia
        .angular_drag = runtime.settings.deck_angular_drag;
    physics.board.set_collision_group(4);
    [
        physics.settings.wheel_material,
        physics.settings.truck_material,
        physics.settings.deck_material,
    ] = runtime.settings.standard_materials;
    runtime.state.prevent_manual = false;
}

pub(crate) fn post_physics(skater: &mut Skate3ControllerState) {
    skater.wipeout_state.state.post_physics(
        skater.skeleton.record.velocities[23],
        skater.skeleton.record.velocities[1],
        skater.collision_feedback.flags.compliant,
        &skater.wipeout_state.settings.recovery,
    );
}
