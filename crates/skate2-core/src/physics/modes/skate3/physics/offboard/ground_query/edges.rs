use super::{Bounds, EdgeBody, PrimaryEdges, Scene, Segment, transform};
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
pub(super) fn candidates(scene: &Scene<'_>, search: &EdgeSearch) -> Vec<Edge> {
    let query = Bounds {
        min: search.min,
        max: search.max,
    };
    let mut output = Vec::with_capacity(40);
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
    output
}
