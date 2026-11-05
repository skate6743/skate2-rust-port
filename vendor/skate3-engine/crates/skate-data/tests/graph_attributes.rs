use skate_data::state_graph::GraphAttribute;
use skate_data::state_graph::attributes::Attributes;

#[test]
fn binary_attributes_keep_first_record_and_separate_native_encodings() {
    let records = vec![
        GraphAttribute {
            name: "value".into(),
            text: "".into(),
            float_bits: 0x80000000,
            boolean_byte: 255,
        },
        GraphAttribute {
            name: "value".into(),
            text: "replacement".into(),
            float_bits: 0x3f800000,
            boolean_byte: 0,
        },
    ];
    let attributes = Attributes::new(&records);
    assert_eq!(attributes.text("value"), Some(""));
    assert_eq!(attributes.float_bits("value", 0), 0x80000000);
    assert_eq!(attributes.boolean_byte("value", 0), 255);
    assert_eq!(attributes.text("missing"), None);
    assert_eq!(attributes.float_bits("missing", 0x3f800000), 0x3f800000);
    assert_eq!(attributes.boolean_byte("missing", 2), 2);
}
