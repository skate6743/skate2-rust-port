//! Inlined physical input calculations from TU3 82DB4048.
//! Xenon estimates use the shared, explicitly approximation-derived model.
use super::requests::{GroundHistoryRequest, GroundHistoryResult, PrepareJumpRequest};
use super::types::RawVector;
use crate::physics::{
    board_motion_output::inverse_length_squared,
    native_arithmetic::{dot3, reciprocal_estimate},
};

///82DB5184..51D4: retain displacement separately from the damped velocity.
pub fn ground_history(request: GroundHistoryRequest) -> GroundHistoryResult {
    let old = request.previous_position.map(f32::from_bits);
    let current = request.current_position.map(f32::from_bits);
    let previous = request.previous_filtered_delta.map(f32::from_bits);
    let delta: [f32; 4] = std::array::from_fn(|i| current[i] - old[i]);
    let timestep = f32::from_bits(request.timestep_bits);
    let mut reciprocal = reciprocal_estimate(timestep);
    for _ in 0..2 {
        let error = (-reciprocal).mul_add(timestep, 1.0);
        reciprocal = reciprocal.mul_add(error, reciprocal);
    }
    //Literal82072818, then8231A844 minus that coefficient.
    let retention = f32::from_bits(0x3f73_3333);
    let change = 1.0 - retention;
    GroundHistoryResult {
        delta: delta.map(f32::to_bits),
        filtered_delta: std::array::from_fn(|i| {
            let scaled = reciprocal * (delta[i] * change);
            previous[i].mul_add(retention, scaled).to_bits()
        }),
    }
}

///82DB53F0..5508 integrates the gravity component along the board velocity.
///The PrepareJump flag's direct velocity copy is handled by the caller.
pub fn prepared_jump_velocity(request: PrepareJumpRequest) -> RawVector {
    let velocity = request.skateboard_vector.map(f32::from_bits);
    let previous = request.previous_velocity.map(f32::from_bits);
    let squared = dot3(velocity, velocity);
    let inverse = inverse_length_squared(squared, 2);
    let length = if squared == 0.0 {
        0.0
    } else {
        squared * inverse
    };
    //Initializer82F826F8 broadcasts82181A88 into830BD350.
    let direction = if length > f32::from_bits(0x3586_37bd) {
        velocity.map(|v| v * inverse)
    } else {
        [0.0; 4]
    };
    let down_component = f32::from_bits(direction[1].to_bits() ^ 0x8000_0000);
    let acceleration = down_component * f32::from_bits(0x411c_cccd); //822F8BD4
    std::array::from_fn(|i| {
        (direction[i] * acceleration)
            .mul_add(f32::from_bits(0x3c88_8889), previous[i]) //820849C8
            .to_bits()
    })
}
