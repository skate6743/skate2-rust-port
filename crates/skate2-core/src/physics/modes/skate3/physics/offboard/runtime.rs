//! Persistent native Biped/controller/contact owners shared across physical states.
use skate3_core::player::offboard::{
    contact_packet::Packet,
    contact_queries::Layout,
    contact_toolkit::Toolkit,
    controller::{Controller, GroundResult, PlacementInput},
    ground_entry, ground_job, ground_query, ground_sync,
};
use skate3_core::point_graph::PointGraph;
use skate_data::{animation_metadata::AnimationMetadata, collections::Collections};

pub(crate) struct Runtime {
    pub air_diagnostics: super::air_diagnostics::History,
    pub air_state: skate3_core::player::offboard::air_state::State,
    pub air_prediction: Option<skate3_core::player::offboard::air_prediction::Prediction>,
    pub air_blend_curve: PointGraph<8>,
    pub air_query_settings: skate3_core::player::offboard::air_queries::Settings,
    pub jump_height: f32,
    pub jump_speed_scalar: f32,
    pub geometry_position: [f32; 4],
    pub feet: skate3_core::player::offboard::board_possession::manager::State,
    pub board_policy: super::board_effects::Policy,
    pub standard_deck_drag: f32,
    pub possession: skate3_core::player::offboard::board_possession::State,
    pub possession_settings: skate3_core::player::offboard::board_possession::lifecycle::Settings,
    pub controller: Controller,
    pub ground: ground_entry::State,
    pub contacts: Toolkit,
    pub layout: Layout,
    pub retained_contact: Packet,
    pub geometry: Option<ground_query::GroundGeometry>,
    pub geometry_flags: [bool; 3],
    pub board_settings: ground_sync::BoardSettings,
    movement_curve: PointGraph<8>,
    turn_curve: PointGraph<8>,
}

