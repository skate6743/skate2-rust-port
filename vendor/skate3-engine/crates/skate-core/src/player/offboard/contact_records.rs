//! BipedToolkit contact insertion82D81F80. Records are classified in the
//! surface forward/up plane; this is not the later obstacle classifier.
use super::contact_queries::{Input, V};
use crate::physics::native_arithmetic::dot3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    pub position: V,
    pub normal: V,
    pub coordinates: V,
    pub flags: u32,
    pub distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source { Probe, Support, Intersection }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { None, Forward, Reverse }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location { Surface(usize), Obstacle(usize) }

#[derive(Default)]
pub struct Records {
    /// Toolkit10832, count14928. Native capacity is64.
    pub surface: Vec<Record>,
    /// Toolkit14944, count19040.
    pub obstacle: Vec<Record>,
    ///14932 captures the insertion count before subsequent classifier edits.
    pub inserted_surface_count: usize,
}

impl Records {
    pub fn clear(&mut self) {
        self.surface.clear(); self.obstacle.clear(); self.inserted_surface_count = 0;
    }

    /// The native routine writes the projected normal even when it rejects
    /// the record. Callers that reuse this normal must observe that write.
    pub fn insert(&mut self, input: Input, position: V, normal: &mut V,
        source: Source, direction: Direction, requested_distance: f32) -> Option<Location>
    {
        let delta = std::array::from_fn(|i| position[i] - input.position[i]);
        let forward = dot3(input.surface_forward, delta);
        let height = dot3(input.surface_up, delta);
        //822FB890 permutes [forward,height,forward,forward].
        let coordinates = [forward, height, forward, forward];
        let elevated = height >= f32::from_bits(0x3f4a_3d71) && forward >= 0.;
        let lateral = dot3(*normal, input.surface_right);
        *normal = std::array::from_fn(|i| normal[i] - input.surface_right[i] * lateral);
        let square = dot3(*normal, *normal);
        let mut inverse = crate::physics::reciprocal_sqrt::estimate(square);
        for _ in 0..2 {
            inverse = (inverse * 0.5).mul_add((-square).mul_add(inverse * inverse, 1.), inverse);
        }
        let length = if square == 0. { 0. } else { square * inverse };
        if !(length >= f32::from_bits(0x3a83_126f)) {
            if !(0. > height) {
                *normal = input.surface_forward.map(|x| f32::from_bits(x.to_bits() ^ 0x8000_0000));
            } else { return None; }
        } else {
            let mut reciprocal = crate::physics::native_arithmetic::reciprocal_estimate(length);
            for _ in 0..2 {
                reciprocal = reciprocal.mul_add((-reciprocal).mul_add(length, 1.), reciprocal);
            }
            *normal = normal.map(|x| x * reciprocal);
        }
        let surface = source == Source::Support || elevated;
        if surface {
            if self.surface.len() == 64 { return None; }
        } else if forward < 0. { return None; }
        let flags = match direction { Direction::None => 0, Direction::Forward => 1, Direction::Reverse => 4 }
            | if elevated { 8 } else { 0 }
            | if source == Source::Intersection { 16 } else { 0 };
        let record = Record { position, normal: *normal, coordinates, flags,
            distance: if requested_distance >= 0. { requested_distance } else { forward } };
        Some(if surface {
            let index = self.surface.len(); self.surface.push(record);
            self.inserted_surface_count = self.surface.len(); Location::Surface(index)
        } else {
            let index = self.obstacle.len(); self.obstacle.push(record); Location::Obstacle(index)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input { position: [0.;4], surface_forward: [0.,0.,1.,0.], surface_up: [0.,1.,0.,0.],
            surface_right: [1.,0.,0.,0.], velocity: [0.;4],
            animation_up: [0.,1.,0.,0.], animation_right: [1.,0.,0.,0.] }
    }
    #[test]
    fn native_branch_boundaries_and_record_words() {
        let mut records = Records::default();
        let mut normal = [0.,1.,0.,0.];
        let threshold = f32::from_bits(0x3f4a_3d71);
        assert_eq!(records.insert(input(),[0.,threshold,2.,0.],&mut normal,
            Source::Intersection,Direction::Reverse,-1.),Some(Location::Surface(0)));
        let r = records.surface[0];
        assert_eq!(r.coordinates,[2.,threshold,2.,2.]);
        assert_eq!(r.flags,28);
        assert_eq!(r.distance,2.);
        assert_eq!(records.insert(input(),[0.,f32::from_bits(threshold.to_bits()-1),2.,0.],&mut normal,
            Source::Probe,Direction::Forward,3.),Some(Location::Obstacle(0)));
        assert_eq!(records.obstacle[0].flags,1);
        assert_eq!(records.obstacle[0].distance,3.);
        assert_eq!(records.insert(input(),[0.,0.,-1.,0.],&mut normal,
            Source::Probe,Direction::None,-1.),None);
        assert_eq!(records.insert(input(),[0.,0.,-1.,0.],&mut normal,
            Source::Support,Direction::None,-1.),Some(Location::Surface(1)));
    }
    #[test]
    fn lateral_normals_use_native_height_dependent_fallback() {
        let mut records = Records::default();
        let mut normal = [1.,0.,0.,0.];
        assert_eq!(records.insert(input(),[0.,-0.1,1.,0.],&mut normal,
            Source::Support,Direction::None,-1.),None);
        assert_eq!(normal,[0.;4]);
        normal = [1.,0.,0.,0.];
        assert_eq!(records.insert(input(),[0.,0.,1.,0.],&mut normal,
            Source::Support,Direction::None,-1.),Some(Location::Surface(0)));
        assert_eq!(normal.map(f32::to_bits),[0x80000000,0x80000000,0xbf800000,0x80000000]);
    }
    #[test]
    fn native_surface_capacity_does_not_limit_obstacles() {
        let mut records = Records::default();
        for _ in 0..64 {
            records.insert(input(),[0.;4],&mut [0.,1.,0.,0.],Source::Support,Direction::None,-1.).unwrap();
        }
        assert_eq!(records.insert(input(),[0.;4],&mut [0.,1.,0.,0.],Source::Support,Direction::None,-1.),None);
        assert_eq!(records.insert(input(),[0.;4],&mut [0.,1.,0.,0.],Source::Probe,Direction::None,-1.),Some(Location::Obstacle(0)));
        assert_eq!(records.inserted_surface_count,64);
        records.clear();
        assert!(records.surface.is_empty() && records.obstacle.is_empty());
        assert_eq!(records.inserted_surface_count,0);
    }
}
