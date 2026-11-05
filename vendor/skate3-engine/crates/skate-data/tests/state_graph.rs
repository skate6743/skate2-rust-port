use skate_data::state_graph::StateGraph;

fn element(tag: &str, attributes: &[(&str, &str, u32, u8)], children: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(tag.bytes().chain([0]));
    bytes.extend((attributes.len() as u32).to_be_bytes());
    for (key, text, bits, flag) in attributes {
        bytes.extend(key.bytes().chain([0]));
        bytes.extend(text.bytes().chain([0]));
        bytes.extend(bits.to_be_bytes());
        bytes.push(*flag);
    }
    bytes.extend((children.len() as u32).to_be_bytes());
    for child in children {
        bytes.extend(child);
    }
    bytes
}

#[test]
fn preserves_tree_order_duplicate_attributes_and_encoded_values() {
    let grandchild = element("condition", &[], &[]);
    let first = element("expression", &[("op", "and", 0, 0)], &[grandchild]);
    let second = element(
        "behaviour",
        &[("value", "0", 0x80000000, 2), ("value", "1", 0x7fc00001, 0)],
        &[],
    );
    let bytes = element("state", &[("name", "Root", 0, 0)], &[first, second]);
    let graph = StateGraph::decode(&bytes).unwrap();
    assert_eq!(
        graph
            .elements
            .iter()
            .map(|e| e.tag.as_str())
            .collect::<Vec<_>>(),
        ["state", "expression", "condition", "behaviour"]
    );
    assert_eq!(graph.elements[0].children, [1, 3]);
    assert_eq!(graph.elements[1].children, [2]);
    assert_eq!(graph.elements[3].attributes[0].float_bits, 0x80000000);
    assert_eq!(graph.elements[3].attributes[0].boolean_byte, 2);
    assert_eq!(graph.elements[3].attributes[1].float_bits, 0x7fc00001);
    assert_eq!(graph.elements[3].attributes[1].text, "1");
    assert!(
        graph
            .elements
            .windows(2)
            .all(|e| e[0].source_offset < e[1].source_offset)
    );
}

#[test]
fn rejects_truncation_and_trailing_data_without_partial_graph() {
    let bytes = element(
        "state",
        &[("name", "Root", 0, 0)],
        &[element("behaviour", &[], &[])],
    );
    for end in 0..bytes.len() {
        assert!(
            StateGraph::decode(&bytes[..end]).is_err(),
            "accepted truncation at {end}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(
        StateGraph::decode(&extra)
            .unwrap_err()
            .to_string()
            .contains("trailing")
    );
    let excessive = b"state\0\xff\xff\xff\xff";
    assert!(StateGraph::decode(excessive).is_err());
}
