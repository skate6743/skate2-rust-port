//! Camera condition leaves82DF4A90..82DF61DC, bound to the stock binary graph.
use skate3_core::{camera::{CameraMan, ManagerSubject}, graph::conditions::NumericCondition};
use skate_data::state_graph::attributes::{Attributes, key_hash};
use super::graph_subject::{CameraGraphEnvironment, CameraGraphSubject};

#[derive(Clone, Debug)]
pub(super) enum Condition {
    Type(u32), Previous(Vec<u32>), Tweak(u32), Volume(String),
    Boolean(Boolean), Numeric(Value, NumericCondition),
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Boolean {
    Observer, WipingOut, BrokenBone, Offboard, OffboardAir, OnboardAir,
    FixedInAir, Preparing, Manual, DroppingIn, MovingObject, Skitching,
    Footplant, Handplant, Boneless, HippyJump, HippyHurdle, Grinding,
    WasGrinding, OnGround, Reentry, Road, LedgeLeft, LedgeRight,
    WorldPainter, Threat, BadCollision,
}
#[derive(Clone, Copy, Debug)]
pub(super) enum Value {
    Speed, SpeedY, Acceleration, AirTime, LandingHeight, MaxHeight, HeightRatio,
    FrontsideAngle, TransferAngle, LaunchIncline, LandingIncline, GrindIncline,
    Incline, AbsoluteIncline, CentredTime, TurningTime, TimeSinceInput,
}
impl Condition {
    pub fn parse(a: &Attributes<'_>) -> Result<Self, String> {
        use Boolean as B;
        use Value as V;
        let name = a.text("name").ok_or("Camera condition requires name")?;
        let boolean = match name {
            "IsInObserverMode" => Some(B::Observer), "IsWipingOut" => Some(B::WipingOut),
            "IsBrokenBoneSlowMo" => Some(B::BrokenBone), "IsOffboard" => Some(B::Offboard),
            "IsAirOffboard" => Some(B::OffboardAir), "IsInOnBoardAir" => Some(B::OnboardAir),
            "IsFixedInAir" => Some(B::FixedInAir), "IsPreparingToJump" => Some(B::Preparing),
            "IsInManual" => Some(B::Manual), "SkaterIsDroppingIn" => Some(B::DroppingIn),
            "SkaterIsMovingObject" => Some(B::MovingObject), "SkaterIsSkitching" => Some(B::Skitching),
            "SkaterIsPerformingFootPlant" => Some(B::Footplant),
            "SkaterIsPerformingHandPlant" => Some(B::Handplant),
            "SkaterIsPerformingBoneless" => Some(B::Boneless),
            "SkaterIsPerformingHippyJump" => Some(B::HippyJump),
            "SkaterIsPerformingHippyHurdle" => Some(B::HippyHurdle),
            "SkaterIsGrinding" => Some(B::Grinding), "SkaterWasGrinding" => Some(B::WasGrinding),
            "IsPlayerSkateboardOnGround" => Some(B::OnGround), "IsReentry" => Some(B::Reentry),
            "SkaterIsOnRoad" => Some(B::Road), "IsGrindLedgeLeft" => Some(B::LedgeLeft),
            "IsGrindLedgeRight" => Some(B::LedgeRight), "IsInWorldPainterRegion" => Some(B::WorldPainter),
            "SkaterIsUnderThreat" => Some(B::Threat), "IsCameraInBadCollision" => Some(B::BadCollision),
            _ => None,
        };
        if let Some(value) = boolean { return Ok(Self::Boolean(value)); }
        let value = match name {
            "CameraSubjectSpeed" => Some(V::Speed), "CameraSubjectSpeedY" => Some(V::SpeedY),
            "CameraSubjectAcceleration" => Some(V::Acceleration), "PredictedAirTime" => Some(V::AirTime),
            "PredictedLandingHeight" => Some(V::LandingHeight), "PredictedMaxHeight" => Some(V::MaxHeight),
            "PredictedHeightDistanceRatio" => Some(V::HeightRatio), "FrontsideAngle" => Some(V::FrontsideAngle),
            "TransferAngle" => Some(V::TransferAngle), "LaunchInclineAngle" => Some(V::LaunchIncline),
            "LandingInclineAngle" => Some(V::LandingIncline), "GrindInclineAngle" => Some(V::GrindIncline),
            "InclineAngle" => Some(V::Incline), "InclineAngleAbs" => Some(V::AbsoluteIncline),
            "TurningCentredTime" => Some(V::CentredTime), "TurningTime" => Some(V::TurningTime),
            "TimeSinceLastPlayerInput" => Some(V::TimeSinceInput), _ => None,
        };
        if let Some(value) = value {
            return Ok(Self::Numeric(value, crate::skate3_native::graph_host::parse_numeric_condition(a)));
        }
        Ok(match name {
            "IsCameraTypeActive" => Self::Type(a.text("type")
                .ok_or("IsCameraTypeActive requires type")?
                .parse()
                .map_err(|_| "Invalid normal camera type")?),
            "IsPreviousShot" => Self::Previous(shot_names(a).iter().map(|v| key_hash(v)).collect()),
            "IsInVolume" => Self::Volume(a.text("volume")
                .ok_or("IsInVolume requires volume")?
                .to_owned()),
            // Constructor82DF5588 uses case-sensitive comparison and keeps0
            // for both Freefall and an unrecognized tweak name.
            "IsWipeoutBodyTweak" => Self::Tweak(match a.text("tweak").unwrap_or("") {
                "CannonBall" => 1, "JudoKick" => 2, "SwanDive" => 3, "Torpedo" => 4, _ => 0 }),
            _ => return Err(format!("Unimplemented stock camera condition {name}")),
        })
    }

