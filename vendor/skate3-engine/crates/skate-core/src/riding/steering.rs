//! TU3 steering tilt (`0x82D92440`) and truck targets (`0x82C040F0`).
use crate::point_graph::PointGraph;

#[derive(Clone, Copy, Debug)]
pub struct SteeringSettings {
    pub hard_turn_increase: f32,
    pub damping: f32,
    pub speed_graph_max_speed: f32,
    pub push_scalar_increment: f32,
    pub push_scalar_decrement: f32,
    pub push_scalar_min: f32,
    pub manual_scalar: f32,
    pub general_scalar: f32,
    pub tight_trucks_scalar: f32,
    pub speed_graph: PointGraph<8>,
    pub input_graph: PointGraph<8>,
    pub tilt_blending: f32,
}

/// Field names describe the reads from `ProcessedPhysIn`. No yaw-rate or
/// bicycle steering interpretation is applied to the returned tilt.
#[derive(Clone, Copy, Debug, Default)]
pub struct SteeringInput {
    pub turn: f32,
    pub hard_turn: f32,
    pub absolute_body_speed: f32,
    pub flipped_controls_scalar: f32,
    pub balance: f32,
    pub truck_tightness: f32,
    /// `ProcessedPhysIn+2468`, bit 27 (distinct from push-force bit 25).
    pub pushing: bool,
}

/// Optional pointers in the native function are represented independently.
/// Updates are per physics call; the native routine has no timestep argument.
pub fn calculate_tilt(
    settings: &SteeringSettings,
    input: SteeringInput,
    push_scalar: Option<&mut f32>,
    damped_turn: Option<&mut f32>,
) -> f32 {
    let hard_scalar = input
        .hard_turn
        .abs()
        .mul_add(settings.hard_turn_increase, 1.0);
    let mut turn = input.turn;
    if input.hard_turn != 0.0 {
        turn = if turn >= 0.0 { 1.0 } else { -1.0 };
    }
    if let Some(previous) = damped_turn {
        turn = (1.0 - settings.damping).mul_add(*previous, settings.damping * turn);
        *previous = turn;
    }
    let speed_scalar = settings
        .speed_graph
        .evaluate((input.absolute_body_speed / settings.speed_graph_max_speed).clamp(0.0, 1.0));
    let input_scalar = settings.input_graph.evaluate(turn.abs());
    let push = if let Some(previous) = push_scalar {
        let next = if input.pushing {
            *previous - settings.push_scalar_decrement
        } else {
            settings.push_scalar_increment + *previous
        };
        *previous = next.clamp(settings.push_scalar_min, 1.0);
        *previous
    } else {
        1.0
    };
    let manual = if input.balance != 0.0 {
        settings.manual_scalar
    } else {
        1.0
    };
    let tightness = settings
        .tight_trucks_scalar
        .mul_add(input.truck_tightness, 1.0)
        - input.truck_tightness;
    input.flipped_controls_scalar
        * settings.general_scalar
        * manual
        * tightness
        * push
        * input_scalar
        * speed_scalar
        * hard_scalar
        * turn
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TruckSteeringState {
    pub deck_tilt: f32,
    /// Native `SkateboardBody+7680/+7684` order, not stance-relative labels.
    pub targets: [f32; 2],
    pub activation_time: [f32; 2],
}

impl TruckSteeringState {
    /// Flags are retained in their native words to make stance-dependent
    /// contact ownership explicit (`+2468` bit 20, `+2472` bits 26 and 27).
    pub fn update(&mut self, target: f32, blend: f32, flags_2468: u32, flags_2472: u32) {
        self.deck_tilt = (1.0 - blend).mul_add(self.deck_tilt, blend * target);
        let mut active = [flags_2472 & (1 << 27) != 0, flags_2472 & (1 << 26) != 0];
        if flags_2468 & (1 << 20) != 0 {
            active.swap(0, 1);
        }
        for (index, enabled) in active.into_iter().enumerate() {
            let timer = &mut self.activation_time[index];
            let angle = &mut self.targets[index];
            if enabled {
                if *timer <= 0.16500001 {
                    *timer += f32::from_bits(0x3C88_8889);
                    let fraction = (*timer * 6.060606).clamp(0.0, 1.0);
                    *angle = (1.0 - fraction).mul_add(*angle, fraction * self.deck_tilt);
                } else {
                    *angle = self.deck_tilt;
                }
            } else {
                // The first inactive frame holds the angle; subsequent frames decay.
                if *timer == 0.0 {
                    *angle *= 0.983;
                }
                *timer = 0.0;
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/steering.rs"]
mod tests;
