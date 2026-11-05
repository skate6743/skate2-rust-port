//! Six native substate1 dispatches. Manager query/family selection is upstream.
use super::super::{Skate3PhysicsState, Skate3ControllerState};
use super::arithmetic::dot3;
use super::{Family, ManagerObservation, board::apply_world_force, forces, lifecycle};
use skate3_core::{
    math::{Basis3, Vector3},
    physics::{
        drive_frames::RetailAffineTransform,
        grind_forces::{noise, orientation, slide, support},
    },
};
type V = [f32; 4];

pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    manager: &ManagerObservation,
) -> Result<(), String> {
    let family = skater
        .grind
        .active
        .ok_or("Grind contact requires active family")?;
    let toolkit = *skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Grind contact requires board toolkit")?;
    let board_frame = toolkit.deck;
    let p = &skater.player_input.processed;
    let velocity = p.vectors_400_416[0].map(f32::from_bits);
    let mut state = skater.grind.states[family as usize];
    let position = board_frame[3];
    let normal = state.output.normal;
    let across = state.output.across;
    let direction = state.output.direction;
    let mut operations = Vec::with_capacity(8);
    // Preserve the original order, including force calls surrounding orientation.
    use Op::*;
    match family {
        Family::Darkslide | Family::Boardslide => operations.extend([
            Orient,
            Translate,
            Pin,
            Friction([55., 55., 60.]),
            Coping,
            ExitLean,
            Wheels,
        ]),
        Family::FiftyFifty => operations.extend([
            Friction([50., 40., 40.]),
            Align(800., 0., 0.07),
            Coping,
            Pin,
            ExitLean,
            Orient,
        ]),
        Family::Tipslide => operations.extend([
            Orient,
            TipStability,
            Friction([60.; 3]),
            Coping,
            Pin,
            ExitLean,
            Wheels,
        ]),
        Family::FiveO => operations.extend([
            Orient,
            Align(3000., 0.234, 0.07),
            Pin,
            Friction([50., 40., 40.]),
            Coping,
            ExitLean,
        ]),
        Family::Backslash => operations.extend([
            Orient,
            Align(3000., 0.39, 0.025),
            Friction([70., 80., 80.]),
            Coping,
            Pin,
            ExitLean,
            Wheels,
        ]),
    }
    for operation in operations {
        match operation {
            Orient => {
                let target = orientation::target(
                    family as u32,
                    orientation::Input {
                        board: board_frame,
                        direction,
                        normal,
                        point: manager.geometry.point_1120,
                        yaw_1504: p.grind.yaw_1504,
                        pitch_1508: p.grind.pitch_1508,
                        switched: p.flags_2468 & 0x0010_0000 != 0,
                    },
                )?;
                //82D40890 blends from the current physical board, not the
                //preceding target;82D40A98 adds noise after that blend.
                let mut target_frame = orientation::blend(board_frame, target.frame);
                if target.noise_amount > 0. {
                    let draws = skater.grind.take_orientation_noise();
                    target_frame =
                        noise::apply(target_frame, p.scalar_2652, target.noise_amount, draws);
                }
                state.frame = target_frame;
                physics.board.set_hook_transform(RetailAffineTransform {
                    basis: Basis3 {
                        columns: core::array::from_fn(|i| {
                            [state.frame[i][0], state.frame[i][1], state.frame[i][2]]
                        }),
                    },
                    translation: Vector3::new(
                        state.frame[3][0],
                        state.frame[3][1],
                        state.frame[3][2],
                    ),
                });
                physics
                    .board
                    .hook_mut()
                    .drive
                    .enable_angular_only(&mut skater.ground_lifecycle.board_animated_290);
                if matches!(family, Family::FiveO | Family::FiftyFifty) {
                    apply_world_force(
                        &mut physics.board,
                        support::truck_compensation(normal, p.grind.pitch_1508),
                        position,
                    );
                }
            }
            Friction(strengths) => forces::friction(
                &mut physics.board,
                manager,
                position,
                velocity,
                normal,
                strengths,
            ),
            Align(strength, forward, up) => forces::lateral_pin(
                &mut physics.board,
                manager,
                board_frame,
                across,
                normal,
                velocity,
                strength,
                forward,
                up,
                &skater.grind.settings.pin_vs_slope,
            ),
            Pin => {
                if let Some(f) = support::pin(normal, manager.surface.gravity_relief_1512) {
                    apply_world_force(&mut physics.board, f, position);
                }
            }
            Coping => {
                if let Some(f) = support::coping(
                    manager.geometry.upmost_normal_1408,
                    velocity,
                    manager.surface.gravity_relief_1512,
                ) {
                    apply_world_force(&mut physics.board, f, position);
                }
            }
            ExitLean => {
                if let Some(f) = support::exit_lean(
                    manager.geometry.high_side_1440,
                    manager.surface.reckon_blend_selector_1500,
                    &skater.grind.settings.exit_assist,
                ) {
                    apply_world_force(&mut physics.board, f, position);
                }
            }
            Wheels => lifecycle::manage_wheel_spin(
                &mut physics.board,
                core::array::from_fn(|i| physics.riding.ground.parts[i].in_contact),
            ),
            Translate => {
                state.preparing_jump |= p.flags_2468 & 0x0020_0000 != 0;
                forces::slide(
                    &mut physics.board,
                    if family == Family::Darkslide {
                        slide::Slide::Darkslide
                    } else {
                        slide::Slide::Boardslide
                    },
                    slide::Input {
                        position,
                        point: manager.geometry.point_1120,
                        direction,
                        across,
                        velocity,
                        translation_2796: manager.control.translation_2796,
                        total_mass_2660: toolkit.total_mass,
                        update_frequency: physics.settings.step.simulation.frequency,
                        preparing_jump: state.preparing_jump,
                        geometry_kind: manager.geometry.kind_1464,
                    },
                );
            }
            TipStability => {
                let offset = sub(position, manager.geometry.point_1120);
                let inward = scale(across, if dot3(across, offset) > 0. { -1. } else { 1. });
                let amount = if manager.control.flags_1516 & 0x0200_0000 != 0 {
                    39.
                } else {
                    (if dot3(normal, cross(direction, offset)) > 0. {
                        manager.control.balance_2800
                    } else {
                        -manager.control.balance_2800
                    }) * 37.
                };
                apply_world_force(&mut physics.board, scale(inward, amount), position);
            }
        }
    }
    if family == Family::Darkslide {
        // Darkslide contact82D41194..1234 alone writes classification104 here.
        state.output.classification_104 = if manager.geometry.kind_1464 == 2 {
            if dot3(manager.geometry.high_side_1440, toolkit.effective[2]) > 0. {
                2
            } else {
                1
            }
        } else {
            let perpendicular = scale(
                across,
                dot3(sub(position, manager.geometry.point_1120), across),
            );
            if dot3(perpendicular, toolkit.effective[2]) > 0. {
                1
            } else {
                2
            }
        };
    }
    skater.grind.states[family as usize] = state;
    Ok(())
}
enum Op {
    Orient,
    Translate,
    Pin,
    Friction([f32; 3]),
    Coping,
    ExitLean,
    Wheels,
    Align(f32, f32, f32),
    TipStability,
}
fn scale(v: V, s: f32) -> V {
    v.map(|x| x * s)
}
fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