    pub fn evaluate(&self, m: &CameraMan, s: &ManagerSubject, g: CameraGraphSubject,
        world: &CameraGraphEnvironment) -> bool {
        use Boolean as B;
        use Value as V;
        let f = &m.rig.fields;
        let state = &m.state;
        const DEGREES: f32 = f32::from_bits(0x42652ee1);
        match self {
            Self::Type(value) => *value == world.camera_type,
            Self::Previous(names) => names.contains(&key_hash(&m.shots.current().name)),
            Self::Tweak(value) => *value == g.wipeout_tweak,
            //82DF90C0 compares exactly strlen(requested) bytes with82990158.
            Self::Volume(name) => world.volumes.iter()
                .any(|v| v.as_bytes().starts_with(name.as_bytes())),
            Self::Boolean(value) => match value {
                B::Observer => s.special_effect != 0, B::WipingOut => s.rig.wiping_out != 0,
                B::BrokenBone => s.rig.broken_bone_slowmo != 0,
                B::Offboard => s.rig.off_board != 0 || g.running_out,
                B::OffboardAir => g.offboard_air, B::OnboardAir => g.onboard_air,
                B::FixedInAir => s.rig.air_flag_452 != 0, B::Preparing => g.preparing_to_jump,
                B::Manual => g.manual, B::DroppingIn => g.dropping_in, B::MovingObject => g.moving_object,
                B::Skitching => g.skitching, B::Footplant => g.footplant, B::Handplant => g.handplant,
                B::Boneless => g.boneless, B::HippyJump => g.hippy_jump, B::HippyHurdle => g.hippy_hurdle,
                B::Grinding => f.height_mode == 2, B::WasGrinding => f.previous_height_mode == 2,
                B::OnGround => f.height_mode == 0, B::Reentry => state.flags & 0x10 != 0,
                B::Road => world.on_road, B::LedgeLeft => world.ledge_left, B::LedgeRight => world.ledge_right,
                // TU382DF5050 returns false. AI is intentionally absent from this game.
                B::WorldPainter | B::Threat => false,
                B::BadCollision => m.rig.positioner.flags & 0x80 != 0
                    && m.rig.positioner.distance < f.distance * 0.75,
            },
            Self::Numeric(value, condition) => condition.matches(match value {
                //82E03060: signed velocity projected on the subject's forward axis.
                V::Speed => { let n = s.rig.transform[2]; let v = f.velocity;
                    (v[0] * n[0] + v[1] * n[1]) + v[2] * n[2] },
                V::SpeedY => f.velocity[1], V::Acceleration => f.smoothed_acceleration,
                V::AirTime => s.valid_trajectory_duration(), V::LandingHeight => state.landing_height_delta,
                V::MaxHeight => state.apex_height, V::HeightRatio => state.apex_height_ratio,
                V::FrontsideAngle => state.frontside_angle,
                V::TransferAngle => state.launch_landing_heading_delta * DEGREES,
                V::LaunchIncline => state.launch_incline * DEGREES,
                V::LandingIncline => state.landing_incline * DEGREES,
                V::GrindIncline => state.direction_incline * DEGREES,
                V::Incline => state.velocity_incline * DEGREES,
                V::AbsoluteIncline => (state.absolute_velocity_incline * DEGREES).abs(),
                V::CentredTime => state.centred_time, V::TurningTime => state.steering_time,
                V::TimeSinceInput => g.time_since_player_input,
            }),
        }
    }
}

pub(super) fn shot_names(a: &Attributes<'_>) -> Vec<String> {
    (1..).map(|i| format!("shot{i}")).map_while(|key| a.text(&key).map(str::to_owned)).collect()
}
