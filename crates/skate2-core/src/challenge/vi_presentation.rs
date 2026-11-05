//! `Sk8::VIPresentation` (ctor `0x828044c8`): the ribbon splines of
//! `visual_indicators_sk82.rx2`, the 22 location ribbon and 32 race gate
//! `VisualIndicator::SplineIndicator`s and the follow trail's `DynamicIndicator`.
//! `Update` (`0x828055e0`) runs each presentation frame from the restored
//! `VDReaderData` (`visual_director`); `Render` (`0x82803a18`) draws through the
//! `spline` effect after the particles, before PostFX: every indicator with its
//! back texture in the `darken` technique, then again with its front texture in
//! `default`, each control-point layer one triangle strip.
//!
//! The registered arenas' visual indicator data become streamed spline instances
//! (`AddStreamedSplineInstance` 0x82805a08), each drawn while the frame's spline
//! indicators name it (`UpdateCustomizedSplines` 0x828033a0).
//!
//! Not ported: the Create-a-Spot mesh indicator (`UpdateCreateASpotIndicator` 0x82805328).

use super::visual_director::{RibbonStyle, VisualIndicatorFrame, VisualIndicatorSettings};
use crate::ai::vmx::{self, V4};
use skate2_formats::pegasus::{VisualIndicator, VisualIndicatorData};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

/// `gVisualIndicatorAssetHelper`'s one asset (`GameModule::OneOffLoadTexture` 0x82735730).
pub const ASSET_PATH: &str = "data\\content\\world\\models\\visual_indicators_sk82.rx2";
/// `cTexturePackManager::LoadTextures` 0x8282ad58: the ribbon textures.
pub const TEXTURE_PATH: &str = "data\\content\\textures\\vitextures.rx2";
/// `szFrontTextureAssetName`.
pub const FRONT_TEXTURE: &str = "visual_indicator_orange.Texture";
/// `szBackTextureAssetName`.
pub const BACK_TEXTURE: &str = "visual_indicator_background.Texture";

/// `Spline::mControlPoints` (142): one edge, then the other from 71.
pub const CONTROL_POINTS: usize = 142;
const EDGE: usize = 71;
/// `SplineIndicator::mControlPoints[3]`.
const LAYERS: usize = 3;
/// `mLocationRibbonInstances` and `mRaceGateRibbonInstances`.
const LOCATION_RIBBONS: usize = 22;
const RACE_GATES: usize = 32;
/// `kbEnableVIs` (cvar `0x82f6e048`, default true).
const ENABLE_VIS: bool = true;
/// `kDynamicIndicatorPointTime` (cvar `0x82f6e208`, default 0.05).
const DYNAMIC_INDICATOR_POINT_TIME: f32 = f32::from_bits(0x3d4c_cccd);
/// `VisualIndicator::kUpdateDistance` (cvar `0x82f6e428`, default 0.25).
const UPDATE_DISTANCE: f32 = 0.25;
/// `cParticleRandomManager::mBaseSeed` (0x12345678) plus the 42 `cParticleManager`'s
/// constructor adds before seeding `mRandomManager` (`0x82de6570`).
const RANDOM_SEED: u32 = 0x1234_5678 + 42;

pub type ControlPoints = [V4; CONTROL_POINTS];

/// `VisualIndicator::Spline` (0x970 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Spline {
    /// `mAssetID`: the indicator's `m_uiGuidLocal`.
    pub asset: u64,
    pub control_points: ControlPoints,
    /// `mBB`, min then max.
    bounds: [V4; 2],
    /// `mBC`.
    centre: V4,
    cp_count: u16,
    segment_count: u16,
    /// `mVtxCount` and the parameter step its vertices were written with.
    vertex_count: u16,
    step: f32,
    billboard: bool,
}

impl Spline {
    /// `Spline::InitFromSplineHeader` (`0x82801d00`) of an indicator: the B-spline control
    /// points (each segment matrix's rows summed, less `mBC`), the first repeated four times
    /// and the last three more, on both edges; `20 (n + 4)` vertices at steps of 0.1.
    #[must_use]
    pub fn new(indicator: &VisualIndicator) -> Self {
        let segments = &indicator.segments[..indicator.segments.len().min(64)];
        let bounds = [indicator.bounds.minimum, indicator.bounds.maximum];
        let centre = vmx::scale(vmx::add(bounds[1], bounds[0]), 0.5);
        let n = u16::try_from(segments.len()).unwrap_or(64);
        let mut control_points = [[0.0; 4]; CONTROL_POINTS];
        if let Some(first) = segments.first() {
            let start = vmx::sub(first[3], centre);
            control_points[..4].fill(start);
            control_points[EDGE..EDGE + 4].fill(start);
        }
        let mut last = control_points[0];
        for (index, segment) in segments.iter().enumerate() {
            let point = vmx::sub(
                vmx::add(
                    vmx::add(segment[0], segment[1]),
                    vmx::add(segment[2], segment[3]),
                ),
                centre,
            );
            control_points[4 + index] = point;
            control_points[EDGE + 4 + index] = point;
            last = point;
        }
        let tail = 4 + usize::from(n);
        control_points[tail..tail + 3].fill(last);
        control_points[EDGE + tail..EDGE + tail + 3].fill(last);
        Self {
            asset: indicator.guid_local,
            control_points,
            bounds,
            centre,
            cp_count: n + 7,
            segment_count: n + 4,
            vertex_count: 20 * (n + 4),
            step: f32::from_bits(0x3dcc_cccd),
            billboard: indicator.flags & 1 != 0,
        }
    }

    /// `InitFromSplineHeader(nullptr)`: 71 zero control points on the first edge, 68
    /// segments, 1360 vertices and the box (-1, -1, 0)..(1, 1, 0).
    fn unbound() -> Self {
        let bounds = [[-1.0, -1.0, 0.0, 0.0], [1.0, 1.0, 0.0, 0.0]];
        Self {
            asset: u64::MAX,
            control_points: [[0.0; 4]; CONTROL_POINTS],
            centre: vmx::scale(vmx::add(bounds[1], bounds[0]), 0.5),
            bounds,
            cp_count: 71,
            segment_count: 68,
            vertex_count: 1360,
            step: f32::from_bits(0x3dcc_cccd),
            billboard: false,
        }
    }

