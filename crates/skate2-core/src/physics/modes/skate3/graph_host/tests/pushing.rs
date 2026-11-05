use super::*;
use crate::skate3_native::graph_host::pushing_settings::{PushTreeAttribute, PushTreeMetadata, PushTreeSource};
use skate3_core::{
    point_graph::PointGraph,
    riding::{
        push_animation::{PushAnimationCurves, PushBlendParameters},
        push_behaviors::{PushAttributes, PushClipMetrics},
    },
};
use skate_data::{collections::Collections, state_graph::GraphAttribute};
fn settings() -> PushingSettings {
    let curve = PointGraph {
        x: std::array::from_fn(|i| i as f32),
        y: [1.0; 8],
    };
    let regular = PushAttributes::from_clips([
        PushClipMetrics {
            length: 1.0,
            begin_velocity: 0.0,
            end_velocity: 4.0,
        },
        PushClipMetrics {
            length: 2.0,
            begin_velocity: 4.0,
            end_velocity: 8.0,
        },
        PushClipMetrics {
            length: 1.0,
            begin_velocity: 0.0,
            end_velocity: 1.0,
        },
        PushClipMetrics {
            length: 2.0,
            begin_velocity: 4.0,
            end_velocity: 5.0,
        },
    ]);
    let mongo = PushAttributes::from_clips(regular.clips.map(|mut c| {
        c.begin_velocity += 2.0;
        c.end_velocity += 5.0;
        c
    }));
    PushingSettings {
        curves: PushAnimationCurves {
            button_time_max: curve,
            button_time_to_dv: curve,
            blend_speed_over_frames: curve,
            blend_acc_over_frames: curve,
        },
        teleport_window: 0.5,
        maximum_holding_acceleration: 4.0,
        out_speed_weight: 0.4,
        maximum_out_factor: 0.659,
        regular,
        mongo,
    }
}
fn state() -> PushState {
    let unset = PushBlendParameters {
        hstr_vel_b: -1.0,
        lstr_vel_b: -1.0,
        vel_e: -1.0,
    };
    PushState {
        out_factor: 0.375,
        current_push_dv: 2.5,
        current: unset,
        target: unset,
        continue_push: false,
    }
}
fn context<'a>(
    settings: &'a PushingSettings,
    state: &'a mut PushState,
    intents: &'a IntentMap,
) -> PushContext<'a> {
    PushContext {
        settings,
        shared: state,
        motion_intents: intents,
        forward_speed: 2.0,
        delta_seconds: 1.0,
        time_since_teleport: 1.0,
        is_switch: false,
        foot_frame: None,
    }
}
#[test]
fn authored_defaults_match_tu3_factories() {
    let attributes = |name: &str| {
        vec![GraphAttribute {
            name: "name".into(),
            text: name.into(),
            boolean_byte: 0,
            float_bits: 0,
        }]
    };
    for (name, expected) in [
        (
            "PushCycle",
            PushOperation::PushCycle {
                new_push_name: "LeftPush".into(),
            },
        ),
        (
            "PushOut",
            PushOperation::PushOut {
                is_right_foot: true,
            },
        ),
        (
            "SetPushCoefs",
            PushOperation::SetPushCoefs {
                on_first_update_only: false,
            },
        ),
        (
            "ComputeRepushDeadline",
            PushOperation::ComputeRepushDeadline {
                regular_attributes: true,
            },
        ),
    ] {
        assert_eq!(
            PushOperation::from_attributes(&Attributes::new(&attributes(name))),
            Some(expected)
        );
    }
}
#[test]
fn first_update_only_blends_once_per_activation_and_publishes_in_source_order() {
    let settings = settings();
    let mut state = state();
    let intents = IntentMap::new();
    let mut context = context(&settings, &mut state, &intents);
    context.shared.target = PushBlendParameters {
        hstr_vel_b: 0.25,
        lstr_vel_b: 0.5,
        vel_e: 0.75,
    };
    let mut node = PushInstance::new(PushOperation::SetPushCoefs {
        on_first_update_only: true,
    });
    let mut sink = Vec::new();
    node.begin(&mut context, &mut sink);
    assert!(sink.is_empty());
    node.update(&mut context, &mut sink).unwrap();
    assert_eq!(
        sink.iter().map(|a| a.name).collect::<Vec<_>>(),
        ["HStr_Vel_B", "LStr_Vel_B", "Vel_E"]
    );
    assert!(sink.iter().all(|a| a.normalized && a.sequence_id == -1));
    let first = sink.clone();
    context.shared.target.vel_e = 0.1;
    node.update(&mut context, &mut sink).unwrap();
    assert_eq!(sink, first);
    node.begin(&mut context, &mut sink);
    node.update(&mut context, &mut sink).unwrap();
    assert_eq!(sink[2].value, 0.1);
}
#[test]
fn target_and_out_factor_select_attributes_using_switch_state() {
    let settings = settings();
    let mut state = state();
    let intents = IntentMap::new();
    let mut context = context(&settings, &mut state, &intents);
    let mut sink = Vec::new();
    let mut target = PushInstance::new(PushOperation::ComputeTargetCoefsFromSpeedAndStrength {
        regular_attributes: true,
    });
    target.update(&mut context, &mut sink).unwrap();
    assert_eq!(context.shared.target, settings.regular.target(2.0, 2.5));
    context.is_switch = true;
    target.update(&mut context, &mut sink).unwrap();
    assert_eq!(context.shared.target, settings.mongo.target(2.0, 2.5));
    assert_ne!(
        settings.regular.target(2.0, 2.5),
        settings.mongo.target(2.0, 2.5)
    );
    let mut out = PushInstance::new(PushOperation::ComputeRepushDeadline {
        regular_attributes: true,
    });
    out.begin(&mut context, &mut sink);
    out.update(&mut context, &mut sink).unwrap();
    assert_eq!(context.shared.out_factor, 0.375);
    out.end(&mut context);
    assert_eq!(
        context.shared.out_factor,
        settings.mongo.out_factor(2.0, 2.5, 0.4, 0.659)
    );
    assert!(sink.is_empty());
}
#[test]
fn push_out_projects_actual_selected_foot_and_publishes_only_on_begin() {
    let settings = settings();
    let mut state = state();
    let intents = IntentMap::new();
    let mut context = context(&settings, &mut state, &intents);
    context.foot_frame = Some(PushFootFrame {
        left_foot: [2.0, 3.0, 4.0, 0.0],
        right_foot: [5.0, 6.0, 7.0, 0.0],
        deck_position: [1.0, 1.0, 1.0, 0.0],
        deck_y: [0.0, 1.0, 0.0, 0.0],
        deck_z: [0.0, 0.0, 1.0, 0.0],
        skateboard_flipped: true,
    });
    let mut node = PushInstance::new(PushOperation::PushOut {
        is_right_foot: true,
    });
    let mut sink = Vec::new();
    node.begin(&mut context, &mut sink);
    assert_eq!(
        sink,
        [
            SettableAttribute {
                name: "OutDistanceX",
                value: -6.0,
                normalized: false,
                sequence_id: -1
            },
            SettableAttribute {
                name: "OutDistanceY",
                value: 5.0,
                normalized: false,
                sequence_id: -1
            }
        ]
    );
    context.foot_frame = None;
    node.update(&mut context, &mut sink).unwrap();
    node.end(&mut context);
    assert_eq!(sink[0].value, -6.0);
}
#[test]
fn settable_sink_updates_first_matching_name_and_sequence_without_reordering() {
    let mut sink = vec![
        SettableAttribute {
            name: "Vel_E",
            value: 0.1,
            normalized: true,
            sequence_id: 7,
        },
        SettableAttribute {
            name: "Vel_E",
            value: 0.2,
            normalized: true,
            sequence_id: -1,
        },
        SettableAttribute {
            name: "Vel_E",
            value: 0.3,
            normalized: true,
            sequence_id: -1,
        },
    ];
    sink.set_attribute(SettableAttribute {
        name: "Vel_E",
        value: 3.0,
        normalized: false,
        sequence_id: -1,
    });
    assert_eq!(
        sink.iter().map(|a| a.value).collect::<Vec<_>>(),
        [0.1, 3.0, 0.3]
    );
    assert!(!sink[1].normalized);
}
struct Trees {
    requested: Vec<String>,
    missing_end: bool,
}
impl PushTreeSource for Trees {
    fn metadata(&mut self, name: &str, mask: u32) -> Result<PushTreeMetadata, String> {
        assert_eq!(mask, 15);
        self.requested.push(name.into());
        let mut attributes = vec![
            PushTreeAttribute {
                name: "HStr_Vel_B".into(),
                first_payload: 1.0,
            },
            PushTreeAttribute {
                name: "Unrelated".into(),
                first_payload: 50.0,
            },
            PushTreeAttribute {
                name: "LStr_Vel_B".into(),
                first_payload: 2.0,
            },
        ];
        if !self.missing_end {
            attributes.push(PushTreeAttribute {
                name: "Vel_E".into(),
                first_payload: 6.0,
            });
        }
        Ok(PushTreeMetadata {
            length: 0.5,
            attributes,
        })
    }
}
fn collections() -> Collections {
    let mut fields = Vec::new();
    for name in [
        "pushing_usemaxpushfromteleporttime",
        "max_holding_acc",
        "dynamic_out_factor_speed_vs_acc",
        "last_push_out_time",
    ] {
        fields.push(format!(
            r#""{name}":{{"type":"EA::Reflection::Float","data":"3F000000"}}"#
        ));
    }
    let data = (0..20)
        .map(|i| format!("{:08X}", (i as f32).to_bits()))
        .collect::<String>();
    for name in [
        "button_time_max",
        "button_time_to_dv",
        "blend_speed_over_frames",
        "blend_acc_over_frames",
    ] {
        fields.push(format!(r#""{name}":{{"type":"Graph","data":"{data}"}}"#));
    }
    let json = format!(
        r#"{{"version":1,"collections":[{{"class":"anim_motion","key":"pushing","parent":"","fields":{{{}}},"source":"test","sha256":"test"}}]}}"#,
        fields.join(",")
    );
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skate-pushing-test-{}-{unique}",
        std::process::id()
    ));
    let stock = root.join("private/stock");
    std::fs::create_dir_all(&stock).unwrap();
    let file = stock.join("skater-collections.json");
    std::fs::write(&file, json).unwrap();
    let result = Collections::load(&root);
    std::fs::remove_file(file).unwrap();
    std::fs::remove_dir(stock).unwrap();
    std::fs::remove_dir(root.join("private")).unwrap();
    std::fs::remove_dir(root).unwrap();
    result.unwrap()
}
#[test]
fn metadata_loader_uses_all_eight_authored_names_and_last_matching_attribute() {
    let mut trees = Trees {
        requested: vec![],
        missing_end: false,
    };
    let settings = PushingSettings::load(&collections(), &mut trees).unwrap();
    assert_eq!(
        trees.requested,
        [
            "R_PUSHLSP_HSTR_N_0_CYC1",
            "R_PUSHHSP_HSTR_N_0_CYC1",
            "R_PUSHLSP_LSTR_N_0_CYC1",
            "R_PUSHHSP_LSTR_N_0_CYC1",
            "R_PUSHLSP_HSTR_MONGO_0_CYC1",
            "R_PUSHHSP_HSTR_MONGO_0_CYC1",
            "R_PUSHLSP_LSTR_MONGO_0_CYC1",
            "R_PUSHHSP_LSTR_MONGO_0_CYC1"
        ]
    );
    assert_eq!(settings.regular.clips[0].begin_velocity, 2.0);
    assert_eq!(settings.regular.clips[0].end_velocity, 6.0);
    assert_eq!(
        settings.curves.button_time_max.x,
        [4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0]
    );
    assert_eq!(
        settings.curves.button_time_max.y,
        [12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0]
    );
}
#[test]
fn missing_clip_metadata_is_reported_without_guessed_velocity() {
    let mut trees = Trees {
        requested: vec![],
        missing_end: true,
    };
    let error = PushingSettings::load(&collections(), &mut trees).unwrap_err();
    assert!(error.contains("R_PUSHLSP_HSTR_N_0_CYC1: missing Vel_E"));
}
