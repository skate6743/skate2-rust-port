//! TU3 PHYSICS_STATE_GRIND_TRICK202, vtable82327174, owner PhysicalPlayer+1716.
//! Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70.
//! FillPhysOut is empty; PredictFutureOfDeck shares82D34DA8.
use super::{Skate3PhysicsState, Skate3ControllerState};
use skate3_core::{
    math::Vector3,
    physics::{board::BodyId, force_queue::QueuedPointForce},
    riding::{collision_response::CollisionResponsePhysical, grounded::drag::DRAG_FREQUENCY},
};
type V = [f32; 4];

#[derive(Default)]
pub(crate) struct GrindTrick {
    velocity: V,              //48: authored motion translated to world at60Hz
    contact_free_frames: u32, //64
    released: bool,           //68, sticky until Enter
}
impl GrindTrick {
    fn update_contact(&mut self, flags: u32) {
        if !self.released {
            self.contact_free_frames = if flags & 0x4000 != 0 {
                0
            } else {
                self.contact_free_frames.wrapping_add(1)
            };
            self.released = self.contact_free_frames > 3;
        }
    }
    pub fn wipeout_scale(&self) -> f32 {
        if self.released { 0.5 } else { 2.0 }
    }
}
pub(super) fn enter(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    skater.grind_trick = GrindTrick::default();
    skater.ground_lifecycle.skeleton_elapsed_16505 = true;
    physics
        .board
        .hook_mut()
        .drive
        .enable_angular_only(&mut skater.ground_lifecycle.board_animated_290);
    if skater.wipeout.state.mode != 1 {
        skater.wipeout.state.mode = 1;
        skater.wipeout.state.balance = 0.0;
    }
    Ok(())
}
pub(super) fn exit(physics: &mut Skate3PhysicsState) {
    set_drag(physics, 0.0);
    physics.settings.wheel_material = physics.settings.standard_wheel_material;
}
fn set_drag(physics: &mut Skate3PhysicsState, drag: f32) {
    for body in physics.board.bodies_mut() {
        body.inertia.linear_drag = drag * DRAG_FREQUENCY;
    }
}
fn rotate(m: [V; 4], v: V) -> V {
    std::array::from_fn(|i| m[2][i].mul_add(v[2], m[1][i].mul_add(v[1], m[0][i] * v[0])))
}
fn xyz(v: V) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn normalize(v: V) -> V {
    let len = skate3_core::physics::board_motion_output::length(xyz(v));
    if len > f32::from_bits(0x358637bd) {
        v.map(|x| x / len)
    } else {
        [0.0; 4]
    }
}
pub(super) fn advance(physics: &mut Skate3PhysicsState, skater: &mut Skate3ControllerState) -> Result<(), String> {
    let p = &skater.player_input.processed;
    skater.grind_trick.update_contact(p.flags_2468);
    let motion = skater.animated_skeleton.motion.trajectory;
    skater.grind_trick.velocity = rotate(
        skater.animated_skeleton.roots.animation_to_world,
        motion[3].map(|x| x * 60.0),
    );
    let heading = normalize(rotate(motion, physics.riding.reckoning_frames.heading));
    physics.riding.reckoning.dynamic_up = xyz(p.vectors_464_480_496_512_528[4].map(f32::from_bits));
    physics.riding.update_ground_reckoning_with_heading(
        &physics.board,
        super::riding_outputs::RidingPoseInputs {
            com_to_deck: xyz(p.animation_com_to_deck_752.map(f32::from_bits)),
            body_spin: skater.animation_input.extra.physical_body_spin,
        },
        p.flags_2468,
        skater.animation_input.fields.balance,
        p.flags_2476 & 0x4000_0000 != 0,
        p,
        heading,
    );
    super::ground_animation::skeleton::advance(physics, skater)?;
    skater.skeleton_output.correction.pending = true;
    let p = &skater.player_input.processed;
    skater.ground.steering.update(
        0.0,
        skater.ground_settings.board().steering.tilt_blending,
        p.flags_2468,
        p.flags_2472,
    );
    physics.settings.wheel_material = skater.ground_settings.wheel_material;
    let t = skater
        .player_input
        .toolkit
        .as_ref()
        .ok_or("GrindTrick requires current BoardToolkit")?;
    let n = physics.riding.reckoning.ground_normal;
    let drag = skate3_core::riding::braking::calculate_linear_drag(
        skate3_core::riding::braking::LinearDragInput {
            flags_2468: p.flags_2468,
            absolute_body_speed: t.absolute_speed,
            balance_2720: skater.animation_input.fields.balance,
            scalar_2724: skater.ground_lifecycle.manual_drag_2724,
            comparison_scalar: n.y,
        },
        skater.ground_settings.board().linear_drag,
    );
    if let Some(force) =
        skater
            .ground_runtime
            .calculate_collision_force(CollisionResponsePhysical {
                flags_2472: p.flags_2472,
                collision_displacement: p.collision_pose_error_736.map(f32::from_bits),
                velocity: p.vectors_400_416[1].map(f32::from_bits),
                forward: t.travel_direction,
                up: p.vectors_544_560_592_608[0].map(f32::from_bits),
                ground_normal: [n.x, n.y, n.z, 0.0],
                time_step: p.timestep_2604,
                mass: t.total_mass,
            })
    {
        physics.board.forces_mut().append(QueuedPointForce {
            tag: 15,
            force_world: xyz(force.force_2528),
            point_body: xyz(force.point_2544),
        });
    } else {
        set_drag(physics, drag);
    }
    Ok(())
}
//82D33B38..BC4 modifies only the deck body velocity after wipeout checks.
fn authored_velocity(current: V, target: V) -> V {
    let direction = normalize(target);
    let dot = (direction[0] * current[0] + direction[1] * current[1]) + direction[2] * current[2];
    let blend = f32::from_bits(0x3f666666); //82098F70:0.9
    let speed = f32::from_bits(0x3fb33333); //1.4
    std::array::from_fn(|i| {
        let along = direction[i] * dot;
        along.mul_add(
            1.0 - blend,
            target[i].mul_add(blend * speed, current[i] - along),
        )
    })
}
pub(super) fn post_velocity(physics: &mut Skate3PhysicsState, skater: &Skate3ControllerState) {
    if skater.player_input.processed.flags_2472 & 0x8000 == 0 {
        let deck = &mut physics.board.bodies_mut()[BodyId::Deck.index()];
        let v = deck.rates.linear_velocity;
        deck.rates.linear_velocity = xyz(authored_velocity(
            [v.x, v.y, v.z, 0.0],
            skater.grind_trick.velocity,
        ));
    }
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn grind_trick_latches_after_four_contact_free_updates_and_resets_on_enter() {
        let mut s = GrindTrick::default();
        for _ in 0..3 {
            s.update_contact(0);
            assert_eq!(s.wipeout_scale(), 2.0);
        }
        s.update_contact(0x4000);
        for _ in 0..3 {
            s.update_contact(0);
            assert!(!s.released);
        }
        s.update_contact(0);
        assert!(s.released);
        assert_eq!(s.wipeout_scale(), 0.5);
        s.update_contact(0x4000);
        assert!(s.released);
        s = GrindTrick::default();
        assert_eq!(s.wipeout_scale(), 2.0);
    }
    #[test]
    fn grind_trick_motion_preserves_velocity_perpendicular_to_authored_direction() {
        let v = authored_velocity([10.0, -3.0, 4.0, 0.0], [2.0, 0.0, 0.0, 0.0]);
        assert!((v[0] - 3.52).abs() < 0.00001);
        assert_eq!(&v[1..3], &[-3.0, 4.0]);
        assert_eq!(
            authored_velocity([10.0, -3.0, 4.0, 0.0], [0.0; 4]),
            [10.0, -3.0, 4.0, 0.0]
        );
    }
}
