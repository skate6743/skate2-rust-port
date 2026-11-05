//! Small predicates called by TU3 `CalcSuggestedState` (`0x82D8ADE8`).

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardBodyState {
    /// `SkateboardBody + 856`.
    pub field_856: f32,
    /// `SkateboardBody + 7692`.
    pub field_7692: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoStageThresholds {
    pub field_856_primary: f32,
    pub field_856_secondary: f32,
    pub field_7692: f32,
}

/// TU3 `0x82D8BBB8`. All comparisons are strict, matching `fcmpu`/`bgt`.
pub fn condition_is_off_ground_skitching(
    body: BoardBodyState,
    thresholds: TwoStageThresholds,
) -> bool {
    body.field_856 > thresholds.field_856_primary
        || (body.field_856 > thresholds.field_856_secondary
            && body.field_7692 > thresholds.field_7692)
}

/// TU3 `0x82D8BD50`. The separate settings source is intentional: the native
/// function reads these three values from the live physics-mode collection.
pub fn condition_is_off_ground(body: BoardBodyState, thresholds: TwoStageThresholds) -> bool {
    body.field_856 > thresholds.field_856_primary
        || (body.field_856 > thresholds.field_856_secondary
            && body.field_7692 > thresholds.field_7692)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonAnimationState {
    /// `Skeleton + 16420`.
    pub mode_16420: u32,
    /// Lane one of the vector at `Skeleton + 12624 + 32`.
    pub back_chain_lane_12656: f32,
    /// `*(Skeleton + 24) + 800`.
    pub threshold_800: f32,
}

/// TU3 `0x82BDC6C8`. The native absolute value clears the scalar sign bit.
pub fn is_skateboard_animated(input: SkeletonAnimationState) -> bool {
    if input.mode_16420 == 2 {
        return true;
    }
    let magnitude = f32::from_bits(input.back_chain_lane_12656.to_bits() & 0x7fff_ffff);
    if magnitude <= input.threshold_800 {
        false
    } else {
        input.mode_16420 != 1
    }
}