    /// `Spline::Update` (`0x82800f60`): `2 * mSegmentCount * min(samples, 10)` vertices at
    /// steps of `1 / samples` when the count changes.
    fn update(&mut self, settings: &VisualIndicatorSettings) {
        let samples = settings.samples.min(10);
        let count = 2 * i32::from(self.segment_count) * i32::from(samples);
        if i32::from(self.vertex_count) != count {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "mVtxCount is a u16"
            )]
            let stored = count as u16;
            self.vertex_count = stored;
            self.step = 1.0 / f32::from(samples);
        }
    }

    #[must_use]
    pub const fn vertex_count(&self) -> u16 {
        self.vertex_count
    }

    #[must_use]
    pub const fn step(&self) -> f32 {
        self.step
    }
}

/// The spline vertex buffer `InitFromSplineHeader` and `Spline::Update` write: pairs
/// `(t, t, 0)` and `(t + 71, t, 1)`, `t` advancing by `step` in single precision.
#[must_use]
pub fn spline_vertices(count: u16, step: f32) -> Vec<[f32; 3]> {
    let mut vertices = Vec::with_capacity(usize::from(count));
    let mut t = 0.0_f32;
    for _ in 0..usize::from(count).div_ceil(2) {
        vertices.push([t, t, 0.0]);
        vertices.push([t + 71.0, t, 1.0]);
        t += step;
    }
    vertices.truncate(usize::from(count));
    vertices
}

/// `VisualIndicator::SplineContainer` (ctor `0x82804348`): one spline per indicator with
/// segments, in data order. `InsertSpline` (`0x82804200`) points every `ribbon_spline`
/// collection whose `SplineAssetID` matches at the spline, so a later match wins.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RibbonResource {
    pub splines: Vec<Spline>,
}

impl RibbonResource {
    #[must_use]
    pub fn new(data: &VisualIndicatorData) -> Self {
        Self {
            splines: data
                .indicators
                .iter()
                .filter(|indicator| !indicator.segments.is_empty())
                .map(Spline::new)
                .collect(),
        }
    }

    fn find(&self, asset: Option<u64>) -> Option<usize> {
        let asset = asset?;
        self.splines
            .iter()
            .rposition(|spline| spline.asset == asset)
    }
}

/// The camera `CameraPresentation::GetMatrix(true)` gives (rows right, up, at, position)
/// and the six planes of `GfxMan->mCameras[5].mFrustum` (`ComputeFrustum` 0x8282bd28).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewInputs {
    pub camera: [V4; 4],
    pub planes: [V4; 6],
}

impl ViewInputs {
    /// `ComputeFrustum`'s normalized w+x, w-x, w-y, w+y, w+z, w-z of the clip rows.
    #[must_use]
    pub fn new(camera: [V4; 4], clip_rows: [V4; 4]) -> Self {
        let [x, y, z, w] = clip_rows;
        let mut planes = [
            vmx::add(x, w),
            vmx::sub(w, x),
            vmx::sub(w, y),
            vmx::add(w, y),
            vmx::add(w, z),
            vmx::sub(w, z),
        ];
        for plane in &mut planes {
            *plane = crate::presentation_shadow_culling::normalize_plane(
                *plane,
                &mut crate::reckoning_numeric::HostReckoningNumeric,
            )
            .unwrap_or(*plane);
        }
        Self { camera, planes }
    }

    /// `Frustum::TestAABB_VMX` (`0x82825830`): outside when the box's corner furthest along
    /// some plane's normal is strictly behind it.
    fn intersects(&self, bounds: &[V4; 2]) -> bool {
        self.planes.iter().all(|plane| {
            let corner: V4 = std::array::from_fn(|lane| {
                if plane[lane] >= 0.0 {
                    bounds[1][lane]
                } else {
                    bounds[0][lane]
                }
            });
            !(0.0 > vmx::dot3(corner, *plane) + plane[3])
        })
    }
}

/// `cParticleManager::mRandomManager` (`cParticleRandomManager`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParticleRandom(u32);

impl ParticleRandom {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x10dcd).wrapping_add(0x29);
        self.0
    }

    /// `cParticleRandomManager::Build` (`0x828009a8`): four draws, each a mantissa in
    /// [1, 2) less one, applied to W, Z, Y, X in that order.
    fn build(&mut self, base: V4, variance: V4) -> V4 {
        let unit = |seed: u32| f32::from_bits((seed & 0x007f_ffff) | 0x3f80_0000) - 1.0;
        let [w, z, y, x] = [self.next(), self.next(), self.next(), self.next()].map(unit);
        [
            x.mul_add(variance[0], base[0]),
            y.mul_add(variance[1], base[1]),
            z.mul_add(variance[2], base[2]),
            w.mul_add(variance[3], base[3]),
        ]
    }
}

/// Which spline a `SplineIndicator` points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SplineRef {
    /// The spline its own constructor made.
    Own,
    Resource(usize),
    /// `mpSpline` null.
    None,
}

/// `VisualIndicator::SplineIndicator` (0x1bf0 bytes).
#[derive(Clone, Debug)]
struct SplineIndicator {
    own: Spline,
    spline: SplineRef,
    front_texture: Option<u64>,
    back_texture: Option<u64>,
    intensity: V4,
    clip_values: V4,
    control_points: Box<[ControlPoints; LAYERS]>,
    anim_time: f32,
    global_bounds: [V4; 2],
    local_bounds: [V4; 2],
    global_centre: V4,
    local_centre: V4,
    /// `mMatLtoW`.
    matrix: [V4; 4],
    visible: bool,
    enable: bool,
}

impl SplineIndicator {
    /// The default constructor (`0x82804cf0`) and `Init` (`0x828023c0`).
    fn new() -> Self {
        let mut indicator = Self {
            own: Spline::unbound(),
            spline: SplineRef::Own,
            front_texture: None,
            back_texture: None,
            intensity: [1.0, 1.0, 0.0, 0.0],
            clip_values: [5.0, 10.0, 70.0, 10.0],
            control_points: Box::new([[[0.0; 4]; CONTROL_POINTS]; LAYERS]),
            anim_time: 0.0,
            global_bounds: [[0.0; 4]; 2],
            local_bounds: [[0.0; 4]; 2],
            global_centre: [0.0; 4],
            local_centre: [0.0; 4],
            matrix: [[0.0; 4]; 4],
            visible: false,
            enable: false,
        };
        indicator.calculate_bounds(None);
        indicator
    }

