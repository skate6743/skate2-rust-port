//! Selective reset of the represented ProcessedPhysIn fields, TU3 82BF9EF0.
//! Packet buffers and retained flag lanes deliberately survive this reset.
use skate3_core::player::input_phase::ProcessedPhysicsInput;

pub(crate) fn reset_processed(input: &mut ProcessedPhysicsInput) {
    let external = input.external_physics_1616;
    let vector_832 = input.vectors_720_784_800_816_832_864[4];
    let mut reset = ProcessedPhysicsInput {
        effective_anim_transform_192: [
            [1.0f32.to_bits(), 0, 0, 0],
            [0, 1.0f32.to_bits(), 0, 0],
            [0, 0, 1.0f32.to_bits(), 0],
            [0; 4],
        ],
        vector_1520: input.vector_1520,
        matrix_1536: input.matrix_1536,
        byte_1600: input.byte_1600,
        external_physics_1616: external,
        probe_1792: input.probe_1792,
        flags_2468: 0x2000 | (input.flags_2468 & 8),
        flags_2488: input.flags_2488 & 0x001f_ffff,
        state_variant_index_2528: 1,
        grind_words_2532_2536: [u32::MAX, 0],
        timestep_2604: f32::from_bits(0x3c88_8889),
        gravity_2648: f32::from_bits(0xc11c_cccd),
        actor_query_2952: u32::MAX,
        ..ProcessedPhysicsInput::default()
    };
    reset.external_physics_1616.flags &= 0x01ff_ffff;
    reset.vectors_544_560_592_608[0] = [0, 1.0f32.to_bits(), 0, 0];
    // Raw 82BFA1D0 and 82BFA1D8 both store to +816; +832 is retained.
    reset.vectors_720_784_800_816_832_864[4] = vector_832;
    *input = reset;
}

#[cfg(any())]
mod tests {
    use super::*;
    #[test]
    fn reset_retains_only_native_packet_lanes_and_masks() {
        let mut input = ProcessedPhysicsInput::default();
        input.flags_2468 = u32::MAX;
        input.flags_2488 = u32::MAX;
        input.external_physics_1616.flags = u32::MAX;
        input.external_physics_1616.vectors[0] = [7; 4];
        input.vectors_720_784_800_816_832_864 = [[9; 4]; 6];
        input.vector_1520 = [11; 4];
        input.scalar_2652 = 123.0;
        reset_processed(&mut input);
        assert_eq!(input.flags_2468, 0x2008);
        assert_eq!(input.flags_2488, 0x001f_ffff);
        assert_eq!(input.external_physics_1616.flags, 0x01ff_ffff);
        assert_eq!(input.external_physics_1616.vectors[0], [7; 4]);
        assert_eq!(input.vectors_720_784_800_816_832_864[3], [0; 4]);
        assert_eq!(input.vectors_720_784_800_816_832_864[4], [9; 4]);
        assert_eq!(input.vector_1520, [11; 4]);
        assert_eq!(input.scalar_2652, 0.0);
        assert_eq!(input.state_variant_index_2528, 1);
    }
}
