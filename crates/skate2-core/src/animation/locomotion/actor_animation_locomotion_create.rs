//! Full retained CreateLocomotionTrajectory82cac998 mutation stage.
pub(crate) use super::locomotion_create_finish::StepLaunchContinuation;
use super::{
    ActorAnimationConstruction,
    locomotion_collision_offset::{self, CollisionOffsetError, CollisionOffsetTuning},
    locomotion_create_attribute::CreateAttributeError,
    locomotion_create_correction::CreateCorrectionError,
    locomotion_create_finish::{CreateFinishError, CreatePredictionTuning},
    locomotion_create_mg::{CreateMotionError, CreateMotionTuning},
    locomotion_debug::LocomotionDebugError,
    locomotion_producer::{FootPredictionTuning, TrajectoryProducerError},
    locomotion_turn::TurnError,
    physics_update_data::AnimationPhysicsDataError,
};
use crate::{
    actor_graph_construction::ActorMgAttribute,
    attribute_runtime::{AttributeRuntimeError, AttributeRuntimeInstance},
    debug_overlay::DebugOverlaySlot,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physics::PointGraph,
    reckoning_numeric::ReckoningNumeric,
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
    skeleton_root_transforms::{SkeletonRootMatrix as M, SkeletonRootVector as V},
};
use skate2_formats::andale::AndaleRuntimeAttribute;
use std::convert::Infallible;

