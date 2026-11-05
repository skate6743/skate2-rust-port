//! Original force leaves wired to CURRENT's immediate deck accumulator.
use super::{ManagerObservation, board::apply_world_force};
use skate3_core::{
    physics::{board_runtime::BoardRuntime, grind_forces},
    point_graph::PointGraph,
};
type V = [f32; 4];

///82D3FD88. The family owns its three strengths and position/velocity snapshot.
pub(crate) fn friction(
    board: &mut BoardRuntime,
    manager: &ManagerObservation,
    position: V,
    velocity: V,
    normal: V,
    strengths: [f32; 3],
) {
    let force = grind_forces::friction(
        velocity,
        normal,
        manager.geometry.upmost_normal_1408,
        manager.surface.friction_vs_time_1496,
        manager.geometry.flags_1476 & 0x4000_0000 != 0,
        grind_forces::material_multiplier(manager.surface.material_1472),
        manager.geometry.kind_1464,
        strengths,
    );
    // Native skips AddForce for speed<=.001, not an unconditional zero-force
    // call (which would still wake the body). The leaf exposes that separately.
    if grind_forces::friction_applies(velocity, normal) {
        apply_world_force(board, force, position);
    }
}

pub(crate) fn lateral_pin(
    board: &mut BoardRuntime,
    manager: &ManagerObservation,
    frame: [V; 4],
    across: V,
    normal: V,
    velocity: V,
    strength: f32,
    forward_offset: f32,
    up_offset: f32,
    pin_vs_slope: &PointGraph<4>,
) {
    let slope = grind_forces::pin_slope(
        normal,
        manager.geometry.upmost_normal_1408,
        manager.surface.gravity_relief_1512,
        pin_vs_slope,
    );
    let force = grind_forces::lateral_pin(
        frame,
        manager.geometry.point_1120,
        across,
        velocity,
        strength,
        forward_offset,
        up_offset,
        manager.control.flags_1516 & 0x2000_0000 != 0,
        slope,
    );
    if let Some(force) = force {
        apply_world_force(board, force, frame[3]);
    }
}

pub(crate) fn slide(
    board: &mut BoardRuntime,
    family: grind_forces::slide::Slide,
    input: grind_forces::slide::Input,
) {
    for force in grind_forces::slide::control(family, input) {
        apply_world_force(board, force, input.position);
    }
}

pub(crate) fn release(board: &mut BoardRuntime, input: grind_forces::release::Input) {
    if let Some(force) = grind_forces::release::force(input) {
        apply_world_force(board, force, input.position);
    }
}
