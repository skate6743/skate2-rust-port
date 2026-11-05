//! SetTurning enter 82BB3810 and update 82BB3830 at resolved component boundaries.
//! This is a reusable node operation; the loaded graph owns its activation.
use super::turn_remap::TurnRemap;
use crate::point_graph::PointGraph;

/// One retained native slide record (SpecificMotionGraph3204..3220).
/// Both SetTurning and authored slide handlers read and mutate this owner.
/// Reserved bits stay intact; reset82595450..78 only clears the defined bits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SlideLatch {
    words: [u32; 5],
}
impl SlideLatch {
    pub fn captured_fakie(&self) -> bool {
        self.words[4] & 0x8000_0000 != 0
    }
    pub fn candidate_enabled(&self) -> bool {
        self.words[4] & 0x4000_0000 != 0
    }
    pub fn set_candidate_enabled(&mut self, enabled: bool) {
        self.words[4] = (self.words[4] & !0x4000_0000) | ((enabled as u32) << 30);
    }
    pub fn elapsed(&self, right: bool) -> f32 {
        f32::from_bits(self.words[if right { 2 } else { 0 }])
    }
    pub fn start(&self, right: bool) -> bool {
        self.words[if right { 3 } else { 1 }] & 0x8000_0000 != 0
    }
    pub fn end(&self, right: bool) -> bool {
        self.words[if right { 3 } else { 1 }] & 0x4000_0000 != 0
    }
    pub fn set_start(&mut self, right: bool, value: bool) {
        let word = &mut self.words[if right { 3 } else { 1 }];
        *word = (*word & !0x8000_0000) | ((value as u32) << 31);
    }
    pub fn set_end(&mut self, right: bool, value: bool) {
        let word = &mut self.words[if right { 3 } else { 1 }];
        *word = (*word & !0x4000_0000) | ((value as u32) << 30);
    }
    pub fn advance_elapsed(&mut self, right: bool, dt: f32) {
        let word = &mut self.words[if right { 2 } else { 0 }];
        *word = (dt + f32::from_bits(*word)).to_bits();
    }
    /// CreateSlide Begin82BB2E88; do not clear start/end/grab/candidate flags.
    pub fn begin_slide(&mut self, fakie: bool) {
        self.words[0] = 0;
        self.words[2] = 0;
        self.words[4] = (self.words[4] & !0x8000_0000) | ((fakie as u32) << 31);
    }
    /// GrabSlide82BBB848 maps authored side through the captured stance.
    pub fn grab(&mut self, authored_right: bool) {
        let right = authored_right ^ self.captured_fakie();
        self.words[if right { 3 } else { 1 }] |= 0x2000_0000;
    }
    /// ShouldLeaveSlide82BA7130 uses captured, not current, stance.
    pub fn should_leave(&self, authored_right: bool) -> bool {
        self.end(authored_right ^ self.captured_fakie())
    }
    pub fn reset(&mut self) {
        self.words[0] = 0;
        self.words[2] = 0;
        self.words[1] &= 0x1fff_ffff;
        self.words[3] &= 0x1fff_ffff;
        self.words[4] &= 0x3fff_ffff;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub elapsed: f32,
    pub smoothed: f32,
    pub mode: u32,
}
impl State {
    /// Native enter and allocator 82BB4138 use the same three initial values.
    pub fn enter(&mut self) {
        self.elapsed = 0.0;
        self.smoothed = 0.0;
        self.mode = 2;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// Native helper mode 0 and mode 1, in that order.
    pub remaps: [TurnRemap; 2],
    pub speed_tuck: PointGraph<8>,
    pub blend: PointGraph<8>,
    pub speed_threshold: f32,
    pub maximum_delta: f32,
    /// Global collection slot 332, layout 1680.
    pub override_turn: f32,
}

/// Resolved physical output bundle+56 and bundle+4 inputs. Offset names avoid
/// assigning unverified meanings to upstream physical producer fields.
#[derive(Clone, Copy, Debug)]
pub struct Physical {
    pub field_32: f32,
    pub field_36: f32,
    pub field_52: f32,
    pub field_56: f32,
    pub field_60: f32,
    pub body_168: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Intents {
    pub fakie_turn: Option<f32>,
    /// Native keys 830C067C and 830C05F4, respectively.
    pub mode_0_slide: Option<f32>,
    pub mode_1_slide: Option<f32>,
}

/// Skeleton selectors bind to the decoded node parameter keys. Packet Turn and
/// Slide are native fixed attribute keys. Emissions occur in native call order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attribute {
    Angle,
    Direction,
    Quickness,
    Speed,
    Holding,
    Turn,
    Slide,
}

/// `latch` preserves all five words returned by motion graph virtual+64.
/// `stance` is the component's virtual+12 and virtual+28 result pair.
/// `dt` is the graph context clock virtual+40 result, not a guessed fixed tick.
pub fn update(
    state: &mut State,
    latch: &mut SlideLatch,
    physical: Physical,
    stance: (bool, bool),
    dt: f32,
    settings: &Settings,
    intents: Intents,
    mut emit: impl FnMut(Attribute, f32),
) {
    let latch = &mut latch.words;
    let a = settings.remaps[0].apply([physical.field_32, physical.field_56]);
    let _ = settings.remaps[0].apply([physical.field_36, physical.field_56]);
    let b = settings.remaps[1].apply([physical.field_32, physical.field_56]);
    let _ = settings.remaps[1].apply([physical.field_36, physical.field_56]);
    let complement = 1.0 - physical.field_52;
    let mut target = -a[0].mul_add(physical.field_52, b[0] * complement);
    let direction = -a[1].mul_add(physical.field_52, b[1] * complement);
    let old_0 = latch[1];
    let old_1 = latch[3];
    latch[1] &= !0x2000_0000;
    latch[3] &= !0x2000_0000;
    let prior_0 = old_0 & 0x2000_0000 != 0;
    let prior_1 = old_1 & 0x2000_0000 != 0;
    let enabled = latch[4] & 0x4000_0000 != 0;
    let mut mode = state.mode;
    if mode == 2 {
        let first = enabled && (latch[3] & 0x8000_0000 != 0 || prior_1);
        let second = enabled && (latch[1] & 0x8000_0000 != 0 || prior_0);
        if first || second {
            if !prior_0 && !prior_1 {
                latch[0] = 0;
                latch[2] = 0;
                latch[4] = (latch[4] & 0x7fff_ffff) | ((stance.0 as u32) << 31);
            }
            mode = if first { 1 } else { 0 };
        }
    } else if (mode == 1 && (!enabled || latch[3] & 0x4000_0000 != 0))
        || (mode == 0 && (!enabled || latch[1] & 0x4000_0000 != 0))
    {
        mode = 2;
    }
    if mode <= 1 {
        target = if (mode == 1) ^ stance.1 { 1.0 } else { -1.0 };
        if latch[4] & 0x8000_0000 != 0 {
            target = -target;
        }
    }
    if mode != state.mode {
        state.elapsed = 0.0;
    }
    state.mode = mode;
    let blend = settings.blend.evaluate(state.elapsed);
    let lower = state.smoothed - settings.maximum_delta;
    let upper = state.smoothed + settings.maximum_delta;
    let value = (1.0 - blend).mul_add(state.smoothed, blend * target);
    let value = if lower - value >= 0.0 { lower } else { value };
    state.elapsed += dt;
    state.smoothed = if upper - value >= 0.0 { value } else { upper };
    let speed = physical.body_168.abs();
    let tuck = if speed > settings.speed_threshold {
        settings
            .speed_tuck
            .evaluate(speed - settings.speed_threshold)
    } else {
        0.0
    };
    emit(Attribute::Angle, state.smoothed);
    emit(Attribute::Direction, direction);
    emit(Attribute::Quickness, physical.field_52);
    emit(Attribute::Speed, tuck);
    emit(Attribute::Holding, physical.field_60);
    if mode <= 1 {
        let flip = latch[4] & 0x8000_0000 != 0;
        let turn = if (mode == 1) ^ flip {
            settings.override_turn
        } else {
            -settings.override_turn
        };
        emit(Attribute::Turn, turn);
        let slide = if mode == 0 {
            intents.mode_0_slide
        } else {
            intents.mode_1_slide
        }
        .unwrap_or(0.0);
        let slide = if flip {
            let negative = -slide;
            let absolute = negative.abs();
            let value = if absolute > 0.5 {
                1.0 - absolute
            } else {
                absolute
            };
            if negative >= 0.0 { value } else { -value }
        } else {
            slide
        };
        emit(Attribute::Slide, slide);
    } else if let Some(value) = intents.fakie_turn {
        emit(Attribute::Turn, value);
    }
}
