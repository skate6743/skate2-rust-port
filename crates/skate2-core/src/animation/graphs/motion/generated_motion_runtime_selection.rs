//! Initial Motion `Evaluate`8292bf58 and generated Pre evaluator8290aa50.
use super::MotionUpdateError;
use crate::generated_action_load::NULL_INDEX;
#[cfg(test)]
use crate::generated_action_load::StateDeclaration;
use crate::generated_motion_load::ids::{condition as cond, expression as expr};
#[cfg(test)]
#[path = "generated_motion_runtime_selection_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) fn evaluate(
    states: &[StateDeclaration],
    index: u16,
    condition: &mut impl FnMut(u16, u16) -> Result<bool, MotionUpdateError>,
) -> Result<u16, MotionUpdateError> {
    let state = states
        .get(usize::from(index))
        .ok_or(MotionUpdateError::InvalidState(index))?;
    if !state.active || !expression(state.expression, &mut |c| condition(c, index))? {
        return Ok(NULL_INDEX);
    }
    if state.children.count == 0 {
        return Ok(index);
    }
    for offset in 0..state.children.count {
        let child = state
            .children
            .index
            .checked_add(offset)
            .ok_or(MotionUpdateError::Capacity)?;
        let selected = evaluate(states, child, condition)?;
        if selected != NULL_INDEX {
            return Ok(selected);
        }
    }
    Ok(NULL_INDEX)
}

