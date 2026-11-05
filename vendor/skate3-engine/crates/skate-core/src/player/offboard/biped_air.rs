//! Pre-migration runtime restored for the Ground build checkpoint.
//! Its known Air lifecycle defects are NOT repaired or validated by this rollback.
//! The separately compiled recovered module is the unfinished replacement.
//! Persistent BipedAir owner storage and lifecycle publication.
//!
//! The native object retains the launch packet, cadence, phase timing and
//! output vectors across Enter/Update/Post/Output/Exit.  Keeping that state in
//! one owner prevents the old failure mode where a transient launch result was
//! mistaken for the whole off-board state.
use super::{air_launch::Packet, cadence::BipedCadence};

pub type Vector = [f32; 4];
pub type Frame = [Vector; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Input {
    pub position_592: Vector,
    pub velocity_608: Vector,
    pub up_544: Vector,
    pub forward_224: Vector,
    pub board_position_112: Vector,
    pub departure_geometry: Option<super::air_launch::DepartureGeometry>,
    pub velocity_912: Vector,
    pub flags_2472: u32,
    pub flags_2476: u32,
    pub flags_2480: u32,
    pub previous_state_2504: u32,
    pub current_state_2508: u32,
    pub current_category_2512: u32,
    pub previous_category_2516: u32,
    pub raw_x_2692: f32,
    pub raw_z_2688: f32,
    pub current: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Output {
    pub packet: Packet,
    pub position_288: Vector,
    pub velocity_304: Vector,
    pub phase_444: f32,
    pub phase_counter_448: f32,
    pub update_count_452: u32,
    pub landed_404: bool,
    pub wipeout_547: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub packet: Packet,
    pub cadence: BipedCadence,
    pub position_288: Vector,
    pub velocity_304: Vector,
    pub phase_444: f32,
    pub phase_counter_448: f32,
    pub update_count_452: u32,
    pub landed_404: bool,
    pub wipeout_547: bool,
    pub active: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            packet: Packet::initialized(0.0),
            cadence: BipedCadence::default(),
            position_288: [0.0; 4],
            velocity_304: [0.0; 4],
            phase_444: 0.0,
            phase_counter_448: 0.0,
            update_count_452: 0,
            landed_404: false,
            wipeout_547: false,
            active: false,
        }
    }
}

impl State {
    ///82D2E5C0: clear the retained landing/output flags and enter with a
    ///fresh launch packet.  The caller supplies the actual retained scalar.
    pub fn enter(&mut self, retained_packet_scalar_104: f32, position: Vector) {
        self.packet = Packet::initialized(retained_packet_scalar_104);
        self.position_288 = position;
        self.velocity_304 = [0.0; 4];
        self.phase_444 = 0.0;
        self.phase_counter_448 = 0.0;
        self.update_count_452 = 0;
        self.landed_404 = false;
        self.wipeout_547 = false;
        self.active = true;
    }

    ///82D2EF20: the native exit clears the trajectory selector and leaves no
    ///live BipedAir publication behind.
    pub fn exit(&mut self) {
        self.active = false;
        self.landed_404 = false;
    }

    pub fn update(
        &mut self,
        input: &Input,
        controller: &super::controller::State,
        turn_vs_speed: &crate::point_graph::PointGraph<8>,
        settings: super::air_launch::Settings,
    ) -> Result<(), &'static str> {
        if !self.active {
            return Ok(());
        }
        let processed = super::air_launch::Processed {
            board_position_112: input.board_position_112,
            forward_224: input.forward_224,
            up_544: input.up_544,
            position_592: input.position_592,
            velocity_608: input.velocity_608,
            velocity_912: input.velocity_912,
            departure_geometry: input.departure_geometry,
            flags_2472: input.flags_2472,
            flags_2476: input.flags_2476,
            flags_2480: input.flags_2480,
            previous_state_2504: input.previous_state_2504,
            current_state_2508: input.current_state_2508,
            current_category_2512: input.current_category_2512,
            previous_category_2516: input.previous_category_2516,
            raw_x_2692: input.raw_x_2692,
            raw_z_2688: input.raw_z_2688,
        };
        super::air_launch::produce(
            &mut self.packet,
            controller,
            turn_vs_speed,
            settings,
            &processed,
            input.current,
        )?;
        self.position_288 = self.packet.position_32;
        self.velocity_304 = self.packet.velocity_0;
        self.phase_counter_448 += f32::from_bits(0x3c888889);
        self.update_count_452 = self.update_count_452.wrapping_add(1);
        Ok(())
    }

    pub fn output(&self) -> Output {
        Output {
            packet: self.packet,
            position_288: self.position_288,
            velocity_304: self.velocity_304,
            phase_444: self.phase_444,
            phase_counter_448: self.phase_counter_448,
            update_count_452: self.update_count_452,
            landed_404: self.landed_404,
            wipeout_547: self.wipeout_547,
        }
    }
}

pub mod recovered;
