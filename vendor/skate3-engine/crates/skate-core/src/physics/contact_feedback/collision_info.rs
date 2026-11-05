use super::CollisionInfo;

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

/// Complete 82C00CA0 selective per-frame reset. Wheel surface/audio/object
/// slots and line-test state survive; this differs from startup 82C00D68.
pub fn reset_frame_collision_info(info: &mut CollisionInfo) {
    up(info, 0);
    for i in 0..7 {
        up(info, 32 + i * 16);
        for offset in [144, 368, 480] {
            info[offset + i * 16..offset + i * 16 + 16].fill(0);
        }
        info[780 + i] = 0;
    }
    for i in 4..7 {
        for offset in [692, 720, 748] {
            put(info, offset + i * 4, 0);
        }
    }
    info[592..608].fill(0);
    info[688..692].fill(1);
    for offset in [776, 788, 796, 800] {
        put(info, offset, 0);
    }
    info[804] = 0;
    info[805] = 0;
    let flags = word(info, 808) & 0x01ff_ffff;
    put(info, 808, flags);
}

/// Complete 82C08818 surface vote. Noncontacting wheels still contribute one
/// vote; contacting wheels contribute four. Equal counts favor the lower ID.
/// IDs14/15 are accumulated but not considered by the native winner scan.
pub fn choose_physics_surface(info: &mut CollisionInfo) {
    let surfaces = std::array::from_fn(|i| word(info, 692 + i * 4));
    let contacts = std::array::from_fn(|i| info[780 + i] != 0);
    let result = choose_surface(surfaces, contacts, word(info, 808) & 0x0200_0000 != 0);
    put(info, 776, result);
}

/// Typed82C08818 boundary. Values are the retained wheel surface outputs;
/// query misses and contact replacements belong to their original producers.
pub fn choose_surface(surfaces: [u32; 4], contacts: [bool; 4], forced_twelve: bool) -> u32 {
    let mut votes = [0u32; 16];
    for i in 0..4 {
        let surface = surfaces[i] as usize;
        if surface != 0 {
            assert!(surface < 16, "native surface histogram index exceeded");
            votes[surface] += if contacts[i] { 4 } else { 1 };
        }
    }
    let mut winner = 1;
    let mut maximum = 0;
    for (surface, &count) in votes.iter().enumerate().take(14).skip(1) {
        if count > maximum {
            maximum = count;
            winner = surface as u32;
        }
    }
    if forced_twelve { 12 } else { winner }
}
