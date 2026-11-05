//! TU3 Fill825999F0 -> manual producer8259BA28, after Derived input.
use super::{
    controller::{DerivedControllerInput, magnitude},
    riding_intentions::RidingIntent,
};

/// Continuous Manual/ManualBrake belong to the input listener. The authored
/// graphs own engagement time, allowed states, balance and animation selection.
pub fn produce(controller: &DerivedControllerInput, actor_flags: u32) -> Vec<RidingIntent> {
    let mut intents = Vec::with_capacity(2);
    if actor_flags & (1 << 9) != 0 {
        return intents;
    }
    let words = controller.words();
    let [x, y] = [words[9], words[10]].map(f32::from_bits);
    // Fill82599BC8..9C40: x*x + rounded(y*y), then two rsqrt refinements.
    let length = magnitude(x.mul_add(x, y * y));
    if y != 0.0 {
        let value = if y > 0.0 { length } else { -length };
        intents.push(RidingIntent {
            name: "Manual",
            value,
        });
    }
    // Original constants820997B8/822F9274; strictly greater, independent
    // of the Manual presence gate. A horizontal stick can emit ManualBrake.
    let threshold = f32::from_bits(0x3f66_6666);
    if length > threshold {
        let brake = (length - threshold) * f32::from_bits(0x411f_fffe);
        intents.push(RidingIntent {
            name: "ManualBrake",
            value: if y > 0.0 { brake } else { -brake },
        });
    }
    intents
}