#[derive(Debug)]
pub(crate) enum CreateTuningError {
    Missing(&'static str),
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    GraphSize(usize),
}
#[derive(Debug)]
pub(crate) enum CreateError<N, R> {
    Missing(&'static str),
    UnknownFlags(u32),
    FootDown,
    SequenceIndex(i32),
    Collision(CollisionOffsetError<N, CreateTuningError>),
    Motion(CreateMotionError<CreateTuningError>),
    Attribute(CreateAttributeError<InputCvarError>),
    CurrentAction(AnimationPhysicsDataError),
    Trajectory(TrajectoryProducerError<Infallible>),
    Tuning(CreateTuningError),
    SinCos(R),
    Turn(TurnError<N, R>),
    Correction(CreateCorrectionError<N>),
    Finish(CreateFinishError<CreateTuningError, LocomotionDebugError<N>>),
}
// Shared exact cvar adapter for GetFootPrediction and second-step admission.
pub(super) fn prediction_tuning(
    refs: &InputCvarReferences,
) -> impl CreatePredictionTuning<Error = CreateTuningError> + '_ {
    Tuning {
        refs,
        actor: None,
        global: None,
    }
}
struct Tuning<'a> {
    refs: &'a InputCvarReferences,
    actor: Option<AttributeRuntimeInstance>,
    global: Option<AttributeRuntimeInstance>,
}
impl Tuning<'_> {
    fn global_scalar(&self, key: u64) -> Result<f32, CreateTuningError> {
        Ok(f32::from_bits(
            self.global
                .as_ref()
                .ok_or(CreateTuningError::Missing(
                    "Global locomotion attributes+16c",
                ))?
                .try_scalar_word(key, 0)
                .map_err(CreateTuningError::Attribute)?
                .unwrap_or(0),
        ))
    }
    fn graph(
        &self,
        key: u64,
        offset: usize,
        size: usize,
        input: f32,
    ) -> Result<f32, CreateTuningError> {
        let data = self
            .actor
            .as_ref()
            .ok_or(CreateTuningError::Missing(
                "Actor locomotion attributes+6190",
            ))?
            .try_get_data(key, 0)
            .map_err(CreateTuningError::Attribute)?;
        if let Some(ref data) = data {
            if data.bytes().len() < size {
                return Err(CreateTuningError::GraphSize(data.bytes().len()));
            }
        }
        let word = |i| {
            data.as_ref().map_or(0.0, |d| {
                f32::from_be_bytes(
                    d.bytes()[offset + i * 4..offset + i * 4 + 4]
                        .try_into()
                        .expect("bounded graph"),
                )
            })
        };
        Ok(PointGraph::<8> {
            x: std::array::from_fn(word),
            y: std::array::from_fn(|i| word(i + 8)),
        }
        .evaluate(input))
    }
}
impl CreateMotionTuning for Tuning<'_> {
    type Error = CreateTuningError;
    fn sprint_increment(&mut self) -> Result<f32, Self::Error> {
        self.global_scalar(0x320f3450e99ab013)
    }
    fn sprint_decrement(&mut self) -> Result<f32, Self::Error> {
        self.global_scalar(0xf2cdca4243861327)
    }
    fn use_sprint_button(&mut self) -> Result<bool, Self::Error> {
        self.refs
            .try_create_locomotion_sprint_button()
            .map_err(CreateTuningError::Cvar)
    }
    fn desired_speed_graph(&mut self, speed: f32) -> Result<f32, Self::Error> {
        self.graph(0x9a266da3b97d0018, 0, 0x40, speed)
    }
}
impl CollisionOffsetTuning for Tuning<'_> {
    type Error = CreateTuningError;
    fn displacement_blend(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_displacement_blend()
            .map_err(CreateTuningError::Cvar)
    }
    fn minimum_displacement(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_minimum_displacement()
            .map_err(CreateTuningError::Cvar)
    }
    fn avoidance_duration(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_avoidance_duration()
            .map_err(CreateTuningError::Cvar)
    }
    fn maximum_displacement(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_maximum_collision_displacement()
            .map_err(CreateTuningError::Cvar)
    }
}
impl FootPredictionTuning for Tuning<'_> {
    type Error = CreateTuningError;
    fn start_y_offset(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_start_y_offset()
            .map_err(CreateTuningError::Cvar)
    }
    fn minimum_apex_height(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_minimum_apex_height()
            .map_err(CreateTuningError::Cvar)
    }
    fn jump_height(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_loco_jump_height()
            .map_err(CreateTuningError::Cvar)
    }
}
impl CreatePredictionTuning for Tuning<'_> {
    fn failure_frame_count(&mut self) -> Result<i32, Self::Error> {
        self.refs
            .try_create_locomotion_failure_frame_count()
            .map_err(CreateTuningError::Cvar)
    }
    fn minimum_spread_degrees(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_minimum_spread_degrees()
            .map_err(CreateTuningError::Cvar)
    }
    fn maximum_spread_degrees(&mut self) -> Result<f32, Self::Error> {
        self.refs
            .try_create_locomotion_maximum_spread_degrees()
            .map_err(CreateTuningError::Cvar)
    }
    fn maximum_step_height(&mut self, jump: bool) -> Result<f32, Self::Error> {
        if jump {
            self.refs.try_create_locomotion_jump_maximum_step_height()
        } else {
            self.refs.try_create_locomotion_run_maximum_step_height()
        }
        .map_err(CreateTuningError::Cvar)
    }
}
const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0; 4],
];
impl<K, W> ActorAnimationConstruction<K, W> {
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(crate) fn try_create_locomotion_trajectory<
        N: ReckoningNumeric,
        R: ReckoningSinCosNumeric,
    >(
        &mut self,
        dt: f32,
        attributes: &[AndaleRuntimeAttribute],
        mg_attributes: &[ActorMgAttribute],
        collision_displacement: V,
        _centre_of_mass_velocity: V,
        proxy_collision: bool,
        world_only: bool,
        refs: &InputCvarReferences,
        overlay: &DebugOverlaySlot,
        numeric: &mut N,
        rounding: &mut R,
    ) -> Result<StepLaunchContinuation, CreateError<N::Error, R::Error>> {
        use CreateError as E;
        let player = self.player.as_mut().ok_or(E::Missing("player"))?;
        player.flags_bits = (player.flags_bits & !1) | u32::from(world_only);
        player.flags_known_mask |= 1;
        let mut tuning = Tuning {
            refs,
            actor: self.offboard_attributes.clone(),
            global: self.locomotion_global_attributes.clone(),
        };
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion suffix"))?;
        let collision_updated = locomotion_collision_offset::update_collision_offset(
            &mut suffix.collision_displacement,
            &mut suffix.collision_trajectory_displacement,
            &mut suffix.last_collision_normal,
            &mut suffix.collision_normal_timer,
            collision_displacement,
            proxy_collision,
            numeric,
            &mut tuning,
        )
        .map_err(E::Collision)?;
        let mut world = suffix.world_position;
        if player.flags_known_mask & 0x40000000 == 0 {
            return Err(E::UnknownFlags(0x40000000));
        }
        let mirrored = player.flags_bits & 0x40000000 != 0;
        if mirrored {
            let (sin, cos) =
                try_sin_cos(f32::from_bits(0x40490fdb), rounding).map_err(E::SinCos)?;
            world = world.compose(&M::heading_yaw(sin, cos));
        }
        let actions = self
            .try_create_motion_prefix(mg_attributes.iter().map(|a| (a.name, a.value)), &mut tuning)
            .map_err(E::Motion)?;
        self.before_face
            .as_mut()
            .expect("retained")
            .extra_y_rotation = IDENTITY;
        let previous_action = self
            .skater
            .as_ref()
            .ok_or(E::Missing("trajectory"))?
            .trajectory
            .physics_action_probe()
            .map_err(E::CurrentAction)? as i32;
        self.before_face.as_mut().expect("retained").turn = 0.0;
        let mut first = true;
        let mut changed = false;
        for attribute in attributes {
            let id = attribute.sequence_id;
            if id == -1 {
                continue;
            }
            if id < 0 {
                return Err(E::SequenceIndex(id));
            }
            let foot = (id / 3 + i32::from(mirrored)) % 4;
            let state = id % 3;
            if first {
                changed = self
                    .skater
                    .as_mut()
                    .expect("retained")
                    .trajectory
                    .try_set_state(foot, state)
                    .map_err(E::Trajectory)?;
                self.before_face.as_mut().expect("retained").current_stage = id;
                first = false;
                if state < 2 || previous_action == 0 {
                    self.before_sprint
                        .as_mut()
                        .ok_or(E::Missing("locomotion fields"))?
                        .foot_down = foot;
                    let mut input_turn = 0.0;
                    let name = crate::play_animation::FastString30::encode("OB_TURN")
                        .expect("source literal");
                    for row in mg_attributes {
                        if row.name == name {
                            input_turn = row.value;
                        }
                    }
                    let absolute = input_turn.abs();
                    let lower = if -absolute >= 0.0 { 0.0 } else { absolute };
                    let bounded = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
                    let sign = if input_turn <= 0.0 { 1.0 } else { -1.0 };
                    // 1.02 82a9b224..b374: three quarters past 0.85 desired speed, and 80
                    // times the step lands through (target - turn) * 0.04 on the turn
                    // cleared above, where 1.00 took 1.5 times it.
                    let suffix = self.before_face.as_ref().expect("retained");
                    let speed = if suffix.desired_speed <= f32::from_bits(0x3f59999a) {
                        1.0
                    } else {
                        0.75
                    };
                    let curve = tuning
                        .graph(0xe8c68cdbc3a35230, 0x10, 0x50, bounded)
                        .map_err(E::Tuning)?;
                    let radians = curve * f32::from_bits(0x3c8efa35);
                    let signed = radians * sign;
                    let timed = signed * dt;
                    let target = (timed * 80.0) * speed;
                    let suffix = self.before_face.as_mut().expect("retained");
                    suffix.turn = (target - suffix.turn) * f32::from_bits(0x3d23d70a);
                } else if self
                    .before_sprint
                    .as_ref()
                    .ok_or(E::Missing("locomotion fields"))?
                    .foot_down
                    == -1
                {
                    return Err(E::FootDown);
                }
                if collision_updated {
                    let turn = self
                        .try_calculate_ob_collision_turn(
                            world.columns()[2],
                            matches!(previous_action, 1 | 3),
                            refs,
                            numeric,
                            rounding,
                        )
                        .map_err(E::Turn)?;
                    self.before_face.as_mut().expect("retained").turn = turn;
                    let player = self.player.as_mut().expect("retained");
                    player.flags_bits &= !0x1e0;
                    player.flags_known_mask |= 0x1e0;
                } else {
                    let suffix = self.before_face.as_mut().expect("retained");
                    suffix.turn = suffix.step_turn_adjustment + suffix.turn;
                }
                let suffix = self.before_face.as_mut().expect("retained");
                if suffix.turn != 0.0 {
                    let (sin, cos) = try_sin_cos(suffix.turn, rounding).map_err(E::SinCos)?;
                    let rotation = M::heading_yaw(sin, cos);
                    suffix.extra_y_rotation = rotation
                        .columns()
                        .map(|v| v.known_lanes().expect("local yaw"));
                    world = world.compose(&rotation);
                    suffix.world_position = suffix.world_position.compose(&rotation);
                }
            }
            self.skater
                .as_mut()
                .expect("retained")
                .trajectory
                .try_ingest_locomotion_attribute(attribute, mirrored, changed, &world, || {
                    refs.try_create_locomotion_maximum_raw_time()
                })
                .map_err(E::Attribute)?;
        }
        self.try_correct_create_locomotion(collision_updated, refs, numeric)
            .map_err(E::Correction)?;
        self.try_finish_create_locomotion(
            actions,
            previous_action,
            &mut tuning,
            |trajectory, index| {
                trajectory.try_debug_draw_locomotion(index, false, refs, overlay, numeric)
            },
        )
        .map_err(E::Finish)
    }
}
