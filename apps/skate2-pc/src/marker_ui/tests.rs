use super::*;
use skate2_formats::mount::{MountedBig, SKATE2_ARCHIVES};

#[test]
fn minimap_native_heading_ring_and_fade() {
    use skate2_core::ui::minimap::{Icon,heading,placement};
    for (direction,expected) in [([0.,1.],0.),([1.,0.],std::f32::consts::FRAC_PI_2),
        ([0.,-1.],-std::f32::consts::PI),([-1.,0.],-std::f32::consts::FRAC_PI_2)] {
        let actual=heading(direction);
        let difference=(actual-expected+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
        assert!(difference.abs()<0.0001,"{direction:?}: {actual} vs {expected}");
    }
    assert!(heading([0.,0.]).is_finite());
    let mut icon=Icon {texture:String::new(),position:[0.,60.],rotation:0.,scale:1.,visible:true,
        fade_at_edge:true,maximum_range:0.,pinch_point:false};
    let (position,alpha)=placement(&icon,[0.,0.],0.).unwrap();
    assert!((position[1]-60.).abs()<0.0001);assert_eq!(alpha,1.);
    icon.position=[200.,0.];
    let (position,alpha)=placement(&icon,[0.,0.],0.).unwrap();
    assert!((position[0]-71.281097).abs()<0.0001);
    assert!((alpha-(1.-140./290.)).abs()<0.0001);
    icon.maximum_range=100.;assert!(placement(&icon,[0.,0.],0.).is_none());
    icon.fade_at_edge=false;assert_eq!(placement(&icon,[0.,0.],0.).unwrap().1,1.);
    icon.visible=false;assert!(placement(&icon,[0.,0.],0.).is_none());
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_minimap_and_district_timelines() {
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let mut minimap=minimap_movie::MiniMapMovie::new(&assets).unwrap();
    assert!(!minimap.root.visible);
    minimap.show(&assets,true).unwrap();
    for _ in 0..24 {minimap.tick(&assets).unwrap();}
    assert!(minimap.root.visible);
    let mut batches=Vec::new();
    flatten_projected(&assets,&minimap.root,IDENTITY,Colour::default(),(1280,720),&mut batches,true,None).unwrap();
    assert!(!batches.is_empty());
    minimap.no_skate_zone(&assets,true).unwrap();
    for _ in 0..150 {minimap.tick(&assets).unwrap();}
    minimap.no_skate_zone(&assets,false).unwrap();
    for _ in 0..150 {minimap.tick(&assets).unwrap();}
    minimap.show(&assets,false).unwrap();
    for _ in 0..24 {minimap.tick(&assets).unwrap();}
    assert!(!minimap.root.visible);
    let mut central=central_movie::CentralMovie::new(&assets).unwrap();
    central.district(&assets,"#Waterfront").unwrap();
    for _ in 0..600 {central.tick(&assets).unwrap();}
    assert!(!central.root.named(&["mcDistrict"]).unwrap().visible);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_cas_projection_retains_depth_across_postfx_changes() {
    use skate2_core::ui::edit_skater_presentation::{EditSkaterCamera, EditSkaterPresentationAttributes};
    let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts = MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
    }
    let attributes = EditSkaterPresentationAttributes::load(&mounts).unwrap();
    let mut camera = EditSkaterCamera {
        position: [0.; 3], look_position: [0., 0., 1.], up: [0., 1., 0.],
        fov: 40., postfx_csm_collection: 11555447452688460307,
    };
    let projection = attributes.project_camera(&camera, 0.05).unwrap().clip_from_world;
    let depth = |distance:f32| (projection[10] * distance + projection[14]) /
        (projection[11] * distance + projection[15]);
    assert!(depth(0.1) > 0. && depth(0.1) < depth(1.));
    assert!(depth(1.) < depth(10.) && depth(10.) < 1.);
    camera.postfx_csm_collection = 0;
    assert_eq!(projection, attributes.project_camera(&camera, 0.05).unwrap().clip_from_world);
    assert!(attributes.project_camera(&camera, 0.).is_err());
    // Both shops and each part/face camera must resolve the FE shadow
    // collection independently of rendering/default's clipping planes.
    use skate2_core::ui::edit_skater::EditSkaterOption as O;
    let mut cameras = vec![0xad454a0ecd0e2edf, 0xb5c3cac4b3dc0165];
    for option in [O::Hats, O::TShirts, O::Hair, O::Legs, O::Shoes,
        O::Glasses, O::Wrist, O::Boards, O::Trucks, O::Wheels,
        O::PiercingsLE, O::PiercingsRE, O::PiercingsLB, O::PiercingsRB,
        O::PiercingsLN, O::PiercingsMN, O::PiercingsRN, O::PiercingsLip,
        O::Necklaces, O::Rings, O::Skin, O::Eyes, O::Brow, O::Nose,
        O::Jaw, O::Mouth, O::Chin] {
        cameras.push(option.camera_key().unwrap());
    }
    for key in cameras {
        let camera = attributes.camera(key).unwrap();
        let csm = crate::render_environment::EnvironmentLighting::load_frontend_csm(
            &mounts, camera.postfx_csm_collection)
            .unwrap_or_else(|e| panic!("CAS camera {key:016x}: {e}"));
        assert!(csm.scale > 0. && csm.depth_range.is_finite());
        let shadows = crate::render_shadows::frontend_cascades(
            [0.3, 1., 0.2, 0.], &csm,
            &crate::render_shadows::ShadowCasterInputs {
                camera_position: camera.position,
                camera_forward: [0., 0., 1.],
                skater_hips: [0., 1., 0.],
                skater_bounds: [[-0.5, 0., -0.5], [0.5, 2., 0.5]],
            });
        assert!(shadows.rows.iter().flatten().flatten().all(|x| x.is_finite()));
        if csm.depth_range == 0. {
            assert_eq!(shadows.rows[0][2], [0., 0., 1., 0.]);
        }
        assert!(attributes.project_camera(&camera, 0.05).unwrap()
            .clip_from_world.iter().all(|x| x.is_finite()));
    }
}

#[test]
fn native_multiline_wrap_preserves_words_and_authored_breaks() {
    fn lines(s: &str, multiline: bool, width: f32) -> Vec<String> {
        let units: Vec<_> = s.encode_utf16().collect();
        native_text_lines(&units, multiline, width, |_| 1.).iter()
            .map(|s|String::from_utf16(s).unwrap()).collect()
    }
    assert_eq!(lines("Back Alley Pete's Body Modification", true, 17.),
        ["Back Alley Pete's", "Body Modification"]);
    assert_eq!(lines("first^second", true, 99.), ["first", "second"]);
    assert_eq!(lines("unbrokenword next", true, 4.), ["unbrokenword", "next"]);
    assert_eq!(lines("one two three", false, 4.), ["one two three"]);
    assert_eq!(lines("one two", true, 3.), ["one", "two"]);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_map_selection_animates_and_only_background_expands() {
    let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts = MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game), spec.use_external_header).unwrap());
    }
    let assets = Assets::load(&mounts).unwrap();
    let mut map = map_movie::MapMovie::new(&assets).unwrap();
    let rows = [("Locations", true), ("Waterfront", true)];
    map.update(&assets, true, &rows, 0, "Locations", None).unwrap();
    for _ in 0..60 { map.tick(&assets).unwrap(); }
    assert!(map.intro_complete && map.root.visible);
    let path = ["mcStuffPanel", "mMenuPicker", "mMenuPicker", "mIndicator"];
    fn geometry(assets: &Assets, clip: &Clip, size: (u32,u32)) -> Vec<[f32;10]> {
        let mut batches = Vec::new();
        flatten_projected(assets, clip, IDENTITY, Colour::default(), size,
            &mut batches, true, None).unwrap();
        batches.into_iter().flat_map(|batch| batch.vertices).collect()
    }
    let first = geometry(&assets, map.root.named(&path).unwrap(), (1280,720));
    assert!(!first.is_empty(), "selected row must draw the original arrow");
    let mut changed = false;
    for _ in 0..30 {
        map.tick(&assets).unwrap();
        changed |= geometry(&assets, map.root.named(&path).unwrap(), (1280,720)) != first;
    }
    assert!(changed, "nested arrow artwork must keep animating under its stopped parent");
    map.update(&assets, true, &rows, 1, "Locations", None).unwrap();
    let picker = map.root.named(&path[..3]).unwrap();
    let indicator = picker.children.values().find(|child|
        child.entry.name.as_deref() == Some("mIndicator")).unwrap();
    assert!(indicator.clip.visible && indicator.entry.matrix.ty > 0.);
    let normal = geometry(&assets, &indicator.clip, (1280,720));
    let wide = geometry(&assets, &indicator.clip, (2560,720));
    assert_eq!(normal.len(), wide.len());
    for (a,b) in normal.iter().zip(&wide) {
        assert!((a[0]*0.5-b[0]).abs()<0.00001);
        assert_eq!(&a[1..], &b[1..]);
    }
    for (asset, character) in [(9,19),(11,3),(12,2)] {
        let shape = Clip::new(&assets, asset, character).unwrap();
        let normal = geometry(&assets, &shape, (1280,720));
        assert!(!normal.is_empty());
        assert_eq!(normal, geometry(&assets, &shape, (2560,720)),
            "background clip extent and UVs must fill the wider window");
    }
}

