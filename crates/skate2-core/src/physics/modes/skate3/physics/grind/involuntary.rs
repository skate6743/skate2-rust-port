//! Native family substate2 dispatch; no generic tipslide substitution.
use super::super::{Skate3PhysicsState, Skate3ControllerState};
use super::arithmetic::dot3;
use super::{Family, ManagerObservation, forces, lifecycle};
use skate3_core::physics::grind_forces::release::Input;

pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    manager: &ManagerObservation,
) -> Result<(), String> {
    let family = skater
        .grind
        .active
        .ok_or("Grind exit requires active family")?;
    let frame = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Grind exit requires BoardToolkit")?
        .deck;
    let state = &mut skater.grind.states[family as usize];
    let kind = manager.geometry.kind_1464;
    let mut input = Input {
        position: frame[3],
        point: manager.geometry.point_1120,
        across: state.output.across,
        normal: state.output.normal,
        velocity: skater.player_input.processed.vectors_400_416[0].map(f32::from_bits),
        geometry_kind: kind,
        high_side: manager.geometry.high_side_1440,
        force_across: false,
        enable_lift: true,
        strength: 100.,
        speed_limit: 1.,
        lift: 125.,
    };
    match family {
        Family::Boardslide | Family::Darkslide => {
            //82D41250: rail branches request10 and publish direction*136 to
            //Wipeout16; only the ledge branch applies the lateral exit force.
            if kind == 2 {
                input.strength = 90.;
                input.speed_limit = 0.75;
                input.enable_lift = false;
                forces::release(&mut physics.board, input);
            } else {
                state.output.slide_wipeout = true;
                state.output.slide_impulse = manager.geometry.direction_1136.map(|x| x * 136.);
                skater.wipeout.state.request(10, 0.);
            }
            lifecycle::clear_wheel_spin(&mut physics.board);
        }
        Family::FiftyFifty => {
            if kind >= 2 {
                input.strength = 10.;
                input.lift = 0.;
            }
            forces::release(&mut physics.board, input);
        }
        Family::FiveO => {
            input.strength = if kind < 2 { 120. } else { 10. };
            input.lift = if kind < 2 { 125. } else { 0. };
            input.speed_limit = 0.9;
            // Raw82D42948/4C passes computed force-across in r8 and lift in r7.
            input.force_across = kind == 1 && dot3(frame[2], input.across).abs() > 0.23;
            forces::release(&mut physics.board, input);
        }
        Family::Backslash => {
            input.speed_limit = 0.75;
            input.force_across = kind == 1;
            input.enable_lift = kind == 1;
            forces::release(&mut physics.board, input);
            lifecycle::clear_wheel_spin(&mut physics.board);
        }
        Family::Tipslide => {
            lifecycle::clear_wheel_spin(&mut physics.board);
            if manager.geometry.flags_1476 & 0x0800_0000 != 0 || state.output.tipslide_97_98_99[0] {
                if !state.output.tipslide_97_98_99[0] {
                    physics
                        .board
                        .hook_mut()
                        .drive
                        .disable_animation(&mut skater.ground_lifecycle.board_animated_290);
                    state.output.tipslide_97_98_99[0] = true;
                    state.output.tipslide_97_98_99[2] = true;
                }
                super::tip_nudge::apply(
                    &mut physics.board,
                    input.position,
                    input.point,
                    input.across,
                )?;
            } else {
                physics
                    .board
                    .hook_mut()
                    .drive
                    .enable_angular_only(&mut skater.ground_lifecycle.board_animated_290);
                input.speed_limit = 1.5;
                //82D420EC/F0: r8=1 (across), r7=0 (no normal lift).
                input.force_across = true;
                input.enable_lift = false;
                forces::release(&mut physics.board, input);
                state.output.tipslide_97_98_99[1] = true;
                if manager.control.flags_2488 & 0x1000_0000 != 0 {
                    state.output.tipslide_97_98_99[2] = true;
                }
            }
        }
    }
    Ok(())
}
