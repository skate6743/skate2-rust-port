//! Stock runtime data. No map/query settings or manager defaults live here.
use skate3_core::physics::grind_forces::{post, reckoning};
use skate3_core::point_graph::PointGraph;
use skate_data::collections::Collections;

pub(super) struct Settings {
    pub standard_angular_drag: f32,
    pub pin_vs_slope: PointGraph<4>,
    pub look_ahead: f32,
    pub exit_assist: PointGraph<4>,
    pub post: post::Settings,
    pub reckoning: reckoning::Settings,
    pub substate: super::substate::Settings,
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let words = data
            .words::<8>("physics_grinds", "default", "PinVsSlope")?
            .map(f32::from_bits);
        let exit = data
            .words::<12>("physics_grinds", "default", "ExitAssistVsLeanAngle")?
            .map(f32::from_bits);
        let wipeout = |name| data.float("physics_wipeout", "default", name);
        let graph = |name| -> Result<PointGraph<8>, String> {
            let w = data
                .words::<16>("physics_reckoning", "default", name)?
                .map(f32::from_bits);
            Ok(PointGraph {
                x: w[..8].try_into().unwrap(),
                y: w[8..].try_into().unwrap(),
            })
        };
        Ok(Self {
            standard_angular_drag: data.float("physicsdeck", "default", "DeckAngularDrag")?
                * f32::from_bits(0x426f_ffff),
            pin_vs_slope: PointGraph {
                x: words[..4].try_into().unwrap(),
                y: words[4..].try_into().unwrap(),
            },
            look_ahead: data.float("physics_grinds", "default", "GrindLookAheadScalar")?,
            exit_assist: PointGraph {
                x: exit[4..8].try_into().unwrap(),
                y: exit[8..].try_into().unwrap(),
            },
            post: post::Settings {
                // Original82D90898 loads164/168. Stock schema proves Ground
                // contact thresholds, not similarly named Air fields240/244.
                max_arm_contact_164: wipeout("Wipeout_GroundSkeletonMaxContactArms")?,
                max_body_contact_168: wipeout("Wipeout_GroundSkeletonMaxContact")?,
                xz_acceleration_204: wipeout("Wipeout_GrindXZDeck")?,
                max_displacement_208: wipeout("Wipeout_GrindSkeletonMaxDisp")?,
                max_angular_deck_error_212: wipeout("Wipeout_GrindMaxAngularDeckError")?,
            },
            reckoning: reckoning::Settings {
                ground_normal_smoothing: data
                    .words::<4>("physics_reckoning", "default", "GroundNormalSmoothing")?
                    .map(f32::from_bits),
                tilt_vs_rotation: graph("TiltVsRotGround")?,
                tilt_vs_slope: graph("TiltVsSlopeGround")?,
            },
            substate: super::substate::Settings::load(data)?,
        })
    }
}
