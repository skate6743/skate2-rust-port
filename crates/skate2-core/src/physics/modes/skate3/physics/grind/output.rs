//! Common82D40EA0 and six family FillOut overrides. Post/selector are separate.
use super::{Family, ManagerObservation};
use skate3_core::player::input_phase::GrindOutputFields;
type V = [f32; 4];

/// Retained physical-state fields, not reconstructed from filtered output.
#[derive(Clone, Copy, Debug)]
pub(crate) struct State {
    pub direction: V,
    pub normal: V,
    pub across: V,
    pub crouch: f32,
    pub leaving: bool,
    pub substate: u32,
    pub classification_104: u32,
    pub just_jumped: bool,
    pub jump_velocity: V,
    pub slide_wipeout: bool,
    pub slide_impulse: V,
    pub tipslide_97_98_99: [bool; 3],
}

/// Fields owned by these FillOut methods. The caller retains this between
/// publications: unwritten optional fields must survive exactly as in S3.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PhysicalOutput {
    pub direction: V,
    pub point: V,
    pub normal: V,
    pub across: V,
    pub primitive_endpoints: [V; 2],
    pub high_side: V,
    pub impact_speed: f32,
    pub crouch: f32,
    pub family: Family,
    pub substate: u32,
    pub audio_surface: u32,
    pub spline_guids: [u64; 2],
    pub classification: u32,
    pub leaving: bool,
    pub on_front: bool,
    pub is_ledge: bool,
    pub curb: bool,
    pub tipslide_324_325_326: [bool; 3],
}

/// Optional writes to OTHER physical output blocks. No inactive-event clearing
/// is performed here; the owner of Air/Wipeout output owns those resets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SideEffects {
    pub air_jump_velocity_128: Option<V>,
    pub wipeout_impulse_16: Option<V>,
}

pub(crate) fn fill(
    family: Family,
    state: State,
    manager: &ManagerObservation,
    output: &mut PhysicalOutput,
) -> SideEffects {
    let geometry = manager.geometry;
    output.crouch = state.crouch;
    output.leaving = state.leaving;
    output.direction = state.direction;
    output.normal = state.normal;
    output.across = state.across;
    output.point = geometry.point_1120;
    output.primitive_endpoints = [geometry.primitive_start_1264, geometry.primitive_end_1280];
    output.impact_speed = geometry.impact_speed_1492;
    output.audio_surface = manager.surface.audio_surface_1468;
    output.substate = state.substate;
    output.is_ledge = geometry.kind_1464 == 2;
    output.curb = geometry.flags_1476 & 0x4000_0000 != 0;
    output.classification = state.classification_104;
    if let Some(guids) = geometry.spline_guids_1296 {
        output.spline_guids = guids;
    }
    if output.is_ledge {
        output.high_side = geometry.high_side_1440;
    }
    output.family = family;
    if matches!(family, Family::Tipslide | Family::FiveO | Family::Backslash) {
        output.on_front = manager.control.flags_1516 & 0x2000_0000 != 0;
    }
    if family == Family::Tipslide {
        output.tipslide_324_325_326 = state.tipslide_97_98_99;
    }
    SideEffects {
        air_jump_velocity_128: state.just_jumped.then_some(state.jump_velocity),
        wipeout_impulse_16: (state.slide_wipeout
            && matches!(family, Family::Boardslide | Family::Darkslide))
        .then_some(state.slide_impulse),
    }
}

/// Adapt the retained native publication directly. Unwritten family fields,
/// camera state, graph names and selector flags are not reset by FillOut.
pub(crate) fn fill_fields(
    family: Family,
    state: State,
    manager: &ManagerObservation,
    out: &mut GrindOutputFields,
) -> SideEffects {
    let v = |v: [u32; 4]| v.map(f32::from_bits);
    let mut physical = PhysicalOutput {
        direction: v(out.direction_0),
        point: v(out.point_16),
        normal: v(out.normal_32),
        across: v(out.across_48),
        primitive_endpoints: [v(out.primitive_start_64), v(out.primitive_end_80)],
        high_side: v(out.high_side_112),
        impact_speed: out.impact_speed_128,
        crouch: out.crouch_132,
        family,
        substate: out.words_136_140[1],
        audio_surface: out.audio_surface_216,
        spline_guids: out.spline_guids_224_232,
        classification: out.trick_out_240,
        leaving: out.leaving_317 != 0,
        on_front: out.flag_318 != 0,
        is_ledge: out.is_ledge_320 != 0,
        curb: out.curb_321 != 0,
        tipslide_324_325_326: [
            out.dropping_in_324 != 0,
            out.tipslide_325 != 0,
            out.tipslide_326 != 0,
        ],
    };
    let side_effects = fill(family, state, manager, &mut physical);
    let raw = |v: V| v.map(f32::to_bits);
    out.direction_0 = raw(physical.direction);
    out.point_16 = raw(physical.point);
    out.normal_32 = raw(physical.normal);
    out.across_48 = raw(physical.across);
    out.primitive_start_64 = raw(physical.primitive_endpoints[0]);
    out.primitive_end_80 = raw(physical.primitive_endpoints[1]);
    // The optional stores retain the complete existing bytes, not boolized data.
    if manager.geometry.kind_1464 == 2 {
        out.high_side_112 = raw(physical.high_side);
    }
    if let Some(guids) = manager.geometry.spline_guids_1296 {
        out.spline_guids_224_232 = guids;
    }
    out.impact_speed_128 = physical.impact_speed;
    out.crouch_132 = physical.crouch;
    out.words_136_140 = [family as u32, physical.substate];
    out.audio_surface_216 = physical.audio_surface;
    out.trick_out_240 = physical.classification;
    out.leaving_317 = u8::from(physical.leaving);
    out.is_ledge_320 = u8::from(physical.is_ledge);
    out.curb_321 = u8::from(physical.curb);
    if matches!(family, Family::Tipslide | Family::FiveO | Family::Backslash) {
        out.flag_318 = u8::from(physical.on_front);
    }
    if family == Family::Tipslide {
        [out.dropping_in_324, out.tipslide_325, out.tipslide_326] =
            physical.tipslide_324_325_326.map(u8::from);
    }
    side_effects
}
