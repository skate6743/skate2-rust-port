//! Complete 82C0D680 and its 82C00D68 dependency, guest big-endian record bytes.

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}
fn up(bytes: &mut [u8], offset: usize) {
    bytes[offset..offset + 16].fill(0);
    put(bytes, offset + 4, 0x3f80_0000);
}

/// Complete CollisionInfo initialization/reset at 82C00D68. This is distinct
/// from the selective per-frame reset at 82C00CA0. Unwritten bytes survive.
pub fn reset_collision_info(info: &mut [u8; 812]) {
    up(info, 0);
    up(info, 16);
    for i in 0..7 {
        up(info, 32 + i * 16);
        for offset in [144, 256, 368, 480] {
            info[offset + i * 16..offset + i * 16 + 16].fill(0);
        }
        for offset in [692, 720, 748] {
            put(info, offset + i * 4, 0);
        }
        info[780 + i] = 0;
    }
    info[592..608].fill(0);
    for i in 0..4 {
        up(info, 608 + i * 16);
        put(info, 672 + i * 4, 0);
        info[688 + i] = 0;
    }
    for offset in [776, 788, 792, 796, 800] {
        put(info, offset, 0);
    }
    info[804] = 0;
    info[805] = 0;
    let flags = word(info, 808) & 0x01ff_ffff;
    put(info, 808, flags);
}

/// Complete SkateboardBody reset at 82C0D680. `bodies` are assembly parts in
/// their native order; each gravity vector is read from that body's Simulation
/// at +144. The hook assembly is separate and is not traversed here.
///
/// XYZ rates/torque clear; force XYZ receives the simulation vector. Fourth
/// lanes, pose, body flags/cooldown and all other body words survive. Ancillary
/// board bytes are preserved except for stores present in the native function.
/// Returns the original byte at board+8384, matching native r3.
pub fn reset_board_body(
    board: &mut [u8; 8400],
    bodies: &mut [[u32; 44]],
    simulation_gravity: &[[u32; 4]],
) -> u8 {
    assert_eq!(bodies.len(), simulation_gravity.len());
    for (body, gravity) in bodies.iter_mut().zip(simulation_gravity) {
        body[8..11].fill(0);
        body[12..15].fill(0);
        body[36..39].copy_from_slice(&gravity[..3]);
        body[40..43].fill(0);
    }
    put(board, 7680, 0);
    put(board, 7684, 0);
    reset_collision_info((&mut board[64..876]).try_into().unwrap());
    let old_flags = board[8384];
    board[8384] &= 0x3f;
    for offset in [7968, 8032] {
        board[offset..offset + 52].fill(0);
        board[offset + 52] = 0;
    }
    board[1008..1044].fill(0);
    board[1044] = 0;
    old_flags
}
