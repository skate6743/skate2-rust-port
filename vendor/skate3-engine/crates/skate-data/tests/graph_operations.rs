use skate_data::state_graph::{
    GraphAttribute, GraphElement, StateGraph,
    attributes::Attributes,
    binding::{Binding, Node, OperationFactory, OperationKind},
};

fn attr(name: &str, text: &str, bits: u32, byte: u8) -> GraphAttribute {
    GraphAttribute {
        name: name.into(),
        text: text.into(),
        float_bits: bits,
        boolean_byte: byte,
    }
}

fn element(tag: &str, attributes: Vec<GraphAttribute>, children: Vec<usize>) -> GraphElement {
    GraphElement {
        source_offset: 0x40,
        tag: tag.into(),
        attributes,
        children,
    }
}

#[derive(Default)]
struct Factory {
    events: Vec<String>,
    registered: bool,
}

impl OperationFactory for Factory {
    type Instance = String;
    type Error = &'static str;

    fn create(
        &mut self,
        kind: OperationKind,
        parent: Node,
        attributes: &Attributes<'_>,
    ) -> Result<Option<Self::Instance>, Self::Error> {
        let name = attributes.text("name").unwrap().to_owned();
        self.events
            .push(format!("create:{kind:?}:{parent:?}:{name}"));
        Ok(self.registered.then_some(name))
    }

    fn add_parameter(
        &mut self,
        instance: &mut Self::Instance,
        attributes: &Attributes<'_>,
    ) -> Result<(), Self::Error> {
        self.events.push(format!(
            "param:{instance}:{}:{:08X}:{}",
            attributes.text("name").unwrap(),
            attributes.float_bits("value", 0),
            attributes.boolean_byte("value", 0)
        ));
        Ok(())
    }
}

fn source() -> StateGraph {
    StateGraph {
        elements: vec![
            element("state", vec![attr("name", "Root", 0, 0)], vec![1]),
            element("behaviour", vec![attr("name", "Drive", 0, 0)], vec![2, 3]),
            element(
                "param",
                vec![
                    attr("name", "first", 0, 0),
                    attr("value", "one", 0x3F80_0000, 7),
                ],
                vec![],
            ),
            element(
                "param",
                vec![
                    attr("name", "second", 0, 0),
                    attr("value", "two", 0x4000_0000, 9),
                ],
                vec![],
            ),
        ],
    }
}

#[test]
fn factory_receives_complete_operation_then_ordered_typed_parameters() {
    let source = source();
    let binding = Binding::from_graph(&source).unwrap();
    let mut factory = Factory {
        registered: true,
        ..Default::default()
    };
    let instances = binding
        .instantiate_operations(&source, &mut factory)
        .unwrap();
    assert_eq!(instances.operations, ["Drive"]);
    assert_eq!(
        factory.events,
        [
            "create:Behavior:State(0):Drive",
            "param:Drive:first:3F800000:7",
            "param:Drive:second:40000000:9",
        ]
    );
}

#[test]
fn missing_registration_is_a_source_located_error() {
    let source = source();
    let binding = Binding::from_graph(&source).unwrap();
    let error = binding
        .instantiate_operations(&source, &mut Factory::default())
        .unwrap_err();
    assert!(error.0.contains("byte 64"));
    assert!(
        error
            .0
            .contains("behaviour `Drive`: unregistered operation")
    );
}
