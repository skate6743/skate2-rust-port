//! Contact packet reset82D2DDD0 and primary support consumption in
//!82D81610..82D818A0. Later obstacle classifiers amend this packet.
use super::{contact_queries::{Input, V, accepts_side_support},
    contact_records::{Records, Source, Direction}, controller::Frame};
use crate::physics::native_arithmetic::dot3;

#[derive(Clone, Copy, Debug)]
pub struct SupportHit {
    pub position: V,
    pub normal: V,
    pub frame: Frame,
    /// Original query result136, kept opaque at this boundary.
    pub support_id: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Packet {
    pub position: V,
    pub normal: V,
    pub support_frame: Frame,
    pub target_position: V,
    pub target_normal: V,
    pub edge_position: V,
    pub edge_normal: V,
    pub value_160: f32,
    pub kind_164: u32,
    pub distance_168: f32,
    pub distance_172: f32,
    pub flags: u32,
    pub support_id: u32,
}
impl Default for Packet {
    fn default() -> Self {
        Self { position: [0.;4], normal: [0.,1.,0.,0.],
            support_frame: [[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.;4]],
            target_position: [0.;4], target_normal: [0.,1.,0.,0.],
            edge_position: [0.;4], edge_normal: [0.;4], value_160: 0., kind_164: 0,
            distance_168: f32::from_bits(0x5015_02f9), distance_172: f32::from_bits(0x5015_02f9),
            flags: 0, support_id: 0 }
    }
}
impl Packet {
    /// Called after reset and completion of the pending native queries.
    /// Returns the three query hit bytes written at10652/10732/10812.
    pub fn consume_support(&mut self, input: Input, hits: [Option<SupportHit>;3],
        previous_flags_30416: u32, records: &mut Records) -> [bool;3]
    {
        let mut accepted = [false;3];
        if let Some(hit) = hits[0] {
            let mut projected_normal = hit.normal;
            records.insert(input,hit.position,&mut projected_normal,Source::Support,Direction::None,-1.);
            self.position = hit.position;
            self.normal = hit.normal;
            self.support_frame = hit.frame;
            self.flags |= 1;
            let delta = std::array::from_fn(|i| hit.position[i] - input.position[i]);
            let height = dot3(input.surface_up,delta);
            if !(height >= f32::from_bits(0xbdcc_cccd)) {
                self.flags |= 8;
                if previous_flags_30416 & 64 != 0 { self.flags |= 64; }
            } else { self.flags &= !8; }
            self.support_id = hit.support_id;
            accepted[0] = true;
        }
        for i in 1..3 {
            if let Some(hit) = hits[i] {
                if accepts_side_support(input,hit.position) {
                    accepted[i] = true;
                    self.flags |= if i == 1 { 16 } else { 32 };
                    // The original compares world Y, not the input up projection.
                    if hits[0].is_some() && hit.normal[1] > self.normal[1] {
                        self.normal = hit.normal;
                    }
                }
            }
        }
        accepted
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
    fn hit(height: f32, normal_y: f32) -> SupportHit {
        SupportHit { position: [0.,height,0.,0.], normal: [0.,normal_y,1.,0.],
            frame: Packet::default().support_frame, support_id: 0x12345678 }
    }
    #[test]
    fn side_hits_cannot_create_central_support_or_change_its_position() {
        let mut packet = Packet::default();
        let mut records = Records::default();
        let side = hit(0.,1.);
        assert_eq!(packet.consume_support(input(),[None,Some(side),None],0,&mut records),[false,true,false]);
        assert_eq!(packet.flags,16);
        assert_eq!(packet.support_id,0);
        assert!(records.surface.is_empty());
        packet = Packet::default();
        packet.consume_support(input(),[Some(hit(-0.1,0.5)),Some(side),Some(hit(0.2,2.))],0,&mut records);
        assert_eq!(packet.flags,17);
        assert_eq!(packet.position,[0.,-0.1,0.,0.]);
        assert_eq!(packet.normal,side.normal);
        assert_eq!(packet.support_id,0x12345678);
        // Insertion retains the original centre normal before side improvement.
        assert!(records.surface[0].normal[1] < 0.5);
    }
    #[test]
    fn deep_support_inherits_bit64_only_past_the_native_threshold() {
        let threshold = f32::from_bits(0xbdcc_cccd);
        for (height, previous, expected) in [
            (threshold,64,1),(f32::from_bits(threshold.to_bits()+1),0,9),
            (f32::from_bits(threshold.to_bits()+1),64,73)
        ] {
            let mut packet = Packet::default();
            packet.consume_support(input(),[Some(hit(height,1.)),None,None],previous,&mut Records::default());
            assert_eq!(packet.flags,expected);
        }
    }
}
