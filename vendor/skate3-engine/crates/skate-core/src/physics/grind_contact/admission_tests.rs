use super::*;

const GRAPH: PointGraph<4> = PointGraph {
    x: [0.0, 0.3, 0.7, 1.0],
    y: [1.0; 4],
};
const RAIL: V = [1.0, 0.0, 0.0, 0.0];
fn context(category: u32, state: u32, velocity: V) -> Admission<'static> {
    Admission {
        category,
        state,
        speed: 2.0,
        velocity,
        threshold_vs_slope: &GRAPH,
    }
}

#[test]
fn current_grind_and_family_change_have_different_angle_gates() {
    let input = context(400, 401, [0.8660254, 0.0, 0.5, 0.0]);
    assert_eq!(
        input.test(401, RAIL, false),
        Decision {
            kind: EntryKind::StayInGrind,
            allowed: true
        }
    );
    assert_eq!(
        input.test(403, RAIL, true),
        Decision {
            kind: EntryKind::ChangeGrind,
            allowed: false
        }
    );
    let backslash = Admission {
        state: 404,
        ..input
    };
    assert_eq!(backslash.test(402, RAIL, true).kind, EntryKind::StayInGrind);
}

#[test]
fn ground_entries_distinguish_above_below_coping_and_drop_in() {
    let above = context(100, 100, [2.0, -1.0, 0.0, 0.0]);
    assert_eq!(above.test(401, RAIL, false).kind, EntryKind::RideFromAbove);
    let below = context(100, 100, [2.0, 1.0, 1.0, 0.0]);
    assert_eq!(below.test(401, RAIL, false).kind, EntryKind::RideFromBelow);
    let coping = context(100, 100, [2.0, 1.0, 0.0, 0.0]);
    assert_eq!(
        coping.test(401, RAIL, false).kind,
        EntryKind::RideIntoCoping
    );
    let drop_in = Admission {
        speed: 1.0,
        ..above
    };
    assert_eq!(
        drop_in.test(403, RAIL, true),
        Decision {
            kind: EntryKind::DropIn,
            allowed: true
        }
    );
    assert_eq!(
        drop_in.test(401, RAIL, false).kind,
        EntryKind::RideFromAbove
    );
    let boundary = Admission {
        speed: 1.45,
        ..drop_in
    };
    assert_ne!(boundary.test(403, RAIL, true).kind, EntryKind::DropIn);
}

#[test]
fn coping_admission_uses_cross_rail_speed_and_the_authored_graph() {
    let slow = context(100, 100, [1.0, 1.0, 0.0, 0.0]);
    assert!(slow.test(401, RAIL, false).allowed);
    let fast = context(100, 100, [1.0, 10.0, 0.0, 0.0]);
    assert!(!fast.test(401, RAIL, false).allowed);
    let zero = PointGraph {
        x: GRAPH.x,
        y: [0.0; 4],
    };
    assert!(
        !Admission {
            threshold_vs_slope: &zero,
            ..slow
        }
        .test(401, RAIL, false)
        .allowed
    );
}

#[test]
fn airborne_admission_and_projection_keep_the_original_leaf_boundary() {
    assert_eq!(
        context(200, 201, [0.0, 0.0, 8.0, 0.0]).test(401, RAIL, false),
        Decision {
            kind: EntryKind::AirToGrind,
            allowed: true
        }
    );
    assert!(
        context(100, 100, [0.0, -8.0, 0.0, 0.0])
            .test(401, RAIL, false)
            .allowed
    );
    assert_eq!(approach_slope_sine(RAIL, [3.0, 0.0, 0.0, 0.0]), 0.0);
    assert_eq!(approach_slope_sine(RAIL, [3.0, 0.01, 0.0, 0.0]), 0.0);
}
