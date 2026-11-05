use super::*;
struct Scene {
    current: bool,
    rejected_x: Option<f32>,
    calls: Vec<&'static str>,
}
impl Validation for Scene {
    type Error = ();
    fn ground(&mut self, _: &Matrix) -> Result<Option<Ground>, ()> {
        self.calls.push("ground");
        Ok(self.current.then_some(Ground {
            position: [8., 2., 9., 0.],
            offboard: true,
        }))
    }
    fn location(&mut self, m: &Matrix) -> Result<bool, ()> {
        self.calls.push("location");
        Ok(self.rejected_x != Some(m[3][0]))
    }
    fn occupants(&mut self, _: &Matrix) -> Result<bool, ()> {
        self.calls.push("occupants");
        Ok(true)
    }
    fn edges(&mut self, _: &Matrix) -> Result<bool, ()> {
        self.calls.push("edges");
        Ok(true)
    }
}
fn candidate(x: f32, score: f32) -> Candidate {
    let mut transform = IDENTITY;
    transform[3][0] = x;
    Candidate {
        transform,
        stance: 1,
        offboard: false,
        score,
    }
}
fn scene() -> Scene {
    Scene {
        current: false,
        rejected_x: None,
        calls: vec![],
    }
}

#[test]
fn scored_selection_consumes_entries_and_does_not_recheck_terrain() {
    let mut h = History::new(candidate(-10., 0.));
    h.insert(candidate(10., 100.));
    h.insert(candidate(20., 100.));
    let mut s = scene();
    assert_eq!(h.automatic(0, &mut s).unwrap().transform[3][0], 10.);
    assert_eq!(s.calls, ["ground", "location", "occupants"]);
    //Latest is now penalized; initial0 ties its adjusted100-100, newer wins.
    assert_eq!(h.automatic(0, &mut s).unwrap().transform[3][0], 20.);
    assert_eq!(h.automatic(0, &mut s).unwrap().transform[3][0], -10.);
    assert_eq!(h.automatic(0, &mut s).unwrap().transform[3][0], -10.);
}

#[test]
fn rejection_removes_candidate_and_recomputes_age() {
    let mut h = History::new(candidate(-10., 0.));
    h.insert(candidate(10., 100.));
    h.insert(candidate(20., 50.));
    let mut s = scene();
    s.rejected_x = Some(10.);
    assert_eq!(h.automatic(1, &mut s).unwrap().transform[3][0], -10.);
    assert_eq!(h.entries.len(), 1);
    assert_eq!(h.entries[0].transform[3][0], 20.);
}

#[test]
fn current_candidate_keeps_heading_and_horizontal_position_and_history() {
    let mut h = History::new(candidate(-10., 0.));
    h.current_position = [3., 4., 5., 0.];
    h.current_orientation[2] = [1., 0., 0., 0.];
    let mut s = scene();
    s.current = true;
    let c = h.automatic(7, &mut s).unwrap();
    assert_eq!(c.transform[3], [3., 2., 5., 0.]);
    assert_eq!(c.transform[2], [1., 0., 0., 0.]);
    assert_eq!((c.stance, c.offboard), (7, true));
    assert_eq!(h.entries.len(), 1);
    assert_eq!(s.calls, ["ground", "location", "occupants", "edges"]);
}

#[test]
fn insertion_capacity_and_native_gate_boundaries() {
    let mut h = History::new(candidate(0., 0.));
    assert!(!h.recording_due(119, [10., 0., 0., 0.]));
    assert!(h.recording_due(120, [1.5, 0., 0., 0.]));
    assert!(!h.recording_due(120, [1.499, 0., 0., 0.]));
    h.insert(candidate(10., 0.));
    for _ in 0..20 {
        assert!(!h.recording_due(120, [1000., 0., 0., 0.]));
    }
    assert!(h.recording_due(120, [1000., 0., 0., 0.]));
    for n in 0..40 {
        h.insert(candidate(n as f32, 0.));
    }
    assert_eq!(h.entries.len(), 32);
    assert_eq!(h.entries[0].transform[3][0], 8.);
    assert_eq!(h.initial.transform[3][0], 0.);
}

#[test]
fn category_dispatch_matches_decrementing_ppc_branch_table() {
    let accepted: Vec<_> = (0..=15).filter(|&x| surface_allowed(x)).collect();
    assert_eq!(accepted, [0, 1, 2, 3, 4, 7, 8, 10, 11, 14, 15]);
    assert_eq!(
        (0..=12).map(surface_score).collect::<Vec<_>>(),
        [0., 100., 100., 50., 50., 0., 0., 0., 0., 0., 0., 20., 0.]
    );
}

#[test]
fn exhausted_history_fallback_uses_current_actor_stance() {
    let mut h = History::new(candidate(-10., 100.));
    let mut s = scene();
    s.rejected_x = Some(-10.);
    let c = h.automatic(0, &mut s).unwrap();
    assert_eq!(c.transform, h.initial.transform);
    assert_eq!((c.stance, c.offboard, c.score), (0, false, 0.));
    assert!(h.entries.is_empty());
}

#[test]
fn suggestion_keeps_the_history_and_takes_the_newest_entry_the_living_world_accepts() {
    let mut h = History::new(candidate(-10., 0.));
    h.insert(candidate(10., 100.));
    h.insert(candidate(20., 0.));
    let mut s = scene();
    assert_eq!(h.suggested(0, &mut s).unwrap().transform[3][0], 20.);
    assert_eq!(s.calls, ["ground", "occupants"]);
    assert_eq!(h.entries.len(), 3);
}

#[test]
fn a_new_actor_restarts_at_its_spawn_and_takes_over_the_actor_ring() {
    let mut h = History::new(candidate(-10., 0.));
    h.insert(candidate(10., 100.));
    h.current_position = [3., 4., 5., 0.];
    h.respawn(candidate(50., 0.));
    assert_eq!(h.entries().len(), 1);
    assert_eq!(h.initial().transform[3][0], 50.);
    assert_eq!(h.current_position, candidate(50., 0.).transform[3]);
    assert_eq!(h.current_orientation, IDENTITY);
    let ring: Vec<_> = (0..40).map(|n| candidate(n as f32, 0.)).collect();
    h.import(&ring);
    assert_eq!(h.entries().len(), 32);
    assert_eq!(h.entries()[0].transform[3][0], 8.);
    //An empty ring's last resort is the spawn, on board, with the current stance.
    h.import(&[]);
    let c = h.automatic(1, &mut scene()).unwrap();
    assert_eq!((c.transform[3][0], c.stance, c.offboard), (50., 1, false));
}
