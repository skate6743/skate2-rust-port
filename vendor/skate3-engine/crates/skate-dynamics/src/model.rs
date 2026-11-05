//! Bounded render-mesh -> compound-solid cooking. No asset paths or game types
//! live here: the host supplies vertices/triangles already in body-local space.
use super::Shape;
use rapier3d::parry::transformation::vhacd::VHACDParameters;
use rapier3d::prelude::{Pose, SharedShape, Vector};
use serde::{Deserialize, Serialize};

pub const MAX_MODEL_VERTICES: usize = 65_536;
pub const MAX_MODEL_TRIANGLES: usize = 131_072;
pub const MAX_COMPOUND_HULLS: usize = 64;
pub const MAX_HULL_POINTS: usize = 2_048;
/// Also bounds the compact network definition (96 KiB of vertex data).
pub const MAX_BODY_POINTS: usize = 8_192;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct ModelColliderOptions {
    pub max_hulls: u32,
    pub resolution: u32,
    pub concavity: f32,
    /// Must match the scale supplied to graphics.mesh for this instance.
    pub scale: [f32; 3],
}

impl Default for ModelColliderOptions {
    fn default() -> Self {
        Self {
            max_hulls: 32,
            resolution: 96,
            concavity: 0.0025,
            scale: [1.; 3],
        }
    }
}

impl ModelColliderOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_COMPOUND_HULLS as u32).contains(&self.max_hulls)
            || !(32..=128).contains(&self.resolution)
            || !self.concavity.is_finite()
            || !(0.0001..=0.1).contains(&self.concavity)
            || self
                .scale
                .iter()
                .any(|v| !v.is_finite() || v.abs() < 0.0001 || v.abs() > 100.)
        {
            return Err("model options: max_hulls 1..64, resolution 32..128, concavity 0.0001..0.1, finite nonzero scale required".into());
        }
        Ok(())
    }
}

pub fn validate_hulls(hulls: &[Vec<[f32; 3]>]) -> Result<(), String> {
    if hulls.is_empty() || hulls.len() > MAX_COMPOUND_HULLS {
        return Err("compound needs 1..64 convex parts".into());
    }
    let mut total = 0usize;
    for points in hulls {
        if !(4..=MAX_HULL_POINTS).contains(&points.len())
            || points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1000.)
        {
            return Err("compound contains invalid or oversized convex part".into());
        }
        total += points.len();
        if total > MAX_BODY_POINTS {
            return Err("compound exceeds 8192 hull vertices; lower max_hulls/resolution".into());
        }
    }
    Ok(())
}

pub fn compound_shape(hulls: &[Vec<[f32; 3]>]) -> Result<SharedShape, String> {
    validate_hulls(hulls)?;
    let mut shapes = Vec::with_capacity(hulls.len());
    for points in hulls {
        let vertices: Vec<_> = points.iter().copied().map(Vector::from_array).collect();
        let hull = SharedShape::convex_hull(&vertices).ok_or("degenerate compound hull")?;
        shapes.push((Pose::identity(), hull));
    }
    Ok(SharedShape::compound(shapes))
}

