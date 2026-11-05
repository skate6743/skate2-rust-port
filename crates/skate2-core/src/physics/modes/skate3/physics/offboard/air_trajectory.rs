//! Biped trajectory launch/completion through the actual scene query owner.
use crate::skate3_native::physics::{Skate3PhysicsState, Skate3ControllerState, air_trajectory::AirTrajectoryRuntime};
use ground_query::GroundQueryScene;
use skate3_core::player::offboard::{
    air_completion, air_launch, air_prediction, air_queries, ground_query,
};
pub(crate) fn prepare(skater: &Skate3ControllerState, current: bool, height_multiplier: f32) -> Result<air_launch::Launch, String> {
    let p = &skater.player_input.processed;
    let toolkit = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("Biped launch has no board toolkit")?;
    let c = &skater.offboard.controller;
    let input = air_launch::Input {
        previous_state: p.state_2504,
        previous_category: p.category_2516,
        current_state: p.state_2508,
        current_category: p.category_2512,
        flags_2472: p.flags_2472,
        flags_2476: p.flags_2476,
        flags_2480: p.flags_2480,
        frame_forward_224: p.effective_anim_transform_192[2].map(f32::from_bits),
        up_544: p.vectors_544_560_592_608[0].map(f32::from_bits),
        velocity_608: p.vectors_544_560_592_608[3].map(f32::from_bits),
        position_592: p.vectors_544_560_592_608[2].map(f32::from_bits),
        target_112: toolkit.deck[3],
        velocity_912: p.vectors_880_896_912_928_944[2].map(f32::from_bits),
        //Retained live grind primitive, published before native state entry.
        grind_position_1120: p.grind_position_1120.map(f32::from_bits),
        grind_axis_1136: p.grind_direction_1136.map(f32::from_bits),
        stick: [
            skater.animation_input.extra.biped_world_x,
            skater.animation_input.extra.biped_world_z,
        ],
    };
    Ok(air_launch::prepare(
        &input,
        &air_launch::JumpInput {
            reference_up_144: c.state.frame_output.frame[1],
            contact_active_708: c.state.contact.active,
            height_792: skater.offboard.jump_height * height_multiplier,
            velocity_scalar_796: skater.offboard.jump_speed_scalar,
            speed_704: c.state.motion.speed_704,
            steering_760: c.state.intent.steering,
            angular_velocity_688: c.state.motion.angular_velocity_688,
            turn_curve_1328: &c.settings.movement_velocity.turn_vs_speed,
        },
        current,
    ))
}
pub(crate) fn launch(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    launch: air_launch::Launch,
) -> Result<(), String> {
    let g = physics.settings.step.simulation.gravity_acceleration;
    let prepared = air_queries::prepare(
        &launch,
        [g.x, g.y, g.z, 0.],
        &skater.offboard.air_query_settings,
    );
    let metadata = physics.world.query_metadata()?;
    let results = prepared
        .candidates
        .iter()
        .map(|candidate| {
            let mut hit = AirTrajectoryRuntime::query(
                &physics.world,
                candidate.request(&skater.offboard.air_query_settings),
            )?;
            if hit.valid() {
                hit.surface = *metadata
                    .packed_surfaces
                    .get(hit.geometry as usize)
                    .ok_or("Biped trajectory hit lost native surface metadata")?
                    as u32;
            }
            Ok(hit)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let p = &skater.player_input.processed;
    let completed = air_completion::complete(
        &launch,
        &prepared.candidates,
        &results,
        p.vectors_544_560_592_608[0].map(f32::from_bits),
        |candidate| {
            if !candidate.result.valid() || candidate.result.contact_frame < 16 {
                return Ok(false);
            }
            //82D6D4D0 queries the real edge AABB before entering its ledge adjustment loop.
            let position = candidate
                .candidate
                .trajectory
                .position_at(candidate.result.contact_time);
            let apex = candidate.candidate.trajectory.highest_position().0;
            let vector = |v: [f32; 4]| skate3_core::math::Vector3::new(v[0], v[1], v[2]);
            let search = ground_query::EdgeSearch {
                min: vector(std::array::from_fn(|i| position[i].min(apex[i]) - 2.)),
                max: vector(std::array::from_fn(|i| position[i].max(apex[i]) + 2.)),
                frame: ground_query::Frame::IDENTITY,
                context: ground_query::QueryContext {
                    selection_flags_2948: p.actor_query_2948,
                    matching_id_2952: p.actor_query_2952 as i32,
                },
                narrow_forward: false,
            };
            let cache = super::mod_solid_ground::VehicleEdgeCache::build(
                &physics.world_proxies.solids,
            );
            let edges = cache.with_primary_edges(|vehicle_edges| {
                super::ground_query::with_world_scene(
                    &physics.world,
                    vehicle_edges,
                    &[],
                    |scene| scene.edge_candidates(&search),
                )
            })?;
            let mut selected = None;
            for edge in skate3_core::player::offboard::air_ledge::visible_edges(
                &edges,
                candidate.candidate.trajectory.position,
            ) {
                if let Some(ledge) = skate3_core::player::offboard::air_ledge::candidate(
                    candidate.candidate.trajectory,
                    edge,
                    skater.offboard.air_query_settings.sphere_radius,
                    p.effective_anim_transform_192[2].map(f32::from_bits),
                    p.vectors_544_560_592_608[0].map(f32::from_bits),
                    candidate.impact_position[1],
                ) {
                    if selected.as_ref().is_none_or(
                        |old: &skate3_core::player::offboard::air_ledge::Candidate| {
                            ledge.point[1] >= old.point[1]
                        },
                    ) {
                        selected = Some(ledge);
                    }
                }
            }
            let Some(ledge) = selected else {
                return Ok(false);
            };
            let Some(packet) = ground_query::prepare_packet(
                search.frame,
                search.context,
                ground_query::EdgeSelection {
                    edge: ledge.edge,
                    closest: vector(ledge.point),
                },
                physics.settings.authored[0].translation.z.abs(),
            ) else {
                return Ok(false);
            };
            let cache = super::mod_solid_ground::VehicleEdgeCache::build(
                &physics.world_proxies.solids,
            );
            let mut hits = cache.with_primary_edges(|vehicle_edges| {
                super::ground_query::with_world_scene(
                    &physics.world,
                    vehicle_edges,
                    &[],
                    |scene| scene.query_lines(&packet),
                )
            })?;
            for (line, destination) in packet.lines.iter().zip(&mut hits) {
                if let Some(hit) = physics.world.external_line(line.start, line.end, line.radius) {
                    if destination.as_ref().is_none_or(|old| hit.hit.geometry.fraction < old.fraction) {
                        *destination = Some(ground_query::LineHit {
                            position: hit.hit.geometry.position,
                            face_normal: hit.hit.geometry.normal,
                            fraction: hit.hit.geometry.fraction,
                            packed_surface: hit.surface,
                        });
                    }
                }
            }
            if ground_query::interpret_hits(&packet, hits).kind == 0 {
                return Ok(false);
            }
            candidate.candidate.trajectory = ledge.arc;
            candidate.normal = ledge.normal;
            candidate.impact_position = ledge.point;
            candidate.impact_velocity = ledge
                .arc
                .velocity_at(ledge.frame as f32 * f32::from_bits(0x3c888889));
            candidate.landing_frame = ledge.frame;
            Ok(true)
        },
    )?;
    let index =
        air_completion::selected(&completed).ok_or("Biped trajectory produced no candidates")?;
    skater.offboard.air_prediction = Some(air_prediction::Prediction::complete(
        &prepared,
        completed[index].clone(),
    ));
    Ok(())
}
pub(crate) fn requery(physics: &Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = &skater.player_input.processed;
    if p.flags_2472 & 0x10000000 != 0 || p.flags_2488 & 0x400000 != 0 {
        return Ok(());
    }
    let prediction = skater
        .offboard
        .air_prediction
        .as_mut()
        .ok_or("BipedAir requery has no trajectory")?;
    if !prediction.can_requery {
        return Ok(());
    }
    prediction.can_requery = false;
    let request = prediction
        .result
        .candidate
        .request(&skater.offboard.air_query_settings);
    let result = AirTrajectoryRuntime::query(&physics.world, request)?;
    if result.valid() {
        prediction.requery_position = Some(result.contact_position);
    }
    Ok(())
}
