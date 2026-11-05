use super::*;
fn constant(value: f32) -> PointGraph<8> {
    PointGraph {
        x: [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
        y: [value; 8],
    }
}
fn input() -> Input {
    Input {
        special_mode_714: false,
        surface_normal_560: [0.0, 1.0, 0.0, 0.0],
        movement_velocity_480: [0.0; 4],
        contact_direction_400: None,
    }
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn special_mode_retains_elapsed_until_low_heading_allows_reentry() {
    let mut mode = SpecialMode {
        enabled_714: false,
        elapsed_788: 0.0,
    };
    mode.update(1.0, 0x200, 0.0);
    assert!(mode.enabled_714);
    mode.update(0.0, 0, 0.5);
    assert!(!mode.enabled_714);
    assert!(mode.elapsed_788 > 0.0);
    mode.update(1.0, 0x200, 0.5);
    assert!(!mode.enabled_714);
    mode.update(1.0, 0x200, 0.0);
    assert_eq!(mode.elapsed_788, 0.0);
    assert!(!mode.enabled_714);
    mode.update(1.0, 0x200, 0.0);
    assert!(mode.enabled_714);
}

#[test]
fn special_mode_skips_both_slide_fields() {
    let mut slide = Sliding {
        velocity_528: [1.0, 2.0, 3.0, 0.0],
        active_710: true,
    };
    let prior = slide;
    let mut i = input();
    i.special_mode_714 = true;
    slide.update(i, &constant(1.0), &constant(1.0));
    assert_eq!(slide, prior);
}

#[test]
fn flat_surface_decays_old_slide_without_downhill_force() {
    let mut slide = Sliding {
        velocity_528: [1.0, 0.0, 0.0, 0.0],
        active_710: true,
    };
    slide.update(input(), &constant(1.0), &constant(1.0));
    close(slide.velocity_528[0], 0.95 * 0.9);
    assert!(slide.active_710);
}

#[test]
fn slope_force_follows_surface_and_contact_removes_into_wall_component() {
    let mut i = input();
    i.surface_normal_560 = [0.0, 0.8, 0.6, 0.0];
    let mut slide = Sliding {
        velocity_528: [0.0; 4],
        active_710: false,
    };
    slide.update(i, &constant(1.0), &constant(1.0));
    assert!(slide.velocity_528[1] < 0.0);
    assert!(slide.velocity_528[2] > 0.0);
    assert!(slide.active_710);
    i.contact_direction_400 = Some([0.0, 0.0, -1.0, 0.0]);
    slide.update(i, &constant(1.0), &constant(1.0));
    close(slide.velocity_528[2], 0.0);
}
