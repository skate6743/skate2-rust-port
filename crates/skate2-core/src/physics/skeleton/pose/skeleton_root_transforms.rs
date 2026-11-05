//! Retained `Skeleton::UpdateRootTransforms` (0x82b42938).
//! Current animation/processed-input production remains an explicit caller
//! boundary. This is not the remainder of `GroundSkateboard` or `GeneralUpdate`.
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "root update awaits live Actor scheduling")
)]

use crate::{
    animation::AnimationMatrix,
    reckoning::Reckoning,
    reckoning_attributes::ReckoningAttributeRead,
    reckoning_numeric::{ReckoningNumeric, try_signed_angle},
    reckoning_sin_cos::{ReckoningSinCosNumeric, try_sin_cos},
    skateboard_spawn_math::{
        SkateboardSpawnMathNumeric, skateboard_spawn_compose, skateboard_spawn_normalize,
        skateboard_spawn_rigid_inverse,
    },
};

const IDENTITY: AnimationMatrix = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
];

/// Native velocity W is an intrusive link, not a known homogeneous zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonRootVector {
    pub(crate) xyz: [f32; 3],
    pub(crate) w: Option<f32>,
}
impl SkeletonRootVector {
    pub(crate) fn map(self, f: impl Fn(f32) -> f32) -> Self {
        Self {
            xyz: self.xyz.map(&f),
            w: self.w.map(f),
        }
    }
    pub(crate) fn zip(self, other: Self, f: impl Fn(f32, f32) -> f32) -> Self {
        Self {
            xyz: std::array::from_fn(|i| f(self.xyz[i], other.xyz[i])),
            w: self.w.zip(other.w).map(|(a, b)| f(a, b)),
        }
    }
    pub(crate) fn known_lanes(self) -> Option<[f32; 4]> {
        Some([self.xyz[0], self.xyz[1], self.xyz[2], self.w?])
    }
    pub(crate) const fn known(value: [f32; 4]) -> Self {
        Self {
            xyz: [value[0], value[1], value[2]],
            w: Some(value[3]),
        }
    }
}

impl From<[f32; 4]> for SkeletonRootVector {
    fn from(value: [f32; 4]) -> Self {
        Self::known(value)
    }
}

/// Each native lane is represented once. Unknown W never becomes a stored0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkeletonRootMatrix {
    columns: [SkeletonRootVector; 4],
}
impl SkeletonRootMatrix {
    pub(crate) fn from_columns(columns: [SkeletonRootVector; 4]) -> Self {
        Self { columns }
    }
    pub(crate) fn known_lanes(&self) -> Option<AnimationMatrix> {
        let mut lanes = [0.0; 16];
        for (index, column) in self.columns.iter().enumerate() {
            lanes[index * 4..index * 4 + 3].copy_from_slice(&column.xyz);
            lanes[index * 4 + 3] = column.w?;
        }
        Some(lanes)
    }
    pub(crate) fn known(value: AnimationMatrix) -> Self {
        Self {
            columns: std::array::from_fn(|column| {
                SkeletonRootVector::known(std::array::from_fn(|lane| value[column * 4 + lane]))
            }),
        }
    }
    pub(crate) fn set_column(&mut self, index: usize, value: SkeletonRootVector) {
        self.columns[index] = value;
    }
    pub(crate) fn columns(&self) -> &[SkeletonRootVector; 4] {
        &self.columns
    }

    /// Board effective-transform getters/setters XOR the sign bit of every
    /// X/Z lane when Processed+0x8c4 says the board is flipped. Preserve
    /// negative zero and NaN payloads; an unknown fourth lane stays unknown.
    pub(crate) fn with_board_flip(mut self, flipped: bool) -> Self {
        if flipped {
            for index in [0, 2] {
                let sign = |value: f32| f32::from_bits(value.to_bits() ^ 0x8000_0000);
                self.columns[index].xyz = self.columns[index].xyz.map(sign);
                self.columns[index].w = self.columns[index].w.map(sign);
            }
        }
        self
    }

