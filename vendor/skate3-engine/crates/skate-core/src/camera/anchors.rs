//! Seven normal-camera anchors created by TU382DF2DB8 and updated82DF69C0.
//! Names come from that constructor; notably HipsAnchor reads reckoned COM.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorInputs {
    /// Subject388: physical output entry0+144.
    pub board_position: [f32; 4],
    /// Subject392/396: board output entry1+80/+64 respectively.
    pub board_velocity: [f32; 4],
    pub board_acceleration: [f32; 4],
    /// Subject456/460/464: reckoning direction, COM and filtered COM.
    pub board_offset_direction: [f32; 4],
    pub center_of_mass: [f32; 4],
    pub damped_center_of_mass: [f32; 4],
    /// Subject384 matrix+16 and436.
    pub skeleton_root_up: [f32; 4],
    pub grind_point: [f32; 4],
    /// Subject596/544; reset is state output byte62.
    pub grinding: bool,
    pub reset: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnchorState {
    pub position: [f32; 4],
    pub velocity: [f32; 4],
    pub acceleration: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Anchors {
    /// Skateboard, Hips, Air, Grind, World, DampedCOM, POI in native order.
    pub entries: [AnchorState; 7],
    grind_position: [f32; 4],
}

impl Anchors {
    pub fn new() -> Self {
        Self {
            entries: [AnchorState {
                position: [0.0; 4],
                velocity: [0.0; 4],
                acceleration: [0.0; 4],
            }; 7],
            grind_position: [0.0; 4],
        }
    }

    pub fn update(&mut self, input: AnchorInputs) {
        // Publisher82DF7790 always passes this fixed timestep to every anchor,
        // independently of CameraMan's render update timestep.
        let dt = f32::from_bits(0x3c888889);
        let inverse = super::vector_tracker::refined_reciprocal(dt);
        let board = core::array::from_fn(|i| {
            input.board_offset_direction[i]
                .mul_add(f32::from_bits(0x3e99999a), input.board_position[i])
        });
        let air = core::array::from_fn(|i| {
            input.center_of_mass[i] - input.skeleton_root_up[i] * f32::from_bits(0x3ea8f5c3)
        });
        // GrindAnchor82DF2C78 retains and advects its point after leaving a rail.
        if input.reset || input.grinding {
            let point = if input.reset {
                input.board_position
            } else {
                input.grind_point
            };
            self.grind_position =
                core::array::from_fn(|i| input.board_offset_direction[i].mul_add(0.5, point[i]));
        } else {
            self.grind_position = core::array::from_fn(|i| {
                input.board_velocity[i].mul_add(dt, self.grind_position[i])
            });
        }
        let positions = [
            board,
            input.center_of_mass,
            air,
            self.grind_position,
            [0.0; 4],
            input.damped_center_of_mass,
            [0.0; 4],
        ];
        for (index, (state, position)) in self.entries.iter_mut().zip(positions).enumerate() {
            // Common82DF2950: first publish position, then derive velocity;
            // acceleration comes from a separate getter and is not differenced.
            let previous = state.position;
            state.position = position;
            state.velocity = if input.reset {
                [0.0; 4]
            } else {
                core::array::from_fn(|i| inverse * (position[i] - previous[i]))
            };
            state.acceleration = if matches!(index, 0 | 2 | 3) {
                input.board_acceleration
            } else {
                [0.0; 4]
            };
        }
    }
}
