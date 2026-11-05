//! BipedToolkit query initialization82D856C8 and submission geometry82D811C8.
//! These are contact probes, separate from the seven-line ledge query.
use crate::physics::native_arithmetic::dot3;
pub type V = [f32; 4];
#[cfg(test)]
mod reference;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Probe {
    pub start: V,
    pub end: V,
    pub radius: f32,
    pub id: usize,
    pub reverse_id: Option<usize>,
}

pub struct Layout {
    pub support: [Probe; 3],
    pub vertical: Vec<Probe>,
    pub horizontal: Vec<Probe>,
}
impl Default for Layout {
    fn default() -> Self {
        let point = |x, y, z| [x, y, z, 0.];
        let probe = |start, end, radius, id, reverse_id| Probe { start, end, radius, id, reverse_id };
        let height = f32::from_bits(0x3f4c_cccd);
        let support = [
            probe(point(0., f32::from_bits(0x3f99_999a), 0.), point(0., -height, 0.),
                f32::from_bits(0x3c23_d70a), 0, None),
            probe(point(f32::from_bits(0xbdf5_c28f), height, 0.),
                point(f32::from_bits(0xbe2e_147b), f32::from_bits(0xbe4c_cccd), 0.),
                f32::from_bits(0x3cf5_c28f), 1, None),
            probe(point(f32::from_bits(0x3df5_c28f), height, 0.),
                point(f32::from_bits(0x3e2e_147b), f32::from_bits(0xbe4c_cccd), 0.),
                f32::from_bits(0x3cf5_c28f), 2, None),
        ];
        let mut vertical = Vec::new();
        let mut z = f32::from_bits(0x3d19_999a);
        // The loop compares the incremented value, preserving accumulated
        // float rounding. Computing z from index would change later probes.
        loop {
            vertical.push(probe(point(0., height, z), point(0., -height, z),
                0., vertical.len(), None));
            z += f32::from_bits(0x3d99_999a);
            if !(z < f32::from_bits(0x3fe1_9999)) { break; }
        }
        let mut id = vertical.len();
        let mut horizontal = Vec::new();
        for n in 0..12 {
            let fraction = n as f32 * f32::from_bits(0x3dba_2e8c);
            let y = fraction.mul_add(height, -((1. - fraction) * height));
            let reverse_id = (y < 0.).then_some(id + 1);
            horizontal.push(probe(point(0., y, 0.), point(0., y, f32::from_bits(0x3fe6_6666)),
                0., id, reverse_id));
            id += if reverse_id.is_some() { 2 } else { 1 };
        }
        let radius = f32::from_bits(0x3dcc_cccc);
        for n in 0..3 {
            let y = (n as f32).mul_add(2., 1.).mul_add(radius, height);
            horizontal.push(probe(point(0., y, radius), point(0., y, f32::from_bits(0x3fd9_9999)),
                radius, id, None));
            id += 1;
        }
        Self { support, vertical, horizontal }
    }
}

/// Seven-vector input populated by Ground Sync82D31DC0 and copied by811C8.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub position: V,
    pub surface_forward: V,
    pub surface_up: V,
    pub surface_right: V,
    pub velocity: V,
    pub animation_up: V,
    pub animation_right: V,
}
impl Input {
    fn transform(self, p: V, animation: bool) -> V {
        let (x, y, z) = if animation {
            let a = self.animation_right;
            let b = self.animation_up;
            let forward = [
                (-a[2]).mul_add(b[1], a[1] * b[2]),
                (-a[0]).mul_add(b[2], a[2] * b[0]),
                (-a[1]).mul_add(b[0], a[0] * b[1]),
                (-a[3]).mul_add(b[3], a[3] * b[3]),
            ];
            (a, b, forward)
        } else { (self.surface_right, self.surface_up, self.surface_forward) };
        std::array::from_fn(|i| z[i].mul_add(p[2], y[i].mul_add(p[1], x[i].mul_add(p[0], self.position[i]))))
    }
}

