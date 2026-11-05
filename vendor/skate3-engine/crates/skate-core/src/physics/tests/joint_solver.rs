use super::*;

#[test]
fn raw_layout_matches_tu3_strides() {
    assert_eq!(core::mem::size_of::<RetailJointJacobian>(), 384);
    assert_eq!(core::mem::size_of::<RetailJointReactionBlock>(), 64);
}

#[test]
fn reaction_bridge_preserves_contact_only_position_corrections() {
    let mut reactions = RetailReactionCorrections {
        linear_displacement: crate::math::Vector3::new(1.0, 2.0, 3.0),
        position_displacement: crate::math::Vector3::new(4.0, 5.0, 6.0),
        angular_displacement: crate::math::Vector3::new(7.0, 8.0, 9.0),
        orientation_displacement: crate::math::Vector3::new(10.0, 11.0, 12.0),
    };
    let mut block = RetailJointReactionBlock::from_reactions(reactions);
    block.words[0] = 13.0f32.to_bits();
    block.words[9] = 14.0f32.to_bits();
    block.write_velocity_reactions(&mut reactions);

    assert_eq!(
        reactions.linear_displacement,
        crate::math::Vector3::new(13.0, 2.0, 3.0)
    );
    assert_eq!(
        reactions.angular_displacement,
        crate::math::Vector3::new(7.0, 14.0, 9.0)
    );
    assert_eq!(
        reactions.position_displacement,
        crate::math::Vector3::new(4.0, 5.0, 6.0)
    );
    assert_eq!(
        reactions.orientation_displacement,
        crate::math::Vector3::new(10.0, 11.0, 12.0)
    );
}