#[test]
fn scoring_frontend_preserves_nollie_variant_and_cab_name_rules() {
    use skate2_core::ui::{trick_backend::{DisplayEvent,TrickName,TricksBackend},
        trick_display::{DisplayTuning,MovieCommand,TrickDisplay}};
    let tuning=DisplayTuning { allowed:true,nollie_delay:140,jolt_threshold:0.73,jolt_distance:50.,jolt_speed:9.98,jolt_recovery:0.3258 };
    let mut b=TricksBackend::default(); let mut d=TrickDisplay::default(); let mut commands=Vec::new();
    let mut locale=|label:&str| -> RenderResult<String> {Ok(label.into())};
    b.current=Some(TrickName {index:12,attrib_key:0,valid:true,label:Some("nollie".into()),accepts_spin:true,
        nollie:true,fakie_variant:Some(("ollie".into(),false))});
    b.switched=true;
    d.event(&mut b,DisplayEvent::Start,tuning,&mut commands);
    d.update(&mut b,tuning,&mut locale,&mut commands).unwrap();
    assert!(commands.contains(&MovieCommand::RefreshTrick {text:"#ollie ".into(),new_trick:true}));
    assert!(!d.switched && d.fakie && !d.can_update_stance);
    assert!(d.is_nollie());
    // Reset/readiness is separate from landing completion and clears the
    // backend before the next original trick notification.
    d.event(&mut b,DisplayEvent::ReadyToScore,tuning,&mut commands);
    b.current=Some(TrickName {index:13,valid:true,label:Some("ollie".into()),accepts_spin:true,..Default::default()});
    b.fakie=true;commands.clear();
    d.event(&mut b,DisplayEvent::Start,tuning,&mut commands);
    d.update(&mut b,tuning,&mut locale,&mut commands).unwrap();
    b.spins=1;commands.clear();
    d.update(&mut b,tuning,&mut locale,&mut commands).unwrap();
    assert!(commands.contains(&MovieCommand::RefreshTrick {
        text:"#ollie ID_TRICK_AUTHENTIC_FS_HALFCAB ".into(),new_trick:false}));
    assert!(!d.fakie);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_score_movie_trick_multiplier_timer_and_bail() {
    use skate2_core::ui::{trick_backend::{PhysicsFrame,TrickName}, trick_display::DisplayTuning};
    let game = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts = MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets = Assets::load(&mounts).unwrap();
    // Actual authored 1.5x symbol must pass through the same localization
    // boundary as dynamic strings, before glyph generation.
    let label = Clip::new(&assets, 2, 74).unwrap();
    assert_eq!(String::from_utf16(label.text.as_ref().unwrap()).unwrap(), "x1.5");
    let mut label_draws = Vec::new();
    flatten(&assets, &label, IDENTITY, Colour::default(), (1280,720), &mut label_draws).unwrap();
    assert_eq!(label_draws.iter().map(|b| b.vertices.len()).sum::<usize>(), 4 * 6);
    // Raw shipped text RGBA, verified independently of the renderer. These
    // checks exercise ARGB decoding and the actual glyph vertex submission.
    for (character, argb) in [(4, 0xffff_feec_u32), (22, 0xff00_0000),
        (24, 0xff00_0000), (27, 0xfff3_c81a), (28, 0xfff3_c81a),
        (29, 0xfff3_c81a), (74, 0xff08_4d15)] {
        let mut text = Clip::new(&assets, 2, character).unwrap();
        text.text = Some("123".encode_utf16().collect());
        let mut glyphs = Vec::new();
        flatten(&assets, &text, IDENTITY, Colour::default(), (1280,720), &mut glyphs).unwrap();
        assert!(!glyphs.is_empty());
        let expected = [(argb>>16) as u8, (argb>>8) as u8, argb as u8, (argb>>24) as u8];
        for vertex in glyphs.iter().flat_map(|b| &b.vertices) {
            for lane in 0..4 {
                assert!((vertex[6+lane] - expected[lane] as f32 / 255.).abs() < 0.000001,
                    "text symbol {character} colour lane {lane}");
            }
        }
    }
    assert_eq!(assets.initial_text("#literal"), "literal".encode_utf16().collect::<Vec<_>>());
    assert_eq!(assets.initial_text("TRICK NAME"), vec![32]);
    let mut score = score_movie::ScoreMovie::new(&assets).unwrap();
    let tuning = DisplayTuning { allowed:true, nollie_delay:140, jolt_threshold:0.73, jolt_distance:50.,jolt_speed:9.98,jolt_recovery:0.3258 };
    let mut draws = Vec::new();
    for (round, (multiplier, background, ring_rgb)) in [(1.,68,0x02bf38), (1.5,68,0x02bf38),
        (2.,76,0xf7ee0d), (3.,79,0x000000), (1.5,68,0x02bf38), (1.,68,0x02bf38)].into_iter().enumerate() {
        score.update(&assets,&[PhysicsFrame {
            begun: Some(TrickName {index:12,valid:true,label:Some("ID_TRICK_AIR".into()),accepts_spin:true,..Default::default()}),
            sequence_begun:true, hud_score:123, multiplier,line_seconds:8.,line_points:400,line_max_points:400,
            // Native HUD seeks the timer on a line-score change or remembered
            // full-meter notification, not simply because seconds changed.
            line_score:1234 + round as u32,..Default::default()
        }],tuning,Some(true)).unwrap();
        let timer = score.root.named(&["multiplier_mc", "multiTimer_mc"]).unwrap();
        assert_eq!(timer.playback.timeline().frame(), Some(49),
            "8 integer seconds -> AS frame50 -> native frame49");
        let ring_before = timer.children[&15].entry.matrix;
        score.tick(&assets).unwrap();
        let timer = score.root.named(&["multiplier_mc", "multiTimer_mc"]).unwrap();
        assert_eq!(timer.playback.timeline().frame(), Some(50));
        assert_ne!(timer.children[&15].entry.matrix, ring_before,
            "authored ring must advance even when its parent is stopped");
        for _ in 0..60 {score.tick(&assets).unwrap();}
        let bg = score.root.named(&["multiplier_mc", "multiBG_mc"]).unwrap();
        assert_eq!(bg.children[&1].clip.character, background);
        assert!(!bg.playback.playing(), "multiplier intro must reach its authored stop");
        assert_eq!(bg.children.contains_key(&2), multiplier == 1.5);
        let ring = score.root.named(&["multiplier_mc", "multiTimer_mc", "mcSpecialTL"]).unwrap();
        assert_eq!(ring.children[&1].entry.colour_bias, ring_rgb);
        assert_eq!(ring.children[&1].entry.colour_scale, 0xff00_0000);
        let painted = &ring.children[&1];
        let mut ring_draws = Vec::new();
        flatten(&assets, &painted.clip, painted.entry.matrix,
            Colour::default().append(painted.entry.colour_scale, painted.entry.colour_bias),
            (1280,720), &mut ring_draws).unwrap();
        assert!(!ring_draws.is_empty());
        for vertex in ring_draws.iter().flat_map(|b| &b.vertices) {
            for (lane, byte) in [(0, (ring_rgb>>16) as u8), (1, (ring_rgb>>8) as u8), (2, ring_rgb as u8)] {
                assert!((vertex[6+lane] - byte as f32 / 255.).abs() < 0.000001);
            }
        }
        draws.clear();
        flatten(&assets,&score.root,IDENTITY,Colour::default(),(1280,720),&mut draws).unwrap();
        assert!(!draws.is_empty(),"authored score/multiplier must draw");
        assert!(draws.iter().any(|d| d.texture.as_ref().is_some_and(|n| n.contains("trickdisplay2"))));
        score.update(&assets,&[PhysicsFrame { sequence_completed:true, finished_sequence_score:123,
            finished_tricks:1,line_seconds:6.,line_score:1234,multiplier,line_max_points:400,
            ..Default::default() }],tuning,Some(true)).unwrap();
    }
    use skate2_core::ui::trick_context::Entry;
    score.update(&assets,&[PhysicsFrame { simulation_rate:1./60., multiplier:3., hud_score:123,
        line_score:1234,line_seconds:6.,line_max_points:400,
        context_entries:vec![Entry::Big {index:0,achievement:1,measurement:12.5},
            Entry::Entity {entity_type:9,is_entity:true,count:1,context_type:0}],
        ..Default::default()
    }],tuning,Some(true)).unwrap();
    for _ in 0..30 {score.tick(&assets).unwrap();}
    let title=score.root.named(&["bigTrick_mc0","dmo1_mc","title"]).unwrap();
    let text=String::from_utf16(title.text.as_ref().unwrap()).unwrap();
    // TrickDisplay_GetFormatDistance 0x826b1f28: FormatNumber(12.5, precision 1),
    // then FEBackEnd::ProcessDistance 0x8255a2b8 multiplies by 3.0 for feet.
    assert!(text.contains("37.5 ft"),"original formats metres at precision 1 before its 3.0 Imperial conversion: {text}");
    for _ in 0..121 {score.update(&assets,&[PhysicsFrame { simulation_rate:1./60.,
        multiplier:3.,hud_score:123,line_score:1234,line_seconds:6.,line_max_points:400,
        ..Default::default() }],tuning,Some(true)).unwrap();score.tick(&assets).unwrap();}
    score.update(&assets,&[PhysicsFrame { just_wiped_out:true, interrupted:true,..Default::default() }],tuning,Some(true)).unwrap();
    for _ in 0..150 {score.tick(&assets).unwrap();}
    draws.clear();
    flatten(&assets,&score.root,IDENTITY,Colour::default(),(1280,720),&mut draws).unwrap();
    assert!(draws.is_empty(),"bail timeline must hide the score screen");
}

/// Exercises real decoded assets through the production instance traversal,
/// including imports, event assignments, text, geometry and hidden alpha.
#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_marker_movie_opens_pulses_and_closes() {
    let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts = MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&root), spec.use_external_header).unwrap());
    }
    let assets = Assets::load(&mounts).unwrap();
    let mut clip = Clip::new(&assets, 0, 0).unwrap();
    let mut draws = Vec::new();
    flatten(
        &assets,
        &clip,
        IDENTITY,
        Colour::default(),
        (1280, 720),
        &mut draws,
    )
    .unwrap();
    assert!(
        draws.is_empty(),
        "authored hidden state must not show a generic LB prompt"
    );
    clip.named(&["mcQuickMenu"])
        .unwrap()
        .goto(&assets, "maximized")
        .unwrap();
    clip.named(&["mcQuickMenu", "mQuickOpen"])
        .unwrap()
        .goto(&assets, "main2")
        .unwrap();
    for _ in 0..60 {
        clip.tick(&assets).unwrap();
    }
    flatten(
        &assets,
        &clip,
        IDENTITY,
        Colour::default(),
        (1280, 720),
        &mut draws,
    )
    .unwrap();
    assert!(draws.iter().any(|d| {
        d.texture
            .as_ref()
            .is_some_and(|t| t.to_ascii_lowercase().contains("dpad_up"))
    }));
    assert!(draws.iter().any(|d| {
        d.texture
            .as_ref()
            .is_some_and(|t| t.to_ascii_lowercase().contains("dpad_down"))
    }));
    assert!(!draws.iter().any(|d| {
        d.texture
            .as_ref()
            .is_some_and(|t| t.to_ascii_lowercase().ends_with("button_y.texture"))
    }));
    let pulse = clip
        .named(&["mcQuickMenu", "mQuickOpen", "mTextHelp1"])
        .unwrap();
    pulse.goto(&assets, "pulse").unwrap();
    assert!(pulse.playback.playing());
    for _ in 0..24 {
        clip.tick(&assets).unwrap();
    }
    assert!(
        !clip
            .named(&["mcQuickMenu", "mQuickOpen", "mTextHelp1"])
            .unwrap()
            .playback
            .playing()
    );
    clip.named(&["mcQuickMenu"])
        .unwrap()
        .goto(&assets, "hidden")
        .unwrap();
    for _ in 0..24 {
        clip.tick(&assets).unwrap();
    }
    draws.clear();
    flatten(
        &assets,
        &clip,
        IDENTITY,
        Colour::default(),
        (1280, 720),
        &mut draws,
    )
    .unwrap();
    assert!(draws.is_empty());
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_objective_movie_lists_scales_and_completes() {
    use skate2_core::challenge::objective_hud::{ObjectiveHudView, objective_state};
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let text=|units: Vec<u16>| String::from_utf16(&units).unwrap();
    assert_eq!(text(localizer::format_custom(&assets,"ID_CHALLENGE_M_1A_04_TUTORIAL_MANUALS_OBJ1","2").unwrap()),"Manual for 2.0 s");
    assert_eq!(text(localizer::format_custom(&assets,"ID_CHALLENGE_F_1A_12_HOM","46000").unwrap()),"46,000 pts Anywhere");
    assert_eq!(text(localizer::format_custom(&assets,"ID_CHALLENGE_F_3A_13_HOM","17.7778,2.5").unwrap()),"40.0 MPH and 2.5 s of Air Time");
    assert_eq!(text(localizer::format_custom(&assets,"#Jump through the O","").unwrap())," ");
    assert_eq!(localizer::format_float(&assets,1234.56,1),"1,234.6");
    assert_eq!(localizer::format_float(&assets,0.96,1),"1.0");
    let mut movie=objective_movie::ObjectiveMovie::new(&assets).unwrap();
    assert!(!movie.root.visible);
    let mut view=ObjectiveHudView::default();
    view.title="ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_TITLE".into();
    view.num_objectives=1;
    view.rows[0].state=objective_state::ACTIVE;
    view.rows[0].value="#0/1".into();
    // ObjectiveHUD::Refresh: the row's state change runs before its title copy.
    movie.set_state(&assets,4,&view).unwrap();
    view.rows[0].title=("ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1".into(),String::new());
    movie.set_state(&assets,3,&view).unwrap();
    movie.set_state(&assets,1,&view).unwrap();
    for _ in 0..30 {movie.tick(&assets).unwrap();}
    assert!(movie.root.visible);
    for i in 0..3 {assert!(!movie.root.named(&[&format!("mcResult{i}")]).unwrap().visible);}
    let row=movie.root.named(&["mcResult3"]).unwrap();
    assert!(row.visible);
    assert_eq!(row.named(&["label"]).unwrap().text.as_deref().map(String::from_utf16_lossy).as_deref(),Some("Do an Ollie"));
    assert_eq!(row.named(&["mcFraction","labelText"]).unwrap().text.as_deref().map(String::from_utf16_lossy).as_deref(),Some("0/1"));
    let title=movie.root.named(&["mBackground","mcTextTitle","title100","labelText"]).unwrap();
    assert_eq!(title.text.as_deref().map(String::from_utf16_lossy).as_deref(),Some("Flipping Wood 101"));
    // ScaleTitleSection's _x and the later _y keep mcTextTitle off the 1no move.
    let background=movie.root.named(&["mBackground"]).unwrap();
    let placed=pause_movie::child(background,"mcTextTitle").unwrap();
    assert_eq!((placed.entry.matrix.tx,placed.entry.matrix.ty),(231.,106.));
    assert_eq!(pause_movie::child(background,"mTitleBack").unwrap().entry.matrix.ty,104.);
    let mut batches=Vec::new();
    flatten_projected(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches,true,None).unwrap();
    assert!(!batches.is_empty());
    view.rows[0].state=objective_state::COMPLETE;
    movie.set_state(&assets,4,&view).unwrap();
    assert_eq!(movie.sounds,["FE_SOUND_TRICK_SUCCESSFUL"]);
    for _ in 0..80 {movie.tick(&assets).unwrap();}
    movie.set_state(&assets,2,&view).unwrap();
    for _ in 0..30 {movie.tick(&assets).unwrap();}
    assert!(!movie.root.visible);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_tutorial_movie_shows_tips_animations_and_trick_guides() {
    use skate2_core::challenge::tutorial_hud::{TrickSlot, TutorialHudView, screen_state};
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let text=|clip: &mut Clip| clip.text.as_deref().map(String::from_utf16_lossy);
    let g_ollie=assets.movies[31].movie.exports.iter().find(|e| e.name=="g_ollie").unwrap().character_id;
    let mut movie=tutorial_movie::TutorialMovie::new(&assets).unwrap();
    assert!(!movie.root.visible);
    // The constructor's ClearTricks: every guide item hidden at its slot; each
    // Gesture Item already holds its placement's g_ollie at depth 0x4000.
    let group=movie.root.named(&["container_mc","mcGestureButtonGroup"]).unwrap();
    for i in 0..4 {
        let item=pause_movie::child(group,&format!("mcGestureItem{i}")).unwrap();
        assert!(!item.clip.visible);
        assert_eq!(item.entry.matrix.tx,i as f32*130.+48.);
        let gesture=item.clip.children.get(&0x4000).unwrap();
        assert_eq!((gesture.entry.name.as_deref(),gesture.clip.character),(Some("gestureitem"),g_ollie));
        assert!(!item.clip.named(&["placeholder"]).unwrap().visible);
    }
    let mut view=TutorialHudView {
        hal_id:"ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1_TIP".into(),
        slots:Vec::new(), stance_regular:true, stance_flipped:false,
    };
    movie.set_state(&assets,screen_state::SHOW,&view).unwrap();
    movie.set_state(&assets,screen_state::OLLIE,&view).unwrap();
    assert!(movie.root.visible);
    assert_eq!(text(movie.root.named(&["container_mc","mTextMC","label"]).unwrap()).as_deref(),Some("Ollie"));
    let group=movie.root.named(&["container_mc","mcGestureButtonGroup"]).unwrap();
    let ollie=pause_movie::child(group,"mAnimOllie").unwrap();
    assert!(ollie.clip.visible);
    assert_eq!(ollie.entry.matrix.a,-1.);
    assert_eq!(pause_movie::child(&mut ollie.clip,"mCommonText").unwrap().entry.matrix.a,-1.);
    // The stance's gestures flipped (77) turn the shown animation round.
    view.stance_flipped=true;
    movie.set_state(&assets,screen_state::STANCE,&view).unwrap();
    let group=movie.root.named(&["container_mc","mcGestureButtonGroup"]).unwrap();
    assert_eq!(pause_movie::child(group,"mAnimOllie").unwrap().entry.matrix.a,1.);
    // mAnim360Popshuvit reaches the authored mAnim360popshuvit.
    movie.set_state(&assets,screen_state::POPSHUVIT_360,&view).unwrap();
    let group=movie.root.named(&["container_mc","mcGestureButtonGroup"]).unwrap();
    assert!(!pause_movie::child(group,"mAnimOllie").unwrap().clip.visible);
    assert!(pause_movie::child(group,"mAnim360popshuvit").unwrap().clip.visible);
    // A button joined by Plus to a gesture: kerning pulls the gesture left and
    // the group re-centres by its two-slot offset and half the kerning.
    view.stance_flipped=false;
    view.slots=vec![
        TrickSlot {gesture:String::new(),button:"A".into(),connector:"Plus".into()},
        TrickSlot {gesture:"ollie".into(),button:String::new(),connector:String::new()},
    ];
    movie.set_state(&assets,screen_state::INSTRUCTION,&view).unwrap();
    let group=pause_movie::child(movie.root.named(&["container_mc"]).unwrap(),"mcGestureButtonGroup").unwrap();
    assert_eq!((group.entry.matrix.tx,group.entry.matrix.ty),(200.5,-69.));
    let group=&mut group.clip;
    let button=pause_movie::child(group,"mcButtonItem0").unwrap();
    assert!(button.clip.visible);
    assert_eq!(button.entry.matrix.tx,19.);
    assert_eq!(button.clip.named(&["mButtonRender"]).unwrap().properties.get("_test").map(String::as_str),Some("A"));
    let plus=pause_movie::child(group,"mcConnectorPlus0").unwrap();
    assert!(plus.clip.visible);
    assert_eq!(plus.entry.matrix.tx,40.);
    let gesture=pause_movie::child(group,"mcGestureItem1").unwrap();
    assert!(gesture.clip.visible);
    assert_eq!(gesture.entry.matrix.tx,122.);
    assert_eq!(gesture.clip.children.get(&0x4000).unwrap().entry.matrix.a,-1.);
    assert!(!pause_movie::child(group,"mcGestureItem0").unwrap().clip.visible);
    assert!(!pause_movie::child(group,"mcConnectorArrow0").unwrap().clip.visible);
    let mut batches=Vec::new();
    flatten_projected(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches,true,None).unwrap();
    assert!(!batches.is_empty());
    // A tip the large label wraps moves mTextMC to its small text for good and
    // grows the background 18 per further small line.
    view.slots.clear();
    view.hal_id="ID_CHALLENGE_M_1A_04_TUTORIAL_MANUALS_OBJ1_TIP".into();
    movie.set_state(&assets,screen_state::INSTRUCTION,&view).unwrap();
    let container=movie.root.named(&["container_mc"]).unwrap();
    assert_eq!(container.named(&["mTextMC","label"]).unwrap().character,68);
    let background=container.named(&["mTutBackground"]).unwrap();
    let extra=pause_movie::child(background,"mBackerbottom").unwrap().entry.matrix.ty-108.;
    assert!(extra>=18. && extra%18.==0.,"{extra}");
    assert_eq!(pause_movie::child(background,"mBackerreflect").unwrap().entry.matrix.ty,15.+extra);
    assert!((pause_movie::child(background,"mBackercore").unwrap().entry.matrix.d*16.-(16.+extra)).abs()<0.001);
    view.hal_id="ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1_TIP".into();
    movie.set_state(&assets,screen_state::NONE,&view).unwrap();
    let container=movie.root.named(&["container_mc"]).unwrap();
    assert_eq!(container.named(&["mTextMC","label"]).unwrap().character,68);
    assert_eq!(text(container.named(&["mTextMC","label"]).unwrap()).as_deref(),Some("Ollie"));
    for _ in 0..200 {movie.tick(&assets).unwrap();}
    movie.set_state(&assets,screen_state::HIDE,&view).unwrap();
    assert!(!movie.root.visible);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_action_overlay_signs_up_and_hides() {
    use skate2_core::challenge::signup_hud::SignUpHudView;
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let text=|clip: &mut Clip| clip.text.as_deref().map(String::from_utf16_lossy);
    let mut movie=action_overlay_movie::ActionOverlayMovie::new(&assets).unwrap();
    for _ in 0..3 {movie.tick(&assets).unwrap();}
    // The constructor: `Sign up` in both symbol fields, the button left of it
    // by SetSymbol's -4 - _width / 2 and the symbol 16 right of its placement.
    let symbol=movie.root.named(&["mSignup","mOverlaySymbol","signup"]).unwrap();
    assert_eq!(text(symbol).as_deref(),Some("Sign up"));
    let width=objective_movie::text_field_width(movie.root.named(&["mSignup","mOverlaySymbol","signup"]).unwrap(),&assets).unwrap();
    let signup=movie.root.named(&["mSignup"]).unwrap();
    assert_eq!(pause_movie::child(signup,"mButtonItem").unwrap().entry.matrix.tx,-4.-width/2.);
    assert_eq!(pause_movie::child(signup,"mOverlaySymbol").unwrap().entry.matrix.tx,-215.+16.);
    let view=SignUpHudView {title:"ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_TITLE".into(),formatted:true,dmo:None};
    movie.set_state(&assets,1,&view).unwrap();
    movie.set_state(&assets,1,&view).unwrap();
    for _ in 0..15 {movie.tick(&assets).unwrap();}
    let placed=pause_movie::child(&mut movie.root,"mSignup").unwrap();
    assert_eq!((placed.entry.matrix.tx,placed.entry.matrix.ty),(640.,352.));
    assert_eq!(placed.entry.colour_scale>>24,0xff);
    let signup=movie.root.named(&["mSignup"]).unwrap();
    assert!(signup.named(&["label"]).unwrap().visible);
    assert_eq!(text(signup.named(&["label"]).unwrap()).as_deref(),Some("Flipping Wood 101"));
    assert_eq!(signup.named(&["mButtonItem","mButtonRender"]).unwrap().properties.get("_test").map(String::as_str),Some("DPad_Up"));
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty());
    // Hidden: the button outro fades it out and stops.
    movie.set_state(&assets,0,&view).unwrap();
    for _ in 0..15 {movie.tick(&assets).unwrap();}
    assert_eq!(pause_movie::child(&mut movie.root,"mSignup").unwrap().entry.colour_scale>>24,0);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_signup_poster_lays_out_the_tutorial_challenge() {
    use skate2_core::challenge::signup_poster::{PosterText, PosterView};
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let text=|clip: &mut Clip| clip.text.as_deref().map(String::from_utf16_lossy);
    // m_1a_03's poster natives, as the core's sign-up test reads them.
    let view=PosterView {
        title:PosterText {text:"ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_TITLE".into(),formatted:true},
        description:PosterText {text:"ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_DESCRIPTION".into(),formatted:true},
        cash:300,
        description_enabled:true,
        tutorial_nis:Some("ID_CIC_VIEW_FLIP_TRICKS".into()),
        objectives:vec![("ID_CHALLENGE_M_1A_03_TUTORIAL_FLIPTRICKS_OBJ1".into(),String::new())],
        ..PosterView::default()
    };
    let mut movie=signup_poster_movie::SignUpPosterMovie::default();
    movie.update(&assets,Some(&view),(1280,720)).unwrap();
    assert!(movie.open);
    let root=movie.root.as_mut().unwrap();
    // HasTutorialNIS: the button backer's tutorial frames.
    let backer=root.named(&["mBackTop","mBacker","mButtonbacker"]).unwrap();
    let frame=backer.playback.timeline().frame();
    assert!(frame.is_some_and(|f| f>=14),"{frame:?}");
    let info=root.named(&["mBackTop","mcChallengeInfo"]).unwrap();
    assert!(info.visible);
    // The header at the prototype's zero title offsets with the formatted
    // title, which fits one large line.
    let header=pause_movie::child(info,"mHeader").unwrap();
    assert_eq!((header.entry.matrix.tx,header.entry.matrix.ty),(0.,0.));
    assert_eq!(text(info.named(&["mHeader","labelText"]).unwrap()).as_deref(),Some("Flipping Wood 101"));
    assert_eq!(info.named(&["mHeader","labelText"]).unwrap().character,105);
    // The description 5 below the header's 40 + 10.
    let description=pause_movie::child(info,"mDescription").unwrap();
    assert!(description.clip.visible);
    assert_eq!(description.entry.matrix.ty,55.);
    eprintln!("description {:?}",text(info.named(&["mDescription","labelText"]).unwrap()));
    // Objectives are off in m_1a_03's local data; no fee.
    assert!(!pause_movie::child(info,"mObjectives").unwrap().clip.visible);
    assert!(!pause_movie::child(info,"mEntryFee").unwrap().clip.visible);
    // The tutorial-video option and the Accept/Decline menu at their rows.
    let option=pause_movie::child(info,"mNISOption").unwrap();
    assert!(option.clip.visible);
    assert_eq!(option.entry.matrix.ty,525.);
    let label=text(info.named(&["mNISOption","labeltext"]).unwrap()).unwrap();
    eprintln!("tutorial option {label:?}");
    assert!(!label.trim().is_empty());
    let menu=pause_movie::child(info,"mView").unwrap();
    assert!(menu.clip.visible);
    assert_eq!(menu.entry.matrix.ty,577.);
    let menu=info.named(&["mView"]).unwrap();
    assert_eq!(text(menu.named(&["mText0"]).unwrap()).as_deref(),Some("Accept"));
    assert_eq!(text(menu.named(&["mText1"]).unwrap()).as_deref(),Some("Decline"));
    assert!(!menu.named(&["mText2"]).unwrap().visible);
    assert_eq!(menu.named(&["mButton0","mButtonRender"]).unwrap().properties.get("_test").map(String::as_str),Some("A"));
    assert_eq!(menu.named(&["mButton1","mButtonRender"]).unwrap().properties.get("_test").map(String::as_str),Some("B"));
    let mut batches=Vec::new();
    flatten(&assets,movie.root.as_ref().unwrap(),IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty());
    // The intro reports IntroComplete on frame 56.
    let mut ticks=0;
    while !movie.take_intro_complete() {
        movie.tick(&assets).unwrap();
        ticks+=1;
        assert!(ticks<120,"no IntroComplete");
    }
    assert_eq!(ticks,55);
    // A 2:1 window is 160 stage units wider than 16:9: the vignette grows by them about its centre.
    let extent=|movie:&mut signup_poster_movie::SignUpPosterMovie| {
        let root=movie.root.as_mut().unwrap();
        let parent=pause_movie::child(root,"vignette_mc").unwrap().entry.matrix;
        let vignette=pause_movie::child(root.named(&["vignette_mc"]).unwrap(),"mVignette").unwrap();
        let r=pause_movie::bounds(&vignette.clip,&assets).unwrap().unwrap();
        let m=vignette.entry.matrix;
        let stage=|x:f32| parent.tx+parent.a*(m.tx+m.a*x);
        (stage(r.left),stage(r.right))
    };
    let (left,right)=extent(&mut movie);
    movie.update(&assets,Some(&view),(2000,1000)).unwrap();
    let (wide_left,wide_right)=extent(&mut movie);
    assert!((left-wide_left-80.).abs()<0.01 && (wide_right-right-80.).abs()<0.01,"{left} {right} {wide_left} {wide_right}");
    movie.update(&assets,Some(&view),(1280,720)).unwrap();
    assert_eq!(extent(&mut movie),(left,right));
    // Closing plays the outro, whose OutroComplete unloads the screen.
    movie.update(&assets,None,(1280,720)).unwrap();
    assert!(!movie.open);
    for _ in 0..12 {movie.tick(&assets).unwrap();}
    assert!(movie.root.is_none());
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_unlock_overlay_counts_the_rewards_and_cash() {
    use skate2_core::{challenge::unlock_overlay::UnlockOverlayView, ui::apt_projection::{SPIN_DISTANCE, SPIN_START_ANGLE, Spin, SpinMode}};
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let text=|clip: &mut Clip| clip.text.as_deref().map(String::from_utf16_lossy);
    // m_1a_03's overlay, as the core's fliptrick test reads it: Reda and the
    // player's $300, no cash award.
    let view=UnlockOverlayView {
        phone_numbers:vec!["ID_UNLOCK_PHONE_NUMBER".into()],
        characters:vec!["ID_COMMON_REDA".into()],
        character_unlocked:true,
        location_icon_text:"ID_UNLOCK_LOCATION".into(),
        thrasher_coverage:true,
        sponsor_unlock_text:"ID_COMMON_UNKNOWN".into(),
        cash:300,
        ..UnlockOverlayView::default()
    };
    // The front end advances its one spin before the movies.
    let mut spin=Spin::default();
    let mut movie=unlock_overlay_movie::UnlockOverlayMovie::default();
    movie.update(&assets,Some(&view),&mut spin).unwrap();
    assert!(movie.open);
    assert_eq!(spin.mode,SpinMode::Intro);
    let root=movie.root.as_mut().unwrap();
    // One reward (the character), no sponsor items, the cash: 101-intro.
    let main=root.named(&["main_mc"]).unwrap();
    assert_eq!(main.playback.timeline().frame(),Some(19));
    assert!(!root.named(&["main_mc","mUnlocks","labeltext"]).unwrap().visible);
    // No prize, wager or sponsor bonus: their values hidden, the wallet at $300.
    for value in ["mCashPrizeVal","mWagerVal","mSponsorVal"] {
        assert!(!root.named(&["main_mc","mCash",value]).unwrap().visible,"{value}");
    }
    let wallet=text(root.named(&["main_mc","mCash","mWalletVal"]).unwrap()).unwrap();
    eprintln!("wallet {wallet:?}");
    assert!(wallet.contains("300"),"{wallet}");
    let proceed=assets.localize("ID_COMMON_PROCEED").unwrap();
    let menu=root.named(&["main_mc","mEndOptions","mButtonMenu"]).unwrap();
    assert_eq!(text(menu.named(&["mText0"]).unwrap()),Some(proceed));
    assert!(!menu.named(&["mText1"]).unwrap().visible);
    assert_eq!(menu.named(&["mButton0","mButtonRender"]).unwrap().properties.get("_test").map(String::as_str),Some("A"));
    // The intro's IntroComplete on root frame 10, then the steps: a 30-update
    // delay, Reda's card (icon_update frames 37..86), two empty slots, the
    // sponsor logo (iconintro 10..21), the five items off, then the cash lines
    // (prize, wager, sponsor bonus, wallet) and Cash_SetCashAnimDone.
    let mut intro=None;
    let mut ticks=0;
    while !movie.take_cash_anim_done() {
        spin.update();
        movie.tick(&assets).unwrap();
        ticks+=1;
        if intro.is_none() && movie.take_intro_complete() {intro=Some(ticks);}
        if ticks==31 {
            let root=movie.root.as_mut().unwrap();
            let card=root.named(&["main_mc","mAwards","mMainComp0","label_mc","labeltext"]).unwrap();
            assert_eq!(card.text.as_deref(),Some(assets.initial_text("ID_COMMON_REDA").as_slice()));
        }
        assert!(ticks<400,"no Cash_SetCashAnimDone");
    }
    assert_eq!(intro,Some(9));
    assert_eq!(ticks,104);
    assert_eq!((spin.mode,spin.angle <= SPIN_DISTANCE),(SpinMode::Idle,true));
    // Drawn through the spin's projection.
    let mut batches=Vec::new();
    flatten_projected(&assets,movie.root.as_ref().unwrap(),IDENTITY,Colour::default(),(1280,720),&mut batches,true,None).unwrap();
    assert!(!batches.is_empty());
    project_spin((1280,720),spin.angle,&mut batches);
    assert!(batches.iter().flat_map(|b| &b.vertices).all(|v| v[3]>0.));
    // Closing plays the outro and spins back; OutroComplete unloads it.
    movie.update(&assets,None,&mut spin).unwrap();
    assert!(!movie.open);
    for _ in 0..9 {spin.update(); movie.tick(&assets).unwrap();}
    assert!(movie.root.is_none());
    for _ in 0..30 {
        if spin.mode==SpinMode::None {break;}
        spin.update();
        movie.tick(&assets).unwrap();
    }
    assert_eq!((spin.mode,spin.angle),(SpinMode::None,SPIN_START_ANGLE));
    // numMainRewards stays on the class: an overlay without rewards still
    // lays out the rewards panel.
    movie.update(&assets,Some(&UnlockOverlayView {cash:300,..UnlockOverlayView::default()}),&mut spin).unwrap();
    let main=movie.root.as_mut().unwrap().named(&["main_mc"]).unwrap();
    assert_eq!(main.playback.timeline().frame(),Some(19));
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_skate101_shows_the_title_text_and_button() {
    use skate2_core::challenge::system::Skate101View;
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    // 201_Follow_Slappy, which Trigger_m_1a_06_photo_puzzle's final state
    // sends.
    let view=Skate101View {title:"ID_201_TITLE_FOLLOW_SLAPPY".into(),text:"ID_201_TEXT_FOLLOW_SLAPPY".into()};
    let mut movie=skate101_movie::Skate101Movie::default();
    movie.update(&assets,Some(&view)).unwrap();
    assert!(movie.open);
    let root=movie.root.as_mut().unwrap();
    assert_eq!(root.playback.timeline().frame(),Some(1));
    let title=root.named(&["mcTitle","mcHeader","labelText"]).unwrap().text.clone().unwrap();
    assert_eq!(title,localizer::format_custom(&assets,"ID_201_TITLE_FOLLOW_SLAPPY","x").unwrap());
    let body=root.named(&["mcBodyText","mcDescription","labelText"]).unwrap();
    assert_eq!(body.text.as_deref(),Some(assets.initial_text("ID_201_TEXT_FOLLOW_SLAPPY").as_slice()));
    eprintln!("title {:?} text {:?}",String::from_utf16_lossy(&title),body.text.as_deref().map(String::from_utf16_lossy));
    assert_eq!(root.named(&["mButton","mButtonRender"]).unwrap().properties.get("_test").map(String::as_str),Some("A"));
    // Frame 14's IntroComplete, 13 updates after the intro label.
    let mut ticks=0;
    while !movie.take_intro_complete() {
        movie.tick(&assets).unwrap();
        ticks+=1;
        assert!(ticks<60,"no IntroComplete");
    }
    assert_eq!(ticks,13);
    // The state pops: the outro from frame 15, OutroComplete on frame 29.
    movie.update(&assets,None).unwrap();
    assert!(!movie.open);
    for _ in 0..13 {movie.tick(&assets).unwrap();}
    assert!(movie.root.is_some());
    movie.tick(&assets).unwrap();
    assert!(movie.root.is_none());
}

/// The marker UI assets from the boot archives.
fn boot_assets() -> (MountSet, Assets) {
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    (mounts,assets)
}

/// `mcTextSubtitle` and `mDimmer` show together.
fn subtitle_shown(movie: &mut subtitle_movie::SubtitleMovie) -> bool {
    let text=movie.root.named(&["mcTextSubtitle"]).unwrap().visible;
    assert_eq!(text,movie.root.named(&["mDimmer"]).unwrap().visible);
    text
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_subtitle_shows_a_nis_line_over_its_band_and_clears() {
    let (_mounts,assets)=boot_assets();
    let mut movie=subtitle_movie::SubtitleMovie::new(&assets).unwrap();
    // The constructor's Clear().
    assert!(!subtitle_shown(&mut movie));
    // tutorial_m_1a_goto_slappys' first line: nothing refreshes with the
    // subtitles option off.
    movie.update(&assets,Some(0x96c6_5a5b),false).unwrap();
    assert!(!subtitle_shown(&mut movie));
    movie.update(&assets,Some(0x96c6_5a5b),true).unwrap();
    assert!(subtitle_shown(&mut movie));
    let field=pause_movie::child(movie.root.named(&["mcTextSubtitle"]).unwrap(),"labelText").unwrap();
    assert_eq!(field.clip.text.as_deref(),Some(assets.localized[&0x96c6_5a5b].as_slice()));
    let lines=tutorial_movie::max_scroll(&field.clip,&assets).unwrap();
    eprintln!("{:?}: {lines} line(s)",String::from_utf16_lossy(field.clip.text.as_deref().unwrap()));
    #[expect(clippy::cast_precision_loss, reason = "a line count")]
    let y=(3.-lines as f32)*12.;
    assert_eq!(field.entry.matrix.ty,y);
    assert!(field.clip.script_transform);
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.len()>=2,"band and text");
    // The NULL key's clear: "#" hides the line, an empty one-line field.
    movie.update(&assets,None,true).unwrap();
    assert!(!subtitle_shown(&mut movie));
    let field=pause_movie::child(movie.root.named(&["mcTextSubtitle"]).unwrap(),"labelText").unwrap();
    assert_eq!(field.clip.text.as_deref(),Some(&[][..]));
    assert_eq!(field.entry.matrix.ty,24.);
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.is_empty());
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_nis_subtitle_lines_format_without_arguments() {
    use skate2_core::nis::subtitles::{BIG_PATH,NULL_HASH,parse};
    use skate2_formats::big::{BigArchive,BigEntryKey};
    let (mounts,assets)=boot_assets();
    let big=mounts.read_path(BIG_PATH).unwrap();
    let archive=BigArchive::parse(&big).unwrap();
    let mut movie=subtitle_movie::SubtitleMovie::new(&assets).unwrap();
    let (mut lines,mut missing,mut heights)=(0,Vec::new(),std::collections::BTreeMap::new());
    for entry in &archive.entries {
        let BigEntryKey::Name(name)=&entry.key else {continue};
        if !name.to_ascii_lowercase().contains("subtitles_english\\") {continue}
        for key in parse(&big[entry.data_range().unwrap()]) {
            if key.start_time==0xffff || key.hash==NULL_HASH {continue}
            lines+=1;
            let Some(text)=assets.localized.get(&key.hash) else {
                missing.push((name.clone(),key.hash));
                continue;
            };
            // SetSubtitleText's FormatString passes no arguments.
            assert!(!text.contains(&u16::from(b'{')),"{name} {:08x}",key.hash);
            movie.update(&assets,Some(key.hash),true).unwrap();
            let field=movie.root.named(&["mcTextSubtitle","labelText"]).unwrap();
            *heights.entry(tutorial_movie::max_scroll(field,&assets).unwrap()).or_insert(0)+=1;
        }
    }
    eprintln!("{lines} lines; lines per subtitle {heights:?}; without a string {missing:x?}");
    assert!(lines>0);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_gesture_tip_shows_its_backer_and_fades_in_the_stick_over_its_intro() {
    let (_mounts,assets)=boot_assets();
    let mut movie=gestollie_movie::GestOllieMovie::new(&assets).unwrap();
    // The tip reads ID_CHALLENGE_TIPS_OLLIE's two lines: maxscroll 2 keeps
    // the backer as authored.
    let backer=movie.root.named(&["mBacker"]).unwrap();
    let label=backer.named(&["label"]).unwrap();
    assert_eq!(label.text.as_deref(),Some(assets.initial_text("ID_CHALLENGE_TIPS_OLLIE").as_slice()));
    assert_eq!(tutorial_movie::max_scroll(label,&assets).unwrap(),2);
    assert!(!pause_movie::child(backer,"mBackerbottom").unwrap().clip.script_transform);
    // PlayClip('intro'): the root plays from frame 0, mName at alpha 0.
    let alpha=|movie:&mut gestollie_movie::GestOllieMovie| pause_movie::child(&mut movie.root,"mName").unwrap().entry.colour_scale>>24;
    assert!(movie.root.playback.playing());
    assert_eq!(movie.root.playback.timeline().frame(),Some(0));
    assert_eq!(alpha(&mut movie),0);
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty(),"the backer and its text show at once");
    for _ in 0..28 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(alpha(&mut movie),0xff);
    movie.tick(&assets).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(29));
    assert!(!movie.root.playback.playing());
    // TutorialGesture_OlliePt1 and Pt2.
    movie.set_state(&assets,1).unwrap();
    assert_eq!(movie.root.named(&["mName"]).unwrap().playback.timeline().frame(),Some(4));
    movie.set_state(&assets,2).unwrap();
    assert_eq!(movie.root.named(&["mName"]).unwrap().playback.timeline().frame(),Some(49));
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_spot_owned_banner_plays_a_corner_intro_holds_and_outros() {
    let (_mounts,assets)=boot_assets();
    let mut movie=ownthespot_movie::OwnTheSpotMovie::new(&assets).unwrap();
    // Constructed and stopped on frame 0; PlayClip('intro') finds no label.
    assert!(!movie.root.playback.playing());
    assert_eq!(movie.root.playback.timeline().frame(),Some(0));
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.is_empty(),"nothing shows before a state");
    // SpotOwned_BottomRight's OnEnterState: kScreenState_IntroBottomRight.
    movie.set_state(&assets,0).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(1));
    assert!(movie.root.visible);
    assert_eq!(movie.sounds,vec!["FE_SOUND_CHALLENGE_OTS"]);
    // The banner shows from frame 19; frame 105 drops it to alpha 0 and the
    // hold label stops there.
    for _ in 0..59 {
        movie.tick(&assets).unwrap();
    }
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty(),"the owned banner shows");
    for _ in 0..46 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(106));
    assert!(!movie.root.playback.playing());
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.is_empty(),"the hold is transparent");
    // Its OnLeaveState: kScreenState_OutroBottomRight runs to the hide.
    movie.set_state(&assets,1).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(114));
    for _ in 0..12 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(126));
    assert!(!movie.root.visible);
    assert!(!movie.root.playback.playing());
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_spot_found_banner_names_the_spot_and_plays_its_corner() {
    use skate2_core::challenge::system::MissionTitle;
    let (_mounts,assets)=boot_assets();
    let mut movie=spotfound_movie::SpotFoundMovie::new(&assets).unwrap();
    assert!(!movie.root.playback.playing());
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.is_empty(),"nothing shows before a state");
    // SpotFound_BottomLeft's OnEnterState: kScreenState_IntroBottomLeft (4),
    // with a localized key standing in for the main challenge's title.
    let title=MissionTitle::Formatted("ID_EVENT_BEAT_THE_ABD_CAPS".into());
    movie.set_state(&assets,4,&title).unwrap();
    let expected=assets.initial_text("ID_EVENT_BEAT_THE_ABD_CAPS");
    let header=assets.initial_text("ID_EVENT_SPOT_FOUND_CAPS");
    for holder in ["mcOwned","mcTextLeftJustified"] {
        assert_eq!(movie.root.named(&[holder,"mcSpotName","labelText"]).unwrap().text.as_deref(),Some(expected.as_slice()));
        assert_eq!(movie.root.named(&[holder,"mcSpotFound","labelText"]).unwrap().text.as_deref(),Some(header.as_slice()));
        assert_eq!(movie.root.named(&[holder,"mcSpotFound","labelText_glow"]).unwrap().text.as_deref(),Some(header.as_slice()));
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(140));
    assert!(movie.root.visible);
    assert_eq!(movie.sounds,vec!["FE_SOUND_SPOT_FOUND"]);
    for _ in 0..12 {
        movie.tick(&assets).unwrap();
    }
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty(),"the banner shows");
    // Its OnLeaveState: kScreenState_OutroBottomLeft runs to the hide.
    movie.set_state(&assets,5,&MissionTitle::Missing).unwrap();
    let missing:Vec<u16>="Missing Instance Title...".encode_utf16().collect();
    assert_eq!(movie.root.named(&["mcOwned","mcSpotName","labelText"]).unwrap().text.as_deref(),Some(missing.as_slice()));
    assert_eq!(movie.root.playback.timeline().frame(),Some(194));
    for _ in 0..11 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(205));
    assert!(!movie.root.visible);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_killed_it_banner_plays_a_corner_through_its_flourish_and_outro() {
    let (_mounts,assets)=boot_assets();
    let mut movie=killedit_movie::KilledItMovie::new(&assets).unwrap();
    assert!(!movie.root.playback.playing());
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.is_empty(),"nothing shows before a state");
    // KilledIt's OnEnterState: SetScreenState(mKilledItScreenId, 1).
    movie.set_state(&assets,1).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(231));
    assert_eq!(movie.sounds,vec!["FE_SOUND_KILLEDIT_INTRO"]);
    for _ in 0..27 {
        movie.tick(&assets).unwrap();
    }
    // tlflourish places sprite 32: its mask (shape 17, depth 1, clip
    // depth 8) over the ornaments at depths 2 and 5.
    assert_eq!(movie.root.playback.timeline().frame(),Some(258));
    let flourish=&movie.root.children[&1];
    let m=concatenate(IDENTITY,flourish.entry.matrix);
    let mask=&flourish.clip.children[&1];
    let rect=mask_rect(&assets,&mask.clip,concatenate(m,mask.entry.matrix)).unwrap();
    assert_eq!(rect,[197.5,250.5,492.5,428.5]);
    let mut batches=Vec::new();
    flatten(&assets,&flourish.clip,m,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty());
    let corner=|x,y| vertex(IDENTITY,[x,y],[0.;2],[0.;4],(1280,720),true);
    let (a,b)=(corner(rect[0],rect[1]),corner(rect[2],rect[3]));
    for t in batches.iter().flat_map(|b|b.vertices.chunks_exact(3)) {
        let c=[(t[0][0]+t[1][0]+t[2][0])/3.,(t[0][1]+t[1][1]+t[2][1])/3.];
        let inside=c[0]>a[0].min(b[0])&&c[0]<a[0].max(b[0])&&c[1]>a[1].min(b[1])&&c[1]<a[1].max(b[1]);
        assert!(!inside,"ornament drawn inside the mask rectangle: {c:?}");
    }
    for _ in 0..13 {
        movie.tick(&assets).unwrap();
    }
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty(),"the banner shows");
    // It runs its flourish and outro unattended and stops on 345.
    for _ in 0..80 {
        movie.tick(&assets).unwrap();
        let mut batches=Vec::new();
        flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(345));
    assert!(!movie.root.playback.playing());
    assert_eq!(movie.sounds,vec!["FE_SOUND_KILLEDIT_INTRO","FE_SOUND_KILLEDIT_FLOURISH","FE_SOUND_KILLEDIT_OUTRO"]);
    // kOutro only addresses the absent logoAnimation.
    movie.set_state(&assets,5).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(345));
}

