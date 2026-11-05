//! Contact iteration, TU3 82AE2914..82AE2B8C.
//!
//! The compiled row boundary is decoded once. Calculations preserve the source
//! operation order for finite inputs; console exceptional-float behavior has
//! not been established by hardware captures.
#[path = "contact_data.rs"]
mod data;
use data::{ContactRows, Reaction};

pub(super) fn solve(record: &mut [u32], a: &mut [u32], b: &mut [u32]) {
    let rows = ContactRows::read(record);
    let mut reaction_a = Reaction::read(a);
    let mut reaction_b = Reaction::read(b);
    let position_a = reaction_a.point_position(rows.arm_a);
    let position_b = reaction_b.point_position(rows.arm_b);
    let velocity_a = reaction_a.point_velocity(rows.arm_a);
    let velocity_b = reaction_b.point_velocity(rows.arm_b);
    let position_error: [f32; 4] = core::array::from_fn(|i| position_b[i] - position_a[i]);
    let total_error: [f32; 4] =
        core::array::from_fn(|i| (velocity_b[i] - velocity_a[i]) + position_error[i]);

    // 82AE2A94..2ABC: the first three impulses respond to combined position
    // and velocity correction. The fourth responds only to position error.
    let candidate: [f32; 4] = core::array::from_fn(|lane| {
        let error = if lane == 3 {
            position_error
        } else {
            total_error
        };
        let mut impulse = rows.accumulated[lane] + rows.target[lane];
        for (component, &value) in error[..3].iter().enumerate() {
            let coefficient_lane = if lane == 3 { 0 } else { lane };
            impulse = rows.correction_columns[component][coefficient_lane].mul_add(value, impulse);
        }
        impulse
    });

    // 82AE2AC4..2B08: friction uses the preceding normal impulse. Crossing
    // the static limit selects the dynamic limit. The two normal lanes use
    // [0, f32::MAX]. Ordered comparisons also preserve the native selectors.
    let next: [f32; 4] = core::array::from_fn(|lane| {
        let normal = if lane == 1 || lane == 2 {
            rows.accumulated[0]
        } else {
            0.0
        };
        let upper_offset = if lane == 0 || lane == 3 {
            f32::MAX
        } else {
            0.0
        };
        let static_low = (-rows.static_friction).mul_add(normal, 0.0);
        let static_high = rows.static_friction.mul_add(normal, upper_offset);
        let dynamic_low = (-rows.dynamic_friction).mul_add(normal, 0.0);
        let dynamic_high = rows.dynamic_friction.mul_add(normal, upper_offset);
        let lower_selected = if candidate[lane] >= static_low {
            candidate[lane]
        } else {
            dynamic_low
        };
        if static_high >= candidate[lane] {
            lower_selected
        } else {
            dynamic_high
        }
    });
    let change = core::array::from_fn(|lane| next[lane] - rows.accumulated[lane]);
    rows.publish_impulses(record, next);
    reaction_a.apply_contact(&rows, change, true);
    reaction_b.apply_contact(&rows, change, false);
    reaction_a.write(a);
    reaction_b.write(b);
}
