//! Skate 3 controllers and calculations ported into Skate 2's physical player.
//! No standalone game, Bevy App, window, renderer, updater or device polling.
#![allow(dead_code)]
pub(crate) mod math_host {
    pub(crate) use glam::*;
    pub(crate) use super::transform::Transform;
    pub(crate) use super::trace::info_span;
    pub(crate) fn default<T: Default>() -> T { T::default() }
}
/// The source engine's log lines: info and debug only with `SKATE2_TRACE_SKATE3=1`, warnings and
/// errors always. Spans are names only.
pub(crate) mod trace {
    /// One log line from tracing-style fields (`key = value`, `key = ?value`, `key = %value`,
    /// `?value`, `%value`) and an optional message.
    macro_rules! trace_fields {
        ($line:ident;) => {};
        ($line:ident; $fmt:literal $(, $arg:expr)* $(,)?) => { { use ::std::fmt::Write; let _ = write!($line, $fmt $(, $arg)*); } };
        ($line:ident; $key:ident = ?$value:expr $(, $($rest:tt)*)?) => {
            { use ::std::fmt::Write; let _ = write!($line, "{}={:?} ", stringify!($key), $value); } $($crate::skate3_native::trace::trace_fields!($line; $($rest)*);)? };
        ($line:ident; $key:ident = %$value:expr $(, $($rest:tt)*)?) => {
            { use ::std::fmt::Write; let _ = write!($line, "{}={} ", stringify!($key), $value); } $($crate::skate3_native::trace::trace_fields!($line; $($rest)*);)? };
        ($line:ident; $key:ident = $value:expr $(, $($rest:tt)*)?) => {
            { use ::std::fmt::Write; let _ = write!($line, "{}={:?} ", stringify!($key), $value); } $($crate::skate3_native::trace::trace_fields!($line; $($rest)*);)? };
        ($line:ident; ?$value:ident $(, $($rest:tt)*)?) => {
            { use ::std::fmt::Write; let _ = write!($line, "{}={:?} ", stringify!($value), $value); } $($crate::skate3_native::trace::trace_fields!($line; $($rest)*);)? };
        ($line:ident; %$value:ident $(, $($rest:tt)*)?) => {
            { use ::std::fmt::Write; let _ = write!($line, "{}={} ", stringify!($value), $value); } $($crate::skate3_native::trace::trace_fields!($line; $($rest)*);)? };
        ($line:ident; $value:ident $(, $($rest:tt)*)?) => {
            { use ::std::fmt::Write; let _ = write!($line, "{}={:?} ", stringify!($value), $value); } $($crate::skate3_native::trace::trace_fields!($line; $($rest)*);)? };
    }
    macro_rules! trace_line {
        ($level:literal, $show:expr, target: $target:expr, $($rest:tt)*) => { $crate::skate3_native::trace::trace_line!($level, $show, $($rest)*) };
        ($level:literal, $show:expr, $($rest:tt)*) => {
            if $show {
                let mut line = ::std::string::String::new();
                $crate::skate3_native::trace::trace_fields!(line; $($rest)*);
                ::std::eprintln!("skate3 {}: {}", $level, line);
            }
        };
    }
    macro_rules! trace_info { ($($arg:tt)*) => { $crate::skate3_native::trace::trace_line!("info", $crate::diagnostics::test_log::enabled(), $($arg)*) } }
    macro_rules! trace_debug { ($($arg:tt)*) => { $crate::skate3_native::trace::trace_line!("debug", $crate::diagnostics::test_log::enabled(), $($arg)*) } }
    macro_rules! trace_warn { ($($arg:tt)*) => { $crate::skate3_native::trace::trace_line!("warning", true, $($arg)*) } }
    macro_rules! trace_error { ($($arg:tt)*) => { $crate::skate3_native::trace::trace_line!("error", true, $($arg)*) } }
    macro_rules! trace_span { ($($arg:tt)*) => { $crate::skate3_native::trace::Span } }
    pub(crate) use {trace_fields, trace_line};
    pub(crate) use {trace_debug as debug, trace_error as error, trace_info as info, trace_span as info_span, trace_warn as warn};
    pub(crate) struct Span;
    impl Span {
        pub(crate) fn in_scope<R>(&self, run: impl FnOnce() -> R) -> R { run() }
        pub(crate) fn entered(self) -> Self { self }
    }
}
/// The source engine's translation, rotation and scale, as Bevy's `Transform` gave them.
pub(crate) mod transform {
    use glam::{Mat4, Quat, Vec3};
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) struct Transform {
        pub(crate) translation: Vec3,
        pub(crate) rotation: Quat,
        pub(crate) scale: Vec3,
    }
    impl Default for Transform {
        fn default() -> Self { Self::IDENTITY }
    }
    impl Transform {
        pub(crate) const IDENTITY: Self = Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };
        pub(crate) fn from_matrix(matrix: Mat4) -> Self {
            let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
            Self { translation, rotation, scale }
        }
        pub(crate) const fn from_translation(translation: Vec3) -> Self { Self { translation, ..Self::IDENTITY } }
        pub(crate) const fn from_rotation(rotation: Quat) -> Self { Self { rotation, ..Self::IDENTITY } }
        pub(crate) const fn from_xyz(x: f32, y: f32, z: f32) -> Self { Self::from_translation(Vec3::new(x, y, z)) }
        #[must_use]
        pub(crate) const fn with_rotation(mut self, rotation: Quat) -> Self { self.rotation = rotation; self }
        #[must_use]
        pub(crate) const fn with_translation(mut self, translation: Vec3) -> Self { self.translation = translation; self }
        #[must_use]
        pub(crate) const fn with_scale(mut self, scale: Vec3) -> Self { self.scale = scale; self }
        pub(crate) fn to_matrix(&self) -> Mat4 { Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation) }
        pub(crate) fn compute_matrix(&self) -> Mat4 { self.to_matrix() }
        pub(crate) fn transform_point(&self, point: Vec3) -> Vec3 { self.rotation * (self.scale * point) + self.translation }
        pub(crate) fn forward(&self) -> Vec3 { self.rotation * Vec3::NEG_Z }
        pub(crate) fn back(&self) -> Vec3 { self.rotation * Vec3::Z }
        pub(crate) fn up(&self) -> Vec3 { self.rotation * Vec3::Y }
        pub(crate) fn down(&self) -> Vec3 { self.rotation * Vec3::NEG_Y }
        pub(crate) fn left(&self) -> Vec3 { self.rotation * Vec3::NEG_X }
        pub(crate) fn right(&self) -> Vec3 { self.rotation * Vec3::X }
    }
    impl std::ops::Mul for Transform {
        type Output = Self;
        fn mul(self, child: Self) -> Self {
            Self {
                translation: self.transform_point(child.translation),
                rotation: self.rotation * child.rotation,
                scale: self.scale * child.scale,
            }
        }
    }
    impl std::ops::Mul<Vec3> for Transform {
        type Output = Vec3;
        fn mul(self, point: Vec3) -> Vec3 { self.transform_point(point) }
    }
}
pub(crate) mod physics;
pub(crate) mod animation_pose;
pub(crate) mod skater_animation;
pub(crate) mod graph_host;
pub(crate) mod graph_runtime;
pub(crate) mod camera;
pub(crate) mod input;
pub(crate) mod grind_world;
pub(crate) mod skate_world;
pub(crate) mod scoring_runtime;
pub(crate) mod difficulty;
pub(crate) mod custom_difficulty;
pub(crate) mod crash_context;
pub(crate) mod modding;
pub(crate) mod animation;

pub(crate) mod world_types;
pub(crate) use world_types::{BodyOutput, BodyReaction, FrameOutput, SkeletonPartOutput, HostGrindAsset, HostGrindSegment, RawController, Scene, SceneRail, SceneTriangle, SceneTriangleFeatures};
pub(crate) use skate3_core::physics::board_world::{ExternalLineHit, ExternalQueries};
pub(crate) use skate3_core::player::offboard::grab_scene::{AssemblyData as GrabAssembly, Geometry as GrabGeometry, Object as GrabObject, Provider as GrabProvider, Spline as GrabSpline};
pub(crate) use skate_dynamics::{SolidBody, SolidCollider, rapier3d};
