//! TU3 world-contact production and native temporary-contact retention.
//! General primitive dispatch remains a separate provider dependency.
mod arithmetic;
mod axes;
mod box_feature;
mod buffer;
mod dispatch;
mod face_clip;
mod face_edge;
mod feature;
mod point_feature;
mod primitive_pair;
mod primitive_query;
mod prism_math;
pub use primitive_pair::{PrimitivePairSettings, primitive_pair_contacts};
mod projection;
mod reduction;
mod segment_face;
mod segment_prism;
mod triangle_feature;

pub use axes::separating_axis_candidates;
pub use box_feature::box_maximum_feature;
pub use buffer::ContactBuffer;
pub use dispatch::find_feature_intersection_prism;
pub use face_clip::{clamp_point_to_feature, clip_segment_to_feature};
pub use face_edge::intersect_feature_corner_edge;
pub use feature::{capsule_maximum_feature, initialize_feature_segment};
pub use point_feature::{closest_feature_segment, intersect_point_face};
pub use primitive_query::{
    ContactPrimitive, PrimitiveContactManifold, primitive_triangle_world_contacts,
    transform_triangle_volume, triangle_from_volume,
};
pub use projection::{
    PrimitiveKind, best_separating_direction, project_direction, project_directions,
};
pub use segment_face::intersect_segment_face;
pub use segment_prism::intersect_feature_segments;
pub use triangle_feature::{build_feature_edge_planes, triangle_maximum_feature};

/// Maximum-feature record consumed by 82ACE190.
pub type MaximumFeature = [u32; 144];
/// Native two point arrays, normal, count, flags and untouched trailing padding.
pub type FeaturePrism = [u32; 136];
pub use reduction::{coplanar_contacts, select_contact_points};

/// Native 256-byte contact record, including both body workspaces.
pub type ContactRecord = [u32; 64];
