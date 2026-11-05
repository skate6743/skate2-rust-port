//! Retained `Sk8::Physics::Reckoning` state (constructor 0x82dd7ab8).
//! Grounded state evolution is reconstructed separately from its pending
//! `PhysicalPlayerHiLOD` scheduling/processed-input producer. Inputs here are a
//! borrowed, explicitly supplied projection, not a fabricated live player.
//! Native concurrent/aliased input writes during this exclusive update are not
//! modeled. Source numeric primitives and current cvar reads remain services.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Reckoning awaits the live processed-input and player caller"
    )
)]

use crate::{
    animation::AnimationMatrix,
    reckoning_attributes::{ReckoningAttributeRead, ReckoningAttributeScope as Scope},
    reckoning_body_spin::{BodySpin, BodySpinCvars},
    reckoning_numeric::{self as numeric, ReckoningNumeric},
    reckoning_pid::{PhysicsControllerData, VectorPidFilter},
    skeleton_root_transforms::{SkeletonRootMatrix as Matrix, SkeletonRootVector as Vector},
};

#[path = "reckoning_air.rs"]
pub(crate) mod air;
#[path = "reckoning_grind.rs"]
mod grind;
#[path = "reckoning_grounded.rs"]
mod grounded;
#[path = "reckoning_grounded_input.rs"]
pub(crate) mod grounded_input;
#[cfg(test)]
#[path = "reckoning_tests.rs"]
mod tests;

const ZERO: [f32; 4] = [0.0; 4];
const I: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
const J: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// Current fields consumed from native `ProcessedPhysIn`, without unrelated
/// invented defaults. Production provenance and all other input fields remain
/// the responsibility of the pending original processed-input producer.
pub(crate) struct ReckoningGroundInput {
    pub(crate) com_to_deck: [f32; 4],    // +2f0
    pub(crate) deck_y: [f32; 4],         // +50
    pub(crate) deck_z: [f32; 4],         // +60
    pub(crate) effective_z: [f32; 4],    // +a0
    pub(crate) body_spin: f32,           // +9f8
    pub(crate) deck_speed: f32,          // +968
    pub(crate) absolute_body_speed: f32, // +948
    pub(crate) balance: f32,             // +9a0
    pub(crate) time_delta: f32,          // +93c
    pub(crate) wheels_in_contact: i32,   // +90c
    pub(crate) flags_8c4: u32,
    pub(crate) flags_8c8: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReckoningDebug {
    Reckoning,
    Coffin,
    BodyTilt,
}
/// Current reference resolution/value load at each source debug branch. A true
/// value is an explicit unsupported-overlay stop, never silently forced false.
pub(crate) trait ReckoningCvars: BodySpinCvars {
    fn debug_enabled(&mut self, kind: ReckoningDebug) -> Result<bool, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum ReckoningError<A, C, N> {
    Attribute(A),
    Input(grounded_input::GroundInputError),
    Cvar(C),
    Numeric(N),
    NegativeClampMaximum { bits: u32 },
    FlippingAssertion,
    DebugOverlayUnsupported(ReckoningDebug),
}
type RunResult<A, C, N, T = ()> = Result<T, ReckoningError<A, C, N>>;

impl<A: std::fmt::Display, C: std::fmt::Display, N: std::fmt::Display> std::fmt::Display
    for ReckoningError<A, C, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(e) => write!(f, "Reckoning attribute: {e}"),
            Self::Input(e) => e.fmt(f),
            Self::Cvar(e) => write!(f, "Reckoning current setting: {e}"),
            Self::Numeric(e) => write!(f, "Reckoning numeric service: {e}"),
            Self::NegativeClampMaximum { bits } => {
                write!(f, "Reckoning negative acceleration clamp ({bits:08x})")
            }
            Self::FlippingAssertion => {
                f.write_str("grounded Reckoning requires non-flipping state")
            }
            Self::DebugOverlayUnsupported(kind) => {
                write!(f, "Reckoning debug overlay is not reconstructed: {kind:?}")
            }
        }
    }
}
impl<A: std::error::Error + 'static, C: std::error::Error + 'static, N: std::error::Error + 'static>
    std::error::Error for ReckoningError<A, C, N>
{
}

