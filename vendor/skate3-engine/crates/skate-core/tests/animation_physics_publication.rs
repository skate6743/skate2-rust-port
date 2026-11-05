use skate_core::animation::output::physics_packet;
use skate_core::animation::output::physics_packet::{
    ActorPoseBuffers, AnimationSignal, PhysicsPosePacket, SkaterPublicationState,
};
use skate_core::animation::{
    commands::{
        batch::SetDataQueue,
        buffers::{BufferError, PoseBuffers},
    },
    output::NativeMatrix,
};
use std::num::NonZeroU16;
fn matrix(value: f32) -> NativeMatrix {
    [[value; 4]; 4]
}
fn state() -> SkaterPublicationState {
    SkaterPublicationState {
        orientation_bit31: false,
        mirrored: false,
        riding_fakie: false,
        weight_on_nose: false,
        relative_stance: 0,
        natural_stance: 0,
        request_bit16: true,
        request_bit15: true,
        air_dismount_revert_requested: true,
        air_dismount_revert_frames: 17,
        signal: Some(AnimationSignal {
            name_hash: 0,
            active: 255,
        }),
    }
}
fn packet() -> PhysicsPosePacket {
    PhysicsPosePacket {
        bone_count: 2,
        hierarchy: vec![matrix(-1.0); 3],
        local: vec![matrix(-2.0); 3],
        timestep: 0.0,
        foot_surface_ids: [35, 46],
        flags: u32::MAX,
        board_flipped: false,
        mirrored: false,
        riding_switch: false,
        riding_fakie: false,
        weight_forwards: false,
        regular_stance: false,
        air_dismount_revert_frames: 0,
    }
}
#[test]
fn publication_copies_separate_arrays_consumes_latches_and_preserves_unrelated_flags() {
    let mut buffers = PoseBuffers::default();
    let hierarchy = buffers.matrices.initialized([matrix(1.0), matrix(2.0)]);
    let local = buffers.matrices.initialized([matrix(3.0), matrix(4.0)]);
    let completed = SetDataQueue::new(buffers, NonZeroU16::new(1).unwrap())
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    let poses = ActorPoseBuffers {
        hierarchy: hierarchy.at(0),
        local: local.at(0),
    };
    let mut state = state();
    let mut packet = packet();
    assert_eq!(
        physics_packet::publish(
            &mut state,
            &completed,
            &poses,
            &mut packet,
            1.0 / 60.0,
            b"signup"
        ),
        Ok(17)
    );
    assert_eq!(packet.hierarchy, [matrix(1.0), matrix(2.0), matrix(-1.0)]);
    assert_eq!(packet.local, [matrix(3.0), matrix(4.0), matrix(-2.0)]);
    assert_eq!(packet.flags, u32::MAX & !((1 << 25) | (1 << 26)));
    assert_eq!(packet.foot_surface_ids, [0, 0]);
    assert_eq!(packet.timestep.to_bits(), (1.0f32 / 60.0).to_bits());
    assert!(!state.request_bit16 && !state.request_bit15 && !state.air_dismount_revert_requested);
    assert_eq!(state.signal.as_ref().unwrap().active, 0);
    // ASCII shift/add evaluated independently: no overflow fold for six bytes.
    assert_eq!(state.signal.as_ref().unwrap().name_hash, 127_919_552);
    packet.hierarchy[0] = matrix(100.0);
    assert_eq!(
        completed
            .buffers()
            .matrices
            .read(hierarchy.at(0), 1)
            .unwrap(),
        [matrix(1.0)]
    );
    physics_packet::publish(&mut state, &completed, &poses, &mut packet, 0.25, b"signup").unwrap();
    assert_eq!(packet.flags & ((1 << 27) | (1 << 23) | (1 << 28)), 0);
    assert_eq!(packet.air_dismount_revert_frames, 17);
    assert_eq!(state.air_dismount_revert_frames, 17);
}
#[test]
fn concrete_stance_truth_table_keeps_natural_relative_and_fakie_distinct() {
    let mut scratch = PoseBuffers::default();
    let empty = scratch.matrices.allocate(0);
    let completed = SetDataQueue::new(scratch, NonZeroU16::new(1).unwrap())
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    let poses = ActorPoseBuffers {
        hierarchy: empty.at(0),
        local: empty.at(0),
    };
    for reversed in [false, true] {
        for mirrored in [false, true] {
            for fakie in [false, true] {
                for nose in [false, true] {
                    let mut state = state();
                    state.orientation_bit31 = reversed;
                    state.mirrored = mirrored;
                    state.riding_fakie = fakie;
                    state.weight_on_nose = nose;
                    state.relative_stance = 1;
                    state.natural_stance = 2;
                    state.signal = None;
                    let mut packet = packet();
                    packet.bone_count = 0;
                    physics_packet::publish(&mut state, &completed, &poses, &mut packet, 0.0, b"")
                        .unwrap();
                    assert_eq!(packet.board_flipped, reversed != mirrored);
                    assert_eq!(packet.mirrored, mirrored);
                    assert_eq!(packet.riding_fakie, fakie);
                    assert_eq!(packet.weight_forwards, fakie != nose);
                    assert!(packet.riding_switch);
                    assert!(!packet.regular_stance);
                }
            }
        }
    }
}
#[test]
fn unsafe_source_extent_is_rejected_before_consuming_requests() {
    let mut buffers = PoseBuffers::default();
    let pose = buffers.matrices.allocate(2);
    let completed = SetDataQueue::new(buffers, NonZeroU16::new(1).unwrap())
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    let mut state = state();
    let mut packet = packet();
    let poses = ActorPoseBuffers {
        hierarchy: pose.at(0),
        local: pose.at(0),
    };
    assert_eq!(
        physics_packet::publish(&mut state, &completed, &poses, &mut packet, 0.1, b"signup"),
        Err(BufferError::UninitializedBone { bone: 0 })
    );
    assert!(state.request_bit16 && state.request_bit15 && state.air_dismount_revert_requested);
    assert_eq!(packet.foot_surface_ids, [35, 46]);
}
#[test]
fn signal_hash_uses_signed_bytes_nul_termination_and_high_nibble_fold() {
    assert_eq!(physics_packet::signal_hash(b""), 0);
    assert_eq!(physics_packet::signal_hash(b"a\0ignored"), 97);
    assert_eq!(physics_packet::signal_hash(&[0xff]), 0x0fff_fe1f);
    assert_eq!(physics_packet::signal_hash(&[0x80]), 0x0fff_fe60);
}

