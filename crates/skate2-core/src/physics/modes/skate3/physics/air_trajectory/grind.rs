//! Real static University provider -> native trajectory admission -> landing orientation.
use crate::skate3_native::grind_world::StaticProvider;
use skate3_core::{
    air::trajectory::{
        Prediction,
        grind::*,
        grind_surface::{self, GeometryType, InvestigationInput, LandingOrientation},
    },
    physics::board_world::BoardWorld,
    player::input_phase::ProcessedPhysicsInput,
    point_graph::PointGraph,
};
use skate_data::collections::Collections;
type V = [f32; 4];
#[derive(Clone, Copy, Debug)]
pub(crate) struct GrindContext {
    pub board_position: V,
    pub body_position: V,
    pub actor: [u32; 2],
}
impl GrindContext {
    pub fn from_processed(p: &ProcessedPhysicsInput, board_position: V) -> Self {
        Self {
            board_position,
            body_position: p.vectors_544_560_592_608[2].map(f32::from_bits),
            actor: [p.actor_query_2948, p.actor_query_2952],
        }
    }
}
pub(super) struct Settings {
    limits: GrindAssistLimits,
    height: PointGraph<8>,
    padding: f32,
    maximum_adjust: f32,
    velocity_scalar: f32,
    max_angle: f32,
    score: f32,
    truck_distance: f32,
    penalty_domain: f32,
}
impl Settings {
    pub fn load(data: &Collections) -> Result<Self, String> {
        let t = |key| data.float("physics_trajectory", "default", key);
        Ok(Self {
            limits: GrindAssistLimits {
                lock_distance: 0.,
                max_speed_squared_ledge: t("GrindMaxSpeedSqrIntoLedge")?,
                max_speed_squared_rail: t("GrindMaxSpeedSqrIntoGrind")?,
                max_downward_speed: t("GrindMaxSpeedDownOntoGrind")?,
                ledge_scalars: [
                    t("GrindLockLedgeScalar2")?,
                    t("GrindLockLedgeScalar")?,
                    t("GrindLockLedgeLowSideScalar2")?,
                    t("GrindLockLedgeLowSideScalar")?,
                ],
                tip_scalar: t("GrindTipScalar")?,
                maximum_adjust_angle: t("GrindAdjustMaxAngle")?,
                deck_dimensions: [
                    data.float("physicsdeck", "default", "DeckMidLength")?,
                    data.float("physicsdeck", "default", "DeckFrontEndSize")?,
                ],
            },
            height: graph8(data, "physics_trajectory", "RequiredAngleVsHeight", true)?,
            padding: t("GrindOffset")?,
            maximum_adjust: t("MaxTrajectoryAdjust")?,
            velocity_scalar: t("GrindLandingVelScalar")?,
            max_angle: t("GrindLandingMaxAngle")?,
            score: t("ScoreGrind")?,
            truck_distance: data.float("physics_grinds", "default", "DeckCenterToTruck")?,
            penalty_domain: f32::from_bits(
                data.words::<20>("physics_trajectory", "default", "GrindPenaltyVsDistToGrind")?[2],
            ),
        })
    }
    pub fn evaluate(
        &self,
        prediction: &mut Prediction,
        acquire: bool,
        world: &BoardWorld,
        provider: &StaticProvider,
        nearby: &mut Vec<usize>,
        lock_distance: f32,
        context: GrindContext,
    ) -> Result<GrindEvaluation, String> {
        let collision_position = prediction.collision_position();
        let collision_velocity = prediction.collision_velocity();
        if acquire {
            let min =
                core::array::from_fn(|i| collision_position[i] - if i == 1 { 0.5 } else { 2. });
            let max =
                core::array::from_fn(|i| collision_position[i] + if i == 1 { 4. } else { 2. });
            let indices = provider.query(min, max)?;
            *nearby =
                trajectory_box_filter(&indices, provider.primitives(), context.board_position);
        }
        let mut result = GrindEvaluation {
            target: None,
            score: 0.,
            distance: 1000.,
            effective_lock_distance: None,
            penalty_domain: self.penalty_domain,
        };
        //82D69D68: retain the last real query for non-acquisition candidates.
        let mut square = 1_000_000.;
        for &index in nearby.iter() {
            let edge = provider.primitives()[index];
            let axis = unit(sub(edge.end, edge.start));
            let from = sub(collision_position, edge.start);
            let end = sub(collision_position, edge.end);
            let perpendicular = sub(from, scale(axis, dot(axis, from)));
            square = dot(perpendicular, perpendicular)
                .min(dot(from, from))
                .min(dot(end, end))
                .min(square);
        }
        if !nearby.is_empty() {
            result.distance = length_square(square);
        }
        if !acquire || nearby.is_empty() {
            return Ok(result);
        }
        let mut candidates = nearby
            .iter()
            .filter_map(|&i| {
                consider_grind_primitive(*prediction, provider.primitives()[i], i, self.padding)
            })
            .collect();
        let mut limits = self.limits.clone();
        limits.lock_distance = lock_distance;
        while let Some(c) = take_best_grind(&mut candidates, lock_distance, &self.height) {
            let edge = provider.primitives()[c.primitive];
            let query = InvestigationInput {
                start: edge.start,
                end: edge.end,
                reference: c.point,
                optional_probe: None,
                deck_center_to_truck: self.truck_distance,
            };
            //No submission is a real failed candidate, not an INVALID completed fallback.
            if grind_surface::prepare(query).is_none() {
                continue;
            }
            let surface = grind_surface::investigate(query, |index, probe| {
                crate::skate3_native::physics::player_input::grind::world::surface_probe(
                    world,
                    context.actor,
                    index,
                    probe,
                )
            })?;
            //All fields below are written by the completed producer before consumption.
            let mut orientation = LandingOrientation {
                kind: GeometryType::Impossible,
                garbage: false,
                boardslide_dir: [0.; 4],
                tipslide_dir: [0.; 4],
                backslash_dir: [0.; 4],
                high_side: [0.; 4],
            };
            let mut support = [0.; 4];
            grind_surface::update_landing_orientation(
                Some(&surface),
                context.board_position,
                c.point,
                &mut support,
                &mut orientation,
            );
            let Some(delta) = admitted_displacement(
                *prediction,
                c,
                edge,
                collision_velocity,
                context.body_position,
                GrindSurfaceEvidence {
                    kind: orientation.kind as u32,
                    side: orientation.high_side,
                },
                &limits,
                &mut result.effective_lock_distance,
            ) else {
                continue;
            };
            let original_velocity = prediction.request.trajectory.velocity;
            apply_admitted_target(
                prediction,
                c,
                delta,
                support,
                original_velocity,
                self.maximum_adjust,
                self.velocity_scalar,
                self.max_angle,
            );
            let vertical = cross(cross(c.direction, [0., 1., 0., 0.]), c.direction);
            let vertical = if vertical[1] < 0. {
                scale(vertical, -1.)
            } else {
                vertical
            };
            let vertical = if length_square(dot(vertical, vertical)) > f32::from_bits(0x3586_37bd) {
                unit(vertical)
            } else {
                [0., 1., 0., 0.]
            };
            let primitive_flags = provider
                .metadata(c.primitive)
                .ok_or("Accepted grind primitive lacks native metadata")?
                .flags;
            result.target = Some(GrindTarget {
                edge,
                provider_index: c.primitive,
                primitive_flags,
                orientation,
                point: c.point,
                vertical_normal: vertical,
            });
            result.score = self.score;
            return Ok(result);
        }
        Ok(result)
    }
}
fn dot(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn graph8(
    data: &Collections,
    class: &str,
    name: &str,
    negative: bool,
) -> Result<PointGraph<8>, String> {
    let expected = if negative {
        "Sk8::PointNegGraphData8"
    } else {
        "Sk8::PointGraphData8"
    };
    if data.field(class, "default", name)?.type_name != expected {
        return Err(format!("Wrong stock graph type {class}/{name}"));
    }
    let words = data.words::<20>(class, "default", name)?;
    Ok(PointGraph {
        x: core::array::from_fn(|i| f32::from_bits(words[4 + i])),
        y: core::array::from_fn(|i| f32::from_bits(words[12 + i])),
    })
}
fn sub(a: V, b: V) -> V {
    core::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V, s: f32) -> V {
    a.map(|v| v * s)
}
fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        0.,
    ]
}
fn inverse(square: f32) -> f32 {
    let mut r = square.sqrt().recip();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-square).mul_add(r * r, 1.), r);
    }
    r
}
fn length_square(square: f32) -> f32 {
    if square == 0. {
        0.
    } else {
        square * inverse(square)
    }
}
fn unit(a: V) -> V {
    let q = dot(a, a);
    if length_square(q) > f32::from_bits(0x3586_37bd) {
        scale(a, inverse(q))
    } else {
        [0.; 4]
    }
}
