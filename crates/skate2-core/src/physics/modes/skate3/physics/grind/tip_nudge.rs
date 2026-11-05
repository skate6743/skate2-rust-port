//! S3 tipslide dropping-in nudge82D421B0, separate from common exit nudge.
//! Scalar reconstruction of original operand flow; ordinary PC floating point,
//! not a claim of bit-exact VMX128 execution or an instruction emulator.
use super::{arithmetic::dot3, board::apply_world_force};
use skate3_core::physics::board_runtime::BoardRuntime;
type V = [f32; 4];

pub(super) fn apply(
    board: &mut BoardRuntime,
    position: V,
    point: V,
    across: V,
) -> Result<(), String> {
    let offset = core::array::from_fn(|i| position[i] - point[i]);
    let inward = across.map(|v| if dot3(across, offset) > 0. { -v } else { v });
    //82D422A0..C4 multiplies original820996DC=45 by8206D110=deg-to-rad,
    //then RELOADS that product before cosine82473930/sine824531C8.
    //Hex-Rays' stale scalar prints one degree; that is not the loaded value.
    let angle = 45.0_f32 * f32::from_bits(0x3c8e_fa35);
    let (sine, cosine) = skate3_core::trigonometry::sin_cos(angle);
    //Original v43 combines sine/cosine and v44 is zero. Shift/permutation
    //extract cosine for the lateral multiplier and sine for the Y-only subtract.
    //Do not rotate the full force vector or scale the Y subtract by100.
    let mut force = inward.map(|v| (v * 100.) * cosine);
    force[1] -= sine;
    apply_world_force(board, force, position);
    Ok(())
}
