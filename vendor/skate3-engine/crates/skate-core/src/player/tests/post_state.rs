use super::*;

#[derive(Default)]
struct Services {
    calls: u32,
}

impl PostStateServices for Services {
    fn update_current_state_vtable_12(&mut self) {
        self.calls += 1;
    }
}

#[test]
fn dispatches_the_current_state_post_state_slot_once() {
    let mut services = Services::default();

    run_post_state(&mut services);

    assert_eq!(services.calls, 1);
}
