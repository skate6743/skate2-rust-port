//! Original landing Begin/Update/End effects, called by the production graph.
use super::{
    motion_animation::MotionAnimation,
    motion_landing::{Flags, Operation, Physical, State},
    motion_riding_conditions::MotionRandom,
};
use skate3_core::animation::{
    playback_parameters::{AttributeSink, SettableAttribute},
    skeleton_input::name::encode,
};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    operation: &Operation,
    state: &mut State,
    flags: &mut Flags,
    animation: &mut MotionAnimation,
    random: &MotionRandom,
    physical: Option<Physical>,
    mirrored: Option<bool>,
    dt: f32,
    phase: u8,
) -> Result<(), String> {
    match *operation {
        Operation::IsAnticipating => {
            //82BACB40/82BACB90: v80 ->8258F870, CA4 bit30.
            if phase != 1 {
                flags.anticipating = phase == 0;
            }
        }
        Operation::IsLanding => {
            //82BACBE0/82BACC30: v88 ->8258F898, CA4 bit29.
            if phase != 1 {
                flags.landing = phase == 0;
            }
        }
        Operation::IsManualing => {
            //82BACC80/82BACCD0: v96 ->8258F8C0, CA4 bit28.
            if phase != 1 {
                flags.manualing = phase == 0;
            }
        }
        Operation::IsDoingTrick => {
            //82BBDFF8/82BBE048: v104 ->8258F8E8, CA4 bit27.
            if phase != 1 {
                flags.doing_trick = phase == 0;
            }
        }
        Operation::SetLandingData => match phase {
            0 => {
                //82BB0180: sign follows ISkaterAnim v28, then MG v36.
                let p = physical.ok_or("SetLandingData requires actual landing publication")?;
                let mirror = mirrored.ok_or("SetLandingData requires animation stance")?;
                set(animation, b"Spin", if mirror { p.spin } else { -p.spin });
                set(animation, b"AvgVelY", p.last_good_landing_velocity);
                state.value = p.height;
            }
            1 => set(animation, b"disttocog", state.value), //82BB0338.
            _ => {}                                         //Original End82B61BB8 is empty.
        },
        Operation::DisableTricks { length } => match phase {
            0 => {
                //82BB8DC8.
                flags.tricks_allowed = false;
                state.value = length;
                state.complete = false;
            }
            1 => {
                //82BB8E58; strict comparison, no reference counting.
                if !state.complete {
                    state.value -= dt;
                    if state.value < 0.0 {
                        flags.tricks_allowed = true;
                        state.complete = true;
                    }
                }
            }
            _ => flags.tricks_allowed = true, //82BB8F20 unconditionally.
        },
        Operation::ChooseRandomLanding { count } => match phase {
            0 => {
                //82BB0398 draws from the SAME specific MotionGraph RNG v256.
                //The original traps for a nonpositive divisor; reject that asset.
                if count <= 0 {
                    return Err("ChooseRandomLanding requires numlandings > 0".into());
                }
                let value = (random.next_u32()? as i32).wrapping_abs() % count;
                let name = encode(b"Random");
                let value = encode(value.to_string().as_bytes());
                if let Some(item) = animation
                    .construction_values
                    .iter_mut()
                    .find(|(key, _)| *key == name)
                {
                    item.1 = value;
                } else {
                    animation.construction_values.push((name, value));
                }
            }
            2 => animation
                .construction_values
                .retain(|(name, _)| *name != encode(b"Random")),
            _ => {} //Update82B61BB8 is empty; End82BB0498 erases the key.
        },
    }
    Ok(())
}
fn set(animation: &mut MotionAnimation, name: &[u8], value: f32) {
    animation.set_attribute(SettableAttribute {
        name: encode(name),
        value,
        normalized: false,
        sequence_id: -1,
    });
}
