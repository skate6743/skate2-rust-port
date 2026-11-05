//! Deck acceleration conditioning82DF0508 and IsBumped82BA7310, original TU3.
//! Matrices are the raw physical deck and Reckoning ground frames; the effective
//! animation deck is a different source. No render transform participates here.
use crate::physics::{native_arithmetic, skeleton_animation_record::AnimationPartTransform};

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// PhysOutDeck0..48, actual Part6 transform (82C02AD8..2B20).
    pub deck: AnimationPartTransform,
    /// PhysOutGround0..48, Reckoning752..800 (82DB70E0..7100).
    pub ground: AnimationPartTransform,
    /// PhysOutMotion112, BoardBody528 from observed deck velocity differences.
    pub world_acceleration: [f32; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Globals344 anim_motion/bumps layout1700, scale_x_acc.
    pub scale_x_acc: f32,
    /// Same collection layout1768, min_bump_mag.
    pub min_bump_mag: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub acceleration: [f32; 4],
    pub bumped: bool,
}

pub fn publish(input: Input, settings: &Settings) -> Output {
    let acceleration = condition(input);
    Output {
        acceleration,
        bumped: is_bumped(acceleration, settings),
    }
}

///82DF0508: remove deck-local Y, rotate back to world, express in the
///ground frame, then remove ground-local Y. Both original stores are required.
pub fn condition(input: Input) -> [f32; 4] {
    let mut deck_local = inverse_rotate(input.deck, input.world_acceleration);
    deck_local[1] = 0.0; //82DF05C4
    let world = rotate(input.deck, deck_local);
    let mut ground_local = inverse_rotate(input.ground, world);
    ground_local[1] = 0.0; //82DF0600
    ground_local
}

///82BA7310 scales X only, then compares the refined length strictly > threshold.
///Independent Rust estimates are used; this is not a bit-exact Xenon claim.
pub fn is_bumped(mut acceleration: [f32; 4], settings: &Settings) -> bool {
    acceleration[0] *= settings.scale_x_acc;
    let squared = native_arithmetic::dot3(acceleration, acceleration);
    let mut reciprocal = native_arithmetic::reciprocal_square_root_estimate(squared);
    for _ in 0..2 {
        let square = reciprocal * reciprocal;
        let half = reciprocal * 0.5;
        let error = (-squared).mul_add(square, 1.0);
        reciprocal = half.mul_add(error, reciprocal);
    }
    let magnitude = if squared == 0.0 {
        0.0
    } else {
        squared * reciprocal
    };
    magnitude > settings.min_bump_mag
}

fn inverse_rotate(frame: AnimationPartTransform, vector: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|lane| {
        frame[lane][2].mul_add(
            vector[2],
            frame[lane][1].mul_add(vector[1], frame[lane][0] * vector[0]),
        )
    })
}
fn rotate(frame: AnimationPartTransform, vector: [f32; 4]) -> [f32; 4] {
    core::array::from_fn(|lane| {
        frame[2][lane].mul_add(
            vector[2],
            frame[1][lane].mul_add(vector[1], frame[0][lane] * vector[0]),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pitched_deck_is_projected_before_the_distinct_ground_frame() {
        // Deck Y points world Z. Ground X points world Z. World Z is removed
        // before changing frame; removing only final Y would incorrectly keep it.
        let deck = [
            [1., 0., 0., 0.],
            [0., 0., 1., 0.],
            [0., -1., 0., 0.],
            [7., 8., 9., 0.],
        ];
        let ground = [
            [0., 0., 1., 0.],
            [0., 1., 0., 0.],
            [-1., 0., 0., 0.],
            [0.; 4],
        ];
        let result = condition(Input {
            deck,
            ground,
            world_acceleration: [2., 3., 4., 0.],
        });
        assert_eq!(&result[..3], &[0., 0., -2.]);
    }
    #[test]
    fn bump_uses_only_scaled_horizontal_magnitude_and_strict_threshold() {
        let settings = Settings {
            scale_x_acc: 2.,
            min_bump_mag: 10.,
        };
        assert!(!is_bumped([0.; 4], &settings));
        assert!(!is_bumped([0., 0., 10., 0.], &settings));
        assert!(is_bumped([6., 0., 0., 0.], &settings));
        assert!(!is_bumped([0., 0., 0., 999.], &settings));
    }
}
