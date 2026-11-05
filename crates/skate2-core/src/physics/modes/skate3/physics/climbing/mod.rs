//! Hybrid ledge climbing. This extension owns movement while attached; the
//! stock action/motion graphs resume on release. It does not forge stock states
//! or animation-bank records, and uses the same skeleton and render snapshots.
pub(super) mod approach;
mod clip;
mod contacts;
mod ledge;
#[cfg(any())]
mod tests;
use super::{Skate3PhysicsState, PlayerControls, Skate3ControllerState};
use crate::skate3_native::math_host::*;
use clip::{Clips, blend};
use skate3_core::{
    animation::output::NativeMatrix,
    math::{Basis3, Vector3},
    physics::{
        drive_frames::RetailAffineTransform, skeleton_animation_record::map_animation_parts,
    },
    player::state::PhysicalStateId,
};

#[derive(Default)]
pub(crate) struct Runtime {
    clips: Option<Clips>,
    indices: Vec<usize>,
    active: Option<Attached>,
    cooldown: f32,
    approach: Option<approach::Approach>,
    ground_entry: Vec<Transform>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Catch,
    Hang,
    Mantle,
    Settle,
}
struct Attached {
    phase: Phase,
    time: f32,
    ledge: ledge::Ledge,
    start_root: Transform,
    entry: Vec<Transform>,
    fallback: Vec<NativeMatrix>,
    board_world: Mat4,
    physical_board_world: Mat4,
    carry_board: bool,
}
impl Runtime {
    pub fn load(root: &std::path::Path, names: &[String]) -> Result<Self, String> {
        let clips = Clips::load(root)?;
        let indices = match &clips {
            Some(clips) => clips
                .reach
                .names
                .iter()
                .map(|name| {
                    names
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(name))
                        .ok_or_else(|| format!("Climbing rig bone {name} missing from skater"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        Ok(Self {
            clips,
            indices,
            ..Default::default()
        })
    }
}
fn matrix(m: NativeMatrix) -> Mat4 {
    crate::skate3_native::animation::native_matrix(m)
}
fn native(m: Mat4) -> NativeMatrix {
    let mut result = m.to_cols_array_2d();
    // Native physical frames use vector padding, not a homogeneous position.
    // A one in this lane becomes a false 60 m/s root derivative on handoff.
    result[3][3] = 0.;
    result
}
fn point(p: Vec3) -> [f32; 4] {
    [p.x, p.y, p.z, 0.]
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn pressed(controls: &PlayerControls, bit: u32) -> bool {
    let w = controls.controller.words();
    w[13] & (1 << bit) != 0 && w[6] & (1 << bit) == 0
}

/// Called before native animation/input/solve. Attached motion has a single
/// owner; no gravity, foot IK, or stock root motion competes with hand contact.
pub(super) fn advance(
    physics: &mut Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    controls: &PlayerControls,
    camera: &mut crate::skate3_native::camera::CameraRuntime,
) -> Result<bool, String> {
    let mut runtime = std::mem::take(&mut skater.climbing);
    let result = runtime.advance(physics, skater, controls, camera);
    skater.climbing = runtime;
    result
}
impl Runtime {
    fn advance(
        &mut self,
        physics: &mut Skate3PhysicsState,
        skater: &mut Skate3ControllerState,
        controls: &PlayerControls,
        camera: &mut crate::skate3_native::camera::CameraRuntime,
    ) -> Result<bool, String> {
        let dt = physics.settings.step.simulation.time_step;
        if skater.player_input.pending_teleport().is_some() {
            self.active = None;
            self.approach = None;
            self.cooldown = 0.3;
            return Ok(false);
        }
        self.cooldown = (self.cooldown - dt).max(0.);
        let Some(clips) = &self.clips else {
            return Ok(false);
        };
        let c = &clips.reach;
        if self.active.is_none() {
            return Ok(false);
        }
        let a = self.active.as_mut().unwrap();
        // A press during catch settling is not buffered into Mantle. Holding takeoff's
        // X therefore never skips the player's explicit hanging step.
        if a.phase == Phase::Hang && pressed(controls, 23) && ledge::clear(&physics.world, a.ledge)
        {
            a.phase = Phase::Mantle;
            a.time = 0.;
            crate::skate3_native::trace::info!("Climbing: pull up");
        }
        a.time += dt;
        let yaw = Quat::from_rotation_y(a.ledge.forward.x.atan2(a.ledge.forward.z));
        let hang = c.sample(c.duration());
        let hang_globals = c.globals(&hang);
        let hang_root = a.ledge.anchor - yaw * c.hands(&hang_globals)
            + yaw * contacts::clearance(c, &hang_globals);
        let mantle = &clips.mantle;
        let mantle_end = mantle.sample(mantle.duration());
        let mut complete = false;
        let sample_phase = a.phase;
        let sample_time = a.time;
        let (mut locals, root) = match a.phase {
            Phase::Catch => {
                // Give a high catch time to lower the body beneath its hands.
                // A fixed short blend made a tucked stock jump drop too fast.
                let duration =
                    (0.22 + a.start_root.translation.distance(hang_root) * 0.18).min(0.5);
                let t = (a.time / duration).min(1.);
                let mut locals = hang.clone();
                let weight = smooth(t);
                for (p, from) in locals.iter_mut().zip(&a.entry) {
                    *p = blend(*from, *p, weight);
                }
                let root = Transform {
                    translation: a.start_root.translation.lerp(hang_root, weight),
                    rotation: a.start_root.rotation.slerp(yaw, weight),
                    ..default()
                };
                if t >= 1. {
                    a.phase = Phase::Hang;
                    a.time = 0.;
                    crate::skate3_native::trace::info!("Climbing: hanging; press X to climb");
                }
                (locals, root)
            }
            Phase::Hang => (
                hang.clone(),
                Transform::from_translation(hang_root).with_rotation(yaw),
            ),
            Phase::Mantle => {
                let mut locals = mantle.sample(a.time);
                for (p, from) in locals.iter_mut().zip(&hang) {
                    *p = blend(*from, *p, smooth(a.time / 0.22));
                }
                let g = c.globals(&locals);
                // Lock the hands through the weight-bearing half, then release
                // into the authored vault and warp feet onto the clear top.
                let attached =
                    a.ledge.anchor - yaw * c.hands(&g) + yaw * contacts::clearance(c, &g);
                let end_attached = a.ledge.anchor
                    - yaw * c.hands(&c.globals(&mantle.sample(mantle.duration() * 0.55)));
                let release = smooth((a.time / mantle.duration() - 0.55) / 0.45);
                let final_root = a.ledge.landing - yaw * c.feet(&c.globals(&mantle_end));
                let translation = if release > 0. {
                    end_attached.lerp(final_root, release)
                } else {
                    attached
                };
                if a.time >= mantle.duration() {
                    a.phase = Phase::Settle;
                    a.time = 0.;
                }
                (
                    locals,
                    Transform::from_translation(translation).with_rotation(yaw),
                )
            }
            Phase::Settle => {
                let weight = smooth(a.time / 0.25);
                let locals: Vec<_> = mantle_end
                    .iter()
                    .zip(&self.ground_entry)
                    .map(|(&from, &to)| blend(from, to, weight))
                    .collect();
                let translation = a.ledge.landing - yaw * c.feet(&c.globals(&locals));
                complete = a.time >= 0.25;
                (
                    locals,
                    Transform::from_translation(translation).with_rotation(yaw),
                )
            }
        };
        // Store and publish the actual bones into the existing physical body.
        // Board helpers are never driven by the Mixamo board-at-origin track.
        let root_matrix = root.to_matrix();
        let contact_weight = match sample_phase {
            Phase::Catch => 1.,
            Phase::Hang => 1.,
            Phase::Mantle => 1. - smooth((sample_time / mantle.duration() - 0.50) / 0.05),
            Phase::Settle => 0.,
        };
        if contact_weight > 0. {
            contacts::hands(c, &mut locals, root_matrix, a.ledge, contact_weight);
        }
        let mut globals = c.globals(&locals);
        let board_index = c.index("SKATEBOARD_ROOT");
        let board_world = if a.carry_board {
            let hips = root_matrix.transform_point3(globals[c.index("HIPS")].w_axis.truncate());
            let chest = root_matrix.transform_point3(globals[c.index("SPINE3")].w_axis.truncate());
            let up = (chest - hips).normalize();
            let shoulders = root_matrix.transform_vector3(
                globals[c.index("LEFTARM")].w_axis.truncate()
                    - globals[c.index("RIGHTARM")].w_axis.truncate(),
            );
            let forward = shoulders.cross(up).normalize();
            let right = up.cross(forward).normalize();
            let stowed = Transform::from_translation((hips + chest) * 0.5 - forward * 0.20)
                .with_rotation(Quat::from_mat3(&Mat3::from_cols(right, -forward, up)));
            let mut board = blend(
                Transform::from_matrix(a.board_world),
                stowed,
                smooth(a.time / 0.2).max(if a.phase != Phase::Catch { 1. } else { 0. }),
            );
            if sample_phase == Phase::Settle {
                let hand_carry = root_matrix * c.globals(&self.ground_entry)[board_index];
                board = blend(
                    board,
                    Transform::from_matrix(hand_carry),
                    smooth(sample_time / 0.25),
                );
            }
            board.to_matrix()
        } else {
            // A dropped board remains a physical object while the skater is
            // attached. Continue its gravity/collisions independently.
            physics.board.clear_forces();
            physics
                .riding
                .start_wheel_queries(&physics.board, &physics.world)?;
            physics.riding.finish_wheel_queries()?;
            let volumes = super::colliders::world_volumes(&physics.board, &physics.settings);
            let collisions = physics
                .world
                .query_primitives(&volumes, physics.query, physics.retention)
                .to_vec();
            physics.contact_count = collisions.len();
            physics
                .board
                .advance(&collisions, [0.; 2], physics.settings.step);
            physics.riding.finish_post_physics(
                &mut physics.board,
                physics.board_wiping_out,
                physics.processed_flags_2468,
                dt,
                &Default::default(),
            )?;
            matrix(super::solve::deck_frame(&physics.board))
                * a.physical_board_world.inverse()
                * a.board_world
        };
        let board_delta = root_matrix.inverse() * board_world * globals[board_index].inverse();
        for i in 0..globals.len() {
            let mut parent = i as i32;
            while parent >= 0 && parent != board_index as i32 {
                parent = c.parents[parent as usize];
            }
            if parent == board_index as i32 {
                globals[i] = board_delta * globals[i];
            }
        }
        let mut pose = a.fallback.clone();
        for (&index, &g) in self.indices.iter().zip(&globals) {
            pose[index] = native(g);
        }
        let actual_board = board_world * a.board_world.inverse() * a.physical_board_world;
        if a.carry_board {
            physics.board.set_transform(RetailAffineTransform {
                basis: Basis3 {
                    columns: [
                        actual_board.x_axis.truncate().to_array(),
                        actual_board.y_axis.truncate().to_array(),
                        actual_board.z_axis.truncate().to_array(),
                    ],
                },
                translation: Vector3::new(
                    actual_board.w_axis.x,
                    actual_board.w_axis.y,
                    actual_board.w_axis.z,
                ),
            });
            for b in physics.board.bodies_mut() {
                zero_rates(&mut b.rates);
            }
        }
        publish_pose(physics, skater, root_matrix, pose)?;
        // Camera still uses its normal graph and collision avoidance. Read live
        // right stick while the stock animation graph is temporarily paused.
        let w = controls.controller.words();
        skater.animation_input.extra.look_x = f32::from_bits(w[9]);
        skater.animation_input.extra.look_y = f32::from_bits(w[10]);
        let feedback = skater.physical_feedback;
        super::camera_output::advance(physics, skater, &feedback, camera)?;
        physics.ticks += 1;
        physics.clock.finish_tick();
        if complete {
            // Re-seed the real ground controller at the landing. Old ground
            // query/velocity history must not pull the skater back to takeoff.
            super::player_state::resume_after_climb(physics, skater)?;
            self.active = None;
            self.cooldown = 0.3;
            crate::skate3_native::trace::info!("Climbing: returned to walking");
        }
        Ok(true)
    }
}
fn zero_rates(r: &mut skate3_core::physics::rigid_body::RetailBodyRates) {
    r.linear_velocity = Vector3::ZERO;
    r.angular_velocity = Vector3::ZERO;
    r.force_acceleration = Vector3::ZERO;
    r.torque_acceleration = Vector3::ZERO;
}
fn publish_pose(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    root: Mat4,
    pose: Vec<NativeMatrix>,
) -> Result<(), String> {
    let s = &mut skater.animated_skeleton;
    let parts =
        map_animation_parts(&pose, &s.bone_indices, &s.physics_frames).map_err(str::to_owned)?;
    s.roots.animation_to_world = native(root);
    s.roots.world_to_animation = native(root.inverse());
    s.record
        .update(&parts, &s.roots.animation_to_board, &s.masses);
    s.animation_hips = parts[23];
    s.animation_board = parts[0];
    let com = root.transform_point3(Vec3::from_slice(&s.record.centre_of_mass[..3]));
    s.board_frames
        .update_com_lift(&native(root), point(com), 0.24);
    s.board_frames.centre_of_mass = point(com);
    s.board_frames.previous_centre_of_mass = point(com);
    s.board_frames.com_velocity = [0.; 4];
    for (i, p) in parts.iter().enumerate() {
        skater
            .skeleton
            .set_part_transform(i, native(root * matrix(*p)));
    }
    // The two auxiliary COM bodies participate in the same constraints as the
    // visible bones. Leaving them at takeoff causes a false collision impulse.
    skater
        .skeleton
        .set_part_transform(24, s.board_frames.lifted_com_frame);
    skater
        .skeleton
        .set_part_transform(25, s.board_frames.com_frame);
    skater.skeleton_drives.targets.reset(
        native(root * matrix(parts[23])),
        native(root * matrix(parts[0])),
    );
    let targets = skater.skeleton_drives.targets.update_extra_targets(
        &mut skater.skeleton,
        &s.board_frames.com_frame,
        &s.board_frames.lifted_com_frame,
    );
    skater.skeleton_input.extra_target_positions =
        [targets.com, targets.lifted_com, targets.following_com];
    skater.pose_errors.set_targets(targets);
    skater.skeleton_input.drive_frames = parts;
    for b in skater.skeleton.bodies_mut() {
        zero_rates(&mut b.rates);
    }
    skater.skeleton.animation_to_world = native(root);
    skater.skeleton_input.previous_root_position = point(root.w_axis.truncate());
    skater.skeleton_input.root_velocity = [0.; 4];
    skater.skeleton_output.correction = Default::default();
    skater
        .skeleton
        .publish_physical_record(native(root * matrix(parts[0])));
    let p = &mut skater.player_input.processed;
    p.effective_anim_transform_192 = native(root).map(|v| v.map(f32::to_bits));
    p.vectors_544_560_592_608[2] = point(com).map(f32::to_bits);
    p.vectors_544_560_592_608[3] = [0; 4];
    p.flags_2472 &= !(1 << 10);
    let physical = &mut skater.player_input.physical;
    physical.skeleton.anim_to_world_11920 = native(root).map(|v| v.map(f32::to_bits));
    physical.reckoning.vector_64 = point(com).map(f32::to_bits);
    physical.reckoning.vector_16 = [0; 4];
    physical.reckoning.vector_96 = [0., 1., 0., 0.].map(f32::to_bits);
    physical.air.flag_441 = 0;
    physical.air.use_air_reckoning_452 = 0;
    skater.centre_of_mass_output = skater.centre_of_mass_filter.update(point(com), [0.; 4]);
    skater.render_pose = pose;
    skater.pose_generation = skater.pose_generation.wrapping_add(1);
    skater.player_input.player.update_count_1316 =
        skater.player_input.player.update_count_1316.wrapping_add(1);
    let _ = physics;
    Ok(())
}
