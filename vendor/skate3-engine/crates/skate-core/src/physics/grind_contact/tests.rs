use super::*;
use crate::point_graph::PointGraph;

const BOARD: [V; 4] = [
    [1., 0., 0., 0.],
    [0., 1., 0., 0.],
    [0., 0., 1., 0.],
    [0.; 4],
];
const GRAPH: PointGraph<4> = PointGraph {
    x: [0., 0.3, 0.7, 1.],
    y: [1.; 4],
};
fn admission(category: u32, state: u32) -> admission::Admission<'static> {
    admission::Admission {
        category,
        state,
        speed: 2.,
        velocity: [0., 0., 2., 0.],
        threshold_vs_slope: &GRAPH,
    }
}
fn rail(start: V, end: V) -> Primitive {
    Primitive {
        start,
        end,
        owner: 1,
    }
}

#[test]
fn one_truck_admission_happens_before_front_rear_selection() {
    let edges = [
        rail([0.; 4], [0.5, 0., 0.8660254, 0.]),
        rail([0.; 4], [0., 0., 1., 0.]),
    ];
    let hits = [
        Some(TruckContact {
            position: [0., 0., 0.2, 0.],
            primitive: 0,
        }),
        Some(TruckContact {
            position: [0., 0., -0.2, 0.],
            primitive: 1,
        }),
    ];
    let result = families::five_o(
        BOARD,
        hits,
        &edges,
        [0., 0., 2., 0.],
        0,
        0,
        0,
        1.,
        &admission(400, 400),
    )
    .unwrap();
    assert!(!result.front);
    assert_eq!(result.geometry.primitive, 1);
    assert_eq!(result.entry_kind, admission::EntryKind::ChangeGrind);
}

#[test]
fn tipslide_drop_in_rejects_contact_above_deck_without_descent() {
    let edges = [rail([0.; 4], [0., 0., 1., 0.])];
    let hit = Some(TruckContact {
        position: [0., 0.01, 0.2, 0.],
        primitive: 0,
    });
    let input = admission::Admission {
        speed: 1.,
        velocity: [0., 0.1, 0., 0.],
        ..admission(100, 100)
    };
    assert!(
        families::tipslide(
            BOARD,
            [hit, None],
            &edges,
            input.velocity,
            100,
            0,
            0,
            0,
            0.,
            BOARD[0],
            &input
        )
        .is_none()
    );
}

#[test]
fn darkslide_uses_its_own_tilt_and_depth_thresholds() {
    let edges = [rail([0.; 4], [0., 0., 1., 0.])];
    let board = [
        [-0.6, -0.8, 0., 0.],
        [0.8, -0.6, 0., 0.],
        [0., 0., 1., 0.],
        [0.056, -0.042, 0., 0.],
    ];
    let hit = TruckContact {
        position: [0.; 4],
        primitive: 0,
    };
    let result = families::darkslide(
        board,
        hit,
        edges[0],
        [0., 0., 2., 0.],
        200,
        0,
        &admission(200, 201),
    );
    assert!(result.is_some());
    let mut deeper = board;
    deeper[3] = scale(board[1], 0.08);
    assert!(
        families::darkslide(
            deeper,
            hit,
            edges[0],
            [0., 0., 2., 0.],
            200,
            0,
            &admission(200, 201)
        )
        .is_none()
    );
}

#[test]
fn one_truck_clamps_match_both_originals_not_zip_reversed_signs() {
    for (front, tilt, input, expected) in [
        (true, -0.3, 0.5, 0.),
        (true, -0.3, -0.5, -0.011249996),
        (false, 0.3, -0.5, 0.),
        (false, 0.3, 0.5, 0.011249996),
        (true, 0.3, -0.5, 0.),
        (true, 0.3, 0.5, 0.011249996),
        (false, -0.3, 0.5, 0.),
        (false, -0.3, -0.5, -0.011249996),
    ] {
        let mut control = control::Control::default();
        control.update(
            3,
            [0., 0., 1., 0.],
            [0., 1., tilt, 0.],
            front,
            false,
            false,
            0.,
            0.,
            input,
            0.,
        );
        assert!(
            (control.pitch - expected).abs() < 1e-7,
            "front={front} tilt={tilt} input={input}: {}",
            control.pitch
        );
    }
}
