use core::mem::size_of;

use super::*;

#[test]
fn raw_record_sizes_match_tu3_owner_strides() {
    assert_eq!(size_of::<RetailJointParametersRaw>(), 64);
    assert_eq!(size_of::<RetailJointFramesRaw>(), 80);
}

#[test]
fn stock_records_preserve_unrolled_definition_and_live_pair_order() {
    let joints = default_joint_records();
    assert_eq!(
        joints.map(|joint| (joint.definition_body_0, joint.definition_body_1)),
        [
            (BodyId::Deck, BodyId::FrontTruck),
            (BodyId::Deck, BodyId::BackTruck),
            (BodyId::FrontTruck, BodyId::RightFrontWheel),
            (BodyId::FrontTruck, BodyId::LeftFrontWheel),
            (BodyId::BackTruck, BodyId::RightBackWheel),
            (BodyId::BackTruck, BodyId::LeftBackWheel),
        ]
    );
    assert_eq!(
        joints.map(|joint| (joint.live_body_a(), joint.live_body_b())),
        [
            (BodyId::FrontTruck, BodyId::Deck),
            (BodyId::BackTruck, BodyId::Deck),
            (BodyId::RightFrontWheel, BodyId::FrontTruck),
            (BodyId::LeftFrontWheel, BodyId::FrontTruck),
            (BodyId::RightBackWheel, BodyId::BackTruck),
            (BodyId::LeftBackWheel, BodyId::BackTruck),
        ]
    );
}

#[test]
fn stock_parameter_arithmetic_matches_create_joints_words() {
    let joints = default_joint_records();
    assert_eq!(
        joints[0].parameters.words,
        [
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0x42B4_53D1,
            0,
            0,
            0x3E7A_35DD,
            0x3F80_0000,
            0x3F78_654D,
            0,
            1,
        ]
    );
    assert_eq!(
        joints[2].parameters.words,
        [
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0x497F_A9D8,
            0,
            0,
            0x3F80_0000,
            0x3F80_0000,
            3,
            0,
        ]
    );
}

#[test]
fn stock_frame_construction_matches_native_payloads() {
    let joints = default_joint_records();
    assert_eq!(
        joints[0].frames.words,
        [
            0,
            0,
            0,
            0x3F80_0000,
            0,
            0,
            0,
            0,
            0xBE41_8051,
            0xBF2E_6F8D,
            0x3E41_804F,
            0x3F2E_6F8D,
            0,
            0xBD67_6C8B,
            0x3E78_D4FD,
            0,
            0xBE41_8051,
            0xBF2E_6F8D,
            0x3E41_804F,
            0x3F2E_6F8D,
        ]
    );
    assert_eq!(
        joints[1].frames.words,
        [
            0,
            0,
            0,
            0x3F80_0000,
            0,
            0,
            0,
            0,
            0x3E41_8051,
            0x3F2E_6F8D,
            0x3E41_804F,
            0x3F2E_6F8D,
            0,
            0xBD67_6C8B,
            0xBE78_D4FD,
            0,
            0x3E41_8051,
            0x3F2E_6F8D,
            0x3E41_804F,
            0x3F2E_6F8D,
        ]
    );
    let wheel_orientation = [0x3F35_04F3, 0, 0, 0x3F35_04F3];
    assert_eq!(&joints[2].frames.words[0..4], &wheel_orientation);
    assert_eq!(&joints[2].frames.words[8..12], &wheel_orientation);
    assert_eq!(&joints[2].frames.words[16..20], &wheel_orientation);
    assert_eq!(joints[2].frames.words[14], 0x3DC2_8F5C);
    assert_eq!(joints[3].frames.words[14], 0xBDC2_8F5C);
    assert_eq!(joints[4].frames, joints[2].frames);
    assert_eq!(joints[5].frames, joints[3].frames);
}
