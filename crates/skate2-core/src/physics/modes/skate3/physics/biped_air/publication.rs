//! Recovered output adapter; switches with the complete lifecycle migration.
pub(crate) fn publish(
    output: skate3_core::player::offboard::biped_air::recovered::OffBoardOutput,
    off_board: &mut skate3_core::player::input_phase::OffBoardOutputFields,
) {
    //82D30300 writes OffBoard, not the overlapping on-board Air packet.
    //Common Biped cadence80/state84 publication remains coordinator-owned.
    off_board.scalar_32 = output.scalar_32;
    off_board.vector_64 = output.vector_64.map(f32::to_bits);
    off_board.scalar_92 = output.scalar_92;
    off_board.vector_96 = output.vector_96.map(f32::to_bits);
    off_board.word_144 = output.word_144;
    off_board.scalar_148 = output.scalar_148;
    off_board.scalar_152 = output.scalar_152;
    off_board.scalar_156 = output.scalar_156;
    off_board.vector_160 = output.vector_160.map(f32::to_bits);
    off_board.vector_176 = output.vector_176.map(f32::to_bits);
    off_board.vector_192 = output.vector_192.map(f32::to_bits);
    off_board.vector_208 = output.vector_208.map(f32::to_bits);
    off_board.vector_224 = output.vector_224.map(f32::to_bits);
    off_board.vector_240 = output.vector_240.map(f32::to_bits);
    off_board.flag_320 = u8::from(output.flag_320);
    off_board.flag_328 = u8::from(output.flag_328);
    off_board.trajectory_valid_331 = u8::from(output.flag_331);
}
