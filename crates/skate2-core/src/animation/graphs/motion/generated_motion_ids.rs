//! The Motion state graph's ids under names from its own data, written by
//! `tools/generate_graph_ids.py`: conditions by class and attributes, expressions by the
//! states and transitions that evaluate them, states by name and index.
#![allow(dead_code, reason = "the complete graph; code names the ids it reads")]

/// Condition declarations: what each condition index tests.
pub(crate) mod condition {
    /// `PhysFilteredState(state = teleport)`.
    pub(crate) const PHYS_FILTERED_STATE_TELEPORT: u16 = 0;
    /// `CurrentState(state = Teleport)`.
    pub(crate) const CURRENT_STATE_TELEPORT: u16 = 1;
    /// `HasIntent(intent = WipeOutRequest)`.
    pub(crate) const HAS_INTENT_WIPE_OUT_REQUEST: u16 = 2;
    /// `IsPhysicsWiping`.
    pub(crate) const IS_PHYSICS_WIPING: u16 = 3;
    /// `PhysicsWantsWipeOut`.
    pub(crate) const PHYSICS_WANTS_WIPE_OUT: u16 = 4;
    /// `PhysFilteredState(state = wipeout)`.
    pub(crate) const PHYS_FILTERED_STATE_WIPEOUT: u16 = 5;
    /// `HasIntent(intent = OffBoard)`.
    pub(crate) const HAS_INTENT_OFF_BOARD: u16 = 6;
    /// `HasIntent(intent = OB_SpawnDismount)`.
    pub(crate) const HAS_INTENT_OB_SPAWN_DISMOUNT: u16 = 7;
    /// `WillExpire(InTime = 0.0)`.
    pub(crate) const WILL_EXPIRE_IN_0: u16 = 8;
    /// `HasIntent(intent = OB_Dismount)`.
    pub(crate) const HAS_INTENT_OB_DISMOUNT: u16 = 9;
    /// `IsDismounting`.
    pub(crate) const IS_DISMOUNTING: u16 = 10;
    /// `IsLanding`.
    pub(crate) const IS_LANDING: u16 = 11;
    /// `IsRunningOut`.
    pub(crate) const IS_RUNNING_OUT: u16 = 12;
    /// `PhysFilteredState(state = grind)`.
    pub(crate) const PHYS_FILTERED_STATE_GRIND: u16 = 13;
    /// `IsRidingFakie`.
    pub(crate) const IS_RIDING_FAKIE: u16 = 14;
    /// `WillExpire(InTime = 0.25, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_25_NOT_WAIT: u16 = 15;
    /// `ComVelCompare(abs = true, less = 4.0)`.
    pub(crate) const COM_VEL_COMPARE_ABS_LESS_4: u16 = 16;
    /// `LastState(state = RunDismount)`.
    pub(crate) const LAST_STATE_RUN_DISMOUNT: u16 = 17;
    /// `DisablePushBrake`.
    pub(crate) const DISABLE_PUSH_BRAKE: u16 = 18;
    /// `WillExpire(InTime = 0.1)`.
    pub(crate) const WILL_EXPIRE_IN_0_1: u16 = 19;
    /// `ComVelCompare(abs = true, greaterEqual = 4.0)`.
    pub(crate) const COM_VEL_COMPARE_ABS_GREATER_EQUAL_4: u16 = 20;
    /// `LastState(state = IdleDismount)`.
    pub(crate) const LAST_STATE_IDLE_DISMOUNT: u16 = 21;
    /// `OBTimeToLand(lessEqual = 0.0)`.
    pub(crate) const OB_TIME_TO_LAND_LESS_EQUAL_0: u16 = 22;
    /// `OBTimeToLand(greater = 0.0)`.
    pub(crate) const OB_TIME_TO_LAND_GREATER_0: u16 = 23;
    /// `OBTimeToLand(greater = 0.15)`.
    pub(crate) const OB_TIME_TO_LAND_GREATER_0_15: u16 = 24;
    /// `IsGrabbingObject`.
    pub(crate) const IS_GRABBING_OBJECT: u16 = 25;
    /// `HasIntent(intent = OB_Mount)`.
    pub(crate) const HAS_INTENT_OB_MOUNT: u16 = 26;
    /// `IsMovingObject(angleend = 0, anglestart = -110)`.
    pub(crate) const IS_MOVING_OBJECT_FROM_NEG_110_TO_0: u16 = 27;
    /// `WillExpire(InTime = 0.05)`.
    pub(crate) const WILL_EXPIRE_IN_0_05: u16 = 28;
    /// `IsMovingObject(angleend = 110, anglestart = 0)`.
    pub(crate) const IS_MOVING_OBJECT_FROM_0_TO_110: u16 = 29;
    /// `IsMovingObject(angleend = -110, anglestart = 180)`.
    pub(crate) const IS_MOVING_OBJECT_FROM_180_TO_NEG_110: u16 = 30;
    /// `IsMovingObject(angleend = 180, anglestart = 110)`.
    pub(crate) const IS_MOVING_OBJECT_FROM_110_TO_180: u16 = 31;
    /// `IsMovingObject(angleend = 110, anglestart = 0)`, declared again: the test of
    /// `IS_MOVING_OBJECT_FROM_0_TO_110` under its own index.
    pub(crate) const IS_MOVING_OBJECT_FROM_0_TO_110_2ND: u16 = 32;
    /// `AngleAdjustByAnimationEndLessThan`.
    pub(crate) const ANGLE_ADJUST_BY_ANIMATION_END_LESS_THAN: u16 = 33;
    /// `IsMovingObject(angleend = 0, anglestart = -110)`, declared again: the test of
    /// `IS_MOVING_OBJECT_FROM_NEG_110_TO_0` under its own index.
    pub(crate) const IS_MOVING_OBJECT_FROM_NEG_110_TO_0_2ND: u16 = 34;
    /// `IsMovingObject(angleend = 180, anglestart = 110)`, declared again: the test of
    /// `IS_MOVING_OBJECT_FROM_110_TO_180` under its own index.
    pub(crate) const IS_MOVING_OBJECT_FROM_110_TO_180_2ND: u16 = 35;
    /// `IsMovingObject(angleend = -110, anglestart = 180)`, declared again: the test of
    /// `IS_MOVING_OBJECT_FROM_180_TO_NEG_110` under its own index.
    pub(crate) const IS_MOVING_OBJECT_FROM_180_TO_NEG_110_2ND: u16 = 36;
    /// `IsMovingObject(angleend = 180, anglestart = -180)`.
    pub(crate) const IS_MOVING_OBJECT_FROM_NEG_180_TO_180: u16 = 37;
    /// `LocomotionValid`.
    pub(crate) const LOCOMOTION_VALID: u16 = 38;
    /// `IsDismounting`, declared again: the test of `IS_DISMOUNTING` under its own index.
    pub(crate) const IS_DISMOUNTING_2ND: u16 = 39;
    /// `HasIntent(greater = 0.3, intent = OBL_SteerMagnitude)`.
    pub(crate) const HAS_INTENT_OBL_STEER_MAGNITUDE_GREATER_0_3: u16 = 40;
    /// `HasIntent(intent = OB_SteerMagnitudeHeld)`.
    pub(crate) const HAS_INTENT_OB_STEER_MAGNITUDE_HELD: u16 = 41;
    /// `IsLocomotionQuitNotAllowed`.
    pub(crate) const IS_LOCOMOTION_QUIT_NOT_ALLOWED: u16 = 42;
    /// `HasIntent(intent = OB_Steer, lessEqual = 0.05)`.
    pub(crate) const HAS_INTENT_OB_STEER_LESS_EQUAL_0_05: u16 = 43;
    /// `HasIntent(greaterEqual = -0.05, intent = OB_Steer)`.
    pub(crate) const HAS_INTENT_OB_STEER_GREATER_EQUAL_NEG_0_05: u16 = 44;
    /// `HasIntent(intent = OB_Steer)`.
    pub(crate) const HAS_INTENT_OB_STEER: u16 = 45;
    /// `HasIntent(intent = OB_SteerMagnitudeHeld)`, declared again: the test of
    /// `HAS_INTENT_OB_STEER_MAGNITUDE_HELD` under its own index.
    pub(crate) const HAS_INTENT_OB_STEER_MAGNITUDE_HELD_2ND: u16 = 46;
    /// `HasIntent(greater = 0.05, intent = OB_Steer)`.
    pub(crate) const HAS_INTENT_OB_STEER_GREATER_0_05: u16 = 47;
    /// `IsMirrored`.
    pub(crate) const IS_MIRRORED: u16 = 48;
    /// `HasIntent(intent = OB_Steer, less = -0.05)`.
    pub(crate) const HAS_INTENT_OB_STEER_LESS_NEG_0_05: u16 = 49;
    /// `HasIntent(intent = StepForward)`.
    pub(crate) const HAS_INTENT_STEP_FORWARD: u16 = 50;
    /// `LastState(state = ToStepForward)`.
    pub(crate) const LAST_STATE_TO_STEP_FORWARD: u16 = 51;
    /// `WillExpire(inTime = 0.1)`, declared again: the test of `WILL_EXPIRE_IN_0_1` under its own
    /// index.
    pub(crate) const WILL_EXPIRE_IN_0_1_2ND: u16 = 52;
    /// `HasIntent(intent = StepBackward)`.
    pub(crate) const HAS_INTENT_STEP_BACKWARD: u16 = 53;
    /// `LastState(state = ToStepBackward)`.
    pub(crate) const LAST_STATE_TO_STEP_BACKWARD: u16 = 54;
    /// `InStateForTime(greater = 0.4, state = Idle)`.
    pub(crate) const IN_STATE_FOR_TIME_IDLE_GREATER_0_4: u16 = 55;
    /// `HasIntent(intent = OB_Jump)`.
    pub(crate) const HAS_INTENT_OB_JUMP: u16 = 56;
    /// `WillExpire(InTime = 0.017)`.
    pub(crate) const WILL_EXPIRE_IN_0_017: u16 = 57;
    /// `InStateForTime(less = 0.3, state = Descent)`.
    pub(crate) const IN_STATE_FOR_TIME_DESCENT_LESS_0_3: u16 = 58;
    /// `PhysFilteredState(state = offboard)`.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD: u16 = 59;
    /// `HasIntent(intent = BSGRAB)`.
    pub(crate) const HAS_INTENT_BSGRAB: u16 = 60;
    /// `IsInJumpAir(foot = Left)`.
    pub(crate) const IS_IN_JUMP_AIR_LEFT: u16 = 61;
    /// `IsInJumpAir(foot = Right)`.
    pub(crate) const IS_IN_JUMP_AIR_RIGHT: u16 = 62;
    /// `IsLocomotionQuitNotAllowed`, declared again: the test of `IS_LOCOMOTION_QUIT_NOT_ALLOWED`
    /// under its own index.
    pub(crate) const IS_LOCOMOTION_QUIT_NOT_ALLOWED_2ND: u16 = 63;
    /// `IsLocomotionFootDown(foot = Right)`.
    pub(crate) const IS_LOCOMOTION_FOOT_DOWN_RIGHT: u16 = 64;
    /// `CurrentState(state = StallRunIntoMountRight)`.
    pub(crate) const CURRENT_STATE_STALL_RUN_INTO_MOUNT_RIGHT: u16 = 65;
    /// `DoSprintMount`.
    pub(crate) const DO_SPRINT_MOUNT: u16 = 66;
    /// `IsRetrievingSkateboard`.
    pub(crate) const IS_RETRIEVING_SKATEBOARD: u16 = 67;
    /// `IsDroppingSkateboard`.
    pub(crate) const IS_DROPPING_SKATEBOARD: u16 = 68;
    /// `IsLocomotionFootDown(foot = Left)`.
    pub(crate) const IS_LOCOMOTION_FOOT_DOWN_LEFT: u16 = 69;
    /// `CurrentState(state = StallRunIntoMountLeft)`.
    pub(crate) const CURRENT_STATE_STALL_RUN_INTO_MOUNT_LEFT: u16 = 70;
    /// `WillExpire(InTime = 0.3)`.
    pub(crate) const WILL_EXPIRE_IN_0_3: u16 = 71;
    /// `IsAirMountRequested`.
    pub(crate) const IS_AIR_MOUNT_REQUESTED: u16 = 72;
    /// `IsPastLocomotionApex`.
    pub(crate) const IS_PAST_LOCOMOTION_APEX: u16 = 73;
    /// `CurrentState(state = OBAir.RightFootStride)`.
    pub(crate) const CURRENT_STATE_OB_AIR_RIGHT_FOOT_STRIDE: u16 = 74;
    /// `CurrentState(state = OBAir.LeftFootStride)`.
    pub(crate) const CURRENT_STATE_OB_AIR_LEFT_FOOT_STRIDE: u16 = 75;
    /// `PhysFilteredState(state = offboardair)`.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD_AIR: u16 = 76;
    /// `HasIntent(intent = OffBoardAir)`.
    pub(crate) const HAS_INTENT_OFF_BOARD_AIR: u16 = 77;
    /// `HasAnimAttribute(attribute = LocoStartingFoot)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_LOCO_STARTING_FOOT: u16 = 78;
    /// `CurrentState(state = KickoutToAir)`.
    pub(crate) const CURRENT_STATE_KICKOUT_TO_AIR: u16 = 79;
    /// `CurrentState(state = TrickDismount)`.
    pub(crate) const CURRENT_STATE_TRICK_DISMOUNT: u16 = 80;
    /// `CurrentState(state = AccidentalDismount.Front.Fly)`.
    pub(crate) const CURRENT_STATE_ACCIDENTAL_DISMOUNT_FRONT_FLY: u16 = 81;
    /// `CurrentState(state = AccidentalDismount.Front.Finish)`.
    pub(crate) const CURRENT_STATE_ACCIDENTAL_DISMOUNT_FRONT_FINISH: u16 = 82;
    /// `OBTimeToLand(lessEqual = 0.03)`.
    pub(crate) const OB_TIME_TO_LAND_LESS_EQUAL_0_03: u16 = 83;
    /// `WillExpire(inTime = 0.033)`.
    pub(crate) const WILL_EXPIRE_IN_0_033: u16 = 84;
    /// `PhysFilteredState(state = air)`.
    pub(crate) const PHYS_FILTERED_STATE_AIR: u16 = 85;
    /// `OBTimeToLand(lessEqual = 0.05)`.
    pub(crate) const OB_TIME_TO_LAND_LESS_EQUAL_0_05: u16 = 86;
    /// `PhysicsWantsRunout`.
    pub(crate) const PHYSICS_WANTS_RUNOUT: u16 = 87;
    /// `PhysFilteredState(state = offboardair)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_OFFBOARD_AIR` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD_AIR_2ND: u16 = 88;
    /// `PhysFilteredState(state = ground)`.
    pub(crate) const PHYS_FILTERED_STATE_GROUND: u16 = 89;
    /// `InTimeWindow(StartTime = 0.0, WindowFrameLength = 0.3)`.
    pub(crate) const IN_TIME_WINDOW_START_0_LENGTH_0_3: u16 = 90;
    /// `IsTransitioning`.
    pub(crate) const IS_TRANSITIONING: u16 = 91;
    /// `WillExpire(InTime = 0.01)`.
    pub(crate) const WILL_EXPIRE_IN_0_01: u16 = 92;
    /// `HasIntent(intent = ForceBrake)`.
    pub(crate) const HAS_INTENT_FORCE_BRAKE: u16 = 93;
    /// `PhysFilteredState(state = grind)`, declared again: the test of `PHYS_FILTERED_STATE_GRIND`
    /// under its own index.
    pub(crate) const PHYS_FILTERED_STATE_GRIND_2ND: u16 = 94;
    /// `IsGrinding(grindName = BS_FEEBLE)`.
    pub(crate) const IS_GRINDING_BS_FEEBLE: u16 = 95;
    /// `IsGrinding(grindName = FS_FEEBLE)`.
    pub(crate) const IS_GRINDING_FS_FEEBLE: u16 = 96;
    /// `IsGrinding(grindName = BS_SMITH)`.
    pub(crate) const IS_GRINDING_BS_SMITH: u16 = 97;
    /// `IsGrinding(grindName = FS_SMITH)`.
    pub(crate) const IS_GRINDING_FS_SMITH: u16 = 98;
    /// `IsGrinding(grindName = FS_WILLY)`.
    pub(crate) const IS_GRINDING_FS_WILLY: u16 = 99;
    /// `IsGrinding(grindName = BS_WILLY)`.
    pub(crate) const IS_GRINDING_BS_WILLY: u16 = 100;
    /// `IsGrinding(grindName = FS_OVERWILLY)`.
    pub(crate) const IS_GRINDING_FS_OVERWILLY: u16 = 101;
    /// `IsGrinding(grindName = BS_OVERWILLY)`.
    pub(crate) const IS_GRINDING_BS_OVERWILLY: u16 = 102;
    /// `IsGrinding(grindName = BS_5_O)`.
    pub(crate) const IS_GRINDING_BS_5_O: u16 = 103;
    /// `HasIntent(intent = GrindFSGrab)`.
    pub(crate) const HAS_INTENT_GRIND_FS_GRAB: u16 = 104;
    /// `HasIntent(intent = GrindDoubleGrab)`.
    pub(crate) const HAS_INTENT_GRIND_DOUBLE_GRAB: u16 = 105;
    /// `CurrentGrabType(grab = FS)`.
    pub(crate) const CURRENT_GRAB_TYPE_FS: u16 = 106;
    /// `HasIntent(intent = GrindBSGrab)`.
    pub(crate) const HAS_INTENT_GRIND_BS_GRAB: u16 = 107;
    /// `CurrentGrabType(grab = Nose)`.
    pub(crate) const CURRENT_GRAB_TYPE_NOSE: u16 = 108;
    /// `IsGrinding(grindName = FS_5_O)`.
    pub(crate) const IS_GRINDING_FS_5_O: u16 = 109;
    /// `IsGrinding(grindName = BS_50_50)`.
    pub(crate) const IS_GRINDING_BS_50_50: u16 = 110;
    /// `CurrentGrabType(grab = BS)`.
    pub(crate) const CURRENT_GRAB_TYPE_BS: u16 = 111;
    /// `IsGrinding(grindName = FS_50_50)`.
    pub(crate) const IS_GRINDING_FS_50_50: u16 = 112;
    /// `IsGrinding(grindName = BS_CROOK)`.
    pub(crate) const IS_GRINDING_BS_CROOK: u16 = 113;
    /// `CurrentGrabType(grab = Tail)`.
    pub(crate) const CURRENT_GRAB_TYPE_TAIL: u16 = 114;
    /// `IsGrinding(grindName = FS_CROOK)`.
    pub(crate) const IS_GRINDING_FS_CROOK: u16 = 115;
    /// `IsGrinding(grindName = FS_OVERCROOK)`.
    pub(crate) const IS_GRINDING_FS_OVERCROOK: u16 = 116;
    /// `IsGrinding(grindName = BS_OVERCROOK)`.
    pub(crate) const IS_GRINDING_BS_OVERCROOK: u16 = 117;
    /// `IsGrinding(grindName = BS_NOSEGRIND)`.
    pub(crate) const IS_GRINDING_BS_NOSEGRIND: u16 = 118;
    /// `IsGrinding(grindName = FS_NOSEGRIND)`.
    pub(crate) const IS_GRINDING_FS_NOSEGRIND: u16 = 119;
    /// `IsGrinding(grindName = BS_SALAD)`.
    pub(crate) const IS_GRINDING_BS_SALAD: u16 = 120;
    /// `IsGrinding(grindName = FS_SALAD)`.
    pub(crate) const IS_GRINDING_FS_SALAD: u16 = 121;
    /// `IsGrinding(grindName = BS_NOSESLIDE)`.
    pub(crate) const IS_GRINDING_BS_NOSESLIDE: u16 = 122;
    /// `IsGrinding(grindName = FS_NOSESLIDE)`.
    pub(crate) const IS_GRINDING_FS_NOSESLIDE: u16 = 123;
    /// `IsGrinding(grindName = BS_NOSEBLUNT)`.
    pub(crate) const IS_GRINDING_BS_NOSEBLUNT: u16 = 124;
    /// `IsGrindBluntingBackslash`.
    pub(crate) const IS_GRIND_BLUNTING_BACKSLASH: u16 = 125;
    /// `IsGrinding(grindName = FS_NOSEBLUNT)`.
    pub(crate) const IS_GRINDING_FS_NOSEBLUNT: u16 = 126;
    /// `IsGrinding(grindName = BS_BLUNT)`.
    pub(crate) const IS_GRINDING_BS_BLUNT: u16 = 127;
    /// `IsGrinding(grindName = FS_BLUNT)`.
    pub(crate) const IS_GRINDING_FS_BLUNT: u16 = 128;
    /// `IsGrinding(grindName = FS_TAILSLIDE)`.
    pub(crate) const IS_GRINDING_FS_TAILSLIDE: u16 = 129;
    /// `IsGrinding(grindName = BS_TAILSLIDE)`.
    pub(crate) const IS_GRINDING_BS_TAILSLIDE: u16 = 130;
    /// `IsGrinding(grindName = BS_BOARD)`.
    pub(crate) const IS_GRINDING_BS_BOARD: u16 = 131;
    /// `HasIntent(intent = GrindFSGrab)`, declared again: the test of `HAS_INTENT_GRIND_FS_GRAB`
    /// under its own index.
    pub(crate) const HAS_INTENT_GRIND_FS_GRAB_2ND: u16 = 132;
    /// `HasIntent(intent = GrindBSGrab)`, declared again: the test of `HAS_INTENT_GRIND_BS_GRAB`
    /// under its own index.
    pub(crate) const HAS_INTENT_GRIND_BS_GRAB_2ND: u16 = 133;
    /// `HasIntent(intent = GrindDoubleGrab)`, declared again: the test of
    /// `HAS_INTENT_GRIND_DOUBLE_GRAB` under its own index.
    pub(crate) const HAS_INTENT_GRIND_DOUBLE_GRAB_2ND: u16 = 134;
    /// `IsGrinding(grindName = FS_BOARD)`.
    pub(crate) const IS_GRINDING_FS_BOARD: u16 = 135;
    /// `IsGrinding(grindName = BS_LIP)`.
    pub(crate) const IS_GRINDING_BS_LIP: u16 = 136;
    /// `IsGrinding(grindName = FS_LIP)`.
    pub(crate) const IS_GRINDING_FS_LIP: u16 = 137;
    /// `IsGrabbingObject`, declared again: the test of `IS_GRABBING_OBJECT` under its own index.
    pub(crate) const IS_GRABBING_OBJECT_2ND: u16 = 138;
    /// `HasIntent(intent = Trick)`.
    pub(crate) const HAS_INTENT_TRICK: u16 = 139;
    /// `AllowedToTrick`.
    pub(crate) const ALLOWED_TO_TRICK: u16 = 140;
    /// `OkToDoTrickOnStairs`.
    pub(crate) const OK_TO_DO_TRICK_ON_STAIRS: u16 = 141;
    /// `IsProSkater(skater = MikeCarroll)`.
    pub(crate) const IS_PRO_SKATER_MIKE_CARROLL: u16 = 142;
    /// `HasIntent(intent = Kickflip)`.
    pub(crate) const HAS_INTENT_KICKFLIP: u16 = 143;
    /// `IsGrinding(grindName = BS_LIP)`, declared again: the test of `IS_GRINDING_BS_LIP` under its
    /// own index.
    pub(crate) const IS_GRINDING_BS_LIP_2ND: u16 = 144;
    /// `IsGrinding(grindName = FS_LIP)`, declared again: the test of `IS_GRINDING_FS_LIP` under its
    /// own index.
    pub(crate) const IS_GRINDING_FS_LIP_2ND: u16 = 145;
    /// `IsGrinding(grindName = BS_BOARD)`, declared again: the test of `IS_GRINDING_BS_BOARD` under
    /// its own index.
    pub(crate) const IS_GRINDING_BS_BOARD_2ND: u16 = 146;
    /// `IsGrinding(grindName = FS_BOARD)`, declared again: the test of `IS_GRINDING_FS_BOARD` under
    /// its own index.
    pub(crate) const IS_GRINDING_FS_BOARD_2ND: u16 = 147;
    /// `IsGrinding(grindName = BS_FEEBLE)`, declared again: the test of `IS_GRINDING_BS_FEEBLE`
    /// under its own index.
    pub(crate) const IS_GRINDING_BS_FEEBLE_2ND: u16 = 148;
    /// `IsGrinding(grindName = FS_FEEBLE)`, declared again: the test of `IS_GRINDING_FS_FEEBLE`
    /// under its own index.
    pub(crate) const IS_GRINDING_FS_FEEBLE_2ND: u16 = 149;
    /// `IsGrinding(grindName = BS_SMITH)`, declared again: the test of `IS_GRINDING_BS_SMITH` under
    /// its own index.
    pub(crate) const IS_GRINDING_BS_SMITH_2ND: u16 = 150;
    /// `IsGrinding(grindName = FS_SMITH)`, declared again: the test of `IS_GRINDING_FS_SMITH` under
    /// its own index.
    pub(crate) const IS_GRINDING_FS_SMITH_2ND: u16 = 151;
    /// `IsGrinding(grindName = BS_WILLY)`, declared again: the test of `IS_GRINDING_BS_WILLY` under
    /// its own index.
    pub(crate) const IS_GRINDING_BS_WILLY_2ND: u16 = 152;
    /// `IsGrinding(grindName = FS_WILLY)`, declared again: the test of `IS_GRINDING_FS_WILLY` under
    /// its own index.
    pub(crate) const IS_GRINDING_FS_WILLY_2ND: u16 = 153;
    /// `IsGrinding(grindName = BS_OVERWILLY)`, declared again: the test of
    /// `IS_GRINDING_BS_OVERWILLY` under its own index.
    pub(crate) const IS_GRINDING_BS_OVERWILLY_2ND: u16 = 154;
    /// `IsGrinding(grindName = FS_OVERWILLY)`, declared again: the test of
    /// `IS_GRINDING_FS_OVERWILLY` under its own index.
    pub(crate) const IS_GRINDING_FS_OVERWILLY_2ND: u16 = 155;
    /// `WillExpire(InTime = 0.05, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_05_NOT_WAIT: u16 = 156;
    /// `WillExpire(InTime = 0.02, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_02_NOT_WAIT: u16 = 157;
    /// `HasIntent(intent = KickflipHold)`.
    pub(crate) const HAS_INTENT_KICKFLIP_HOLD: u16 = 158;
    /// `TimeToLand(greater = 0.5)`.
    pub(crate) const TIME_TO_LAND_GREATER_0_5: u16 = 159;
    /// `IsBodyFlipping`.
    pub(crate) const IS_BODY_FLIPPING: u16 = 160;
    /// `LastState(state = OnBoard.OnGround.RidingIdle.Riding.Turning.Manual)`.
    pub(crate) const LAST_STATE_RIDING_TURNING_MANUAL: u16 = 161;
    /// `ManualOutTimerIsActive`.
    pub(crate) const MANUAL_OUT_TIMER_IS_ACTIVE: u16 = 162;
    /// `LastState(state = OnBoard.Grinding)`.
    pub(crate) const LAST_STATE_ON_BOARD_GRINDING: u16 = 163;
    /// `TimeToLand(less = 0.6)`.
    pub(crate) const TIME_TO_LAND_LESS_0_6: u16 = 164;
    /// `IsProSkater(skater = MarkGonzales)`.
    pub(crate) const IS_PRO_SKATER_MARK_GONZALES: u16 = 165;
    /// `TimeToLand(greater = 0.45)`.
    pub(crate) const TIME_TO_LAND_GREATER_0_45: u16 = 166;
    /// `HasIntent(intent = Ollie)`.
    pub(crate) const HAS_INTENT_OLLIE: u16 = 167;
    /// `HasAnimAttribute(attribute = canBranch)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_CAN_BRANCH: u16 = 168;
    /// `TimeToLand(less = 0.525)`.
    pub(crate) const TIME_TO_LAND_LESS_0_525: u16 = 169;
    /// `TimeToLand(less = 0.8)`.
    pub(crate) const TIME_TO_LAND_LESS_0_8: u16 = 170;
    /// `HasIntent(intent = Heelflip)`.
    pub(crate) const HAS_INTENT_HEELFLIP: u16 = 171;
    /// `HasIntent(intent = HeelflipHold)`.
    pub(crate) const HAS_INTENT_HEELFLIP_HOLD: u16 = 172;
    /// `HasIntent(intent = PopShuvit)`.
    pub(crate) const HAS_INTENT_POP_SHUVIT: u16 = 173;
    /// `HasIntent(intent = FSPopShuvit)`.
    pub(crate) const HAS_INTENT_FS_POP_SHUVIT: u16 = 174;
    /// `HasIntent(intent = VarialKickflip)`.
    pub(crate) const HAS_INTENT_VARIAL_KICKFLIP: u16 = 175;
    /// `HasIntent(intent = VarialHeelflip)`.
    pub(crate) const HAS_INTENT_VARIAL_HEELFLIP: u16 = 176;
    /// `HasIntent(intent = Hardflip)`.
    pub(crate) const HAS_INTENT_HARDFLIP: u16 = 177;
    /// `HasIntent(intent = InwardHeelflip)`.
    pub(crate) const HAS_INTENT_INWARD_HEELFLIP: u16 = 178;
    /// `HasIntent(intent = 360PopShuvit)`.
    pub(crate) const HAS_INTENT_360_POP_SHUVIT: u16 = 179;
    /// `HasIntent(intent = FS360PopShuvit)`.
    pub(crate) const HAS_INTENT_FS_360_POP_SHUVIT: u16 = 180;
    /// `HasIntent(intent = 360Flip)`.
    pub(crate) const HAS_INTENT_360_FLIP: u16 = 181;
    /// `HasIntent(intent = Laserflip)`.
    pub(crate) const HAS_INTENT_LASERFLIP: u16 = 182;
    /// `HasIntent(intent = 360Hardflip)`.
    pub(crate) const HAS_INTENT_360_HARDFLIP: u16 = 183;
    /// `HasIntent(intent = 360InwardHeelflip)`.
    pub(crate) const HAS_INTENT_360_INWARD_HEELFLIP: u16 = 184;
    /// `HasIntent(intent = Nollie)`.
    pub(crate) const HAS_INTENT_NOLLIE: u16 = 185;
    /// `HasIntent(intent = N_Kickflip)`.
    pub(crate) const HAS_INTENT_N_KICKFLIP: u16 = 186;
    /// `HasIntent(intent = N_KickflipHold)`.
    pub(crate) const HAS_INTENT_N_KICKFLIP_HOLD: u16 = 187;
    /// `HasIntent(intent = N_Heelflip)`.
    pub(crate) const HAS_INTENT_N_HEELFLIP: u16 = 188;
    /// `HasIntent(intent = N_HeelflipHold)`.
    pub(crate) const HAS_INTENT_N_HEELFLIP_HOLD: u16 = 189;
    /// `HasIntent(intent = N_PopShuvit)`.
    pub(crate) const HAS_INTENT_N_POP_SHUVIT: u16 = 190;
    /// `HasIntent(intent = N_FSPopShuvit)`.
    pub(crate) const HAS_INTENT_N_FS_POP_SHUVIT: u16 = 191;
    /// `HasIntent(intent = N_VarialKickflip)`.
    pub(crate) const HAS_INTENT_N_VARIAL_KICKFLIP: u16 = 192;
    /// `HasIntent(intent = N_VarialHeelflip)`.
    pub(crate) const HAS_INTENT_N_VARIAL_HEELFLIP: u16 = 193;
    /// `HasIntent(intent = N_Hardflip)`.
    pub(crate) const HAS_INTENT_N_HARDFLIP: u16 = 194;
    /// `HasIntent(intent = N_InwardHeelflip)`.
    pub(crate) const HAS_INTENT_N_INWARD_HEELFLIP: u16 = 195;
    /// `HasIntent(intent = N_360PopShuvit)`.
    pub(crate) const HAS_INTENT_N_360_POP_SHUVIT: u16 = 196;
    /// `HasIntent(intent = N_FS360PopShuvit)`.
    pub(crate) const HAS_INTENT_N_FS_360_POP_SHUVIT: u16 = 197;
    /// `HasIntent(intent = N_360Flip)`.
    pub(crate) const HAS_INTENT_N_360_FLIP: u16 = 198;
    /// `HasIntent(intent = N_Laserflip)`.
    pub(crate) const HAS_INTENT_N_LASERFLIP: u16 = 199;
    /// `HasIntent(intent = N_360Hardflip)`.
    pub(crate) const HAS_INTENT_N_360_HARDFLIP: u16 = 200;
    /// `HasIntent(intent = N_360InwardHeelflip)`.
    pub(crate) const HAS_INTENT_N_360_INWARD_HEELFLIP: u16 = 201;
    /// `PhysFilteredState(state = ground)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_GROUND` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_GROUND_2ND: u16 = 202;
    /// `HasIntent(intent = FSBoneless)`.
    pub(crate) const HAS_INTENT_FS_BONELESS: u16 = 203;
    /// `HasIntent(intent = NewFootPlant)`.
    pub(crate) const HAS_INTENT_NEW_FOOT_PLANT: u16 = 204;
    /// `HasIntent(intent = LeftFootPlant)`.
    pub(crate) const HAS_INTENT_LEFT_FOOT_PLANT: u16 = 205;
    /// `HasIntent(intent = RightFootPlant)`.
    pub(crate) const HAS_INTENT_RIGHT_FOOT_PLANT: u16 = 206;
    /// `ComVelCompare(abs = true, greaterEqual = 15)`.
    pub(crate) const COM_VEL_COMPARE_ABS_GREATER_EQUAL_15: u16 = 207;
    /// `HasIntent(intent = BSBoneless)`.
    pub(crate) const HAS_INTENT_BS_BONELESS: u16 = 208;
    /// `HasIntent(intent = FSFastPlant)`.
    pub(crate) const HAS_INTENT_FS_FAST_PLANT: u16 = 209;
    /// `HasIntent(intent = BSFastPlant)`.
    pub(crate) const HAS_INTENT_BS_FAST_PLANT: u16 = 210;
    /// `HasIntent(intent = NoComply)`.
    pub(crate) const HAS_INTENT_NO_COMPLY: u16 = 211;
    /// `IsHandPlanting(dir = FS, state = air)`.
    pub(crate) const IS_HAND_PLANTING_FS_AIR: u16 = 212;
    /// `ShouldPlayHandPlantAnim(anim = into)`.
    pub(crate) const SHOULD_PLAY_HAND_PLANT_ANIM_INTO: u16 = 213;
    /// `WillExpire(InTime = 0.02)`.
    pub(crate) const WILL_EXPIRE_IN_0_02: u16 = 214;
    /// `WillExpire(InTime = 0.15)`.
    pub(crate) const WILL_EXPIRE_IN_0_15: u16 = 215;
    /// `ShouldPlayHandPlantAnim(anim = antic)`.
    pub(crate) const SHOULD_PLAY_HAND_PLANT_ANIM_ANTIC: u16 = 216;
    /// `ShouldPlayHandPlantAnim(anim = out)`.
    pub(crate) const SHOULD_PLAY_HAND_PLANT_ANIM_OUT: u16 = 217;
    /// `IsRidingGoofy`.
    pub(crate) const IS_RIDING_GOOFY: u16 = 218;
    /// `HasIntent(intent = HandPlantOneFootRight)`.
    pub(crate) const HAS_INTENT_HAND_PLANT_ONE_FOOT_RIGHT: u16 = 219;
    /// `HasIntent(intent = HandPlantOneFootLeft)`.
    pub(crate) const HAS_INTENT_HAND_PLANT_ONE_FOOT_LEFT: u16 = 220;
    /// `HasIntent(intent = HandPlantDismount)`.
    pub(crate) const HAS_INTENT_HAND_PLANT_DISMOUNT: u16 = 221;
    /// `IsHandPlanting(dir = BS, state = air)`.
    pub(crate) const IS_HAND_PLANTING_BS_AIR: u16 = 222;
    /// `PhysFilteredState(state = air)`, declared again: the test of `PHYS_FILTERED_STATE_AIR`
    /// under its own index.
    pub(crate) const PHYS_FILTERED_STATE_AIR_2ND: u16 = 223;
    /// `HasIntent(intent = DBLGrab)`.
    pub(crate) const HAS_INTENT_DBL_GRAB: u16 = 224;
    /// `HasIntent(intent = Coffin)`.
    pub(crate) const HAS_INTENT_COFFIN: u16 = 225;
    /// `HasIntent(intent = Superman)`.
    pub(crate) const HAS_INTENT_SUPERMAN: u16 = 226;
    /// `TrucksOrDeckInContact`.
    pub(crate) const TRUCKS_OR_DECK_IN_CONTACT: u16 = 227;
    /// `HasTweak(greaterEqual = 0.8)`.
    pub(crate) const HAS_TWEAK_GREATER_EQUAL_0_8: u16 = 228;
    /// `HasTweak(less = 0.2)`.
    pub(crate) const HAS_TWEAK_LESS_0_2: u16 = 229;
    /// `HasIntent(intent = FSChrist)`.
    pub(crate) const HAS_INTENT_FS_CHRIST: u16 = 230;
    /// `HasIntent(intent = BSChrist)`.
    pub(crate) const HAS_INTENT_BS_CHRIST: u16 = 231;
    /// `HasIntent(intent = FSGrab)`.
    pub(crate) const HAS_INTENT_FS_GRAB: u16 = 232;
    /// `HasIntent(intent = MuteGrab)`.
    pub(crate) const HAS_INTENT_MUTE_GRAB: u16 = 233;
    /// `HasIntent(intent = FingerFlipShuv)`.
    pub(crate) const HAS_INTENT_FINGER_FLIP_SHUV: u16 = 234;
    /// `CurrentState(state = FingerFlipVar.GrabOut)`.
    pub(crate) const CURRENT_STATE_FINGER_FLIP_VAR_GRAB_OUT: u16 = 235;
    /// `HasTweak(greaterEqual = 0.4)`.
    pub(crate) const HAS_TWEAK_GREATER_EQUAL_0_4: u16 = 236;
    /// `HasAnimAttribute(attribute = ChristEarlyOut)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_CHRIST_EARLY_OUT: u16 = 237;
    /// `HasAGIntent(intent = LeftAirGrab)`.
    pub(crate) const HAS_AG_INTENT_LEFT_AIR_GRAB: u16 = 238;
    /// `HasAGIntent(intent = RightAirGrab)`.
    pub(crate) const HAS_AG_INTENT_RIGHT_AIR_GRAB: u16 = 239;
    /// `HasIntent(intent = RightOneFootAir)`.
    pub(crate) const HAS_INTENT_RIGHT_ONE_FOOT_AIR: u16 = 240;
    /// `HasAnimAttribute(attribute = OneFootEarlyOut)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_ONE_FOOT_EARLY_OUT: u16 = 241;
    /// `ShouldPrepareOneFootAirForFootplant`.
    pub(crate) const SHOULD_PREPARE_ONE_FOOT_AIR_FOR_FOOTPLANT: u16 = 242;
    /// `IsFootPlanting`.
    pub(crate) const IS_FOOT_PLANTING: u16 = 243;
    /// `HasIntent(intent = LeftOneFootAir)`.
    pub(crate) const HAS_INTENT_LEFT_ONE_FOOT_AIR: u16 = 244;
    /// `HasIntent(intent = BSGrab)`.
    pub(crate) const HAS_INTENT_BS_GRAB: u16 = 245;
    /// `HasIntent(intent = FingerFlip)`.
    pub(crate) const HAS_INTENT_FINGER_FLIP: u16 = 246;
    /// `HasIntent(intent = FSGrab)`, declared again: the test of `HAS_INTENT_FS_GRAB` under its own
    /// index.
    pub(crate) const HAS_INTENT_FS_GRAB_2ND: u16 = 247;
    /// `HasIntent(intent = FingerFlipVar)`.
    pub(crate) const HAS_INTENT_FINGER_FLIP_VAR: u16 = 248;
    /// `HasIntent(intent = DBLGrab)`, declared again: the test of `HAS_INTENT_DBL_GRAB` under its
    /// own index.
    pub(crate) const HAS_INTENT_DBL_GRAB_2ND: u16 = 249;
    /// `HasIntent(intent = Coffin)`, declared again: the test of `HAS_INTENT_COFFIN` under its own
    /// index.
    pub(crate) const HAS_INTENT_COFFIN_2ND: u16 = 250;
    /// `WillExpire(inTime = 0.0)`, declared again: the test of `WILL_EXPIRE_IN_0` under its own
    /// index.
    pub(crate) const WILL_EXPIRE_IN_0_2ND: u16 = 251;
    /// `HasIntent(intent = BSGrab)`, declared again: the test of `HAS_INTENT_BS_GRAB` under its own
    /// index.
    pub(crate) const HAS_INTENT_BS_GRAB_2ND: u16 = 252;
    /// `HasTweak(greaterEqual = 0.2)`.
    pub(crate) const HAS_TWEAK_GREATER_EQUAL_0_2: u16 = 253;
    /// `HasFilteredMotionGraphIntent(abs = true, intent = TweakY, less = 0.2)`.
    pub(crate) const HAS_FILTERED_MOTION_GRAPH_INTENT_TWEAK_Y_ABS_LESS_0_2: u16 = 254;
    /// `HasIntent(intent = MuteGrab)`, declared again: the test of `HAS_INTENT_MUTE_GRAB` under its
    /// own index.
    pub(crate) const HAS_INTENT_MUTE_GRAB_2ND: u16 = 255;
    /// `HasIntent(intent = StaleGrab)`.
    pub(crate) const HAS_INTENT_STALE_GRAB: u16 = 256;
    /// `HasIntent(intent = StaleGrab)`, declared again: the test of `HAS_INTENT_STALE_GRAB` under
    /// its own index.
    pub(crate) const HAS_INTENT_STALE_GRAB_2ND: u16 = 257;
    /// `HasIntent(intent = BoardAdjustLeft)`.
    pub(crate) const HAS_INTENT_BOARD_ADJUST_LEFT: u16 = 258;
    /// `HasFilteredMotionGraphIntent(abs = true, intent = BoardAdjustAngle, less = 0.4)`.
    pub(crate) const HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_ANGLE_ABS_LESS_0_4: u16 = 259;
    /// `HasFilteredMotionGraphIntent(abs = true, intent = BoardAdjustMag, less = 0.4)`.
    pub(crate) const HAS_FILTERED_MOTION_GRAPH_INTENT_BOARD_ADJUST_MAG_ABS_LESS_0_4: u16 = 260;
    /// `HasIntent(intent = BoardAdjustRight)`.
    pub(crate) const HAS_INTENT_BOARD_ADJUST_RIGHT: u16 = 261;
    /// `HasIntent(intent = BoardAdjustUp)`.
    pub(crate) const HAS_INTENT_BOARD_ADJUST_UP: u16 = 262;
    /// `HasIntent(intent = TailGrab)`.
    pub(crate) const HAS_INTENT_TAIL_GRAB: u16 = 263;
    /// `HasIntent(intent = TailGrab)`, declared again: the test of `HAS_INTENT_TAIL_GRAB` under its
    /// own index.
    pub(crate) const HAS_INTENT_TAIL_GRAB_2ND: u16 = 264;
    /// `HasIntent(intent = NoFootAirWalk)`.
    pub(crate) const HAS_INTENT_NO_FOOT_AIR_WALK: u16 = 265;
    /// `HasIntent(intent = SeatbeltGrab)`.
    pub(crate) const HAS_INTENT_SEATBELT_GRAB: u16 = 266;
    /// `HasIntent(intent = SeatbeltGrab)`, declared again: the test of `HAS_INTENT_SEATBELT_GRAB`
    /// under its own index.
    pub(crate) const HAS_INTENT_SEATBELT_GRAB_2ND: u16 = 267;
    /// `HasIntent(intent = BoardAdjustDown)`.
    pub(crate) const HAS_INTENT_BOARD_ADJUST_DOWN: u16 = 268;
    /// `HasIntent(intent = RocketAir)`.
    pub(crate) const HAS_INTENT_ROCKET_AIR: u16 = 269;
    /// `HasIntent(intent = RocketAir)`, declared again: the test of `HAS_INTENT_ROCKET_AIR` under
    /// its own index.
    pub(crate) const HAS_INTENT_ROCKET_AIR_2ND: u16 = 270;
    /// `HasIntent(intent = NoseGrab)`.
    pub(crate) const HAS_INTENT_NOSE_GRAB: u16 = 271;
    /// `HasIntent(intent = NoseGrab)`, declared again: the test of `HAS_INTENT_NOSE_GRAB` under its
    /// own index.
    pub(crate) const HAS_INTENT_NOSE_GRAB_2ND: u16 = 272;
    /// `HasIntent(intent = CrailGrab)`.
    pub(crate) const HAS_INTENT_CRAIL_GRAB: u16 = 273;
    /// `HasIntent(intent = CrailGrab)`, declared again: the test of `HAS_INTENT_CRAIL_GRAB` under
    /// its own index.
    pub(crate) const HAS_INTENT_CRAIL_GRAB_2ND: u16 = 274;
    /// `HasAnimAttribute(attribute = sequencetoair)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_SEQUENCE_TO_AIR: u16 = 275;
    /// `HasIntent(intent = L_F_Kickflip)`.
    pub(crate) const HAS_INTENT_L_F_KICKFLIP: u16 = 276;
    /// `HasIntent(intent = L_F_Heelflip)`.
    pub(crate) const HAS_INTENT_L_F_HEELFLIP: u16 = 277;
    /// `HasIntent(intent = L_B_Kickflip)`.
    pub(crate) const HAS_INTENT_L_B_KICKFLIP: u16 = 278;
    /// `HasIntent(intent = L_B_Heelflip)`.
    pub(crate) const HAS_INTENT_L_B_HEELFLIP: u16 = 279;
    /// `HasIntent(intent = L_BS_Shuvit)`.
    pub(crate) const HAS_INTENT_L_BS_SHUVIT: u16 = 280;
    /// `HasIntent(intent = L_FS_Shuvit)`.
    pub(crate) const HAS_INTENT_L_FS_SHUVIT: u16 = 281;
    /// `HasTiltToLargeForPreland`.
    pub(crate) const HAS_TILT_TO_LARGE_FOR_PRELAND: u16 = 282;
    /// `HasLandingType(landingType = spin)`.
    pub(crate) const HAS_LANDING_TYPE_SPIN: u16 = 283;
    /// `HasLandingType(landingType = sketchy)`.
    pub(crate) const HAS_LANDING_TYPE_SKETCHY: u16 = 284;
    /// `InParentStateForTime(greater = 0.1)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_0_1: u16 = 285;
    /// `HasAnimAttribute(attribute = absorbLand)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_ABSORB_LAND: u16 = 286;
    /// `HasIntent(intent = GroundDBLGrab)`.
    pub(crate) const HAS_INTENT_GROUND_DBL_GRAB: u16 = 287;
    /// `HasIntent(intent = GroundBSGrab)`.
    pub(crate) const HAS_INTENT_GROUND_BS_GRAB: u16 = 288;
    /// `IsEnteringSkitch`.
    pub(crate) const IS_ENTERING_SKITCH: u16 = 289;
    /// `HasIntent(intent = NewKickTurn)`.
    pub(crate) const HAS_INTENT_NEW_KICK_TURN: u16 = 290;
    /// `HasIntent(intent = KickTurn)`.
    pub(crate) const HAS_INTENT_KICK_TURN: u16 = 291;
    /// `IsEnteringSkitch`, declared again: the test of `IS_ENTERING_SKITCH` under its own index.
    pub(crate) const IS_ENTERING_SKITCH_2ND: u16 = 292;
    /// `IsSkitching`.
    pub(crate) const IS_SKITCHING: u16 = 293;
    /// `SkitchingPosition(side = both)`.
    pub(crate) const SKITCHING_POSITION_BOTH: u16 = 294;
    /// `IsSkitchingWithAbsorb`.
    pub(crate) const IS_SKITCHING_WITH_ABSORB: u16 = 295;
    /// `WillExpire(InTime = 0.5)`.
    pub(crate) const WILL_EXPIRE_IN_0_5: u16 = 296;
    /// `IsSkitchShimmying(direction = left)`.
    pub(crate) const IS_SKITCH_SHIMMYING_LEFT: u16 = 297;
    /// `IsSkitchShimmying(direction = right)`.
    pub(crate) const IS_SKITCH_SHIMMYING_RIGHT: u16 = 298;
    /// `SkitchingPosition(side = front)`.
    pub(crate) const SKITCHING_POSITION_FRONT: u16 = 299;
    /// `SkitchingPosition(side = back)`.
    pub(crate) const SKITCHING_POSITION_BACK: u16 = 300;
    /// `PhysicsSpeedCompare(abs = true, less = 0.1)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_LESS_0_1: u16 = 301;
    /// `HasIntent(intent = NoInput)`.
    pub(crate) const HAS_INTENT_NO_INPUT: u16 = 302;
    /// `SkateSlope(less = 5.0)`.
    pub(crate) const SKATE_SLOPE_LESS_5: u16 = 303;
    /// `IsProSkater(skater = PJLadd)`.
    pub(crate) const IS_PRO_SKATER_PJ_LADD: u16 = 304;
    /// `RandomCond`.
    pub(crate) const RANDOM_COND: u16 = 305;
    /// `IsProSkater(skater = JasonDill)`.
    pub(crate) const IS_PRO_SKATER_JASON_DILL: u16 = 306;
    /// `IsProSkater(skater = JerryHsu)`.
    pub(crate) const IS_PRO_SKATER_JERRY_HSU: u16 = 307;
    /// `IsBumped`.
    pub(crate) const IS_BUMPED: u16 = 308;
    /// `SurfaceSlope(less = 70)`.
    pub(crate) const SURFACE_SLOPE_LESS_70: u16 = 309;
    /// `PhysicsSpeedCompare(abs = true, less = 0.15)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_LESS_0_15: u16 = 310;
    /// `HasIntent(intent = Brake)`.
    pub(crate) const HAS_INTENT_BRAKE: u16 = 311;
    /// `LastState(state = Turning.Idle)`.
    pub(crate) const LAST_STATE_TURNING_IDLE: u16 = 312;
    /// `LastState(state = KickTurn)`.
    pub(crate) const LAST_STATE_KICK_TURN: u16 = 313;
    /// `HasIntent(intent = KickTurn)`, declared again: the test of `HAS_INTENT_KICK_TURN` under its
    /// own index.
    pub(crate) const HAS_INTENT_KICK_TURN_2ND: u16 = 314;
    /// `WillExpire(InTime = 0.25)`.
    pub(crate) const WILL_EXPIRE_IN_0_25: u16 = 315;
    /// `HasIntent(greater = 0.0, intent = KickTurn)`.
    pub(crate) const HAS_INTENT_KICK_TURN_GREATER_0: u16 = 316;
    /// `HasIntent(intent = KickTurn, less = 0.0)`.
    pub(crate) const HAS_INTENT_KICK_TURN_LESS_0: u16 = 317;
    /// `CanEnterSlide(right = false)`.
    pub(crate) const CAN_ENTER_SLIDE_NOT_RIGHT: u16 = 318;
    /// `InParentStateForTime(greater = 0.3)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_0_3: u16 = 319;
    /// `ShouldLeaveSlide(right = false)`.
    pub(crate) const SHOULD_LEAVE_SLIDE_NOT_RIGHT: u16 = 320;
    /// `HasAnimAttribute(attribute = earlyout)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_EARLY_OUT: u16 = 321;
    /// `HasAnimAttribute(attribute = midout)`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_MID_OUT: u16 = 322;
    /// `WillExpire(InTime = 0.1, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_1_NOT_WAIT: u16 = 323;
    /// `CanEnterSlide`.
    pub(crate) const CAN_ENTER_SLIDE: u16 = 324;
    /// `ShouldLeaveSlide`.
    pub(crate) const SHOULD_LEAVE_SLIDE: u16 = 325;
    /// `IsHandPlanting(dir = FS, state = ground)`.
    pub(crate) const IS_HAND_PLANTING_FS_GROUND: u16 = 326;
    /// `HasNewHandPlantPos`.
    pub(crate) const HAS_NEW_HAND_PLANT_POS: u16 = 327;
    /// `IsHandPlanting(dir = BS, state = ground)`.
    pub(crate) const IS_HAND_PLANTING_BS_GROUND: u16 = 328;
    /// `HasIntent(intent = SlideBs180)`.
    pub(crate) const HAS_INTENT_SLIDE_BS_180: u16 = 329;
    /// `HasIntent(intent = SlideFs180)`.
    pub(crate) const HAS_INTENT_SLIDE_FS_180: u16 = 330;
    /// `IsProSkater(skater = Loose)`.
    pub(crate) const IS_PRO_SKATER_LOOSE: u16 = 331;
    /// `IsProSkater(skater = Aggressive)`.
    pub(crate) const IS_PRO_SKATER_AGGRESSIVE: u16 = 332;
    /// `IsProSkater(skater = MarkGonzales)`, declared again: the test of
    /// `IS_PRO_SKATER_MARK_GONZALES` under its own index.
    pub(crate) const IS_PRO_SKATER_MARK_GONZALES_2ND: u16 = 333;
    /// `IsProSkater(skater = DannyWay)`.
    pub(crate) const IS_PRO_SKATER_DANNY_WAY: u16 = 334;
    /// `IsProSkater(skater = JerryHsu)`, declared again: the test of `IS_PRO_SKATER_JERRY_HSU`
    /// under its own index.
    pub(crate) const IS_PRO_SKATER_JERRY_HSU_2ND: u16 = 335;
    /// `InParentStateForTime(greater = 3.0)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_3: u16 = 336;
    /// `IsRidingSwitch`.
    pub(crate) const IS_RIDING_SWITCH: u16 = 337;
    /// `InParentStateForTime(greater = 0.6)`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_0_6: u16 = 338;
    /// `HasIntent(intent = GroundBSGrab)`, declared again: the test of `HAS_INTENT_GROUND_BS_GRAB`
    /// under its own index.
    pub(crate) const HAS_INTENT_GROUND_BS_GRAB_2ND: u16 = 339;
    /// `CanEnterSlide(right = true)`.
    pub(crate) const CAN_ENTER_SLIDE_RIGHT: u16 = 340;
    /// `ShouldLeaveSlide(right = true)`.
    pub(crate) const SHOULD_LEAVE_SLIDE_RIGHT: u16 = 341;
    /// `HasIntent(intent = GroundFSGrab)`.
    pub(crate) const HAS_INTENT_GROUND_FS_GRAB: u16 = 342;
    /// `HasIntent(intent = GroundFSGrab)`, declared again: the test of `HAS_INTENT_GROUND_FS_GRAB`
    /// under its own index.
    pub(crate) const HAS_INTENT_GROUND_FS_GRAB_2ND: u16 = 343;
    /// `HasIntent(intent = GroundDBLGrab)`, declared again: the test of
    /// `HAS_INTENT_GROUND_DBL_GRAB` under its own index.
    pub(crate) const HAS_INTENT_GROUND_DBL_GRAB_2ND: u16 = 344;
    /// `HasIntent(greater = 0.2, intent = ManualEngageTime)`.
    pub(crate) const HAS_INTENT_MANUAL_ENGAGE_TIME_GREATER_0_2: u16 = 345;
    /// `LastState(state = InAir)`.
    pub(crate) const LAST_STATE_IN_AIR: u16 = 346;
    /// `ComVelCompare(axis = Y, less = -8.0)`.
    pub(crate) const COM_VEL_COMPARE_AXIS_Y_LESS_NEG_8: u16 = 347;
    /// `SurfaceSlope(greater = 45)`.
    pub(crate) const SURFACE_SLOPE_GREATER_45: u16 = 348;
    /// `InStateForTime(greater = 0.5, state = OnGround)`.
    pub(crate) const IN_STATE_FOR_TIME_ON_GROUND_GREATER_0_5: u16 = 349;
    /// `HasIntent(intent = Manual, less = 0.0)`.
    pub(crate) const HAS_INTENT_MANUAL_LESS_0: u16 = 350;
    /// `HasIntent(intent = ManualBrake)`.
    pub(crate) const HAS_INTENT_MANUAL_BRAKE: u16 = 351;
    /// `PhysicsSpeedCompare(alongSkateZ = true, less = -0.5)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_LESS_NEG_0_5: u16 = 352;
    /// `HasIntent(intent = ManualBrake)`, declared again: the test of `HAS_INTENT_MANUAL_BRAKE`
    /// under its own index.
    pub(crate) const HAS_INTENT_MANUAL_BRAKE_2ND: u16 = 353;
    /// `PhysicsSpeedCompare(abs = true, alongSkateZ = true, greater = 0.1)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ABS_ALONG_SKATE_Z_GREATER_0_1: u16 = 354;
    /// `HasIntent(greater = 0.0, intent = Manual)`.
    pub(crate) const HAS_INTENT_MANUAL_GREATER_0: u16 = 355;
    /// `HasIntent(greater = 0.0, intent = Manual)`, declared again: the test of
    /// `HAS_INTENT_MANUAL_GREATER_0` under its own index.
    pub(crate) const HAS_INTENT_MANUAL_GREATER_0_2ND: u16 = 356;
    /// `PhysicsSpeedCompare(alongSkateZ = true, greater = 0.5)`.
    pub(crate) const PHYSICS_SPEED_COMPARE_ALONG_SKATE_Z_GREATER_0_5: u16 = 357;
    /// `MongoPushFootToFar`.
    pub(crate) const MONGO_PUSH_FOOT_TO_FAR: u16 = 358;
    /// `CurrentState(state = OnGround.RidingIdle.Riding.Turning.Push.PushStart.Right.Default)`.
    pub(crate) const CURRENT_STATE_PUSH_START_RIGHT_DEFAULT: u16 = 359;
    /// `IsRidingGoofy`, declared again: the test of `IS_RIDING_GOOFY` under its own index.
    pub(crate) const IS_RIDING_GOOFY_2ND: u16 = 360;
    /// `CurrentState(state = OnGround.RidingIdle.Riding.Turning.Push.PushStart.Left.Default)`.
    pub(crate) const CURRENT_STATE_PUSH_START_LEFT_DEFAULT: u16 = 361;
    /// `HasIntent(intent = RightPush)`.
    pub(crate) const HAS_INTENT_RIGHT_PUSH: u16 = 362;
    /// `HasIntent(intent = LeftPush)`.
    pub(crate) const HAS_INTENT_LEFT_PUSH: u16 = 363;
    /// `CurrentState(state = PushFromFakie)`.
    pub(crate) const CURRENT_STATE_PUSH_FROM_FAKIE: u16 = 364;
    /// `CurrentState(state = Switch)`.
    pub(crate) const CURRENT_STATE_SWITCH: u16 = 365;
    /// `HasGestureIntent(group = Square)`.
    pub(crate) const HAS_GESTURE_INTENT_SQUARE: u16 = 366;
    /// `WillExpire(InTime = 0.01, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_01_NOT_WAIT: u16 = 367;
    /// `BreakOutOfPush`.
    pub(crate) const BREAK_OUT_OF_PUSH: u16 = 368;
    /// `HasIntent(intent = NewPush)`.
    pub(crate) const HAS_INTENT_NEW_PUSH: u16 = 369;
    /// `WillExpire(InTime = 0.0000001, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_0000001_NOT_WAIT: u16 = 370;
    /// `HasIntent(intent = OllieAnticipation)`.
    pub(crate) const HAS_INTENT_OLLIE_ANTICIPATION: u16 = 371;
    /// `HasIntent(intent = PopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_POP_SHUVIT_ANTICIPATION: u16 = 372;
    /// `HasIntent(intent = FSPopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION: u16 = 373;
    /// `HasIntent(intent = 360PopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_360_POP_SHUVIT_ANTICIPATION: u16 = 374;
    /// `HasIntent(intent = FS360PopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION: u16 = 375;
    /// `HasIntent(intent = NollieAnticipation)`.
    pub(crate) const HAS_INTENT_NOLLIE_ANTICIPATION: u16 = 376;
    /// `HasIntent(intent = N_PopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_N_POP_SHUVIT_ANTICIPATION: u16 = 377;
    /// `HasIntent(intent = N_FSPopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION: u16 = 378;
    /// `HasIntent(intent = N_360PopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION: u16 = 379;
    /// `HasIntent(intent = N_FS360PopShuvitAnticipation)`.
    pub(crate) const HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION: u16 = 380;
    /// `WillExpire(InTime = 0.07, waitForTransitions = false)`.
    pub(crate) const WILL_EXPIRE_IN_0_07_NOT_WAIT: u16 = 381;
    /// `HasIntent(intent = OllieAnticipation)`, declared again: the test of
    /// `HAS_INTENT_OLLIE_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_OLLIE_ANTICIPATION_2ND: u16 = 382;
    /// `HasIntent(intent = 360PopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_360_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_360_POP_SHUVIT_ANTICIPATION_2ND: u16 = 383;
    /// `HasIntent(intent = PopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_POP_SHUVIT_ANTICIPATION_2ND: u16 = 384;
    /// `HasIntent(intent = FS360PopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_FS_360_POP_SHUVIT_ANTICIPATION_2ND: u16 = 385;
    /// `HasIntent(intent = FSPopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_FS_POP_SHUVIT_ANTICIPATION_2ND: u16 = 386;
    /// `HasIntent(intent = NollieAnticipation)`, declared again: the test of
    /// `HAS_INTENT_NOLLIE_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_NOLLIE_ANTICIPATION_2ND: u16 = 387;
    /// `HasIntent(intent = N_360PopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_N_360_POP_SHUVIT_ANTICIPATION_2ND: u16 = 388;
    /// `HasIntent(intent = N_PopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_N_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_N_POP_SHUVIT_ANTICIPATION_2ND: u16 = 389;
    /// `HasIntent(intent = N_FS360PopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_N_FS_360_POP_SHUVIT_ANTICIPATION_2ND: u16 = 390;
    /// `HasIntent(intent = N_FSPopShuvitAnticipation)`, declared again: the test of
    /// `HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION` under its own index.
    pub(crate) const HAS_INTENT_N_FS_POP_SHUVIT_ANTICIPATION_2ND: u16 = 391;
    /// `WillExpire(InTime = 0.075)`.
    pub(crate) const WILL_EXPIRE_IN_0_075: u16 = 392;
    /// `WillExpire(InTime = 0.75)`.
    pub(crate) const WILL_EXPIRE_IN_0_75: u16 = 393;
    /// `HasIntent(intent = NewPush)`, declared again: the test of `HAS_INTENT_NEW_PUSH` under its
    /// own index.
    pub(crate) const HAS_INTENT_NEW_PUSH_2ND: u16 = 394;
    /// `HasIntent(intent = RightPush)`, declared again: the test of `HAS_INTENT_RIGHT_PUSH` under
    /// its own index.
    pub(crate) const HAS_INTENT_RIGHT_PUSH_2ND: u16 = 395;
    /// `HasIntent(intent = LeftPush)`, declared again: the test of `HAS_INTENT_LEFT_PUSH` under its
    /// own index.
    pub(crate) const HAS_INTENT_LEFT_PUSH_2ND: u16 = 396;
    /// `InStateForTime(greater = 0.49, state = Anticipation)`.
    pub(crate) const IN_STATE_FOR_TIME_ANTICIPATION_GREATER_0_49: u16 = 397;
    /// `PhysFilteredState(state = offboard)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_OFFBOARD` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD_2ND: u16 = 398;
    /// `ApexReached`.
    pub(crate) const APEX_REACHED: u16 = 399;
    /// `CanLandOnBoard`.
    pub(crate) const CAN_LAND_ON_BOARD: u16 = 400;
    /// `OBTimeToLand(greater = 0.05)`.
    pub(crate) const OB_TIME_TO_LAND_GREATER_0_05: u16 = 401;
    /// `PhysFilteredState(state = teleport)`, declared again: the test of
    /// `PHYS_FILTERED_STATE_TELEPORT` under its own index.
    pub(crate) const PHYS_FILTERED_STATE_TELEPORT_2ND: u16 = 402;
    /// `WipeoutTimeToLand(less = 0.75)`.
    pub(crate) const WIPEOUT_TIME_TO_LAND_LESS_0_75: u16 = 403;
    /// `IsDoneWipingOut`.
    pub(crate) const IS_DONE_WIPING_OUT: u16 = 404;
    /// `IsInDebugAnimationsMode`.
    pub(crate) const IS_IN_DEBUG_ANIMATIONS_MODE: u16 = 405;
    /// `HasIntent(intent = StepForwardRight)`.
    pub(crate) const HAS_INTENT_STEP_FORWARD_RIGHT: u16 = 406;
    /// `HasIntent(intent = StepForwardLeft)`.
    pub(crate) const HAS_INTENT_STEP_FORWARD_LEFT: u16 = 407;
    /// `LastState(state = ToStepForwardRight)`.
    pub(crate) const LAST_STATE_TO_STEP_FORWARD_RIGHT: u16 = 408;
    /// `HasIntent(intent = StepRight)`.
    pub(crate) const HAS_INTENT_STEP_RIGHT: u16 = 409;
    /// `HasIntent(intent = StepLeft)`.
    pub(crate) const HAS_INTENT_STEP_LEFT: u16 = 410;
    /// `LastState(state = ToStepRight)`.
    pub(crate) const LAST_STATE_TO_STEP_RIGHT: u16 = 411;
    /// `HasIntent(intent = StepBackRight)`.
    pub(crate) const HAS_INTENT_STEP_BACK_RIGHT: u16 = 412;
    /// `HasIntent(intent = StepBackLeft)`.
    pub(crate) const HAS_INTENT_STEP_BACK_LEFT: u16 = 413;
    /// `LastState(state = ToStepBackRight)`.
    pub(crate) const LAST_STATE_TO_STEP_BACK_RIGHT: u16 = 414;
    /// `LastState(state = ToStepBackLeft)`.
    pub(crate) const LAST_STATE_TO_STEP_BACK_LEFT: u16 = 415;
    /// `LastState(state = ToStepLeft)`.
    pub(crate) const LAST_STATE_TO_STEP_LEFT: u16 = 416;
    /// `LastState(state = ToStepForwardLeft)`.
    pub(crate) const LAST_STATE_TO_STEP_FORWARD_LEFT: u16 = 417;
}

