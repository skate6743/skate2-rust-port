//! Original TU3 physical-state condition leaves used by the stock MotionGraph.
use super::motion::MotionHost;
use skate_data::state_graph::attributes::Attributes;

/// The preceding completed physical publication. These are native output
/// fields, separate from controller intents and presentation transforms.
#[derive(Clone, Copy, Debug)]
pub struct GameplayConditions {
    pub state: u32,
    pub wants_runout: bool,
    pub physics_wiping: bool,
    pub body_flipping: bool,
    pub wants_wipeout: bool,
    pub bumped: bool,
    pub grabbing_object: bool,
    pub retrieving_board: bool,
    pub dropping_board: bool,
    pub in_biped_air: bool,
    pub hippy_hurdling: bool,
    pub handplant_flags: u32,
    pub handplant_time: f32,
    /// anim_handplant/default layout0/4/8: out, into, antic.
    pub handplant_thresholds: [f32; 3],
    pub footplant_active: bool,
    pub footplant_duration: f32,
    pub footplant_contact_time: f32,
    pub time_to_skitch: f32,
    pub skitch_transition_time: f32,
    /// TimeToLand82BA7250: PhysOutAir+184, gated by byte437.
    pub time_to_land: f32,
    pub time_to_land_valid: bool,
    /// OBTimeToLand82BA5770: PhysOutOffBoard+32.
    pub offboard_time_to_land: f32,
    /// OffboardBodyTweakBlend82BBAFC8 reads completed OffBoard+92.
    pub offboard_air_scalar_92: f32,
    /// MatchAirTime82BBA200 consumes the completed OffBoard+96 vector.
    pub offboard_air_translation: [f32; 4],
    /// CanBipedLand82BA8214 tests OffBoard+192.Y strictly above0.85.
    pub offboard_landing_normal: [f32; 4],
    ///IsBipedCommittedToMotion82BA80A0: completed OffBoard byte329.
    pub offboard_committed_to_motion: bool,
    ///EnoughDistToObstacle82BA7FD0: completed OffBoard+112, not distance116.
    pub offboard_obstacle_distance: f32,
    ///DistToEdge82BA82A8: completed OffBoard+116, not obstacle scalar112.
    pub offboard_edge_distance: f32,
    /// OBTrajTime82BA4528: PhysOutOffBoard+120, gated by byte331.
    pub offboard_trajectory_time: f32,
    pub offboard_trajectory_valid: bool,
    /// AirOutputFields::reached_apex_436, published by the known-air owner.
    pub reached_apex: bool,
    ///TU3 CanLandOnBoard82BA5D30 and LandOnBoard82BB95D8.
    pub can_land_on_board: bool,
    pub landing_turning: bool,
    /// GrindOutputFields::flag_318/322, published by the contact solver.
    pub grind_contact: bool,
    /// CollisionOutputFields::wheel_contact_3296_3299.
    pub wheel_contact: bool,
    /// Collision3472 (trucks) OR3475 (deck), excluding wheels.
    pub trucks_or_deck_contact: bool,
    /// StateGraph's moving-object state publication. TU3 camera and motion
    /// consumers use state 502 for the moving-object riding branch.
    pub moving_object: bool,
    pub tricks_blocked_on_stairs: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GameplayCondition {
    RetrievingBoard,
    DroppingBoard,
    InBipedAir,
    HippyHurdling,
    WantsRunout,
    PhysicsWiping,
    BodyFlipping,
    WantsWipeout,
    Bumped,
    CanEnterSlide { right: bool },
    GrabbingObject,
    Dark,
    UnderflipRequested,
    DarkCatchRequested,
    OkToDoTrickOnStairs,
    HandPlanting { state: u8, direction: u8 },
    FootPlanting,
    PrepareFootplant,
    NewHandplantPosition,
    PlayHandplant { phase: usize },
    EnteringSkitch,
    Skitching,
    /// ActivationFunc 0x82ba6038: the pushing record's movement (PhysOut+0x48: x +4, z +0) is
    /// longer than 0.1 and its heading atan2(-x, z) lies in the authored arc (radians).
    IsMovingObject { start: f32, end: f32 },
}
impl GameplayCondition {
    pub fn recognizes(name: &str) -> bool {
        matches!(
            name,
            "IsRetrievingSkateboard"
                | "IsDroppingSkateboard"
                | "IsInBipedAir"
                | "IsHippyHurdling"
                | "PhysicsWantsRunout"
                | "IsPhysicsWiping"
                | "IsBodyFlipping"
                | "PhysicsWantsWipeOut"
                | "IsBumped"
                | "CanEnterSlide"
                | "IsGrabbingObject"
                | "IsDark"
                | "IsUnderflipRequested"
                | "IsDarkCatchRequested"
                | "OkToDoTrickOnStairs"
                | "IsHandPlanting"
                | "IsFootPlanting"
                | "ShouldPrepareOneFootAirForFootplant"
                | "HasNewHandPlantPos"
                | "ShouldPlayHandPlantAnim"
                | "IsEnteringSkitch"
                | "IsSkitching"
                | "IsMovingObject"
        )
    }
    pub fn parse(a: &Attributes<'_>) -> Result<Self, String> {
        Ok(match a.text("name").unwrap_or("") {
            "IsRetrievingSkateboard" => Self::RetrievingBoard,
            "IsDroppingSkateboard" => Self::DroppingBoard,
            "IsInBipedAir" => Self::InBipedAir,
            "IsHippyHurdling" => Self::HippyHurdling,
            "PhysicsWantsRunout" => Self::WantsRunout,
            "IsPhysicsWiping" => Self::PhysicsWiping,
            "IsBodyFlipping" => Self::BodyFlipping,
            "PhysicsWantsWipeOut" => Self::WantsWipeout,
            "IsBumped" => Self::Bumped,
            "CanEnterSlide" => Self::CanEnterSlide {
                right: a.boolean_byte("right", 1) != 0, //82BC53F0
            },
            "IsGrabbingObject" => Self::GrabbingObject,
            "IsDark" => Self::Dark,
            "IsUnderflipRequested" => Self::UnderflipRequested,
            "IsDarkCatchRequested" => Self::DarkCatchRequested,
            "OkToDoTrickOnStairs" => Self::OkToDoTrickOnStairs,
            "IsEnteringSkitch" => Self::EnteringSkitch,
            "IsSkitching" => Self::Skitching,
            "IsFootPlanting" => Self::FootPlanting,
            "ShouldPrepareOneFootAirForFootplant" => Self::PrepareFootplant,
            "HasNewHandPlantPos" => Self::NewHandplantPosition,
            "ShouldPlayHandPlantAnim" => Self::PlayHandplant { phase: match a.text("anim") {
                Some("antic") => 2, Some("into") => 1, Some("out") => 0,
                value => return Err(format!("Unauthored handplant animation phase {value:?}")),
            } },
            //ClassHelper 0x82bc4d60: degrees times 0.017453292.
            "IsMovingObject" => Self::IsMovingObject {
                start: f32::from_bits(a.float_bits("anglestart", 0)) * f32::from_bits(0x3c8e_fa35),
                end: f32::from_bits(a.float_bits("angleend", 0)) * f32::from_bits(0x3c8e_fa35),
            },
            "IsHandPlanting" => Self::HandPlanting {
                //82BA54A0 compares these authored strings in this order.
                state: match a.text("state").unwrap_or("") {
                    "air" => 0,
                    "ground" => 1,
                    value => return Err(format!("IsHandPlanting has unauthored state {value:?}")),
                },
                direction: match a.text("dir").unwrap_or("") {
                    "FS" => 1,
                    "BS" => 0,
                    _ => 2,
                },
            },
            name => return Err(format!("Unknown gameplay condition {name}")),
        })
    }
    /// Both stock graphs register these same native leaves. Return None for
    /// conditions that also require the MotionGraph/channel owner.
    pub fn evaluate_physical(&self, p: &GameplayConditions) -> Option<bool> {
        Some(match self {
            Self::OkToDoTrickOnStairs => !p.tricks_blocked_on_stairs,
            Self::RetrievingBoard => p.retrieving_board, //82BA5EF0:Offboard323
            Self::InBipedAir => p.in_biped_air,          //82BA8030:Offboard328
            Self::HippyHurdling => p.hippy_hurdling,     //82BA5DA0:Offboard317
            Self::WantsRunout => p.wants_runout,         //82BA44B8:State78
            Self::PhysicsWiping => p.physics_wiping,     //82BA4390:State59
            Self::BodyFlipping => p.body_flipping,       //82BA71E0:Air441
            Self::WantsWipeout => p.wants_wipeout,       //82BA4400:State63 || State65
            Self::Bumped => p.bumped, //82BA7310: published acceleration and anim_motion/bumps
            Self::GrabbingObject => p.grabbing_object, //82BA5700:Offboard304
            Self::Skitching => p.state == 104, //82BBBC88:State16
            Self::EnteringSkitch => {
                //82BBBDE0:Ground276,Globals400/layout96
                !(p.time_to_skitch < 0.0) && p.time_to_skitch <= p.skitch_transition_time
            }
            Self::HandPlanting { state, direction } => {
                //82BA55A8:Air324,State16
                let active = if *state == 0 {
                    p.state == 600
                } else {
                    p.handplant_flags & 0x8000_0000 != 0
                };
                active
                    && match direction {
                        2 => true,
                        1 => p.handplant_flags & 0x2000_0000 != 0,
                        0 => p.handplant_flags & 0x2000_0000 == 0,
                        _ => false,
                    }
            }
            // Factory82BCB308 -> VT82321070 slot48=82BBD888.
            // Both graphs read Air448 and Air212, including the entry frame.
            Self::FootPlanting => p.footplant_active && p.footplant_duration >= 0.0,
            //82BBD7F0, literals820641A8 and8209975C.
            Self::PrepareFootplant => p.footplant_contact_time >= 0.1 && p.footplant_contact_time <= 0.5,
            Self::NewHandplantPosition => p.handplant_flags & 0x4000_0000 != 0, //82BBDB98
            Self::PlayHandplant { phase } => {
                //82BBDA98 subtracts one simulation tick before the comparison.
                let threshold = p.handplant_thresholds[*phase];
                p.handplant_time - f32::from_bits(0x3c888889) < if *phase == 0 { -threshold } else { threshold }
            }
            Self::DroppingBoard
            | Self::Dark
            | Self::UnderflipRequested
            | Self::DarkCatchRequested
            | Self::IsMovingObject { .. }
            | Self::CanEnterSlide { .. } => return None,
        })
    }

