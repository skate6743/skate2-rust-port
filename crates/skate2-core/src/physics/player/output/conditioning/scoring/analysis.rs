//! Nursary::ProcessAnimData82dfc358 / StartScorable82df74a8.
use super::{
    super::{scoring::Scoring2, trick::AnimOutTrickData},
    collector_base::CollectorBase,
    scorable::{Scorable, TrickSettings},
    scoring_attributes::ScoreError,
    scoring_database::TrickDatabase,
};
impl CollectorBase {
    /// Nursary::CheckForHippyFlip82deb658: the existing ground probes straddle
    /// an intervening hit. No new collision query or object classification.
    pub(super) fn check_hippy_flip(
        &self,
        ground: &super::super::simple::GroundRemainder,
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        use crate::reckoning_numeric::{HostReckoningNumeric, ReckoningNumeric};
        if !ground.below_hips_ground_data_is_valid {
            return Ok(());
        }
        let deck = ground
            .below_deck_ground_data_start_pos
            .zip(ground.below_hips_ground_data_position, |a, b| a - b);
        let hips = ground
            .below_hips_ground_data_start_pos
            .zip(ground.below_hips_ground_data_position, |a, b| a - b);
        let mut numeric = HostReckoningNumeric;
        if numeric
            .dot_vector(deck, deck)
            .map_err(ScoreError::Numeric)?
            > 0.01
            && numeric
                .dot_vector(hips, hips)
                .map_err(ScoreError::Numeric)?
                > 0.01
            && 0.0
                > numeric
                    .dot_vector(deck, hips)
                    .map_err(ScoreError::Numeric)?
        {
            out.is_hippy_flipping = true;
        }
        Ok(())
    }

    pub(super) fn process_animation(
        &mut self,
        db: &TrickDatabase,
        animation: &AnimOutTrickData,
        enabled: &[bool; 14],
        out: &mut Scoring2,
    ) -> Result<(), ScoreError> {
        self.reset_analysis();
        self.animation_trick = animation.clone();
        if animation.bits & 0x0300_0000 != 0 {
            self.raw_scorable_id = db.animation_id(&animation.name);
        }
        self.filtered_scorable_id = self.raw_scorable_id;
        if self.raw_scorable_id >= 0 {
            let mut settings = TrickSettings::new();
            db.lookup_id(self.raw_scorable_id, &mut settings)?;
            if !settings.is_valid || !enabled[settings.trick_type as usize] {
                self.filtered_scorable_id = -1;
            }
        }
        if self.animation_trick.bits & 0x0200_0000 != 0 && (67..=69).contains(&self.raw_scorable_id)
        {
            self.filtered_scorable_id = -1;
            self.animation_trick.bits &= !0x0200_0000;
        }
        out.raw_scorable_id = self.raw_scorable_id;
        #[cfg(feature = "skate3-runtime")]
        if db.use_skate3 {
            out.skate3_scorable_audio = super::skate3_catalog::audio(self.raw_scorable_id);
            trace_scorable(self.raw_scorable_id, out.skate3_scorable_audio);
        }
        Ok(())
    }
    pub(super) fn start_scorable(
        &self,
        db: &TrickDatabase,
        name: &str,
        id: i32,
        point_scalar: f32,
        scorable: &mut Scorable,
        tick: u32,
        extra_min_time: f32,
        switched: bool,
        fakie: bool,
    ) -> Result<(), ScoreError> {
        let penalty = self
            .score_holder
            .as_ref()
            .ok_or(ScoreError::Missing("collector score holder"))?
            .lock()
            .map_err(|_| ScoreError::Poisoned)?
            .penalty(id)?;
        scorable.init(
            db,
            &self.general,
            name,
            penalty * point_scalar,
            switched,
            fakie,
            tick,
            extra_min_time,
        )
    }
}

/// `SKATE2_TRACE_SKATE3_DARKCATCH`: each change of the raw scorable with its Skate 3 audio record.
#[cfg(feature = "skate3-runtime")]
fn trace_scorable(id: i32, audio: Option<(i32, i32, bool)>) {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    static LAST: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(i32::MIN);
    if !*TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE3_DARKCATCH").is_some())
        || LAST.swap(id, std::sync::atomic::Ordering::Relaxed) == id {
        return;
    }
    let name = super::scoring_catalog::info(id).map_or("none", |entry| entry.name);
    eprintln!("skate 3 darkcatch audio: raw scorable {id} {name} audio {audio:?}");
}
