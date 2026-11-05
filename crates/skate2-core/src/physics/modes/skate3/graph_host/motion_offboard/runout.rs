//! AddRunoutAttribs: Begin82BBA4A8, Update82BBA770, state82BBA830.
//! End is82B61BB8. Values are sampled once per activation, never on Update.
use skate3_core::{
    animation::{
        playback_parameters::{AttributeSink, SettableAttribute},
        skeleton_input::name::encode,
    },
    player::wipeout_state::{math, orientation},
};
use skate_data::state_graph::attributes::Attributes;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operation;
impl Operation {
    pub fn parse(attributes: &Attributes<'_>) -> Option<Self> {
        (attributes.text("name") == Some("AddRunoutAttribs")).then_some(Self)
    }
}

///Borrowed values from the actual published physical bundle and animation owner.
#[derive(Clone, Copy, Debug)]
pub struct Observation {
    pub offboard_flag_331: bool,
    pub offboard_velocity_128: [f32; 4],
    pub reckoning_velocity_16: [f32; 4],
    pub reckoning_up_96: [f32; 4],
    ///Exact first vector of physical bundle20, loaded at82BBA620.
    pub skeleton_vector_0: [f32; 4],
    ///Live ISkaterAnim virtual28 result at Begin, not a retained pose mirror.
    pub animation_mirrored: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub angle_degrees_8: f32,
    ///Native constructor does not initialize12. None records that fact without
    ///inventing a zero speed if a Begin observation was unavailable.
    pub speed_12: Option<f32>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            angle_degrees_8: 0.0,
            speed_12: None,
        }
    }
}
impl State {
    ///None is permitted only for an actually absent native physical/animation
    ///component. An unavailable implementation must remain an upstream error.
    pub fn begin(&mut self, observation: Option<Observation>) {
        let Some(observation) = observation else {
            return;
        };
        let velocity = if observation.offboard_flag_331 {
            observation.offboard_velocity_128
        } else {
            observation.reckoning_velocity_16
        };
        let mut angle = orientation::projected_angle(
            observation.skeleton_vector_0,
            velocity,
            observation.reckoning_up_96,
        );
        if observation.animation_mirrored {
            angle = -angle;
        }
        self.speed_12 = Some(math::length(velocity));
        self.angle_degrees_8 = math::wrap_angle(angle) * f32::from_bits(0x4265_2EE1);
    }

    ///Write to the canonical playback queue in the original order. This does
    ///not modify the captured state. A genuinely absent animation skips Update.
    pub fn update(&self, animation: Option<&mut impl AttributeSink>) -> Result<(), &'static str> {
        let Some(animation) = animation else {
            return Ok(());
        };
        let speed = self
            .speed_12
            .ok_or("AddRunoutAttribs speed12 has no successful Begin observation")?;
        for (name, value) in [
            ("BipedStartAngle", self.angle_degrees_8),
            ("BipedSpeed", speed),
        ] {
            animation.set_attribute(SettableAttribute {
                name: encode(name.as_bytes()),
                value,
                normalized: false,
                sequence_id: -1,
            });
        }
        Ok(())
    }
}

#[cfg(any())]
#[path = "runout/tests.rs"]
mod tests;
