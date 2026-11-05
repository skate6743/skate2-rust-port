//! Skateboard::PrepareBoardToolkit82C013F0, TU3. Derived physical axes and
//! filtered normal are calculated from the same live deck used by the solver.
use super::{
    board::BodyId, board_motion_output::inverse_length_squared, board_runtime::BoardRuntime,
    force_queue::total_body_mass, native_arithmetic::dot3,
};

#[derive(Clone, Copy, Debug)]
pub struct BoardToolkit {
    pub deck: [[f32; 4]; 4],
    pub effective: [[f32; 4]; 4],
    pub inverse_effective: [[f32; 4]; 4],
    pub side: [f32; 4],
    pub up: [f32; 4],
    pub forward: [f32; 4],
    pub horizontal_forward: [f32; 4],
    pub transverse_up: [f32; 4],
    pub forward_velocity: [f32; 4],
    pub travel_direction: [f32; 4],
    pub filtered_normal: [f32; 4],
    pub absolute_speed: f32,
    pub control_sign: f32,
    pub total_mass: f32,
}
impl BoardToolkit {
    /// Board192 is retained by the owner and replaced by filtered_normal after
    /// this call. Reset must supply the recovered retained normal, not derive
    /// grounded state from board height.
    pub fn from_board(
        board: &BoardRuntime,
        flags_2468: u32,
        speed_2612: f32,
        ground_normal_464: [f32; 4],
        retained_normal: [f32; 4],
    ) -> Self {
        let part = board.part_transforms()[BodyId::Deck.index()];
        let axis = |i: usize| {
            let c = part.basis.columns[i];
            [c[0], c[1], c[2], 0.0]
        };
        let deck = [
            axis(0),
            axis(1),
            axis(2),
            [
                part.translation.x,
                part.translation.y,
                part.translation.z,
                1.0,
            ],
        ];
        let masses = board.bodies().map(|body| body.inertia.inverse_mass);
        Self::calculate(
            deck,
            &masses,
            flags_2468,
            speed_2612,
            ground_normal_464,
            retained_normal,
        )
    }
    pub fn calculate(
        deck: [[f32; 4]; 4],
        inverse_masses: &[f32],
        flags: u32,
        speed: f32,
        normal: [f32; 4],
        retained_normal: [f32; 4],
    ) -> Self {
        let control_sign = if flags & 0x0010_0000 != 0 { -1.0 } else { 1.0 };
        let mut effective = deck;
        for axis in [0, 2] {
            effective[axis] = deck[axis].map(|v| v * control_sign);
        }
        //82C0149C..150C transposes XYZ, clears all W lanes, and composes the
        //negative translation in Z,Y,X fused order. Native inverse W is zero.
        let mut inverse_effective = [[0.0; 4]; 4];
        for axis in 0..3 {
            for lane in 0..3 {
                inverse_effective[axis][lane] = effective[lane][axis];
            }
        }
        for lane in 0..4 {
            let z = (0.0 - effective[3][2]) * inverse_effective[2][lane];
            let yz = (0.0 - effective[3][1]).mul_add(inverse_effective[1][lane], z);
            inverse_effective[3][lane] =
                (0.0 - effective[3][0]).mul_add(inverse_effective[0][lane], yz);
        }
        let side = effective[0];
        let up = deck[1];
        let forward = effective[2];
        let mut horizontal_forward = forward;
        horizontal_forward[1] = 0.0;
        let horizontal_squared = dot3(horizontal_forward, horizontal_forward);
        if horizontal_squared > f32::from_bits(0x3727_c5ac) {
            horizontal_forward =
                horizontal_forward.map(|v| v * inverse_length_squared(horizontal_squared, 2));
        }
        let parallel = dot3(up, horizontal_forward);
        let rejected = std::array::from_fn(|i| up[i] - horizontal_forward[i] * parallel);
        let inverse = inverse_length_squared(dot3(rejected, rejected), 2);
        let transverse_up = rejected.map(|v| v * inverse);
        //The bgt takes only ordered positive values; zero and NaN use -1.
        let travel_sign = if speed > 0.0 { 1.0 } else { -1.0 };
        let filtered_normal = if !(dot3(retained_normal, normal) <= f32::from_bits(0x3f6f_5c29)) {
            normal
        } else {
            let blended = std::array::from_fn(|i| {
                retained_normal[i].mul_add(
                    f32::from_bits(0x3f66_6666),
                    normal[i] * f32::from_bits(0x3dcc_cccd),
                )
            });
            let squared = dot3(blended, blended);
            let inverse = inverse_length_squared(squared, 2);
            let length = if squared == 0.0 {
                0.0
            } else {
                squared * inverse
            };
            if length > f32::from_bits(0x3586_37bd) {
                blended.map(|v| v * inverse)
            } else {
                [0.0; 4]
            }
        };
        Self {
            deck,
            effective,
            inverse_effective,
            side,
            up,
            forward,
            horizontal_forward,
            transverse_up,
            forward_velocity: forward.map(|v| v * speed),
            travel_direction: forward.map(|v| v * travel_sign),
            filtered_normal,
            absolute_speed: speed * travel_sign,
            control_sign,
            total_mass: total_body_mass(inverse_masses),
        }
    }
}