/// Expressions: which states and transitions evaluate each expression index.
pub(crate) mod expression {
    /// State `OBGround8`.
    pub(crate) const OB_GROUND_8: u16 = 0;
    /// State `SpawnDismount31`.
    pub(crate) const SPAWN_DISMOUNT_31: u16 = 1;
    /// Shared by 9: `SpawnDismount31` -> `Locomotion33`, `Antic1354` -> `Takeoff1355`,
    /// `Takeoff1355` -> `JumpingIntoMount1358`, `JumpingIntoMount1358` -> `IntoBSGrabOut1359` and 5
    /// more.
    pub(crate) const WILL_EXPIRE_IN_0: u16 = 2;
    /// Shared by 2: state `DismountFromFakie100`, state `PushFromFakie1178`.
    pub(crate) const IS_RIDING_FAKIE: u16 = 3;
    /// Shared by 4: `DismountFromFakie100` -> `IdleDismount101`, `DismountFromFakie100` ->
    /// `RunDismount102`, `Right1276` -> `FromFakie1339`, `Left1277` -> `FromFakie1341`.
    pub(crate) const WILL_EXPIRE_IN_0_25_NOT_WAIT: u16 = 4;
    /// Shared by 51: `IdleDismount101` -> `Locomotion33`, `MVOBJ_Into307` -> `MVOBJ_Still306`,
    /// `Land1357` -> `Standing308`, `Land1357` -> `RunAround107` and 47 more.
    pub(crate) const WILL_EXPIRE_IN_0_1: u16 = 5;
    /// Shared by 3: `RunDismountWaitForLanding103` -> `RunAround107`, `KickoutToAir14` ->
    /// `Locomotion33`, `TrickDismount15` -> `Locomotion33`.
    pub(crate) const OB_TIME_TO_LAND_LESS_EQUAL_0: u16 = 6;
    /// `RunDismountWaitForLanding103` -> `RightFootStride37`.
    pub(crate) const RUN_DISMOUNT_WAIT_FOR_LANDING_103_TO_RIGHT_FOOT_STRIDE_37: u16 = 7;
    /// State `MovingObject104`.
    pub(crate) const MOVING_OBJECT_104: u16 = 8;
    /// `MovingObject104` -> `Locomotion33`.
    pub(crate) const MOVING_OBJECT_104_TO_LOCOMOTION_33: u16 = 9;
    /// Shared by 2: state `MVOBJ_PUSH_LEFT1282`, state `MVOBJ_PUSH_LEFT2283`.
    pub(crate) const MVOBJ_PUSH_LEFT: u16 = 10;
    /// Shared by 141: `MVOBJ_PUSH_LEFT1282` -> `MVOBJ_PUSH_LEFT2283`, `MVOBJ_PUSH_LEFT2283` ->
    /// `MVOBJ_PUSH_LEFT1282`, `MVOBJ_PUSH_RIGHT1284` -> `MVOBJ_PUSH_RIGHT2285`,
    /// `MVOBJ_PUSH_RIGHT2285` -> `MVOBJ_PUSH_RIGHT1284` and 137 more.
    pub(crate) const WILL_EXPIRE_IN_0_05: u16 = 11;
    /// Shared by 2: state `MVOBJ_PUSH_RIGHT1284`, state `MVOBJ_PUSH_RIGHT2285`.
    pub(crate) const MVOBJ_PUSH_RIGHT: u16 = 12;
    /// Shared by 2: state `MVOBJ_PULL_LEFT1286`, state `MVOBJ_PULL_LEFT2287`.
    pub(crate) const MVOBJ_PULL_LEFT: u16 = 13;
    /// Shared by 2: state `MVOBJ_PULL_RIGHT1288`, state `MVOBJ_PULL_RIGHT2289`.
    pub(crate) const MVOBJ_PULL_RIGHT: u16 = 14;
    /// Shared by 4: state `MVOBJ_PUSHLEFTToRIGHT1290`, state `MVOBJ_PUSHLEFTToRIGHT2291`, state
    /// `MVOBJ_PULLToPUSHRIGHT1304`, state `MVOBJ_PULLToPUSHRIGHT2305`.
    pub(crate) const MVOBJ_TO_PUSH_RIGHT: u16 = 15;
    /// Shared by 16: state `Immediate720`, state `Immediate722`, state `Immediate724`, state
    /// `Immediate726` and 12 more.
    pub(crate) const IMMEDIATE: u16 = 16;
    /// Shared by 16: state `WaitForEnd1032`, state `WaitForEnd1034`, state `WaitForEnd1036`, state
    /// `WaitForEnd1038` and 12 more.
    pub(crate) const WAIT_FOR_END: u16 = 17;
    /// Shared by 4: state `MVOBJ_PUSHRIGHTToLEFT1292`, state `MVOBJ_PUSHRIGHTToLEFT2293`, state
    /// `MVOBJ_PULLToPUSHLEFT1302`, state `MVOBJ_PULLToPUSHLEFT2303`.
    pub(crate) const MVOBJ_TO_PUSH_LEFT: u16 = 18;
    /// Shared by 4: state `MVOBJ_PULLLEFTToRIGHT1294`, state `MVOBJ_PULLLEFTToRIGHT2295`, state
    /// `MVOBJ_PUSHToPULLRIGHT1300`, state `MVOBJ_PUSHToPULLRIGHT2301`.
    pub(crate) const MVOBJ_TO_PULL_RIGHT: u16 = 19;
    /// Shared by 4: state `MVOBJ_PULLRIGHTToLEFT1296`, state `MVOBJ_PULLRIGHTToLEFT2297`, state
    /// `MVOBJ_PUSHToPULLLEFT1298`, state `MVOBJ_PUSHToPULLLEFT2299`.
    pub(crate) const MVOBJ_TO_PULL_LEFT: u16 = 20;
    /// Shared by 2: state `MVOBJ_Still306`, state `MVOBJ_Into307`.
    pub(crate) const MVOBJ_NOT_MOVING: u16 = 21;
    /// Shared by 9: `Standing308` -> `RunAround107`, `StepForward309` -> `RunAround107`,
    /// `StepForwardRight310` -> `RunAround107`, `StepRight311` -> `RunAround107` and 5 more.
    pub(crate) const IDLE_TO_RUN_AROUND: u16 = 22;
    /// Shared by 82: `StepForward309` -> `Standing308`, `StepBackward313` -> `Standing308`,
    /// `ToStepForward317` -> `StepForward309`, `ToStepBackward318` -> `StepBackward313` and 78
    /// more.
    pub(crate) const OFF_BOARD_WILL_EXPIRE_IN_0_1: u16 = 23;
    /// Shared by 2: `ToStepForward317` -> `Mount34`, `ToStepBackward318` -> `Mount34`.
    pub(crate) const TO_MOUNT: u16 = 24;
    /// State `StandingJump106`.
    pub(crate) const STANDING_JUMP_106: u16 = 25;
    /// `Takeoff1355` -> `Descent1356`.
    pub(crate) const TAKEOFF_1355_TO_DESCENT_1356: u16 = 26;
    /// `Descent1356` -> `JumpingIntoMount1358`.
    pub(crate) const DESCENT_1356_TO_JUMPING_INTO_MOUNT_1358: u16 = 27;
    /// Shared by 3: state `Land1357`, `JumpingIntoMountLeft9` -> `WipeOut4`,
    /// `JumpingIntoMountRight10` -> `WipeOut4`.
    pub(crate) const PHYS_FILTERED_STATE_OFFBOARD: u16 = 28;
    /// State `JumpingIntoMount1358`.
    pub(crate) const JUMPING_INTO_MOUNT_1358: u16 = 29;
    /// Shared by 2: state `IntoBSGrabOut11`, state `IntoBSGrabOut1359`.
    pub(crate) const INTO_BS_GRAB_OUT: u16 = 30;
    /// `LocoNew319` -> `ToLandingOnBoardFromLeft26`.
    pub(crate) const LOCO_NEW_319_TO_TO_LANDING_ON_BOARD_FROM_LEFT_26: u16 = 31;
    /// `LocoNew319` -> `ToLandingOnBoardFromRight27`.
    pub(crate) const LOCO_NEW_319_TO_TO_LANDING_ON_BOARD_FROM_RIGHT_27: u16 = 32;
    /// `Mount34` -> `OnBoard3`.
    pub(crate) const MOUNT_34_TO_ON_BOARD_3: u16 = 33;
    /// Shared by 2: `RightFootStride37` -> `RunAround107`, `LeftFootStride38` -> `RunAround107`.
    pub(crate) const STRIDE_TO_RUN_AROUND: u16 = 34;
    /// Shared by 7: state `KickoutDismount13`, state `TrickDismount15`, state `ToOffBoard646`,
    /// state `ToOffBoard657` and 3 more.
    pub(crate) const HAS_INTENT_OB_DISMOUNT: u16 = 35;
    /// `KickoutDismount13` -> `KickoutToAir14`.
    pub(crate) const KICKOUT_DISMOUNT_13_TO_KICKOUT_TO_AIR_14: u16 = 36;
    /// `KickoutDismount13` -> `WipeOut4`.
    pub(crate) const KICKOUT_DISMOUNT_13_TO_WIPE_OUT_4: u16 = 37;
    /// `HippyRunout16` -> `WipeOut4`.
    pub(crate) const HIPPY_RUNOUT_16_TO_WIPE_OUT_4: u16 = 38;
    /// Shared by 22: state `AccidentalDismount17`, `Grinding18` -> `AccidentalDismount17`,
    /// `PlantingFoot604` -> `WipeOut4`, `FootPlant605` -> `WipeOut4` and 18 more.
    pub(crate) const PHYSICS_WANTS_RUNOUT: u16 = 39;
    /// State `Fly116`.
    pub(crate) const FLY_116: u16 = 40;
    /// Shared by 212: `Fly116` -> `RunAround107`, `Fly116` -> `RightFootStride37`, `Finish117` ->
    /// `RunAround107`, `Into320` -> `GrabCycle322` and 208 more.
    pub(crate) const WILL_EXPIRE_IN_0_01: u16 = 41;
    /// State `ForceBrake2`.
    pub(crate) const FORCE_BRAKE_2: u16 = 42;
    /// State `Grinding18`.
    pub(crate) const GRINDING_18: u16 = 43;
    /// State `BS_FEEBLE40`.
    pub(crate) const BS_FEEBLE_40: u16 = 44;
    /// State `FS_FEEBLE41`.
    pub(crate) const FS_FEEBLE_41: u16 = 45;
    /// State `BS_SMITH42`.
    pub(crate) const BS_SMITH_42: u16 = 46;
    /// State `FS_SMITH43`.
    pub(crate) const FS_SMITH_43: u16 = 47;
    /// State `FS_WILLY44`.
    pub(crate) const FS_WILLY_44: u16 = 48;
    /// State `BS_WILLY45`.
    pub(crate) const BS_WILLY_45: u16 = 49;
    /// State `FS_OVERWILLY46`.
    pub(crate) const FS_OVERWILLY_46: u16 = 50;
    /// State `BS_OVERWILLY47`.
    pub(crate) const BS_OVERWILLY_47: u16 = 51;
    /// State `BS_5_O48`.
    pub(crate) const BS_5_O_48: u16 = 52;
    /// State `FS_5_O49`.
    pub(crate) const FS_5_O_49: u16 = 53;
    /// State `BS_50_5050`.
    pub(crate) const BS_50_50_50: u16 = 54;
    /// State `FS_50_5051`.
    pub(crate) const FS_50_50_51: u16 = 55;
    /// State `BS_CROOK52`.
    pub(crate) const BS_CROOK_52: u16 = 56;
    /// State `FS_CROOK53`.
    pub(crate) const FS_CROOK_53: u16 = 57;
    /// State `FS_OVERCROOK54`.
    pub(crate) const FS_OVERCROOK_54: u16 = 58;
    /// State `BS_OVERCROOK55`.
    pub(crate) const BS_OVERCROOK_55: u16 = 59;
    /// State `BS_NOSEGRIND56`.
    pub(crate) const BS_NOSEGRIND_56: u16 = 60;
    /// State `FS_NOSEGRIND57`.
    pub(crate) const FS_NOSEGRIND_57: u16 = 61;
    /// State `BS_SALAD58`.
    pub(crate) const BS_SALAD_58: u16 = 62;
    /// State `FS_SALAD59`.
    pub(crate) const FS_SALAD_59: u16 = 63;
    /// State `BS_NOSESLIDE60`.
    pub(crate) const BS_NOSESLIDE_60: u16 = 64;
    /// State `FS_NOSESLIDE61`.
    pub(crate) const FS_NOSESLIDE_61: u16 = 65;
    /// State `BS_NOSEBLUNT62`.
    pub(crate) const BS_NOSEBLUNT_62: u16 = 66;
    /// Shared by 16: state `Backslash428`, state `Backslash434`, state `Backslash440`, state
    /// `Backslash446` and 12 more.
    pub(crate) const BACKSLASH: u16 = 67;
    /// Shared by 16: state `Rail429`, state `Rail435`, state `Rail441`, state `Rail447` and 12
    /// more.
    pub(crate) const RAIL: u16 = 68;
    /// State `FS_NOSEBLUNT63`.
    pub(crate) const FS_NOSEBLUNT_63: u16 = 69;
    /// State `BS_BLUNT64`.
    pub(crate) const BS_BLUNT_64: u16 = 70;
    /// State `FS_BLUNT65`.
    pub(crate) const FS_BLUNT_65: u16 = 71;
    /// State `FS_TAILSLIDE66`.
    pub(crate) const FS_TAILSLIDE_66: u16 = 72;
    /// State `BS_TAILSLIDE67`.
    pub(crate) const BS_TAILSLIDE_67: u16 = 73;
    /// State `BS_BOARD68`.
    pub(crate) const BS_BOARD_68: u16 = 74;
    /// Shared by 4: state `Into456`, state `Into468`, state `Into480`, state `Into492`.
    pub(crate) const INTO_HAS_INTENT_GRIND_FS_GRAB: u16 = 75;
    /// Shared by 4: state `GrabCycle458`, state `GrabCycle470`, state `GrabCycle482`, state
    /// `GrabCycle494`.
    pub(crate) const GRAB_CYCLE_HAS_INTENT_GRIND_FS_GRAB: u16 = 76;
    /// Shared by 4: state `Out459`, state `Out471`, state `Out483`, state `Out495`.
    pub(crate) const OUT_NOT_HAS_INTENT_GRIND_FS_GRAB: u16 = 77;
    /// Shared by 4: state `Into460`, state `Into472`, state `Into484`, state `Into496`.
    pub(crate) const INTO_HAS_INTENT_GRIND_BS_GRAB: u16 = 78;
    /// Shared by 4: state `GrabCycle462`, state `GrabCycle474`, state `GrabCycle486`, state
    /// `GrabCycle498`.
    pub(crate) const GRAB_CYCLE_HAS_INTENT_GRIND_BS_GRAB: u16 = 79;
    /// Shared by 4: state `Out463`, state `Out475`, state `Out487`, state `Out499`.
    pub(crate) const OUT_NOT_HAS_INTENT_GRIND_BS_GRAB: u16 = 80;
    /// Shared by 4: state `Into464`, state `Into476`, state `Into488`, state `Into500`.
    pub(crate) const INTO_HAS_INTENT_GRIND_DOUBLE_GRAB: u16 = 81;
    /// Shared by 4: state `GrabCycle466`, state `GrabCycle478`, state `GrabCycle490`, state
    /// `GrabCycle502`.
    pub(crate) const GRAB_CYCLE_HAS_INTENT_GRIND_DOUBLE_GRAB: u16 = 82;
    /// State `FS_BOARD69`.
    pub(crate) const FS_BOARD_69: u16 = 83;
    /// State `BS_LIP70`.
    pub(crate) const BS_LIP_70: u16 = 84;
    /// State `FS_LIP71`.
    pub(crate) const FS_LIP_71: u16 = 85;
    /// Shared by 2: state `Kickflip_MikeCarroll186`, state `RandCarroll2709`.
    pub(crate) const IS_PRO_SKATER_MIKE_CARROLL: u16 = 86;
    /// Shared by 3: state `TrickInto504`, state `TrickInto509`, state `In517`.
    pub(crate) const HAS_INTENT_KICKFLIP: u16 = 87;
    /// Shared by 276: `Takeoff1064` -> `LeftGround1065`, `TakeOff780` -> `LeftGround781`,
    /// `TrickOut508` -> `InAir21`, `TrickOut508` -> `Land22` and 272 more.
    pub(crate) const WILL_EXPIRE_IN_0_05_NOT_WAIT: u16 = 88;
    /// Shared by 10: `LeftGround1065` -> `TrickOut508`, `LeftGround781` -> `TrickOut508`,
    /// `TrickCyc1505` -> `TrickCyc2506`, `TrickCyc2506` -> `TrickCyc3507` and 6 more.
    pub(crate) const WILL_EXPIRE_IN_0_02_NOT_WAIT: u16 = 89;
    /// Shared by 32: state `FromGrind788`, state `FromGrind807`, state `FromGrind812`, state
    /// `FromGrind817` and 28 more.
    pub(crate) const FROM_GRIND: u16 = 90;
    /// Shared by 2: state `Kickflip_MarkGonzales187`, state `RandGonz706`.
    pub(crate) const IS_PRO_SKATER_MARK_GONZALES: u16 = 91;
    /// State `Ollie188`.
    pub(crate) const OLLIE_188: u16 = 92;
    /// Shared by 2: `Ollie188` -> `BoardAdjusts78`, `Nollie203` -> `BoardAdjusts78`.
    pub(crate) const TO_BOARD_ADJUSTS: u16 = 93;
    /// State `In520`.
    pub(crate) const IN_520: u16 = 94;
    /// State `PopShuvit191`.
    pub(crate) const POP_SHUVIT_191: u16 = 95;
    /// State `FSPopShuvit192`.
    pub(crate) const FS_POP_SHUVIT_192: u16 = 96;
    /// State `VarialKickflip193`.
    pub(crate) const VARIAL_KICKFLIP_193: u16 = 97;
    /// State `VarialHeelflip194`.
    pub(crate) const VARIAL_HEELFLIP_194: u16 = 98;
    /// State `Hardflip195`.
    pub(crate) const HARDFLIP_195: u16 = 99;
    /// State `InwardHeelflip196`.
    pub(crate) const INWARD_HEELFLIP_196: u16 = 100;
    /// State `360PopShuvit197`.
    pub(crate) const STATE_360_POP_SHUVIT_197: u16 = 101;
    /// State `FS360PopShuvit198`.
    pub(crate) const FS_360_POP_SHUVIT_198: u16 = 102;
    /// State `360Flip199`.
    pub(crate) const STATE_360_FLIP_199: u16 = 103;
    /// State `Laserflip200`.
    pub(crate) const LASERFLIP_200: u16 = 104;
    /// State `360Hardflip201`.
    pub(crate) const STATE_360_HARDFLIP_201: u16 = 105;
    /// State `360InwardHeelflip202`.
    pub(crate) const STATE_360_INWARD_HEELFLIP_202: u16 = 106;
    /// State `Nollie203`.
    pub(crate) const NOLLIE_203: u16 = 107;
    /// State `In562`.
    pub(crate) const IN_562: u16 = 108;
    /// State `In565`.
    pub(crate) const IN_565: u16 = 109;
    /// State `N_PopShuvit206`.
    pub(crate) const N_POP_SHUVIT_206: u16 = 110;
    /// State `N_FSPopShuvit207`.
    pub(crate) const N_FS_POP_SHUVIT_207: u16 = 111;
    /// State `N_VarialKickflip208`.
    pub(crate) const N_VARIAL_KICKFLIP_208: u16 = 112;
    /// State `N_VarialHeelflip209`.
    pub(crate) const N_VARIAL_HEELFLIP_209: u16 = 113;
    /// State `N_Hardflip210`.
    pub(crate) const N_HARDFLIP_210: u16 = 114;
    /// State `N_InwardHeelflip211`.
    pub(crate) const N_INWARD_HEELFLIP_211: u16 = 115;
    /// State `N_360PopShuvit212`.
    pub(crate) const N_360_POP_SHUVIT_212: u16 = 116;
    /// State `N_FS360PopShuvit213`.
    pub(crate) const N_FS_360_POP_SHUVIT_213: u16 = 117;
    /// State `N_360Flip214`.
    pub(crate) const N_360_FLIP_214: u16 = 118;
    /// State `N_Laserflip215`.
    pub(crate) const N_LASERFLIP_215: u16 = 119;
    /// State `N_360Hardflip216`.
    pub(crate) const N_360_HARDFLIP_216: u16 = 120;
    /// State `N_360InwardHeelflip217`.
    pub(crate) const N_360_INWARD_HEELFLIP_217: u16 = 121;
    /// State `PushTrick74`.
    pub(crate) const PUSH_TRICK_74: u16 = 122;
    /// Shared by 5: state `Left943`, state `Left945`, state `Left947`, state `Left949` and 1 more.
    pub(crate) const PLANTING_FOOT_LEFT: u16 = 123;
    /// Shared by 5: state `Right944`, state `Right946`, state `Right948`, state `Right950` and 1
    /// more.
    pub(crate) const PLANTING_FOOT_RIGHT: u16 = 124;
    /// State `FS75`.
    pub(crate) const FS_75: u16 = 125;
    /// Shared by 2: state `FromGroundAntic223`, state `FromGroundAntic233`.
    pub(crate) const FROM_GROUND_ANTIC: u16 = 126;
    /// Shared by 2: state `Into224`, state `Into234`.
    pub(crate) const HAND_PLANT_INTO: u16 = 127;
    /// Shared by 9: `Into224` -> `Cyc227`, `Into624` -> `Cyc625`, `Into627` -> `Cyc628`, `Into630`
    /// -> `Cyc631` and 5 more.
    pub(crate) const WILL_EXPIRE_IN_0_02: u16 = 128;
    /// Shared by 6: `Into224` -> `Dismount232`, `Into224` -> `OneFootRight230`, `Into224` ->
    /// `OneFootLeft231`, `Into234` -> `Dismount242` and 2 more.
    pub(crate) const WILL_EXPIRE_IN_0_15: u16 = 129;
    /// Shared by 2: state `Antic225`, state `Antic235`.
    pub(crate) const HAND_PLANT_ANTIC: u16 = 130;
    /// Shared by 2: state `Out228`, state `Out238`.
    pub(crate) const HAND_PLANT_OUT: u16 = 131;
    /// Shared by 2: state `Into630`, state `Into639`.
    pub(crate) const HAND_PLANT_DISMOUNT_INTO: u16 = 132;
    /// State `BS76`.
    pub(crate) const BS_76: u16 = 133;
    /// Shared by 4: state `InAir21`, state `BoardAdjusts78`, state `InAirStatic79`, state
    /// `OneFootOllie81`.
    pub(crate) const PHYS_FILTERED_STATE_AIR: u16 = 134;
    /// State `Cyc643`.
    pub(crate) const CYC_643: u16 = 135;
    /// Shared by 3: `Cyc643` -> `Out644`, `Cyc648` -> `Out654`, `Cyc659` -> `Out665`.
    pub(crate) const CYC_TO_OUT_HAS_TWEAK_LESS_0_2: u16 = 136;
    /// State `Out644`.
    pub(crate) const OUT_644: u16 = 137;
    /// State `SupermanInto953`.
    pub(crate) const SUPERMAN_INTO_953: u16 = 138;
    /// State `SupermanOut955`.
    pub(crate) const SUPERMAN_OUT_955: u16 = 139;
    /// Shared by 2: state `ToFSChrist956`, state `Into958`.
    pub(crate) const HAS_INTENT_FS_CHRIST: u16 = 140;
    /// Shared by 2: state `ToBSChrist957`, state `Into970`.
    pub(crate) const HAS_INTENT_BS_CHRIST: u16 = 141;
    /// State `Into647`.
    pub(crate) const INTO_647: u16 = 142;
    /// Shared by 2: `Into958` -> `EarlyOut1106`, `Into970` -> `EarlyOut1124`.
    pub(crate) const HAS_ANIM_ATTRIBUTE_CHRIST_EARLY_OUT: u16 = 143;
    /// State `Out960`.
    pub(crate) const OUT_960: u16 = 144;
    /// Shared by 12: `Into1108` -> `EarlyOut1189`, `Into1112` -> `EarlyOut1194`, `Into1116` ->
    /// `EarlyOut1199`, `Into1120` -> `EarlyOut1204` and 8 more.
    pub(crate) const HAS_ANIM_ATTRIBUTE_ONE_FOOT_EARLY_OUT: u16 = 145;
    /// Shared by 8: state `Out999`, state `Out1005`, state `Out1110`, state `Out1118` and 4 more.
    pub(crate) const OUT_NOT_HAS_INTENT_RIGHT_ONE_FOOT_AIR: u16 = 146;
    /// Shared by 8: state `Footplant1111`, state `Footplant1115`, state `Footplant1119`, state
    /// `Footplant1123` and 4 more.
    pub(crate) const FOOTPLANT: u16 = 147;
    /// Shared by 8: `Into1191` -> `Absorb1192`, `Into1196` -> `Absorb1197`, `Into1201` ->
    /// `Absorb1202`, `Into1206` -> `Absorb1207` and 4 more.
    pub(crate) const INTO_TO_ABSORB: u16 = 148;
    /// Shared by 8: state `Out1002`, state `Out1008`, state `Out1114`, state `Out1122` and 4 more.
    pub(crate) const OUT_NOT_HAS_INTENT_LEFT_ONE_FOOT_AIR: u16 = 149;
    /// Shared by 6: state `FingerFlip651`, state `FingerFlip662`, state `FingerFlip671`, state
    /// `FingerFlip678` and 2 more.
    pub(crate) const FINGER_FLIP: u16 = 150;
    /// Shared by 2: state `FingerFlipVar653`, state `FingerFlipVar664`.
    pub(crate) const FINGER_FLIP_VAR: u16 = 151;
    /// State `Out654`.
    pub(crate) const OUT_654: u16 = 152;
    /// Shared by 4: `Out654` -> `WipeOut4`, `Out665` -> `WipeOut4`, `Out674` -> `WipeOut4`,
    /// `Out681` -> `WipeOut4`.
    pub(crate) const OUT_TO_WIPE_OUT: u16 = 153;
    /// Shared by 3: state `CoffinOut86`, `ToDBLGrab655` -> `Out644`, `ToDBLGrab666` -> `Out644`.
    pub(crate) const NOT_HAS_INTENT_COFFIN: u16 = 154;
    /// State `FromDBLGrab656`.
    pub(crate) const FROM_DBL_GRAB_656: u16 = 155;
    /// Shared by 2: `ToOffBoard657` -> `OBAir12`, `ToOffBoard668` -> `OBAir12`.
    pub(crate) const TO_OFF_BOARD_TO_OB_AIR: u16 = 156;
    /// State `Into658`.
    pub(crate) const INTO_658: u16 = 157;
    /// State `Out972`.
    pub(crate) const OUT_972: u16 = 158;
    /// State `Out665`.
    pub(crate) const OUT_665: u16 = 159;
    /// State `FromDBLGrab667`.
    pub(crate) const FROM_DBL_GRAB_667: u16 = 160;
    /// Shared by 2: state `Into669`, state `Cyc670`.
    pub(crate) const HAS_INTENT_MUTE_GRAB: u16 = 161;
    /// Shared by 2: `Cyc670` -> `Out674`, `Cyc677` -> `Out681`.
    pub(crate) const CYC_TO_OUT_TWEAK_Y_ABS_LESS_0_2: u16 = 162;
    /// State `Out674`.
    pub(crate) const OUT_674: u16 = 163;
    /// Shared by 2: state `Into676`, state `Cyc677`.
    pub(crate) const HAS_INTENT_STALE_GRAB: u16 = 164;
    /// State `Out681`.
    pub(crate) const OUT_681: u16 = 165;
    /// State `Left248`.
    pub(crate) const LEFT_248: u16 = 166;
    /// State `Right249`.
    pub(crate) const RIGHT_249: u16 = 167;
    /// Shared by 2: state `Idle685`, state `SeatbeltGrab1147`.
    pub(crate) const HAS_INTENT_BOARD_ADJUST_UP: u16 = 168;
    /// State `Cycle1143`.
    pub(crate) const CYCLE_1143: u16 = 169;
    /// Shared by 2: state `Out1231`, state `Out1242`.
    pub(crate) const OUT_NOT_HAS_INTENT_NO_FOOT_AIR_WALK: u16 = 170;
    /// State `Into1236`.
    pub(crate) const INTO_1236: u16 = 171;
    /// State `Cycle1237`.
    pub(crate) const CYCLE_1237: u16 = 172;
    /// State `Out1238`.
    pub(crate) const OUT_1238: u16 = 173;
    /// State `Out990`.
    pub(crate) const OUT_990: u16 = 174;
    /// Shared by 3: state `Idle687`, state `RocketAir688`, state `CrailGrab1153`.
    pub(crate) const HAS_INTENT_BOARD_ADJUST_DOWN: u16 = 175;
    /// State `Into992`.
    pub(crate) const INTO_992: u16 = 176;
    /// State `Cycle993`.
    pub(crate) const CYCLE_993: u16 = 177;
    /// State `Out994`.
    pub(crate) const OUT_994: u16 = 178;
    /// State `Cycle1149`.
    pub(crate) const CYCLE_1149: u16 = 179;
    /// State `Into1247`.
    pub(crate) const INTO_1247: u16 = 180;
    /// State `Cycle1248`.
    pub(crate) const CYCLE_1248: u16 = 181;
    /// State `Out1249`.
    pub(crate) const OUT_1249: u16 = 182;
    /// State `Out996`.
    pub(crate) const OUT_996: u16 = 183;
    /// State `Sequence690`.
    pub(crate) const SEQUENCE_690: u16 = 184;
    /// State `LateKickflip253`.
    pub(crate) const LATE_KICKFLIP_253: u16 = 185;
    /// State `LateHeelflip254`.
    pub(crate) const LATE_HEELFLIP_254: u16 = 186;
    /// State `LateBackfootKickflip255`.
    pub(crate) const LATE_BACKFOOT_KICKFLIP_255: u16 = 187;
    /// State `LateBackfootHeelflip256`.
    pub(crate) const LATE_BACKFOOT_HEELFLIP_256: u16 = 188;
    /// State `LateBSShuv257`.
    pub(crate) const LATE_BS_SHUV_257: u16 = 189;
    /// State `LateFSShuv258`.
    pub(crate) const LATE_FS_SHUV_258: u16 = 190;
    /// State `RightFootForward259`.
    pub(crate) const RIGHT_FOOT_FORWARD_259: u16 = 191;
    /// State `LeftFootForward260`.
    pub(crate) const LEFT_FOOT_FORWARD_260: u16 = 192;
    /// State `Spin261`.
    pub(crate) const SPIN_261: u16 = 193;
    /// State `Sketchy262`.
    pub(crate) const SKETCHY_262: u16 = 194;
    /// Shared by 2: `Land22` -> `Push1028`, `Land22` -> `DismountFromFakie100`.
    pub(crate) const IN_PARENT_STATE_FOR_TIME_GREATER_0_1: u16 = 195;
    /// `Land22` -> `Anticipation1030`.
    pub(crate) const LAND_22_TO_ANTICIPATION_1030: u16 = 196;
    /// State `CoffinInto84`.
    pub(crate) const COFFIN_INTO_84: u16 = 197;
    /// State `CoffinToDBL267`.
    pub(crate) const COFFIN_TO_DBL_267: u16 = 198;
    /// Shared by 2: state `CoffinToBS268`, state `BSGrabInto1255`.
    pub(crate) const HAS_INTENT_GROUND_BS_GRAB_INTO: u16 = 199;
    /// State `OnGround24`.
    pub(crate) const ON_GROUND_24: u16 = 200;
    /// Shared by 2: state `Into270`, state `Cyc271`.
    pub(crate) const IS_ENTERING_SKITCH: u16 = 201;
    /// Shared by 2: `Into270` -> `RidingIdle89`, `Cyc271` -> `RidingIdle89`.
    pub(crate) const TO_RIDING_IDLE: u16 = 202;
    /// State `Skitching88`.
    pub(crate) const SKITCHING_88: u16 = 203;
    /// State `Skitch2Hand272`.
    pub(crate) const SKITCH_2_HAND_272: u16 = 204;
    /// State `Absorb700`.
    pub(crate) const ABSORB_700: u16 = 205;
    /// State `ShimmyingLeft1011`.
    pub(crate) const SHIMMYING_LEFT_1011: u16 = 206;
    /// State `ShimmyingRight1012`.
    pub(crate) const SHIMMYING_RIGHT_1012: u16 = 207;
    /// State `Idle1013`.
    pub(crate) const IDLE_1013: u16 = 208;
    /// State `Skitch1HandFS273`.
    pub(crate) const SKITCH_1_HAND_FS_273: u16 = 209;
    /// State `Skitch1HandBS274`.
    pub(crate) const SKITCH_1_HAND_BS_274: u16 = 210;
    /// State `RandPJLadd707`.
    pub(crate) const RAND_PJ_LADD_707: u16 = 211;
    /// State `RandDill710`.
    pub(crate) const RAND_DILL_710: u16 = 212;
    /// State `RandHsu711`.
    pub(crate) const RAND_HSU_711: u16 = 213;
    /// Shared by 2: state `Rand1712`, state `Rand2713`.
    pub(crate) const RANDOM_COND: u16 = 214;
    /// State `Moving1014`.
    pub(crate) const MOVING_1014: u16 = 215;
    /// Shared by 2: state `Into1160`, state `Into1163`.
    pub(crate) const BRAKE_INTO: u16 = 216;
    /// Shared by 2: state `Out1162`, state `Out1166`.
    pub(crate) const OUT_NOT_HAS_INTENT_BRAKE: u16 = 217;
    /// State `Stationary1015`.
    pub(crate) const STATIONARY_1015: u16 = 218;
    /// Shared by 2: `FromFakie1016` -> `Right1017`, `FromFakie1016` -> `Left1018`.
    pub(crate) const WILL_EXPIRE_IN_0_25: u16 = 219;
    /// Shared by 4: state `LeftSlide1167`, state `LeftSlide1319`, state `LeftSlide1322`, state
    /// `LeftSlide1325`.
    pub(crate) const LEFT_SLIDE: u16 = 220;
    /// Shared by 2: state `Early1310`, state `Early1315`.
    pub(crate) const SLIDE_OUT_EARLY: u16 = 221;
    /// Shared by 2: state `Mid1311`, state `Mid1316`.
    pub(crate) const SLIDE_OUT_MID: u16 = 222;
    /// Shared by 2: `Out1252` -> `Brake1271`, `Out1254` -> `Brake1271`.
    pub(crate) const OUT_TO_BRAKE: u16 = 223;
    /// State `RightSlide1168`.
    pub(crate) const RIGHT_SLIDE_1168: u16 = 224;
    /// Shared by 2: `FS1021` -> `Riding277`, `BS1022` -> `Riding277`.
    pub(crate) const TO_RIDING: u16 = 225;
    /// Shared by 3: state `Bs1023`, state `Bs1331`, state `Bs1336`.
    pub(crate) const REVERT_BS: u16 = 226;
    /// Shared by 3: state `Fs1024`, state `Fs1332`, state `Fs1337`.
    pub(crate) const REVERT_FS: u16 = 227;
    /// State `Still1171`.
    pub(crate) const STILL_1171: u16 = 228;
    /// `Still1171` -> `Random275`.
    pub(crate) const STILL_1171_TO_RANDOM_275: u16 = 229;
    /// Shared by 4: `BSGrabInto1255` -> `Turning719`, `BSGrabCyc1256` -> `BSGrabOut1257`,
    /// `FSGrabToBSGrab1263` -> `BSGrabOut1257`, `DBLGrabToBSGrab1268` -> `BSGrabOut1257`.
    pub(crate) const NOT_HAS_INTENT_GROUND_BS_GRAB: u16 = 230;
    /// Shared by 3: state `BSGrabCyc1256`, state `FSGrabToBSGrab1263`, state `DBLGrabToBSGrab1268`.
    pub(crate) const HAS_INTENT_GROUND_BS_GRAB_CYCLE: u16 = 231;
    /// Shared by 3: `LeftSlide1319` -> `Idle1318`, `LeftSlide1322` -> `Idle1321`, `LeftSlide1325`
    /// -> `Idle1324`.
    pub(crate) const LEFT_SLIDE_TO_IDLE: u16 = 232;
    /// Shared by 3: state `RightSlide1320`, state `RightSlide1323`, state `RightSlide1326`.
    pub(crate) const RIGHT_SLIDE: u16 = 233;
    /// Shared by 3: `RightSlide1320` -> `Idle1318`, `RightSlide1323` -> `Idle1321`,
    /// `RightSlide1326` -> `Idle1324`.
    pub(crate) const RIGHT_SLIDE_TO_IDLE: u16 = 234;
    /// Shared by 3: state `BSGrabToFSGrab1258`, state `FSGrabCyc1261`, state `DBLGrabToFSGrab1269`.
    pub(crate) const HAS_INTENT_GROUND_FS_GRAB_CYCLE: u16 = 235;
    /// Shared by 4: `BSGrabToFSGrab1258` -> `FSGrabOut1262`, `FSGrabInto1260` -> `Turning719`,
    /// `FSGrabCyc1261` -> `FSGrabOut1262`, `DBLGrabToFSGrab1269` -> `FSGrabOut1262`.
    pub(crate) const NOT_HAS_INTENT_GROUND_FS_GRAB: u16 = 236;
    /// State `FSGrabInto1260`.
    pub(crate) const FS_GRAB_INTO_1260: u16 = 237;
    /// `DBLGrabInto1265` -> `Turning719`.
    pub(crate) const DBL_GRAB_INTO_1265_TO_TURNING_719: u16 = 238;
    /// State `DBLGrabCyc1266`.
    pub(crate) const DBL_GRAB_CYC_1266: u16 = 239;
    /// `DBLGrabCyc1266` -> `DBLGrabOut1267`.
    pub(crate) const DBL_GRAB_CYC_1266_TO_DBL_GRAB_OUT_1267: u16 = 240;
    /// `Manual1027` -> `Bumped276`.
    pub(crate) const MANUAL_1027_TO_BUMPED_276: u16 = 241;
    /// State `TailManual1176`.
    pub(crate) const TAIL_MANUAL_1176: u16 = 242;
    /// Shared by 5: state `Cycle1270`, state `Into1333`, state `Cycle1334`, `Brake1271` ->
    /// `Cycle1270` and 1 more.
    pub(crate) const NOT_HAS_INTENT_MANUAL_BRAKE: u16 = 243;
    /// State `RollingBackwards1327`.
    pub(crate) const ROLLING_BACKWARDS_1327: u16 = 244;
    /// State `RollingForwardsOrStationary1328`.
    pub(crate) const ROLLING_FORWARDS_OR_STATIONARY_1328: u16 = 245;
    /// Shared by 2: state `Brake1271`, state `Brake1335`.
    pub(crate) const BRAKE: u16 = 246;
    /// Shared by 2: state `Moving1329`, state `Moving1352`.
    pub(crate) const MOVING: u16 = 247;
    /// Shared by 2: state `Stationary1330`, state `Stationary1353`.
    pub(crate) const STATIONARY: u16 = 248;
    /// State `Holding1273`.
    pub(crate) const HOLDING_1273: u16 = 249;
    /// Shared by 2: state `Out1274`, `Into1333` -> `Idle1025`.
    pub(crate) const NOT_HAS_INTENT_MANUAL_GREATER_0: u16 = 250;
    /// State `RollingForwards1350`.
    pub(crate) const ROLLING_FORWARDS_1350: u16 = 251;
    /// State `RollingBackwardsOrStationary1351`.
    pub(crate) const ROLLING_BACKWARDS_OR_STATIONARY_1351: u16 = 252;
    /// State `Push1028`.
    pub(crate) const PUSH_1028: u16 = 253;
    /// Shared by 2: state `Right1276`, state `Right1294`.
    pub(crate) const HAS_INTENT_RIGHT_PUSH: u16 = 254;
    /// State `Left1277`.
    pub(crate) const LEFT_1277: u16 = 255;
    /// Shared by 6: `Right1278` -> `Right1280`, `Left1279` -> `Left1281`, `Right1280` ->
    /// `Right1282`, `Left1281` -> `Left1283` and 2 more.
    pub(crate) const WILL_EXPIRE_IN_0_01_NOT_WAIT: u16 = 256;
    /// Shared by 2: state `PushQueued1188`, `PushEnd1182` -> `Push1028`.
    pub(crate) const HAS_INTENT_NEW_PUSH: u16 = 257;
    /// `AnticOut1029` -> `Idle1025`.
    pub(crate) const ANTIC_OUT_1029_TO_IDLE_1025: u16 = 258;
    /// Shared by 2: `AnticTail1286` -> `AnticCyc1186`, `AnticNose1287` -> `AnticCyc1186`.
    pub(crate) const TO_ANTIC_CYC: u16 = 259;
    /// State `OllieAnticCycle1288`.
    pub(crate) const OLLIE_ANTIC_CYCLE_1288: u16 = 260;
    /// State `NollieAnticCycle1291`.
    pub(crate) const NOLLIE_ANTIC_CYCLE_1291: u16 = 261;
    /// `Right1294` -> `FromFakie1339`.
    pub(crate) const RIGHT_1294_TO_FROM_FAKIE_1339: u16 = 262;
    /// `Left1295` -> `FromFakie1341`.
    pub(crate) const LEFT_1295_TO_FROM_FAKIE_1341: u16 = 263;
    /// `Anticipation91` -> `AnticCycle92`.
    pub(crate) const ANTICIPATION_91_TO_ANTIC_CYCLE_92: u16 = 264;
    /// Shared by 2: `Into93` -> `LandingOnBoard28`, `Cyc94` -> `LandingOnBoard28`.
    pub(crate) const TO_LANDING_ON_BOARD: u16 = 265;
    /// `Cyc94` -> `WipeOut4`.
    pub(crate) const CYC_94_TO_WIPE_OUT_4: u16 = 266;
    /// Shared by 2: state `ToLandingOnBoardFromLeft26`, state `ToLandingOnBoardFromRight27`.
    pub(crate) const CAN_LAND_ON_BOARD: u16 = 267;
    /// State `WipeOut4`.
    pub(crate) const WIPE_OUT_4: u16 = 268;
    /// State `BraceForImpact98`.
    pub(crate) const BRACE_FOR_IMPACT_98: u16 = 269;
    /// State `RestAndRecover30`.
    pub(crate) const REST_AND_RECOVER_30: u16 = 270;
    /// State `Teleport5`.
    pub(crate) const TELEPORT_5: u16 = 271;
    /// State `DebugAnimations7`.
    pub(crate) const DEBUG_ANIMATIONS_7: u16 = 272;
    /// `Motion0` -> `Teleport5`.
    pub(crate) const MOTION_0_TO_TELEPORT_5: u16 = 273;
    /// State `IdleDismount101`.
    pub(crate) const IDLE_DISMOUNT_101: u16 = 274;
    /// State `RunDismount102`.
    pub(crate) const RUN_DISMOUNT_102: u16 = 275;
    /// `RunDismount102` -> `RunAround107`.
    pub(crate) const RUN_DISMOUNT_102_TO_RUN_AROUND_107: u16 = 276;
    /// `RunDismount102` -> `RunDismountWaitForLanding103`.
    pub(crate) const RUN_DISMOUNT_102_TO_RUN_DISMOUNT_WAIT_FOR_LANDING_103: u16 = 277;
    /// `MovingObject104` -> `Mount34`.
    pub(crate) const MOVING_OBJECT_104_TO_MOUNT_34: u16 = 278;
    /// State `Still752`.
    pub(crate) const STILL_752: u16 = 279;
    /// State `ToStepForward317`.
    pub(crate) const TO_STEP_FORWARD_317: u16 = 280;
    /// Shared by 2: `ToStepForward317` -> `RunAround107`, `ToStepBackward318` -> `RunAround107`.
    pub(crate) const STEP_TO_RUN_AROUND: u16 = 281;
    /// State `ToStepBackward318`.
    pub(crate) const TO_STEP_BACKWARD_318: u16 = 282;
    /// State `Dismount36`.
    pub(crate) const DISMOUNT_36: u16 = 283;
    /// State `Finish117`.
    pub(crate) const FINISH_117: u16 = 284;
    /// Shared by 32: state `Into320`, state `GrabCycle322`, state `Into328`, state `GrabCycle330`
    /// and 28 more.
    pub(crate) const HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB: u16 = 285;
    /// Shared by 16: state `Out323`, state `Out331`, state `Out339`, state `Out347` and 12 more.
    pub(crate) const NOT_HAS_INTENT_GRIND_FS_OR_DOUBLE_GRAB: u16 = 286;
    /// Shared by 32: state `Into324`, state `GrabCycle326`, state `Into332`, state `GrabCycle334`
    /// and 28 more.
    pub(crate) const HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB: u16 = 287;
    /// Shared by 16: state `Out327`, state `Out335`, state `Out343`, state `Out351` and 12 more.
    pub(crate) const NOT_HAS_INTENT_GRIND_BS_OR_DOUBLE_GRAB: u16 = 288;
    /// Shared by 4: state `GrabCycleDirect457`, state `GrabCycleDirect469`, state
    /// `GrabCycleDirect481`, state `GrabCycleDirect493`.
    pub(crate) const GRAB_CYCLE_DIRECT_FS_GRAB_TAIL: u16 = 289;
    /// Shared by 4: state `GrabCycleDirect461`, state `GrabCycleDirect473`, state
    /// `GrabCycleDirect485`, state `GrabCycleDirect497`.
    pub(crate) const GRAB_CYCLE_DIRECT_BS_GRAB_NOSE: u16 = 290;
    /// Shared by 4: state `Out467`, state `Out479`, state `Out491`, state `Out503`.
    pub(crate) const GRIND_DOUBLE_GRAB_OUT: u16 = 291;
    /// Shared by 8: `Trick19` -> `TrickDismount15`, `LeftGround786` -> `KickoutDismount13`,
    /// `LeftGround516` -> `KickoutDismount13`, `LeftGround865` -> `KickoutDismount13` and 4 more.
    pub(crate) const HAS_INTENT_OB_DISMOUNT_IN_AIR: u16 = 292;
    /// `Trick19` -> `HippyRunout16`.
    pub(crate) const TRICK_19_TO_HIPPY_RUNOUT_16: u16 = 293;
    /// Shared by 32: state `FromManual787`, state `FromManual806`, state `FromManual811`, state
    /// `FromManual816` and 28 more.
    pub(crate) const FROM_MANUAL: u16 = 294;
    /// Shared by 2: `TrickCyc2506` -> `TrickOut508`, `TrickCyc2511` -> `TrickOut513`.
    pub(crate) const TRICK_CYC_2_TO_TRICK_OUT: u16 = 295;
    /// State `Out31081`.
    pub(crate) const OUT_3_1081: u16 = 296;
    /// State `Out31089`.
    pub(crate) const OUT_3_1089: u16 = 297;
    /// State `Out31097`.
    pub(crate) const OUT_3_1097: u16 = 298;
    /// State `Out31105`.
    pub(crate) const OUT_3_1105: u16 = 299;
    /// Shared by 10: `PlantingFoot604` -> `WipeOut4`, `Lift606` -> `WipeOut4`, `PlantingFoot608` ->
    /// `WipeOut4`, `Lift610` -> `WipeOut4` and 6 more.
    pub(crate) const NEITHER_GROUND_NOR_AIR: u16 = 300;
    /// Shared by 2: state `Into624`, state `Into633`.
    pub(crate) const HAND_PLANT_ONE_FOOT_RIGHT_INTO: u16 = 301;
    /// Shared by 2: state `Into627`, state `Into636`.
    pub(crate) const HAND_PLANT_ONE_FOOT_LEFT_INTO: u16 = 302;
    /// Shared by 2: state `Out632`, state `Out641`.
    pub(crate) const HAND_PLANT_DISMOUNT_OUT: u16 = 303;
    /// Shared by 5: `Dismount649` -> `WipeOut4`, `Dismount660` -> `WipeOut4`, `OneFootAir1145` ->
    /// `WipeOut4`, `RocketAir688` -> `WipeOut4` and 1 more.
    pub(crate) const TRUCKS_OR_DECK_IN_CONTACT_OR_NOT_AIR: u16 = 304;
    /// Shared by 4: state `Into1108`, state `Into1126`, state `Into1191`, state `Into1211`.
    pub(crate) const INTO_FS_GRAB_RIGHT_ONE_FOOT_AIR: u16 = 305;
    /// Shared by 4: state `Into1112`, state `Into1130`, state `Into1196`, state `Into1216`.
    pub(crate) const INTO_FS_GRAB_LEFT_ONE_FOOT_AIR: u16 = 306;
    /// Shared by 4: state `Into1116`, state `Into1134`, state `Into1201`, state `Into1221`.
    pub(crate) const INTO_BS_GRAB_RIGHT_ONE_FOOT_AIR: u16 = 307;
    /// Shared by 4: state `Into1120`, state `Into1138`, state `Into1206`, state `Into1226`.
    pub(crate) const INTO_BS_GRAB_LEFT_ONE_FOOT_AIR: u16 = 308;
    /// Shared by 2: `GrabOut967` -> `Cyc648`, `GrabOut969` -> `Cyc648`.
    pub(crate) const FS_GRAB_OUT_TO_CYC: u16 = 309;
    /// Shared by 2: `GrabOut967` -> `InAir21`, `GrabOut969` -> `InAir21`.
    pub(crate) const FS_GRAB_OUT_TO_IN_AIR: u16 = 310;
    /// State `FSGrabFingerFlipShuv652`.
    pub(crate) const FS_GRAB_FINGER_FLIP_SHUV_652: u16 = 311;
    /// Shared by 2: state `ToDBLGrab655`, state `ToDBLGrab666`.
    pub(crate) const TO_DBL_GRAB: u16 = 312;
    /// Shared by 2: `GrabOut979` -> `Cyc659`, `GrabOut981` -> `Cyc659`.
    pub(crate) const BS_GRAB_OUT_TO_CYC: u16 = 313;
    /// Shared by 2: `GrabOut979` -> `InAir21`, `GrabOut981` -> `InAir21`.
    pub(crate) const BS_GRAB_OUT_TO_IN_AIR: u16 = 314;
    /// State `BSGrabFingerFlipShuv663`.
    pub(crate) const BS_GRAB_FINGER_FLIP_SHUV_663: u16 = 315;
    /// `GrabOut983` -> `Cyc670`.
    pub(crate) const GRAB_OUT_983_TO_CYC_670: u16 = 316;
    /// `GrabOut983` -> `InAir21`.
    pub(crate) const GRAB_OUT_983_TO_IN_AIR_21: u16 = 317;
    /// Shared by 2: state `MuteFingerFlipShuv672`, state `MuteFingerFlipShuv679`.
    pub(crate) const MUTE_FINGER_FLIP_SHUV: u16 = 318;
    /// Shared by 2: state `StaleFingerFlipShuv673`, state `StaleFingerFlipShuv680`.
    pub(crate) const STALE_FINGER_FLIP_SHUV: u16 = 319;
    /// `GrabOut985` -> `Cyc677`.
    pub(crate) const GRAB_OUT_985_TO_CYC_677: u16 = 320;
    /// `GrabOut985` -> `InAir21`.
    pub(crate) const GRAB_OUT_985_TO_IN_AIR_21: u16 = 321;
    /// State `Into1142`.
    pub(crate) const INTO_1142: u16 = 322;
    /// State `Into1229`.
    pub(crate) const INTO_1229: u16 = 323;
    /// State `Into1296`.
    pub(crate) const INTO_1296: u16 = 324;
    /// State `Into1299`.
    pub(crate) const INTO_1299: u16 = 325;
    /// `GrabOut1235` -> `Cycle1143`.
    pub(crate) const GRAB_OUT_1235_TO_CYCLE_1143: u16 = 326;
    /// `GrabOut1235` -> `InAir21`.
    pub(crate) const GRAB_OUT_1235_TO_IN_AIR_21: u16 = 327;
    /// State `SeatbeltFingerFlipShuv1239`.
    pub(crate) const SEATBELT_FINGER_FLIP_SHUV_1239: u16 = 328;
    /// State `Into1240`.
    pub(crate) const INTO_1240: u16 = 329;
    /// State `Into1302`.
    pub(crate) const INTO_1302: u16 = 330;
    /// State `Into1305`.
    pub(crate) const INTO_1305: u16 = 331;
    /// `GrabOut1246` -> `Cycle1149`.
    pub(crate) const GRAB_OUT_1246_TO_CYCLE_1149: u16 = 332;
    /// `GrabOut1246` -> `InAir21`.
    pub(crate) const GRAB_OUT_1246_TO_IN_AIR_21: u16 = 333;
    /// State `CrailFingerFlipShuv1250`.
    pub(crate) const CRAIL_FINGER_FLIP_SHUV_1250: u16 = 334;
    /// State `Lateflip80`.
    pub(crate) const LATEFLIP_80: u16 = 335;
    /// Shared by 2: state `Into997`, state `Into1003`.
    pub(crate) const INTO_RIGHT_ONE_FOOT_AIR: u16 = 336;
    /// Shared by 2: state `Into1000`, state `Into1006`.
    pub(crate) const INTO_LEFT_ONE_FOOT_AIR: u16 = 337;
    /// `InAir21` -> `WipeOut4`.
    pub(crate) const IN_AIR_21_TO_WIPE_OUT_4: u16 = 338;
    /// State `Land22`.
    pub(crate) const LAND_22: u16 = 339;
    /// Shared by 10: state `KickTurn278`, state `KickTurn280`, state `SkitchKickTurn696`, state
    /// `SkitchKickTurn698` and 6 more.
    pub(crate) const HAS_INTENT_ANY_KICK_TURN: u16 = 340;
    /// Shared by 4: `SkitchKickTurn696` -> `SkitchCarve697`, `SkitchKickTurn698` ->
    /// `SkitchCarve699`, `SkitchKickTurn1154` -> `SkitchCarve1155`, `SkitchKickTurn1156` ->
    /// `SkitchCarve1157`.
    pub(crate) const SKITCH_KICK_TURN_TO_SKITCH_CARVE: u16 = 341;
    /// `Absorb700` -> `Holding701`.
    pub(crate) const ABSORB_700_TO_HOLDING_701: u16 = 342;
    /// State `RandCarroll1708`.
    pub(crate) const RAND_CARROLL_1_708: u16 = 343;
    /// State `Bumped276`.
    pub(crate) const BUMPED_276: u16 = 344;
    /// `Bumped276` -> `Switch1031`.
    pub(crate) const BUMPED_276_TO_SWITCH_1031: u16 = 345;
    /// State `FromFakie1016`.
    pub(crate) const FROM_FAKIE_1016: u16 = 346;
    /// State `Right1017`.
    pub(crate) const RIGHT_1017: u16 = 347;
    /// State `Left1018`.
    pub(crate) const LEFT_1018: u16 = 348;
    /// `Into1251` -> `Out1252`.
    pub(crate) const INTO_1251_TO_OUT_1252: u16 = 349;
    /// `Into1253` -> `Out1254`.
    pub(crate) const INTO_1253_TO_OUT_1254: u16 = 350;
    /// State `FS1021`.
    pub(crate) const FS_1021: u16 = 351;
    /// State `BS1022`.
    pub(crate) const BS_1022: u16 = 352;
    /// Shared by 2: state `BSGrabToDBLGrab1259`, state `FSGrabToDBLGrab1264`.
    pub(crate) const HAS_INTENT_GROUND_DBL_GRAB_OR_COFFIN: u16 = 353;
    /// Shared by 2: `BSGrabToDBLGrab1259` -> `DBLGrabOut1267`, `FSGrabToDBLGrab1264` ->
    /// `DBLGrabOut1267`.
    pub(crate) const TO_DBL_GRAB_OUT: u16 = 354;
    /// State `DBLGrabInto1265`.
    pub(crate) const DBL_GRAB_INTO_1265: u16 = 355;
    /// `Push1028` -> `Anticipation1030`.
    pub(crate) const PUSH_1028_TO_ANTICIPATION_1030: u16 = 356;
    /// `Right1278` -> `FootPlant621`.
    pub(crate) const RIGHT_1278_TO_FOOT_PLANT_621: u16 = 357;
    /// `Left1279` -> `FootPlant621`.
    pub(crate) const LEFT_1279_TO_FOOT_PLANT_621: u16 = 358;
    /// State `360PopShuvitAnticCycle1289`.
    pub(crate) const STATE_360_POP_SHUVIT_ANTIC_CYCLE_1289: u16 = 359;
    /// State `FS360PopShuvitAnticCycle1290`.
    pub(crate) const FS_360_POP_SHUVIT_ANTIC_CYCLE_1290: u16 = 360;
    /// State `Nollie360PopShuvitAnticCycle1292`.
    pub(crate) const NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1292: u16 = 361;
    /// State `FSNollie360PopShuvitAnticCycle1293`.
    pub(crate) const FS_NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1293: u16 = 362;
    /// Shared by 2: state `Anticipation91`, state `AnticCycle92`.
    pub(crate) const HAS_INTENT_RIGHT_AND_LEFT_PUSH: u16 = 363;
    /// State `Into93`.
    pub(crate) const INTO_93: u16 = 364;
    /// `Into93` -> `WipeOut4`.
    pub(crate) const INTO_93_TO_WIPE_OUT_4: u16 = 365;
    /// State `LandingOnBoard28`.
    pub(crate) const LANDING_ON_BOARD_28: u16 = 366;
    /// `Teleport5` -> `Motion0`.
    pub(crate) const TELEPORT_5_TO_MOTION_0: u16 = 367;
    /// State `Idle105`.
    pub(crate) const IDLE_105: u16 = 368;
    /// State `TurningRight753`.
    pub(crate) const TURNING_RIGHT_753: u16 = 369;
    /// State `TurningLeft754`.
    pub(crate) const TURNING_LEFT_754: u16 = 370;
    /// State `StepForward309`.
    pub(crate) const STEP_FORWARD_309: u16 = 371;
    /// State `StepBackward313`.
    pub(crate) const STEP_BACKWARD_313: u16 = 372;
    /// `LocoNew319` -> `Mount34`.
    pub(crate) const LOCO_NEW_319_TO_MOUNT_34: u16 = 373;
    /// State `RunningIntoMountFromRight108`.
    pub(crate) const RUNNING_INTO_MOUNT_FROM_RIGHT_108: u16 = 374;
    /// State `RunningIntoMountFromLeft110`.
    pub(crate) const RUNNING_INTO_MOUNT_FROM_LEFT_110: u16 = 375;
    /// State `StallRunIntoMountRight112`.
    pub(crate) const STALL_RUN_INTO_MOUNT_RIGHT_112: u16 = 376;
    /// State `StallRunIntoMountLeft113`.
    pub(crate) const STALL_RUN_INTO_MOUNT_LEFT_113: u16 = 377;
    /// State `StallIntoMount114`.
    pub(crate) const STALL_INTO_MOUNT_114: u16 = 378;
    /// State `Mount34`.
    pub(crate) const MOUNT_34: u16 = 379;
    /// `AccidentalDismount17` -> `WipeOut4`.
    pub(crate) const ACCIDENTAL_DISMOUNT_17_TO_WIPE_OUT_4: u16 = 380;
    /// `ForceBrake2` -> `WipeOut4`.
    pub(crate) const FORCE_BRAKE_2_TO_WIPE_OUT_4: u16 = 381;
    /// Shared by 6: state `GrabCycleDirect321`, state `GrabCycleDirect329`, state
    /// `GrabCycleDirect337`, state `GrabCycleDirect345` and 2 more.
    pub(crate) const GRAB_CYCLE_DIRECT_FS: u16 = 382;
    /// Shared by 8: state `GrabCycleDirect325`, state `GrabCycleDirect333`, state
    /// `GrabCycleDirect405`, state `GrabCycleDirect413` and 4 more.
    pub(crate) const GRAB_CYCLE_DIRECT_NOSE: u16 = 383;
    /// Shared by 8: state `GrabCycleDirect341`, state `GrabCycleDirect349`, state
    /// `GrabCycleDirect357`, state `GrabCycleDirect365` and 4 more.
    pub(crate) const GRAB_CYCLE_DIRECT_BS: u16 = 384;
    /// Shared by 10: state `GrabCycleDirect353`, state `GrabCycleDirect361`, state
    /// `GrabCycleDirect369`, state `GrabCycleDirect377` and 6 more.
    pub(crate) const GRAB_CYCLE_DIRECT_TAIL: u16 = 385;
    /// Shared by 4: state `GrabCycleDirect465`, state `GrabCycleDirect477`, state
    /// `GrabCycleDirect489`, state `GrabCycleDirect501`.
    pub(crate) const GRAB_CYCLE_DIRECT_DOUBLE: u16 = 386;
    /// `Trick19` -> `AccidentalDismount17`.
    pub(crate) const TRICK_19_TO_ACCIDENTAL_DISMOUNT_17: u16 = 387;
    /// Shared by 2: `TrickCyc1505` -> `TrickOut508`, `TrickCyc1510` -> `TrickOut513`.
    pub(crate) const TRICK_CYC_1_TO_TRICK_OUT: u16 = 388;
    /// State `Out11079`.
    pub(crate) const OUT_1_1079: u16 = 389;
    /// State `Out21080`.
    pub(crate) const OUT_2_1080: u16 = 390;
    /// State `Out11087`.
    pub(crate) const OUT_1_1087: u16 = 391;
    /// State `Out21088`.
    pub(crate) const OUT_2_1088: u16 = 392;
    /// State `Out11095`.
    pub(crate) const OUT_1_1095: u16 = 393;
    /// State `Out21096`.
    pub(crate) const OUT_2_1096: u16 = 394;
    /// State `Out11103`.
    pub(crate) const OUT_1_1103: u16 = 395;
    /// State `Out21104`.
    pub(crate) const OUT_2_1104: u16 = 396;
    /// State `PlantingFoot604`.
    pub(crate) const PLANTING_FOOT_604: u16 = 397;
    /// Shared by 5: `FootPlant605` -> `WipeOut4`, `FootPlant609` -> `WipeOut4`, `FootPlant613` ->
    /// `WipeOut4`, `FootPlant617` -> `WipeOut4` and 1 more.
    pub(crate) const FOOT_PLANT_TO_WIPE_OUT: u16 = 398;
    /// State `PlantingFoot608`.
    pub(crate) const PLANTING_FOOT_608: u16 = 399;
    /// State `PlantingFoot612`.
    pub(crate) const PLANTING_FOOT_612: u16 = 400;
    /// State `PlantingFoot616`.
    pub(crate) const PLANTING_FOOT_616: u16 = 401;
    /// State `PlantingFoot620`.
    pub(crate) const PLANTING_FOOT_620: u16 = 402;
    /// Shared by 2: state `Out626`, state `Out635`.
    pub(crate) const HAND_PLANT_ONE_FOOT_RIGHT_OUT: u16 = 403;
    /// Shared by 2: state `Out629`, state `Out638`.
    pub(crate) const HAND_PLANT_ONE_FOOT_LEFT_OUT: u16 = 404;
    /// State `Into642`.
    pub(crate) const INTO_642: u16 = 405;
    /// Shared by 2: state `ToSuperman961`, state `ToSuperman973`.
    pub(crate) const TO_SUPERMAN: u16 = 406;
    /// `Idle683` -> `InAir21`.
    pub(crate) const IDLE_683_TO_IN_AIR_21: u16 = 407;
    /// `Idle684` -> `InAir21`.
    pub(crate) const IDLE_684_TO_IN_AIR_21: u16 = 408;
    /// `Idle685` -> `InAir21`.
    pub(crate) const IDLE_685_TO_IN_AIR_21: u16 = 409;
    /// `Idle687` -> `InAir21`.
    pub(crate) const IDLE_687_TO_IN_AIR_21: u16 = 410;
    /// State `Into1148`.
    pub(crate) const INTO_1148: u16 = 411;
    /// State `Random275`.
    pub(crate) const RANDOM_275: u16 = 412;
    /// State `KickTurn715`.
    pub(crate) const KICK_TURN_715: u16 = 413;
    /// `Idle1025` -> `Switch1031`.
    pub(crate) const IDLE_1025_TO_SWITCH_1031: u16 = 414;
    /// Shared by 2: `Push1028` -> `AnticTail1286`, `Push1028` -> `AnticNose1287`.
    pub(crate) const PUSH_TO_ANTIC: u16 = 415;
    /// State `Right1278`.
    pub(crate) const RIGHT_1278: u16 = 416;
    /// State `Left1279`.
    pub(crate) const LEFT_1279: u16 = 417;
    /// State `StartAnticipation90`.
    pub(crate) const START_ANTICIPATION_90: u16 = 418;
    /// `HippyJump25` -> `HippyRunout16`.
    pub(crate) const HIPPY_JUMP_25_TO_HIPPY_RUNOUT_16: u16 = 419;
    /// Shared by 2: `OffBoard1` -> `WipeOut4`, `OnBoard3` -> `WipeOut4`.
    pub(crate) const TO_WIPE_OUT: u16 = 420;
    /// State `SprintingIntoMountFromRight109`.
    pub(crate) const SPRINTING_INTO_MOUNT_FROM_RIGHT_109: u16 = 421;
    /// State `SprintingIntoMountFromLeft111`.
    pub(crate) const SPRINTING_INTO_MOUNT_FROM_LEFT_111: u16 = 422;
    /// State `LeftFootStride38`.
    pub(crate) const LEFT_FOOT_STRIDE_38: u16 = 423;
    /// Shared by 2: `LeftGround1065` -> `TrickCyc1505`, `LeftGround781` -> `TrickCyc1505`.
    pub(crate) const LEFT_GROUND_TO_TRICK_CYC_1_505: u16 = 424;
    /// Shared by 2: `LeftGround1070` -> `TrickCyc1510`, `LeftGround784` -> `TrickCyc1510`.
    pub(crate) const LEFT_GROUND_TO_TRICK_CYC_1_510: u16 = 425;
    /// `Cyc643` -> `WipeOut4`.
    pub(crate) const CYC_643_TO_WIPE_OUT_4: u16 = 426;
    /// State `Cyc648`.
    pub(crate) const CYC_648: u16 = 427;
    /// State `Cyc659`.
    pub(crate) const CYC_659: u16 = 428;
    /// State `Manual1027`.
    pub(crate) const MANUAL_1027: u16 = 429;
    /// State `Dismount32`.
    pub(crate) const DISMOUNT_32: u16 = 430;
    /// State `JumpingIntoMountLeft9`.
    pub(crate) const JUMPING_INTO_MOUNT_LEFT_9: u16 = 431;
    /// State `JumpingIntoMountRight10`.
    pub(crate) const JUMPING_INTO_MOUNT_RIGHT_10: u16 = 432;
    /// State `RightFootStride37`.
    pub(crate) const RIGHT_FOOT_STRIDE_37: u16 = 433;
    /// State `Trick19`.
    pub(crate) const TRICK_19: u16 = 434;
    /// Shared by 2: `Cyc648` -> `WipeOut4`, `Cyc659` -> `WipeOut4`.
    pub(crate) const CYC_TO_WIPE_OUT_TWEAK_0_4: u16 = 435;
    /// Shared by 2: `Cyc670` -> `WipeOut4`, `Cyc677` -> `WipeOut4`.
    pub(crate) const CYC_TO_WIPE_OUT_TWEAK_0_2: u16 = 436;
    /// State `AggressiveOrLoose1169`.
    pub(crate) const AGGRESSIVE_OR_LOOSE_1169: u16 = 437;
    /// Shared by 4: `Right1278` -> `Right1284`, `Left1279` -> `Left1285`, `Right1280` ->
    /// `Right1284`, `Left1281` -> `Left1285`.
    pub(crate) const PUSH_START_OR_CONTACT_TO_PUSH_END: u16 = 438;
    /// State `AnticTail1183`.
    pub(crate) const ANTIC_TAIL_1183: u16 = 439;
    /// State `AnticNose1184`.
    pub(crate) const ANTIC_NOSE_1184: u16 = 440;
    /// State `AnticTail1286`.
    pub(crate) const ANTIC_TAIL_1286: u16 = 441;
    /// State `AnticNose1287`.
    pub(crate) const ANTIC_NOSE_1287: u16 = 442;
    /// `PushFromFakie1178` -> `Idle1025`.
    pub(crate) const PUSH_FROM_FAKIE_1178_TO_IDLE_1025: u16 = 443;
    /// Shared by 2: `Right1282` -> `Right1284`, `Left1283` -> `Left1285`.
    pub(crate) const PUSH_CYCLE_TO_PUSH_END: u16 = 444;
    /// State `RunAround107`.
    pub(crate) const RUN_AROUND_107: u16 = 445;
    /// Shared by 32: state `GrindOutAssist514`, state `GrindOutAssist523`, state
    /// `GrindOutAssist526`, state `GrindOutAssist529` and 28 more.
    pub(crate) const GRIND_OUT_ASSIST: u16 = 446;
    /// State `StepForwardRight310`.
    pub(crate) const STEP_FORWARD_RIGHT_310: u16 = 447;
    /// State `StepRight311`.
    pub(crate) const STEP_RIGHT_311: u16 = 448;
    /// State `StepBackRight312`.
    pub(crate) const STEP_BACK_RIGHT_312: u16 = 449;
    /// State `StepBackLeft314`.
    pub(crate) const STEP_BACK_LEFT_314: u16 = 450;
    /// State `StepLeft315`.
    pub(crate) const STEP_LEFT_315: u16 = 451;
    /// State `StepForwardLeft316`.
    pub(crate) const STEP_FORWARD_LEFT_316: u16 = 452;
}

