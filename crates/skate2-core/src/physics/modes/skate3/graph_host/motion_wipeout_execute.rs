//! Original82BBFF88/82BC01C0; graph ownership is ours, filter order is stock.
use super::super::motion_wipeout::{Operation, Settings};
use super::*;

fn bounded(value: f32, limit: f32) -> f32 {
    //Original fsel comparisons, preserving unordered and signed-zero behavior.
    let value = if -limit - value >= 0.0 { -limit } else { value };
    if limit - value >= 0.0 { value } else { limit }
}
fn positive_turn(value: f32) -> f32 {
    if value >= 0.0 {
        value
    } else {
        value + f32::from_bits(0x40C9_0FDB)
    }
}
impl MotionHost {
    pub(super) fn execute_wipeout(
        &mut self,
        behavior: BehaviorId,
        operation: Operation,
        phase: u8,
    ) -> Result<(), String> {
        if operation == Operation::EnableGestures {
            //82BC07A8/07F8, Update82B61BB8 is empty.
            if phase != 1 {
                self.wipeout_controls.gestures_enabled = phase == 0;
            }
            return Ok(());
        }
        if phase > 1 {
            return Ok(());
        } //Original End82B61BB8.
        let Instance::Wipeout(state) = self
            .instances
            .get_mut(behavior)
            .ok_or("Unallocated Wipeout behavior")?
        else {
            return Err("Wipeout operation/instance mismatch".into());
        };
        let physical = self
            .wipeout_physical
            .ok_or("Wipeout requires completed physical outputs")?;
        if phase == 0 {
            let mirrored = self
                .animation
                .skater_animation_flags
                .ok_or("Wipeout requires animation stance")?
                & 0x4000_0000
                != 0;
            state.ticks = 0;
            state.lean = physical.hips_right_angle_496;
            let twist = if mirrored {
                -physical.hips_up_angle_500
            } else {
                physical.hips_up_angle_500
            };
            state.twist = positive_turn(twist);
            state.twist_velocity = 0.0;
            state.released = self.wipeout_controls.seed_from_air_tweak;
            state.gesture = if state.released {
                self.wipeout_controls.seed_from_air_tweak = false;
                [
                    self.action_controls.wipeout.air_body_tweak[0],
                    self.action_controls.wipeout.air_body_tweak[1],
                ]
            } else {
                [0.0; 2]
            };
            return Ok(());
        }
        if self.action_controls.wipeout.request || (state.ticks as i32) > 80 {
            state.released = true;
        }
        if self.wipeout_controls.gestures_enabled {
            let mirrored = self
                .animation
                .skater_animation_flags
                .ok_or("Wipeout requires animation stance")?
                & 0x4000_0000
                != 0;
            let Settings {
                threshold,
                twist_velocity,
                twist_acceleration,
                lean_velocity,
                twist_blend,
                lean_blend,
                gesture_y_blend,
                gesture_x_blend,
            } = self.wipeout_settings;
            let delta = ((1.0 - lean_blend) * state.lean
                + physical.hips_right_angle_496 * lean_blend)
                - state.lean;
            state.lean += bounded(delta, lean_velocity);
            set(&mut self.animation, b"Lean", state.lean);
            let twist = positive_turn(if mirrored {
                -physical.hips_up_angle_500
            } else {
                physical.hips_up_angle_500
            });
            let mut delta = twist - state.twist;
            let pi = f32::from_bits(0x4049_0FDB);
            let turn = f32::from_bits(0x40C9_0FDB);
            if delta > pi {
                delta -= turn;
            } else if delta < -pi {
                delta += turn;
            }
            let delta = ((state.twist + delta) * twist_blend + (1.0 - twist_blend) * state.twist)
                - state.twist;
            let mut velocity = bounded(delta, twist_velocity);
            if state.ticks >= 2 {
                velocity = bounded(velocity - state.twist_velocity, twist_acceleration)
                    + state.twist_velocity;
            }
            let mut twist = state.twist + velocity;
            if twist > turn {
                twist -= turn;
            }
            state.twist_velocity = velocity;
            state.twist = positive_turn(twist);
            set(&mut self.animation, b"Twist", state.twist);
            let gesture = self.action_controls.wipeout.gesture;
            let x = gesture.map(|values| values[0]);
            let y = gesture.map(|values| values[1]);
            let xv = x.unwrap_or(0.0);
            let yv = y.unwrap_or(0.0);
            if !state.released && x.is_some() && y.is_some() && xv.abs() < 0.1 && yv.abs() < 0.1 {
                state.released = true;
            }
            if state.released {
                state.gesture[0] =
                    (1.0 - gesture_x_blend) * state.gesture[0] + xv * gesture_x_blend;
                state.gesture[1] =
                    (1.0 - gesture_y_blend) * state.gesture[1] + yv * gesture_y_blend;
            }
            if state.gesture[1] * state.gesture[1] + state.gesture[0] * state.gesture[0]
                > threshold * threshold
            {
                self.animation
                    .emit_packet(encode(b"ControlledWipeout"), 1.0);
            }
            self.wipeout_controls.gesture = state.gesture; //MGv2528258FB50.
        }
        set(&mut self.animation, b"WipeoutGestureX", state.gesture[0]);
        set(&mut self.animation, b"WipeoutGestureY", state.gesture[1]);
        state.ticks = state.ticks.wrapping_add(1);
        Ok(())
    }
}

fn set(animation: &mut MotionAnimation, name: &[u8], value: f32) {
    animation.set_attribute(SettableAttribute {
        name: encode(name),
        value,
        normalized: false,
        sequence_id: -1,
    });
}

impl MotionHost {
    pub(super) fn execute_twist_lean(
        &mut self,
        behavior: BehaviorId,
        operation: super::super::motion_twist_lean::Operation,
        phase: u8,
    ) -> Result<(), String> {
        let Instance::TwistLean(state) = self
            .instances
            .get_mut(behavior)
            .ok_or("Unallocated MatchTwistAndLean behavior")?
        else {
            return Err("MatchTwistAndLean operation/instance mismatch".into());
        };
        let observation = if phase == 0 || (phase == 1 && operation.always) {
            let p = self
                .wipeout_physical
                .ok_or("MatchTwistAndLean requires canonical physical angles")?;
            let flags = self
                .animation
                .skater_animation_flags
                .ok_or("MatchTwistAndLean requires live animation stance")?;
            Some(skate3_core::player::offboard::twist_lean::Observation {
                hips_right_angle: p.hips_right_angle_496,
                hips_up_angle: p.hips_up_angle_500,
                mirrored: flags & 0x4000_0000 != 0,
            })
        } else {
            None
        };
        operation.execute(state, observation, Some(&mut self.animation), phase);
        Ok(())
    }
}