    // Only XYZ-consuming operations may use this private scratch. Every
    // affected result W is separately established below before publication.
    pub(crate) fn xyz_scratch(&self) -> AnimationMatrix {
        std::array::from_fn(|index| {
            let column = self.columns[index / 4];
            if index % 4 == 3 {
                column.w.unwrap_or(0.0)
            } else {
                column.xyz[index % 4]
            }
        })
    }
    pub(crate) fn translation(value: SkeletonRootVector) -> Self {
        let mut result = Self::known(IDENTITY);
        result.columns[3] = value;
        result
    }
    /// Native heading yaw columns used by grounded and OffBoard root updates;
    /// the X/Z W lanes intentionally repeat cosine/sine.
    pub(crate) fn heading_yaw(sin: f32, cos: f32) -> Self {
        Self::known([
            cos,
            0.0,
            f32::from_bits(sin.to_bits() ^ 0x8000_0000),
            cos,
            0.0,
            1.0,
            0.0,
            0.0,
            sin,
            0.0,
            cos,
            sin,
            0.0,
            0.0,
            0.0,
            0.0,
        ])
    }
    pub(crate) fn compose(&self, inner: &Self) -> Self {
        let mut result = Self::known(skateboard_spawn_compose(
            &self.xyz_scratch(),
            &inner.xyz_scratch(),
        ));
        for column in 0..4 {
            // Inner W is never a coefficient. Unknown outer W cannot be
            // elided even for coefficient0 (native W could be NaN/infinity).
            if self.columns[..3].iter().any(|value| value.w.is_none())
                || (column == 3 && self.columns[3].w.is_none())
            {
                result.columns[column].w = None;
            }
        }
        result
    }
    pub(crate) fn inverse(&self) -> Self {
        // The rigid inverse consumes only input XYZ, explicitly sets basis
        // W+0, then computes translation W with the actual zero-term graph.
        Self::known(skateboard_spawn_rigid_inverse(&self.xyz_scratch()))
    }
    pub(crate) fn has_nan_xyz(&self) -> bool {
        self.columns
            .iter()
            .any(|column| column.xyz.iter().any(|value| value.is_nan()))
    }
}

pub(crate) trait SkeletonRootBoard {
    type Error;
    fn transform(&mut self) -> Result<AnimationMatrix, Self::Error>;
    /// Controlled raw-lane compatibility. Actual Parts override this service.
    fn transform_typed(&mut self) -> Result<SkeletonRootMatrix, Self::Error> {
        self.transform().map(SkeletonRootMatrix::known)
    }
    fn velocity(&mut self) -> Result<SkeletonRootVector, Self::Error>;
}

/// Supplied current fields, not a recovered `ProcessData` producer. In
/// particular `skate_animation` is CURRENT `SkateTransformAnim`, never last world.
pub(crate) struct SkeletonRootInput {
    pub(crate) time_delta: f32,
    pub(crate) skate_animation: AnimationMatrix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SkeletonRootAssertion {
    AnimationToBoard,
    PredictedBoardPosition,
    AnimationToWorld,
    NormalizedAnimationToWorld,
}
#[derive(Debug)]
pub(crate) enum SkeletonRootError<B, N, S> {
    Board(B),
    Numeric(N),
    SinCos(S),
    Assertion(SkeletonRootAssertion),
}
type RootResult<B, N, S> = Result<(), SkeletonRootError<B, N, S>>;
impl<B: std::fmt::Debug, N: std::fmt::Debug, S: std::fmt::Debug> std::fmt::Display
    for SkeletonRootError<B, N, S>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Skeleton root transforms: {self:?}")
    }
}
impl<B: std::fmt::Debug, N: std::fmt::Debug, S: std::fmt::Debug> std::error::Error
    for SkeletonRootError<B, N, S>
{
}

