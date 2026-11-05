use crate::input::animation_packet::{self, ProcessedPacketFields};

use super::types::{
    AnimationInputPacket, NativeReferenceBase, NativeRelativeReference, PhysicalPlayerInput,
    PlayerInputState, ProcessedPhysicsInput, RawMatrix,
};

pub(crate) fn publish_player_flags(player: &mut PlayerInputState, out: &mut ProcessedPhysicsInput) {
    transfer_bit(&mut out.flags_2472, 14, player.flags_1296, 27);
    transfer_bit(&mut out.flags_2488, 26, player.flags_1296, 20);
    out.vectors_880_896_912_928_944[3] = player.queued_vector_1280;
    transfer_bit(&mut out.flags_2488, 22, player.flags_1296, 18);
    transfer_bit(&mut out.flags_2480, 16, player.flags_1296, 17);
    player.queued_vector_1280 = [0; 4];
    player.flags_1296 &= !(1 << 20);
}

pub(crate) fn publish_force_braking(
    player: &PlayerInputState,
    physical: &PhysicalPlayerInput,
    packet: &AnimationInputPacket<'_>,
    out: &mut ProcessedPhysicsInput,
) {
    let force = packet.force_braking_10796 != 0
        && (player.state_count_1312 as i32) > 5
        && physical.skateboard.scalar_160 < f32::from_bits(0x3dcc_cccd)
        && physical.state.state_16 == 100
        && (physical.collision.wheel_count_0 as i32) > 2;
    replace_bool(&mut out.flags_2476, 24, force);
}

pub(crate) fn publish_collision_prefix(
    player: &PlayerInputState,
    physical: &PhysicalPlayerInput,
    out: &mut ProcessedPhysicsInput,
) {
    replace_byte(&mut out.flags_2484, 26, physical.collision.flag_215);
    replace_byte(&mut out.flags_2484, 22, physical.collision.flag_216);
    out.vectors_720_784_800_816_832_864[5] = player.manager_1852_vector_176;
}

pub(crate) fn publish_animation_packet(
    packet: &AnimationInputPacket<'_>,
    out: &mut ProcessedPhysicsInput,
) {
    let mut fields = ProcessedPacketFields {
        flags_2468: out.flags_2468,
        flags_2476: out.flags_2476,
        timestep: out.timestep_2604,
        scalar_2668: out.scalar_2668,
        vector_1520: out.vector_1520,
        matrix_1536: out.matrix_1536,
        byte_1600: out.byte_1600,
        truck_tightness: out.truck_tightness_2760,
        scalar_2764: out.scalar_2764,
    };
    animation_packet::publish(packet.publication, &mut fields);
    out.flags_2468 = fields.flags_2468;
    out.flags_2476 = fields.flags_2476;
    out.timestep_2604 = fields.timestep;
    out.scalar_2668 = fields.scalar_2668;
    out.vector_1520 = fields.vector_1520;
    out.matrix_1536 = fields.matrix_1536;
    out.byte_1600 = fields.byte_1600;
    out.truck_tightness_2760 = fields.truck_tightness;
    out.scalar_2764 = fields.scalar_2764;
}

pub(crate) fn select_surface(
    player: &PlayerInputState,
    packet: &AnimationInputPacket<'_>,
    surface_default_mode: u32,
    out: &mut ProcessedPhysicsInput,
) -> Result<(), u32> {
    let index = packet.state_variant_10928;
    let Some(variant) = player.state_variants_1408.get(index as usize) else {
        out.state_variant_index_2528 = index;
        return Err(index);
    };
    out.state_variant_ref_2548 = Some(NativeRelativeReference {
        base: NativeReferenceBase::Player,
        offset: 0x580 + 0x10 * index as u16,
    });
    out.state_variant_index_2528 = index;

    let requested = if variant.surface_override_enabled_60 != 0 {
        match surface_default_mode {
            1..=3 => 1,
            5 => 3,
            13 => 2,
            value => value,
        }
    } else {
        surface_default_mode
    };
    select_surface_mode(requested, out);
    Ok(())
}