/// Raw switch cases retain lazy operands and repeated cached condition reads.
pub(super) fn expression(
    index: u16,
    c: &mut impl FnMut(u16) -> Result<bool, MotionUpdateError>,
) -> Result<bool, MotionUpdateError> {
    Ok(match index {
        expr::LEFT_SLIDE_TO_IDLE => c(cond::SHOULD_LEAVE_SLIDE_NOT_RIGHT)?, //82910b4c: ShouldLeaveSlide(left), cached positive result
        expr::RIGHT_SLIDE_TO_IDLE => c(cond::SHOULD_LEAVE_SLIDE_RIGHT)?, //82910c0c: cached ShouldLeaveSlide, no negation
        expr::N_360_HARDFLIP_216 => c(cond::HAS_INTENT_N_360_HARDFLIP)?, //8290de98: HasIntent
        expr::CRAIL_FINGER_FLIP_SHUV_1250 => {
            c(cond::HAS_INTENT_FINGER_FLIP_SHUV)? && c(cond::HAS_INTENT_CRAIL_GRAB)?
        } //82915034: ordered HasIntent pair
        expr::N_360_INWARD_HEELFLIP_217 => c(cond::HAS_INTENT_N_360_INWARD_HEELFLIP)?, //8290def8: HasIntent(N_360InwardHeelflip), cached
        expr::SEATBELT_FINGER_FLIP_SHUV_1239 => {
            c(cond::HAS_INTENT_FINGER_FLIP_SHUV)? && c(cond::HAS_INTENT_SEATBELT_GRAB)?
        } //82914b74: FingerFlipShuv, then SeatbeltGrab
        expr::KICKOUT_DISMOUNT_13_TO_KICKOUT_TO_AIR_14 => c(cond::WILL_EXPIRE_IN_0_033)?, //8290bec4: WillExpire(InTime=0.033)
        expr::KICKOUT_DISMOUNT_13_TO_WIPE_OUT_4 => !c(cond::PHYS_FILTERED_STATE_AIR)?, //8290bf24: negated PhysFilteredState(air), both paths
        expr::OUT_960 => !c(cond::HAS_INTENT_FS_CHRIST)?, //8290e7f0: HasIntent(FSChrist), cntlzw on both paths
        expr::OUT_1238 => !c(cond::HAS_INTENT_SEATBELT_GRAB)?, //8290f3bc: HasIntent(SeatbeltGrab), cntlzw on both paths
        expr::OUT_1249 => !c(cond::HAS_INTENT_CRAIL_GRAB)?, //8290f764: HasIntent(CrailGrab), cntlzw on both paths
        expr::RUN_DISMOUNT_102 => {
            c(cond::COM_VEL_COMPARE_ABS_GREATER_EQUAL_4)?
                && !c(cond::LAST_STATE_IDLE_DISMOUNT)?
                && !c(cond::IS_RIDING_FAKIE)?
                && !c(cond::DISABLE_PUSH_BRAKE)?
        } //82911dc4: |ComVel|>=4, then LastState(IdleDismount), fakie, push/brake exclusions
        expr::TO_OFF_BOARD_TO_OB_AIR => c(cond::WILL_EXPIRE_IN_0_2ND)?, //8290ece4: WillExpire(InTime=0.0)
        expr::OB_TIME_TO_LAND_LESS_EQUAL_0 => c(cond::OB_TIME_TO_LAND_LESS_EQUAL_0)?, //8290b3b4: OBTimeToLand(lessEqual 0.0)
        expr::RUN_DISMOUNT_WAIT_FOR_LANDING_103_TO_RIGHT_FOOT_STRIDE_37 => {
            c(cond::OB_TIME_TO_LAND_GREATER_0_15)?
        } //8290b410: OBTimeToLand(greater 0.15); cached lbz/clrlwi reads bit7
        expr::RUN_DISMOUNT_102_TO_RUN_AROUND_107 => {
            c(cond::WILL_EXPIRE_IN_0_1)? && c(cond::OB_TIME_TO_LAND_LESS_EQUAL_0)?
        } //82911f34: WillExpire(0.1), then OBTimeToLand(lessEqual 0.0)
        expr::RUN_DISMOUNT_102_TO_RUN_DISMOUNT_WAIT_FOR_LANDING_103 => {
            c(cond::WILL_EXPIRE_IN_0)? && c(cond::OB_TIME_TO_LAND_GREATER_0)?
        } //82911ffc: WillExpire(0.0), then OBTimeToLand(greater 0.0)
        expr::IDLE_684_TO_IN_AIR_21 => {
            c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_0_4)?
                && c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_MAG_ABS_LESS_0_4)?
                && !c(cond::HAS_INTENT_BOARD_ADJUST_RIGHT)?
        } //82919ff4: filtered angle and magnitude, then not BoardAdjustRight
        expr::N_LASERFLIP_215 => c(cond::HAS_INTENT_N_LASERFLIP)?,                    //8290de38
        expr::OUT_972 => !c(cond::HAS_INTENT_BS_CHRIST)?,                             //8290eda4
        expr::CYCLE_1237 => c(cond::HAS_INTENT_SEATBELT_GRAB_2ND)?,                   //8290f35c
        expr::IDLE_685_TO_IN_AIR_21 => {
            c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_0_4)?
                && c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_MAG_ABS_LESS_0_4)?
                && !c(cond::HAS_INTENT_BOARD_ADJUST_UP)?
        } //8291a118: filtered tweak then intent exclusion
        expr::N_360_FLIP_214 => c(cond::HAS_INTENT_N_360_FLIP)?, //8290ddd8: HasIntent
        expr::INTO_1236 => c(cond::HAS_INTENT_SEATBELT_GRAB)?,   //8290f2fc: HasIntent
        expr::CYCLE_1248 => c(cond::HAS_INTENT_CRAIL_GRAB_2ND)?, //8290f704: HasIntent
        expr::COFFIN_TO_DBL_267 => c(cond::HAS_INTENT_GROUND_DBL_GRAB)?, //8290fdc4: HasIntent
        expr::N_FS_360_POP_SHUVIT_213 => c(cond::HAS_INTENT_N_FS_360_POP_SHUVIT)?, //8290dd78: HasIntent
        expr::HAS_ANIM_ATTRIBUTE_CHRIST_EARLY_OUT => c(cond::HAS_ANIM_ATTRIBUTE_CHRIST_EARLY_OUT)?, //8290e790: HasAnimAttribute
        expr::NEITHER_GROUND_NOR_AIR => {
            !c(cond::PHYS_FILTERED_STATE_GROUND)? && !c(cond::PHYS_FILTERED_STATE_AIR)?
        } //82913420: ordered filtered-state exclusions
        expr::INTO_1142 => c(cond::HAS_INTENT_BOARD_ADJUST_UP)? && c(cond::HAS_INTENT_TAIL_GRAB)?, //829146b4: ordered HasIntent pair
        expr::WILL_EXPIRE_IN_0_25_NOT_WAIT => c(cond::WILL_EXPIRE_IN_0_25_NOT_WAIT)?, //8290b2fc: WillExpire
        expr::N_360_POP_SHUVIT_212 => c(cond::HAS_INTENT_N_360_POP_SHUVIT)?, //8290dd18: HasIntent
        expr::FOOT_PLANT_TO_WIPE_OUT => {
            c(cond::COM_VEL_COMPARE_ABS_GREATER_EQUAL_15)?
                || (!c(cond::PHYS_FILTERED_STATE_GROUND)? && !c(cond::PHYS_FILTERED_STATE_AIR)?)
        } //8291949c: velocity or neither filtered state
        expr::N_INWARD_HEELFLIP_211 => c(cond::HAS_INTENT_N_INWARD_HEELFLIP)?, //8290dcb8: cached HasIntent
        expr::TO_SUPERMAN => {
            c(cond::HAS_INTENT_SUPERMAN)?
                || (c(cond::HAS_AG_INTENT_LEFT_AIR_GRAB)? && c(cond::HAS_AG_INTENT_RIGHT_AIR_GRAB)?)
        } //82919db0: intent OR ordered action-intent pair
        expr::N_HARDFLIP_210 => c(cond::HAS_INTENT_N_HARDFLIP)?,               //8290dc58: HasIntent
        expr::TRUCKS_OR_DECK_IN_CONTACT_OR_NOT_AIR => {
            c(cond::TRUCKS_OR_DECK_IN_CONTACT)? || !c(cond::PHYS_FILTERED_STATE_AIR)?
        } //82913884: contact first, otherwise not filtered air
        expr::N_VARIAL_HEELFLIP_209 => c(cond::HAS_INTENT_N_VARIAL_HEELFLIP)?, //8290dbf8: HasIntent, cached unchanged
        expr::WILL_EXPIRE_IN_0 => c(cond::WILL_EXPIRE_IN_0)?, //8290b244: WillExpire(InTime=0.0), spawn/mount-jump transitions
        expr::MOUNT_34_TO_ON_BOARD_3 => c(cond::WILL_EXPIRE_IN_0_3)?, //8290bda8: WillExpire(InTime=0.3), Mount34 -> OnBoard3
        expr::OUT_644 => !c(cond::HAS_INTENT_DBL_GRAB)?, //8290e520: HasIntent(DBLGrab), cntlzw negation on both paths
        expr::FLY_116 => c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)?, //8290c05c: PhysFilteredState(offboardair), Fly116 guard
        expr::FINISH_117 => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD)? || c(cond::HAS_INTENT_OFF_BOARD)?
        } //82912768: PhysFilteredState(offboard), else HasIntent(OffBoard)
        expr::STRIDE_TO_RUN_AROUND => c(cond::OB_TIME_TO_LAND_LESS_EQUAL_0_03)?, //8290be08: OBTimeToLand(lessEqual 0.03), stride -> RunAround107
        expr::TAKEOFF_1355_TO_DESCENT_1356 => c(cond::WILL_EXPIRE_IN_0_017)?, //8290bb50: WillExpire(InTime=0.017), Takeoff1355 -> Descent1356
        expr::DESCENT_1356_TO_JUMPING_INTO_MOUNT_1358 => {
            c(cond::IN_STATE_FOR_TIME_DESCENT_LESS_0_3)?
        } //8290bbb0: InStateForTime(Descent, less 0.3) -> JumpingIntoMount1358
        expr::PHYS_FILTERED_STATE_OFFBOARD => c(cond::PHYS_FILTERED_STATE_OFFBOARD)?, //8290bc10: PhysFilteredState(offboard), Land1357 guard
        expr::TO_MOUNT | expr::JUMPING_INTO_MOUNT_1358 => c(cond::HAS_INTENT_OB_MOUNT)?, //8290ba94: HasIntent(OB_Mount), shared by cases 24 and 29
        expr::INTO_BS_GRAB_OUT => !c(cond::HAS_INTENT_BSGRAB)?, //8290bc70: HasIntent(BSGRAB), cntlzw negation on both paths
        expr::LANDING_ON_BOARD_28 => {
            c(cond::CAN_LAND_ON_BOARD)? && c(cond::OB_TIME_TO_LAND_GREATER_0_05)?
        } //82916ae0: CanLandOnBoard, then OBTimeToLand(greater 0.05)
        expr::FROM_DBL_GRAB_656 => c(cond::HAS_INTENT_FS_GRAB_2ND)?, //8290ec84: HasIntent(FSGrab), Out644 -> FromDBLGrab656
        expr::FROM_DBL_GRAB_667 => c(cond::HAS_INTENT_BS_GRAB_2ND)?, //8290ee94: HasIntent(BSGrab), Out644 -> FromDBLGrab667
        //82918028: AccidentalDismount17 -> WipeOut4. Neither offboardair, offboard
        //nor ground is true; else OBTimeToLand<=0.05 and (InTimeWindow or IsTransitioning).
        expr::ACCIDENTAL_DISMOUNT_17_TO_WIPE_OUT_4 => {
            !(c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND)?
                || c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
                || c(cond::PHYS_FILTERED_STATE_GROUND)?)
                || (c(cond::OB_TIME_TO_LAND_LESS_EQUAL_0_05)?
                    && (c(cond::IN_TIME_WINDOW_START_0_LENGTH_0_3)? || c(cond::IS_TRANSITIONING)?))
        }
        expr::PUSH_FROM_FAKIE_1178_TO_IDLE_1025 => {
            c(cond::WILL_EXPIRE_IN_0_02)?
                && (c(cond::DISABLE_PUSH_BRAKE)?
                    || c(cond::HAS_INTENT_GROUND_BS_GRAB)?
                    || c(cond::HAS_INTENT_GROUND_FS_GRAB_2ND)?
                    || c(cond::HAS_INTENT_GROUND_DBL_GRAB)?
                    || c(cond::HAS_INTENT_COFFIN)?)
        } //8291d71c: WillExpire AND ordered interruption gates
        expr::CAN_LAND_ON_BOARD => c(cond::CAN_LAND_ON_BOARD)?, //82911988: CanLandOnBoard
        expr::LOCO_NEW_319_TO_TO_LANDING_ON_BOARD_FROM_LEFT_26 => c(cond::IS_IN_JUMP_AIR_LEFT)?, //8290bce8
        expr::LOCO_NEW_319_TO_TO_LANDING_ON_BOARD_FROM_RIGHT_27 => c(cond::IS_IN_JUMP_AIR_RIGHT)?, //8290bd48
        expr::JUMPING_INTO_MOUNT_LEFT_9 => {
            (c(cond::IS_AIR_MOUNT_REQUESTED)?
                && c(cond::IS_IN_JUMP_AIR_LEFT)?
                && c(cond::IS_PAST_LOCOMOTION_APEX)?
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?)
                || (c(cond::CURRENT_STATE_OB_AIR_RIGHT_FOOT_STRIDE)?
                    && c(cond::HAS_INTENT_OB_MOUNT)?)
        } //8291beb8
        expr::JUMPING_INTO_MOUNT_RIGHT_10 => {
            (c(cond::IS_AIR_MOUNT_REQUESTED)?
                && c(cond::IS_IN_JUMP_AIR_RIGHT)?
                && c(cond::IS_PAST_LOCOMOTION_APEX)?
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?)
                || (c(cond::CURRENT_STATE_OB_AIR_LEFT_FOOT_STRIDE)?
                    && c(cond::HAS_INTENT_OB_MOUNT)?)
        } //8291c13c
        expr::IDLE_TO_RUN_AROUND => c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?, //8290b9d4: offboard locomotion request
        expr::OFF_BOARD_WILL_EXPIRE_IN_0_1 => c(cond::WILL_EXPIRE_IN_0_1_2ND)?, //8290ba34: offboard WillExpire
        expr::STANDING_JUMP_106 => c(cond::HAS_INTENT_OB_JUMP)?, //8290baf0: offboard idle target intent
        expr::SLIDE_OUT_EARLY => c(cond::HAS_ANIM_ATTRIBUTE_EARLY_OUT)?, //82910714: slide early-out animation attribute
        expr::INTO_1251_TO_OUT_1252 => {
            c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_0_3)? && c(cond::SHOULD_LEAVE_SLIDE_NOT_RIGHT)?
        } //82915c14: minimum slide time then left exit
        expr::INTO_1253_TO_OUT_1254 => {
            c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_0_3)? && c(cond::SHOULD_LEAVE_SLIDE)?
        } //82915cdc: same timer then right exit
        expr::SLIDE_OUT_MID => c(cond::HAS_ANIM_ATTRIBUTE_MID_OUT)?, //82910774: slide mid-out animation attribute
        expr::OUT_TO_BRAKE => c(cond::WILL_EXPIRE_IN_0_1_NOT_WAIT)?, //829107d4: slide-out WillExpire
        expr::CYC_643 => c(cond::HAS_INTENT_DBL_GRAB)?, //8290e460: ground double-grab alternative target intent
        expr::COFFIN_INTO_84 => c(cond::HAS_INTENT_COFFIN)?, //8290fd64: Coffin target evaluated from double ground grab
        expr::WILL_EXPIRE_IN_0_02 => c(cond::WILL_EXPIRE_IN_0_02)?, //8290e1c4: stance-switch WillExpire
        expr::MANUAL_1027_TO_BUMPED_276 => c(cond::IN_STATE_FOR_TIME_ON_GROUND_GREATER_0_5)?, //82910f40: manual parent timer
        expr::IDLE_683_TO_IN_AIR_21 => {
            c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_0_4)?
                && c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_MAG_ABS_LESS_0_4)?
                && !c(cond::HAS_INTENT_BOARD_ADJUST_LEFT)?
        } //82919ed0: filtered tweak direction/magnitude, then release
        expr::BRAKE => c(cond::HAS_INTENT_MANUAL_BRAKE_2ND)?, //8291114c: manual Brake intent
        expr::MOVING => c(cond::PHYSICS_SPEED_COMPARE_ABS_ALONG_SKATE_Z_GREATER_0_1)?, //829111ac: manual braking speed
        expr::STATIONARY => !c(cond::PHYSICS_SPEED_COMPARE_ABS_ALONG_SKATE_Z_GREATER_0_1)?, //8291120c: stationary complement, cache raw comparison
        expr::STEP_BACKWARD_313 => {
            c(cond::LAST_STATE_TO_STEP_BACKWARD)?
                || (c(cond::HAS_INTENT_STEP_BACKWARD)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_BACKWARD)? && c(cond::IS_MIRRORED)?)
        } //82917464
        expr::RIGHT_SLIDE => c(cond::CAN_ENTER_SLIDE_RIGHT)?, //82910bac: CanEnterSlide
        expr::HAS_INTENT_GROUND_DBL_GRAB_OR_COFFIN => {
            c(cond::HAS_INTENT_GROUND_DBL_GRAB_2ND)? || c(cond::HAS_INTENT_COFFIN_2ND)?
        } //82915f3c: double grab or coffin
        expr::DBL_GRAB_INTO_1265_TO_TURNING_719 | expr::TO_DBL_GRAB_OUT => {
            !c(cond::HAS_INTENT_GROUND_DBL_GRAB)? && !c(cond::HAS_INTENT_COFFIN)?
        } //82910da4 /82916008: ordered ground-grab/coffin exit
        expr::NOT_HAS_INTENT_GROUND_BS_GRAB => !c(cond::HAS_INTENT_GROUND_BS_GRAB)?, //82910a74: ground BS grab release
        expr::NOT_HAS_INTENT_GROUND_FS_GRAB => !c(cond::HAS_INTENT_GROUND_FS_GRAB_2ND)?, //82910ccc: ground FS grab release
        expr::DBL_GRAB_CYC_1266_TO_DBL_GRAB_OUT_1267 => !c(cond::HAS_INTENT_GROUND_DBL_GRAB)?, //82910ecc: ground double grab release
        expr::STEP_FORWARD_309 => {
            c(cond::LAST_STATE_TO_STEP_FORWARD)?
                || (c(cond::HAS_INTENT_STEP_FORWARD)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_FORWARD)? && c(cond::IS_MIRRORED)?)
        } //82917294: preserve repeated HasIntent reads
        expr::STEP_FORWARD_RIGHT_310 => {
            c(cond::LAST_STATE_TO_STEP_FORWARD_RIGHT)?
                || (c(cond::HAS_INTENT_STEP_FORWARD_RIGHT)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_FORWARD_LEFT)? && c(cond::IS_MIRRORED)?)
        } //1.02 827632b0: mirrored, the other side's step
        expr::STEP_RIGHT_311 => {
            c(cond::LAST_STATE_TO_STEP_RIGHT)?
                || (c(cond::HAS_INTENT_STEP_RIGHT)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_LEFT)? && c(cond::IS_MIRRORED)?)
        } //1.02 8276345c
        expr::STEP_BACK_RIGHT_312 => {
            c(cond::LAST_STATE_TO_STEP_BACK_RIGHT)?
                || (c(cond::HAS_INTENT_STEP_BACK_RIGHT)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_BACK_LEFT)? && c(cond::IS_MIRRORED)?)
        } //1.02 82763608
        expr::STEP_BACK_LEFT_314 => {
            c(cond::LAST_STATE_TO_STEP_BACK_LEFT)?
                || (c(cond::HAS_INTENT_STEP_BACK_LEFT)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_BACK_RIGHT)? && c(cond::IS_MIRRORED)?)
        } //1.02 8276395c
        expr::STEP_LEFT_315 => {
            c(cond::LAST_STATE_TO_STEP_LEFT)?
                || (c(cond::HAS_INTENT_STEP_LEFT)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_RIGHT)? && c(cond::IS_MIRRORED)?)
        } //1.02 82763b08
        expr::STEP_FORWARD_LEFT_316 => {
            c(cond::LAST_STATE_TO_STEP_FORWARD_LEFT)?
                || (c(cond::HAS_INTENT_STEP_FORWARD_LEFT)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_FORWARD_RIGHT)? && c(cond::IS_MIRRORED)?)
        } //1.02 82763cb4
        expr::NOT_HAS_INTENT_COFFIN => !c(cond::HAS_INTENT_COFFIN)?, //8290ec0c: HasIntent negated
        expr::HAS_INTENT_RIGHT_AND_LEFT_PUSH => {
            c(cond::HAS_INTENT_RIGHT_PUSH_2ND)? && c(cond::HAS_INTENT_LEFT_PUSH_2ND)?
        } //8291682c: ordered two-intent gate
        expr::RUN_AROUND_107 => {
            c(cond::LOCOMOTION_VALID)?
                && (c(cond::IS_DISMOUNTING_2ND)?
                    || c(cond::IS_LANDING)?
                    || c(cond::IS_RUNNING_OUT)?
                    || c(cond::IS_LOCOMOTION_QUIT_NOT_ALLOWED)?
                    || (c(cond::HAS_INTENT_OBL_STEER_MAGNITUDE_GREATER_0_3)?
                        && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD)?))
        } //8291db50
        expr::SPIN_261 => c(cond::HAS_LANDING_TYPE_SPIN)?,           //8290fbcc
        expr::SKETCHY_262 => c(cond::HAS_LANDING_TYPE_SKETCHY)?,     //8290fc2c
        expr::IN_PARENT_STATE_FOR_TIME_GREATER_0_1 => {
            c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_0_1)?
        } //8290fc8c: retained parent-state timer
        expr::LAND_22_TO_ANTICIPATION_1030 => !c(cond::HAS_ANIM_ATTRIBUTE_ABSORB_LAND)?, //8290fcec: cache raw HasAnimAttribute, negate either return path
        expr::HAS_INTENT_OB_DISMOUNT => c(cond::HAS_INTENT_OB_DISMOUNT)?, //8290be68: HasIntent(OB_Dismount), one cached read
        expr::CYC_659 => {
            c(cond::HAS_INTENT_BS_GRAB)?
                || c(cond::CURRENT_STATE_FINGER_FLIP_VAR_GRAB_OUT)?
                || (c(cond::HAS_INTENT_MUTE_GRAB)? && c(cond::HAS_INTENT_FINGER_FLIP_SHUV)?)
        } //8291bacc -> shared8291b9f8
        expr::OUT_665 => !c(cond::HAS_INTENT_BS_GRAB)?,                   //8290ee1c: BSGrab release
        expr::HAS_INTENT_BS_CHRIST => c(cond::HAS_INTENT_BS_CHRIST)?, //8290e6d0: BSChrist candidate
        expr::HAS_INTENT_GROUND_BS_GRAB_CYCLE => c(cond::HAS_INTENT_GROUND_BS_GRAB_2ND)?, //82910aec: BSGrabCyc target gate
        expr::BS_GRAB_FINGER_FLIP_SHUV_663 => {
            c(cond::HAS_INTENT_FINGER_FLIP_SHUV)? && c(cond::HAS_INTENT_BS_GRAB)?
        } //82914138: FingerFlipShuv then BSGrab
        expr::CYC_648 => {
            c(cond::HAS_INTENT_FS_GRAB)?
                || c(cond::CURRENT_STATE_FINGER_FLIP_VAR_GRAB_OUT)?
                || (c(cond::HAS_INTENT_MUTE_GRAB)? && c(cond::HAS_INTENT_FINGER_FLIP_SHUV)?)
        } //8291b948: FSGrab Cyc gate
        expr::HAS_INTENT_GROUND_FS_GRAB_CYCLE => c(cond::HAS_INTENT_GROUND_FS_GRAB)?, //82910c6c: GroundFSGrab state gate
        expr::TO_DBL_GRAB => c(cond::HAS_INTENT_DBL_GRAB_2ND)? || c(cond::HAS_INTENT_COFFIN_2ND)?, //82913edc: DBLGrab, then Coffin
        expr::HAS_INTENT_FS_CHRIST => c(cond::HAS_INTENT_FS_CHRIST)?, //8290e670: FSChrist state gate
        expr::INTO_FS_GRAB_RIGHT_ONE_FOOT_AIR => {
            c(cond::HAS_INTENT_FS_GRAB)? && c(cond::HAS_INTENT_RIGHT_ONE_FOOT_AIR)?
        } //82913950: FSGrab, then RightOneFootAir
        expr::INTO_FS_GRAB_LEFT_ONE_FOOT_AIR => {
            c(cond::HAS_INTENT_FS_GRAB)? && c(cond::HAS_INTENT_LEFT_ONE_FOOT_AIR)?
        } //82913a1c: FSGrab, then LeftOneFootAir
        expr::INTO_BS_GRAB_RIGHT_ONE_FOOT_AIR => {
            c(cond::HAS_INTENT_BS_GRAB)? && c(cond::HAS_INTENT_RIGHT_ONE_FOOT_AIR)?
        } //82913ae8: BSGrab, then RightOneFootAir
        expr::INTO_BS_GRAB_LEFT_ONE_FOOT_AIR => {
            c(cond::HAS_INTENT_BS_GRAB)? && c(cond::HAS_INTENT_LEFT_ONE_FOOT_AIR)?
        } //82913bb4: BSGrab, then LeftOneFootAir
        expr::FINGER_FLIP => c(cond::HAS_INTENT_FINGER_FLIP)?,        //8290ea78: FingerFlip
        expr::FINGER_FLIP_VAR => c(cond::HAS_INTENT_FINGER_FLIP_VAR)?, //8290ead8: FingerFlipVar
        expr::FS_GRAB_FINGER_FLIP_SHUV_652 => {
            c(cond::HAS_INTENT_FINGER_FLIP_SHUV)? && c(cond::HAS_INTENT_FS_GRAB)?
        } //82913e10: FingerFlipShuv, then FSGrab
        // Finger flip GrabOut exits: WillExpire(0.05), then the grab intent held
        // (back to the grab cycle) or released (back to InAir21).
        expr::FS_GRAB_OUT_TO_CYC => {
            c(cond::WILL_EXPIRE_IN_0_05)? && c(cond::HAS_INTENT_FS_GRAB_2ND)?
        } //82913c80: FSGrab held
        expr::FS_GRAB_OUT_TO_IN_AIR => {
            c(cond::WILL_EXPIRE_IN_0_05)? && !c(cond::HAS_INTENT_FS_GRAB_2ND)?
        } //82913d48: FSGrab released
        expr::BS_GRAB_OUT_TO_CYC => {
            c(cond::WILL_EXPIRE_IN_0_05)? && c(cond::HAS_INTENT_BS_GRAB_2ND)?
        } //82913fa8: BSGrab held
        expr::BS_GRAB_OUT_TO_IN_AIR => {
            c(cond::WILL_EXPIRE_IN_0_05)? && !c(cond::HAS_INTENT_BS_GRAB_2ND)?
        } //82914070: BSGrab released
        expr::GRAB_OUT_983_TO_CYC_670 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && c(cond::HAS_INTENT_MUTE_GRAB_2ND)?
        } //82914204: MuteGrab held
        expr::GRAB_OUT_983_TO_IN_AIR_21 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && !c(cond::HAS_INTENT_MUTE_GRAB_2ND)?
        } //829142c8: MuteGrab released
        expr::GRAB_OUT_985_TO_CYC_677 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && c(cond::HAS_INTENT_STALE_GRAB_2ND)?
        } //82914524: StaleGrab held
        expr::GRAB_OUT_985_TO_IN_AIR_21 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && !c(cond::HAS_INTENT_STALE_GRAB_2ND)?
        } //829145ec: StaleGrab released
        expr::GRAB_OUT_1235_TO_CYCLE_1143 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && c(cond::HAS_INTENT_TAIL_GRAB_2ND)?
        } //829149e4: TailGrab held
        expr::GRAB_OUT_1235_TO_IN_AIR_21 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && !c(cond::HAS_INTENT_TAIL_GRAB_2ND)?
        } //82914aac: TailGrab released
        expr::GRAB_OUT_1246_TO_CYCLE_1149 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && c(cond::HAS_INTENT_NOSE_GRAB_2ND)?
        } //82914ea4: NoseGrab held
        expr::GRAB_OUT_1246_TO_IN_AIR_21 => {
            c(cond::WILL_EXPIRE_IN_0_05)? && !c(cond::HAS_INTENT_NOSE_GRAB_2ND)?
        } //82914f6c: NoseGrab released
        expr::CYC_TO_OUT_HAS_TWEAK_LESS_0_2 => c(cond::HAS_TWEAK_LESS_0_2)?, //8290e4c0: HasTweak(less than 0.2) FSGrab Cyc exit
        expr::OUT_654 => !c(cond::HAS_INTENT_FS_GRAB)?, //8290eb38: FSGrab Out target-state gate negates cached HasIntent
        expr::OUT_TO_WIPE_OUT => c(cond::PHYSICS_WANTS_WIPE_OUT)?, //8290ebb0: FSGrab Out priority-2 PhysicsWantsWipeOut
        expr::OUT_NOT_HAS_INTENT_BRAKE => !c(cond::HAS_INTENT_BRAKE)?, //8291057c: Brake.Out target-state gate negates cached Brake intent
        expr::FS_1021 => {
            c(cond::IS_HAND_PLANTING_FS_GROUND)? && c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_ANTIC)?
        } //82915da4: actual handplant state before animation gate
        expr::BS_1022 => {
            c(cond::IS_HAND_PLANTING_BS_GROUND)? && c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_ANTIC)?
        } //82915e70: companion state, same lazy animation gate

        expr::PUSH_TRICK_74 => !c(cond::IS_RIDING_FAKIE)?, //8290df58: cached IsRidingFakie, negated
        expr::ROLLING_FORWARDS_1350 => c(cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_GREATER_0_5)?, //8291135c: PhysicsSpeedCompare
        expr::ROLLING_BACKWARDS_OR_STATIONARY_1351 => {
            !c(cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_GREATER_0_5)?
        } //829113bc: opposite speed branch
        expr::HAS_INTENT_ANY_KICK_TURN => {
            c(cond::HAS_INTENT_NEW_KICK_TURN)? || c(cond::HAS_INTENT_KICK_TURN)?
        } //829154f0: ordered predicates
        expr::SKITCH_KICK_TURN_TO_SKITCH_CARVE => {
            !c(cond::HAS_INTENT_NEW_KICK_TURN)? && !c(cond::HAS_INTENT_KICK_TURN)?
        } //829155bc: ordered negated HasIntent pair
        expr::NOT_HAS_INTENT_MANUAL_GREATER_0 => !c(cond::HAS_INTENT_MANUAL_GREATER_0_2ND)?, //829112e4: raw condition cached before negation
        expr::HOLDING_1273 => c(cond::HAS_INTENT_MANUAL_GREATER_0)?, //82911284: Holding target ancestor
        expr::ROLLING_BACKWARDS_1327 => c(cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_LESS_NEG_0_5)?, //82911074: PhysicsSpeedCompare, cached positive result
        expr::ROLLING_FORWARDS_OR_STATIONARY_1328 => {
            !c(cond::PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_LESS_NEG_0_5)?
        } //829110d4: negate the cached speed comparison
        expr::DBL_GRAB_CYC_1266 => c(cond::HAS_INTENT_GROUND_DBL_GRAB_2ND)?, //82910e6c: HasIntent
        expr::N_VARIAL_KICKFLIP_208 => c(cond::HAS_INTENT_N_VARIAL_KICKFLIP)?, //8290db98: HasIntent
        expr::HIPPY_JUMP_25_TO_HIPPY_RUNOUT_16 => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD)?
                && (c(cond::HAS_INTENT_OB_DISMOUNT)? || c(cond::PHYSICS_WANTS_RUNOUT)?)
        } //8291ac08: physical gate before dismount/runout
        expr::TURNING_RIGHT_753 => {
            (c(cond::HAS_INTENT_OB_STEER_GREATER_0_05)?
                && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?
                && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_OB_STEER_LESS_NEG_0_05)?
                    && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?
                    && c(cond::IS_MIRRORED)?)
        } //82916ee4 and shared82912464
        expr::N_FS_POP_SHUVIT_207 => c(cond::HAS_INTENT_N_FS_POP_SHUVIT)?, //8290db3c: HasIntent, cached positive result

        NULL_INDEX => true,
        expr::OB_GROUND_8 => c(cond::HAS_INTENT_OFF_BOARD)?,
        expr::SPAWN_DISMOUNT_31 => c(cond::HAS_INTENT_OB_SPAWN_DISMOUNT)?, //8290b1e8: SpawnDismount checks OB_SpawnDismount first
        expr::WIPE_OUT_4 => !c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?, //829119e8: cached original PhysFilteredState(Teleport), negated return
        expr::TELEPORT_5_TO_MOTION_0 => {
            c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_0_1)?
                && !c(cond::PHYS_FILTERED_STATE_TELEPORT)?
        } //82916bac: parent time >0.1, then not physical Teleport
        expr::IDLE_105 => {
            !(c(cond::LOCOMOTION_VALID)?
                && (c(cond::IS_DISMOUNTING_2ND)?
                    || c(cond::IS_LANDING)?
                    || c(cond::IS_RUNNING_OUT)?
                    || c(cond::IS_LOCOMOTION_QUIT_NOT_ALLOWED)?
                    || (c(cond::HAS_INTENT_OBL_STEER_MAGNITUDE_GREATER_0_3)?
                        && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD)?)))
        } //82916c74..6ee0: ordered locomotion exit gate
        expr::STILL_752 => {
            (c(cond::HAS_INTENT_OB_STEER_LESS_EQUAL_0_05)?
                && c(cond::HAS_INTENT_OB_STEER_GREATER_EQUAL_NEG_0_05)?)
                || !c(cond::HAS_INTENT_OB_STEER)?
                || !c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?
        } //8291218c..2304: ordered four-intent gate
        expr::TO_STEP_FORWARD_317 => {
            (c(cond::HAS_INTENT_STEP_FORWARD)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_FORWARD)? && c(cond::IS_MIRRORED)?)
        } //82912308: StepForward, preserve repeated reads
        expr::STEP_TO_RUN_AROUND => {
            c(cond::IN_STATE_FOR_TIME_IDLE_GREATER_0_4)?
                && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?
        } //82912480: idle over 0.4 s, then steer held
        expr::TO_STEP_BACKWARD_318 => {
            (c(cond::HAS_INTENT_STEP_BACKWARD)? && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_STEP_BACKWARD)? && c(cond::IS_MIRRORED)?)
        } //8291254c: StepBackward, preserve repeated reads
        expr::TURNING_LEFT_754 => {
            (c(cond::HAS_INTENT_OB_STEER_LESS_NEG_0_05)?
                && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?
                && !c(cond::IS_MIRRORED)?)
                || (c(cond::HAS_INTENT_OB_STEER_GREATER_0_05)?
                    && c(cond::HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND)?
                    && c(cond::IS_MIRRORED)?)
        } //829170e8..7290 and shared tail82917090: preserve repeated cached reads
        expr::N_POP_SHUVIT_206 => c(cond::HAS_INTENT_N_POP_SHUVIT)?, //8290dadc: HasIntent via sConditions+0x2f8
        expr::BRACE_FOR_IMPACT_98 => c(cond::WIPEOUT_TIME_TO_LAND_LESS_0_75)?, //82911a60: cached original WipeoutTimeToLand comparison
        expr::REST_AND_RECOVER_30 => c(cond::IS_DONE_WIPING_OUT)?, //82911ac0: cached IsDoneWipingOut
        expr::DEBUG_ANIMATIONS_7 => c(cond::IS_IN_DEBUG_ANIMATIONS_MODE)?, //82911b80: cached IsInDebugAnimationsMode
        expr::WILL_EXPIRE_IN_0_1 => c(cond::WILL_EXPIRE_IN_0_1)?, //8290b358: cached WillExpire
        expr::MOVING_OBJECT_104 => c(cond::IS_GRABBING_OBJECT)?, //8290b46c: cached IsGrabbingObject, direct positive result
        // MovingObject104 subtree; raw switch paths recorded in
        // dmo-motion-expression-paths-2026-09-28.json. Expression11 already
        // shares the native WillExpire condition28 elsewhere in this match.
        expr::MOVING_OBJECT_104_TO_LOCOMOTION_33 => !c(cond::IS_GRABBING_OBJECT)?,
        expr::MVOBJ_PUSH_LEFT => c(cond::IS_MOVING_OBJECT_FROM_NEG_110_TO_0)?,
        expr::MVOBJ_PUSH_RIGHT => c(cond::IS_MOVING_OBJECT_FROM_0_TO_110)?,
        expr::MVOBJ_PULL_LEFT => c(cond::IS_MOVING_OBJECT_FROM_180_TO_NEG_110)?,
        expr::MVOBJ_PULL_RIGHT => c(cond::IS_MOVING_OBJECT_FROM_110_TO_180)?,
        expr::MVOBJ_TO_PUSH_RIGHT => c(cond::IS_MOVING_OBJECT_FROM_0_TO_110_2ND)?,
        expr::IMMEDIATE => c(cond::ANGLE_ADJUST_BY_ANIMATION_END_LESS_THAN)?,
        expr::WAIT_FOR_END => !c(cond::ANGLE_ADJUST_BY_ANIMATION_END_LESS_THAN)?,
        expr::MVOBJ_TO_PUSH_LEFT => c(cond::IS_MOVING_OBJECT_FROM_NEG_110_TO_0_2ND)?,
        expr::MVOBJ_TO_PULL_RIGHT => c(cond::IS_MOVING_OBJECT_FROM_110_TO_180_2ND)?,
        expr::MVOBJ_TO_PULL_LEFT => c(cond::IS_MOVING_OBJECT_FROM_180_TO_NEG_110_2ND)?,
        expr::MVOBJ_NOT_MOVING => !c(cond::IS_MOVING_OBJECT_FROM_NEG_180_TO_180)?,
        expr::MOVING_OBJECT_104_TO_MOUNT_34 => {
            c(cond::HAS_INTENT_OB_MOUNT)? && !c(cond::IS_GRABBING_OBJECT)?
        }
        expr::FORCE_BRAKE_2 => c(cond::HAS_INTENT_FORCE_BRAKE)?, //8290c11c: ForceBrake
        expr::ANTIC_OUT_1029_TO_IDLE_1025 => c(cond::WILL_EXPIRE_IN_0_0000001_NOT_WAIT)?, //82911628: WillExpire, waitForTransitions=false
        expr::OLLIE_ANTIC_CYCLE_1288 => c(cond::HAS_INTENT_OLLIE_ANTICIPATION_2ND)?, //829116e8: OllieAnticipation
        expr::NOLLIE_ANTIC_CYCLE_1291 => c(cond::HAS_INTENT_NOLLIE_ANTICIPATION_2ND)?, //82911748: NollieAnticipation
        expr::RIGHT_1294_TO_FROM_FAKIE_1339 => c(cond::WILL_EXPIRE_IN_0_075)?, //829117a8: WillExpire(InTime=0.075), direct return
        expr::LEFT_1295_TO_FROM_FAKIE_1341 => c(cond::WILL_EXPIRE_IN_0_75)?, //82911808: WillExpire(InTime=0.75), FromFakie1341
        expr::STATE_360_POP_SHUVIT_ANTIC_CYCLE_1289 => {
            c(cond::HAS_INTENT_360_POP_SHUVIT_ANTICIPATION_2ND)?
                || c(cond::HAS_INTENT_POP_SHUVIT_ANTICIPATION_2ND)?
        } //82916500: 360/ordinary pop-shuvit anticipation
        expr::FS_360_POP_SHUVIT_ANTIC_CYCLE_1290 => {
            c(cond::HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION_2ND)?
                || c(cond::HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION_2ND)?
        } //829165c8: frontside counterpart
        expr::NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1292 => {
            c(cond::HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION_2ND)?
                || c(cond::HAS_INTENT_N_POP_SHUVIT_ANTICIPATION_2ND)?
        } //82916694: nollie counterpart
        expr::FS_NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1293 => {
            c(cond::HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION_2ND)?
                || c(cond::HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION_2ND)?
        } //82916760: nollie frontside counterpart
        expr::FORCE_BRAKE_2_TO_WIPE_OUT_4 => {
            c(cond::HAS_INTENT_WIPE_OUT_REQUEST)?
                || c(cond::IS_PHYSICS_WIPING)?
                || c(cond::PHYSICS_WANTS_WIPE_OUT)?
        } //82918250: ForceBrake wipeout transition
        expr::ANTIC_NOSE_1184 => {
            !(c(cond::HAS_INTENT_NOLLIE_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION)?)
        } //8291d1a0: nose AnticOut
        expr::WILL_EXPIRE_IN_0_01 => c(cond::WILL_EXPIRE_IN_0_01)?, //8290c0bc: cached WillExpire(InTime=0.01)
        expr::WILL_EXPIRE_IN_0_25 => c(cond::WILL_EXPIRE_IN_0_25)?, //82910654: cached WillExpire(InTime=0.25)
        expr::BUMPED_276_TO_SWITCH_1031 => {
            c(cond::WILL_EXPIRE_IN_0_1)? && c(cond::IS_RIDING_FAKIE)?
        } //829158ec: WillExpire before lazy IsRidingFakie
        expr::IS_RIDING_FAKIE => c(cond::IS_RIDING_FAKIE)?, //8290b2a0: actual shared IsRidingFakie
        expr::HAS_INTENT_RIGHT_PUSH => c(cond::HAS_INTENT_RIGHT_PUSH)?, //829114a8: RightPush
        expr::LEFT_1277 => c(cond::HAS_INTENT_LEFT_PUSH)?,  //82911508: LeftPush
        expr::WILL_EXPIRE_IN_0_01_NOT_WAIT => c(cond::WILL_EXPIRE_IN_0_01_NOT_WAIT)?, //82911568: PushStart WillExpire
        expr::HAS_INTENT_NEW_PUSH => c(cond::HAS_INTENT_NEW_PUSH)?, //829115c8: HasIntent(NewPush), PushEnd to PushQueued
        expr::INTO_658 => c(cond::HAS_INTENT_BS_GRAB)?,
        expr::HAS_INTENT_MUTE_GRAB => c(cond::HAS_INTENT_MUTE_GRAB)?,
        expr::HAS_INTENT_STALE_GRAB => c(cond::HAS_INTENT_STALE_GRAB)?,
        //8290efb4 /8290f08c: Mute/Stale Out674/681 negate one cached HasIntent.
        expr::OUT_674 => !c(cond::HAS_INTENT_MUTE_GRAB)?,
        expr::OUT_681 => !c(cond::HAS_INTENT_STALE_GRAB)?,
        //8290ef54: Cyc670/677 -> Out, HasFilteredMotionGraphIntent(|TweakY| < 0.2).
        expr::CYC_TO_OUT_TWEAK_Y_ABS_LESS_0_2 => {
            c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_TWEAK_Y_ABS_LESS_0_2)?
        }
        //8291438c /82914458: FingerFlipShuv first, then MuteGrab or StaleGrab.
        expr::MUTE_FINGER_FLIP_SHUV => {
            c(cond::HAS_INTENT_FINGER_FLIP_SHUV)? && c(cond::HAS_INTENT_MUTE_GRAB)?
        }
        expr::STALE_FINGER_FLIP_SHUV => {
            c(cond::HAS_INTENT_FINGER_FLIP_SHUV)? && c(cond::HAS_INTENT_STALE_GRAB)?
        }
        //8290dfcc /8290e02c: PlantingFoot Left/Right, HasIntent(Left/RightFootPlant).
        expr::PLANTING_FOOT_LEFT => c(cond::HAS_INTENT_LEFT_FOOT_PLANT)?,
        expr::PLANTING_FOOT_RIGHT => c(cond::HAS_INTENT_RIGHT_FOOT_PLANT)?,
        //82919378 /829195bc /829196e0 /82919804 /82919928: PlantingFoot guards,
        //PhysFilteredState(ground), the push-trick intent, then NewFootPlant.
        expr::PLANTING_FOOT_604 => {
            c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
                && c(cond::HAS_INTENT_FS_BONELESS)?
                && c(cond::HAS_INTENT_NEW_FOOT_PLANT)?
        }
        expr::PLANTING_FOOT_608 => {
            c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
                && c(cond::HAS_INTENT_BS_BONELESS)?
                && c(cond::HAS_INTENT_NEW_FOOT_PLANT)?
        }
        expr::PLANTING_FOOT_612 => {
            c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
                && c(cond::HAS_INTENT_FS_FAST_PLANT)?
                && c(cond::HAS_INTENT_NEW_FOOT_PLANT)?
        }
        expr::PLANTING_FOOT_616 => {
            c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
                && c(cond::HAS_INTENT_BS_FAST_PLANT)?
                && c(cond::HAS_INTENT_NEW_FOOT_PLANT)?
        }
        expr::PLANTING_FOOT_620 => {
            c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
                && c(cond::HAS_INTENT_NO_COMPLY)?
                && c(cond::HAS_INTENT_NEW_FOOT_PLANT)?
        }
        expr::LEFT_248 => c(cond::HAS_INTENT_BOARD_ADJUST_LEFT)?,
        expr::RIGHT_249 => c(cond::HAS_INTENT_BOARD_ADJUST_RIGHT)?,
        expr::HAS_INTENT_BOARD_ADJUST_UP => c(cond::HAS_INTENT_BOARD_ADJUST_UP)?,
        expr::HAS_INTENT_BOARD_ADJUST_DOWN => c(cond::HAS_INTENT_BOARD_ADJUST_DOWN)?,
        expr::INTO_1247 => c(cond::HAS_INTENT_CRAIL_GRAB)?, //8290f6a4: CrailGrab
        expr::SEQUENCE_690 => c(cond::HAS_ANIM_ATTRIBUTE_SEQUENCE_TO_AIR)?,
        expr::LATE_KICKFLIP_253 => c(cond::HAS_INTENT_L_F_KICKFLIP)?,
        expr::LATE_HEELFLIP_254 => c(cond::HAS_INTENT_L_F_HEELFLIP)?,
        expr::LATE_BACKFOOT_KICKFLIP_255 => c(cond::HAS_INTENT_L_B_KICKFLIP)?,
        expr::LATE_BACKFOOT_HEELFLIP_256 => c(cond::HAS_INTENT_L_B_HEELFLIP)?,
        expr::LATE_BS_SHUV_257 => c(cond::HAS_INTENT_L_BS_SHUVIT)?,
        expr::LATE_FS_SHUV_258 => c(cond::HAS_INTENT_L_FS_SHUVIT)?,
        expr::HAS_INTENT_OB_DISMOUNT_IN_AIR => {
            c(cond::HAS_INTENT_OB_DISMOUNT)? && c(cond::PHYS_FILTERED_STATE_AIR)?
        } //82912dc4: dismount intent before physical state
        expr::RIGHT_FOOT_FORWARD_259 => c(cond::IS_RIDING_GOOFY)?, //8290faf4
        expr::LEFT_FOOT_FORWARD_260 => !c(cond::IS_RIDING_GOOFY)?, //8290fb54
        expr::INTO_RIGHT_ONE_FOOT_AIR => {
            c(cond::HAS_INTENT_RIGHT_ONE_FOOT_AIR)? && c(cond::TIME_TO_LAND_GREATER_0_5)?
        } //829151c8
        expr::INTO_LEFT_ONE_FOOT_AIR => {
            c(cond::HAS_INTENT_LEFT_ONE_FOOT_AIR)? && c(cond::TIME_TO_LAND_GREATER_0_5)?
        } //82915290
        expr::IN_AIR_21_TO_WIPE_OUT_4 => {
            c(cond::IS_BODY_FLIPPING)? && c(cond::PHYSICS_WANTS_RUNOUT)?
        } //82915358
        expr::LAND_22 => {
            c(cond::PHYS_FILTERED_STATE_GROUND)? && !c(cond::HAS_TILT_TO_LARGE_FOR_PRELAND)?
        } //82915424
        expr::TAIL_MANUAL_1176 => c(cond::HAS_INTENT_MANUAL_LESS_0)?, //82910fa0: direct AirStatic priority0 target gate
        expr::NOT_HAS_INTENT_MANUAL_BRAKE => !c(cond::HAS_INTENT_MANUAL_BRAKE)?, //82911000
        expr::LATEFLIP_80 => c(cond::PHYS_FILTERED_STATE_AIR_2ND)? && c(cond::HAS_INTENT_TRICK)?, //82915100: Air before lateflip intent
        expr::INTO_647 => c(cond::HAS_INTENT_FS_GRAB)?, //8290e730: cached HasIntent(FSGrab)
        expr::INTO_642 => {
            c(cond::HAS_INTENT_DBL_GRAB)?
                || c(cond::HAS_INTENT_COFFIN)?
                || c(cond::HAS_INTENT_SUPERMAN)?
        } //82919c8c: native ordered HasIntent OR
        expr::IDLE_687_TO_IN_AIR_21 => {
            c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_0_4)?
                && c(cond::HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_MAG_ABS_LESS_0_4)?
                && !c(cond::HAS_INTENT_BOARD_ADJUST_DOWN)?
        } //8291a23c: filtered angle, magnitude, then Down
        expr::PUSH_TO_ANTIC => {
            c(cond::MONGO_PUSH_FOOT_TO_FAR)?
                && ((c(cond::IS_RIDING_GOOFY_2ND)?
                    && !c(cond::CURRENT_STATE_PUSH_START_RIGHT_DEFAULT)?)
                    || (!c(cond::CURRENT_STATE_PUSH_START_LEFT_DEFAULT)?
                        && !c(cond::IS_RIDING_GOOFY_2ND)?))
        } //8291a7f4: mongo-push foot, stance/state gates
        expr::INTO_1148 => {
            c(cond::HAS_INTENT_BOARD_ADJUST_DOWN)?
                && (c(cond::HAS_INTENT_NOSE_GRAB)? || c(cond::HAS_INTENT_ROCKET_AIR)?)
        } //8291a360: BoardAdjustDown, NoseGrab, RocketAir
        expr::RIGHT_1278 => {
            c(cond::HAS_INTENT_RIGHT_PUSH)?
                || c(cond::CURRENT_STATE_PUSH_FROM_FAKIE)?
                || c(cond::CURRENT_STATE_SWITCH)?
        } //8291a89c
        expr::LEFT_1279 => {
            c(cond::HAS_INTENT_LEFT_PUSH)?
                || c(cond::CURRENT_STATE_PUSH_FROM_FAKIE)?
                || c(cond::CURRENT_STATE_SWITCH)?
        } //8291a9c0
        expr::ANTIC_NOSE_1287 => {
            c(cond::HAS_INTENT_NOLLIE_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION)?
        } //8291d548
        expr::ANTIC_TAIL_1286 => {
            c(cond::HAS_INTENT_OLLIE_ANTICIPATION)?
                || c(cond::HAS_INTENT_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_360_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION)?
        } //8291d374
        expr::ANTIC_TAIL_1183 => {
            !(c(cond::HAS_INTENT_OLLIE_ANTICIPATION)?
                || c(cond::HAS_INTENT_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_360_POP_SHUVIT_ANTICIPATION)?
                || c(cond::HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION)?)
        } //8291cfcc: AnticOut sibling
        expr::DISMOUNT_32 => {
            c(cond::HAS_INTENT_OB_DISMOUNT)?
                && !c(cond::IS_DISMOUNTING)?
                && !c(cond::IS_LANDING)?
                && !c(cond::IS_RUNNING_OUT)?
                && !c(cond::PHYS_FILTERED_STATE_GRIND)?
        } //8291bcf4: Dismount intent and four native exclusion gates
        expr::IDLE_DISMOUNT_101 => {
            c(cond::COM_VEL_COMPARE_ABS_LESS_4)?
                && !c(cond::LAST_STATE_RUN_DISMOUNT)?
                && !c(cond::DISABLE_PUSH_BRAKE)?
        } //82911ca8: IdleDismount velocity, last-state, push/brake gates
        expr::MOUNT_34 => {
            c(cond::HAS_INTENT_OB_MOUNT)?
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?
        } //82917f0c: OB_Mount, then neither retrieving nor dropping
        expr::LOCO_NEW_319_TO_MOUNT_34 => {
            !c(cond::IS_LOCOMOTION_QUIT_NOT_ALLOWED_2ND)?
                && !c(cond::IS_DISMOUNTING_2ND)?
                && !c(cond::IS_LANDING)?
                && !c(cond::IS_RUNNING_OUT)?
        } //82917610
        expr::RUNNING_INTO_MOUNT_FROM_RIGHT_108 => {
            c(cond::HAS_INTENT_OB_MOUNT)?
                && (c(cond::IS_LOCOMOTION_FOOT_DOWN_RIGHT)?
                    || c(cond::CURRENT_STATE_STALL_RUN_INTO_MOUNT_RIGHT)?)
                && !c(cond::DO_SPRINT_MOUNT)?
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?
        } //82917784
        expr::RUNNING_INTO_MOUNT_FROM_LEFT_110 => {
            c(cond::HAS_INTENT_OB_MOUNT)?
                && (c(cond::IS_LOCOMOTION_FOOT_DOWN_LEFT)?
                    || c(cond::CURRENT_STATE_STALL_RUN_INTO_MOUNT_LEFT)?)
                && !c(cond::DO_SPRINT_MOUNT)?
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?
        } //8291799c
        expr::STALL_RUN_INTO_MOUNT_RIGHT_112 => {
            c(cond::IS_LOCOMOTION_FOOT_DOWN_RIGHT)?
                && c(cond::HAS_INTENT_OB_MOUNT)?
                && c(cond::IS_RETRIEVING_SKATEBOARD)?
        } //82917bb4
        expr::STALL_RUN_INTO_MOUNT_LEFT_113 => {
            c(cond::IS_LOCOMOTION_FOOT_DOWN_LEFT)?
                && c(cond::HAS_INTENT_OB_MOUNT)?
                && c(cond::IS_RETRIEVING_SKATEBOARD)?
        } //82917cd4
        expr::STALL_INTO_MOUNT_114 => {
            c(cond::HAS_INTENT_OB_MOUNT)?
                && c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_LOCOMOTION_QUIT_NOT_ALLOWED_2ND)?
        } //82917df4
        expr::PUSH_START_OR_CONTACT_TO_PUSH_END => {
            c(cond::DISABLE_PUSH_BRAKE)?
                || c(cond::HAS_INTENT_GROUND_BS_GRAB)?
                || c(cond::HAS_INTENT_GROUND_FS_GRAB_2ND)?
                || c(cond::HAS_INTENT_GROUND_DBL_GRAB)?
                || c(cond::HAS_INTENT_COFFIN)?
        } //8291ce0c: DisablePushBrake, ground grabs, Coffin
        expr::PUSH_CYCLE_TO_PUSH_END => {
            c(cond::BREAK_OUT_OF_PUSH)?
                || c(cond::DISABLE_PUSH_BRAKE)?
                || c(cond::HAS_INTENT_GROUND_BS_GRAB)?
                || c(cond::HAS_INTENT_GROUND_FS_GRAB_2ND)?
                || c(cond::HAS_INTENT_GROUND_DBL_GRAB)?
                || c(cond::HAS_INTENT_COFFIN)?
        } //8291d938: BreakOutOfPush, then same lazy five-condition tail as438
        expr::RIGHT_1278_TO_FOOT_PLANT_621 => {
            c(cond::IS_RIDING_GOOFY)? && c(cond::HAS_GESTURE_INTENT_SQUARE)?
        } //82916368: IsRidingGoofy before HasGestureIntent
        expr::LEFT_1279_TO_FOOT_PLANT_621 => {
            c(cond::HAS_GESTURE_INTENT_SQUARE)? && !c(cond::IS_RIDING_GOOFY)?
        } //82916434: gesture first, then not goofy
        expr::PUSH_1028_TO_ANTICIPATION_1030 => {
            !c(cond::MONGO_PUSH_FOOT_TO_FAR)?
                && ((c(cond::IS_RIDING_GOOFY_2ND)?
                    && !c(cond::CURRENT_STATE_PUSH_START_RIGHT_DEFAULT)?)
                    || (!c(cond::CURRENT_STATE_PUSH_START_LEFT_DEFAULT)?
                        && !c(cond::IS_RIDING_GOOFY_2ND)?))
        } //82916198: non-far mongo foot, same stance/state tail as Pre415
        expr::DISMOUNT_36 => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR)? && c(cond::HAS_INTENT_OB_DISMOUNT)?
        }
        expr::RIGHT_FOOT_STRIDE_37 => {
            (c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR)?
                && c(cond::HAS_INTENT_OFF_BOARD_AIR)?
                && ((c(cond::IS_MIRRORED)? && c(cond::HAS_ANIM_ATTRIBUTE_LOCO_STARTING_FOOT)?)
                    || (c(cond::HAS_ANIM_ATTRIBUTE_LOCO_STARTING_FOOT)? && !c(cond::IS_MIRRORED)?)))
                || (c(cond::HAS_INTENT_OB_MOUNT)?
                    && (c(cond::CURRENT_STATE_KICKOUT_TO_AIR)?
                        || c(cond::CURRENT_STATE_TRICK_DISMOUNT)?
                        || c(cond::CURRENT_STATE_ACCIDENTAL_DISMOUNT_FRONT_FLY)?
                        || c(cond::CURRENT_STATE_ACCIDENTAL_DISMOUNT_FRONT_FINISH)?))
        }
        //8291ae9c /8291b0b4: left/right sprint mounts; native cache order.
        expr::SPRINTING_INTO_MOUNT_FROM_RIGHT_109 => {
            c(cond::HAS_INTENT_OB_MOUNT)?
                && c(cond::DO_SPRINT_MOUNT)?
                && (c(cond::IS_LOCOMOTION_FOOT_DOWN_RIGHT)?
                    || c(cond::CURRENT_STATE_STALL_RUN_INTO_MOUNT_RIGHT)?)
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?
        }
        expr::SPRINTING_INTO_MOUNT_FROM_LEFT_111 => {
            c(cond::HAS_INTENT_OB_MOUNT)?
                && c(cond::DO_SPRINT_MOUNT)?
                && (c(cond::IS_LOCOMOTION_FOOT_DOWN_LEFT)?
                    || c(cond::CURRENT_STATE_STALL_RUN_INTO_MOUNT_LEFT)?)
                && !c(cond::IS_RETRIEVING_SKATEBOARD)?
                && !c(cond::IS_DROPPING_SKATEBOARD)?
        }
        //8291b2cc: both mirror arms require no LocoStartingFoot (a 2nd IsMirrored read, not XOR).
        expr::LEFT_FOOT_STRIDE_38 => {
            c(cond::PHYS_FILTERED_STATE_OFFBOARD_AIR)?
                && c(cond::HAS_INTENT_OFF_BOARD_AIR)?
                && ((c(cond::IS_MIRRORED)? && !c(cond::HAS_ANIM_ATTRIBUTE_LOCO_STARTING_FOOT)?)
                    || (!c(cond::IS_MIRRORED)? && !c(cond::HAS_ANIM_ATTRIBUTE_LOCO_STARTING_FOOT)?))
        }
        expr::PHYSICS_WANTS_RUNOUT => c(cond::PHYSICS_WANTS_RUNOUT)?,
        expr::GRINDING_18 => c(cond::PHYS_FILTERED_STATE_GRIND_2ND)?,
        expr::TRICK_19 => {
            c(cond::HAS_INTENT_TRICK)?
                && c(cond::ALLOWED_TO_TRICK)?
                && c(cond::OK_TO_DO_TRICK_ON_STAIRS)?
                && (c(cond::PHYS_FILTERED_STATE_GROUND)? || c(cond::PHYS_FILTERED_STATE_GRIND)?)
                && !c(cond::IS_GRABBING_OBJECT_2ND)?
        }
        expr::CYC_TO_WIPE_OUT_TWEAK_0_4 => {
            c(cond::PHYSICS_WANTS_WIPE_OUT)?
                || c(cond::PHYSICS_WANTS_RUNOUT)?
                || (c(cond::HAS_TWEAK_GREATER_EQUAL_0_4)?
                    && (c(cond::TRUCKS_OR_DECK_IN_CONTACT)? || !c(cond::PHYS_FILTERED_STATE_AIR)?))
        } //8291c998: wipeout, runout, tweak/contact/air
        //8291b7f8: DBLGrab Cyc643 -> WipeOut4: wipeout, then tweak with contact or not air.
        expr::CYC_643_TO_WIPE_OUT_4 => {
            c(cond::PHYSICS_WANTS_WIPE_OUT)?
                || (c(cond::HAS_TWEAK_GREATER_EQUAL_0_8)?
                    && (c(cond::TRUCKS_OR_DECK_IN_CONTACT)? || !c(cond::PHYS_FILTERED_STATE_AIR)?))
        }
        //8290f224: Cycle1143 guard, cached HasIntent(TailGrab), direct return.
        expr::CYCLE_1143 => c(cond::HAS_INTENT_TAIL_GRAB_2ND)?,
        //8290e598 /8290e5f8: Superman Into and Out guards, one cached HasIntent(Superman).
        expr::SUPERMAN_INTO_953 => c(cond::HAS_INTENT_SUPERMAN)?,
        expr::SUPERMAN_OUT_955 => !c(cond::HAS_INTENT_SUPERMAN)?,
        //8290e868: one-foot Into -> EarlyOut, cached HasAnimAttribute(OneFootEarlyOut).
        expr::HAS_ANIM_ATTRIBUTE_ONE_FOOT_EARLY_OUT => {
            c(cond::HAS_ANIM_ATTRIBUTE_ONE_FOOT_EARLY_OUT)?
        }
        //8290e940 /8290e9a0: Footplant guard and Into -> Absorb, one cached read each.
        expr::FOOTPLANT => c(cond::SHOULD_PREPARE_ONE_FOOT_AIR_FOR_FOOTPLANT)?,
        expr::INTO_TO_ABSORB => c(cond::IS_FOOT_PLANTING)?,
        //8290e8c8 /8290ea00 /8290f284 /8290f434: one-foot, no-foot and tail-grab Outs
        //negate one cached HasIntent.
        expr::OUT_NOT_HAS_INTENT_RIGHT_ONE_FOOT_AIR => !c(cond::HAS_INTENT_RIGHT_ONE_FOOT_AIR)?,
        expr::OUT_NOT_HAS_INTENT_LEFT_ONE_FOOT_AIR => !c(cond::HAS_INTENT_LEFT_ONE_FOOT_AIR)?,
        expr::OUT_NOT_HAS_INTENT_NO_FOOT_AIR_WALK => !c(cond::HAS_INTENT_NO_FOOT_AIR_WALK)?,
        expr::OUT_990 => !c(cond::HAS_INTENT_TAIL_GRAB)?,
        //82914780 /8291484c /82914918: TailGrab first, then the foot intent.
        expr::INTO_1229 => c(cond::HAS_INTENT_TAIL_GRAB)? && c(cond::HAS_INTENT_NO_FOOT_AIR_WALK)?,
        expr::INTO_1296 => c(cond::HAS_INTENT_TAIL_GRAB)? && c(cond::HAS_INTENT_LEFT_ONE_FOOT_AIR)?,
        expr::INTO_1299 => {
            c(cond::HAS_INTENT_TAIL_GRAB)? && c(cond::HAS_INTENT_RIGHT_ONE_FOOT_AIR)?
        }
        //8290bf9c: HippyRunout16 -> WipeOut4, cached OBTimeToLand(lessEqual 0.05).
        expr::HIPPY_RUNOUT_16_TO_WIPE_OUT_4 => c(cond::OB_TIME_TO_LAND_LESS_EQUAL_0_05)?,
        //8290f644: Cycle1149 guard, cached HasIntent(NoseGrab), direct return.
        expr::CYCLE_1149 => c(cond::HAS_INTENT_NOSE_GRAB_2ND)?,
        //8290f50c /8290f56c /8290f5cc: RocketAir Into, Cycle and Out, one cached HasIntent.
        expr::INTO_992 => c(cond::HAS_INTENT_ROCKET_AIR)?,
        expr::CYCLE_993 => c(cond::HAS_INTENT_ROCKET_AIR_2ND)?,
        expr::OUT_994 => !c(cond::HAS_INTENT_ROCKET_AIR)?,
        //8290f7dc: nose-grab Out996, cached HasIntent(NoseGrab) negated.
        expr::OUT_996 => !c(cond::HAS_INTENT_NOSE_GRAB)?,
        //82914c40 /82914d0c /82914dd8: NoseGrab first, then the foot intent.
        expr::INTO_1240 => c(cond::HAS_INTENT_NOSE_GRAB)? && c(cond::HAS_INTENT_NO_FOOT_AIR_WALK)?,
        expr::INTO_1302 => c(cond::HAS_INTENT_NOSE_GRAB)? && c(cond::HAS_INTENT_LEFT_ONE_FOOT_AIR)?,
        expr::INTO_1305 => {
            c(cond::HAS_INTENT_NOSE_GRAB)? && c(cond::HAS_INTENT_RIGHT_ONE_FOOT_AIR)?
        }
        //82911868 /829118c8 /82911928: hippy Anticipation timer, ApexReached, ground state.
        expr::ANTICIPATION_91_TO_ANTIC_CYCLE_92 => {
            c(cond::IN_STATE_FOR_TIME_ANTICIPATION_GREATER_0_49)?
        }
        expr::TO_LANDING_ON_BOARD => c(cond::APEX_REACHED)?,
        expr::CYC_94_TO_WIPE_OUT_4 => c(cond::PHYS_FILTERED_STATE_GROUND)?,
        //829168f8: Into93 guard; RightPush absent short-circuits true before LeftPush.
        expr::INTO_93 => !c(cond::HAS_INTENT_RIGHT_PUSH)? || !c(cond::HAS_INTENT_LEFT_PUSH)?,
        //829169c4: WillExpire(0.1), then neither PhysFilteredState ground nor offboard.
        expr::INTO_93_TO_WIPE_OUT_4 => {
            c(cond::WILL_EXPIRE_IN_0_1)?
                && !c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?
                && !c(cond::PHYS_FILTERED_STATE_OFFBOARD_2ND)?
        }
        //8291cae8: Mute/Stale Cyc670/677 -> WipeOut4: wipeout, runout, then
        //HasTweak(>=0.2) with contact or not air through the shared 426 tail.
        expr::CYC_TO_WIPE_OUT_TWEAK_0_2 => {
            c(cond::PHYSICS_WANTS_WIPE_OUT)?
                || c(cond::PHYSICS_WANTS_RUNOUT)?
                || (c(cond::HAS_TWEAK_GREATER_EQUAL_0_2)?
                    && (c(cond::TRUCKS_OR_DECK_IN_CONTACT)? || !c(cond::PHYS_FILTERED_STATE_AIR)?))
        }
        expr::IS_PRO_SKATER_MIKE_CARROLL => c(cond::IS_PRO_SKATER_MIKE_CARROLL)?, //8290d1dc: IsProSkater(MikeCarroll)
        expr::HAS_INTENT_KICKFLIP => c(cond::HAS_INTENT_KICKFLIP)?, //8290d23c: HasIntent(Kickflip)
        expr::WILL_EXPIRE_IN_0_05_NOT_WAIT => c(cond::WILL_EXPIRE_IN_0_05_NOT_WAIT)?, //8290d29c: WillExpire(InTime=0.05)
        expr::IS_PRO_SKATER_MARK_GONZALES => c(cond::IS_PRO_SKATER_MARK_GONZALES)?, //8290d3bc: IsProSkater(MarkGonzales)
        expr::OLLIE_188 => c(cond::HAS_INTENT_OLLIE)?, //8290d41c: HasIntent(Ollie)
        expr::TO_BOARD_ADJUSTS => c(cond::HAS_ANIM_ATTRIBUTE_CAN_BRANCH)?, //8290d47c: HasAnimAttribute(canBranch)
        // Held Kickflip cycle exits. The native Pre switch checks landing
        // urgency before body flipping, then negates KickflipHold last.
        expr::IN_520 => c(cond::HAS_INTENT_HEELFLIP)?, //8290d4dc: Heelflip
        // Earlier authored candidates are evaluated before the nollie flip entries.
        // Native Pre cases95..107, 0x8290d53c..0x8290d9bc: one cached HasIntent each.
        expr::POP_SHUVIT_191 => c(cond::HAS_INTENT_POP_SHUVIT)?,
        expr::FS_POP_SHUVIT_192 => c(cond::HAS_INTENT_FS_POP_SHUVIT)?,
        expr::VARIAL_KICKFLIP_193 => c(cond::HAS_INTENT_VARIAL_KICKFLIP)?,
        expr::VARIAL_HEELFLIP_194 => c(cond::HAS_INTENT_VARIAL_HEELFLIP)?,
        expr::HARDFLIP_195 => c(cond::HAS_INTENT_HARDFLIP)?,
        expr::INWARD_HEELFLIP_196 => c(cond::HAS_INTENT_INWARD_HEELFLIP)?,
        expr::STATE_360_POP_SHUVIT_197 => c(cond::HAS_INTENT_360_POP_SHUVIT)?,
        expr::FS_360_POP_SHUVIT_198 => c(cond::HAS_INTENT_FS_360_POP_SHUVIT)?,
        expr::STATE_360_FLIP_199 => c(cond::HAS_INTENT_360_FLIP)?,
        expr::LASERFLIP_200 => c(cond::HAS_INTENT_LASERFLIP)?,
        expr::STATE_360_HARDFLIP_201 => c(cond::HAS_INTENT_360_HARDFLIP)?,
        expr::STATE_360_INWARD_HEELFLIP_202 => c(cond::HAS_INTENT_360_INWARD_HEELFLIP)?,
        expr::NOLLIE_203 => c(cond::HAS_INTENT_NOLLIE)?,
        expr::IN_562 => c(cond::HAS_INTENT_N_KICKFLIP)?, //8290da1c: N_Kickflip
        expr::IN_565 => c(cond::HAS_INTENT_N_HEELFLIP)?, //8290da7c: N_Heelflip
        expr::OUT_1_1087 => {
            c(cond::TIME_TO_LAND_LESS_0_525)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_HEELFLIP_HOLD)?
        } //82918ca0
        expr::OUT_2_1088 => {
            c(cond::TIME_TO_LAND_LESS_0_8)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_HEELFLIP_HOLD)?
        } //82918dc4
        expr::OUT_3_1089 => c(cond::IS_BODY_FLIPPING)? || !c(cond::HAS_INTENT_HEELFLIP_HOLD)?, //829131bc
        expr::OUT_1_1095 => {
            c(cond::TIME_TO_LAND_LESS_0_525)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_N_KICKFLIP_HOLD)?
        } //82918ee8
        expr::OUT_2_1096 => {
            c(cond::TIME_TO_LAND_LESS_0_8)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_N_KICKFLIP_HOLD)?
        } //8291900c
        expr::OUT_3_1097 => c(cond::IS_BODY_FLIPPING)? || !c(cond::HAS_INTENT_N_KICKFLIP_HOLD)?, //82913288
        expr::OUT_1_1103 => {
            c(cond::TIME_TO_LAND_LESS_0_525)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_N_HEELFLIP_HOLD)?
        } //82919130
        expr::OUT_2_1104 => {
            c(cond::TIME_TO_LAND_LESS_0_8)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_N_HEELFLIP_HOLD)?
        } //82919254
        expr::OUT_3_1105 => c(cond::IS_BODY_FLIPPING)? || !c(cond::HAS_INTENT_N_HEELFLIP_HOLD)?, //82913354
        expr::OUT_1_1079 => {
            c(cond::TIME_TO_LAND_LESS_0_525)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_KICKFLIP_HOLD)?
        } //82918a58: Out1
        expr::OUT_2_1080 => {
            c(cond::TIME_TO_LAND_LESS_0_8)?
                || c(cond::IS_BODY_FLIPPING)?
                || !c(cond::HAS_INTENT_KICKFLIP_HOLD)?
        } //82918b7c: Out2
        expr::OUT_3_1081 => c(cond::IS_BODY_FLIPPING)? || !c(cond::HAS_INTENT_KICKFLIP_HOLD)?, //829130f0: Out3
        expr::WILL_EXPIRE_IN_0_05 => c(cond::WILL_EXPIRE_IN_0_05)?, //8290b598: WillExpire InTime=0.05
        expr::FROM_MANUAL => {
            c(cond::LAST_STATE_RIDING_TURNING_MANUAL)? || c(cond::MANUAL_OUT_TIMER_IS_ACTIVE)?
        } //82912f58: LastState then ManualOutTimer
        expr::FROM_GRIND => c(cond::LAST_STATE_ON_BOARD_GRINDING)?, //8290d35c: LastState
        expr::TRICK_19_TO_ACCIDENTAL_DISMOUNT_17 => {
            c(cond::PHYSICS_WANTS_RUNOUT)?
                && (c(cond::PHYS_FILTERED_STATE_GROUND)? || c(cond::PHYS_FILTERED_STATE_GRIND)?)
        } //8291880c: Runout then Ground/Grind
        expr::TRICK_19_TO_HIPPY_RUNOUT_16 => {
            c(cond::PHYSICS_WANTS_RUNOUT)? && c(cond::PHYS_FILTERED_STATE_AIR)?
        } //82912e8c: cached Runout then Air
        // 8291ddcc: native lazy OR across the twelve specific grind names.
        expr::GRIND_OUT_ASSIST => {
            c(cond::IS_GRINDING_BS_LIP_2ND)?
                || c(cond::IS_GRINDING_FS_LIP_2ND)?
                || c(cond::IS_GRINDING_BS_BOARD_2ND)?
                || c(cond::IS_GRINDING_FS_BOARD_2ND)?
                || c(cond::IS_GRINDING_BS_FEEBLE_2ND)?
                || c(cond::IS_GRINDING_FS_FEEBLE_2ND)?
                || c(cond::IS_GRINDING_BS_SMITH_2ND)?
                || c(cond::IS_GRINDING_FS_SMITH_2ND)?
                || c(cond::IS_GRINDING_BS_WILLY_2ND)?
                || c(cond::IS_GRINDING_FS_WILLY_2ND)?
                || c(cond::IS_GRINDING_BS_OVERWILLY_2ND)?
                || c(cond::IS_GRINDING_FS_OVERWILLY_2ND)?
        }
        expr::TO_RIDING => c(cond::HAS_NEW_HAND_PLANT_POS)?, //82910894: HasNewHandPlantPos
        expr::FROM_GROUND_ANTIC => !c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_INTO)?, //8290e0ec
        expr::HAND_PLANT_INTO => c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_INTO)?,
        expr::WILL_EXPIRE_IN_0_15 => c(cond::WILL_EXPIRE_IN_0_15)?,
        expr::HAND_PLANT_ANTIC => c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_ANTIC)?,
        expr::HAND_PLANT_OUT => c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_OUT)?,
        expr::HAND_PLANT_DISMOUNT_INTO => c(cond::HAS_INTENT_HAND_PLANT_DISMOUNT)?,
        expr::HAND_PLANT_ONE_FOOT_RIGHT_INTO => {
            (c(cond::IS_RIDING_GOOFY)? && c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_RIGHT)?)
                || (c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_LEFT)? && !c(cond::IS_RIDING_GOOFY)?)
        }
        expr::HAND_PLANT_ONE_FOOT_LEFT_INTO => {
            (c(cond::IS_RIDING_GOOFY)? && c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_LEFT)?)
                || (c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_RIGHT)? && !c(cond::IS_RIDING_GOOFY)?)
        }
        expr::HAND_PLANT_DISMOUNT_OUT => {
            c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_OUT)? || !c(cond::HAS_INTENT_HAND_PLANT_DISMOUNT)?
        }
        expr::HAND_PLANT_ONE_FOOT_RIGHT_OUT => {
            c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_OUT)?
                || !(c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_RIGHT)?
                    || c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_LEFT)?)
        }
        expr::HAND_PLANT_ONE_FOOT_LEFT_OUT => {
            c(cond::SHOULD_PLAY_HAND_PLANT_ANIM_OUT)?
                || !(c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_LEFT)?
                    || c(cond::HAS_INTENT_HAND_PLANT_ONE_FOOT_RIGHT)?)
        }
        expr::FS_75 => c(cond::IS_HAND_PLANTING_FS_AIR)?,
        expr::BS_76 => c(cond::IS_HAND_PLANTING_BS_AIR)?,
        expr::PHYS_FILTERED_STATE_AIR => c(cond::PHYS_FILTERED_STATE_AIR_2ND)?,
        expr::FS_GRAB_INTO_1260 => c(cond::HAS_INTENT_GROUND_FS_GRAB_2ND)?, //82910d44: GroundFSGrab
        expr::DBL_GRAB_INTO_1265 => {
            c(cond::HAS_INTENT_GROUND_DBL_GRAB)? || c(cond::HAS_INTENT_COFFIN)?
        } //829160d0: GroundDBLGrab or Coffin
        expr::HAS_INTENT_GROUND_BS_GRAB_INTO => c(cond::HAS_INTENT_GROUND_BS_GRAB)?, //8290fe20: GroundBSGrab
        expr::BUMPED_276 => c(cond::IS_BUMPED)? && c(cond::SURFACE_SLOPE_LESS_70)?, //82915820: IsBumped then lazy SurfaceSlope
        expr::ON_GROUND_24 => c(cond::PHYS_FILTERED_STATE_GROUND_2ND)?,
        expr::IS_ENTERING_SKITCH => c(cond::IS_ENTERING_SKITCH)?,
        expr::TO_RIDING_IDLE => !c(cond::IS_ENTERING_SKITCH_2ND)?, //8290ff40: lost entry prediction
        expr::SKITCHING_88 => c(cond::IS_SKITCHING)?,
        expr::SKITCH_2_HAND_272 => c(cond::SKITCHING_POSITION_BOTH)?, //82910018: two hands
        expr::ABSORB_700 => c(cond::IS_SKITCHING_WITH_ABSORB)?, //82910078: absorb contact speed
        expr::SHIMMYING_LEFT_1011 => c(cond::IS_SKITCH_SHIMMYING_LEFT)?, //829100d8: shimmy left
        expr::SHIMMYING_RIGHT_1012 => c(cond::IS_SKITCH_SHIMMYING_RIGHT)?, //82910138: shimmy right
        expr::IDLE_1013 => {
            !c(cond::IS_SKITCH_SHIMMYING_RIGHT)? && !c(cond::IS_SKITCH_SHIMMYING_LEFT)?
        } //82910198: right first, then lazy left
        expr::SKITCH_1_HAND_FS_273 => c(cond::SKITCHING_POSITION_FRONT)?, //82910264: front hand
        expr::SKITCH_1_HAND_BS_274 => c(cond::SKITCHING_POSITION_BACK)?, //829102c4: back hand
        expr::ABSORB_700_TO_HOLDING_701 => {
            c(cond::WILL_EXPIRE_IN_0_5)? && !c(cond::IS_SKITCHING_WITH_ABSORB)?
        } //82915688: expiry first, then absorb finished
        expr::START_ANTICIPATION_90 => {
            c(cond::HAS_INTENT_NEW_PUSH_2ND)?
                && c(cond::HAS_INTENT_RIGHT_PUSH_2ND)?
                && c(cond::HAS_INTENT_LEFT_PUSH_2ND)?
        }
        expr::TELEPORT_5 => c(cond::PHYS_FILTERED_STATE_TELEPORT_2ND)?,
        expr::MOTION_0_TO_TELEPORT_5 => {
            c(cond::PHYS_FILTERED_STATE_TELEPORT)? && !c(cond::CURRENT_STATE_TELEPORT)?
        }
        // Grinding18 children 8290c1dc..8290d17c read one cached condition each;
        // 68, 77 and 80 are the negated forms.
        expr::BS_FEEBLE_40 => c(cond::IS_GRINDING_BS_FEEBLE)?, //8290c1dc: IsGrinding(BS_FEEBLE)
        expr::FS_FEEBLE_41 => c(cond::IS_GRINDING_FS_FEEBLE)?, //8290c238: IsGrinding(FS_FEEBLE)
        expr::BS_SMITH_42 => c(cond::IS_GRINDING_BS_SMITH)?,   //8290c298: IsGrinding(BS_SMITH)
        expr::FS_SMITH_43 => c(cond::IS_GRINDING_FS_SMITH)?,   //8290c2f8: IsGrinding(FS_SMITH)
        expr::FS_WILLY_44 => c(cond::IS_GRINDING_FS_WILLY)?,   //8290c358: IsGrinding(FS_WILLY)
        expr::BS_WILLY_45 => c(cond::IS_GRINDING_BS_WILLY)?,   //8290c3b8: IsGrinding(BS_WILLY)
        expr::FS_OVERWILLY_46 => c(cond::IS_GRINDING_FS_OVERWILLY)?, //8290c418: IsGrinding(FS_OVERWILLY)
        expr::BS_OVERWILLY_47 => c(cond::IS_GRINDING_BS_OVERWILLY)?, //8290c478: IsGrinding(BS_OVERWILLY)
        expr::BS_5_O_48 => c(cond::IS_GRINDING_BS_5_O)?,             //8290c4d8: IsGrinding(BS_5_O)
        expr::FS_5_O_49 => c(cond::IS_GRINDING_FS_5_O)?,             //8290c538: IsGrinding(FS_5_O)
        expr::BS_50_50_50 => c(cond::IS_GRINDING_BS_50_50)?, //8290c598: IsGrinding(BS_50_50)
        expr::FS_50_50_51 => c(cond::IS_GRINDING_FS_50_50)?, //8290c5f8: IsGrinding(FS_50_50)
        expr::BS_CROOK_52 => c(cond::IS_GRINDING_BS_CROOK)?, //8290c658: IsGrinding(BS_CROOK)
        expr::FS_CROOK_53 => c(cond::IS_GRINDING_FS_CROOK)?, //8290c6b8: IsGrinding(FS_CROOK)
        expr::FS_OVERCROOK_54 => c(cond::IS_GRINDING_FS_OVERCROOK)?, //8290c718: IsGrinding(FS_OVERCROOK)
        expr::BS_OVERCROOK_55 => c(cond::IS_GRINDING_BS_OVERCROOK)?, //8290c778: IsGrinding(BS_OVERCROOK)
        expr::BS_NOSEGRIND_56 => c(cond::IS_GRINDING_BS_NOSEGRIND)?, //8290c7d8: IsGrinding(BS_NOSEGRIND)
        expr::FS_NOSEGRIND_57 => c(cond::IS_GRINDING_FS_NOSEGRIND)?, //8290c838: IsGrinding(FS_NOSEGRIND)
        expr::BS_SALAD_58 => c(cond::IS_GRINDING_BS_SALAD)?, //8290c898: IsGrinding(BS_SALAD)
        expr::FS_SALAD_59 => c(cond::IS_GRINDING_FS_SALAD)?, //8290c8f8: IsGrinding(FS_SALAD)
        expr::BS_NOSESLIDE_60 => c(cond::IS_GRINDING_BS_NOSESLIDE)?, //8290c958: IsGrinding(BS_NOSESLIDE)
        expr::FS_NOSESLIDE_61 => c(cond::IS_GRINDING_FS_NOSESLIDE)?, //8290c9b8: IsGrinding(FS_NOSESLIDE)
        expr::BS_NOSEBLUNT_62 => c(cond::IS_GRINDING_BS_NOSEBLUNT)?, //8290ca18: IsGrinding(BS_NOSEBLUNT)
        expr::BACKSLASH => c(cond::IS_GRIND_BLUNTING_BACKSLASH)?, //8290ca78: IsGrindBluntingBackslash
        expr::RAIL => !c(cond::IS_GRIND_BLUNTING_BACKSLASH)?, //8290cad8: IsGrindBluntingBackslash
        expr::FS_NOSEBLUNT_63 => c(cond::IS_GRINDING_FS_NOSEBLUNT)?, //8290cb50: IsGrinding(FS_NOSEBLUNT)
        expr::BS_BLUNT_64 => c(cond::IS_GRINDING_BS_BLUNT)?, //8290cbb0: IsGrinding(BS_BLUNT)
        expr::FS_BLUNT_65 => c(cond::IS_GRINDING_FS_BLUNT)?, //8290cc0c: IsGrinding(FS_BLUNT)
        expr::FS_TAILSLIDE_66 => c(cond::IS_GRINDING_FS_TAILSLIDE)?, //8290cc6c: IsGrinding(FS_TAILSLIDE)
        expr::BS_TAILSLIDE_67 => c(cond::IS_GRINDING_BS_TAILSLIDE)?, //8290cccc: IsGrinding(BS_TAILSLIDE)
        expr::BS_BOARD_68 => c(cond::IS_GRINDING_BS_BOARD)?, //8290cd2c: IsGrinding(BS_BOARD)
        expr::INTO_HAS_INTENT_GRIND_FS_GRAB => c(cond::HAS_INTENT_GRIND_FS_GRAB)?, //8290cd8c: HasIntent(GrindFSGrab)
        expr::GRAB_CYCLE_HAS_INTENT_GRIND_FS_GRAB => c(cond::HAS_INTENT_GRIND_FS_GRAB_2ND)?, //8290cdec: HasIntent(GrindFSGrab)
        expr::OUT_NOT_HAS_INTENT_GRIND_FS_GRAB => !c(cond::HAS_INTENT_GRIND_FS_GRAB)?, //8290ce4c: HasIntent(GrindFSGrab)
        expr::INTO_HAS_INTENT_GRIND_BS_GRAB => c(cond::HAS_INTENT_GRIND_BS_GRAB)?, //8290cec4: HasIntent(GrindBSGrab)
        expr::GRAB_CYCLE_HAS_INTENT_GRIND_BS_GRAB => c(cond::HAS_INTENT_GRIND_BS_GRAB_2ND)?, //8290cf24: HasIntent(GrindBSGrab)
        expr::OUT_NOT_HAS_INTENT_GRIND_BS_GRAB => !c(cond::HAS_INTENT_GRIND_BS_GRAB)?, //8290cf84: HasIntent(GrindBSGrab)
        expr::INTO_HAS_INTENT_GRIND_DOUBLE_GRAB => c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?, //8290cffc: HasIntent(GrindDoubleGrab)
        expr::GRAB_CYCLE_HAS_INTENT_GRIND_DOUBLE_GRAB => c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB_2ND)?, //8290d05c: HasIntent(GrindDoubleGrab)
        expr::FS_BOARD_69 => c(cond::IS_GRINDING_FS_BOARD)?, //8290d0bc: IsGrinding(FS_BOARD)
        expr::BS_LIP_70 => c(cond::IS_GRINDING_BS_LIP)?,     //8290d11c: IsGrinding(BS_LIP)
        expr::FS_LIP_71 => c(cond::IS_GRINDING_FS_LIP)?,     //8290d17c: IsGrinding(FS_LIP)
        // Grind grab Into/Cycle/Out leaves 82912830..82912cf8 and 8291836c..829186e4:
        // HasIntent(GrindFSGrab/GrindBSGrab/GrindDoubleGrab), CurrentGrabType and
        // PhysFilteredState(air), each lazily in native read order.
        expr::HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB => {
            c(cond::HAS_INTENT_GRIND_FS_GRAB)? || c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?
        } //82912830
        expr::NOT_HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB => {
            !c(cond::HAS_INTENT_GRIND_FS_GRAB)? && !c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?
        } //829128fc
        expr::HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB => {
            c(cond::HAS_INTENT_GRIND_BS_GRAB)? || c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?
        } //829129c8
        expr::NOT_HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB => {
            !c(cond::HAS_INTENT_GRIND_BS_GRAB)? && !c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?
        } //82912a94
        expr::GRAB_CYCLE_DIRECT_FS_GRAB_TAIL => {
            c(cond::HAS_INTENT_GRIND_FS_GRAB_2ND)? && c(cond::CURRENT_GRAB_TYPE_TAIL)?
        } //82912b60
        expr::GRAB_CYCLE_DIRECT_BS_GRAB_NOSE => {
            c(cond::HAS_INTENT_GRIND_BS_GRAB_2ND)? && c(cond::CURRENT_GRAB_TYPE_NOSE)?
        } //82912c2c
        expr::GRIND_DOUBLE_GRAB_OUT => {
            c(cond::PHYS_FILTERED_STATE_AIR)? || !c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?
        } //82912cf8
        expr::GRAB_CYCLE_DIRECT_FS => {
            c(cond::CURRENT_GRAB_TYPE_FS)?
                && (c(cond::HAS_INTENT_GRIND_FS_GRAB)? || c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?)
        } //8291836c
        expr::GRAB_CYCLE_DIRECT_NOSE => {
            c(cond::CURRENT_GRAB_TYPE_NOSE)?
                && (c(cond::HAS_INTENT_GRIND_BS_GRAB)? || c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?)
        } //82918494
        expr::GRAB_CYCLE_DIRECT_BS => {
            c(cond::CURRENT_GRAB_TYPE_BS)?
                && (c(cond::HAS_INTENT_GRIND_BS_GRAB)? || c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?)
        } //82918594
        expr::GRAB_CYCLE_DIRECT_TAIL => {
            c(cond::CURRENT_GRAB_TYPE_TAIL)?
                && (c(cond::HAS_INTENT_GRIND_FS_GRAB)? || c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB)?)
        } //8291863c
        expr::GRAB_CYCLE_DIRECT_DOUBLE => {
            c(cond::HAS_INTENT_GRIND_DOUBLE_GRAB_2ND)?
                && (c(cond::CURRENT_GRAB_TYPE_TAIL)? || c(cond::CURRENT_GRAB_TYPE_NOSE)?)
        } //829186e4
        expr::WILL_EXPIRE_IN_0_02_NOT_WAIT => c(cond::WILL_EXPIRE_IN_0_02_NOT_WAIT)?, //8290d2fc: WillExpire
        expr::RAND_PJ_LADD_707 => c(cond::IS_PRO_SKATER_PJ_LADD)?, //82910324: IsProSkater
        expr::RAND_DILL_710 => c(cond::IS_PRO_SKATER_JASON_DILL)?, //82910384: IsProSkater
        expr::RAND_HSU_711 => c(cond::IS_PRO_SKATER_JERRY_HSU)?,   //829103e4: IsProSkater
        expr::RANDOM_COND => c(cond::RANDOM_COND)?,                //82910444: RandomCond
        expr::STILL_1171 => c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_0_1)?, //829109b4: PhysicsSpeedCompare
        expr::STILL_1171_TO_RANDOM_275 => c(cond::IN_PARENT_STATE_FOR_TIME_GREATER_3)?, //82910a14: InParentStateForTime
        expr::TRICK_CYC_2_TO_TRICK_OUT => {
            c(cond::WILL_EXPIRE_IN_0_02_NOT_WAIT)? && !c(cond::HAS_INTENT_KICKFLIP_HOLD)?
        } //82913024: WillExpire, HasIntent
        expr::RAND_CARROLL_1_708 => c(cond::IS_PRO_SKATER_MIKE_CARROLL)? && c(cond::RANDOM_COND)?, //82915754: IsProSkater, RandomCond
        expr::TRICK_CYC_1_TO_TRICK_OUT => {
            c(cond::WILL_EXPIRE_IN_0_02_NOT_WAIT)?
                && (c(cond::TIME_TO_LAND_LESS_0_6)? || !c(cond::HAS_INTENT_KICKFLIP_HOLD)?)
        } //82918930: WillExpire, TimeToLand, HasIntent
        expr::RANDOM_275 => {
            c(cond::PHYSICS_SPEED_COMPARE_ABS_LESS_0_1)?
                && c(cond::SKATE_SLOPE_LESS_5)?
                && c(cond::HAS_INTENT_NO_INPUT)?
        } //8291a488: speed, SkateSlope, HasIntent
        expr::LEFT_GROUND_TO_TRICK_CYC_1_505 => {
            c(cond::WILL_EXPIRE_IN_0_02_NOT_WAIT)?
                && c(cond::HAS_INTENT_KICKFLIP_HOLD)?
                && c(cond::TIME_TO_LAND_GREATER_0_5)?
                && !c(cond::IS_BODY_FLIPPING)?
        } //8291b504: TimeToLand, IsBodyFlipping
        expr::LEFT_GROUND_TO_TRICK_CYC_1_510 => {
            c(cond::WILL_EXPIRE_IN_0_02_NOT_WAIT)?
                && c(cond::HAS_INTENT_KICKFLIP_HOLD)?
                && c(cond::TIME_TO_LAND_GREATER_0_45)?
                && !c(cond::IS_BODY_FLIPPING)?
        } //8291b67c
        _ => return super::grounded_selection::expression(index, c),
    })
}