    pub fn evaluate(&self, host: &MotionHost) -> Result<bool, String> {
        let p = host
            .gameplay_conditions
            .as_ref()
            .ok_or("MotionGraph requires the actual physical condition publication")?;
        if let Some(result) = self.evaluate_physical(p) {
            return Ok(result);
        }
        Ok(match self {
            //82BA5F60: Offboard322 or the actual RetrieveBoard channel.
            Self::DroppingBoard => p.dropping_board || host.animation.channels.has("RetrieveBoard"),
            //82BA79A0 calls the specific MotionGraph getter8258FB68.
            Self::Dark => host.riding.dark,
            Self::UnderflipRequested => host.trick_requests.underflip,
            Self::DarkCatchRequested => host.trick_requests.dark_catch,
            Self::IsMovingObject { start, end } => {
                let [x, z] = host.moving_objects.physical.movement;
                crate::actor_construction::motion_runtime_session::trick_conditions::moving_object_in_sector(
                    x,
                    z,
                    *start,
                    *end,
                    &mut crate::reckoning_numeric::HostReckoningNumeric,
                )
                .map_err(|error| format!("IsMovingObject: {error:?}"))?
            }
            Self::CanEnterSlide { right } => {
                //82BA6FE0 rejects RevertGround102, reverses the side for fakie,
                //then reads its start bit in the retained C84 slide packet.
                if p.state == 102 {
                    false
                } else {
                    //ISkaterAnim virtual12=82B970D8 reads bit29=fakie.
                    let fakie = host
                        .animation
                        .skater_animation_flags
                        .ok_or("CanEnterSlide requires actual SkaterAnim stance flags")?
                        & 0x2000_0000
                        != 0;
                    if *right ^ fakie {
                        host.slide_latch.start(true)
                    } else {
                        host.slide_latch.start(false)
                    }
                }
            }
            _ => unreachable!("Physical condition handled above"),
        })
    }
}
