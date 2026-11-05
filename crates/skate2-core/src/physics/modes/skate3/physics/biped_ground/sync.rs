//! Ground-owned Sync stages. The caller executes returned launch on its ONE
//! persistent AirSelector before frame correction; no speculative transition.
use super::Owner;
use skate3_core::player::offboard::{air_launch, ground_entry::Vector};

pub(crate) struct LaunchInput {
    pub processed: air_launch::Processed,
    pub elapsed_2664: f32,
    pub skeleton_point_10960: Vector,
}
impl Owner {
    ///82D32338. This writes state146 on every call, including the no-launch path.
    pub(crate) fn prepare_air(
        &mut self,
        input: &LaunchInput,
    ) -> Result<Option<air_launch::Packet>, String> {
        let motion = self
            .result
            .ok_or("Ground Sync requires completed Ground job")?;
        let p = &input.processed;
        let flags = self.contact.flags_176;
        let mut launch = flags & 1 == 0 && input.elapsed_2664 > f32::from_bits(0x3d4ccccd);
        launch |= motion.alternate;
        if flags & 8 == 0 && p.flags_2480 & 0x20180 == 0 && !self.ground.flags_144_to_150[6] {
            launch |= p.flags_2476 & 0x80000 != 0;
        }
        self.ground.flags_144_to_150[2] = launch;
        if !launch {
            return Ok(None);
        }
        //104 is uninitialized by82D2DB98 and overwritten on every produce path.
        let mut packet = air_launch::Packet::initialized(0.);
        air_launch::produce(
            &mut packet,
            &self.controller.state,
            &self.controller.settings.movement_velocity.turn_vs_speed,
            self.air_launch,
            p,
            false,
        )
        .map_err(str::to_owned)?;
        let frame = motion.animation_frame;
        let point = input.skeleton_point_10960;
        packet.position_32 = std::array::from_fn(|i| {
            frame[2][i].mul_add(
                point[2],
                frame[1][i].mul_add(point[1], frame[0][i].mul_add(point[0], frame[3][i])),
            )
        });
        if !packet.position_32[3].is_finite() {
            packet.position_32[3] = 0.0;
        }
        Ok(Some(packet))
    }
}