/// Sole retained root projection inside the actual `SkeletonNormalAssembly`.
/// Constructor82b56d70 initializes these matrices I/J/K/P0 (all W0), prior
/// positions0, prediction-valid0 and heading1. ResetSkeleton/TeleportReset
/// do not reset them. Not Clone: frame-to-frame state has one mutable owner.
pub(crate) struct SkeletonRootTransforms {
    board: SkeletonRootMatrix,
    board_inverse: SkeletonRootMatrix,
    animation_angle: SkeletonRootMatrix,
    animation_to_board: SkeletonRootMatrix,
    animation_to_world: SkeletonRootMatrix,
    world_to_animation: SkeletonRootMatrix,
    predicted_board_position: SkeletonRootVector,
    last_board_position: SkeletonRootVector,
    prediction_valid: bool,
    update_heading: bool,
}
impl SkeletonRootTransforms {
    /// `Skeleton::UpdateOffBoard` 0x82b5c428 sets this byte before calling
    /// `SetOffBoardSkateRootTransform` on the same retained Skeleton.
    pub(crate) fn begin_offboard_update(&mut self) {
        self.update_heading = true;
    }
    /// `PhysicsAir::Enter` 0x82d98fa8..fb0 stores 1 to the same Skeleton
    /// byte+3ded when the predecessor was not Ground/GroundAnimation/KnownAir.
    pub(crate) fn request_air_entry_heading(&mut self) {
        self.update_heading = true;
    }
    pub(crate) fn new() -> Self {
        let identity = SkeletonRootMatrix::known(IDENTITY);
        Self {
            board: identity,
            board_inverse: identity,
            animation_angle: identity,
            animation_to_board: identity,
            animation_to_world: identity,
            world_to_animation: identity,
            predicted_board_position: SkeletonRootVector::known([0.0; 4]),
            last_board_position: SkeletonRootVector::known([0.0; 4]),
            prediction_valid: false,
            update_heading: true,
        }
    }
    /// One-time solver handoff. A dormant S2 prediction must not override the
    /// completed physical root when this assembly resumes its ordinary updates.
    #[cfg(feature = "skate3-runtime")]
    pub(crate) fn accept_mode_return(&mut self, root: SkeletonRootMatrix, board: SkeletonRootMatrix) {
        self.animation_to_world = root;
        self.world_to_animation = root.inverse();
        self.board = board;
        self.board_inverse = board.inverse();
        self.animation_to_board = self.board_inverse.compose(&root);
        self.last_board_position = board.columns()[3];
        self.predicted_board_position = self.last_board_position;
        self.prediction_valid = false;
    }
    pub(crate) fn animation_to_world(&self) -> &SkeletonRootMatrix {
        &self.animation_to_world
    }
    /// ReorientSkaterUpright 0x82b3b804..820 and UpdateRootTransformsOffBoard
    /// 0x82b44db8..c4 publish all four columns to this same retained matrix.
    pub(crate) fn set_offboard_animation_to_world(&mut self, value: SkeletonRootMatrix) {
        self.animation_to_world = value;
    }

    /// OffBoard COM-step correction 0x82b4541c stores only column P.
    pub(crate) fn set_offboard_animation_translation(&mut self, value: SkeletonRootVector) {
        self.animation_to_world.set_column(3, value);
    }
    pub(crate) fn world_to_animation(&self) -> &SkeletonRootMatrix {
        &self.world_to_animation
    }
    /// OffBoard 0x82b45740..758 stores the rigid inverse in native column
    /// order 2, 1, 0, P on the same retained world-to-animation matrix.
    pub(crate) fn set_offboard_world_to_animation_inverse(&mut self, inverse: SkeletonRootMatrix) {
        for column in [2, 1, 0, 3] {
            self.world_to_animation
                .set_column(column, inverse.columns()[column]);
        }
    }
    /// Constructor identity and the sole inverse producer both establish every
    /// lane. Other retained roots may still have unknown W.
    pub(crate) fn world_to_animation_lanes(&self) -> AnimationMatrix {
        std::array::from_fn(|index| {
            let column = self.world_to_animation.columns[index / 4];
            if index % 4 == 3 {
                column.w.expect("constructor and rigid inverse establish W")
            } else {
                column.xyz[index % 4]
            }
        })
    }
    pub(crate) fn animation_to_board(&self) -> &SkeletonRootMatrix {
        &self.animation_to_board
    }
    pub(crate) fn board(&self) -> &SkeletonRootMatrix {
        &self.board
    }
    pub(crate) fn board_inverse(&self) -> &SkeletonRootMatrix {
        &self.board_inverse
    }
    pub(crate) fn last_board_position(&self) -> SkeletonRootVector {
        self.last_board_position
    }
    pub(crate) fn predicted_board_position(&self) -> SkeletonRootVector {
        self.predicted_board_position
    }

