//! BipedGround's live Enter, ground job, Sync and Exit owners.
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState};
use ground_entry::{Frame, Vector};
use ground_query::GroundQueryScene;
use skate3_core::player::offboard::{contact_queries, ground_entry, ground_query, ground_sync};

pub(crate) fn publish(skater: &mut Skate3ControllerState) {
    let runtime = &skater.offboard;
    let motion = runtime.controller.output();
    let packet = &runtime.retained_contact;
    let value = skate3_core::player::offboard::ground_lifecycle::publish(
        &runtime.ground,
        &skate3_core::player::offboard::ground_lifecycle::PublicationInput {
            ground_flags_752_to_754: runtime.geometry_flags,
            contact_flags_368: packet.flags,
            contact_position_192: packet.position,
            query_position_816: runtime.geometry_position,
            processed_flags_2476: skater.player_input.processed.flags_2476,
            ground_kind_356: packet.kind_164,
            ground_scalar_360: packet.distance_168,
            motion_vector_1040: motion.velocity,
            motion_up_864: motion.physical_frame[1],
            hand_flags: runtime.feet.hands.map(|h| {
                [
                    h.flags_104_to_107[1],
                    h.flags_104_to_107[2],
                    h.flags_104_to_107[3],
                ]
            }),
        },
    );
    let physical = &mut skater.player_input.physical;
    let out = &mut physical.off_board;
    out.flag_304 = u8::from(value.offboard_flag_304);
    out.kind_88 = value.offboard_kind_88;
    out.scalar_112 = value.offboard_scalar_112;
    out.flag_329 = u8::from(value.offboard_flag_329);
    out.flag_330 = u8::from(value.offboard_flag_330);
    out.distance_116 = value.offboard_distance_116;
    out.flag_334 = u8::from(value.offboard_flag_334);
    out.scalar_32 = value.offboard_scalar_32;
    out.flag_328 = u8::from(value.offboard_flag_328);
    out.flags_306_307 = value.offboard_hand_flags_306_307.map(u8::from);
    physical.reckoning.vector_144 = value.animation_vector_144.map(f32::to_bits);
    physical.reckoning.flag_164 = u8::from(value.animation_flag_164);
    skater.player_state.state_flags[86 - 52] = value.physics_flag_86;
    out.phase_80 = runtime.controller.state.cadence.phase.phase;
    out.locomotion_84 = runtime.controller.state.cadence.locomotion_index;
}

