//! PhysOutModule_Hud827c79b8 reads completed Scoring2/Animation output.
use super::{Conditioners, PhysicalPlayerPhysOut, scorable::TrickSettings,
    scoring_attributes::{ScoreError, scalar, LINE_MAX}, scoring_catalog::info,
    scoring_database::TrickDatabase};
use crate::ui::trick_backend::{PhysicsFrame, TrickName};
use crate::ui::hom_scoring::{HomHudData, HomPhysicsFrame};

// UpdateHoM827c73b0 copies the newly appended collision tail, not the complete
// accumulated set. The native output has eight slots; the HUD has ten.
fn collision_scores(set: &super::super::scoring::EntitySet) -> Result<[f32; 10], ScoreError> {
    let start = set.count.checked_sub(set.num_new_collisions_this_frame)
        .ok_or(ScoreError::Invalid("HUD HoM collision count"))?;
    let entries = set.collided_entities.get(start as usize..set.count as usize)
        .ok_or(ScoreError::Invalid("HUD HoM collision range"))?;
    let mut scores = [0.; 10];
    for (dst, entry) in scores.iter_mut().zip(entries) { *dst = entry.score as f32; }
    Ok(scores)
}

fn nollie(id: i32) -> bool {
    info(id).is_some_and(|i| matches!(i.trick_type, 1 | 2)
        && matches!(i.name.as_bytes().first(), Some(b'n' | b'N')))
}
fn name(db: &TrickDatabase, settings: &TrickSettings) -> Result<TrickName, ScoreError> {
    if !settings.is_valid { return Ok(TrickName { index: -1, ..Default::default() }); }
    let mut variant = TrickSettings::new();
    if settings.fakie_variant_id != -1 {
        db.lookup_id(settings.fakie_variant_id, &mut variant)?;
    }
    Ok(TrickName {
        index: settings.trick_index as i32, attrib_key: settings.trick_attrib_key, valid: true,
        label: settings.trick_name_hal.clone(),
        // RefreshTrickDisplay82619580: all aerial types and grabs except coffin.
        accepts_spin: matches!(settings.trick_type, 1 | 2 | 3 | 7 | 10 | 12 | 13)
            || (settings.trick_type == 4
                && !info(settings.scorable_id).is_some_and(|i| i.name == "coffin")),
        nollie: nollie(settings.scorable_id),
        fakie_variant: variant.trick_name_hal.map(|label| (label, nollie(settings.fakie_variant_id))),
    })
}
impl Conditioners {
    /// The HUD's name of the trick with a Vault key, as `try_scoring_hud_frame` names a begun
    /// trick: a followed online skater's trick, sent as its key (`online::hud_record`). `None` for
    /// a key this database does not hold.
    pub(crate) fn try_hud_trick_name(&self, key: u64) -> Result<Option<TrickName>, ScoreError> {
        let retained = &self.scoring.published_value().ok_or(ScoreError::Missing("HUD scoring owner"))?.payload;
        let db = retained.database.as_ref().ok_or(ScoreError::Missing("HUD trick database"))?;
        let mut settings = TrickSettings::new();
        db.lookup_attrib_key(key, &mut settings)?;
        if !settings.is_valid {
            return Ok(None);
        }
        name(db, &settings).map(Some)
    }