fn select_surface_mode(requested: u32, out: &mut ProcessedPhysicsInput) {
    let mode = match requested {
        1..=5 => requested,
        6 => 3,
        _ => 1,
    };
    let (primary, secondary) = match mode {
        1 => (24, 104),
        2 => (40, 116),
        3 => (56, 128),
        4 => (72, 140),
        5 => (88, 152),
        _ => unreachable!("surface mode normalization is exhaustive"),
    };
    out.surface_mode_2540 = mode;
    out.surface_primary_ref_2544 = Some(NativeRelativeReference {
        base: NativeReferenceBase::SurfaceSelector,
        offset: primary,
    });
    out.surface_secondary_ref_2552 = Some(NativeRelativeReference {
        base: NativeReferenceBase::SurfaceSelector,
        offset: secondary,
    });
}

pub(crate) fn publish_external_physics(
    player: &mut PlayerInputState,
    packet: &AnimationInputPacket<'_>,
    out: &mut ProcessedPhysicsInput,
) {
    if packet.use_external_physics_10688 != 0 {
        out.flags_2472 |= 1 << 29;
        out.external_physics_1616
            .copy_from(packet.external_physics_10512);
        if out.external_physics_1616.flags & (1 << 26) != 0 {
            player
                .external_physics_cache_1008
                .copy_from(packet.external_physics_10512);
        } else {
            out.external_physics_1616.vectors[5..9]
                .copy_from_slice(&player.external_physics_cache_1008.vectors[5..9]);
            out.external_physics_1616.vectors[9] = player.external_physics_cache_1008.vectors[9];
            transfer_bit(
                &mut out.external_physics_1616.flags,
                27,
                player.external_physics_cache_1008.flags,
                27,
            );
            out.external_physics_1616.flags |= 1 << 26;
        }
    }
    replace_byte(&mut out.flags_2472, 28, packet.external_physics_flag_10689);
}

pub(crate) fn publish_state_prefix(
    player: &mut PlayerInputState,
    physical: &PhysicalPlayerInput,
    packet: &AnimationInputPacket<'_>,
    captured_state: u32,
    out: &mut ProcessedPhysicsInput,
) {
    transfer_bit(&mut out.flags_2468, 13, player.flags_1296, 31);
    let has_probe_or_animation_flag =
        out.flags_2468 & (1 << 2) != 0 || out.probe_1792.byte_104 != 0;
    replace_bool(&mut out.flags_2468, 18, has_probe_or_animation_flag);
    player.frames_since_teleport_1328 = if player.flags_1296 & (1 << 29) != 0 {
        0
    } else {
        player.frames_since_teleport_1328.wrapping_add(1)
    };
    out.frames_since_teleport_2584 = player.frames_since_teleport_1328;
    player.flags_1296 &= !(1 << 29);
    out.state_count_2564 = player.state_count_1312;
    out.state_timer_2664 = player.state_timer_1344;
    out.update_count_2568 = player.update_count_1316;
    replace_byte(&mut out.flags_2472, 12, packet.flag_10373);
    replace_byte(&mut out.flags_2468, 0, packet.flag_10786);
    replace_byte(&mut out.flags_2472, 31, packet.flag_10787);
    out.state_2508 = captured_state;
    out.state_2504 = captured_state;
    out.category_2512 = physical.state.category_12;
    replace_byte(&mut out.flags_2472, 30, physical.state.flag_61);
    replace_byte(&mut out.flags_2472, 18, physical.state.flag_69);
    replace_byte(&mut out.flags_2480, 31, physical.state.flag_74);
    out.grind_words_2532_2536 = physical.grinds.words_136_140;
    out.category_2516 = physical.state.category_12;
    out.player_state_value_2520 = player.state_value_1336;
}

