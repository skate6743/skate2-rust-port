use super::*;
fn hit(primitive: usize) -> Option<TruckContact> {
    Some(TruckContact {
        position: [0.; 4],
        primitive,
    })
}
#[test]
fn fallback_keeps_native_contact_order_and_classifies_only_ordinary_deck() {
    let result = fallback([hit(1), hit(2)], hit(3), hit(4), [hit(5), hit(6)]).unwrap();
    assert_eq!(result.contact.primitive, 1);
    assert!(!result.deck_contact);
    let result = fallback([None; 2], hit(3), hit(4), [hit(5), hit(6)]).unwrap();
    assert_eq!(result.contact.primitive, 3);
    assert!(result.deck_contact);
    let result = fallback([None; 2], None, hit(4), [hit(5), hit(6)]).unwrap();
    assert_eq!(result.contact.primitive, 4);
    assert!(!result.deck_contact);
    let result = fallback([None; 2], None, None, [None, hit(6)]).unwrap();
    assert_eq!(result.contact.primitive, 6);
    assert!(!result.deck_contact);
    assert!(fallback([None; 2], None, None, [None; 2]).is_none());
}
