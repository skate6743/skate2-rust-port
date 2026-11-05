//! Shared collision-policy controller used by Ground and both Air states.
//! Requests82D911A8 and ascending/falling update82D91660 precede the solve.
use skate3_core::physics::skeleton_body::SkeletonCollisionMode;

pub(crate) struct SkeletonControllerState {
    pub effective: u32,
    pub requested: u32,
    pub has_request: bool,
    pub override_enabled: bool,
    ///Native byte18 tracks upward motion/jump input; it is not an enable bit.
    pub flag_18: bool,
}
impl SkeletonControllerState {
    pub fn new() -> Self {
        //Original inline constructor82DB2498..24B0.
        Self {
            effective: 0,
            requested: 0,
            has_request: false,
            override_enabled: false,
            flag_18: false,
        }
    }

    pub fn request_ground(&mut self, collision: &mut SkeletonCollisionMode) -> Result<(), String> {
        self.request(6, collision)
    }

    pub fn request(
        &mut self,
        requested: u32,
        collision: &mut SkeletonCollisionMode,
    ) -> Result<(), String> {
        if let Some(mode) = self.select_request(requested) {
            collision.select_driven(mode).map_err(str::to_owned)?;
        }
        Ok(())
    }

    ///Original request gate shared by driven and physical ragdoll adapters.
    pub(crate) fn select_request(&mut self, requested: u32) -> Option<u32> {
        if self.has_request && self.requested == requested {
            return None;
        }
        let effective = if self.override_enabled {
            self.requested = requested;
            self.has_request = true;
            //82D911D8..1264: unsigned requested-4 indexes the eight cases.
            match requested {
                4 => 1,
                5..=7 => 2,
                8 => 3,
                9..=11 => requested,
                _ => return None,
            }
        } else {
            if (1..=2).contains(&self.effective) {
                return None;
            }
            requested
        };
        self.effective = effective;
        let mode = effective.wrapping_sub(1);
        if mode > 10 {
            return None;
        }
        Some(mode)
    }

    ///Original82D91660 changes the collision request only when byte18 changes.
    pub fn update_air(
        &mut self,
        processed_flags_2472: u32,
        velocity_y: f32,
        collision: &mut SkeletonCollisionMode,
    ) -> Result<(), String> {
        let ascending = processed_flags_2472 & 0x8000 != 0 || velocity_y > 0.0;
        if self.flag_18 != ascending {
            self.flag_18 = ascending;
            self.request(if ascending { 7 } else { 6 }, collision)?;
        }
        Ok(())
    }
}
