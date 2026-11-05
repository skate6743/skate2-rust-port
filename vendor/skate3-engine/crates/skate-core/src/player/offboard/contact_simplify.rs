//! Native record reduction82D830A8. Marks discarded distances, invokes the
//! native sort and truncates to the retained count; neighbors are not rewritten.
use super::{
    contact_queries::Input,
    contact_records::Record,
    contact_segments::{length_inverse, reciprocal},
};
use crate::physics::native_arithmetic::dot3;

/// Returns Toolkit284's forward limit, reset even for fewer than two records.
pub struct Reduction {
    pub forward_limit: f32,
    /// Native count14928 shrinks, but84860 still scans count14932 including
    ///the sorted tail in the fixed backing array. Retain these physical records.
    pub inactive_records: Vec<Record>,
}
pub fn simplify(input: Input, records: &mut Vec<Record>) -> Reduction {
    let mut limit = f32::from_bits(0x5015_02f9);
    let original = records.len();
    if original < 2 {
        return Reduction {
            forward_limit: limit,
            inactive_records: Vec::new(),
        };
    }
    let sentinel = records[original - 1].distance + 1.;
    let mut retained = original;
    for i in 1..original - 1 {
        if records[i].flags & 8 != 0 {
            retained -= original - i - 1;
            limit = dot3(
                std::array::from_fn(|k| records[i].position[k] - input.position[k]),
                input.surface_forward,
            );
            break;
        }
        let a = records[i - 1].coordinates;
        let b = records[i].coordinates;
        let c = records[i + 1].coordinates;
        let incoming = [b[0] - a[0], b[1] - a[1], 0., 0.];
        let outgoing = [c[0] - b[0], c[1] - b[1], 0., 0.];
        let in_length = length_inverse(incoming).0;
        let out_length = length_inverse(outgoing).0;
        let mut remove = in_length < f32::from_bits(0x3ca3_d70a);
        if !remove && !(out_length < f32::from_bits(0x3ca3_d70a)) {
            if !(dot3(incoming, outgoing) < 0. && dot3(incoming, input.surface_up) >= 0.) {
                let a = incoming.map(|v| v * reciprocal(in_length));
                let b = outgoing.map(|v| v * reciprocal(out_length));
                let cross = [
                    (-a[2]).mul_add(b[1], a[1] * b[2]),
                    (-a[0]).mul_add(b[2], a[2] * b[0]),
                    (-a[1]).mul_add(b[0], a[0] * b[1]),
                    (-a[3]).mul_add(b[3], a[3] * b[3]),
                ];
                remove = !(length_inverse(cross).0 >= f32::from_bits(0x3d4c_cccd));
            }
        }
        if remove {
            records[i].distance = sentinel;
            retained -= 1;
        }
    }
    super::contact_sort::sort(records);
    let inactive_records = records.split_off(retained);
    Reduction {
        forward_limit: limit,
        inactive_records,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            position: [0.; 4],
            surface_forward: [0., 0., 1., 0.],
            surface_up: [0., 1., 0., 0.],
            surface_right: [1., 0., 0., 0.],
            velocity: [0.; 4],
            animation_up: [0., 1., 0., 0.],
            animation_right: [1., 0., 0., 0.],
        }
    }
    fn r(y: f32, z: f32) -> Record {
        Record {
            position: [0., y, z, 0.],
            normal: [0., 1., 0., 0.],
            coordinates: [z, y, z, z],
            flags: 0,
            distance: z,
        }
    }
    #[test]
    fn flat_samples_reduce_to_endpoints_but_step_corner_survives() {
        let mut records = vec![r(0., 0.), r(0., 0.5), r(0., 1.), r(0., 1.5)];
        let reduced = simplify(input(), &mut records);
        assert_eq!(reduced.forward_limit.to_bits(), 0x501502f9);
        assert_eq!(reduced.inactive_records.len(), 2);
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].distance, 1.5);
        let mut records = vec![r(0., 0.), r(0., 1.), r(1., 1.), r(1., 2.)];
        simplify(input(), &mut records);
        assert_eq!(records.len(), 4);
    }
    #[test]
    fn elevated_record_caps_forward_search_after_prior_removals() {
        let mut records = vec![r(0., 0.), r(0., 0.5), r(0., 1.), r(1., 1.5), r(1., 2.)];
        records[3].flags = 8;
        assert_eq!(simplify(input(), &mut records).forward_limit, 1.5);
        assert_eq!(records.len(), 3);
        assert_eq!(records[2].position, [0., 1., 1.5, 0.]);
    }
}
