//! TU3 Hall of Meat for Skate 3 physics: WipeoutScorer::Update 82DAE4F0 and the TU3 helpers
//! it calls, over the shared scorer's bones, trackers, air drops and entity scorers. Skate 3
//! scores the run-up into a bail (air, drop and rotation while airborne), decays repeated bone
//! hits, and adds body-tweak spins and the wipeout response to the bonus.
use super::{
    super::scoring::Scoring2,
    scoring_attributes::{ScoreError, skate3_hom},
    scoring_external::ScoringExternalInput,
    scoring_wipeout_lifecycle::{WipeoutFrame, WipeoutTuning},
    wipeout_score::WipeoutScorer,
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    physics::FilteredPhysicsState,
    reckoning_numeric::{HostReckoningNumeric, ReckoningVector, try_length, try_unsigned_angle_2d},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::Arc;

const DT: f32 = f32::from_bits(0x3c88_8889);
static TRACE: std::sync::OnceLock<(bool, bool)> = std::sync::OnceLock::new();
/// (SKATE2_TRACE_HOM per-frame, SKATE2_TRACE_SCORING=1 per bail), read once.
fn trace() -> (bool, bool) {
    *TRACE.get_or_init(|| {
        (
            std::env::var_os("SKATE2_TRACE_HOM").is_some(),
            std::env::var("SKATE2_TRACE_SCORING").is_ok_and(|v| v == "1"),
        )
    })
}
const DEGREES: f32 = f32::from_bits(0x4265_2ee1);

/// TU3 WipeoutScorer members Skate 2's scorer does not have (TU3 byte offsets).
#[derive(Default)]
pub(super) struct Skate3HoM {
    /// +2229 the pre-wipeout window (airborne, or within 0.1 s of it) and its timer +2236.
    window: bool,
    window_timer: f32,
    /// +2235 the bail speed has been scored this run.
    speed_scored: bool,
    /// +2036 frames left of the broken-bone hold.
    broken_hold: u32,
    /// +2068/+2092/+2096 wipeout response strength samples, sum and maximum.
    response_samples: u32,
    response_sum: f32,
    response_max: f32,
    /// +2100 the combined rotation a body tweak's spin is measured from.
    last_rotation: f32,
    /// +2104/+2108 grounded time since the last air in a wipeout and its maximum.
    ground_time: f32,
    max_ground_time: f32,
    /// +2112 scored time; +2116 grounded time in the wipeout.
    time: f32,
    wipeout_ground_time: f32,
    /// +2120/+2136/+2152 per body tweak: longest hold, total hold, rotation.
    tweak_max_time: [f32; 4],
    tweak_time: [f32; 4],
    tweak_rotation: [f32; 4],
    /// +2168 entity collisions while body tweak 2 is held; +2176 the previous tweak.
    tweak_entity_score: u32,
    previous_tweak: i32,
}

fn hom() -> Result<&'static skate3_hom::Data, ScoreError> {
    skate3_hom::data().map_err(ScoreError::NativeCatalog)
}
fn score(value: f32) -> u32 {
    value as i64 as u32
}
fn length(vector: SkeletonRootVector) -> Result<f32, ScoreError> {
    try_length(vector.operand(), &mut HostReckoningNumeric).map_err(ScoreError::Numeric)
}
fn airborne(filtered: FilteredPhysicsState) -> bool {
    matches!(filtered, FilteredPhysicsState::Air | FilteredPhysicsState::OffBoardAir)
}