/// States by name and index, with their path below the root state.
pub(crate) mod state {
    /// `Motion`.
    pub(crate) const MOTION_0: u16 = 0;
    /// `OffBoard`.
    pub(crate) const OFF_BOARD_1: u16 = 1;
    /// `ForceBrake`.
    pub(crate) const FORCE_BRAKE_2: u16 = 2;
    /// `OnBoard`.
    pub(crate) const ON_BOARD_3: u16 = 3;
    /// `WipeOut`.
    pub(crate) const WIPE_OUT_4: u16 = 4;
    /// `Teleport`.
    pub(crate) const TELEPORT_5: u16 = 5;
    /// `Default`.
    pub(crate) const DEFAULT_6: u16 = 6;
    /// `DebugAnimations`.
    pub(crate) const DEBUG_ANIMATIONS_7: u16 = 7;
    /// `OffBoard/OBGround`.
    pub(crate) const OB_GROUND_8: u16 = 8;
    /// `OffBoard/JumpingIntoMountLeft`.
    pub(crate) const JUMPING_INTO_MOUNT_LEFT_9: u16 = 9;
    /// `OffBoard/JumpingIntoMountRight`.
    pub(crate) const JUMPING_INTO_MOUNT_RIGHT_10: u16 = 10;
    /// `OffBoard/IntoBSGrabOut`.
    pub(crate) const INTO_BS_GRAB_OUT_11: u16 = 11;
    /// `OffBoard/OBAir`.
    pub(crate) const OB_AIR_12: u16 = 12;
    /// `OffBoard/KickoutDismount`.
    pub(crate) const KICKOUT_DISMOUNT_13: u16 = 13;
    /// `OffBoard/KickoutToAir`.
    pub(crate) const KICKOUT_TO_AIR_14: u16 = 14;
    /// `OffBoard/TrickDismount`.
    pub(crate) const TRICK_DISMOUNT_15: u16 = 15;
    /// `OffBoard/HippyRunout`.
    pub(crate) const HIPPY_RUNOUT_16: u16 = 16;
    /// `OffBoard/AccidentalDismount`.
    pub(crate) const ACCIDENTAL_DISMOUNT_17: u16 = 17;
    /// `OnBoard/Grinding`.
    pub(crate) const GRINDING_18: u16 = 18;
    /// `OnBoard/Trick`.
    pub(crate) const TRICK_19: u16 = 19;
    /// `OnBoard/HandPlantAir`.
    pub(crate) const HAND_PLANT_AIR_20: u16 = 20;
    /// `OnBoard/InAir`.
    pub(crate) const IN_AIR_21: u16 = 21;
    /// `OnBoard/Land`.
    pub(crate) const LAND_22: u16 = 22;
    /// `OnBoard/Coffin`.
    pub(crate) const COFFIN_23: u16 = 23;
    /// `OnBoard/OnGround`.
    pub(crate) const ON_GROUND_24: u16 = 24;
    /// `OnBoard/HippyJump`.
    pub(crate) const HIPPY_JUMP_25: u16 = 25;
    /// `OnBoard/ToLandingOnBoardFromLeft`.
    pub(crate) const TO_LANDING_ON_BOARD_FROM_LEFT_26: u16 = 26;
    /// `OnBoard/ToLandingOnBoardFromRight`.
    pub(crate) const TO_LANDING_ON_BOARD_FROM_RIGHT_27: u16 = 27;
    /// `OnBoard/LandingOnBoard`.
    pub(crate) const LANDING_ON_BOARD_28: u16 = 28;
    /// `WipeOut/Air`.
    pub(crate) const AIR_29: u16 = 29;
    /// `WipeOut/RestAndRecover`.
    pub(crate) const REST_AND_RECOVER_30: u16 = 30;
    /// `OffBoard/OBGround/SpawnDismount`.
    pub(crate) const SPAWN_DISMOUNT_31: u16 = 31;
    /// `OffBoard/OBGround/Dismount`.
    pub(crate) const DISMOUNT_32: u16 = 32;
    /// `OffBoard/OBGround/Locomotion`.
    pub(crate) const LOCOMOTION_33: u16 = 33;
    /// `OffBoard/OBGround/Mount`.
    pub(crate) const MOUNT_34: u16 = 34;
    /// `OffBoard/OBGround/InAirMount`.
    pub(crate) const IN_AIR_MOUNT_35: u16 = 35;
    /// `OffBoard/OBAir/Dismount`.
    pub(crate) const DISMOUNT_36: u16 = 36;
    /// `OffBoard/OBAir/RightFootStride`.
    pub(crate) const RIGHT_FOOT_STRIDE_37: u16 = 37;
    /// `OffBoard/OBAir/LeftFootStride`.
    pub(crate) const LEFT_FOOT_STRIDE_38: u16 = 38;
    /// `OffBoard/AccidentalDismount/Front`.
    pub(crate) const FRONT_39: u16 = 39;
    /// `OnBoard/Grinding/BS_FEEBLE`.
    pub(crate) const BS_FEEBLE_40: u16 = 40;
    /// `OnBoard/Grinding/FS_FEEBLE`.
    pub(crate) const FS_FEEBLE_41: u16 = 41;
    /// `OnBoard/Grinding/BS_SMITH`.
    pub(crate) const BS_SMITH_42: u16 = 42;
    /// `OnBoard/Grinding/FS_SMITH`.
    pub(crate) const FS_SMITH_43: u16 = 43;
    /// `OnBoard/Grinding/FS_WILLY`.
    pub(crate) const FS_WILLY_44: u16 = 44;
    /// `OnBoard/Grinding/BS_WILLY`.
    pub(crate) const BS_WILLY_45: u16 = 45;
    /// `OnBoard/Grinding/FS_OVERWILLY`.
    pub(crate) const FS_OVERWILLY_46: u16 = 46;
    /// `OnBoard/Grinding/BS_OVERWILLY`.
    pub(crate) const BS_OVERWILLY_47: u16 = 47;
    /// `OnBoard/Grinding/BS_5_O`.
    pub(crate) const BS_5_O_48: u16 = 48;
    /// `OnBoard/Grinding/FS_5_O`.
    pub(crate) const FS_5_O_49: u16 = 49;
    /// `OnBoard/Grinding/BS_50_50`.
    pub(crate) const BS_50_50_50: u16 = 50;
    /// `OnBoard/Grinding/FS_50_50`.
    pub(crate) const FS_50_50_51: u16 = 51;
    /// `OnBoard/Grinding/BS_CROOK`.
    pub(crate) const BS_CROOK_52: u16 = 52;
    /// `OnBoard/Grinding/FS_CROOK`.
    pub(crate) const FS_CROOK_53: u16 = 53;
    /// `OnBoard/Grinding/FS_OVERCROOK`.
    pub(crate) const FS_OVERCROOK_54: u16 = 54;
    /// `OnBoard/Grinding/BS_OVERCROOK`.
    pub(crate) const BS_OVERCROOK_55: u16 = 55;
    /// `OnBoard/Grinding/BS_NOSEGRIND`.
    pub(crate) const BS_NOSEGRIND_56: u16 = 56;
    /// `OnBoard/Grinding/FS_NOSEGRIND`.
    pub(crate) const FS_NOSEGRIND_57: u16 = 57;
    /// `OnBoard/Grinding/BS_SALAD`.
    pub(crate) const BS_SALAD_58: u16 = 58;
    /// `OnBoard/Grinding/FS_SALAD`.
    pub(crate) const FS_SALAD_59: u16 = 59;
    /// `OnBoard/Grinding/BS_NOSESLIDE`.
    pub(crate) const BS_NOSESLIDE_60: u16 = 60;
    /// `OnBoard/Grinding/FS_NOSESLIDE`.
    pub(crate) const FS_NOSESLIDE_61: u16 = 61;
    /// `OnBoard/Grinding/BS_NOSEBLUNT`.
    pub(crate) const BS_NOSEBLUNT_62: u16 = 62;
    /// `OnBoard/Grinding/FS_NOSEBLUNT`.
    pub(crate) const FS_NOSEBLUNT_63: u16 = 63;
    /// `OnBoard/Grinding/BS_BLUNT`.
    pub(crate) const BS_BLUNT_64: u16 = 64;
    /// `OnBoard/Grinding/FS_BLUNT`.
    pub(crate) const FS_BLUNT_65: u16 = 65;
    /// `OnBoard/Grinding/FS_TAILSLIDE`.
    pub(crate) const FS_TAILSLIDE_66: u16 = 66;
    /// `OnBoard/Grinding/BS_TAILSLIDE`.
    pub(crate) const BS_TAILSLIDE_67: u16 = 67;
    /// `OnBoard/Grinding/BS_BOARD`.
    pub(crate) const BS_BOARD_68: u16 = 68;
    /// `OnBoard/Grinding/FS_BOARD`.
    pub(crate) const FS_BOARD_69: u16 = 69;
    /// `OnBoard/Grinding/BS_LIP`.
    pub(crate) const BS_LIP_70: u16 = 70;
    /// `OnBoard/Grinding/FS_LIP`.
    pub(crate) const FS_LIP_71: u16 = 71;
    /// `OnBoard/Trick/TailTrick`.
    pub(crate) const TAIL_TRICK_72: u16 = 72;
    /// `OnBoard/Trick/NoseTrick`.
    pub(crate) const NOSE_TRICK_73: u16 = 73;
    /// `OnBoard/Trick/PushTrick`.
    pub(crate) const PUSH_TRICK_74: u16 = 74;
    /// `OnBoard/HandPlantAir/FS`.
    pub(crate) const FS_75: u16 = 75;
    /// `OnBoard/HandPlantAir/BS`.
    pub(crate) const BS_76: u16 = 76;
    /// `OnBoard/InAir/GrabsTweaks`.
    pub(crate) const GRABS_TWEAKS_77: u16 = 77;
    /// `OnBoard/InAir/BoardAdjusts`.
    pub(crate) const BOARD_ADJUSTS_78: u16 = 78;
    /// `OnBoard/InAir/InAirStatic`.
    pub(crate) const IN_AIR_STATIC_79: u16 = 79;
    /// `OnBoard/InAir/Lateflip`.
    pub(crate) const LATEFLIP_80: u16 = 80;
    /// `OnBoard/InAir/OneFootOllie`.
    pub(crate) const ONE_FOOT_OLLIE_81: u16 = 81;
    /// `OnBoard/Land/Normal`.
    pub(crate) const NORMAL_82: u16 = 82;
    /// `OnBoard/Land/GrabLanding`.
    pub(crate) const GRAB_LANDING_83: u16 = 83;
    /// `OnBoard/Coffin/CoffinInto`.
    pub(crate) const COFFIN_INTO_84: u16 = 84;
    /// `OnBoard/Coffin/CoffinCyc`.
    pub(crate) const COFFIN_CYC_85: u16 = 85;
    /// `OnBoard/Coffin/CoffinOut`.
    pub(crate) const COFFIN_OUT_86: u16 = 86;
    /// `OnBoard/OnGround/EnterSkitching`.
    pub(crate) const ENTER_SKITCHING_87: u16 = 87;
    /// `OnBoard/OnGround/Skitching`.
    pub(crate) const SKITCHING_88: u16 = 88;
    /// `OnBoard/OnGround/RidingIdle`.
    pub(crate) const RIDING_IDLE_89: u16 = 89;
    /// `OnBoard/HippyJump/StartAnticipation`.
    pub(crate) const START_ANTICIPATION_90: u16 = 90;
    /// `OnBoard/HippyJump/Anticipation`.
    pub(crate) const ANTICIPATION_91: u16 = 91;
    /// `OnBoard/HippyJump/AnticCycle`.
    pub(crate) const ANTIC_CYCLE_92: u16 = 92;
    /// `OnBoard/HippyJump/Into`.
    pub(crate) const INTO_93: u16 = 93;
    /// `OnBoard/HippyJump/Cyc`.
    pub(crate) const CYC_94: u16 = 94;
    /// `OnBoard/ToLandingOnBoardFromLeft/Default`.
    pub(crate) const DEFAULT_95: u16 = 95;
    /// `OnBoard/ToLandingOnBoardFromRight/Default`.
    pub(crate) const DEFAULT_96: u16 = 96;
    /// `OnBoard/LandingOnBoard/Default`.
    pub(crate) const DEFAULT_97: u16 = 97;
    /// `WipeOut/Air/BraceForImpact`.
    pub(crate) const BRACE_FOR_IMPACT_98: u16 = 98;
    /// `WipeOut/Air/Flailing`.
    pub(crate) const FLAILING_99: u16 = 99;
    /// `OffBoard/OBGround/Dismount/DismountFromFakie`.
    pub(crate) const DISMOUNT_FROM_FAKIE_100: u16 = 100;
    /// `OffBoard/OBGround/Dismount/IdleDismount`.
    pub(crate) const IDLE_DISMOUNT_101: u16 = 101;
    /// `OffBoard/OBGround/Dismount/RunDismount`.
    pub(crate) const RUN_DISMOUNT_102: u16 = 102;
    /// `OffBoard/OBGround/Dismount/RunDismountWaitForLanding`.
    pub(crate) const RUN_DISMOUNT_WAIT_FOR_LANDING_103: u16 = 103;
    /// `OffBoard/OBGround/Locomotion/MovingObject`.
    pub(crate) const MOVING_OBJECT_104: u16 = 104;
    /// `OffBoard/OBGround/Locomotion/Idle`.
    pub(crate) const IDLE_105: u16 = 105;
    /// `OffBoard/OBGround/Locomotion/StandingJump`.
    pub(crate) const STANDING_JUMP_106: u16 = 106;
    /// `OffBoard/OBGround/Locomotion/RunAround`.
    pub(crate) const RUN_AROUND_107: u16 = 107;
    /// `OffBoard/OBGround/Locomotion/RunningIntoMountFromRight`.
    pub(crate) const RUNNING_INTO_MOUNT_FROM_RIGHT_108: u16 = 108;
    /// `OffBoard/OBGround/Locomotion/SprintingIntoMountFromRight`.
    pub(crate) const SPRINTING_INTO_MOUNT_FROM_RIGHT_109: u16 = 109;
    /// `OffBoard/OBGround/Locomotion/RunningIntoMountFromLeft`.
    pub(crate) const RUNNING_INTO_MOUNT_FROM_LEFT_110: u16 = 110;
    /// `OffBoard/OBGround/Locomotion/SprintingIntoMountFromLeft`.
    pub(crate) const SPRINTING_INTO_MOUNT_FROM_LEFT_111: u16 = 111;
    /// `OffBoard/OBGround/Locomotion/StallRunIntoMountRight`.
    pub(crate) const STALL_RUN_INTO_MOUNT_RIGHT_112: u16 = 112;
    /// `OffBoard/OBGround/Locomotion/StallRunIntoMountLeft`.
    pub(crate) const STALL_RUN_INTO_MOUNT_LEFT_113: u16 = 113;
    /// `OffBoard/OBGround/Locomotion/StallIntoMount`.
    pub(crate) const STALL_INTO_MOUNT_114: u16 = 114;
    /// `OffBoard/AccidentalDismount/Front/Start`.
    pub(crate) const START_115: u16 = 115;
    /// `OffBoard/AccidentalDismount/Front/Fly`.
    pub(crate) const FLY_116: u16 = 116;
    /// `OffBoard/AccidentalDismount/Front/Finish`.
    pub(crate) const FINISH_117: u16 = 117;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_FSGRAB`.
    pub(crate) const BS_5_O_FSGRAB_118: u16 = 118;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_NOSEGRAB`.
    pub(crate) const BS_5_O_NOSEGRAB_119: u16 = 119;
    /// `OnBoard/Grinding/BS_5_O/Cycle`.
    pub(crate) const CYCLE_120: u16 = 120;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_FSGRAB`.
    pub(crate) const FS_5_O_FSGRAB_121: u16 = 121;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_NOSEGRAB`.
    pub(crate) const FS_5_O_NOSEGRAB_122: u16 = 122;
    /// `OnBoard/Grinding/FS_5_O/Cycle`.
    pub(crate) const CYCLE_123: u16 = 123;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_FSGRAB`.
    pub(crate) const BS_50_50_FSGRAB_124: u16 = 124;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_BSGRAB`.
    pub(crate) const BS_50_50_BSGRAB_125: u16 = 125;
    /// `OnBoard/Grinding/BS_50_50/Cycle`.
    pub(crate) const CYCLE_126: u16 = 126;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_FSGRAB`.
    pub(crate) const FS_50_50_FSGRAB_127: u16 = 127;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_BSGRAB`.
    pub(crate) const FS_50_50_BSGRAB_128: u16 = 128;
    /// `OnBoard/Grinding/FS_50_50/Cycle`.
    pub(crate) const CYCLE_129: u16 = 129;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_TAILGRAB`.
    pub(crate) const BS_CROOK_TAILGRAB_130: u16 = 130;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_BSGRAB`.
    pub(crate) const BS_CROOK_BSGRAB_131: u16 = 131;
    /// `OnBoard/Grinding/BS_CROOK/Cycle`.
    pub(crate) const CYCLE_132: u16 = 132;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_TAILGRAB`.
    pub(crate) const FS_CROOK_TAILGRAB_133: u16 = 133;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_BSGRAB`.
    pub(crate) const FS_CROOK_BSGRAB_134: u16 = 134;
    /// `OnBoard/Grinding/FS_CROOK/Cycle`.
    pub(crate) const CYCLE_135: u16 = 135;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_TAILGRAB`.
    pub(crate) const FS_OVERCROOK_TAILGRAB_136: u16 = 136;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_BSGRAB`.
    pub(crate) const FS_OVERCROOK_BSGRAB_137: u16 = 137;
    /// `OnBoard/Grinding/FS_OVERCROOK/Cycle`.
    pub(crate) const CYCLE_138: u16 = 138;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_TAILGRAB`.
    pub(crate) const BS_OVERCROOK_TAILGRAB_139: u16 = 139;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_BSGRAB`.
    pub(crate) const BS_OVERCROOK_BSGRAB_140: u16 = 140;
    /// `OnBoard/Grinding/BS_OVERCROOK/Cycle`.
    pub(crate) const CYCLE_141: u16 = 141;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_TAILGRAB`.
    pub(crate) const BS_NOSEGRIND_TAILGRAB_142: u16 = 142;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_BSGRAB`.
    pub(crate) const BS_NOSEGRIND_BSGRAB_143: u16 = 143;
    /// `OnBoard/Grinding/BS_NOSEGRIND/Cycle`.
    pub(crate) const CYCLE_144: u16 = 144;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_TAILGRAB`.
    pub(crate) const FS_NOSEGRIND_TAILGRAB_145: u16 = 145;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_BSGRAB`.
    pub(crate) const FS_NOSEGRIND_BSGRAB_146: u16 = 146;
    /// `OnBoard/Grinding/FS_NOSEGRIND/Cycle`.
    pub(crate) const CYCLE_147: u16 = 147;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_FSGRAB`.
    pub(crate) const BS_SALAD_FSGRAB_148: u16 = 148;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_NOSEGRAB`.
    pub(crate) const BS_SALAD_NOSEGRAB_149: u16 = 149;
    /// `OnBoard/Grinding/BS_SALAD/Cycle`.
    pub(crate) const CYCLE_150: u16 = 150;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_FSGRAB`.
    pub(crate) const FS_SALAD_FSGRAB_151: u16 = 151;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_NOSEGRAB`.
    pub(crate) const FS_SALAD_NOSEGRAB_152: u16 = 152;
    /// `OnBoard/Grinding/FS_SALAD/Cycle`.
    pub(crate) const CYCLE_153: u16 = 153;
    /// `OnBoard/Grinding/BS_NOSESLIDE/BS_NOSESLIDE_TAILGRAB`.
    pub(crate) const BS_NOSESLIDE_TAILGRAB_154: u16 = 154;
    /// `OnBoard/Grinding/BS_NOSESLIDE/Cycle`.
    pub(crate) const CYCLE_155: u16 = 155;
    /// `OnBoard/Grinding/FS_NOSESLIDE/FS_NOSESLIDE_TAILGRAB`.
    pub(crate) const FS_NOSESLIDE_TAILGRAB_156: u16 = 156;
    /// `OnBoard/Grinding/FS_NOSESLIDE/Cycle`.
    pub(crate) const CYCLE_157: u16 = 157;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB`.
    pub(crate) const BS_NOSEBLUNT_TAILGRAB_158: u16 = 158;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/Cycle`.
    pub(crate) const CYCLE_159: u16 = 159;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB`.
    pub(crate) const FS_NOSEBLUNT_TAILGRAB_160: u16 = 160;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/Cycle`.
    pub(crate) const CYCLE_161: u16 = 161;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB`.
    pub(crate) const BS_BLUNT_TAILGRAB_162: u16 = 162;
    /// `OnBoard/Grinding/BS_BLUNT/Cycle`.
    pub(crate) const CYCLE_163: u16 = 163;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB`.
    pub(crate) const FS_BLUNT_TAILGRAB_164: u16 = 164;
    /// `OnBoard/Grinding/FS_BLUNT/Cycle`.
    pub(crate) const CYCLE_165: u16 = 165;
    /// `OnBoard/Grinding/FS_TAILSLIDE/FS_TAILSLIDE_NOSEGRAB`.
    pub(crate) const FS_TAILSLIDE_NOSEGRAB_166: u16 = 166;
    /// `OnBoard/Grinding/FS_TAILSLIDE/Cycle`.
    pub(crate) const CYCLE_167: u16 = 167;
    /// `OnBoard/Grinding/BS_TAILSLIDE/BS_TAILSLIDE_NOSEGRAB`.
    pub(crate) const BS_TAILSLIDE_NOSEGRAB_168: u16 = 168;
    /// `OnBoard/Grinding/BS_TAILSLIDE/Cycle`.
    pub(crate) const CYCLE_169: u16 = 169;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_TAIL`.
    pub(crate) const BS_BOARD_TAIL_170: u16 = 170;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_NOSE`.
    pub(crate) const BS_BOARD_NOSE_171: u16 = 171;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_DOUBLE`.
    pub(crate) const BS_BOARD_DOUBLE_172: u16 = 172;
    /// `OnBoard/Grinding/BS_BOARD/Cycle`.
    pub(crate) const CYCLE_173: u16 = 173;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_TAIL`.
    pub(crate) const FS_BOARD_TAIL_174: u16 = 174;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_NOSE`.
    pub(crate) const FS_BOARD_NOSE_175: u16 = 175;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_DOUBLE`.
    pub(crate) const FS_BOARD_DOUBLE_176: u16 = 176;
    /// `OnBoard/Grinding/FS_BOARD/Cycle`.
    pub(crate) const CYCLE_177: u16 = 177;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_TAIL`.
    pub(crate) const BS_LIP_TAIL_178: u16 = 178;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_NOSE`.
    pub(crate) const BS_LIP_NOSE_179: u16 = 179;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_DOUBLE`.
    pub(crate) const BS_LIP_DOUBLE_180: u16 = 180;
    /// `OnBoard/Grinding/BS_LIP/Cycle`.
    pub(crate) const CYCLE_181: u16 = 181;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_TAIL`.
    pub(crate) const FS_LIP_TAIL_182: u16 = 182;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_NOSE`.
    pub(crate) const FS_LIP_NOSE_183: u16 = 183;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_DOUBLE`.
    pub(crate) const FS_LIP_DOUBLE_184: u16 = 184;
    /// `OnBoard/Grinding/FS_LIP/Cycle`.
    pub(crate) const CYCLE_185: u16 = 185;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll`.
    pub(crate) const KICKFLIP_MIKE_CARROLL_186: u16 = 186;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales`.
    pub(crate) const KICKFLIP_MARK_GONZALES_187: u16 = 187;
    /// `OnBoard/Trick/TailTrick/Ollie`.
    pub(crate) const OLLIE_188: u16 = 188;
    /// `OnBoard/Trick/TailTrick/Kickflip`.
    pub(crate) const KICKFLIP_189: u16 = 189;
    /// `OnBoard/Trick/TailTrick/Heelflip`.
    pub(crate) const HEELFLIP_190: u16 = 190;
    /// `OnBoard/Trick/TailTrick/PopShuvit`.
    pub(crate) const POP_SHUVIT_191: u16 = 191;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit`.
    pub(crate) const FS_POP_SHUVIT_192: u16 = 192;
    /// `OnBoard/Trick/TailTrick/VarialKickflip`.
    pub(crate) const VARIAL_KICKFLIP_193: u16 = 193;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip`.
    pub(crate) const VARIAL_HEELFLIP_194: u16 = 194;
    /// `OnBoard/Trick/TailTrick/Hardflip`.
    pub(crate) const HARDFLIP_195: u16 = 195;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip`.
    pub(crate) const INWARD_HEELFLIP_196: u16 = 196;
    /// `OnBoard/Trick/TailTrick/360PopShuvit`.
    pub(crate) const STATE_360_POP_SHUVIT_197: u16 = 197;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit`.
    pub(crate) const FS_360_POP_SHUVIT_198: u16 = 198;
    /// `OnBoard/Trick/TailTrick/360Flip`.
    pub(crate) const STATE_360_FLIP_199: u16 = 199;
    /// `OnBoard/Trick/TailTrick/Laserflip`.
    pub(crate) const LASERFLIP_200: u16 = 200;
    /// `OnBoard/Trick/TailTrick/360Hardflip`.
    pub(crate) const STATE_360_HARDFLIP_201: u16 = 201;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip`.
    pub(crate) const STATE_360_INWARD_HEELFLIP_202: u16 = 202;
    /// `OnBoard/Trick/NoseTrick/Nollie`.
    pub(crate) const NOLLIE_203: u16 = 203;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip`.
    pub(crate) const N_KICKFLIP_204: u16 = 204;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip`.
    pub(crate) const N_HEELFLIP_205: u16 = 205;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit`.
    pub(crate) const N_POP_SHUVIT_206: u16 = 206;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit`.
    pub(crate) const N_FS_POP_SHUVIT_207: u16 = 207;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip`.
    pub(crate) const N_VARIAL_KICKFLIP_208: u16 = 208;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip`.
    pub(crate) const N_VARIAL_HEELFLIP_209: u16 = 209;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip`.
    pub(crate) const N_HARDFLIP_210: u16 = 210;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip`.
    pub(crate) const N_INWARD_HEELFLIP_211: u16 = 211;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit`.
    pub(crate) const N_360_POP_SHUVIT_212: u16 = 212;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit`.
    pub(crate) const N_FS_360_POP_SHUVIT_213: u16 = 213;
    /// `OnBoard/Trick/NoseTrick/N_360Flip`.
    pub(crate) const N_360_FLIP_214: u16 = 214;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip`.
    pub(crate) const N_LASERFLIP_215: u16 = 215;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip`.
    pub(crate) const N_360_HARDFLIP_216: u16 = 216;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip`.
    pub(crate) const N_360_INWARD_HEELFLIP_217: u16 = 217;
    /// `OnBoard/Trick/PushTrick/FSBoneless`.
    pub(crate) const FS_BONELESS_218: u16 = 218;
    /// `OnBoard/Trick/PushTrick/BSBoneless`.
    pub(crate) const BS_BONELESS_219: u16 = 219;
    /// `OnBoard/Trick/PushTrick/FSFastPlant`.
    pub(crate) const FS_FAST_PLANT_220: u16 = 220;
    /// `OnBoard/Trick/PushTrick/BSFastPlant`.
    pub(crate) const BS_FAST_PLANT_221: u16 = 221;
    /// `OnBoard/Trick/PushTrick/NoComply`.
    pub(crate) const NO_COMPLY_222: u16 = 222;
    /// `OnBoard/HandPlantAir/FS/FromGroundAntic`.
    pub(crate) const FROM_GROUND_ANTIC_223: u16 = 223;
    /// `OnBoard/HandPlantAir/FS/Into`.
    pub(crate) const INTO_224: u16 = 224;
    /// `OnBoard/HandPlantAir/FS/Antic`.
    pub(crate) const ANTIC_225: u16 = 225;
    /// `OnBoard/HandPlantAir/FS/PreAir`.
    pub(crate) const PRE_AIR_226: u16 = 226;
    /// `OnBoard/HandPlantAir/FS/Cyc`.
    pub(crate) const CYC_227: u16 = 227;
    /// `OnBoard/HandPlantAir/FS/Out`.
    pub(crate) const OUT_228: u16 = 228;
    /// `OnBoard/HandPlantAir/FS/AirOut`.
    pub(crate) const AIR_OUT_229: u16 = 229;
    /// `OnBoard/HandPlantAir/FS/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_230: u16 = 230;
    /// `OnBoard/HandPlantAir/FS/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_231: u16 = 231;
    /// `OnBoard/HandPlantAir/FS/Dismount`.
    pub(crate) const DISMOUNT_232: u16 = 232;
    /// `OnBoard/HandPlantAir/BS/FromGroundAntic`.
    pub(crate) const FROM_GROUND_ANTIC_233: u16 = 233;
    /// `OnBoard/HandPlantAir/BS/Into`.
    pub(crate) const INTO_234: u16 = 234;
    /// `OnBoard/HandPlantAir/BS/Antic`.
    pub(crate) const ANTIC_235: u16 = 235;
    /// `OnBoard/HandPlantAir/BS/PreAir`.
    pub(crate) const PRE_AIR_236: u16 = 236;
    /// `OnBoard/HandPlantAir/BS/Cyc`.
    pub(crate) const CYC_237: u16 = 237;
    /// `OnBoard/HandPlantAir/BS/Out`.
    pub(crate) const OUT_238: u16 = 238;
    /// `OnBoard/HandPlantAir/BS/AirOut`.
    pub(crate) const AIR_OUT_239: u16 = 239;
    /// `OnBoard/HandPlantAir/BS/OneFootRight`.
    pub(crate) const ONE_FOOT_RIGHT_240: u16 = 240;
    /// `OnBoard/HandPlantAir/BS/OneFootLeft`.
    pub(crate) const ONE_FOOT_LEFT_241: u16 = 241;
    /// `OnBoard/HandPlantAir/BS/Dismount`.
    pub(crate) const DISMOUNT_242: u16 = 242;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab`.
    pub(crate) const DBL_GRAB_243: u16 = 243;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab`.
    pub(crate) const FS_GRAB_244: u16 = 244;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab`.
    pub(crate) const BS_GRAB_245: u16 = 245;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab`.
    pub(crate) const MUTE_GRAB_246: u16 = 246;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab`.
    pub(crate) const STALE_GRAB_247: u16 = 247;
    /// `OnBoard/InAir/BoardAdjusts/Left`.
    pub(crate) const LEFT_248: u16 = 248;
    /// `OnBoard/InAir/BoardAdjusts/Right`.
    pub(crate) const RIGHT_249: u16 = 249;
    /// `OnBoard/InAir/BoardAdjusts/Up`.
    pub(crate) const UP_250: u16 = 250;
    /// `OnBoard/InAir/BoardAdjusts/Down`.
    pub(crate) const DOWN_251: u16 = 251;
    /// `OnBoard/InAir/InAirStatic/Air`.
    pub(crate) const AIR_252: u16 = 252;
    /// `OnBoard/InAir/Lateflip/LateKickflip`.
    pub(crate) const LATE_KICKFLIP_253: u16 = 253;
    /// `OnBoard/InAir/Lateflip/LateHeelflip`.
    pub(crate) const LATE_HEELFLIP_254: u16 = 254;
    /// `OnBoard/InAir/Lateflip/LateBackfootKickflip`.
    pub(crate) const LATE_BACKFOOT_KICKFLIP_255: u16 = 255;
    /// `OnBoard/InAir/Lateflip/LateBackfootHeelflip`.
    pub(crate) const LATE_BACKFOOT_HEELFLIP_256: u16 = 256;
    /// `OnBoard/InAir/Lateflip/LateBSShuv`.
    pub(crate) const LATE_BS_SHUV_257: u16 = 257;
    /// `OnBoard/InAir/Lateflip/LateFSShuv`.
    pub(crate) const LATE_FS_SHUV_258: u16 = 258;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward`.
    pub(crate) const RIGHT_FOOT_FORWARD_259: u16 = 259;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward`.
    pub(crate) const LEFT_FOOT_FORWARD_260: u16 = 260;
    /// `OnBoard/Land/Normal/Spin`.
    pub(crate) const SPIN_261: u16 = 261;
    /// `OnBoard/Land/Normal/Sketchy`.
    pub(crate) const SKETCHY_262: u16 = 262;
    /// `OnBoard/Land/Normal/Straight`.
    pub(crate) const STRAIGHT_263: u16 = 263;
    /// `OnBoard/Land/GrabLanding/BSGrab`.
    pub(crate) const BS_GRAB_264: u16 = 264;
    /// `OnBoard/Land/GrabLanding/FSGrab`.
    pub(crate) const FS_GRAB_265: u16 = 265;
    /// `OnBoard/Land/GrabLanding/DBLGrab`.
    pub(crate) const DBL_GRAB_266: u16 = 266;
    /// `OnBoard/Coffin/CoffinOut/CoffinToDBL`.
    pub(crate) const COFFIN_TO_DBL_267: u16 = 267;
    /// `OnBoard/Coffin/CoffinOut/CoffinToBS`.
    pub(crate) const COFFIN_TO_BS_268: u16 = 268;
    /// `OnBoard/Coffin/CoffinOut/CoffinToFS`.
    pub(crate) const COFFIN_TO_FS_269: u16 = 269;
    /// `OnBoard/OnGround/EnterSkitching/Into`.
    pub(crate) const INTO_270: u16 = 270;
    /// `OnBoard/OnGround/EnterSkitching/Cyc`.
    pub(crate) const CYC_271: u16 = 271;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand`.
    pub(crate) const SKITCH_2_HAND_272: u16 = 272;
    /// `OnBoard/OnGround/Skitching/Skitch1HandFS`.
    pub(crate) const SKITCH_1_HAND_FS_273: u16 = 273;
    /// `OnBoard/OnGround/Skitching/Skitch1HandBS`.
    pub(crate) const SKITCH_1_HAND_BS_274: u16 = 274;
    /// `OnBoard/OnGround/RidingIdle/Random`.
    pub(crate) const RANDOM_275: u16 = 275;
    /// `OnBoard/OnGround/RidingIdle/Bumped`.
    pub(crate) const BUMPED_276: u16 = 276;
    /// `OnBoard/OnGround/RidingIdle/Riding`.
    pub(crate) const RIDING_277: u16 = 277;
    /// `OnBoard/HippyJump/Anticipation/KickTurn`.
    pub(crate) const KICK_TURN_278: u16 = 278;
    /// `OnBoard/HippyJump/Anticipation/Carve`.
    pub(crate) const CARVE_279: u16 = 279;
    /// `OnBoard/HippyJump/AnticCycle/KickTurn`.
    pub(crate) const KICK_TURN_280: u16 = 280;
    /// `OnBoard/HippyJump/AnticCycle/Carve`.
    pub(crate) const CARVE_281: u16 = 281;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSH_LEFT1`.
    pub(crate) const MVOBJ_PUSH_LEFT_1_282: u16 = 282;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSH_LEFT2`.
    pub(crate) const MVOBJ_PUSH_LEFT_2_283: u16 = 283;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSH_RIGHT1`.
    pub(crate) const MVOBJ_PUSH_RIGHT_1_284: u16 = 284;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSH_RIGHT2`.
    pub(crate) const MVOBJ_PUSH_RIGHT_2_285: u16 = 285;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULL_LEFT1`.
    pub(crate) const MVOBJ_PULL_LEFT_1_286: u16 = 286;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULL_LEFT2`.
    pub(crate) const MVOBJ_PULL_LEFT_2_287: u16 = 287;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULL_RIGHT1`.
    pub(crate) const MVOBJ_PULL_RIGHT_1_288: u16 = 288;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULL_RIGHT2`.
    pub(crate) const MVOBJ_PULL_RIGHT_2_289: u16 = 289;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT1`.
    pub(crate) const MVOBJ_PUSHLEFT_TO_RIGHT_1_290: u16 = 290;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT2`.
    pub(crate) const MVOBJ_PUSHLEFT_TO_RIGHT_2_291: u16 = 291;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT1`.
    pub(crate) const MVOBJ_PUSHRIGHT_TO_LEFT_1_292: u16 = 292;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT2`.
    pub(crate) const MVOBJ_PUSHRIGHT_TO_LEFT_2_293: u16 = 293;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT1`.
    pub(crate) const MVOBJ_PULLLEFT_TO_RIGHT_1_294: u16 = 294;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT2`.
    pub(crate) const MVOBJ_PULLLEFT_TO_RIGHT_2_295: u16 = 295;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT1`.
    pub(crate) const MVOBJ_PULLRIGHT_TO_LEFT_1_296: u16 = 296;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT2`.
    pub(crate) const MVOBJ_PULLRIGHT_TO_LEFT_2_297: u16 = 297;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT1`.
    pub(crate) const MVOBJ_PUSH_TO_PULLLEFT_1_298: u16 = 298;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT2`.
    pub(crate) const MVOBJ_PUSH_TO_PULLLEFT_2_299: u16 = 299;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT1`.
    pub(crate) const MVOBJ_PUSH_TO_PULLRIGHT_1_300: u16 = 300;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT2`.
    pub(crate) const MVOBJ_PUSH_TO_PULLRIGHT_2_301: u16 = 301;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT1`.
    pub(crate) const MVOBJ_PULL_TO_PUSHLEFT_1_302: u16 = 302;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT2`.
    pub(crate) const MVOBJ_PULL_TO_PUSHLEFT_2_303: u16 = 303;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT1`.
    pub(crate) const MVOBJ_PULL_TO_PUSHRIGHT_1_304: u16 = 304;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT2`.
    pub(crate) const MVOBJ_PULL_TO_PUSHRIGHT_2_305: u16 = 305;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_Still`.
    pub(crate) const MVOBJ_STILL_306: u16 = 306;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_Into`.
    pub(crate) const MVOBJ_INTO_307: u16 = 307;
    /// `OffBoard/OBGround/Locomotion/Idle/Standing`.
    pub(crate) const STANDING_308: u16 = 308;
    /// `OffBoard/OBGround/Locomotion/Idle/StepForward`.
    pub(crate) const STEP_FORWARD_309: u16 = 309;
    /// `OffBoard/OBGround/Locomotion/Idle/StepForwardRight`.
    pub(crate) const STEP_FORWARD_RIGHT_310: u16 = 310;
    /// `OffBoard/OBGround/Locomotion/Idle/StepRight`.
    pub(crate) const STEP_RIGHT_311: u16 = 311;
    /// `OffBoard/OBGround/Locomotion/Idle/StepBackRight`.
    pub(crate) const STEP_BACK_RIGHT_312: u16 = 312;
    /// `OffBoard/OBGround/Locomotion/Idle/StepBackward`.
    pub(crate) const STEP_BACKWARD_313: u16 = 313;
    /// `OffBoard/OBGround/Locomotion/Idle/StepBackLeft`.
    pub(crate) const STEP_BACK_LEFT_314: u16 = 314;
    /// `OffBoard/OBGround/Locomotion/Idle/StepLeft`.
    pub(crate) const STEP_LEFT_315: u16 = 315;
    /// `OffBoard/OBGround/Locomotion/Idle/StepForwardLeft`.
    pub(crate) const STEP_FORWARD_LEFT_316: u16 = 316;
    /// `OffBoard/OBGround/Locomotion/Idle/ToStepForward`.
    pub(crate) const TO_STEP_FORWARD_317: u16 = 317;
    /// `OffBoard/OBGround/Locomotion/Idle/ToStepBackward`.
    pub(crate) const TO_STEP_BACKWARD_318: u16 = 318;
    /// `OffBoard/OBGround/Locomotion/RunAround/LocoNew`.
    pub(crate) const LOCO_NEW_319: u16 = 319;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_FSGRAB/Into`.
    pub(crate) const INTO_320: u16 = 320;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_FSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_321: u16 = 321;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_FSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_322: u16 = 322;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_FSGRAB/Out`.
    pub(crate) const OUT_323: u16 = 323;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_NOSEGRAB/Into`.
    pub(crate) const INTO_324: u16 = 324;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_NOSEGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_325: u16 = 325;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_NOSEGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_326: u16 = 326;
    /// `OnBoard/Grinding/BS_5_O/BS_5_O_NOSEGRAB/Out`.
    pub(crate) const OUT_327: u16 = 327;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_FSGRAB/Into`.
    pub(crate) const INTO_328: u16 = 328;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_FSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_329: u16 = 329;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_FSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_330: u16 = 330;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_FSGRAB/Out`.
    pub(crate) const OUT_331: u16 = 331;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_NOSEGRAB/Into`.
    pub(crate) const INTO_332: u16 = 332;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_NOSEGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_333: u16 = 333;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_NOSEGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_334: u16 = 334;
    /// `OnBoard/Grinding/FS_5_O/FS_5_O_NOSEGRAB/Out`.
    pub(crate) const OUT_335: u16 = 335;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_FSGRAB/Into`.
    pub(crate) const INTO_336: u16 = 336;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_FSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_337: u16 = 337;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_FSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_338: u16 = 338;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_FSGRAB/Out`.
    pub(crate) const OUT_339: u16 = 339;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_BSGRAB/Into`.
    pub(crate) const INTO_340: u16 = 340;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_341: u16 = 341;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_342: u16 = 342;
    /// `OnBoard/Grinding/BS_50_50/BS_50_50_BSGRAB/Out`.
    pub(crate) const OUT_343: u16 = 343;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_FSGRAB/Into`.
    pub(crate) const INTO_344: u16 = 344;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_FSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_345: u16 = 345;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_FSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_346: u16 = 346;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_FSGRAB/Out`.
    pub(crate) const OUT_347: u16 = 347;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_BSGRAB/Into`.
    pub(crate) const INTO_348: u16 = 348;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_349: u16 = 349;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_350: u16 = 350;
    /// `OnBoard/Grinding/FS_50_50/FS_50_50_BSGRAB/Out`.
    pub(crate) const OUT_351: u16 = 351;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_TAILGRAB/Into`.
    pub(crate) const INTO_352: u16 = 352;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_353: u16 = 353;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_354: u16 = 354;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_TAILGRAB/Out`.
    pub(crate) const OUT_355: u16 = 355;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_BSGRAB/Into`.
    pub(crate) const INTO_356: u16 = 356;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_357: u16 = 357;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_358: u16 = 358;
    /// `OnBoard/Grinding/BS_CROOK/BS_CROOK_BSGRAB/Out`.
    pub(crate) const OUT_359: u16 = 359;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_TAILGRAB/Into`.
    pub(crate) const INTO_360: u16 = 360;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_361: u16 = 361;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_362: u16 = 362;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_TAILGRAB/Out`.
    pub(crate) const OUT_363: u16 = 363;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_BSGRAB/Into`.
    pub(crate) const INTO_364: u16 = 364;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_365: u16 = 365;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_366: u16 = 366;
    /// `OnBoard/Grinding/FS_CROOK/FS_CROOK_BSGRAB/Out`.
    pub(crate) const OUT_367: u16 = 367;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_TAILGRAB/Into`.
    pub(crate) const INTO_368: u16 = 368;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_369: u16 = 369;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_370: u16 = 370;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_TAILGRAB/Out`.
    pub(crate) const OUT_371: u16 = 371;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_BSGRAB/Into`.
    pub(crate) const INTO_372: u16 = 372;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_373: u16 = 373;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_374: u16 = 374;
    /// `OnBoard/Grinding/FS_OVERCROOK/FS_OVERCROOK_BSGRAB/Out`.
    pub(crate) const OUT_375: u16 = 375;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_TAILGRAB/Into`.
    pub(crate) const INTO_376: u16 = 376;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_377: u16 = 377;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_378: u16 = 378;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_TAILGRAB/Out`.
    pub(crate) const OUT_379: u16 = 379;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_BSGRAB/Into`.
    pub(crate) const INTO_380: u16 = 380;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_381: u16 = 381;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_382: u16 = 382;
    /// `OnBoard/Grinding/BS_OVERCROOK/BS_OVERCROOK_BSGRAB/Out`.
    pub(crate) const OUT_383: u16 = 383;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_TAILGRAB/Into`.
    pub(crate) const INTO_384: u16 = 384;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_385: u16 = 385;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_386: u16 = 386;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_TAILGRAB/Out`.
    pub(crate) const OUT_387: u16 = 387;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_BSGRAB/Into`.
    pub(crate) const INTO_388: u16 = 388;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_389: u16 = 389;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_390: u16 = 390;
    /// `OnBoard/Grinding/BS_NOSEGRIND/BS_NOSEGRIND_BSGRAB/Out`.
    pub(crate) const OUT_391: u16 = 391;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_TAILGRAB/Into`.
    pub(crate) const INTO_392: u16 = 392;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_393: u16 = 393;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_394: u16 = 394;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_TAILGRAB/Out`.
    pub(crate) const OUT_395: u16 = 395;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_BSGRAB/Into`.
    pub(crate) const INTO_396: u16 = 396;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_BSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_397: u16 = 397;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_BSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_398: u16 = 398;
    /// `OnBoard/Grinding/FS_NOSEGRIND/FS_NOSEGRIND_BSGRAB/Out`.
    pub(crate) const OUT_399: u16 = 399;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_FSGRAB/Into`.
    pub(crate) const INTO_400: u16 = 400;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_FSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_401: u16 = 401;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_FSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_402: u16 = 402;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_FSGRAB/Out`.
    pub(crate) const OUT_403: u16 = 403;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_NOSEGRAB/Into`.
    pub(crate) const INTO_404: u16 = 404;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_NOSEGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_405: u16 = 405;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_NOSEGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_406: u16 = 406;
    /// `OnBoard/Grinding/BS_SALAD/BS_SALAD_NOSEGRAB/Out`.
    pub(crate) const OUT_407: u16 = 407;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_FSGRAB/Into`.
    pub(crate) const INTO_408: u16 = 408;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_FSGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_409: u16 = 409;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_FSGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_410: u16 = 410;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_FSGRAB/Out`.
    pub(crate) const OUT_411: u16 = 411;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_NOSEGRAB/Into`.
    pub(crate) const INTO_412: u16 = 412;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_NOSEGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_413: u16 = 413;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_NOSEGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_414: u16 = 414;
    /// `OnBoard/Grinding/FS_SALAD/FS_SALAD_NOSEGRAB/Out`.
    pub(crate) const OUT_415: u16 = 415;
    /// `OnBoard/Grinding/BS_NOSESLIDE/BS_NOSESLIDE_TAILGRAB/Into`.
    pub(crate) const INTO_416: u16 = 416;
    /// `OnBoard/Grinding/BS_NOSESLIDE/BS_NOSESLIDE_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_417: u16 = 417;
    /// `OnBoard/Grinding/BS_NOSESLIDE/BS_NOSESLIDE_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_418: u16 = 418;
    /// `OnBoard/Grinding/BS_NOSESLIDE/BS_NOSESLIDE_TAILGRAB/Out`.
    pub(crate) const OUT_419: u16 = 419;
    /// `OnBoard/Grinding/FS_NOSESLIDE/FS_NOSESLIDE_TAILGRAB/Into`.
    pub(crate) const INTO_420: u16 = 420;
    /// `OnBoard/Grinding/FS_NOSESLIDE/FS_NOSESLIDE_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_421: u16 = 421;
    /// `OnBoard/Grinding/FS_NOSESLIDE/FS_NOSESLIDE_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_422: u16 = 422;
    /// `OnBoard/Grinding/FS_NOSESLIDE/FS_NOSESLIDE_TAILGRAB/Out`.
    pub(crate) const OUT_423: u16 = 423;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/Into`.
    pub(crate) const INTO_424: u16 = 424;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_425: u16 = 425;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_426: u16 = 426;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/Out`.
    pub(crate) const OUT_427: u16 = 427;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/Cycle/Backslash`.
    pub(crate) const BACKSLASH_428: u16 = 428;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/Cycle/Rail`.
    pub(crate) const RAIL_429: u16 = 429;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/Into`.
    pub(crate) const INTO_430: u16 = 430;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_431: u16 = 431;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_432: u16 = 432;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/Out`.
    pub(crate) const OUT_433: u16 = 433;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/Cycle/Backslash`.
    pub(crate) const BACKSLASH_434: u16 = 434;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/Cycle/Rail`.
    pub(crate) const RAIL_435: u16 = 435;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/Into`.
    pub(crate) const INTO_436: u16 = 436;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_437: u16 = 437;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_438: u16 = 438;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/Out`.
    pub(crate) const OUT_439: u16 = 439;
    /// `OnBoard/Grinding/BS_BLUNT/Cycle/Backslash`.
    pub(crate) const BACKSLASH_440: u16 = 440;
    /// `OnBoard/Grinding/BS_BLUNT/Cycle/Rail`.
    pub(crate) const RAIL_441: u16 = 441;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/Into`.
    pub(crate) const INTO_442: u16 = 442;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_443: u16 = 443;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_444: u16 = 444;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/Out`.
    pub(crate) const OUT_445: u16 = 445;
    /// `OnBoard/Grinding/FS_BLUNT/Cycle/Backslash`.
    pub(crate) const BACKSLASH_446: u16 = 446;
    /// `OnBoard/Grinding/FS_BLUNT/Cycle/Rail`.
    pub(crate) const RAIL_447: u16 = 447;
    /// `OnBoard/Grinding/FS_TAILSLIDE/FS_TAILSLIDE_NOSEGRAB/Into`.
    pub(crate) const INTO_448: u16 = 448;
    /// `OnBoard/Grinding/FS_TAILSLIDE/FS_TAILSLIDE_NOSEGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_449: u16 = 449;
    /// `OnBoard/Grinding/FS_TAILSLIDE/FS_TAILSLIDE_NOSEGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_450: u16 = 450;
    /// `OnBoard/Grinding/FS_TAILSLIDE/FS_TAILSLIDE_NOSEGRAB/Out`.
    pub(crate) const OUT_451: u16 = 451;
    /// `OnBoard/Grinding/BS_TAILSLIDE/BS_TAILSLIDE_NOSEGRAB/Into`.
    pub(crate) const INTO_452: u16 = 452;
    /// `OnBoard/Grinding/BS_TAILSLIDE/BS_TAILSLIDE_NOSEGRAB/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_453: u16 = 453;
    /// `OnBoard/Grinding/BS_TAILSLIDE/BS_TAILSLIDE_NOSEGRAB/GrabCycle`.
    pub(crate) const GRAB_CYCLE_454: u16 = 454;
    /// `OnBoard/Grinding/BS_TAILSLIDE/BS_TAILSLIDE_NOSEGRAB/Out`.
    pub(crate) const OUT_455: u16 = 455;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_TAIL/Into`.
    pub(crate) const INTO_456: u16 = 456;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_TAIL/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_457: u16 = 457;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_TAIL/GrabCycle`.
    pub(crate) const GRAB_CYCLE_458: u16 = 458;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_TAIL/Out`.
    pub(crate) const OUT_459: u16 = 459;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_NOSE/Into`.
    pub(crate) const INTO_460: u16 = 460;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_NOSE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_461: u16 = 461;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_NOSE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_462: u16 = 462;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_NOSE/Out`.
    pub(crate) const OUT_463: u16 = 463;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_DOUBLE/Into`.
    pub(crate) const INTO_464: u16 = 464;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_DOUBLE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_465: u16 = 465;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_DOUBLE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_466: u16 = 466;
    /// `OnBoard/Grinding/BS_BOARD/BS_BOARD_DOUBLE/Out`.
    pub(crate) const OUT_467: u16 = 467;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_TAIL/Into`.
    pub(crate) const INTO_468: u16 = 468;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_TAIL/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_469: u16 = 469;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_TAIL/GrabCycle`.
    pub(crate) const GRAB_CYCLE_470: u16 = 470;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_TAIL/Out`.
    pub(crate) const OUT_471: u16 = 471;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_NOSE/Into`.
    pub(crate) const INTO_472: u16 = 472;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_NOSE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_473: u16 = 473;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_NOSE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_474: u16 = 474;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_NOSE/Out`.
    pub(crate) const OUT_475: u16 = 475;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_DOUBLE/Into`.
    pub(crate) const INTO_476: u16 = 476;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_DOUBLE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_477: u16 = 477;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_DOUBLE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_478: u16 = 478;
    /// `OnBoard/Grinding/FS_BOARD/FS_BOARD_DOUBLE/Out`.
    pub(crate) const OUT_479: u16 = 479;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_TAIL/Into`.
    pub(crate) const INTO_480: u16 = 480;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_TAIL/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_481: u16 = 481;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_TAIL/GrabCycle`.
    pub(crate) const GRAB_CYCLE_482: u16 = 482;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_TAIL/Out`.
    pub(crate) const OUT_483: u16 = 483;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_NOSE/Into`.
    pub(crate) const INTO_484: u16 = 484;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_NOSE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_485: u16 = 485;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_NOSE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_486: u16 = 486;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_NOSE/Out`.
    pub(crate) const OUT_487: u16 = 487;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_DOUBLE/Into`.
    pub(crate) const INTO_488: u16 = 488;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_DOUBLE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_489: u16 = 489;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_DOUBLE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_490: u16 = 490;
    /// `OnBoard/Grinding/BS_LIP/BS_LIP_DOUBLE/Out`.
    pub(crate) const OUT_491: u16 = 491;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_TAIL/Into`.
    pub(crate) const INTO_492: u16 = 492;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_TAIL/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_493: u16 = 493;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_TAIL/GrabCycle`.
    pub(crate) const GRAB_CYCLE_494: u16 = 494;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_TAIL/Out`.
    pub(crate) const OUT_495: u16 = 495;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_NOSE/Into`.
    pub(crate) const INTO_496: u16 = 496;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_NOSE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_497: u16 = 497;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_NOSE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_498: u16 = 498;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_NOSE/Out`.
    pub(crate) const OUT_499: u16 = 499;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_DOUBLE/Into`.
    pub(crate) const INTO_500: u16 = 500;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_DOUBLE/GrabCycleDirect`.
    pub(crate) const GRAB_CYCLE_DIRECT_501: u16 = 501;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_DOUBLE/GrabCycle`.
    pub(crate) const GRAB_CYCLE_502: u16 = 502;
    /// `OnBoard/Grinding/FS_LIP/FS_LIP_DOUBLE/Out`.
    pub(crate) const OUT_503: u16 = 503;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto`.
    pub(crate) const TRICK_INTO_504: u16 = 504;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickCyc1`.
    pub(crate) const TRICK_CYC_1_505: u16 = 505;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickCyc2`.
    pub(crate) const TRICK_CYC_2_506: u16 = 506;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickCyc3`.
    pub(crate) const TRICK_CYC_3_507: u16 = 507;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickOut`.
    pub(crate) const TRICK_OUT_508: u16 = 508;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto`.
    pub(crate) const TRICK_INTO_509: u16 = 509;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickCyc1`.
    pub(crate) const TRICK_CYC_1_510: u16 = 510;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickCyc2`.
    pub(crate) const TRICK_CYC_2_511: u16 = 511;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickCyc3`.
    pub(crate) const TRICK_CYC_3_512: u16 = 512;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickOut`.
    pub(crate) const TRICK_OUT_513: u16 = 513;
    /// `OnBoard/Trick/TailTrick/Ollie/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_514: u16 = 514;
    /// `OnBoard/Trick/TailTrick/Ollie/Takeoff`.
    pub(crate) const TAKEOFF_515: u16 = 515;
    /// `OnBoard/Trick/TailTrick/Ollie/LeftGround`.
    pub(crate) const LEFT_GROUND_516: u16 = 516;
    /// `OnBoard/Trick/TailTrick/Kickflip/In`.
    pub(crate) const IN_517: u16 = 517;
    /// `OnBoard/Trick/TailTrick/Kickflip/Cyc`.
    pub(crate) const CYC_518: u16 = 518;
    /// `OnBoard/Trick/TailTrick/Kickflip/End`.
    pub(crate) const END_519: u16 = 519;
    /// `OnBoard/Trick/TailTrick/Heelflip/In`.
    pub(crate) const IN_520: u16 = 520;
    /// `OnBoard/Trick/TailTrick/Heelflip/Cyc`.
    pub(crate) const CYC_521: u16 = 521;
    /// `OnBoard/Trick/TailTrick/Heelflip/End`.
    pub(crate) const END_522: u16 = 522;
    /// `OnBoard/Trick/TailTrick/PopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_523: u16 = 523;
    /// `OnBoard/Trick/TailTrick/PopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_524: u16 = 524;
    /// `OnBoard/Trick/TailTrick/PopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_525: u16 = 525;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_526: u16 = 526;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_527: u16 = 527;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_528: u16 = 528;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_529: u16 = 529;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/Takeoff`.
    pub(crate) const TAKEOFF_530: u16 = 530;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/LeftGround`.
    pub(crate) const LEFT_GROUND_531: u16 = 531;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_532: u16 = 532;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/Takeoff`.
    pub(crate) const TAKEOFF_533: u16 = 533;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/LeftGround`.
    pub(crate) const LEFT_GROUND_534: u16 = 534;
    /// `OnBoard/Trick/TailTrick/Hardflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_535: u16 = 535;
    /// `OnBoard/Trick/TailTrick/Hardflip/Takeoff`.
    pub(crate) const TAKEOFF_536: u16 = 536;
    /// `OnBoard/Trick/TailTrick/Hardflip/LeftGround`.
    pub(crate) const LEFT_GROUND_537: u16 = 537;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_538: u16 = 538;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/Takeoff`.
    pub(crate) const TAKEOFF_539: u16 = 539;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/LeftGround`.
    pub(crate) const LEFT_GROUND_540: u16 = 540;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_541: u16 = 541;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_542: u16 = 542;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_543: u16 = 543;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_544: u16 = 544;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_545: u16 = 545;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_546: u16 = 546;
    /// `OnBoard/Trick/TailTrick/360Flip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_547: u16 = 547;
    /// `OnBoard/Trick/TailTrick/360Flip/Takeoff`.
    pub(crate) const TAKEOFF_548: u16 = 548;
    /// `OnBoard/Trick/TailTrick/360Flip/LeftGround`.
    pub(crate) const LEFT_GROUND_549: u16 = 549;
    /// `OnBoard/Trick/TailTrick/Laserflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_550: u16 = 550;
    /// `OnBoard/Trick/TailTrick/Laserflip/Takeoff`.
    pub(crate) const TAKEOFF_551: u16 = 551;
    /// `OnBoard/Trick/TailTrick/Laserflip/LeftGround`.
    pub(crate) const LEFT_GROUND_552: u16 = 552;
    /// `OnBoard/Trick/TailTrick/360Hardflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_553: u16 = 553;
    /// `OnBoard/Trick/TailTrick/360Hardflip/Takeoff`.
    pub(crate) const TAKEOFF_554: u16 = 554;
    /// `OnBoard/Trick/TailTrick/360Hardflip/LeftGround`.
    pub(crate) const LEFT_GROUND_555: u16 = 555;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_556: u16 = 556;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/Takeoff`.
    pub(crate) const TAKEOFF_557: u16 = 557;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/LeftGround`.
    pub(crate) const LEFT_GROUND_558: u16 = 558;
    /// `OnBoard/Trick/NoseTrick/Nollie/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_559: u16 = 559;
    /// `OnBoard/Trick/NoseTrick/Nollie/Takeoff`.
    pub(crate) const TAKEOFF_560: u16 = 560;
    /// `OnBoard/Trick/NoseTrick/Nollie/LeftGround`.
    pub(crate) const LEFT_GROUND_561: u16 = 561;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In`.
    pub(crate) const IN_562: u16 = 562;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/Cyc`.
    pub(crate) const CYC_563: u16 = 563;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/End`.
    pub(crate) const END_564: u16 = 564;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In`.
    pub(crate) const IN_565: u16 = 565;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/Cyc`.
    pub(crate) const CYC_566: u16 = 566;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/End`.
    pub(crate) const END_567: u16 = 567;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_568: u16 = 568;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_569: u16 = 569;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_570: u16 = 570;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_571: u16 = 571;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_572: u16 = 572;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_573: u16 = 573;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_574: u16 = 574;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/Takeoff`.
    pub(crate) const TAKEOFF_575: u16 = 575;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/LeftGround`.
    pub(crate) const LEFT_GROUND_576: u16 = 576;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_577: u16 = 577;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/Takeoff`.
    pub(crate) const TAKEOFF_578: u16 = 578;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/LeftGround`.
    pub(crate) const LEFT_GROUND_579: u16 = 579;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_580: u16 = 580;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/Takeoff`.
    pub(crate) const TAKEOFF_581: u16 = 581;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/LeftGround`.
    pub(crate) const LEFT_GROUND_582: u16 = 582;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_583: u16 = 583;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/Takeoff`.
    pub(crate) const TAKEOFF_584: u16 = 584;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/LeftGround`.
    pub(crate) const LEFT_GROUND_585: u16 = 585;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_586: u16 = 586;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_587: u16 = 587;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_588: u16 = 588;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_589: u16 = 589;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/Takeoff`.
    pub(crate) const TAKEOFF_590: u16 = 590;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/LeftGround`.
    pub(crate) const LEFT_GROUND_591: u16 = 591;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_592: u16 = 592;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/Takeoff`.
    pub(crate) const TAKEOFF_593: u16 = 593;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/LeftGround`.
    pub(crate) const LEFT_GROUND_594: u16 = 594;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_595: u16 = 595;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/Takeoff`.
    pub(crate) const TAKEOFF_596: u16 = 596;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/LeftGround`.
    pub(crate) const LEFT_GROUND_597: u16 = 597;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_598: u16 = 598;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/Takeoff`.
    pub(crate) const TAKEOFF_599: u16 = 599;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/LeftGround`.
    pub(crate) const LEFT_GROUND_600: u16 = 600;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_601: u16 = 601;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/Takeoff`.
    pub(crate) const TAKEOFF_602: u16 = 602;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/LeftGround`.
    pub(crate) const LEFT_GROUND_603: u16 = 603;
    /// `OnBoard/Trick/PushTrick/FSBoneless/PlantingFoot`.
    pub(crate) const PLANTING_FOOT_604: u16 = 604;
    /// `OnBoard/Trick/PushTrick/FSBoneless/FootPlant`.
    pub(crate) const FOOT_PLANT_605: u16 = 605;
    /// `OnBoard/Trick/PushTrick/FSBoneless/Lift`.
    pub(crate) const LIFT_606: u16 = 606;
    /// `OnBoard/Trick/PushTrick/FSBoneless/LeftGround`.
    pub(crate) const LEFT_GROUND_607: u16 = 607;
    /// `OnBoard/Trick/PushTrick/BSBoneless/PlantingFoot`.
    pub(crate) const PLANTING_FOOT_608: u16 = 608;
    /// `OnBoard/Trick/PushTrick/BSBoneless/FootPlant`.
    pub(crate) const FOOT_PLANT_609: u16 = 609;
    /// `OnBoard/Trick/PushTrick/BSBoneless/Lift`.
    pub(crate) const LIFT_610: u16 = 610;
    /// `OnBoard/Trick/PushTrick/BSBoneless/LeftGround`.
    pub(crate) const LEFT_GROUND_611: u16 = 611;
    /// `OnBoard/Trick/PushTrick/FSFastPlant/PlantingFoot`.
    pub(crate) const PLANTING_FOOT_612: u16 = 612;
    /// `OnBoard/Trick/PushTrick/FSFastPlant/FootPlant`.
    pub(crate) const FOOT_PLANT_613: u16 = 613;
    /// `OnBoard/Trick/PushTrick/FSFastPlant/Lift`.
    pub(crate) const LIFT_614: u16 = 614;
    /// `OnBoard/Trick/PushTrick/FSFastPlant/LeftGround`.
    pub(crate) const LEFT_GROUND_615: u16 = 615;
    /// `OnBoard/Trick/PushTrick/BSFastPlant/PlantingFoot`.
    pub(crate) const PLANTING_FOOT_616: u16 = 616;
    /// `OnBoard/Trick/PushTrick/BSFastPlant/FootPlant`.
    pub(crate) const FOOT_PLANT_617: u16 = 617;
    /// `OnBoard/Trick/PushTrick/BSFastPlant/Lift`.
    pub(crate) const LIFT_618: u16 = 618;
    /// `OnBoard/Trick/PushTrick/BSFastPlant/LeftGround`.
    pub(crate) const LEFT_GROUND_619: u16 = 619;
    /// `OnBoard/Trick/PushTrick/NoComply/PlantingFoot`.
    pub(crate) const PLANTING_FOOT_620: u16 = 620;
    /// `OnBoard/Trick/PushTrick/NoComply/FootPlant`.
    pub(crate) const FOOT_PLANT_621: u16 = 621;
    /// `OnBoard/Trick/PushTrick/NoComply/Lift`.
    pub(crate) const LIFT_622: u16 = 622;
    /// `OnBoard/Trick/PushTrick/NoComply/LeftGround`.
    pub(crate) const LEFT_GROUND_623: u16 = 623;
    /// `OnBoard/HandPlantAir/FS/OneFootRight/Into`.
    pub(crate) const INTO_624: u16 = 624;
    /// `OnBoard/HandPlantAir/FS/OneFootRight/Cyc`.
    pub(crate) const CYC_625: u16 = 625;
    /// `OnBoard/HandPlantAir/FS/OneFootRight/Out`.
    pub(crate) const OUT_626: u16 = 626;
    /// `OnBoard/HandPlantAir/FS/OneFootLeft/Into`.
    pub(crate) const INTO_627: u16 = 627;
    /// `OnBoard/HandPlantAir/FS/OneFootLeft/Cyc`.
    pub(crate) const CYC_628: u16 = 628;
    /// `OnBoard/HandPlantAir/FS/OneFootLeft/Out`.
    pub(crate) const OUT_629: u16 = 629;
    /// `OnBoard/HandPlantAir/FS/Dismount/Into`.
    pub(crate) const INTO_630: u16 = 630;
    /// `OnBoard/HandPlantAir/FS/Dismount/Cyc`.
    pub(crate) const CYC_631: u16 = 631;
    /// `OnBoard/HandPlantAir/FS/Dismount/Out`.
    pub(crate) const OUT_632: u16 = 632;
    /// `OnBoard/HandPlantAir/BS/OneFootRight/Into`.
    pub(crate) const INTO_633: u16 = 633;
    /// `OnBoard/HandPlantAir/BS/OneFootRight/Cyc`.
    pub(crate) const CYC_634: u16 = 634;
    /// `OnBoard/HandPlantAir/BS/OneFootRight/Out`.
    pub(crate) const OUT_635: u16 = 635;
    /// `OnBoard/HandPlantAir/BS/OneFootLeft/Into`.
    pub(crate) const INTO_636: u16 = 636;
    /// `OnBoard/HandPlantAir/BS/OneFootLeft/Cyc`.
    pub(crate) const CYC_637: u16 = 637;
    /// `OnBoard/HandPlantAir/BS/OneFootLeft/Out`.
    pub(crate) const OUT_638: u16 = 638;
    /// `OnBoard/HandPlantAir/BS/Dismount/Into`.
    pub(crate) const INTO_639: u16 = 639;
    /// `OnBoard/HandPlantAir/BS/Dismount/Cyc`.
    pub(crate) const CYC_640: u16 = 640;
    /// `OnBoard/HandPlantAir/BS/Dismount/Out`.
    pub(crate) const OUT_641: u16 = 641;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Into`.
    pub(crate) const INTO_642: u16 = 642;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Cyc`.
    pub(crate) const CYC_643: u16 = 643;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Out`.
    pub(crate) const OUT_644: u16 = 644;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Superman`.
    pub(crate) const SUPERMAN_645: u16 = 645;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/ToOffBoard`.
    pub(crate) const TO_OFF_BOARD_646: u16 = 646;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Into`.
    pub(crate) const INTO_647: u16 = 647;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Cyc`.
    pub(crate) const CYC_648: u16 = 648;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount`.
    pub(crate) const DISMOUNT_649: u16 = 649;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir`.
    pub(crate) const ONE_FOOT_AIR_650: u16 = 650;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_651: u16 = 651;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FSGrabFingerFlipShuv`.
    pub(crate) const FS_GRAB_FINGER_FLIP_SHUV_652: u16 = 652;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FingerFlipVar`.
    pub(crate) const FINGER_FLIP_VAR_653: u16 = 653;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Out`.
    pub(crate) const OUT_654: u16 = 654;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/ToDBLGrab`.
    pub(crate) const TO_DBL_GRAB_655: u16 = 655;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FromDBLGrab`.
    pub(crate) const FROM_DBL_GRAB_656: u16 = 656;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/ToOffBoard`.
    pub(crate) const TO_OFF_BOARD_657: u16 = 657;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Into`.
    pub(crate) const INTO_658: u16 = 658;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Cyc`.
    pub(crate) const CYC_659: u16 = 659;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount`.
    pub(crate) const DISMOUNT_660: u16 = 660;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir`.
    pub(crate) const ONE_FOOT_AIR_661: u16 = 661;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_662: u16 = 662;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/BSGrabFingerFlipShuv`.
    pub(crate) const BS_GRAB_FINGER_FLIP_SHUV_663: u16 = 663;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FingerFlipVar`.
    pub(crate) const FINGER_FLIP_VAR_664: u16 = 664;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Out`.
    pub(crate) const OUT_665: u16 = 665;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/ToDBLGrab`.
    pub(crate) const TO_DBL_GRAB_666: u16 = 666;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FromDBLGrab`.
    pub(crate) const FROM_DBL_GRAB_667: u16 = 667;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/ToOffBoard`.
    pub(crate) const TO_OFF_BOARD_668: u16 = 668;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/Into`.
    pub(crate) const INTO_669: u16 = 669;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/Cyc`.
    pub(crate) const CYC_670: u16 = 670;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_671: u16 = 671;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/MuteFingerFlipShuv`.
    pub(crate) const MUTE_FINGER_FLIP_SHUV_672: u16 = 672;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/StaleFingerFlipShuv`.
    pub(crate) const STALE_FINGER_FLIP_SHUV_673: u16 = 673;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/Out`.
    pub(crate) const OUT_674: u16 = 674;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/ToOffBoard`.
    pub(crate) const TO_OFF_BOARD_675: u16 = 675;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/Into`.
    pub(crate) const INTO_676: u16 = 676;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/Cyc`.
    pub(crate) const CYC_677: u16 = 677;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/FingerFlip`.
    pub(crate) const FINGER_FLIP_678: u16 = 678;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/MuteFingerFlipShuv`.
    pub(crate) const MUTE_FINGER_FLIP_SHUV_679: u16 = 679;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/StaleFingerFlipShuv`.
    pub(crate) const STALE_FINGER_FLIP_SHUV_680: u16 = 680;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/Out`.
    pub(crate) const OUT_681: u16 = 681;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/ToOffBoard`.
    pub(crate) const TO_OFF_BOARD_682: u16 = 682;
    /// `OnBoard/InAir/BoardAdjusts/Left/Idle`.
    pub(crate) const IDLE_683: u16 = 683;
    /// `OnBoard/InAir/BoardAdjusts/Right/Idle`.
    pub(crate) const IDLE_684: u16 = 684;
    /// `OnBoard/InAir/BoardAdjusts/Up/Idle`.
    pub(crate) const IDLE_685: u16 = 685;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab`.
    pub(crate) const GRAB_686: u16 = 686;
    /// `OnBoard/InAir/BoardAdjusts/Down/Idle`.
    pub(crate) const IDLE_687: u16 = 687;
    /// `OnBoard/InAir/BoardAdjusts/Down/RocketAir`.
    pub(crate) const ROCKET_AIR_688: u16 = 688;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab`.
    pub(crate) const GRAB_689: u16 = 689;
    /// `OnBoard/InAir/InAirStatic/Air/Sequence`.
    pub(crate) const SEQUENCE_690: u16 = 690;
    /// `OnBoard/InAir/InAirStatic/Air/Default`.
    pub(crate) const DEFAULT_691: u16 = 691;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/RightFoot`.
    pub(crate) const RIGHT_FOOT_692: u16 = 692;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/LeftFoot`.
    pub(crate) const LEFT_FOOT_693: u16 = 693;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/RightFoot`.
    pub(crate) const RIGHT_FOOT_694: u16 = 694;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/LeftFoot`.
    pub(crate) const LEFT_FOOT_695: u16 = 695;
    /// `OnBoard/OnGround/EnterSkitching/Into/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_696: u16 = 696;
    /// `OnBoard/OnGround/EnterSkitching/Into/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_697: u16 = 697;
    /// `OnBoard/OnGround/EnterSkitching/Cyc/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_698: u16 = 698;
    /// `OnBoard/OnGround/EnterSkitching/Cyc/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_699: u16 = 699;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Absorb`.
    pub(crate) const ABSORB_700: u16 = 700;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding`.
    pub(crate) const HOLDING_701: u16 = 701;
    /// `OnBoard/OnGround/Skitching/Skitch1HandFS/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_702: u16 = 702;
    /// `OnBoard/OnGround/Skitching/Skitch1HandFS/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_703: u16 = 703;
    /// `OnBoard/OnGround/Skitching/Skitch1HandBS/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_704: u16 = 704;
    /// `OnBoard/OnGround/Skitching/Skitch1HandBS/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_705: u16 = 705;
    /// `OnBoard/OnGround/RidingIdle/Random/RandGonz`.
    pub(crate) const RAND_GONZ_706: u16 = 706;
    /// `OnBoard/OnGround/RidingIdle/Random/RandPJLadd`.
    pub(crate) const RAND_PJ_LADD_707: u16 = 707;
    /// `OnBoard/OnGround/RidingIdle/Random/RandCarroll1`.
    pub(crate) const RAND_CARROLL_1_708: u16 = 708;
    /// `OnBoard/OnGround/RidingIdle/Random/RandCarroll2`.
    pub(crate) const RAND_CARROLL_2_709: u16 = 709;
    /// `OnBoard/OnGround/RidingIdle/Random/RandDill`.
    pub(crate) const RAND_DILL_710: u16 = 710;
    /// `OnBoard/OnGround/RidingIdle/Random/RandHsu`.
    pub(crate) const RAND_HSU_711: u16 = 711;
    /// `OnBoard/OnGround/RidingIdle/Random/Rand1`.
    pub(crate) const RAND_1_712: u16 = 712;
    /// `OnBoard/OnGround/RidingIdle/Random/Rand2`.
    pub(crate) const RAND_2_713: u16 = 713;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake`.
    pub(crate) const BRAKE_714: u16 = 714;
    /// `OnBoard/OnGround/RidingIdle/Riding/KickTurn`.
    pub(crate) const KICK_TURN_715: u16 = 715;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding`.
    pub(crate) const SLIDING_716: u16 = 716;
    /// `OnBoard/OnGround/RidingIdle/Riding/HandPlantGround`.
    pub(crate) const HAND_PLANT_GROUND_717: u16 = 717;
    /// `OnBoard/OnGround/RidingIdle/Riding/Revert`.
    pub(crate) const REVERT_718: u16 = 718;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning`.
    pub(crate) const TURNING_719: u16 = 719;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT1/Immediate`.
    pub(crate) const IMMEDIATE_720: u16 = 720;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_721: u16 = 721;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT2/Immediate`.
    pub(crate) const IMMEDIATE_722: u16 = 722;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_723: u16 = 723;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT1/Immediate`.
    pub(crate) const IMMEDIATE_724: u16 = 724;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_725: u16 = 725;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT2/Immediate`.
    pub(crate) const IMMEDIATE_726: u16 = 726;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_727: u16 = 727;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT1/Immediate`.
    pub(crate) const IMMEDIATE_728: u16 = 728;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_729: u16 = 729;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT2/Immediate`.
    pub(crate) const IMMEDIATE_730: u16 = 730;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_731: u16 = 731;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT1/Immediate`.
    pub(crate) const IMMEDIATE_732: u16 = 732;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_733: u16 = 733;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT2/Immediate`.
    pub(crate) const IMMEDIATE_734: u16 = 734;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_735: u16 = 735;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT1/Immediate`.
    pub(crate) const IMMEDIATE_736: u16 = 736;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_737: u16 = 737;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT2/Immediate`.
    pub(crate) const IMMEDIATE_738: u16 = 738;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_739: u16 = 739;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT1/Immediate`.
    pub(crate) const IMMEDIATE_740: u16 = 740;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_741: u16 = 741;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT2/Immediate`.
    pub(crate) const IMMEDIATE_742: u16 = 742;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_743: u16 = 743;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT1/Immediate`.
    pub(crate) const IMMEDIATE_744: u16 = 744;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_745: u16 = 745;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT2/Immediate`.
    pub(crate) const IMMEDIATE_746: u16 = 746;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_747: u16 = 747;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT1/Immediate`.
    pub(crate) const IMMEDIATE_748: u16 = 748;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT1/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_749: u16 = 749;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT2/Immediate`.
    pub(crate) const IMMEDIATE_750: u16 = 750;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT2/DelayedCycle`.
    pub(crate) const DELAYED_CYCLE_751: u16 = 751;
    /// `OffBoard/OBGround/Locomotion/Idle/Standing/Still`.
    pub(crate) const STILL_752: u16 = 752;
    /// `OffBoard/OBGround/Locomotion/Idle/Standing/TurningRight`.
    pub(crate) const TURNING_RIGHT_753: u16 = 753;
    /// `OffBoard/OBGround/Locomotion/Idle/Standing/TurningLeft`.
    pub(crate) const TURNING_LEFT_754: u16 = 754;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/Into/Backslash`.
    pub(crate) const BACKSLASH_755: u16 = 755;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/Into/Rail`.
    pub(crate) const RAIL_756: u16 = 756;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/GrabCycle/Backslash`.
    pub(crate) const BACKSLASH_757: u16 = 757;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/GrabCycle/Rail`.
    pub(crate) const RAIL_758: u16 = 758;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/Out/Backslash`.
    pub(crate) const BACKSLASH_759: u16 = 759;
    /// `OnBoard/Grinding/BS_NOSEBLUNT/BS_NOSEBLUNT_TAILGRAB/Out/Rail`.
    pub(crate) const RAIL_760: u16 = 760;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/Into/Backslash`.
    pub(crate) const BACKSLASH_761: u16 = 761;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/Into/Rail`.
    pub(crate) const RAIL_762: u16 = 762;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/GrabCycle/Backslash`.
    pub(crate) const BACKSLASH_763: u16 = 763;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/GrabCycle/Rail`.
    pub(crate) const RAIL_764: u16 = 764;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/Out/Backslash`.
    pub(crate) const BACKSLASH_765: u16 = 765;
    /// `OnBoard/Grinding/FS_NOSEBLUNT/FS_NOSEBLUNT_TAILGRAB/Out/Rail`.
    pub(crate) const RAIL_766: u16 = 766;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/Into/Backslash`.
    pub(crate) const BACKSLASH_767: u16 = 767;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/Into/Rail`.
    pub(crate) const RAIL_768: u16 = 768;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/GrabCycle/Backslash`.
    pub(crate) const BACKSLASH_769: u16 = 769;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/GrabCycle/Rail`.
    pub(crate) const RAIL_770: u16 = 770;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/Out/Backslash`.
    pub(crate) const BACKSLASH_771: u16 = 771;
    /// `OnBoard/Grinding/BS_BLUNT/BS_BLUNT_TAILGRAB/Out/Rail`.
    pub(crate) const RAIL_772: u16 = 772;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/Into/Backslash`.
    pub(crate) const BACKSLASH_773: u16 = 773;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/Into/Rail`.
    pub(crate) const RAIL_774: u16 = 774;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/GrabCycle/Backslash`.
    pub(crate) const BACKSLASH_775: u16 = 775;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/GrabCycle/Rail`.
    pub(crate) const RAIL_776: u16 = 776;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/Out/Backslash`.
    pub(crate) const BACKSLASH_777: u16 = 777;
    /// `OnBoard/Grinding/FS_BLUNT/FS_BLUNT_TAILGRAB/Out/Rail`.
    pub(crate) const RAIL_778: u16 = 778;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_779: u16 = 779;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/TakeOff`.
    pub(crate) const TAKE_OFF_780: u16 = 780;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/LeftGround`.
    pub(crate) const LEFT_GROUND_781: u16 = 781;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_782: u16 = 782;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/TakeOff`.
    pub(crate) const TAKE_OFF_783: u16 = 783;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/LeftGround`.
    pub(crate) const LEFT_GROUND_784: u16 = 784;
    /// `OnBoard/Trick/TailTrick/Ollie/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_785: u16 = 785;
    /// `OnBoard/Trick/TailTrick/Ollie/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_786: u16 = 786;
    /// `OnBoard/Trick/TailTrick/Ollie/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_787: u16 = 787;
    /// `OnBoard/Trick/TailTrick/Ollie/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_788: u16 = 788;
    /// `OnBoard/Trick/TailTrick/Ollie/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_789: u16 = 789;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_790: u16 = 790;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/TakeOff`.
    pub(crate) const TAKE_OFF_791: u16 = 791;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/LeftGround`.
    pub(crate) const LEFT_GROUND_792: u16 = 792;
    /// `OnBoard/Trick/TailTrick/Kickflip/Cyc/Cyc1`.
    pub(crate) const CYC_1_793: u16 = 793;
    /// `OnBoard/Trick/TailTrick/Kickflip/Cyc/Cyc2`.
    pub(crate) const CYC_2_794: u16 = 794;
    /// `OnBoard/Trick/TailTrick/Kickflip/End/Cyc3`.
    pub(crate) const CYC_3_795: u16 = 795;
    /// `OnBoard/Trick/TailTrick/Kickflip/End/Out`.
    pub(crate) const OUT_796: u16 = 796;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_797: u16 = 797;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/TakeOff`.
    pub(crate) const TAKE_OFF_798: u16 = 798;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/LeftGround`.
    pub(crate) const LEFT_GROUND_799: u16 = 799;
    /// `OnBoard/Trick/TailTrick/Heelflip/Cyc/Cyc1`.
    pub(crate) const CYC_1_800: u16 = 800;
    /// `OnBoard/Trick/TailTrick/Heelflip/Cyc/Cyc2`.
    pub(crate) const CYC_2_801: u16 = 801;
    /// `OnBoard/Trick/TailTrick/Heelflip/End/Cyc3`.
    pub(crate) const CYC_3_802: u16 = 802;
    /// `OnBoard/Trick/TailTrick/Heelflip/End/Out`.
    pub(crate) const OUT_803: u16 = 803;
    /// `OnBoard/Trick/TailTrick/PopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_804: u16 = 804;
    /// `OnBoard/Trick/TailTrick/PopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_805: u16 = 805;
    /// `OnBoard/Trick/TailTrick/PopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_806: u16 = 806;
    /// `OnBoard/Trick/TailTrick/PopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_807: u16 = 807;
    /// `OnBoard/Trick/TailTrick/PopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_808: u16 = 808;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_809: u16 = 809;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_810: u16 = 810;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_811: u16 = 811;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_812: u16 = 812;
    /// `OnBoard/Trick/TailTrick/FSPopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_813: u16 = 813;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_814: u16 = 814;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_815: u16 = 815;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_816: u16 = 816;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_817: u16 = 817;
    /// `OnBoard/Trick/TailTrick/VarialKickflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_818: u16 = 818;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_819: u16 = 819;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_820: u16 = 820;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_821: u16 = 821;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_822: u16 = 822;
    /// `OnBoard/Trick/TailTrick/VarialHeelflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_823: u16 = 823;
    /// `OnBoard/Trick/TailTrick/Hardflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_824: u16 = 824;
    /// `OnBoard/Trick/TailTrick/Hardflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_825: u16 = 825;
    /// `OnBoard/Trick/TailTrick/Hardflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_826: u16 = 826;
    /// `OnBoard/Trick/TailTrick/Hardflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_827: u16 = 827;
    /// `OnBoard/Trick/TailTrick/Hardflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_828: u16 = 828;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_829: u16 = 829;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_830: u16 = 830;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_831: u16 = 831;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_832: u16 = 832;
    /// `OnBoard/Trick/TailTrick/InwardHeelflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_833: u16 = 833;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_834: u16 = 834;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_835: u16 = 835;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_836: u16 = 836;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_837: u16 = 837;
    /// `OnBoard/Trick/TailTrick/360PopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_838: u16 = 838;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_839: u16 = 839;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_840: u16 = 840;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_841: u16 = 841;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_842: u16 = 842;
    /// `OnBoard/Trick/TailTrick/FS360PopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_843: u16 = 843;
    /// `OnBoard/Trick/TailTrick/360Flip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_844: u16 = 844;
    /// `OnBoard/Trick/TailTrick/360Flip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_845: u16 = 845;
    /// `OnBoard/Trick/TailTrick/360Flip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_846: u16 = 846;
    /// `OnBoard/Trick/TailTrick/360Flip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_847: u16 = 847;
    /// `OnBoard/Trick/TailTrick/360Flip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_848: u16 = 848;
    /// `OnBoard/Trick/TailTrick/Laserflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_849: u16 = 849;
    /// `OnBoard/Trick/TailTrick/Laserflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_850: u16 = 850;
    /// `OnBoard/Trick/TailTrick/Laserflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_851: u16 = 851;
    /// `OnBoard/Trick/TailTrick/Laserflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_852: u16 = 852;
    /// `OnBoard/Trick/TailTrick/Laserflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_853: u16 = 853;
    /// `OnBoard/Trick/TailTrick/360Hardflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_854: u16 = 854;
    /// `OnBoard/Trick/TailTrick/360Hardflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_855: u16 = 855;
    /// `OnBoard/Trick/TailTrick/360Hardflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_856: u16 = 856;
    /// `OnBoard/Trick/TailTrick/360Hardflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_857: u16 = 857;
    /// `OnBoard/Trick/TailTrick/360Hardflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_858: u16 = 858;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_859: u16 = 859;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_860: u16 = 860;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_861: u16 = 861;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_862: u16 = 862;
    /// `OnBoard/Trick/TailTrick/360InwardHeelflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_863: u16 = 863;
    /// `OnBoard/Trick/NoseTrick/Nollie/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_864: u16 = 864;
    /// `OnBoard/Trick/NoseTrick/Nollie/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_865: u16 = 865;
    /// `OnBoard/Trick/NoseTrick/Nollie/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_866: u16 = 866;
    /// `OnBoard/Trick/NoseTrick/Nollie/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_867: u16 = 867;
    /// `OnBoard/Trick/NoseTrick/Nollie/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_868: u16 = 868;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_869: u16 = 869;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/TakeOff`.
    pub(crate) const TAKE_OFF_870: u16 = 870;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/LeftGround`.
    pub(crate) const LEFT_GROUND_871: u16 = 871;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/Cyc/Cyc1`.
    pub(crate) const CYC_1_872: u16 = 872;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/Cyc/Cyc2`.
    pub(crate) const CYC_2_873: u16 = 873;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/End/Cyc3`.
    pub(crate) const CYC_3_874: u16 = 874;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/End/Out`.
    pub(crate) const OUT_875: u16 = 875;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/GrindOutAssist`.
    pub(crate) const GRIND_OUT_ASSIST_876: u16 = 876;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/TakeOff`.
    pub(crate) const TAKE_OFF_877: u16 = 877;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/LeftGround`.
    pub(crate) const LEFT_GROUND_878: u16 = 878;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/Cyc/Cyc1`.
    pub(crate) const CYC_1_879: u16 = 879;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/Cyc/Cyc2`.
    pub(crate) const CYC_2_880: u16 = 880;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/End/Cyc3`.
    pub(crate) const CYC_3_881: u16 = 881;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/End/Out`.
    pub(crate) const OUT_882: u16 = 882;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_883: u16 = 883;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_884: u16 = 884;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_885: u16 = 885;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_886: u16 = 886;
    /// `OnBoard/Trick/NoseTrick/N_PopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_887: u16 = 887;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_888: u16 = 888;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_889: u16 = 889;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_890: u16 = 890;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_891: u16 = 891;
    /// `OnBoard/Trick/NoseTrick/N_FSPopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_892: u16 = 892;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_893: u16 = 893;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_894: u16 = 894;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_895: u16 = 895;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_896: u16 = 896;
    /// `OnBoard/Trick/NoseTrick/N_VarialKickflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_897: u16 = 897;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_898: u16 = 898;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_899: u16 = 899;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_900: u16 = 900;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_901: u16 = 901;
    /// `OnBoard/Trick/NoseTrick/N_VarialHeelflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_902: u16 = 902;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_903: u16 = 903;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_904: u16 = 904;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_905: u16 = 905;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_906: u16 = 906;
    /// `OnBoard/Trick/NoseTrick/N_Hardflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_907: u16 = 907;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_908: u16 = 908;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_909: u16 = 909;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_910: u16 = 910;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_911: u16 = 911;
    /// `OnBoard/Trick/NoseTrick/N_InwardHeelflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_912: u16 = 912;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_913: u16 = 913;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_914: u16 = 914;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_915: u16 = 915;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_916: u16 = 916;
    /// `OnBoard/Trick/NoseTrick/N_360PopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_917: u16 = 917;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_918: u16 = 918;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_919: u16 = 919;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_920: u16 = 920;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_921: u16 = 921;
    /// `OnBoard/Trick/NoseTrick/N_FS360PopShuvit/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_922: u16 = 922;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_923: u16 = 923;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_924: u16 = 924;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_925: u16 = 925;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_926: u16 = 926;
    /// `OnBoard/Trick/NoseTrick/N_360Flip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_927: u16 = 927;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_928: u16 = 928;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_929: u16 = 929;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_930: u16 = 930;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_931: u16 = 931;
    /// `OnBoard/Trick/NoseTrick/N_Laserflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_932: u16 = 932;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_933: u16 = 933;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_934: u16 = 934;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_935: u16 = 935;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_936: u16 = 936;
    /// `OnBoard/Trick/NoseTrick/N_360Hardflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_937: u16 = 937;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_938: u16 = 938;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_939: u16 = 939;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/Takeoff/FromManual`.
    pub(crate) const FROM_MANUAL_940: u16 = 940;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/Takeoff/FromGrind`.
    pub(crate) const FROM_GRIND_941: u16 = 941;
    /// `OnBoard/Trick/NoseTrick/N_360InwardHeelflip/Takeoff/FromAntic`.
    pub(crate) const FROM_ANTIC_942: u16 = 942;
    /// `OnBoard/Trick/PushTrick/FSBoneless/PlantingFoot/Left`.
    pub(crate) const LEFT_943: u16 = 943;
    /// `OnBoard/Trick/PushTrick/FSBoneless/PlantingFoot/Right`.
    pub(crate) const RIGHT_944: u16 = 944;
    /// `OnBoard/Trick/PushTrick/BSBoneless/PlantingFoot/Left`.
    pub(crate) const LEFT_945: u16 = 945;
    /// `OnBoard/Trick/PushTrick/BSBoneless/PlantingFoot/Right`.
    pub(crate) const RIGHT_946: u16 = 946;
    /// `OnBoard/Trick/PushTrick/FSFastPlant/PlantingFoot/Left`.
    pub(crate) const LEFT_947: u16 = 947;
    /// `OnBoard/Trick/PushTrick/FSFastPlant/PlantingFoot/Right`.
    pub(crate) const RIGHT_948: u16 = 948;
    /// `OnBoard/Trick/PushTrick/BSFastPlant/PlantingFoot/Left`.
    pub(crate) const LEFT_949: u16 = 949;
    /// `OnBoard/Trick/PushTrick/BSFastPlant/PlantingFoot/Right`.
    pub(crate) const RIGHT_950: u16 = 950;
    /// `OnBoard/Trick/PushTrick/NoComply/PlantingFoot/Left`.
    pub(crate) const LEFT_951: u16 = 951;
    /// `OnBoard/Trick/PushTrick/NoComply/PlantingFoot/Right`.
    pub(crate) const RIGHT_952: u16 = 952;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Superman/SupermanInto`.
    pub(crate) const SUPERMAN_INTO_953: u16 = 953;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Superman/SupermanCyc`.
    pub(crate) const SUPERMAN_CYC_954: u16 = 954;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Superman/SupermanOut`.
    pub(crate) const SUPERMAN_OUT_955: u16 = 955;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Superman/ToFSChrist`.
    pub(crate) const TO_FS_CHRIST_956: u16 = 956;
    /// `OnBoard/InAir/GrabsTweaks/DBLGrab/Superman/ToBSChrist`.
    pub(crate) const TO_BS_CHRIST_957: u16 = 957;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount/Into`.
    pub(crate) const INTO_958: u16 = 958;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount/Cyc`.
    pub(crate) const CYC_959: u16 = 959;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount/Out`.
    pub(crate) const OUT_960: u16 = 960;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount/ToSuperman`.
    pub(crate) const TO_SUPERMAN_961: u16 = 961;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot`.
    pub(crate) const FS_RIGHT_FOOT_962: u16 = 962;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot`.
    pub(crate) const FS_LEFT_FOOT_963: u16 = 963;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot`.
    pub(crate) const BS_RIGHT_FOOT_964: u16 = 964;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot`.
    pub(crate) const BS_LEFT_FOOT_965: u16 = 965;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FingerFlip/Into`.
    pub(crate) const INTO_966: u16 = 966;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FingerFlip/GrabOut`.
    pub(crate) const GRAB_OUT_967: u16 = 967;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FingerFlipVar/Into`.
    pub(crate) const INTO_968: u16 = 968;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/FingerFlipVar/GrabOut`.
    pub(crate) const GRAB_OUT_969: u16 = 969;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount/Into`.
    pub(crate) const INTO_970: u16 = 970;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount/Cyc`.
    pub(crate) const CYC_971: u16 = 971;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount/Out`.
    pub(crate) const OUT_972: u16 = 972;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount/ToSuperman`.
    pub(crate) const TO_SUPERMAN_973: u16 = 973;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot`.
    pub(crate) const FS_RIGHT_FOOT_974: u16 = 974;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot`.
    pub(crate) const FS_LEFT_FOOT_975: u16 = 975;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot`.
    pub(crate) const BS_RIGHT_FOOT_976: u16 = 976;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot`.
    pub(crate) const BS_LEFT_FOOT_977: u16 = 977;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FingerFlip/Into`.
    pub(crate) const INTO_978: u16 = 978;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FingerFlip/GrabOut`.
    pub(crate) const GRAB_OUT_979: u16 = 979;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FingerFlipVar/Into`.
    pub(crate) const INTO_980: u16 = 980;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/FingerFlipVar/GrabOut`.
    pub(crate) const GRAB_OUT_981: u16 = 981;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/FingerFlip/Into`.
    pub(crate) const INTO_982: u16 = 982;
    /// `OnBoard/InAir/GrabsTweaks/MuteGrab/FingerFlip/GrabOut`.
    pub(crate) const GRAB_OUT_983: u16 = 983;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/FingerFlip/Into`.
    pub(crate) const INTO_984: u16 = 984;
    /// `OnBoard/InAir/GrabsTweaks/StaleGrab/FingerFlip/GrabOut`.
    pub(crate) const GRAB_OUT_985: u16 = 985;
    /// `OnBoard/InAir/BoardAdjusts/Left/Idle/Air`.
    pub(crate) const AIR_986: u16 = 986;
    /// `OnBoard/InAir/BoardAdjusts/Right/Idle/Air`.
    pub(crate) const AIR_987: u16 = 987;
    /// `OnBoard/InAir/BoardAdjusts/Up/Idle/Air`.
    pub(crate) const AIR_988: u16 = 988;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing`.
    pub(crate) const GRABBING_989: u16 = 989;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Out`.
    pub(crate) const OUT_990: u16 = 990;
    /// `OnBoard/InAir/BoardAdjusts/Down/Idle/Air`.
    pub(crate) const AIR_991: u16 = 991;
    /// `OnBoard/InAir/BoardAdjusts/Down/RocketAir/Into`.
    pub(crate) const INTO_992: u16 = 992;
    /// `OnBoard/InAir/BoardAdjusts/Down/RocketAir/Cycle`.
    pub(crate) const CYCLE_993: u16 = 993;
    /// `OnBoard/InAir/BoardAdjusts/Down/RocketAir/Out`.
    pub(crate) const OUT_994: u16 = 994;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing`.
    pub(crate) const GRABBING_995: u16 = 995;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Out`.
    pub(crate) const OUT_996: u16 = 996;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/RightFoot/Into`.
    pub(crate) const INTO_997: u16 = 997;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/RightFoot/Cyc`.
    pub(crate) const CYC_998: u16 = 998;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/RightFoot/Out`.
    pub(crate) const OUT_999: u16 = 999;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/LeftFoot/Into`.
    pub(crate) const INTO_1000: u16 = 1000;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/LeftFoot/Cyc`.
    pub(crate) const CYC_1001: u16 = 1001;
    /// `OnBoard/InAir/OneFootOllie/RightFootForward/LeftFoot/Out`.
    pub(crate) const OUT_1002: u16 = 1002;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/RightFoot/Into`.
    pub(crate) const INTO_1003: u16 = 1003;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/RightFoot/Cyc`.
    pub(crate) const CYC_1004: u16 = 1004;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/RightFoot/Out`.
    pub(crate) const OUT_1005: u16 = 1005;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/LeftFoot/Into`.
    pub(crate) const INTO_1006: u16 = 1006;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/LeftFoot/Cyc`.
    pub(crate) const CYC_1007: u16 = 1007;
    /// `OnBoard/InAir/OneFootOllie/LeftFootForward/LeftFoot/Out`.
    pub(crate) const OUT_1008: u16 = 1008;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Absorb/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_1009: u16 = 1009;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Absorb/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_1010: u16 = 1010;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/ShimmyingLeft`.
    pub(crate) const SHIMMYING_LEFT_1011: u16 = 1011;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/ShimmyingRight`.
    pub(crate) const SHIMMYING_RIGHT_1012: u16 = 1012;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/Idle`.
    pub(crate) const IDLE_1013: u16 = 1013;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Moving`.
    pub(crate) const MOVING_1014: u16 = 1014;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Stationary`.
    pub(crate) const STATIONARY_1015: u16 = 1015;
    /// `OnBoard/OnGround/RidingIdle/Riding/KickTurn/FromFakie`.
    pub(crate) const FROM_FAKIE_1016: u16 = 1016;
    /// `OnBoard/OnGround/RidingIdle/Riding/KickTurn/Right`.
    pub(crate) const RIGHT_1017: u16 = 1017;
    /// `OnBoard/OnGround/RidingIdle/Riding/KickTurn/Left`.
    pub(crate) const LEFT_1018: u16 = 1018;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left`.
    pub(crate) const LEFT_1019: u16 = 1019;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right`.
    pub(crate) const RIGHT_1020: u16 = 1020;
    /// `OnBoard/OnGround/RidingIdle/Riding/HandPlantGround/FS`.
    pub(crate) const FS_1021: u16 = 1021;
    /// `OnBoard/OnGround/RidingIdle/Riding/HandPlantGround/BS`.
    pub(crate) const BS_1022: u16 = 1022;
    /// `OnBoard/OnGround/RidingIdle/Riding/Revert/Bs`.
    pub(crate) const BS_1023: u16 = 1023;
    /// `OnBoard/OnGround/RidingIdle/Riding/Revert/Fs`.
    pub(crate) const FS_1024: u16 = 1024;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Idle`.
    pub(crate) const IDLE_1025: u16 = 1025;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing`.
    pub(crate) const GRABBING_1026: u16 = 1026;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual`.
    pub(crate) const MANUAL_1027: u16 = 1027;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push`.
    pub(crate) const PUSH_1028: u16 = 1028;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/AnticOut`.
    pub(crate) const ANTIC_OUT_1029: u16 = 1029;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation`.
    pub(crate) const ANTICIPATION_1030: u16 = 1030;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Switch`.
    pub(crate) const SWITCH_1031: u16 = 1031;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1032: u16 = 1032;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1033: u16 = 1033;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1034: u16 = 1034;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHLEFTToRIGHT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1035: u16 = 1035;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1036: u16 = 1036;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1037: u16 = 1037;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1038: u16 = 1038;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHRIGHTToLEFT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1039: u16 = 1039;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1040: u16 = 1040;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1041: u16 = 1041;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1042: u16 = 1042;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLLEFTToRIGHT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1043: u16 = 1043;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1044: u16 = 1044;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1045: u16 = 1045;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1046: u16 = 1046;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLRIGHTToLEFT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1047: u16 = 1047;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1048: u16 = 1048;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1049: u16 = 1049;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1050: u16 = 1050;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLLEFT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1051: u16 = 1051;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1052: u16 = 1052;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1053: u16 = 1053;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1054: u16 = 1054;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PUSHToPULLRIGHT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1055: u16 = 1055;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1056: u16 = 1056;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1057: u16 = 1057;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1058: u16 = 1058;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHLEFT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1059: u16 = 1059;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT1/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1060: u16 = 1060;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT1/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1061: u16 = 1061;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT2/DelayedCycle/WaitForEnd`.
    pub(crate) const WAIT_FOR_END_1062: u16 = 1062;
    /// `OffBoard/OBGround/Locomotion/MovingObject/MVOBJ_PULLToPUSHRIGHT2/DelayedCycle/NewCycle`.
    pub(crate) const NEW_CYCLE_1063: u16 = 1063;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_1064: u16 = 1064;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_1065: u16 = 1065;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/TakeOff/FromManual`.
    pub(crate) const FROM_MANUAL_1066: u16 = 1066;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/TakeOff/FromGrind`.
    pub(crate) const FROM_GRIND_1067: u16 = 1067;
    /// `OnBoard/Trick/TailTrick/Kickflip_MikeCarroll/TrickInto/TakeOff/FromAntic`.
    pub(crate) const FROM_ANTIC_1068: u16 = 1068;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_1069: u16 = 1069;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_1070: u16 = 1070;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/TakeOff/FromManual`.
    pub(crate) const FROM_MANUAL_1071: u16 = 1071;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/TakeOff/FromGrind`.
    pub(crate) const FROM_GRIND_1072: u16 = 1072;
    /// `OnBoard/Trick/TailTrick/Kickflip_MarkGonzales/TrickInto/TakeOff/FromAntic`.
    pub(crate) const FROM_ANTIC_1073: u16 = 1073;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_1074: u16 = 1074;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_1075: u16 = 1075;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/TakeOff/FromManual`.
    pub(crate) const FROM_MANUAL_1076: u16 = 1076;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/TakeOff/FromGrind`.
    pub(crate) const FROM_GRIND_1077: u16 = 1077;
    /// `OnBoard/Trick/TailTrick/Kickflip/In/TakeOff/FromAntic`.
    pub(crate) const FROM_ANTIC_1078: u16 = 1078;
    /// `OnBoard/Trick/TailTrick/Kickflip/End/Out/Out1`.
    pub(crate) const OUT_1_1079: u16 = 1079;
    /// `OnBoard/Trick/TailTrick/Kickflip/End/Out/Out2`.
    pub(crate) const OUT_2_1080: u16 = 1080;
    /// `OnBoard/Trick/TailTrick/Kickflip/End/Out/Out3`.
    pub(crate) const OUT_3_1081: u16 = 1081;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_1082: u16 = 1082;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_1083: u16 = 1083;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/TakeOff/FromManual`.
    pub(crate) const FROM_MANUAL_1084: u16 = 1084;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/TakeOff/FromGrind`.
    pub(crate) const FROM_GRIND_1085: u16 = 1085;
    /// `OnBoard/Trick/TailTrick/Heelflip/In/TakeOff/FromAntic`.
    pub(crate) const FROM_ANTIC_1086: u16 = 1086;
    /// `OnBoard/Trick/TailTrick/Heelflip/End/Out/Out1`.
    pub(crate) const OUT_1_1087: u16 = 1087;
    /// `OnBoard/Trick/TailTrick/Heelflip/End/Out/Out2`.
    pub(crate) const OUT_2_1088: u16 = 1088;
    /// `OnBoard/Trick/TailTrick/Heelflip/End/Out/Out3`.
    pub(crate) const OUT_3_1089: u16 = 1089;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_1090: u16 = 1090;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_1091: u16 = 1091;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/TakeOff/FromManual`.
    pub(crate) const FROM_MANUAL_1092: u16 = 1092;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/TakeOff/FromGrind`.
    pub(crate) const FROM_GRIND_1093: u16 = 1093;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/In/TakeOff/FromAntic`.
    pub(crate) const FROM_ANTIC_1094: u16 = 1094;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/End/Out/Out1`.
    pub(crate) const OUT_1_1095: u16 = 1095;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/End/Out/Out2`.
    pub(crate) const OUT_2_1096: u16 = 1096;
    /// `OnBoard/Trick/NoseTrick/N_Kickflip/End/Out/Out3`.
    pub(crate) const OUT_3_1097: u16 = 1097;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/GrindOutAssist/Takeoff`.
    pub(crate) const TAKEOFF_1098: u16 = 1098;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/GrindOutAssist/LeftGround`.
    pub(crate) const LEFT_GROUND_1099: u16 = 1099;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/TakeOff/FromManual`.
    pub(crate) const FROM_MANUAL_1100: u16 = 1100;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/TakeOff/FromGrind`.
    pub(crate) const FROM_GRIND_1101: u16 = 1101;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/In/TakeOff/FromAntic`.
    pub(crate) const FROM_ANTIC_1102: u16 = 1102;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/End/Out/Out1`.
    pub(crate) const OUT_1_1103: u16 = 1103;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/End/Out/Out2`.
    pub(crate) const OUT_2_1104: u16 = 1104;
    /// `OnBoard/Trick/NoseTrick/N_Heelflip/End/Out/Out3`.
    pub(crate) const OUT_3_1105: u16 = 1105;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1106: u16 = 1106;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/Dismount/Out/FullOut`.
    pub(crate) const FULL_OUT_1107: u16 = 1107;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Into`.
    pub(crate) const INTO_1108: u16 = 1108;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Cyc`.
    pub(crate) const CYC_1109: u16 = 1109;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Out`.
    pub(crate) const OUT_1110: u16 = 1110;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Footplant`.
    pub(crate) const FOOTPLANT_1111: u16 = 1111;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Into`.
    pub(crate) const INTO_1112: u16 = 1112;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Cyc`.
    pub(crate) const CYC_1113: u16 = 1113;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Out`.
    pub(crate) const OUT_1114: u16 = 1114;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Footplant`.
    pub(crate) const FOOTPLANT_1115: u16 = 1115;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Into`.
    pub(crate) const INTO_1116: u16 = 1116;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Cyc`.
    pub(crate) const CYC_1117: u16 = 1117;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Out`.
    pub(crate) const OUT_1118: u16 = 1118;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Footplant`.
    pub(crate) const FOOTPLANT_1119: u16 = 1119;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Into`.
    pub(crate) const INTO_1120: u16 = 1120;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Cyc`.
    pub(crate) const CYC_1121: u16 = 1121;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Out`.
    pub(crate) const OUT_1122: u16 = 1122;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Footplant`.
    pub(crate) const FOOTPLANT_1123: u16 = 1123;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1124: u16 = 1124;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/Dismount/Out/FullOut`.
    pub(crate) const FULL_OUT_1125: u16 = 1125;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Into`.
    pub(crate) const INTO_1126: u16 = 1126;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Cyc`.
    pub(crate) const CYC_1127: u16 = 1127;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Out`.
    pub(crate) const OUT_1128: u16 = 1128;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Footplant`.
    pub(crate) const FOOTPLANT_1129: u16 = 1129;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Into`.
    pub(crate) const INTO_1130: u16 = 1130;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Cyc`.
    pub(crate) const CYC_1131: u16 = 1131;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Out`.
    pub(crate) const OUT_1132: u16 = 1132;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Footplant`.
    pub(crate) const FOOTPLANT_1133: u16 = 1133;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Into`.
    pub(crate) const INTO_1134: u16 = 1134;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Cyc`.
    pub(crate) const CYC_1135: u16 = 1135;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Out`.
    pub(crate) const OUT_1136: u16 = 1136;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Footplant`.
    pub(crate) const FOOTPLANT_1137: u16 = 1137;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Into`.
    pub(crate) const INTO_1138: u16 = 1138;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Cyc`.
    pub(crate) const CYC_1139: u16 = 1139;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Out`.
    pub(crate) const OUT_1140: u16 = 1140;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Footplant`.
    pub(crate) const FOOTPLANT_1141: u16 = 1141;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/Into`.
    pub(crate) const INTO_1142: u16 = 1142;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/Cycle`.
    pub(crate) const CYCLE_1143: u16 = 1143;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/NoFootAirWalk`.
    pub(crate) const NO_FOOT_AIR_WALK_1144: u16 = 1144;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir`.
    pub(crate) const ONE_FOOT_AIR_1145: u16 = 1145;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/FingerFlip`.
    pub(crate) const FINGER_FLIP_1146: u16 = 1146;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/SeatbeltGrab`.
    pub(crate) const SEATBELT_GRAB_1147: u16 = 1147;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/Into`.
    pub(crate) const INTO_1148: u16 = 1148;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/Cycle`.
    pub(crate) const CYCLE_1149: u16 = 1149;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/NoFootAirWalk`.
    pub(crate) const NO_FOOT_AIR_WALK_1150: u16 = 1150;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir`.
    pub(crate) const ONE_FOOT_AIR_1151: u16 = 1151;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/FingerFlip`.
    pub(crate) const FINGER_FLIP_1152: u16 = 1152;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/CrailGrab`.
    pub(crate) const CRAIL_GRAB_1153: u16 = 1153;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/ShimmyingLeft/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_1154: u16 = 1154;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/ShimmyingLeft/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_1155: u16 = 1155;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/ShimmyingRight/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_1156: u16 = 1156;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/ShimmyingRight/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_1157: u16 = 1157;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/Idle/SkitchKickTurn`.
    pub(crate) const SKITCH_KICK_TURN_1158: u16 = 1158;
    /// `OnBoard/OnGround/Skitching/Skitch2Hand/Holding/Idle/SkitchCarve`.
    pub(crate) const SKITCH_CARVE_1159: u16 = 1159;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Moving/Into`.
    pub(crate) const INTO_1160: u16 = 1160;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Moving/Cyc`.
    pub(crate) const CYC_1161: u16 = 1161;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Moving/Out`.
    pub(crate) const OUT_1162: u16 = 1162;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Stationary/Into`.
    pub(crate) const INTO_1163: u16 = 1163;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Stationary/FromMoving`.
    pub(crate) const FROM_MOVING_1164: u16 = 1164;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Stationary/Cyc`.
    pub(crate) const CYC_1165: u16 = 1165;
    /// `OnBoard/OnGround/RidingIdle/Riding/Brake/Stationary/Out`.
    pub(crate) const OUT_1166: u16 = 1166;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide`.
    pub(crate) const LEFT_SLIDE_1167: u16 = 1167;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide`.
    pub(crate) const RIGHT_SLIDE_1168: u16 = 1168;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Idle/AggressiveOrLoose`.
    pub(crate) const AGGRESSIVE_OR_LOOSE_1169: u16 = 1169;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Idle/Default`.
    pub(crate) const DEFAULT_1170: u16 = 1170;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Idle/Still`.
    pub(crate) const STILL_1171: u16 = 1171;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Idle/Moving`.
    pub(crate) const MOVING_1172: u16 = 1172;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab`.
    pub(crate) const BS_GRAB_1173: u16 = 1173;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab`.
    pub(crate) const FS_GRAB_1174: u16 = 1174;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab`.
    pub(crate) const DBL_GRAB_1175: u16 = 1175;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual`.
    pub(crate) const TAIL_MANUAL_1176: u16 = 1176;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual`.
    pub(crate) const NOSE_MANUAL_1177: u16 = 1177;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushFromFakie`.
    pub(crate) const PUSH_FROM_FAKIE_1178: u16 = 1178;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart`.
    pub(crate) const PUSH_START_1179: u16 = 1179;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushContact`.
    pub(crate) const PUSH_CONTACT_1180: u16 = 1180;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushCycle`.
    pub(crate) const PUSH_CYCLE_1181: u16 = 1181;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushEnd`.
    pub(crate) const PUSH_END_1182: u16 = 1182;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/AnticOut/AnticTail`.
    pub(crate) const ANTIC_TAIL_1183: u16 = 1183;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/AnticOut/AnticNose`.
    pub(crate) const ANTIC_NOSE_1184: u16 = 1184;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticInto`.
    pub(crate) const ANTIC_INTO_1185: u16 = 1185;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc`.
    pub(crate) const ANTIC_CYC_1186: u16 = 1186;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Switch/NoPush`.
    pub(crate) const NO_PUSH_1187: u16 = 1187;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Switch/PushQueued`.
    pub(crate) const PUSH_QUEUED_1188: u16 = 1188;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1189: u16 = 1189;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1190: u16 = 1190;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Footplant/Into`.
    pub(crate) const INTO_1191: u16 = 1191;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1192: u16 = 1192;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSRightFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1193: u16 = 1193;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1194: u16 = 1194;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1195: u16 = 1195;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Footplant/Into`.
    pub(crate) const INTO_1196: u16 = 1196;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1197: u16 = 1197;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/FSLeftFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1198: u16 = 1198;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1199: u16 = 1199;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1200: u16 = 1200;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Footplant/Into`.
    pub(crate) const INTO_1201: u16 = 1201;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1202: u16 = 1202;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSRightFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1203: u16 = 1203;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1204: u16 = 1204;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1205: u16 = 1205;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Footplant/Into`.
    pub(crate) const INTO_1206: u16 = 1206;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1207: u16 = 1207;
    /// `OnBoard/InAir/GrabsTweaks/FSGrab/OneFootAir/BSLeftFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1208: u16 = 1208;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1209: u16 = 1209;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1210: u16 = 1210;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Footplant/Into`.
    pub(crate) const INTO_1211: u16 = 1211;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1212: u16 = 1212;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSRightFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1213: u16 = 1213;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1214: u16 = 1214;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1215: u16 = 1215;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Footplant/Into`.
    pub(crate) const INTO_1216: u16 = 1216;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1217: u16 = 1217;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/FSLeftFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1218: u16 = 1218;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1219: u16 = 1219;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1220: u16 = 1220;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Footplant/Into`.
    pub(crate) const INTO_1221: u16 = 1221;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1222: u16 = 1222;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSRightFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1223: u16 = 1223;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1224: u16 = 1224;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1225: u16 = 1225;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Footplant/Into`.
    pub(crate) const INTO_1226: u16 = 1226;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Footplant/Absorb`.
    pub(crate) const ABSORB_1227: u16 = 1227;
    /// `OnBoard/InAir/GrabsTweaks/BSGrab/OneFootAir/BSLeftFoot/Footplant/JumpOut`.
    pub(crate) const JUMP_OUT_1228: u16 = 1228;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/NoFootAirWalk/Into`.
    pub(crate) const INTO_1229: u16 = 1229;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/NoFootAirWalk/Cyc`.
    pub(crate) const CYC_1230: u16 = 1230;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/NoFootAirWalk/Out`.
    pub(crate) const OUT_1231: u16 = 1231;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailLeftFoot`.
    pub(crate) const TAIL_LEFT_FOOT_1232: u16 = 1232;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailRightFoot`.
    pub(crate) const TAIL_RIGHT_FOOT_1233: u16 = 1233;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/FingerFlip/Into`.
    pub(crate) const INTO_1234: u16 = 1234;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/FingerFlip/GrabOut`.
    pub(crate) const GRAB_OUT_1235: u16 = 1235;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/SeatbeltGrab/Into`.
    pub(crate) const INTO_1236: u16 = 1236;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/SeatbeltGrab/Cycle`.
    pub(crate) const CYCLE_1237: u16 = 1237;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/SeatbeltGrab/Out`.
    pub(crate) const OUT_1238: u16 = 1238;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/SeatbeltGrab/SeatbeltFingerFlipShuv`.
    pub(crate) const SEATBELT_FINGER_FLIP_SHUV_1239: u16 = 1239;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/NoFootAirWalk/Into`.
    pub(crate) const INTO_1240: u16 = 1240;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/NoFootAirWalk/Cyc`.
    pub(crate) const CYC_1241: u16 = 1241;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/NoFootAirWalk/Out`.
    pub(crate) const OUT_1242: u16 = 1242;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseLeftFoot`.
    pub(crate) const NOSE_LEFT_FOOT_1243: u16 = 1243;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseRightFoot`.
    pub(crate) const NOSE_RIGHT_FOOT_1244: u16 = 1244;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/FingerFlip/Into`.
    pub(crate) const INTO_1245: u16 = 1245;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/FingerFlip/GrabOut`.
    pub(crate) const GRAB_OUT_1246: u16 = 1246;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/CrailGrab/Into`.
    pub(crate) const INTO_1247: u16 = 1247;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/CrailGrab/Cycle`.
    pub(crate) const CYCLE_1248: u16 = 1248;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/CrailGrab/Out`.
    pub(crate) const OUT_1249: u16 = 1249;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/CrailGrab/CrailFingerFlipShuv`.
    pub(crate) const CRAIL_FINGER_FLIP_SHUV_1250: u16 = 1250;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Into`.
    pub(crate) const INTO_1251: u16 = 1251;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Out`.
    pub(crate) const OUT_1252: u16 = 1252;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Into`.
    pub(crate) const INTO_1253: u16 = 1253;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Out`.
    pub(crate) const OUT_1254: u16 = 1254;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabInto`.
    pub(crate) const BS_GRAB_INTO_1255: u16 = 1255;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabCyc`.
    pub(crate) const BS_GRAB_CYC_1256: u16 = 1256;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabOut`.
    pub(crate) const BS_GRAB_OUT_1257: u16 = 1257;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabToFSGrab`.
    pub(crate) const BS_GRAB_TO_FS_GRAB_1258: u16 = 1258;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabToDBLGrab`.
    pub(crate) const BS_GRAB_TO_DBL_GRAB_1259: u16 = 1259;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabInto`.
    pub(crate) const FS_GRAB_INTO_1260: u16 = 1260;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabCyc`.
    pub(crate) const FS_GRAB_CYC_1261: u16 = 1261;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabOut`.
    pub(crate) const FS_GRAB_OUT_1262: u16 = 1262;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabToBSGrab`.
    pub(crate) const FS_GRAB_TO_BS_GRAB_1263: u16 = 1263;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabToDBLGrab`.
    pub(crate) const FS_GRAB_TO_DBL_GRAB_1264: u16 = 1264;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabInto`.
    pub(crate) const DBL_GRAB_INTO_1265: u16 = 1265;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabCyc`.
    pub(crate) const DBL_GRAB_CYC_1266: u16 = 1266;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabOut`.
    pub(crate) const DBL_GRAB_OUT_1267: u16 = 1267;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabToBSGrab`.
    pub(crate) const DBL_GRAB_TO_BS_GRAB_1268: u16 = 1268;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabToFSGrab`.
    pub(crate) const DBL_GRAB_TO_FS_GRAB_1269: u16 = 1269;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/Cycle`.
    pub(crate) const CYCLE_1270: u16 = 1270;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/Brake`.
    pub(crate) const BRAKE_1271: u16 = 1271;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/TailManualRevert`.
    pub(crate) const TAIL_MANUAL_REVERT_1272: u16 = 1272;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding`.
    pub(crate) const HOLDING_1273: u16 = 1273;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Out`.
    pub(crate) const OUT_1274: u16 = 1274;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/NoseManualRevert`.
    pub(crate) const NOSE_MANUAL_REVERT_1275: u16 = 1275;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushFromFakie/Right`.
    pub(crate) const RIGHT_1276: u16 = 1276;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushFromFakie/Left`.
    pub(crate) const LEFT_1277: u16 = 1277;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart/Right`.
    pub(crate) const RIGHT_1278: u16 = 1278;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart/Left`.
    pub(crate) const LEFT_1279: u16 = 1279;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushContact/Right`.
    pub(crate) const RIGHT_1280: u16 = 1280;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushContact/Left`.
    pub(crate) const LEFT_1281: u16 = 1281;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushCycle/Right`.
    pub(crate) const RIGHT_1282: u16 = 1282;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushCycle/Left`.
    pub(crate) const LEFT_1283: u16 = 1283;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushEnd/Right`.
    pub(crate) const RIGHT_1284: u16 = 1284;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushEnd/Left`.
    pub(crate) const LEFT_1285: u16 = 1285;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticInto/AnticTail`.
    pub(crate) const ANTIC_TAIL_1286: u16 = 1286;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticInto/AnticNose`.
    pub(crate) const ANTIC_NOSE_1287: u16 = 1287;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc/OllieAnticCycle`.
    pub(crate) const OLLIE_ANTIC_CYCLE_1288: u16 = 1288;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc/360PopShuvitAnticCycle`.
    pub(crate) const STATE_360_POP_SHUVIT_ANTIC_CYCLE_1289: u16 = 1289;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc/FS360PopShuvitAnticCycle`.
    pub(crate) const FS_360_POP_SHUVIT_ANTIC_CYCLE_1290: u16 = 1290;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc/NollieAnticCycle`.
    pub(crate) const NOLLIE_ANTIC_CYCLE_1291: u16 = 1291;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc/Nollie360PopShuvitAnticCycle`.
    pub(crate) const NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1292: u16 = 1292;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Anticipation/AnticCyc/FSNollie360PopShuvitAnticCycle`.
    pub(crate) const FS_NOLLIE_360_POP_SHUVIT_ANTIC_CYCLE_1293: u16 = 1293;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Switch/PushQueued/Right`.
    pub(crate) const RIGHT_1294: u16 = 1294;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Switch/PushQueued/Left`.
    pub(crate) const LEFT_1295: u16 = 1295;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailLeftFoot/Into`.
    pub(crate) const INTO_1296: u16 = 1296;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailLeftFoot/Cyc`.
    pub(crate) const CYC_1297: u16 = 1297;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailLeftFoot/Out`.
    pub(crate) const OUT_1298: u16 = 1298;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailRightFoot/Into`.
    pub(crate) const INTO_1299: u16 = 1299;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailRightFoot/Cyc`.
    pub(crate) const CYC_1300: u16 = 1300;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailRightFoot/Out`.
    pub(crate) const OUT_1301: u16 = 1301;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseLeftFoot/Into`.
    pub(crate) const INTO_1302: u16 = 1302;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseLeftFoot/Cyc`.
    pub(crate) const CYC_1303: u16 = 1303;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseLeftFoot/Out`.
    pub(crate) const OUT_1304: u16 = 1304;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseRightFoot/Into`.
    pub(crate) const INTO_1305: u16 = 1305;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseRightFoot/Cyc`.
    pub(crate) const CYC_1306: u16 = 1306;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseRightFoot/Out`.
    pub(crate) const OUT_1307: u16 = 1307;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Into/Into`.
    pub(crate) const INTO_1308: u16 = 1308;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Into/Cyc`.
    pub(crate) const CYC_1309: u16 = 1309;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Out/Early`.
    pub(crate) const EARLY_1310: u16 = 1310;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Out/Mid`.
    pub(crate) const MID_1311: u16 = 1311;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Left/LeftSlide/Out/Default`.
    pub(crate) const DEFAULT_1312: u16 = 1312;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Into/Into`.
    pub(crate) const INTO_1313: u16 = 1313;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Into/Cyc`.
    pub(crate) const CYC_1314: u16 = 1314;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Out/Early`.
    pub(crate) const EARLY_1315: u16 = 1315;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Out/Mid`.
    pub(crate) const MID_1316: u16 = 1316;
    /// `OnBoard/OnGround/RidingIdle/Riding/Sliding/Right/RightSlide/Out/Default`.
    pub(crate) const DEFAULT_1317: u16 = 1317;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabCyc/Idle`.
    pub(crate) const IDLE_1318: u16 = 1318;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabCyc/LeftSlide`.
    pub(crate) const LEFT_SLIDE_1319: u16 = 1319;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/BSGrab/BSGrabCyc/RightSlide`.
    pub(crate) const RIGHT_SLIDE_1320: u16 = 1320;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabCyc/Idle`.
    pub(crate) const IDLE_1321: u16 = 1321;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabCyc/LeftSlide`.
    pub(crate) const LEFT_SLIDE_1322: u16 = 1322;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/FSGrab/FSGrabCyc/RightSlide`.
    pub(crate) const RIGHT_SLIDE_1323: u16 = 1323;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabCyc/Idle`.
    pub(crate) const IDLE_1324: u16 = 1324;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabCyc/LeftSlide`.
    pub(crate) const LEFT_SLIDE_1325: u16 = 1325;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Grabbing/DBLGrab/DBLGrabCyc/RightSlide`.
    pub(crate) const RIGHT_SLIDE_1326: u16 = 1326;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/Cycle/RollingBackwards`.
    pub(crate) const ROLLING_BACKWARDS_1327: u16 = 1327;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/Cycle/RollingForwardsOrStationary`.
    pub(crate) const ROLLING_FORWARDS_OR_STATIONARY_1328: u16 = 1328;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/Brake/Moving`.
    pub(crate) const MOVING_1329: u16 = 1329;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/Brake/Stationary`.
    pub(crate) const STATIONARY_1330: u16 = 1330;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/TailManualRevert/Bs`.
    pub(crate) const BS_1331: u16 = 1331;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/TailManual/TailManualRevert/Fs`.
    pub(crate) const FS_1332: u16 = 1332;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Into`.
    pub(crate) const INTO_1333: u16 = 1333;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Cycle`.
    pub(crate) const CYCLE_1334: u16 = 1334;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Brake`.
    pub(crate) const BRAKE_1335: u16 = 1335;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/NoseManualRevert/Bs`.
    pub(crate) const BS_1336: u16 = 1336;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/NoseManualRevert/Fs`.
    pub(crate) const FS_1337: u16 = 1337;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart/Right/Default`.
    pub(crate) const DEFAULT_1338: u16 = 1338;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart/Right/FromFakie`.
    pub(crate) const FROM_FAKIE_1339: u16 = 1339;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart/Left/Default`.
    pub(crate) const DEFAULT_1340: u16 = 1340;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Push/PushStart/Left/FromFakie`.
    pub(crate) const FROM_FAKIE_1341: u16 = 1341;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailLeftFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1342: u16 = 1342;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailLeftFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1343: u16 = 1343;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailRightFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1344: u16 = 1344;
    /// `OnBoard/InAir/BoardAdjusts/Up/Grab/Grabbing/OneFootAir/TailRightFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1345: u16 = 1345;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseLeftFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1346: u16 = 1346;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseLeftFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1347: u16 = 1347;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseRightFoot/Out/EarlyOut`.
    pub(crate) const EARLY_OUT_1348: u16 = 1348;
    /// `OnBoard/InAir/BoardAdjusts/Down/Grab/Grabbing/OneFootAir/NoseRightFoot/Out/FullOut`.
    pub(crate) const FULL_OUT_1349: u16 = 1349;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Cycle/RollingForwards`.
    pub(crate) const ROLLING_FORWARDS_1350: u16 = 1350;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Cycle/RollingBackwardsOrStationary`.
    pub(crate) const ROLLING_BACKWARDS_OR_STATIONARY_1351: u16 = 1351;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Brake/Moving`.
    pub(crate) const MOVING_1352: u16 = 1352;
    /// `OnBoard/OnGround/RidingIdle/Riding/Turning/Manual/NoseManual/Holding/Brake/Stationary`.
    pub(crate) const STATIONARY_1353: u16 = 1353;
    /// `OffBoard/OBGround/Locomotion/StandingJump/Antic`.
    pub(crate) const ANTIC_1354: u16 = 1354;
    /// `OffBoard/OBGround/Locomotion/StandingJump/Takeoff`.
    pub(crate) const TAKEOFF_1355: u16 = 1355;
    /// `OffBoard/OBGround/Locomotion/StandingJump/Descent`.
    pub(crate) const DESCENT_1356: u16 = 1356;
    /// `OffBoard/OBGround/Locomotion/StandingJump/Land`.
    pub(crate) const LAND_1357: u16 = 1357;
    /// `OffBoard/OBGround/Locomotion/StandingJump/JumpingIntoMount`.
    pub(crate) const JUMPING_INTO_MOUNT_1358: u16 = 1358;
    /// `OffBoard/OBGround/Locomotion/StandingJump/IntoBSGrabOut`.
    pub(crate) const INTO_BS_GRAB_OUT_1359: u16 = 1359;
}
