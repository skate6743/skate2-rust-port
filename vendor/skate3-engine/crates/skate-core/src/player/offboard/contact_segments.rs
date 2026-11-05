//! Surface segment construction82D83438 and candidate insertion82D82380.
//! Input records must already have passed the earlier native classifiers.
use super::{
    contact_queries::{Input, V},
    contact_records::Record,
};
use crate::physics::native_arithmetic::{dot3, reciprocal_estimate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Surface,
    Rising,
    Falling,
}
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub kind: Kind,
    pub start: V,
    pub end: V,
    pub direction: V,
    pub normal: V,
    pub length: f32,
}
#[derive(Default)]
pub struct Segments {
    pub items: Vec<Segment>,
    ///25232 is one past the last Surface segment, not the number of all segments.
    pub last_surface_end: usize,
}
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V, s: f32) -> V {
    a.map(|x| x * s)
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
pub(super) fn length_inverse(v: V) -> (f32, f32) {
    let sq = dot3(v, v);
    let mut inverse = crate::physics::reciprocal_sqrt::estimate(sq);
    for _ in 0..2 {
        inverse = (inverse * 0.5).mul_add((-sq).mul_add(inverse * inverse, 1.), inverse);
    }
    (if sq == 0. { 0. } else { sq * inverse }, inverse)
}
pub(super) fn reciprocal(v: f32) -> f32 {
    let mut r = reciprocal_estimate(v);
    for _ in 0..2 {
        r = r.mul_add((-r).mul_add(v, 1.), r);
    }
    r
}
fn normal(delta: V, input: Input) -> V {
    let cross = cross(delta, input.surface_right);
    let (len, inv) = length_inverse(cross);
    //830BD350 is initialized by82F826F8 from82181A88, not its zero dump image.
    if len > f32::from_bits(0x3586_37bd) {
        scale(cross, inv)
    } else {
        input.surface_up
    }
}
impl Segments {
    pub fn rebuild(&mut self, input: Input, records: &[Record]) {
        self.items.clear();
        self.last_surface_end = 0;
        for pair in records.windows(2) {
            let start = pair[0].position;
            let end = pair[1].position;
            let delta = sub(end, start);
            let (length, _) = length_inverse(delta);
            if length < f32::from_bits(0x38d1_b717) {
                continue;
            }
            let direction = scale(delta, reciprocal(length));
            let kind = if dot3(input.surface_forward, direction).abs() > 0.5 {
                Kind::Surface
            } else if dot3(input.surface_up, direction) < 0. {
                Kind::Falling
            } else {
                Kind::Rising
            };
            if kind != Kind::Surface {
                if let Some(previous) = self.items.last_mut().filter(|s| s.kind == kind) {
                    previous.end = end;
                    let delta = sub(end, previous.start);
                    previous.length = length_inverse(delta).0;
                    previous.direction = scale(delta, reciprocal(previous.length));
                    previous.normal = normal(delta, input);
                    continue;
                }
            }
            self.items.push(Segment {
                kind,
                start,
                end,
                direction,
                normal: normal(delta, input),
                length,
            });
            if kind == Kind::Surface {
                self.last_surface_end = self.items.len();
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub position: V,
    pub normal: V,
    pub tangent: V,
    pub flags: u32,
    pub minimum_distance: f32,
    pub maximum_distance: f32,
    pub position_distance: f32,
    pub segment_index: Option<usize>,
    pub kind: u32,
}
impl Candidate {
    ///82D82288 uses input forward for a point closer than .02; otherwise the
    ///un-normalized displacement is retained as the candidate tangent.
    pub fn from_point(
        input: Input,
        segments: &Segments,
        position: V,
        normal: V,
        segment_index: Option<usize>,
        flags: u32,
        kind: u32,
    ) -> Self {
        let delta = sub(position, input.position);
        let tangent = if !(length_inverse(delta).0 >= f32::from_bits(0x3ca3_d70a)) {
            input.surface_forward
        } else {
            delta
        };
        Self::new(
            input,
            segments,
            position,
            normal,
            tangent,
            segment_index,
            flags,
            kind,
        )
    }
    /// Caller supplies the actual native segment index; absent means r7=-1.
    pub fn new(
        input: Input,
        segments: &Segments,
        position: V,
        normal: V,
        tangent: V,
        segment_index: Option<usize>,
        flags: u32,
        kind: u32,
    ) -> Self {
        let distance = dot3(input.surface_forward, sub(position, input.position));
        let mut result = Self {
            position,
            normal,
            tangent,
            flags,
            minimum_distance: distance,
            maximum_distance: distance,
            position_distance: distance,
            segment_index,
            kind,
        };
        if let Some(index) = segment_index {
            let segment = segments.items[index];
            if segment.kind != Kind::Surface {
                let endpoint = if segment.kind == Kind::Falling {
                    segment.start
                } else {
                    segment.end
                };
                let edge_distance = dot3(input.surface_forward, sub(endpoint, input.position));
                if edge_distance < distance {
                    result.minimum_distance = edge_distance;
                } else {
                    result.maximum_distance = edge_distance;
                }
            }
        } else {
            result.minimum_distance = 0.;
        }
        result
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
    fn record(y: f32, z: f32) -> Record {
        Record {
            position: [0., y, z, 0.],
            normal: [0., 1., 0., 0.],
            coordinates: [z, y, z, z],
            flags: 0,
            distance: z,
        }
    }
    #[test]
    fn consecutive_vertical_segments_merge_but_surface_segments_remain_distinct() {
        let mut segments = Segments::default();
        segments.rebuild(
            input(),
            &[
                record(0., 0.),
                record(0., 1.),
                record(1., 1.),
                record(2., 1.),
                record(2., 2.),
                record(2., 3.),
            ],
        );
        assert_eq!(segments.items.len(), 4);
        assert_eq!(segments.last_surface_end, 4);
        assert_eq!(segments.items[1].kind, Kind::Rising);
        assert_eq!(segments.items[1].length, 2.);
        assert_eq!(segments.items[1].normal, [0., 0., -1., 0.]);
        assert_eq!(segments.items[0].normal, [0., 1., 0., 0.]);
        segments.rebuild(input(), &[record(0., 0.), record(1., 0.)]);
        assert_eq!(segments.last_surface_end, 0);
    }
    #[test]
    fn candidates_expand_only_toward_the_native_selected_endpoint() {
        let mut segments = Segments::default();
        segments.rebuild(input(), &[record(0., 0.), record(2., 0.5)]);
        let candidate = Candidate::new(
            input(),
            &segments,
            [0., 1., 0.25, 0.],
            [0., 1., 0., 0.],
            [1., 0., 0., 0.],
            Some(0),
            64,
            4,
        );
        assert_eq!(candidate.minimum_distance, 0.25);
        assert_eq!(candidate.maximum_distance, 0.5);
        assert_eq!(candidate.position_distance, 0.25);
        let no_segment = Candidate::new(
            input(),
            &segments,
            [0., 1., 0.25, 0.],
            [0.; 4],
            [0.; 4],
            None,
            0,
            0,
        );
        assert_eq!(no_segment.minimum_distance, 0.);
        assert_eq!(no_segment.maximum_distance, 0.25);
    }
}
