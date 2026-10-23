//! Bounded Ground/Air Motion Sustain expressions, evaluator8291e218.
use super::{MotionUpdateError, graph::Mask};
use crate::generated_motion_load::ids::{condition as cond, expression as expr};

/// The existing controller owns condition202, its parent and its shared cache.
/// Native Sustain predicates differ from Pre; explicit true arms perform no reads.
/// Other Sustain cases remain pending until their own source is recovered.
pub(super) fn expression(
    index: u16,
    condition: &mut impl FnMut(u16) -> Result<bool, MotionUpdateError>,
) -> Result<bool, MotionUpdateError> {
    match index {
        expr::N_POP_SHUVIT_206 | expr::LEFT_SLIDE_TO_IDLE => Ok(true), //8291e250 entries ->829221c8, no condition reads
        expr::FS_75 => condition(cond::IS_HAND_PLANTING_FS_AIR),       //8291f9cc
        expr::BS_76 => condition(cond::IS_HAND_PLANTING_BS_AIR),       //8291fa2c
        expr::FS_1021 => condition(cond::IS_HAND_PLANTING_FS_GROUND), //82920dbc: drops the Pre anticipation gate
        expr::BS_1022 => condition(cond::IS_HAND_PLANTING_BS_GROUND), //82920e1c
        expr::TO_RIDING => Ok(true),                                  //829221c8
        expr::WILL_EXPIRE_IN_0_05_NOT_WAIT => Ok(true),               //829221c8

        //829206b8/82920784 retain repeated cached stance reads, omitting intents.
        expr::HAND_PLANT_ONE_FOOT_RIGHT_INTO | expr::HAND_PLANT_ONE_FOOT_LEFT_INTO => {
            Ok(condition(cond::IS_RIDING_GOOFY)? || !condition(cond::IS_RIDING_GOOFY)?)
        }
        expr::FROM_GROUND_ANTIC
        | expr::HAND_PLANT_INTO
        | expr::WILL_EXPIRE_IN_0_15
        | expr::HAND_PLANT_ANTIC
        | expr::HAND_PLANT_OUT
        | expr::HAND_PLANT_DISMOUNT_INTO
        | expr::HAND_PLANT_DISMOUNT_OUT
        | expr::HAND_PLANT_ONE_FOOT_RIGHT_OUT
        | expr::HAND_PLANT_ONE_FOOT_LEFT_OUT => Ok(true),

        // MovingObject104: Sustain omits Pre's time-window/expiry conditions.
        // Independently extracted from8291e218; do not reuse the Pre evaluator.
        expr::MOVING_OBJECT_104 => condition(cond::IS_GRABBING_OBJECT),
        expr::MOVING_OBJECT_104_TO_LOCOMOTION_33 | expr::MOVING_OBJECT_104_TO_MOUNT_34 => {
            Ok(!condition(cond::IS_GRABBING_OBJECT)?)
        }
        expr::MVOBJ_PUSH_LEFT => condition(cond::IS_MOVING_OBJECT_FROM_NEG_110_TO_0),
        expr::MVOBJ_PUSH_RIGHT => condition(cond::IS_MOVING_OBJECT_FROM_0_TO_110),
        expr::MVOBJ_PULL_LEFT => condition(cond::IS_MOVING_OBJECT_FROM_180_TO_NEG_110),
        expr::MVOBJ_PULL_RIGHT => condition(cond::IS_MOVING_OBJECT_FROM_110_TO_180),
        expr::WILL_EXPIRE_IN_0_05
        | expr::MVOBJ_TO_PUSH_RIGHT
        | expr::IMMEDIATE
        | expr::WAIT_FOR_END
        | expr::MVOBJ_TO_PUSH_LEFT
        | expr::MVOBJ_TO_PULL_RIGHT
        | expr::MVOBJ_TO_PULL_LEFT => Ok(true),
        expr::MVOBJ_NOT_MOVING => Ok(!condition(cond::IS_MOVING_OBJECT_FROM_NEG_180_TO_180)?),
        expr::N_360_HARDFLIP_216 | expr::CRAIL_FINGER_FLIP_SHUV_1250 => Ok(true), //829221c8: independently checked Sustain defaults
        expr::N_360_INWARD_HEELFLIP_217
        | expr::SEATBELT_FINGER_FLIP_SHUV_1239
        | expr::KICKOUT_DISMOUNT_13_TO_KICKOUT_TO_AIR_14
        | expr::KICKOUT_DISMOUNT_13_TO_WIPE_OUT_4 => Ok(true), //829221c8: each case lands on the native default
        expr::OB_TIME_TO_LAND_LESS_EQUAL_0
        | expr::OUT_960
        | expr::TO_OFF_BOARD_TO_OB_AIR
        | expr::OUT_1238
        | expr::OUT_1249
        | expr::RUN_DISMOUNT_102
        | expr::IDLE_684_TO_IN_AIR_21 => Ok(true), //829221c8: independently resolved defaults
        expr::FROM_GRIND | expr::FROM_MANUAL => Ok(true), //829221c8: FromGrind/FromManual takeoff leaves
        expr::RUN_DISMOUNT_WAIT_FOR_LANDING_103_TO_RIGHT_FOOT_STRIDE_37
        | expr::RUN_DISMOUNT_102_TO_RUN_AROUND_107
        | expr::RUN_DISMOUNT_102_TO_RUN_DISMOUNT_WAIT_FOR_LANDING_103 => Ok(true), //829221c8: RunDismount exits
        expr::DISMOUNT_36 => condition(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR), //82920648: OBAir Dismount36 keeps only the filtered-state read
        expr::N_LASERFLIP_215 | expr::OUT_972 => Ok(true),                      //829221c8
        expr::CYCLE_1237 => Ok(condition(cond::HAS_INTENT_SEATBELT_GRAB_2ND)?), //8291fbd8
        expr::IDLE_685_TO_IN_AIR_21 => Ok(true), //829221c8: native default
        expr::N_360_FLIP_214 | expr::INTO_1236 | expr::COFFIN_TO_DBL_267 => Ok(true), //829221c8: native Sustain default
        expr::CYCLE_1248 => Ok(condition(cond::HAS_INTENT_CRAIL_GRAB_2ND)?), //8291fce8: independently retained intent
        expr::N_FS_360_POP_SHUVIT_213
        | expr::HAS_ANIM_ATTRIBUTE_CHRIST_EARLY_OUT
        | expr::NEITHER_GROUND_NOR_AIR
        | expr::INTO_1142 => Ok(true), //829221c8: independent Sustain switch default
        expr::WILL_EXPIRE_IN_0_25_NOT_WAIT
        | expr::N_INWARD_HEELFLIP_211
        | expr::N_360_POP_SHUVIT_212
        | expr::FOOT_PLANT_TO_WIPE_OUT
        | expr::TO_SUPERMAN => Ok(true), //829221c8: native Sustain switch default
        expr::N_HARDFLIP_210 | expr::TRUCKS_OR_DECK_IN_CONTACT_OR_NOT_AIR => Ok(true), //829221c8: native Sustain switch default
        expr::N_VARIAL_HEELFLIP_209 | expr::PUSH_FROM_FAKIE_1178_TO_IDLE_1025 => Ok(true), //829221c8: independently checked Sustain switch default
        expr::WILL_EXPIRE_IN_0
        | expr::TAKEOFF_1355_TO_DESCENT_1356
        | expr::DESCENT_1356_TO_JUMPING_INTO_MOUNT_1358
        | expr::PHYS_FILTERED_STATE_OFFBOARD
        | expr::JUMPING_INTO_MOUNT_1358
        | expr::INTO_BS_GRAB_OUT
        | expr::MOUNT_34_TO_ON_BOARD_3
        | expr::STRIDE_TO_RUN_AROUND
        | expr::OUT_644
        | expr::PHYSICS_WANTS_RUNOUT
        | expr::FLY_116
        | expr::FINISH_117
        | expr::ACCIDENTAL_DISMOUNT_17_TO_WIPE_OUT_4 => Ok(true), //829221c8: each case resolved to the native default
        expr::RIGHT_1294_TO_FROM_FAKIE_1339 => Ok(true), //829221c8: Right1294 -> FromFakie1339 guard, native default
        expr::TO_STEP_FORWARD_317 | expr::TO_STEP_BACKWARD_318 => Ok(true), //829221c8: off-board idle step guards, native default
        // Both entries independently resolve to829221c8 (li r3,1), no reads.
        // GrindOutAssist must not recheck Pre446's IsGrinding conditions in air.
        expr::LEFT_1295_TO_FROM_FAKIE_1341 | expr::GRIND_OUT_ASSIST => Ok(true),
        //82921b24 /82921cb8: stride guards keep only PhysFilteredState(offboardair),
        //then HasIntent(OffBoardAir); the Pre foot/stance tail is not re-read.
        expr::LEFT_FOOT_STRIDE_38 | expr::RIGHT_FOOT_STRIDE_37 => {
            Ok(condition(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR)?
                && condition(cond::HAS_INTENT_OFF_BOARD_AIR)?)
        }
        //829215a4..82921810: idle survives while the ordered locomotion
        //continuation is false; 82920774 is the shared true return.
        expr::IDLE_105 => Ok(!(condition(cond::LOCOMOTION_VALID)?
            && (condition(cond::IS_DISMOUNTING_2ND)?
                || condition(cond::IS_LANDING)?
                || condition(cond::IS_RUNNING_OUT)?
                || condition(cond::IS_LOCOMOTION_QUIT_NOT_ALLOWED)?
                || (condition(cond::HAS_INTENT_OBL_STEER_MAGNITUDE_GREATER_0_3)?
                    && condition(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD)?)))),
        expr::IDLE_TO_RUN_AROUND
        | expr::OFF_BOARD_WILL_EXPIRE_IN_0_1
        | expr::STANDING_JUMP_106
        | expr::LOCO_NEW_319_TO_TO_LANDING_ON_BOARD_FROM_LEFT_26
        | expr::LOCO_NEW_319_TO_TO_LANDING_ON_BOARD_FROM_RIGHT_27
        | expr::JUMPING_INTO_MOUNT_LEFT_9
        | expr::JUMPING_INTO_MOUNT_RIGHT_10
        | expr::WILL_EXPIRE_IN_0_02
        | expr::CYC_643
        | expr::COFFIN_INTO_84
        | expr::MANUAL_1027_TO_BUMPED_276
        | expr::STILL_752
        | expr::IDLE_683_TO_IN_AIR_21 => Ok(true), //829221c8: native default
        expr::PUSH_TRICK_74 => Ok(true), //829221c8
        expr::ROLLING_FORWARDS_1350 => {
            condition(cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_GREATER_0_5)
        } //829202b8
        expr::ROLLING_BACKWARDS_OR_STATIONARY_1351 => Ok(!condition(
            cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_GREATER_0_5,
        )?), //82920318
        expr::NOT_HAS_INTENT_MANUAL_GREATER_0 => Ok(true), //829221c8: native default, no condition query
        expr::CAN_LAND_ON_BOARD | expr::LANDING_ON_BOARD_28 => condition(cond::CAN_LAND_ON_BOARD), //82920450: shared case, CanLandOnBoard only
        expr::FROM_DBL_GRAB_656 | expr::FS_GRAB_OUT_TO_CYC => {
            condition(cond::HAS_INTENT_FS_GRAB_2ND)
        } //8291fae8: shared case, HasIntent(FSGrab)
        expr::FROM_DBL_GRAB_667 | expr::BS_GRAB_OUT_TO_CYC => {
            condition(cond::HAS_INTENT_BS_GRAB_2ND)
        } //8291fb38: shared case, HasIntent(BSGrab)
        expr::BRAKE => condition(cond::HAS_INTENT_MANUAL_BRAKE_2ND), //829201f8
        expr::SLIDE_OUT_EARLY
        | expr::SLIDE_OUT_MID
        | expr::OUT_TO_BRAKE
        | expr::INTO_1251_TO_OUT_1252
        | expr::INTO_1253_TO_OUT_1254 => Ok(true), //829221c8: slide transitions, native default
        expr::HAS_INTENT_GROUND_DBL_GRAB_OR_COFFIN => {
            Ok(condition(cond::HAS_INTENT_GROUND_DBL_GRAB_2ND)?
                || condition(cond::HAS_INTENT_COFFIN_2ND)?)
        } //82920e7c
        expr::NOT_HAS_INTENT_GROUND_BS_GRAB
        | expr::RIGHT_SLIDE
        | expr::NOT_HAS_INTENT_GROUND_FS_GRAB
        | expr::DBL_GRAB_INTO_1265_TO_TURNING_719
        | expr::DBL_GRAB_CYC_1266_TO_DBL_GRAB_OUT_1267
        | expr::MOVING
        | expr::STATIONARY
        | expr::TO_DBL_GRAB_OUT
        | expr::STEP_BACKWARD_313 => Ok(true), //829221c8: native default
        expr::STEP_FORWARD_309 | expr::NOT_HAS_INTENT_COFFIN => Ok(true), //829221c8: native default
        expr::STEP_FORWARD_RIGHT_310
        | expr::STEP_RIGHT_311
        | expr::STEP_BACK_RIGHT_312
        | expr::STEP_BACK_LEFT_314
        | expr::STEP_LEFT_315
        | expr::STEP_FORWARD_LEFT_316 => Ok(true), //1.02 8276ea48: native default
        expr::TURNING_LEFT_754
        | expr::RUNNING_INTO_MOUNT_FROM_RIGHT_108
        | expr::RUNNING_INTO_MOUNT_FROM_LEFT_110
        | expr::STALL_RUN_INTO_MOUNT_RIGHT_112
        | expr::STALL_RUN_INTO_MOUNT_LEFT_113
        | expr::STALL_INTO_MOUNT_114
        | expr::MOUNT_34
        | expr::SPRINTING_INTO_MOUNT_FROM_RIGHT_109
        | expr::SPRINTING_INTO_MOUNT_FROM_LEFT_111 => Ok(true), //829221c8: offboard turn/mount transitions
        expr::LOCO_NEW_319_TO_MOUNT_34 => Ok(!condition(cond::IS_DISMOUNTING_2ND)?
            && !condition(cond::IS_LANDING)?
            && !condition(cond::IS_RUNNING_OUT)?), //82921814: no quit-lock gate in Sustain
        expr::HAS_INTENT_RIGHT_AND_LEFT_PUSH => Ok(condition(cond::HAS_INTENT_RIGHT_PUSH_2ND)?
            && condition(cond::HAS_INTENT_LEFT_PUSH_2ND)?), //8292140c
        expr::TURNING_RIGHT_753
        | expr::HIPPY_JUMP_25_TO_HIPPY_RUNOUT_16
        | expr::N_VARIAL_KICKFLIP_208 => Ok(true), // native default829221c8
        expr::ROLLING_FORWARDS_OR_STATIONARY_1328 => Ok(!condition(
            cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_LESS_NEG_0_5,
        )?), //82920180
        expr::DBL_GRAB_CYC_1266 => condition(cond::HAS_INTENT_GROUND_DBL_GRAB_2ND), //82920060
        expr::N_FS_POP_SHUVIT_207 => Ok(true), //829221c8: native Sustain default, no condition read
        expr::ROLLING_BACKWARDS_1327 => {
            condition(cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_LESS_NEG_0_5)
        } //82920120: PhysicsSpeedCompare
        expr::RUN_AROUND_107 => Ok(condition(cond::LOCOMOTION_VALID)?
            && (condition(cond::IS_DISMOUNTING_2ND)?
                || condition(cond::IS_LANDING)?
                || condition(cond::IS_RUNNING_OUT)?
                || condition(cond::IS_LOCOMOTION_QUIT_NOT_ALLOWED)?
                || (condition(cond::HAS_INTENT_OBL_STEER_MAGNITUDE_GREATER_0_3)?
                    && condition(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD)?))), //82921f58
        expr::FORCE_BRAKE_2 => condition(cond::HAS_INTENT_FORCE_BRAKE), //8291ec04: ForceBrake remains intent-gated
        expr::OLLIE_ANTIC_CYCLE_1288 => condition(cond::HAS_INTENT_OLLIE_ANTICIPATION_2ND), //82920390: same OllieAnticipation intent
        expr::NOLLIE_ANTIC_CYCLE_1291 => condition(cond::HAS_INTENT_NOLLIE_ANTICIPATION_2ND), //829203f0: same NollieAnticipation intent
        expr::STATE_360_POP_SHUVIT_ANTIC_CYCLE_1289 => {
            Ok(condition(cond::HAS_INTENT_360_POP_SHUVIT_ANTICIPATION_2ND)?
                || condition(cond::HAS_INTENT_POP_SHUVIT_ANTICIPATION_2ND)?)
        } //829210e0
        expr::FS_360_POP_SHUVIT_ANTIC_CYCLE_1290 => Ok(condition(
            cond::HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION_2ND,
        )? || condition(
            cond::HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION_2ND,
        )?), //829211a8
        expr::NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1292 => Ok(condition(
            cond::HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION_2ND,
        )? || condition(
            cond::HAS_INTENT_N_POP_SHUVIT_ANTICIPATION_2ND,
        )?), //82921274
        expr::FS_NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1293 => Ok(condition(
            cond::HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION_2ND,
        )? || condition(
            cond::HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION_2ND,
        )?), //82921340
        expr::ANTIC_OUT_1029_TO_IDLE_1025
        | expr::TO_ANTIC_CYC
        | expr::FORCE_BRAKE_2_TO_WIPE_OUT_4
        | expr::ANTIC_TAIL_1183
        | expr::ANTIC_NOSE_1184 => Ok(true), //829221c8, independent native switch entries

        expr::POP_SHUVIT_191
        | expr::FS_POP_SHUVIT_192
        | expr::VARIAL_KICKFLIP_193
        | expr::VARIAL_HEELFLIP_194
        | expr::HARDFLIP_195
        | expr::INWARD_HEELFLIP_196
        | expr::STATE_360_POP_SHUVIT_197
        | expr::FS_360_POP_SHUVIT_198
        | expr::STATE_360_FLIP_199
        | expr::LASERFLIP_200
        | expr::STATE_360_HARDFLIP_201
        | expr::STATE_360_INWARD_HEELFLIP_202
        | expr::NOLLIE_203 => Ok(true), //8291e250 default ->829221c8, independent of Pre intent.
        expr::HAS_INTENT_GROUND_BS_GRAB_CYCLE => condition(cond::HAS_INTENT_GROUND_BS_GRAB_2ND), //8291ffa0: same BSGrabCyc intent
        expr::HAS_INTENT_BS_CHRIST
        | expr::OUT_665
        | expr::BS_GRAB_FINGER_FLIP_SHUV_663
        | expr::CYC_659 => Ok(true), //829221c8
        expr::IN_520
        | expr::IN_562
        | expr::IN_565
        | expr::OUT_3_1089
        | expr::OUT_3_1097
        | expr::OUT_3_1105
        | expr::OUT_1_1087
        | expr::OUT_2_1088
        | expr::OUT_1_1095
        | expr::OUT_2_1096
        | expr::OUT_1_1103
        | expr::OUT_2_1104 => Ok(true), //829221c8
        expr::HOLDING_1273 => condition(cond::HAS_INTENT_MANUAL_GREATER_0), //82920258: same Manual intent
        expr::RIGHT_SLIDE_TO_IDLE => Ok(true), //829221c8: native Sustain default; no condition reads
        expr::HAS_INTENT_GROUND_FS_GRAB_CYCLE => condition(cond::HAS_INTENT_GROUND_FS_GRAB), //82920000: GroundFSGrab state gate
        expr::TO_DBL_GRAB => Ok(
            condition(cond::HAS_INTENT_DBL_GRAB_2ND)? || condition(cond::HAS_INTENT_COFFIN_2ND)?
        ), //829208d8: DBLGrab, then Coffin

        expr::WIPE_OUT_4 => Ok(!condition(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?), //829204b0: same cached physical predicate, negated return
        expr::TELEPORT_5 => condition(cond::PHYS_FILTERED_STATE_TELEPORT_2ND), //82920588: cached physical Teleport predicate, direct return
        expr::BRACE_FOR_IMPACT_98 => condition(cond::WIPEOUT_TIME_TO_LAND_LESS_0_75), //82920528: original WipeoutTimeToLand comparison
        expr::RIGHT_FOOT_FORWARD_259 => condition(cond::IS_RIDING_GOOFY),             //8291fd48
        expr::LEFT_FOOT_FORWARD_260 => Ok(!condition(cond::IS_RIDING_GOOFY)?),        //8291fda8
        expr::TAIL_MANUAL_1176 => condition(cond::HAS_INTENT_MANUAL_LESS_0),          //829200c0
        expr::HAS_INTENT_OB_DISMOUNT
        | expr::HAS_INTENT_FS_CHRIST
        | expr::SPIN_261
        | expr::SKETCHY_262
        | expr::NOT_HAS_INTENT_MANUAL_BRAKE
        | expr::HAS_INTENT_NEW_PUSH
        | expr::OUT_3_1081
        | expr::INTO_RIGHT_ONE_FOOT_AIR
        | expr::INTO_LEFT_ONE_FOOT_AIR
        | expr::IN_AIR_21_TO_WIPE_OUT_4
        | expr::LAND_22
        | expr::OUT_1_1079
        | expr::OUT_2_1080
        | expr::CYC_648 => Ok(true), //829221c8
        expr::LATEFLIP_80 | expr::PHYS_FILTERED_STATE_AIR => {
            condition(cond::PHYS_FILTERED_STATE_AIR_2ND)
        } //8291fa8c: same Air sustain as134

        expr::ON_GROUND_24 => condition(cond::PHYS_FILTERED_STATE_GROUND_2ND), // 8291fe20
        expr::IS_ENTERING_SKITCH => condition(cond::IS_ENTERING_SKITCH),       // 8291fe80
        expr::SKITCHING_88 => condition(cond::IS_SKITCHING),                   // 8291fee0
        expr::TO_RIDING_IDLE
        | expr::SKITCH_2_HAND_272
        | expr::ABSORB_700
        | expr::SHIMMYING_LEFT_1011
        | expr::SHIMMYING_RIGHT_1012
        | expr::IDLE_1013
        | expr::SKITCH_1_HAND_FS_273
        | expr::SKITCH_1_HAND_BS_274
        | expr::ABSORB_700_TO_HOLDING_701 => Ok(true), //8291e254 switch entries all select829221c8
        expr::DISMOUNT_32 => Ok(!condition(cond::IS_LANDING)? && !condition(cond::IS_RUNNING_OUT)?), // 82921bf0: IsLanding, then IsRunningOut
        expr::OB_GROUND_8
        | expr::WILL_EXPIRE_IN_0_01
        | expr::HAS_INTENT_KICKFLIP
        | expr::OLLIE_188
        | expr::CYC_TO_OUT_HAS_TWEAK_LESS_0_2
        | expr::FINGER_FLIP
        | expr::FINGER_FLIP_VAR
        | expr::OUT_654
        | expr::OUT_TO_WIPE_OUT
        | expr::OUT_NOT_HAS_INTENT_BRAKE
        | expr::INTO_FS_GRAB_RIGHT_ONE_FOOT_AIR
        | expr::INTO_FS_GRAB_LEFT_ONE_FOOT_AIR
        | expr::INTO_BS_GRAB_RIGHT_ONE_FOOT_AIR
        | expr::INTO_BS_GRAB_LEFT_ONE_FOOT_AIR
        | expr::FS_GRAB_FINGER_FLIP_SHUV_652
        | expr::TRICK_19
        | expr::INTO_658
        | expr::HAS_INTENT_MUTE_GRAB
        | expr::HAS_INTENT_STALE_GRAB
        | expr::LEFT_248
        | expr::RIGHT_249
        | expr::HAS_INTENT_BOARD_ADJUST_UP
        | expr::HAS_INTENT_BOARD_ADJUST_DOWN
        | expr::INTO_1247
        | expr::SEQUENCE_690
        | expr::LATE_KICKFLIP_253
        | expr::LATE_HEELFLIP_254
        | expr::LATE_BACKFOOT_KICKFLIP_255
        | expr::LATE_BACKFOOT_HEELFLIP_256
        | expr::LATE_BS_SHUV_257
        | expr::LATE_FS_SHUV_258
        | expr::HAS_INTENT_OB_DISMOUNT_IN_AIR
        | expr::INTO_647
        | expr::INTO_642
        | expr::IDLE_687_TO_IN_AIR_21
        | expr::INTO_1148
        | expr::IS_RIDING_FAKIE
        | expr::WILL_EXPIRE_IN_0_1
        | expr::REST_AND_RECOVER_30
        | expr::HAS_INTENT_GROUND_BS_GRAB_INTO
        | expr::FS_GRAB_INTO_1260
        | expr::DBL_GRAB_INTO_1265
        | expr::BUMPED_276
        | expr::BUMPED_276_TO_SWITCH_1031
        | expr::MOVING_1014
        | expr::BRAKE_INTO
        | expr::STATIONARY_1015
        | expr::LEFT_SLIDE
        | expr::RIGHT_SLIDE_1168
        | expr::REVERT_BS
        | expr::REVERT_FS
        | expr::PUSH_1028
        | expr::HAS_INTENT_RIGHT_PUSH
        | expr::LEFT_1277
        | expr::IDLE_DISMOUNT_101
        | expr::FROM_FAKIE_1016
        | expr::KICK_TURN_715
        | expr::IDLE_1025_TO_SWITCH_1031
        | expr::RIGHT_1278
        | expr::LEFT_1279
        | expr::TO_WIPE_OUT
        | expr::MANUAL_1027
        | expr::CYC_TO_WIPE_OUT_TWEAK_0_4
        | expr::ANTIC_TAIL_1286
        | expr::ANTIC_NOSE_1287 => Ok(true), // 829221c8
        expr::HAS_INTENT_ANY_KICK_TURN | expr::RIGHT_1017 | expr::LEFT_1018 => {
            condition(cond::HAS_INTENT_KICK_TURN)
        } // 82920ce4
        expr::SKITCH_KICK_TURN_TO_SKITCH_CARVE => Ok(!condition(cond::HAS_INTENT_KICK_TURN)?), //82920d44: Sustain drops the first Pre intent
        expr::AGGRESSIVE_OR_LOOSE_1169 => Ok(condition(cond::IS_PRO_SKATER_LOOSE)?
            || condition(cond::IS_PRO_SKATER_AGGRESSIVE)?
            || condition(cond::IS_PRO_SKATER_MARK_GONZALES_2ND)?
            || condition(cond::IS_PRO_SKATER_DANNY_WAY)?
            || condition(cond::IS_PRO_SKATER_JERRY_HSU_2ND)?), // 82921d84
        // 8291fb88: shared Sustain case 169/326, cached HasIntent(TailGrab).
        expr::CYCLE_1143 => condition(cond::HAS_INTENT_TAIL_GRAB_2ND),
        // 829221c8: DBLGrab Cyc643 -> WipeOut4 guard, native default.
        expr::CYC_643_TO_WIPE_OUT_4 => Ok(true),
        // 829221c8: Superman, one-foot and tail-grab guards, native default.
        expr::SUPERMAN_INTO_953
        | expr::SUPERMAN_OUT_955
        | expr::HAS_ANIM_ATTRIBUTE_ONE_FOOT_EARLY_OUT
        | expr::OUT_NOT_HAS_INTENT_RIGHT_ONE_FOOT_AIR
        | expr::FOOTPLANT
        | expr::INTO_TO_ABSORB
        | expr::OUT_NOT_HAS_INTENT_LEFT_ONE_FOOT_AIR
        | expr::OUT_NOT_HAS_INTENT_NO_FOOT_AIR_WALK
        | expr::OUT_990
        | expr::INTO_1229
        | expr::INTO_1296
        | expr::INTO_1299 => Ok(true),
        // 829221c8: HippyRunout16 -> WipeOut4 guard, native default.
        expr::HIPPY_RUNOUT_16_TO_WIPE_OUT_4 => Ok(true),
        // 8291fc98: shared Sustain case 179/332, cached HasIntent(NoseGrab).
        expr::CYCLE_1149 => condition(cond::HAS_INTENT_NOSE_GRAB_2ND),
        // 8291fc38: RocketAir Cycle993 keeps its own cached HasIntent(RocketAir).
        expr::CYCLE_993 => condition(cond::HAS_INTENT_ROCKET_AIR_2ND),
        // 829221c8: RocketAir Into/Out, nose Out996 and nose foot Intos, native default.
        expr::INTO_992
        | expr::OUT_994
        | expr::OUT_996
        | expr::INTO_1240
        | expr::INTO_1302
        | expr::INTO_1305 => Ok(true),
        // 829221c8: SpawnDismount31 guard and hippy guards, native default.
        expr::SPAWN_DISMOUNT_31
        | expr::ANTICIPATION_91_TO_ANTIC_CYCLE_92
        | expr::TO_LANDING_ON_BOARD
        | expr::CYC_94_TO_WIPE_OUT_4
        | expr::INTO_93 => Ok(true),
        // 829214d8: hippy Into93 -> WipeOut4 keeps both PhysFilteredState reads, not WillExpire.
        expr::INTO_93_TO_WIPE_OUT_4 => Ok(!condition(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
            && !condition(cond::PHYS_FILTERED_STATE_OFFBOARD_2ND)?),
        // 829221c8: Mute/Stale Cyc670/677 -> WipeOut4 guard, native default.
        expr::CYC_TO_WIPE_OUT_TWEAK_0_2 => Ok(true),
        // 829221c8: Mute/Stale FingerFlipShuv and Out guards, PlantingFoot
        // Left/Right guards, native default.
        expr::PLANTING_FOOT_LEFT
        | expr::PLANTING_FOOT_RIGHT
        | expr::OUT_674
        | expr::OUT_681
        | expr::MUTE_FINGER_FLIP_SHUV
        | expr::STALE_FINGER_FLIP_SHUV => Ok(true),
        // 8291fe20: PlantingFoot guards share Sustain case 200, PhysFilteredState(ground).
        expr::PLANTING_FOOT_604
        | expr::PLANTING_FOOT_608
        | expr::PLANTING_FOOT_612
        | expr::PLANTING_FOOT_616
        | expr::PLANTING_FOOT_620 => condition(cond::PHYS_FILTERED_STATE_GROUND_2ND),
        // 8291ec64..8291f96c: Grinding18 and its children re-read their Pre
        // condition; the blunt backslash, grab Into/Out and cycle leaves take
        // the native default 829221c8.
        expr::GRINDING_18 => condition(cond::PHYS_FILTERED_STATE_GRIND_2ND), //8291ec64: PhysFilteredState(grind)
        expr::BS_FEEBLE_40 => condition(cond::IS_GRINDING_BS_FEEBLE), //8291ecc4: IsGrinding(BS_FEEBLE)
        expr::FS_FEEBLE_41 => condition(cond::IS_GRINDING_FS_FEEBLE), //8291ed20: IsGrinding(FS_FEEBLE)
        expr::BS_SMITH_42 => condition(cond::IS_GRINDING_BS_SMITH), //8291ed80: IsGrinding(BS_SMITH)
        expr::FS_SMITH_43 => condition(cond::IS_GRINDING_FS_SMITH), //8291ede0: IsGrinding(FS_SMITH)
        expr::FS_WILLY_44 => condition(cond::IS_GRINDING_FS_WILLY), //8291ee40: IsGrinding(FS_WILLY)
        expr::BS_WILLY_45 => condition(cond::IS_GRINDING_BS_WILLY), //8291eea0: IsGrinding(BS_WILLY)
        expr::FS_OVERWILLY_46 => condition(cond::IS_GRINDING_FS_OVERWILLY), //8291ef00: IsGrinding(FS_OVERWILLY)
        expr::BS_OVERWILLY_47 => condition(cond::IS_GRINDING_BS_OVERWILLY), //8291ef60: IsGrinding(BS_OVERWILLY)
        expr::BS_5_O_48 => condition(cond::IS_GRINDING_BS_5_O), //8291efc0: IsGrinding(BS_5_O)
        expr::FS_5_O_49 => condition(cond::IS_GRINDING_FS_5_O), //8291f020: IsGrinding(FS_5_O)
        expr::BS_50_50_50 => condition(cond::IS_GRINDING_BS_50_50), //8291f080: IsGrinding(BS_50_50)
        expr::FS_50_50_51 => condition(cond::IS_GRINDING_FS_50_50), //8291f0e0: IsGrinding(FS_50_50)
        expr::BS_CROOK_52 => condition(cond::IS_GRINDING_BS_CROOK), //8291f140: IsGrinding(BS_CROOK)
        expr::FS_CROOK_53 => condition(cond::IS_GRINDING_FS_CROOK), //8291f1a0: IsGrinding(FS_CROOK)
        expr::FS_OVERCROOK_54 => condition(cond::IS_GRINDING_FS_OVERCROOK), //8291f200: IsGrinding(FS_OVERCROOK)
        expr::BS_OVERCROOK_55 => condition(cond::IS_GRINDING_BS_OVERCROOK), //8291f260: IsGrinding(BS_OVERCROOK)
        expr::BS_NOSEGRIND_56 => condition(cond::IS_GRINDING_BS_NOSEGRIND), //8291f2c0: IsGrinding(BS_NOSEGRIND)
        expr::FS_NOSEGRIND_57 => condition(cond::IS_GRINDING_FS_NOSEGRIND), //8291f320: IsGrinding(FS_NOSEGRIND)
        expr::BS_SALAD_58 => condition(cond::IS_GRINDING_BS_SALAD), //8291f380: IsGrinding(BS_SALAD)
        expr::FS_SALAD_59 => condition(cond::IS_GRINDING_FS_SALAD), //8291f3e0: IsGrinding(FS_SALAD)
        expr::BS_NOSESLIDE_60 => condition(cond::IS_GRINDING_BS_NOSESLIDE), //8291f440: IsGrinding(BS_NOSESLIDE)
        expr::FS_NOSESLIDE_61 => condition(cond::IS_GRINDING_FS_NOSESLIDE), //8291f4a0: IsGrinding(FS_NOSESLIDE)
        expr::BS_NOSEBLUNT_62 => condition(cond::IS_GRINDING_BS_NOSEBLUNT), //8291f500: IsGrinding(BS_NOSEBLUNT)
        expr::FS_NOSEBLUNT_63 => condition(cond::IS_GRINDING_FS_NOSEBLUNT), //8291f560: IsGrinding(FS_NOSEBLUNT)
        expr::BS_BLUNT_64 => condition(cond::IS_GRINDING_BS_BLUNT), //8291f5c0: IsGrinding(BS_BLUNT)
        expr::FS_BLUNT_65 => condition(cond::IS_GRINDING_FS_BLUNT), //8291f61c: IsGrinding(FS_BLUNT)
        expr::FS_TAILSLIDE_66 => condition(cond::IS_GRINDING_FS_TAILSLIDE), //8291f67c: IsGrinding(FS_TAILSLIDE)
        expr::BS_TAILSLIDE_67 => condition(cond::IS_GRINDING_BS_TAILSLIDE), //8291f6dc: IsGrinding(BS_TAILSLIDE)
        expr::BS_BOARD_68 => condition(cond::IS_GRINDING_BS_BOARD), //8291f73c: IsGrinding(BS_BOARD)
        expr::GRAB_CYCLE_HAS_INTENT_GRIND_FS_GRAB => condition(cond::HAS_INTENT_GRIND_FS_GRAB_2ND), //8291f79c: HasIntent(GrindFSGrab)
        expr::GRAB_CYCLE_HAS_INTENT_GRIND_BS_GRAB => condition(cond::HAS_INTENT_GRIND_BS_GRAB_2ND), //8291f7ec: HasIntent(GrindBSGrab)
        expr::GRAB_CYCLE_HAS_INTENT_GRIND_DOUBLE_GRAB => {
            condition(cond::HAS_INTENT_GRIND_DOUBLE_GRAB_2ND)
        } //8291f84c: HasIntent(GrindDoubleGrab)
        expr::FS_BOARD_69 => condition(cond::IS_GRINDING_FS_BOARD), //8291f8ac: IsGrinding(FS_BOARD)
        expr::BS_LIP_70 => condition(cond::IS_GRINDING_BS_LIP),     //8291f90c: IsGrinding(BS_LIP)
        expr::FS_LIP_71 => condition(cond::IS_GRINDING_FS_LIP),     //8291f96c: IsGrinding(FS_LIP)
        expr::GRAB_CYCLE_DIRECT_FS_GRAB_TAIL => condition(cond::HAS_INTENT_GRIND_FS_GRAB_2ND), //8291f79c: HasIntent(GrindFSGrab)
        expr::GRAB_CYCLE_DIRECT_BS_GRAB_NOSE => condition(cond::HAS_INTENT_GRIND_BS_GRAB_2ND), //8291f7ec: HasIntent(GrindBSGrab)
        expr::GRAB_CYCLE_DIRECT_DOUBLE => condition(cond::HAS_INTENT_GRIND_DOUBLE_GRAB_2ND), //8291f84c: HasIntent(GrindDoubleGrab)
        expr::BACKSLASH
        | expr::RAIL
        | expr::INTO_HAS_INTENT_GRIND_FS_GRAB
        | expr::OUT_NOT_HAS_INTENT_GRIND_FS_GRAB
        | expr::INTO_HAS_INTENT_GRIND_BS_GRAB
        | expr::OUT_NOT_HAS_INTENT_GRIND_BS_GRAB
        | expr::INTO_HAS_INTENT_GRIND_DOUBLE_GRAB
        | expr::HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB
        | expr::NOT_HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB
        | expr::HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB
        | expr::NOT_HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB
        | expr::GRIND_DOUBLE_GRAB_OUT
        | expr::GRAB_CYCLE_DIRECT_FS
        | expr::GRAB_CYCLE_DIRECT_NOSE
        | expr::GRAB_CYCLE_DIRECT_BS
        | expr::GRAB_CYCLE_DIRECT_TAIL => Ok(true),
        // 829221c8 (li r3, 1): the rest of the switch's default cases, as the jump table
        // 8291e254 lists them.
        expr::TO_MOUNT
        | expr::IS_PRO_SKATER_MIKE_CARROLL
        | expr::WILL_EXPIRE_IN_0_02_NOT_WAIT
        | expr::IS_PRO_SKATER_MARK_GONZALES
        | expr::TO_BOARD_ADJUSTS
        | expr::CYC_TO_OUT_TWEAK_Y_ABS_LESS_0_2
        | expr::IN_PARENT_STATE_FOR_TIME_GREATER_0_1
        | expr::LAND_22_TO_ANTICIPATION_1030
        | expr::RAND_PJ_LADD_707
        | expr::RAND_DILL_710
        | expr::RAND_HSU_711
        | expr::RANDOM_COND
        | expr::WILL_EXPIRE_IN_0_25
        | expr::STILL_1171_TO_RANDOM_275
        | expr::WILL_EXPIRE_IN_0_01_NOT_WAIT
        | expr::MOTION_0_TO_TELEPORT_5
        | expr::STEP_TO_RUN_AROUND
        | expr::TRICK_19_TO_HIPPY_RUNOUT_16
        | expr::TRICK_CYC_2_TO_TRICK_OUT
        | expr::RAND_CARROLL_1_708
        | expr::PUSH_1028_TO_ANTICIPATION_1030
        | expr::TELEPORT_5_TO_MOTION_0
        | expr::TRICK_19_TO_ACCIDENTAL_DISMOUNT_17
        | expr::TRICK_CYC_1_TO_TRICK_OUT
        | expr::PUSH_TO_ANTIC
        | expr::LEFT_GROUND_TO_TRICK_CYC_1_505
        | expr::LEFT_GROUND_TO_TRICK_CYC_1_510
        | expr::PUSH_START_OR_CONTACT_TO_PUSH_END
        | expr::PUSH_CYCLE_TO_PUSH_END => Ok(true),
        expr::STILL_1171 => condition(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_0_1), //8291ff40: PhysicsSpeedCompare
        expr::DEBUG_ANIMATIONS_7 => condition(cond::IS_IN_DEBUG_ANIMATIONS_MODE), //829205e8: IsInDebugAnimationsMode
        expr::FS_GRAB_OUT_TO_IN_AIR => Ok(!condition(cond::HAS_INTENT_FS_GRAB_2ND)?), //82920860: HasIntent(FSGrab) negated
        expr::BS_GRAB_OUT_TO_IN_AIR => Ok(!condition(cond::HAS_INTENT_BS_GRAB_2ND)?), //829209b4: HasIntent(BSGrab) negated
        expr::GRAB_OUT_983_TO_CYC_670 => condition(cond::HAS_INTENT_MUTE_GRAB_2ND),   //82920a2c
        expr::GRAB_OUT_983_TO_IN_AIR_21 => Ok(!condition(cond::HAS_INTENT_MUTE_GRAB_2ND)?), //82920a88
        expr::GRAB_OUT_985_TO_CYC_677 => condition(cond::HAS_INTENT_STALE_GRAB_2ND), //82920afc
        expr::GRAB_OUT_985_TO_IN_AIR_21 => Ok(!condition(cond::HAS_INTENT_STALE_GRAB_2ND)?), //82920b5c
        expr::GRAB_OUT_1235_TO_CYCLE_1143 => condition(cond::HAS_INTENT_TAIL_GRAB_2ND), //8291fb88: shared with 169
        expr::GRAB_OUT_1235_TO_IN_AIR_21 => Ok(!condition(cond::HAS_INTENT_TAIL_GRAB_2ND)?), //82920be4
        expr::GRAB_OUT_1246_TO_CYCLE_1149 => condition(cond::HAS_INTENT_NOSE_GRAB_2ND), //8291fc98: shared with 179
        expr::GRAB_OUT_1246_TO_IN_AIR_21 => Ok(!condition(cond::HAS_INTENT_NOSE_GRAB_2ND)?), //82920c6c
        // 82920f48 / 82921014: IsRidingGoofy and HasGestureIntent, each read only when the
        // first holds.
        expr::RIGHT_1278_TO_FOOT_PLANT_621 => {
            Ok(condition(cond::IS_RIDING_GOOFY)? && condition(cond::HAS_GESTURE_INTENT_SQUARE)?)
        }
        expr::LEFT_1279_TO_FOOT_PLANT_621 => {
            Ok(condition(cond::HAS_GESTURE_INTENT_SQUARE)? && !condition(cond::IS_RIDING_GOOFY)?)
        }
        expr::RANDOM_275 => Ok(condition(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_0_1)?
            && condition(cond::HAS_INTENT_NO_INPUT)?), //82921934
        expr::START_ANTICIPATION_90 => Ok(condition(cond::HAS_INTENT_NEW_PUSH_2ND)?
            && condition(cond::HAS_INTENT_RIGHT_PUSH_2ND)?
            && condition(cond::HAS_INTENT_LEFT_PUSH_2ND)?), //82921a00
        _ => Err(MotionUpdateError::PendingMaskedExpression {
            mask: Mask::Sustain,
            expression: index,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 86 and the other remaining jump-table defaults return true without reads; the late
    /// cases read their conditions in the native order and stop at the first false one.
    #[test]
    fn remaining_defaults_and_late_cases_follow_the_jump_table() {
        for index in [24, 86, 89, 211, 343, 444] {
            assert!(expression(index, &mut |_| panic!("default Sustain has no reads")).unwrap());
        }
        let mut reads = Vec::new();
        assert!(
            !expression(357, &mut |i| {
                reads.push(i);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [218]);
        reads.clear();
        assert!(
            expression(358, &mut |i| {
                reads.push(i);
                Ok(i == 366)
            })
            .unwrap()
        );
        assert_eq!(reads, [366, 218]);
        assert!(!expression(310, &mut |_| Ok(true)).unwrap());
        assert!(expression(418, &mut |_| Ok(true)).unwrap());
    }

    #[test]
    fn air_entry_sustain_is_constant_but_lateflip_retains_air_check() {
        for index in [
            92, 157, 161, 164, 166, 167, 168, 175, 184, 185, 186, 187, 188, 189, 190, 292,
        ] {
            assert!(expression(index, &mut |_| panic!("constant native Sustain")).unwrap());
        }
        let mut reads = Vec::new();
        assert!(
            !expression(335, &mut |i| {
                reads.push(i);
                Ok(false)
            })
            .unwrap()
        );
        assert_eq!(reads, [223]);
    }
    #[test]
    fn into405_sustain_does_not_sample_pre_intents() {
        for index in [142, 257, 405] {
            assert!(
                expression(index, &mut |_| panic!(
                    "native constant Sustain has no reads"
                ))
                .unwrap()
            );
        }
    }

    #[test]
    fn sustain_ground_predicate_propagates_both_results_and_failed_live_read() {
        for expected in [false, true] {
            let mut reads = Vec::new();
            let result = expression(200, &mut |index| {
                reads.push(index);
                Ok(expected)
            })
            .unwrap();
            assert_eq!(result, expected);
            assert_eq!(reads, [202]);
        }
        assert!(matches!(
            expression(200, &mut |index| {
                assert_eq!(index, 202);
                Err(MotionUpdateError::MissingOwner("actual PhysOut"))
            }),
            Err(MotionUpdateError::MissingOwner("actual PhysOut"))
        ));
    }

    #[test]
    fn unreviewed_sustain_does_not_fall_back_to_pre_or_sample_conditions() {
        assert!(matches!(
            expression(u16::MAX, &mut |_| panic!(
                "unreviewed Sustain must not read Pre predicate"
            )),
            Err(MotionUpdateError::PendingMaskedExpression {
                mask: Mask::Sustain,
                expression: u16::MAX,
            })
        ));
    }

    #[test]
    fn native_sustain_true_arms_do_not_sample_pre_conditions() {
        for index in [
            0, 3, 5, 270, 199, 237, 355, 344, 345, 215, 216, 218, 220, 224, 226, 227, 253, 254,
            255, 35, 274, 296, 346, 389, 390, 413, 414, 416, 417, 420, 427, 429, 441, 442,
        ] {
            assert!(expression(index, &mut |_| panic!("native true arm has no reads")).unwrap());
        }
    }

    #[test]
    fn sustain_leaves_preserve_live_result_and_failure() {
        for (expression_index, condition_index) in
            [(134, 223), (201, 289), (203, 293), (347, 291), (348, 291)]
        {
            for expected in [false, true] {
                let mut reads = Vec::new();
                assert_eq!(
                    expression(expression_index, &mut |index| {
                        reads.push(index);
                        Ok(expected)
                    })
                    .unwrap(),
                    expected
                );
                assert_eq!(reads, [condition_index]);
            }
            assert!(matches!(
                expression(expression_index, &mut |index| {
                    assert_eq!(index, condition_index);
                    Err(MotionUpdateError::MissingOwner("live condition"))
                }),
                Err(MotionUpdateError::MissingOwner("live condition"))
            ));
        }
    }

    #[test]
    fn sustain_pro_skater_or_stops_at_first_true_or_error() {
        for stop in 331..=336 {
            let mut reads = Vec::new();
            let result = expression(437, &mut |index| {
                reads.push(index);
                Ok(index == stop)
            })
            .unwrap();
            assert_eq!(result, stop <= 335);
            assert_eq!(reads, (331..=stop.min(335)).collect::<Vec<_>>());
        }
        for stop in 331..=335 {
            let mut reads = Vec::new();
            assert!(matches!(
                expression(437, &mut |index| {
                    reads.push(index);
                    if index == stop {
                        Err(MotionUpdateError::MissingOwner("live condition"))
                    } else {
                        Ok(false)
                    }
                }),
                Err(MotionUpdateError::MissingOwner("live condition"))
            ));
            assert_eq!(reads, (331..=stop).collect::<Vec<_>>());
        }
    }

    #[test]
    fn dismount_sustain_checks_landing_before_running_out() {
        for (landing, running_out, expected_reads, expected) in [
            (true, false, vec![11], false),
            (false, true, vec![11, 12], false),
            (false, false, vec![11, 12], true),
        ] {
            let mut reads = Vec::new();
            let actual = expression(430, &mut |index| {
                reads.push(index);
                Ok(if index == 11 { landing } else { running_out })
            })
            .unwrap();
            assert_eq!(reads, expected_reads);
            assert_eq!(actual, expected);
        }
    }
}

#[cfg(test)]
mod air_transition_sustain_tests {
    use super::*;
    #[test]
    fn transition_sustain_uses_its_own_native_cases() {
        for index in [243, 250, 336, 337, 338, 339] {
            assert!(expression(index, &mut |_| panic!("constant source Sustain")).unwrap());
        }
        for (index, condition, negate) in [(191, 218, false), (192, 218, true), (242, 350, false)] {
            for answer in [false, true] {
                let mut reads = Vec::new();
                assert_eq!(
                    expression(index, &mut |i| {
                        reads.push(i);
                        Ok(answer)
                    })
                    .unwrap(),
                    answer != negate
                );
                assert_eq!(reads, [condition]);
            }
        }
    }
}
