//! Expected branch outcomes are specified directly from TU3 82481E10's
//! fcmpu/blt control flow, not generated-code execution.
use skate_core::point_graph::PointGraph;

#[test]
fn unordered_endpoint_comparison_selects_the_final_value() {
    let graph = PointGraph {
        x: [0.0, 1.0, 2.0],
        y: [3.0, 5.0, 7.0],
    };
    assert_eq!(graph.evaluate(f32::NAN), 7.0);
    let last_key_nan = PointGraph {
        x: [0.0, 1.0, f32::NAN],
        y: graph.y,
    };
    assert_eq!(last_key_nan.evaluate(0.5), 7.0);
}

#[test]
fn unordered_interior_key_does_not_select_an_interval() {
    let graph = PointGraph {
        x: [0.0, f32::NAN, 1.0, 2.0],
        y: [0.0, 99.0, 10.0, 20.0],
    };
    assert_eq!(graph.evaluate(1.5), 15.0);
}

#[test]
fn finite_endpoints_and_interpolation_remain_unchanged() {
    let graph = PointGraph {
        x: [0.0, 1.0, 2.0],
        y: [0.0, 2.0, 8.0],
    };
    for (input, expected) in [
        (-1.0, 0.0),
        (0.0, 0.0),
        (0.5, 1.0),
        (1.0, 2.0),
        (1.5, 5.0),
        (2.0, 8.0),
        (3.0, 8.0),
    ] {
        assert_eq!(graph.evaluate(input), expected);
    }
}