/// Use every valid source triangle; do not pick every Nth render vertex or
/// inflate one hull around disconnected details. All final parts share a single
/// rigid body. The caller applies one mass/inertia to the entire compound.
pub fn decompose(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    options: &ModelColliderOptions,
) -> Result<Shape, String> {
    options.validate()?;
    if !(4..=MAX_MODEL_VERTICES).contains(&vertices.len())
        || triangles.is_empty()
        || triangles.len() > MAX_MODEL_TRIANGLES
        || vertices
            .iter()
            .flatten()
            .any(|v| !v.is_finite() || v.abs() > 1000.)
        || triangles
            .iter()
            .flatten()
            .any(|&i| i as usize >= vertices.len())
    {
        return Err("invalid or oversized render-mesh collision input".into());
    }
    let points: Vec<Vector> = vertices.iter().copied().map(Vector::from_array).collect();
    let mut min = points[0];
    let mut max = min;
    for &p in &points {
        min = min.min(p);
        max = max.max(p);
    }
    let extent = max - min;
    if extent.min_element() < 0.0001 {
        return Err("model collision requires a nonzero 3D extent, not a flat/line mesh".into());
    }
    let parameters = VHACDParameters {
        max_convex_hulls: options.max_hulls,
        resolution: options.resolution,
        concavity: options.concavity,
        plane_downsampling: 2,
        convex_hull_downsampling: 2,
        convex_hull_approximation: false,
        ..Default::default()
    };
    let decomposed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        SharedShape::convex_decomposition_with_params(&points, triangles, &parameters)
    }))
    .map_err(|_| "render-model convex decomposition failed".to_owned())?;
    let compound = decomposed
        .as_compound()
        .ok_or("decomposition did not produce a compound")?;
    let mut hulls = Vec::with_capacity(compound.shapes().len());
    for (pose, shape) in compound.shapes() {
        let polyhedron = shape
            .as_convex_polyhedron()
            .ok_or("decomposition returned a nonconvex part")?;
        let (positions, _) = polyhedron.to_trimesh();
        let points: Vec<[f32; 3]> = positions
            .into_iter()
            .map(|p| (*pose * p).to_array())
            .collect();
        hulls.push(points);
    }
    validate_hulls(&hulls)?;
    Ok(Shape::Compound { hulls })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BodyDesc, DynamicsWorld};
    fn cube(center: [f32; 3]) -> Vec<[f32; 3]> {
        [-0.5, 0.5]
            .into_iter()
            .flat_map(|x| {
                [-0.5, 0.5].into_iter().flat_map(move |y| {
                    [-0.5, 0.5]
                        .into_iter()
                        .map(move |z| [x + center[0], y + center[1], z + center[2]])
                })
            })
            .collect()
    }
    #[test]
    fn compound_retains_empty_gap_and_one_body_mass() {
        let mut world = DynamicsWorld::default();
        let shape = Shape::Compound {
            hulls: vec![cube([-2., 0., 0.]), cube([2., 0., 0.])],
        };
        let id = world
            .spawn(BodyDesc {
                shape: shape.clone(),
                mass: 1400.,
                center_of_mass: [0., -0.27, 0.16],
                inertia_half_extents: Some([0.92, 0.55, 2.1]),
                ..Default::default()
            })
            .unwrap();
        let bodies = world.solid_bodies();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].colliders.len(), 2);
        // Inspect Rapier-derived physical properties, NOT snapshot metadata.
        assert!((bodies[0].inverse_mass - 1. / 1400.).abs() < 1e-8);
        assert!((bodies[0].center_of_mass - Vector::new(0., -0.27, 0.16)).length() < 1e-5);
        let expected_i = Vector::new(
            0.55 * 0.55 + 2.1 * 2.1,
            0.92 * 0.92 + 2.1 * 2.1,
            0.92 * 0.92 + 0.55 * 0.55,
        ) * (1400. / 3.);
        assert!((bodies[0].inverse_inertia - Vector::ONE / expected_i).length() < 1e-7);
        assert!(crate::solid::sweep_sphere(&bodies, [0., 3., 0.], [0., -3., 0.], 0.1).is_none());
        assert!(crate::solid::sweep_sphere(&bodies, [2., 3., 0.], [2., -3., 0.], 0.1).is_some());
        let definition = world.body_definition(id).unwrap();
        let mut remote = DynamicsWorld::default();
        let replica = remote.spawn_replica(&definition).unwrap();
        assert_eq!(remote.body_definition(replica).unwrap().body.shape, shape);
        assert_eq!(remote.solid_bodies()[0].colliders.len(), 2);
        assert!((remote.solid_bodies()[0].inverse_mass - bodies[0].inverse_mass).abs() < 1e-8);
    }
    #[test]
    fn invalid_geometry_and_options_are_rejected() {
        assert!(validate_hulls(&[]).is_err());
        assert!(validate_hulls(&[vec![[f32::NAN; 3]; 4]]).is_err());
        assert!(
            ModelColliderOptions {
                resolution: 4096,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
        assert!(decompose(&cube([0.; 3]), &[[0, 1, 999]], &Default::default()).is_err());
    }
    #[test]
    fn a_closed_cube_cooks_to_real_solid_parts() {
        let vertices = cube([0.; 3]);
        let triangles = [
            [0, 1, 3],
            [0, 3, 2],
            [4, 6, 7],
            [4, 7, 5],
            [0, 4, 5],
            [0, 5, 1],
            [2, 3, 7],
            [2, 7, 6],
            [0, 2, 6],
            [0, 6, 4],
            [1, 5, 7],
            [1, 7, 3],
        ];
        let options = ModelColliderOptions {
            resolution: 32,
            max_hulls: 4,
            ..Default::default()
        };
        let Shape::Compound { hulls } = decompose(&vertices, &triangles, &options).unwrap() else {
            panic!()
        };
        assert!(!hulls.is_empty());
        assert!(compound_shape(&hulls).is_ok());
    }
}
