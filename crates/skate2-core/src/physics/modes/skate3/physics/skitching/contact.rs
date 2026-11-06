//! Skate 3 Skitching push and brake contact on the grabbed object (no Skate 2 twins):
//! `sub_82D47BD8` and `sub_82D479C8`, run by Update 0x82d47900/0x82d47908 after
//! UpdateDeckAnimOffset. Each force goes to `ApplyForceToGrabSplineObjectDeferred` (grab
//! query manager vtable+0x24), which only moves dynamic objects.
use super::{
    Skitching, Vector, ZERO,
    skitch_state::{SkitchError, need},
};
use crate::physical_player_grind_manager::math::{GrindNum, cross, fsel, neg, scale, sub};

/// One deferred grab-spline force: world force, local position and torque.
pub(crate) struct SkitchContactForce {
    pub(crate) force: Vector,
    pub(crate) position: Vector,
    pub(crate) torque: Vector,
}

fn clamp_length(v: Vector, maximum: f32, n: &mut GrindNum<'_>) -> Result<Vector, SkitchError> {
    crate::reckoning_numeric::try_clamp(v, maximum, n.n)
        .map_err(crate::physical_player_grind_manager::view::error)
        .map_err(SkitchError::Geometry)
}

impl Skitching {
    /// `sub_82D47BD8`. Until the PushContact record (Processed+0x9B8 0x2000_0000) latches
    /// +0x541 0x10, the target speed (+0x3E4) is the spline ground speed plus the smaller of
    /// max(8 - speed, 0) and min(200 * inverse mass, 3.5). While latched, the push is the
    /// grab-spline forward axis times 60 times the forward speed error toward min(target, 8)
    /// clamped to [0, 1.5], at most 50 long, with the Y torque of -(body - Processed+0xB0) x
    /// Processed+0xA0 clamped to 1; releasing the record unlatches after the push. Unlatched,
    /// the first 0.2 s in state push back the board speed force (+0x330) times the inverse mass.
    pub(crate) fn push_contact(
        &mut self,
        push_contact: bool,
        state_timer: f32,
        effective_position: Vector,
        effective_z: Vector,
        n: &mut GrindNum<'_>,
    ) -> Result<Option<SkitchContactForce>, SkitchError> {
        let spline = need(self.grab_spline.as_ref(), "Skitch spline")?;
        let inverse_mass = spline.inertia.inverse_mass;
        let body = spline.body_position;
        if !need(self.push_latched, "push latch")? {
            let speed = need(self.grab_spline_speed_ground, "spline speed")?;
            let mass = 1.0 / inverse_mass;
            let headroom = 8.0 - speed;
            let headroom = fsel(headroom, headroom, 0.0);
            let push = 200.0 / mass;
            let push = fsel(3.5 - push, push, 3.5);
            let push = fsel(headroom - push, push, headroom);
            self.push_target_speed = Some(push + speed);
            if push_contact {
                self.push_latched = Some(true);
            }
        }
        if need(self.push_latched, "push latch")? {
            let target = need(self.push_target_speed, "push target speed")?;
            let capped = fsel(8.0 - target, target, 8.0);
            let forward = need(self.grab_spline_current_space, "Skitch current space")?[2];
            let error = sub(
                scale(forward, capped),
                need(self.grab_spline_velocity, "spline velocity")?,
            );
            let along = n.dot(forward, error).map_err(SkitchError::Geometry)?;
            let gain = fsel(-along, 0.0, along);
            let gain = fsel(1.5 - gain, gain, 1.5);
            let force = clamp_length(scale(scale(forward, 60.0), gain), 50.0, n)?;
            let lever = sub(body, effective_position);
            let torque = clamp_length(scale(cross(lever, effective_z), -1.0), 1.0, n)?;
            // 0x82d47e40..e60: only the Y lane of the torque is kept.
            let torque = [0.0, torque[1], 0.0, torque[3]];
            if !push_contact {
                self.push_latched = Some(false);
            }
            return Ok(Some(SkitchContactForce {
                force,
                position: ZERO,
                torque,
            }));
        }
        // 0x82d47ecc bge: an unordered timer also pushes back.
        if !(state_timer >= 0.2) {
            let speed_force = need(self.speed_force, "board speed force+330")?;
            return Ok(Some(SkitchContactForce {
                force: scale(scale(neg(speed_force), 1.0), inverse_mass),
                position: ZERO,
                torque: ZERO,
            }));
        }
        Ok(None)
    }

    /// `sub_82D479C8`. Clears +0x541 0x08; with a nonzero `Brake` record (Processed+0xAA8) and
    /// a spline speed above 2^-16, the grabbed object is slowed by min(speed, min(50 * inverse
    /// mass, 0.5)) along its velocity, times 25, and +0x541 0x08 (FillPhysOut State+0x35) sets.
    pub(crate) fn brake_contact(
        &mut self,
        brake: f32,
        n: &mut GrindNum<'_>,
    ) -> Result<Option<SkitchContactForce>, SkitchError> {
        self.braking = Some(false);
        // 0x82d479fc beq: an unordered record also brakes.
        if brake == 0.0 {
            return Ok(None);
        }
        let velocity = need(self.grab_spline_velocity, "spline velocity")?;
        let inverse_mass = need(self.grab_spline.as_ref(), "Skitch spline")?
            .inertia
            .inverse_mass;
        let speed = n.length(velocity).map_err(SkitchError::Geometry)?;
        let limit = 50.0 / (1.0 / inverse_mass);
        let limit = fsel(0.5 - limit, limit, 0.5);
        // 0x82d47a9c ble: an unordered speed also brakes.
        if speed <= f32::from_bits(0x3780_0000) {
            return Ok(None);
        }
        let step = fsel(speed - limit, limit, speed);
        let inverse = n.reciprocal(speed).map_err(SkitchError::Geometry)?;
        let change = scale(scale(neg(velocity), step), inverse);
        self.braking = Some(true);
        Ok(Some(SkitchContactForce {
            force: scale(change, 25.0),
            position: ZERO,
            torque: ZERO,
        }))
    }
}
