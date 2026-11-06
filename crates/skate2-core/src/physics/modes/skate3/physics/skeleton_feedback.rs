//! Post-solver contact reports and physical/animation errors for the live skater.
//! Engine report storage is ours; spy arithmetic and report order are native.
use super::{Skate3PhysicsState, Skate3ControllerState, solve::deck_frame};
use skate3_core::physics::{
    assembly::BodySnapshot,
    board_step::CollisionBody,
    contact_feedback::spy_contact_jacobians,
    skeleton_animation_record::AnimationPartTransform,
    skeleton_board_frames::SkeletonBoardFrames,
    skeleton_body::{SkeletonCollisionInput, SkeletonContactBody, SkeletonContactReport},
    skeleton_root::SkeletonRootFrames,
};

pub(super) fn publish(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    request_partial_ragdoll: bool,
) {
    let (reports, host_contacts) = collect(physics, skater);
    skater.host_contacts = host_contacts;
    let p = &skater.player_input.processed;
    let deck = deck_frame(&physics.board);
    skater.skeleton_output.correction.observe_board(
        deck[3],
        skater.animated_skeleton.roots.predicted_board_position,
    );
    let state = p.state_2508;
    let offboard = p.category_2512 == 500 && !matches!(state, 501 | 503);
    let mut point = p.vectors_464_480_496_512_528[if matches!(state, 601 | 602) { 1 } else { 2 }]
        .map(f32::from_bits);
    // Both82BD8280 and82BE2538 read Skeleton16424 ->Reckoning1216.
    let n = physics.riding.reckoning.ground_normal;
    let mut normal = [n.x, n.y, n.z, 0.0];
    //82BD8328..8334 tests the low bit of Processed+9B0 (2480).
    if offboard && p.flags_2480 & 1 != 0 {
        point = p.vectors_880_896_912_928_944[0].map(f32::from_bits);
        normal = p.vectors_880_896_912_928_944[1].map(f32::from_bits);
    }
    super::offboard_audit_trace::contacts(physics.ticks,&reports,point,normal,request_partial_ragdoll,
        (p.state_2508,p.category_2512,p.flags_2472,offboard,skater.collision_feedback.timer));
    let frames = skater.skeleton.part_transforms();
    skater.collision_feedback.update(
        &SkeletonCollisionInput {
            dt: physics.settings.step.simulation.time_step,
            plane_point: point,
            plane_normal: normal,
            reference_velocity: if state == 201 {
                skater.skeleton.record.velocities[23]
            } else {
                skater.skeleton_input.deck_velocity
            },
            com_velocity: skater.animated_skeleton.board_frames.com_velocity,
            ragdoll: skater.skeleton_collision.is_ragdoll,
            disable_ground_filter: p.flags_2472 & 0x8000 != 0,
            ai_collision_scalar: p.flags_2472 & 0x1000_0000 != 0,
            offboard,
            entering_offboard: matches!(state, 501 | 503),
            category_600: p.category_2512 == 600,
            request_partial_ragdoll,
            physical: &skater.skeleton.record,
            part_weights: &skater.skeleton.definition.animation_masses.part_weights,
            body_frames: &frames,
        },
        &reports,
    );
    super::offboard_audit_trace::contacts_result(physics.ticks,&skater.collision_feedback);
    let region_response = skater.collision_feedback.fill_response(
        &skater.skeleton.record,
        &skater.skeleton.definition.animation_masses.part_weights,
    );
    // SkeletonCollision Fill82BD60C8 -> Collision+80/+196/+200/+208/+214/+217.
    let collision = &mut skater.player_input.physical.collision;
    collision.contact_region_response_80 = region_response;
    collision.body_response_196 = skater.collision_feedback.wipeout_times[0];
    collision.force_response_200 = skater.collision_feedback.wipeout_times[1];
    collision.surface_height_208 = skater.collision_feedback.material_12_height;
    collision.flag_214 = u8::from(skater.collision_feedback.flags.material_6);
    collision.flag_217 = u8::from(skater.collision_feedback.flags.material_12);
    skater.skeleton_collision.finish_contact_frame();
    skater.pose_errors.targets = skater.skeleton_input.extra_target_positions;
    skater.pose_errors.update(
        &skater.skeleton.record,
        skater.animated_skeleton.roots.animation_to_world,
        &skater.skeleton_input.drive_frames,
    );
    let response = if skater.skeleton_collision.partial_ragdoll {
        skater.pose_errors.partial_response()
    } else {
        skater
            .pose_errors
            .normal_response(&skater.collision_feedback, [n.x, n.y, n.z, 0.0])
    };
    skater.collision_pose_error = response.impulse;
    skater.collision_extra_errors = response.extra;
    skater.collision_maximum_error = Some(response.maximum_error);
    publish_board_observations(
        &mut skater.animated_skeleton.board_frames,
        &skater.animated_skeleton.roots,
        &deck,
        skater.skeleton.record.centre_of_mass,
        physics.settings.step.simulation.time_step,
        p.flags_2472,
    );
}