    /// `SplineIndicator(const tVisualIndicator*)` (`0x82804e08`): its own spline of the
    /// indicator, then `Init`.
    fn with_spline(indicator: &VisualIndicator) -> Self {
        let mut this = Self::new();
        this.own = Spline::new(indicator);
        this.calculate_bounds(None);
        this
    }

    fn spline<'a>(&'a self, resource: Option<&'a RibbonResource>) -> Option<&'a Spline> {
        match self.spline {
            SplineRef::Own => Some(&self.own),
            SplineRef::Resource(index) => resource?.splines.get(index),
            SplineRef::None => None,
        }
    }

    /// `SplineIndicator::CalculateBounds` (`0x828010c8`): the spline's box as the world box,
    /// recentred as the local box, and the identity placed at the box centre.
    fn calculate_bounds(&mut self, resource: Option<&RibbonResource>) {
        let Some(spline) = self.spline(resource) else {
            return;
        };
        let bounds = spline.bounds;
        self.global_bounds = bounds;
        self.global_centre = vmx::scale(vmx::add(bounds[1], bounds[0]), 0.5);
        self.local_bounds = [
            vmx::sub(bounds[0], self.global_centre),
            vmx::sub(bounds[1], self.global_centre),
        ];
        let centre = vmx::scale(vmx::add(self.local_bounds[1], self.local_bounds[0]), 0.5);
        self.local_centre = [centre[0], centre[1], centre[2], 1.0];
        self.matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            self.global_centre,
        ];
    }

    /// `SplineIndicator::SetLocalToWorldMatrix` (`0x82800548`): the world centre through the
    /// matrix, and the world box from the local box's two corners (W one) unless that centre
    /// equals the local one.
    fn set_local_to_world(&mut self, matrix: [V4; 4]) {
        self.matrix = matrix;
        let transform = |point: V4| {
            let x = vmx::scale(matrix[0], point[0]);
            let y = vmx::madd(matrix[1], [point[1]; 4], x);
            let z = vmx::madd(matrix[2], [point[2]; 4], y);
            vmx::madd(matrix[3], [point[3]; 4], z)
        };
        self.global_centre = transform(self.local_centre);
        if self.global_centre != self.local_centre {
            let corner = |point: V4| {
                let world = transform([point[0], point[1], point[2], 1.0]);
                [world[0], world[1], world[2], 0.0]
            };
            self.global_bounds = [corner(self.local_bounds[0]), corner(self.local_bounds[1])];
        }
    }

    /// `SplineIndicator::Cull` (`0x82801278`): a billboard spline turns its X and Z axes to
    /// face the camera about its own Y axis first.
    fn cull(&mut self, resource: Option<&RibbonResource>, view: &ViewInputs) -> bool {
        self.face_camera(resource, view);
        self.visible = view.intersects(&self.global_bounds);
        self.visible
    }

    /// `Cull`'s billboarding (0x828012c4..0x828013b8): the negated camera at axis, Y cleared
    /// and normalized, becomes the Z axis and its quarter turn about Y the X axis; Y and the
    /// translation stay.
    fn face_camera(&mut self, resource: Option<&RibbonResource>, view: &ViewInputs) {
        if self.spline(resource).is_some_and(|spline| spline.billboard) {
            let at = view.camera[2];
            let facing = [-at[0], 0.0, -at[2], -at[3]];
            let normal = vmx::scale(facing, vmx::rsqrt(vmx::dot3(facing, facing)));
            let y = self.matrix[1];
            let w = self.matrix[3];
            self.set_local_to_world([
                [normal[2], normal[1], -normal[0], 0.0],
                [y[0], y[1], y[2], 0.0],
                [normal[0], normal[1], normal[2], 0.0],
                [w[0], w[1], w[2], 1.0],
            ]);
        }
    }

    /// The jittered layers `SplineIndicator::Update` and `DynamicIndicator::Update` build:
    /// each control point plus a centred random offset of up to `amplitude`, and its partner
    /// on the second edge `width` back along the camera's right plus up.
    fn rebuild_layers(
        &mut self,
        layers: i16,
        amplitude: f32,
        width: f32,
        view: &ViewInputs,
        random: &mut ParticleRandom,
        resource: Option<&RibbonResource>,
    ) {
        let Some(spline) = self.spline(resource) else {
            return;
        };
        let points = spline.control_points;
        let count = usize::from(spline.cp_count);
        let right = [view.camera[0][0], view.camera[0][1], view.camera[0][2], 0.0];
        let up = [view.camera[1][0], view.camera[1][1], view.camera[1][2], 0.0];
        let across = vmx::scale(vmx::add(right, up), width);
        let variance = [amplitude, amplitude, amplitude, 0.0];
        let half = vmx::scale(variance, 0.5);
        for layer in self
            .control_points
            .iter_mut()
            .take(usize::try_from(layers).unwrap_or(0))
        {
            for (index, point) in points.iter().enumerate().take(count) {
                let jitter = vmx::sub(random.build([0.0; 4], variance), half);
                let near = vmx::add(*point, jitter);
                layer[index] = near;
                layer[EDGE + index] = vmx::sub(near, across);
            }
        }
    }

    /// `SplineIndicator::Update` (`0x828013f8`).
    fn update(
        &mut self,
        dt: f32,
        settings: &VisualIndicatorSettings,
        view: &ViewInputs,
        random: &mut ParticleRandom,
        resource: &mut Option<RibbonResource>,
    ) {
        if self.spline(resource.as_ref()).is_none() {
            return;
        }
        self.anim_time = dt + self.anim_time;
        let offset = vmx::sub(view.camera[3], self.global_centre);
        let distance = vmx::sqrt_guarded(vmx::dot3(offset, offset));
        let p = settings.params;
        if !(distance <= p[7]) {
            self.visible = false;
            return;
        }
        self.clip_values = [p[4], p[5] - p[4], p[6], p[7] - p[6]];
        self.intensity = [settings.intensity, settings.darken_intensity, 0.0, 0.0];
        match (self.spline, resource.as_mut()) {
            (SplineRef::Own, _) => self.own.update(settings),
            (SplineRef::Resource(index), Some(resource)) => {
                resource.splines[index].update(settings)
            }
            _ => {}
        }
        // fcmpu/blt (0x8280157c): only a time below the period skips the rebuild.
        if !(self.anim_time < settings.jitter_period) {
            let blend = (distance - settings.blend_distance[0])
                / (settings.blend_distance[1] - settings.blend_distance[0]);
            let blend = if blend >= 0.0 { blend.min(1.0) } else { 0.0 };
            let rest = 1.0 - blend;
            let amplitude = settings.jitter[0].mul_add(rest, settings.jitter[1] * blend);
            let width = settings.width[0].mul_add(rest, settings.width[1] * blend);
            self.rebuild_layers(
                settings.layers,
                amplitude,
                width,
                view,
                random,
                resource.as_ref(),
            );
            self.anim_time = 0.0;
        }
    }

    /// `SplineIndicator::Render` (`0x82802940`): one strip of the spline's vertices per layer
    /// of the default settings.
    fn draw(&self, resource: Option<&RibbonResource>, default_layers: i16) -> Option<SplineDraw> {
        let spline = self.spline(resource)?;
        Some(SplineDraw {
            vertex_count: u32::from(spline.vertex_count),
            vertices: (spline.vertex_count, spline.step),
            world: world_columns(&self.matrix),
            intensity: self.intensity,
            clip_values: self.clip_values,
            textures: Some([self.back_texture, self.front_texture]),
            layers: self
                .control_points
                .iter()
                .take(usize::try_from(default_layers).unwrap_or(0).min(LAYERS))
                .copied()
                .collect(),
        })
    }
}

