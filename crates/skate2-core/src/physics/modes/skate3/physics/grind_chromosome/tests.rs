use super::*;
fn input() -> Input {
    Input {
        category: 400,
        grinding_316: true,
        air_event_439: false,
        family: Some(Family::FiftyFifty),
        basic_right_0: [1.0, 0.0, 0.0, 0.0],
        basic_forward_32: [0.0, 0.0, 1.0, 0.0],
        basic_position_48: [0.0; 4],
        basic_location_axis_96: [0.0, 0.0, 1.0, 0.0],
        basic_twist_axis_128: [1.0, 0.0, 0.0, 0.0],
        basic_location_position_144: [0.0; 4],
        feet_256_272: [[1.0, 0.0, 0.0, 0.0]; 2],
        fakie_155: false,
        point: [0.0; 4],
        direction: [1.0, 0.0, 0.0, 0.0],
        normal: [0.0, 1.0, 0.0, 0.0],
        across: [0.0, 0.0, 1.0, 0.0],
    }
}
#[test]
fn family_switch_does_not_fake_new_category_or_skip_scoring_delay() {
    let mut state = Chromosome::new(false); //Explicit synthetic constructor observation.
    let mut i = input();
    let first = state.update(i).unwrap();
    assert_eq!(first.animation, Some(first.volatile));
    assert_eq!(first.scoring, Some(first.volatile));
    i.family = Some(Family::FiveO);
    let switched = state.update(i).unwrap();
    assert_eq!(switched.animation, first.animation);
    assert_eq!(switched.scoring, first.scoring);
    let repeated = state.update(i).unwrap();
    assert_eq!(repeated.animation, Some(repeated.volatile));
    for _ in 0..11 {
        assert_eq!(state.update(i).unwrap().scoring, first.scoring);
    }
    assert_eq!(state.update(i).unwrap().scoring, Some(repeated.volatile));
}
#[test]
fn air_snapshot_clears_history_and_short_ground_history_does_not_replace_saved() {
    let mut state = Chromosome::new(false);
    let mut i = input();
    i.category = 100;
    i.grinding_316 = false;
    for tick in 0..30 {
        i.basic_position_48[0] = tick as f32;
        state.update(i);
    }
    assert_eq!(state.reference().position[0], 0.0);
    i.basic_position_48[0] = 30.0;
    state.update(i);
    assert_eq!(state.reference().position[0], 1.0);
    i.category = 200;
    i.air_event_439 = true;
    i.basic_position_48[0] = 99.0;
    state.update(i);
    assert_eq!(state.reference().position[0], 99.0);
    i.category = 100;
    i.air_event_439 = false;
    i.basic_position_48[0] = 1000.0;
    state.update(i);
    assert_eq!(state.reference().position[0], 99.0);
}
#[test]
fn twist_uses_128_not_forward32_and_names_keep_both_id_domains() {
    assert!(pose::straight(input()));
    let name = names::lookup([0, 0, 0, 0, 0, 5]).unwrap();
    assert_eq!(name.attribute, "FS_DARKSLIDE");
    assert_eq!(name.skating_id, 17);
    assert_eq!(name.scorable_id, 270);
    assert!(names::lookup([0, 0, 0, 0, 4, 0]).is_none());
    assert!(names::lookup([0, 0, 0, 0, 0, u32::MAX]).is_none());
}

#[test]
fn frontlip_and_frontboard_publish_distinct_native_scorables() {
    let mut chromosome = Chromosome::new(false);
    let mut i = input();
    i.family = Some(Family::Boardslide);
    i.basic_twist_axis_128 = [0., 0., 1., 0.];
    let lip = chromosome.update(i).unwrap();
    let mut out = skate3_core::player::input_phase::GrindOutputFields::default();
    publish(lip, &mut out);
    assert_eq!(out.scorable_id_152, 28);
    assert_eq!(lip.scoring.unwrap().name().attribute, "FS_LIP");
    i.feet_256_272 = [[-1., 0., 0., 0.]; 2];
    for _ in 0..14 {
        publish(chromosome.update(i).unwrap(), &mut out);
    }
    assert_eq!(out.scorable_id_152, 25);
}