impl WipeoutScorer {
    /// Update 82DAE4F0.
    pub(super) fn update_wipeout_skate3(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        input: &ScoringExternalInput,
        frame: &WipeoutFrame<'_>,
        tuning: &WipeoutTuning,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let data = hom()?;
        let was_wiping = self.is_wiping_out;
        let was_window = self.skate3.window;
        let in_air = airborne(frame.filtered);
        // Scoring stops once the respawn countdown (State+68) or a teleport request (+69) starts.
        let wiping = frame.filtered == FilteredPhysicsState::Wipeout
            && !frame.skate3_teleport_requested
            && !frame.skate3_countdown;
        self.is_wiping_out = wiping;
        if frame.filtered == FilteredPhysicsState::Wipeout && trace().0 {
            let force = frame.collision.bones.iter().map(|b| b.main.force.max(b.sub.force)).fold(0.0f32, f32::max);
            eprintln!(
                "hom skate3 wiping={wiping} countdown={} teleport={} locked={} over={} frames={} moving={} force={force:.3} response={:.3} bones={}",
                frame.skate3_countdown, frame.skate3_teleport_requested, frame.skate3_air_locked,
                frame.skeleton_wipeout_over, self.overall_wipeout_duration_frames,
                self.moving_wipeout_duration_frames, frame.skate3_response_strength,
                self.bone_score(None).unwrap_or(0),
            );
        }
        self.skate3.window_timer = if in_air { 0.0 } else { self.skate3.window_timer + DT };
        let window = in_air || (was_window && self.skate3.window_timer < 0.1);
        self.skate3.window = window;
        let window_opened = !was_window && window;
        let unannounced = !was_window && !was_wiping && wiping;
        if window_opened || unannounced {
            self.reset_skate3(frame.hips_matrix)?;
            if unannounced {
                self.bail_start_velocity = frame.centre_of_mass_velocity;
                self.bail_speed_score = score(data.s3_curve(2820, length(self.bail_start_velocity)?));
                self.skate3.speed_scored = true;
            }
        }
        if was_wiping != wiping {
            if wiping {
                self.start_wipeout_skate3(data)?;
            } else {
                out.ho_m_bail_completed = true;
                self.output_bail_skate3(data, out)?;
                if trace().1 {
                    let bail = &out.ho_m_bail_data;
                    let damage = self.bones.iter().map(|b| b.damage_amount).fold(0.0f32, f32::max);
                    eprintln!(
                        "scoring skate3 bail score={} bones={} bonus={} speed={}({:.2} m/s) rotation={}({:.0} deg) drop={}+{}({:.2} m) response={} duration={}({:.2} s) tweaks={} max_damage={damage:.3}",
                        bail.score, bail.bone_score, bail.bonus_score, bail.bail_speed_score, bail.bail_speed,
                        bail.rotation_score, bail.rotation, bail.accumulated_air_drop.drop_time_score,
                        bail.accumulated_air_drop.drop_height_score, bail.max_air_drop.drop_height,
                        self.skate3.response_sum, bail.bail_duration_score,
                        bail.moving_wipeout_duration_in_seconds, bail.body_tweak_score,
                    );
                }
            }
        }
        if wiping {
            self.update_in_wipeout_skate3(vault, input, frame, data, out)?;
        } else {
            self.update_outside_skate3(frame, data)?;
        }
        self.update_character_damage(
            input.reset_player_damage,
            tuning.forced_render_damage_level,
            tuning.damage_fade_in,
            out,
        )
    }

    /// Reset 82DAD0A8: Skate 2's reset set, without touching the wipeout flag, plus the TU3
    /// members.
    fn reset_skate3(&mut self, matrix: SkeletonRootMatrix) -> Result<(), ScoreError> {
        let wiping = self.is_wiping_out;
        let (window, window_timer) = (self.skate3.window, self.skate3.window_timer);
        self.reset_wipeout(matrix)?;
        self.is_wiping_out = wiping;
        self.skate3 = Skate3HoM {
            window,
            window_timer,
            last_rotation: self.banked_combined_rotation,
            previous_tweak: -1,
            ..Skate3HoM::default()
        };
        for bone in &mut self.bones {
            bone.running_damage = 0.0;
        }
        Ok(())
    }

    /// 82DADB48: heavy landing foot damage on entering the wipeout.
    fn start_wipeout_skate3(&mut self, data: &skate3_hom::Data) -> Result<(), ScoreError> {
        let left = data.s3_curve(2420, self.left_foot_heavy_landing_speed) * data.damage_scale;
        let right = data.s3_curve(2420, self.right_foot_heavy_landing_speed) * data.damage_scale;
        self.damage_bone_skate3(data, 15, 15, left)?;
        self.damage_bone_skate3(data, 14, 14, data.toe_scale * left)?;
        self.damage_bone_skate3(data, 19, 19, right)?;
        self.damage_bone_skate3(data, 18, 18, data.toe_scale * right)?;
        self.left_foot_heavy_landing_speed = 0.0;
        self.right_foot_heavy_landing_speed = 0.0;
        Ok(())
    }

