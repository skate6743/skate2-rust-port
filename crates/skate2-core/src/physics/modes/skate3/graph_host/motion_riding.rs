//! Native riding packet and timer leaves. The graph host owns these states;
//! physics/pose callers consume the resulting values in actor update order.
use super::motion_animation::MotionAnimation;
use skate3_core::{
    animation::{
        output::attributes::AttributeName,
        playback_parameters::{AttributeSink, SettableAttribute},
        skeleton_input::name::encode,
    },
    graph::controller::Frame,
};
use skate_data::state_graph::attributes::Attributes;

pub fn body_tilt_settings(
    data: &skate_data::collections::Collections,
) -> Result<skate3_core::animation::body_tilt::Settings, String> {
    let words = data.words::<12>("anim_motion", "body_tilt", "body_tilt_bodyspin_factor")?;
    let float = |name| data.float("anim_motion", "body_tilt", name);
    Ok(skate3_core::animation::body_tilt::Settings {
        body_spin_factor: skate3_core::point_graph::PointGraph {
            x: std::array::from_fn(|i| f32::from_bits(words[4 + i])),
            y: std::array::from_fn(|i| f32::from_bits(words[8 + i])),
        },
        ground_velocity: float("body_tilt_ground_clamp_vel")?,
        ground_acceleration: float("body_tilt_ground_clamp_acc")?,
        air_velocity: float("body_tilt_air_clamp_vel")?,
        air_acceleration: float("body_tilt_air_clamp_acc")?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceMode {
    AnimationData,
    PhysicsBoard,
    AnimationBoard,
}
impl ForceMode {
    pub fn native_value(self) -> u32 {
        match self {
            Self::AnimationData => 0,
            Self::PhysicsBoard => 1,
            Self::AnimationBoard => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RidingOperation {
    CreateAttribute {
        name: AttributeName,
        values: [Option<f32>; 3],
        set: bool,
    },
    SetDistComToBoard {
        name: AttributeName,
        manually: bool,
        adjust_for_velocity: bool,
    },
    ForcePhysics(ForceMode),
    TimeSinceTeleport,
    TimeSinceKickturn,
    ManualOutTimer,
    SetManualOutTimer(f32),
}
impl RidingOperation {
    pub fn parse(a: &Attributes<'_>) -> Option<Self> {
        Some(match a.text("name")? {
            "CreateAttribute" => {
                //82BAE918: the always branch returns before reading set and
                //writes Update twice, leaving End's initialized zero intact.
                let (values, set) = if let Some(value) = a.get("always") {
                    let value = f32::from_bits(value.float_bits);
                    ([Some(value), Some(value), Some(0.0)], false)
                } else if ["begin", "update", "end"]
                    .iter()
                    .all(|key| a.get(key).is_none())
                {
                    ([None, Some(0.0), None], false)
                } else {
                    (
                        ["begin", "update", "end"]
                            .map(|key| a.get(key).map(|v| f32::from_bits(v.float_bits))),
                        a.boolean_byte("set", 0) != 0,
                    )
                };
                Self::CreateAttribute {
                    name: encode(a.text("attName").unwrap_or("").as_bytes()),
                    values,
                    set,
                }
            }
            "SetDistComToBoard" => Self::SetDistComToBoard {
                name: encode(a.text("attribute").unwrap_or("").as_bytes()),
                manually: a.boolean_byte("setManually", 0) != 0,
                adjust_for_velocity: a.boolean_byte("adjustForVel", 0) != 0,
            },
            "ForcePhysics" => {
                Self::ForcePhysics(match a.text("force").unwrap_or("FOLLOW_ANIMATION_DATA") {
                    "FORCE_PHYSICS_SKATEBOARD" => ForceMode::PhysicsBoard,
                    "FORCE_ANIM_SKATEBOARD" => ForceMode::AnimationBoard,
                    _ => ForceMode::AnimationData,
                })
            }
            "UpdateTimeSinceTeleport" => Self::TimeSinceTeleport,
            "UpdateTimeSinceKickturn" => Self::TimeSinceKickturn,
            "UpdateManualOutTimer" => Self::ManualOutTimer,
            "SetManualOutTimer" => Self::SetManualOutTimer(f32::from_bits(
                a.float_bits("length", 0x3e29_fbe7), //factory82BCA0FC
            )),
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct RidingState {
    pub time_since_teleport: f32,
    pub time_since_kickturn: f32,
    pub manual_out_timer: f32,

    ///Specific MotionGraph CA4 bit23, getter8258FB68/setter8258FB78.
    ///Constructor8258F5B8 clears it; Reset82595480 preserves it.
    pub dark: bool,
    /// Specific MotionGraph+3156, GetLastGoodLandingVelocity8258F7C8.
    pub last_good_landing_velocity: f32,
    ///82BB2868 writes ProcessedPhysIn+16420 only on behavior Begin.
    ///None means no graph request has changed the physical mode yet.
    pub force_mode: Option<ForceMode>,
}
impl RidingState {
    pub fn new() -> Self {
        //8258F488 initializes full5912/5916; final reset825953B0 clears
        //specific3156/3224.
        Self {
            time_since_teleport: 0.0,
            time_since_kickturn: 0.0,
            manual_out_timer: 0.0,

            dark: false,
            last_good_landing_velocity: 0.0,
            force_mode: None,
        }
    }
    pub fn execute(
        &mut self,
        operation: RidingOperation,
        phase: u8,
        frame: &Frame,
        animation: &mut MotionAnimation,
        physical_category: Option<u32>,
        animation_height: Option<f32>,
        manualing: bool,
    ) -> Result<(), String> {
        match operation {
            RidingOperation::CreateAttribute { name, values, set } => {
                if let Some(value) = values[usize::from(phase)] {
                    //82BAEC90 emits the graph record before setting tree data.
                    animation.emit_packet(name, value);
                    if set {
                        animation.set_attribute(SettableAttribute {
                            name,
                            value,
                            normalized: false,
                            sequence_id: -1,
                        });
                    }
                }
            }
            RidingOperation::SetDistComToBoard {
                name,
                manually,
                adjust_for_velocity,
            } => {
                if phase == 0 {
                    //82BB04E8: source literal0.65 only when explicitly authored.
                    let height = if manually {
                        f32::from_bits(0x3f266666)
                    } else {
                        animation_height
                            .ok_or("SetDistComToBoard requires actual PhysOutAnimation distance")?
                    };
                    let correction = if adjust_for_velocity {
                        let v = frame.dt * self.last_good_landing_velocity;
                        let cap = f32::from_bits(0x3e19999a);
                        if cap - v >= 0.0 { v } else { cap }
                    } else {
                        0.0
                    };
                    animation.set_attribute(SettableAttribute {
                        name,
                        value: height - correction,
                        normalized: false,
                        sequence_id: -1,
                    });
                }
            }
            RidingOperation::ForcePhysics(mode) => {
                if phase == 0 {
                    self.force_mode = Some(mode);
                }
            }
            RidingOperation::TimeSinceTeleport => {
                if phase == 1 {
                    //82BB9058: actual filtered category5 resets; all others add
                    //controller delta through8258F968, with no imposed maximum.
                    if physical_category
                        .ok_or("UpdateTimeSinceTeleport needs filtered physical state")?
                        == 5
                    {
                        self.time_since_teleport = 0.0;
                    } else {
                        self.time_since_teleport += frame.dt;
                    }
                }
            }
            RidingOperation::TimeSinceKickturn => {
                if phase == 1 {
                    self.time_since_kickturn += frame.dt;
                }
            } //82BAE848/8258F940
            RidingOperation::ManualOutTimer => {
                if phase == 2 {
                    self.manual_out_timer = 0.0;
                }
                //82BB9260/8258F980
                else if phase == 1 && !manualing {
                    let remaining = self.manual_out_timer - frame.dt;
                    self.manual_out_timer = if remaining >= 0.0 { remaining } else { 0.0 };
                } //82BB91A8/8258F998
            }
            RidingOperation::SetManualOutTimer(length) => {
                //Vtable8232090C: Begin/Update empty; End82BB9158 sets length.
                if phase == 2 {
                    self.manual_out_timer = length;
                }
            }
        }
        Ok(())
    }
}
