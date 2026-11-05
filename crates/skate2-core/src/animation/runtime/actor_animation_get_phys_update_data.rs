//! Complete selected non-locomotion `GetPhysUpdateData82c96d58` path.
//! Reads the retained animation buffers after their actual synchronization.
use super::{ActorAnimationConstruction, AnimationLocomotionTrajectory};
use crate::skeleton_root_transforms::SkeletonRootVector as WalkVector;
use crate::{
    actor_animation_construction::initialization::bind_pose::BindPoseError,
    actor_construction::anim_out_phys_in::{AnimOutConstructionStage, AnimOutPhysInOwner},
    attribute_runtime::AttributeRuntimeError,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
};

#[cfg(test)]
#[path = "actor_animation_get_phys_update_data_tests.rs"]
mod tests;

#[path = "../locomotion/actor_animation_locomotion_physics.rs"]
pub(crate) mod locomotion;
#[path = "../locomotion/actor_animation_locomotion_trajectory.rs"]
pub(crate) mod trajectory;

const REQUEST_WIPEOUT: u32 = 0x0800_0000;

#[derive(Debug)]
pub(crate) enum AnimationPhysicsDataError {
    Missing(&'static str),
    UnwrittenFlag(u32),
    BoneCount { animation: i32, destination: u32 },
    FootIndex(i32),
    StateIndex(i32),
    UnwrittenInactiveActionProbe,
    PendingLocomotion,
    Attribute(AttributeRuntimeError),
    Pose(BindPoseError),
    PoseNan { bone: usize, axis: usize },
    FootNan(usize),
    Cvar(InputCvarError),
}
impl std::fmt::Display for AnimationPhysicsDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(field) => write!(f, "animation physics missing {field}"),
            Self::UnwrittenFlag(mask) => write!(f, "animation physics flags {mask:08x} unknown"),
            Self::BoneCount {
                animation,
                destination,
            } => write!(f, "animation physics bone counts {animation}/{destination}"),
            Self::FootIndex(index) => write!(
                f,
                "animation physics foot index {index} outside source array"
            ),
            Self::StateIndex(index) => {
                write!(f, "animation physics state index {index} unsupported")
            }
            Self::Pose(error) => write!(f, "animation physics pose: {error:?}"),
            Self::PoseNan { bone, axis } => {
                write!(f, "animation physics bone {bone} axis {axis} has NaN XYZ")
            }
            Self::FootNan(foot) => write!(f, "animation physics foot {foot} has NaN XYZ"),
            Self::Cvar(error) => write!(f, "animation physics cvar: {error:?}"),
            Self::Attribute(error) => write!(f, "animation physics attribute: {error:?}"),
            other @ (Self::UnwrittenInactiveActionProbe | Self::PendingLocomotion) => {
                write!(f, "animation physics continuation: {other:?}")
            }
        }
    }
}

impl AnimationLocomotionTrajectory {
    /// Native action probe `82c96dcc..6e14`, also used by both runout getters.
    /// Inactive state -1 reads the preceding foot's reserved trailing word,
    /// never state zero. Its value is known only on the actual Sim allocation.
    pub(crate) fn physics_action_probe(&self) -> Result<u32, AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        let wrapped = if self.physical_foot_index < 0 {
            self.physical_foot_index
                .checked_add(4)
                .ok_or(E::FootIndex(self.physical_foot_index))?
        } else {
            self.physical_foot_index % 4
        };
        let index = usize::try_from(wrapped)
            .ok()
            .filter(|index| *index < 4)
            .ok_or(E::FootIndex(self.physical_foot_index))?;
        if self.current_state == -1 {
            let predecessor = if index == 0 {
                &self.input_feet[3]
            } else {
                &self.physical_feet[index - 1]
            };
            return predecessor
                .trailing_action_probe_word
                .ok_or(E::UnwrittenInactiveActionProbe);
        }
        let state = usize::try_from(self.current_state)
            .ok()
            .filter(|state| *state < 3)
            .ok_or(E::StateIndex(self.current_state))?;
        Ok(u32::from_ne_bytes(
            self.physical_feet[index].states[state].action.to_ne_bytes(),
        ))
    }
}