    /// DamageBone 82DAE6F0: a decaying running total per bone, the damage its maximum.
    fn damage_bone_skate3(
        &mut self,
        data: &skate3_hom::Data,
        index: usize,
        source: usize,
        amount: f32,
    ) -> Result<(), ScoreError> {
        let bone = self.bones.get_mut(index).ok_or(ScoreError::Invalid("wipeout bone index"))?;
        bone.running_damage += amount;
        bone.damage_amount = if bone.damage_amount - bone.running_damage >= 0.0 {
            bone.damage_amount
        } else {
            bone.running_damage
        };
        for link in 0..4 {
            let offset = index * 84 + 4 + link * 8;
            let word = |at: usize| {
                data.bone_word(at).ok_or(ScoreError::Invalid("Skate 3 HoM bone layout offset"))
            };
            let other = word(offset)? as i32;
            if other != -1 && other != source as i32 {
                let damage = f32::from_bits(word(offset + 4)?) * amount;
                if damage > 0.1 {
                    self.damage_bone_skate3(data, other as usize, index, damage)?;
                }
            }
        }
        Ok(())
    }

    /// PreUpdateBones 82DAE7A8 and UpdateBones 82DAE9A8 (CheckForDamage inlined: contact force
    /// times 2 and the authored damage scale, above 0.1).
    fn update_bones_skate3(
        &mut self,
        data: &skate3_hom::Data,
        frame: &WipeoutFrame<'_>,
    ) -> Result<(), ScoreError> {
        for bone in &mut self.bones {
            bone.position_last_frame = bone.position;
            bone.damage_level_last_frame = bone.damage_level;
        }
        self.is_airborne = true;
        for index in 0..25 {
            let part = match index {
                23 => 1,
                24 => 23,
                _ => index + 1,
            };
            let contact = &frame.collision.bones[part];
            let part_data = if index >= 23 { &contact.sub } else { &contact.main };
            let bone = &mut self.bones[index];
            bone.position = contact.position;
            bone.contact_surface_id = part_data.surface_id;
            bone.is_airborne =
                !(contact.hit_world && part_data.normal.xyz[1] > f32::from_bits(0x3ea8_f5c3));
            let damage = part_data.force * 2.0 * data.damage_scale;
            if damage > 0.1 {
                self.damage_bone_skate3(data, index, index, damage)?;
            }
            self.is_airborne &= self.bones[index].is_airborne;
        }
        Ok(())
    }

