use super::reset_board_body;
use crate::physics::board_pose::{PartPose, PoseMatrix, set_board_transform, set_part_transform};
/// Complete 82C05F50 reset orchestration. `authored` supplies the seven matrices
/// produced by InitializeTransforms82C0ADF0 for the current stock geometry;
/// deck6 is published first, then wheels0..3 and trucks4..5. This explicit input
/// does not select a spawn or change joint anchors to a presumed settled pose.
///
/// `wrapper` is the native Skateboard prefix; `wrapper_base` only reconstructs
/// its three force-queue pointers. `body_state` is its SkateboardBody record.
/// Parts must have live bodies, as in the native seven-part board assembly.
/// Processed+2468 bit20 flips all four lanes of Ri and At before SetTransform.
/// The hook receives that target separately and its body rates are not reset.
#[allow(clippy::too_many_arguments)]
pub fn reset_skateboard(
    wrapper: &mut [u8; 1360],
    wrapper_base: u32,
    body_state: &mut [u8; 8400],
    parts: &mut [PartPose; 7],
    hook: &mut PartPose,
    authored: [PoseMatrix; 7],
    mut requested: PoseMatrix,
    processed_flags_2468: u32,
    gravity: &[[u32; 4]; 7],
) -> u32 {
    assert!(parts.iter().all(|part| part.body.is_some()));
    for i in [6, 0, 1, 2, 3, 4, 5] {
        set_part_transform(&mut parts[i], authored[i]);
    }
    if processed_flags_2468 & 0x0010_0000 != 0 {
        for i in [0, 1, 2, 3, 8, 9, 10, 11] {
            requested[i] ^= 0x8000_0000;
        }
    }
    set_board_transform(parts, hook, requested);
    let mut bodies = core::array::from_fn::<_, 7, _>(|i| parts[i].body.unwrap());
    reset_board_body(body_state, &mut bodies, gravity);
    for (part, body) in parts.iter_mut().zip(bodies) {
        part.body = Some(body);
    }
    put(wrapper, 256, 0);
    put(body_state, 7680, 0);
    put(body_state, 7684, 0);
    wrapper[240..256].fill(0);
    put(wrapper, 252, 0x3f80_0000);
    wrapper[290] = 0;
    body_state[960..992].fill(0);
    put(body_state, 972, 2);
    put(body_state, 988, 2);
    put(wrapper, 324, wrapper_base.wrapping_add(352));
    put(wrapper, 320, wrapper_base.wrapping_add(352));
    put(wrapper, 328, wrapper_base.wrapping_add(1360));
    put(wrapper, 260, 0);
    wrapper[192..208].fill(0);
    put(wrapper, 196, 0x3f80_0000);
    wrapper[291] = 0;
    240
}

fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
