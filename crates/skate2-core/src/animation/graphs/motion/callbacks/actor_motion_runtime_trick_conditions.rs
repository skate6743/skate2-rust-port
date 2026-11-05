//! Actual retained-owner reads for Motion conditions140,141,138.
use super::{ActorConstruction, ActorIdentity, BoundMotionGraphContext, MotionUpdateError};
use crate::actor_graph_construction::ActorGraphKnownBits;

fn allow_trick(flags: ActorGraphKnownBits) -> Result<bool, MotionUpdateError> {
    const MASK: u32 = 0x0200_0000;
    if flags.known_mask & MASK == 0 {
        return Err(MotionUpdateError::MissingOwner("Motion AllowTrick bit"));
    }
    Ok(flags.bits & MASK != 0)
}

impl ActorConstruction {
    /// Condition82c9d0e8 resolves Motion before virtual+74, `AllowTrick`825fdff0.
    pub(crate) fn motion_runtime_allowed_to_trick(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let actor = self.prefix().map_err(E::Actor)?;
        let state = actor
            .graphs
            .as_ref()
            .and_then(|graphs| graphs.motion())
            .and_then(|motion| motion.state.as_ref())
            .ok_or(E::MissingOwner("Motion state"))?;
        allow_trick(state.flags)
    }

    /// Condition82c9bfd0 consumes actual `PhysOut.Animation`+a2, without a latch read.
    pub(crate) fn motion_runtime_ok_to_trick_on_stairs(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(!output
            .try_animation_output()
            .map_err(E::PhysOut)?
            .should_disallow_tricks_due_to_stairs)
    }

    /// Condition82c9aef0 consumes actual `PhysOut.OffBoard`+38, without a latch read.
    pub(crate) fn motion_runtime_is_grabbing_object(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
        Ok(output
            .try_offboard_output()
            .map_err(E::PhysOut)?
            .is_grabbing_object)
    }

    /// IsMovingObject.Activation82c9b808: actual OffBoard movement, not intent.
    pub(crate) fn motion_runtime_is_moving_object(
        &self,
        context: &BoundMotionGraphContext<ActorIdentity>,
        angle_start: f32,
        angle_end: f32,
    ) -> Result<bool, MotionUpdateError> {
        use MotionUpdateError as E;
        self.check_motion_runtime_context(context)?;
        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        let (x, z) = {
            let output = physical.owner().phys_out().lock().map_err(|_| E::Lock)?;
            let offboard = output.try_offboard_output().map_err(E::PhysOut)?;
            (offboard.anim_obj_mov_x, offboard.anim_obj_mov_z)
        };
        moving_object_in_sector(
            x,
            z,
            angle_start,
            angle_end,
            &mut crate::reckoning_numeric::HostReckoningNumeric,
        )
        .map_err(E::ConditionNumeric)
    }
}

/// Scalar FMULS/FMADDS length prefix, then the same one-refinement ratio and
/// XMVectorATan/quadrant graph already used by the camera. Both numeric producers
/// execute even when movement is below the threshold (including zero Z).
/// TU3's IsMovingObject::ActivationFunc 0x82ba6038 is the same graph.
pub(crate) fn moving_object_in_sector<N: crate::reckoning_numeric::ReckoningNumeric>(
    x: f32,
    z: f32,
    start: f32,
    end: f32,
    numeric: &mut N,
) -> Result<bool, N::Error> {
    let squared = x.mul_add(x, z * z);
    let seed = numeric.reciprocal_sqrt_estimate(squared)?;
    let inverse = crate::simulation_drive_jacobian::refine_drive_reciprocal_sqrt(squared, seed);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    let angle = crate::camera_compass::runtime::math::camera_atan2_pending(-x, z, numeric)?.finish();
    Ok(moving_object_sector_result(length, angle, start, end))
}

#[expect(
    clippy::neg_cmp_op_on_partial_ord,
    reason = "preserve PPC unordered branch outcomes"
)]
fn moving_object_sector_result(length: f32, angle: f32, start: f32, end: f32) -> bool {
    if length <= f32::from_bits(0x3dcc_cccd) {
        return false;
    }
    // Raw branches82c9ba44/4c/54/70/7c, rather than HexRays' ordered shorthand.
    if !(start >= end) {
        !(angle < start) && angle <= end
    } else {
        angle >= start || !(angle > end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_object_sectors_keep_threshold_and_inclusive_wrapped_edges() {
        let pi = f32::from_bits(0x4049_0fdb);
        assert!(!moving_object_sector_result(0.1, 0.0, -pi, pi));
        assert!(moving_object_sector_result(
            f32::from_bits(0x3dcc_ccce),
            0.0,
            -pi,
            pi
        ));
        for angle in [-pi, -2.0, 2.0, pi] {
            assert!(moving_object_sector_result(1.0, angle, 2.0, -2.0));
        }
        assert!(!moving_object_sector_result(1.0, 0.0, 2.0, -2.0));
        assert!(moving_object_sector_result(1.0, 0.0, -2.0, 0.0));
        assert!(moving_object_sector_result(1.0, 0.0, 0.0, 2.0));
        // Equal endpoints select the native wrap branch, not an empty sector.
        assert!(moving_object_sector_result(1.0, 1.0, 0.0, 0.0));
    }

    #[test]
    fn moving_object_raw_branches_preserve_unordered_results() {
        assert!(moving_object_sector_result(f32::NAN, 0.0, -1.0, 1.0));
        assert!(!moving_object_sector_result(1.0, f32::NAN, -1.0, 1.0));
        assert!(moving_object_sector_result(1.0, f32::NAN, 1.0, -1.0));
        assert!(moving_object_sector_result(1.0, 0.0, f32::NAN, 1.0));
    }

    #[test]
    fn moving_object_uses_negative_x_over_z_and_handles_zero_denominator() {
        use crate::reckoning_numeric::HostReckoningNumeric;
        for (x, z, start, end) in [
            (1.0, 0.0, -1.6, -1.5),
            (-1.0, 0.0, 1.5, 1.6),
            (0.0, 1.0, -0.1, 0.1),
            (0.0, -1.0, 3.0, -3.0),
        ] {
            assert!(moving_object_in_sector(x, z, start, end, &mut HostReckoningNumeric).unwrap());
        }
        assert!(!moving_object_in_sector(0.0, 0.0, -4.0, 4.0, &mut HostReckoningNumeric).unwrap());
    }

    #[test]
    fn unrelated_known_bits_do_not_establish_allow_trick() {
        assert!(matches!(
            allow_trick(ActorGraphKnownBits {
                known_mask: 0xfdff_ffff,
                bits: u32::MAX,
            }),
            Err(MotionUpdateError::MissingOwner("Motion AllowTrick bit"))
        ));
    }

    #[test]
    fn constructor_mask_and_later_clear_are_both_observed() {
        assert!(
            allow_trick(ActorGraphKnownBits {
                known_mask: 0xff00_0000,
                bits: 0x0300_0000,
            })
            .unwrap()
        );
        assert!(
            !allow_trick(ActorGraphKnownBits {
                known_mask: 0x0200_0000,
                bits: 0xfdff_ffff,
            })
            .unwrap()
        );
    }
}