#[test]
fn a_mask_rectangle_cuts_its_area_out_of_the_content() {
    let size=(1280,720);
    let content=|| vec![Batch{texture:None,wrapped:false,
        vertices:quad(IDENTITY,[0.,0.,640.,360.],[0.,0.,1.,1.],[1.;4],size,true)}];
    let area=|batches:&[Batch]| batches.iter().flat_map(|b|b.vertices.chunks_exact(3)).map(|t|
        ((t[1][0]-t[0][0])*(t[2][1]-t[0][1])-(t[2][0]-t[0][0])*(t[1][1]-t[0][1])).abs()*0.5).sum::<f32>();
    let mut batches=content();
    let whole=area(&batches);
    occlude(&mut batches,[160.,90.,480.,270.],size);
    // The middle half of each side: a quarter of the area is gone.
    assert!((area(&batches)-whole*0.75).abs()<1e-5,"{}",area(&batches));
    for t in batches[0].vertices.chunks_exact(3) {
        let c=[(t[0][0]+t[1][0]+t[2][0])/3.,(t[0][1]+t[1][1]+t[2][1])/3.];
        assert!(!(c[0]>-0.75&&c[0]< -0.25&&c[1]>0.25&&c[1]<0.75),"{c:?}");
        // UVs follow the cut positions.
        for v in t {
            assert!((v[4]-(v[0]+1.)).abs()<1e-6&&(v[5]-(1.-v[1])).abs()<1e-6,"{v:?}");
        }
    }
    // A rectangle without area hides nothing.
    let mut batches=content();
    occlude(&mut batches,[100.,100.,100.,300.],size);
    assert_eq!(batches[0].vertices,content()[0].vertices);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_photo_treatment_flashes_its_frame_on_snapshot_and_fades() {
    let (_mounts,assets)=boot_assets();
    let mut movie=phototreatment_movie::PhotoTreatmentMovie::new(&assets,phototreatment_movie::ASSET).unwrap();
    assert!(!movie.root.playback.playing());
    assert_eq!(movie.root.playback.timeline().frame(),Some(0));
    let draws=|movie:&phototreatment_movie::PhotoTreatmentMovie| {
        let mut batches=Vec::new();
        flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
        batches.iter().map(|b|b.vertices.len()).sum::<usize>()
    };
    // snapshot_mc waits at alpha 0 in its widescreen layout.
    assert_eq!(draws(&movie),0);
    movie.set_state(&assets,0).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(0));
    // PhotoTreatment's state 1: the snapshot flashes in with its sound.
    movie.set_state(&assets,1).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(1));
    assert_eq!(movie.sounds,vec!["FE_SOUND_PHOTO_FLYTHROUGH"]);
    assert!(draws(&movie)>0);
    // It fades out over frames 17 to 25 and stops transparent on 26.
    for _ in 0..25 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(26));
    assert!(!movie.root.playback.playing());
    assert_eq!(draws(&movie),0);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_pro_bio_card_fills_from_get_pro_info_and_plays_a_side() {
    let (_mounts,assets)=boot_assets();
    let mut movie=pro_bio_movie::ProBioMovie::new(&assets).unwrap();
    assert!(!movie.root.visible);
    assert!(!movie.root.playback.playing());
    // NIS.GetProInfo() for fp_u3_tutorial03: Danny Way.
    let pro=["ID_CHEAT_PROSKATER19".to_string(),"ID_CHEAT_PRODETAIL_LOCATION13".into(),"ID_CHEAT_PRODETAIL_REGULAR".into()];
    movie.set_state(&assets,0,&pro).unwrap();
    assert_eq!(movie.root.playback.timeline().frame(),Some(10));
    assert!(movie.root.visible);
    assert_eq!(movie.sounds,vec!["PlayInterfaceOverlayIn"]);
    let text=|movie:&mut pro_bio_movie::ProBioMovie,path:&[&str]| movie.root.named(path).unwrap().text.clone().unwrap();
    for (path,label) in [(&["mcTextHeader","labelText"][..],"ID_CHEAT_PROSKATER19"),
        (&["mcTextBody2","labelText"],"ID_CHEAT_PRODETAIL_LOCATION13"),(&["mcTextBody2","labelTitle"],"ID_NIS_HOMETOWN"),
        (&["mcTextBody3","labelText"],"ID_CHEAT_PRODETAIL_REGULAR"),(&["mcTextBody3","labelTitle"],"ID_NIS_STANCE")] {
        assert_eq!(text(&mut movie,path),assets.initial_text(label));
    }
    // The shipped strings carry ID_NIS_STANCE only; the others show the
    // lookup's default.
    assert_eq!(text(&mut movie,&["mcTextBody3","labelTitle"]),"STANCE:".encode_utf16().collect::<Vec<_>>());
    assert_eq!(text(&mut movie,&["mcTextHeader","labelText"]),vec![u16::from(b' ')]);
    for _ in 0..21 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(31));
    assert!(!movie.root.playback.playing());
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(!batches.is_empty(),"the card holds");
    movie.set_state(&assets,1,&pro).unwrap();
    assert_eq!(movie.sounds,vec!["PlayInterfaceOverlayIn","PlayInterfaceOverlayOut"]);
    for _ in 0..33 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(65));
    assert!(!movie.root.visible);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn apt_fonts_match_without_case_and_fall_back_to_the_first_non_debug_font() {
    let (_mounts,assets)=boot_assets();
    // The default group's first reference.
    assert_eq!(assets.font_fallback,"Helvetica LT Std");
    let texture=|name:&str| assets.apt_font(name).unwrap().texture.clone();
    assert_eq!(texture("helveticapresto"),texture("HelveticaPresto"));
    // pro_bioinfo_overlay's "TradeBld Stroke" is in no font group.
    assert_eq!(texture("TradeBld Stroke"),texture("Helvetica LT Std"));
    assert_ne!(texture("Marker_streak"),texture("Helvetica LT Std"));
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_stance_badges_wait_unseen_then_scale_in_and_fade_out() {
    let (_mounts,assets)=boot_assets();
    for asset in [gvr_movie::REGULAR,gvr_movie::GOOFY] {
        let mut movie=gvr_movie::GvrMovie::new(&assets,asset).unwrap();
        let draws=|movie:&gvr_movie::GvrMovie| {
            let mut batches=Vec::new();
            flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
            batches.iter().map(|b|b.vertices.len()).sum::<usize>()
        };
        // Nothing stops the root: it runs to frame 3 with the badge at
        // alpha 0.
        for _ in 0..5 {
            movie.tick(&assets).unwrap();
        }
        assert_eq!(movie.root.playback.timeline().frame(),Some(3));
        assert_eq!(draws(&movie),0);
        movie.set_state(&assets,1).unwrap();
        assert_eq!(movie.root.playback.timeline().frame(),Some(4));
        assert!(draws(&movie)>0);
        for _ in 0..20 {
            movie.tick(&assets).unwrap();
        }
        assert_eq!(movie.root.playback.timeline().frame(),Some(22));
        movie.set_state(&assets,-1).unwrap();
        assert_eq!(movie.root.playback.timeline().frame(),Some(23));
        for _ in 0..25 {
            movie.tick(&assets).unwrap();
        }
        assert_eq!(movie.root.playback.timeline().frame(),Some(42));
        assert!(!movie.root.playback.playing());
    }
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_san_van_tv_logo_holds_its_first_frame_until_cleared() {
    let (_mounts,assets)=boot_assets();
    let mut movie=sanvantv_movie::SanVanTvMovie::new(&assets).unwrap();
    // PlayClip('intro') plays the root; mLogo waits on its first frame.
    assert_eq!(movie.root.playback.timeline().frame(),Some(1));
    assert!(movie.root.playback.playing());
    // mLogo's first frame draws nothing, and OnEnterState's SetScreenState
    // (id, 1) (0x82667cbc) is no state the class handles: the logo never
    // shows before its hide.
    let mut batches=Vec::new();
    flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
    assert!(batches.is_empty());
    for _ in 0..30 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(movie.root.playback.timeline().frame(),Some(29));
    let logo_frame=|movie:&mut sanvantv_movie::SanVanTvMovie| movie.root.named(&["mLogo"]).unwrap().playback.timeline().frame();
    assert_eq!(logo_frame(&mut movie),Some(0));
    movie.set_state(&assets,1).unwrap();
    assert_eq!(logo_frame(&mut movie),Some(0));
    movie.set_state(&assets,-1).unwrap();
    assert_eq!(logo_frame(&mut movie),Some(121));
    for _ in 0..10 {
        movie.tick(&assets).unwrap();
    }
    assert_eq!(logo_frame(&mut movie),Some(129));
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_fixed_and_non_fixed_photo_treatments_snap_with_their_own_sound() {
    let (_mounts,assets)=boot_assets();
    for (asset,sound) in [(phototreatment_movie::FIXED,"FE_SOUND_PHOTO_SNAPPING"),(phototreatment_movie::NON_FIXED,"FE_SOUND_PHOTO_SKATE1")] {
        let mut movie=phototreatment_movie::PhotoTreatmentMovie::new(&assets,asset).unwrap();
        assert_eq!(movie.root.playback.timeline().frame(),Some(0));
        assert!(!movie.root.playback.playing());
        movie.set_state(&assets,1).unwrap();
        assert_eq!(movie.sounds,vec![sound]);
        for _ in 0..25 {
            movie.tick(&assets).unwrap();
        }
        assert_eq!(movie.root.playback.timeline().frame(),Some(26));
        assert!(!movie.root.playback.playing());
    }
}

/// The end-of-game screens build from the original movies for a three-player party: S.K.A.T.E.'s
/// throwdown results (whose `skate` section authors four rows), the results list, and Change Game
/// through its intro, a state and its outro.
#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_party_play_end_screens_build() {
    use skate2_core::{
        EndOptionsView,
        challenge::{results::{self, EndChallengeOptions}, system::MissionTitle},
        ui::{apt_projection::Spin, party_play::{ChangeSnapshot, Game, change_state}},
    };
    let (_mounts,assets)=boot_assets();
    let mut leaderboard=results::LeaderboardState::default().final_results;
    leaderboard.num_players=3;
    for (i,player) in (0_i32..).zip(leaderboard.players.iter_mut().take(3)) {
        player.position=i+1;
        player.score=10*(3-i);
        player.name=format!("#Player {}",i+1);
    }
    for (column,title) in ["ID_LEADERBOARD_HEADER_RANK","ID_LEADERBOARD_NAME_HEADER","ID_LEADERBOARD_POINTS_HEADER"].into_iter().enumerate() {
        leaderboard.col_titles[column]=title.into();
    }
    let view=|throwdown,screen| EndOptionsView {
        options:EndChallengeOptions {win:true,throwdown,challenge:0,screen},
        leaderboard:leaderboard.clone(),
        second:leaderboard.clone(),
        stance_regular:true, team_totals:[0;2], online:None,
    };
    let title=MissionTitle::Missing;
    let mut spin=Spin::default();
    let text=|clip:&mut clips::Clip,path:&[&str]| clip.named(path).unwrap().text.clone().map(|t| String::from_utf16_lossy(&t));

    let mut skate=throwdown_results_movie::ThrowdownResultsMovie::default();
    skate.update(&assets,Some(&view(results::throwdown::SKATE,results::screen::THROWDOWN)),&title,&mut spin).unwrap();
    for _ in 0..30 {skate.tick(&assets).unwrap();}
    let root=skate.root.as_mut().unwrap();
    assert_eq!(text(root,&["mThrowDown","playerName1","name_label"]).as_deref(),Some("Player 2"));

    let mut list=end_challenge_movie::EndChallengeMovie::default();
    list.update(&assets,Some(&view(results::throwdown::NONE,results::screen::RESULTS)),&title,&mut spin).unwrap();
    for _ in 0..30 {list.tick(&assets).unwrap();}
    let root=list.root.as_mut().unwrap();
    let cell=|row:usize,column:usize| ["mResults".to_owned(),"mcList".into(),format!("ROW{row}"),format!("COL{column}"),"mTextBox".into(),"textbox".into()];
    let path=cell(0,1);
    assert_eq!(text(root,&path.iter().map(String::as_str).collect::<Vec<_>>()).as_deref(),Some("Player 1"));
    let path=cell(2,2);
    assert_eq!(text(root,&path.iter().map(String::as_str).collect::<Vec<_>>()).as_deref(),Some("10"));

    let mut change=party_play_change_movie::PartyPlayChangeMovie::default();
    let snapshot=ChangeSnapshot {game:Game::Skate,location:"#S.K.A.T.E. the Gap".into(),image:None};
    change.update(&assets,Some(&snapshot),&[],None).unwrap();
    for _ in 0..70 {change.tick(&assets).unwrap();}
    assert!(change.take_intro_complete());
    let root=change.root.as_mut().unwrap();
    assert_eq!(text(root,&["mMenu","item0","mSubtitle","TEXTLABEL"]),Some(assets.localize("ID_PARTYPLAY_SKATE_TITLE").unwrap()));
    assert_eq!(text(root,&["mMenu","item1","mSubtitle","TEXTLABEL"]).as_deref(),Some("S.K.A.T.E. the Gap"));
    change.update(&assets,Some(&snapshot),&[change_state::SELECT_LOCATION],None).unwrap();
    assert!(change.root.as_mut().unwrap().named(&["mMenu","item1","mArrowright"]).unwrap().visible);
    change.update(&assets,None,&[],None).unwrap();
    for _ in 0..20 {change.tick(&assets).unwrap();}
    assert!(change.root.is_none(),"the outro unloads the screen");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_failed_small_screen_builds() {
    use skate2_core::{EndOptionsView, challenge::results::{self, EndChallengeOptions}};
    let (_mounts,assets)=boot_assets();
    let view=EndOptionsView {
        options:EndChallengeOptions {win:false,throwdown:results::throwdown::NONE,challenge:0,screen:results::screen::FAILED_SMALL},
        leaderboard:results::LeaderboardState::default().round,
        second:results::LeaderboardState::default().round_second,
        stance_regular:true, team_totals:[0;2], online:None,
    };
    let text=|clip:&mut clips::Clip,path:&[&str]| clip.named(path).unwrap().text.clone().map(|t| String::from_utf16_lossy(&t));
    let x=|menu:&clips::Clip,name:&str| menu.children.values().find(|c| c.entry.name.as_deref()==Some(name)).unwrap().entry.matrix.tx;
    let mut random=apt_random::AptRandom::new();
    let mut screen=challenge_failed_small_movie::ChallengeFailedSmallMovie::default();
    screen.update(&assets,Some(&view),&mut random).unwrap();
    let root=screen.root.as_mut().unwrap();
    // Without players: A then B, left-justified, for Continue_Retry and Retry_Quit after a loss.
    assert_eq!(text(root,&["mEndOptions","mButtonMenu","mText0"]),Some(assets.localize("ID_COMMON_RETRY").unwrap()));
    assert_eq!(text(root,&["mEndOptions","mButtonMenu","mText1"]),Some(assets.localize("ID_COMMON_QUIT").unwrap()));
    let menu=root.named(&["mEndOptions","mButtonMenu"]).unwrap();
    assert!(x(menu,"mButton0")<x(menu,"mText0") && x(menu,"mText0")<x(menu,"mButton1") && x(menu,"mButton1")<x(menu,"mText1"));
    let title=text(root,&["failTitle","title_txt"]).unwrap();
    assert!((0..5).any(|i| assets.localize(&format!("ID_CHALLENGE_FAILED_COMMENT{i}")).unwrap()==title));
    for _ in 0..40 {screen.tick(&assets).unwrap();}
    {let mut draws=Vec::new(); flatten(&assets,screen.root.as_ref().unwrap(),IDENTITY,Colour::default(),(1280,720),&mut draws).unwrap(); assert!(!draws.is_empty(),"the screen draws");}
    assert!(screen.take_intro_complete());
    screen.update(&assets,None,&mut random).unwrap();
    for _ in 0..12 {screen.tick(&assets).unwrap();}
    assert!(screen.take_outro_complete());
    assert!(screen.root.is_none(),"the outro unloads the screen");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_distance_meter_shows_the_distance() {
    use skate2_core::challenge::turn_hud::{DistanceText, TurnHudView};
    let (_mounts,assets)=boot_assets();
    let mut meter=hud_distance_movie::DistanceMovie::new(&assets).unwrap();
    assert!(!meter.root.visible,"the constructor's frame hides the movie");
    let view=TurnHudView::Distance(DistanceText {distance:Some(12.34),red:true});
    meter.set_state(&assets,3,&view).unwrap();
    meter.set_state(&assets,1,&view).unwrap();
    for _ in 0..12 {meter.tick(&assets).unwrap();}
    assert!(meter.root.visible,"hudintro shows the movie");
    {let mut draws=Vec::new(); flatten(&assets,&meter.root,IDENTITY,Colour::default(),(1280,720),&mut draws).unwrap(); assert!(!draws.is_empty(),"the screen draws");}
    let label=meter.root.named(&["mcTextTimer","mcTimer","labelText"]).unwrap();
    assert_eq!(label.text.clone(),Some(localizer::format_custom(&assets,"ID_COMMON_DISTANCE","12.3").unwrap()));
    assert_eq!(label.properties.get("text_colour").map(String::as_str),Some("4288094987"));
    meter.set_state(&assets,2,&view).unwrap();
    for _ in 0..12 {meter.tick(&assets).unwrap();}
    assert!(!meter.root.visible,"hudoutro hides the movie");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_end_challenge_small_screen_builds() {
    use skate2_core::{EndOptionsView, challenge::{results::{self, EndChallengeOptions}, system::MissionTitle}, ui::apt_projection::Spin};
    let (_mounts,assets)=boot_assets();
    let view=EndOptionsView {
        options:EndChallengeOptions {win:true,throwdown:results::throwdown::NONE,challenge:0,screen:results::screen::RESULTS_SMALL},
        leaderboard:results::LeaderboardState::default().round,
        second:results::LeaderboardState::default().round_second,
        stance_regular:true, team_totals:[0;2], online:None,
    };
    let text=|clip:&mut clips::Clip,path:&[&str]| clip.named(path).unwrap().text.clone().map(|t| String::from_utf16_lossy(&t));
    let mut random=apt_random::AptRandom::new();
    let mut spin=Spin::default();
    let mut screen=end_challenge_small_movie::EndChallengeSmallMovie::default();
    screen.update(&assets,Some(&view),&MissionTitle::Missing,&mut spin,&mut random).unwrap();
    let root=screen.root.as_mut().unwrap();
    // Continue_Retry and Retry_Quit after a win: B Retry, then A Continue.
    assert_eq!(text(root,&["mEndOptions","mButtonMenu","mText0"]),Some(assets.localize("ID_COMMON_RETRY").unwrap()));
    assert_eq!(text(root,&["mEndOptions","mButtonMenu","mText1"]),Some(assets.localize("ID_COMMON_CONTINUE").unwrap()));
    let name=text(root,&["mcMiddleInfo","mcTextChallenegeName"]).unwrap();
    assert!((0..19).any(|i| assets.localize(&format!("ID_CHALLENGE_COMPLETE_COMMENT{i}")).unwrap()==name));
    for _ in 0..12 {screen.tick(&assets).unwrap();}
    {let mut draws=Vec::new(); flatten(&assets,screen.root.as_ref().unwrap(),IDENTITY,Colour::default(),(1280,720),&mut draws).unwrap(); assert!(!draws.is_empty(),"the screen draws");}
    assert!(screen.take_intro_complete());
    screen.update(&assets,None,&MissionTitle::Missing,&mut spin,&mut random).unwrap();
    for _ in 0..12 {screen.tick(&assets).unwrap();}
    assert!(screen.take_outro_complete());
    assert!(screen.root.is_none(),"the outro unloads the screen");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_dpo_values_format_as_the_native() {
    use skate2_core::challenge::{dpo_hud::DpoHud, turn_hud::{DpoView, TurnHudView}};
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    // FormatString(ID_COMMON_NUMBER "{0:d}", int) groups through FormatNumber.
    assert_eq!(String::from_utf16(&localizer::format_string_int(&assets,"ID_COMMON_NUMBER",28212).unwrap()).unwrap(),"28,212");
    let mut dpo=hud_dpo_movie::DpoMovie::new(&assets).unwrap();
    let mut data=DpoHud::default();
    data.set_params(27971.,28212.);
    data.title="ID_COMMON_SCORE".into();
    let view=TurnHudView::Dpo(DpoView {data,leaderboard_players:3,challenge_icon:"nochallenge".into()});
    dpo.set_state(&assets,1,&view).unwrap();
    dpo.set_state(&assets,3,&view).unwrap();
    assert!(dpo.root.visible);
    let mut text=|path:&[&str]| String::from_utf16(dpo.root.named(path).unwrap().text.as_ref().unwrap()).unwrap();
    assert_eq!(text(&["mDPO","mPosHolder","mParams","numerator","labeltext"]),"27,971");
    assert_eq!(text(&["mDPO","mPosHolder","mParams","denominator","labeltext"]),"28,212");
    assert_eq!(text(&["mDPO","mPosHolder","mTitle","title_txt"]),"Score");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_script_format_arguments_as_the_native() {
    use localizer::AptArgument::{Integer, Text};
    let game=std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").unwrap());
    let mut mounts=MountSet::new();
    for spec in SKATE2_ARCHIVES.iter() {
        mounts.push(MountedBig::open(spec.path_under(&game),spec.use_external_header).unwrap());
    }
    let assets=Assets::load(&mounts).unwrap();
    let text=|label:&str,arguments:&[localizer::AptArgument]| String::from_utf16(&localizer::localized_format_string(&assets,label,arguments).unwrap()).unwrap();
    // GetLocalizedFormatString groups an integer through FormatNumber and passes a string as it is.
    assert_eq!(text("ID_COMMON_NUMBER",&[Integer(28212)]),"28,212");
    assert_eq!(text("ID_COMMON_PERCENT",&[Integer(100)]),"100%");
    assert_eq!(text("ID_PARTYPLAY_PLAYER_JOIN",&[Integer(1)]),"Player 1, jump in!");
    assert_eq!(text("ID_PARTYPLAY_PLAYERNUM",&[Text("2")]),"P2");
    assert_eq!(text("ID_PARTYPLAY_ROUND",&[Text("3")]),"Round 3!");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_skate_letter_banner_masks_its_letters_with_a_sprite() {
    let (_mounts,assets)=boot_assets();
    let mut movie=hud_throwmes_movie::ThrowDownMessageMovie::new(&assets).unwrap();
    movie.set_state(&assets,1).unwrap();
    let triangles=|batches:&[Batch]| batches.iter().map(|b|b.vertices.len()/3).sum::<usize>();
    let mut cut=false;
    for _ in 0..300 {
        movie.tick(&assets).unwrap();
        let mut batches=Vec::new();
        flatten(&assets,&movie.root,IDENTITY,Colour::default(),(1280,720),&mut batches).unwrap();
        // mSkate (character 43) masks depths 7 to 13 with sprite 8 (shape 7) at depth 6.
        let named=|clip:&Clip,name:&str| clip.children.values().find(|c|c.entry.name.as_deref()==Some(name)).cloned().unwrap();
        let holder=named(&movie.root,"mHolder");
        let skate=named(&holder.clip,"mSkate");
        let m=concatenate(concatenate(IDENTITY,holder.entry.matrix),skate.entry.matrix);
        let colour=Colour::default().append(holder.entry.colour_scale,holder.entry.colour_bias)
            .append(skate.entry.colour_scale,skate.entry.colour_bias);
        if !movie.root.visible||!holder.clip.visible||!skate.clip.visible {continue;}
        let mask=&skate.clip.children[&6];
        assert_eq!(mask.entry.clip_depth,13);
        let shape=&mask.clip.children[&1];
        let rect=mask_rect(&assets,&shape.clip,concatenate(concatenate(m,mask.entry.matrix),shape.entry.matrix)).unwrap();
        let mut whole=Vec::new();
        flatten_projected(&assets,&skate.clip,m,colour,(1280,720),&mut whole,true,None).unwrap();
        let mut expected=0;
        for (&depth,child) in &skate.clip.children {
            if depth==6 {continue;}
            let mut part=Vec::new();
            flatten_projected(&assets,&child.clip,concatenate(m,child.entry.matrix),
                colour.append(child.entry.colour_scale,child.entry.colour_bias),(1280,720),&mut part,true,None).unwrap();
            if (7..=13).contains(&depth) && mask.entry.colour_scale>>24!=0 {
                let before=triangles(&part);
                occlude(&mut part,rect,(1280,720));
                cut|=triangles(&part)!=before;
            }
            expected+=triangles(&part);
        }
        assert_eq!(triangles(&whole),expected,"only depths 7 to 13 are cut by the mask");
    }
    // The outro carries the letter panel under the mask rectangle, which hides it.
    assert!(cut,"the mask hides the letters as the banner leaves");
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_hidden_trick_display_plays_no_multiplier_sound() {
    use skate2_core::ui::{trick_backend::PhysicsFrame, trick_display::DisplayTuning};
    let (_mounts,assets)=boot_assets();
    let mut score=score_movie::ScoreMovie::new(&assets).unwrap();
    let tuning=DisplayTuning { allowed:true, nollie_delay:140, jolt_threshold:0.73, jolt_distance:50.,jolt_speed:9.98,jolt_recovery:0.3258 };
    let frame=|multiplier| PhysicsFrame { sequence_begun:true, hud_score:123, multiplier, ..Default::default() };
    // S.K.A.T.E. hides "main"'s TrickDisplayHUD; its Update (0x826bda40) does nothing while hidden.
    score.update(&assets,&[frame(2.)],tuning,Some(false)).unwrap();
    assert!(score.sounds.is_empty());
    score.update(&assets,&[frame(3.)],tuning,None).unwrap();
    assert!(score.sounds.is_empty());
    // Shown again, it reports the multiplier change it missed.
    score.update(&assets,&[frame(3.)],tuning,Some(true)).unwrap();
    assert_eq!(score.sounds,vec!["FE_SOUND_MULTIPLYER_3"]);
}

#[test]
#[ignore = "requires external SKATE2_GAME_ROOT"]
fn original_receipt_lists_the_cart_and_scrolls_its_window() {
    use skate2_core::ui::edit_skater::{RECEIPT_OPTIONS, ReceiptLine, ReceiptView};
    let (_mounts,assets)=boot_assets();
    let lines=(0..9).map(|i| ReceiptLine {component:"ID_CAC_COMPONENT_TYPE_OUTERTORSO".into(),
        description_candidates:Vec::new(),price:25,kept:i!=3}).collect();
    let mut view=ReceiptView {list:false,option:0,row:0,lines,cash:300,subtotal:200,balance:100};
    let text=|clip:&mut clips::Clip,path:&[&str]| clip.named(path).unwrap().text.clone().map(|t| String::from_utf16_lossy(&t));
    let mut screen=receipt_movie::ReceiptMovie::default();
    screen.update(&assets,Some(&view)).unwrap();
    let root=screen.root.as_mut().unwrap();
    assert_eq!(text(root,&["mReceipt","titleText"]),Some(assets.localize("ID_CAC_RECEIPT_TITLE").unwrap()));
    assert_eq!(text(root,&["mReceipt","mCash","labelText"]).as_deref(),Some("$300"));
    assert_eq!(text(root,&["mReceipt","mSubtotal","labelText"]).as_deref(),Some("$200"));
    assert_eq!(text(root,&["mReceipt","mBalance","labelText"]).as_deref(),Some("$100"));
    // The picker holds the three options; the list shows the first eight of nine lines.
    for (i,label) in RECEIPT_OPTIONS.iter().enumerate() {
        assert_eq!(text(root,&["mReceipt","mMenu","mMenuPicker","mItemGroup",&format!("item{i}"),"itemText"]),
            Some(assets.localize(label).unwrap()));
    }
    let component=assets.localize("ID_CAC_COMPONENT_TYPE_OUTERTORSO").unwrap();
    assert!(text(root,&["mReceipt","mItem0","nameText"]).unwrap().starts_with(&component));
    assert_eq!(text(root,&["mReceipt","mItem0","amountText"]).as_deref(),Some("$25"));
    assert!(root.named(&["mReceipt","mItem7"]).unwrap().visible);
    assert_eq!(text(root,&["mReceipt","mButtonmenu","mText0"]),Some(assets.localize("ID_COMMON_SELECT").unwrap()));
    // Edit Cart at the last line: the picker hides, the window moves one line on and that line
    // takes the highlight's text colour; B and X are the button help.
    view.list=true;
    view.row=8;
    screen.update(&assets,Some(&view)).unwrap();
    let root=screen.root.as_mut().unwrap();
    assert!(!root.named(&["mReceipt","mMenu"]).unwrap().visible);
    let colour=|clip:&mut clips::Clip,path:&[&str]| clip.named(path).unwrap().properties.get("text_colour").cloned();
    assert_eq!(colour(root,&["mReceipt","mItem7","nameText"]),Some(0xff22_1c0au32.to_string()));
    assert_eq!(colour(root,&["mReceipt","mItem2","nameText"]),Some(0xffd8_c279u32.to_string()));
    assert_eq!(text(root,&["mReceipt","mButtonmenu","mText0"]),Some(assets.localize("ID_COMMON_BACK").unwrap()));
    assert_eq!(text(root,&["mReceipt","mButtonmenu","mText1"]),Some(assets.localize("ID_CAC_RECEIPT_ITEMTOGGLE").unwrap()));
    for _ in 0..20 {screen.tick(&assets).unwrap();}
    {let mut draws=Vec::new(); flatten(&assets,screen.root.as_ref().unwrap(),IDENTITY,Colour::default(),(1280,720),&mut draws).unwrap(); assert!(!draws.is_empty(),"the receipt draws");}
    screen.update(&assets,None).unwrap();
    for _ in 0..20 {screen.tick(&assets).unwrap();}
    assert!(screen.root.is_none(),"the outro unloads the screen");
}
