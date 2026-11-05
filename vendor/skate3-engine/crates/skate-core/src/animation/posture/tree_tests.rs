use crate::animation::{
    playback_attributes::AttributeMirror,
    playback_clip::PlaybackClip,
    playback_tree::{Evaluation, PlaybackTree, PoseCommand},
    posture::PosturePose,
};
use std::sync::Arc;

#[test]
fn posture_commands_stay_inside_bind_and_mirror_layers() {
    for posture in [
        None,
        Some(PosturePose::Stiff),
        Some(PosturePose::Slouch),
        Some(PosturePose::Buff),
    ] {
        let mut tree = PlaybackTree::BindPose {
            motion: Box::new(PlaybackTree::Clip {
                name: "motion".into(),
                clip: PlaybackClip::new(31.0, 30.0, 1.0, 0, Vec::new()),
            }),
            posture,
            board_backwards: true,
            mirror_modes: vec![2, 1],
            attribute_mirror: Arc::new(AttributeMirror(Vec::new())),
        };
        tree.set_time(0.25);
        assert_eq!(tree.time(), 0.25);
        assert_eq!(tree.length(), 1.0);
        let parameters = Evaluation {
            cull_threshold: 0.0,
            update_history: false,
        };
        let mut commands = Vec::new();
        assert!(!tree.evaluate(parameters, false, &mut commands).unwrap());
        assert!(commands.is_empty());
        //Cloning preserves the inner posture selection through tree ownership changes.
        assert!(
            tree.clone()
                .evaluate(parameters, true, &mut commands)
                .unwrap()
        );
        assert!(matches!(commands.remove(0), PoseCommand::Clip { .. }));
        let mut expected = Vec::new();
        if let Some(pose) = posture {
            expected.push(PoseCommand::Pose {
                name: pose.name().into(),
            });
            expected.push(PoseCommand::Add { motion_is_a: true });
        }
        expected.extend([
            PoseCommand::Pose {
                name: "RIG_TPOSE".into(),
            },
            PoseCommand::Add { motion_is_a: true },
            PoseCommand::Pose {
                name: "BOARD_BACKWARDS".into(),
            },
            PoseCommand::Add { motion_is_a: false },
            PoseCommand::Pose {
                name: "BOARD_BACKWARDS_IK".into(),
            },
            PoseCommand::Add { motion_is_a: true },
            PoseCommand::Mirror { trajectory_mode: 2 },
            PoseCommand::Mirror { trajectory_mode: 1 },
        ]);
        assert_eq!(commands, expected);
    }
}
