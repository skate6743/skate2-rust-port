//! Native common82D3F4E8 and Nonspecific82D42DF0 Update order.
use super::super::{Skate3PhysicsState, Skate3ControllerState};
use super::Family;
use super::arithmetic::dot3;
use skate3_core::physics::grind_forces::reckoning;
type V = [f32; 4];

pub(crate) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    if skater.grind.nonspecific_active {
        super::substate::execute_nonspecific(physics, skater)?;
        let frame = skater
            .player_input
            .toolkit
            .as_ref()
            .ok_or("701 requires BoardToolkit")?
            .deck;
        //82F825D0/F0 initialize830BD320/+4A0 from8231A844(+1)/8216DEE0(-1).
        let sign = if skater.player_input.processed.flags_2484 & 0x0020_0000 != 0 {
            -1.
        } else {
            1.
        };
        reckon(physics, skater, frame[1].map(|x| x * sign), frame[2], 0.98);
        return Ok(());
    }
    let family = skater
        .grind
        .active
        .ok_or("Grind Update requires active family")?;
    let manager = skater
        .grind
        .manager
        .ok_or("Grind Update requires manager observation")?;
    let frame = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Grind Update requires BoardToolkit")?
        .deck;
    let p = &skater.player_input.processed;
    let speed = p.scalar_2652;
    let index = family as usize;
    let state = &mut skater.grind.states[index];
    state.output.just_jumped = false;
    let tipped = state.output.tipslide_97_98_99[0];
    if !tipped {
        state.output.direction = manager.geometry.direction_1136;
        state.output.normal = manager.geometry.normal_1152;
        state.output.across = cross(state.output.direction, state.output.normal);
    }
    let normal = if tipped {
        frame[1]
    } else {
        manager.geometry.target_up_1168
    };
    let smoothing = if !tipped && manager.surface.reckon_blend_selector_1500 > 0. {
        0.9
    } else {
        0.7
    };
    reckon(physics, skater, normal, frame[2], smoothing);
    let state = &mut skater.grind.states[index];
    // Common UserTriggeredExit82D40D90 is processed2476 bit29. Slides'
    //82D41788 applies it only to ledges (kind2), preserving the native gate.
    let user_exit = skater.player_input.processed.flags_2476 & 0x2000_0000 != 0
        && (!matches!(family, Family::Boardslide | Family::Darkslide)
            || manager.geometry.kind_1464 == 2);
    let should_exit = (manager.engagement.kind_1248 == 2
        && (manager.geometry.flags_1476 & 0x0800_0000 != 0
            || manager.control.flags_2488 & 0x1000_0000 != 0)
        && speed < 1.)
        || (manager.control.flags_1516 as i32) < 0
        || user_exit
        || (state.updates > 60 && speed < 0.15);
    if state.output.substate == 1 && should_exit {
        state.output.substate = 2;
    }
    super::substate::execute(physics, skater, &manager)?;
    let state = &mut skater.grind.states[index];
    state.leaving_updates = if state.output.substate == 2 {
        state.leaving_updates.wrapping_add(1)
    } else {
        0
    };
    if state.leaving_updates > 35 {
        skater.wipeout.state.request(17, 0.);
    }
    state.output.crouch = dot3(frame[2], state.output.normal).abs() * 0.4;
    let p = &skater.player_input.processed;
    skater.ground.steering.update(
        0.,
        skater.air_settings.steering_blend,
        p.flags_2468,
        p.flags_2472,
    );
    let velocity = p.vectors_400_416[0].map(f32::from_bits);
    skater.skeleton_input.head_tracking_history[5] = core::array::from_fn(|i| {
        velocity[i].mul_add(
            skater.grind.settings.look_ahead,
            manager.geometry.point_1120[i],
        )
    });
    //82D3F614 sets r4=1; steering leaf preserves it to the82D3F6E0 store.
    skater.skeleton_input.head_tracking_active = true;
    state.updates = state.updates.wrapping_add(1);
    Ok(())
}

fn reckon(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    normal: V,
    heading: V,
    smoothing: f32,
) {
    reckoning::update(
        &mut physics.riding.reckoning,
        &mut physics.riding.reckoning_frames,
        &mut physics.riding.body_spin,
        &mut skater.air_reckoning.state,
        &skater.grind.settings.reckoning,
        normal,
        heading,
        smoothing,
        skater.player_input.processed.flags_2468 & 0x0010_0000 != 0,
        0.,
    );
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