impl Layout {
    /// Query families have separate native result arrays. IDs are local to
    /// their provider. Reverse obstacle segments are submitted immediately
    /// after their corresponding forward segment.
    pub fn world_probes(&self, input: Input) -> ([Probe; 3], Vec<Probe>) {
        let transform = |p: Probe, animation| Probe {
            start: input.transform(p.start, animation), end: input.transform(p.end, animation), ..p
        };
        let support = std::array::from_fn(|i| transform(self.support[i], i != 0));
        let mut lines = Vec::with_capacity(self.vertical.len() + self.horizontal.len() * 2);
        for p in self.vertical.iter().chain(&self.horizontal) {
            let q = transform(*p, false);
            lines.push(q);
            if let Some(id) = q.reverse_id {
                lines.push(Probe { start: q.end, end: q.start, id, reverse_id: None, ..q });
            }
        }
        (support, lines)
    }
}

/// Side support acceptance82D85660: open interval along the surface up axis.
pub fn accepts_side_support(input: Input, position: V) -> bool {
    let delta = std::array::from_fn(|i| position[i] - input.position[i]);
    let distance = dot3(delta, input.surface_up);
    distance < f32::from_bits(0x3e4c_cccd) && distance > f32::from_bits(0xbe4c_cccd)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_query_layout_retains_counts_ids_and_rounding() {
        let l = Layout::default();
        let actual: Vec<[u32;11]> = l.support.iter().chain(&l.vertical).chain(&l.horizontal)
            .map(|p| { let mut row=[0;11]; row[..4].copy_from_slice(&p.start.map(f32::to_bits));
                row[4..8].copy_from_slice(&p.end.map(f32::to_bits));
                row[8]=p.radius.to_bits();row[9]=p.id as u32;row[10]=p.reverse_id.map_or(u32::MAX,|id|id as u32);row }).collect();
        assert_eq!(actual, reference::EXPECTED);
        assert_eq!((l.vertical.len(), l.horizontal.len()), (23, 15));
        assert_eq!(l.vertical[22].start[2].to_bits(), 0x3fd8_0003);
        assert_eq!(l.horizontal[0].id, 23);
        assert_eq!(l.horizontal[0].reverse_id, Some(24));
        assert_eq!(l.horizontal[6].id, 35);
        assert_eq!(l.horizontal[6].reverse_id, None);
        assert_eq!(l.horizontal[14].id, 43);
        let input = Input { position: [10.,2.,30.,0.], surface_right: [1.,0.,0.,0.],
            surface_up: [0.,1.,0.,0.], surface_forward: [0.,0.,1.,0.],
            animation_right: [0.,0.,-1.,0.], animation_up: [0.,1.,0.,0.], velocity: [0.;4] };
        let (support, rays) = l.world_probes(input);
        assert_eq!(rays.len(), 44);
        assert_eq!(support[0].start, [10.,3.2,30.,0.]);
        assert_eq!(support[1].start[0], 10.);
        assert!(support[1].start[2] > 30.);
        assert_eq!(rays[23].start, rays[24].end);
        assert_eq!(rays[23].end, rays[24].start);
        assert_eq!((rays[23].id,rays[24].id), (23,24));
    }
    #[test]
    fn side_support_has_strict_native_height_limits() {
        let input = Input { position:[0.;4],surface_up:[0.,1.,0.,0.],
            surface_right:[1.,0.,0.,0.],surface_forward:[0.,0.,1.,0.],
            animation_right:[1.,0.,0.,0.],animation_up:[0.,1.,0.,0.],velocity:[0.;4] };
        assert!(accepts_side_support(input,[0.,0.199,0.,0.]));
        assert!(!accepts_side_support(input,[0.,0.2,0.,0.]));
        assert!(!accepts_side_support(input,[0.,-0.2,0.,0.]));
        assert!(!accepts_side_support(input,[0.,f32::NAN,0.,0.]));
    }
}