pub(crate) fn publish_physical_outputs(
    physical: &PhysicalPlayerInput,
    packet: &AnimationInputPacket<'_>,
    out: &mut ProcessedPhysicsInput,
) {
    out.vectors_400_416[0] = physical.skateboard.vector_80;
    out.vectors_400_416[1] = physical.skateboard.vector_96;
    out.scalar_2736 = physical.skateboard.scalar_172;
    out.scalar_2612 = physical.skateboard.scalar_168;
    out.scalar_2656 = physical.skateboard.scalar_164;
    out.scalar_2652 = physical.skateboard.scalar_160;
    out.vectors_720_784_800_816_832_864[0] = physical.skateboard.vector_64;
    out.vectors_720_784_800_816_832_864[1] = physical.skateboard.vector_128;
    out.vectors_720_784_800_816_832_864[2] = physical.skateboard.vector_144;

    out.vectors_624_640_656_672[0] = packet.vector_10816;
    out.vectors_624_640_656_672[1] = packet.vector_10832;
    transfer_bit(&mut out.flags_2476, 6, packet.flags_10932, 31);
    transfer_bit(&mut out.flags_2476, 5, packet.flags_10932, 30);
    transfer_bit(&mut out.flags_2476, 4, packet.flags_10932, 29);
    out.scalar_2696 = packet.scalar_10912;
    out.vectors_624_640_656_672[2] = packet.vector_10848;
    out.scalar_2700 = packet.scalar_10916;
    out.vectors_624_640_656_672[3] = packet.vector_10864;
    replace_byte(&mut out.flags_2480, 0, packet.flag_10369);
    out.vectors_880_896_912_928_944[0] = packet.vector_10880;
    out.vectors_880_896_912_928_944[1] = packet.vector_10896;
    transfer_bit(&mut out.flags_2488, 24, packet.flags_10932, 22);

    out.vectors_544_560_592_608[2] = physical.reckoning.vector_64;
    out.vectors_880_896_912_928_944[4] = physical.reckoning.vector_144;
    replace_byte(&mut out.flags_2488, 21, physical.reckoning.flag_164);
    out.vectors_544_560_592_608[3] = if physical.air.use_air_reckoning_452 != 0 {
        physical.air.vector_160
    } else {
        physical.reckoning.vector_16
    };
    out.vectors_544_560_592_608[0] = physical.reckoning.vector_96;
    out.vectors_544_560_592_608[1] = physical.board_reckoning_side_176;
    out.vectors_464_480_496_512_528[0] = physical.ground.vector_96;
    out.vectors_464_480_496_512_528[4] = physical.ground.vector_64;
    out.vectors_720_784_800_816_832_864[4] = physical.ground.vector_128;
    replace_byte(&mut out.flags_2476, 27, physical.ground.flag_317);
    replace_byte(&mut out.flags_2476, 23, physical.ground.flag_318);

    out.air_scalar_2772 = physical.air.scalar_184;
    replace_byte(&mut out.flags_2476, 26, physical.air.flag_441);
    replace_byte(&mut out.flags_2476, 0, physical.air.flag_446);
    replace_byte(&mut out.flags_2480, 25, physical.air.flag_447);
    replace_byte(&mut out.flags_2480, 24, physical.air.flag_448);
    transfer_bit(
        &mut out.flags_2480,
        30,
        physical.component_1832_word_1876,
        28,
    );
    replace_byte(&mut out.flags_2468, 9, physical.grinds.flag_318);
    replace_byte(&mut out.flags_2468, 8, physical.grinds.flag_322);

    replace_byte(&mut out.flags_2468, 15, physical.collision.flag_3478);
    replace_byte(&mut out.flags_2468, 16, physical.collision.flag_3475);
    replace_byte(&mut out.flags_2468, 17, physical.collision.flag_3472);
    out.wheel_count_2556 = physical.collision.wheel_count_0;
    replace_byte(&mut out.flags_2468, 14, physical.collision.flag_3477);
    out.collision_scalar_2924 = physical.collision.scalar_28;
    replace_byte(&mut out.flags_2488, 30, physical.collision.flag_3481);
    replace_byte(&mut out.flags_2472, 11, physical.physics.flag_32);
    out.vectors_720_784_800_816_832_864[3] = physical.physics.vector_16;
    replace_byte(&mut out.flags_2472, 9, physical.skeleton.flag_597);
    replace_byte(&mut out.flags_2480, 10, physical.skeleton.flag_600);
    replace_byte(&mut out.flags_2480, 9, physical.skeleton.flag_601);
    out.filtered_state_2524 = physical.filtered_state_0;

    replace_byte(&mut out.flags_2476, 21, physical.off_board.flag_304);
    replace_bool(&mut out.flags_2476, 17, physical.off_board.scalar_32 <= 0.0);
    replace_byte(&mut out.flags_2480, 15, physical.off_board.flag_311);
    replace_byte(&mut out.flags_2476, 16, physical.off_board.flag_315);
    replace_byte(&mut out.flags_2476, 8, physical.off_board.flag_316);
    out.off_board_scalar_2832 = physical.off_board.scalar_32;
    replace_byte(&mut out.flags_2480, 1, physical.off_board.flag_318);
    replace_byte(&mut out.flags_2484, 23, physical.off_board.flag_314);
    out.vectors_880_896_912_928_944[2] = physical.off_board.vector_64;
    replace_byte(&mut out.flags_2484, 16, physical.off_board.flag_328);
    publish_wheel_contacts(physical, out);
}