#[cfg(any())]
mod tests {
    use super::*;
    use skate3_core::player::offboard::{contact_queries::Input, ground_input::GroundInput};
    #[test]
    #[ignore = "requires extracted private stock assets"]
    fn stock_controller_consumes_real_floor_contacts_over_multiple_ticks() {
        let root = std::path::PathBuf::from(std::env::var_os("SKATE3_ASSET_ROOT").unwrap());
        let data = Collections::load(&root).unwrap();
        let metadata = skate_data::animation_banks::AnimationBanks::load(&root)
            .unwrap()
            .metadata()
            .unwrap();
        let world =
            crate::skate3_native::physics::ground::world(skate3_core::physics::contact::RetailContactMaterial {
                static_friction: 0.,
                dynamic_friction: 0.,
                restitution: 0.,
            });
        for magnitude in [0., 0.5, 1.] {
            let mut runtime = Runtime::load(&data, &metadata).unwrap();
            let frame = runtime.ground.frame_80;
            runtime.place_ground(
                &ground_entry::Input {
                    animation_frame: frame,
                    processed_velocity_608: [0.; 4],
                    processed_flags_2484: 0,
                    processed_flags_2476: 0,
                    requested_angle_2936: 0.,
                    requested_duration_2896: 0.,
                    previous_state_2504: 100,
                    previous_frame_up_208: frame[1],
                    body_position_15872: [0., 1., 0., 0.],
                },
                frame,
            );
            for tick in 0..60 {
                let frame = runtime.controller.output().physical_frame;
                let output = runtime.controller.output();
                runtime.contacts.submit(
                    super::super::contact_queries::submit_toolkit(
                        &world,
                        &runtime.layout,
                        Input {
                            position: frame[3],
                            surface_right: frame[0],
                            surface_up: frame[1],
                            surface_forward: frame[2],
                            animation_right: output.animation_frame[0],
                            animation_up: output.animation_frame[1],
                            velocity: output.velocity,
                        },
                        0,
                    )
                    .unwrap(),
                );
                runtime.begin_player_update();
                let result = runtime.step_ground(&ground_job::Input {
                    previous_state: 100,
                    frames_since_teleport: 20 + tick,
                    fallback_line_hit: None,
                    controls: GroundInput {
                        processed_flags_2472: 0x1000_0000,
                        processed_direct_2684: magnitude,
                        processed_direct_2680: 0.,
                        processed_stick_2692: 0.,
                        processed_stick_2688: magnitude,
                        processed_scale_2912: 1.,
                        processed_scale_2908: 1.,
                        frame_forward_112: [0., 0., 1.],
                    },
                    flags_2476: 0,
                    flags_2480: 0,
                    flags_2484: 0,
                    flags_2488: 0,
                    animation_motion_2864: [0.; 4],
                    duration_2896: 0.,
                    phase_2900: -1.,
                    override_duration_2904: 0.,
                    position_592: frame[3],
                    velocity_608: output.velocity,
                    skeleton_motion_16320: [0.; 4],
                    skeleton_displacements_16288_16304: [[0.; 4]; 2],
                });
                assert!(
                    result
                        .physical_frame
                        .iter()
                        .flatten()
                        .chain(result.velocity.iter())
                        .all(|v| v.is_finite()),
                    "magnitude {magnitude}, tick {tick}: {result:?}"
                );
                let motion = runtime.completed_motion();
                assert_eq!(
                    motion.contact_flags_368 & 1,
                    1,
                    "magnitude {magnitude}, tick {tick}"
                );
                runtime.ground.frame_80 = result.physical_frame;
            }
            let distance = runtime.controller.output().physical_frame[3][2];
            if magnitude == 0. {
                assert!(distance.abs() < 0.01);
            } else {
                assert!(distance > 0.1, "magnitude {magnitude}: distance {distance}");
            }
        }
    }
}
impl Runtime {
    pub(crate) fn load(data: &Collections, metadata: &AnimationMetadata) -> Result<Self, String> {
        let settings = super::settings::Settings::load(data, metadata)?;
        Ok(Self {
            air_diagnostics: Default::default(),
            air_state: Default::default(),
            air_prediction: None,
            air_blend_curve: settings.air_blend_curve,
            air_query_settings: settings.air_query,
            jump_height: settings.jump_height,
            jump_speed_scalar: settings.jump_speed_scalar,
            geometry_position: [0.; 4],
            feet: Default::default(),
            board_policy: Default::default(),
            standard_deck_drag: data.float("physicsdeck", "default", "DeckAngularDrag")?
                * f32::from_bits(0x426f_ffff),
            possession: Default::default(),
            possession_settings: settings.possession,
            controller: Controller::new(settings.controller, settings.metrics),
            ground: Default::default(),
            contacts: Default::default(),
            layout: Default::default(),
            retained_contact: Default::default(),
            geometry: None,
            geometry_flags: [false; 3],
            board_settings: settings.board,
            movement_curve: settings.movement_vs_stick_angle,
            turn_curve: settings.turn_vs_stick_angle,
        })
    }
    /// Numerical Enter only. Physical board-manager and skeleton entry surround
    /// this operation in82D30808 and must be performed by the state adapter.
    pub(crate) fn place_ground(
        &mut self,
        input: &ground_entry::Input,
        previous_frame: ground_entry::Frame,
    ) {
        self.contacts.reset_contacts(&self.layout);
        self.retained_contact = Packet::default();
        let placement = self.ground.enter(input);
        self.controller.place(PlacementInput {
            frame: placement.frame,
            velocity: placement.planar_velocity,
            body_position: placement.body_position,
            current_state: 500,
            previous_state: input.previous_state_2504,
            previous_frame,
        });
    }
    pub(crate) fn begin_player_update(&mut self) {
        self.contacts.begin_player_update(&self.layout);
    }
    pub(crate) fn step_ground(&mut self, input: &ground_job::Input) -> GroundResult {
        if let Some(geometry) = self.geometry {
            let p = geometry.frame.position;
            self.geometry_position = [p.x, p.y, p.z, 0.];
        }
        let completed = (self.contacts.contact_age > 0).then_some(self.contacts.packet);
        let prepared = ground_job::prepare(
            &mut self.ground,
            &mut self.retained_contact,
            completed,
            self.geometry.take(),
            input,
            &self.movement_curve,
            &self.turn_curve,
        );
        self.geometry_flags = prepared.geometry_flags_752_to_754;
        self.controller.step_ground(&prepared.job)
    }
    pub(crate) fn completed_motion(&self) -> ground_sync::CompletedMotion {
        let output = self.controller.output();
        ground_sync::CompletedMotion {
            contact_position_192: self.retained_contact.position,
            contact_flags_368: self.retained_contact.flags,
            physical_frame_848: output.physical_frame,
            animation_frame_912: output.animation_frame,
            vector_976: output.surface_frame[0],
            vector_992: output.surface_frame[1],
            vector_1008: output.surface_frame[2],
            vector_1040: output.velocity,
            vector_1056: output.position,
            launch_1076: output.alternate,
            flag_1077: output.sliding,
        }
    }
}
