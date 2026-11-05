//! TU3 feature-prism collision. Geometry and triangle adjacency come from the
//! collision mesh, independently of rendering and material response.
mod arithmetic;
mod edge;
mod fixup;
mod sphere_triangle;
mod world_contact;

pub use fixup::{ContactPair, TriangleFeature, TriangleFixup, TriangleRegion, fix_up_triangle};
pub use sphere_triangle::{Sphere, SphereTrianglePair, Triangle, intersect_sphere_triangle};
pub use world_contact::{
    WorldContact, WorldContactSettings, sphere_triangle_world_contact, world_separation_limit,
};
