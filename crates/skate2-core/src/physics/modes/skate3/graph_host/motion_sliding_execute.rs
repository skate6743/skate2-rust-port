//! Production lifecycle for the authored slide family; no detached state copies.
use super::*;
use crate::skate3_native::graph_host::motion_sliding::{self, Operation};

impl MotionHost {
    pub(super) fn execute_slide(
        &mut self,
        behavior: BehaviorId,
        operation: Operation,
        frame: &Frame,
        phase: u8,
    ) -> Result<(), String> {
        match operation {
            Operation::Update if phase == 1 => {
                let category = self
                    .condition_inputs
                    .physical_state
                    .as_ref()
                    .ok_or("PowerSliding requires actual filtered physical category")?
                    .category;
                // Native returns before speed/direction reads outside category1.
                let (speed, direction) = if category == 1 {
                    let speed = self
                        .fakie_physical
                        .ok_or("PowerSliding requires actual Motion164 speed")?
                        .ground_projected_speed;
                    (speed, self.slide_direction()?)
                } else {
                    (0.0, 0.0)
                };
                let Instance::Sliding(state) = &mut self.instances[behavior] else {
                    return Err("PowerSliding has an invalid instance".into());
                };
                state.update(
                    &self.animation,
                    category,
                    speed,
                    direction,
                    frame.dt,
                    &self.sliding,
                    &mut self.slide_latch,
                );
            }
            Operation::Create { right } => match phase {
                0 => {
                    //ISkaterAnim virtual12=82B970D8 reads the live bit29.
                    let fakie = self
                        .animation
                        .skater_animation_flags
                        .ok_or("CreateSlide requires actual animation stance flags")?
                        & 0x2000_0000
                        != 0;
                    self.slide_latch.begin_slide(fakie);
                }
                1 => {
                    let values = motion_sliding::create(
                        &self.slide_latch,
                        right,
                        self.animation.motion_intent("RightSlide"),
                        self.animation.motion_intent("LeftSlide"),
                        &self.sliding,
                    );
                    self.animation.emit_packet(encode(b"slide"), values[0]);
                    self.animation.emit_packet(encode(b"turn"), values[1]);
                }
                _ => {}
            },
            Operation::ManualAttribute if phase == 1 => {
                //82BB34D8 uses Manual830BE900; only a negative present value emits.
                if let Some(value) = self.animation.motion_intent("Manual")
                    && value < 0.0
                {
                    self.animation.emit_packet(encode(b"balance"), value);
                }
            }
            Operation::Deceleration => {
                let Instance::SlideDeceleration(previous) = &mut self.instances[behavior] else {
                    return Err("PowerSlideDecel has an invalid instance".into());
                };
                if phase == 0 {
                    //Begin82BB3198; same zero in native instance constructor.
                    *previous = 0.0;
                } else if phase == 1 {
                    let velocity = self
                        .fakie_physical
                        .ok_or("PowerSlideDecel requires actual Motion80 velocity")?
                        .deck_velocity;
                    let ground = self
                        .native_physical
                        .ok_or("PowerSlideDecel requires actual Reckoning Ground frame")?
                        .board_reckoning;
                    let value =
                        motion_sliding::deceleration(previous, velocity, ground, &self.sliding);
                    self.animation.set_attribute(SettableAttribute {
                        name: encode(b"decel"),
                        value,
                        normalized: false,
                        sequence_id: -1,
                    });
                }
            }
            Operation::Spin if phase == 0 => {
                let value = motion_sliding::spin(self.slide_direction()?, &self.sliding);
                self.animation.set_attribute(SettableAttribute {
                    name: encode(b"slidespin"),
                    value,
                    normalized: false,
                    sequence_id: -1,
                });
            }
            Operation::Candidate if phase != 1 => {
                self.slide_latch.set_candidate_enabled(phase == 0);
            }
            Operation::IsPowerSliding if phase != 1 => {
                //Begin82BACD20/End82BACD70 -> specific setter8258F910.
                self.is_power_sliding = phase == 0;
            }
            _ => {}
        }
        Ok(())
    }

    fn slide_direction(&self) -> Result<f32, String> {
        let velocity = self
            .fakie_physical
            .ok_or("Slide direction requires actual Motion80 velocity")?
            .deck_velocity;
        let flipped = self
            .physical
            .and_then(|p| p.foot_frame)
            .ok_or("Slide direction requires actual Motion273 orientation flag")?
            .skateboard_flipped;
        let z = self
            .native_physical
            .ok_or("Slide direction requires actual Reckoning Ground Z")?
            .board_reckoning_z;
        Ok(motion_sliding::direction(velocity, z, flipped))
    }
}