/// One retained state/attribute owner, not a matrix-only output wrapper.
pub(crate) struct Reckoning<A> {
    attributes: A,
    body_spin: BodySpin,
    ground_transform: Matrix,
    transform: Matrix,
    transform_no_flip: Matrix,
    inverse_transform: Matrix,
    flip_transform: Matrix,
    body_spin_transform: Matrix,
    dynamic_up: Vector,
    up: Vector,
    up_target: Vector,
    up_speed: Vector,
    heading: Vector,
    ground_normal: Vector,
    flip_axis: Vector,
    animation_tilt: Vector,
    animation_tilt_speed: Vector,
    ground_filter: VectorPidFilter,
    slow_filter: VectorPidFilter,
    fast_filter: VectorPidFilter,
    spin_angle: f32,
    spin_speed: f32,
    dynamic_lean: f32,
    ground_blend: f32,
    flip_angle: f32,
    flip_speed: f32,
    target_flip_speed: f32,
    is_flipping: bool,
    is_front_flipping: bool,
}

impl<A: ReckoningAttributeRead> Reckoning<A> {
    /// MaintainFilters 0x82dd8248 updates both histories before overriding
    /// their current values; Air and Grind call this same native leaf.
    fn maintain_filters(&mut self) {
        self.slow_filter.update(self.up);
        self.fast_filter.update(self.up);
        self.slow_filter.set_value(self.up);
        self.fast_filter.set_value(self.up);
    }
    /// DoesRequestBodyFlipWipeout82dd8760..87d0, ordered comparisons.
    pub(crate) fn does_request_body_flip_wipeout(&self) -> bool {
        self.is_flipping
            && self.target_flip_speed == 0.0
            && self.transform.columns()[1].xyz[1] < 0.0
    }

    /// Constructor 0x82dd7ab8: source headings start I, not K. The successful
    /// host allocation path is modeled; native nullable allocation/debugger
    /// continuation and unpublished partial-object destruction are not claimed.
    pub(crate) fn try_new(mut attributes: A) -> Result<Self, A::Error> {
        let body_spin = BodySpin::new();
        let ground = read_initial_control(&mut attributes, 0x4c0)?;
        let slow = read_initial_control(&mut attributes, 0x4a0)?;
        let fast = read_initial_control(&mut attributes, 0x4b0)?;
        Ok(Self {
            attributes,
            body_spin,
            ground_transform: Matrix::known(IDENTITY),
            transform: Matrix::known(IDENTITY),
            transform_no_flip: Matrix::known(IDENTITY),
            inverse_transform: Matrix::known(IDENTITY),
            flip_transform: Matrix::known(IDENTITY),
            body_spin_transform: Matrix::known(IDENTITY),
            dynamic_up: Vector::known(J),
            up: Vector::known(J),
            up_target: Vector::known(J),
            up_speed: Vector::known(ZERO),
            heading: Vector::known(I),
            ground_normal: Vector::known(J),
            flip_axis: Vector::known(I),
            animation_tilt: Vector::known(ZERO),
            animation_tilt_speed: Vector::known(ZERO),
            ground_filter: VectorPidFilter::new(ground, J),
            slow_filter: VectorPidFilter::new(slow, J),
            fast_filter: VectorPidFilter::new(fast, J),
            spin_angle: 0.0,
            spin_speed: 0.0,
            dynamic_lean: 0.0,
            ground_blend: 0.0,
            flip_angle: 0.0,
            flip_speed: 0.0,
            target_flip_speed: 0.0,
            is_flipping: false,
            is_front_flipping: false,
        })
    }

    /// Selective `Reset` 0x82dd3c50; not `ResetSystems` or a fresh reconstruction.
    pub(crate) fn reset(&mut self) {
        self.up = Vector::known(J);
        self.up_target = Vector::known(J);
        self.dynamic_up = Vector::known(J);
        self.ground_normal = Vector::known(J);
        self.heading = Vector::known(I);
        self.up_speed = Vector::known(ZERO);
        self.animation_tilt = Vector::known(ZERO);
        self.animation_tilt_speed = Vector::known(ZERO);
        self.spin_angle = 0.0;
        self.spin_speed = 0.0;
        self.dynamic_lean = 0.0;
        self.ground_blend = 0.0;
    }