/// `mMatLtoWTrans`'s first three rows, the `i_matWorld` registers c4..c6.
fn world_columns(matrix: &[V4; 4]) -> [V4; 3] {
    std::array::from_fn(|column| {
        [
            matrix[0][column],
            matrix[1][column],
            matrix[2][column],
            matrix[3][column],
        ]
    })
}

/// `VisualIndicator::DynamicIndicator` (0x1c10 bytes): the follow trail.
#[derive(Clone, Debug)]
struct DynamicIndicator {
    base: SplineIndicator,
    /// `mPoints`: at most 71, oldest first.
    points: VecDeque<V4>,
    enable: bool,
}

impl DynamicIndicator {
    fn new() -> Self {
        Self {
            base: SplineIndicator::new(),
            points: VecDeque::new(),
            enable: false,
        }
    }

    /// `DynamicIndicator::AddPoint` (`0x82803df0`): a point at least `kUpdateDistance` from
    /// the last; past 71 the oldest goes. Both edges of the spline then hold the points.
    fn add_point(&mut self, point: [f32; 3]) {
        if let Some(last) = self.points.back() {
            let offset = vmx::sub(*last, [point[0], point[1], point[2], 0.0]);
            if !(vmx::dot3(offset, offset) - UPDATE_DISTANCE * UPDATE_DISTANCE > 0.0) {
                return;
            }
        }
        self.points.push_back([point[0], point[1], point[2], 0.0]);
        if self.points.len() > EDGE {
            self.points.pop_front();
        }
        for (index, point) in self.points.iter().enumerate() {
            self.base.own.control_points[index] = *point;
            self.base.own.control_points[EDGE + index] = *point;
        }
    }

    fn clear(&mut self) {
        self.points.clear();
        self.enable = false;
    }

    /// `DynamicIndicator::Update` (`0x82801770`): no distance test; the jitter and width take
    /// the middle of their ranges.
    fn update(
        &mut self,
        dt: f32,
        settings: &VisualIndicatorSettings,
        view: &ViewInputs,
        random: &mut ParticleRandom,
    ) {
        let base = &mut self.base;
        base.anim_time = dt + base.anim_time;
        let p = settings.params;
        base.clip_values = [p[4], p[5] - p[4], p[6], p[7] - p[6]];
        base.intensity = [settings.intensity, settings.darken_intensity, 0.0, 0.0];
        base.own.update(settings);
        if !(base.anim_time < settings.jitter_period) {
            let amplitude = settings.jitter[1].mul_add(0.5, settings.jitter[0] * 0.5);
            let width = settings.width[1].mul_add(0.5, settings.width[0] * 0.5);
            base.rebuild_layers(settings.layers, amplitude, width, view, random, None);
            base.anim_time = 0.0;
        }
    }

    /// `DynamicIndicator::Render` (`0x82802c00`): `2 (points - 3) min(samples, 10)` vertices
    /// of the default settings, and no texture of its own.
    fn draw(&self, defaults: &VisualIndicatorSettings) -> Option<SplineDraw> {
        let count = 2
            * (i32::try_from(self.points.len()).unwrap_or(0) - 3)
            * i32::from(defaults.samples.min(10));
        let count = u32::try_from(count).ok().filter(|&count| count > 0)?;
        Some(SplineDraw {
            vertex_count: count,
            vertices: (self.base.own.vertex_count, self.base.own.step),
            world: world_columns(&self.base.matrix),
            intensity: self.base.intensity,
            clip_values: self.base.clip_values,
            textures: None,
            layers: self
                .base
                .control_points
                .iter()
                .take(usize::try_from(defaults.layers).unwrap_or(0).min(LAYERS))
                .copied()
                .collect(),
        })
    }
}

/// One `SplineIndicator::Render`: a triangle strip of `vertex_count` vertices of the spline's
/// vertex buffer per control-point layer.
#[derive(Clone, Debug, PartialEq)]
pub struct SplineDraw {
    pub vertex_count: u32,
    /// The vertex buffer: its `mVtxCount` and parameter step (`spline_vertices`).
    pub vertices: (u16, f32),
    /// `i_matWorld` (c4..c6).
    pub world: [V4; 3],
    /// `i_intensity` (c199): x for `default`, y for `darken`.
    pub intensity: V4,
    /// `i_clipvalues` (c200).
    pub clip_values: V4,
    /// Back (`darken`) and front (`default`) texture assets; `None` binds nothing, so the
    /// technique keeps its last texture.
    pub textures: Option<[Option<u64>; 2]>,
    /// `i_cp` (c7..c148) of each layer drawn.
    pub layers: Vec<ControlPoints>,
}