    /// Update3 82dfe774/778 publishes the full predicted position before validity.
    pub(crate) fn publish_board_prediction(&mut self, position: SkeletonRootVector) {
        self.predicted_board_position = position;
        self.prediction_valid = true;
    }

    /// `ProcessData82b5f300` clears only this retained prediction-valid byte.
    pub(crate) fn invalidate_prediction_after_process_data_teleport(&mut self) {
        self.prediction_valid = false;
    }

    #[cfg(test)]
    pub(crate) fn seed_general_update_inputs(
        &mut self,
        animation_to_board: SkeletonRootMatrix,
        animation_to_world: SkeletonRootMatrix,
        board_inverse: SkeletonRootMatrix,
        predicted_board_position: SkeletonRootVector,
    ) {
        self.animation_to_board = animation_to_board;
        self.animation_to_world = animation_to_world;
        self.board_inverse = board_inverse;
        self.predicted_board_position = predicted_board_position;
    }

    /// `GroundSkateboard` 0x82b5e488 heading store and its root-update call ONLY.
    /// No claim that handplants, processed orientation, hooks/reset/IK ran.
    pub(crate) fn try_update_grounded<A, B, N, S, M>(
        &mut self,
        input: &SkeletonRootInput,
        reckoning: &Reckoning<A>,
        board: &mut B,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> RootResult<B::Error, N::Error, S::Error>
    where
        A: ReckoningAttributeRead,
        B: SkeletonRootBoard,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        self.update_heading = true;
        let result =
            self.try_update_typed(input, reckoning.transform(), board, numeric, sin_cos, ortho);
        if let Err(SkeletonRootError::Assertion(SkeletonRootAssertion::AnimationToBoard)) = &result
        {
            // Native physicsskeleton.cpp:3128 FailAssert. Fatal-path context
            // only: which composed input carried the NaN. The error is unchanged.
            std::eprintln!(
                "diagnostic grounded root assertion AnimationToBoard: reckoning up={:?} heading={:?} transform={:?}; skate_animation={:?}; animation_angle={:?}; board={:?}",
                reckoning.up(),
                reckoning.heading(),
                reckoning.transform(),
                input.skate_animation,
                self.animation_angle,
                self.board,
            );
        }
        result
    }

    /// `UpdateRootTransforms82b42938` without `GroundSkateboard`'s heading store.
    pub(crate) fn try_update_native<A, B, N, S, M>(
        &mut self,
        input: &SkeletonRootInput,
        reckoning: &Reckoning<A>,
        board: &mut B,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> RootResult<B::Error, N::Error, S::Error>
    where
        A: ReckoningAttributeRead,
        B: SkeletonRootBoard,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        self.try_update_typed(input, reckoning.transform(), board, numeric, sin_cos, ortho)
    }

    #[cfg(test)]
    fn try_update<B, N, S, M>(
        &mut self,
        input: &SkeletonRootInput,
        reckoning: &AnimationMatrix,
        board: &mut B,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> RootResult<B::Error, N::Error, S::Error>
    where
        B: SkeletonRootBoard,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        self.try_update_typed(
            input,
            &SkeletonRootMatrix::known(*reckoning),
            board,
            numeric,
            sin_cos,
            ortho,
        )
    }

