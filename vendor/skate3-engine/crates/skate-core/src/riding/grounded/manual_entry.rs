//! Ground's manual entry slice82D376F4..7750 and velocity helper82D37960.
//! Original-image bytes confirm the complete entry-helper branch/write order.
//! The game adapter supplies live board bodies and the shared dot calculation.
use crate::physics::manual::state::{ManualEntryContinuation, ManualState};

#[derive(Clone, Copy, Debug)]
pub struct ManualGroundInput {
    /// ProcessedPhysIn+2720; either signed zero bypasses every body operation.
    pub balance: f32,
    /// ProcessedPhysIn+464, used directly without normalization.
    pub ground_normal: [f32; 4],
    /// ProcessedPhysIn+2468. Bit20 swaps the two contact-controlled groups.
    pub flags_2468: u32,
    /// ProcessedPhysIn+2472. Bits26/27 select the two groups.
    pub flags_2472: u32,
}

/// Direct part-body velocity access, not a force queue or delayed write list.
/// Resolve the live board assembly part on each call: native reads follow every
/// earlier write, including when multiple parts happen to refer to one body.
pub trait ManualGroundBodies {
    /// Part indices0..6 resolve body pointers at assembly+76+96*part; velocity
    /// is the four-component body field+32. No angular velocity is touched.
    fn linear_velocity(&mut self, part: usize) -> [f32; 4];
    fn set_linear_velocity(&mut self, part: usize, velocity: [f32; 4]);
}

/// Three-component projection supplied by the host's shared numerical adapter.
pub trait ManualGroundProjection {
    type Error;
    /// Native three-component projection at82D3799C (and six repeated sites).
    /// W does not contribute. Independent hardware numerical parity of the
    /// shared dot implementation is separate from this helper's control flow.
    fn normal_speed(
        &mut self,
        ground_normal: [f32; 4],
        linear_velocity: [f32; 4],
    ) -> Result<f32, Self::Error>;
}

/// Performs the actual manual-state reset/scaling, then its required velocity
/// continuation before returning to Ground Enter's subsequent category200 work.
/// An error retains the executed state/body prefix and must abort the tick.
pub fn enter_ground<B: ManualGroundBodies, P: ManualGroundProjection>(
    state: &mut ManualState,
    previous_category: u32,
    powerslide_exit_scale: f32,
    input: ManualGroundInput,
    bodies: &mut B,
    projection: &mut P,
) -> Result<(), P::Error> {
    if state.enter_ground(previous_category, powerslide_exit_scale)
        == ManualEntryContinuation::RemoveVelocityIntoGround
    {
        remove_velocity_into_ground(input, bodies, projection)?;
    }
    Ok(())
}

/// Full branch/write order of82D37960, conditional on the required projection.
/// Despite the recovered name, both signs of normal motion are removed. There
/// is no clamp, normal-length division, timestep or balance-sign selection.
pub fn remove_velocity_into_ground<B: ManualGroundBodies, P: ManualGroundProjection>(
    input: ManualGroundInput,
    bodies: &mut B,
    projection: &mut P,
) -> Result<(), P::Error> {
    if input.balance == 0.0 {
        return Ok(());
    }
    remove_part(6, input.ground_normal, bodies, projection)?;

    // Native flags are read after the deck write82D379B4..B8.
    let reversed = input.flags_2468 & (1 << 20) != 0;
    let contact26 = input.flags_2472 & (1 << 26) != 0;
    let contact27 = input.flags_2472 & (1 << 27) != 0;
    let (first_group, second_group) = if reversed {
        (contact26, contact27)
    } else {
        (contact27, contact26)
    };
    if first_group {
        for part in [0, 1, 4] {
            remove_part(part, input.ground_normal, bodies, projection)?;
        }
    }
    if second_group {
        for part in [2, 3, 5] {
            remove_part(part, input.ground_normal, bodies, projection)?;
        }
    }
    Ok(())
}

fn remove_part<B: ManualGroundBodies, P: ManualGroundProjection>(
    part: usize,
    ground_normal: [f32; 4],
    bodies: &mut B,
    projection: &mut P,
) -> Result<(), P::Error> {
    let velocity = bodies.linear_velocity(part);
    let normal_speed = projection.normal_speed(ground_normal, velocity)?;
    let mut remaining = velocity;
    for axis in 0..3 {
        // Separate multiply then subtract82D379A4..A8, not a fused operation.
        let component = ground_normal[axis] * normal_speed;
        remaining[axis] = velocity[axis] - component;
    }
    // W is retained bit-for-bit by the native insert before each store.
    bodies.set_linear_velocity(part, remaining);
    Ok(())
}