/// A `SplineIndicatorContainer` (ctor `0x82805840`) of registered indicator data: the asset
/// IDs of its splines, one per indicator with segments.
#[derive(Debug)]
struct StreamedContainer {
    data: Arc<VisualIndicatorData>,
    splines: Vec<u64>,
}

/// `VIPresentation`.
#[derive(Debug)]
pub struct ViPresentation {
    resource: Option<RibbonResource>,
    location_ribbons: Vec<SplineIndicator>,
    race_gates: Vec<SplineIndicator>,
    /// The registered indicator data the streamed splines were last synced to.
    streamed_registered: Vec<Arc<VisualIndicatorData>>,
    /// The containers `AddStreamedSplineInstance` made, in registration order.
    streamed: Vec<StreamedContainer>,
    /// `mStreamedSplineInstances`: a streamed spline's indicator under its asset ID, the first
    /// container's of an ID.
    streamed_splines: BTreeMap<u64, SplineIndicator>,
    dynamic: DynamicIndicator,
    /// `mDynamicIndicatorSampleTimer`.
    sample_timer: f32,
    previous_frame: f32,
    previous_delta: f32,
    /// The registered texture assets `GetRegAsset` finds.
    textures: BTreeSet<u64>,
    front_texture: Option<u64>,
    back_texture: Option<u64>,
    random: ParticleRandom,
}

impl Default for ViPresentation {
    fn default() -> Self {
        Self::new()
    }
}

impl ViPresentation {
    /// The constructor and `Init` (`0x82805030`): no ribbon resource yet, the trail.
    #[must_use]
    pub fn new() -> Self {
        Self {
            resource: None,
            location_ribbons: Vec::new(),
            race_gates: Vec::new(),
            streamed_registered: Vec::new(),
            streamed: Vec::new(),
            streamed_splines: BTreeMap::new(),
            dynamic: DynamicIndicator::new(),
            sample_timer: 0.0,
            previous_frame: -15.0,
            previous_delta: 0.0,
            textures: BTreeSet::new(),
            front_texture: None,
            back_texture: None,
            random: ParticleRandom(RANDOM_SEED),
        }
    }

    /// `AddInstancedSplineInstance` (`0x828050a8`): the ribbon resource of
    /// `visual_indicators_sk82.rx2`'s visual indicator data, and new instance pools.
    pub fn add_instanced_splines(&mut self, data: &VisualIndicatorData) {
        if data.indicators.is_empty() || data.segment_count == 0 {
            return;
        }
        self.resource = Some(RibbonResource::new(data));
        self.location_ribbons = (0..LOCATION_RIBBONS)
            .map(|_| SplineIndicator::new())
            .collect();
        self.race_gates = (0..RACE_GATES).map(|_| SplineIndicator::new()).collect();
    }

    /// The registered arenas' indicator data: `RemStreamedSplineInstance` (`0x82803208`) for
    /// each departed one (every spline ID of its container leaves `mStreamedSplineInstances`),
    /// then `AddStreamedSplineInstance` (`0x82805a08`) for each new one (data without
    /// indicators or segments adds nothing; an ID already held keeps its indicator).
    pub fn sync_streamed_splines(&mut self, registered: &[Arc<VisualIndicatorData>]) {
        if self.streamed_registered.len() == registered.len()
            && self.streamed_registered.iter().zip(registered).all(|(held, data)| Arc::ptr_eq(held, data))
        {
            return;
        }
        self.streamed_registered = registered.to_vec();
        let (kept, departed): (Vec<_>, Vec<_>) = std::mem::take(&mut self.streamed)
            .into_iter()
            .partition(|container| registered.iter().any(|data| Arc::ptr_eq(data, &container.data)));
        for container in departed {
            for asset in container.splines {
                self.streamed_splines.remove(&asset);
            }
        }
        self.streamed = kept;
        for data in registered {
            if self.streamed.iter().any(|container| Arc::ptr_eq(&container.data, data))
                || data.indicators.is_empty()
                || data.segment_count == 0
            {
                continue;
            }
            let mut splines = Vec::new();
            for indicator in data.indicators.iter().filter(|indicator| !indicator.segments.is_empty()) {
                let asset = indicator.guid_local;
                splines.push(asset);
                self.streamed_splines
                    .entry(asset)
                    .or_insert_with(|| SplineIndicator::with_spline(indicator));
            }
            self.streamed.push(StreamedContainer { data: Arc::clone(data), splines });
        }
    }

    /// The texture assets the presentation registry holds (`vitextures.rx2`).
    pub fn register_textures(&mut self, assets: impl IntoIterator<Item = u64>) {
        self.textures.extend(assets);
    }

    fn registered(&self, asset: Option<u64>) -> Option<u64> {
        asset.filter(|asset| self.textures.contains(asset))
    }

    /// `VIPresentation::Update` (`0x828055e0`) at the shown simulation tick and fraction:
    /// `dT` is the ticks and fraction since the last update over 60, none from 15 ticks away.
    /// The native updates once per presented frame, at 60 Hz one tick apart, so its timers
    /// step by exactly 1/60: the trail's point timer (`mDynamicIndicatorSampleTimer` against
    /// `kDynamicIndicatorPointTime`) and its 0.05 s jitter period (`mAnimTime`) then reach
    /// their ends on the same frame, `AddPoint` before the rebuild.
    /// `in_nis` is `SceneManager::IsInNIS`; `hidden` clears the render types a scene set.
    pub fn update(
        &mut self,
        frame: &VisualIndicatorFrame,
        view: &ViewInputs,
        (tick, fraction): (u32, f32),
        in_nis: bool,
        hidden: bool,
    ) {
        #[expect(clippy::cast_precision_loss, reason = "fcfid of the tick count")]
        let now = tick as f32;
        let frames = now - self.previous_frame;
        let dt = if frames >= 15.0 || frames <= -15.0 {
            0.0
        } else {
            (((fraction - self.previous_delta) + frames) * f32::from_bits(0x3c88_8889)).abs()
        };
        self.previous_frame = now;
        self.previous_delta = fraction;
        if !ENABLE_VIS || in_nis {
            return;
        }
        if self.front_texture.is_none() {
            self.front_texture =
                self.registered(Some(skate2_formats::hash::rw_hash64_string(FRONT_TEXTURE)));
        }
        if self.back_texture.is_none() {
            self.back_texture =
                self.registered(Some(skate2_formats::hash::rw_hash64_string(BACK_TEXTURE)));
        }
        self.update_location_splines(dt, frame, view, hidden);
        self.update_gate_splines(dt, frame, view, hidden);
        self.update_customized_splines(dt, frame, view);
        self.update_dynamic_indicator(dt, frame, view);
    }

