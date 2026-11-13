//! The completed output `PhysOutModule_MissionSystem` (`0x82e313a0`) reads
//! for the challenge system, and the `Actor` `cTriggerTester` positions
//! (`GetBasePosition` `0x825ed928`, `GetTopPosition` `0x825eda30`,
//! `GetCentreOfMass` `0x825eda80`).

use super::{PhysOutComponent, PhysOutSystemVector, PhysicalPlayerPhysOut, construct::ConcretePhysOutError};
use crate::{
    challenge::mission::{ContextUidLists, EndedTrick, FinishedAir, FinishedGrind, FinishedSequence, HomBail, MissionFrame},
    physics::FilteredPhysicsState,
    physics_state_ids::PHYSICS_CATEGORY_WIPEOUT,
};

type Error = ConcretePhysOutError<core::convert::Infallible>;

impl PhysicalPlayerPhysOut {
    /// One simulation update's mission facts. `tester_valid` is the Actor's
    /// own active flag; a teleporting skater is never a valid tester.
    #[expect(clippy::too_many_lines, reason = "one flat native output projection")]
    pub(crate) fn try_mission_frame(&self, player: i32, local: bool, tester_valid: bool) -> Result<MissionFrame, Error> {
        for component in [
            PhysOutComponent::Motion,
            PhysOutComponent::Skeleton,
            PhysOutComponent::State,
            PhysOutComponent::Ground,
            PhysOutComponent::SystemReckoning,
            PhysOutComponent::Intents,
            PhysOutComponent::Animation,
            PhysOutComponent::Scoring2,
            PhysOutComponent::Filtered,
            PhysOutComponent::OffBoard,
            PhysOutComponent::Trick,
            PhysOutComponent::Air,
        ] {
            self.require_complete_component(component)?;
        }
        let c = &self.concrete;
        let motion = c.motion.as_deref().ok_or(Error::NotConstructed)?;
        let skeleton = c.skeleton.as_deref().ok_or(Error::NotConstructed)?;
        let state = c.state.as_deref().ok_or(Error::NotConstructed)?;
        let ground = c.ground.as_deref().ok_or(Error::NotConstructed)?;
        let intents = c.intents.as_deref().ok_or(Error::NotConstructed)?;
        let animation = c.animation.as_deref().ok_or(Error::NotConstructed)?;
        let scoring = c.scoring.as_deref().ok_or(Error::NotConstructed)?;
        let filtered = c.filtered.as_deref().ok_or(Error::NotConstructed)?;
        let offboard = c.offboard.as_deref().ok_or(Error::NotConstructed)?;
        let trick = c.trick.as_deref().ok_or(Error::NotConstructed)?;
        let air_output = c.air.as_deref().ok_or(Error::NotConstructed)?;
        let position = skeleton.skeleton_root_matrix.columns()[3].xyz;
        let teleporting = filtered.filtered_physics_state == FilteredPhysicsState::Teleport;
        // `GetBasePosition`: a wiped-out skater's base is between the feet.
        let tester_base = if state.physics_state_category == PHYSICS_CATEGORY_WIPEOUT {
            let left = skeleton.skeleton_foot_left_pos.xyz;
            let right = skeleton.skeleton_foot_right_pos.xyz;
            std::array::from_fn(|i| (right[i] - left[i]).mul_add(0.5, left[i]))
        } else {
            position
        };
        let tricks_ended = scoring.tricks_ended.elements
            [..usize::try_from(scoring.tricks_ended.num_entries).unwrap_or(0).min(4)]
            .iter()
            .map(|e| {
                let s = &e.trick_scoring_settings;
                EndedTrick {
                    name: s.trick_name_hal.as_deref().unwrap_or_default().to_owned(),
                    is_valid: s.is_valid,
                    was_a_tap_trick: e.was_a_tap_trick,
                    trick_index: s.trick_index,
                    trick_type: s.trick_type,
                    score: e.score,
                    started_switched: e.started_switched,
                    trick_attrib_key: s.trick_attrib_key,
                }
            })
            .collect();
        let air = &scoring.air_finished_data;
        let initial = &air.initial_trick.trick_scoring_settings;
        let bail = &scoring.ho_m_bail_data;
        Ok(MissionFrame {
            player,
            local,
            position,
            tester_base,
            tester_top: skeleton.skeleton_head_pos.xyz,
            tester_centre_of_mass: self.try_camera_system_vector(PhysOutSystemVector::CentreOfMass)?.xyz,
            tester_valid: tester_valid && !teleporting,
            is_pushing: intents.is_pushing,
            is_new_auto_pump: intents.is_new_auto_pump,
            is_physically_pushing: motion.is_physically_pushing,
            is_intentional_pump: motion.is_intentional_pump,
            deck_speed: self.try_motion_deck_speed()?,
            surface_speed: self.try_motion_surface_speed()?,
            is_teleport_finished: state.is_teleport_finished,
            physics_requested_teleport: state.physics_requested_teleport,
            teleporting,
            is_regular_stance: animation.is_regular_stance,
            is_riding_switch: animation.is_riding_switch,
            is_riding_fakie: animation.is_riding_fakie,
            is_wipeout_teleport_pending: skeleton.is_wipeout_teleport_pending,
            time_until_wipeout_teleport: skeleton.time_until_wipeout_teleport,
            is_grabbing_object: offboard.is_grabbing_object,
            pumping_acceleration: ground.pumping_acceleration,
            finished_grind: scoring.just_finished_grind.then_some(FinishedGrind {
                length: scoring.finished_grind_info.length,
                pathed_guid: scoring.finished_grind_info.spline_pathed_guid,
                local_guid: scoring.finished_grind_info.spline_local_guid,
            }),
            finished_air: scoring.air_finished.then_some(FinishedAir {
                landed_successfully: air.landed_succesfully,
                first_scorable_id: if initial.is_valid { initial.scorable_id } else { -1 },
                landing_was_sketchy: air.landing_was_sketchy,
                player_spin180_count: air.player_spin180_count,
                started_switched: air.started_switched,
                grab_occured: air.grab_occured,
                tweak_occured: air.tweak_occured,
                in_air_frames: air.in_air_frames,
                height_to_peak: air.height_to_peak,
                horizontal_distance: air.horizontal_distance,
                grab_time: air.grab_time,
                player_num_flips: air.player_num_flips,
                big_trick_thresholds: std::array::from_fn(|i| air.big_trick_achievements.score_thresholds[i + 5].was_achieved),
                initial_context: context_uid_lists(&air.initial_context_data),
                end_context: context_uid_lists(&air.end_context_data),
                accumulated_context: context_uid_lists(&air.accumulated_context_data),
            }),
            finished_manual: scoring.just_finished_manual.then_some(scoring.finished_manual_length),
            finished_powerslide: scoring.just_finished_powerslide.then_some(scoring.finished_powerslide_length),
            tricks_ended,
            line_timer_finished: scoring.line_timer_finished,
            just_wiped_out: scoring.just_wiped_out,
            performed_gesture: scoring.just_performed_gesture.then_some(scoring.current_gesture),
            sequence_begun: scoring.sequence_begun,
            sequence_completed: scoring.sequence_completed.then_some(FinishedSequence {
                score: scoring.finished_sequence_score,
                start_visual_tick: scoring.finished_sequence_start_visual_game_tick,
                start_sim_tick: scoring.finished_sequence_start_sim_game_tick,
                replay_start_frame: scoring.finished_sequence_replay_start_frame_number,
                trick_count: scoring.finished_sequence_trick_count,
                stored_trick_count: scoring.finished_sequence_stored_trick_count,
                successfully: scoring.sequence_completed_successfully,
            }),
            banked_point_change: scoring.banked_point_change_this_frame,
            hom_bail_in_progress: scoring.ho_m_bail_in_progress,
            hom_bail_completed: scoring.ho_m_bail_completed,
            hom_body_tweak: bail.just_performed_body_tweak.then_some(bail.current_body_tweak),
            just_started_line_timer: scoring.just_started_line_timer,
            is_onboard_or_hippy_jumping: scoring.is_onboard_or_hippy_jumping,
            was_onboard_or_hippy_jumping: scoring.was_onboard_or_hippy_jumping,
            is_offboard_and_not_hippy_jumping: scoring.is_offboard_and_not_hippy_jumping,
            was_offboard_and_not_hippy_jumping: scoring.was_offboard_and_not_hippy_jumping,
            in_sequence: scoring.in_sequence,
            sequence_completed_successfully: scoring.sequence_completed_successfully,
            is_in_state_ground: scoring.is_in_state_ground,
            is_in_state_air: scoring.is_in_state_air,
            moving_wipeout_duration: bail.moving_wipeout_duration_in_seconds,
            manual_length: scoring.manual_length,
            manual_time: scoring.manual_time,
            anim_trick_bits: trick.current_animation.bits,
            wiping_out: filtered.filtered_physics_state == FilteredPhysicsState::Wipeout,
            finished_pattern_matches: scoring.finished_sequence_pattern_matches.did_match,
            current_pattern_matches: scoring.current_sequence_pattern_matches.did_match,
            current_air_initial_scorable: {
                let launch = &scoring.current_air_data.initial_trick.trick_scoring_settings;
                if launch.is_valid { launch.scorable_id } else { -1 }
            },
            finished_sequence_score: scoring.finished_sequence_score,
            line_score: scoring.line_score,
            in_a_current_trick: scoring.active_tricks.num_entries > 0,
            active_trick_types: scoring.active_tricks.elements
                [..usize::try_from(scoring.active_tricks.num_entries).unwrap_or(0).min(4)]
                .iter()
                .filter(|e| e.trick_scoring_settings.is_valid)
                .filter_map(|e| crate::physical_player_phys_out::conditioners::scorable_id_trick_type(e.trick_scoring_settings.scorable_id))
                .collect(),
            skateboard_airborne: air_output.is_skateboard_airborne,
            is_in_state_grinding: scoring.is_in_state_grinding,
            finished_grind_length: scoring.finished_grind_info.length,
            trick_types_in_completed_sequence: scoring.trick_types_in_completed_sequence,
            hom_score: bail.score,
            hom_effect_enabled: scoring.ho_m_is_effect_enabled,
            hom_bail: HomBail {
                save_image:bail.save_image(),
                score:bail.score,bone_scores:bail.bone_score_for_damage_level,
                bonus_scores:[bail.bail_duration_score,bail.accumulated_air_drop.drop_height_score,
                    bail.accumulated_air_drop.drop_time_score,bail.rotation_score,bail.bail_speed_score,
                    bail.body_tweak_score,bail.entity_sets[0].score,bail.entity_sets[1].score,bail.entity_sets[2].score],
                overall_duration:bail.overall_wipeout_duration_in_seconds,
                max_air_frames:bail.max_air_drop.duration_in_frames,rotation:bail.rotation,
                accumulated_air_drop_height:bail.accumulated_air_drop.drop_height,
                moving_duration:bail.moving_wipeout_duration_in_seconds,
                skeleton_over_duration:bail.skeleton_wipeout_over_duration_in_seconds,
                severe_duration:bail.severe_wipeout_duration_in_seconds,
                bone_damage_levels: std::array::from_fn(|i| bail.bones.bones[i].damage_level),
                entity_counts: std::array::from_fn(|i| bail.entity_sets[i].count),
                security_guards_hit:bail.entity_sets[0].collided_entities[..bail.entity_sets[0].count as usize].iter().filter(|e|e.entity_type==3).count() as u32,
                max_air_drop_height: bail.max_air_drop.drop_height,
                accumulated_air_drop_frames: bail.accumulated_air_drop.duration_in_frames,
                bone_score: bail.bone_score,
                bonus_score: bail.bonus_score,
                rotation_score: bail.rotation_score,
                num_body_tweaks_performed: bail.num_body_tweaks_performed,
                bail_speed: bail.bail_speed,
                current_body_tweak: bail.current_body_tweak,
            },
            filtered_grinding: filtered.is_grinding,
            filtered_grind_pathed_guid: filtered.grind_pathed_guid,
            filtered_grind_local_guid: filtered.grind_local_guid,
            current_gesture: scoring.current_gesture,
            active_tricks: scoring.active_tricks.elements
                [..usize::try_from(scoring.active_tricks.num_entries).unwrap_or(0).min(4)]
                .iter()
                .map(|e| (e.trick_scoring_settings.is_valid, e.trick_scoring_settings.scorable_id))
                .collect(),
            big_trick_achieved: std::array::from_fn(|i| scoring.big_trick_achieved_data.score_thresholds[i].was_achieved),
            finished_grind_object: {
                let object = &scoring.finished_grind_info.object_info;
                (object.is_valid, object.entity_type, object.unique_id)
            },
            vote: None,
        })
    }
}

