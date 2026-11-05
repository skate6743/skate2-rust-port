//! Required producer inputs. None implements Default: absence of a producer is
//! not permission to use world-up, metal friction, geometry zero or full energy.
type V = [f32; 4];

/// Snapshot after the manager's geometry, material, assistance, balance,
/// engagement, controls and jumper phases. Production wiring belongs to main.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ManagerObservation {
    pub geometry: GeometryObservation,
    pub surface: SurfaceObservation,
    pub control: ControlObservation,
    pub engagement: EngagementObservation,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GeometryObservation {
    pub point_1120: V,
    pub direction_1136: V,
    pub normal_1152: V,
    pub target_up_1168: V,
    pub primitive_start_1264: V,
    pub primitive_end_1280: V,
    /// Native optional spline header contains TWO independent GUIDs.
    pub spline_guids_1296: Option<[u64; 2]>,
    pub upmost_normal_1408: V,
    pub high_side_1440: V,
    /// Retain every value; force leaves select 0, 1, or all other geometry.
    pub kind_1464: u32,
    pub flags_1476: u32,
    pub impact_speed_1492: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SurfaceObservation {
    /// Output/audio identifier is NOT the friction material identifier.
    pub audio_surface_1468: u32,
    pub material_1472: u32,
    pub friction_vs_time_1496: f32,
    pub reckon_blend_selector_1500: f32,
    /// S3 gravity-relief producer82D8ACF0; NOT UpdateFrictionVsTime.
    pub gravity_relief_1512: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ControlObservation {
    /// Signed bit31 forces exit; bit29 selects the front end; bit30 delivers
    /// engagement velocity. Keep the source word rather than guessing booleans.
    pub flags_1516: u32,
    pub flags_2488: u32,
    pub translation_2796: f32,
    pub balance_2800: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EngagementObservation {
    pub kind_1248: u32,
}