    /// The frames drawn between two updates: each shown ribbon faces their camera as `Cull`
    /// turns it; time, jitter, trail and visibility stay the update's.
    pub fn face_camera(&mut self, view: &ViewInputs) {
        let resource = self.resource.as_ref();
        for indicator in self
            .location_ribbons
            .iter_mut()
            .chain(&mut self.race_gates)
            .filter(|indicator| indicator.enable && indicator.visible)
        {
            indicator.face_camera(resource, view);
        }
        for indicator in self
            .streamed_splines
            .values_mut()
            .filter(|indicator| indicator.enable && indicator.visible)
        {
            indicator.face_camera(None, view);
        }
    }

    /// `VIPresentation::UpdateLocationSplines` (`0x82803528`).
    fn update_location_splines(
        &mut self,
        dt: f32,
        frame: &VisualIndicatorFrame,
        view: &ViewInputs,
        hidden: bool,
    ) {
        let count = if hidden {
            0
        } else {
            frame.location_ribbons.len()
        };
        let mut indicators = std::mem::take(&mut self.location_ribbons);
        for (indicator, ribbon) in indicators
            .iter_mut()
            .zip(&frame.location_ribbons)
            .take(count)
        {
            self.place(indicator, &ribbon.style, ribbon.transform, dt, view);
        }
        for indicator in indicators.iter_mut().skip(count) {
            indicator.enable = false;
        }
        self.location_ribbons = indicators;
    }

    fn place(
        &mut self,
        indicator: &mut SplineIndicator,
        style: &RibbonStyle,
        transform: [f32; 16],
        dt: f32,
        view: &ViewInputs,
    ) {
        let spline = match self
            .resource
            .as_ref()
            .and_then(|resource| resource.find(style.spline_asset))
        {
            Some(index) => SplineRef::Resource(index),
            None => SplineRef::None,
        };
        if !indicator.enable || indicator.spline != spline {
            indicator.spline = spline;
            indicator.enable = true;
            indicator.calculate_bounds(self.resource.as_ref());
        }
        indicator.set_local_to_world(std::array::from_fn(|row| {
            std::array::from_fn(|lane| transform[row * 4 + lane])
        }));
        indicator.front_texture = self.registered(style.front_texture).or(self.front_texture);
        indicator.back_texture = self.registered(style.back_texture).or(self.back_texture);
        if indicator.cull(self.resource.as_ref(), view) {
            indicator.update(
                dt,
                &style.settings,
                view,
                &mut self.random,
                &mut self.resource,
            );
        }
    }

    /// VIPresentation::UpdateGateSplines828037e0. Instance clocks survive gate/shape changes.
    fn update_gate_splines(&mut self, dt: f32, frame: &VisualIndicatorFrame,
        view: &ViewInputs, hidden: bool) {
        let count = if hidden { 0 } else { frame.race_gates.len() };
        let mut indicators = std::mem::take(&mut self.race_gates);
        for (indicator, gate) in indicators.iter_mut().zip(&frame.race_gates).take(count) {
            let spline = self.resource.as_ref().and_then(|r| r.find(gate.style.spline_asset))
                .map_or(SplineRef::None, SplineRef::Resource);
            if !indicator.enable || indicator.spline != spline {
                indicator.spline = spline;
                indicator.enable = true;
                indicator.calculate_bounds(self.resource.as_ref());
            }
            if gate.colour_valid {
                indicator.front_texture = self.registered(gate.style.front_texture).or(self.front_texture);
                indicator.back_texture = self.registered(gate.style.back_texture).or(self.back_texture);
            }
            // GetTransformMatrix82800c70: 2*halfLength/bounds for each axis.
            // A planar spline (bounds.z == 0) uses the X ratio on Z too.
            // Raw VX128 operands decoded: two reciprocal Newton rounds, then multiply.
            let bounds = vmx::sub(indicator.local_bounds[1], indicator.local_bounds[0]);
            let mut matrix: [V4; 4] = std::array::from_fn(|axis|
                std::array::from_fn(|lane| gate.transform[axis * 4 + lane]));
            for axis in 0..3 {
                let ratio_axis = if axis == 2 && bounds[2] == 0.0 { 0 } else { axis };
                let scale = vmx::reciprocal(bounds[ratio_axis]) * (gate.half_lengths[ratio_axis] * 2.0);
                matrix[axis][3] = 0.0;
                matrix[axis] = vmx::scale(matrix[axis], scale);
            }
            matrix[3][3] = 1.0;
            indicator.set_local_to_world(matrix);
            if indicator.cull(self.resource.as_ref(), view) {
                indicator.update(dt, &gate.style.settings, view, &mut self.random, &mut self.resource);
            }
        }
        for indicator in indicators.iter_mut().skip(count) { indicator.enable = false; }
        self.race_gates = indicators;
    }

    /// `VIPresentation::UpdateCustomizedSplines` (`0x828033a0`): every streamed spline in view
    /// updates with the `default` settings (`key_default`) and is disabled, then those the
    /// frame's spline indicators name are enabled for the render. PC port: a linked Skate 3
    /// data folder's take the frame's Skate 3 area textures.
    fn update_customized_splines(&mut self, dt: f32, frame: &VisualIndicatorFrame, view: &ViewInputs) {
        let front = self.registered(frame.skate3_area_textures.0).or(self.front_texture);
        let back = self.registered(frame.skate3_area_textures.1).or(self.back_texture);
        let mut no_resource = None;
        for (&asset, indicator) in &mut self.streamed_splines {
            if crate::skate3::is_id(asset) {
                indicator.front_texture = front;
                indicator.back_texture = back;
            }
            if indicator.cull(None, view) {
                indicator.update(dt, &frame.default_settings, view, &mut self.random, &mut no_resource);
            }
            indicator.enable = false;
        }
        for spline in &frame.splines {
            if let Some(indicator) = self.streamed_splines.get_mut(&spline.asset) {
                indicator.enable = true;
            }
        }
    }

