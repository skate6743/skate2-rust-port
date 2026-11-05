use skate_core::animation::{
    commands::buffers::BufferError,
    output::{
        NativeMatrix,
        packet_reset::{self, AdditionalResetFields},
        physics_packet::PhysicsPosePacket,
    },
};
fn filled(value: f32) -> NativeMatrix {
    [[value; 4]; 4]
}
fn packet(count: u32) -> PhysicsPosePacket {
    PhysicsPosePacket {
        bone_count: count,
        hierarchy: vec![filled(7.0); 3],
        local: vec![filled(8.0); 3],
        timestep: 99.0,
        foot_surface_ids: [123, 456],
        flags: u32::MAX,
        board_flipped: true,
        mirrored: true,
        riding_switch: true,
        riding_fakie: true,
        weight_forwards: true,
        regular_stance: true,
        air_dismount_revert_frames: 100,
    }
}
fn fields() -> AdditionalResetFields {
    AdditionalResetFields {
        compression: 9.0,
        foot_ik_influence: [4.0, 5.0],
        next_step_position_valid: true,
        actor_flag_1904_bit23: true,
        actor_flag_1908_bit2: true,
        external_impulse_active: true,
        external_physics_input_active: true,
        externally_controlled: true,
        prevent_manual_respawn: true,
        ignore_respawn_reset_button: 0xff,
        force_braking: true,
        truck_tightness: 4.0,
        wheel_hardness: 5.0,
        auxiliary_vectors: [[f32::NAN; 4]; 6],
        requested_physics_mode: 4,
    }
}
#[test]
fn reset_clears_all_owned_fields_and_uses_packed_basis_with_zero_translation_w() {
    let mut packet = packet(2);
    let mut fields = fields();
    let hierarchy_allocation = packet.hierarchy.as_ptr();
    let local_allocation = packet.local.as_ptr();
    packet_reset::reset(&mut packet, &mut fields).unwrap();
    let expected = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0; 4],
    ];
    assert_eq!(&packet.local[..2], [expected, expected]);
    assert_eq!(&packet.hierarchy[..2], [expected, expected]);
    assert_eq!(packet.local[2], filled(8.0));
    assert_eq!(packet.hierarchy[2], filled(7.0));
    assert_eq!(packet.hierarchy.as_ptr(), hierarchy_allocation);
    assert_eq!(packet.local.as_ptr(), local_allocation);
    assert_eq!(packet.bone_count, 2);
    assert_eq!(packet.timestep.to_bits(), 0x3c888889);
    assert_eq!(packet.flags, 0x013fffff);
    assert_eq!(packet.foot_surface_ids, [0, 0]);
    assert_eq!(packet.air_dismount_revert_frames, 0);
    assert!(
        !packet.board_flipped
            && !packet.mirrored
            && !packet.riding_switch
            && !packet.riding_fakie
            && !packet.weight_forwards
            && !packet.regular_stance
    );
    assert_eq!(fields.compression, 0.5);
    assert_eq!(fields.foot_ik_influence, [0.0, 0.0]);
    assert_eq!(fields.truck_tightness, 0.0);
    assert_eq!(fields.wheel_hardness, 0.0);
    assert_eq!(fields.ignore_respawn_reset_button, 0);
    assert_eq!(fields.requested_physics_mode, 1);
    assert!(
        !fields.next_step_position_valid
            && !fields.actor_flag_1904_bit23
            && !fields.actor_flag_1908_bit2
            && !fields.external_impulse_active
            && !fields.external_physics_input_active
            && !fields.externally_controlled
            && !fields.prevent_manual_respawn
            && !fields.force_braking
    );
    for vector in fields.auxiliary_vectors {
        assert_eq!(vector.map(f32::to_bits), [0; 4]);
    }
}
#[test]
fn reset_preserves_every_flag_outside_the_recovered_clear_mask() {
    for bit in 0..32 {
        let mut packet = packet(0);
        let mut fields = fields();
        packet.flags = 1 << bit;
        packet_reset::reset(&mut packet, &mut fields).unwrap();
        let preserved = bit <= 21 || bit == 24;
        assert_eq!(
            packet.flags,
            if preserved { 1 << bit } else { 0 },
            "bit {bit}"
        );
    }
}
#[test]
fn zero_and_signed_negative_counts_skip_matrices_but_still_reset_scalars() {
    for count in [0, 0x80000000, u32::MAX] {
        let mut packet = packet(count);
        let mut fields = fields();
        packet_reset::reset(&mut packet, &mut fields).unwrap();
        assert_eq!(packet.bone_count, count);
        assert_eq!(packet.local, vec![filled(8.0); 3]);
        assert_eq!(fields.compression, 0.5);
        assert_eq!(packet.flags, 0x013fffff);
    }
}
#[test]
fn short_allocation_is_rejected_without_partial_reset() {
    let mut packet = packet(4);
    let mut fields = fields();
    assert_eq!(
        packet_reset::reset(&mut packet, &mut fields),
        Err(BufferError::RangeOutsideAllocation)
    );
    assert_eq!(packet.flags, u32::MAX);
    assert_eq!(fields.requested_physics_mode, 4);
    assert_eq!(packet.local, vec![filled(8.0); 3]);
}