fn publish_board_observations(
    frames: &mut SkeletonBoardFrames,
    roots: &SkeletonRootFrames,
    deck: &AnimationPartTransform,
    physical_com: [f32; 4],
    dt: f32,
    flags_2472: u32,
) {
    //82BD9F3C..9F70 transforms retained16144 into16208;9F90 uses the
    //current deck for16240. Only later82BDA024 replaces16144 with physical
    //COM10928. Contact/error processing above does not consume these locals.
    frames.publish_local_observations(roots, deck);
    frames.publish_centre_of_mass(physical_com, dt, flags_2472);
}

/// A solver contact between the skater (its board or a Skeleton part) and a host world body, with
/// the normal the host body's side of the contact carries.
#[derive(Clone, Copy, Debug)]
pub(crate) struct HostContact {
    pub(crate) id: u64,
    pub(crate) board: bool,
    pub(crate) normal: [f32; 3],
    /// The contact point in world space.
    pub(crate) point: [f32; 3],
}

fn collect(physics: &Skate3PhysicsState, skater: &Skate3ControllerState) -> (Vec<SkeletonContactReport>, Vec<HostContact>) {
    let rows = physics.board.solved_contacts();
    let mut storage: Vec<_> = rows
        .iter()
        .flat_map(|row| row.words().iter().copied())
        .collect();
    let mut count = 0;
    spy_contact_jacobians(
        &mut storage,
        rows.len() as u32,
        &mut count,
        physics.settings.step.simulation.frequency,
        |id| {
            resolve(id, physics, skater).map_or([0; 4], |body| {
                let p = body.rates.position;
                [p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), 0]
            })
        },
    );
    let mut reports = Vec::with_capacity(16);
    let mut mod_reports = Vec::new();
    let mut host_contacts = Vec::new();
    for spy in storage[..count as usize * 28].chunks_exact(28) {
        let a = CollisionBody::from_contact_id(spy[24]);
        let b = CollisionBody::from_contact_id(spy[25]);
        for (own, other, side_a) in [(a, b, true), (b, a, false)] {
            let board = match own {
                CollisionBody::Board(_) => true,
                CollisionBody::Attached(part) if part < 24 => false,
                _ => continue,
            };
            let CollisionBody::Attached(index) = other else { continue };
            let Some((_, solid)) = physics.world_proxies.solids.iter().find(|(i, _)| *i == index) else { continue };
            let sign = if side_a { -1.0 } else { 1.0 };
            host_contacts.push(HostContact { id: solid.id, board, normal: std::array::from_fn(|i| f32::from_bits(spy[i]) * sign),
                point: std::array::from_fn(|i| f32::from_bits(spy[12 + i])) });
        }
        //82768418 rejects the same logical Body and equal nonnull Body+32
        //owners. Board82C067F4 and Skeleton82BE4620 both store this actor.
        //Their eligible cross contacts still participate in the shared solve.
        let remote_base = skater.skeleton.bodies().len() + skater.skeleton_drives.targets.bodies.len();
        let external = |body| matches!(body, CollisionBody::StaticWorld)
            || matches!(body, CollisionBody::Attached(index) if index >= remote_base);
        if !external(a) && !external(b) {
            continue;
        }
        for (own, other, side_a) in [(a, b, true), (b, a, false)] {
            let CollisionBody::Attached(part) = own else {
                continue;
            };
            if part >= 24 {
                continue;
            }
            let vector = |offset| std::array::from_fn(|i| f32::from_bits(spy[offset + i]));
            let point = vector(12);
            let normal = vector(0).map(|v| v * if side_a { 1.0 } else { -1.0 });
            let other_group = match other {
                CollisionBody::StaticWorld => 0,
                CollisionBody::Board(_) => physics.board.collision_group(),
                CollisionBody::Attached(index) => physics.world_proxies.groups.get(&index)
                    .copied().unwrap_or(skater.skeleton_collision.assembly_group),
            };
            let destination=if matches!(other,CollisionBody::Attached(index)
                if physics.world_proxies.solids.iter().any(|(i,_)|*i==index)) { &mut mod_reports } else { &mut reports };
            let vehicle = other_group == 8;
            destination.push(SkeletonContactReport {
                part,
                normal,
                point,
                tag: if side_a {
                    spy[26] & 0xffff
                } else {
                    spy[26] >> 16
                },
                other_group,
                other_entity: None,
                body_a: contact_body(
                    resolve(spy[24], physics, skater),
                    point,
                    vehicle && is_mod_solid_contact(spy[24], physics),
                ),
                body_b: contact_body(
                    resolve(spy[25], physics, skater),
                    point,
                    vehicle && is_mod_solid_contact(spy[25], physics),
                ),
                side_a,
                solved_vector: vector(20),
            });
        }
    }
    // A large set of floor contacts must not crowd a moving prop impact out
    // of the native 16-report observation budget. Preserve order within each
    // group; the physical solve above still receives EVERY contact.
    mod_reports.extend(reports);
    mod_reports.truncate(16);
    (mod_reports, host_contacts)
}
fn is_mod_solid_contact(id: u32, physics: &Skate3PhysicsState) -> bool {
    matches!(
        CollisionBody::from_contact_id(id),
        CollisionBody::Attached(index)
            if physics.world_proxies.solids.iter().any(|(i, _)| *i == index)
    )
}

