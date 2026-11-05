use skate_core::animation::commands::{
    batch::SetDataQueue,
    buffers::{BufferError, PoseBuffers},
    set_data::{CopyCommand, InternalUpdate, SetDataCommand, resolve_set_count},
};
use skate_core::animation::output::{NativeMatrix, Sqt};
use std::num::NonZeroU16;

fn matrix(value: f32) -> NativeMatrix {
    [[value; 4]; 4]
}
fn queue(buffers: PoseBuffers, limit: u16) -> SetDataQueue {
    SetDataQueue::new(buffers, NonZeroU16::new(limit).unwrap())
}
#[test]
fn partial_set_exports_whole_skeleton_and_preserves_tail() {
    let mut buffers = PoseBuffers::default();
    let source = buffers.matrices.initialized([matrix(9.0)]);
    let internal = buffers
        .matrices
        .initialized([matrix(1.0), matrix(2.0), matrix(3.0)]);
    let output = buffers.matrices.initialized([matrix(-1.0); 4]);
    let mut queue = queue(buffers, 4);
    queue
        .enqueue(
            3,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Set {
                    source: source.at(0),
                    bone_count: 1,
                },
                internal: internal.at(0),
                external: Some(output.at(0)),
            }),
            &[],
        )
        .unwrap();
    assert!(queue.flush());
    assert!(!queue.flush());
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    assert_eq!(
        done.buffers().matrices.read(output.at(0), 4).unwrap(),
        [matrix(9.0), matrix(2.0), matrix(3.0), matrix(-1.0)]
    );
}
#[test]
fn alias_writes_feed_later_records_and_final_external_export() {
    let mut buffers = PoseBuffers::default();
    let source = buffers.matrices.initialized([matrix(7.0)]);
    let scratch = buffers.matrices.initialized([matrix(0.0); 2]);
    let output = buffers.matrices.allocate(1);
    let mut queue = queue(buffers, 8);
    queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Set {
                    source: source.at(0),
                    bone_count: 1,
                },
                internal: scratch.at(0),
                external: Some(scratch.at(1)),
            }),
            &[],
        )
        .unwrap();
    queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: scratch.at(1),
                external: Some(output.at(0)),
            }),
            &[],
        )
        .unwrap();
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    assert_eq!(
        done.buffers().matrices.read(output.at(0), 1).unwrap(),
        [matrix(7.0)]
    );
}
#[test]
fn uninitialized_tail_does_not_become_a_valid_zero_pose() {
    let mut buffers = PoseBuffers::default();
    let source = buffers.matrices.initialized([matrix(8.0)]);
    let internal = buffers.matrices.allocate(2);
    let output = buffers.matrices.initialized([matrix(3.0); 2]);
    let mut queue = queue(buffers, 8);
    queue
        .enqueue(
            2,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Set {
                    source: source.at(0),
                    bone_count: 1,
                },
                internal: internal.at(0),
                external: Some(output.at(0)),
            }),
            &[],
        )
        .unwrap();
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    assert_eq!(
        done.buffers().matrices.read(output.at(0), 2),
        Err(BufferError::UninitializedBone { bone: 1 })
    );
    assert_eq!(
        done.buffers().matrices.read(output.at(0), 1).unwrap(),
        [matrix(8.0)]
    );
}
#[test]
fn sqt_copy_preserves_every_float_bit_and_nonzero_bone_offset() {
    let mut buffers = PoseBuffers::default();
    let value = Sqt {
        scale: [f32::from_bits(0x7fc12345), -0.0, 3.0, 4.0],
        rotation: [5.0; 4],
        translation: [6.0; 4],
    };
    let source = buffers.sqt.initialized([value]);
    let output = buffers.sqt.allocate(3);
    let mut queue = queue(buffers, 1);
    queue
        .enqueue(
            1,
            SetDataCommand::Sqt(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: source.at(0),
                external: Some(output.at(1)),
            }),
            &[],
        )
        .unwrap();
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    let copied = done.buffers().sqt.read(output.at(1), 1).unwrap()[0];
    assert_eq!(
        copied.scale.map(f32::to_bits),
        value.scale.map(f32::to_bits)
    );
    assert_eq!(copied.rotation, value.rotation);
    assert_eq!(copied.translation, value.translation);
    assert!(done.buffers().sqt.read(output.at(0), 1).is_err());
}
#[test]
fn capacity_boundary_and_dependency_preserve_producer_before_consumer() {
    let mut buffers = PoseBuffers::default();
    let input = buffers.matrices.initialized([matrix(42.0)]);
    let intermediate = buffers.matrices.allocate(1);
    let output = buffers.matrices.allocate(1);
    let mut queue = queue(buffers, 1);
    let first = queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: input.at(0),
                external: Some(intermediate.at(0)),
            }),
            &[None],
        )
        .unwrap();
    let second = queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: intermediate.at(0),
                external: Some(output.at(0)),
            }),
            &[Some(first), None, Some(first)],
        )
        .unwrap();
    assert_ne!(first, second);
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    assert_eq!(
        done.buffers().matrices.read(output.at(0), 1).unwrap(),
        [matrix(42.0)]
    );
}
#[test]
fn prepare_and_cancel_never_execute_copies() {
    let mut buffers = PoseBuffers::default();
    let source = buffers.matrices.initialized([matrix(9.0)]);
    let output = buffers.matrices.initialized([matrix(1.0)]);
    let mut queue = queue(buffers, 1);
    queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: source.at(0),
                external: Some(output.at(0)),
            }),
            &[],
        )
        .unwrap();
    queue.flush();
    assert_eq!(
        queue.cancel().matrices.read(output.at(0), 1).unwrap(),
        [matrix(1.0)]
    );
}
#[test]
fn unresolved_partial_overlap_fails_without_undoing_prior_set() {
    let mut buffers = PoseBuffers::default();
    let source = buffers.matrices.initialized([matrix(8.0)]);
    let internal = buffers.matrices.initialized([matrix(0.0); 3]);
    let mut queue = queue(buffers, 1);
    queue
        .enqueue(
            2,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Set {
                    source: source.at(0),
                    bone_count: 1,
                },
                internal: internal.at(0),
                external: Some(internal.at(1)),
            }),
            &[],
        )
        .unwrap();
    let failure = match queue.submit().complete() {
        Ok(_) => panic!("overlap accepted"),
        Err(e) => e,
    };
    assert_eq!(failure.cause, BufferError::UnsupportedPartialOverlap);
    assert_eq!(
        failure.buffers.matrices.read(internal.at(0), 3).unwrap(),
        [matrix(8.0), matrix(0.0), matrix(0.0)]
    );
}
#[test]
fn latest_enqueued_skeleton_count_applies_to_every_command_in_batch() {
    let mut buffers = PoseBuffers::default();
    let input = buffers.matrices.initialized([matrix(1.0), matrix(2.0)]);
    let output = buffers.matrices.allocate(2);
    let mut queue = queue(buffers, 3);
    let first = queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: input.at(0),
                external: Some(output.at(0)),
            }),
            &[],
        )
        .unwrap();
    queue
        .enqueue(
            2,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: input.at(0),
                external: None,
            }),
            &[Some(first)],
        )
        .unwrap();
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    assert_eq!(
        done.buffers().matrices.read(output.at(0), 2).unwrap(),
        [matrix(1.0), matrix(2.0)]
    );
    assert_eq!(resolve_set_count(0, 258), 2);
    assert_eq!(resolve_set_count(0, 256), 0);
    assert_eq!(resolve_set_count(255, 20), 255);
}