pub(crate) fn enter(_physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = &skater.player_input.processed;
    let mut frame = skater.animated_skeleton.roots.animation_to_world;
    //GetEffectiveRoot82BE3650 applies the animation stance before Enter.
    if p.flags_2476 & 4 != 0 {
        for i in [0, 2] {
            frame[i] = frame[i].map(|v| -v);
        }
    }
    skater.offboard.place_ground(
        &ground_entry::Input {
            animation_frame: frame,
            processed_velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
            processed_flags_2484: p.flags_2484,
            processed_flags_2476: p.flags_2476,
            requested_angle_2936: skater.animation_input.extra.biped_start_angle,
            requested_duration_2896: skater.animation_input.fields.animation_time,
            previous_state_2504: p.state_2504,
            previous_frame_up_208: p.effective_anim_transform_192[1].map(f32::from_bits),
            body_position_15872: skater.animated_skeleton.board_frames.com_frame[3],
        },
        p.effective_anim_transform_192
            .map(|v| v.map(f32::from_bits)),
    );
    if !matches!(p.state_2504, 500..=502) {
        skater.offboard.feet.reset();
        skater.foot_ik.state.enable_feet(false);
        for limb in &mut skater.foot_ik.state.limbs {
            limb.board_blend = 0.;
            limb.external_blend = 0.;
            limb.mode = skate3_core::animation::foot_ik::status::Mode::Disabled;
        }
    }
    skater.offboard.feet.flags_304_to_307[0] = p.state_2508 == 502;
    skater
        .ground_lifecycle
        .skeleton_controller
        .request(4, &mut skater.skeleton_collision)?;
    skater.ground_lifecycle.trajectory.cancel();
    Ok(())
}
pub(crate) fn exit(_physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) {
    skater
        .offboard
        .contacts
        .reset_contacts(&skater.offboard.layout);
    skater.offboard.geometry = None;
}
pub(crate) fn update(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let input = super::ground_input::prepare(skater);
    skater.offboard.step_ground(&input);
    let motion = skater.offboard.completed_motion();
    let mut state = std::mem::take(&mut skater.offboard.ground);
    let mut services = Services {
        physics,
        skater,
        error: None,
    };
    let result = ground_sync::sync(&mut state, &motion, &mut services);
    services.skater.offboard.ground = state;
    result.map_err(str::to_owned)?;
    match services.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct Services<'a> {
    physics: &'a mut Skate3PhysicsState,
    skater: &'a mut Skate3ControllerState,
    error: Option<String>,
}
impl GroundQueryScene for Services<'_> {
    type Error = &'static str;
    fn edge_candidates(
        &mut self,
        search: &ground_query::EdgeSearch,
    ) -> Result<Vec<ground_query::Edge>, Self::Error> {
        let cache = super::mod_solid_ground::VehicleEdgeCache::build(
            &self.physics.world_proxies.solids,
        );
        cache.with_primary_edges(|edges| {
            super::ground_query::with_world_scene(
                &self.physics.world,
                edges,
                &[],
                |scene| scene.edge_candidates(search),
            )
        })
    }
    fn query_lines(
        &mut self,
        packet: &ground_query::GroundQueryPacket,
    ) -> Result<[Option<ground_query::LineHit>; 7], Self::Error> {
        let cache = super::mod_solid_ground::VehicleEdgeCache::build(
            &self.physics.world_proxies.solids,
        );
        cache.with_primary_edges(|edges| {
            super::ground_query::with_world_scene(
                &self.physics.world,
                edges,
                &[],
                |scene| scene.query_lines(packet),
            )
        })
    }
}
impl ground_sync::Services for Services<'_> {
    type Launch = Result<skate3_core::player::offboard::air_launch::Launch, String>;
    type Candidate = super::mod_solid_ground::VehicleCandidate;
    fn processed(&self) -> ground_sync::Processed {
        let p = &self.skater.player_input.processed;
        ground_sync::Processed {
            frame_192: p
                .effective_anim_transform_192
                .map(|v| v.map(f32::from_bits)),
            position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
            flags_2476: p.flags_2476,
            flags_2480: p.flags_2480,
            flags_2484: p.flags_2484,
            flags_2488: p.flags_2488,
            elapsed_2664: p.state_timer_2664,
            value_2852: p.secondary_ground_timer_2852,
            query_context: ground_query::QueryContext {
                selection_flags_2948: p.actor_query_2948,
                matching_id_2952: p.actor_query_2952 as i32,
            },
        }
    }
    fn prepare_biped_launch(&mut self) -> Self::Launch {
        super::air_trajectory::prepare(self.skater, false, 1.0)
    }
    fn skeleton_point_10960(&self) -> Vector {
        self.skater.animated_skeleton.record.centre_of_mass
    }
    fn launch_air(&mut self, launch: Self::Launch, position: Vector) {
        let result = launch.and_then(|mut launch| {
            launch.position = position;
            super::air_trajectory::launch(self.physics, self.skater, launch)
        });
        if let Err(error) = result {
            self.error = Some(error);
        }
    }
    fn update_skeleton(&mut self, frame: Frame, com: Vector) {
        if let Err(error) = super::skeleton_ground::update(self.physics, self.skater, frame, com) {
            self.error = Some(error);
        }
    }
    fn board_settings(&self) -> ground_sync::BoardSettings {
        self.skater.offboard.board_settings
    }
    fn board_flags_12836(&self) -> u8 {
        self.skater.ground_lifecycle.trajectory.flags_12836
    }
    fn probe_board(&mut self, position: Vector) -> Option<Self::Candidate> {
        super::mod_solid_ground::probe_vehicle(&self.physics.world_proxies.solids, position)
    }
    fn candidate_key_188(&self, candidate: &Self::Candidate) -> [u32; 2] {
        super::mod_solid_ground::candidate_key(candidate)
    }
    fn bone_23_position(&mut self) -> Vector {
        skate3_core::physics::skeleton_animation_record::compose_affine(
            &self.skater.animated_skeleton.roots.animation_to_world,
            &self.skater.animated_skeleton.record.pose[23],
        )[3]
    }
    fn classify_board(
        &mut self,
        candidate: &Self::Candidate,
        bone: Vector,
        bounds: ground_sync::Bounds,
        limits: ground_sync::BoardLimits,
    ) -> bool {
        super::mod_solid_ground::solid_by_id(&self.physics.world_proxies.solids, candidate.id)
            .is_some_and(|solid| {
                super::mod_solid_ground::classify_vehicle(solid, bone, bounds, limits)
            })
    }
    fn query_board(&mut self, _: Vector, _: ground_sync::Bounds, _: ground_sync::BoardLimits) {
        // Mod vehicles are classified directly from live solid poses; there is
        // no separate native object query record for them yet.
    }
    fn bind_board(&mut self, _: [u32; 2]) {}
    fn commit_bound_board(&mut self) {}
    fn commit_free_board(&mut self, _: Frame) {
        //No selected object means no object action to commit. Holding GrabWorld
        //still reaches the authored graphs, skeleton, toolkit and edge queries.
    }
    fn reset_board(&mut self) {
        self.skater.ground_lifecycle.trajectory.cancel();
    }
    fn update_toolkit(&mut self, input: ground_sync::ToolkitInput) {
        let [
            position,
            forward,
            up,
            right,
            velocity,
            animation_up,
            animation_right,
        ] = input.0;
        match super::contact_queries::submit_toolkit(
            &self.physics.world,
            &self.skater.offboard.layout,
            contact_queries::Input {
                position,
                surface_right: right,
                surface_up: up,
                surface_forward: forward,
                animation_right,
                animation_up,
                velocity,
            },
            self.skater.player_input.processed.actor_query_2948,
        ) {
            Ok(observation) => self.skater.offboard.contacts.submit(observation),
            Err(error) => self.error = Some(error.into()),
        }
    }
    fn deck_half_wheelbase(&self) -> f32 {
        self.physics.settings.authored[0].translation.z.abs()
    }
    fn submit_ground_query(&mut self, packet: ground_query::GroundQueryPacket) {
        match self.query_lines(&packet) {
            Ok(hits) => {
                self.skater.offboard.geometry = Some(ground_query::interpret_hits(&packet, hits))
            }
            Err(error) => self.error = Some(error.into()),
        }
    }
}
