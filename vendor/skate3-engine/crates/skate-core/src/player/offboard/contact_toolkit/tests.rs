use super::*;
use crate::air::trajectory::QueryResult;
fn input() -> Input {
    Input::from_vectors([
        ZERO,
        [0., 0., 1., 0.],
        UP,
        [1., 0., 0., 0.],
        ZERO,
        UP,
        [1., 0., 0., 0.],
    ])
}
struct EmptyScene;
impl Scene for EmptyScene {
    type Error = ();
    fn execute(&self, batch: &Batch) -> Result<QueryResults, ()> {
        Ok(QueryResults {
            trajectories: [QueryResult::miss(); 3],
            lines: vec![None; batch.lines.len()],
            edges: Vec::new(),
        })
    }
}
#[test]
fn empty_world_refresh_cannot_publish_walkable_support() {
    let mut owner = Owner::default();
    assert!(owner.refresh().is_none());
    owner.submit(input(), -1, &EmptyScene).unwrap();
    assert!(owner.has_pending());
    let result = owner.refresh().unwrap();
    assert_eq!(result.prefix.flags_176 & 3, 0);
    assert_eq!(result.prefix.support_180, 0);
    assert_eq!(result.prefix.distance_172, 1e10);
    assert!(!owner.has_pending());
    assert!(owner.refresh().is_none());
}
#[test]
fn classification_requires_three_prior_matches_and_flat_clears_history() {
    let mut history = classification::History::default();
    let mut prefix = ContactPrefix::reset();
    prefix.flags_176 = 0x40;
    for _ in 0..3 {
        history.classify(&mut prefix, [0., 0.3, 1., 0.]);
        assert_eq!(prefix.kind_164, 0);
    }
    history.classify(&mut prefix, [0., 0.3, 1., 0.]);
    assert_eq!(prefix.kind_164, 1);
    history.classify(&mut prefix, [0., 0., 1., 0.]);
    assert_eq!(prefix.kind_164, 0);
    history.classify(&mut prefix, [0., 0.3, 1., 0.]);
    assert_eq!(prefix.kind_164, 0);
}
#[test]
fn source_tangent_polynomial_has_reasonable_stock_angle_outputs() {
    for degrees in [6., 15., 30., 32., 35., 50.] {
        let angle: f32 = degrees * 0.017453292;
        assert!((analyzer_math::tangent(angle) - angle.tan()).abs() < 0.00001);
    }
}
#[test]
fn secondary_edge_is_selected_only_when_higher_than_real_line_contact() {
    let layout = ProbeLayout::stock();
    let batch = layout.prepare(input(), -1);
    let mut result = EmptyScene.execute(&batch).unwrap();
    result.edges.push([[0., 0.1, -1., 0.], [0., 0.1, 2., 0.]]);
    let descriptor = batch.secondary[0];
    let mut position = descriptor.line.start;
    position[1] = 0.2;
    result.lines[descriptor.forward_index] = Some(LineHit {
        position,
        normal: UP,
        fraction: 0.375,
        surface: 17,
        mesh_frame: IDENTITY,
        geometry: 73,
    });
    let samples = collection::collect(&batch, &layout, &result);
    assert_eq!(samples.ground[0].position, position);
    assert!(
        samples.ground[1..]
            .iter()
            .all(|point| (point.position[1] - 0.1).abs() < 1e-6)
    );
}
#[test]
fn stock_probe_counts_order_and_reverse_indices() {
    let layout = ProbeLayout::stock();
    assert_eq!(layout.secondary.len(), 23);
    assert_eq!(layout.primary.len(), 15);
    let batch = layout.prepare(input(), 27);
    assert_eq!(batch.lines.len(), 44);
    assert_eq!(batch.matching_group, 27);
    assert_eq!(batch.mesh_reject_mask, 0x6000);
    for d in &batch.primary {
        assert_eq!(batch.lines[d.forward_index], d.line);
        if let Some(i) = d.reverse_index {
            assert_eq!(batch.lines[i].start, d.line.end);
            assert_eq!(batch.lines[i].end, d.line.start);
        }
    }
    assert_eq!(batch.trajectories[0].trajectory.position, [0., 1.2, 0., 0.]);
    assert_eq!(batch.trajectories[0].trajectory.velocity, [0., -2., 0., 0.]);
    assert_eq!(batch.trajectories[0].radius, 0.01);
}
#[test]
fn side_probes_follow_animation_basis_but_centre_follows_surface() {
    let mut i = input();
    i.animation_right = [0., 0., -1., 0.];
    let batch = ProbeLayout::stock().prepare(i, -1);
    assert_eq!(batch.trajectories[0].trajectory.position, [0., 1.2, 0., 0.]);
    assert_eq!(
        batch.trajectories[1].trajectory.position,
        [0., 0.8, 0.12, 0.]
    );
    assert_eq!(
        batch.trajectories[2].trajectory.position,
        [0., 0.8, -0.12, 0.]
    );
}
#[test]
fn prefix_reset_does_not_invent_support_or_reach() {
    let p = ContactPrefix::reset();
    assert_eq!(p.flags_176, 0);
    assert_eq!(p.kind_164, 0);
    assert_eq!(p.distance_168, 1e10);
    assert_eq!(p.distance_172, 1e10);
    assert_eq!(p.edge_normal, ZERO);
}
#[test]
fn support_uses_refined_normal_and_mesh_frame_and_strict_side_window() {
    let mut hits = [QueryResult::miss(); 3];
    hits[0] = QueryResult {
        contact_time: 0.,
        contact_position: [0., -0.11, 0., 0.],
        geometry: 92,
        ..QueryResult::miss()
    };
    hits[1] = QueryResult {
        contact_time: 0.,
        contact_position: [0., 0.2, 0., 0.],
        ..QueryResult::miss()
    };
    let mut p = ContactPrefix::reset();
    p.consume_support(input(), &hits, 0x40);
    assert_eq!(p.flags_176, 0x49);
    assert_eq!(p.support_180, 92);
    hits[1].contact_position[1] = 0.199;
    p = ContactPrefix::reset();
    p.consume_support(input(), &hits, 0);
    assert_eq!(p.flags_176, 0x19);
}
#[test]
fn projected_contacts_are_bounded_and_reject_behind_obstacles() {
    let mut samples = Samples::default();
    assert!(!samples.insert(input(), [0., 0., -1., 0.], UP, 0, -1., 0));
    for n in 0..64 {
        assert!(samples.insert(input(), [0., 0., n as f32, 0.], UP, 1, -1., 0));
    }
    assert!(!samples.insert(input(), ZERO, UP, 1, -1., 0));
    assert_eq!(samples.original_ground_count, 64);
}
#[test]
fn sweep_contact_time_is_not_the_line_fraction() {
    let p = LineProbe {
        start: [0., 1., 0., 0.],
        end: [0., -1., 0., 0.],
        radius: 0.01,
    };
    let hit = LineHit {
        position: ZERO,
        normal: UP,
        fraction: 0.495,
        surface: 0,
        mesh_frame: IDENTITY,
        geometry: 7,
    };
    let r = query_sweep(p, |_| Ok::<_, ()>(Some(hit)), |_, _| Ok(vec![])).unwrap();
    assert!((r.contact_time - 0.5 / 60.).abs() < 1e-7);
    assert_eq!(r.contact_frame, 0);
    assert_eq!(r.geometry, 7);
}
