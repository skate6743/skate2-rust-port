use skate_core::riding::grounded::mode::ProcessedMode;

#[test]
fn unsupported_mode_retains_last_valid_attributes_but_publishes_request() {
    let attributes = ["zero", "one", "two", "three", "four"];
    let mut mode = ProcessedMode {
        requested_mode: 99,
        selected_attributes: "external initial binding",
    };
    mode.update(u32::MAX, &attributes);
    assert_eq!(mode.selected_attributes, "external initial binding");
    for (index, selected) in attributes.iter().enumerate() {
        mode.update(index as u32, &attributes);
        assert_eq!(mode.selected_attributes, *selected);
        mode.update(5, &attributes);
        assert_eq!(mode.requested_mode, 5);
        assert_eq!(mode.selected_attributes, *selected);
    }
}
