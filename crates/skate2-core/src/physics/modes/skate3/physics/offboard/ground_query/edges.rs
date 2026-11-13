use super::{Bounds, EdgeBody, PrimaryEdges, Scene, Segment, transform};
use skate3_core::math::Vector3;
use skate3_core::player::offboard::ground_query::{Edge, EdgeSearch, Frame};
fn append(output: &mut Vec<Edge>, segments: &[Segment], frame: Frame, query: Bounds) {
    for segment in segments {
        if output.len() == 40 {
            break;
        }
        if transform::overlaps(transform::bounds(frame, segment.local_bounds), query) {
            output.push(Edge {
                start: transform::point(frame, segment.edge.start),
                end: transform::point(frame, segment.edge.end),
            });
        }
    }
}
fn body(output: &mut Vec<Edge>, body: &EdgeBody<'_>, query: Bounds) {
    if transform::overlaps(
        transform::bounds(body.local_to_world, body.local_bounds),
        query,
    ) {
        append(output, body.segments, body.local_to_world, query);
    }
}
pub(super) fn candidates(scene: &Scene<'_>, search: &EdgeSearch) -> Result<Vec<Edge>, &'static str> {
    let query = Bounds {
        min: search.min,
        max: search.max,
    };
    let mut output = Vec::with_capacity(40);
    //82C1EAD8: the grind data's static pass comes first and shares the cap of forty.
    if let Some(grind) = scene.grind {
        let primitives = grind
            .box_test(
                [search.min.x, search.min.y, search.min.z],
                [search.max.x, search.max.y, search.max.z],
            )
            .map_err(|_| "Invalid grind edge query bounds")?;
        output.extend(primitives.into_iter().map(|primitive| Edge {
            start: Vector3::new(primitive.start[0], primitive.start[1], primitive.start[2]),
            end: Vector3::new(primitive.end[0], primitive.end[1], primitive.end[2]),
        }));
    }
    append(&mut output, scene.static_edges, Frame::IDENTITY, query);
    match &scene.primary_edges {
        PrimaryEdges::Normal { dynamic, vehicles } => {
            let center = transform::scale(transform::add(query.min, query.max), 0.5);
            for provider in [*dynamic, *vehicles] {
                for entry in provider {
                    let d = transform::sub(entry.local_to_world.position, center);
                    let squared = d.z.mul_add(d.z, d.y.mul_add(d.y, d.x * d.x));
                    if squared >= 225. {
                        continue;
                    }
                    //82C4C4E0 checks individual segment bounds, no asset bound gate.
                    append(&mut output, entry.segments, entry.local_to_world, query);
                    if output.len() == 40 {
                        break;
                    }
                }
            }
        }
        PrimaryEdges::Alternate(records) => {
            for record in *records {
                if !record.enabled {
                    continue;
                }
                let choice = usize::from(search.context.selection_flags_2948 & 2 != 0);
                if let Some((entry, group)) = record.choices[choice] {
                    if transform::matches(search.context.matching_id_2952, group) {
                        body(&mut output, entry, query);
                    }
                }
            }
        }
    }
    let mut ordered: Vec<_> = scene.indexed_edges.iter().collect();
    ordered.sort_by_key(|entry| entry.id);
    for entry in ordered {
        if !entry.disabled {
            body(&mut output, entry.body, query);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod grind_edge_tests {
    use super::super::{PrimaryEdges, Scene};
    use crate::skate3_native::{
        grind_world::StaticProvider,
        world_types::{HostGrindAsset, HostGrindSegment},
    };
    use skate3_core::{
        math::Vector3,
        player::offboard::ground_query::{EdgeSearch, Frame, GroundQueryScene, QueryContext},
    };

    /// One straight 2 m grind segment along +X from (x, 1, 0).
    fn rail(x: f32) -> HostGrindAsset {
        HostGrindAsset {
            segments: vec![HostGrindSegment {
                coefficients: [[-4., 0., 0., 0.], [6., 0., 0., 0.], [0.; 4], [x, 1., 0., 1.]],
                minimum: [x, 1., 0.],
                maximum: [x + 2., 1., 0.],
                owner: 1,
                spline_guids: [1, 2],
                segment_index: 0,
            }],
        }
    }

    #[test]
    fn the_edge_query_returns_the_grind_segments_in_its_box() {
        let provider = StaticProvider::from_host(&[rail(0.), rail(10.)]).unwrap();
        let mut scene = Scene {
            ground_pool: &[],
            island_pool: &[],
            conditional_pool: &[],
            island_flags: 0,
            grind: Some(&provider),
            static_edges: &[],
            primary_edges: PrimaryEdges::Normal { dynamic: &[], vehicles: &[] },
            indexed_edges: &[],
        };
        let search = EdgeSearch {
            min: Vector3::new(-1., 0., -1.),
            max: Vector3::new(3., 2., 1.),
            frame: Frame::IDENTITY,
            context: QueryContext { selection_flags_2948: 0, matching_id_2952: -1 },
            narrow_forward: false,
        };
        let edges = scene.edge_candidates(&search).unwrap();
        assert_eq!(edges.len(), 1);
        let edge = edges[0];
        assert_eq!(
            [edge.start.x, edge.start.y, edge.start.z, edge.end.x, edge.end.y, edge.end.z],
            [0., 1., 0., 2., 1., 0.]
        );
    }
}
