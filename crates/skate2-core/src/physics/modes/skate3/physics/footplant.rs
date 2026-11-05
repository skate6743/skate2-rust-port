//! FootPlantManager state, original TU3 inline ctor82DB217C and Reset82D6F440.
//! Query results are retained separately from pending query ownership.
use skate3_core::air::trajectory::{QueryResult, Trajectory};
use skate_data::collections::Collections;
pub(super) mod math;
mod pose;
mod observations;
pub(crate) use observations::{CandidatePose, GroundStep, PoseTarget};
mod settings;
mod update;
pub(super) mod ground;
use settings::Settings;
pub(crate) use update::FootplantFrame;
pub(crate) type V = [f32; 4];
#[derive(Clone)]
pub(crate) struct Footplant {
    pub enabled: bool, //240; FullReset and Air Enter clear this separately.
    pub(super) result: QueryResult,
    settings: Settings,
    pub selected_toe: Option<usize>, //616 signed -1
    pub candidate: bool,             //624
    pub hit: bool,                   //625
    pub perform: bool,               //626
    pub flag_627: bool,
    pub launch_valid: bool,   //628
    pub lock_valid: bool,     //629
    pub contact_active: bool, //630
    pub requested: bool,      //631
    pub contact_time: f32,    //592
    pub scalar_596: f32,
    pub scalar_600: f32,
    pub target_blend: f32,   //604
    pub active_elapsed: f32, //608
    pub scalar_612: f32,
    pub surface: u32,              //620 survives Reset
    pub contact: V,                //288
    pub adjusted_contact: V,       //304
    pub(super) current_up: V,      //320: Reset preserves it; ctor does not define it
    pub(super) selected_world: V,  //256
    pub(super) selected_record: V, //272
    pub(super) leg_direction: V,   //336
    pub(super) vectors_352_368: [V; 2],
    pub(super) curve: [V; 4], //448/464/480/496
    pub(super) physical_com: V,     //384
    pub(super) animation_com: V,    //400
    pub(super) launch_direction: V, //416
    pub(super) locked_target: V,    //432
    pub(super) request: Trajectory, //528..591
    pub(super) completed_trajectory: Trajectory, //query144, retained with result
}
impl Footplant {
    pub(crate) fn load_plant_wipeout_settings(data:&Collections)->Result<skate3_core::player::wipeout::Settings,String> {
        super::wipeout::plant_settings(data)
    }
    pub fn load(data: &Collections) -> Result<Self, String> {
        //Deterministic host storage for fields the native ctor leaves unwritten.
        //The first-ever320 value is not claimed as native memory parity.
        let mut state = Self {
            enabled: false,
            result: QueryResult::miss(),
            settings: Settings::load(data)?,
            selected_toe: None,
            candidate: false,
            hit: false,
            perform: false,
            flag_627: false,
            launch_valid: false,
            lock_valid: false,
            contact_active: false,
            requested: false,
            contact_time: -1.0,
            scalar_596: -1.0,
            scalar_600: 0.0,
            target_blend: 0.0,
            active_elapsed: 0.0,
            scalar_612: 0.0,
            surface: 0,
            contact: [0.0; 4],
            adjusted_contact: [0.0; 4],
            current_up: [0.0; 4],
            selected_world: [0.0; 4],
            selected_record: [0.0; 4],
            leg_direction: [0.0; 4],
            vectors_352_368: [[0.0; 4]; 2],
            curve: [[0.0; 4]; 4],
            physical_com: [0.0; 4],
            animation_com: [0.0; 4],
            launch_direction: [0.0; 4],
            locked_target: [0.0; 4],
            request: Trajectory {
                position: [0.0; 4],
                velocity: [0.0; 4],
                acceleration: [0.0; 4],
                duration: -1.0,
            },
            completed_trajectory: Trajectory { position: [0.0;4], velocity: [0.0;4], acceleration: [0.0;4], duration: -1.0 },
        };
        state.reset();
        Ok(state)
    }
    ///82D6F440. Does not cancel the query or erase its last delivered result,
    ///current up, selected positions, or surface. Enter clears240 separately.
    pub fn reset(&mut self) {
        self.contact_time = -1.0;
        self.scalar_596 = -1.0;
        self.selected_toe = None;
        self.scalar_600 = 0.0;
        self.target_blend = 0.0;
        self.active_elapsed = 0.0;
        self.scalar_612 = 0.0;
        self.candidate = false;
        self.hit = false;
        self.perform = false;
        self.flag_627 = false;
        self.launch_valid = false;
        self.lock_valid = false;
        self.contact_active = false;
        self.requested = false;
        self.request = Trajectory {
            position: [0.0; 4],
            velocity: [0.0; 4],
            acceleration: [0.0; 4],
            duration: -1.0,
        };
        self.contact = [0.0; 4];
        self.adjusted_contact = [0.0; 4];
        self.vectors_352_368 = [[0.0; 4]; 2];
        self.animation_com = [0.0; 4];
        self.locked_target = [0.0; 4];
        self.launch_direction = [0.0; 4];
        self.physical_com = [0.0; 4];
    }
    pub fn full_reset(&mut self) {
        self.enabled = false;
        self.reset();
    }
    /// Common ProcessOutput82DB71B0 calls82D71040 before selected state Fill.
    pub fn publish(&self, out: &mut skate3_core::player::input_phase::AirOutputFields) {
        if self.hit {
            out.footplant_contact_time_208 = self.contact_time;
            out.footplant_duration_212 = self.scalar_596;
            out.flag_447 = u8::from(self.perform);
            out.flag_448 = u8::from(self.perform || self.flag_627);
            out.footplant_surface_224 = self.surface;
            out.footplant_left_449 = u8::from(self.flag_627 && self.selected_toe == Some(15));
            out.footplant_right_450 = u8::from(self.flag_627 && self.selected_toe == Some(19));
            out.footplant_surface_height_216 = self.contact[1];
        }
    }
}

#[cfg(test)]
impl Footplant {
    pub(crate) fn test_release_velocity(&self)->V {
        math::scale(self.vectors_352_368[0],self.scalar_612)
    }
}