    /// `ResetBodySpin` 0x82dd4068: speed then angle, without resetting `BodySpin`.
    pub(crate) fn reset_body_spin(&mut self) {
        self.spin_speed = 0.0;
        self.spin_angle = 0.0;
    }
    /// ResetFlipping82dd4080 bakes the flip rotation into both retained axes.
    pub(crate) fn reset_flipping(&mut self) {
        let columns = self.flip_transform.columns();
        let rotate = |v: Vector| {
            blend_fma(
                columns[2],
                v.xyz[2],
                blend_fma(columns[1], v.xyz[1], scale(columns[0], v.xyz[0])),
            )
        };
        self.up = rotate(self.up);
        self.heading = rotate(self.heading);
        self.flip_transform = Matrix::known(IDENTITY);
        self.is_flipping = false;
        self.flip_speed = 0.0;
        self.is_front_flipping = false;
        self.flip_angle = 0.0;
    }

    /// Clone only immutable bound Vault instances; no Reckoning gameplay state.
    pub(crate) fn cloned_attribute_bindings(&self) -> A
    where
        A: Clone,
    {
        self.attributes.clone()
    }

    #[cfg(all(test, feature = "skate3-runtime"))]
    pub(crate) fn skate3_assist_snapshot(&self) -> (bool, [f32; 3]) {
        (self.is_flipping, [self.flip_angle, self.flip_speed, self.target_flip_speed])
    }

    pub(crate) fn body_spin(&self) -> (f32, f32) {
        (self.spin_speed, self.spin_angle)
    }
    pub(crate) fn heading(&self) -> Vector {
        self.heading
    }

    pub(crate) fn transform(&self) -> &Matrix {
        &self.transform
    }

    /// Skeleton.GetLaunchInfo82b46148 copies the retained inverse directly.
    pub(crate) fn launch_inverse_transform(&self) -> &Matrix {
        &self.inverse_transform
    }
    pub(crate) fn up(&self) -> Vector {
        self.up
    }
    /// Retained ground normal at native +0x1f0. Hook displacement reads this
    /// field, independently of the filtered current up vector at +0x1b0.
    pub(crate) fn ground_normal(&self) -> Vector {
        self.ground_normal
    }
    /// Retained native +20 ground basis, distinct from ordinary transform+60.
    pub(crate) fn ground_transform(&self) -> &Matrix {
        &self.ground_transform
    }
    pub(crate) fn dynamic_lean(&self) -> f32 {
        self.dynamic_lean
    }
    pub(crate) fn animation_tilt(&self) -> Vector {
        self.animation_tilt
    }

    fn graph(&mut self, scope: Scope, offset: usize, input: f32) -> Result<f32, A::Error> {
        let layout = self.attributes.capture_layout(scope)?;
        self.attributes.evaluate_graph8(&layout, offset, input)
    }

    /// `UpdateGroundNormal` 0x82dd8178. The raw result store is AFTER dot but
    /// BEFORE its estimate; errors preserve those distinct completed prefixes.
    fn update_ground_normal<C, N: ReckoningNumeric>(
        &mut self,
        target: Vector,
        n: &mut N,
    ) -> RunResult<A::Error, C, N::Error> {
        let layout = self
            .attributes
            .capture_layout(Scope::Own)
            .map_err(ReckoningError::Attribute)?;
        replace_control(
            &mut self.attributes,
            &layout,
            0x4c0,
            &mut self.ground_filter,
        )
        .map_err(ReckoningError::Attribute)?;
        let raw = self.ground_filter.update(target);
        let squared = n.dot_vector(raw, raw).map_err(ReckoningError::Numeric)?;
        self.ground_normal = raw;
        let seed = n
            .reciprocal_sqrt_estimate(squared)
            .map_err(ReckoningError::Numeric)?;
        self.ground_normal = numeric::normalize_from_squared_and_estimate(raw, squared, seed);
        self.ground_filter.set_value(self.ground_normal);
        Ok(())
    }

