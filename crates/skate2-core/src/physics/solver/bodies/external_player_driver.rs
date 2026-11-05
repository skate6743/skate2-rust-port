//! PC mod ownership boundary. The original body state/list membership stays
//! intact while an alternate solver owns an occurrence-qualified set of bodies.
use super::{
    SimulationRigidBodyDynamicUpdatePhase, SimulationRigidBodyObservation, SimulationRigidBodyPool,
    SimulationRigidBodyPoolError,
};

impl SimulationRigidBodyPool {
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_external_principal_inertia(
        &self,
        body: SimulationRigidBodyObservation,
    ) -> Result<[f32; 3], String> {
        let index = self.validate_observation(body).map_err(|e| e.to_string())?;
        self.slots[index]
            .inertia
            .as_ref()
            .map(|inertia| inertia.try_read().map(|value| value.inverse_tensor))
            .transpose()
            .map(|value| value.unwrap_or([0.; 3]))
            .map_err(|e| e.to_string())
    }

    /// The source solver already integrated its own player. This applies only
    /// its returned velocity and split-impulse corrections to the host body.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_apply_external_reaction(
        &mut self,
        body: SimulationRigidBodyObservation,
        reaction: crate::skate3_native::BodyReaction,
    ) -> Result<(), String> {
        use crate::skate3_native::rapier3d::prelude::{Rotation, Vector};
        let index = self.validate_observation(body).map_err(|e| e.to_string())?;
        if self.is_externally_driven(body) {
            return Err("World reaction targeted the player mirror".into());
        }
        if reaction
            .linear_velocity_delta
            .iter()
            .chain(&reaction.angular_velocity_delta)
            .chain(&reaction.position_delta)
            .chain(&reaction.orientation_delta)
            .any(|v| !v.is_finite())
        {
            return Err("Non-finite Skate 3 world reaction".into());
        }
        // The captured solver emits a row for every shadow, including bodies
        // with no contact response. A zero row must not thaw an untouched
        // original-world object or reset its sleep timer.
        if reaction
            .linear_velocity_delta
            .iter()
            .chain(&reaction.angular_velocity_delta)
            .chain(&reaction.position_delta)
            .chain(&reaction.orientation_delta)
            .all(|v| *v == 0.)
        {
            return Ok(());
        }
        let snapshot = self
            .try_drive_build_snapshot(body)
            .map_err(|e| e.to_string())?;
        if snapshot.state & 7 == 1 || snapshot.inverse_mass == 0. {
            return Ok(());
        }
        if snapshot.state & 7 == 2 {
            self.try_activate_frozen_body(body)
                .map_err(|e| e.to_string())?;
        }
        self.try_clear_body_cooldown(body)
            .map_err(|e| e.to_string())?;
        let q = snapshot.quaternion;
        let rotation = (Rotation::from_scaled_axis(Vector::from_array(reaction.orientation_delta))
            * Rotation::from_xyzw(q[0], q[1], q[2], q[3]))
        .normalize();
        let right = rotation * Vector::X;
        let up = rotation * Vector::Y;
        let at = rotation * Vector::Z;
        let p: [f32; 3] = std::array::from_fn(|axis| {
            snapshot.center_of_mass[axis] + reaction.position_delta[axis]
        });
        let transform = [
            right.x, right.y, right.z, 0., up.x, up.y, up.z, 0., at.x, at.y, at.z, 0., p[0], p[1],
            p[2], 1.,
        ];
        let handle = super::SimulationRigidBodyHandle {
            authority: body.authority,
            id: body.id,
            occurrence: body.occurrence,
        };
        self.try_set_part_body_transform_host(handle, transform)
            .map_err(|e| e.to_string())?;
        let velocity = std::array::from_fn(|axis| {
            snapshot.velocity[axis] + reaction.linear_velocity_delta[axis]
        });
        let omega = std::array::from_fn(|axis| {
            snapshot.omega[axis] + reaction.angular_velocity_delta[axis]
        });
        self.try_store_post_solve_linear_velocity(body, velocity)
            .map_err(|e| e.to_string())?;
        self.try_store_post_solve_angular_velocity(body, omega)
            .map_err(|e| e.to_string())?;
        // The occurrence/ring and RF solver rows are retained by every store.
        debug_assert_eq!(self.slots[index].occurrence, Some(body.occurrence));
        Ok(())
    }

    /// Publish one completed mode frame for read-only world/render/replay
    /// consumers. This does not mutate any native Skate 2 solver body slot.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_publish_mode_body_views(
        &mut self,
        values: &[(SimulationRigidBodyObservation, [f32;16], [f32;3], [f32;3])],
    ) -> Result<(), String> {
        use super::{RetainedInertia, orthonormalize_collision_affine,
            simulation_quaternion_from_orthonormalized_matrix_host,
            simulation_rotated_inverse_inertia_host};
        let mut views = Vec::with_capacity(values.len());
        for &(body, transform, velocity, omega) in values {
            let index = self.validate_observation(body).map_err(|e|e.to_string())?;
            if !self.is_externally_driven(body) { return Err("Mode publication requires its own physical occurrence".into()); }
            if views.iter().any(|view: &ModeBodyView| view.body == body) { return Err("Duplicate mode body publication".into()); }
            if transform.iter().chain(&velocity).chain(&omega).any(|v| !v.is_finite()) {
                return Err("Nonfinite mode body publication".into());
            }
            let transform = orthonormalize_collision_affine(transform);
            let slot = &self.slots[index];
            let inertia = slot.inertia.as_ref().map(RetainedInertia::try_read).transpose().map_err(|e|e.to_string())?;
            let mut view = ModeBodyView { body, transform, velocity, omega,
                quaternion: simulation_quaternion_from_orthonormalized_matrix_host(transform),
                inverse_inertia_full: slot.inverse_inertia_full,
                inverse_inertia_split: slot.inverse_inertia_split, inverse_mass: slot.inverse_mass };
            if let Some(inertia) = inertia {
                let rotated = simulation_rotated_inverse_inertia_host(transform, inertia.inverse_tensor, inertia.inverse_mass);
                view.inverse_inertia_full = rotated.full; view.inverse_inertia_split = rotated.split; view.inverse_mass = rotated.inverse_mass;
            }
            views.push(view);
        }
        if views.len() != self.externally_driven.len() { return Err("Incomplete mode body publication".into()); }
        self.mode_body_views = views;
        Ok(())
    }
    pub(super) fn mode_body_view(&self, body: SimulationRigidBodyObservation) -> Option<&ModeBodyView> {
        self.mode_body_views.iter().find(|view| view.body == body)
    }

    /// Return the completed mass-frame poses once when native S2 regains this
    /// allocation. Its reset spans ordinary animation/physics phases; leaving
    /// the old dormant body poses here would show one frame at the old position.
    /// Every ordinary frame after this is integrated by the S2 solver itself.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_return_external_player(&mut self) -> Result<(), String> {
        self.validate_integrity().map_err(|error|error.to_string())?;
        if self.mode_body_views.len() != self.externally_driven.len() {
            return Err("Incomplete native solver return".into());
        }
        let mut writes = Vec::with_capacity(self.mode_body_views.len());
        for view in &self.mode_body_views {
            if !self.is_externally_driven(view.body) { return Err("Foreign native solver return".into()); }
            let index = self.validate_observation(view.body).map_err(|error|error.to_string())?;
            writes.push((index, *view));
        }
        // These are the original S2 mass-frame publications, including its
        // authored inertia and inverse mass, prepared by publish_mode_body_views.
        for (index, view) in writes {
            let slot = &mut self.slots[index];
            slot.right = view.axis(0); slot.up = view.axis(1); slot.at = view.axis(2);
            slot.center_of_mass = view.center_of_mass(); slot.quaternion = view.quaternion;
            slot.velocity = view.velocity; slot.omega = view.omega;
            slot.inverse_inertia_full = view.inverse_inertia_full;
            slot.inverse_inertia_split = view.inverse_inertia_split; slot.inverse_mass = view.inverse_mass;
        }
        let bodies = std::mem::take(&mut self.externally_driven);
        self.mode_body_views.clear();
        self.frame_identity = None;
        for body in bodies {
            let index = self.validate_observation(body).map_err(|error|error.to_string())?;
            if self.slots[index].state & 7 == 2 {
                self.try_activate_frozen_body(body).map_err(|error|error.to_string())?;
            }
            self.try_clear_body_cooldown(body).map_err(|error|error.to_string())?;
        }
        Ok(())
    }

    /// A single boundary transfer after the original body reset. Validate every
    /// allocation occurrence and transport the rigid velocity to each S2 COM
    /// before making any stores. These are ordinary pre-solve writes.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn try_apply_mode_exit_rates(
        &mut self,
        rates: &[crate::skate3_native::physics::player::exit::BodyRate],
    ) -> Result<(), String> {
        self.validate_integrity().map_err(|error|error.to_string())?;
        let mut writes = Vec::with_capacity(rates.len());
        for rate in rates {
            if self.is_externally_driven(rate.body) { return Err("Native exit still owned by Skate 3".into()); }
            if writes.iter().any(|(body, _, _)| *body == rate.body) { return Err("Duplicate native exit body".into()); }
            let target = self.try_drive_build_snapshot(rate.body).map_err(|error|error.to_string())?;
            let velocity = rate.at(target.center_of_mass)?;
            writes.push((rate.body, velocity, rate.omega));
        }
        for (body, velocity, omega) in writes {
            self.try_set_part_linear_velocity(body, velocity).map_err(|error|error.to_string())?;
            self.try_set_part_angular_velocity(body, omega).map_err(|error|error.to_string())?;
        }
        Ok(())
    }

    /// Validate the complete replacement before changing ownership. Native
    /// authority, allocation occurrence, and state remain validated normally.
    pub(crate) fn try_set_externally_driven(
        &mut self,
        bodies: &[SimulationRigidBodyObservation],
    ) -> Result<(), SimulationRigidBodyPoolError> {
        for body in bodies {
            self.try_validate_observation(*body)?;
        }
        self.mode_body_views.retain(|view| bodies.contains(&view.body));
        self.externally_driven = bodies.to_vec();
        Ok(())
    }

    pub(crate) fn is_externally_driven(&self, body: SimulationRigidBodyObservation) -> bool {
        self.externally_driven.contains(&body)
    }
}

impl SimulationRigidBodyDynamicUpdatePhase<'_> {
    pub(crate) fn is_externally_driven(&self, body: SimulationRigidBodyObservation) -> bool {
        self.pool.is_externally_driven(body)
    }
}

/// A completed publication, not another physical body or solver allocation.
#[derive(Clone, Copy, Debug)]
pub(super) struct ModeBodyView {
    pub body: SimulationRigidBodyObservation,
    pub transform: [f32;16], pub velocity: [f32;3], pub omega: [f32;3], pub quaternion: [f32;4],
    pub inverse_inertia_full: [f32;3], pub inverse_inertia_split: [f32;3], pub inverse_mass: f32,
}
impl ModeBodyView {
    pub fn center_of_mass(&self) -> [f32;3] { [self.transform[12], self.transform[13], self.transform[14]] }
    pub fn axis(&self, axis: usize) -> [f32;3] { std::array::from_fn(|row|self.transform[axis*4+row]) }
}
