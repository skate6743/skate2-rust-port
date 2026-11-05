//! Typed Grinds PhysOut block. Offsets identify native producers, not Rust ABI.
//! Reset82DE3518; common Fill82D40EA0; tipslide Fill82D42788;
//! chromosome publication82DEE508; contact conditioner82DF0640.
use super::types::RawVector;
use crate::animation::output::attributes::AttributeName;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrindOutputFields {
    pub direction_0: RawVector,
    pub point_16: RawVector,
    pub normal_32: RawVector,
    pub across_48: RawVector,
    pub primitive_start_64: RawVector,
    pub primitive_end_80: RawVector,
    pub camera_target_96: RawVector,
    pub high_side_112: RawVector,
    pub impact_speed_128: f32,
    pub crouch_132: f32,
    /// Physical family0..5 (not admission kind), then substate.
    pub words_136_140: [u32; 2],
    pub animation_id_144: u32,
    pub scoring_id_148: u32,
    pub scorable_id_152: u32,
    /// Five-word stock FastString representation, not a hash of the display name.
    /// None means the external reset-name template has not been supplied.
    pub animation_name_156: Option<AttributeName>,
    pub scoring_name_176: Option<AttributeName>,
    pub surface_name_196: Option<AttributeName>,
    pub audio_surface_216: u32,
    pub spline_guids_224_232: [u64; 2],
    /// Common physical classification104, consumed by GrindTrickOutTypeAllowed.
    pub trick_out_240: u32,
    /// Approach, location, twist, tilt, orientation, family. No game enum here.
    pub volatile_chromosome_244: [u32; 6],
    /// First component at268 is the graph's filtered approach observation.
    pub animation_chromosome_268: [u32; 6],
    pub scoring_chromosome_292: [u32; 6],
    /// Selector-owned; the camera conditioner must not infer this flag.
    pub grinding_316: u8,
    pub leaving_317: u8,
    pub flag_318: u8,
    pub flag_319: u8,
    pub is_ledge_320: u8,
    pub curb_321: u8,
    pub flag_322: u8,
    pub flag_323: u8,
    pub dropping_in_324: u8,
    pub tipslide_325: u8,
    pub tipslide_326: u8,
}

impl Default for GrindOutputFields {
    fn default() -> Self {
        const X: RawVector = [0x3f80_0000, 0, 0, 0];
        const Y: RawVector = [0, 0x3f80_0000, 0, 0];
        const EMPTY: [u32; 6] = [0, 0, 0, 0, 0, u32::MAX];
        Self {
            direction_0: X,
            point_16: [0; 4],
            normal_32: Y,
            across_48: Y,
            primitive_start_64: [0; 4],
            primitive_end_80: [0; 4],
            camera_target_96: [0; 4],
            high_side_112: [0; 4],
            impact_speed_128: 0.,
            crouch_132: 0.,
            words_136_140: [u32::MAX, 0],
            animation_id_144: 0,
            scoring_id_148: 0,
            scorable_id_152: u32::MAX,
            //82F8AF08 initializes template830C0D60 from empty string82060799.
            animation_name_156: Some(crate::animation::skeleton_input::name::encode(b"")),
            scoring_name_176: Some(crate::animation::skeleton_input::name::encode(b"")),
            surface_name_196: None,
            audio_surface_216: 0,
            spline_guids_224_232: [u64::MAX; 2],
            trick_out_240: 0,
            volatile_chromosome_244: EMPTY,
            animation_chromosome_268: EMPTY,
            scoring_chromosome_292: EMPTY,
            grinding_316: 0,
            leaving_317: 0,
            flag_318: 0,
            flag_319: 0,
            is_ledge_320: 0,
            curb_321: 0,
            flag_322: 0,
            flag_323: 0,
            dropping_in_324: 0,
            tipslide_325: 0,
            tipslide_326: 0,
        }
    }
}

impl GrindOutputFields {
    /// The native reset copies two external FastString templates; it does not
    /// construct a grind name from family0. Supply decoded original templates.
    /// Keeping these optional distinguishes an unavailable template from zeros.
    pub fn reset_names(&mut self, grind: Option<AttributeName>, surface: Option<AttributeName>) {
        self.animation_name_156 = grind;
        self.scoring_name_176 = grind;
        self.surface_name_196 = surface;
    }
}