fn resolve<'a>(
    id: u32,
    physics: &'a Skate3PhysicsState,
    skater: &'a Skate3ControllerState,
) -> Option<&'a BodySnapshot> {
    match CollisionBody::from_contact_id(id) {
        CollisionBody::StaticWorld => None,
        CollisionBody::Board(id) => Some(&physics.board.bodies()[id.index()]),
        CollisionBody::Attached(part) => skater.skeleton.bodies().iter()
            .chain(skater.skeleton_drives.targets.bodies.iter())
            .chain(physics.world_proxies.bodies.iter()).nth(part),
    }
}
fn contact_body(body: Option<&BodySnapshot>, point: [f32; 4], vehicle: bool) -> SkeletonContactBody {
    match body {
        Some(body) => {
            let v = if vehicle {
                linear_velocity_at_point(body, point)
            } else {
                let v = body.rates.linear_velocity;
                [v.x, v.y, v.z, 0.0]
            };
            SkeletonContactBody {
                state_flags: body.state_flags,
                inverse_mass: body.inertia.inverse_mass,
                linear_velocity: v,
            }
        }
        None => SkeletonContactBody {
            state_flags: 1,
            inverse_mass: 0.0,
            linear_velocity: [0.0; 4],
        },
    }
}

/// Group-8 contacts use the surface point velocity (v + ω×r), matching the
/// solver contact row and keeping mVehicleContact low when riding a moving car.
fn linear_velocity_at_point(body: &BodySnapshot, point: [f32; 4]) -> [f32; 4] {
    let r = [
        point[0] - body.rates.position.x,
        point[1] - body.rates.position.y,
        point[2] - body.rates.position.z,
        0.0,
    ];
    let w = body.rates.angular_velocity;
    let v = body.rates.linear_velocity;
    [
        v.x + w.y * r[2] - w.z * r[1],
        v.y + w.z * r[0] - w.x * r[2],
        v.z + w.x * r[1] - w.y * r[0],
        0.0,
    ]
}

#[cfg(any())]
mod tests {
    use super::*;
    use skate3_core::physics::skeleton_animation_record::IDENTITY;

