//! The Skate 2 world's moving bodies as extra bodies of the Skate 3 player's solve: each joins
//! at its centre of mass as a finite-mass shadow, and only a body this game moves keeps the
//! reaction the solve gives it.
use super::*;
use skate3_core::physics::{
    assembly::BodySnapshot,
    board_world::BoardWorldVolume,
    rigid_body::{RetailQuaternion, world_inverse_inertia},
    world_contact::ContactPrimitive,
};

fn xyz(v: Vector3) -> [f32; 3] {
    [v.x, v.y, v.z]
}
fn vector(v: [f32; 3]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
/// Conservative enclosing spheres for the remote broad phase.
pub(crate) fn bounds(p: ContactPrimitive) -> (Vec3, f32) {
    match p {
        ContactPrimitive::Sphere(s) => (Vec3::from_array(xyz(s.center)), s.radius),
        ContactPrimitive::Capsule {
            center,
            half_length,
            radius,
            ..
        } => (Vec3::from_array(xyz(center)), half_length + radius),
        ContactPrimitive::RoundedBox {
            center,
            half_extents,
            radius,
            ..
        } => (
            Vec3::from_array(xyz(center)),
            Vec3::from_array(xyz(half_extents)).length() + radius,
        ),
        ContactPrimitive::Triangle(t) => {
            let vertices = t.vertices.map(|v| Vec3::from_array(xyz(v)));
            let center = (vertices[0] + vertices[1] + vertices[2]) / 3.;
            (
                center,
                vertices
                    .iter()
                    .map(|v| v.distance(center))
                    .fold(0., f32::max)
                    + t.fatness,
            )
        }
    }
}
#[derive(Default)]
pub(crate) struct Proxies {
    pub bodies: Vec<BodySnapshot>,
    pub volumes: Vec<BoardWorldVolume>,
    pub solids: Vec<(usize, skate_dynamics::SolidBody)>,
    /// Solids (by attached index) whose board/skeleton contacts come from
    /// their authored mesh's native triangles instead of their Parry shapes.
    pub meshes: Vec<(usize, crate::skate3_native::modding::moving_queries::MeshQuery)>,
    pub groups: std::collections::BTreeMap<usize, u32>,
    pub actors: std::collections::BTreeMap<usize,(u64,usize)>,
    pub dynamics_before: Vec<(u64, usize, [f32; 3], [f32; 3])>,
    pub dynamics_deltas: Vec<(u64, [f32; 3], [f32; 3], [f32; 3], [f32; 3])>,
}
impl Proxies {
    /// One actual compound rigid body, expressed at its COM in its principal
    /// inertia frame. Collider poses remain in world space and are not shells.
    pub fn append_solid(
        &mut self, solid: skate_dynamics::SolidBody, physics: &Skate3PhysicsState,
        skater: &Skate3ControllerState, locally_owned: bool,
    ) {
        if !solid.center_of_mass.is_finite() || !solid.linvel.is_finite()
            || !solid.angvel.is_finite() || !solid.inertia_rotation.is_finite() { return; }
        let index = skater.skeleton.bodies().len()
            + skater.skeleton_drives.targets.bodies.len() + self.bodies.len();
        let mut body = physics.board.bodies()[0];
        let q = solid.inertia_rotation;
        let q = Quat::from_xyzw(q.x, q.y, q.z, q.w).normalize();
        body.rates.position = vector(solid.center_of_mass.to_array());
        body.rates.orientation = RetailQuaternion { x:q.x, y:q.y, z:q.z, w:q.w };
        body.rates.basis = skate3_core::math::Basis3 { columns: Mat3::from_quat(q).to_cols_array_2d() };
        // Ownership controls feedback, not physical mass. A remote moving body
        // participates as a finite-mass shadow; only our actor keeps its reaction.
        body.inertia.inverse_mass = solid.inverse_mass;
        body.inertia.inverse_tensor = vector(solid.inverse_inertia.to_array());
        // A host-owned body's drag and caps belong to its original solver.
        body.inertia.linear_drag = 0.;
        body.inertia.angular_drag = 0.;
        body.inertia.maximum_linear_velocity = 100_000.;
        body.inertia.maximum_angular_velocity = 100_000.;
        body.rates.world_inverse_inertia = world_inverse_inertia(body.rates.basis, body.inertia.inverse_tensor);
        body.rates.linear_velocity = vector(solid.linvel.to_array());
        body.rates.angular_velocity = vector(solid.angvel.to_array());
        body.rates.force_acceleration = Vector3::ZERO;
        body.rates.torque_acceleration = Vector3::ZERO;
        body.state_flags = 4;
        self.groups.insert(index, solid.contact_group);
        if locally_owned && solid.inverse_mass > 0. {
            self.dynamics_before.push((solid.id, index, solid.linvel.to_array(), solid.angvel.to_array()));
        }
        self.bodies.push(body);
        self.solids.push((index, solid));
    }

    pub fn capture_dynamics_reactions(
        &mut self, reactions: &[skate3_core::physics::rigid_body::RetailReactionCorrections], dt: f32,
    ) {
        self.dynamics_deltas.clear();
        if !(dt.is_finite() && dt > 0.) { return; }
        for &(id, index, _, _) in &self.dynamics_before {
            let Some(r) = reactions.get(skate3_core::physics::board_step::ATTACHED_REACTION_BASE + index) else { continue };
            self.dynamics_deltas.push((id,
                xyz(r.linear_displacement).map(|v| v / dt),
                xyz(r.angular_displacement).map(|v| v / dt),
                xyz(r.position_displacement), xyz(r.orientation_displacement),
            ));
        }
    }
}