    /// `CalculateTransform` 0x82dd3da0: second cross uses UNNORMALIZED first.
    #[track_caller]
    fn calculate_transform<N: ReckoningNumeric>(&mut self, n: &mut N) -> Result<(), N::Error> {
        if !self.transform.has_nan_xyz()
            && (self.up.xyz.iter().chain(&self.heading.xyz).any(|v| !v.is_finite())
                || numeric::cross_typed(self.up, self.heading).xyz.iter().all(|v| v.abs() < 1e-12))
            && std::env::var_os("SKATE2_TRACE_RECKONING_NAN").is_some()
        {
            std::eprintln!(
                "reckoning basis degenerate after {}: up={:?} heading={:?} up_target={:?} up_speed={:?} dynamic_up={:?} ground_normal={:?} dynamic_lean={} spin_speed={} flipping={} flip_angle={} flip_speed={} animation_tilt={:?}",
                std::panic::Location::caller(),
                self.up, self.heading, self.up_target, self.up_speed, self.dynamic_up, self.ground_normal,
                self.dynamic_lean, self.spin_speed, self.is_flipping, self.flip_angle, self.flip_speed, self.animation_tilt,
            );
        }
        let (x, z) = basis(self.up, self.heading, n)?;
        self.heading = z;
        put_column(&mut self.transform, 0, x);
        put_column(&mut self.transform, 1, self.up);
        put_column(&mut self.transform, 2, z);
        self.transform_no_flip = self.transform;
        self.transform = self.flip_transform.compose(&self.transform_no_flip);
        self.inverse_transform = self.transform.inverse();
        let (x, z) = basis(self.ground_normal, self.heading, n)?;
        put_column(&mut self.ground_transform, 0, x);
        put_column(&mut self.ground_transform, 1, self.ground_normal);
        put_column(&mut self.ground_transform, 2, z);
        Ok(())
    }

    /// `CalculateDynamicLean` 0x82dd82d0, using FLIPPED transform Z.
    fn calculate_dynamic_lean<N: ReckoningNumeric>(&mut self, n: &mut N) -> Result<(), N::Error> {
        let z = column(&self.transform, 2);
        let up_dot = n.dot_vector(self.up, z)?;
        let dynamic_dot = n.dot_vector(self.dynamic_up, z)?;
        let up = project_off(self.up, z, up_dot);
        let dynamic = project_off(self.dynamic_up, z, dynamic_dot);
        let threshold = f32::from_bits(0x3727_c5ac);
        let value =
            if n.dot_vector(up, up)? > threshold && n.dot_vector(dynamic, dynamic)? > threshold {
                let angle = numeric::try_signed_angle(up, dynamic, z, n)?;
                numeric::try_normalize_angle(angle, n)?
            } else {
                0.0
            };
        self.dynamic_lean = value;
        Ok(())
    }

    /// `CalculateTilt` 0x82dd7e60; failed gate retains OLD animation tilt.
    fn calculate_tilt<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        input: &impl grounded_input::GroundInputRead,
        c: &mut C,
        n: &mut N,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        self.calculate_tilt_with_graphs(input, c, n, 0x320, 0x2a0)
    }
    /// Same `CalculateTilt82dd7e60`; Air caller passes layout+360/+2e0.
    fn calculate_tilt_with_graphs<C: ReckoningCvars, N: ReckoningNumeric>(
        &mut self,
        input: &impl grounded_input::GroundInputRead,
        c: &mut C,
        n: &mut N,
        rotation_offset: usize,
        slope_offset: usize,
    ) -> RunResult<A::Error, C::Error, N::Error> {
        // The caller captures layout BEFORE entering this helper, even if the
        // helper's first gate means no graph values will be consumed.
        let layout = self
            .attributes
            .capture_layout(Scope::Own)
            .map_err(ReckoningError::Attribute)?;
        let y = column(&self.transform_no_flip, 1);
        let dot = n.dot_vector(y, J).map_err(ReckoningError::Numeric)?;
        let projection = scale(y, dot);
        let perpendicular = sub(J, projection);
        let q = n
            .dot_vector(perpendicular, perpendicular)
            .map_err(ReckoningError::Numeric)?;
        if q.partial_cmp(&f32::from_bits(0x3a03_126f)) != Some(std::cmp::Ordering::Greater) {
            return Ok(());
        }
        #[expect(
            clippy::float_cmp,
            reason = "source exact XYZ equality gate, including signed zero and unordered NaN"
        )]
        let angle = if projection.xyz == J[..3] {
            0.0
        } else {
            let seed = n
                .reciprocal_sqrt_estimate(q)
                .map_err(ReckoningError::Numeric)?;
            let normal = numeric::normalize_from_squared_and_estimate(perpendicular, q, seed);
            numeric::try_signed_angle(column(&self.transform_no_flip, 2), normal, y, n)
                .map_err(ReckoningError::Numeric)?
        };
        let half_pi = f32::from_bits(0x3fc9_0fdb);
        let turns = (angle - half_pi) * f32::from_bits(0x3e22_f983);
        let radians = numeric::wrap_turns_to_radians(turns);
        let magnitude = f32::from_bits(radians.to_bits() & 0x7fff_ffff);
        let two_over_pi = f32::from_bits(0x3f22_f983);
        let difference = magnitude - half_pi;
        #[expect(
            clippy::neg_multiply,
            reason = "retain the native rounded multiply after subtraction, including signed-zero behavior"
        )]
        let negative = difference * -1.0;
        let mut rotation = negative * two_over_pi;
        if input.flag8c4(0x0010_0000).map_err(ReckoningError::Input)? {
            rotation = -rotation;
        }
        let slope = two_over_pi
            * (half_pi - numeric::try_asin(y.xyz[1], n).map_err(ReckoningError::Numeric)?);
        let rotational = self
            .attributes
            .evaluate_graph8(&layout, rotation_offset, rotation.abs())
            .map_err(ReckoningError::Attribute)?;
        let sloping = self
            .attributes
            .evaluate_graph8(&layout, slope_offset, slope.abs())
            .map_err(ReckoningError::Attribute)?;
        let signed = fsel(rotation, rotational, -rotational);
        debug_off::<A::Error, _, N::Error>(c, ReckoningDebug::BodyTilt)?;
        self.animation_tilt = Vector::known([sloping * signed, 0.0, 0.0, 0.0]);
        Ok(())
    }
}

