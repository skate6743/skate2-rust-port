use super::*;
const X: V = [1.0, 0.0, 0.0, 0.0];
const Y: V = [0.0, 1.0, 0.0, 0.0];
const Z: V = [0.0, 0.0, 1.0, 0.0];
const O: V = [0.0; 4];
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.0001, "{a} != {b}");
}

#[test]
fn material_and_geometry_are_independent_and_support_is_not_the_grind_normal() {
    assert_eq!(material_multiplier(13), material_multiplier(2));
    assert_eq!(material_multiplier(999), 1.3);
    for (kind, strength) in [(0, 50.0), (1, 40.0), (2, 30.0), (99, 30.0)] {
        let force = friction(
            [4.0, 9.0, 0.0, 0.0],
            Y,
            [0.0, 0.25, 0.0, 0.0],
            0.5,
            true,
            material_multiplier(4),
            kind,
            [50.0, 40.0, 30.0],
        );
        near(force[0], -0.25 * 0.5 * 1.9 * 0.6 * strength * 0.48);
        near(force[1], 0.0);
    }
    assert!(!friction_applies([0.001, 10.0, 0.0, 0.0], Y));
    assert!(friction_applies([0.002, 10.0, 0.0, 0.0], Y));
}

#[test]
fn pin_uses_signed_reference_and_skips_zero_error_instead_of_waking_body() {
    assert_eq!(
        lateral_pin([X, Y, Z, O], O, X, X, 800.0, 0.0, 0.0, true, 1.0),
        None
    );
    let force = lateral_pin(
        [X, Y, Z, O],
        X,
        X,
        [2.0, 0.0, 0.0, 0.0],
        100.0,
        0.0,
        0.0,
        true,
        0.5,
    )
    .unwrap();
    near(force[0], (100.0 - 100.0 * 0.133 * 2.0) * 0.5);
    let graph = crate::point_graph::PointGraph {
        x: [-1.0, 0.0, 0.5, 1.0],
        y: [0.0, 0.0, 0.5, 0.75],
    };
    assert_eq!(pin_slope(Y, Y, 0.0, &graph), 0.75);
    assert_eq!(pin_slope(Y, Y, 0.01, &graph), 1.0);
}

fn slide_input() -> slide::Input {
    slide::Input {
        position: O,
        point: O,
        direction: Z,
        across: X,
        velocity: [2.0, 3.0, 4.0, 0.0],
        translation_2796: 0.5,
        total_mass_2660: 10.0,
        update_frequency: 30.0,
        preparing_jump: false,
        geometry_kind: 0,
    }
}

#[test]
fn slide_force_order_mass_frequency_and_darkslide_translation() {
    let mut input = slide_input();
    let f = slide::control(slide::Slide::Boardslide, input);
    assert_eq!(f, vec![[12.5, 0.0, 0.0, 0.0], [-40.0, 0.0, 0.0, 0.0]]);
    input.position[0] = 0.08;
    let f = slide::control(slide::Slide::Darkslide, input);
    near(f[0][0], 10.0); //(.5 remaining)*40*.5 translation.
    input.position[0] = 0.2;
    let f = slide::control(slide::Slide::Boardslide, input);
    near(f[0][0], -10.0);
    near(f[1][0], -600.0); //Actual input frequency30, not ZIP fixed60.
    near(f[1][1], 0.0);
    input.preparing_jump = true;
    input.position[0] = 0.12;
    assert!(slide::control(slide::Slide::Boardslide, input).is_empty());
}

#[test]
fn exit_lift_flag_is_independent_and_speed_equality_skips_force() {
    let mut input = release::Input {
        position: X,
        point: O,
        across: X,
        normal: Y,
        velocity: O,
        geometry_kind: 2,
        high_side: Z,
        force_across: false,
        enable_lift: false,
        strength: 90.0,
        speed_limit: 0.75,
        lift: 125.0,
    };
    assert_eq!(release::force(input), Some([0.0, 0.0, -90.0, 0.0]));
    input.enable_lift = true;
    assert_eq!(release::force(input), Some([0.0, 125.0, -90.0, 0.0]));
    input.force_across = true;
    input.velocity = [0.75, 0.0, 0.0, 0.0];
    assert_eq!(release::force(input), None);
}

#[test]
fn noise_preserves_draw_order_modulo_speed_cap_and_position() {
    let a = noise::angles(2.0, 1.0, [0, 50_000, 99_999]);
    near(a[0], -0.06);
    near(a[1], 0.0);
    near(a[2], 0.0599988);
    assert_eq!(a, noise::angles(20.0, 1.0, [100_000, 150_000, 199_999]));
    let frame = [X, Y, Z, [8.0, 9.0, 10.0, 1.0]];
    let n = noise::apply(frame, 2.0, 1.0, [12_345, 54_321, 87_654]);
    assert_eq!(n[3], frame[3]);
    for column in &n[..3] {
        near(dot3(*column, *column), 1.0);
    }
    near(dot3(n[0], n[1]), 0.0);
    assert_eq!(noise::apply(frame, 0.0, 1.0, [0, 1, 2]), frame);
}
