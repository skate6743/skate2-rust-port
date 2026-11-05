//! Stateless score and bump lifecycles using the same MotionHost publication.
use super::*;
impl MotionHost {
    pub(super) fn execute_score_or_bump(&mut self, operation: &MotionOperation, phase: u8) -> Result<bool, String> {
        match operation {
            MotionOperation::ScoringTrick(operation) => {
                operation.execute(&mut self.score_packet.trick_names, &mut self.score_packet.flags, phase);
            }
            MotionOperation::SetBumpCoefficients { x, y } => {
                if phase == 0 {
                    let values = self.bump_settings.coefficients(
                        self.bump_acceleration.ok_or("SetBumpCoefficients requires completed acceleration")?,
                        self.animation.skater_animation_flags.ok_or("SetBumpCoefficients requires actual stance")? & 0x40000000 != 0,
                    );
                    for (name, value) in [*x, *y].into_iter().zip(values) {
                        self.animation.set_attribute(SettableAttribute { name, value, normalized: false, sequence_id: -1 });
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}