    fn try_update_typed<B, N, S, M>(
        &mut self,
        input: &SkeletonRootInput,
        reckoning: &SkeletonRootMatrix,
        board: &mut B,
        numeric: &mut N,
        sin_cos: &mut S,
        ortho: &mut M,
    ) -> RootResult<B::Error, N::Error, S::Error>
    where
        B: SkeletonRootBoard,
        N: ReckoningNumeric,
        S: ReckoningSinCosNumeric,
        M: SkateboardSpawnMathNumeric,
    {
        use SkeletonRootError as Error;
        self.last_board_position = self.board.columns[3];
        self.board = board.transform_typed().map_err(Error::Board)?;
        self.board_inverse = self.board.inverse();
        let supplied = self.prediction_valid;
        if !self.prediction_valid {
            let velocity = board.velocity().map_err(Error::Board)?;
            self.predicted_board_position = SkeletonRootVector {
                xyz: std::array::from_fn(|lane| {
                    velocity.xyz[lane].mul_add(input.time_delta, self.board.columns[3].xyz[lane])
                }),
                w: velocity
                    .w
                    .zip(self.board.columns[3].w)
                    .map(|(velocity, position)| velocity.mul_add(input.time_delta, position)),
            };
        }
        self.prediction_valid = false;
        if self.update_heading {
            let heading = [
                input.skate_animation[8],
                0.0,
                input.skate_animation[10],
                input.skate_animation[11],
            ];
            let angle =
                try_signed_angle(heading, [0.0, 0.0, 1.0, 0.0], [0.0, 1.0, 0.0, 0.0], numeric)
                    .map_err(Error::Numeric)?;
            let (sin, cos) = try_sin_cos(angle, sin_cos).map_err(Error::SinCos)?;
            self.animation_angle = SkeletonRootMatrix::heading_yaw(sin, cos);
            self.update_heading = false;
        }
        let negative_position = SkeletonRootVector::known(std::array::from_fn(|lane| {
            f32::from_bits(input.skate_animation[12 + lane].to_bits() ^ 0x8000_0000)
        }));
        let shifted = self
            .animation_angle
            .compose(&SkeletonRootMatrix::translation(negative_position));
        self.animation_to_board = reckoning.compose(&shifted);
        if self.animation_to_board.has_nan_xyz() {
            return Err(Error::Assertion(SkeletonRootAssertion::AnimationToBoard));
        }
        if self
            .predicted_board_position
            .xyz
            .iter()
            .any(|value| value.is_nan())
        {
            return Err(Error::Assertion(
                SkeletonRootAssertion::PredictedBoardPosition,
            ));
        }
        self.animation_to_world = SkeletonRootMatrix::translation(self.predicted_board_position)
            .compose(&self.animation_to_board);
        if self.animation_to_world.has_nan_xyz() {
            return Err(Error::Assertion(SkeletonRootAssertion::AnimationToWorld));
        }
        // This particular composition has known basis W. Ortho reads basis
        // only and copies P; preserve the actual possibly-unknown P after the
        // existing complete source helper, not per-axis root-state stores.
        if trace_visual_root() {
            let velocity = board.velocity().map(|v| v.xyz).unwrap_or([f32::NAN; 3]);
            eprintln!(
                "skate2 visual root: supplied={supplied} predicted={:?} board={:?} last_board={:?} velocity={velocity:?} dt={} root={:?}",
                self.predicted_board_position.xyz, self.board.columns[3].xyz, self.last_board_position.xyz,
                input.time_delta, self.animation_to_world.columns[3].xyz,
            );
        }
        let position = self.animation_to_world.columns[3];
        let normalized = skateboard_spawn_normalize(&self.animation_to_world.xyz_scratch(), ortho);
        self.animation_to_world = SkeletonRootMatrix::known(normalized);
        self.animation_to_world.columns[3] = position;
        if self.animation_to_world.has_nan_xyz() {
            return Err(Error::Assertion(
                SkeletonRootAssertion::NormalizedAnimationToWorld,
            ));
        }
        self.world_to_animation = self.animation_to_world.inverse();
        Ok(())
    }
}

#[cfg(test)]
#[path = "skeleton_root_transforms_tests.rs"]
mod tests;

#[path = "skeleton_root_known_air.rs"]
pub(crate) mod known_air;

#[path = "skeleton_root_fixed_part.rs"]
pub(crate) mod fixed_part;

#[path = "skeleton_root_hand_plant.rs"]
pub(crate) mod hand_plant;

#[path = "skeleton_root_wipeout.rs"]
pub(crate) mod wipeout;

/// SKATE2_TRACE_SKATE2_VISUAL_ROOT: each root update's predicted board position, whether it was
/// supplied, and the board it was placed against.
fn trace_visual_root() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("SKATE2_TRACE_SKATE2_VISUAL_ROOT").is_some())
}
