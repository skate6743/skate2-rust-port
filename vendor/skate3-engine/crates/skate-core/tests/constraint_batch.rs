use skate_core::physics::constraint_batch::compile_active;

#[test]
fn inactive_constraints_never_reach_compilation_and_survivors_keep_registration_order() {
    let records = [
        (0, [0, 8]),
        (1, [4, 0]),
        (2, [1, 2]),
        (3, [0, 4]),
        (4, [12, 4]),
    ];
    let mut called = Vec::new();
    let batch = compile_active(
        records,
        |record| record.1,
        |(id, _)| {
            assert!(![0, 2].contains(&id), "inactive row reached compiler");
            called.push(id);
            id * 10
        },
    );
    assert_eq!(called, [1, 3, 4]);
    assert_eq!(batch, [10, 30, 40]);
}

#[test]
fn each_batch_recomputes_eligibility_after_endpoint_state_changes() {
    let mut states = [4, 0];
    assert_eq!(compile_active([7], |_| states, |id| id), [7]);
    states[0] = 8;
    let sleeping: Vec<u32> = compile_active([7], |_| states, |_| panic!("sleeping row compiled"));
    assert!(sleeping.is_empty());
    states[1] = 4;
    assert_eq!(compile_active([7], |_| states, |id| id), [7]);
}