    /// 82DAF1D8.
    fn update_in_wipeout_skate3(
        &mut self,
        vault: &Arc<AttributeRuntimeVault>,
        input: &ScoringExternalInput,
        frame: &WipeoutFrame<'_>,
        data: &skate3_hom::Data,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        let was_airborne = self.is_airborne;
        let locked = frame.skate3_air_locked;
        if !locked {
            self.skate3.time += DT;
        }
        self.update_bones_skate3(data, frame)?;
        let first = self.overall_wipeout_duration_frames == 0;
        for (index, bone) in self.bones.iter_mut().enumerate() {
            bone.damage_level = -1;
            for level in (0..6).rev() {
                let threshold = data
                    .bone_word(index * 84 + 40 + level * 8)
                    .map(f32::from_bits)
                    .ok_or(ScoreError::Invalid("Skate 3 HoM bone layout offset"))?;
                if threshold > 0.0 && bone.damage_amount > threshold {
                    bone.damage_level = level as i32;
                    break;
                }
            }
            bone.running_damage *= 0.8;
            if first {
                bone.position_last_frame = bone.position;
            }
        }
        if !was_airborne && self.is_airborne {
            self.start_drop_skate3(frame.damped_centre_of_mass);
        } else if was_airborne && !self.is_airborne {
            self.air_drop_skate3(data, locked, frame)?;
            self.end_air_skate3();
        }
        let before = self.skate3.tweak_entity_score;
        self.update_entities(vault, input)?;
        if self.current_body_tweak == 2 || self.skate3.previous_tweak == 2 {
            let pedestrian = self.pedestrian.as_ref().map_or(0, |s| s.num_new_collisions_this_frame);
            let vehicle = self.vehicle.as_ref().map_or(0, |s| s.num_new_collisions_this_frame);
            let dmo = self.dmo.published_value().map_or(0, |s| s.base.num_new_collisions_this_frame);
            self.skate3.tweak_entity_score = self
                .skate3
                .tweak_entity_score
                .wrapping_add(pedestrian)
                .wrapping_add(vehicle)
                .wrapping_add(dmo);
        }
        if before > self.skate3.tweak_entity_score && self.skate3.previous_tweak == 2 {
            self.skate3.previous_tweak = -1;
        }
        if self.is_airborne {
            self.air_drop_skate3(data, locked, frame)?;
            self.skate3.ground_time = 0.0;
        } else if !locked {
            self.skate3.wipeout_ground_time += DT;
            self.skate3.ground_time += DT;
            if self.skate3.ground_time - self.skate3.max_ground_time >= 0.0 {
                self.skate3.max_ground_time = self.skate3.ground_time;
            }
        }
        self.bank_rotation_skate3(frame.hips_matrix, locked || frame.skeleton_wipeout_over)?;
        self.rotation_scores_skate3(data);
        if !locked {
            self.overall_wipeout_duration_frames = self.overall_wipeout_duration_frames.wrapping_add(1);
            if !frame.skeleton_wipeout_over {
                self.moving_wipeout_duration_frames =
                    self.moving_wipeout_duration_frames.wrapping_add(1);
            }
            self.is_skeleton_wipeout_over |= frame.skeleton_wipeout_over;
            if self.is_skeleton_wipeout_over {
                self.skeleton_wipeout_over_duration_frames =
                    self.skeleton_wipeout_over_duration_frames.wrapping_add(1);
            }
            if !self.is_in_severe_wipeout {
                self.is_in_severe_wipeout = data.severe_threshold < self.bone_score(None)?;
            }
            if self.is_in_severe_wipeout {
                self.severe_wipeout_duration_frames =
                    self.severe_wipeout_duration_frames.wrapping_add(1);
            }
        }
        let response = frame.skate3_response_strength;
        if response > 0.0 {
            self.skate3.response_sum += response;
            if self.skate3.response_max - response < 0.0 {
                self.skate3.response_max = response;
            }
            self.skate3.response_samples = self.skate3.response_samples.wrapping_add(1);
        }
        let broken = self
            .bones
            .iter()
            .filter(|bone| bone.damage_level >= 4 && bone.damage_level_last_frame < 4)
            .count();
        if broken > 0 {
            self.skate3.broken_hold = 30;
        } else if self.skate3.broken_hold > 0 {
            self.skate3.broken_hold -= 1;
        }
        out.ho_m_bail_in_progress = true;
        self.output_bail_skate3(data, out)
    }

    /// 82DAEDC8: heavy landing memory, then the pre-wipeout window's run-up tracking.
    fn update_outside_skate3(
        &mut self,
        frame: &WipeoutFrame<'_>,
        data: &skate3_hom::Data,
    ) -> Result<(), ScoreError> {
        if self.frames_since_heavy_landing_signal >= 0 {
            self.frames_since_heavy_landing_signal += 1;
            if self.frames_since_heavy_landing_signal == 3 {
                self.left_foot_heavy_landing_speed = 0.0;
                self.right_foot_heavy_landing_speed = 0.0;
                self.frames_since_heavy_landing_signal = -1;
            }
        }
        let speed = frame.heavy_landing_speed;
        match frame.heavy_landing_kind {
            0 => {
                self.frames_since_heavy_landing_signal = 0;
                self.left_foot_heavy_landing_speed = speed;
                self.right_foot_heavy_landing_speed = speed;
            }
            1 => {
                self.frames_since_heavy_landing_signal = 0;
                self.left_foot_heavy_landing_speed = speed;
                self.right_foot_heavy_landing_speed = 0.0;
            }
            2 => {
                self.frames_since_heavy_landing_signal = 0;
                self.left_foot_heavy_landing_speed = 0.0;
                self.right_foot_heavy_landing_speed = speed;
            }
            _ => {}
        }
        if !self.skate3.window {
            return Ok(());
        }
        let locked = frame.skate3_air_locked;
        if !locked {
            self.skate3.time += DT;
        }
        let was_airborne = self.is_airborne;
        self.is_airborne = airborne(frame.filtered);
        if self.is_airborne && !was_airborne {
            self.start_drop_skate3(frame.damped_centre_of_mass);
        } else if was_airborne && !self.is_airborne {
            self.air_drop_skate3(data, locked, frame)?;
            self.end_air_skate3();
        }
        if self.is_airborne {
            self.air_drop_skate3(data, locked, frame)?;
            if !self.skate3.speed_scored
                || length(frame.centre_of_mass_velocity)? > length(self.bail_start_velocity)?
            {
                self.bail_start_velocity = frame.centre_of_mass_velocity;
            }
            self.bail_speed_score = score(data.s3_curve(2820, length(self.bail_start_velocity)?));
            self.skate3.speed_scored = true;
        }
        self.bank_rotation_skate3(frame.hips_matrix, locked)?;
        self.rotation_scores_skate3(data);
        if !locked {
            self.overall_wipeout_duration_frames = self.overall_wipeout_duration_frames.wrapping_add(1);
        }
        Ok(())
    }

