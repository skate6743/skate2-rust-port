use super::*;

fn geometry() -> GeometryInput {
    GeometryInput {
        valid: true,
        family: 2,
        current_state: 100,
        category: 100,
        air_frames: 0,
        geometry_kind: 2,
        geometry_flags: 0,
        far_points: [[0., 0.1, 0.2, 0.], [0., -0.1, -0.2, 0.]],
        upmost: [0., 1., 0., 0.],
        high_side: [0., 0., 1., 0.],
        point: [0.; 4],
        direction: [1., 0., 0., 0.],
        board_position: [0., 0.1, 0.2, 0.],
        board_forward: [1., 0., 0., 0.],
        velocity: [1., 0., 0., 0.],
        deck_to_truck: 0.243,
        previous_exit_angle: 0.,
        previous_exit_direction: [1., 0., 0., 0.],
        flags: 0,
    }
}

#[test]
fn backslash_uses_height_difference_and_preserves_existing_tipslide() {
    let mut counter = 0;
    assert_eq!(tweak_geometry(geometry(), &mut counter).family, 4);
    assert_eq!(
        tweak_geometry(
            GeometryInput {
                current_state: 402,
                ..geometry()
            },
            &mut counter
        )
        .family,
        2
    );
    assert_eq!(
        tweak_geometry(
            GeometryInput {
                board_position: [0., 0., -1., 0.],
                ..geometry()
            },
            &mut counter
        )
        .family,
        2
    );
}

#[test]
fn five_o_hanging_deck_sets_pry_then_excludes_thirty_frames() {
    let mut counter = 0;
    let input = GeometryInput {
        family: 3,
        board_forward: [0., -0.1, 0.9, 0.],
        ..geometry()
    };
    let result = tweak_geometry(input, &mut counter);
    assert!(!result.valid);
    assert_ne!(result.flags & 0x1000_0000, 0);
    assert_eq!(counter, 30);
    assert!(
        !tweak_geometry(
            GeometryInput {
                board_forward: [1., 0., 0., 0.],
                ..input
            },
            &mut counter
        )
        .valid
    );
}

#[test]
fn coping_timer_is_reduced_on_activation_and_expires_without_contact() {
    let vertical = PointGraph {
        x: [-1., 0., 1., 2.],
        y: [2.; 4],
    };
    let linear = PointGraph {
        x: [0., 1., 2., 3.],
        y: [0.5; 4],
    };
    let mut timer = 0.;
    let tangent = [1., 0., 0., 0.];
    let velocity = [3., 1., 0., 0.];
    assert_eq!(
        gravity_relief(
            &mut timer, true, 100, tangent, velocity, 0.25, &vertical, &linear
        ),
        0.75
    );
    assert_eq!(
        gravity_relief(
            &mut timer, false, 200, tangent, velocity, 1., &vertical, &linear
        ),
        0.
    );
}

#[test]
fn jumper_retains_geometry_and_regenerates_per_update_not_per_second() {
    let mut jumper = Jumper {
        cooldown: 2,
        energy: 0.4,
        ..Jumper::default()
    };
    assert_eq!(jumper.update(None, u32::MAX), 0x0200_0000);
    assert_eq!(jumper.family, 3);
    assert_eq!(jumper.geometry.normal, [0., 1., 0., 0.]);
    assert_eq!(jumper.energy, 0.4 + 0.0035);
    assert_eq!(jumper.update(None, 5), 0);
    assert_eq!(jumper.family, 5);
}

#[test]
fn directed_rail_retains_previous_sign_only_at_low_speed() {
    let start = [0.; 4];
    let end = [1., 0., 0., 0.];
    let previous = [-1., 0., 0., 0.];
    assert!(directed_tangent(start, end, [0.05, 0., 0., 0.], previous)[0] < 0.);
    assert!(directed_tangent(start, end, [0.1, 0., 0., 0.], previous)[0] > 0.);
}