    /// `VIPresentation::UpdateDynamicIndicator` (`0x82803fa0`): the first follow target's hips
    /// every `kDynamicIndicatorPointTime`, or no trail.
    fn update_dynamic_indicator(
        &mut self,
        dt: f32,
        frame: &VisualIndicatorFrame,
        view: &ViewInputs,
    ) {
        if frame.follow_targets.is_empty() {
            self.dynamic.clear();
            self.sample_timer = 0.0;
            return;
        }
        self.sample_timer = dt + self.sample_timer;
        if self.sample_timer > DYNAMIC_INDICATOR_POINT_TIME {
            if let Some(hips) = frame.follow_target_hips {
                self.dynamic.add_point(hips);
            }
            self.sample_timer -= DYNAMIC_INDICATOR_POINT_TIME;
        }
        self.dynamic.enable = true;
        self.dynamic
            .update(dt, &frame.trail_settings, view, &mut self.random);
    }

    /// `VIPresentation::Render` (`0x82803a18`)'s draws in order: streamed splines, location
    /// ribbons, race gates, then the trail, each enabled (and visible) one. Both techniques
    /// draw this list.
    #[must_use]
    pub fn render(&self, frame: &VisualIndicatorFrame, in_nis: bool) -> Vec<SplineDraw> {
        if !ENABLE_VIS || in_nis {
            return Vec::new();
        }
        let layers = frame.default_settings.layers;
        let resource = self.resource.as_ref();
        let mut draws: Vec<SplineDraw> = self
            .streamed_splines
            .values()
            .filter(|indicator| indicator.enable && indicator.visible)
            .filter_map(|indicator| indicator.draw(None, layers))
            .collect();
        draws.extend(
            self.location_ribbons
                .iter()
                .chain(&self.race_gates)
                .filter(|indicator| indicator.enable && indicator.visible)
                .filter_map(|indicator| indicator.draw(resource, layers)),
        );
        if self.dynamic.enable {
            draws.extend(self.dynamic.draw(&frame.default_settings));
        }
        draws
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skate2_formats::pegasus::AxisAlignedBox;

    fn indicator(flags: u32, segments: Vec<[[f32; 4]; 4]>) -> VisualIndicator {
        VisualIndicator {
            guid: 1,
            guid_local: 7,
            bounds: AxisAlignedBox {
                minimum: [-1.0, 0.0, 0.0, 0.0],
                maximum: [1.0, 2.0, 0.0, 0.0],
            },
            flags,
            segments,
        }
    }

    fn segment(x: f32, y: f32) -> [[f32; 4]; 4] {
        [
            [0.25, 0.0, 0.0, 0.0],
            [-0.25, 0.5, 0.0, 0.0],
            [0.0; 4],
            [x, y, 0.0, 1.0],
        ]
    }

    #[test]
    fn spline_control_points_repeat_the_ends_on_both_edges() {
        let spline = Spline::new(&indicator(9, vec![segment(0.0, 0.0), segment(1.0, 0.0)]));
        assert_eq!(
            (spline.cp_count, spline.segment_count, spline.vertex_count),
            (9, 6, 120)
        );
        assert!(spline.billboard);
        // The first segment's translation less the box centre (0, 1, 0).
        assert_eq!(spline.control_points[0], [0.0, -1.0, 0.0, 1.0]);
        assert_eq!(spline.control_points[3], spline.control_points[0]);
        // Then each segment's rows summed.
        assert_eq!(spline.control_points[4], [0.0, -0.5, 0.0, 1.0]);
        assert_eq!(spline.control_points[5], [1.0, -0.5, 0.0, 1.0]);
        assert_eq!(spline.control_points[8], spline.control_points[5]);
        assert_eq!(
            spline.control_points[EDGE..EDGE + 9],
            spline.control_points[..9]
        );
    }

    #[test]
    fn spline_vertices_pair_the_edges_and_accumulate_the_step() {
        let vertices = spline_vertices(6, 0.1);
        assert_eq!(vertices[0], [0.0, 0.0, 0.0]);
        assert_eq!(vertices[1], [71.0, 0.0, 1.0]);
        assert_eq!(vertices[4][0], 0.1_f32 + 0.1);
        let mut spline = Spline::new(&indicator(0, vec![segment(0.0, 0.0)]));
        spline.update(&VisualIndicatorSettings {
            samples: 3,
            ..Default::default()
        });
        assert_eq!((spline.vertex_count, spline.step), (30, 1.0 / 3.0));
        spline.update(&VisualIndicatorSettings {
            samples: 40,
            ..Default::default()
        });
        assert_eq!(spline.vertex_count, 100);
    }

    #[test]
    fn the_random_build_matches_the_particle_lcg() {
        let mut random = ParticleRandom(0);
        assert_eq!(ParticleRandom(1).next(), 0x10dcd + 0x29);
        let built = random.build([0.0; 4], [1.0; 4]);
        let mut seeds = ParticleRandom(0);
        let s = [seeds.next(), seeds.next(), seeds.next(), seeds.next()];
        let unit = |seed: u32| f32::from_bits((seed & 0x007f_ffff) | 0x3f80_0000) - 1.0;
        assert_eq!(built, [unit(s[3]), unit(s[2]), unit(s[1]), unit(s[0])]);
        assert_eq!(random, seeds);
    }

    #[test]
    fn a_trail_point_needs_a_quarter_metre() {
        let mut trail = DynamicIndicator::new();
        trail.add_point([0.0, 0.0, 0.0]);
        trail.add_point([0.2, 0.0, 0.0]);
        assert_eq!(trail.points.len(), 1);
        trail.add_point([0.3, 0.0, 0.0]);
        assert_eq!(trail.points.len(), 2);
        for step in 2..80 {
            #[expect(clippy::cast_precision_loss, reason = "test points")]
            trail.add_point([step as f32, 0.0, 0.0]);
        }
        assert_eq!(trail.points.len(), EDGE);
        assert_eq!(
            trail.base.own.control_points[EDGE + 70],
            [79.0, 0.0, 0.0, 0.0]
        );
    }

    fn shipped_data() -> VisualIndicatorData {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
        let archive = SKATE2_ARCHIVES
            .iter()
            .find(|a| a.relative_path == "data\\big\\miscboot.big")
            .unwrap();
        let mut mounts = MountSet::new();
        mounts.push(
            MountedBig::open(
                archive.path_under(std::path::Path::new(&root)),
                archive.use_external_header,
            )
            .unwrap(),
        );
        let bytes = mounts.read_path(ASSET_PATH).unwrap();
        let resources = skate2_formats::rw4::Arena::split_file_resources(&bytes).unwrap();
        let arena = skate2_formats::rw4::Arena::parse(&resources).unwrap();
        let entry = arena
            .dictionary
            .iter()
            .find(|e| e.type_id == skate2_formats::pegasus::VISUAL_INDICATOR_DATA_TYPE_ID)
            .unwrap();
        VisualIndicatorData::parse(arena.object_bytes(&resources, entry).unwrap()).unwrap()
    }

    /// `visualindicators` 0x8b41719b1f829fec as the Vault holds it.
    const SIGN_UP_SETTINGS: VisualIndicatorSettings = VisualIndicatorSettings {
        params: [
            0.0, 0.0, 100.0, 1.0, 4.0, 10.0, 90.0, 100.0, 0.0, 1.0, 1.0, 0.0,
        ],
        jitter: [f32::from_bits(0x3dcc_cccd), 0.5],
        jitter_period: f32::from_bits(0x3dcc_cccd),
        width: [f32::from_bits(0x3e19_999a), f32::from_bits(0x3f19_999a)],
        intensity: f32::from_bits(0x3e4c_cccd),
        blend_distance: [10.0, 80.0],
        darken_intensity: 1.0,
        samples: 3,
        layers: 3,
    };

    #[test]
    #[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
    fn the_shipped_ribbons_build_and_a_billboard_ribbon_in_view_draws_its_layers() {
        use super::super::visual_director::LocationRibbon;
        use skate2_formats::hash::rw_hash64_string;
        let data = shipped_data();
        assert_eq!((data.indicators.len(), data.segment_count), (36, 1769));
        let mut presentation = ViPresentation::new();
        presentation.add_instanced_splines(&data);
        let (front, back) = (
            rw_hash64_string(FRONT_TEXTURE),
            rw_hash64_string(BACK_TEXTURE),
        );
        presentation.register_textures([front, back]);
        // ribbon_spline `default`'s SplineAssetID.
        let asset = 0x0001_8e2b_03e3_8707;
        let resource = presentation.resource.as_ref().unwrap();
        let spline = resource.splines[resource.find(Some(asset)).unwrap()].clone();
        assert!(spline.billboard);
        let ribbon = LocationRibbon {
            transform: [
                1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 2., 20., 1.,
            ],
            spline_key: 0,
            colour_key: 0,
            render_settings_key: 0,
            style: RibbonStyle {
                spline_asset: Some(asset),
                front_texture: Some(front),
                back_texture: None,
                settings: SIGN_UP_SETTINGS,
            },
        };
        let frame = VisualIndicatorFrame {
            location_ribbons: vec![ribbon],
            default_settings: VisualIndicatorSettings {
                samples: 10,
                ..SIGN_UP_SETTINGS
            },
            ..VisualIndicatorFrame::default()
        };
        let camera = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let (near, far) = (0.1_f32, 1000.0_f32);
        let clip = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., far / (far - near), -near * far / (far - near)],
            [0., 0., 1., 0.],
        ];
        let view = ViewInputs::new(camera, clip);
        for tick in 0..12 {
            presentation.update(&frame, &view, (tick, 0.0), false, false);
        }
        let draws = presentation.render(&frame, false);
        assert_eq!(draws.len(), 1);
        let draw = &draws[0];
        assert_eq!(draw.textures, Some([Some(back), Some(front)]));
        assert_eq!(draw.vertices, (2 * spline.segment_count * 3, 1.0 / 3.0));
        assert_eq!(draw.vertex_count, u32::from(draw.vertices.0));
        assert_eq!(draw.layers.len(), 3);
        // The billboard turned about Y to face the camera down -Z.
        assert_eq!(
            draw.world,
            [[-1., 0., 0., 0.], [0., 1., 0., 2.], [0., 0., -1., 20.]]
        );
        // The second edge sits the blended width back along right plus up; the jitter stays
        // within half the blended amplitude.
        let blend = (20.0 - 10.0) / 70.0;
        let width = 0.15 * (1.0 - blend) + 0.6 * blend;
        for layer in &draw.layers {
            for index in 0..usize::from(spline.cp_count) {
                let edge = vmx::sub(layer[index], layer[EDGE + index]);
                assert!(
                    (edge[0] - width).abs() < 0.02
                        && (edge[1] - width).abs() < 0.02
                        && edge[2] == 0.0,
                    "{edge:?}"
                );
                let jitter = vmx::sub(layer[index], spline.control_points[index]);
                assert!(jitter.iter().all(|lane| lane.abs() <= 0.3), "{jitter:?}");
            }
        }
        // Nothing draws inside a scene, or once a scene cleared the render types.
        assert!(presentation.render(&frame, true).is_empty());
        presentation.update(&frame, &view, (12, 0.0), false, true);
        assert!(presentation.render(&frame, false).is_empty());
    }

    #[test]
    fn a_box_behind_the_near_plane_is_culled() {
        // Clip rows of an orthographic-like view looking down +Z from the origin.
        let clip = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, -1.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let camera = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let view = ViewInputs::new(camera, clip);
        assert!(view.intersects(&[[-0.5, -0.5, 4.0, 0.0], [0.5, 0.5, 5.0, 0.0]]));
        assert!(!view.intersects(&[[-0.5, -0.5, -5.0, 0.0], [0.5, 0.5, -4.0, 0.0]]));
    }
}
