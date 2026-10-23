//! Semantic declarations recovered from `ActionGraph_OnBoard.cpp` native tables.
//! Original addresses identify evidence; no guest code or memory is executed.
use super::{
    Attribute, BehaviourDeclaration, ConditionDeclaration, Section, StateDeclaration,
    TransitionDeclaration,
};
pub(super) const ATTRIBUTES: [Attribute; 457] = [
    Attribute {
        key: "AGIntent",
        value: "GestureDown",
    },
    Attribute {
        key: "MGIntent",
        value: "GestureDown",
    },
    Attribute {
        key: "AGIntent",
        value: "GestureLeft",
    },
    Attribute {
        key: "MGIntent",
        value: "GestureLeft",
    },
    Attribute {
        key: "AGIntent",
        value: "GestureRight",
    },
    Attribute {
        key: "MGIntent",
        value: "GestureRight",
    },
    Attribute {
        key: "AGIntent",
        value: "GestureUp",
    },
    Attribute {
        key: "MGIntent",
        value: "GestureUp",
    },
    Attribute {
        key: "AGIntent",
        value: "NoInput",
    },
    Attribute {
        key: "MGIntent",
        value: "NoInput",
    },
    Attribute {
        key: "AGIntent",
        value: "GrabWorld",
    },
    Attribute {
        key: "MGIntent",
        value: "GrabWorld",
    },
    Attribute {
        key: "AGIntent",
        value: "PhysGrindTranslation",
    },
    Attribute {
        key: "MGIntent",
        value: "PhysGrindTranslation",
    },
    Attribute {
        key: "AGIntent",
        value: "PhysGrindStabilityNudge",
    },
    Attribute {
        key: "MGIntent",
        value: "PhysGrindStabilityNudge",
    },
    Attribute {
        key: "AGIntent",
        value: "PhysGrindUpDown",
    },
    Attribute {
        key: "MGIntent",
        value: "PhysGrindUpDown",
    },
    Attribute {
        key: "AGIntent",
        value: "PhysBodySpin",
    },
    Attribute {
        key: "MGIntent",
        value: "PhysBodySpin",
    },
    Attribute {
        key: "MGIntent",
        value: "Brake",
    },
    Attribute {
        key: "value",
        value: "1.0",
    },
    Attribute {
        key: "MGIntent",
        value: "ForceBrake",
    },
    Attribute {
        key: "value",
        value: "1.0",
    },
    Attribute {
        key: "AGIntent",
        value: "ExitGrind",
    },
    Attribute {
        key: "MGIntent",
        value: "ExitGrind",
    },
    Attribute {
        key: "AGIntent",
        value: "WipeOutRecover",
    },
    Attribute {
        key: "MGIntent",
        value: "WipeOutRecover",
    },
    Attribute {
        key: "AGIntent",
        value: "WipeOutRequest",
    },
    Attribute {
        key: "MGIntent",
        value: "WipeOutRequest",
    },
    Attribute {
        key: "AGIntent",
        value: "Manual",
    },
    Attribute {
        key: "MGIntent",
        value: "Manual",
    },
    Attribute {
        key: "AGIntent",
        value: "ManualBrake",
    },
    Attribute {
        key: "MGIntent",
        value: "ManualBrake",
    },
    Attribute {
        key: "AGIntent",
        value: "Manual",
    },
    Attribute {
        key: "MGIntent",
        value: "ManualEngageTime",
    },
    Attribute {
        key: "AGIntent",
        value: "BodySpin",
    },
    Attribute {
        key: "MGIntent",
        value: "BodySpin",
    },
    Attribute {
        key: "AGIntent",
        value: "Crouch",
    },
    Attribute {
        key: "MGIntent",
        value: "Crouch",
    },
    Attribute {
        key: "AGIntent",
        value: "GrindBalanceX",
    },
    Attribute {
        key: "MGIntent",
        value: "GrindBalanceX",
    },
    Attribute {
        key: "mirrorFilter",
        value: "negate",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftGroundGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "WantsLeftGrindGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "RightGroundGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "WantsRightGrindGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "GrindFSGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "GrindBSGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "GrindDoubleGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftAirGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "WantsLeftAirGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "RightAirGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "WantsRightAirGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "HandPlantTweakX",
    },
    Attribute {
        key: "MGIntent",
        value: "HandPlantTweakX",
    },
    Attribute {
        key: "AGIntent",
        value: "HandPlantTweakY",
    },
    Attribute {
        key: "MGIntent",
        value: "HandPlantTweakY",
    },
    Attribute {
        key: "AGIntent",
        value: "HandPlantDismount",
    },
    Attribute {
        key: "MGIntent",
        value: "HandPlantDismount",
    },
    Attribute {
        key: "AGIntent",
        value: "HandPlantOneFootRight",
    },
    Attribute {
        key: "MGIntent",
        value: "HandPlantOneFootRight",
    },
    Attribute {
        key: "AGIntent",
        value: "HandPlantOneFootLeft",
    },
    Attribute {
        key: "MGIntent",
        value: "HandPlantOneFootLeft",
    },
    Attribute {
        key: "AGIntent",
        value: "TweakX",
    },
    Attribute {
        key: "defaultValue",
        value: "0.0",
    },
    Attribute {
        key: "MGIntent",
        value: "TweakX",
    },
    Attribute {
        key: "mirrorFilter",
        value: "negate",
    },
    Attribute {
        key: "AGIntent",
        value: "TweakY",
    },
    Attribute {
        key: "defaultValue",
        value: "0.0",
    },
    Attribute {
        key: "MGIntent",
        value: "TweakY",
    },
    Attribute {
        key: "MGIntent",
        value: "DBLGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "Grab",
    },
    Attribute {
        key: "MGIntent",
        value: "Superman",
    },
    Attribute {
        key: "MGIntent",
        value: "FSGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "FSChrist",
    },
    Attribute {
        key: "MGIntent",
        value: "RightOneFootAir",
    },
    Attribute {
        key: "MGIntent",
        value: "LeftOneFootAir",
    },
    Attribute {
        key: "MGIntent",
        value: "FingerFlip",
    },
    Attribute {
        key: "MGIntent",
        value: "FingerFlipShuv",
    },
    Attribute {
        key: "MGIntent",
        value: "FingerFlipVar",
    },
    Attribute {
        key: "MGIntent",
        value: "BSGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "BSChrist",
    },
    Attribute {
        key: "MGIntent",
        value: "MuteGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "StaleGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "BoardAdjustUp",
    },
    Attribute {
        key: "angleFilter",
        value: "angleFlip",
    },
    Attribute {
        key: "MGIntentAngle",
        value: "UpBoardAdjustAngle",
    },
    Attribute {
        key: "MGIntentMag",
        value: "UpBoardAdjustMag",
    },
    Attribute {
        key: "negateOnMirror",
        value: "true",
    },
    Attribute {
        key: "MGIntent",
        value: "TailGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "NoFootAirWalk",
    },
    Attribute {
        key: "MGIntent",
        value: "SeatbeltGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "BoardAdjustDown",
    },
    Attribute {
        key: "MGIntentAngle",
        value: "DownBoardAdjustAngle",
    },
    Attribute {
        key: "MGIntentMag",
        value: "DownBoardAdjustMag",
    },
    Attribute {
        key: "negateOnMirror",
        value: "true",
    },
    Attribute {
        key: "MGIntent",
        value: "NoseGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "CrailGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "RocketAir",
    },
    Attribute {
        key: "MGIntent",
        value: "BoardAdjustLeft",
    },
    Attribute {
        key: "angleFilter",
        value: "angleRotN90",
    },
    Attribute {
        key: "MGIntentAngle",
        value: "LeftBoardAdjustAngle",
    },
    Attribute {
        key: "MGIntentMag",
        value: "LeftBoardAdjustMag",
    },
    Attribute {
        key: "negateOnMirror",
        value: "false",
    },
    Attribute {
        key: "MGIntent",
        value: "BoardAdjustRight",
    },
    Attribute {
        key: "angleFilter",
        value: "angleRotN90",
    },
    Attribute {
        key: "MGIntentAngle",
        value: "RightBoardAdjustAngle",
    },
    Attribute {
        key: "MGIntentMag",
        value: "RightBoardAdjustMag",
    },
    Attribute {
        key: "angleFilter",
        value: "angleRot90",
    },
    Attribute {
        key: "MGIntentAngle",
        value: "RightBoardAdjustAngle",
    },
    Attribute {
        key: "MGIntentMag",
        value: "RightBoardAdjustMag",
    },
    Attribute {
        key: "angleFilter",
        value: "angleRot90",
    },
    Attribute {
        key: "MGIntentAngle",
        value: "LeftBoardAdjustAngle",
    },
    Attribute {
        key: "MGIntentMag",
        value: "LeftBoardAdjustMag",
    },
    Attribute {
        key: "negateOnMirror",
        value: "false",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Dismount",
    },
    Attribute {
        key: "AGIntent",
        value: "Turn",
    },
    Attribute {
        key: "MGIntent",
        value: "Turn",
    },
    Attribute {
        key: "AGIntent",
        value: "Turn",
    },
    Attribute {
        key: "fakieFilter",
        value: "negate",
    },
    Attribute {
        key: "MGIntent",
        value: "FakieTurn",
    },
    Attribute {
        key: "MGIntent",
        value: "Coffin",
    },
    Attribute {
        key: "AGIntent",
        value: "RightSlideStart",
    },
    Attribute {
        key: "MGIntent",
        value: "RightSlideStart",
    },
    Attribute {
        key: "AGIntent",
        value: "RightSlide",
    },
    Attribute {
        key: "MGIntent",
        value: "RightSlide",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftSlideStart",
    },
    Attribute {
        key: "MGIntent",
        value: "LeftSlideStart",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftSlide",
    },
    Attribute {
        key: "MGIntent",
        value: "LeftSlide",
    },
    Attribute {
        key: "AGIntent",
        value: "HardTurnCrouch",
    },
    Attribute {
        key: "MGIntent",
        value: "HardTurnCrouch",
    },
    Attribute {
        key: "AGIntent",
        value: "SlideFs180",
    },
    Attribute {
        key: "MGIntent",
        value: "SlideFs180",
    },
    Attribute {
        key: "AGIntent",
        value: "SlideBs180",
    },
    Attribute {
        key: "MGIntent",
        value: "SlideBs180",
    },
    Attribute {
        key: "AGIntent",
        value: "AutoPumpAngle",
    },
    Attribute {
        key: "MGIntent",
        value: "AutoPumpAngle",
    },
    Attribute {
        key: "AGIntent",
        value: "AutoPumpMag",
    },
    Attribute {
        key: "MGIntent",
        value: "AutoPumpMag",
    },
    Attribute {
        key: "MGIntent",
        value: "GroundDBLGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "NewPush",
    },
    Attribute {
        key: "MGIntent",
        value: "NewFootPlant",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "MGIntent",
        value: "LeftFootPlant",
    },
    Attribute {
        key: "AGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "MGIntent",
        value: "RightFootPlant",
    },
    Attribute {
        key: "AGIntent",
        value: "NewPush",
    },
    Attribute {
        key: "MGIntent",
        value: "Trick",
    },
    Attribute {
        key: "MGIntent",
        value: "GroundFSGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "MGIntent",
        value: "FSBoneless",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "MGIntent",
        value: "FSFastPlant",
    },
    Attribute {
        key: "MGIntent",
        value: "GroundBSGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "MGIntent",
        value: "BSBoneless",
    },
    Attribute {
        key: "AGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "MGIntent",
        value: "BSFastPlant",
    },
    Attribute {
        key: "AGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "MGIntent",
        value: "BSBoneless",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "MGIntent",
        value: "BSFastPlant",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "MGIntent",
        value: "FSBoneless",
    },
    Attribute {
        key: "AGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "MGIntent",
        value: "FSFastPlant",
    },
    Attribute {
        key: "AGIntent",
        value: "AnticMag",
    },
    Attribute {
        key: "MGIntent",
        value: "Compression",
    },
    Attribute {
        key: "MGIntent",
        value: "OllieAnticipation",
    },
    Attribute {
        key: "AGIntent",
        value: "AnticMag",
    },
    Attribute {
        key: "MGIntent",
        value: "AnticMag",
    },
    Attribute {
        key: "MGIntent",
        value: "PopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "FSPopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "360PopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "FS360PopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "NollieAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "N_PopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "N_FSPopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "N_360PopShuvitAnticipation",
    },
    Attribute {
        key: "MGIntent",
        value: "N_FS360PopShuvitAnticipation",
    },
    Attribute {
        key: "AGIntent",
        value: "Pushing",
    },
    Attribute {
        key: "MGIntent",
        value: "Pushing",
    },
    Attribute {
        key: "AGIntent",
        value: "NewPush",
    },
    Attribute {
        key: "MGIntent",
        value: "NewPush",
    },
    Attribute {
        key: "AGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "MGIntent",
        value: "RightPush",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "MGIntent",
        value: "LeftPush",
    },
    Attribute {
        key: "AGIntent",
        value: "Brake",
    },
    Attribute {
        key: "MGIntent",
        value: "Brake",
    },
    Attribute {
        key: "MGIntent",
        value: "NewKickTurn",
    },
    Attribute {
        key: "onUpdate",
        value: "false",
    },
    Attribute {
        key: "AGIntent",
        value: "KickTurn",
    },
    Attribute {
        key: "MGIntent",
        value: "KickTurn",
    },
    Attribute {
        key: "AGIntent",
        value: "HardTurn",
    },
    Attribute {
        key: "fakieFilter",
        value: "negate",
    },
    Attribute {
        key: "MGIntent",
        value: "FakieHardTurn",
    },
    Attribute {
        key: "AGIntent",
        value: "Trick",
    },
    Attribute {
        key: "MGIntent",
        value: "Trick",
    },
    Attribute {
        key: "AGIntent",
        value: "TrickHeight",
    },
    Attribute {
        key: "MGIntent",
        value: "TrickHeight",
    },
    Attribute {
        key: "AGIntent",
        value: "GestureSpeed",
    },
    Attribute {
        key: "MGIntent",
        value: "GestureSpeed",
    },
    Attribute {
        key: "group",
        value: "Square",
    },
    Attribute {
        key: "group",
        value: "Tail",
    },
    Attribute {
        key: "group",
        value: "Nose",
    },
    Attribute {
        key: "AGIntent",
        value: "L_F_Kickflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_F_Kickflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_F_Kickflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_F_Heelflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_F_Heelflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_F_Heelflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_F_Heelflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_F_Kickflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_B_Kickflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_B_Kickflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_B_Kickflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_B_Heelflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_B_Heelflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_B_Heelflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_B_Heelflip",
    },
    Attribute {
        key: "MGIntent",
        value: "L_B_Kickflip",
    },
    Attribute {
        key: "AGIntent",
        value: "L_FS_Shuvit",
    },
    Attribute {
        key: "MGIntent",
        value: "L_FS_Shuvit",
    },
    Attribute {
        key: "AGIntent",
        value: "L_FS_Shuvit",
    },
    Attribute {
        key: "MGIntent",
        value: "L_BS_Shuvit",
    },
    Attribute {
        key: "AGIntent",
        value: "L_BS_Shuvit",
    },
    Attribute {
        key: "MGIntent",
        value: "L_BS_Shuvit",
    },
    Attribute {
        key: "AGIntent",
        value: "L_BS_Shuvit",
    },
    Attribute {
        key: "MGIntent",
        value: "L_FS_Shuvit",
    },
    Attribute {
        key: "group",
        value: "90Tail",
    },
    Attribute {
        key: "group",
        value: "90Nose",
    },
    Attribute {
        key: "group",
        value: "N90Tail",
    },
    Attribute {
        key: "group",
        value: "N90Nose",
    },
    Attribute {
        key: "AGIntent",
        value: "WipeoutGestureX",
    },
    Attribute {
        key: "MGIntent",
        value: "WipeoutGestureX",
    },
    Attribute {
        key: "AGIntent",
        value: "WipeoutGestureY",
    },
    Attribute {
        key: "MGIntent",
        value: "WipeoutGestureY",
    },
    Attribute {
        key: "AGIntent",
        value: "WipeoutControlX",
    },
    Attribute {
        key: "MGIntent",
        value: "WipeoutControlX",
    },
    Attribute {
        key: "AGIntent",
        value: "WipeoutControlY",
    },
    Attribute {
        key: "MGIntent",
        value: "WipeoutControlY",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_SpawnDismount",
    },
    Attribute {
        key: "MGIntent",
        value: "OffBoard",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Mount",
    },
    Attribute {
        key: "AGIntent",
        value: "LeftAirGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "BSGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "RightAirGrab",
    },
    Attribute {
        key: "MGIntent",
        value: "BSGrab",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_Steer",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Steer",
    },
    Attribute {
        key: "AGIntent",
        value: "OBL_Steer",
    },
    Attribute {
        key: "MGIntent",
        value: "OBL_Steer",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_Mag",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_SteerMagnitude",
    },
    Attribute {
        key: "AGIntent",
        value: "OBL_Mag",
    },
    Attribute {
        key: "MGIntent",
        value: "OBL_SteerMagnitude",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_Sprint",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Sprint",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_MagHeld",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_SteerMagnitudeHeld",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_MOMag",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_MOMag",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_MOSteer",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_MOSteer",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_ObjectMvZ",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_ObjectMvZ",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_ObjectMvX",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_ObjectMvX",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_ObjectMvRot",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_ObjectMvRot",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_LookAtX",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_LookAtX",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_LookAtY",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_LookAtY",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_HoldBoard",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_HoldBoard",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_DropBoard",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_DropBoard",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_ThrowBoard",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_ThrowBoard",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_RetrieveBoard",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_RetrieveBoard",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkForward",
    },
    Attribute {
        key: "MGIntent",
        value: "StepForward",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkForwardRight",
    },
    Attribute {
        key: "MGIntent",
        value: "StepForwardRight",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkRight",
    },
    Attribute {
        key: "MGIntent",
        value: "StepRight",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkBackwardRight",
    },
    Attribute {
        key: "MGIntent",
        value: "StepBackRight",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkBackward",
    },
    Attribute {
        key: "MGIntent",
        value: "StepBackward",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkBackwardLeft",
    },
    Attribute {
        key: "MGIntent",
        value: "StepBackLeft",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkLeft",
    },
    Attribute {
        key: "MGIntent",
        value: "StepLeft",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_WalkForwardLeft",
    },
    Attribute {
        key: "MGIntent",
        value: "StepForwardLeft",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_Jump",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Jump",
    },
    Attribute {
        key: "AGIntent",
        value: "NewToggleOffBoardState",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Mount",
    },
    Attribute {
        key: "AGIntent",
        value: "ToggleOffBoardState",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Dismount",
    },
    Attribute {
        key: "MGIntent",
        value: "OffBoardAir",
    },
    Attribute {
        key: "AGIntent",
        value: "OB_Air",
    },
    Attribute {
        key: "MGIntent",
        value: "OB_Air",
    },
    Attribute {
        key: "numFrames",
        value: "100",
    },
    Attribute {
        key: "text",
        value: "WARNING: No Action Graph state has been resolved. We need to fix this.",
    },
    Attribute {
        key: "x",
        value: "200",
    },
    Attribute {
        key: "y",
        value: "200",
    },
    Attribute {
        key: "intent",
        value: "ForceBrake",
    },
    Attribute {
        key: "state",
        value: "teleport",
    },
    Attribute {
        key: "greater",
        value: "10.0",
    },
    Attribute {
        key: "greater",
        value: "5.0",
    },
    Attribute {
        key: "less",
        value: "0.5",
    },
    Attribute {
        key: "grindName",
        value: "FS_5_O",
    },
    Attribute {
        key: "grindName",
        value: "BS_5_O",
    },
    Attribute {
        key: "grindName",
        value: "FS_SALAD",
    },
    Attribute {
        key: "grindName",
        value: "BS_SALAD",
    },
    Attribute {
        key: "grindName",
        value: "FS_SMITH",
    },
    Attribute {
        key: "grindName",
        value: "BS_SMITH",
    },
    Attribute {
        key: "grindName",
        value: "FS_FEEBLE",
    },
    Attribute {
        key: "grindName",
        value: "BS_FEEBLE",
    },
    Attribute {
        key: "intent",
        value: "LeftGroundGrab",
    },
    Attribute {
        key: "intent",
        value: "RightGroundGrab",
    },
    Attribute {
        key: "grindName",
        value: "BS_TAILSLIDE",
    },
    Attribute {
        key: "grindName",
        value: "BS_BLUNT",
    },
    Attribute {
        key: "grindName",
        value: "FS_TAILSLIDE",
    },
    Attribute {
        key: "grindName",
        value: "FS_BLUNT",
    },
    Attribute {
        key: "grindName",
        value: "FS_NOSEGRIND",
    },
    Attribute {
        key: "grindName",
        value: "BS_NOSEGRIND",
    },
    Attribute {
        key: "grindName",
        value: "FS_OVERCROOK",
    },
    Attribute {
        key: "grindName",
        value: "BS_OVERCROOK",
    },
    Attribute {
        key: "grindName",
        value: "FS_CROOK",
    },
    Attribute {
        key: "grindName",
        value: "BS_CROOK",
    },
    Attribute {
        key: "grindName",
        value: "FS_OVERWILLY",
    },
    Attribute {
        key: "grindName",
        value: "BS_OVERWILLY",
    },
    Attribute {
        key: "grindName",
        value: "FS_WILLY",
    },
    Attribute {
        key: "grindName",
        value: "BS_WILLY",
    },
    Attribute {
        key: "grindName",
        value: "BS_NOSESLIDE",
    },
    Attribute {
        key: "grindName",
        value: "BS_NOSEBLUNT",
    },
    Attribute {
        key: "grindName",
        value: "FS_NOSESLIDE",
    },
    Attribute {
        key: "grindName",
        value: "FS_NOSEBLUNT",
    },
    Attribute {
        key: "state",
        value: "air",
    },
    Attribute {
        key: "intent",
        value: "LeftAirGrab",
    },
    Attribute {
        key: "intent",
        value: "RightAirGrab",
    },
    Attribute {
        key: "attribute",
        value: "NewDismount",
    },
    Attribute {
        key: "intent",
        value: "Dismount",
    },
    Attribute {
        key: "intent",
        value: "NewPush",
    },
    Attribute {
        key: "intent",
        value: "RightPush",
    },
    Attribute {
        key: "intent",
        value: "LeftPush",
    },
    Attribute {
        key: "intent",
        value: "L_F_Kickflip",
    },
    Attribute {
        key: "intent",
        value: "L_F_Heelflip",
    },
    Attribute {
        key: "intent",
        value: "L_B_Kickflip",
    },
    Attribute {
        key: "intent",
        value: "L_B_Heelflip",
    },
    Attribute {
        key: "intent",
        value: "FingerFlip",
    },
    Attribute {
        key: "intent",
        value: "FS_Varial",
    },
    Attribute {
        key: "intent",
        value: "BS_Varial",
    },
    Attribute {
        key: "intent",
        value: "FingerFlipShuv",
    },
    Attribute {
        key: "intent",
        value: "FingerFlipVar",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "greaterEqual",
        value: "2.355",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustAngle",
    },
    Attribute {
        key: "state",
        value: "Down.CrailGrab.FingerFlipShuv",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustMag",
    },
    Attribute {
        key: "intent",
        value: "L_BS_Shuvit",
    },
    Attribute {
        key: "intent",
        value: "L_FS_Shuvit",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "0.785",
    },
    Attribute {
        key: "state",
        value: "Up.SeatbeltGrab.FingerFlipShuv",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "-0.785",
    },
    Attribute {
        key: "greaterEqual",
        value: "-2.355",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustAngle",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "2.355",
    },
    Attribute {
        key: "greaterEqual",
        value: "0.785",
    },
    Attribute {
        key: "intent",
        value: "BoardAdjustAngle",
    },
    Attribute {
        key: "intent",
        value: "NewToggleOffBoardState",
    },
    Attribute {
        key: "greater",
        value: "0.3",
    },
    Attribute {
        key: "state",
        value: "offboardair",
    },
    Attribute {
        key: "state",
        value: "ground",
    },
    Attribute {
        key: "intent",
        value: "ToggleOffBoardState",
    },
    Attribute {
        key: "greater",
        value: "0.9",
    },
    Attribute {
        key: "intent",
        value: "AnticMag",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "0.52",
    },
    Attribute {
        key: "greaterEqual",
        value: "-1.05",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "-0.52",
    },
    Attribute {
        key: "greaterEqual",
        value: "0.52",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "1.05",
    },
    Attribute {
        key: "greaterEqual",
        value: "-1.57",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "-1.05",
    },
    Attribute {
        key: "greaterEqual",
        value: "1.05",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "1.57",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "greaterEqual",
        value: "2.62",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "greaterEqual",
        value: "-2.62",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "-2.09",
    },
    Attribute {
        key: "greaterEqual",
        value: "2.09",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "2.62",
    },
    Attribute {
        key: "greaterEqual",
        value: "-2.09",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "-1.57",
    },
    Attribute {
        key: "greaterEqual",
        value: "1.57",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "intent",
        value: "AnticAngle",
    },
    Attribute {
        key: "lessEqual",
        value: "2.09",
    },
    Attribute {
        key: "intent",
        value: "Brake",
    },
    Attribute {
        key: "less",
        value: "0.1",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "greater",
        value: "0.3",
    },
    Attribute {
        key: "intent",
        value: "KickTurn",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "less",
        value: "4.0",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "less",
        value: "4.6",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "less",
        value: "1.5",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "less",
        value: "2.1",
    },
    Attribute {
        key: "abs",
        value: "true",
    },
    Attribute {
        key: "greater",
        value: "0.0",
    },
    Attribute {
        key: "intent",
        value: "HardTurn",
    },
    Attribute {
        key: "intent",
        value: "Sliding",
    },
    Attribute {
        key: "intent",
        value: "Trick",
    },
    Attribute {
        key: "intent",
        value: "HoldPattern",
    },
    Attribute {
        key: "less",
        value: "0.4",
    },
    Attribute {
        key: "intent",
        value: "DontMirrorTrick",
    },
    Attribute {
        key: "state",
        value: "wipeout",
    },
    Attribute {
        key: "greater",
        value: "0.5",
    },
    Attribute {
        key: "attribute",
        value: "IdleDismounting",
    },
    Attribute {
        key: "state",
        value: "offboard",
    },
    Attribute {
        key: "attribute",
        value: "IdleMounting",
    },
    Attribute {
        key: "attribute",
        value: "OB_Mounting",
    },
    Attribute {
        key: "lessEqual",
        value: "0.03",
    },
    Attribute {
        key: "policy",
        value: "Priority",
    },
];
pub(super) const BEHAVIOURS: [BehaviourDeclaration; 311] = [
    BehaviourDeclaration {
        name: "JuiceHook",
        parent: 0,
        attributes: Section {
            count: 0,
            index: 65535,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section { count: 2, index: 0 },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section { count: 2, index: 2 },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section { count: 2, index: 4 },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section { count: 2, index: 6 },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section { count: 2, index: 8 },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section {
            count: 2,
            index: 10,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section {
            count: 2,
            index: 12,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section {
            count: 2,
            index: 14,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section {
            count: 2,
            index: 16,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 0,
        attributes: Section {
            count: 2,
            index: 18,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 251,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 253,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 255,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 257,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 259,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 261,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 263,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 265,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 267,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 269,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 271,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 273,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 275,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 277,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 279,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 281,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 283,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 285,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 287,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 289,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 291,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 293,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 295,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 297,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 299,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 26,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 2,
        attributes: Section {
            count: 2,
            index: 28,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "PrintText2D",
        parent: 3,
        attributes: Section {
            count: 4,
            index: 310,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 4,
        attributes: Section {
            count: 2,
            index: 20,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 4,
        attributes: Section {
            count: 2,
            index: 22,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 5,
        attributes: Section {
            count: 2,
            index: 20,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 24,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 26,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 28,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BodyFlippingSignal",
        parent: 6,
        attributes: Section {
            count: 0,
            index: 65535,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 30,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 32,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGTimeIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 34,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 6,
        attributes: Section {
            count: 2,
            index: 36,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 7,
        attributes: Section {
            count: 1,
            index: 244,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 7,
        attributes: Section {
            count: 1,
            index: 245,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 8,
        attributes: Section {
            count: 1,
            index: 116,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 8,
        attributes: Section {
            count: 1,
            index: 245,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 9,
        attributes: Section {
            count: 1,
            index: 116,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 9,
        attributes: Section {
            count: 1,
            index: 245,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 10,
        attributes: Section {
            count: 1,
            index: 246,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 11,
        attributes: Section {
            count: 1,
            index: 246,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 16,
        attributes: Section {
            count: 2,
            index: 38,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 16,
        attributes: Section {
            count: 3,
            index: 40,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 16,
        attributes: Section {
            count: 2,
            index: 43,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 16,
        attributes: Section {
            count: 2,
            index: 45,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 17,
        attributes: Section {
            count: 2,
            index: 50,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 17,
        attributes: Section {
            count: 2,
            index: 52,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 18,
        attributes: Section {
            count: 2,
            index: 117,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 18,
        attributes: Section {
            count: 3,
            index: 119,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 18,
        attributes: Section {
            count: 1,
            index: 72,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 18,
        attributes: Section {
            count: 1,
            index: 122,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 123,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 125,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 127,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 129,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 3,
            index: 119,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 117,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 38,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 131,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 133,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 135,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 137,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 19,
        attributes: Section {
            count: 2,
            index: 139,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 123,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 125,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 127,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 129,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 199,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 20,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 21,
        attributes: Section {
            count: 2,
            index: 199,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 22,
        attributes: Section {
            count: 2,
            index: 199,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 22,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 23,
        attributes: Section {
            count: 2,
            index: 236,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 23,
        attributes: Section {
            count: 2,
            index: 238,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 23,
        attributes: Section {
            count: 2,
            index: 240,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 23,
        attributes: Section {
            count: 2,
            index: 242,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 25,
        attributes: Section {
            count: 2,
            index: 247,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 26,
        attributes: Section {
            count: 2,
            index: 249,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 27,
        attributes: Section {
            count: 1,
            index: 245,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 27,
        attributes: Section {
            count: 2,
            index: 301,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 27,
        attributes: Section {
            count: 2,
            index: 303,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 28,
        attributes: Section {
            count: 2,
            index: 36,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 28,
        attributes: Section {
            count: 2,
            index: 305,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 30,
        attributes: Section {
            count: 1,
            index: 245,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 30,
        attributes: Section {
            count: 2,
            index: 301,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 30,
        attributes: Section {
            count: 2,
            index: 303,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 31,
        attributes: Section {
            count: 1,
            index: 307,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 32,
        attributes: Section {
            count: 1,
            index: 116,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 33,
        attributes: Section {
            count: 1,
            index: 307,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 33,
        attributes: Section {
            count: 2,
            index: 308,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 37,
        attributes: Section {
            count: 2,
            index: 54,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 37,
        attributes: Section {
            count: 2,
            index: 56,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 37,
        attributes: Section {
            count: 2,
            index: 58,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 37,
        attributes: Section {
            count: 2,
            index: 60,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 37,
        attributes: Section {
            count: 2,
            index: 62,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 38,
        attributes: Section {
            count: 4,
            index: 64,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 38,
        attributes: Section {
            count: 3,
            index: 68,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 41,
        attributes: Section {
            count: 1,
            index: 76,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 42,
        attributes: Section {
            count: 1,
            index: 77,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 44,
        attributes: Section {
            count: 1,
            index: 116,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 45,
        attributes: Section {
            count: 1,
            index: 72,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 46,
        attributes: Section {
            count: 2,
            index: 168,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGTimeIntentFromAGIntent",
        parent: 47,
        attributes: Section {
            count: 2,
            index: 182,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 47,
        attributes: Section {
            count: 2,
            index: 184,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 47,
        attributes: Section {
            count: 2,
            index: 186,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 47,
        attributes: Section {
            count: 2,
            index: 188,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGTimeIntentFromAGIntent",
        parent: 48,
        attributes: Section {
            count: 2,
            index: 182,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 48,
        attributes: Section {
            count: 2,
            index: 184,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 48,
        attributes: Section {
            count: 2,
            index: 186,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 48,
        attributes: Section {
            count: 2,
            index: 188,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 49,
        attributes: Section {
            count: 2,
            index: 190,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 50,
        attributes: Section {
            count: 2,
            index: 192,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 50,
        attributes: Section {
            count: 2,
            index: 194,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 52,
        attributes: Section {
            count: 1,
            index: 205,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 53,
        attributes: Section {
            count: 1,
            index: 206,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 54,
        attributes: Section {
            count: 1,
            index: 207,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 61,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 61,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 62,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 62,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 69,
        attributes: Section {
            count: 1,
            index: 49,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 71,
        attributes: Section {
            count: 1,
            index: 71,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 72,
        attributes: Section {
            count: 1,
            index: 72,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 72,
        attributes: Section {
            count: 1,
            index: 73,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 73,
        attributes: Section {
            count: 1,
            index: 74,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 74,
        attributes: Section {
            count: 1,
            index: 81,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 76,
        attributes: Section {
            count: 1,
            index: 85,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BoardAdjust",
        parent: 76,
        attributes: Section {
            count: 4,
            index: 86,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 77,
        attributes: Section {
            count: 1,
            index: 93,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BoardAdjust",
        parent: 77,
        attributes: Section {
            count: 3,
            index: 94,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 80,
        attributes: Section {
            count: 1,
            index: 141,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 81,
        attributes: Section {
            count: 2,
            index: 142,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 81,
        attributes: Section {
            count: 2,
            index: 144,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 81,
        attributes: Section {
            count: 2,
            index: 146,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 81,
        attributes: Section {
            count: 2,
            index: 148,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 82,
        attributes: Section {
            count: 2,
            index: 142,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 82,
        attributes: Section {
            count: 2,
            index: 144,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 82,
        attributes: Section {
            count: 2,
            index: 146,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 82,
        attributes: Section {
            count: 2,
            index: 148,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 83,
        attributes: Section {
            count: 1,
            index: 170,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 83,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 88,
        attributes: Section {
            count: 1,
            index: 177,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 88,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 93,
        attributes: Section {
            count: 3,
            index: 196,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 95,
        attributes: Section {
            count: 2,
            index: 208,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 95,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 95,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 96,
        attributes: Section {
            count: 2,
            index: 210,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 96,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 96,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 97,
        attributes: Section {
            count: 2,
            index: 212,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 97,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 97,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 98,
        attributes: Section {
            count: 2,
            index: 214,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 98,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 98,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 99,
        attributes: Section {
            count: 2,
            index: 216,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 99,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 99,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 100,
        attributes: Section {
            count: 2,
            index: 218,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 100,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 100,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 101,
        attributes: Section {
            count: 2,
            index: 220,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 101,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 101,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 102,
        attributes: Section {
            count: 2,
            index: 222,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 102,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 102,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 103,
        attributes: Section {
            count: 2,
            index: 224,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 103,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 103,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 104,
        attributes: Section {
            count: 2,
            index: 226,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 104,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 104,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 105,
        attributes: Section {
            count: 2,
            index: 228,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 105,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 105,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 106,
        attributes: Section {
            count: 2,
            index: 230,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 106,
        attributes: Section {
            count: 2,
            index: 203,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 106,
        attributes: Section {
            count: 2,
            index: 201,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 107,
        attributes: Section {
            count: 1,
            index: 232,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 108,
        attributes: Section {
            count: 1,
            index: 233,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 109,
        attributes: Section {
            count: 1,
            index: 234,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateTrickIntentFromGesture",
        parent: 110,
        attributes: Section {
            count: 1,
            index: 235,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 113,
        attributes: Section {
            count: 1,
            index: 49,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 117,
        attributes: Section {
            count: 1,
            index: 49,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 121,
        attributes: Section {
            count: 1,
            index: 49,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 125,
        attributes: Section {
            count: 1,
            index: 49,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 127,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 128,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 129,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 130,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 131,
        attributes: Section {
            count: 1,
            index: 75,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 133,
        attributes: Section {
            count: 1,
            index: 76,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 134,
        attributes: Section {
            count: 1,
            index: 77,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 135,
        attributes: Section {
            count: 1,
            index: 78,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 136,
        attributes: Section {
            count: 1,
            index: 79,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 137,
        attributes: Section {
            count: 1,
            index: 80,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 139,
        attributes: Section {
            count: 1,
            index: 82,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 141,
        attributes: Section {
            count: 1,
            index: 76,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 142,
        attributes: Section {
            count: 1,
            index: 77,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 143,
        attributes: Section {
            count: 1,
            index: 78,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 144,
        attributes: Section {
            count: 1,
            index: 79,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 145,
        attributes: Section {
            count: 1,
            index: 80,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 147,
        attributes: Section {
            count: 1,
            index: 83,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 148,
        attributes: Section {
            count: 1,
            index: 84,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 149,
        attributes: Section {
            count: 1,
            index: 90,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 150,
        attributes: Section {
            count: 1,
            index: 92,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 152,
        attributes: Section {
            count: 1,
            index: 97,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 153,
        attributes: Section {
            count: 1,
            index: 98,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 154,
        attributes: Section {
            count: 1,
            index: 99,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 156,
        attributes: Section {
            count: 1,
            index: 100,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BoardAdjust",
        parent: 156,
        attributes: Section {
            count: 4,
            index: 101,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 157,
        attributes: Section {
            count: 1,
            index: 105,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BoardAdjust",
        parent: 157,
        attributes: Section {
            count: 3,
            index: 106,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 158,
        attributes: Section {
            count: 1,
            index: 105,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BoardAdjust",
        parent: 158,
        attributes: Section {
            count: 3,
            index: 109,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 159,
        attributes: Section {
            count: 1,
            index: 100,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "BoardAdjust",
        parent: 159,
        attributes: Section {
            count: 4,
            index: 112,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 160,
        attributes: Section {
            count: 1,
            index: 150,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 160,
        attributes: Section {
            count: 2,
            index: 151,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 160,
        attributes: Section {
            count: 2,
            index: 153,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 161,
        attributes: Section {
            count: 1,
            index: 155,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 161,
        attributes: Section {
            count: 2,
            index: 156,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 161,
        attributes: Section {
            count: 2,
            index: 158,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 162,
        attributes: Section {
            count: 1,
            index: 155,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 162,
        attributes: Section {
            count: 2,
            index: 160,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 162,
        attributes: Section {
            count: 2,
            index: 162,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 163,
        attributes: Section {
            count: 1,
            index: 150,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 163,
        attributes: Section {
            count: 2,
            index: 164,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 163,
        attributes: Section {
            count: 2,
            index: 166,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 164,
        attributes: Section {
            count: 1,
            index: 173,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 164,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 165,
        attributes: Section {
            count: 1,
            index: 174,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 165,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 166,
        attributes: Section {
            count: 1,
            index: 174,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 166,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 167,
        attributes: Section {
            count: 1,
            index: 173,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 167,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 168,
        attributes: Section {
            count: 1,
            index: 175,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 168,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 169,
        attributes: Section {
            count: 1,
            index: 176,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 169,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 170,
        attributes: Section {
            count: 1,
            index: 176,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 170,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 171,
        attributes: Section {
            count: 1,
            index: 175,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 171,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 172,
        attributes: Section {
            count: 1,
            index: 178,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 172,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 173,
        attributes: Section {
            count: 1,
            index: 179,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 173,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 174,
        attributes: Section {
            count: 1,
            index: 179,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 174,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 175,
        attributes: Section {
            count: 1,
            index: 178,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 175,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 176,
        attributes: Section {
            count: 1,
            index: 180,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 176,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 177,
        attributes: Section {
            count: 1,
            index: 181,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 177,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 178,
        attributes: Section {
            count: 1,
            index: 181,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 178,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 179,
        attributes: Section {
            count: 1,
            index: 180,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateMGIntentFromAGIntent",
        parent: 179,
        attributes: Section {
            count: 2,
            index: 171,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 180,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 181,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 182,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 183,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 184,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 185,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 186,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 187,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 188,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 189,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 190,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 191,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 192,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 193,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 194,
        attributes: Section {
            count: 1,
            index: 48,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 195,
        attributes: Section {
            count: 1,
            index: 47,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 204,
        attributes: Section {
            count: 1,
            index: 78,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 205,
        attributes: Section {
            count: 1,
            index: 79,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 207,
        attributes: Section {
            count: 1,
            index: 78,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 208,
        attributes: Section {
            count: 1,
            index: 79,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 210,
        attributes: Section {
            count: 1,
            index: 76,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 211,
        attributes: Section {
            count: 1,
            index: 77,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 212,
        attributes: Section {
            count: 1,
            index: 91,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 213,
        attributes: Section {
            count: 1,
            index: 78,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 216,
        attributes: Section {
            count: 1,
            index: 79,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 217,
        attributes: Section {
            count: 1,
            index: 76,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 218,
        attributes: Section {
            count: 1,
            index: 77,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 219,
        attributes: Section {
            count: 1,
            index: 91,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 220,
        attributes: Section {
            count: 1,
            index: 78,
        },
        parameters: Section { count: 0, index: 0 },
    },
    BehaviourDeclaration {
        name: "CreateConstMGIntent",
        parent: 223,
        attributes: Section {
            count: 1,
            index: 79,
        },
        parameters: Section { count: 0, index: 0 },
    },
];
pub(super) const CONDITIONS: [ConditionDeclaration; 146] = [
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 314,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 315,
        },
    },
    ConditionDeclaration {
        name: "TimeSinceLastInput",
        attributes: Section {
            count: 1,
            index: 316,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedCompare",
        attributes: Section {
            count: 1,
            index: 317,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedCompare",
        attributes: Section {
            count: 1,
            index: 318,
        },
    },
    ConditionDeclaration {
        name: "InParentStateForTime",
        attributes: Section {
            count: 1,
            index: 316,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 319,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 320,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 321,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 322,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 323,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 324,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 325,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 326,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 327,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 328,
        },
    },
    ConditionDeclaration {
        name: "IsMirrored",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 329,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 330,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 331,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 332,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 333,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 334,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 335,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 336,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 337,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 338,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 339,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 340,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 341,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 342,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 343,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 344,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 345,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 346,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 347,
        },
    },
    ConditionDeclaration {
        name: "IsHandPlanting",
        attributes: Section {
            count: 1,
            index: 347,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 348,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 349,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 350,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 351,
        },
    },
    ConditionDeclaration {
        name: "IsMirrored",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 352,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 353,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 354,
        },
    },
    ConditionDeclaration {
        name: "IsFootPlanting",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 355,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 356,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 357,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 358,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 359,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 360,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 361,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 362,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 363,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 3,
            index: 364,
        },
    },
    ConditionDeclaration {
        name: "CurrentState",
        attributes: Section {
            count: 1,
            index: 367,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 368,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 369,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 370,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 368,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 3,
            index: 371,
        },
    },
    ConditionDeclaration {
        name: "CurrentState",
        attributes: Section {
            count: 1,
            index: 374,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 349,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 348,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 375,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 377,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 379,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 381,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 383,
        },
    },
    ConditionDeclaration {
        name: "TimeToLand",
        attributes: Section {
            count: 1,
            index: 384,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 347,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 385,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 386,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 387,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 388,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 389,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 3,
            index: 390,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 393,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 395,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 397,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 399,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 401,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 403,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 405,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 407,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 3,
            index: 409,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 412,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 414,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 416,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 418,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 420,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 422,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 424,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 2,
            index: 426,
        },
    },
    ConditionDeclaration {
        name: "DisablePushBrake",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 428,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedAndSlopeCompare",
        attributes: Section {
            count: 1,
            index: 429,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 3,
            index: 430,
        },
    },
    ConditionDeclaration {
        name: "IsRidingFakie",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedCompare",
        attributes: Section {
            count: 2,
            index: 433,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedCompare",
        attributes: Section {
            count: 2,
            index: 435,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedCompare",
        attributes: Section {
            count: 2,
            index: 437,
        },
    },
    ConditionDeclaration {
        name: "PhysicsSpeedCompare",
        attributes: Section {
            count: 2,
            index: 439,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 3,
            index: 441,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 444,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 445,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 205,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 446,
        },
    },
    ConditionDeclaration {
        name: "InParentStateForTime",
        attributes: Section {
            count: 1,
            index: 447,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 206,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 207,
        },
    },
    ConditionDeclaration {
        name: "HasAGIntent",
        attributes: Section {
            count: 1,
            index: 448,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 329,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 344,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 345,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 332,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 343,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 330,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 331,
        },
    },
    ConditionDeclaration {
        name: "IsGrinding",
        attributes: Section {
            count: 1,
            index: 346,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 232,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 233,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 234,
        },
    },
    ConditionDeclaration {
        name: "HasGestureIntent",
        attributes: Section {
            count: 1,
            index: 235,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 449,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 315,
        },
    },
    ConditionDeclaration {
        name: "PhysicsRequestsDismount",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "InParentStateForTime",
        attributes: Section {
            count: 1,
            index: 450,
        },
    },
    ConditionDeclaration {
        name: "IsGrabbingObject",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "IsDismounting",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "HasAnimAttribute",
        attributes: Section {
            count: 1,
            index: 451,
        },
    },
    ConditionDeclaration {
        name: "DisablePushBrake",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 452,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 385,
        },
    },
    ConditionDeclaration {
        name: "HasAnimAttribute",
        attributes: Section {
            count: 1,
            index: 453,
        },
    },
    ConditionDeclaration {
        name: "HasAnimAttribute",
        attributes: Section {
            count: 1,
            index: 453,
        },
    },
    ConditionDeclaration {
        name: "HasAnimAttribute",
        attributes: Section {
            count: 1,
            index: 454,
        },
    },
    ConditionDeclaration {
        name: "IsInLocomotion",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "PhysFilteredState",
        attributes: Section {
            count: 1,
            index: 452,
        },
    },
    ConditionDeclaration {
        name: "IsInLocomotion",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "IsLandingOnBoard",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "OBTimeToLand",
        attributes: Section {
            count: 1,
            index: 455,
        },
    },
    ConditionDeclaration {
        name: "PhysicsWantsRunout",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
    ConditionDeclaration {
        name: "IsDoneAirDismount",
        attributes: Section {
            count: 0,
            index: 65535,
        },
    },
];
pub(super) const STATES: [StateDeclaration; 224] = [
    StateDeclaration {
        name: "Action",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 65535,
        expression: 65535,
        children: Section { count: 3, index: 1 },
        behaviours: Section {
            count: 11,
            index: 0,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_cbc0,
        trailing: 0,
    },
    StateDeclaration {
        name: "OnBoardGeneral",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 0,
        expression: 65535,
        children: Section { count: 8, index: 4 },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 1, index: 0 },
        update_address: 0x828e_ccd0,
        trailing: 0,
    },
    StateDeclaration {
        name: "OffBoard",
        interrupt_type: 1,
        active: false,
        interrupt_target: 65535,
        parent: 0,
        expression: 65535,
        children: Section {
            count: 2,
            index: 12,
        },
        behaviours: Section {
            count: 27,
            index: 11,
        },
        transitions: Section { count: 2, index: 1 },
        update_address: 0x828e_ccd8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Default",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 0,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 38,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_cf30,
        trailing: 0,
    },
    StateDeclaration {
        name: "ForceBrake",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 1,
        expression: 37,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 39,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_cf80,
        trailing: 0,
    },
    StateDeclaration {
        name: "NoInputBrake",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 1,
        expression: 38,
        children: Section {
            count: 2,
            index: 14,
        },
        behaviours: Section {
            count: 1,
            index: 41,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d0b8,
        trailing: 0,
    },
    StateDeclaration {
        name: "OnBoard",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 1,
        expression: 65535,
        children: Section {
            count: 9,
            index: 16,
        },
        behaviours: Section {
            count: 8,
            index: 42,
        },
        transitions: Section { count: 3, index: 3 },
        update_address: 0x828e_cfe0,
        trailing: 0,
    },
    StateDeclaration {
        name: "TeleportOffBoard",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 1,
        expression: 29,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 50,
        },
        transitions: Section { count: 3, index: 6 },
        update_address: 0x828e_cf80,
        trailing: 0,
    },
    StateDeclaration {
        name: "TransToOffBoard",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 1,
        expression: 31,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 52,
        },
        transitions: Section { count: 6, index: 9 },
        update_address: 0x828e_cf80,
        trailing: 0,
    },
    StateDeclaration {
        name: "TransitioningToOffBoard",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 1,
        expression: 71,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 54,
        },
        transitions: Section {
            count: 4,
            index: 15,
        },
        update_address: 0x828e_cf80,
        trailing: 0,
    },
    StateDeclaration {
        name: "TransToOnBoard",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 1,
        expression: 72,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 56,
        },
        transitions: Section {
            count: 3,
            index: 19,
        },
        update_address: 0x828e_d0b8,
        trailing: 0,
    },
    StateDeclaration {
        name: "TransitioningToOnBoard",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 1,
        expression: 98,
        children: Section {
            count: 2,
            index: 25,
        },
        behaviours: Section {
            count: 1,
            index: 57,
        },
        transitions: Section {
            count: 3,
            index: 22,
        },
        update_address: 0x828e_d0b8,
        trailing: 0,
    },
    StateDeclaration {
        name: "OBGround",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 2,
        expression: 65535,
        children: Section {
            count: 4,
            index: 27,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d4a8,
        trailing: 0,
    },
    StateDeclaration {
        name: "OBAir",
        interrupt_type: 1,
        active: false,
        interrupt_target: 65535,
        parent: 2,
        expression: 65535,
        children: Section {
            count: 3,
            index: 31,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d4a8,
        trailing: 0,
    },
    StateDeclaration {
        name: "StartBrake",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 5,
        expression: 0,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 25,
        },
        update_address: 0x828e_d0b8,
        trailing: 0,
    },
    StateDeclaration {
        name: "WaitBrake",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 5,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 26,
        },
        update_address: 0x828e_d0b8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Grinding",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 6,
        expression: 3,
        children: Section {
            count: 3,
            index: 34,
        },
        behaviours: Section {
            count: 4,
            index: 58,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "InAir",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 6,
        expression: 6,
        children: Section {
            count: 8,
            index: 37,
        },
        behaviours: Section {
            count: 2,
            index: 62,
        },
        transitions: Section {
            count: 1,
            index: 27,
        },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "Coffin",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 6,
        expression: 101,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 4,
            index: 64,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d168,
        trailing: 0,
    },
    StateDeclaration {
        name: "OnGround",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 6,
        expression: 56,
        children: Section {
            count: 7,
            index: 45,
        },
        behaviours: Section {
            count: 12,
            index: 68,
        },
        transitions: Section {
            count: 2,
            index: 28,
        },
        update_address: 0x828e_d1f0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Trick",
        interrupt_type: 1,
        active: false,
        interrupt_target: 65535,
        parent: 6,
        expression: 20,
        children: Section {
            count: 3,
            index: 52,
        },
        behaviours: Section {
            count: 7,
            index: 80,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d318,
        trailing: 0,
    },
    StateDeclaration {
        name: "LateFlips",
        interrupt_type: 1,
        active: false,
        interrupt_target: 65535,
        parent: 6,
        expression: 20,
        children: Section {
            count: 6,
            index: 55,
        },
        behaviours: Section {
            count: 1,
            index: 87,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickOutOfTipSlashGrind",
        interrupt_type: 1,
        active: false,
        interrupt_target: 65535,
        parent: 6,
        expression: 20,
        children: Section {
            count: 2,
            index: 61,
        },
        behaviours: Section {
            count: 2,
            index: 88,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "Wipeout",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 6,
        expression: 27,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 4,
            index: 90,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Teleport",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 6,
        expression: 28,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d3e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Mirrored",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 11,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 94,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d3e8,
        trailing: 0,
    },
    StateDeclaration {
        name: "UnMirrored",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 11,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 95,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d3e8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Running",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 12,
        expression: 73,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 96,
        },
        transitions: Section {
            count: 4,
            index: 30,
        },
        update_address: 0x828e_d4b0,
        trailing: 0,
    },
    StateDeclaration {
        name: "HippyJump",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 12,
        expression: 74,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 99,
        },
        transitions: Section {
            count: 2,
            index: 34,
        },
        update_address: 0x828e_d448,
        trailing: 0,
    },
    StateDeclaration {
        name: "JumpingOntoBoard",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 12,
        expression: 74,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 36,
        },
        update_address: 0x828e_d4a8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Default",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 12,
        expression: 88,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 101,
        },
        transitions: Section {
            count: 4,
            index: 37,
        },
        update_address: 0x828e_d4b0,
        trailing: 0,
    },
    StateDeclaration {
        name: "AccidentalDismount",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 13,
        expression: 35,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 104,
        },
        transitions: Section {
            count: 3,
            index: 41,
        },
        update_address: 0x828e_d528,
        trailing: 0,
    },
    StateDeclaration {
        name: "Dismount",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 13,
        expression: 36,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 105,
        },
        transitions: Section {
            count: 1,
            index: 44,
        },
        update_address: 0x828e_d528,
        trailing: 0,
    },
    StateDeclaration {
        name: "Default",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 13,
        expression: 75,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 106,
        },
        transitions: Section {
            count: 2,
            index: 45,
        },
        update_address: 0x828e_d578,
        trailing: 0,
    },
    StateDeclaration {
        name: "Tail",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 16,
        expression: 65535,
        children: Section {
            count: 2,
            index: 63,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Nose",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 16,
        expression: 65535,
        children: Section {
            count: 2,
            index: 65,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Square",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 16,
        expression: 65535,
        children: Section {
            count: 4,
            index: 67,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "HandPlant",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 17,
        expression: 7,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 5,
            index: 108,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d318,
        trailing: 0,
    },
    StateDeclaration {
        name: "Grabbing",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 17,
        expression: 65535,
        children: Section {
            count: 5,
            index: 71,
        },
        behaviours: Section {
            count: 2,
            index: 113,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "BoardAdjusting",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 17,
        expression: 65535,
        children: Section {
            count: 4,
            index: 76,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "NewOneFoot",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 17,
        expression: 9,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 47,
        },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootRight",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 17,
        expression: 15,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 115,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d5d8,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootLeft",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 17,
        expression: 16,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 116,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d5d8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 17,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 49,
        },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "TransToOffBoardAir",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 17,
        expression: 55,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 117,
        },
        transitions: Section {
            count: 2,
            index: 50,
        },
        update_address: 0x828e_d5d8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Grabbing",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 65535,
        children: Section {
            count: 3,
            index: 80,
        },
        behaviours: Section {
            count: 1,
            index: 118,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d650,
        trailing: 0,
    },
    StateDeclaration {
        name: "Antic",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 58,
        children: Section {
            count: 10,
            index: 83,
        },
        behaviours: Section {
            count: 1,
            index: 119,
        },
        transitions: Section {
            count: 1,
            index: 52,
        },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "HippyJumping",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 87,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 4,
            index: 120,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d6a0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Pushing",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 67,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 4,
            index: 124,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d6a0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Brake",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 68,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 128,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "Kickturn",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 102,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 129,
        },
        transitions: Section {
            count: 1,
            index: 53,
        },
        update_address: 0x828e_d728,
        trailing: 0,
    },
    StateDeclaration {
        name: "IdleTurning",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 19,
        expression: 65535,
        children: Section {
            count: 2,
            index: 93,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 54,
        },
        update_address: 0x828e_d788,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFromSquare",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 20,
        expression: 91,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 131,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d790,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFromTail",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 20,
        expression: 92,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 132,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d790,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFromNose",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 20,
        expression: 93,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 133,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d790,
        trailing: 0,
    },
    StateDeclaration {
        name: "L_F_Kickflip",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 21,
        expression: 21,
        children: Section {
            count: 2,
            index: 95,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "L_F_Heelflip",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 21,
        expression: 22,
        children: Section {
            count: 2,
            index: 97,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "L_B_Kickflip",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 21,
        expression: 23,
        children: Section {
            count: 2,
            index: 99,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "L_B_Heelflip",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 21,
        expression: 24,
        children: Section {
            count: 2,
            index: 101,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "L_FS_Shuvit",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 21,
        expression: 25,
        children: Section {
            count: 2,
            index: 103,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "L_BS_Shuvit",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 21,
        expression: 26,
        children: Section {
            count: 2,
            index: 105,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d7e0,
        trailing: 0,
    },
    StateDeclaration {
        name: "At90",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 22,
        expression: 103,
        children: Section {
            count: 2,
            index: 107,
        },
        behaviours: Section {
            count: 2,
            index: 134,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "AtN90",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 22,
        expression: 65535,
        children: Section {
            count: 2,
            index: 109,
        },
        behaviours: Section {
            count: 2,
            index: 136,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Straight",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 34,
        expression: 105,
        children: Section {
            count: 4,
            index: 111,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Sideways",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 34,
        expression: 89,
        children: Section {
            count: 4,
            index: 115,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Straight",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 35,
        expression: 106,
        children: Section {
            count: 4,
            index: 119,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Sideways",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 35,
        expression: 90,
        children: Section {
            count: 4,
            index: 123,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 36,
        expression: 39,
        children: Section {
            count: 2,
            index: 127,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 36,
        expression: 40,
        children: Section {
            count: 2,
            index: 129,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "DoubleGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 36,
        expression: 41,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 138,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 36,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 55,
        },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "DBLGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 38,
        expression: 42,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 139,
        },
        transitions: Section {
            count: 2,
            index: 56,
        },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Superman",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 38,
        expression: 99,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 140,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 38,
        expression: 43,
        children: Section {
            count: 8,
            index: 131,
        },
        behaviours: Section {
            count: 1,
            index: 142,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 38,
        expression: 46,
        children: Section {
            count: 8,
            index: 139,
        },
        behaviours: Section {
            count: 1,
            index: 143,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "StaleMute",
        interrupt_type: 1,
        active: false,
        interrupt_target: 65535,
        parent: 38,
        expression: 65535,
        children: Section {
            count: 2,
            index: 147,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Up",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 39,
        expression: 47,
        children: Section {
            count: 3,
            index: 149,
        },
        behaviours: Section {
            count: 2,
            index: 144,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dac8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Down",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 39,
        expression: 49,
        children: Section {
            count: 4,
            index: 152,
        },
        behaviours: Section {
            count: 2,
            index: 146,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dac8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Left",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 39,
        expression: 85,
        children: Section {
            count: 2,
            index: 156,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 58,
        },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "Right",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 39,
        expression: 86,
        children: Section {
            count: 2,
            index: 158,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 59,
        },
        update_address: 0x828e_d108,
        trailing: 0,
    },
    StateDeclaration {
        name: "DBLGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 45,
        expression: 57,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 148,
        },
        transitions: Section {
            count: 1,
            index: 60,
        },
        update_address: 0x828e_d830,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 45,
        expression: 39,
        children: Section {
            count: 2,
            index: 160,
        },
        behaviours: Section {
            count: 4,
            index: 149,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d890,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 45,
        expression: 40,
        children: Section {
            count: 2,
            index: 162,
        },
        behaviours: Section {
            count: 4,
            index: 153,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d890,
        trailing: 0,
    },
    StateDeclaration {
        name: "OllieAnticipation",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 18,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 157,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "PopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 59,
        children: Section {
            count: 2,
            index: 164,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "FSPopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 60,
        children: Section {
            count: 2,
            index: 166,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "360PopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 61,
        children: Section {
            count: 2,
            index: 168,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "FS360PopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 62,
        children: Section {
            count: 2,
            index: 170,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "NollieAnticipation",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 19,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 159,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "N_PopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 63,
        children: Section {
            count: 2,
            index: 172,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "N_FSPopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 64,
        children: Section {
            count: 2,
            index: 174,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "N_360PopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 65,
        children: Section {
            count: 2,
            index: 176,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "N_FS360PopShuvitAnticipation",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 46,
        expression: 66,
        children: Section {
            count: 2,
            index: 178,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "HardTurn",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 51,
        expression: 69,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 161,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d930,
        trailing: 0,
    },
    StateDeclaration {
        name: "IdleTurning",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 51,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d788,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 55,
        expression: 70,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 162,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 55,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 165,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 56,
        expression: 70,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 168,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 56,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 171,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 57,
        expression: 70,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 174,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 57,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 177,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 58,
        expression: 70,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 180,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 58,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 183,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 59,
        expression: 70,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 186,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 59,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 189,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 60,
        expression: 70,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 192,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 60,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 195,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_d980,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFrom90Tail",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 61,
        expression: 94,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 198,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da08,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFrom90Nose",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 61,
        expression: 95,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 199,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da08,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFromN90Tail",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 62,
        expression: 96,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 200,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da08,
        trailing: 0,
    },
    StateDeclaration {
        name: "TrickFromN90Nose",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 62,
        expression: 97,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 201,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da08,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 63,
        expression: 39,
        children: Section {
            count: 2,
            index: 180,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 63,
        expression: 40,
        children: Section {
            count: 2,
            index: 182,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "DoubleGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 63,
        expression: 41,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 202,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 63,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 61,
        },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 64,
        expression: 39,
        children: Section {
            count: 2,
            index: 184,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 64,
        expression: 40,
        children: Section {
            count: 2,
            index: 186,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "DoubleGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 64,
        expression: 41,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 203,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 64,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 62,
        },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 65,
        expression: 39,
        children: Section {
            count: 2,
            index: 188,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 65,
        expression: 40,
        children: Section {
            count: 2,
            index: 190,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "DoubleGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 65,
        expression: 41,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 204,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 65,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 64,
        },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "LeftGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 66,
        expression: 39,
        children: Section {
            count: 2,
            index: 192,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "RightGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 66,
        expression: 40,
        children: Section {
            count: 2,
            index: 194,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "DoubleGrab",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 66,
        expression: 41,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 205,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 66,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 65,
        },
        update_address: 0x828e_da58,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 67,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 206,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 67,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 207,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 68,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 208,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 68,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 209,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Dismount",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 73,
        expression: 8,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 210,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "NewOneFoot",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 73,
        expression: 9,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 67,
        },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootRight",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 73,
        expression: 44,
        children: Section {
            count: 2,
            index: 196,
        },
        behaviours: Section {
            count: 1,
            index: 211,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootLeft",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 73,
        expression: 45,
        children: Section {
            count: 2,
            index: 198,
        },
        behaviours: Section {
            count: 1,
            index: 212,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlip",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 73,
        expression: 100,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 213,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipShuv",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 73,
        expression: 76,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 214,
        },
        transitions: Section {
            count: 1,
            index: 69,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipVar",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 73,
        expression: 77,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 215,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 73,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Dismount",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 74,
        expression: 8,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 216,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "NewOneFoot",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 74,
        expression: 9,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 70,
        },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootRight",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 74,
        expression: 44,
        children: Section {
            count: 2,
            index: 200,
        },
        behaviours: Section {
            count: 1,
            index: 217,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootLeft",
        interrupt_type: 0,
        active: false,
        interrupt_target: 65535,
        parent: 74,
        expression: 45,
        children: Section {
            count: 2,
            index: 202,
        },
        behaviours: Section {
            count: 1,
            index: 218,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlip",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 74,
        expression: 100,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 219,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipShuv",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 74,
        expression: 76,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 220,
        },
        transitions: Section {
            count: 1,
            index: 72,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipVar",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 74,
        expression: 77,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 221,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 74,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "MuteGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 75,
        expression: 46,
        children: Section {
            count: 3,
            index: 204,
        },
        behaviours: Section {
            count: 1,
            index: 222,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "StaleGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 75,
        expression: 43,
        children: Section {
            count: 3,
            index: 207,
        },
        behaviours: Section {
            count: 1,
            index: 223,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "TailGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 76,
        expression: 48,
        children: Section {
            count: 5,
            index: 210,
        },
        behaviours: Section {
            count: 1,
            index: 224,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "SeatbeltGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 76,
        expression: 83,
        children: Section {
            count: 2,
            index: 215,
        },
        behaviours: Section {
            count: 1,
            index: 225,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 76,
        expression: 11,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 3,
            index: 73,
        },
        update_address: 0x828e_dac0,
        trailing: 0,
    },
    StateDeclaration {
        name: "NoseGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 77,
        expression: 50,
        children: Section {
            count: 5,
            index: 217,
        },
        behaviours: Section {
            count: 1,
            index: 226,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "CrailGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 77,
        expression: 84,
        children: Section {
            count: 2,
            index: 222,
        },
        behaviours: Section {
            count: 1,
            index: 227,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "RocketAir",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 77,
        expression: 51,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 228,
        },
        transitions: Section {
            count: 1,
            index: 76,
        },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 77,
        expression: 12,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 3,
            index: 77,
        },
        update_address: 0x828e_dac0,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 78,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 229,
        },
        transitions: Section {
            count: 1,
            index: 80,
        },
        update_address: 0x828e_dac8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 78,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 231,
        },
        transitions: Section {
            count: 1,
            index: 81,
        },
        update_address: 0x828e_dac8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 79,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 233,
        },
        transitions: Section {
            count: 1,
            index: 82,
        },
        update_address: 0x828e_dac8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 79,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 235,
        },
        transitions: Section {
            count: 1,
            index: 83,
        },
        update_address: 0x828e_dac8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Goofy",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 81,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 237,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_db50,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 81,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 240,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_db50,
        trailing: 0,
    },
    StateDeclaration {
        name: "Goofy",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 82,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 243,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_db50,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 82,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 3,
            index: 246,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_db50,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 84,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 249,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 84,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 251,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 85,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 253,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 85,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 255,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 86,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 257,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 86,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 259,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 87,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 261,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 87,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 263,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 89,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 265,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 89,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 267,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 90,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 269,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 90,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 271,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 91,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 273,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 91,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 275,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 92,
        expression: 13,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 277,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 92,
        expression: 14,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 2,
            index: 279,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dbc8,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 111,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 281,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 111,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 282,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 112,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 283,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 112,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 284,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 115,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 285,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 115,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 286,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 116,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 287,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 116,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 288,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 119,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 289,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 119,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 290,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 120,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 291,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 120,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 292,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 123,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 293,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 123,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 294,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Regular",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 124,
        expression: 4,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 295,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Switch",
        interrupt_type: 1,
        active: true,
        interrupt_target: 65535,
        parent: 124,
        expression: 5,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 296,
        },
        transitions: Section { count: 0, index: 0 },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "Footplant",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 133,
        expression: 10,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 84,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 133,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 85,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Footplant",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 134,
        expression: 10,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 86,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 134,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 87,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Footplant",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 141,
        expression: 10,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 88,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 141,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 89,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Footplant",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 142,
        expression: 10,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 90,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 142,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 91,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlip",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 147,
        expression: 100,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 297,
        },
        transitions: Section {
            count: 1,
            index: 92,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipShuv",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 147,
        expression: 78,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 298,
        },
        transitions: Section {
            count: 1,
            index: 93,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 147,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 94,
        },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlip",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 148,
        expression: 100,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 299,
        },
        transitions: Section {
            count: 1,
            index: 96,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipShuv",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 148,
        expression: 78,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 300,
        },
        transitions: Section {
            count: 1,
            index: 97,
        },
        update_address: 0x828e_da60,
        trailing: 0,
    },
    StateDeclaration {
        name: "Idle",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 148,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 2,
            index: 98,
        },
        update_address: 0x828e_dc40,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootRight",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 149,
        expression: 79,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 301,
        },
        transitions: Section {
            count: 2,
            index: 100,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootLeft",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 149,
        expression: 80,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 302,
        },
        transitions: Section {
            count: 2,
            index: 102,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "NoFootAirWalk",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 149,
        expression: 81,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 303,
        },
        transitions: Section {
            count: 3,
            index: 104,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlip",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 149,
        expression: 100,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 304,
        },
        transitions: Section {
            count: 1,
            index: 107,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "IdleTailGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 149,
        expression: 82,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 4,
            index: 108,
        },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "IdleSeatbeltGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 150,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 112,
        },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipShuv",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 150,
        expression: 104,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 305,
        },
        transitions: Section {
            count: 1,
            index: 113,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootRight",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 152,
        expression: 79,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 306,
        },
        transitions: Section {
            count: 2,
            index: 114,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "OneFootLeft",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 152,
        expression: 80,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 307,
        },
        transitions: Section {
            count: 2,
            index: 116,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "NoFootAirWalk",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 152,
        expression: 81,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 308,
        },
        transitions: Section {
            count: 3,
            index: 118,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlip",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 152,
        expression: 100,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 309,
        },
        transitions: Section {
            count: 1,
            index: 121,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
    StateDeclaration {
        name: "IdleNoseGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 152,
        expression: 82,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 5,
            index: 122,
        },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "IdleCrailGrab",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 153,
        expression: 65535,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 0,
            index: 65535,
        },
        transitions: Section {
            count: 1,
            index: 127,
        },
        update_address: 0x828e_dcf0,
        trailing: 0,
    },
    StateDeclaration {
        name: "FingerFlipShuv",
        interrupt_type: 0,
        active: true,
        interrupt_target: 65535,
        parent: 153,
        expression: 104,
        children: Section {
            count: 0,
            index: 65535,
        },
        behaviours: Section {
            count: 1,
            index: 310,
        },
        transitions: Section {
            count: 1,
            index: 128,
        },
        update_address: 0x828e_dc90,
        trailing: 0,
    },
];
pub(super) const TRANSITIONS: [TransitionDeclaration; 129] = [
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 31,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 15,
        expression: 1,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 6,
        expression: 2,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 4,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 5,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 28,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 53,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 107,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 109,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 54,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 108,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 110,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 20,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 44,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 18,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 72,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 133,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 134,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 73,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 196,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 73,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 198,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 75,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 141,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 142,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 74,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 200,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 74,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 202,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 75,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 206,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 74,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 204,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 205,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 209,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 73,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 207,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 208,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 214,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 212,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 214,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 212,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 214,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 210,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 211,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 214,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 210,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 211,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 212,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 213,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 216,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 152,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 21,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 149,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 150,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 221,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 219,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 221,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 219,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 221,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 217,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 218,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 221,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 217,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 218,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 219,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 220,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 154,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 223,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 149,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 152,
        expression: 52,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 21,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 152,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 153,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 21,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 38,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 75,
        expression: 53,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 21,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 75,
        expression: 54,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 38,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 41,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 42,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 21,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 32,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 1,
        expression: 17,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 8,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 7,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 18,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 20,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 20,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 20,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 32,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 30,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 6,
        expression: 30,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 44,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 9,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 23,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 24,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 32,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 30,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 32,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 30,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 23,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 24,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 11,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 23,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 24,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 6,
        expression: 32,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 23,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 2,
        target: 24,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 23,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 24,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 30,
        expression: 33,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 29,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 10,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 33,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 32,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 33,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 1,
        expression: 34,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 27,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 28,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 10,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 33,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 12,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 33,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 32,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 33,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 30,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
    TransitionDeclaration {
        name: "",
        priority: 0,
        target: 10,
        expression: 65535,
        hooks: Section { count: 0, index: 0 },
    },
];
pub(super) const INDICES: [u16; 129] = [
    0, 110, 111, 3, 4, 5, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106,
    107, 108, 109, 1, 2, 13, 85, 86, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123,
    124, 125, 126, 127, 128, 80, 81, 82, 83, 84, 88, 89, 90, 12, 14, 15, 74, 77, 87, 6, 7, 8, 9,
    10, 11, 16, 17, 22, 23, 24, 29, 52, 53, 54, 70, 71, 72, 73, 75, 76, 78, 79, 18, 19, 20, 21, 25,
    26, 27, 28, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69,
];