    /// Trackers follow the hips without scoring while held; otherwise bank at 30 degrees.
    fn bank_rotation_skate3(&mut self, hips: SkeletonRootMatrix, hold: bool) -> Result<(), ScoreError> {
        if hold {
            self.pitch_tracker.last_frame_matrix = Some(hips);
            self.yaw_tracker.last_frame_matrix = Some(hips);
            self.roll_tracker.last_frame_matrix = Some(hips);
            return Ok(());
        }
        self.banked_combined_rotation =
            self.pitch_tracker.update_with_banking(hips, 30.0)? + self.banked_combined_rotation;
        self.banked_combined_rotation =
            self.yaw_tracker.update_with_banking(hips, 30.0)? + self.banked_combined_rotation;
        self.banked_combined_rotation =
            self.roll_tracker.update_with_banking(hips, 30.0)? + self.banked_combined_rotation;
        Ok(())
    }

    /// 82DAD640: the window scores rotation on its own curve (+2260), the wipeout on +2900.
    fn rotation_scores_skate3(&mut self, data: &skate3_hom::Data) {
        let yaw = (self.yaw_tracker.accumulated_angle_radians * DEGREES).abs();
        let pitch = (self.pitch_tracker.accumulated_angle_radians * DEGREES).abs();
        let roll = (self.roll_tracker.accumulated_angle_radians * DEGREES).abs();
        self.current_combined_rotation = ((yaw + pitch) + roll) + self.banked_combined_rotation;
        let curve = if self.skate3.window { 2260 } else { 2900 };
        self.current_rotation_score = score(data.s3_curve(curve, self.current_combined_rotation));
        if self.current_combined_rotation - self.max_combined_rotation >= 0.0 {
            self.max_combined_rotation = self.current_combined_rotation;
        }
        self.max_rotation_score = self.max_rotation_score.max(self.current_rotation_score);
    }

    fn start_drop_skate3(&mut self, position: SkeletonRootVector) {
        self.air_start_position = position;
        self.air_apex_position = position;
        self.current_air_drop.reset();
        self.skate3.last_rotation = self.banked_combined_rotation;
        self.current_body_tweak = -1;
        self.body_tweak_valid = false;
        self.just_performed_body_tweak = false;
        self.body_tweak_duration_frames = 0;
    }

    /// 82DAECB8: apex, airtime and drop height scores, then the body tweaks.
    fn air_drop_skate3(
        &mut self,
        data: &skate3_hom::Data,
        locked: bool,
        frame: &WipeoutFrame<'_>,
    ) -> Result<(), ScoreError> {
        let position = frame.damped_centre_of_mass;
        if position.xyz[1] > self.air_apex_position.xyz[1] {
            self.air_apex_position = position;
        }
        if !locked {
            self.current_air_drop.duration_in_frames =
                self.current_air_drop.duration_in_frames.wrapping_add(1);
        }
        self.current_air_drop.drop_height = self.air_apex_position.xyz[1] - position.xyz[1];
        self.current_air_drop.drop_time_score =
            score(data.s3_curve(2500, self.current_air_drop.duration_in_frames as f32 * DT));
        self.current_air_drop.drop_height_score =
            score(data.s3_curve(2580, self.current_air_drop.drop_height));
        self.update_body_tweaks_skate3(data, locked, frame.gesture)?;
        self.skate3.previous_tweak = self.current_body_tweak;
        Ok(())
    }

