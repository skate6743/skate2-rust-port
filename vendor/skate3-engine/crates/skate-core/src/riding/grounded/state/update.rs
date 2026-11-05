//! `PhysState_PhysicsGround::Update` (`82D37C88`) call composition.

use crate::riding::pumping::{
    controller::{self, PumpingGeometry, PumpingSample},
    settings::{PumpingMode, PumpingSettings},
    state::PumpingState,
};

use super::{
    contact_state::{self, GroundContactServices},
    data::{GroundContactHistory, GroundUpdateInput, PhysicsGroundState},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroundUpdateStage {
    SetElapsedSkeletonFlag,
    UpdateContactState,
    LoadPumpingFrame,
    UpdatePumping,
    UpdateSkateboard,
    PredictFutureDeck,
    SetCollisionSkeletonFlag,
    SetGroundSkeletonFlag,
    UpdateSpecialState,
    UpdateTrajectory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundUpdateError<E> {
    pub stage: GroundUpdateStage,
    pub source: E,
}

/// Required engine-facing operations in the exact `82D37C88` order. This
/// trait has no default implementation because each call mutates stock-owned
/// objects that are outside `skate-core`.
pub trait GroundUpdateServices:
    PumpingGeometry<Error = Self::UpdateError>
    + GroundContactServices<Error = Self::UpdateError, Handle = Self::ContactHandle>
{
    type UpdateError;
    type ContactHandle;

    fn set_elapsed_skeleton_flag(&mut self) -> Result<(), Self::UpdateError>;
    /// Supplies the exact deck-angle result produced at `82D37D34..82D37E54`
    /// together with the other direct Pumping inputs. Generic host angle math
    /// is not an allowed implementation of this producer.
    fn pumping_frame(
        &mut self,
    ) -> Result<(PumpingSettings, PumpingMode, PumpingSample), Self::UpdateError>;
    fn update_skateboard(
        &mut self,
        state: &mut PhysicsGroundState,
    ) -> Result<(), Self::UpdateError>;
    fn predict_future_deck(&mut self) -> Result<(), Self::UpdateError>;
    fn set_collision_skeleton_flag(&mut self) -> Result<(), Self::UpdateError>;
    fn set_ground_skeleton_flag(&mut self) -> Result<(), Self::UpdateError>;
    fn update_special_state(&mut self) -> Result<(), Self::UpdateError>;
    fn update_trajectory(&mut self) -> Result<(), Self::UpdateError>;
}

/// Runs the full recovered outer Ground update. State writes before a failing
/// service remain visible, matching the native non-transactional execution.
pub fn update<S: GroundUpdateServices>(
    state: &mut PhysicsGroundState,
    pumping: &mut PumpingState,
    contacts: &mut GroundContactHistory<S::ContactHandle>,
    input: GroundUpdateInput,
    services: &mut S,
) -> Result<(), GroundUpdateError<S::UpdateError>> {
    let signals = state.begin_update(input);
    if signals.set_skeleton_flag_16505 {
        call(
            GroundUpdateStage::SetElapsedSkeletonFlag,
            services.set_elapsed_skeleton_flag(),
        )?;
    }
    call(
        GroundUpdateStage::UpdateContactState,
        contact_state::update(contacts, input.contact_state, services),
    )?;
    let (settings, mode, sample) = call(
        GroundUpdateStage::LoadPumpingFrame,
        services.pumping_frame(),
    )?;
    call(
        GroundUpdateStage::UpdatePumping,
        controller::update_ground(pumping, &settings, mode, sample, services),
    )?;
    call(
        GroundUpdateStage::UpdateSkateboard,
        services.update_skateboard(state),
    )?;
    call(
        GroundUpdateStage::PredictFutureDeck,
        services.predict_future_deck(),
    )?;
    if state.flag_2722 {
        call(
            GroundUpdateStage::SetCollisionSkeletonFlag,
            services.set_collision_skeleton_flag(),
        )?;
    }
    call(
        GroundUpdateStage::SetGroundSkeletonFlag,
        services.set_ground_skeleton_flag(),
    )?;
    state.finish_update(input.timestep_2604);
    if input.contact_state.flags_2476 & 0x0040_0000 != 0 {
        call(
            GroundUpdateStage::UpdateSpecialState,
            services.update_special_state(),
        )
    } else {
        call(
            GroundUpdateStage::UpdateTrajectory,
            services.update_trajectory(),
        )
    }
}

fn call<T, E>(stage: GroundUpdateStage, result: Result<T, E>) -> Result<T, GroundUpdateError<E>> {
    result.map_err(|source| GroundUpdateError { stage, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point_graph::PointGraph;
    use crate::riding::grounded::state::contact_state::GroundContactStateInput;

    #[derive(Default)]
    struct Services {
        events: Vec<&'static str>,
        collision_signal: bool,
    }

    impl PumpingGeometry for Services {
        type Error = &'static str;

        fn height(
            &mut self,
            _normal: [f32; 4],
            _com_to_deck: [f32; 4],
        ) -> Result<f32, Self::Error> {
            self.events.push("pumping-height");
            Ok(2.0)
        }

        fn speed(
            &mut self,
            _previous: [f32; 4],
            _current: [f32; 4],
            _dt: f32,
        ) -> Result<f32, Self::Error> {
            Err("unexpected speed")
        }

        fn angular_speed(
            &mut self,
            _previous_position: [f32; 4],
            _previous_normal: [f32; 4],
            _sample: &PumpingSample,
            _dt: f32,
        ) -> Result<f32, Self::Error> {
            Err("unexpected angular speed")
        }

        fn inclination_radians(&mut self, _normal_y: f32) -> Result<f32, Self::Error> {
            Err("unexpected inclination")
        }
    }

    impl GroundContactServices for Services {
        type Error = &'static str;
        type Handle = u32;

        fn stop_contact_tracking_82d62f20(&mut self) -> Result<(), Self::Error> {
            self.events.push("contact-stop");
            Ok(())
        }

        fn clear_inactive_contact_output_3024(&mut self) -> Result<(), Self::Error> {
            self.events.push("contact-clear");
            Ok(())
        }

        fn initial_contact_handle_1920(&mut self) -> Result<Option<Self::Handle>, Self::Error> {
            self.events.push("contact-initial");
            Ok(None)
        }

        fn identify_contact_82d63c30(
            &mut self,
            _frame: &GroundContactStateInput,
        ) -> Result<Option<Self::Handle>, Self::Error> {
            self.events.push("contact-identify");
            Ok(None)
        }

        fn publish_contact_82d61268(
            &mut self,
            _frame: &GroundContactStateInput,
            _retained: Option<&Self::Handle>,
        ) -> Result<(), Self::Error> {
            self.events.push("contact-publish");
            Ok(())
        }
    }

    impl GroundUpdateServices for Services {
        type UpdateError = &'static str;
        type ContactHandle = u32;

        fn set_elapsed_skeleton_flag(&mut self) -> Result<(), Self::UpdateError> {
            self.events.push("elapsed-flag");
            Ok(())
        }

        fn pumping_frame(
            &mut self,
        ) -> Result<(PumpingSettings, PumpingMode, PumpingSample), Self::UpdateError> {
            self.events.push("pumping-frame");
            let graph = PointGraph {
                x: [0.0; 8],
                y: [0.0; 8],
            };
            Ok((
                PumpingSettings {
                    pump_vs_speed: graph,
                    pump_vs_time: graph,
                    min_crouch_vs_ground_angle: graph,
                    compression_vs_ground_angle: graph,
                    compression_vs_deck_angle: graph,
                    height_change_damping: 0.0,
                    minimum_height_change: 0.0,
                    maximum_height_change: 0.0,
                    ground_compression_scale: 0.0,
                    deck_compression_scale: 0.0,
                    angular_speed_damping: 0.0,
                },
                PumpingMode {
                    maximum_absorption_per_second: 0.0,
                    maximum_acceleration_per_second: 0.0,
                    absorption_factor: 0.0,
                    acceleration_factor: 0.0,
                },
                PumpingSample {
                    position: [0.0; 4],
                    normal: [0.0, 1.0, 0.0, 0.0],
                    com_to_deck_world: [0.0; 4],
                    deck_angle: 0.0,
                    intentional_pumping: 0,
                },
            ))
        }

        fn update_skateboard(
            &mut self,
            state: &mut PhysicsGroundState,
        ) -> Result<(), Self::UpdateError> {
            self.events.push("skateboard");
            state.flag_2722 = self.collision_signal;
            Ok(())
        }

        fn predict_future_deck(&mut self) -> Result<(), Self::UpdateError> {
            self.events.push("predict");
            Ok(())
        }

        fn set_collision_skeleton_flag(&mut self) -> Result<(), Self::UpdateError> {
            self.events.push("collision-flag");
            Ok(())
        }

        fn set_ground_skeleton_flag(&mut self) -> Result<(), Self::UpdateError> {
            self.events.push("ground-flag");
            Ok(())
        }

        fn update_special_state(&mut self) -> Result<(), Self::UpdateError> {
            self.events.push("special");
            Ok(())
        }

        fn update_trajectory(&mut self) -> Result<(), Self::UpdateError> {
            self.events.push("trajectory");
            Ok(())
        }
    }

    fn state() -> PhysicsGroundState {
        PhysicsGroundState {
            collision_force_2528: [0.0; 4],
            collision_point_2544: [0.0; 4],
            word_2560: 0,
            word_2564: 0,
            vector_2592: [0.0; 4],
            vector_2608: [0.0; 4],
            anti_flip_torque_2624: [9.0; 4],
            steering_push_scalar_2640: 1.0,
            steering_damped_turn_2644: 0.0,
            elapsed_2648: 0.1,
            collision_countdown_2652: 0.0,
            captured_position_x_2656: 0.0,
            captured_position_z_2660: 0.0,
            scalar_2664: 8.0,
            scalar_2668: 0.0,
            straighten_scale_2672: 1.0,
            vector_2688: [0.0; 4],
            scalar_2704: 0.0,
            flag_2708: false,
            flag_2720: false,
            flag_2721: false,
            flag_2722: false,
            anti_flip_nudge_applied_2723: true,
            human_player_2724: false,
            controls_latched_2725: false,
            captured_position_valid_2726: false,
            pinning_2727: true,
            was_pinning_2728: false,
            flag_2729: true,
            push_suppressed_2730: false,
            flag_2731: false,
            manual_correction_2732: false,
            manual_opposition_2733: false,
            hang_detection_frames_2740: 0,
            hang_force_frames_2744: 0,
            hung_wipeout_frames_2748: 0,
            anti_flip_nudge_frames_2752: 0,
        }
    }

    fn input(flags_2476: u32) -> GroundUpdateInput {
        GroundUpdateInput {
            wheel_contact_count_2556: 1,
            position_112: [3.0, 4.0, 5.0, 0.0],
            timestep_2604: 0.25,
            contact_state: GroundContactStateInput {
                flags_2476,
                flags_2480: 0,
                velocity_400: [0.0; 4],
                axis_464: [0.0; 4],
                contact_position_560: [0.0; 4],
                trajectory_position_592: [0.0; 4],
                reckoning_vector_1200: [0.0; 4],
                differing_contact_frame_limit: 0,
            },
        }
    }

    #[test]
    fn update_preserves_complete_outer_order_and_state_prefix() {
        let mut state = state();
        let mut pumping = PumpingState::reset_state();
        let mut contacts = GroundContactHistory::empty();
        let mut services = Services {
            collision_signal: true,
            ..Default::default()
        };

        update(
            &mut state,
            &mut pumping,
            &mut contacts,
            input(0),
            &mut services,
        )
        .unwrap();

        assert_eq!(
            services.events,
            [
                "elapsed-flag",
                "contact-stop",
                "contact-clear",
                "pumping-frame",
                "pumping-height",
                "skateboard",
                "predict",
                "collision-flag",
                "ground-flag",
                "trajectory",
            ]
        );
        assert_eq!(state.captured_position_x_2656, 3.0);
        assert_eq!(state.captured_position_z_2660, 5.0);
        assert!(state.captured_position_valid_2726);
        assert_eq!(state.anti_flip_torque_2624, [0.0; 4]);
        assert!(!state.anti_flip_nudge_applied_2723);
        assert!(!state.pinning_2727);
        assert!(!state.flag_2729);
        assert_eq!(state.scalar_2664, -1.0);
        assert_eq!(state.elapsed_2648, 0.35);
    }

    #[test]
    fn active_contact_and_special_tail_select_only_their_native_paths() {
        let mut state = state();
        state.elapsed_2648 = 0.0;
        let mut pumping = PumpingState::reset_state();
        let mut contacts = GroundContactHistory::empty();
        let mut services = Services::default();

        update(
            &mut state,
            &mut pumping,
            &mut contacts,
            input(0x0040_0000),
            &mut services,
        )
        .unwrap();

        assert_eq!(
            services.events,
            [
                "contact-publish",
                "pumping-frame",
                "pumping-height",
                "skateboard",
                "predict",
                "ground-flag",
                "special",
            ]
        );
    }
}