#[test]
fn foreign_buffer_and_job_handles_are_rejected() {
    let mut foreign = PoseBuffers::default();
    let foreign_pose = foreign.matrices.initialized([matrix(9.0)]);
    let mut foreign_queue = queue(foreign, 2);
    let foreign_job = foreign_queue
        .enqueue(
            1,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Preserve,
                internal: foreign_pose.at(0),
                external: None,
            }),
            &[],
        )
        .unwrap();
    let mut buffers = PoseBuffers::default();
    let local = buffers.matrices.initialized([matrix(1.0)]);
    assert_eq!(
        buffers.matrices.read(foreign_pose.at(0), 1),
        Err(BufferError::UnknownBuffer)
    );
    let mut own_queue = queue(buffers, 2);
    let command = SetDataCommand::Matrix(CopyCommand {
        update: InternalUpdate::Preserve,
        internal: local.at(0),
        external: None,
    });
    assert_eq!(
        own_queue.enqueue(1, command, &[Some(foreign_job)]),
        Err(skate_core::animation::commands::batch::QueueError::UnavailableDependency)
    );
    own_queue.enqueue(1, command, &[]).unwrap();
    assert!(own_queue.submit().complete().is_ok());
}

#[test]
fn full_export_count_is_not_truncated_to_the_set_byte() {
    let mut buffers = PoseBuffers::default();
    let internal = buffers
        .matrices
        .initialized((0..256).map(|i| matrix(i as f32)));
    let empty_source = buffers.matrices.allocate(0);
    let output = buffers.matrices.allocate(256);
    let mut queue = queue(buffers, 1);
    queue
        .enqueue(
            256,
            SetDataCommand::Matrix(CopyCommand {
                update: InternalUpdate::Set {
                    source: empty_source.at(usize::MAX),
                    bone_count: 0,
                },
                internal: internal.at(0),
                external: Some(output.at(0)),
            }),
            &[],
        )
        .unwrap();
    let done = queue
        .submit()
        .complete()
        .unwrap_or_else(|e| panic!("{:?}", e.cause));
    let values = done.buffers().matrices.read(output.at(0), 256).unwrap();
    assert_eq!(values[0], matrix(0.0));
    assert_eq!(values[255], matrix(255.0));
}
