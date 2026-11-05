use super::*;

#[test]
fn reports_use_solved_forces_and_post_integration_pose_and_velocity() {
    for is_a in [true, false] {
        let mut body_state = bodies();
        let point = body_state[6].rates.position;
        let board = CollisionBody::Board(BodyId::Deck);
        let world = CollisionBody::StaticWorld;
        let collision = BoardCollision {
            body_a: if is_a { board } else { world },
            body_b: if is_a { world } else { board },
            contact: RetailContactInput {
                position_on_a: point,
                position_on_b: point,
                normal: Vector3::new(0.0, if is_a { 1.0 } else { -1.0 }, 0.0),
                restitution: 0.0,
                static_friction: 0.0,
                dynamic_friction: 0.0,
                tag: if is_a { 5 << 16 | 7 } else { 7 << 16 | 5 },
            },
        };
        let mut queue = BoardForceQueue::default();
        queue.append(QueuedPointForce {
            tag: 2,
            force_world: Vector3::new(0.0, -60.0, 0.0),
            point_body: Vector3::ZERO,
        });
        let mut tick = BoardStep::default();
        let config = settings(25);
        tick.advance(
            &mut body_state,
            &mut inactive_hook(),
            &queue,
            &[collision],
            [0.0; 2],
            config,
        );
        let report = tick.contact_reports()[0];
        assert_eq!(report.part, BodyId::Deck);
        assert_eq!(report.other_surface, 7);
        assert_eq!(report.is_body_a, is_a);
        assert_eq!(report.normal.y, 1.0);
        let expected = body_state[6].rates.position;
        assert!((report.position.x - expected.x).abs() < 1e-10);
        assert!((report.position.y - expected.y).abs() < 1e-10);
        assert!((report.position.z - expected.z).abs() < 1e-10);
        assert_eq!(
            report.relative_linear_velocity,
            body_state[6].rates.linear_velocity
        );
        assert!(report.normal_force_on_a.y * if is_a { 1.0 } else { -1.0 } > 0.0);

        // A later empty/inactive tick must not expose the previous contact.
        for body in &mut body_state {
            body.state_flags = 0;
        }
        tick.advance(
            &mut body_state,
            &mut inactive_hook(),
            &BoardForceQueue::default(),
            &[],
            [0.0; 2],
            config,
        );
        assert!(tick.contact_reports().is_empty());
    }
}

#[test]
fn report_budget_is_shared_by_parts_and_keeps_native_contact_order() {
    let mut rows = Vec::new();
    for i in 0..20 {
        let mut words = [0; 64];
        words[11] = 8;
        words[20] = 1.0_f32.to_bits();
        words[29] = 1.0_f32.to_bits();
        words[31] = i % 7;
        words[35] = 1.0_f32.to_bits();
        words[43] = u32::MAX;
        words[55] = i + 1;
        rows.push(RetailContactJacobian {
            words,
            reaction_index_a: (i % 7) as usize,
            reaction_index_b: WORLD_REACTION,
        });
    }
    let mut output = Vec::new();
    board_reports::collect(&mut output, &rows, &bodies(), &[], 0, 60.0);
    assert_eq!(
        output.iter().map(|r| r.other_surface).collect::<Vec<_>>(),
        (1..=16).collect::<Vec<_>>()
    );
    rows[0].words[20] = 0;
    rows[1].words[11] = 0;
    rows[2].words[43] = 6; // Different part, same board: excluded from reports.
    board_reports::collect(&mut output, &rows, &bodies(), &[], 0, 60.0);
    assert_eq!(
        output.iter().map(|r| r.other_surface).collect::<Vec<_>>(),
        (4..=19).collect::<Vec<_>>()
    );
}
