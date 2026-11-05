//! Native collector inputs sampled from the original retained PhysOut fields.
use crate::physics_state_ids::{
    PHYSICS_CATEGORY_GRIND, PHYSICS_CATEGORY_GROUND, PHYSICS_STATE_LANDINGONDECK,
};
use super::{
    super::{PhysOutGroundVector, PhysOutMotionScalar, PhysOutSystemVector, PhysicalPlayerPhysOut},
    scoring_air_tricks::AirTrickFrame,
    scoring_attributes::ScoreError,
    scoring_context_collect::ContextFrame,
    scoring_external::ScoringExternalInput,
    scoring_grind::GrindFrame,
    scoring_ground::GroundFrame,
    scoring_module_dispatch::CollectorFrame,
    scoring_module_update::{ModuleFrame, ModuleTuning},
    scoring_offboard::OffboardFrame,
    scoring_wipeout_bones::DamageTuning,
    scoring_wipeout_lifecycle::{WipeoutFrame, WipeoutTuning},
};
use crate::{
    input_cvar_runtime::{
        InputCvarReferences,
        scoring::{ScoringBool as B, ScoringFloat as F, ScoringInt as I},
    },
    physics::PhysicsCategory,
    skeleton_root_transforms::SkeletonRootMatrix,
};

fn need<'a, T>(value: Option<&'a T>, name: &'static str) -> Result<&'a T, ScoreError> {
    value.ok_or(ScoreError::Missing(name))
}
impl<'a> ModuleFrame<'a> {
    pub(super) fn sample(
        output: &'a PhysicalPlayerPhysOut,
        input: &'a ScoringExternalInput,
        cvars: &InputCvarReferences,
    ) -> Result<Self, ScoreError> {
        output.check_current_guard().map_err(ScoreError::Access)?;
        let c = &output.concrete;
        let animation = need(c.animation.as_deref(), "animation output")?;
        let trick = need(c.trick.as_deref(), "trick output")?;
        let board = need(c.skateboard_reckoning.as_deref(), "board output")?;
        let skeleton = need(c.skeleton.as_deref(), "skeleton output")?;
        let ground = need(c.ground.as_deref(), "ground output")?;
        let grinds = need(c.grinds.as_deref(), "grind output")?;
        let system = need(c.system.as_deref(), "system output")?;
        let state = need(c.state.as_deref(), "state output")?;
        let offboard = need(c.offboard.as_deref(), "offboard output")?;
        let filtered = need(c.filtered.as_deref(), "filtered output")?;
        let collision = need(c.collision.as_deref(), "collision output")?;
        let air = need(c.air.as_deref(), "air output")?;
        let bool_value = |key| cvars.try_scoring_bool(key).map_err(ScoreError::Cvar);
        let float_value = |key| cvars.try_scoring_float(key).map_err(ScoreError::Cvar);
        let int_value = |key| cvars.try_scoring_int(key).map_err(ScoreError::Cvar);
        let minimum_distance = float_value(F::kMinDistancePerFrameToAccumulate)?;
        let switched = animation.is_riding_switch;
        let fakie = animation.is_riding_fakie;
        let tick = input.tick;
        let dt = f32::from_bits(0x3c88_8889);
        let deck_position = board.deck_position;
        let system_up = output
            .system_vector(PhysOutSystemVector::Up)
            .map_err(ScoreError::Access)?
            .value;
        Ok(Self {
            collector: CollectorFrame {
                ground: GroundFrame {
                    enabled: &input.enabled,
                    tick,
                    dt,
                    deck_position,
                    deck_speed: output
                        .motion_scalar(PhysOutMotionScalar::DeckSpeed)
                        .map_err(ScoreError::Access)?
                        .value,
                    switched,
                    fakie,
                    is_reverting: state.is_reverting,
                    revert_exited_normally: state.revert_exited_normally,
                    minimum_distance,
                    allow_slow_manuals: bool_value(B::kAllowSlowManualsToAlwaysScore)?,
                    allow_slow_manuals_after_begin: bool_value(
                        B::kAllowSlowManualsToStillScoreOnceBegun,
                    )?,
                    vel_direction: board.vel_direction,
                },
                grind: GrindFrame {
                    filtered_scorable_id: filtered.grind_scorable_id,
                    pathed_guid: filtered.grind_pathed_guid,
                    local_guid: filtered.grind_local_guid,
                    tick,
                    dt,
                    deck_position,
                    minimum_distance,
                    switched,
                    fakie,
                },
                offboard: OffboardFrame {
                    enabled: &input.enabled,
                    tick,
                    switched,
                    fakie,
                    landing_on_deck: state.physics_state == PHYSICS_STATE_LANDINGONDECK,
                    player_matrix: skeleton.skeleton_root_matrix,
                    board_matrix: board.transform,
                    ground,
                    system_velocity: system.damped_centre_of_mass_velocity,
                    system_up,
                },
                air: AirTrickFrame {
                    tick,
                    dt,
                    switched,
                    fakie,
                    ground,
                    #[cfg(feature = "skate3-runtime")]
                    use_air_reckoning: air.use_air_reckoning,
                },
                animation: &trick.current_animation,
                damped_system_velocity: system.damped_centre_of_mass_velocity,
                damped_system_up: system_up,
                wheels_in_contact: collision.wheels_in_contact,
                min_frames_after_landing: int_value(I::kMinFramesBeforeEndingSequenceAfterLanding)?
                    as u32,
                extra_off_ground_frames: int_value(
                    I::kExtraFramesThatOffGroundWheelsKeepSequenceActive,
                )? as u32,
                pumping_acceleration: ground.pumping_acceleration,
                deck_up_y: board.transform.columns()[1].xyz[1],
                flipping: air.is_flipping,
                front_flipping: air.is_front_flipping,
                wiped_out: state.is_wiping_out,
                score_over_skaters: input.score_over_skaters,
            },
            context: ContextFrame {
                ground,
                filtered: filtered.filtered_physics_state,
                damped_com: system.damped_centre_of_mass,
                pedestrian_radius: float_value(F::kPedContextRadius)?,
                skater_radius: float_value(F::kSkaterContextRadius)?,
                score_invalid_dmos: bool_value(B::kScoreEntityTypeInvalidDMOs)?,
            },
            wipeout: WipeoutFrame {
                filtered: filtered.filtered_physics_state,
                collision: &collision.skeleton,
                // Raw reset/update copy Skeleton offsets96,48,80,64 in this order.
                hips_matrix: SkeletonRootMatrix::from_columns([
                    skeleton.skeleton_hips_mat_z,
                    skeleton.skeleton_hips_look,
                    skeleton.skeleton_hips_mat_y,
                    skeleton.skeleton_hips_pos,
                ]),
                system_position: output
                    .system_vector(PhysOutSystemVector::CentreOfMass)
                    .map_err(ScoreError::Access)?
                    .value,
                system_velocity: system.damped_centre_of_mass_velocity,
                skeleton_wipeout_over: skeleton.is_wipeout_over,
                heavy_landing_kind: state.special_wipeout_info.state,
                heavy_landing_speed: state.special_wipeout_info.landing_speed,
                gesture: [trick.wipeout_gesture_x, trick.wipeout_gesture_y],
                skate3_countdown: state.skate3_wipeout_countdown,
                skate3_teleport_requested: state.skate3_teleport_requested,
                skate3_response_strength: skeleton.skate3_response_strength,
                skate3_air_locked: air.use_air_reckoning,
                centre_of_mass_velocity: output
                    .system_vector(PhysOutSystemVector::CentreOfMassVelocity)
                    .map_err(ScoreError::Access)?
                    .value,
                damped_centre_of_mass: system.damped_centre_of_mass,
            },
            animation,
            physics_state: state.physics_state,
            category: match state.physics_state_category {
                PHYSICS_CATEGORY_GROUND => PhysicsCategory::Ground,
                PHYSICS_CATEGORY_GRIND => PhysicsCategory::Grind,
                _ => PhysicsCategory::Other,
            },
            requested_teleport: state.physics_requested_teleport,
            grind_scoring_blocked: grinds.skate3_tipslide_blocks_scoring,
            requested_runout: state.physics_requests_runout,
            retrieving_board: offboard.is_retrieving_board,
            ground_normal: output
                .ground_vector(PhysOutGroundVector::GroundUp)
                .map_err(ScoreError::Access)?
                .value,
            grind_normal: grinds.grind_normal,
            system_com: output
                .system_vector(PhysOutSystemVector::CentreOfMass)
                .map_err(ScoreError::Access)?
                .value,
            gesture: trick.gesture,
            is_gesturing: trick.is_gesturing,
        })
    }
}
impl ModuleTuning {
    pub(super) fn sample(cvars: &InputCvarReferences) -> Result<Self, ScoreError> {
        let b = |key| cvars.try_scoring_bool(key).map_err(ScoreError::Cvar);
        let f = |key| cvars.try_scoring_float(key).map_err(ScoreError::Cvar);
        let i = |key| cvars.try_scoring_int(key).map_err(ScoreError::Cvar);
        Ok(Self {
            minimum_sketchy_scalar: f(F::kMinAnimScalarForSketchyLanding)?,
            force_sketchy: b(B::kMakeAllLandingsSketchy)?,
            multiplier_override: if b(B::kOverrideScoreMultiplier)? {
                Some(i(I::kOverrideScoreMultiplierValue)?)
            } else {
                None
            },
            wipeout: WipeoutTuning {
                damage: DamageTuning {
                    propagate: b(B::kEnableBoneDamagePropogation)?,
                    minimum_propagation: f(F::kMinDamageToPropogate)?,
                    scalar: f(F::kDamageScalar)?,
                    acceleration_for_force: b(B::kUseBoneAccelerationForForce)?,
                },
                ignore_stationary_rotation: b(B::kIgnoreHoMRotationWhenStationary)?,
                minimum_bank_degrees: f(F::kMinAngleToBankInDegrees)?,
                heavy_landing_reset_frames: i(I::kNumFramesToResetHeavyLandingData)?,
                forced_render_damage_level: cvars
                    .try_scoring_forced_damage_level()
                    .map_err(ScoreError::Cvar)?,
                damage_fade_in: f(F::kDamageFadeInRate)?,
            },
        })
    }
}
