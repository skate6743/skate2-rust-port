//! Static bounds hierarchy over authored query meshes. Results are sorted back
//! into source order before narrow-phase, preserving equal-hit/contact order.
use super::query_metadata::{Bounds, QueryMesh};
use std::ops::Range;

#[derive(Default)]
pub(super) struct QueryIndex {
    nodes: Vec<Node>,
    order: Vec<usize>,
}
struct Node {
    bounds: Bounds,
    children: Option<(usize, usize)>,
    range: Range<usize>,
}
impl QueryIndex {
    pub(super) fn new(meshes: &[QueryMesh]) -> Self {
        let mut index = Self {
            nodes: Vec::new(),
            order: (0..meshes.len()).collect(),
        };
        if !meshes.is_empty() {
            index.build(meshes, 0..meshes.len());
        }
        index
    }
    fn build(&mut self, meshes: &[QueryMesh], range: Range<usize>) -> usize {
        let bounds = Bounds::from_points(
            self.order[range.clone()]
                .iter()
                .flat_map(|&i| [meshes[i].local_bounds.min, meshes[i].local_bounds.max]),
        )
        .unwrap();
        let id = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            children: None,
            range: range.clone(),
        });
        if range.len() > 8 {
            let extents = [
                bounds.max.x - bounds.min.x,
                bounds.max.y - bounds.min.y,
                bounds.max.z - bounds.min.z,
            ];
            let axis = (0..3)
                .max_by(|&a, &b| extents[a].total_cmp(&extents[b]))
                .unwrap();
            let center = |i: usize| {
                let b = meshes[i].local_bounds;
                let low = [b.min.x, b.min.y, b.min.z][axis];
                let high = [b.max.x, b.max.y, b.max.z][axis];
                low * 0.5 + high * 0.5
            };
            let mid = range.start + range.len() / 2;
            self.order[range.clone()].select_nth_unstable_by(range.len() / 2, |&a, &b| {
                center(a).total_cmp(&center(b)).then(a.cmp(&b))
            });
            let left = self.build(meshes, range.start..mid);
            let right = self.build(meshes, mid..range.end);
            self.nodes[id].children = Some((left, right));
        }
        id
    }
    pub(super) fn query(&self, bounds: Bounds, meshes: &[QueryMesh]) -> Vec<usize> {
        let mut result = Vec::new();
        if self.nodes.is_empty() {
            return result;
        }
        let mut stack = vec![0];
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id];
            if !node.bounds.overlaps(bounds) {
                continue;
            }
            if let Some((left, right)) = node.children {
                stack.push(right);
                stack.push(left);
            } else {
                result.extend(
                    self.order[node.range.clone()]
                        .iter()
                        .copied()
                        .filter(|&i| meshes[i].local_bounds.overlaps(bounds)),
                );
            }
        }
        result.sort_unstable();
        result
    }
}