#[test]
fn completed_setdata_exports_feed_physics_publication() {
    use skate_core::animation::commands::set_data::{CopyCommand, InternalUpdate, SetDataCommand};
    let mut buffers = PoseBuffers::default();
    let hierarchy_input = buffers.matrices.initialized([matrix(12.0), matrix(13.0)]);
    let local_input = buffers.matrices.initialized([matrix(22.0), matrix(23.0)]);
    let hierarchy_actor = buffers.matrices.allocate(2);
    let local_actor = buffers.matrices.allocate(2);
    let mut queue = SetDataQueue::new(buffers, NonZeroU16::new(1).unwrap());
    queue
        .enqueue(
            2,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: hierarchy_input.at(0),
                external: Some(hierarchy_actor.at(0)),
            }),
            &[],
        )
        .unwrap();
    queue
        .enqueue(
            2,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: local_input.at(0),
                external: Some(local_actor.at(0)),
            }),
            &[],
        )
        .unwrap();
    let completed = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    let poses = ActorPoseBuffers {
        hierarchy: hierarchy_actor.at(0),
        local: local_actor.at(0),
    };
    let mut packet = packet();
    physics_packet::publish(
        &mut state(),
        &completed,
        &poses,
        &mut packet,
        1.0 / 60.0,
        b"signup",
    )
    .unwrap();
    assert_eq!(&packet.hierarchy[..2], [matrix(12.0), matrix(13.0)]);
    assert_eq!(&packet.local[..2], [matrix(22.0), matrix(23.0)]);
}