/// A `ContextSensitiveDataGroups`' `mUIDData[0]` lists.
fn context_uid_lists(groups: &super::scoring::ContextGroups) -> ContextUidLists {
    ContextUidLists {
        uids: std::array::from_fn(|g| {
            std::array::from_fn(|i| {
                let uid = &groups.context_sensitive_data[g].uid_data[0].uids[i];
                (uid.unique_id, uid.entity_type)
            })
        }),
        counts: std::array::from_fn(|g| groups.context_sensitive_data[g].uid_data[0].count),
    }
}

impl PhysicalPlayerPhysOut {
    /// VDWriter.UpdateHoMSlowMotion827fd448. PDB offsets15728+1236/1244/1240.
    pub(crate) fn try_hom_slow_motion_durations(&self)->Result<Option<[f32;3]>,Error> {
        self.require_complete_component(PhysOutComponent::Scoring2)?;
        let scoring=self.concrete.scoring.as_deref().ok_or(Error::NotConstructed)?;
        if !scoring.ho_m_bail_in_progress || !scoring.ho_m_is_effect_enabled {return Ok(None);}
        let bail=&scoring.ho_m_bail_data;
        Ok(Some([bail.overall_wipeout_duration_in_seconds,
            bail.severe_wipeout_duration_in_seconds,bail.skeleton_wipeout_over_duration_in_seconds]))
    }
}