    /// EndAir 82DAEBC0.
    fn end_air_skate3(&mut self) {
        let current = &self.current_air_drop;
        let total = &mut self.accumulated_air_drop;
        total.duration_in_frames = total.duration_in_frames.wrapping_add(current.duration_in_frames);
        total.drop_time_score = total.drop_time_score.wrapping_add(current.drop_time_score);
        total.drop_height_score = total.drop_height_score.wrapping_add(current.drop_height_score);
        total.drop_height = current.drop_height + total.drop_height;
        let max = &mut self.max_air_drop;
        max.duration_in_frames = max.duration_in_frames.max(current.duration_in_frames);
        max.drop_time_score = max.drop_time_score.max(current.drop_time_score);
        max.drop_height_score = max.drop_height_score.max(current.drop_height_score);
        if max.drop_height - current.drop_height < 0.0 {
            max.drop_height = current.drop_height;
        }
        self.current_air_drop.reset();
        self.body_tweak_valid = false;
        self.just_performed_body_tweak = false;
        self.body_tweak_duration_frames = 0;
        self.skate3.previous_tweak = self.current_body_tweak;
        self.current_body_tweak = -1;
        self.skate3.last_rotation = self.banked_combined_rotation;
    }

    /// UpdateBodyTweaks 82DAD728: a held tweak also gathers its time and rotation.
    fn update_body_tweaks_skate3(
        &mut self,
        data: &skate3_hom::Data,
        locked: bool,
        gesture: [f32; 2],
    ) -> Result<(), ScoreError> {
        let previous = self.current_body_tweak;
        self.body_tweak_valid = false;
        self.just_performed_body_tweak = false;
        self.current_body_tweak_score = 0;
        self.current_body_tweak = -1;
        let squared = gesture[0] * gesture[0] + gesture[1] * gesture[1];
        let mut matched = false;
        if squared > data.tweak_threshold * data.tweak_threshold {
            let limit = f32::from_bits(0x4049_0fdb) / data.tweaks.len() as f32;
            for tweak in &data.tweaks {
                let angle = try_unsigned_angle_2d(tweak.direction, gesture, &mut HostReckoningNumeric)
                    .map_err(ScoreError::Numeric)?;
                if angle >= limit {
                    continue;
                }
                let rotation = self.current_combined_rotation;
                let last = self.skate3.last_rotation;
                self.skate3.last_rotation = rotation;
                self.current_body_tweak = tweak.id;
                if tweak.id == previous {
                    if locked {
                        self.body_tweak_valid = true;
                    } else {
                        self.body_tweak_duration_frames = self.body_tweak_duration_frames.wrapping_add(1);
                        let slot = usize::try_from(tweak.id)
                            .ok()
                            .filter(|slot| *slot < 4)
                            .ok_or(ScoreError::Invalid("Skate 3 body tweak index"))?;
                        self.skate3.tweak_rotation[slot] += rotation - last;
                        self.skate3.tweak_time[slot] += DT;
                        let held = self.body_tweak_duration_frames as f32 * DT;
                        if held - self.skate3.tweak_max_time[slot] >= 0.0 {
                            self.skate3.tweak_max_time[slot] = held;
                        }
                    }
                } else {
                    self.body_tweak_duration_frames = 0;
                    if locked {
                        self.just_performed_body_tweak = true;
                    }
                }
                if self.body_tweak_duration_frames == tweak.frames {
                    self.num_body_tweaks_performed = self.num_body_tweaks_performed.wrapping_add(1);
                    self.body_tweak_score = self.body_tweak_score.wrapping_add(tweak.points);
                    self.current_body_tweak_score = tweak.points;
                    self.just_performed_body_tweak = true;
                }
                if self.body_tweak_duration_frames >= tweak.frames {
                    self.body_tweak_valid = true;
                }
                matched = true;
                break;
            }
        }
        if !matched {
            self.body_tweak_duration_frames = 0;
        }
        Ok(())
    }

