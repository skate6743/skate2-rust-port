use skate_core::physics::{board_toolkit::BoardToolkit, skeleton_animation_record::IDENTITY};

#[test]
fn toolkit_absolute_speed_preserves_original_ordered_sign_multiply() {
    let up = [0.0, 1.0, 0.0, 0.0];
    //82C01718..744 compares strictly >0 before multiplying by +/-1.
    //Positive zero therefore produces negative zero; fabs is not equivalent.
    for (input, expected) in [(3.0_f32, 3.0_f32), (-3.0, 3.0), (0.0, -0.0), (-0.0, 0.0)] {
        let toolkit = BoardToolkit::calculate(IDENTITY, &[1.0; 7], 0, input, up, up);
        assert_eq!(toolkit.absolute_speed.to_bits(), expected.to_bits());
    }
}