fn read_initial_control<A: ReckoningAttributeRead>(
    a: &mut A,
    offset: usize,
) -> Result<PhysicsControllerData, A::Error> {
    let layout = a.capture_layout(Scope::Own)?;
    Ok(PhysicsControllerData {
        proportional: a.layout_float(&layout, offset)?,
        integral: a.layout_float(&layout, offset + 4)?,
        derivative: a.layout_float(&layout, offset + 8)?,
        integral_blend: a.layout_float(&layout, offset + 12)?,
    })
}
fn replace_control<A: ReckoningAttributeRead>(
    a: &mut A,
    layout: &A::Layout,
    offset: usize,
    filter: &mut VectorPidFilter,
) -> Result<(), A::Error> {
    let mut data = filter.control();
    data.proportional = a.layout_float(layout, offset)?;
    filter.replace_control(data);
    data.integral = a.layout_float(layout, offset + 4)?;
    filter.replace_control(data);
    data.derivative = a.layout_float(layout, offset + 8)?;
    filter.replace_control(data);
    data.integral_blend = a.layout_float(layout, offset + 12)?;
    filter.replace_control(data);
    Ok(())
}
fn basis<N: ReckoningNumeric>(
    up: Vector,
    heading: Vector,
    n: &mut N,
) -> Result<(Vector, Vector), N::Error> {
    let x = numeric::cross_typed(up, heading);
    let z = numeric::cross_typed(x, up);
    let qx = n.dot_vector(x, x)?;
    let sx = n.reciprocal_sqrt_estimate(qx)?;
    let qz = n.dot_vector(z, z)?;
    let sz = n.reciprocal_sqrt_estimate(qz)?;
    Ok((
        numeric::normalize_from_squared_and_estimate(x, qx, sx),
        numeric::normalize_from_squared_and_estimate(z, qz, sz),
    ))
}
fn debug_off<A, C: ReckoningCvars, N>(
    c: &mut C,
    kind: ReckoningDebug,
) -> RunResult<A, C::Error, N> {
    if c.debug_enabled(kind).map_err(ReckoningError::Cvar)? {
        Err(ReckoningError::DebugOverlayUnsupported(kind))
    } else {
        Ok(())
    }
}
fn column(m: &Matrix, index: usize) -> Vector {
    m.columns()[index]
}
fn put_column(m: &mut Matrix, index: usize, v: Vector) {
    m.set_column(index, v);
}
fn scale(a: impl Into<Vector>, s: f32) -> Vector {
    a.into().map(|v| v * s)
}
fn add(a: impl Into<Vector>, b: impl Into<Vector>) -> Vector {
    a.into().zip(b.into(), |a, b| a + b)
}
fn sub(a: impl Into<Vector>, b: impl Into<Vector>) -> Vector {
    a.into().zip(b.into(), |a, b| a - b)
}
fn project_off(a: impl Into<Vector>, axis: impl Into<Vector>, dot: f32) -> Vector {
    sub(a, scale(axis, dot))
}
fn blend_fma(fused: impl Into<Vector>, weight: f32, rounded: impl Into<Vector>) -> Vector {
    fused
        .into()
        .zip(rounded.into(), |a, b| a.mul_add(weight, b))
}
fn fsel(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}

#[path = "reckoning_fill_read.rs"]
pub(crate) mod fill_read;