fn insert_flag(word: &mut u32, mask: u32, value: bool) {
    *word = (*word & !mask) | if value { mask } else { 0 };
}
fn xyz_nan(vector: &[f32]) -> bool {
    vector[..3].iter().any(|value| value.is_nan())
}

impl<K, W> ActorAnimationConstruction<K, W> {
    /// GetIsSwitch82cb7580 reads relative stance, not the previous PhysOut copy.
    pub(crate) fn try_is_riding_switch(&self) -> Result<bool, AnimationPhysicsDataError> {
        Ok(self
            .player
            .as_ref()
            .ok_or(AnimationPhysicsDataError::Missing("player fields"))?
            .relative_stance
            == 1)
    }

    /// IsAirMountRequested82cb7658, retained player bit9.
    pub(crate) fn try_is_air_mount_requested(&self) -> Result<bool, AnimationPhysicsDataError> {
        self.physics_flag(0x200)
    }

    /// IsInJumpAir82c92f88: physical foot state duration, mirrored locked-foot parity.
    pub(crate) fn try_is_in_jump_air(&self, foot: i32) -> Result<bool, AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        let mirrored = self.physics_flag(0x4000_0000)?;
        let trajectory = &self
            .skater
            .as_ref()
            .ok_or(E::Missing("trajectory"))?
            .trajectory;
        let locked = trajectory.foot_locked.wrapping_add(i32::from(mirrored));
        if trajectory.current_state != 2 {
            return Ok(false);
        }
        let physical = trajectory
            .physical_feet
            .get(trajectory.physical_foot_index as usize)
            .ok_or(E::FootIndex(trajectory.physical_foot_index))?;
        let duration = physical.states[2].time;
        let threshold = f32::from_bits(
            self.offboard_attributes()
                .ok_or(E::Missing("offboard attributes"))?
                .try_scalar_word(0x971e_130c_2719_3d76, 0)
                .map_err(E::Attribute)?
                .unwrap_or(0),
        );
        // Native BLE rejects equality, while unordered passes this gate.
        Ok(!(duration <= threshold) && locked % 2 == foot)
    }

    /// TimeToFootChange82cb7720 reads the retained animation suffix timer.
    pub(crate) fn try_time_to_foot_change(&self) -> Result<f32, AnimationPhysicsDataError> {
        Ok(self
            .before_face
            .as_ref()
            .ok_or(AnimationPhysicsDataError::Missing("locomotion suffix"))?
            .time_to_foot_change)
    }

    /// IsInLocomotion82cb7600 && IsPastApex82cb7b48. The latter deliberately
    /// uses INPUT foot state2 velocity/time, not physical feet or trajectory time.
    pub(crate) fn try_is_past_locomotion_apex(&self) -> Result<bool, AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        if !self.physics_flag(0x0010_0000)? {
            return Ok(false);
        }
        let trajectory = &self
            .skater
            .as_ref()
            .ok_or(E::Missing("trajectory"))?
            .trajectory;
        let state = &trajectory
            .input_feet
            .get(trajectory.physical_foot_index as usize)
            .ok_or(E::FootIndex(trajectory.physical_foot_index))?
            .states[2];
        // OneG initializer82f81038, FDIVS then FMULS then ordered LT.
        Ok(state.vectors[1].xyz[1] * (-1.0 / f32::from_bits(0xc11c_f5c3)) < state.time)
    }

    /// Native mirror getter uses the existing retained player flags.
    pub(crate) fn try_camera_mirrored(&self) -> Result<bool, AnimationPhysicsDataError> {
        self.physics_flag(0x4000_0000)
    }

    /// Native front getter precedes the lazy back getter (`82cb7a58/7ad0`).
    pub(crate) fn try_camera_running_out(&self) -> Result<bool, AnimationPhysicsDataError> {
        Ok(self.running_out_front()? || self.running_out_back()?)
    }

    /// SkaterAnim::IsDismounting82cb7940: its explicit bit wins; otherwise
    /// sample the same retained physical-foot action word as the runout getter.
    pub(crate) fn try_is_dismounting(&self) -> Result<bool, AnimationPhysicsDataError> {
        if self.physics_flag(0x0002_0000)? {
            return Ok(true);
        }
        Ok(self.physics_action_probe()? == 12)
    }

    /// SkaterAnim::IsLanding82cb79b8: the live landing bit wins; otherwise
    /// compare the retained physical-foot action with ForcedLand/ForcedLandOut.
    pub(crate) fn try_is_landing(&self) -> Result<bool, AnimationPhysicsDataError> {
        if self.physics_flag(0x0000_8000)? {
            return Ok(true);
        }
        let action = self.physics_action_probe()?;
        Ok(action == 13 || action == 14)
    }

    /// Motion IsRunningOut82c9d538 calls both virtual getters in order before
    /// combining their return bytes, even if the front getter is true.
    pub(crate) fn try_is_running_out(&self) -> Result<bool, AnimationPhysicsDataError> {
        let front = self.running_out_front()?;
        let back = self.running_out_back()?;
        Ok(front || back)
    }

    pub(crate) fn running_out_front(&self) -> Result<bool, AnimationPhysicsDataError> {
        Ok(self.physics_flag(0x4000)? || self.physics_action_probe()? == 15)
    }

    pub(crate) fn running_out_back(&self) -> Result<bool, AnimationPhysicsDataError> {
        Ok(self.physics_flag(0x2000)? || self.physics_action_probe()? == 16)
    }

    /// ISkaterAnim+0x14 -> SetIsWeightOnNose82cb7568 changes only bit 3
    /// in native MSB numbering. Physics publication reads this same flag.
    pub(crate) fn try_set_weight_on_nose(
        &mut self,
        value: bool,
    ) -> Result<(), AnimationPhysicsDataError> {
        let player = self
            .player
            .as_mut()
            .ok_or(AnimationPhysicsDataError::Missing("player fields"))?;
        const MASK: u32 = 0x1000_0000;
        player.flags_bits = (player.flags_bits & !MASK) | if value { MASK } else { 0 };
        player.flags_known_mask |= MASK;
        Ok(())
    }

    /// ISkaterAnim+0x24 -> FlipBoardBackwards82cb75a8 toggles only bit 0 in
    /// native MSB numbering (`mIsAnimBoardBackwards`, read by AddBindPose).
    pub(crate) fn try_flip_board_backwards(&mut self) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        let player = self.player.as_mut().ok_or(E::Missing("player fields"))?;
        const MASK: u32 = 0x8000_0000;
        if player.flags_known_mask & MASK == 0 {
            return Err(E::UnwrittenFlag(MASK));
        }
        player.flags_bits ^= MASK;
        Ok(())
    }

    fn physics_flag(&self, mask: u32) -> Result<bool, AnimationPhysicsDataError> {
        // Keep the same selected movement authority at the SetUpPhysics read.
        if self.skate3_movement && matches!(mask, 0x0010_0000 | 0x0008_0000) {
            return Ok(false);
        }
        use AnimationPhysicsDataError as E;
        let player = self.player.as_ref().ok_or(E::Missing("player fields"))?;
        if player.flags_known_mask & mask != mask {
            return Err(E::UnwrittenFlag(mask));
        }
        Ok(player.flags_bits & mask != 0)
    }
    pub(crate) fn physics_action_probe(&self) -> Result<u32, AnimationPhysicsDataError> {
        self.skater
            .as_ref()
            .ok_or(AnimationPhysicsDataError::Missing("trajectory"))?
            .trajectory
            .physics_action_probe()
    }

    /// Source entry through return when `WasInLocomotion` is false.
    /// Unsupported branches retain every
    /// earlier native write; callers must not retry as a rollback operation.
    pub(crate) fn try_get_non_locomotion_physics_update(
        &mut self,
        destination: &mut AnimOutPhysInOwner,
        time_delta: f32,
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        if self.begin_physics_update(destination, time_delta)? {
            return Err(E::PendingLocomotion);
        }
        self.clear_non_locomotion_history()?;
        self.finish_physics_update(destination, cvars)
    }

    fn begin_physics_update(
        &mut self,
        destination: &mut AnimOutPhysInOwner,
        time_delta: f32,
    ) -> Result<bool, AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        let bones = self
            .base
            .as_ref()
            .ok_or(E::Missing("base animation"))?
            .bone_count;
        if u32::try_from(bones).ok() != Some(destination.size) {
            return Err(E::BoneCount {
                animation: bones,
                destination: destination.size,
            });
        }
        if destination.stage != AnimOutConstructionStage::Returned {
            return Err(E::Missing("constructed animation physics input"));
        }
        // 6dbc..6dc8: these stores precede even the first action probe.
        destination.time_delta = time_delta;
        destination.foot_surface_ids = [0; 2];
        destination.locomotion_flags &= !REQUEST_WIPEOUT;
        let jumping = self.physics_action_probe()? == 1;
        insert_flag(&mut destination.locomotion_flags, 0x0400_0000, jumping);
        // Runout getters independently read live flags then (if needed) action.
        let running = self.try_camera_running_out()?;
        insert_flag(&mut destination.locomotion_flags, 0x0200_0000, running);
        self.physics_flag(0x0008_0000)
    }

    fn clear_non_locomotion_history(&mut self) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        // 973f0..9740c: source non-locomotion branch clears history validity,
        // old WasInLocomotion and all four last-COM lanes before validation.
        let player = self.player.as_mut().expect("flags already checked");
        player.flags_bits &= !0x0008_0008;
        player.flags_known_mask |= 0x0008_0008;
        self.before_face
            .as_mut()
            .ok_or(E::Missing("constructor suffix"))?
            .last_centre_of_mass = WalkVector::known([0.0; 4]);
        Ok(())
    }

    fn finish_physics_update(
        &mut self,
        destination: &mut AnimOutPhysInOwner,
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhysicsDataError> {
        self.copy_physics_pose_buffers(destination, destination.size as usize)?;
        self.publish_physics_state_flags(destination)?;
        self.publish_air_wipeout_flags(destination, cvars)
    }

    /// `82c97710..77f0`: retained physical-state duration and full-width live
    /// Offboard key, followed by both live cvar comparisons.
    fn publish_air_wipeout_flags(
        &self,
        destination: &mut AnimOutPhysInOwner,
        cvars: &InputCvarReferences,
    ) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        let trajectory = &self
            .skater
            .as_ref()
            .ok_or(E::Missing("trajectory"))?
            .trajectory;
        let (air_time, landing_time, big_air) = if trajectory.current_state == 2 {
            // Unlike the entry action query, this access does not wrap index.
            let foot = usize::try_from(trajectory.physical_foot_index)
                .ok()
                .and_then(|index| trajectory.physical_feet.get(index))
                .ok_or(E::FootIndex(trajectory.physical_foot_index))?;
            let landing_time = foot.states[2].time;
            let threshold = self
                .offboard_attributes
                .as_ref()
                .ok_or(E::Missing("Offboard attributes"))?
                .try_scalar_word(0x971e_130c_2719_3d76, 0)
                .map_err(E::Attribute)?
                .map_or(0.0, f32::from_bits);
            // Raw BGT at 97788: unordered is false.
            (trajectory.time, landing_time, landing_time > threshold)
        } else {
            (0.0, 0.0, false)
        };
        insert_flag(&mut destination.locomotion_flags, 0x2000_0000, big_air);
        self.publish_wipeout_flag(destination, cvars, air_time, landing_time)
    }

    fn publish_wipeout_flag(
        &self,
        destination: &mut AnimOutPhysInOwner,
        cvars: &InputCvarReferences,
        air_time: f32,
        landing_time: f32,
    ) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        // Raw4099 at977b0/c8 branches when CR6.GT is clear, including
        // unordered. Both gates require a strict ordered greater result.
        let minimum = cvars.try_min_time_in_air_to_wipeout().map_err(E::Cvar)?;
        let request = if air_time > minimum {
            let landing = cvars.try_time_to_land_to_wipeout().map_err(E::Cvar)?;
            if landing_time > landing {
                !self.physics_flag(0x200)?
            } else {
                false
            }
        } else {
            false
        };
        if request {
            destination.locomotion_flags |= REQUEST_WIPEOUT;
        }
        Ok(())
    }

    fn copy_physics_pose_buffers(
        &self,
        destination: &mut AnimOutPhysInOwner,
        bones: usize,
    ) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        let initialization = self
            .initialization
            .as_ref()
            .ok_or(E::Missing("initialized body buffers"))?;
        let global = initialization
            .global_matrices
            .matrices()
            .map_err(E::Pose)?
            .ok_or(E::Missing("synchronized global body pose"))?;
        if global.len() != bones {
            return Err(E::Missing("global pose size"));
        }
        for (bone, matrix) in global.iter().enumerate() {
            for (axis, vector) in matrix.chunks_exact(4).enumerate() {
                if xyz_nan(vector) {
                    return Err(E::PoseNan { bone, axis });
                }
            }
        }
        for (foot, target) in destination.foot_targets.iter().enumerate() {
            if xyz_nan(&target.xyz) {
                return Err(E::FootNan(foot));
            }
        }
        let output = destination
            .global_pose
            .as_mut()
            .ok_or(E::Missing("global destination"))?;
        if output.len() != bones {
            return Err(E::Missing("global destination size"));
        }
        output.copy_from_slice(&global);
        destination
            .mark_pose_w_known(
                crate::actor_construction::anim_out_phys_in::pose_lanes::PoseBuffer::Global,
            )
            .map_err(|_| E::Missing("global destination metadata"))?;
        // Native global memcpy97590 precedes local memcpy975a4. Do not turn a
        // late missing local output into rollback of the global publication.
        let local = initialization
            .local_matrices
            .matrices()
            .map_err(E::Pose)?
            .ok_or(E::Missing("synchronized local body pose"))?;
        let output = destination
            .local_pose
            .as_mut()
            .ok_or(E::Missing("local destination"))?;
        if local.len() != bones || output.len() != bones {
            return Err(E::Missing("local pose size"));
        }
        output.copy_from_slice(&local);
        destination
            .mark_pose_w_known(
                crate::actor_construction::anim_out_phys_in::pose_lanes::PoseBuffer::Local,
            )
            .map_err(|_| E::Missing("local destination metadata"))?;
        Ok(())
    }

    fn publish_physics_state_flags(
        &mut self,
        destination: &mut AnimOutPhysInOwner,
    ) -> Result<(), AnimationPhysicsDataError> {
        use AnimationPhysicsDataError as E;
        destination.flags.board_flipped =
            self.physics_flag(0x4000_0000)? != self.physics_flag(0x8000_0000)?;
        destination.flags.mirrored = self.physics_flag(0x4000_0000)?;
        destination.flags.riding_switch = self
            .player
            .as_ref()
            .ok_or(E::Missing("player fields"))?
            .relative_stance
            == 1;
        destination.flags.riding_fakie = self.physics_flag(0x2000_0000)?;
        let fakie = self.physics_flag(0x2000_0000)?;
        let nose = self.physics_flag(0x1000_0000)?;
        destination.flags.weight_forwards = if fakie { !nose } else { nose };
        destination.flags.regular_stance =
            self.player.as_ref().expect("player checked").natural_stance == 0;
        insert_flag(
            &mut destination.locomotion_flags,
            0x8000_0000,
            self.physics_flag(0x0010_0000)?,
        );
        insert_flag(
            &mut destination.locomotion_flags,
            0x4000_0000,
            self.physics_flag(0x0008_0000)?,
        );
        insert_flag(
            &mut destination.locomotion_flags,
            0x1000_0000,
            self.physics_flag(0x400)?,
        );
        let frames = self
            .before_sprint
            .as_ref()
            .ok_or(E::Missing("air dismount frame count"))?
            .air_dismount_revert_frames;
        self.player.as_mut().expect("player checked").flags_bits &= !0x400;
        destination.air_dismount_revert_frames = frames;
        let locomotion = self.physics_flag(0x0010_0000)?;
        let suffix = self
            .before_face
            .as_mut()
            .ok_or(E::Missing("locomotion timer"))?;
        suffix.time_since_leaving_locomotion += f32::from_bits(0x3c88_8889);
        insert_flag(
            &mut self.player.as_mut().expect("player checked").flags_bits,
            0x0008_0000,
            locomotion,
        );
        Ok(())
    }
}
