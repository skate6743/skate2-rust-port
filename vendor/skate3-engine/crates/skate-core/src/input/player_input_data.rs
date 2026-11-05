//! TU3 `Sk8::PlayerInputData::Pack` (0x828E1F98) and `Unpack` (0x828E2260). The local player's
//! cache line reaches gameplay through this byte stream (`PlayerUI::Update` 0x82898920 packs,
//! `GameInputSK8::OnCommand` 0x82695438 unpacks), so every consumer reads the quantized values:
//! sticks in 1/127 steps and triggers in 1/255 steps, both truncated toward zero.

const ONE_127TH: f32 = f32::from_bits(0x3C01_0204);
const ONE_255TH: f32 = f32::from_bits(0x3B80_8081);

/// Pack then unpack one focused cache line (24 values from `xbox::convert`).
pub fn round_trip(buttons: &[f32; 24]) -> [f32; 24] {
    let mut out = [0.0; 24];
    for slot in (0..10).chain(12..16) {
        out[slot] = if buttons[slot] > 0.0 { 1.0 } else { 0.0 };
    }
    for slot in [10, 11] {
        let packed = (buttons[slot] * 255.0) as i64 as u8;
        out[slot] = f32::from(packed) * ONE_255TH;
    }
    for (positive, negative) in [(16, 17), (18, 19), (20, 21), (22, 23)] {
        let packed = ((buttons[positive] - buttons[negative]) * 127.0) as i32 as i8;
        out[positive] = if packed > 0 { f32::from(packed) * ONE_127TH } else { 0.0 };
        out[negative] = if packed < 0 { f32::from(packed) * -ONE_127TH } else { 0.0 };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::round_trip;
    use crate::input::xbox::{XboxState, convert};

    fn left_x(raw: i16) -> f32 {
        let state = XboxState { left: [raw, 0], ..Default::default() };
        let values = round_trip(&convert(&state, 0));
        values[16] - values[17]
    }

    #[test]
    fn sticks_match_the_measured_native_steps() {
        // TU3 recomp, Processed +2712 with sThumbLX injected: k/127 at each deflection.
        for (raw, steps) in [(9830, 9), (13107, 27), (16384, 45), (19661, 63), (22938, 81), (26214, 99), (29491, 117)] {
            assert_eq!(left_x(raw), steps as f32 * f32::from_bits(0x3C01_0204), "raw {raw}");
            assert_eq!(left_x(-raw), -(steps as f32) * f32::from_bits(0x3C01_0204), "raw -{raw}");
        }
        assert_eq!(left_x(i16::MAX), 1.0);
    }

    #[test]
    fn buttons_survive_and_triggers_keep_255_steps() {
        let state = XboxState { buttons: 0xF3FF, triggers: [128, 255], ..Default::default() };
        let converted = convert(&state, 0);
        let values = round_trip(&converted);
        for slot in (0..10).chain(12..16) {
            assert_eq!(values[slot], converted[slot]);
        }
        assert_eq!(values[10], 128.0 * f32::from_bits(0x3B80_8081));
        assert!(values[11] >= 254.0 / 255.0);
    }
}