    pub(crate) fn try_scoring_hud_frame(&self, output: &PhysicalPlayerPhysOut) -> Result<PhysicsFrame, ScoreError> {
        output.check_current_guard().map_err(ScoreError::Access)?;
        if !self.matches_output(output) { return Err(ScoreError::Invalid("HUD foreign PhysOut")); }
        let retained = &self.scoring.published_value().ok_or(ScoreError::Missing("HUD scoring owner"))?.payload;
        let db = retained.database.as_ref().ok_or(ScoreError::Missing("HUD trick database"))?;
        let module = retained.module.published_value().ok_or(ScoreError::Missing("HUD score module"))?;
        let out = output.concrete.scoring.as_deref().ok_or(ScoreError::Missing("HUD Scoring2"))?;
        let anim = output.concrete.animation.as_deref().ok_or(ScoreError::Missing("HUD Animation"))?;
        let bail = &out.ho_m_bail_data;
        // All scores already include the native WipeoutScorer calculations.
        // Despite the HUD field names, air scores are accumulated while the
        // displayed measurements use the maximum completed drop.
        let hom = HomPhysicsFrame {
            just_wiped_out: out.just_wiped_out,
            // kIsHoMHUDEnabled initializer82f6b2d0 defaults true.
            // The selected session currently retains that original debug default.
            effect_enabled: out.ho_m_is_effect_enabled,
            bail_in_progress: out.ho_m_bail_in_progress,
            bail_completed: out.ho_m_bail_completed,
            just_reactivated: out.just_reactivated_scoring,
            data: HomHudData {
                score: bail.score,
                wipeout_speed_score: bail.bail_speed_score as f32,
                max_air_drop_score: bail.accumulated_air_drop.drop_height_score as f32,
                max_air_time_score: bail.accumulated_air_drop.drop_time_score as f32,
                wipeout_duration_score: bail.bail_duration_score as f32,
                rotation_score: bail.rotation_score as f32,
                wipeout_speed: bail.bail_speed,
                max_air_drop: bail.max_air_drop.drop_height,
                max_air_time: bail.max_air_drop.duration_in_frames as f32 * f32::from_bits(0x3c88_8889),
                wipeout_duration: bail.moving_wipeout_duration_in_seconds,
                rotation: bail.rotation,
                new_car_collisions: bail.entity_sets[1].num_new_collisions_this_frame,
                new_ped_collisions: bail.entity_sets[0].num_new_collisions_this_frame,
                new_dmo_collisions: bail.entity_sets[2].num_new_collisions_this_frame,
                new_body_tweak: bail.just_performed_body_tweak,
                body_tweak_score: bail.current_body_tweak_score as f32,
                car_collision_score: collision_scores(&bail.entity_sets[1])?,
                ped_collision_score: collision_scores(&bail.entity_sets[0])?,
                dmo_collision_score: collision_scores(&bail.entity_sets[2])?,
                // Native UpdateHoM always writes zero skater collisions and
                // never writes mMult. Both retain the constructor zero here.
                dirty: true,
                ..Default::default()
            },
        };
        let begun = if out.tricks_begun.num_entries > 0 {
            Some(name(db, &out.tricks_begun.elements[0].trick_scoring_settings)?)
        } else { None };
        let converted = if begun.is_none() && out.tricks_converted_to.num_entries > 0 {
            Some(name(db, &out.tricks_converted_to.elements[0].trick_scoring_settings)?)
        } else { None };
        let mut context_entries = Vec::new();
        // BigTrickUpdate827c6b68 iterates all 15 original thresholds.
        for (index, threshold) in out.big_trick_achieved_data.score_thresholds.iter().enumerate() {
            if let Some(achievement) = threshold.was_achieved.iter().rposition(|&v| v) {
                context_entries.push(crate::ui::trick_context::Entry::Big {
                    index: index as i32, achievement: achievement as i32, measurement: threshold.value,
                });
            }
        }
        // UpdateAirContextData827c6c40: only the Over UID bank is displayed;
        // remove takeoff/landing contexts with the existing native UID helper.
        if out.is_in_state_air || out.air_finished {
            let air = if out.air_finished { &out.air_finished_data } else { &out.current_air_data };
            let mut over = air.accumulated_context_data.clone();
            over.remove(&air.initial_context_data)?;
            over.remove(&air.end_context_data)?;
            for group in &over.context_sensitive_data {
                let bank = &group.uid_data[0];
                let entries = bank.uids.get(..bank.count as usize).ok_or(ScoreError::Invalid("HUD context count"))?;
                for entry in entries {
                    context_entries.push(crate::ui::trick_context::Entry::Entity {
                        entity_type: entry.entity_type, is_entity: true, count: 1, context_type: 0,
                    });
                }
            }
        }
        Ok(PhysicsFrame {
            backend_commands:Vec::new(),
            hom,
            context_entries, simulation_rate: 0., // selected world supplies its actual update rate
            begun, converted, ended_count: out.tricks_ended.num_entries as u32,
            sequence_begun: out.sequence_begun, sequence_completed: out.sequence_completed,
            finished_sequence_score: out.finished_sequence_score,
            finished_tricks: out.finished_sequence_trick_count,
            finished_taps: out.finished_sequence_tapped_trick_count,
            interrupted: out.wipeout_interrupted_trick, just_wiped_out: out.just_wiped_out,
            reactivated: out.just_reactivated_scoring,
            line_seconds: out.line_timer_seconds, line_points: out.line_timer_points,
            line_max_points: scalar(&module.general, LINE_MAX)? as i32,
            line_score: out.line_score as u32, multiplier: out.momentum_meter_multiplier,
            hud_score: out.pre_multiplied_hud_sequence_score as u32,
            momentum_points: out.momentum_meter_points,
            aerial: out.is_in_state_air || out.is_hippy_jumping,
            spins: if out.is_hippy_jumping { out.hippy_jump180_count } else { out.current_air_data.player_spin180_count },
            flips: if out.is_hippy_jumping { 0 } else { out.current_air_data.player_num_flips },
            switched: anim.is_riding_switch, fakie: anim.is_riding_fakie,
            manual_length: out.manual_length, grind_distance: out.current_grind_info.length,
            powerslide_length: out.powerslide_length,
            air_distance: out.current_air_data.horizontal_distance,
            air_height: out.current_air_data.height_to_peak,
            air_step: out.current_air_data.total_height_gain,
            air_finished: out.air_finished, just_landed: out.air_finished_data.just_landed,
            sketchy: out.air_finished_data.landing_was_sketchy,
            landed_height: out.air_finished_data.height_to_peak,
            landed_drop: -out.air_finished_data.total_height_gain,
        })
    }
}