    /// 82DAD528: entity hits in tweak 2, tweak 1's hold, and tweaks 3 and 0 in whole spins.
    fn run_up_score_skate3(&self, data: &skate3_hom::Data) -> u32 {
        let spins = |angle: f32| ((angle + 90.0) * f32::from_bits(0x3b36_0b61)).floor();
        let total = data.s3_curve(2340, self.skate3.tweak_entity_score as f32)
            + data.s3_curve(2100, self.skate3.tweak_time[1])
            + data.s3_curve(2740, spins(self.skate3.tweak_rotation[3]))
            + data.s3_curve(2660, spins(self.skate3.tweak_rotation[0]));
        score(total).wrapping_add(self.body_tweak_score)
    }

    /// OutputBailData 82DADC48 into the shared bail record; bonus 82DADA30.
    fn output_bail_skate3(&self, data: &skate3_hom::Data, out: &mut Scoring2) -> Result<(), ScoreError> {
        let pedestrian = self.pedestrian.as_ref().ok_or(ScoreError::Missing("pedestrian scorer"))?;
        let vehicle = self.vehicle.as_ref().ok_or(ScoreError::Missing("vehicle scorer"))?;
        let dmo = &self.dmo.published_value().ok_or(ScoreError::Missing("DMO scorer"))?.base;
        let duration_score = score(data.s3_curve(2980, self.moving_wipeout_duration_frames as f32 * DT));
        let bonus = [
            self.run_up_score_skate3(data),
            self.max_rotation_score,
            self.bail_speed_score,
            self.current_air_drop.drop_height_score,
            self.current_air_drop.drop_time_score,
            self.accumulated_air_drop.drop_height_score,
            self.accumulated_air_drop.drop_time_score,
            score(data.s3_curve(2180, self.skate3.response_sum)),
            duration_score,
            dmo.score,
            vehicle.score,
            pedestrian.score,
        ]
        .into_iter()
        .fold(0u32, u32::wrapping_add);
        let bone_score = self.bone_score(None)?;
        let bail = &mut out.ho_m_bail_data;
        bail.score = bone_score.wrapping_add(bonus);
        bail.bail_duration_score = duration_score;
        bail.bone_score = bone_score;
        bail.bonus_score = bonus;
        for (i, level_score) in bail.bone_score_for_damage_level.iter_mut().enumerate() {
            *level_score = self.bone_score(Some(i as i32))?;
        }
        bail.bail_speed_score = self.bail_speed_score;
        bail.rotation_score = self.max_rotation_score;
        bail.body_tweak_score = self.body_tweak_score;
        bail.current_body_tweak_score = self.current_body_tweak_score;
        bail.current_body_tweak = if self.body_tweak_valid { self.current_body_tweak } else { -1 };
        bail.just_performed_body_tweak = self.just_performed_body_tweak;
        bail.num_body_tweaks_performed = self.num_body_tweaks_performed;
        bail.current_air_drop = self.current_air_drop.clone();
        bail.max_air_drop = self.max_air_drop.clone();
        bail.accumulated_air_drop = self.accumulated_air_drop.clone();
        bail.overall_wipeout_duration_in_seconds = self.overall_wipeout_duration_frames as f32 * DT;
        bail.moving_wipeout_duration_in_seconds = self.moving_wipeout_duration_frames as f32 * DT;
        bail.severe_wipeout_duration_in_seconds = self.severe_wipeout_duration_frames as f32 * DT;
        bail.skeleton_wipeout_over_duration_in_seconds =
            self.skeleton_wipeout_over_duration_frames as f32 * DT;
        bail.bail_speed = length(self.bail_start_velocity)?;
        bail.rotation = self.max_combined_rotation;
        for (bone, dst) in self.bones.iter().zip(&mut bail.bones.bones) {
            dst.damage_level = bone.damage_level_last_frame;
            dst.pos = bone.position;
        }
        pedestrian.fill_entity_output(&mut bail.entity_sets[0])?;
        vehicle.fill_entity_output(&mut bail.entity_sets[1])?;
        dmo.fill_entity_output(&mut bail.entity_sets[2])?;
        Ok(())
    }
}
