//! S3 82D6D020/CC50/E8A0. Incoming scoring/requery are comparison only.
use super::{Candidate, Context, DT, Selector, ledge, math::*};
use crate::air::trajectory::{Prediction, QueryResult};
impl Selector {
    ///Populate native landing observations before candidate0's ledge stage.
    pub fn observe_launch(&mut self, results: &[QueryResult]) -> Result<(), &'static str> {
        if !self.sampling.pending_8492 || results.len() != self.candidates.len() {
            return Err("BipedAir launch completion does not match pending batch");
        }
        for ((candidate, prediction), &result) in self
            .candidates
            .iter_mut()
            .zip(&mut self.predictions)
            .zip(results)
        {
            prediction.result = result;
            if result.valid() {
                candidate.contact_position_96 = result.contact_position;
                candidate.normal_64 = result.suggested_normal();
                prediction.result.contact_normal = candidate.normal_64;
                candidate.contact_velocity_80 = candidate
                    .trajectory
                    .velocity_at(result.contact_frame as f32 * DT);
                candidate.landing_frame_116 = result.contact_frame;
                candidate.valid_120 = true;
            } else {
                candidate.normal_64 = UP;
                candidate.contact_velocity_80 = [0.; 4];
                candidate.landing_frame_116 = 120;
                candidate.valid_120 = false;
            }
        }
        Ok(())
    }
    ///Native scoring uses the pre-ledge normal/displacement even if candidate0
    ///is subsequently adjusted. Caller passes the original observations.
    pub fn select_launch(
        &mut self,
        context: Context,
        original_first: Candidate,
        ledge: Option<ledge::Adjustment>,
    ) -> Result<usize, &'static str> {
        if !self.sampling.pending_8492 || self.candidates.is_empty() {
            return Err("BipedAir select without pending launch");
        }
        if let Some(adjustment) = ledge {
            adjustment.apply(&mut self.candidates[0]);
            self.ledge_normal_8320 = self.candidates[0].normal_64;
            self.ledge_selected_8496 = true;
        }
        for i in 0..self.candidates.len() {
            let c = self.candidates[i];
            self.scores[i] = 0.;
            if !c.valid_120 {
                continue;
            }
            let original = if i == 0 { original_first } else { c };
            let displacement = sub(original.contact_position_96, original.trajectory.position);
            let normal = original.normal_64;
            let time = c.landing_frame_116 as f32 * DT;
            let mut ledge_bonus = if i == 0 && self.ledge_selected_8496 {
                1.
            } else {
                0.
            };
            let mut time_penalty = 0.;
            if time <= 0.5 {
                ledge_bonus = 0.;
                time_penalty = -1. - (0.5 - time) * 2.;
            }
            let center_bonus = if i == 0 { 1. } else { 0. };
            if self.predictions[i].result.surface & 0xf80 == 0x300 {
                self.predictions[i].result.contact_normal = context.up_544;
            }
            let mut normal_penalty = (2.2222223 * ((normal[1] - 0.45) * 5.)).min(0.);
            if dot(normal, self.launch.forward_64) > 0. {
                normal_penalty = 0.;
            }
            let mut forward = dot(self.launch.forward_64, displacement).max(1.);
            let mut height = displacement[1].max(0.);
            if i >= self.launch.kind_108 as usize {
                if displacement[1] < 0.4 {
                    height -= 10000.;
                } else {
                    height *= 8.;
                    forward = dot(self.launch.secondary_velocity_16, displacement);
                }
            }
            self.scores[i] = ((((0. + forward) + height) + normal_penalty) + center_bonus)
                + time_penalty
                + ledge_bonus;
        }
        let index = super::recovered::selection::selected_index(&self.scores)
            .ok_or("BipedAir empty candidate batch")?;
        self.selected_index = Some(index);
        self.selected_candidate = self.candidates[index];
        if self.sampling.commit(
            self.selected_candidate,
            &mut self.predictions[index],
            self.offset_8272,
        ) {
            self.just_changed_8497 = true;
        }
        Ok(index)
    }
    ///82D6E8A0. A miss clears pending only. First hit seeds comparison state;
    ///subsequent moved hits update landing observations, not trajectory8144.
    pub fn complete_requery(&mut self, prediction: Prediction) -> Result<(), &'static str> {
        if !self.requery_pending_8499 {
            return Err("BipedAir requery completion without submission");
        }
        self.requery_pending_8499 = false;
        if let Some(slot) = self.predictions.first_mut() {
            *slot = prediction;
        }
        if !prediction.result.valid() {
            return Ok(());
        }
        let result = prediction.result;
        if self.requery_count_8488 <= 0 {
            self.requery_position_8352 = result.contact_position;
            self.requery_normal_8368 = result.suggested_normal();
        } else {
            let delta = sub(self.requery_position_8352, result.contact_position);
            if dot(delta, delta) > 0.0001 {
                self.requery_position_8352 = result.contact_position;
                self.requery_normal_8368 = result.suggested_normal();
                let s = &mut self.sampling.selection;
                s.landing_frame_8480 = result.contact_frame;
                s.scalar_8392 = result.contact_frame as f32 * DT;
                s.velocity_6160 = prediction
                    .request
                    .trajectory
                    .velocity_at(result.contact_frame as f32 * DT);
                s.position_6176 = result.contact_position;
                s.normal_6144 = result.suggested_normal();
            }
        }
        self.requery_count_8488 = self.requery_count_8488.wrapping_add(1);
        Ok(())
    }
}