    #[test]
    fn vehicle_contact_uses_point_velocity_for_group_eight() {
        // `BodySnapshot` has no `Default`: every field is a native body value,
        // so an implicit zero would be a body with no basis. Spell out a resting
        // unit body the way the other physics tests do.
        let identity = skate3_core::math::Basis3 {
            columns: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        };
        let mut body = skate3_core::physics::assembly::BodySnapshot {
            state_flags: 4,
            rates: skate3_core::physics::rigid_body::RetailBodyRates {
                orientation: skate3_core::physics::rigid_body::RetailQuaternion::IDENTITY,
                basis: identity,
                world_inverse_inertia: identity,
                position: skate3_core::math::Vector3::ZERO,
                linear_velocity: skate3_core::math::Vector3::ZERO,
                angular_velocity: skate3_core::math::Vector3::ZERO,
                force_acceleration: skate3_core::math::Vector3::ZERO,
                torque_acceleration: skate3_core::math::Vector3::ZERO,
                kinetic_energy: 0.,
                cool_down: 0,
            },
            inertia: skate3_core::physics::rigid_body::RetailInertiaDynamics {
                inverse_tensor: skate3_core::math::Vector3::new(1., 1., 1.),
                inverse_mass: 1.,
                spherical: 0.,
                maximum_linear_velocity: f32::MAX,
                maximum_angular_velocity: f32::MAX,
                linear_drag: 0.,
                angular_drag: 0.,
            },
        };
        body.rates.position = skate3_core::math::Vector3::new(0., 0., 0.);
        body.rates.linear_velocity = skate3_core::math::Vector3::new(1., 0., 0.);
        body.rates.angular_velocity = skate3_core::math::Vector3::new(0., 2., 0.);
        let point = [0., 0., 1., 0.];
        let v = linear_velocity_at_point(&body, point);
        assert!((v[0] - 3.).abs() < 1e-5);
        assert!(v[1].abs() < 1e-5);
        assert!(v[2].abs() < 1e-5);
    }

    #[test]
    fn retained_com_is_localized_before_new_physical_com_publication() {
        let mut roots = SkeletonRootFrames::default();
        roots.world_to_animation[3] = [-10.0, -20.0, -30.0, 0.0];
        let mut deck = IDENTITY;
        deck[3] = [15.0, 26.0, 37.0, 0.0];
        let old_com = [11.0, 22.0, 33.0, 4.0];
        let new_com = [13.0, 26.0, 39.0, 8.0];
        for flags in [0, 0x400] {
            let mut frames = SkeletonBoardFrames {
                centre_of_mass: old_com,
                ..Default::default()
            };
            publish_board_observations(&mut frames, &roots, &deck, new_com, 0.5, flags);
            assert_eq!(frames.local_centre_of_mass, [1.0, 2.0, 3.0, 0.0]);
            assert_eq!(frames.local_board_position, [5.0, 6.0, 7.0, 0.0]);
            assert_eq!(frames.centre_of_mass, new_com);
            assert_eq!(frames.com_velocity, [4.0, 8.0, 12.0, 8.0]);
            assert_eq!(
                frames.previous_centre_of_mass,
                if flags == 0 { old_com } else { new_com }
            );

            //The next observation uses the last published COM, but the new
            //root and deck. The history flag must not erase this distinction.
            roots.world_to_animation[3] = [-12.0, -24.0, -36.0, 0.0];
            deck[3] = [20.0, 30.0, 40.0, 0.0];
            publish_board_observations(&mut frames, &roots, &deck, old_com, 0.5, flags);
            assert_eq!(frames.local_centre_of_mass, [1.0, 2.0, 3.0, 0.0]);
            assert_eq!(frames.local_board_position, [8.0, 6.0, 4.0, 0.0]);
            assert_eq!(frames.centre_of_mass, old_com);
            assert_eq!(frames.com_velocity, [-4.0, -8.0, -12.0, -8.0]);
            roots.world_to_animation[3] = [-10.0, -20.0, -30.0, 0.0];
            deck[3] = [15.0, 26.0, 37.0, 0.0];
        }
    }
}
