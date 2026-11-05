//! TU3 Fill825999F0 -> object/BodyAdjust producer8259C4B0.
//! OnBoardGeneral also consumes these intents for footplant release steering.
use super::{
    angle::left_stick_angle,
    controller::{DerivedControllerInput, magnitude},
    riding_intentions::RidingIntent,
};
use crate::point_graph::PointGraph;

#[derive(Clone, Debug)]
pub struct Curves {
    pub rotate: PointGraph<16>,
    pub x: PointGraph<8>,
    pub z: PointGraph<8>,
}
impl Curves {
    ///8259C500 gates the curves on either nonzero left axis;8259C768 still
    ///adds right X and publishes all three names when the left stick is centered.
    pub fn produce(&self, controller: &DerivedControllerInput) -> [RidingIntent; 3] {
        let words = controller.words();
        let [x, z, right_x] = [words[7], words[8], words[9]].map(f32::from_bits);
        let (mut rotation, mut move_x, mut move_z) = (0., 0., 0.);
        if x != 0. || z != 0. {
            let angle = left_stick_angle(x, z);
            //8259C694/6F0/750: original reciprocal-pi at822F8610.
            let graph_input = angle.abs() * f32::from_bits(0x3ea2_f983);
            let sign = if angle >= 0. { 1. } else { -1. };
            rotation = (self.rotate.evaluate(graph_input) * sign) * magnitude(z.mul_add(z, x * x));
            move_x = self.x.evaluate(graph_input) * x;
            move_z = self.z.evaluate(graph_input) * z;
        }
        let rotation = rotation + right_x;
        let rotation = if -1. - rotation >= 0. { -1. } else { rotation };
        let rotation = if 1. - rotation >= 0. { rotation } else { 1. };
        [
            RidingIntent {
                name: "OB_ObjectMvZ",
                value: move_z,
            },
            RidingIntent {
                name: "OB_ObjectMvX",
                value: move_x,
            },
            RidingIntent {
                name: "OB_ObjectMvRot",
                value: rotation,
            },
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn curves() -> Curves {
        Curves {
            rotate: PointGraph {
                x: std::array::from_fn(|i| i as f32 / 15.),
                y: [0.; 16],
            },
            x: PointGraph {
                x: std::array::from_fn(|i| i as f32 / 7.),
                y: [1.; 8],
            },
            z: PointGraph {
                x: std::array::from_fn(|i| i as f32 / 7.),
                y: [1.; 8],
            },
        }
    }
    #[test]
    fn object_movement_preserves_cardinal_signs_and_center_publications() {
        for [x, z] in [
            [0., 0.],
            [1., 0.],
            [-1., 0.],
            [0., 1.],
            [0., -1.],
            [0.25, -0.75],
        ] {
            let mut words = [0; 26];
            words[7] = f32::to_bits(x);
            words[8] = f32::to_bits(z);
            words[9] = 0.4f32.to_bits();
            let controller = DerivedControllerInput::from_words(words);
            let out = curves().produce(&controller);
            assert_eq!(
                out.map(|v| v.name),
                ["OB_ObjectMvZ", "OB_ObjectMvX", "OB_ObjectMvRot"]
            );
            assert!((out[0].value - z).abs() < 1e-6 && (out[1].value - x).abs() < 1e-6);
            assert_eq!(out[2].value, 0.4);
            assert_eq!(controller.words(), &words);
        }
    }
}