fn publish_wheel_contacts(physical: &PhysicalPlayerInput, out: &mut ProcessedPhysicsInput) {
    let wheel = physical.collision.wheel_contact_3296_3299;
    let outer_tail = wheel[2] != 0 || wheel[3] != 0;
    let outer_nose = wheel[0] != 0 || wheel[1] != 0;
    let left_pair = wheel[2] != 0 || wheel[0] != 0;
    let right_pair = wheel[3] != 0 || wheel[1] != 0;
    let switched = out.flags_2468 & (1 << 20) != 0;
    replace_bool(
        &mut out.flags_2472,
        27,
        if switched { outer_nose } else { outer_tail },
    );
    replace_bool(
        &mut out.flags_2472,
        26,
        if switched { outer_tail } else { outer_nose },
    );
    replace_bool(
        &mut out.flags_2472,
        25,
        if switched { right_pair } else { left_pair },
    );
    replace_bool(
        &mut out.flags_2472,
        24,
        if switched { left_pair } else { right_pair },
    );
}

pub(crate) fn effective_animation_transform(source: RawMatrix, flags_2476: u32) -> RawMatrix {
    let mut result = source;
    if flags_2476 & (1 << 2) != 0 {
        for row in [0, 2] {
            for lane in &mut result[row] {
                *lane = (-f32::from_bits(*lane)).to_bits();
            }
        }
    }
    result
}

pub(crate) fn publish_line_tests(player: &PlayerInputState, out: &mut ProcessedPhysicsInput) {
    out.line_tests_960_1008_1056 = [
        player.left_line_test_1536,
        player.right_line_test_1584,
        player.hips_line_test_1488,
    ];
    if out.line_tests_960_1008_1056[0].valid != 0 {
        out.left_surface_2596 = out.line_tests_960_1008_1056[0].surface;
    }
    if out.line_tests_960_1008_1056[1].valid != 0 {
        out.right_surface_2600 = out.line_tests_960_1008_1056[1].surface;
    }
}

pub(crate) fn replace_byte(word: &mut u32, bit: u32, source: u8) {
    *word = (*word & !(1 << bit)) | (u32::from(source & 1) << bit);
}

pub(crate) fn replace_bool(word: &mut u32, bit: u32, source: bool) {
    *word = (*word & !(1 << bit)) | (u32::from(source) << bit);
}

pub(crate) fn transfer_bit(word: &mut u32, bit: u32, source: u32, source_bit: u32) {
    *word = (*word & !(1 << bit)) | (((source >> source_bit) & 1) << bit);
}
