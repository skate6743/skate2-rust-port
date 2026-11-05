//! Original airborne MotionGraph lifecycles on the production host.
use super::*;
impl MotionHost {
    pub(super) fn execute_air(
        &mut self,
        behavior: BehaviorId,
        operation: MotionOperation,
        frame: &Frame,
        phase: u8,
    ) -> Result<(), String> {
        let instance = self
            .instances
            .get_mut(behavior)
            .ok_or("Unallocated airborne MotionGraph behavior")?;
        match (operation, instance) {
            (MotionOperation::ClearTrickAttr, _) => {
                //End82BB5DB0 -> IAnimatable112 (8231E050+112) ->82531278:
                //erase the construction-map key. Begin/Update are82B61BB8.
                if phase == 2 {
                    self.animation
                        .construction_values
                        .retain(|(name, _)| *name != encode(b"Trick"));
                }
            }
            (MotionOperation::AirLeg(operation), Instance::AirLeg(state)) => {
                if phase == 1 {
                    let physical = self
                        .air_leg_physical
                        .ok_or("Air leg extension requires completed physical output")?;
                    let initial = if state.needs_initial_attribute() {
                        self.animation.last_attribute(operation.height)?
                    } else {
                        None
                    };
                    let prepare = if state.needs_prelanding_query(physical) {
                        let query = self
                            .prelanding_physical
                            .ok_or("Air leg extension requires actual prelanding output")?
                            .near_landing(&self.spin);
                        let full = if query {
                            self.animation.last_attribute(encode(b"FullExtension"))?
                        } else {
                            None
                        };
                        skate3_core::animation::air_leg_extension::prepare_to_land(
                            query,
                            full,
                            physical,
                            &self.air_leg,
                        )
                    } else {
                        false
                    };
                    let [height, extension] = state.update(
                        operation.bone,
                        physical,
                        &self.air_leg,
                        frame.dt,
                        initial,
                        prepare,
                    );
                    for (name, value) in
                        [(operation.height, height), (encode(b"extend"), extension)]
                    {
                        self.animation.set_attribute(SettableAttribute {
                            name,
                            value,
                            normalized: false,
                            sequence_id: -1,
                        });
                    }
                }
            }
            (MotionOperation::BodySpin, Instance::BodySpin(state)) => {
                if phase == 1 {
                    state.update(
                        &mut self.animation,
                        self.condition_inputs
                            .physical_state
                            .as_ref()
                            .ok_or("BodySpin requires physicalcategory")?
                            .category,
                        self.gameplay_conditions
                            .ok_or("BodySpin requires physicalstate")?
                            .state,
                        self.flags.doing_trick,
                        self.hand_services.busy_hands,
                        self.playback_context
                            .is_mirrored
                            .ok_or("BodySpin requires animstance")?,
                        self.prelanding_physical,
                        &self.spin,
                    )?;
                }
            }
            _ => return Err("Airborne MotionGraph operation/instance mismatch".into()),
        }
        Ok(())
    }
}
