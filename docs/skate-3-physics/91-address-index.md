# Address index (generated)

Every executable address the Skate 3 physics files, the integration layer and the shared Skate 2
files name in their comments, sorted by address, with the line that names it.

**Which executable:** `TU3` = Skate 3 title update 3 (`default.xex`, the version the port
matches); `Skate 2` = Skate 2's executable. The label comes from the line or the two lines
before it; `TU3?` / `Skate 2?` mean the line does not say and the label is the file's context
(a Skate 3 file or a Skate 2 file): check the line. Both games load at 0x82000000, so the same
number can name different functions in the two executables.

2821 addresses, 3529 mentions.

| Address | Exe | Where | Line |
|---|---|---|---|
| `0x82024d08` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:40` | /// `ambienceCrossFade` 0x82024d08: the crossfade between two ambiences, in either order. |
| `0x82060c44` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_board_adjust.rs:34` | 2 => f32::from_bits(0x40490fdb), //82060C44 |
| `0x820641a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:234` | //82BBD7F0, literals820641A8 and8209975C. |
| `0x820641a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:60` | // by82F75F60 from820641A8, separately from cInputMap's own deadzones. |
| `0x8206d110` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/tip_nudge.rs:16` | //82D422A0..C4 multiplies original820996DC=45 by8206D110=deg-to-rad, |
| `0x8206d110` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:18` | /// `kInclineAngle` (`0x8206D110`), the degree-to-radian factor of both functions. |
| `0x82098e30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:32` | // 82F838B0 loads 82098E30 = BECC_CCCD into 830BDD20: -0.4. |
| `0x82098f70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:146` | let blend = f32::from_bits(0x3f666666); //82098F70:0.9 |
| `0x82099194` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:193` | // kSkitchMinApproachSpeed 0.5 (`0x82099194`). |
| `0x82099280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:286` | // Source literal82099280, not a fitted visual blend duration. |
| `0x82099280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:13` | /// 0x82d7a770's look-ahead time (0x82099280) for the anticipation zone. |
| `0x820996dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/tip_nudge.rs:16` | //82D422A0..C4 multiplies original820996DC=45 by8206D110=deg-to-rad, |
| `0x8209975c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:234` | //82BBD7F0, literals820641A8 and8209975C. |
| `0x8209975c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:19` | //82BC7218: both constructor defaults are literal8209975C=0.5. |
| `0x8209975c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:87` | //82D41FB0 literal8209975C; shared50-50/tip/5-O virtual60. |
| `0x820997b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:77` | //82D41320 reads processed1464, returning literal820997B8 or82165A10. |
| `0x820997b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:85` | //82D42AE8 literal820997B8. |
| `0x820c6d98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:188` | let mut best = 0.25; //820C6D98 |
| `0x820ddea8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:38` | /// TU3 82BBF180..F2EC, constants at 820DDEA8..DEE4. Sector windows |
| `0x820ed5e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:249` | //82898D98..DB4: ABS(dot(world_up, camera_Z)) < literal820ED5E8. |
| `0x821642b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:124` | //original 24-byte literal821642B0. No external runtime RNG is used. |
| `0x821642b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/rng.rs:11` | //seed or a fabricated substitute algorithm. Seed words821642B0. |
| `0x821647e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:50` | //82F82690 splats original821647E0 into830BD300. |
| `0x821647e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:52` | //830BD300 is initialized by82F82690 from821647E0. |
| `0x82165a00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:261` | // Serialized primitive82DC3AE4/82DC3B84, source literal82165A00. |
| `0x82165a10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:77` | //82D41320 reads processed1464, returning literal820997B8 or82165A10. |
| `0x82165a10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:77` | //broadcasts82165A10 (zero) into830BD380 for speed override direction. |
| `0x8216dee0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:17` | //82F825D0/F0 initialize830BD320/+4A0 from8231A844(+1)/8216DEE0(-1). |
| `0x8216dee0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:278` | // Original830BD4A0 initializer82F825F0 splats8216DEE0=-1. |
| `0x8216dee0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames.rs:48` | //830BD4A0 initializer82F825F0 splats8216DEE0=-1 across all lanes. |
| `0x82181a88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:171` | // 82F826F8 loads82181A88 with lvlx128, splats lane0, and stores |
| `0x82181a88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:76` | //82F826F8 broadcasts82181A88 into830BD350.82F82610 similarly |
| `0x821ee79c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:63` | //82BBDE40 compares PhysOutAnimation72 with literal821EE79C. |
| `0x8224dc78` | TU3 | `crates/skate2-audio/src/global_effects.rs:37` | /// TU3's effect class and its eight collections (`qword_8224DC78`). |
| `0x8227e8f0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1055` | // Original gI/gJ/gK at 8227e8f0/e900/e910 have W0; row3 is v127=0. |
| `0x822f860c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:55` | //Globals12/912 at822F860C/822F8990; linear HardDrive, angular SoftDrive. |
| `0x822f860c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:35` | // Existing82C091F8 standard setting, already scaled by822F860C. |
| `0x822f860c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/entry.rs:31` | //82C091F8 restores the standard angular drag with literal822F860C. |
| `0x822f860c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_possession/settings.rs:6` | ///091F8 full hash8D3065CED858A7DF, literal822F860C=426FFFFF. |
| `0x822f860c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_possession.rs:62` | ///091F8: actual8D3065CED858A7DF field multiplied by822F860C. |
| `0x822f88d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/skate_world.rs:99` | /// using the pi-squared word at 822F88D0. This is not acos/angle decoding. |
| `0x822f8908` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_board_adjust.rs:33` | 1 => f32::from_bits(0xc0490fdb), //822F8908 |
| `0x822f8990` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:55` | //Globals12/912 at822F860C/822F8990; linear HardDrive, angular SoftDrive. |
| `0x822f8b40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:15` | ///Original initializer82F826B0/822F8B40. This COM trajectory acceleration is |
| `0x822f8b40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:96` | //830BD330 initializer82F826B0 uses822F8B40, independently read original bytes. |
| `0x822f8c9c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/dismount.rs:17` | //822F8C9C; strict SIMD comparison is false for an unordered normal. |
| `0x822f900c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:21` | /// kMOAngleMaxDelta (2 degrees): flt_822F9010 and its negation flt_822F900C. |
| `0x822f9010` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:21` | /// kMOAngleMaxDelta (2 degrees): flt_822F9010 and its negation flt_822F900C. |
| `0x822f943c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:200` | //Globals3644 at822F943C permits at most 3cm below the triangle hit. |
| `0x822fbac0` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:244` | ///82D61458/64 call standalone Cos/Sin. Permute mask822FBAC0 followed |
| `0x822fbf3c` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:108` | /// TU3's splice voice (16 bytes, vtable 0x822fbf3c, constructor 0x82497f48): a splice playing into |
| `0x823006d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_execute.rs:13` | //823006D0+16 ->8258F330 returns MG+12, its authored intention map. |
| `0x82309664` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:62` | //lifecycle slots in82309664 are82B61BB8 (blr), with no state writes. |
| `0x82309664` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:21` | ///TU3 vtable82309664: Begin/Update/End all point to the empty82B61BB8. |
| `0x8231093c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:87` | //ClusteredMeshTrajectoryBatch vtable8231093C, AddTest8276E280. |
| `0x8231a844` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:17` | //82F825D0/F0 initialize830BD320/+4A0 from8231A844(+1)/8216DEE0(-1). |
| `0x8231d5c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:172` | // Factory82BC9678/84 uses literal8231D5C8, no authored override. |
| `0x8231e050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_execute.rs:17` | //End82BB5DB0 -> IAnimatable112 (8231E050+112) ->82531278: |
| `0x8231e050` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/selection_space_host.rs:14` | //TU3 vtable8231E050+76 ->82E32328 ->82D19648 constructs a |
| `0x8231e128` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:257` | //Actual vtable8231E128+16 is TransitionTo82D1D090. |
| `0x8231e128` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:96` | // Vtable8231E128: +16 = 82D1D090 (blend), +20 = 82D1D030 |
| `0x8231e170` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:53` | // SkaterAnim vtable8231E170+28=82B97140: full15180 bit30. |
| `0x8231e1f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:202` | //vtable8231E1F8. It waits for the old clip's actual end; it is not a |
| `0x8231ed5c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_conditions.rs:82` | //Factory82BC3F68 installs8231ED5C; virtual48=8274CA90 is li r3,0;blr. |
| `0x8231f460` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing.rs:72` | //82BC5860 ->8231F460+48 ->82BA74D8 ->specific v116. |
| `0x8231f6d4` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/push_off.rs:2` | //! The ctor installs vtable8231F6D4; predicate slot+48 is8281DD70. |
| `0x8231f88c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:468` | // Vtable8231F88C End points to empty82B61BB8. |
| `0x8231f97c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:74` | // HandBusy Update vtable8231F97C+52 is blr82B61BB8. |
| `0x8231fbdc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:445` | //8231FBDC Begin/End are empty. Update82BACDC0 consumes |
| `0x8231ff24` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:50` | // Vtable8231FF24 Update/End both point to empty82B61BB8. |
| `0x8232090c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:232` | //Vtable8232090C: Begin/Update empty; End82BB9158 sets length. |
| `0x82320bdc` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_runout.rs:1` | //! TU3 AddRunoutAttribs: factory82BC9BA8, vtable82320BDC, |
| `0x82321000` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:81` | // Begin vtable82321000+48 is blr82B61BB8. |
| `0x82321070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:231` | // Factory82BCB308 -> VT82321070 slot48=82BBD888. |
| `0x82321360` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:70` | //VT82321360 Begin=82BBE5D0; shared clip lengths from82B959E8. |
| `0x82321414` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:4` | //! registers constructor `0x82BCBF60`; vtable `0x82321414` creates a private |
| `0x823214d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_scoring_trick.rs:18` | /// Vtable823214D0: Update at+52, while Begin+48 and End+56 are blr. |
| `0x823269a0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:512` | /// 823269a0+0c/+18 are blr825187c0; only CollidingWithSkater is reset. |
| `0x82327174` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:1` | //! TU3 PHYSICS_STATE_GRIND_TRICK202, vtable82327174, owner PhysicalPlayer+1716. |
| `0x82327210` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:9` | //KnownAir ctor82D35130 installs82327210; vslot24 at82327228 is |
| `0x82327228` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:9` | //KnownAir ctor82D35130 installs82327210; vslot24 at82327228 is |
| `0x823272fc` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:273` | //VT823272FC slot8=82D431F0; slots12/40 are original empty leaves. |
| `0x82327330` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:1` | //! RevertGround102, TU3 vtable82327330: Enter82D43488, Update82D43518, |
| `0x82327364` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:1` | //! PC port: TU3 PhysState_OffBoard_Pushing (vtable `0x82327364`) for the Skate 3 physics. Its |
| `0x82327398` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:29` | // Skitching vtable 0x82327398 slot 6 is the same 0x82D34DA8. |
| `0x82327398` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:1` | //! PhysState_Skitching (104), TU3 vtable 0x82327398: Enter 0x82D47278, Update 0x82D477C0 (its |
| `0x823273cc` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:1` | //! TU3 HandPlantManager82D61040 and physical state600 (823273CC). |
| `0x823273cc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:136` | //HandPlant vtable823273CC slot40 ->82D4C530 ->82D90358(false). |
| `0x82327400` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:1` | //! FootPlant601, vtable82327400 and manager82D704B0/82D70E40. |
| `0x82327434` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:1` | //! Physical state602, TU3 vtable82327434. Stock no-comply/boneless/fastplant |
| `0x823285b4` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state.rs:1` | //! PhysicsSlideGround101 production lifecycle, original TU3 vtable823285B4. |
| `0x8232861c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/phase.rs:45` | // Vtable8232861C+20 is82B61BB8 (empty), not common grind Exit. |
| `0x82328688` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:29` | ///slot16 atA0D0. Actual vtable82328688+16 points to82DB3C78. |
| `0x82328688` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:67` | //Player slot16=82DB3C78 (vtable82328688). Consume with retained input |
| `0x823671b0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/catalog.rs:1` | //! Original gScorableInfo823671b0: IDs, consumed/base tricks, category, type and name. |
| `0x8236c2b8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_dispatch.rs:1` | //! Original retained collector dispatch, vtables8236c2b8..8236c3ac. |
| `0x8236c3ac` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_dispatch.rs:1` | //! Original retained collector dispatch, vtables8236c2b8..8236c3ac. |
| `0x823c3c78` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:96` | // Native encoding823C3C78 folds ASCII case, so this spelling aliases |
| `0x824531c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/tip_nudge.rs:17` | //then RELOADS that product before cosine82473930/sine824531C8. |
| `0x82453298` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/pose.rs:35` | //82453298, shared source-derived polynomial and independent PC estimate seed. |
| `0x82454fd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_finger_flip.rs:46` | //82454FD8 tests membership, including a zero-valued intent. The time |
| `0x82473930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/tip_nudge.rs:17` | //then RELOADS that product before cosine82473930/sine824531C8. |
| `0x82473b98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:189` | ///82BBD250..D390: original one-refinement reciprocal followed by82473B98. |
| `0x824744b8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:149` | /// TU3828D3800 then828D3B58. NewACS824744B8 initializes the |
| `0x82476158` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:1` | //! Original S3 loose octree insertion/query order, 82AD6830/82476158. |
| `0x82476158` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:116` | /// 82AD6D18 pushes resident then ascending leaf lists; 82476158 consumes |
| `0x82481e10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_leg.rs:32` | //82481E10 on the four authored knots, not the negative-graph evaluator. |
| `0x82481e10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/settings.rs:110` | //82481E10 receives the x/y spans after any PointNeg header. |
| `0x82485344` | TU3 | `crates/skate2-audio/src/system.rs:1969` | // TU3 0x82485344: the effect sub mixes off again after the queued splices. |
| `0x82485d18` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:62` | // IActorQuery thunk825f9d98 subtracts34, then folded82485d18 reads Actor+1d8. |
| `0x82490ca0` | TU3 | `crates/skate2-audio/src/global_effects.rs:18` | //! TU3 adds eight effect sub mixes (0x82490CA0, after `CreateLFOs`), stage 253, six channels: |
| `0x82491108` | TU3 | `crates/skate2-audio/src/global_effects.rs:351` | /// TU3 `sub_82491108`: the sub mix a sound routed to effect `index` plays into (the master mix |
| `0x82491108` | TU3 | `crates/skate2-audio/src/player/drop_in.rs:82` | /// The sound `key` names: its cue, then its Skate 3 effect (`sub_82491108`, which switches the |
| `0x82491180` | TU3 | `crates/skate2-audio/src/global_effects.rs:22` | //! (0x82491180). Built only when a Skate 3 sound first asks for one. |
| `0x82491180` | TU3 | `crates/skate2-audio/src/global_effects.rs:402` | /// TU3 `sub_82491180`, each audio update: every effect off again. |
| `0x824916e8` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:21` | //! effect's attributes (0x824916E8); every audio update switches them all off again |
| `0x824916e8` | TU3 | `crates/skate2-audio/src/global_effects.rs:352` | /// for 8 and out of range), and for the local player `sub_824916E8` switching the effect on: |
| `0x82497f48` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:108` | /// TU3's splice voice (16 bytes, vtable 0x822fbf3c, constructor 0x82497f48): a splice playing into |
| `0x82497f48` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:120` | /// The constructor 0x82497f48: the voice, then `new Splice(bank, index, the voice's SubMix)` and |
| `0x82498068` | TU3? | `crates/skate2-audio/src/player/skate3_mix.rs:151` | /// The destructor 0x82498068: the splice deleted, then the voice released. |
| `0x82498140` | TU3? | `crates/skate2-audio/src/player/skate3_mix.rs:142` | /// 0x82498140: `Splice::Update` with `update`, then the environment send's level (the first |
| `0x82498248` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:109` | /// a voice of its own (0x82498248): processing stage 1, a stereo SubMix and two stereo Sends, the |
| `0x824afad8` | TU3 | `crates/skate2-audio/src/player/tricks.rs:175` | // TU3 0x824cc3d0..c564: its constructor (0x824afad8) keeps the first twelve members, |
| `0x824b0278` | Skate 2? | `crates/skate2-audio/src/vmx.rs:90` | /// `XMVectorACos` 0x824b0278 (XNA Math, Cody and Waite) on one lane. |
| `0x824b0c28` | TU3 | `crates/skate2-audio/src/player/physics.rs:418` | // TU3 Reset 0x824b0c28. |
| `0x824b17fc` | TU3 | `crates/skate2-audio/src/player/contacts.rs:460` | // TU3's controller update (0x824b17fc..0x824b1834) keeps a darkcatch for five frames. |
| `0x824b1834` | TU3 | `crates/skate2-audio/src/player/contacts.rs:460` | // TU3's controller update (0x824b17fc..0x824b1834) keeps a darkcatch for five frames. |
| `0x824b1964` | TU3 | `crates/skate2-audio/src/player/physics.rs:650` | // TU3 0x824b1964 and 0x824b1970. |
| `0x824b1970` | TU3 | `crates/skate2-audio/src/player/physics.rs:650` | // TU3 0x824b1964 and 0x824b1970. |
| `0x824b81f0` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:39` | /// The shared `InitSFX` 0x824b81f0: the mixer output on. |
| `0x824b9268` | TU3 | `crates/skate2-audio/src/player/contacts.rs:682` | /// TU3 `PlayFootCatch` 0x824b9268 with Skate 3 physics: Skate 3's footcatch air time and speed |
| `0x824b9468` | TU3 | `crates/skate2-audio/src/player/contacts.rs:746` | /// `SKATE3_EFFECTS` names (TU3 0x824b9468 / 0x824bade4), in a splice voice of its own. |
| `0x824b94dc` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:685` | /// a splice voice of its own (0x824b94dc). |
| `0x824b9508` | TU3 | `crates/skate2-audio/src/player/contacts.rs:819` | /// `PlayFootCatchHelper` 0x8251c938; with Skate 3 physics TU3's 0x824b9508, which catches the |
| `0x824b97c8` | TU3 | `crates/skate2-audio/src/player/contacts.rs:768` | /// TU3 `PlayFootScrapeSound` 0x824b97c8 with the board inverted: Skate 3's |
| `0x824b97c8` | TU3 | `crates/skate2-audio/src/player/contacts.rs:892` | /// `PlayFootScrapeSound` 0x8251cbb0 (TU3 0x824b97c8 plays the inverted cue during a Skate 3 |
| `0x824b9a74` | TU3 | `crates/skate2-audio/src/player/contacts.rs:754` | /// TU3 `CheckForFootScrapes` 0x824b9a74..0x824b9ac0: a darkcatch (`SFXCTL_PlayerPhysics` |
| `0x824b9ac0` | TU3 | `crates/skate2-audio/src/player/contacts.rs:754` | /// TU3 `CheckForFootScrapes` 0x824b9a74..0x824b9ac0: a darkcatch (`SFXCTL_PlayerPhysics` |
| `0x824bac04` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:68` | /// set for the local skater after a landing's collision sound (TU3 LandGround 0x824bac04), and is |
| `0x824bac14` | TU3 | `crates/skate2-audio/src/player/contacts.rs:780` | /// TU3 `LandGround` 0x824bac14..0x824baee8: landing in or just after a darkcatch plays Skate 3's |
| `0x824bade4` | TU3 | `crates/skate2-audio/src/player/contacts.rs:746` | /// `SKATE3_EFFECTS` names (TU3 0x824b9468 / 0x824bade4), in a splice voice of its own. |
| `0x824baedc` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:783` | /// its threshold, through its effect, in a splice voice of its own (0x824baedc). |
| `0x824baee8` | TU3 | `crates/skate2-audio/src/player/contacts.rs:780` | /// TU3 `LandGround` 0x824bac14..0x824baee8: landing in or just after a darkcatch plays Skate 3's |
| `0x824be900` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:166` | /// 0x824be900): deleted once it stops playing, else the mixer's volume output `volume`, pitch |
| `0x824be900` | TU3 | `crates/skate2-audio/src/player/contacts.rs:1472` | // TU3 0x824be900: the darkcatch landing follows Skate 3's contacts mixer (object 13 here) |
| `0x824bebd8` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:165` | /// One update of a sound in a splice voice (TU3 0x824f0038, 0x824bebd8 and UpdateJumpsAndLands |
| `0x824bebd8` | TU3 | `crates/skate2-audio/src/player/contacts.rs:1388` | /// TU3 0x824bebd8, first in its `ProcessUpdate`: each Skate 3 catch follows Skate 3's contacts |
| `0x824c0b90` | TU3 | `crates/skate2-audio/src/player/contacts.rs:684` | /// (inverted with the board upside down, 0x824c0b90) from splice bank 0, through its effect, in |
| `0x824c3668` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_audio.rs:152` | /// What `AISkaterManager::UpdateAudioEvents` 0x824c3668 reads: `PhysOut_Audio::mJustWipedOut` |
| `0x824cbfb8` | TU3 | `crates/skate2-audio/src/player/tricks.rs:49` | /// TU3 `CheckForBoardFlips` 0x824cbfb8's last `Class_Flips` member: `aud_senseofspeed`'s |
| `0x824cc208` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:360` | /// AC_AMBIENT/PT_AI_Ambient unspawn or GetAppropriateRespawnTransform824cc208. |
| `0x824cc3d0` | TU3 | `crates/skate2-audio/src/player/tricks.rs:175` | // TU3 0x824cc3d0..c564: its constructor (0x824afad8) keeps the first twelve members, |
| `0x824cc7d8` | TU3 | `crates/skate2-audio/src/player/tricks.rs:397` | // TU3 UpdateBoardFlips 0x824cc7d8's members of its own Class_Flips. |
| `0x824d3fe0` | TU3 | `crates/skate2-audio/src/world/environment.rs:225` | // PC port: TU3's ChangeAmbience (0x824d3fe0) takes a Skate 3 bed's crossfade sound |
| `0x824de140` | TU3 | `crates/skate2-audio/src/reverb.rs:60` | /// 3's attributes (TU3 `VerbInit` 0x824de140 reads the same offsets). |
| `0x824e2c40` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:345` | /// `Sk8::PointGraphEval::Evaluate` 0x824e2c40. |
| `0x824e2c40` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:49` | /// Reference: `PointGraphEval::Evaluate` at `0x824e2c40`. |
| `0x824e2d10` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:3644` | // native BO4/BI25 at 0x824e2d10 selects y[2], without division. |
| `0x824e2d28` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:3606` | // fmadds at 0x824e2d28 retains -2^-46; separate operations yield +0. |
| `0x824e3360` | Skate 2? | `crates/skate2-audio/src/system.rs:1801` | 0x4bd3_e0cc_ea90_be26, // aud_general::HOM_MinGameSpeed 824e3360 |
| `0x824e4778` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:603` | /// `PauseMan::IsPaused(kPAUSE_FLAG_SIM)` (`0x824e4778`). |
| `0x824e64b8` | Skate 2? | `crates/skate2-audio/src/system.rs:1` | //! `Sk8::Audio::AudioSystem` (0x824e64b8-0x8250a300): the `Update` order, the HUD and menu sound |
| `0x824e64b8` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:427` | /// manager's gameplay; only `AudioSystem::Shutdown` 0x824e64b8 leaves it. The ambience therefore |
| `0x824e6740` | Skate 2? | `crates/skate2-audio/src/system.rs:784` | // `LoadBigFileHeaders` 0x824e6740: the traffic and crowd banks can load. |
| `0x824e67c0` | Skate 2? | `crates/skate2-audio/src/presentation.rs:279` | /// `AudioSystem::GetAudioMaterial` 0x824e67c0: `tAudioSurfaceType` 0 is undefined. |
| `0x824e6808` | Skate 2? | `crates/skate2-audio/src/system.rs:1144` | /// `GetMovieVolume` 0x824e6808: `mMovieVolume` (master output 8, `SetMovieVolume`) over |
| `0x824e69c8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1135` | /// `AudioSystem::GetCurrentReverbVolume` 0x824e69c8 (`EmitterSystem::GetCurrentReverbVolume` |
| `0x824e69e0` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:180` | /// `AudioSystem::PlayCrowdSound` 0x824e69e0: `CSTATEMGR_Crowd::PlayCrowdSound` at once. |
| `0x824e69e0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:7` | //! 0x824ff320) and the crowd reactions (`PlayCrowdSound` 0x824e69e0, `CacheCrowdPodInfos` |
| `0x824e69e0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:451` | /// `AudioSystem::PlayCrowdSound` 0x824e69e0: `CSTATEMGR_Crowd::PlayCrowdSound`. |
| `0x824e69e8` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:53` | /// `AudioSystem::GetGrabbedObjectKey` 0x824e69e8: `CSTATEMGR_DynamicObject::GetGrabbedObjectKey` |
| `0x824e6a08` | Skate 2? | `crates/skate2-audio/src/system.rs:749` | /// The splicer banks `GetHOMBank` 0x824e6a08 names, in `sHOMBankNames` order |
| `0x824e6b24` | Skate 2? | `crates/skate2-audio/src/system.rs:1086` | /// `NotifyPickPhotoScreen` 0x824e6b24 (`SFXObj_Challenge::NotifyPickPhotoScreen` 0x825187e8): |
| `0x824e7b28` | Skate 2? | `crates/skate2-audio/src/system.rs:3` | //! `PlayFEHudSplice` 0x824e7b28 -> `UpdateFEHudSplices` 0x824fea50), the menu flange voice, the MAIN |
| `0x824e7b28` | Skate 2? | `crates/skate2-audio/src/system.rs:1278` | /// `PlayFEHudSplice` 0x824e7b28: needs `mInGameDataLoaded`; takes the first slot that is |
| `0x824e7c20` | Skate 2? | `crates/skate2-audio/src/system.rs:1260` | /// The countdown keys `CheckMenuKey` 0x824e7c20 and `NotifyMenuSoundFinished` 0x824e7cc0 watch. |
| `0x824e7cc0` | Skate 2? | `crates/skate2-audio/src/system.rs:1260` | /// The countdown keys `CheckMenuKey` 0x824e7c20 and `NotifyMenuSoundFinished` 0x824e7cc0 watch. |
| `0x824e7d58` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:95` | /// The jam whose contest has crowds and speakers (`LoadCrowdsAndSpeakerSystem` 0x824e7d58). |
| `0x824e7d58` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:129` | /// `LoadCrowdsAndSpeakerSystem` 0x824e7d58: Demos and Best Tricks, and the Danny Way jam. |
| `0x824e7f10` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:318` | /// `aud_emitter::reverbType` 0x824e7f10: the `aud_reverb` collection of an emitter collection. |
| `0x824e8660` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:421` | /// `SetGlobalReverbVolume` 0x824e8660. |
| `0x824e8668` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:409` | /// `GetReverbSubMix` 0x824e8668: the "EnvSendSub" SubMix. |
| `0x824e8678` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:415` | /// `GetFlangeSubMix` 0x824e8678: flange 1's SubMix for patch 0x4000, flange 0's otherwise. |
| `0x824e86a0` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:426` | /// `SetFlangeAzimuth` 0x824e86a0: both flanges, whatever the patch. |
| `0x824e86b0` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:433` | /// `SetFlangeWet` 0x824e86b0. |
| `0x824e86c8` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:438` | /// `SetFlangeDry` 0x824e86c8. |
| `0x824e86e0` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:582` | /// `RWAudioLFOTask::RwAudioCoreTimer` 0x824e86e0, each frame: the time wraps by the cycle |
| `0x824e87e0` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:471` | /// `FootStepUtils::AdjustFilterParameters` 0x824e87e0: each notch frequency and gain becomes |
| `0x824e8940` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:493` | /// `FootStepUtils::CleanUpSplices` 0x824e8940. |
| `0x824e89e0` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:530` | /// `FootStepUtils::UpdateFootSplices` 0x824e89e0: collision then foot-push splice of each |
| `0x824e9628` | Skate 2? | `crates/skate2-audio/src/system.rs:1194` | /// `MusicSystem::GetCurrentMusicPlayerSongID` 0x824e9628. |
| `0x824e9d80` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:524` | /// loads: `Sk8EmitterSystem::EnterGamePlay` 0x824e9d80 (a `C_emitter_utility`, then |
| `0x824e9e00` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:154` | /// `Sk8EmitterSystem::LoadGameFiles` 0x824e9e00's files in its order: `data\audio\sfx.ems`, |
| `0x824e9e00` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:525` | /// `EmitterSystem::EnterGamePlay` 0x82a81498), `LoadGameFiles` 0x824e9e00 (`LoadDataFile` |
| `0x824e9e90` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1123` | /// `GetCurrentMusicVolume` 0x824e9e90: the music volume while one is current. |
| `0x824ece08` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:305` | /// getters `RegisterGetValueFunctions` 0x824ece08 registers), from its fields as its last update |
| `0x824ed830` | Skate 2? | `crates/skate2-audio/src/system.rs:66` | /// `MAP_CREATE_PARAMS::NumMixStates` (`InitializeMixer` 0x824ed830). |
| `0x824ed830` | Skate 2? | `crates/skate2-audio/src/system.rs:759` | /// `mInGameDataLoaded`) and `InitializeMixer` 0x824ed830; then the front end's `EnterFE` and |
| `0x824ed830` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:426` | /// `AudioSystem::EnterWorld` once at boot, whose `InitializeMixer` 0x824ed830 enters every state |
| `0x824ee348` | Skate 2? | `crates/skate2-audio/src/system.rs:758` | /// 0x824ee348 in `world_banks` as (name, `SPLICE_TYPE`, bytes), the front-end posts, |
| `0x824ee4b8` | Skate 2? | `crates/skate2-audio/src/system.rs:757` | /// voice, the splicer banks of `LoadSkateboardData` 0x824ee4b8 and `LoadCollisionData` |
| `0x824ef7f0` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:67` | /// `Destroy` 0x824ef7f0, which `Detach` runs: both sounds deleted (the flags stay). |
| `0x824effb0` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:184` | /// `AudioSystem::GetWalkCollision` 0x824effb0: `(bank, splice)`, the material's `Soft_L1` walking, |
| `0x824effe0` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:56` | /// `ProcessUpdate` 0x824effe0 (the local skater of an attached state): each sound follows the |
| `0x824f0038` | TU3 | `crates/skate2-audio/src/player/skate3_mix.rs:165` | /// One update of a sound in a splice voice (TU3 0x824f0038, 0x824bebd8 and UpdateJumpsAndLands |
| `0x824f0038` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:57` | /// mixer until it ends, the lock (0x824f0038) at volume output 1 and pitch output 2, the release |
| `0x824f01b8` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:213` | /// `AudioSystem::GetWalkVolume` 0x824f01b8: `Walk_Volume` (layout word 12) over 32767. |
| `0x824f01e8` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:58` | /// (0x824f01e8) at volume output 3 and pitch output 4, both with the reverb send at output 5. |
| `0x824f0298` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:8` | //! 0x824fd6c0, `GetNearestCrowdPodInfo` 0x824f0298). |
| `0x824f03b8` | TU3 | `crates/skate2-audio/src/player/drop_in.rs:2` | //! `CreateObject` 0x824f03b8): the local skater's drop-in. The tail locking on the edge |
| `0x824f03b8` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:25` | /// `SFXObj_DropIn`; the constructor 0x824f03b8 clears everything. |
| `0x824f0578` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:46` | /// `UpdateParams` 0x824f0578 (the local skater of an attached state): the lock (0x824f05d0), |
| `0x824f05d0` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:46` | /// `UpdateParams` 0x824f0578 (the local skater of an attached state): the lock (0x824f05d0), |
| `0x824f0644` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:19` | /// The lock's and the release's attribute keys (0x824f0644, 0x824f078c). |
| `0x824f0718` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:47` | /// then the release (0x824f0718). |
| `0x824f0718` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:5` | //! 0x824f0718, `UnloadVehicleData` 0x824fdbc0), the contest's music, emitter states and crowds |
| `0x824f0718` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:252` | /// `GetVehiclesLoadStatus` 0x824f0718: the engines' slots. |
| `0x824f078c` | Skate 2? | `crates/skate2-audio/src/player/drop_in.rs:19` | /// The lock's and the release's attribute keys (0x824f0644, 0x824f078c). |
| `0x824f0818` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:3` | //! 0x824fdcc8, `GetCrowdsLoadStatus` 0x824f0818, `UnloadCrowdData` 0x824f0918) and the traffic |
| `0x824f0818` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:246` | /// `GetCrowdsLoadStatus` 0x824f0818. |
| `0x824f0918` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:3` | //! 0x824fdcc8, `GetCrowdsLoadStatus` 0x824f0818, `UnloadCrowdData` 0x824f0918) and the traffic |
| `0x824f0918` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:313` | /// `UnloadCrowdData` 0x824f0918: the crowd slots unload and the emitter swap turns back on. |
| `0x824f0a18` | Skate 2? | `crates/skate2-audio/src/system.rs:1092` | /// `PauseForMovie` 0x824f0a18: the music stream paused (`XMPOverrideBackgroundMusic` has no |
| `0x824f0a98` | Skate 2? | `crates/skate2-audio/src/system.rs:1099` | /// `ResumeAfterMovie` 0x824f0a98: after a movie pause, the music stream resumes unless the |
| `0x824f0bf0` | Skate 2? | `crates/skate2-audio/src/system.rs:548` | /// `mpUtility`, the `C_foley_utility` `LoadFoleyData` 0x824f0bf0 creates after the foley banks: |
| `0x824f1080` | Skate 2? | `crates/skate2-audio/src/system.rs:978` | /// `QueueNISSpeech` 0x824f1080 (+0x64): a NIS sample (`Sk8SpeechSystem::QueueNISSpeech`). |
| `0x824f10e8` | Skate 2? | `crates/skate2-audio/src/system.rs:996` | /// `IsSpeechQueued` 0x824f10e8 (+0x6C): a line that is no NIS sample counts as queued. |
| `0x824f1158` | Skate 2? | `crates/skate2-audio/src/system.rs:954` | /// `QueueNISPost` 0x824f1158 (`IAudioInterface` +0x58): the scene's post (`mNISPost`) queued |
| `0x824f11c0` | Skate 2? | `crates/skate2-audio/src/system.rs:961` | /// `PlayNISPost` 0x824f11c0 (+0x5C): `PlayNISSpeech` of `mNISPost`. |
| `0x824f1228` | Skate 2? | `crates/skate2-audio/src/system.rs:967` | /// `IsPostQueued` 0x824f1228 (+0x60): `IsNISSpeechQueued` of `mNISPost`. |
| `0x824f12f0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:304` | /// What `AudioSystem` answers as `Contexts::GameInterface` (`GetValue` 0x824f12f0 over the |
| `0x824f12f0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:342` | /// `AudioSystem::GetValue` 0x824f12f0: variable `n` is getter `n - 1` of `mFuncPointers` |
| `0x824f13c8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:383` | /// `AudioSystem::CreateSplicerReverbVoice` 0x824f13c8 and the splice playing into it. |
| `0x824f1650` | Skate 2? | `crates/skate2-audio/src/system.rs:1656` | /// `UpdateHOMBankStatus` 0x824f1650 over the host's `HOM_Set_<n>.bnk` files. |
| `0x824f1cb8` | Skate 2? | `crates/skate2-audio/src/system.rs:291` | /// `AudioSystem::GetAmbienceKey` 0x824f1cb8: the additional location's ambience key while one |
| `0x824f1cb8` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:432` | /// The presentation's `GlobalData.ambienceKey` (`AudioSystem::GetAmbienceKey` 0x824f1cb8), |
| `0x824f1cf0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:274` | /// `AudioSystem::GetEmitterKey` 0x824f1cf0: the presentation's `GlobalData.emitterKey`, none in a |
| `0x824f1df8` | Skate 2? | `crates/skate2-audio/src/reverb.rs:301` | /// `UpdateParams` 0x8254e1e0 with a `Presentation` (`AudioSystem::GetReverbKey` 0x824f1df8 |
| `0x824f1df8` | Skate 2? | `crates/skate2-audio/src/system.rs:282` | /// `AudioSystem::GetReverbKey` 0x824f1df8: the additional location's reverb key while one is |
| `0x824f1ea8` | Skate 2? | `crates/skate2-audio/src/system.rs:840` | /// `CreateMenuFlangeVoice` 0x824f1ea8: stage 5, `SubMix` "Menu Flange Voice" and two `Send`s; |
| `0x824f2100` | Skate 2? | `crates/skate2-audio/src/system.rs:1329` | /// `UpdateMenuFlangeVoice` 0x824f2100: the flange send follows `mMenuFlangeLevel`. |
| `0x824f21d0` | Skate 2? | `crates/skate2-audio/src/system.rs:1237` | /// `StopMenuSoundImp` 0x824f21d0. The Hall of Meat branch compares the menu splice index, |
| `0x824f2398` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:171` | /// `GetCrowdType` 0x824f2398: none on line; else the `crowd_type` of the challenge's |
| `0x824f24b0` | Skate 2? | `crates/skate2-audio/src/system.rs:1006` | /// `CheckForRoundStart` 0x824f24b0, which sets up a contest's emitters (`mContestId`) and has |
| `0x824f24b0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:6` | //! (`TriggerMissionMusicImp` 0x824fee80, `CheckForRoundStart` 0x824f24b0, `RestartChallenge` |
| `0x824f24b0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:539` | /// `CheckForRoundStart` 0x824f24b0 (`NotifyNISPlaybackStart`): outside a replay, a loaded |
| `0x824f2728` | Skate 2? | `crates/skate2-audio/src/system.rs:1665` | /// `MicrophoneMan::UpdateCameras` 0x824f2728 (`AudioPresentation::mLocal` is the local |
| `0x824f2c20` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:527` | /// `UpdateReverb` 0x824f2c20: the level, notch, filter gains and pans, and delay times and |
| `0x824f2db0` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:545` | /// `UpdateGain` 0x824f2db0: instance `index`'s sends from the last `SetupReverb` parameters |
| `0x824f2ef8` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:457` | /// `SetupReverb` 0x824f2ef8: instance `inst` takes the preset (the notch gain 0.1..20, else |
| `0x824f3618` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:4` | //! `CreateReverbVoice` 0x824f3618 (one channel up to the six-channel pans): |
| `0x824f4220` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:553` | /// `SetFlangeParms` 0x824f4220: flange 1 for patch 0x4000, flange 0 otherwise; its delay time |
| `0x824f43b0` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:573` | /// `UpdateFlange` 0x824f43b0: each flange's pan, wet send and dry gain. |
| `0x824f44a0` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:14` | //! `CreateFlangeVoice` 0x824f44a0: "FlangeSub" (150) twice, index 0 for patches 0x2000/0x3000 |
| `0x824f4920` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:67` | /// The id `RWAudioLFOTask::AddTimer` 0x824f4920 registers its `TIMERSTAGE_BEFORECOMMANDS` timer |
| `0x824f5268` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:256` | /// `FootStepUtils::GetSpliceTuningKey` 0x824f5268: the `aud_cluster` collection of the first |
| `0x824f5510` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:247` | /// `FootStepUtils::GetFootPushSpliceInfoForFootStep` 0x824f5510: the `spliceTypeMap` entry for |
| `0x824f5588` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:231` | /// `FootStepUtils::GetSpliceInfoForFootStep` 0x824f5588: `(bank, splice)`. Grass and metal |
| `0x824f5670` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:221` | /// `FootStepUtils::GetMaterialVolume` 0x824f5670: the walk volume, `None` on grass and metal |
| `0x824f5718` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:369` | /// `FootStepUtils::CreateFootStepSubmixVoice` 0x824f5718: stage 4, `SubMix` "FootStep SubMix", |
| `0x824f5a38` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:509` | /// `FootStepUtils::UpdateSplicerVoice` 0x824f5a38: the splice's filters, the random notches |
| `0x824f5c70` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:279` | /// `FootStepUtils::GetFootStepClothSplice` 0x824f5c70. |
| `0x824f5d88` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:482` | /// `FootStepUtils::SetupSpliceVoices` 0x824f5d88: the missing voices; the foot-push splices |
| `0x824f5e20` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:500` | /// `FootStepUtils::CleanUpSpliceVoices` 0x824f5e20 (`AudioSystem::FreeSubmixVoice`). |
| `0x824f61e0` | Skate 2? | `crates/skate2-audio/src/system.rs:1169` | /// `MusicSystem::SetDefaultMode` 0x824f61e0 (`music::mode`), as `FE::MusicPlayerManager:: |
| `0x824f6d98` | Skate 2? | `crates/skate2-audio/src/system.rs:1175` | /// `MusicSystem::IPodTrigger` 0x824f6d98 as `FE::MusicPlayerManager` sends it (`Play` |
| `0x824f81d0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:609` | /// `Sk8EmitterSystem::GetEmitterKey` 0x824f81d0: `AudioSystem::GetEmitterKey` while the swap |
| `0x824f8248` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:35` | /// `aud_emitter` attributes: the volume type (`GetVolumeType` 0x824f8248), the falloff |
| `0x824f8248` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:585` | /// `GetVolumeType` 0x824f8248. |
| `0x824f83b0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:746` | // `GetEmitterWeight` 0x824f83b0: 10 past the weights. |
| `0x824f8450` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:161` | /// The loader path `GetEmitterFilename` 0x824f8450 builds for a bank of `emitters.big`. |
| `0x824f8450` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:590` | /// `GetEmitterFilename` 0x824f8450: the bank in `emitters.big`, lower case. |
| `0x824f8538` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:600` | /// `ChooseTimeForNextRandomEmitter` 0x824f8538: `Random() % 1000` thousandths of the way |
| `0x824f85d8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:46` | /// `PlayEmitter` 0x824f85d8 draws in. |
| `0x824f85d8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:860` | /// `PlayEmitter` 0x824f85d8: a random emitter at a random azimuth, its volume drawn in |
| `0x824f88e8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:36` | /// (`PlayVolume`), the alternative keys (`GetKeyFromVolume` 0x824f88e8), `reverbType`, the |
| `0x824f88e8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:935` | /// `GetKeyFromVolume` 0x824f88e8: one of the volume's alternative collections at random |
| `0x824f89d0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1057` | /// `UpdateVolumeList` 0x82a818f0: each loaded AEMS volume plays (`PlayVolume` 0x824f89d0) |
| `0x824f8ba8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1058` | /// or follows its distance (`UpdateVolume` 0x824f8ba8). |
| `0x824f8cf0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:37` | /// music playlist, pan size and mid radius (`UpdateCurrentMusicVolume` 0x824f8cf0). |
| `0x824f8cf0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1095` | /// `UpdateMusicVolumes` 0x82a81bd8 and `UpdateCurrentMusicVolume` 0x824f8cf0: the first music |
| `0x824f8ec0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:54` | /// `Sk8::Audio::verbHash` 0x82fd8158: the reverb keys `VerifyReverbKey` 0x824f8ec0 accepts. |
| `0x824f8ec0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:595` | /// `VerifyReverbKey` 0x824f8ec0: the `reverbType` collection is one of `verbHash`. |
| `0x824f8f68` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1167` | /// `AsyncLoadDLCData` 0x824f8f68: the location's emitter data file into the DLC slot. |
| `0x824f9098` | TU3 | `crates/skate2-audio/src/system.rs:1972` | // only `Sk8EmitterSystem::UpdateDLCAsyncLoad` 0x824f9098 runs. |
| `0x824f9098` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1179` | /// `UpdateDLCAsyncLoad` 0x824f9098: the loaded file's volumes join (`AddDataFile` 0x82a81998). |
| `0x824f9148` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1198` | /// `UnloadDLCData` 0x824f9148: `RemoveDataFile` 0x82a836d8, then the DLC slot unloads. |
| `0x824f9db0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:700` | /// 0x824f9db0: `<group>context.big` holds the scripts (entry 0), the trigger functions' |
| `0x824fd6c0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:8` | //! 0x824fd6c0, `GetNearestCrowdPodInfo` 0x824f0298). |
| `0x824fd6c0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:151` | /// `CacheCrowdPodInfos` 0x824fd6c0, from `TriggerMissionMusic` 0x82504f28 at a countdown on the |
| `0x824fd868` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:4` | //! banks they share their memory with (`LoadVehiclesData` 0x824fd868, `GetVehiclesLoadStatus` |
| `0x824fd868` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:327` | /// `LoadVehiclesData` 0x824fd868: the crowd banks unload; the traffic banks wait for them |
| `0x824fdbc0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:5` | //! 0x824f0718, `UnloadVehicleData` 0x824fdbc0), the contest's music, emitter states and crowds |
| `0x824fdbc0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:347` | /// `UnloadVehicleData` 0x824fdbc0. |
| `0x824fdcc8` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:3` | //! 0x824fdcc8, `GetCrowdsLoadStatus` 0x824f0818, `UnloadCrowdData` 0x824f0918) and the traffic |
| `0x824fdcc8` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:270` | /// `LoadCrowdsData` 0x824fdcc8: unless loaded, the contest's banks wait for the emitter swap |
| `0x824fdfe8` | Skate 2? | `crates/skate2-audio/src/system.rs:756` | /// NIS and post samples), `LoadInGameData` 0x824fdfe8 (the two front-end banks, the menu flange |
| `0x824fe178` | Skate 2? | `crates/skate2-audio/src/system.rs:818` | // `SecondaryLoad` 0x824fe178: the traffic banks load on the next update. |
| `0x824fe540` | Skate 2? | `crates/skate2-audio/src/system.rs:1005` | /// `NotifyNISPlaybackStart` 0x824fe540 (+0x74, `Scene::Init`): the scene's info, then |
| `0x824fe558` | Skate 2? | `crates/skate2-audio/src/system.rs:1013` | /// `NotifyNISPlaybackFinish` 0x824fe558 (+0x78, `Scene::Purge`): the music resumes |
| `0x824fea50` | Skate 2? | `crates/skate2-audio/src/system.rs:3` | //! `PlayFEHudSplice` 0x824e7b28 -> `UpdateFEHudSplices` 0x824fea50), the menu flange voice, the MAIN |
| `0x824fea50` | Skate 2? | `crates/skate2-audio/src/system.rs:1294` | /// `UpdateFEHudSplices` 0x824fea50. |
| `0x824fecb8` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:2` | //! (`PlayMissionSFXImp` 0x824fecb8), the crowd banks of `data\audio\crowds.big` (`LoadCrowdsData` |
| `0x824fecb8` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:181` | /// `PlayMissionSFXImp` 0x824fecb8: the camera's shutter (a menu sound), Own the Spot |
| `0x824fee80` | Skate 2? | `crates/skate2-audio/src/system.rs:345` | /// What `TriggerMissionMusicImp` 0x824fee80 reads of the challenge (`Attrib::Gen::challenges` of |
| `0x824fee80` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:6` | //! (`TriggerMissionMusicImp` 0x824fee80, `CheckForRoundStart` 0x824f24b0, `RestartChallenge` |
| `0x824fee80` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:457` | /// `TriggerMissionMusicImp` 0x824fee80. Demos, Best Tricks, Mega Ramps, Street Skating and the |
| `0x824ff320` | Skate 2? | `crates/skate2-audio/src/system.rs:1055` | /// `RestartChallenge` 0x824ff320 (+0x84, `ChallengeAudioManager::RestartChallenge` |
| `0x824ff320` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:7` | //! 0x824ff320) and the crowd reactions (`PlayCrowdSound` 0x824e69e0, `CacheCrowdPodInfos` |
| `0x824ff320` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:564` | /// `RestartChallenge` 0x824ff320 with a contest loaded: the music waits for the next start, the |
| `0x824ff478` | Skate 2? | `crates/skate2-audio/src/system.rs:1049` | /// `QuitChallenge` 0x824ff478, the `cMsgAbandonChallenge` handler `AddDelayedMessageHandlers` |
| `0x824ff7a0` | Skate 2? | `crates/skate2-audio/src/system.rs:1036` | /// (`CameraManCsisEventHandler` 0x824ff7a0). |
| `0x825009c8` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:230` | /// The constructor: `CreateReverbVoice`, `CreateFlangeVoice`, then `CreateLFOs` 0x825009c8 |
| `0x82504450` | Skate 2? | `crates/skate2-audio/src/system.rs:755` | /// `EnterWorld` 0x82504450: `Initialize` (with `SecondaryLoad`'s speech projects and resident |
| `0x82504498` | Skate 2? | `crates/skate2-audio/src/system.rs:1467` | /// `AudioSystem::IsLoading` 0x82504498. |
| `0x82504540` | Skate 2? | `crates/skate2-audio/src/system.rs:939` | /// `PlayNISSfx` 0x82504540, from an `ICEUserChannelAudioEffect` command 2 (`FireEvent` |
| `0x82504848` | Skate 2? | `crates/skate2-audio/src/system.rs:1776` | /// `UpdatePresentationData` 0x82504848, which runs after the state managers' `UpdateParams`: |
| `0x82504848` | Skate 2? | `crates/skate2-audio/src/system.rs:1794` | // 82504848: the bank must be resident; latch once the authored speed |
| `0x82504848` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:14` | //! The volumes' game states are only those `AudioSystem::UpdatePresentationData` 0x82504848 sets |
| `0x82504dd0` | Skate 2? | `crates/skate2-audio/src/system.rs:2` | //! path (`PlayMenuSound` 0x82504dd0 -> `cMsgPlayMenuSound` -> `PlayMenuSoundImp` 0x8250c738 -> |
| `0x82504dd0` | Skate 2? | `crates/skate2-audio/src/system.rs:907` | /// `PlayMenuSound` 0x82504dd0: posts `cMsgPlayMenuSound` to the audio event broadcaster. |
| `0x82504e20` | Skate 2? | `crates/skate2-audio/src/system.rs:912` | /// `StopMenuSound` 0x82504e20. |
| `0x82504e70` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:144` | /// `PlayMissionSFX` 0x82504e70 (+0x38, `ChallengeAudioManager::PlayMissionSFX`): posts |
| `0x82504ec0` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:146` | /// (`PlayMissionSFXMsg` 0x82504ec0). `mission` is what the handler reads of the challenge. |
| `0x82504ed0` | Skate 2? | `crates/skate2-audio/src/system.rs:1023` | /// `PlayMissionSpeech` 0x82504ed0 (+0x3C, `ChallengeAudioManager::PlaySpeech` 0x82da1b98): |
| `0x82504f28` | Skate 2? | `crates/skate2-audio/src/system.rs:1041` | /// `TriggerMissionMusic` 0x82504f28 (+0x34, `ChallengeAudioManager::PlayMusic`): posts |
| `0x82504f28` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:151` | /// `CacheCrowdPodInfos` 0x824fd6c0, from `TriggerMissionMusic` 0x82504f28 at a countdown on the |
| `0x82504fb8` | Skate 2? | `crates/skate2-audio/src/system.rs:1043` | /// (`TriggerMissionMusicMsg` 0x82504fb8). Its countdown's `CacheCrowdPodInfos` is the host's |
| `0x82505220` | TU3 | `crates/skate2-core/src/skate3/skate3_dlc.rs:189` | /// `UnlockablesBackEnd::LoadUnlocksFromFiles` (TU3 `0x82505220`) reads a line: up to `\r` or `\n`, |
| `0x82505740` | Skate 2? | `crates/skate2-audio/src/system.rs:1034` | /// `ChallengeFailedHandler` 0x82505740's `cMsgPlayCameraManCsisEvent` with unset parameters |
| `0x82505c10` | Skate 2? | `crates/skate2-audio/src/global_effects.rs:1` | //! `Sk8::Audio::GlobalEffects` (0x43C bytes, constructor 0x82505c10): the two environment |
| `0x825069f0` | Skate 2? | `crates/skate2-audio/src/system.rs:1200` | /// The new iPod song `MusicSystem::UpdateChyron` 0x825069f0 displayed since the last call. |
| `0x82507b88` | Skate 2? | `crates/skate2-audio/src/system.rs:1918` | /// `AudioSystem::Update` 0x82507b88. Group A (asset loads, the event dispatch, the splice |
| `0x82508378` | Skate 2? | `crates/skate2-audio/src/system.rs:498` | /// `ReceiveFEMusicStateChangeMsg` 0x82508378 writes. |
| `0x82508378` | Skate 2? | `crates/skate2-audio/src/system.rs:2091` | /// `ReceiveFEMusicStateChangeMsg` 0x82508378 for the state `ChangeFEMusicState` posted: |
| `0x82508400` | Skate 2? | `crates/skate2-audio/src/system.rs:2076` | /// `MusicSystem::Update` 0x82508400 (group A, after the event dispatch, which delivers the |
| `0x82508858` | Skate 2? | `crates/skate2-audio/src/system.rs:1050` | /// 0x82508858 puts on the challenge broadcaster. |
| `0x82509ba8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:671` | /// `AudioSystem::GetTime` 0x82509ba8: the audio core's system time in milliseconds (`fctidz`, |
| `0x82509c18` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:5` | //! `AudioSystem::Initialize` 0x82509c18 creates the three systems (`CreateContextSystem` |
| `0x82509c18` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:523` | /// The emitter steps of `AudioSystem::Initialize` 0x82509c18 after `emitter_utility.abk` |
| `0x8250a300` | Skate 2? | `crates/skate2-audio/src/system.rs:1` | //! `Sk8::Audio::AudioSystem` (0x824e64b8-0x8250a300): the `Update` order, the HUD and menu sound |
| `0x8250c6a0` | Skate 2? | `crates/skate2-audio/src/system.rs:985` | /// `PlayNISSpeech` 0x8250c6a0 (+0x68): a NIS sample (`Sk8SpeechSystem::PlayNISSpeech`) or, |
| `0x8250c738` | Skate 2? | `crates/skate2-audio/src/system.rs:2` | //! path (`PlayMenuSound` 0x82504dd0 -> `cMsgPlayMenuSound` -> `PlayMenuSoundImp` 0x8250c738 -> |
| `0x8250c738` | Skate 2? | `crates/skate2-audio/src/system.rs:1218` | /// `PlayMenuSoundImp` 0x8250c738. |
| `0x8250d0a8` | Skate 2? | `crates/skate2-audio/src/system.rs:1025` | /// (`PlayMissionSpeechMsg` 0x8250d0a8). |
| `0x8250d438` | Skate 2? | `crates/skate2-audio/src/system.rs:2017` | /// `Sk8SpeechSystem::Update` 0x8250d438 (group A, after `MusicSystem::Update`): the simulation |
| `0x8250dda0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:103` | /// The `EmitterSystem` constructor's arguments (`AudioSystem::AudioSystem` 0x8250dda0): five |
| `0x8250dda0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:447` | /// `AudioSystem::AudioSystem` 0x8250dda0 sets: two random triggers, new volumes allowed, the |
| `0x8250e39c` | Skate 2? | `crates/skate2-audio/src/system.rs:448` | /// AudioSystem constructor8250e39c: eSk8Character_Max. |
| `0x8250fe58` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:136` | /// The member ranges the constructor 0x8250fe58 and the inline setters clamp to. |
| `0x8250fe58` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:579` | /// 0x8250fe58). |
| `0x82510538` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:370` | /// `SFXCTL_PlayerPhysics::Reset` 0x82510538 (also `Detach`). |
| `0x82510708` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:473` | /// `SFXCTL_PlayerPhysics::InitSFX` 0x82510708 (the position controllers' `Assign*Vector` |
| `0x825137a0` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:37` | /// `Csis::Class_Flips::Class_Flips` 0x825137a0: each member's clamp. |
| `0x82514248` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1549` | /// `AddToBailAverage` 0x82514248. |
| `0x82514f70` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:74` | /// The constructor 0x82514f70. |
| `0x82515028` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:442` | /// `UpdateTricksDone` 0x82515028: the trick-done cloth instance until a wipeout. |
| `0x82515218` | Skate 2? | `crates/skate2-audio/src/system.rs:1623` | /// `SFXObj_Announcer::ProcessUpdate` 0x82515218 (also `SFXObj_CameraMan`'s): azimuth 0, |
| `0x825159e0` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:411` | /// `GetAmbienceInfo` 0x825159e0: the bed with this id, else the first one. |
| `0x82515a40` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:416` | /// `GetAmbienceCrossFadeID` 0x82515a40: 25 to or from [`CROSS_FADE_25`], the pair's entry in |
| `0x82515d80` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:649` | /// `InitSFX` (`SFXObjBase` 0x82515d80): the mixer outputs on. |
| `0x82515d80` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:109` | /// The `InitSFX` (0x82515d80, shared) `SetupLoadData` runs on attach: the mixer outputs on. |
| `0x82516570` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:351` | /// The `tFootStep` constructor's values 0x82516570. |
| `0x82516570` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:436` | /// The `tFootStep` constructor 0x82516570 and the `SFXObj_OffBoard` constructor's footstep |
| `0x82517018` | Skate 2? | `crates/skate2-audio/src/reverb.rs:274` | /// `GetReverbCategory` 0x82517018: the active collection's type word (+0x30): types 6-8 and |
| `0x825170c0` | Skate 2? | `crates/skate2-audio/src/reverb.rs:289` | /// `UpdateNicotine` 0x825170c0: inputs 0-3 are the categories 1-4. |
| `0x825187c0` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_selected_animation_conditioning.rs:135` | // PathRecording is not selected; Stats is the empty825187c0. |
| `0x825187c0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:493` | /// 825187c0 is BLR. Reset overwrites all four lanes of every vector. |
| `0x825187c0` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:71` | /// Camera observer82581c08 forwards to folded825187c0, an actual blr. |
| `0x825187c0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:512` | /// 823269a0+0c/+18 are blr825187c0; only CollidingWithSkater is reset. |
| `0x825187e8` | Skate 2? | `crates/skate2-audio/src/system.rs:1086` | /// `NotifyPickPhotoScreen` 0x824e6b24 (`SFXObj_Challenge::NotifyPickPhotoScreen` 0x825187e8): |
| `0x825195c0` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:502` | /// `UpdateCollisionForces` 0x825195c0. |
| `0x825196a8` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:512` | /// `CalculateWheelOcclusion` 0x825196a8 (input 3). |
| `0x82519840` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:485` | /// `ConvertToImpactLevelFromTime` 0x82519840. |
| `0x82519958` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:493` | /// `UpdateWheelLevelFromTime` 0x82519958. |
| `0x825199d0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:277` | /// `SFXCTL_PlayerPhysics::GetWheelType` 0x825199d0. |
| `0x8251b7b8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:634` | /// `SpliceUtil::GetSK8_Foot_CatchIndex` 0x8251b7b8 / `GetSplice_handsIndex` 0x8251b900: the |
| `0x8251b900` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:634` | /// `SpliceUtil::GetSK8_Foot_CatchIndex` 0x8251b7b8 / `GetSplice_handsIndex` 0x8251b900: the |
| `0x8251c108` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:415` | /// `SFXObj_Contacts::Destroy` 0x8251c108 (also `Detach`); the dismount splice is left playing. |
| `0x8251c398` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:642` | /// `PlayHandGrabOnSound` 0x8251c398. |
| `0x8251c4a0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:650` | /// `PlayHandGrabOffSound` 0x8251c4a0. |
| `0x8251c598` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:657` | /// `CheckForHandGrabs` 0x8251c598. |
| `0x8251c6c8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:719` | /// `PlayFootCatch` 0x8251c6c8. |
| `0x8251c938` | TU3 | `crates/skate2-audio/src/player/contacts.rs:819` | /// `PlayFootCatchHelper` 0x8251c938; with Skate 3 physics TU3's 0x824b9508, which catches the |
| `0x8251c9d0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:845` | /// `CheckForFootCatches` 0x8251c9d0. |
| `0x8251cbb0` | TU3 | `crates/skate2-audio/src/player/contacts.rs:892` | /// `PlayFootScrapeSound` 0x8251cbb0 (TU3 0x824b97c8 plays the inverted cue during a Skate 3 |
| `0x8251cd60` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:913` | /// `CheckForFootScrapes` 0x8251cd60. |
| `0x8251cf00` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:540` | /// `GetOllieLandType` 0x8251cf00. |
| `0x8251cfe0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:554` | /// `GetPushSurface` 0x8251cfe0. |
| `0x8251d098` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:564` | /// `GetFootDragSurface` 0x8251d098. |
| `0x8251d158` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:578` | /// `GetSpliceIndexForLand` 0x8251d158: (bank, index, land type). |
| `0x8251d448` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1248` | /// `LandRail` 0x8251d448. |
| `0x8251d708` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:942` | /// `CheckForManual` 0x8251d708: true when the manual ended on three or four wheels. |
| `0x8251d8c0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:966` | /// `PlayDragForPowerSlide` 0x8251d8c0. |
| `0x8251d938` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:975` | /// `CheckForPushes` 0x8251db18 with `NewPushOnSplice` 0x8251d938 / `NewPushOffSplice` 0x8251da28 |
| `0x8251da28` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:975` | /// `CheckForPushes` 0x8251db18 with `NewPushOnSplice` 0x8251d938 / `NewPushOffSplice` 0x8251da28 |
| `0x8251db18` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:975` | /// `CheckForPushes` 0x8251db18 with `NewPushOnSplice` 0x8251d938 / `NewPushOffSplice` 0x8251da28 |
| `0x8251dd10` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:364` | /// `UpdateIntensityValues` 0x8251dd10. |
| `0x8251e128` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1615` | /// `GetSweetenerLevel` 0x8251e128: (level, low, high), left as given when the force is too low. |
| `0x8251e2d0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1684` | /// `CheckForFirstCollision` 0x8251e2d0: the first collision of a bail adds a torso hit. |
| `0x8251e448` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1717` | /// `CheckForBoardCollisions` 0x8251e448. |
| `0x8251e788` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1742` | /// `CheckForCollisionSpeech` 0x8251e788 without its speech: the wipe-out bookkeeping. |
| `0x8251f368` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:612` | /// `GetVolForWheelHit` 0x8251f368. |
| `0x8251f4c0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1398` | /// `UpdateFootCatches` 0x8251f4c0. |
| `0x8251f640` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1418` | /// `UpdateManual` 0x8251f640. |
| `0x8251f7d0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1351` | /// `UpdateStops` 0x8251f7d0. |
| `0x8251fb70` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1377` | /// `UpdatePushes` 0x8251fb70. |
| `0x8251ff00` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1408` | /// `UpdateFootScrapes` 0x8251ff00. |
| `0x825201d8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1480` | /// `UpdateHandGrabs` 0x825201d8: element 0 of the grab volumes scales the grab, element 1 the |
| `0x82520570` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:627` | /// `GetOllieL4Index` 0x82520570 / `GetLandL4Index` 0x82520678. |
| `0x82520678` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:627` | /// `GetOllieL4Index` 0x82520570 / `GetLandL4Index` 0x82520678. |
| `0x82520780` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:493` | /// `CheckForScore` 0x82520780: a skater's banked score (the local skater's rise in announcer |
| `0x82520920` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1581` | /// `GetHOMMaterial` 0x82520920. |
| `0x825209e8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1654` | /// `CanPlayCollisionSound` 0x825209e8: the Hall of Meat priority gate (`aud_general` "hom"). |
| `0x82520cd8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1494` | /// `CheckForKickUp` 0x82520cd8. |
| `0x82520e98` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1516` | /// `UpdateKickUp` 0x82520e98. |
| `0x82521048` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1528` | /// `CheckForAirDismount` 0x82521048. |
| `0x82521108` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1537` | /// `UpdateAirDismount` 0x82521108. |
| `0x825212b8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1567` | /// `UpdateBailAverage` 0x825212b8 (input 11). |
| `0x82526850` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:148` | /// `CheckForBoardFlips` 0x82526850: a flip instance for a trick in the air, or for a |
| `0x82526c60` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:292` | /// `CheckForTricks` 0x82526c60: a cloth instance per trick. |
| `0x82526e30` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:307` | /// `CheckForTricksDone` 0x82526e30: when a trick ends, its trick-done cloth instance, kept |
| `0x82527088` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:365` | /// `UpdateBoardFlips` 0x82527088: the flip instance follows the deck rotation and the mixer |
| `0x82527610` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:426` | /// `UpdateTricks` 0x82527610: the trick's cloth instance until a wipeout, or until the |
| `0x82527800` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:338` | /// `UpdateFunMeter` 0x82527800: toward the multiplier's fun target at the fall and rise |
| `0x82528468` | Skate 2? | `crates/skate2-audio/src/system.rs:881` | // SFXObj_Announcer::InitSFX 0x82528468 and SFXObj_CameraMan::InitSFX 0x82528748 turn |
| `0x825284f0` | Skate 2? | `crates/skate2-audio/src/system.rs:1616` | /// `SFXObj_Announcer::UpdateParams` 0x825284f0 / `SFXObj_CameraMan::UpdateParams` 0x825287d0: |
| `0x82528748` | Skate 2? | `crates/skate2-audio/src/system.rs:881` | // SFXObj_Announcer::InitSFX 0x82528468 and SFXObj_CameraMan::InitSFX 0x82528748 turn |
| `0x825287d0` | Skate 2? | `crates/skate2-audio/src/system.rs:1616` | /// `SFXObj_Announcer::UpdateParams` 0x825284f0 / `SFXObj_CameraMan::UpdateParams` 0x825287d0: |
| `0x82529b60` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:144` | /// destructor 0x82529b60 that releases the voice and the pool never runs. |
| `0x82529c90` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:304` | /// `PlayAmbience` 0x82529c90: a stage 0 voice (`SndPlayer1`, `Rechannel`, `Resample`, `Gain`, |
| `0x82529fa8` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:361` | /// `StopAmbience` 0x82529fa8: the player's stop event, then the voice's release. |
| `0x8252a058` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:163` | /// `GetAmbienceKey` 0x8252a058: the front end's ambience while `mbIsFE` and for 30 updates |
| `0x8252a220` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:254` | /// `UpdateAmbience` 0x8252a220: a fade-in runs `fadeIn` seconds, then the crossfade sound is |
| `0x8252a310` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:278` | /// `ProcessUpdate` 0x8252a310: the bed's gain (`volume` times mixer volume 0), resample |
| `0x8252a530` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:370` | /// `LoadResidentData` 0x8252a530: `SyncLoad("data\\audio/ambienceresident.big")` and an |
| `0x8252a918` | Skate 2? | `crates/skate2-audio/src/system.rs:1342` | /// `SFXObj_Master::UpdateParams` 0x8252a918. |
| `0x8252af30` | Skate 2? | `crates/skate2-audio/src/system.rs:1396` | /// `SFXObj_Master::ProcessUpdate` 0x8252af30 (`GetVolumeFromMixer` = `SndBase::GetDMixOutput` |
| `0x8252c000` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:116` | /// `Destroy` 0x8252c000, which `Detach` runs: the flip and the trick foley (the trick-done |
| `0x8252e288` | TU3 | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:1` | //! Stock normal-camera rig settings, bound by TU3 8252E288/8252EDD0. |
| `0x8252e680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:101` | // 8252E6CC binds heading; 8252E680 binds elevation. These are |
| `0x8252e6cc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:101` | // 8252E6CC binds heading; 8252E680 binds elevation. These are |
| `0x8252edd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:1` | //! Stock normal-camera rig settings, bound by TU3 8252E288/8252EDD0. |
| `0x8252ee80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:201` | // 8252EE80/8252EE90 read camera/dynamics layout624/576 into |
| `0x8252ee90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:201` | // 8252EE80/8252EE90 read camera/dynamics layout624/576 into |
| `0x8252f448` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:184` | // 8252F448/8252F508 attribute lookup low wordD1CA9582. |
| `0x8252f508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:184` | // 8252F448/8252F508 attribute lookup low wordD1CA9582. |
| `0x82531050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:156` | /// 82D181D0 stores it at13484, and82531050 emits that pose command. |
| `0x825310e0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/animation_owner.rs:18` | // S3 virtual48 ->825310E0 returns the current root at owner+13524. |
| `0x825310f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:195` | ///825310F0: retire completed transitions before advancing; reset property |
| `0x82531200` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:30` | ///82BB9E78 calls SkaterAnim virtual40=82531200, a direct phase14652 store. |
| `0x82531200` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:10` | /// Base animator14652: ctor82D17CDC sets zero; virtual40/82531200 writes it. |
| `0x82531278` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_execute.rs:17` | //End82BB5DB0 -> IAnimatable112 (8231E050+112) ->82531278: |
| `0x82531330` | Skate 2? | `crates/skate2-audio/src/reverb.rs:113` | /// level and instance 1 silent, then `InitializeFlange` 0x82531330. |
| `0x82531560` | Skate 2? | `crates/skate2-audio/src/reverb.rs:153` | /// `VerbInit` 0x82531560: slot `n`'s collection into `tpp` (filters panned 270 and 90 |
| `0x82531870` | Skate 2? | `crates/skate2-audio/src/reverb.rs:217` | /// `VerbUpdate` 0x82531870: the instance's level from its slot, gains, delays and notch from |
| `0x82531988` | Skate 2? | `crates/skate2-audio/src/reverb.rs:234` | /// `SetupWPVerb` 0x82531988: the other slot takes the collection at level 0 and a one-second |
| `0x82531a68` | Skate 2? | `crates/skate2-audio/src/reverb.rs:250` | /// `ProcTimeTrans` 0x82531a68: the target fades in linearly and the active slot out; at full |
| `0x82531b88` | Skate 2? | `crates/skate2-audio/src/reverb.rs:429` | /// `ResetDistTrans` 0x82531b88: the sphere slot's side wins at once and the volumes are |
| `0x82531ca0` | Skate 2? | `crates/skate2-audio/src/reverb.rs:8` | //! towards its centre (`ProcDistTrans` 0x82546c68, `PanTransition` 0x82531ca0), and a second |
| `0x82531ca0` | Skate 2? | `crates/skate2-audio/src/reverb.rs:510` | /// `PanTransition` 0x82531ca0: slot `target` fades to `1 - current` and the other to |
| `0x82532000` | Skate 2? | `crates/skate2-audio/src/reverb.rs:574` | /// `ProcessUpdate` 0x82532000: outputs 0-3 are flange 0's dry and wet and flange 1's, output |
| `0x82532b58` | Skate 2? | `crates/skate2-audio/src/system.rs:1417` | /// `SFXObj_NIS::UpdateParams` 0x82547ab8 with `GetNISType` 0x82532b58. |
| `0x82532f48` | Skate 2? | `crates/skate2-audio/src/system.rs:1476` | /// `SFXObj_Pause::UpdateParams` 0x82532f48. `AudioCore::StartSampleCapture` (`+0x50`) never |
| `0x825333a0` | Skate 2? | `crates/skate2-audio/src/system.rs:1534` | /// `SFXObj_Speech::UpdateParams` 0x825333a0. |
| `0x82533738` | TU3? | `crates/skate2-core/src/animation/runtime/skate3_gesture_streams.rs:160` | // AddSQT::EvalBlendPose 82533738 evaluates only its motion child (+0x14). Every |
| `0x82537038` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:626` | /// The constructor 0x82537038. |
| `0x825379a0` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:893` | /// `Destroy` 0x825379a0. |
| `0x82537af8` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:762` | /// `CheckForJumps` 0x82537af8: an off-board jump, a hippy jump or leaving a foot plant |
| `0x82537e48` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:881` | /// `UpdateJumps` 0x82537e48: the jump-up cloth only updates while playing (it stays allocated |
| `0x825380e8` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:735` | /// `CheckForCloth` 0x825380e8: off the board each new step replaces that foot's cloth splice |
| `0x82538400` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:691` | /// `CheckForFootSteps` 0x82538400: the feet's surfaces and AEMS instances, the local skater's |
| `0x82538eb0` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:807` | /// `UpdateCloth` 0x82538eb0: volume output 3, right foot first. |
| `0x82539168` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:814` | /// `UpdateRattle` 0x82539168: volume output 12, right foot first. |
| `0x82539420` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:821` | /// `UpdateFootSteps` 0x82539420: both feet's `playercharacter_footstep` members, then the |
| `0x8253a4a0` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:9` | //! Not ported: `CheckForOffBoardSpeech` 0x8253a4a0 (a non-local skater stepping back on the board |
| `0x8253ba48` | Skate 2? | `crates/skate2-audio/src/system.rs:1541` | /// `SFXObj_VU::UpdateParams` 0x8253ba48: `AudioCore::GetMusicVU`/`GetSpeechVU` read 0 with no |
| `0x8253c128` | Skate 2? | `crates/skate2-audio/src/system.rs:1549` | /// `SFXObj_Challenge::UpdateParams` 0x8253c128. |
| `0x8253c598` | Skate 2? | `crates/skate2-audio/src/system.rs:1647` | /// `SFXObj_HOM::UpdateParams` 0x8253c598. |
| `0x8253dc30` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:720` | /// `SFXCTL_PlayerPhysics::UpdateNicotine` 0x8253dc30. |
| `0x8253ecb0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:289` | /// `SFXObj_Contacts::SFXObj_Contacts` 0x8253ecb0. |
| `0x8253eec0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1000` | /// `PlayWheelHit` 0x8253eec0. |
| `0x8253f160` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:595` | /// `GetSpliceIndexForOllie` 0x8253f160: (bank, index, the `eOLMaterial` it writes). |
| `0x8253f350` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1121` | /// `TakeOffGround` 0x8253f350. |
| `0x8253f760` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1179` | /// `LandGround` 0x8253f760. |
| `0x8253fea8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1317` | /// `CheckForStops` 0x8253fea8 (creates the `Class_foot_drag` instance). |
| `0x82540328` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1596` | /// `GetImpactorMaterials` 0x82540328: the part's (material, cloth, sweetener, Hall of Meat) |
| `0x825404d8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1430` | /// `UpdateJumpsAndLands` 0x825404d8. |
| `0x82544060` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:124` | /// `UpdateParams` 0x82544060 (local skater only). |
| `0x82544118` | Skate 2? | `crates/skate2-audio/src/player/tricks.rs:141` | /// `ProcessUpdate` 0x82544118. |
| `0x82545168` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:142` | /// The constructor 0x82545168: `LoadResidentData`, then the 'Amb0' stream pool |
| `0x825452d0` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:211` | /// `ChangeAmbience` 0x825452d0, one step per update while the key differs: a playing bed starts |
| `0x82546b68` | Skate 2? | `crates/skate2-audio/src/reverb.rs:112` | /// `InitSFX` 0x82546b68: the first `aud_reverb` collection in both slots, instance 0 at full |
| `0x82546c68` | Skate 2? | `crates/skate2-audio/src/reverb.rs:8` | //! towards its centre (`ProcDistTrans` 0x82546c68, `PanTransition` 0x82531ca0), and a second |
| `0x82546c68` | Skate 2? | `crates/skate2-audio/src/reverb.rs:381` | /// `ProcDistTrans` 0x82546c68: while the followed volume stays current (or is replaced by one |
| `0x82546e18` | Skate 2? | `crates/skate2-audio/src/reverb.rs:9` | //! volume starts a sphere-to-sphere one (`ProcSprhSphrTrans` 0x82546e18). The slot LFO |
| `0x82546e18` | Skate 2? | `crates/skate2-audio/src/reverb.rs:449` | /// `ProcSprhSphrTrans` 0x82546e18: with one volume left its reverb wins; with both followed |
| `0x82547ab8` | Skate 2? | `crates/skate2-audio/src/system.rs:1417` | /// `SFXObj_NIS::UpdateParams` 0x82547ab8 with `GetNISType` 0x82532b58. |
| `0x82547e80` | Skate 2? | `crates/skate2-audio/src/system.rs:1151` | /// `SFXObj_NIS::GetNISVol` 0x82547e80 for the current scene, which |
| `0x82549858` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:1` | //! `Sk8::Audio::SFXObj_OffBoard` (592 bytes, PLAYER object 10, `CreateObject` 0x82549858) and the |
| `0x82549910` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:656` | /// `UpdateParams` 0x82549910. |
| `0x82549d20` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:799` | /// `ProcessUpdate` 0x82549d20. |
| `0x82549fa0` | Skate 2? | `crates/skate2-audio/src/system.rs:1638` | /// `SFXObj_Bloom::UpdateParams` 0x82549fa0. |
| `0x8254a400` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:535` | /// `SFXCTL_PlayerPhysics::UpdateParams` 0x8254a400. `skater` is `GetSkater(arrayIndex)`; |
| `0x8254af78` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1036` | /// `CheckForWheelHits` 0x8254af78. |
| `0x8254b4d0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1276` | /// `CheckForJumpsAndLands` 0x8254b4d0. |
| `0x8254b680` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:1762` | /// `CheckForBodyCollisions` 0x8254b680. |
| `0x8254c1c8` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:526` | /// `SFXObj_Contacts::ProcessUpdate` 0x8254c1c8 (the state is attached). |
| `0x8254dc78` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:3` | //! (0x58 bytes, TypeInfo 0x00020000, `CreateObject` 0x8254dc78), the location's ambience bed. The |
| `0x8254dd30` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:189` | /// `UpdateParams` 0x8254dd30: follows the key (`UpdateAmbience` while it holds, |
| `0x8254e1e0` | Skate 2? | `crates/skate2-audio/src/reverb.rs:301` | /// `UpdateParams` 0x8254e1e0 with a `Presentation` (`AudioSystem::GetReverbKey` 0x824f1df8 |
| `0x8254f1e0` | Skate 2? | `crates/skate2-audio/src/system.rs:1572` | /// `SFXObj_Menu::UpdateParams` 0x8254f1e0. |
| `0x8254f8a0` | Skate 2? | `crates/skate2-audio/src/player/contacts.rs:456` | /// `SFXObj_Contacts::UpdateParams` 0x8254f8a0 (the state is attached), for the skater at |
| `0x8254fc18` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:2` | //! 0x8254fc18 creates one `CSTATE_Environment` with `Setup(1)`: `Sk8::Audio::SFXObj_Ambience` |
| `0x8254fde0` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:431` | /// `CSTATEMGR_Player::EnterGamePlay` 0x8254fde0: two states, each with every player object. |
| `0x8254fee0` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:484` | /// `CSTATEMGR_Player::GetNPCWheelType` 0x8254fee0: the opposite of the first local state's |
| `0x82550250` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:309` | /// `aud_emitter::crowd_aems_id` 0x82550250: the crowd group a crowd volume plays. |
| `0x82550430` | Skate 2? | `crates/skate2-audio/src/player/physics.rs:54` | /// 0x82550430, the first attached DYNAMICOBJECT state's non-zero key. |
| `0x825515d8` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:333` | /// `CSTATE_Player::Attach` 0x825515d8: the state data, `mWheelType = Hard`, then |
| `0x82551630` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:360` | /// `CSTATE_Player::Detach` 0x82551630 / `CSTATE_Base::Detach` 0x82a7e350. |
| `0x82554210` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:437` | /// `CSTATEMGR_Crowd::StopCrowds` 0x82554210, which ends with `UnloadCrowdData`. |
| `0x825561f8` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:568` | /// `CSTATE_Player::IsInRange` 0x825561f8: within 30 m of the camera microphone. |
| `0x82556c60` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:447` | /// `CSTATEMGR_Player::UpdateParams` 0x82556c60: the states first, then attach/detach. |
| `0x82557810` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:430` | /// `CSTATEMGR_Crowd::StartCrowds` 0x82557810 with the crowd banks' status and the local skater. |
| `0x825587b8` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:491` | /// `CSTATE_Player::UpdateParams` 0x825587b8 and `CSTATE_Base::UpdateParams` 0x82a7e0e0. |
| `0x825588d8` | Skate 2? | `crates/skate2-audio/src/system.rs:1748` | // `CSTATEMGR_Crowd::UpdateParams` 0x825588d8: a start that waited for the crowd banks, |
| `0x8255cd08` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:78` | // UnlockablesBackEnd::LoadData 0x8255cd08 over the defaults Init added. |
| `0x82563a88` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:96` | /// `UpdatePlayerName` 0x82563a88's `GM_PartyPlay` name for a player: |
| `0x82567450` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:574` | /// ReplayEditorBackEnd::Start82567450/Stop825675d0 own pause state7. |
| `0x825675d0` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:574` | /// ReplayEditorBackEnd::Start82567450/Stop825675d0 own pause state7. |
| `0x82569948` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_scoring.rs:1066` | /// PDB BailScoringOutData (0x4f0), copied by HOMResultsData.Init82569948. |
| `0x82581bf8` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:108` | /// `GetCameraSkater82581bf8` is GameCameraManager+0xd0. When it is the |
| `0x82581c08` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:71` | /// Camera observer82581c08 forwards to folded825187c0, an actual blr. |
| `0x82581c10` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:349` | /// Destroyed message and `CameraManager::OnSkaterDestroyed` 0x82581c10 hands the Actor to |
| `0x82581c10` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:94` | /// `CameraManager::OnSkaterDestroyed` 0x82581c10 -> `GameCameraManager::OnSkaterDestroyed` |
| `0x82584cdc` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:95` | ///825c1298 (`GameInit`), then Camera.Init82584cdc. Queue is a retained earlier |
| `0x82585dd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab.rs:82` | //82585DD8 descriptor initialization followed by827602E0 actual data request. |
| `0x8258c1e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:8` | /// 8258C1E0 still executes center/half-extent arithmetic when the supplied |
| `0x8258db98` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/math.rs:80` | fn wrap_angle_8258db98(&mut self, mut a: f32) -> f32 { |
| `0x8258db98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:75` | //8258DB98 folds the helper's[0,2pi] output before the direction test. |
| `0x8258f180` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:17` | /// Actor20 resolves IMotionGraph (8258F180); virtual16 returns this map. |
| `0x8258f180` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_execute.rs:12` | //8259176C binds Actor+20 to IMotionGraph (8258F180); vtable |
| `0x8258f330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_execute.rs:13` | //823006D0+16 ->8258F330 returns MG+12, its authored intention map. |
| `0x8258f488` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:209` | //8258F488 and final reset825953B0 explicitly zero the complete push |
| `0x8258f488` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:281` | //8258F488 seeds bit24, and reset825953B0 preserves that bit. |
| `0x8258f488` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:132` | //8258F488 initializes full5912/5916; final reset825953B0 clears |
| `0x8258f488` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:115` | /// Per-MotionGraph generator. Original8258F488 seeds this once; its reset |
| `0x8258f5b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:122` | ///Constructor8258F5B8 clears it; Reset82595480 preserves it. |
| `0x8258f6e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind.rs:92` | // Actor1800 slot16 ->8258F6E0 AddMgAttribute, NOT InsertIntent. |
| `0x8258f6e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:75` | //MGspecific v16=8258F6E0 appends attributes, not animation parameters. |
| `0x8258f6e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:125` | //82BA9434/9628: actor1800 interface slot16 ->8258F6E0 appends |
| `0x8258f7c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:124` | /// Specific MotionGraph+3156, GetLastGoodLandingVelocity8258F7C8. |
| `0x8258f870` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:26` | //82BACB40/82BACB90: v80 ->8258F870, CA4 bit30. |
| `0x8258f898` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:32` | //82BACBE0/82BACC30: v88 ->8258F898, CA4 bit29. |
| `0x8258f8c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:38` | //82BACC80/82BACCD0: v96 ->8258F8C0, CA4 bit28. |
| `0x8258f8d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_conditions.rs:77` | //82BA56B8 uses specific MotionGraph virtual100=8258F8D8 bit27. |
| `0x8258f8e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:44` | //82BBDFF8/82BBE048: v104 ->8258F8E8, CA4 bit27. |
| `0x8258f900` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:87` | ///SpecificCA4 bit26, getter8258F900/setter8258F910; reset clears it. |
| `0x8258f910` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:87` | ///SpecificCA4 bit26, getter8258F900/setter8258F910; reset clears it. |
| `0x8258f910` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:115` | //Begin82BACD20/End82BACD70 -> specific setter8258F910. |
| `0x8258f930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:172` | //82BAE8D0 is both Begin and End; v136=8258F930 clears3224. |
| `0x8258f940` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:220` | } //82BAE848/8258F940 |
| `0x8258f968` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:205` | //controller delta through8258F968, with no imposed maximum. |
| `0x8258f980` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:225` | //82BB9260/8258F980 |
| `0x8258f998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:229` | } //82BB91A8/8258F998 |
| `0x8258fa20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_scoring_trick.rs:23` | //8258FA20: oris flags,0x100; copies the two input names verbatim. |
| `0x8258fa20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:171` | // ISkaterMotionGraph236 ->8258FA20 stores both equal names and bit24. |
| `0x8258fb30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:39` | ///8258FB30 writes specific MotionGraph+CE4 and sets byteCE8 to1. |
| `0x8258fb30` | Skate 2 | `crates/skate2-core/src/animation/runtime/actor_skate3_gestures.rs:64` | /// The gesture fact Skate 2's MotionGraph publishes (`8258FB30`). |
| `0x8258fb38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:41` | ///8258FB38 writes byteCE9 from direction==Down. |
| `0x8258fb60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:77` | //82BA4D50 -> ISkaterMotionGraph256 ->8258FB60 ->82970628. |
| `0x8258fb68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:265` | //82BA79A0 calls the specific MotionGraph getter8258FB68. |
| `0x8258fb68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:121` | ///Specific MotionGraph CA4 bit23, getter8258FB68/setter8258FB78. |
| `0x8258fb78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:121` | ///Specific MotionGraph CA4 bit23, getter8258FB68/setter8258FB78. |
| `0x8258fbf0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:87` | //82BBB328 ->MGv200/8258FBF0;82BBB33C ->ISkaterAnimv116/82B97228. |
| `0x8258fc30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:136` | //SpecificMotionGraph virtual212=8258FC30, bit18. Native reset clears it. |
| `0x82590028` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:225` | //Represented pose fields after AnimOutPhysIn Reset82590028. |
| `0x825903c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:35` | // TU3 82DB4330 calls 825903C8: 82590414 reads action-table +0x104 |
| `0x825903c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:150` | fn transition_action_825903c8(&mut self) -> Result<f32, String> { |
| `0x82590414` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:35` | // TU3 82DB4330 calls 825903C8: 82590414 reads action-table +0x104 |
| `0x82590b50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:143` | /// GetCACSettings82590D20/82590B50: profile selection is independent of |
| `0x82590b50` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1100` | /// GetCACSettings82590D20/82590B50: profile refresh does not invent a |
| `0x82590cc0` | Skate 2? | `crates/skate2-core/src/assets/character.rs:327` | /// compared by its `RwHash32Buffer` (`less` 0x82590cc0, unsigned). The |
| `0x82590d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:139` | // GetCACSettings82590D20..D84: 0=null,1=Loose,2=Gonzo,3=Aggressive. |
| `0x82590d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:143` | /// GetCACSettings82590D20/82590B50: profile selection is independent of |
| `0x82590d20` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1100` | /// GetCACSettings82590D20/82590B50: profile refresh does not invent a |
| `0x82591448` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:262` | // Actor82591448 passes network \|\| ghost for self-pair suppression. An online |
| `0x8259147c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:154` | /// Actor8259147C evaluates the stored initialization pose before creating |
| `0x8259176c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_execute.rs:12` | //8259176C binds Actor+20 to IMotionGraph (8258F180); vtable |
| `0x82591e30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:273` | //82591E30 returns immediately for offboard, before the velocity branch. |
| `0x82591e30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:287` | ///82591E30 retains the source axes at low speed or near vertical travel. |
| `0x82592518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:20` | /// Actor82592518's request from this tick's Teleporting update. |
| `0x82592518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:115` | ///Actor82592518 saves stance then queues the ordinary deferred reply825926F8. |
| `0x82592518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:152` | /// The Actor's side of the tick, in its order: the Teleporting callback's reply (82592518), then |
| `0x82592518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:1` | //! Host checkpoint response to original actor reset callback82592518. |
| `0x82592518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:15` | /// Original actor82592518 inverts checkpoint byte68. Its ground validator |
| `0x82592518` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:968` | /// The Actor's update after the source tick: Actor82592518's reply to this tick's reset |
| `0x825926c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:128` | // 825926C0..E8: SpawnFootForwardStance, then the same Actor teleport (vtable +0x70, |
| `0x825926f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:162` | ///Actor825926F8 reply ->10704/10768/10784, then82DB5BE0 maps physical input. |
| `0x825926f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:115` | ///Actor82592518 saves stance then queues the ordinary deferred reply825926F8. |
| `0x825926f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:129` | // 0x825926f8, no adjust) a marker target takes; 82593640 and 82DB5BE0 copy it raw to the |
| `0x825926f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:89` | ///825926F8 stores the reply and marks it for the next input publication. |
| `0x82592a00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:245` | //82592A00 -> SkateboardReckoning64 ->82C01BF8: solved deck, with bit20 flip. |
| `0x82592b68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:277` | /// Actor::GetFootForwardWithFakie 0x82592b68. |
| `0x82592b68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:89` | ///82592B68: physical leading foot, including the fakie inversion. |
| `0x82592b68` | TU3 | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:725` | /// The S3 SkaterAnim's leading foot with fakie (82592B68), RIGHT as true: markers and |
| `0x82592c08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:95` | ///82592C08/82B97350: save15200; ResetToGivenStance consumes it later. |
| `0x82592fd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:2` | //! Native actor phases82592FD8/82593128/82593230/82593640 preserve this order. |
| `0x82592fd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:245` | ///Actor Phase1/2/3 plus SetUpPhysics:82592FD8,82593128,82593230,82593640. |
| `0x8259303c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:418` | //8259303C..78: on-board category keeps this true even when the |
| `0x82593128` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:2` | //! Native actor phases82592FD8/82593128/82593230/82593640 preserve this order. |
| `0x82593128` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:245` | ///Actor Phase1/2/3 plus SetUpPhysics:82592FD8,82593128,82593230,82593640. |
| `0x82593230` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:2` | //! Native actor phases82592FD8/82593128/82593230/82593640 preserve this order. |
| `0x82593230` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:245` | ///Actor Phase1/2/3 plus SetUpPhysics:82592FD8,82593128,82593230,82593640. |
| `0x82593640` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:2` | //! Native actor phases82592FD8/82593128/82593230/82593640 preserve this order. |
| `0x82593640` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:129` | // 0x825926f8, no adjust) a marker target takes; 82593640 and 82DB5BE0 copy it raw to the |
| `0x82593640` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:245` | ///Actor Phase1/2/3 plus SetUpPhysics:82592FD8,82593128,82593230,82593640. |
| `0x82593640` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:350` | // Actor SetUpPhysics82593640 resets the output after animation has |
| `0x825936ac` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:359` | //Actor825936AC..D4 passes this fixed physical step even when |
| `0x82593760` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:127` | //82593760/70 copies that list into the actual physical input packet. |
| `0x825937ec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:1` | //! Owned PlayerInput packet storage; original actor publication825937EC. |
| `0x825938fc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:77` | // Actor825938FC/393C consumes these request bits after publication. |
| `0x82593924` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:141` | //82593924 reads Actor1904 bit25, not the input-filter word1908. |
| `0x82593988` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:53` | //82593988/94. The host allows users to change these independently. |
| `0x82593998` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1105` | /// The Actor reset 82593998/825939B0 reads the local skater's own PlayersBackEnd trucks and |
| `0x825939b0` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1105` | /// The Actor reset 82593998/825939B0 reads the local skater's own PlayersBackEnd trucks and |
| `0x82594310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:204` | //GenerateActionGraphIntents82594310 clears the AG map through82BC1B68 |
| `0x82594aa8` | TU3 | `crates/skate2-core/src/physics/skate3_player.rs:1063` | // TU3 Actor+44 virtual24 (82594AA8): ForceBrake stops a gesture start only while front-end |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:209` | //8258F488 and final reset825953B0 explicitly zero the complete push |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:281` | //8258F488 seeds bit24, and reset825953B0 preserves that bit. |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute.rs:87` | //825953B0's represented selective MG output reset. |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing.rs:88` | //825953B0 clears bits30..27 and sets bit25. |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:37` | ///825953B0: filtered intents, current MG attrs, selective scalar writes, |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:132` | //8258F488 initializes full5912/5916; final reset825953B0 clears |
| `0x825953b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:116` | ///825953B0 leaves the sequence intact. Locking only gives the host shared |
| `0x82595480` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:122` | ///Constructor8258F5B8 clears it; Reset82595480 preserves it. |
| `0x825954c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_spin.rs:82` | ///825954C0 ->82595670, original source guards and strict comparisons. |
| `0x82595670` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_spin.rs:82` | ///825954C0 ->82595670, original source guards and strict comparisons. |
| `0x82595930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:10` | /// PhysOutGround32 (Reckoning752 Z), original slide helper82595930. |
| `0x82595930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:1` | //! PowerSliding Update82BB28E8, instance82BB2D70 and helper82595930. |
| `0x82595930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:151` | ///82595930 normalizes Motion80 with two root and reciprocal refinements, |
| `0x82595930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:248` | /// PowerSlideSpin Begin82BB33C8; the input is GetSlideDirection82595930. |
| `0x825959d4` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:156` | //825959D4 ble also takes the unordered branch after fcmpu. |
| `0x82595b28` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:90` | /// None only for an actor without a physical interface, as in82595B28. |
| `0x82595b28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:141` | //82BAE210 ->82595B28; output writes are begin-only and ordered X,Y. |
| `0x82595d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:79` | /// this game.82595D08 sets a name and ORs a bit in the graph's score packet. |
| `0x825999f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:376` | // Fill825999F0/8259C260 publishes discrete off-board inputs before the |
| `0x8259a148` | TU3 | `crates/skate2-core/src/physics/skate3_player.rs:1535` | // TU3 Fill 8259A148..A170 reads PhysOut+0x38 (PhysOutAnimation) byte 156, the animation |
| `0x8259affc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:382` | // Fill8259AFFC..B088 emits the four analog AG inputs every frame. |
| `0x8259b1f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:2` | //! Fill8259B1F0..B7D8. Runs on the existing 60 Hz gameplay input boundary. |
| `0x8259b878` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:1` | //! GameInputManager82696030/826962D8 -> listener8259B878/8259B9D0 -> |
| `0x8259b9d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:1` | //! GameInputManager82696030/826962D8 -> listener8259B878/8259B9D0 -> |
| `0x8259bf50` | TU3? | `crates/skate2-core/src/animation/runtime/skate3_gesture_playback.rs:85` | /// Fill8259BF50 consumes the pad's original D-pad and bumper history. |
| `0x8259c260` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:376` | // Fill825999F0/8259C260 publishes discrete off-board inputs before the |
| `0x8259c650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls/object_settings.rs:1` | //! The original inputlistener_default fields fetched by8259C650..C75C. |
| `0x825b84e0` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:348` | /// empty function, `ChallengeAIManager::OnSkaterDestroyed` 0x825b84e0 sends the player's |
| `0x825c1298` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:95` | ///825c1298 (`GameInit`), then Camera.Init82584cdc. Queue is a retained earlier |
| `0x825ecd6c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2915` | /// Reset825ecd6c..a4 stores identity axes and an entirely zero W axis. |
| `0x825ed1c8` | Skate 2? | `crates/skate2-core/src/actors/actor_teleport.rs:1` | //! Actor::Teleport 0x825ed1c8..0x825ed220, after a separate source-selected |
| `0x825ed220` | Skate 2? | `crates/skate2-core/src/actors/actor_teleport.rs:1` | //! Actor::Teleport 0x825ed1c8..0x825ed220, after a separate source-selected |
| `0x825ed228` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:293` | /// PreventManualRespawn825ed228 writes the existing one-shot input bits. |
| `0x825ed560` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:410` | /// `Actor::GetFootForwardWithFakie` 0x825ed560 calls the published |
| `0x825ed638` | Skate 2? | `crates/skate2-core/src/actors/actor_teleport.rs:64` | /// `Actor::SpawnFootForwardStance`825ed638: `SetSpawnFootForwardStance` on |
| `0x825ed698` | Skate 2? | `crates/skate2-core/src/actors/actor_teleport.rs:88` | /// `Actor::SetTransform` 0x825ed698: `mSetTransformCount` counts it, then |
| `0x825ed70c` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:244` | /// `Actor::UpdatePhase2`825ed70c..738: Skater `SetAttributes`, BS dispatch, optional wait. |
| `0x825edad0` | Skate 2 | `crates/skate2-core/src/session/selected_player_tick.rs:907` | // (ForceWipeOut 825edad0) throws it along the shove. |
| `0x825f09a8` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:259` | /// `CalcRespawnTransform` 0x825f09a8. |
| `0x825f0e18` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:359` | /// `Actor::TeleportSomewhereAppropriate` 0x825f0e18. The AI owner handles |
| `0x825f0ecc` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:398` | // 0x825f0ecc..ef0 inline Actor::SpawnFootForwardStance with the |
| `0x825f1040` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_dispatch.rs:1` | //! `DispatchBodyAnim825f1040` / `EvalCurrAnim82d6e658` and the later body ACS Sync. |
| `0x825f1040` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_session.rs:2` | //! References: `DispatchBodyAnim825f1040` and `UpdatePhysics825ffd98`. |
| `0x825f10f0` | TU3 | `crates/skate2-core/src/physics/player/skate3_actor_input.rs:2` | //! S2 GenerateActionGraphIntents825FDA28 and Actor.Update825F10F0 own the |
| `0x825f10f0` | TU3 | `crates/skate2-core/src/session/selected_player_tick.rs:824` | // Actor::Update825f10f0 starts with its AIController::Update (a Skate 3 physics |
| `0x825f1138` | Skate 2? | `crates/skate2-core/src/actors/actor_teleport.rs:33` | /// PC mod counterpart of Actor.Update825f1138's reset acknowledgement. |
| `0x825f1368` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:294` | /// `Simulation::GetCameraManager` for UpdateSkaterOpacity825f1368: the same |
| `0x825f19c0` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:492` | /// ISkaterManager vtable +0x18 is `FindByIndex`825f19c0. |
| `0x825f1b8c` | Skate 2 | `crates/skate2-core/src/physics/player/skate3_spawn.rs:1` | //! PC offboard reset placement. S2 AdjustSpawnPosition825F1B8C adds .15m, |
| `0x825f4264` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:111` | /// Actor825f4264..42c0. Concrete vtable+D0 is `IsMounting82cb7668`; |
| `0x825f4410` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_session.rs:151` | // Native825f4410 asserts IPhysical before Was/AddTime/Toggle. |
| `0x825f443c` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:630` | /// Selected `Actor::UpdatePhase3`825f443c..4680. The outer owner must execute |
| `0x825f44f8` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_session.rs:183` | /// Actor825f44f8..45ac: separate guarded PhysOut reads in original order. |
| `0x825f45b0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:667` | // 825f45b0: both locomotion bits set suppress this phase's Dispatch. |
| `0x825f53b0` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:937` | // SkaterManager::UpdateLocomotion 825f53b0 over the skater list. |
| `0x825f87b8` | Skate 2? | `crates/skate2-core/src/animation/hands_face/actor_animation_hands_face_runtime.rs:1` | //! Actor825f87b8 source dispatch order; manager owns outer hand command sets. |
| `0x825f8ba4` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_postupdate.rs:1` | //! SkaterManager::PostUpdate 0x825f8ba4..8c08 and breadcrumb Update82ce2230. |
| `0x825f9d98` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:62` | // IActorQuery thunk825f9d98 subtracts34, then folded82485d18 reads Actor+1d8. |
| `0x825f9df0` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:436` | //Actor.GetPosition825f9df0 reads PhysOut_Skeleton+1a0, |
| `0x825fa310` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:728` | // UpdateAnimation2 825fa310, then UpdateAnimation3 825fa4e0: Phase3 for every |
| `0x825fa4e0` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:728` | // UpdateAnimation2 825fa310, then UpdateAnimation3 825fa4e0: Phase3 for every |
| `0x825fb350` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:58` | /// SkaterManager825fb350/358/368: actual empty list and no dispatch. |
| `0x825fda28` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:1` | //! SAME Actor/action-map `GenerateActionGraphIntents`, reference 0x825fda28. |
| `0x825fda28` | TU3 | `crates/skate2-core/src/physics/player/skate3_actor_input.rs:2` | //! S2 GenerateActionGraphIntents825FDA28 and Actor.Update825F10F0 own the |
| `0x825fe850` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:1` | //! Retained selected `Actor` constructor prefix, 0x825fe850..0x825fec20. |
| `0x825fec20` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:1` | //! Retained selected `Actor` constructor prefix, 0x825fe850..0x825fec20. |
| `0x825fec58` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:517` | // Actor+1f0 store825fec58 is after all three native registrations. |
| `0x825fec5c` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:531` | /// Source Actor 0x825fec5c..0x825fecc8. This ends before animation, |
| `0x825fecc8` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:531` | /// Source Actor 0x825fec5c..0x825fecc8. This ends before animation, |
| `0x825fedbc` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:800` | /// Actual Actor allocation825fedbc uses Sim category55: maskFFFFFFFF, |
| `0x825fee04` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:813` | /// the constructor, in its own source order 825fee04..825feec0. |
| `0x825feec0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:813` | /// the constructor, in its own source order 825fee04..825feec0. |
| `0x825ff2d8` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:740` | // Actor825ff2d8 selects Sim, whose category OR0x100 clears the HiLOD allocation. |
| `0x825ff300` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:448` | /// Actor constructor 0x825ff300..444: physical publication precedes the |
| `0x825ffc70` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:708` | // SkaterManager::UpdateAnimation1 825ffc70: Phase1 over the skater list. An AI |
| `0x825ffd98` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:806` | // UpdatePhysics 825ffd98: each skater's ACS sync, SetUpPhysics and Actor::Update. |
| `0x825ffd98` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_session.rs:2` | //! References: `DispatchBodyAnim825f1040` and `UpdatePhysics825ffd98`. |
| `0x825ffea8` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_session.rs:315` | /// `UpdatePhysics825ffea8` synchronizes this Actor before its update-mask test. |
| `0x825ffef8` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:323` | /// Created Notify8260036c, followed by actual `UpdateLocalPlayer825ffef8`. |
| `0x825ffef8` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:393` | /// `UpdateLocalPlayer` 0x825ffef8: the local Actor is `FindByIndex(mLocalIndex)`; |
| `0x825fff4c` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:438` | // UpdateLocalPlayer825fff4c..74: only after all callbacks, capture |
| `0x8260036c` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:323` | /// Created Notify8260036c, followed by actual `UpdateLocalPlayer825ffef8`. |
| `0x82600380` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:371` | /// `SetLocalIndex` 0x82600380: with the ghost system on, an offline game's |
| `0x82600630` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:97` | /// (`0x82600630`) makes another: the constructor's (`0x82ce33b8`) spawn position, override |
| `0x82600920` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:278` | /// `SkaterManager::OnCommand` 82600920 case 7: `Actor+0x258` bit `0x04000000` |
| `0x82600920` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:595` | // SkaterManager::OnCommand82600920 for receiver 1, in batch |
| `0x82600920` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:96` | /// `SkaterManager::OnCommand` case 3 (`0x82600920`) destroys it and `SpawnPlayer` |
| `0x826150b0` | Skate 2? | `crates/skate2-audio/src/system.rs:917` | /// `FE::GlobalFEPlaySound` 0x826150b0: plays only keys whose `fe` collection names a sound. |
| `0x826151a0` | Skate 2? | `crates/skate2-audio/src/system.rs:926` | /// `FE::GlobalFEStopSound` 0x826151a0: the same gate, then `StopMenuSound`. |
| `0x82619580` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/hud_frame.rs:33` | // RefreshTrickDisplay82619580: all aerial types and grabs except coffin. |
| `0x8261b820` | Skate 2? | `crates/skate2-audio/src/system.rs:142` | /// `Attrib::StringToLowerCaseKey`. `FrontEndManager::RegisterAudioEnums` 0x8261b820 registers |
| `0x82629cd0` | Skate 2? | `crates/skate2-audio/src/system.rs:1176` | /// 0x82639b00, `PlayPlaylist` 0x82629cd0, `sMethod_Stop` 0x82637ea0): a `music::ipod_trigger` |
| `0x82629fc8` | Skate 2? | `crates/skate2-audio/src/system.rs:1170` | /// SwitchMusicMode` 0x82629fc8 and `RevertMusicMode` 0x8262a088 map the profile's music mode. |
| `0x8262a088` | Skate 2? | `crates/skate2-audio/src/system.rs:1170` | /// SwitchMusicMode` 0x82629fc8 and `RevertMusicMode` 0x8262a088 map the profile's music mode. |
| `0x82637ea0` | Skate 2? | `crates/skate2-audio/src/system.rs:1176` | /// 0x82639b00, `PlayPlaylist` 0x82629cd0, `sMethod_Stop` 0x82637ea0): a `music::ipod_trigger` |
| `0x826381a0` | Skate 2? | `crates/skate2-audio/src/system.rs:1205` | /// `MusicPlayerCmdTranslator::sMethod_GetDefaultSongList` 0x826381a0: `MusicCore:: |
| `0x82639b00` | Skate 2? | `crates/skate2-audio/src/system.rs:1176` | /// 0x82639b00, `PlayPlaylist` 0x82629cd0, `sMethod_Stop` 0x82637ea0): a `music::ipod_trigger` |
| `0x82641648` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:500` | /// TranslateEHudInputBit82641648 accepts FE keycodes16..19/32, which |
| `0x826477b0` | TU3? | `crates/skate2-core/src/session/skate3_input_focus.rs:3` | //! top PauseMan state used by CheckForPauseState826477b0. |
| `0x82647910` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:580` | // StateManagerFrontEnd::PauseGame82647910 / UnpauseGame: FRONTEND=3. |
| `0x8264a870` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:181` | /// `FrontEndState_PartyPlay::OnEnter` 0x8264a870's lists: the FreeSkate group's skaters and |
| `0x8264a9e0` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:246` | /// down with the screen (`OnLeave` 0x8264a9e0). |
| `0x8264aac8` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:59` | /// `OnUpdate` 0x8264aac8's per-player `SetPlayerCharacter`, `CreatePlayer(i, PT_Human, |
| `0x8264ad08` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:56` | /// `EnterGame` 0x8264ad08's `ResetPlayers` and `LoadPartyMember`s, each of whose |
| `0x8264ad08` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:259` | /// `EnterGame` 0x8264ad08 shuts the viewer down (`SimCharacterViewer::Shutdown`) and hands |
| `0x8264b268` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:372` | /// The marquee counterpart of `try_replace_cac`: `UpdateGameSettings` 0x8264b268's |
| `0x82656f78` | Skate 2? | `crates/skate2-audio/src/system.rs:174` | /// The music state `UpdateFEAudioState` 0x82d67948 hands `ChangeFEMusicState` 0x82656f78: the |
| `0x82681338` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:180` | /// `OnMenuSelectGame` 0x82681338 row 3: the screen `UpdateFEState` pushes once the NIS ends, with |
| `0x82682b68` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:555` | /// LoadInputMap826de438 and Replay2::ReceiveInput82682b68. Frontend |
| `0x82695438` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/controllers.rs:116` | // line after PlayerInputData's Pack/Unpack (82898920 -> 82695438). |
| `0x82695a68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:26` | // Original constructor82695A68 insertion order; each file competes |
| `0x82696030` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:1` | //! GameInputManager82696030/826962D8 -> listener8259B878/8259B9D0 -> |
| `0x826962d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:1` | //! GameInputManager82696030/826962D8 -> listener8259B878/8259B9D0 -> |
| `0x82699230` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/controllers.rs:135` | /// TU3 82699230 drains first, then updates each Pad once. The game owns |
| `0x826bed80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:4` | //! (`LivingWorld::Manager` 0x826bed80, answered by the host) and tests it against the strike zone |
| `0x826bed80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:11` | /// 0x826bed80's body margin (the folded constant IDA names `ROPA::cv_NormDistanceScale`). |
| `0x826bed80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:125` | /// 0x826bed80's per-pedestrian test: not wiped out, and the point within the skater collision |
| `0x826bed80` | TU3 | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:2` | //! pedestrian query (`LivingWorld::Manager` vslot 0x44, 0x826bed80) and the game's response to a |
| `0x826bed80` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:12` | /// 0x826bed80's `GetPedestriansWithinRange`: the folded constants 2.0 and 1.5, at most six. |
| `0x826bed80` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:24` | /// 0x826bed80 tests of each. |
| `0x826c0630` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query.rs:46` | /// Original826C0630: dynamic provider, then vehicle provider. |
| `0x826c0848` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:528` | /// PhonelistHUD UpdateInput826c0848: LB maximizes; AptOptionReleased |
| `0x826c3480` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:1` | //! Owner-qualified live Part get/set, 0x826c3480 / 0x82b25480. |
| `0x826c3480` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:179` | /// Source-facing `Part::GetTransform` boundary (0x826c3480). Detached cache |
| `0x826c3480` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_scoring.rs:1074` | // Native Part.GetTransform826c3480 loads adjacent body metadata |
| `0x826d6648` | Skate 2? | `crates/skate2-audio/src/system.rs:940` | /// 0x826d6648): the killed-it and owned-it blooms. Ids from 100 are `PlayLoadScreenData`'s |
| `0x826dab98` | TU3 | `crates/skate2-core/src/skate3/skate3_dlc.rs:6` | //! Skate 3 reads a pack's archive through its `dlcbig:` device (TU3 `0x826dab98`): each |
| `0x826de438` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:555` | /// LoadInputMap826de438 and Replay2::ReceiveInput82682b68. Frontend |
| `0x826de5e8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:501` | /// LoadInputMap826de5e8..618/6e8 binds to150..153/166. The generic |
| `0x826e2788` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:50` | /// Native `InputManager::Init826e2788` precedes Juice construction/Start in |
| `0x826e27ec` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:130` | // `InputManager::Init826e27ec` loads the map between device-manager |
| `0x826f3b48` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:276` | /// VerifyChallengeOkToUse828ab6d0 calls ChaseIsOn826f3b48. |
| `0x826f5218` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:291` | // Manager.AddAllEntitiesRAINPoints826f5218 visits pedestrians first, |
| `0x826f52a8` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:595` | /// Manager.UpdateTransforms826f52a8 calls this after pedestrian transforms |
| `0x82702294` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:159` | // Reset assigns before its non-null assertion (0x82702294/22a0). |
| `0x827022d4` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:168` | // Reset827022d4 samples actual ChaseIsOn (numChasers !=0), then |
| `0x82702eb8` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:93` | /// InitLWMan82702eb8 borrows Simulation's broadcaster; no private LW queue. |
| `0x827030c4` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:291` | // InitLWMan827030c4 publishes the embedded pedestrian manager before |
| `0x8270fe90` | Skate 2? | `crates/skate2-audio/src/system.rs:254` | /// Recorded ChaseMan.GetAudioState8270fe90 / GlobalData count and flags. |
| `0x82710cc0` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:104` | /// Live Manager counter82710cc0; dormant physical constructor snapshot |
| `0x82715878` | Skate 2? | `crates/skate2-core/src/actors/actor_construction.rs:139` | // Actual ctor82715878/88. |
| `0x82715a10` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:357` | /// preserve the native cross-manager phase ordering82715a10. |
| `0x82715ad0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:441` | /// preframe tasks, not just the vehicle tasks82715ad0/82715ae8. |
| `0x82715ae8` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:441` | /// preframe tasks, not just the vehicle tasks82715ad0/82715ae8. |
| `0x8271b4e8` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:221` | /// UpdateOnRoadPoints8271b4e8's skater half. One local Actor in this |
| `0x8271c170` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:40` | /// is +4, so `OnCreated8271c170` `ORs1` at its+A4. No Census population is invented. |
| `0x8271d3a8` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:216` | /// `LivingWorld::Manager::IsBigBlackServiceActive`8271d3a8 reads +0x1b5b4, |
| `0x8271f6f0` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:218` | /// Selected source subsequence of `Manager.Init`8271f6f0: pedestrian Init |
| `0x8271f86c` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:276` | // Manager.Init8271f86c initializes the traffic clock before entity Init. |
| `0x8271f890` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:283` | // Manager.Init8271f890 before pedestrian Init8271f8a8. Preserve |
| `0x8271f8a8` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:283` | // Manager.Init8271f890 before pedestrian Init8271f8a8. Preserve |
| `0x8271f8a8` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:292` | // its virtual Init at8271f8a8. This is only that selected publication, |
| `0x827209f8` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:39` | /// Census ctor827209f8 writes FE to actual object+A8. The observer subobject |
| `0x82720a0c` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:247` | // CensusMan82720a0c captures the linked initial vehicle allowance. |
| `0x82720a74` | Skate 2? | `crates/skate2-core/src/actors/skater/skater_manager_observers.rs:94` | /// Selected workspace callback producer: Census ctor82720a74, then AI.Init |
| `0x82721e40` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:243` | // Manager constructor82721e40 Reset precedes embedded MeshMan/RAIN. |
| `0x82721e4c` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:246` | // Manager ctor82721e4c constructs CensusMan before MeshMan/RAIN; |
| `0x82721e58` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:259` | // Manager constructor82721e58 precedes the selected EntityMan Init calls. |
| `0x82721e58` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:262` | // Native embedded RAIN constructor82721e64 follows MeshMan82721e58. |
| `0x82721e64` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:262` | // Native embedded RAIN constructor82721e64 follows MeshMan82721e58. |
| `0x82721f00` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:105` | /// does not drive frames. Initialized by Manager ctor82721f00. |
| `0x82721f00` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:125` | /// Same embedded ChaseMan constructed at Manager82721f00. Its message |
| `0x82721f18` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:266` | // Manager constructor82721f18 follows the embedded ChaseMan. |
| `0x82721fec` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:129` | /// Manager.mpBigBlack explicitly null at82721fec. The actual spawn handler |
| `0x82722018` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:268` | // Original82722018 seeds this field from rw::math::Random. This is |
| `0x8272590c` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:31` | /// `Manager::CheckSpawnedDynamicObjectAgainstSkaters` 0x8272590c..5914: |
| `0x827281e8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:377` | /// reset of `Manager::Update1` (0x827281e8), for every pedestrian. |
| `0x82728298` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:48` | /// Direct `Actor+0x258` reads (`Manager::Update1` 0x82728298). |
| `0x82729770` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:313` | /// `TimerCallback82729770`: poll, SIM cache, PRES cache; host owns cadence. |
| `0x82729c28` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:46` | /// `CommonBackEnd::GameMode` values read by `CreateSessionMarker`82729c28. |
| `0x82729c28` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:110` | /// `GameModule::ResetGame`'s `CreateSessionMarker`82729c28: PartyPlay has |
| `0x8272c4d8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:389` | /// `Sk8AppMan::PreUpdate`8272c4d8 `InputManager::LastInput(kINPUT_SIM)`: |
| `0x8272e4c0` | Skate 2? | `crates/skate2-audio/src/system.rs:440` | /// The `Sk8SpeechSystem` singleton (`CreateAudioSingletons` 0x8272e4c0 constructs it after |
| `0x82730418` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:5` | //! 0x828aadd0 asks for it and `PlayerUI::Update` 0x828af9f8, run by `DoGameUpdate` 0x82730418 |
| `0x8273a680` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:4` | //! distances). The first player is there too: `ResetGame` 0x8273a680's `PlayerUI::OnReset` |
| `0x8273a680` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:133` | /// A new game's `GameModule::ResetGame` (`0x8273a680`): `CreateSessionMarker` for its mode. |
| `0x8273add8` | Skate 2? | `crates/skate2-audio/src/world/environment.rs:425` | /// `CSTATEMGR_Environment` and its one state. `GameModule::OneOffLoad` 0x8273add8 calls |
| `0x8273b1a0` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:1` | //! The world behind the main menu. `GameModule::OneOffLoad` 0x8273b1a0 boots at the non-streamed |
| `0x8273c8f0` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:229` | /// Leaving the front end for a game: `GameModule::GameLoad` 0x8273c8f0 loads every AI model |
| `0x82740458` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:51` | /// `InitBoot82740458`. This owner can be built before mounted player startup. |
| `0x8274ca90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_conditions.rs:82` | //Factory82BC3F68 installs8231ED5C; virtual48=8274CA90 is li r3,0;blr. |
| `0x8275dcc8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:24` | /// first copies 25 from the hard-coded config built by `8275DCC8` (`li r27,25`); the value is |
| `0x8275e6e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:344` | ///World8275EB10 calls EndBoard8275E6E8 after PlayerInput and before |
| `0x8275ea20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:71` | //World8275EA20 starts both batches before actor SetUpPhysics. Foot |
| `0x8275ea20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:318` | /// backend. World8275EA20 submits these before actor SetUpPhysics. |
| `0x8275eb10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:344` | ///World8275EB10 calls EndBoard8275E6E8 after PlayerInput and before |
| `0x8275ec0c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:143` | //World8275EC0C ends board queries before PostInput/state selection. |
| `0x8275eca4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:300` | //World8275ECA4 ends skeleton tests after state/forces and before solving. |
| `0x8275f0d8` | TU3 | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:3` | //! strike (0x8275f0d8 -> `Pedestrian` 0x82e38cc0), on Skate 2's living world. |
| `0x8275f0d8` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:911` | /// A strike on a pedestrian this tick (`ProcessOutput`'s intents, read by 0x8275f0d8). |
| `0x8275f0d8` | TU3 | `crates/skate2-core/src/session/selected_player_tick.rs:899` | // PC port (Skate 3 physics): TU3's game response to a shove strike (0x8275f0d8) reaches |
| `0x8275f430` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:436` | // World8275F430 -> AirCollector82DA7888 publishes the complete mask. |
| `0x8275f8d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/grab_scene.rs:73` | ///8275F8D8 dispatches ONLY descriptor kinds1/2. Resolve the actual authored |
| `0x8275fa78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:44` | self.query_result = None; //8275FA78 clears ready before enqueue. |
| `0x8275fd00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:54` | ///8275FD00 captures five actual forward lines; query execution is deferred. |
| `0x82760060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:77` | ///82760060: bounded queries, requested data, then interactable results. |
| `0x827602e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab.rs:82` | //82585DD8 descriptor initialization followed by827602E0 actual data request. |
| `0x82761698` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/world.rs:102` | ///82761698 full separating-axis triangle/AABB predicate: nine edge-axis |
| `0x82761698` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:417` | ///82761698: full edge-cross-axis, coordinate-axis, and face-plane separation. |
| `0x82762ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:87` | fn register_candidate_82762ab0(&mut self, registration: CandidateRegistration) { |
| `0x82762ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:93` | Some(record) => copy_grab_record_82762ab0( |
| `0x827631c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:254` | /// ContactCacheWrapper (827631C8 slot 20, 82765AA0). Offline the job writes the solver's |
| `0x82763bc4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_possession/drives.rs:45` | //82763BC4..3C44 normalizes both quaternions of every registered |
| `0x82763e00` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics.rs:226` | /// Online games: Island::Step_UpdatePrePhysics1 82763E00 solves 25 passes for any island |
| `0x827641a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:251` | /// A network game's skater world contacts per tick (Step_Collision1 827641A0..4318): the |
| `0x827642e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:252` | /// online skater ground job gets the cap 0x51 - 1 = 80 (827642E0..F8), its one buffer for |
| `0x8276430c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:271` | // Step_Collision1 8276430C ->827771E8 ->82777384 ->8277C664. |
| `0x82764ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:25` | ///82C20728/82764AB0: caller-sized lines, poolmask7, facing3, no mesh or |
| `0x82764ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query.rs:1` | //! Borrowed scene adapter for original Biped query82764AB0/82770650/82C1EAD8. |
| `0x82764ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:96` | //82764AB0 mask7 and8276EC40 aggregate nearest: ties retain earlier pools. |
| `0x82764cf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:86` | //uses Island virtual84=82764CF8, then82775E48/8276E010 creates |
| `0x82764cf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:59` | /// 82764CF8 pool mask 7; 82770910 walks the actual accelerated trajectory. |
| `0x8276556c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:1` | //! Original single-player SkaterSkaterCollisionPipeline8276556C. |
| `0x82765aa0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:254` | /// ContactCacheWrapper (827631C8 slot 20, 82765AA0). Offline the job writes the solver's |
| `0x82765ef0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/collision_groups.rs:2` | /// Island::InitCullingTable 82765EF0 for an online island (flags != FLAG_Both) also culls |
| `0x82765ef0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/collision_groups.rs:15` | //82765EF0..827679AC, Island.flags==3: inverted native culling bits. |
| `0x82765ef0` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:387` | /// 0x827b8c10, TU3 0x82765ef0) agree on every pair but group 17's, which TU3 moved from the board |
| `0x827679ac` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/collision_groups.rs:15` | //82765EF0..827679AC, Island.flags==3: inverted native culling bits. |
| `0x82768418` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:169` | //82768418 rejects the same logical Body and equal nonnull Body+32 |
| `0x82768728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:263` | // Ground job8277C864/8277C86C; board selection82768728. |
| `0x82768728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve.rs:40` | // Skeleton82BE5094 passes false to82768728: its edge threshold is -1, |
| `0x8276b9cc` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:585` | /// pushes and the sync point's completion (`StateOnlineGame::OnMessage` `0x8276b9cc`) pulls. |
| `0x8276c9d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:28` | // Valid test result initialized by8276C9D8: position0, normalY, |
| `0x8276dbe0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck/query.rs:18` | //8276E280 writes request80/84 from stack54/5C.8276DBE0 forwards |
| `0x8276e010` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:86` | //uses Island virtual84=82764CF8, then82775E48/8276E010 creates |
| `0x8276e280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:87` | //ClusteredMeshTrajectoryBatch vtable8231093C, AddTest8276E280. |
| `0x8276e280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck/query.rs:18` | //8276E280 writes request80/84 from stack54/5C.8276DBE0 forwards |
| `0x8276e6c8` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:103` | /// `PartyPlayLoadingState` 0x8276e6c8 once the location is loaded: each later player's |
| `0x8276e6c8` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:161` | /// `PartyPlayLoadingState` 0x8276e6c8 spawns every player at its location and starts it). |
| `0x8276ec40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:96` | //82764AB0 mask7 and8276EC40 aggregate nearest: ties retain earlier pools. |
| `0x8276edd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:89` | //f1..f4 through AggregateTrajectoryBatch8276EDD0 when multiple batches exist. |
| `0x8276ee88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:60` | /// 8276EE88 retains the first pool on equal contact times. Callers supply |
| `0x8276ee88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:300` | //8276EE88 chooses strict smallest nonnegative time, retaining the |
| `0x82770650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query.rs:1` | //! Borrowed scene adapter for original Biped query82764AB0/82770650/82C1EAD8. |
| `0x82770650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:79` | //82770650's component gate precedes dispatch, including rounded lines. |
| `0x82770910` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:59` | /// 82764CF8 pool mask 7; 82770910 walks the actual accelerated trajectory. |
| `0x82770b40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:1` | //!82770B40/82771D08 mesh traversal and82770E00/82772028 nearby collection. |
| `0x82770b40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck/query.rs:19` | //these to82770B40's matching-group and mesh-rejection checks. |
| `0x82770e00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:1` | //!82770B40/82771D08 mesh traversal and82770E00/82772028 nearby collection. |
| `0x82770e00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:265` | //82770E00 does not apply the trajectory mesh rejection mask here. |
| `0x82770ea8` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:160` | /// `StateLoading` (`PartyPlayLoadState` 0x82770ea8 loads the challenge's assets, then |
| `0x827719b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:130` | //827719B8 explicitly passes zero TRIANGLE fatness, irrespective |
| `0x827719b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:162` | //827719B8's ordered fsel choices, including the unordered upper choice. |
| `0x82771d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/world_query.rs:1` | //! Camera triangle leaf82771D08 stores the authored face normal, clamps every |
| `0x82771d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/world.rs:1` | //! Original trajectory collision leaf82771D08 and nearby triangle collector |
| `0x82771d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:113` | //Same source82771D08 leaf as camera: zero triangle fatness, |
| `0x82771d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:1` | //!82770B40/82771D08 mesh traversal and82770E00/82772028 nearby collection. |
| `0x82771d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:204` | // 82771D08 retains the authored packed surface, including water. |
| `0x82772028` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/world.rs:2` | //!82772028, across the host's world-format boundary. BoardWorld stores static |
| `0x82772028` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:1` | //!82770B40/82771D08 mesh traversal and82770E00/82772028 nearby collection. |
| `0x82775e48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:86` | //uses Island virtual84=82764CF8, then82775E48/8276E010 creates |
| `0x827771e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:271` | // Step_Collision1 8276430C ->827771E8 ->82777384 ->8277C664. |
| `0x82777384` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:271` | // Step_Collision1 8276430C ->827771E8 ->82777384 ->8277C664. |
| `0x8277a508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:82` | //8277B0F0 and8277B2C0 enter the same8277A508 primitive walker with |
| `0x8277a6c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/solid_contacts.rs:15` | /// (8277A6C0), between a skater volume and a host body of `group` (Skate 3 numbering). |
| `0x8277a6c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:22` | //8277AF90 first culls the assembly pair;8277A6C0 then culls each |
| `0x8277a7b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:103` | //8277A7B4/B8 packs original Volume88 tags, both zero for |
| `0x8277ae80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:51` | //8277AE80 suppresses the reversed assembly pair. The same-assembly |
| `0x8277af90` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/solid_contacts.rs:14` | /// TU3's assembly pair culling (8277AF90) and then each primitive pair's by its part group |
| `0x8277af90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:22` | //8277AF90 first culls the assembly pair;8277A6C0 then culls each |
| `0x8277b0f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:82` | //8277B0F0 and8277B2C0 enter the same8277A508 primitive walker with |
| `0x8277b2c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:52` | //branch8277B2C0 instead visits both directed primitive pairs and uses |
| `0x8277b2c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:82` | //8277B0F0 and8277B2C0 enter the same8277A508 primitive walker with |
| `0x8277bd40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/collision_groups.rs:16` | //GroundPipeline8277BD40..BD7C uses this same table before surface filtering. |
| `0x8277bd40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_colliders.rs:33` | //8277BD40..BD7C culls Part+92 against the world assembly group0 |
| `0x8277c5d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:181` | // Ground job8277C5D8 uses context83034F34/38/3C. The |
| `0x8277c664` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:271` | // Step_Collision1 8276430C ->827771E8 ->82777384 ->8277C664. |
| `0x8277c864` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:263` | // Ground job8277C864/8277C86C; board selection82768728. |
| `0x8277c86c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:263` | // Ground job8277C864/8277C86C; board selection82768728. |
| `0x82791a88` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:591` | /// `Net::ResumeGame` (`0x82791ca8`) / `Net::PauseGame` (`0x82791a88`) in an online game. |
| `0x82791ca8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:591` | /// `Net::ResumeGame` (`0x82791ca8`) / `Net::PauseGame` (`0x82791a88`) in an online game. |
| `0x827a1f80` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/skate3_catalog.rs:266` | /// AudioPresentation::FillSkaterData (TU3 827A1F80..2014): AudioTrickType, AudioTrickFinishType |
| `0x827a2c0c` | TU3 | `crates/skate2-audio/src/presentation.rs:180` | /// TU3 SkaterData +0xA8 bit 0 and +0xAC bit 31 (`FillSkaterData` 0x827a2c0c, 0x827a2c1c): |
| `0x827a2c0c` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_audio.rs:49` | /// `FillSkaterData`'s SkaterData +0xA8 bit 0 and +0xAC bit 31 (0x827A2C0C, 0x827A2C1C); |
| `0x827a2c1c` | TU3 | `crates/skate2-audio/src/presentation.rs:180` | /// TU3 SkaterData +0xA8 bit 0 and +0xAC bit 31 (`FillSkaterData` 0x827a2c0c, 0x827a2c1c): |
| `0x827a2c1c` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_audio.rs:49` | /// `FillSkaterData`'s SkaterData +0xA8 bit 0 and +0xAC bit 31 (0x827A2C0C, 0x827A2C1C); |
| `0x827ab5c0` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:118` | /// Attached arm of `Part::GetBodyTransform` 0x827ab5c0. Unlike `GetTransform`, |
| `0x827abed0` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1126` | // World.Update3 827abed0, after every physical scoring update and |
| `0x827ae008` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:839` | // World::Update1 827ae008: the grab query manager, every physical's Update0, |
| `0x827ae138` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:886` | // World::Update1 827ae138: deferred grab messages follow all Update3 |
| `0x827af468` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:632` | // World::UpdateConditioners 827af468 builds one input for every skater; a |
| `0x827af8f0` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1017` | // World::Update3 827af8f0: every physical's Update4/Update5 jobs, then |
| `0x827afa58` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1131` | // World.Update3 827afa58: ResolveContactSpyList follows the flag reset. |
| `0x827b12a4` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:411` | /// by the caller corresponding to World.Initialize827b12a4. |
| `0x827b28f0` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:237` | // 0x827b28f0: the DMO pool's serialized EndUpdate, run once the body |
| `0x827b2904` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:312` | // Step_UpdatePrePhysics2 0x827b2904..2954: the DMO ground pipeline |
| `0x827b298c` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:247` | // (827b298c/29a0). The host refresh shares the established immutable |
| `0x827b3698` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:785` | // Step_Solver2 827b3698 follows thaw. Frozen body labels can be -1 |
| `0x827b386c` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:534` | // Step_Collision1 827b386c..3998: the dense pool has adjacent |
| `0x827b3a20` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:629` | // Step_Collision1 827b3a20..3a3c: flags3 runs the same-pool skater |
| `0x827b3a90` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:384` | /// Island.ResolveContactSpyList827b3a90 -> collision Body.GetContacts82e50e60. |
| `0x827b3a90` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:127` | /// World.Update3 -> ResolveContactSpyList827b3a90 -> |
| `0x827b3c04` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:462` | /// This is the dependency-equivalent result of `0x827b3c04..0x827b3ec4`. |
| `0x827b3ec4` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:462` | /// This is the dependency-equivalent result of `0x827b3c04..0x827b3ec4`. |
| `0x827b5660` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_assembly_backend.rs:1019` | /// Actual state &=7 prefix, before target-map erasure (0x827b5660). |
| `0x827b5660` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1755` | /// State-store prefix of `Island::RemoveSpyTarget` (0x827b5660). |
| `0x827b7874` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_assembly_backend.rs:1007` | /// publishes its Body/Part-tag target mapping (0x827b7874/0x827b7880). |
| `0x827b7874` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1735` | /// State-store prefix of `Island::RegisterSpyTarget` (0x827b7874). |
| `0x827b7880` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_assembly_backend.rs:1007` | /// publishes its Body/Part-tag target mapping (0x827b7874/0x827b7880). |
| `0x827b8ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing_settings.rs:45` | // Init827B8AB0 sets speed1/time0, then recomputes length with this |
| `0x827b8c10` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/collision_groups.rs:17` | //Skate2 827B8C10 has19 groups and different pairs; do not import it. |
| `0x827b8c10` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:387` | /// 0x827b8c10, TU3 0x82765ef0) agree on every pair but group 17's, which TU3 moved from the board |
| `0x827ba954` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1800` | /// reads B then A (827ba954/95c), B's side rereads A then B (827baa54/a5c). |
| `0x827baa54` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1800` | /// reads B then A (827ba954/95c), B's side rereads A then B (827baa54/a5c). |
| `0x827bad10` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:560` | // Island ctor InitCullingTable 0x827bad10 gives every flags3 |
| `0x827bc050` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1101` | /// Vehicle. `AggregateTrajectoryBatch::AddTest` at `0x827bc050` submits the |
| `0x827bc0f0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1103` | /// `0x827bc0f0` admits only nonnegative collision times and replaces the current |
| `0x827bce08` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:934` | /// Reference: `tTrajectory::CalculateTimeOfCollision` at `0x827bce08`. |
| `0x827bdcc0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:88` | //S2 named827BDCC0 corroborates radius/time/error fields; TU3 preserves |
| `0x827bf5a0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1129` | /// `0x827bf5a0`. |
| `0x827bf868` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:916` | /// Reference: `tTrajectory::Get_dTWithErrorR` at `0x827bf868`. |
| `0x827c01a0` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:542` | /// 0x827c1300/0x827c01a0 and `BoxTest<TrajectoryBoxTestFunctor>` 0x827c1548: |
| `0x827c01bc` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:489` | /// per-mesh `LineTest` 0x827c01bc..0250 applies to the line endpoints. |
| `0x827c1300` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:542` | /// 0x827c1300/0x827c01a0 and `BoxTest<TrajectoryBoxTestFunctor>` 0x827c1548: |
| `0x827c1548` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:542` | /// 0x827c1300/0x827c01a0 and `BoxTest<TrajectoryBoxTestFunctor>` 0x827c1548: |
| `0x827c1c78` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:963` | /// Reference addresses: `0x827c2270`, `0x827c1e58`, and `0x827c1c78`. |
| `0x827c1e58` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:963` | /// Reference addresses: `0x827c2270`, `0x827c1e58`, and `0x827c1c78`. |
| `0x827c1f78` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1021` | // Walk827c1f78..1fe0 multiplies errorStart * radius * errorStart * radius. |
| `0x827c2270` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:963` | /// Reference addresses: `0x827c2270`, `0x827c1e58`, and `0x827c1c78`. |
| `0x827c6b68` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/hud_frame.rs:109` | // BigTrickUpdate827c6b68 iterates all 15 original thresholds. |
| `0x827c6c40` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/hud_frame.rs:117` | // UpdateAirContextData827c6c40: only the Over UID bank is displayed; |
| `0x827c73b0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/hud_frame.rs:8` | // UpdateHoM827c73b0 copies the newly appended collision tail, not the complete |
| `0x827c79b8` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1106` | // PhysOutModule_Hud::Update 827c79b8 keeps the statistics every frame, HUD shown or not. |
| `0x827c79b8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/hud_frame.rs:1` | //! PhysOutModule_Hud827c79b8 reads completed Scoring2/Animation output. |
| `0x827c8378` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:13` | //! `ClusteredMesh` through `ClusteredMeshData::Reset` 0x827c8378: the forward |
| `0x827c84e0` | Skate 2? | `crates/skate2-core/src/physics/collision/static_ground_pool.rs:5` | //! `GroundPool::AddVolume` (`0x827c84e0`) with identity placement. Removal |
| `0x827c8578` | Skate 2? | `crates/skate2-core/src/physics/collision/static_ground_pool.rs:3` | //! `WorldStreamerView::AddVolume` (`0x827c8578`) walks an Environment |
| `0x827c86d0` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:5` | //! volume and `Part::GetTransform` to `DynamicPool::AddVolume` 0x827c86d0; a |
| `0x827c86d0` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:150` | /// `AddVolume` 0x827c86d0; `Ok(false)` is its capacity refusal. |
| `0x827c8e60` | Skate 2? | `crates/skate2-core/src/physics/collision/static_ground_pool.rs:6` | //! (`0x827c9088` -> `0x827c8e60`) walks the same roots forward, removes the |
| `0x827c8f88` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:6` | //! (`0x827c8f88..0x827c9084`) resets body state without touching the PDB-named |
| `0x827c9084` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:6` | //! (`0x827c8f88..0x827c9084`) resets body state without touching the PDB-named |
| `0x827c9088` | Skate 2? | `crates/skate2-core/src/physics/collision/static_ground_pool.rs:6` | //! (`0x827c9088` -> `0x827c8e60`) walks the same roots forward, removes the |
| `0x827c9300` | TU3 | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:474` | /// `DynamicPool::Remove` 0x827c9300..93e4: every mesh the assembly owns, |
| `0x827c93f0` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:3` | //! `DynamicPool::BeginUpdate` 0x827c93f0 empties the mesh collection, then |
| `0x827c93f0` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:215` | /// `DynamicPool::BeginUpdate` 0x827c93f0's mesh half: the collection is |
| `0x827cb5c8` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:589` | // `GlobalBuffer::Store` 0x827cb5c8 keeps only what still fits. |
| `0x827cb678` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:176` | /// 0x827cb678, in their exact fused grouping: the sum of the squared row |
| `0x827cb8a0` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:175` | /// `GetOrthogonalError<float>` 0x827cb8a0 of `Mult<float>(m, transpose(m))` |
| `0x827cb980` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:141` | /// `rw::physics::Contact::IsValid` 0x827cb980: every point, velocity and |
| `0x827cd500` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:637` | /// Composes the successful final value at `0x827cd500..0x827cd778`. |
| `0x827cd778` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:637` | /// Composes the successful final value at `0x827cd500..0x827cd778`. |
| `0x827cdb98` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:909` | // the manager static body (0x827cdb98..0x827cdf48). |
| `0x827cdf48` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:909` | // the manager static body (0x827cdb98..0x827cdf48). |
| `0x827ce4e8` | TU3? | `crates/skate2-core/src/physics/assembly/skate3_world.rs:691` | // Match GetClusteredMesh827ce4e8: the first volume record owns |
| `0x827cf140` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:890` | /// `SkaterDMOCollisionJob` 0x827cf140: A is the skater volume's body |
| `0x827d55c0` | Skate 2? | `crates/skate2-audio/src/system.rs:190` | /// `AudioSystem::mNISFadeAmount`, which `BloomToWhiteTransitionEffect::Update` 0x827d55c0 |
| `0x827ef130` | Skate 2? | `crates/skate2-audio/src/presentation.rs:294` | /// `AudioPresentation::ConvertToImpactLevelFromForce` 0x827ef130. |
| `0x827f2708` | Skate 2? | `crates/skate2-core/src/assets/character.rs:249` | /// `PresentationEntity::CalculateBoundsInfo` 0x827f2708: the first part's |
| `0x827f4c48` | Skate 2? | `crates/skate2-audio/src/presentation.rs:1` | //! `Sk8::AudioPresentation` for the skater: the `PhysOut` values `FillSkaterData` 0x827f4c48 reads |
| `0x827f4c48` | Skate 2? | `crates/skate2-audio/src/presentation.rs:331` | /// `AudioPresentation::FillSkaterData` 0x827f4c48 for one skater (`DoSimSubmit` clears the record |
| `0x827f4c48` | Skate 2? | `crates/skate2-audio/src/system.rs:827` | /// `AudioPresentation::FillSkaterData` 0x827f4c48 against this system's vault. |
| `0x827f4c48` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_audio.rs:1` | //! The retained `PhysOut` values `AudioPresentation::FillSkaterData` 0x827f4c48 reads, copied for |
| `0x827f8138` | Skate 2? | `crates/skate2-core/src/assets/character.rs:319` | /// only for a model bone the hierarchy lacks (`RemapBones` 0x827f8138, |
| `0x827f8338` | Skate 2? | `crates/skate2-core/src/assets/character.rs:320` | /// `FillInvBPBuffer` 0x827f8338), so a hierarchy without it still serves |
| `0x827f8338` | Skate 2? | `crates/skate2-core/src/assets/character.rs:323` | /// `FillInvBPBuffer` 0x827f8338 writes every part's matrices in |
| `0x827fa6e8` | Skate 2? | `crates/skate2-core/src/assets/character.rs:344` | /// `PresentationEntity::RemoveComponentInstances` 0x827fa6e8 takes out of |
| `0x827fb2f8` | Skate 2? | `crates/skate2-audio/src/system.rs:280` | /// player (`DoSimSubmit` 0x827fb2f8), -1 without a local skater. |
| `0x827fb2f8` | Skate 2? | `crates/skate2-audio/src/system.rs:284` | /// `AudioPresentation::DoSimSubmit` 0x827fb2f8 reads from `WorldPainter` layer 10 at the local |
| `0x827fb2f8` | TU3 | `crates/skate2-audio/src/system.rs:301` | /// `AudioPresentation` `mVehicleCount`/`mVehicle`: the cars `DoSimSubmit` 0x827fb2f8 found |
| `0x827fb2f8` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:260` | /// `AudioPresentation::DoSimSubmit` 0x827fb2f8's skaters behind the menu: the first player, |
| `0x827fba0c` | Skate 2? | `crates/skate2-audio/src/presentation.rs:313` | /// `AudioPresentation::DoSimSubmit` 0x827fba0c: `GlobalData`'s `timesOneFive`, `timesTwo` and |
| `0x827fcd40` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_audio.rs:138` | /// `VDWriter::_PrepareBailData` 0x827fcd40 records as the bail's `mNormalizedBailValue`. |
| `0x827fdd78` | Skate 2? | `crates/skate2-audio/src/system.rs:269` | /// `VDReader::ReadData` 0x827fdd78 sets on the frames `VDWriter::WriteData` recorded an amount. |
| `0x827ff8c8` | Skate 2? | `crates/skate2-audio/src/system.rs:273` | /// `cMsgFadeAmount` `VDWriter::WriteData` 0x827ff8c8 recorded for the frame, if any. |
| `0x828158f0` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:58` | /// (`ApplyPlayerAttributes` 0x828158f0; the front end has no Actor to reload), then |
| `0x82817f28` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:57` | /// `OnSimCharacterLoaded` 0x82817f28 applies its marquee's character, stance and hat |
| `0x82818170` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:410` | /// `PlayerLoaderImpl::Unload` 0x82818170 (`PlayerLoader::UnloadPartyMember` 0x828182f0): |
| `0x828182f0` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:410` | /// `PlayerLoaderImpl::Unload` 0x82818170 (`PlayerLoader::UnloadPartyMember` 0x828182f0): |
| `0x82818f2c` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:265` | /// PresCharacterManager::Update82818f2c: actual recipe entity membership |
| `0x82819cb8` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:62` | /// (`Sk8CACLoader::ApplyAttributesImpl` 0x82819cb8), which an already loaded one does not |
| `0x82819da0` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:864` | // CAC LoadCompleteImpl82819da0 calls ApplyAttributesImpl when set-as-skater. |
| `0x8281aa80` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:396` | /// `PlayerLoaderImpl::Load` 0x8281aa80's CAS branch for a slot the caller set to the |
| `0x8281aa80` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:41` | /// The profile's created skater, whose parts record `PlayerLoaderImpl::Load` 0x8281aa80 reads |
| `0x8281dd70` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/push_off.rs:2` | //! The ctor installs vtable8231F6D4; predicate slot+48 is8281DD70. |
| `0x8281dd70` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/push_off.rs:16` | ///8281DD70: unconditional true in the verified original Skate3 TU3 image. |
| `0x828219a8` | Skate 2? | `crates/skate2-core/src/assets/character.rs:285` | /// SceneRenderView::UpdateEntityLOD82826a10 and recipe UpdateLOD828219a8. |
| `0x82826a10` | Skate 2? | `crates/skate2-core/src/assets/character.rs:285` | /// SceneRenderView::UpdateEntityLOD82826a10 and recipe UpdateLOD828219a8. |
| `0x8282c7c8` | Skate 2? | `crates/skate2-core/src/assets/character.rs:21` | /// `TextureMan`'s boot stand-in for a texture nothing loaded (`ResolveMissingTexture` 0x8282c7c8). |
| `0x82857ec8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/clock.rs:1` | //! Offline tick cadence. Original TU3 Simulation::OnRequestSetSimRate82857EC8 |
| `0x82857f34` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/clock.rs:43` | //TU3 hardcodes180 at82857F34; S2 used a debug setting here. |
| `0x82857f44` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/clock.rs:25` | //82857F44..7C rounds the reciprocal to an integer timer frequency. |
| `0x82858810` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:197` | //82858810 registers these three static poses in this exact order. |
| `0x82859e70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:28` | ///82859E70 completes earlier trajectory work atA06C/A080 and invokes virtual |
| `0x8285a06c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:66` | //8285A06C/A080 completes prior trajectory batches, then A0D0 calls |
| `0x8285a694` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/clock.rs:50` | //SimUpdate2 8285A694..A6C8 runs after the camera publication. Pending |
| `0x8285c968` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:52` | //SimController8285C968 dispatches the preceding tick's camera messages |
| `0x8286cd88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:58` | //8286CD88 projects both vectors into the plane;8296EC98 signs acos. |
| `0x82898920` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/controllers.rs:116` | // line after PlayerInputData's Pack/Unpack (82898920 -> 82695438). |
| `0x82898920` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:62` | /// PlayerUI82898920 transforms the gameplay packet before Raw/Derived input. |
| `0x82898920` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:104` | //PlayerUI82898920 remaps the gameplay packet before both controller |
| `0x82898b80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:84` | //82898B80..BC4 excludes grabbing an object (304) and state503. |
| `0x82898b80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:299` | /// PlayerUI82898B80..BE0: offboard exclusions and the separate State83 path. |
| `0x82898bc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:87` | //82898BC8..BE0 also remaps State83 while on board. Wipeout Fill |
| `0x82898d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:29` | //One tick's native PlayerUI82898D20 result, shared by animation and PhysIn. |
| `0x82898d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:245` | /// Complete numerical remap82898D20; camera is the native XYZ presentation |
| `0x82898d98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:249` | //82898D98..DB4: ABS(dot(world_up, camera_Z)) < literal820ED5E8. |
| `0x82898ee0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:256` | //CacheLine includes its count at0.82898EE0..F0C reads Buttons17-16 |
| `0x82898f44` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:262` | //82898F44..FAC splits world X/Z into positive/negative pad channels; |
| `0x8289adb8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:584` | /// `kPAUSE_STATE_ONLINE`, which `Online.OnlineChallengeRoundSyncPoint` (`0x8289adb8`) pulls and |
| `0x8289d180` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:1` | //! Stock classes are verified against original8289D180 and828A0054..00A0: |
| `0x8289d270` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout/settings.rs:1` | //! Globals264 -> original8289D270 class EB844CFDBF928EFD/default. |
| `0x8289d5c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/settings.rs:57` | //Globals8289D5C8:ED14 class lookup;ED48 stores188, layout248. |
| `0x8289e448` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:198` | /// `Simulation::IsNetworkGame`8289e448, which `SessionMarker::Update`, `VerifyPosition` and |
| `0x8289f128` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:93` | //8289F128 global220 = AF781D7B614DAB50 physicstrucks/default; |
| `0x8289f474` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:625` | // SimUpdate1 8289f474/488 increments before InputManager.Update. |
| `0x8289f558` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:670` | // SimUpdate1 8289f558: AISkaterManager.Update follows GameInputManager.Update. |
| `0x8289f580` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:675` | // SimUpdate1 8289f580: RecipeMan.Update precedes Manager.Update1. |
| `0x8289f5a8` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:682` | // SimUpdate1 8289f5a8/8289f5e0: LivingWorld Manager::Update1 and |
| `0x8289f5e0` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:682` | // SimUpdate1 8289f5a8/8289f5e0: LivingWorld Manager::Update1 and |
| `0x8289f634` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_collision_settings.rs:51` | /// at8289F634: physics_collision/default. Stock XML baseC6AE0 independently |
| `0x8289f6ac` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:236` | // SimUpdate1's EndDMOUpdate 0x8289f6ac -> Island::Step_UpdatePrePhysics2 |
| `0x8289f77c` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:892` | // SimUpdate1 8289f77c: Manager.Update3 after World.Update1 and before |
| `0x8289f8b4` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/animation_input.rs:82` | //TU3 global constructor8289F8B4 binds292 to anim_motion |
| `0x8289f8f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:102` | //82BA35E0/3790 read Globals296;8289F8F0 binds hash |
| `0x8289f8f0` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:268` | // SimUpdate1 8289f8f0, after World.Update1 and before World.Update2. |
| `0x8289f908` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:284` | // SimUpdate1 8289f908, between Predictor and UpdateLocomotion. |
| `0x8289f934` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:287` | // SimUpdate1 8289f934: result0 has completed; now produce result1. |
| `0x8289f9c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_spin.rs:27` | //Global304 writer8289F9C0..F9EC:DCBADE97CA665643=body_spin. |
| `0x8289fa34` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1137` | // SimUpdate2 8289fa34: World.Update3 completes before AnimMan.Sync. |
| `0x8289fb24` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:46` | //82BA5150 global316/layout1844;8289FB24 binds pushing. |
| `0x8289fb90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:30` | //Original8289FB90..FBB0 binds Globals324 to A168897919752875, |
| `0x8289fc50` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1142` | // SimUpdate2 8289fc50: Manager::UpdatePostTransforms follows World |
| `0x8289fc5c` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1162` | // Native camera gate8289fc5c is updateMask & submitMask, not bit1. |
| `0x8289fca0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_feedback.rs:28` | // Global344 at8289FCA0 binds hash7823ADA8E7C896AA = bumps. |
| `0x8289fce8` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1181` | // SimUpdate2 8289fce8: after CameraManager.Update/Wait, the same |
| `0x8289fd10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_finger_flip.rs:13` | // GlobalAttributeCollection164 is selected at8289FD10..30. The |
| `0x8289fd3c` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1191` | // SimUpdate2 0x8289fd3c invokes SkaterManager::PostUpdate after |
| `0x8289fd40` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:1210` | // SimUpdate2 8289fd40..fd78: updateMask3 has bit0, so the countdown |
| `0x828a0054` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:1` | //! Stock classes are verified against original8289D180 and828A0054..00A0: |
| `0x828a0154` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:40` | //82BBBDE0: Globals400/layout96, bound by828A0154. |
| `0x828a1070` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:596` | /// `NISManager::PauseSim` (`0x828a1070`) / `ResumeSim` (`0x828a1170`): |
| `0x828a1170` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:596` | /// `NISManager::PauseSim` (`0x828a1070`) / `ResumeSim` (`0x828a1170`): |
| `0x828a739c` | Skate 2? | `crates/skate2-core/src/session/selected_player_tick.rs:574` | // SimController::Update1 Dispatch828a739c flushes eEQGameSim before |
| `0x828aa130` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:406` | /// `IsSafeToUseSessionMarker`828aa130: the Skeleton byte is only read |
| `0x828aa390` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:360` | /// `PlayerUI::ReceiveRumbleResetMsg` 0x828aa390: this controller's `Vibration::Reset`. |
| `0x828aa4c0` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:653` | /// `PartyPlayUI::SetPlayer`828aa4c0: `mPlayerIndex`, `mControllerIndex` from |
| `0x828aa8b0` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:177` | /// PlayerUI's GetLocalActor828aa8b0: the ghost, then FindByIndex(mPlayerIndex). |
| `0x828aadd0` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:5` | //! 0x828aadd0 asks for it and `PlayerUI::Update` 0x828af9f8, run by `DoGameUpdate` 0x82730418 |
| `0x828ab060` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:170` | /// SessionMarker uses GetLocal828ab060, independently of PlayerUI's index. |
| `0x828ab060` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:232` | /// `GetLocal`828ab060: `SkaterManager::GetLocal` (+0x170). |
| `0x828ab0f0` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:268` | /// `GetMainActivity`828ab0f0: only a career runs challenge activities. |
| `0x828ab4a0` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:258` | /// `VerifyLocalPlayerOkToSet`828ab4a0 in a network game: `IsPostionOk` at the local Actor's |
| `0x828ab6d0` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:276` | /// VerifyChallengeOkToUse828ab6d0 calls ChaseIsOn826f3b48. |
| `0x828ad280` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:6` | //! every frame, spawns it at the location (`PlayerUI::ResetPlayer` 0x828ad280); it is |
| `0x828af9f8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:409` | /// `PlayerUI::Update`828af9f8 through its focus prefix; false is the |
| `0x828af9f8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:617` | /// The front end's simulation frame's sync command. Natively `PlayerUI::Update` 0x828af9f8 |
| `0x828af9f8` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:5` | //! 0x828aadd0 asks for it and `PlayerUI::Update` 0x828af9f8, run by `DoGameUpdate` 0x82730418 |
| `0x828b14b4` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_score_holder.rs:61` | // Raw828b14b4..14cc class;82df35dc..35f8 default collection. |
| `0x828b31d8` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1032` | // Immutable projection of actual Global+16c constructor828b31d8. |
| `0x828caa00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:207` | /// The times a clip is actually sampled at. `AnimCommandSystem::Fetch` 0x828CAA00 packs the |
| `0x828d3b58` | TU3 | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:149` | /// TU3828D3800 then828D3B58. NewACS824744B8 initializes the |
| `0x828d7b34` | TU3? | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:208` | /// evaluated times into the fetch command and `FetchEngine::EnQueueCmd` 0x828D7B34 unpacks them |
| `0x82966910` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/clock.rs:64` | //Original timer setter82966910..2C: integer division in100ns units. |
| `0x8296d0d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/controllers.rs:101` | /// TU3 8296D288/8296D0D0: write the inactive cache, publish one four-device |
| `0x8296d288` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/controllers.rs:101` | /// TU3 8296D288/8296D0D0: write the inactive cache, publish one four-device |
| `0x8296d480` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/controllers.rs:115` | // 8296D480 sets byte13 only when capability SubType == 7. Gameplay reads the |
| `0x8296e988` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:339` | //8296E988 uses authored InTime if tag is empty/component absent. |
| `0x8296ec98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:58` | //8286CD88 projects both vectors into the plane;8296EC98 signs acos. |
| `0x82970628` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:77` | //82BA4D50 -> ISkaterMotionGraph256 ->8258FB60 ->82970628. |
| `0x82970628` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:142` | //Complete original82970628 carry/update sequence, including the |
| `0x82970628` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/rng.rs:1` | //! Host-owned orientation stream using recovered82970628 arithmetic. |
| `0x82972340` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:801` | /// OR100. `AllocHeapMem82972340/829723bc` clears the full0x61d0 allocation. |
| `0x829723bc` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:801` | /// OR100. `AllocHeapMem82972340/829723bc` clears the full0x61d0 allocation. |
| `0x8297a508` | Skate 2? | `crates/skate2-audio/src/reverb.rs:24` | /// `Attrib::Class::GetFirstCollection` 0x8297a508 of `aud_reverb` in `skater.vlt`: the class's |
| `0x8297a518` | Skate 2? | `crates/skate2-audio/src/reverb.rs:30` | /// (`GetNextCollection` 0x8297a518). |
| `0x8298c8c8` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:456` | /// `cInputMap::GetState`8298c8c8 (nonzero) of a session marker state on |
| `0x82990158` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_conditions.rs:92` | //82DF90C0 compares exactly strlen(requested) bytes with82990158. |
| `0x829b1f78` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:474` | /// `Csis::ClassData::SendParameters` 0x829b1f78 (`SetMemberData`). |
| `0x829b2358` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:482` | /// `Csis::Class::ReleaseFast` 0x829b2358: the destructor clients, then the creator's |
| `0x829b2910` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:496` | /// `Csis::Function::CallFast` 0x829b2910. |
| `0x829b2ad0` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:444` | /// `Csis::Class::CreateInstanceFast` 0x829b2ad0: a new `ClassData` (refcount 1), every |
| `0x829b2c88` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:512` | /// `Csis::GlobalVariable::SetFast` 0x829b2c88: clients hear only a change. |
| `0x829b3180` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1` | //! The EA `spch` speech library (`SPCH_*` / `iSPCH_*`, 0x829b3180-0x829b6ef0 and |
| `0x829b3180` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:684` | /// `VoxEvent_GetFollowEventData` 0x829b3180: the events that may follow this one. |
| `0x829b3248` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:70` | /// `SPCH_MakeEventSpec` 0x829b3248. |
| `0x829b3260` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:538` | /// `iSPCH_FindEventDatInfo` 0x829b3260. |
| `0x829b32d0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:546` | /// `iSPCH_FindEvent` 0x829b32d0. |
| `0x829b3428` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:556` | /// `iSPCH_GetGlobalMatchParmsArray` 0x829b3428: the array's database and offset. |
| `0x829b3530` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:741` | /// `iSPCH_TestEventPriority` 0x829b3530: below the database's filter priority. |
| `0x829b3598` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:446` | /// `iSPCH_InitEventQueue` 0x829b3598. |
| `0x829b36e0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:565` | /// `iSPCH_ClearEvent` 0x829b36e0. |
| `0x829b3778` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:665` | /// `SPCH_PlayLastEvent` 0x829b3778. |
| `0x829b3808` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:586` | /// `iSPCH_FindEventSlot` 0x829b3808: a free slot; when all are pending, the first whose event |
| `0x829b3928` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:614` | /// `iSPCH_AddEvent` 0x829b3928: an event its frequency lets through takes a slot of its |
| `0x829b3b88` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:656` | /// `SPCH_AddSmartEventV` 0x829b3b88 (`gSPCH_AddSmartEvent`): the event spec and its |
| `0x829b3c50` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:699` | /// `iSPCH_ChooseEventSearch` 0x829b3c50: the channel's pending event (one the follow data |
| `0x829b3de8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:746` | /// `SPCH_Choose` 0x829b3de8: choose a sentence for the channel's next event (preferring |
| `0x829b3fb0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:791` | /// `SPCH_Play` 0x829b3fb0: play the chosen sentence, or choose one; the phrases requested. |
| `0x829b40c0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1799` | /// `SPCH_GetSampleDataRate` 0x829b40c0 for uncompressed data. |
| `0x829b40f8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:413` | /// `SPCH_Init` 0x829b40f8 (the request callback is the caller's queue) and |
| `0x829b42c0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1706` | /// `BANKHDR_GetArrayBankBitsAddr` 0x829b42c0: the sub banks' sample bits. |
| `0x829b4340` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1215` | /// `iSPCH_PostMatchParmValue` 0x829b4340. |
| `0x829b43d0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1172` | /// `iSPCH_MatchSample` 0x829b43d0: every filter's mask holds the sample's value, which the |
| `0x829b4508` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1077` | /// `iSPCH_GetPhraseBank` 0x829b4508: bank type 0 by id, 1 by id and sub id (the parameter |
| `0x829b45f0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1233` | /// `iSPCH_SampleExists` 0x829b45f0: in range, in the sub bank's sample bits, and in the |
| `0x829b46f8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1114` | /// `iSPCH_ChooseSamples` 0x829b46f8: the bank's samples whose parameters match the phrase, |
| `0x829b4920` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:899` | /// `iSPCH_OrderSentences` 0x829b4920: weights `4^(w >> 5) * (w & 31)` drawn without |
| `0x829b4a68` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1009` | /// `iSPCH_InitMatchParmIO` 0x829b4a68. |
| `0x829b4ac8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1341` | /// `iSPCH_TestMatchParms` 0x829b4ac8: the picks post their sentence match parameters and the |
| `0x829b4c70` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1018` | /// `iSPCH_SentenceGetChoices` 0x829b4c70: each phrase's bank and matching samples, phrases |
| `0x829b4e30` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1288` | /// `iSPCH_ChooseSentenceIteratively` 0x829b4e30: the first combination (odometer order, |
| `0x829b4ff8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1396` | /// `iSPCH_PostGlobalMatchParms` 0x829b4ff8: the picks' values for the global match |
| `0x829b50f0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1258` | /// `iSPCH_SentenceMakeChoice` 0x829b50f0: with a filter length or match parameters each |
| `0x829b5358` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1480` | /// `iSPCH_MakeSampleRequests` 0x829b5358: a request per phrase (a sub bank's samples after |
| `0x829b55f0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1449` | /// `iSPCH_ClearSentenceChoiceChannel` 0x829b55f0. |
| `0x829b5718` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1420` | /// `iSPCH_SaveChosenSentence` 0x829b5718. |
| `0x829b5860` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1456` | /// `iSPCH_PlayChosen` 0x829b5860: the chosen sentence's sample requests (`iSPCH_RuleSet` and |
| `0x829b5948` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:985` | /// `iSPCH_MakeValidParmsMask` 0x829b5948: bit `k - 1` for each of the first sixteen |
| `0x829b5ad8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:937` | /// `iSPCH_SentenceIsContextMatch` 0x829b5ad8: each of the sentence's context masks holds a |
| `0x829b5bb8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:846` | /// `iSPCH_ChooseSentence` 0x829b5bb8: the sentences in weighted random order; the first |
| `0x829b5e68` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:839` | /// `iSPCH_ChooseSingleSentence` 0x829b5e68 (no `reparm` callback). |
| `0x829b5f70` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:246` | /// `iSPCH_EACrandom` 0x829b5f70 and `iSPCH_Rand` 0x829b6050's history (`seedX`, `gRandArray`, |
| `0x829b5f70` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:278` | /// `iSPCH_EACrandom` 0x829b5f70: a six-word add-with-carry generator. |
| `0x829b6050` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:246` | /// `iSPCH_EACrandom` 0x829b5f70 and `iSPCH_Rand` 0x829b6050's history (`seedX`, `gRandArray`, |
| `0x829b6050` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:316` | /// `iSPCH_Rand` 0x829b6050: a draw in `0..max`. With a handle it redraws (up to 32 times) |
| `0x829b61c8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:256` | /// `iSPCH_InitRandom` 0x829b61c8: the six words from `x = seed * 65537`, then |
| `0x829b6350` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1572` | /// `iSPCH_FindInsertPosition` 0x829b6350 (-1 for a bank already added). |
| `0x829b6498` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1614` | /// `iSPCH_FindBank` 0x829b6498: a bank of the type (binary search), and its handle. |
| `0x829b6528` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1637` | /// `iSPCH_FindSubBank` 0x829b6528. |
| `0x829b65e8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1660` | /// `iSPCH_TestSubBankBounds` 0x829b65e8. |
| `0x829b6638` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1668` | /// `iSPCH_FindBankIndexFromHandle` 0x829b6638. |
| `0x829b6738` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1738` | /// `iSPCHBank_AddToQueue` 0x829b6738. |
| `0x829b67c8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1756` | /// `iSPCHBank_GetSampleTimeInQueue` 0x829b67c8: how many plays ago the sample was queued (1 for |
| `0x829b6870` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1777` | /// `iSPCH_SetCycleBits` 0x829b6870: the samples of cycle `gameNum % count` (the first |
| `0x829b6988` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1543` | /// `SPCH_AddBank` 0x829b6988: the bank inserted in (type, sub id) order with the next unique |
| `0x829b6a70` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1712` | /// `iSPCH_GetSampleParmAddr` 0x829b6a70. |
| `0x829b6aa0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1718` | /// `iSPCH_GetSampleSizeData` 0x829b6aa0: the sample's byte offset and size in blocks of |
| `0x829b6cb0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:951` | /// `iSPCH_GetRuleSettings` 0x829b6cb0 with no `testRule`: every rule's bit in the user mask, |
| `0x829b6e08` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:966` | /// `iSPCH_CheckSentenceRules` 0x829b6e08. |
| `0x829b6ef0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1` | //! The EA `spch` speech library (`SPCH_*` / `iSPCH_*`, 0x829b3180-0x829b6ef0 and |
| `0x82a6dd98` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:187` | //UpdateSimLODGlobal; MemStore Init82a6dd98/FreeBlock82a6df20 zero the block. |
| `0x82a6df20` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:187` | //UpdateSimLODGlobal; MemStore Init82a6dd98/FreeBlock82a6df20 zero the block. |
| `0x82a76fc0` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:13` | // mounted Juice boot paths stop before Initialize; JLogEvent82a76fc0 returns |
| `0x82a76fc0` | Skate 2? | `crates/skate2-core/src/actors/actor_input_generation.rs:17` | /// Shared JLogEvent82a76fc0 boot gate. The selected boot has not initialized |
| `0x82a7a900` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:569` | /// `AudioAssetManager::Update` 0x82a7a900, at the start of `AudioSystem::Update`. |
| `0x82a7d570` | Skate 2? | `crates/skate2-audio/src/system.rs:595` | /// 0x82a7d570: stage 255, `SubMix` "Master", `SampleCapture`, `Dac`, six channels), the music |
| `0x82a7dae0` | Skate 2? | `crates/skate2-audio/src/system.rs:1073` | /// `AudioCore::GetSystemTime` 0x82a7dae0: the RWAC system's time, which the rendered blocks |
| `0x82a7dc40` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:535` | /// `CSTATEMGR_Base::ProcessUpdate` 0x82a7dc40. |
| `0x82a7dd78` | Skate 2? | `crates/skate2-audio/src/system.rs:1397` | /// 0x82a7dd78 with `DMX_VOL`). |
| `0x82a7e0e0` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:491` | /// `CSTATE_Player::UpdateParams` 0x825587b8 and `CSTATE_Base::UpdateParams` 0x82a7e0e0. |
| `0x82a7e350` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:360` | /// `CSTATE_Player::Detach` 0x82551630 / `CSTATE_Base::Detach` 0x82a7e350. |
| `0x82a7f094` | Skate 2? | `crates/skate2-audio/src/system.rs:594` | /// `AudioCore::InitializeAudioCore` 0x82a7f094: the mastering voice (`CreateMasteringVoice` |
| `0x82a7f130` | Skate 2? | `crates/skate2-audio/src/player/mod.rs:3` | //! SFX objects. `Setup(0xFFF)` creates objects 0-11 in id order (`CreateSFXObjs` 0x82a7f130); |
| `0x82a80480` | Skate 2? | `crates/skate2-audio/src/system.rs:1212` | /// `MusicCore::GetSongInfo` 0x82a80480 for the "IPOD" list (`music::music_player_song_info`): |
| `0x82a81100` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:2` | //! `Sk8::Audio::EmitterSystem` (0x82a81100-0x82a83ae8): the sound volumes of the `.ems` data |
| `0x82a81100` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:816` | /// `LoadRandomEmitter` 0x82a81100: the first free slot. |
| `0x82a811a0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:526` | /// 0x82a811a0 each, `ResolveFiles` 0x82a81ae0) and `EnableEmitterSwap(true)`. |
| `0x82a81298` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:292` | /// `ConvertDistanceToVolume` 0x82a81298 by `eVolumeFalloffType`. |
| `0x82a812d8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:610` | /// is not disabled (`IsSwapEnabled` 0x82a812d8). |
| `0x82a812d8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:701` | /// `IsSwapEnabled` 0x82a812d8: the swap is not disabled. |
| `0x82a812f0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:707` | /// `UpdateEmitterSwapStatus` 0x82a812f0: once every slot is unloaded their banks leave AEMS, |
| `0x82a81378` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1158` | /// `AddGameState` 0x82a81378 / `RemoveGameState` 0x82a81388. |
| `0x82a81388` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1158` | /// `AddGameState` 0x82a81378 / `RemoveGameState` 0x82a81388. |
| `0x82a81398` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:173` | /// `cManagedOctTree<tEmitterVolume*, 1500>`: `Insert` 0x82a81398 reuses the last freed entry, |
| `0x82a81498` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:525` | /// `EmitterSystem::EnterGamePlay` 0x82a81498), `LoadGameFiles` 0x824e9e00 (`LoadDataFile` |
| `0x82a81650` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:779` | /// `UpdateLoadedAndPlayingEmitters` 0x82a81650: a random emitter whose state detached |
| `0x82a816e0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:797` | /// `FindRandomEmitter` 0x82a816e0: a weighted draw among the emitters not loaded. |
| `0x82a817a0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:829` | /// `FindLoadedEmitterToPlay` 0x82a817a0: the idle loaded emitter loaded longest. |
| `0x82a81828` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:545` | /// `AddEmitterVolume` 0x82a81828 for each volume of a file: a cube of the largest size. |
| `0x82a818f0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1057` | /// `UpdateVolumeList` 0x82a818f0: each loaded AEMS volume plays (`PlayVolume` 0x824f89d0) |
| `0x82a81998` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1179` | /// `UpdateDLCAsyncLoad` 0x824f9098: the loaded file's volumes join (`AddDataFile` 0x82a81998). |
| `0x82a81ae0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:526` | /// 0x82a811a0 each, `ResolveFiles` 0x82a81ae0) and `EnableEmitterSwap(true)`. |
| `0x82a81b58` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1152` | /// `GetVolumes` 0x82a81b58: up to `count` statuses of `kind`, in list order. |
| `0x82a81bd8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1095` | /// `UpdateMusicVolumes` 0x82a81bd8 and `UpdateCurrentMusicVolume` 0x824f8cf0: the first music |
| `0x82a81c80` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1136` | /// 0x82a81c80): the first reverb volume other than `exclude`, if its reverb is known. |
| `0x82a81e70` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:843` | /// `TryToPlayRandomTrigger` 0x82a81e70. |
| `0x82a81f10` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:755` | /// `UpdateUnloadRandomEmitter` 0x82a81f10: an unloading emitter whose slot emptied is not |
| `0x82a81fd8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1023` | /// `RemoveUntouched` 0x82a81fd8: a volume the camera left detaches; a non-AEMS one leaves |
| `0x82a82188` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:891` | /// `UpdateVolumeUnloads` 0x82a82188: an unloading volume leaves once its slot is empty. |
| `0x82a825d0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:446` | /// `EmitterSystem::EmitterSystem` 0x82a825d0 and the `Sk8EmitterSystem` fields |
| `0x82a82708` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:721` | /// `ChangeEmitters` 0x82a82708: the old random emitters are dropped (unloading the loaded |
| `0x82a82800` | TU3 | `crates/skate2-audio/src/world/environment.rs:389` | // Skate 3's `EB` table keeps a folder per entry; TU3's name accessor (0x82a82800) |
| `0x82a82910` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:953` | /// `UpdateOrAddToVolumeList` 0x82a82910: a known volume is touched (an AEMS one only while |
| `0x82a82bd8` | Skate 2? | `crates/skate2-audio/src/system_contest.rs:232` | /// `EmitterSystem::EnableEmitterSwap` 0x82a82bd8 with the EMITTER states. |
| `0x82a82bd8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:682` | /// `EnableEmitterSwap` 0x82a82bd8: disabling detaches the random emitters, drops them |
| `0x82a82db0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:337` | /// The distance `CheckVolume` 0x82a82db0 computes for a camera in a volume, normalised so the |
| `0x82a82db0` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:924` | /// `CheckVolume` 0x82a82db0: a volume of an active game state that holds the camera. |
| `0x82a832e8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:174` | /// else takes the next; `GetCollisions` 0x82a832e8 sorts the entries whose box overlaps the query |
| `0x82a833c8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:192` | /// `Remove` 0x82a833c8: the first entry holding the volume. |
| `0x82a83448` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:914` | /// `SearchOctTree` 0x82a83448: `CheckVolume` for each volume whose box meets the camera's |
| `0x82a83598` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:5` | //! Each update (`EmitterSystem::Update` 0x82a83598, after the state managers' `ProcessUpdate`) |
| `0x82a83598` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:618` | /// `EmitterSystem::Update` 0x82a83598 at the camera position (`MicrophoneMan` microphone 0); |
| `0x82a836d8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:1198` | /// `UnloadDLCData` 0x824f9148: `RemoveDataFile` 0x82a836d8, then the DLC slot unloads. |
| `0x82a83ae8` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:2` | //! `Sk8::Audio::EmitterSystem` (0x82a81100-0x82a83ae8): the sound volumes of the `.ems` data |
| `0x82a8c330` | Skate 2? | `crates/skate2-core/src/session/selected_production_input_owner.rs:619` | /// 0x82a8c330 reads as Brake held; the port's front end pins that player instead. |
| `0x82a8c330` | Skate 2? | `crates/skate2-core/src/session/selected_session_frontend_world.rs:8` | //! with Brake held (`PlayerInputData::Unpack` 0x82a8c330), so it stands braking; the port parks |
| `0x82a8d250` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:21` | /// `tSubLocationDesc` bytes: `GetSpawnLocation` (`0x82a8d250`) indexes the sub-locations |
| `0x82a8d250` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:180` | /// `GetSpawnLocation(name, index)` (`0x82a8dce8` → `0x82a8d250`): the index's |
| `0x82a8d250` | Skate 2? | `crates/skate2-core/src/session/selected_session_party_play.rs:104` | /// `cMsgSpawnPlayer` at `GetSpawnLocation(location, i)` 0x82a8d250 (the location's rows with |
| `0x82a8dc40` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:151` | /// `GetLocationInfo` (`0x82a8dc40`): the first entry of the hash (streamed entries are |
| `0x82a8dce8` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:180` | /// `GetSpawnLocation(name, index)` (`0x82a8dce8` → `0x82a8d250`): the index's |
| `0x82a8ddd0` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:165` | /// `GetNumSpawnLocations` (`0x82a8ddd0`): the location's sub-locations, 0 for an unknown one. |
| `0x82a8de18` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:170` | /// `IsValidLocation` (`0x82a8de18`). |
| `0x82a8de48` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:175` | /// `GetLocationOrientation` (`0x82a8de48`): the location's transform. |
| `0x82a8e430` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:4` | //! and `DRegArena` (`0x82a8e430`) removes them again. The global locator arenas |
| `0x82a8e980` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:3` | //! `0x82a8e980`) under `GetHashValue32(name)`, after the entries already held for that hash, |
| `0x82a8eff8` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:1` | //! `Sk8::cLocationManager` over the registered arenas: `RegArena` (`0x82a8eff8`) adds every |
| `0x82a8f3b0` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:5` | //! (`cGlobalFileLocationManager::LoadGlobalLocations` `0x82a8f3b0`, every |
| `0x82a8f3b0` | Skate 2? | `crates/skate2-core/src/session/selected_location_manager.rs:93` | /// `LoadGlobalLocations` (`0x82a8f3b0`): every global locator arena, in the |
| `0x82aaf270` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:1` | //! The context systems (`Contexts`, the contextsystem 1.01.00 package at 0x82aaf270-0x82ab89a0, |
| `0x82aaf8b8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:57` | /// `ContextSystemManager::Update` 0x82aaf8b8 walks its `eastl::hash_map` of systems: buckets of |
| `0x82aaf8b8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:747` | /// `Sk8ContextSysMan::Update` (`ContextSystemManager::Update` 0x82aaf8b8): each system in the |
| `0x82ab0278` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:6` | //! 0x82ab0278, seeded with the time base) and posts `kPostLoad` to each. A system is its |
| `0x82ab0fd8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:146` | /// `GetTimeSince` 0x82ab0fd8. |
| `0x82ab15e8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:268` | /// `EACrandom` 0x82ab15e8: the words chained with carries, the last counting up (its wrap |
| `0x82ab17d8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:399` | /// `QueryContext` 0x82ab17d8: the context's last use, times used or time since its last use |
| `0x82ab24d0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:177` | /// `AddContext` 0x82ab25e8 / `ValidContext::AddContext` 0x82ab24d0, while there is room. |
| `0x82ab25e8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:177` | /// `AddContext` 0x82ab25e8 / `ValidContext::AddContext` 0x82ab24d0, while there is room. |
| `0x82ab26b8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:194` | /// `Compact` 0x82ab26b8. |
| `0x82ab2ca0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:478` | /// `CreateTriggerHashTable` 0x82ab2d28) and `PopulatePassiveList` 0x82ab2ca0. |
| `0x82ab2d28` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:478` | /// `CreateTriggerHashTable` 0x82ab2d28) and `PopulatePassiveList` 0x82ab2ca0. |
| `0x82ab2fc8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:477` | /// `kPostLoad` (`HandleMessage` 0x82ab8880): `InitLoadedContexts` 0x82ab2fc8 (the ids and |
| `0x82ab3080` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:15` | //! at random and its output event called (`EventSender::Emit` 0x82ab3080), which is a speech |
| `0x82ab3080` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:594` | /// `EventSender::Emit` 0x82ab3080: the output's parameters, then its `O_*` function |
| `0x82ab3650` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:199` | /// `RemoveExpired` 0x82ab3650: triggered contexts older than their expiry. |
| `0x82ab36e8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:210` | /// `RemoveContext` 0x82ab36e8: after a triggered context spoke, every triggered context of |
| `0x82ab3820` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:226` | /// `Merge` 0x82ab3820: the two lists by descending priority, `b`'s first on a tie (a |
| `0x82ab3ab8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:252` | /// `InitRandom` 0x82ab3ab8: the six words from the seed, then `32 - seed * 65537 % 32` draws. |
| `0x82ab3b80` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:11` | //! parameters and joins the triggered list for its expiry time (`OnEvent` 0x82ab3b80). Each |
| `0x82ab3b80` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:495` | /// `OnEvent` 0x82ab3b80: the trigger's contexts take the call's parameters and join the |
| `0x82ab3cf8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:733` | /// client is `CsisCallback` 0x82ab3cf8 when the system subscribed to it). |
| `0x82ab3dd0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:701` | /// `.csi` (1) and their `.evt` (2, `AddCsisEvents` 0x82ab3dd0); every entry of `<group>xml.big` |
| `0x82ab3dd0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:97` | /// The Csis table `iSPCH_InitCsis` 0x82ee5938 and `ContextSystemImpl::AddCsisEvents` 0x82ab3dd0 |
| `0x82ab4cb0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:542` | /// 0x82ab4cb0) as the current context; the first that fails moves to the front. |
| `0x82ab4fb8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:541` | /// `Context::Evaluate` 0x82ab4fb8: the conditions in order (`Condition::ExecuteScript` |
| `0x82ab54f8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:460` | /// `AddContext` 0x82ab6330: a context replaces those of its id (`RemoveContextById` 0x82ab54f8, |
| `0x82ab55b8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:559` | /// `Choose` 0x82ab55b8: from `start`, the evaluation list down to the best priority found |
| `0x82ab5750` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:523` | /// sources (`ParamGetter::GetParamValue` 0x82ab5750). |
| `0x82ab6330` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:460` | /// `AddContext` 0x82ab6330: a context replaces those of its id (`RemoveContextById` 0x82ab54f8, |
| `0x82ab63c0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:702` | /// is a context file (`AddData` 0x82ab6808, `AddContextXml` 0x82ab63c0). |
| `0x82ab6808` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:702` | /// is a context file (`AddData` 0x82ab6808, `AddContextXml` 0x82ab63c0). |
| `0x82ab7dd8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:13` | //! (`ContextSystemImpl::Update` 0x82ab7dd8): the triggered and passive contexts (passive ones |
| `0x82ab7dd8` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:607` | /// `ContextSystemImpl::Update` 0x82ab7dd8: the speaker's priority decays by |
| `0x82ab8418` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:446` | /// The `ContextSystemImpl` constructor 0x82ab8418 for a setup file's sizes: the lists |
| `0x82ab8880` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:477` | /// `kPostLoad` (`HandleMessage` 0x82ab8880): `InitLoadedContexts` 0x82ab2fc8 (the ids and |
| `0x82ab8880` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:741` | /// `kInterruptsOn` / `kInterruptsOff` (`HandleMessage` 0x82ab8880): while on, only |
| `0x82ab89a0` | Skate 2? | `crates/skate2-audio/src/speech/context/mod.rs:1` | //! The context systems (`Contexts`, the contextsystem 1.01.00 package at 0x82aaf270-0x82ab89a0, |
| `0x82ac7770` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skate_world.rs:155` | // TriangleVolume ctor82AC7770 retains these defaults when |
| `0x82ac8a68` | TU3 | `crates/skate2-core/src/physics/modes/skate3/skate_world.rs:98` | /// TU3 ClusteredMesh::GetUnitVolumes (82AC8A68): fdivs then fsubs, |
| `0x82aca170` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:120` | //82ACA170 decoder's per-volume AABB gate; no leaf tolerance pad. |
| `0x82aca4e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query/transform.rs:51` | //82ACA4E0..550: face is formed from decoded WORLD vertices and refined twice. |
| `0x82aca4e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:168` | //82ACA4E0..550, from decoded authored winding, with two rsqrt refinements. |
| `0x82ad6450` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:43` | /// 82AD6450: select the side with greater containment margin, ties low. |
| `0x82ad6530` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:55` | /// 82AD6530: could this entry fit another level? Strict comparisons. |
| `0x82ad65d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:100` | /// 82AD65D8: walk old head-first list and prepend each redistributed entry; |
| `0x82ad6830` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:1` | //! Original S3 loose octree insertion/query order, 82AD6830/82476158. |
| `0x82ad6d18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:116` | /// 82AD6D18 pushes resident then ascending leaf lists; 82476158 consumes |
| `0x82ad8758` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/colliders.rs:134` | // CreateGP82AD8758 starts basis composition with a product; |
| `0x82ade468` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/colliders.rs:116` | // Same multiply/add order as the Volume-to-world vertex path82ADE468. |
| `0x82adf7b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:141` | //82ADF7B8: wake sleeping state2, preserving bit3, and clear cooldown |
| `0x82ae09b8` | Skate 2? | `crates/skate2-core/src/physics/board/runtime/skateboard_ground_frame.rs:758` | // `Assembly::Activate` 0x82ae09b8 in thaw-list and Part order: a |
| `0x82ae1958` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8263` | /// `0x82ae1958..0x82ae1ea4`. All sixteen source vectors are already copied, so |
| `0x82ae1e30` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8108` | /// `0x82ae1e30`: ordinary floating-point reciprocal, not a Xenon `vrefp` oracle. |
| `0x82ae1ea4` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8263` | /// `0x82ae1958..0x82ae1ea4`. All sixteen source vectors are already copied, so |
| `0x82ae253c` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:2112` | /// Raw `HorusPipeline` anchors are `0x82ae253c..0x82ae2814`. Records execute |
| `0x82ae2540` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:231` | /// `0x82ae2540..0x82ae2820`, with the per-record body at |
| `0x82ae2568` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:232` | /// `0x82ae2568..0x82ae2814`. This helper preserves the exact right/up/at |
| `0x82ae25c4` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:340` | // Both native forms project angular RF directly (82ae25c4/82ae2f60). |
| `0x82ae27b4` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:2116` | /// record and commits A0, B0, A2, B2 (`0x82ae27b4`, `0x82ae27b8`, |
| `0x82ae27b8` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:2116` | /// record and commits A0, B0, A2, B2 (`0x82ae27b4`, `0x82ae27b8`, |
| `0x82ae27d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:22` | /// stages `82AE30F0` / `82AE27D0` as their loop count. Measured in TU3 (recomp, Aletown, |
| `0x82ae2804` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:2117` | /// `0x82ae2804`, `0x82ae2810`). Self-pairs therefore retain native B-wins |
| `0x82ae2810` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:2117` | /// `0x82ae2804`, `0x82ae2810`). Self-pairs therefore retain native B-wins |
| `0x82ae2814` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:232` | /// `0x82ae2568..0x82ae2814`. This helper preserves the exact right/up/at |
| `0x82ae2814` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:2112` | /// Raw `HorusPipeline` anchors are `0x82ae253c..0x82ae2814`. Records execute |
| `0x82ae2820` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:231` | /// `0x82ae2540..0x82ae2820`, with the per-record body at |
| `0x82ae2990` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:3` | //! The indirect-island Contact consumer at `0x82ae2990..0x82ae2c48` |
| `0x82ae2990` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8156` | /// Native consumer: `0x82ae2990..0x82ae2c48`; PDB size: `0x100` bytes. |
| `0x82ae2990` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8600` | /// Native per-record anchors are `0x82ae2990..0x82ae2c48`. The helper retains |
| `0x82ae2c48` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:3` | //! The indirect-island Contact consumer at `0x82ae2990..0x82ae2c48` |
| `0x82ae2c48` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8156` | /// Native consumer: `0x82ae2990..0x82ae2c48`; PDB size: `0x100` bytes. |
| `0x82ae2c48` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:8600` | /// Native per-record anchors are `0x82ae2990..0x82ae2c48`. The helper retains |
| `0x82ae2f60` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:340` | // Both native forms project angular RF directly (82ae25c4/82ae2f60). |
| `0x82ae3054` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:262` | /// Island Horus stores Position then Angular at `0x82ae3054..0x82ae3058`. |
| `0x82ae3058` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:262` | /// Island Horus stores Position then Angular at `0x82ae3054..0x82ae3058`. |
| `0x82ae30f0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:22` | /// stages `82AE30F0` / `82AE27D0` as their loop count. Measured in TU3 (recomp, Aletown, |
| `0x82ae3190` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:5` | //! (`0x82ae3190..0x82ae3224`) appends to its tail. `RigidBody::Reset` |
| `0x82ae3224` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:5` | //! (`0x82ae3190..0x82ae3224`) appends to its tail. `RigidBody::Reset` |
| `0x82ae3750` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:19` | //! `Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`) is recovered completely |
| `0x82ae3750` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3443` | /// Reconstructs `Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`). |
| `0x82ae3750` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:514` | // row typo using the raw 0x82ae3750..0x82ae3930 RQD discriminator. |
| `0x82ae3930` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:19` | //! `Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`) is recovered completely |
| `0x82ae3930` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3443` | /// Reconstructs `Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`). |
| `0x82ae3930` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:514` | // row typo using the raw 0x82ae3750..0x82ae3930 RQD discriminator. |
| `0x82ae3fe4` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1803` | /// Native table facts are frozen at `0x82ae3fe4..0x82ae40e0`: admitted records |
| `0x82ae40e0` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1803` | /// Native table facts are frozen at `0x82ae3fe4..0x82ae40e0`: admitted records |
| `0x82ae4488` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_dynamic_update.rs:5` | //! order come from `0x82ae4488..0x82ae4904`. Exact Xenon `vrsqrtefp128` seed |
| `0x82ae4544` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_dynamic_update.rs:68` | // Native first observable body store: COM at 0x82ae4544. |
| `0x82ae45c8` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_dynamic_update.rs:74` | // Quaternion store at 0x82ae45c8. |
| `0x82ae4904` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_dynamic_update.rs:5` | //! order come from `0x82ae4488..0x82ae4904`. Exact Xenon `vrsqrtefp128` seed |
| `0x82ae4a40` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:2107` | /// `Simulation::BatchIntegrator` (`0x82ae4a40..0x82ae4a84`) is a do/while, |
| `0x82ae4a84` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:2107` | /// `Simulation::BatchIntegrator` (`0x82ae4a40..0x82ae4a84`) is a do/while, |
| `0x82ae4a88` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:1285` | /// One-shot semantic XYZ `SpyContactJacobians` (0x82ae4a88), from the |
| `0x82ae4b40` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1788` | /// Current semantic XYZ read for `SpyContactJacobians` (0x82ae4b40/4b60). |
| `0x82ae4b40` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:1341` | // Source 0x82ae4b40 reads B first, 0x82ae4b60 then A. |
| `0x82ae4b60` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:1341` | // Source 0x82ae4b40 reads B first, 0x82ae4b60 then A. |
| `0x82ae4bf8` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3` | //! `DriveJacobian::Build` (`0x82ae4bf8..0x82ae5918`) begins by clearing three |
| `0x82ae4bf8` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:69` | /// Native reference: `DriveJacobian::Build` at `0x82ae4bf8`; PDB size `0x180`. |
| `0x82ae4c14` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:130` | /// Native anchors: `0x82ae4c14`, `0x82ae4c20`, and `0x82ae4c28`. |
| `0x82ae4c20` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:130` | /// Native anchors: `0x82ae4c14`, `0x82ae4c20`, and `0x82ae4c28`. |
| `0x82ae4c28` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:130` | /// Native anchors: `0x82ae4c14`, `0x82ae4c20`, and `0x82ae4c28`. |
| `0x82ae4c98` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1180` | /// Native anchors are `0x82ae4c98..0x82ae4f14`, with the later separated |
| `0x82ae4ea4` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1218` | /// Native anchors are `0x82ae4ea4..0x82ae4f90`, `0x82ae5098`, |
| `0x82ae4f14` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1180` | /// Native anchors are `0x82ae4c98..0x82ae4f14`, with the later separated |
| `0x82ae4f90` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1218` | /// Native anchors are `0x82ae4ea4..0x82ae4f90`, `0x82ae5098`, |
| `0x82ae4f94` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3509` | /// Linear native anchors are `0x82ae4f94..0x82ae5094`; angular anchors are |
| `0x82ae4fec` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3483` | /// Native occurrences begin at `0x82ae4fec` for linear SOFT coefficients. |
| `0x82ae5094` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3509` | /// Linear native anchors are `0x82ae4f94..0x82ae5094`; angular anchors are |
| `0x82ae5098` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1218` | /// Native anchors are `0x82ae4ea4..0x82ae4f90`, `0x82ae5098`, |
| `0x82ae50fc` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3495` | /// `0x82ae50fc..0x82ae512c`, without claiming Xenon seed parity. |
| `0x82ae512c` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3495` | /// `0x82ae50fc..0x82ae512c`, without claiming Xenon seed parity. |
| `0x82ae5138` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1335` | /// Native anchors are `0x82ae5138..0x82ae54a4`. `dot3` is the replaceable |
| `0x82ae515c` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1219` | /// `0x82ae515c`, and `0x82ae5170`. The cross-add preserves this Build slice's |
| `0x82ae5170` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1219` | /// `0x82ae515c`, and `0x82ae5170`. The cross-add preserves this Build slice's |
| `0x82ae51c8` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1273` | /// `0x82ae51c8..0x82ae5238`. Projection executes Up, Right, At and is returned |
| `0x82ae5238` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1273` | /// `0x82ae51c8..0x82ae5238`. Projection executes Up, Right, At and is returned |
| `0x82ae52c0` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3510` | /// `0x82ae52c0..0x82ae53c0`. Only raw type `1` is SOFT. HARD (`2`), `NO_DRIVE` |
| `0x82ae53c0` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3510` | /// `0x82ae52c0..0x82ae53c0`. Only raw type `1` is SOFT. HARD (`2`), `NO_DRIVE` |
| `0x82ae5474` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1475` | /// Native anchors are `0x82ae5474..0x82ae5748`. A and B are gated |
| `0x82ae54a4` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1335` | /// Native anchors are `0x82ae5138..0x82ae54a4`. `dot3` is the replaceable |
| `0x82ae5748` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1475` | /// Native anchors are `0x82ae5474..0x82ae5748`. A and B are gated |
| `0x82ae574c` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1520` | /// Native anchors are `0x82ae574c..0x82ae5904`. Linear and angular reciprocal |
| `0x82ae5904` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1520` | /// Native anchors are `0x82ae574c..0x82ae5904`. Linear and angular reciprocal |
| `0x82ae5918` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3` | //! `DriveJacobian::Build` (`0x82ae4bf8..0x82ae5918`) begins by clearing three |
| `0x82ae596c` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:3` | //! The pre-Swing numeric/value graph across `0x82ae596c..0x82ae5dcc` |
| `0x82ae596c` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:108` | /// `JointJacobian::Build` `0x82ae596c..0x82ae5dcc`. |
| `0x82ae5dcc` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:3` | //! The pre-Swing numeric/value graph across `0x82ae596c..0x82ae5dcc` |
| `0x82ae5dcc` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:64` | /// Values live immediately after `0x82ae5dcc`, before Swing dispatch. |
| `0x82ae5dcc` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_stage1.rs:108` | /// `JointJacobian::Build` `0x82ae596c..0x82ae5dcc`. |
| `0x82ae6170` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_twist_prediction.rs:1` | //! Recovered typed continuation for `JointJacobian::Build` 0x82ae6170..0x82ae6304. |
| `0x82ae6170` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_twist_prediction.rs:53` | /// Twist dispatch at 0x82ae6170. |
| `0x82ae6170` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_twist_prediction.rs:89` | /// Ordinary-finite translation of 0x82ae6170..0x82ae6304. |
| `0x82ae6304` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_twist_prediction.rs:1` | //! Recovered typed continuation for `JointJacobian::Build` 0x82ae6170..0x82ae6304. |
| `0x82ae6304` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_twist_prediction.rs:73` | /// Value-only boundary after 0x82ae6304. |
| `0x82ae6304` | Skate 2? | `crates/skate2-core/src/physics/solver/joints/simulation_joint_build_twist_prediction.rs:89` | /// Ordinary-finite translation of 0x82ae6170..0x82ae6304. |
| `0x82ae64f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/hand_drives.rs:2` | //! Factory82AE64F8 stores argument5 at drive16 (A), argument4 at20 (B): |
| `0x82ae6e00` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:3` | //! `Simulation::AddRigidBody` (`0x82ae6e00..0x82ae79e0`) takes the head of the |
| `0x82ae79e0` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:3` | //! `Simulation::AddRigidBody` (`0x82ae6e00..0x82ae79e0`) takes the head of the |
| `0x82ae79e0` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1751` | /// Native `Simulation::DriveBatchBuild` at `0x82ae79e0..0x82ae7a74` |
| `0x82ae7a74` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:1751` | /// Native `Simulation::DriveBatchBuild` at `0x82ae79e0..0x82ae7a74` |
| `0x82ae7b30` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:173` | /// `0x82ae7b30..0x82ae7b3c`, then clears all four vectors once at |
| `0x82ae7b3c` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:173` | /// `0x82ae7b30..0x82ae7b3c`, then clears all four vectors once at |
| `0x82ae7b80` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:174` | /// `0x82ae7b80..0x82ae7bb0` and stores each row address in the PDB-named body |
| `0x82ae7b80` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1983` | /// recovered at `0x82ae7b80..0x82ae7bb0`; interpreting the four vectors is |
| `0x82ae7bb0` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:174` | /// `0x82ae7b80..0x82ae7bb0` and stores each row address in the PDB-named body |
| `0x82ae7bb0` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1983` | /// recovered at `0x82ae7b80..0x82ae7bb0`; interpreting the four vectors is |
| `0x82ae7c94` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:175` | /// `mId/+0x1c` field at `0x82ae7c94/0x82ae7cc8`. Add, Reset, Remove, and reuse |
| `0x82ae7cc8` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:175` | /// `mId/+0x1c` field at `0x82ae7c94/0x82ae7cc8`. Add, Reset, Remove, and reuse |
| `0x82ae89b0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/condition_nodes.rs:74` | //82AE89B0 confirms case-sensitive comparison. |
| `0x82ae89b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:33` | // Factory compares against bs with strcmp82AE89B0. Every unequal |
| `0x82ae89b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/twist_lean.rs:16` | ///82BCC490 compares exact bytes through strcmp82AE89B0. |
| `0x82ae9058` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:466` | /// (`0x82ae9058..0x82ae90e8`). |
| `0x82ae90e8` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:466` | /// (`0x82ae9058..0x82ae90e8`). |
| `0x82b24878` | Skate 2? | `crates/skate2-core/src/physics/board/collision/skateboard_contact_surface.rs:2` | //! `ChoosePhysicsSurfaceType` 0x82b24878. Contact production is shared, not repeated. |
| `0x82b25480` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:1` | //! Owner-qualified live Part get/set, 0x826c3480 / 0x82b25480. |
| `0x82b25480` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:145` | /// Detached branch of `Part::SetTransform` (0x82b25480): copy the raw |
| `0x82b25480` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1590` | /// Body write set of attached `Part::SetTransform` (0x82b25480). |
| `0x82b25b70` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:242` | /// (0x82b25b70). Host validation errors occur before mutation; no body is |
| `0x82b26df0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1749` | /// Reference: `Skateboard::Toolkit_CalcBraking` at `0x82b26df0`. |
| `0x82b27078` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1755` | /// Reference: `Skateboard::Toolkit_CalcLinearDrag` at `0x82b27078`. |
| `0x82b272f8` | Skate 2 | `crates/skate2-core/src/physics/skate3_player.rs:1398` | // S2 Toolkit_CalcPushAcceleration82b272f8 has one change cap at all speeds. |
| `0x82b27530` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1758` | /// Reference: `Skateboard::Toolkit_CalcCollisionForce` at `0x82b27530`. |
| `0x82b27530` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1824` | /// Reference: `Skateboard::Toolkit_CalcCollisionForce` at `0x82b27530`. |
| `0x82b28260` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1752` | /// Reference: `Skateboard::ToolKit_CalcSlideFriction` at `0x82b28260`. |
| `0x82b28b70` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1733` | /// Reference: `Skateboard::Toolkit_CalcFrontFootForce` at `0x82b28b70`. |
| `0x82b28d98` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1741` | /// Reference: `Skateboard::Toolkit_CalcRearFootForce` at `0x82b28d98`. |
| `0x82b29de0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1821` | /// Reference: `Skateboard::UpdateSteeringTilt` at `0x82b29de0`. |
| `0x82b29ee0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1839` | /// Reference: `Skateboard::SetVelocity` at `0x82b29ee0`. |
| `0x82b2fbf4` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out.rs:497` | /// 0x82b2fbf4..fc20: output guard before the actual retained source read. |
| `0x82b315a8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2265` | /// Reference: `Skateboard::Toolkit_CalcGroundJump` at `0x82b315a8`. After the |
| `0x82b316a8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2166` | /// Reference: `Skateboard::Toolkit_CalcGroundJump` at `0x82b316a8` through |
| `0x82b3189c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2170` | /// is prepared-tangent length multiplied by the angle curve (0x82b3189c..31908). |
| `0x82b31918` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2167` | /// `0x82b31918`. Both velocities are rejected from the jump normal. The |
| `0x82b31960` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2213` | /// portion of `Skateboard::Toolkit_CalcGroundJump` at `0x82b31960` through |
| `0x82b31dfc` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2214` | /// `0x82b31dfc`. |
| `0x82b36a58` | Skate 2? | `crates/skate2-core/src/physics/board/collision/skateboard_contact_surface.rs:1` | //! Retained surface-only projection of board `PostPhysics` 0x82b36a58 and |
| `0x82b372e0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:398` | // S3 82C08634 writes inertia+36; S2 82B372E0 names this |
| `0x82b374e8` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_materials.rs:3` | //!82B374E8 confirms their identities. Grind resistance is a separate force. |
| `0x82b382dc` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1066` | // Selected Body ctor Reset82b382dc precedes GetPhysicsIsland82b382e8. |
| `0x82b382e8` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1066` | // Selected Body ctor Reset82b382dc precedes GetPhysicsIsland82b382e8. |
| `0x82b382e8` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1078` | // SkateboardBody caches GetPhysicsIsland at 82b382e8 BEFORE Init |
| `0x82b387a4` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:806` | /// Whole board assembly projection of 82b387a4..82b38918. Safe host |
| `0x82b387e8` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1079` | // 82b387e8; Add82b38808 later uses that cached selection. |
| `0x82b38808` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1079` | // 82b387e8; Add82b38808 later uses that cached selection. |
| `0x82b38918` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:806` | /// Whole board assembly projection of 82b387a4..82b38918. Safe host |
| `0x82b38ce0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1668` | /// `Skateboard::AddSkateboardForce` at `0x82b38ce0`. This native semantic type |
| `0x82b38ce0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1717` | /// Reference: `Skateboard::AddSkateboardForce` at `0x82b38ce0`. |
| `0x82b38cec` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1703` | /// Reference: `0x82b38cec..0x82b38d08` silently skips each insertion whose |
| `0x82b38d08` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1703` | /// Reference: `0x82b38cec..0x82b38d08` silently skips each insertion whose |
| `0x82b38fd0` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_grinds.rs:8` | /// Reset82b38fd0: five source zero-valued enums followed by `GRIND_TYPE_NONE`(-1). |
| `0x82b39098` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:3210` | /// Reference: `ProcessedPhysIn::Reset` at `0x82b39098`. |
| `0x82b45d58` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1630` | /// `ResetSkeleton`82b45d58..82b45d84 after its Part transform update. |
| `0x82b45d84` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1630` | /// `ResetSkeleton`82b45d58..82b45d84 after its Part transform update. |
| `0x82b46098` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:224` | /// `Skeleton::GetLaunchInfo` at `0x82b46098`. |
| `0x82b4c044` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1812` | /// 0x82b4c044/0x82b4c054, NOT the current retained inertia owner's value. |
| `0x82b4c054` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1812` | /// 0x82b4c044/0x82b4c054, NOT the current retained inertia owner's value. |
| `0x82b50614` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1464` | /// `0x82b50614`, including W bits independently of XYZ. The checked host |
| `0x82b545e0` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1650` | /// `SkeletonBody::ApplyDisplacementToPart` 0x82b545e0..0x82b546c0. |
| `0x82b54634` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:383` | /// Actual Part/body lookup for 0x82b54634. Unlike reset, source has no |
| `0x82b546c0` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1650` | /// `SkeletonBody::ApplyDisplacementToPart` 0x82b545e0..0x82b546c0. |
| `0x82b56a10` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3368` | /// Also matches `SkeletonData::Init` at `0x82b56a10..0x82b56b5c`: |
| `0x82b56b5c` | Skate 2? | `crates/skate2-core/src/physics/solver/drives/simulation_drive_jacobian.rs:3368` | /// Also matches `SkeletonData::Init` at `0x82b56a10..0x82b56b5c`: |
| `0x82b61bb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_dismounting.rs:1` | //! TU3 AirDismounting Begin82BB93E0, Update82BB94C0, empty End82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_execute.rs:18` | //erase the construction-map key. Begin/Update are82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:50` | // Vtable8231FF24 Update/End both point to empty82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:62` | //lifecycle slots in82309664 are82B61BB8 (blr), with no state writes. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:468` | // Vtable8231F88C End points to empty82B61BB8. |
| `0x82b61bb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_finger_flip.rs:1` | //! FingerFlipOut: TU3 Begin82BB3198, Update82BB7C40, empty End82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:74` | // HandBusy Update vtable8231F97C+52 is blr82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:81` | // Begin vtable82321000+48 is blr82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:57` | //Native End82B61BB8 is the no-op lifecycle method. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:59` | _ => {}                                         //Original End82B61BB8 is empty. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:103` | _ => {} //Update82B61BB8 is empty; End82BB0498 erases the key. |
| `0x82b61bb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:21` | ///TU3 vtable82309664: Begin/Update/End all point to the empty82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:32` | /// Component absence skips the write. Begin/End are82B61BB8 no-ops. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:74` | ///Original vtable End82B61BB8 does not reset the capture or pending byte. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:2` | //! Update82BBA070, instance82BBA390. End is the empty82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:2` | //! Both have Begin effects; Update and End point to82B61BB8 (raw blr). |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/runout.rs:2` | //! End is82B61BB8. Values are sampled once per activation, never on Update. |
| `0x82b61bb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:1` | //! Original ToggleBoard lifecycle adapter, TU3 82BA8FD8/8FE8 and 82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:40` | // End82B61BB8 does not stop the persistent channel. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/twist_lean.rs:23` | ///not an absent-component value. End is the original82B61BB8 no-op. |
| `0x82b61bb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_runout.rs:2` | //! Begin82BBA4A8, Update82BBA770, End82B61BB8. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout_execute.rs:25` | //82BC07A8/07F8, Update82B61BB8 is empty. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout_execute.rs:33` | } //Original End82B61BB8. |
| `0x82b61bb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/phase.rs:45` | // Vtable8232861C+20 is82B61BB8 (empty), not common grind Exit. |
| `0x82b61bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/transition.rs:181` | PhysicalStateId::Sleeping \| PhysicalStateId::Teleporting => {} //82B61BB8. |
| `0x82b6cc78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings/board.rs:3` | //!82B6CC78 stores the returned collection layout at wrapper4 = state40. |
| `0x82b8c678` | Skate 2? | `crates/skate2-audio/src/system.rs:1108` | /// A movie's `AudioRenderer_Rwac` (constructor 0x82b8cc00, `Init` 0x82b8c678), replacing any |
| `0x82b8c868` | Skate 2? | `crates/skate2-audio/src/system.rs:1130` | /// The movie renderer's `GetRenderingPosition` (0x82b8c868) in milliseconds; none without a |
| `0x82b8c988` | Skate 2? | `crates/skate2-audio/src/system.rs:1123` | /// The movie renderer's `SetVolume` (0x82b8c988). |
| `0x82b8cc00` | Skate 2? | `crates/skate2-audio/src/system.rs:1108` | /// A movie's `AudioRenderer_Rwac` (constructor 0x82b8cc00, `Init` 0x82b8c678), replacing any |
| `0x82b8d320` | Skate 2? | `crates/skate2-audio/src/system.rs:1116` | /// The movie renderer's `Begin` (0x82b8d320). |
| `0x82b95620` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing_settings.rs:14` | /// non-scalar types: native82B95620 does not filter the attribute type. |
| `0x82b9584c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing_settings.rs:1` | //! Stock pushing settings and authored clip metadata (TU382B953E8..82B9584C). |
| `0x82b959e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:70` | //VT82321360 Begin=82BBE5D0; shared clip lengths from82B959E8. |
| `0x82b965d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:171` | // pose evaluation (82B965D8), not parameter publication. |
| `0x82b96ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:194` | //82B96AB0 queries channel first regardless of use_attributes or weight. |
| `0x82b96b20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:28` | // Channel::SetAttributes82B96B20 reaches the child even before |
| `0x82b96b50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:170` | //82B96B50..64 forwards parameters unconditionally. Weight gates |
| `0x82b96f88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:144` | //82B96F88: complete channel transitions, remove expired channels, |
| `0x82b96ff0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:64` | ///82B96FF0 reads Channel136, set/cleared by Start/CompleteTransition. |
| `0x82b97060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings/board.rs:1` | //! Ground ctor82D305D8 passes state36 to82B97060, whose full64 class/key |
| `0x82b970d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:627` | //ISkaterAnim v12=82B970D8/v28=82B97140 read live bits29/30. |
| `0x82b970d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:297` | //ISkaterAnim virtual12=82B970D8 reads bit29=fakie. |
| `0x82b970d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:46` | //ISkaterAnim virtual12=82B970D8 reads the live bit29. |
| `0x82b97110` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:214` | //82BB2670/26D0 -> ISkaterAnim20 ->82B97110, bit28. |
| `0x82b97140` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:627` | //ISkaterAnim v12=82B970D8/v28=82B97140 read live bits29/30. |
| `0x82b97140` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:53` | // SkaterAnim vtable8231E170+28=82B97140: full15180 bit30. |
| `0x82b97150` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:361` | //82BB95D8 -> FlipBoardBackwards82B97150. End/Update are empty. |
| `0x82b971a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:419` | //off-board owner's board-present byte is false. 82B971A8 copies it |
| `0x82b971c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:303` | // TU3 ISkaterAnim v84/82B971C0: write count and OR request bit20. |
| `0x82b97218` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:91` | //82BBB36C ->ISkaterAnimv112/82B97218: one-frame request bit16. |
| `0x82b97228` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:87` | //82BBB328 ->MGv200/8258FBF0;82BBB33C ->ISkaterAnimv116/82B97228. |
| `0x82b972a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:43` | ///82B97308 consumes saved15200 via82B972A8; no playback reset. |
| `0x82b97308` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:43` | ///82B97308 consumes saved15200 via82B972A8; no playback reset. |
| `0x82b97308` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:116` | // clears outgoing fakie/weight bits before GivenStance82B97308. |
| `0x82b97350` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:95` | ///82592C08/82B97350: save15200; ResetToGivenStance consumes it later. |
| `0x82b975b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:27` | //82B975B8..82B97674: GOOFY natural stance, NATURAL relative. |
| `0x82b975f4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:16` | //82B975F4 selects bit27;82B97674 selects bit17 and clears the |
| `0x82b97674` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:16` | //82B975F4 selects bit27;82B97674 selects bit17 and clears the |
| `0x82b97674` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:27` | //82B975B8..82B97674: GOOFY natural stance, NATURAL relative. |
| `0x82b97e38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:48` | // Init82B97E38 retains these references from the actor's |
| `0x82b97e38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:128` | /// Native Initialize82B97E38 selects both orientation/mirror bits for |
| `0x82b97e38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:155` | /// the physical skeleton. Initialize82B97E38 selects rig_Tpose; |
| `0x82b98050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:81` | //82B98050 preserves local-player and board-present flags, and saved15200. |
| `0x82b98050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:40` | ///82B98050 +82D1CAE8 +82D180E8: masks/stance bit, channel teardown, |
| `0x82b98050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:115` | // one-shot request for its ordinary dispatch. ResetSkaterFlags82B98050 |
| `0x82b98078` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:79` | ///SkaterAnim virtual172/82B98078, selected by local-player bit27. |
| `0x82b980a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:567` | //SkaterAnim::GetAnimTree82B980A0 calls AddBindPose82B98118 before |
| `0x82b98118` | TU3? | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:49` | // initial database. AddBindPose82B98118 reuses them even |
| `0x82b98118` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:567` | //SkaterAnim::GetAnimTree82B980A0 calls AddBindPose82B98118 before |
| `0x82b98118` | TU3? | `crates/skate2-core/src/animation/runtime/skate3_gesture_streams.rs:161` | // AddBindPose 82B98118 layer's motion is the chain under its static pose, whichever |
| `0x82b985e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:113` | ///The animation packet is the real82B985E8 publication, including its |
| `0x82b988b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:29` | /// ISkaterAnim virtual120, evaluated by82B988B8. |
| `0x82b988b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:63` | //82B988B8 returns false when its optional context is absent. |
| `0x82b98980` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:49` | ///82B98980 calls82D19010's first-match query. No scalar/status test: |
| `0x82b98980` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:322` | //Collect before pose evaluation consumes clip history;82B98980 then |
| `0x82b98db8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:2` | //! The catalog is constructed by82B98DB8 ->82B98F70, independently checked |
| `0x82b98f70` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:2` | //! The catalog is constructed by82B98DB8 ->82B98F70, independently checked |
| `0x82b98f70` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/gesture_mapping_data.rs:1` | //! TU3 82B98F70 records, re-extracted from original bytes and all 270 map inserts. |
| `0x82b99060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:45` | // Square82B99060..99DAC; Tail82B99E20..9AAD0; Nose82B9AB40..9B7F0. |
| `0x82b99e20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:45` | // Square82B99060..99DAC; Tail82B99E20..9AAD0; Nose82B9AB40..9B7F0. |
| `0x82b9ab40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:45` | // Square82B99060..99DAC; Tail82B99E20..9AAD0; Nose82B9AB40..9B7F0. |
| `0x82b9b864` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:78` | // Tail90 map5676,82B9B864..9BE84; commonkeys82B9BEF4..9CBA4. |
| `0x82b9bef4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:78` | // Tail90 map5676,82B9B864..9BE84; commonkeys82B9BEF4..9CBA4. |
| `0x82b9cc18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:96` | // TailN90 map9460,82B9CC18..9D238; commonkeys82B9D2A8..9DF58. |
| `0x82b9d2a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:96` | // TailN90 map9460,82B9CC18..9D238; commonkeys82B9D2A8..9DF58. |
| `0x82b9dfcc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:114` | // Nose90 map3784,82B9DFCC..9E5EC; commonkeys82B9E65C..9F30C. |
| `0x82b9e65c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:114` | // Nose90 map3784,82B9DFCC..9E5EC; commonkeys82B9E65C..9F30C. |
| `0x82b9f380` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:132` | // NoseN90 map7568,82B9F380..9F9A0; commonkeys82B9FA10..BA06C0. |
| `0x82b9fa10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:132` | // NoseN90 map7568,82B9F380..9F9A0; commonkeys82B9FA10..BA06C0. |
| `0x82ba07f0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:1` | //! Original TU3 gesture membership,82BA10C0 ->82BA07F0. |
| `0x82ba07f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:31` | ///82BA07F0 tests map-key presence, never the intent value. All seven maps |
| `0x82ba07f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_mapping.rs:1` | //! GestureTrickMapping82BA07F0 and CreateTrickIntentFromGesture82BA1C30..2228. |
| `0x82ba07f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_mapping.rs:8` | // 82BC1BE8 prepends entries; 82BA07F0 visits ascending buckets. |
| `0x82ba0a48` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_nodes.rs:62` | // TU382F847C0..82F84850 initialize the names consumed by82BA0A48. |
| `0x82ba0a48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_intent_filter.rs:65` | // 82BA0A48 and original initializers 82F847C0..4850. |
| `0x82ba0f78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:17` | /// Original82BA0F78 does case-sensitive constructor comparisons. |
| `0x82ba10c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/input/gesture_catalog.rs:1` | //! Original TU3 gesture membership,82BA10C0 ->82BA07F0. |
| `0x82ba1680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:20` | /// Original State output consumed by82BA1680/82BA16F0. |
| `0x82ba16f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:20` | /// Original State output consumed by82BA1680/82BA16F0. |
| `0x82ba1768` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_nodes.rs:42` | // Both TU3 constructors82BA23C0/82BA1768 default this to true. |
| `0x82ba1c30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_mapping.rs:1` | //! GestureTrickMapping82BA07F0 and CreateTrickIntentFromGesture82BA1C30..2228. |
| `0x82ba23c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_nodes.rs:42` | // Both TU3 constructors82BA23C0/82BA1768 default this to true. |
| `0x82ba2a90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_nodes.rs:55` | //BoardAdjust82BA2A90 passes true as the constructor default. |
| `0x82ba2ba8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_board_adjust.rs:1` | //! TU3 BoardAdjust instance+8/+12, Begin82BA2BA8, Update82BA2BC8. |
| `0x82ba2bc8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_board_adjust.rs:1` | //! TU3 BoardAdjust instance+8/+12, Begin82BA2BA8, Update82BA2BC8. |
| `0x82ba2d70` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_board_adjust.rs:20` | //82BA2D70: crossing the +/-pi seam retains the previous side until |
| `0x82ba2e88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_board_adjust.rs:14` | //82BA2E88: remove both outputs and reset both instance fields. |
| `0x82ba2eb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:445` | //82BA2EB8: remove both authored names; End does not reset state. |
| `0x82ba35e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:102` | //82BA35E0/3790 read Globals296;8289F8F0 binds hash |
| `0x82ba39a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:52` | /// update removes that pending key and resets it,82BA39A8. |
| `0x82ba3a78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/condition_nodes.rs:6` | ///Original82BA3A78: first cached record, sequence filter, then numeric scalar |
| `0x82ba3a78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:262` | //HasAnimAttribute82BA3A78 reads the preceding cached animation |
| `0x82ba3ed0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:59` | // Ctor82BA3ED0 examines only the first byte, default "0". |
| `0x82ba3fa8` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:87` | //82BA3FA8 uses relative X/Z but WORLD Y. The named Skate2 |
| `0x82ba41f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/conditions.rs:46` | Self::DroppingIn => p.dropping_in_324,          //82BA41F0 |
| `0x82ba4268` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/conditions.rs:47` | Self::DropInWithoutTransition => p.drop_in_without_transition_325, //82BA4268 |
| `0x82ba42e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:55` | //82BA42E0 reads Collision3472 OR3475, excluding wheel contacts. |
| `0x82ba4390` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:206` | Self::PhysicsWiping => p.physics_wiping,     //82BA4390:State59 |
| `0x82ba4400` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:208` | Self::WantsWipeout => p.wants_wipeout,       //82BA4400:State63 \|\| State65 |
| `0x82ba44b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:205` | Self::WantsRunout => p.wants_runout,         //82BA44B8:State78 |
| `0x82ba4528` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:46` | /// OBTrajTime82BA4528: PhysOutOffBoard+120, gated by byte331. |
| `0x82ba45d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/condition_nodes.rs:21` | ///Original82BA45D0/82BA4658 rejects an inactive containing state's time. |
| `0x82ba45d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:163` | }, //82BA45D0 |
| `0x82ba4658` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/condition_nodes.rs:21` | ///Original82BA45D0/82BA4658 rejects an inactive containing state's time. |
| `0x82ba4658` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:155` | //82BC3788/82BA4658 |
| `0x82ba4798` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:292` | //82BA4798 obtains the last state, then uses the same native |
| `0x82ba4d50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:77` | //82BA4D50 -> ISkaterMotionGraph256 ->8258FB60 ->82970628. |
| `0x82ba4dd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:110` | //82BA4DD0/82BA4F90 inline the same original inverse-sine polynomial as |
| `0x82ba4f90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:110` | //82BA4DD0/82BA4F90 inline the same original inverse-sine polynomial as |
| `0x82ba5150` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:111` | //DisablePushBrake82BA5150; no clamp or absolute value precedes asin. |
| `0x82ba5150` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:46` | //82BA5150 global316/layout1844;8289FB24 binds pushing. |
| `0x82ba5368` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/dismount.rs:1` | //! DisableDismount condition82BA5368 in the original Skate3 TU3 executable. |
| `0x82ba54a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:182` | //82BA54A0 compares these authored strings in this order. |
| `0x82ba55a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:217` | //82BA55A8:Air324,State16 |
| `0x82ba56b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_conditions.rs:77` | //82BA56B8 uses specific MotionGraph virtual100=8258F8D8 bit27. |
| `0x82ba5700` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:210` | Self::GrabbingObject => p.grabbing_object, //82BA5700:Offboard304 |
| `0x82ba5770` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:32` | /// OBTimeToLand82BA5770: PhysOutOffBoard+32. |
| `0x82ba58e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:61` | //Retail ctor82BA58E8 retains only diagnostic text/layout. All three |
| `0x82ba59d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:128` | //82BA59D8/82BA6AC0: missing intent is false even for NotEqual. |
| `0x82ba5aa8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:115` | /// PhysOutAnimation158/157, used directly by IsRidingGoofy82BA5AA8. |
| `0x82ba5c78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:164` | //ApexReached82BA5C78 reads Reckoning16.Y, not Air436. |
| `0x82ba5d30` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:51` | ///TU3 CanLandOnBoard82BA5D30 and LandOnBoard82BB95D8. |
| `0x82ba5da0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:204` | Self::HippyHurdling => p.hippy_hurdling,     //82BA5DA0:Offboard317 |
| `0x82ba5ed8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:51` | //82BA5ED8 reads byte312 independently of held311. |
| `0x82ba5ef0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:202` | Self::RetrievingBoard => p.retrieving_board, //82BA5EF0:Offboard323 |
| `0x82ba5f60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:263` | //82BA5F60: Offboard322 or the actual RetrieveBoard channel. |
| `0x82ba6038` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:96` | /// ActivationFunc 0x82ba6038: the pushing record's movement (PhysOut+0x48: x +4, z +0) is |
| `0x82ba6258` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:369` | //82BA6258: current tree time >= out factor * length, and |
| `0x82ba6378` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:390` | //82BA6378 queries the FIRST cached tree record via82D19010. |
| `0x82ba65a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:139` | "ExpireInTime" => Self::ExpireInTime(numeric()), //82BA65A0 |
| `0x82ba6620` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:141` | //82BA6620/82BA6708 |
| `0x82ba6708` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:141` | //82BA6620/82BA6708 |
| `0x82ba6808` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:150` | //82BC4558/82BA6808 |
| `0x82ba6a48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:164` | "IsProSkater" => Self::ProSkater(encode(a.text("skater").unwrap_or("").as_bytes())), //82BC56C0/82BA6A48 |
| `0x82ba6ac0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:128` | //82BA59D8/82BA6AC0: missing intent is false even for NotEqual. |
| `0x82ba6b90` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:214` | // TU3 82BA6B90 uses the filtered MG map (virtual +12), |
| `0x82ba6fe0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:292` | //82BA6FE0 rejects RevertGround102, reverses the side for fakie, |
| `0x82ba7130` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:166` | //Factory82BC54B8 and Activation82BA7130. |
| `0x82ba71e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:207` | Self::BodyFlipping => p.body_flipping,       //82BA71E0:Air441 |
| `0x82ba7250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:65` | /// TimeToLand82BA7250: valid Air trajectory remaining time. |
| `0x82ba7250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:29` | /// TimeToLand82BA7250: PhysOutAir+184, gated by byte437. |
| `0x82ba7310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:209` | Self::Bumped => p.bumped, //82BA7310: published acceleration and anim_motion/bumps |
| `0x82ba74d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing.rs:72` | //82BC5860 ->8231F460+48 ->82BA74D8 ->specific v116. |
| `0x82ba7598` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:3` | //! (factory 0x82bc6398, Activation 0x82ba7598) over SkaterAnim's 0x28-byte |
| `0x82ba7760` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:211` | // 82BA7760/82BA7848 compares the authored type through |
| `0x82ba7848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:211` | // 82BA7760/82BA7848 compares the authored type through |
| `0x82ba78b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:224` | // Native 82BA78B0: strictly positive retained manual-out timer. |
| `0x82ba7930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:66` | //82BA7930 reads completed Animation168. |
| `0x82ba79a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:265` | //82BA79A0 calls the specific MotionGraph getter8258FB68. |
| `0x82ba79e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:83` | ///82BA79E8 compares case-sensitive locostate values. An unknown spelling |
| `0x82ba7b50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:1` | //! BipedCadence82BB9E78, MatchCadence82BBA938/82BBAA00 and LocoState82BA7B50. |
| `0x82ba7b50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:95` | ///82BA7B50 requires the physical component and compares OffBoard84 exactly. |
| `0x82ba7bd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_ground_slope.rs:1` | //! GroundSlopeType: TU3 ctor82BA7BD8 and condition82BA7E80. |
| `0x82ba7e80` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_ground_slope.rs:1` | //! GroundSlopeType: TU3 ctor82BA7BD8 and condition82BA7E80. |
| `0x82ba7e80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:50` | // GroundSlopeType82BA7E80 and IsBipedGroundThin82BA8110 read the |
| `0x82ba7fa4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/stock_clip_query.rs:6` | //82BA7FA4/7FDC: missing bank/clip/attribute leaves the scalar zero. |
| `0x82ba7fd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:42` | ///EnoughDistToObstacle82BA7FD0: completed OffBoard+112, not distance116. |
| `0x82ba7ff0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:73` | //82BA7FF0 fadds;7FF4 fcmpu;7FF8 blt. Preserve unordered behavior. |
| `0x82ba8030` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:203` | Self::InBipedAir => p.in_biped_air,          //82BA8030:Offboard328 |
| `0x82ba80a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:40` | ///IsBipedCommittedToMotion82BA80A0: completed OffBoard byte329. |
| `0x82ba80fc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:53` | //82BA80FC reads completed OffBoard329. |
| `0x82ba8110` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:50` | // GroundSlopeType82BA7E80 and IsBipedGroundThin82BA8110 read the |
| `0x82ba8180` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:49` | //82BA8180: no Air437/time-valid or moving-object gate. |
| `0x82ba8214` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:38` | /// CanBipedLand82BA8214 tests OffBoard+192.Y strictly above0.85. |
| `0x82ba8238` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:58` | //82BA8238 compares completed OffBoard116. |
| `0x82ba82a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:44` | ///DistToEdge82BA82A8: completed OffBoard+116, not obstacle scalar112. |
| `0x82ba82c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:84` | //82BA82C0: exact authored names; invalid strings leave native storage undefined. |
| `0x82ba8468` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:110` | Self::GestureType(kind) => p.profile_148 == kind, //82BA8468. |
| `0x82ba84f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:77` | /// IsArmChannelPlaying (Activation 82BA84F0): a GestureBoth, GestureLeft, GestureRight or |
| `0x82ba8fd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:1` | //! Original ToggleBoard lifecycle adapter, TU3 82BA8FD8/8FE8 and 82B61BB8. |
| `0x82ba8fd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_board.rs:1` | //! TU3 ToggleBoard82BA8FD8/82BA8FE8, retrieval82BA9310/82BA9E20, |
| `0x82ba8fe8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_board.rs:1` | //! TU3 ToggleBoard82BA8FD8/82BA8FE8, retrieval82BA9310/82BA9E20, |
| `0x82ba9060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_execute.rs:11` | //82BA9060/9080 reads Actor+20 virtual16, not raw AG input. |
| `0x82ba9310` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_board.rs:1` | //! TU3 ToggleBoard82BA8FD8/82BA8FE8, retrieval82BA9310/82BA9E20, |
| `0x82ba9434` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:125` | //82BA9434/9628: actor1800 interface slot16 ->8258F6E0 appends |
| `0x82ba9464` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:129` | //The second call (82BA9464/9658) separately sets the animation parameter. |
| `0x82ba968c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:66` | //ToggleBoard82BA968C consumes returning313, not retrieval323. |
| `0x82ba9990` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_board.rs:2` | //! drop82BA9990. Instance and channel clocks remain owned by the graph. |
| `0x82ba9e20` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_board.rs:1` | //! TU3 ToggleBoard82BA8FD8/82BA8FE8, retrieval82BA9310/82BA9E20, |
| `0x82baa320` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:1` | //! Original CharacterGesture82BAA320, instance82BAADD0 and helpers |
| `0x82baa320` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:67` | /// EndGesture82BAA320's explicit teardown path. The native operation ends |
| `0x82baa320` | TU3 | `crates/skate2-core/src/animation/runtime/skate3_gesture_playback.rs:1` | //! Skate 3 CharacterGesture82BAA320 on the existing Skate 2 Actor's ACS channels. |
| `0x82baa848` | TU3 | `crates/skate2-core/src/physics/skate3_player.rs:867` | /// Skate 3 gestures in the skater's own channel stack (CharacterGesture 82BAA848), under a |
| `0x82baa954` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:130` | //82BAA954..AA4C ends the other two channels, in source order. |
| `0x82baadd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:1` | //! Original CharacterGesture82BAA320, instance82BAADD0 and helpers |
| `0x82baae70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:57` | ///82BAAE70/7C/88/8C: no channel is owned until a real gesture starts. |
| `0x82baaed8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:2` | //!82BAAFB0/82BAB080/82BAB258/82BAAED8. Allocation and channel names are owned |
| `0x82baaed8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:201` | //82BAAED8 -> channel virtual20=82D1D030 chooses embedded Sequence, |
| `0x82baafb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:2` | //!82BAAFB0/82BAB080/82BAB258/82BAAED8. Allocation and channel names are owned |
| `0x82bab080` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:2` | //!82BAAFB0/82BAB080/82BAB258/82BAAED8. Allocation and channel names are owned |
| `0x82bab258` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:2` | //!82BAAFB0/82BAB080/82BAB258/82BAAED8. Allocation and channel names are owned |
| `0x82bab258` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:260` | ///82BAB258: suppress occupied special channels/mount actions, then choose |
| `0x82bab258` | TU3? | `crates/skate2-core/src/animation/runtime/skate3_gesture_playback.rs:99` | // This is the same blocked-channel/mount gate as native82BAB258. |
| `0x82bab4d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:2` | //! HandBusy: factory82BC8B08, Begin82BAB4D0, End82BAB588. |
| `0x82bab588` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:2` | //! HandBusy: factory82BC8B08, Begin82BAB4D0, End82BAB588. |
| `0x82bab640` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_spin.rs:1` | //! BodySpin82BAB640, including its native five-state channel controller. |
| `0x82babc68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_spin.rs:280` | //82BABC68..BD9C: first clamp acceleration, then change, then [0,1]. |
| `0x82babe20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_spin.rs:107` | ///82BABE20 initializes every represented scalar and the state enum to zero. |
| `0x82babfd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:410` | // TU3 82BABFD0/82BABFFC read the same slot+8 map |
| `0x82babfd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_execute.rs:55` | // TU3 82BABFD0/82BABFFC read the same slot+8 map |
| `0x82babffc` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:410` | // TU3 82BABFD0/82BABFFC read the same slot+8 map |
| `0x82babffc` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_execute.rs:55` | // TU3 82BABFD0/82BABFFC read the same slot+8 map |
| `0x82bac778` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:220` | ///FakieHeadChannel82BAC778, instance82BACA30; both instance fields seed0. |
| `0x82baca30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:220` | ///FakieHeadChannel82BAC778, instance82BACA30; both instance fields seed0. |
| `0x82bacb40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:26` | //82BACB40/82BACB90: v80 ->8258F870, CA4 bit30. |
| `0x82bacb90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:26` | //82BACB40/82BACB90: v80 ->8258F870, CA4 bit30. |
| `0x82bacbe0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:32` | //82BACBE0/82BACC30: v88 ->8258F898, CA4 bit29. |
| `0x82bacc30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:32` | //82BACBE0/82BACC30: v88 ->8258F898, CA4 bit29. |
| `0x82bacc80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:38` | //82BACC80/82BACCD0: v96 ->8258F8C0, CA4 bit28. |
| `0x82baccd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:38` | //82BACC80/82BACCD0: v96 ->8258F8C0, CA4 bit28. |
| `0x82bacd20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:115` | //Begin82BACD20/End82BACD70 -> specific setter8258F910. |
| `0x82bacd70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:115` | //Begin82BACD20/End82BACD70 -> specific setter8258F910. |
| `0x82bacdc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:183` | ///JumpInto Update82BACDC0: apply pending parameters, then query the |
| `0x82bacdc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:445` | //8231FBDC Begin/End are empty. Update82BACDC0 consumes |
| `0x82bacef0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:131` | PushOperation::SetPushCoefs { .. } => self.first_update = true, //82BACEF0 |
| `0x82bacf00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:198` | self.first_update = false; //82BACF00 |
| `0x82bacf50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:192` | ); //82BACF50 |
| `0x82bad258` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:173` | .target(context.forward_speed, context.shared.current_push_dv); //82BAD258/82BAD260 |
| `0x82bad260` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:173` | .target(context.forward_speed, context.shared.current_push_dv); //82BAD258/82BAD260 |
| `0x82bad5b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:124` | PushOperation::InitPush => context.shared.initialize_push(), //82BAD5B8 |
| `0x82bad6e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:129` | )); //82BAD6E8 |
| `0x82bada90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:227` | ); //82BADA90 |
| `0x82badbb8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing_settings.rs:141` | /// TU382BAD3B4..3DC/82BADBB8..BE0; GetIsSwitch, not natural stance. |
| `0x82bae210` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:141` | //82BAE210 ->82595B28; output writes are begin-only and ordered X,Y. |
| `0x82bae51c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:62` | //82BAE51C..560: IAnimatable84 applies pending parameters through |
| `0x82bae848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:220` | } //82BAE848/8258F940 |
| `0x82bae8d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:172` | //82BAE8D0 is both Begin and End; v136=8258F930 clears3224. |
| `0x82bae918` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:69` | //82BAE918: the always branch returns before reading set and |
| `0x82baec90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:157` | //82BAEC90 emits the graph record before setting tree data. |
| `0x82baee90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:102` | // 82BAEE90: graph intents override animation only on the authored paths. |
| `0x82baf100` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:49` | // Begin82BAF100 copies completed Skeleton536 then540. |
| `0x82baf450` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_leg.rs:1` | //! Stock ControlAirLegExtension configuration82BAF450 and settings Globals336. |
| `0x82bafff0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:7` | /// PhysOutSystemReckoning16 and96, original StoreLandingData82BAFFF0. |
| `0x82bb0180` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:51` | //82BB0180: sign follows ISkaterAnim v28, then MG v36. |
| `0x82bb0338` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:58` | 1 => set(animation, b"disttocog", state.value), //82BB0338. |
| `0x82bb0398` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:82` | //82BB0398 draws from the SAME specific MotionGraph RNG v256. |
| `0x82bb0498` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:103` | _ => {} //Update82B61BB8 is empty; End82BB0498 erases the key. |
| `0x82bb04e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:175` | //82BB04E8: source literal0.65 only when explicitly authored. |
| `0x82bb0660` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_bump.rs:1` | //! SetBumpCoefficients Begin82BB0660, original Skate 3 TU3. |
| `0x82bb0660` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_bump.rs:22` | /// Begin82BB0660 uses the retained Xenon arithmetic implementation. |
| `0x82bb0948` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:254` | //82BB0948 sets a tree parameter; it does not call SetSpeed |
| `0x82bb0a30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/animation.rs:1` | //! Begin82BB0A30: live set/apply/query, including partial output on a miss. |
| `0x82bb12c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_intent_filter.rs:13` | /// TU3 82BB12C8. Presence flags remain distinct from numeric defaults. |
| `0x82bb17d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_filter_execute.rs:21` | //82BB17D0 reads live ISkaterAnim v12/v28; earlier graph |
| `0x82bb19d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:411` | // written by FilterMotionGraphIntent at 82BB19D0. |
| `0x82bb19d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_execute.rs:56` | // written by FilterMotionGraphIntent at 82BB19D0. |
| `0x82bb1ee0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_subject.rs:20` | /// Physical output bundle2+184, read directly by82BB1EE0. |
| `0x82bb2670` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:214` | //82BB2670/26D0 -> ISkaterAnim20 ->82B97110, bit28. |
| `0x82bb2868` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:126` | ///82BB2868 writes ProcessedPhysIn+16420 only on behavior Begin. |
| `0x82bb2868` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:99` | //ForcePhysics Begin82BB2868 writes the live Skeleton16420 mode once. |
| `0x82bb28e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:1` | //! PowerSliding Update82BB28E8, instance82BB2D70 and helper82595930. |
| `0x82bb2d70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:1` | //! PowerSliding Update82BB28E8, instance82BB2D70 and helper82595930. |
| `0x82bb2f60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:176` | /// CreateSlide Update82BB2F60, ordered packet values (Slide, Turn). |
| `0x82bb3198` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_finger_flip.rs:1` | //! FingerFlipOut: TU3 Begin82BB3198, Update82BB7C40, empty End82B61BB8. |
| `0x82bb3198` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:81` | //Begin82BB3198; same zero in native instance constructor. |
| `0x82bb31b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:12` | /// Complete PhysOutGround0/16/32/48 from Reckoning752, for82BB31B0. |
| `0x82bb31b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:217` | /// PowerSlideDecel82BB31B0: inverse Ground basis rotates velocity without |
| `0x82bb33c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:248` | /// PowerSlideSpin Begin82BB33C8; the input is GetSlideDirection82595930. |
| `0x82bb34d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:69` | //82BB34D8 uses Manual830BE900; only a negative present value emits. |
| `0x82bb36d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:211` | // Ctor82BB36D8; layout88 is holding and108 is speed, but |
| `0x82bb3830` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/turning_settings.rs:1` | //! SetTurning82BB3830/TurnRemap82BB4248 stock layout binding. |
| `0x82bb4248` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/turning_settings.rs:1` | //! SetTurning82BB3830/TurnRemap82BB4248 stock layout binding. |
| `0x82bb54d4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:62` | ///both PlayAnimation82BB54D4 and Phase1 consume the byte without inversion. |
| `0x82bb56b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:241` | //82BB56B8: all unrecognized from strings take filteredIntent. |
| `0x82bb5930` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_instances.rs:36` | ///82BB5930 initializes byte8 once per allocated behavior instance. |
| `0x82bb5db0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_execute.rs:17` | //End82BB5DB0 -> IAnimatable112 (8231E050+112) ->82531278: |
| `0x82bb63d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:345` | // InitMovingObjects::Begin 0x82bb63d8; no Update or End. |
| `0x82bb63d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:2` | //! 0x82bb79d8), InitMovingObjects::Begin 0x82bb63d8 and AngleAdjustByAnimationEndLessThan |
| `0x82bb64b8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:1` | //! TU3 MovingObject (ctor 0x82bb64b8, Begin 0x82bb6860, Update 0x82bb6a20, ClampAngle |
| `0x82bb6860` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:1` | //! TU3 MovingObject (ctor 0x82bb64b8, Begin 0x82bb6860, Update 0x82bb6a20, ClampAngle |
| `0x82bb6a20` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:1` | //! TU3 MovingObject (ctor 0x82bb64b8, Begin 0x82bb6860, Update 0x82bb6a20, ClampAngle |
| `0x82bb79d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:2` | //! 0x82bb79d8), InitMovingObjects::Begin 0x82bb63d8 and AngleAdjustByAnimationEndLessThan |
| `0x82bb7c40` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_finger_flip.rs:1` | //! FingerFlipOut: TU3 Begin82BB3198, Update82BB7C40, empty End82B61BB8. |
| `0x82bb8dc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:63` | //82BB8DC8. |
| `0x82bb8e58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:69` | //82BB8E58; strict comparison, no reference counting. |
| `0x82bb8f20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:78` | _ => flags.tricks_allowed = true, //82BB8F20 unconditionally. |
| `0x82bb8f70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:17` | ///82BB8F70 guarantees AG-intents -> MG-output/intents -> SkaterAnim order. |
| `0x82bb9058` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:204` | //82BB9058: actual filtered category5 resets; all others add |
| `0x82bb9158` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:232` | //Vtable8232090C: Begin/Update empty; End82BB9158 sets length. |
| `0x82bb91a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:229` | } //82BB91A8/8258F998 |
| `0x82bb9260` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:225` | //82BB9260/8258F980 |
| `0x82bb93e0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_dismounting.rs:1` | //! TU3 AirDismounting Begin82BB93E0, Update82BB94C0, empty End82B61BB8. |
| `0x82bb94c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_air_dismounting.rs:1` | //! TU3 AirDismounting Begin82BB93E0, Update82BB94C0, empty End82B61BB8. |
| `0x82bb95d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:361` | //82BB95D8 -> FlipBoardBackwards82B97150. End/Update are empty. |
| `0x82bb95d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:51` | ///TU3 CanLandOnBoard82BA5D30 and LandOnBoard82BB95D8. |
| `0x82bb9bd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:33` | // TU3 82BB9BD0: zero is no grab; authored types are one through four. |
| `0x82bb9e78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:1` | //! BipedCadence82BB9E78, MatchCadence82BBA938/82BBAA00 and LocoState82BA7B50. |
| `0x82bb9e78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:30` | ///82BB9E78 calls SkaterAnim virtual40=82531200, a direct phase14652 store. |
| `0x82bb9f48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/air.rs:1` | //! Offboard animation lifecycle:82BB9F48/A070 and82BBACC0/ACE0/B418. |
| `0x82bb9f48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:1` | //! Native MatchAirTime: factory82BCA308, Begin82BB9F48, |
| `0x82bba070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:2` | //! Update82BBA070, instance82BBA390. End is the empty82B61BB8. |
| `0x82bba16c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:47` | //82BBA16C skips the first update;184 is ble (unordered falls through). |
| `0x82bba198` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:173` | /// MatchAirTime82BBA198..1CC: current tree v20 length, v28 SetTime. |
| `0x82bba200` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:36` | /// MatchAirTime82BBA200 consumes the completed OffBoard+96 vector. |
| `0x82bba218` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:56` | //82BBA218 vmsum3, two rsqrt refinements, zero selection. Independent |
| `0x82bba2a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:67` | //82BBA2A0 scales all four lanes. Published scalar parameters are XYZ. |
| `0x82bba2c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:38` | //82BBA2C0..378: r6=0, r7=-1 for each v80 SetAttribute. |
| `0x82bba390` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:2` | //! Update82BBA070, instance82BBA390. End is the empty82B61BB8. |
| `0x82bba4a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/runout.rs:1` | //! AddRunoutAttribs: Begin82BBA4A8, Update82BBA770, state82BBA830. |
| `0x82bba4a8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_runout.rs:2` | //! Begin82BBA4A8, Update82BBA770, End82B61BB8. |
| `0x82bba620` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/runout.rs:27` | ///Exact first vector of physical bundle20, loaded at82BBA620. |
| `0x82bba770` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/runout.rs:1` | //! AddRunoutAttribs: Begin82BBA4A8, Update82BBA770, state82BBA830. |
| `0x82bba770` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_runout.rs:2` | //! Begin82BBA4A8, Update82BBA770, End82B61BB8. |
| `0x82bba830` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/runout.rs:1` | //! AddRunoutAttribs: Begin82BBA4A8, Update82BBA770, state82BBA830. |
| `0x82bba938` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:1` | //! BipedCadence82BB9E78, MatchCadence82BBA938/82BBAA00 and LocoState82BA7B50. |
| `0x82bba938` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:53` | ///82BBA938 requires both physical and animation components. Missing either |
| `0x82bbaa00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:1` | //! BipedCadence82BB9E78, MatchCadence82BBA938/82BBAA00 and LocoState82BA7B50. |
| `0x82bbaa00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:61` | ///82BBAA00 publishes on every Update, even if byte12 is already false. |
| `0x82bbaa98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:111` | ///CreateInstance82BBAA98 seeds velocity0 and prior-air false. |
| `0x82bbaa98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence.rs:45` | /// Per-node state from82BBAA98: value8=0 and byte12=false. This is a captured |
| `0x82bbab98` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:1` | //! TU3 OffboardBodyTweakBlend: ctor82BBAB98, Begin82BBACC0, |
| `0x82bbacc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/air.rs:1` | //! Offboard animation lifecycle:82BB9F48/A070 and82BBACC0/ACE0/B418. |
| `0x82bbacc0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:1` | //! TU3 OffboardBodyTweakBlend: ctor82BBAB98, Begin82BBACC0, |
| `0x82bbacc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:50` | // Raw82BBACC0: zero8,20,21,22. Axes12/16 are only zeroed at allocation. |
| `0x82bbace0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:2` | //! Update82BBACE0, End82BBB418, instance82BBB4D0. Original IDA only. |
| `0x82bbaed8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:111` | //82BBAED8..EE8 uses signed comparisons, not abs(axis). |
| `0x82bbafc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:34` | /// OffboardBodyTweakBlend82BBAFC8 reads completed OffBoard+92. |
| `0x82bbafd4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:128` | //82BBAFD4 is ble. This preserves its unordered path as well. |
| `0x82bbb030` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:69` | //82BBB030 is blt: unordered does not take the near-landing branch. |
| `0x82bbb164` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:163` | //82BBB164/270 call IChannelAnimatablev16=82D1D090. Stack57=false |
| `0x82bbb290` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:133` | //Raw82BBB290..2C8: preserve fmuls/fmadds order and axis identity. |
| `0x82bbb328` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:87` | //82BBB328 ->MGv200/8258FBF0;82BBB33C ->ISkaterAnimv116/82B97228. |
| `0x82bbb33c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:87` | //82BBB328 ->MGv200/8258FBF0;82BBB33C ->ISkaterAnimv116/82B97228. |
| `0x82bbb36c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:91` | //82BBB36C ->ISkaterAnimv112/82B97218: one-frame request bit16. |
| `0x82bbb418` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:2` | //! Update82BBACE0, End82BBB418, instance82BBB4D0. Original IDA only. |
| `0x82bbb418` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:142` | //82BBB418 ends the channel from its last frame, but does not clear MGbit20. |
| `0x82bbb4d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:2` | //! Update82BBACE0, End82BBB418, instance82BBB4D0. Original IDA only. |
| `0x82bbb848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hooks.rs:12` | ///82BBB848, factory82BCA6A8 default right=true. |
| `0x82bbbb68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hooks.rs:7` | ///82BBBB68 copies the authored transition into the next-play override. |
| `0x82bbbbb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion.rs:390` | //82BBBBB8 passes the authored name for both channel/tree. |
| `0x82bbbc88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:211` | Self::Skitching => p.state == 104, //82BBBC88:State16 |
| `0x82bbbd00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:90` | /// IsSkitchingWithAbsorb 0x82BBBD00. |
| `0x82bbbd00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:4` | //! conditions IsSkitchingWithAbsorb 0x82BBBD00, SkitchingPosition 0x82BBC068 and |
| `0x82bbbde0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:213` | //82BBBDE0:Ground276,Globals400/layout96 |
| `0x82bbbde0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:40` | //82BBBDE0: Globals400/layout96, bound by828A0154. |
| `0x82bbbe90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:112` | //82BBBE90 returns before the vector read when not submerged. |
| `0x82bbbf90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:92` | /// SkitchingPosition 0x82BBC068, ctor 0x82BBBF90: "side" front 1, back 2, both 3. |
| `0x82bbc068` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:92` | /// SkitchingPosition 0x82BBC068, ctor 0x82BBBF90: "side" front 1, back 2, both 3. |
| `0x82bbc068` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:4` | //! conditions IsSkitchingWithAbsorb 0x82BBBD00, SkitchingPosition 0x82BBC068 and |
| `0x82bbc1a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:94` | /// IsSkitchShimmying 0x82BBC1A0, ClassHelper 0x82BCACD8: "direction" right 2, left 1. |
| `0x82bbc1a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:5` | //! IsSkitchShimmying 0x82BBC1A0. All read the anim_skitching default collection. |
| `0x82bbc2d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:2` | //! Update 0x82BBC5E0), SkitchingBehaviour (Begin 0x82BBC2D8, Update 0x82BBC390), |
| `0x82bbc390` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:2` | //! Update 0x82BBC5E0), SkitchingBehaviour (Begin 0x82BBC2D8, Update 0x82BBC390), |
| `0x82bbc538` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:1` | //! The TU3 skitching leaves of the MotionGraph: EnterSkitchingBehaviour (Begin 0x82BBC538, |
| `0x82bbc5e0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:2` | //! Update 0x82BBC5E0), SkitchingBehaviour (Begin 0x82BBC2D8, Update 0x82BBC390), |
| `0x82bbc734` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:139` | // 0x82BBC734 bge: an unordered or non-negative previous blend limits the step. |
| `0x82bbc7c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:3` | //! SkitchShimmyingBehaviour (Begin 0x82BBC7C8, Update 0x82BBC858, End 0x82BBCAB8) and the |
| `0x82bbc858` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:3` | //! SkitchShimmyingBehaviour (Begin 0x82BBC7C8, Update 0x82BBC858, End 0x82BBCAB8) and the |
| `0x82bbc910` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:195` | // 0x82BBC910..930 clamps against the current speed, not the retained one. |
| `0x82bbcab8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_skitching.rs:3` | //! SkitchShimmyingBehaviour (Begin 0x82BBC7C8, Update 0x82BBC858, End 0x82BBCAB8) and the |
| `0x82bbcb50` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:5` | //! elapsed-time record in `0x82BBCB50` and dispatches Update `0x82BBE900`. |
| `0x82bbcc58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:135` | //82BBCC58; virtual40 uses EndChannelWithTime, not instantaneous removal. |
| `0x82bbccf0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:1` | //! Original TU3 Shove constructor82BBCCF0, Begin82BBCDD0, |
| `0x82bbcdd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:1` | //! Original TU3 Shove constructor82BBCCF0, Begin82BBCDD0, |
| `0x82bbcdd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:66` | //82BBCDD0 clears both fields; this does not terminate any channel. |
| `0x82bbcdf0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:2` | //! Update82BBCDF0 and End82BBD540. The host owns channels and graph values. |
| `0x82bbcdf0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:171` | // Update82BBCDF0 never writes the settings' use-attributes byte (var_D8+1), so it is |
| `0x82bbd250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:189` | ///82BBD250..D390: original one-refinement reciprocal followed by82473B98. |
| `0x82bbd540` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:2` | //! Update82BBCDF0 and End82BBD540. The host owns channels and graph values. |
| `0x82bbd690` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:3` | //! MaintainShove: factory82BCB1C8, Update82BBD690, End82BBD6E8. |
| `0x82bbd6e8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:3` | //! MaintainShove: factory82BCB1C8, Update82BBD690, End82BBD6E8. |
| `0x82bbd6e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:61` | // End82BBD6E8 first clears the retention flag, then requests |
| `0x82bbd7f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:234` | //82BBD7F0, literals820641A8 and8209975C. |
| `0x82bbd888` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:231` | // Factory82BCB308 -> VT82321070 slot48=82BBD888. |
| `0x82bbda10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/conditions.rs:44` | Self::TrickOutType { value } => p.trick_out_240 == value, //82BBDA10 |
| `0x82bbda98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:238` | //82BBDA98 subtracts one simulation tick before the comparison. |
| `0x82bbdb98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:236` | Self::NewHandplantPosition => p.handplant_flags & 0x4000_0000 != 0, //82BBDB98 |
| `0x82bbdc08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/conditions.rs:40` | Self::BluntingBackslash => p.blunting_136 == 4, //82BBDC08 |
| `0x82bbdc80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/conditions.rs:43` | } //82BBDC80 |
| `0x82bbdd70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/conditions.rs:45` | Self::LandingIntoGrind => p.air_grind_443 && p.air_time_184 < 0.2, //82BBDD70 |
| `0x82bbde40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions.rs:63` | //82BBDE40 compares PhysOutAnimation72 with literal821EE79C. |
| `0x82bbdff8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:44` | //82BBDFF8/82BBE048: v104 ->8258F8E8, CA4 bit27. |
| `0x82bbe048` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing_execute.rs:44` | //82BBDFF8/82BBE048: v104 ->8258F8E8, CA4 bit27. |
| `0x82bbe4e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:88` | //82BBE4E0 reads the manager's curve duration, Air212. |
| `0x82bbe5d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:70` | //VT82321360 Begin=82BBE5D0; shared clip lengths from82B959E8. |
| `0x82bbe900` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:5` | //! elapsed-time record in `0x82BBCB50` and dispatches Update `0x82BBE900`. |
| `0x82bbef60` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:389` | // TU3 82BBEF60 publishes a score name and vector. |
| `0x82bbef60` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:83` | /// ScoringGrabs 82BBEF60: selected authored name and tweak vector. |
| `0x82bbef60` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_execute.rs:34` | // TU3 82BBEF60 publishes a score name and vector. |
| `0x82bbef60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:82` | //82BBF758 uses the same angular windows as ScoringGrabs82BBEF60. |
| `0x82bbf180` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:38` | /// TU3 82BBF180..F2EC, constants at 820DDEA8..DEE4. Sector windows |
| `0x82bbf758` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:82` | //82BBF758 uses the same angular windows as ScoringGrabs82BBEF60. |
| `0x82bbfa88` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_scoring_trick.rs:1` | //! ScoringTrick lifecycle from original Skate 3 TU3 82BCC030/82BBFA88. |
| `0x82bbfa88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_scoring_trick.rs:24` | //82BBFA88 supplies two copies of the same name: no mirror selection. |
| `0x82bbfb28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:38` | //82BBFB28's source enum; absent mirrorAugment inherits augment. |
| `0x82bbfe98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_native.rs:59` | //Original82BBFE98 writes the regular slot for this name, |
| `0x82bbff88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout_execute.rs:1` | //! Original82BBFF88/82BC01C0; graph ownership is ours, filter order is stock. |
| `0x82bc01c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout_execute.rs:1` | //! Original82BBFF88/82BC01C0; graph ownership is ours, filter order is stock. |
| `0x82bc0358` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:42` | //82BC0358/051C and stock anim_wipeout schema offsets0..28. |
| `0x82bc0690` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:19` | /// Wipeout instance82BC0690: counter8, angles12/16, velocity24, axes28/32, latch36. |
| `0x82bc07a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout_execute.rs:25` | //82BC07A8/07F8, Update82B61BB8 is empty. |
| `0x82bc0ab8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:107` | Self::Done => p.over_599, //82BC0AB8: skeleton599. |
| `0x82bc0b28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:108` | Self::TimeToLand(n) => n.matches(p.collision_time_144), //82BC0B28. |
| `0x82bc0bb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:109` | Self::TimeSinceContact(n) => n.matches(p.no_support_time_548), //82BC0BB0. |
| `0x82bc0d90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_mapping.rs:7` | // 82BC0D90 uses greatest prime <=46, then maximum load10000. |
| `0x82bc1068` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_intent_filter.rs:58` | /// Native map removal 82BC1068 has no per-behavior ownership counter. |
| `0x82bc1b68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:35` | ///82C0E308 ->82BC1B68: clear actual ActionGraph intent map only. |
| `0x82bc1b68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:204` | //GenerateActionGraphIntents82594310 clears the AG map through82BC1B68 |
| `0x82bc1be8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_mapping.rs:8` | // 82BC1BE8 prepends entries; 82BA07F0 visits ascending buckets. |
| `0x82bc31b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:68` | // Factories82BC3258/82BC31B8 use NumericCondition82C126F0. |
| `0x82bc3258` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:68` | // Factories82BC3258/82BC31B8 use NumericCondition82C126F0. |
| `0x82bc3788` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:155` | //82BC3788/82BA4658 |
| `0x82bc3ce8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:56` | // Factory82BC3CE8 adds no attributes beyond the base condition. |
| `0x82bc3f68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_conditions.rs:82` | //Factory82BC3F68 installs8231ED5C; virtual48=8274CA90 is li r3,0;blr. |
| `0x82bc4558` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:150` | //82BC4558/82BA6808 |
| `0x82bc4d60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:176` | //ClassHelper 0x82bc4d60: degrees times 0.017453292. |
| `0x82bc53f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:144` | right: a.boolean_byte("right", 1) != 0, //82BC53F0 |
| `0x82bc54b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:166` | //Factory82BC54B8 and Activation82BA7130. |
| `0x82bc56c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:164` | "IsProSkater" => Self::ProSkater(encode(a.text("skater").unwrap_or("").as_bytes())), //82BC56C0/82BA6A48 |
| `0x82bc5860` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing.rs:72` | //82BC5860 ->8231F460+48 ->82BA74D8 ->specific v116. |
| `0x82bc62f8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/push_off.rs:1` | //! IsPushOffEnabled: original TU3 registration82F88150 -> ctor82BC62F8. |
| `0x82bc6398` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:84` | /// AngleAdjustByAnimationEndLessThan (factory 0x82bc6398: MaxDeltaAngle, default 2). |
| `0x82bc6398` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:3` | //! (factory 0x82bc6398, Activation 0x82ba7598) over SkaterAnim's 0x28-byte |
| `0x82bc6d40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:205` | // Factory82BC6D40 defaults both identifiers to the string0. |
| `0x82bc7218` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:19` | //82BC7218: both constructor defaults are literal8209975C=0.5. |
| `0x82bc7618` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing.rs:21` | //82BC7618 fixes this operation's attribute to disttocog. |
| `0x82bc7788` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_landing.rs:29` | //82BC7788 truncates the configured float toward zero. |
| `0x82bc7848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:2` | //! identify SetTrickAttr82BC7848 and ScoringTrick82BCC030. |
| `0x82bc7b20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_nodes.rs:119` | // Native constructor82BC7B20 retains authored FastStrings. |
| `0x82bc7c18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:159` | //Factory82BC7C18 binds authored grabintent and literal Grabbing. |
| `0x82bc7ed0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:21` | /// Defaults verified in TU3 factories82BC7ED0/7F98/80B8/8180/8248. |
| `0x82bc8b08` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:2` | //! HandBusy: factory82BC8B08, Begin82BAB4D0, End82BAB588. |
| `0x82bc9100` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding.rs:65` | // Factory82BC9100 explicitly defaults authored right to true. |
| `0x82bc9510` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:152` | // TU3 factory 82BC9510 binds these names directly. |
| `0x82bc9678` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:172` | // Factory82BC9678/84 uses literal8231D5C8, no authored override. |
| `0x82bc9ba8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_runout.rs:1` | //! TU3 AddRunoutAttribs: factory82BC9BA8, vtable82320BDC, |
| `0x82bca0fc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding.rs:108` | a.float_bits("length", 0x3e29_fbe7), //factory82BCA0FC |
| `0x82bca308` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time.rs:1` | //! Native MatchAirTime: factory82BCA308, Begin82BB9F48, |
| `0x82bca6a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hooks.rs:12` | ///82BBB848, factory82BCA6A8 default right=true. |
| `0x82bcaaf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_wipeout.rs:93` | //82BCAAF8: unrecognized orientation has no native initialized value. |
| `0x82bcacd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:94` | /// IsSkitchShimmying 0x82BBC1A0, ClassHelper 0x82BCACD8: "direction" right 2, left 1. |
| `0x82bcaf40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_gameplay.rs:99` | /// Ctor 0x82BCAF40: "direction" right is 2, left 1, otherwise 0. |
| `0x82bcb1c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services.rs:3` | //! MaintainShove: factory82BCB1C8, Update82BBD690, End82BBD6E8. |
| `0x82bcb308` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_gameplay_conditions.rs:231` | // Factory82BCB308 -> VT82321070 slot48=82BBD888. |
| `0x82bcbf60` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:4` | //! registers constructor `0x82BCBF60`; vtable `0x82321414` creates a private |
| `0x82bcc030` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_scoring_trick.rs:1` | //! ScoringTrick lifecycle from original Skate 3 TU3 82BCC030/82BBFA88. |
| `0x82bcc030` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_tricks.rs:2` | //! identify SetTrickAttr82BC7848 and ScoringTrick82BCC030. |
| `0x82bcc490` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/twist_lean.rs:16` | ///82BCC490 compares exact bytes through strcmp82AE89B0. |
| `0x82bd3150` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/rotation.rs:1` | //! CreateMtxFromHeadingAndUp82D61B60 and axis-angle frame blend82BD3150. |
| `0x82bd3720` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik.rs:56` | //82BD3720 walks parents until it finds a represented physical part; |
| `0x82bd3720` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body.rs:108` | /// SkeletonData::Init82BD6C40 / closest physical ancestor82BD3720. |
| `0x82bd41b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:139` | /// PhysicsUtility::ClampVectorAWithinRangeOfVectorB 0x82BD41B0. |
| `0x82bd41b0` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:453` | /// PhysicsUtility::ClampVectorAWithinRangeOfVectorB (0x82bd41b0): `a` when within `range` of |
| `0x82bd4a30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_collision_settings.rs:52` | /// resolves every layout offset consumed by82BD4A30 and82BD6038. |
| `0x82bd6038` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_collision_settings.rs:52` | /// resolves every layout offset consumed by82BD4A30 and82BD6038. |
| `0x82bd60c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:71` | // SkeletonCollision Fill82BD60C8 -> Collision+80/+196/+200/+208/+214/+217. |
| `0x82bd6c40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body.rs:108` | /// SkeletonData::Init82BD6C40 / closest physical ancestor82BD3720. |
| `0x82bd7760` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/state.rs:32` | //Skeleton ctor82BD7760..7884 initializes16016 to identity. |
| `0x82bd7a78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:93` | //82BD7A78/84/8C clear16504/16506/16508. Frames are native identity. |
| `0x82bd7dd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:60` | //82BD7DD0..7E88 names and stores these six ids; no authored index literals. |
| `0x82bd8280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:31` | // Both82BD8280 and82BE2538 read Skeleton16424 ->Reckoning1216. |
| `0x82bd8328` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:34` | //82BD8328..8334 tests the low bit of Processed+9B0 (2480). |
| `0x82bd83e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:120` | ///Call in82BD83E0's post-wipeout-check phase, before the pose job. |
| `0x82bd83e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:106` | ///Player postphysics82BD83E0 follows contact/error publication with the |
| `0x82bd8918` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:174` | fn process_skeleton_82bd8918( |
| `0x82bd8918` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:33` | /// Skeleton11008 -> Processed752 in82BD8918. |
| `0x82bd8918` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:1` | //! The production Skeleton::ProcessData82BD8918 boundary. All borrows point |
| `0x82bd8cb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animated_skeleton.rs:36` | ///Skeleton12560, copied BEFORE board offsets and IK by82BD8CB0..8D10. |
| `0x82bd8cb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:98` | ///translation copied by82BD8CB0..8D14, not animation_board after offsets. |
| `0x82bd8e04` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animated_skeleton.rs:181` | // ProcessData82BD8E04..8E10 writes local board11728 AFTER COM. |
| `0x82bd8f10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/pose_adjust.rs:1` | //! Original82BD8F10 writes the existing AnimatedSkeleton board-offset owner. |
| `0x82bd9728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/ik.rs:1` | //! HandPlantManager82D633B0 and Skeleton82BD9728/97D0, shared four-limb IK. |
| `0x82bd9728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/ik.rs:77` | //82BD9728 clamps against the authored reparented hand target. |
| `0x82bd9728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:754` | /// UpdateHandIK 0x82D4A378, then Skeleton::SetLeft/RightHandIKTargetPosition (0x82BD9728 and |
| `0x82bd9728` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:679` | // Skeleton::SetLeft/RightHandIKTargetPosition (0x82bd9728/0x82bd97d0): each hand's |
| `0x82bd97d0` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:679` | // Skeleton::SetLeft/RightHandIKTargetPosition (0x82bd9728/0x82bd97d0): each hand's |
| `0x82bd9990` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:252` | // Reset82BD9990 finishes by publishing both COM targets to the |
| `0x82bd9f3c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:114` | //82BD9F3C..9F70 transforms retained16144 into16208;9F90 uses the |
| `0x82bda024` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:115` | //current deck for16240. Only later82BDA024 replaces16144 with physical |
| `0x82bda0d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_input.rs:7` | //ProcessAnimAttributes82BDA0D0 already owns these Processed fields in the |
| `0x82bdc6c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/selection.rs:50` | // KickTurn attribute active: the animated deck that IsSkateboardAnimated82BDC6C8 tests, |
| `0x82bdc6c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:67` | //Skeleton+24 physics_animation, original82BDC6C8 layout800. |
| `0x82bdc9e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik.rs:25` | ///DataIn416 as UpdateIKData82BDC9E8 last copied it. UpdateWipeoutAdjust |
| `0x82bdcaf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_general.rs:86` | //GeneralUpdate82BDCAF8 skips UpdateIKData for702. |
| `0x82bdcfb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_grind_air.rs:1` | //!82BDCFB0 at the existing ProcessData boundary; never a physical teleport. |
| `0x82bdda10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/skeleton.rs:1` | //!82D40D28 calls82BDDA10 with fast_blend=false only for a pop THIS update. |
| `0x82bdda10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/skeleton.rs:1` | //!82D33E00 calls SkeletonUpdateAnimated82BDDA10 with fast blending enabled. |
| `0x82bdda10` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:2` | //! Original TU3 82BDDA10 / 82BDE600. All owners are the same objects used by |
| `0x82bdda10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:79` | ///82BDDA10. PhysicsAir passes fast=false; GroundAnimation passes true. |
| `0x82bddc20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:105` | //82BDDC20..54 clears only next trajectory, after GeneralUpdate. |
| `0x82bddd70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_air.rs:1` | //! BipedAir Skeleton82BDDD70 with the existing skeleton, drives and solver. |
| `0x82bdddc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_air.rs:47` | //82BDEE08 and82BDE310 use the OLD root, before82BDDDC8 changes it. |
| `0x82bddffc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:123` | //82BDDFFC..82BDE010 calls the SAME82D8E3E0 as Ground. |
| `0x82bddffc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:39` | ///Ground82BDE2CC and Air82BDDFFC both supply enable-body-spin r7=1. |
| `0x82bde010` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:123` | //82BDDFFC..82BDE010 calls the SAME82D8E3E0 as Ground. |
| `0x82bde060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:1` | //! Original Biped Skeleton update82BDE060 with the existing physical owners. |
| `0x82bde158` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/state.rs:12` | //82BDE158..17C copies mapped12624 here only when2484bit0 is clear. |
| `0x82bde2cc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:39` | ///Ground82BDE2CC and Air82BDDFFC both supply enable-body-spin r7=1. |
| `0x82bde2cc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/state.rs:43` | ///82BDE2CC explicitly passes r7=1: consume physical body spin2812. |
| `0x82bde310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_air.rs:47` | //82BDEE08 and82BDE310 use the OLD root, before82BDDDC8 changes it. |
| `0x82bde310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames.rs:19` | //82BDE310 also uses the old basis, with world-space state1056 COM. |
| `0x82bde310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:104` | //82BDF29C..2DC: Skeleton15872 plus the motion, then 82BDE310 with no lift. |
| `0x82bde398` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:233` | //82BDE398..3E0: COM frame15824 + translation48 = Skeleton15872. |
| `0x82bde428` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck/skeleton.rs:1` | //! Skeleton82BDE428: old-root COM, new root, shared drives, shared reckoning. |
| `0x82bde600` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:2` | //! Original TU3 82BDDA10 / 82BDE600. All owners are the same objects used by |
| `0x82bde600` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:111` | ///82BDE600. target_com is PhysicsAir's actually integrated COM trajectory |
| `0x82bdec48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/plant_skeleton.rs:1` | //! Skeleton82BDEC48 (anchored foot) and82BDF090 (plant COM). |
| `0x82bdee08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_air.rs:47` | //82BDEE08 and82BDE310 use the OLD root, before82BDDDC8 changes it. |
| `0x82bdee08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames.rs:17` | //82BDEE08 consumes animation record6464+16 using the OLD world root. |
| `0x82bdee08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:102` | //82BDEE08 consumes animation record6464+16 using the OLD world root. |
| `0x82bdf090` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/plant_skeleton.rs:1` | //! Skeleton82BDEC48 (anchored foot) and82BDF090 (plant COM). |
| `0x82bdf268` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:113` | /// target and turns the root by the align angle (Skeleton::UpdateOffBoardPushing 0x82bdf268) |
| `0x82bdf268` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:86` | /// Skeleton::UpdateOffBoardPushing 0x82BDF268, Pushing::Update's skeleton step in place of |
| `0x82bdf29c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:104` | //82BDF29C..2DC: Skeleton15872 plus the motion, then 82BDE310 with no lift. |
| `0x82bdf2f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:109` | //82BDF2F8..37C: the motion in animation space, a translation for the root update. |
| `0x82bdf3bc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:117` | //82BDF3BC..474: processed +0 and Skeleton16208 = the mapped pose under the new root. |
| `0x82bdf530` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:109` | //82BDF530 then82C04368: CURRENT wrapper also retains physics error. |
| `0x82bdf530` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:95` | //Unlike Ground82BDF530, this does NOT force initialize_heading=true. |
| `0x82bdf530` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_general.rs:18` | ///Ground82BDF530: update actual roots/board frames, GeneralUpdate, then |
| `0x82bdf550` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animated_skeleton.rs:228` | self.roots.initialize_heading = true; //82BDF550, on every Ground update. |
| `0x82bdf694` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:118` | //Ground82BDF694..6A4 stores this target into Processed0..48; |
| `0x82bdf778` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animated_skeleton.rs:238` | ///82BDF778..F7AC runs AFTER the current GeneralUpdate consumes trajectory. |
| `0x82bdf7c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_teleport.rs:1` | //! Actual teleport skeleton stages82BE3508 and82BDF7C0. The player owns the |
| `0x82bdf7c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_teleport.rs:49` | ///82DB8998 brackets82BDF7C0 with Skeleton16508=true/false after the real |
| `0x82bdfa88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:1` | //! Original Skeleton::UpdateWipeout82BDFA88 on the live physical owners. |
| `0x82bdfa88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:64` | //82BDFA88 ignores the target continuity result: no GeneralUpdate reset. |
| `0x82bdfac8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:50` | //82BDFAC8 skips the whole82BE1618 call. Its nested82BE1870 call to |
| `0x82bdfb84` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:92` | //82BDFB84 passes Skeleton+12624, not the solved physical pose at8016. |
| `0x82be090c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck/skeleton.rs:35` | //82BE090C..0950 resets only packet bone0; inverse trajectory12112 survives. |
| `0x82be0cc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:194` | /// Skeleton::ReorientSkaterUpright 0x82BE0CC0: a tenth of the way toward the upright frame with |
| `0x82be1058` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:154` | /// Skeleton::UpdateRootTransformsOffBoard 0x82BE1058 as Pushing calls it: no ground adjust and no |
| `0x82be1618` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:50` | //82BDFAC8 skips the whole82BE1618 call. Its nested82BE1870 call to |
| `0x82be1870` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:50` | //82BDFAC8 skips the whole82BE1618 call. Its nested82BE1870 call to |
| `0x82be18b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:51` | //82BE18B8 is skipped too: retain all four targets and extra-body velocities. |
| `0x82be19a8` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:168` | /// `Pan2DKernelBase::ComputeInteriorTerm` 0x82be19a8 (writes all five levels). |
| `0x82be1ae8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_runout.rs:8` | /// Skeleton0: effective animation root Z (Fill82BE1AE8 / GetEffectiveRoot). |
| `0x82be1b50` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:222` | /// `Pan2DKernelBase::ComputePerimeterTerm` 0x82be1b50 (adds to the levels). |
| `0x82be1d00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:57` | //82BE1D00..1E58: physical pose starts8016;9232=part19,8976=part15. |
| `0x82be1d84` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:473` | // Skeleton16144 -> PhysOutSystemReckoning64,82BE1D84/8C. |
| `0x82be1d98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik.rs:104` | ///Skeleton Fill82BE1D98..1E00 publishes each physical toe frame applied |
| `0x82be1da0` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:107` | /// `Pan2DKernel::EmitterConfig` 0x82be1da0. |
| `0x82be1e64` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:338` | //82BE1E64..1ED4 copies these actual physical volume positions. |
| `0x82be1e94` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:86` | // Skeleton Fill82BE1E94/98 copies hips physical up into output80. |
| `0x82be20f4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:156` | //82BE3650 -> PhysOutSkeleton0/432 at82BE20F4/2100. Its stance flag is |
| `0x82be2138` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_physical_output.rs:2` | //! record correction, at Skeleton::FillPhysOut82BE2138/2140 publication time. |
| `0x82be2538` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:31` | // Both82BD8280 and82BE2538 read Skeleton16424 ->Reckoning1216. |
| `0x82be2798` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_general.rs:115` | ///Real82BE2798 reset is also callable by the coordinated player reset; |
| `0x82be27b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_general.rs:122` | //82BE27B0 clears the same16388 that states arm and PostWipeoutCheck |
| `0x82be3170` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/input.rs:47` | //82BE3170 reads mapped physical target12624; NOT record6480. |
| `0x82be3170` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:151` | //82BE3170 composes Skeleton11920 with current12624; not physical pose. |
| `0x82be3220` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/services.rs:26` | ///82D773F8 reads real line tests and82BE3220 world animation-record bones. |
| `0x82be3350` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll.rs:34` | //Request11 calls82BE3350(true), selecting normal limits while the |
| `0x82be33d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/launch.rs:1` | //! Wall-jump launch packet;82D33448 constructs it,82BE33D0 fills it. |
| `0x82be33d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:231` | fn publish_animated_pose_82be33d0(&mut self, info: &GroundLaunchInfo) -> Result<(), String> { |
| `0x82be33d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:233` | "Ground wall jump requires actual Skeleton/Reckoning launch observations82BE33D0", |
| `0x82be3508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:290` | // placement (whose skeleton reset82BE3508 clears Skeleton16420) runs while the state is |
| `0x82be3508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:75` | ///Skeleton16420, changed by graph force-mode requests and reset82BE3508. |
| `0x82be3508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_teleport.rs:1` | //! Actual teleport skeleton stages82BE3508 and82BDF7C0. The player owns the |
| `0x82be3508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_teleport.rs:18` | ///82BE3508, after board reset and before player ResetSystems82DB92D0. |
| `0x82be3550` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:77` | ///HeadTracking32..144 and160. Reset82BE3550..35A4 preserves its settings. |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:156` | //82BE3650 -> PhysOutSkeleton0/432 at82BE20F4/2100. Its stance flag is |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:18` | /// GetEffectiveRoot82BE3650 negates both iff Processed2476 bit2 is set. |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:322` | // Skeleton GetEffectiveRoot82BE3650 negates X/Z iff Processed2476 bit2. |
| `0x82be3650` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/input.rs:21` | //82BE3650: actual animation root, with effective stance axes0/2. |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/entry.rs:1` | //! Effective animation root82BE3650, called by Ground Enter at82D30848. |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:215` | //82D30848 calls GetEffectiveRoot82BE3650 BEFORE Ground placement. |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_state.rs:54` | //GetEffectiveRoot82BE3650 applies the animation stance before Enter. |
| `0x82be3650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:73` | // GetEffectiveRoot 0x82be3650 negates X and Z with Processed2476 bit 2. |
| `0x82be4620` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:170` | //owners. Board82C067F4 and Skeleton82BE4620 both store this actor. |
| `0x82be5094` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve.rs:40` | // Skeleton82BE5094 passes false to82768728: its edge threshold is -1, |
| `0x82be58a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll/settings.rs:1` | //! Authored inputs to normal82BE58A8 and ragdoll82BE59A8/6D60. |
| `0x82be59a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll/settings.rs:1` | //! Authored inputs to normal82BE58A8 and ragdoll82BE59A8/6D60. |
| `0x82be6f40` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:60` | /// `Pan2DKernelBase::SpeakerConfig` 0x82be6f40. |
| `0x82be7170` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:262` | /// `Pan2DKernel::ComputeLevels` 0x82be72e8 / `ComputeLevels5` 0x82be7170. |
| `0x82be7280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_collision_settings.rs:1` | //! Original82BE7860/82BE7280 and exact locations in stock physics_skeleton XML. |
| `0x82be7280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll.rs:1` | //! Physical-owner adapter for selector82D913C0 requests8..11 and82BE7280. |
| `0x82be7280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll.rs:55` | ///Original SetUpNormal82BE7280: limits, normal table/group, observation |
| `0x82be72e8` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:262` | /// `Pan2DKernel::ComputeLevels` 0x82be72e8 / `ComputeLevels5` 0x82be7170. |
| `0x82be7484` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_collision_settings.rs:24` | compliant[0] = true; //82BE7484; part0 priority is cleared by collisionReset. |
| `0x82be7860` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_collision_settings.rs:1` | //! Original82BE7860/82BE7280 and exact locations in stock physics_skeleton XML. |
| `0x82be7860` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/drive_settings.rs:1` | //! Native82BE7860's per-part records and82BEB0C8/82BEB790 settings. |
| `0x82be7b98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body.rs:77` | //82BE7B98..BAC builds057EDDFA341F9492=CollisionDriveScalar. |
| `0x82beb0c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/drive_settings.rs:1` | //! Native82BE7860's per-part records and82BEB0C8/82BEB790 settings. |
| `0x82beb0c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/skeleton.rs:94` | //IK may modify it above.82BEB0C8 builds its inverses from that supplied |
| `0x82beb790` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/drive_settings.rs:1` | //! Native82BE7860's per-part records and82BEB0C8/82BEB790 settings. |
| `0x82bec4a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_drive_settings.rs:1` | //! Original BoneDrives constructor82BEC4A0/82BEC7B0 collection bindings. |
| `0x82bec50c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_drive_settings.rs:2` | //!82BEC50C..544 builds local=00132D1E84F0BC32;82BEC5B4..D4 builds |
| `0x82bec5b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_drive_settings.rs:2` | //!82BEC50C..544 builds local=00132D1E84F0BC32;82BEC5B4..D4 builds |
| `0x82bec7b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_body_drive_settings.rs:1` | //! Original BoneDrives constructor82BEC4A0/82BEC7B0 collection bindings. |
| `0x82bed418` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik.rs:191` | ///82BED418 classF2BD493E resolves to physics_skeletonik/default. XML schema |
| `0x82bed688` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animated_skeleton.rs:46` | ///Existing stock82BED688 target mappings; left and right reparented hands. |
| `0x82bed688` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animated_skeleton.rs:91` | // Native82BED688 exact order and names. These are board-parented |
| `0x82bed688` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/pose_adjust.rs:7` | /// Reparented indices are the existing target_bones[2] and[3] loaded by82BED688. |
| `0x82bed74c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames_tests.rs:7` | //TU3 Init82BED74C..770:1120 target parts and1136 adjacent parts. |
| `0x82bedcc8` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:378` | /// `Pan2D1::RampPanOutput` 0x82bedcc8: levels ramp from `old` over 64 samples. |
| `0x82bedfd8` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:353` | /// `Pan2D1::PanOutput` 0x82bedfd8. |
| `0x82bee200` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:417` | /// `Pan2D1::Process` 0x82bee200. |
| `0x82beee58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames_tests.rs:31` | //82BEEE58..88 selects target zero from2480bit27 and bounds its delta. |
| `0x82bf1b90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik.rs:26` | ///82BF1B90 reruns the contact pass without refreshing it. |
| `0x82bf2f18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:105` | ///82BF2F18 is called on the actual landing/takeoff transition; reverse is |
| `0x82bf7ba8` | Skate 2? | `crates/skate2-audio/src/rwac/plugins/pan2d.rs:303` | /// `Pan2D1::CreateInstance` 0x82bf7ba8. `constructor` is (front angle, rear angle, norm mode). |
| `0x82bf9ef0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/input.rs:42` | /// Reset82BF9EF0 writes0; preserve subsequent producer updates. |
| `0x82bf9ef0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/reset.rs:1` | //! Selective reset of the represented ProcessedPhysIn fields, TU3 82BF9EF0. |
| `0x82bf9ef0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:90` | fn reset_processed_input_82bf9ef0( |
| `0x82bf9ef0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics.rs:129` | /// ProcessedPhysIn reset82BF9EF0 sets0x2000; initial stancebit20 is clear. |
| `0x82bfa1d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/reset.rs:31` | // Raw 82BFA1D0 and 82BFA1D8 both store to +816; +832 is retained. |
| `0x82bfa1d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/reset.rs:31` | // Raw 82BFA1D0 and 82BFA1D8 both store to +816; +832 is retained. |
| `0x82bfa35c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:38` | /// Processed2724: Reset82BFA35C clears this. The original image contains |
| `0x82bfaff0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:228` | /// IsPositionOk82BFAFF0: the ground test (its category8 off-board answer is discarded), then the |
| `0x82bfaff0` | TU3 | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:165` | /// IsPositionOk 82BFAFF0 when the Actor's physics is Skate 3's; None otherwise. |
| `0x82bfaff0` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1024` | /// IsPositionOk82BFAFF0 for a session marker. |
| `0x82bfaff0` | TU3 | `crates/skate2-core/src/session/selected_session_marker.rs:332` | // Skate 3 physics: TU3's IsPositionOk 82BFAFF0 (its ground test also takes category 8). |
| `0x82bfb1e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:22` | /// This tick's completed output for the Actor's PostUpdate (82BFB1E0). |
| `0x82bfb1e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:39` | /// `INetworkGameQuery` slot 0 (IsNetworkGame); breadcrumb Update82BFB1E0 skips its ground |
| `0x82bfb1e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:153` | /// PostUpdate's breadcrumb Update (82BFB1E0) over this tick's output. |
| `0x82bfb1e0` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:969` | /// request, then PostUpdate's breadcrumb Update82BFB1E0, with the host's network and |
| `0x82bfb1e0` | TU3 | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:123` | /// Skate 3 physics records the Actor's crumbs with TU3's breadcrumb Update (82BFB1E0) and |
| `0x82bfb1e0` | TU3 | `crates/skate2-core/src/actors/actor_breadcrumb_postupdate.rs:87` | // With Skate 3 physics, TU3's breadcrumb Update (82BFB1E0) runs after the source tick |
| `0x82bfb928` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:36` | /// (82BFBB48) and the living world's DMO, vehicle and pedestrian tests (82BFB928). A standalone |
| `0x82bfb928` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:44` | /// `TestForLWRespawnContraints` 82BFB928, which accepts every position in a network game. |
| `0x82bfb928` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:354` | //82BFB928 queries the LivingWorldManager's DMOs, vehicles and pedestrians. The player and |
| `0x82bfbb48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:36` | /// (82BFBB48) and the living world's DMO, vehicle and pedestrian tests (82BFB928). A standalone |
| `0x82bfbb48` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:347` | //82BFBB48 asks the collision query only in a network game, 0.1m above the position. |
| `0x82bfbb48` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:186` | /// `82BFBB48`: the network game's collision query with radius -1; with no boundaries loaded |
| `0x82bfbc18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:16` | ///82BFBC18 sets that byte for surface category8. Authored rideable spawn |
| `0x82bfc828` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:197` | /// GetSuggestedRespawnPosition82BFC828, the history unchanged. |
| `0x82bfc828` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1016` | /// GetSuggestedRespawnPosition82BFC828 over the history. |
| `0x82bfc828` | TU3 | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:296` | /// (`pop`) or GetSuggestedRespawnPosition 82BFC828 over that history. None otherwise. |
| `0x82bfc9b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:212` | /// PopSuggestedRespawnPosition82BFC9B0 for a reset the source did not request (security capture's |
| `0x82bfc9b0` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:1020` | /// PopSuggestedRespawnPosition82BFC9B0 over the history. |
| `0x82bfc9b0` | TU3 | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:295` | /// With Skate 3 physics the Actor's crumbs are TU3's: PopSuggestedRespawnPosition 82BFC9B0 |
| `0x82c010dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:32` | /// Skateboard constructor binds +0x554 to FindCollection(physicsdeck, "debug") (82C010DC.. |
| `0x82c0111c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/probes.rs:28` | //Skateboard ctor82C0111C starts optional wall line byte291 clear. |
| `0x82c01178` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:33` | /// 82C01178, key 550740B00AA8D4AC), not the default collection, and ApplyForces reads |
| `0x82c013f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:162` | fn prepare_board_toolkit_82c013f0( |
| `0x82c01440` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:119` | //82C01440..60: Processed64 is the unmodified deck part transform. |
| `0x82c01460` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/probes.rs:63` | ///82C01460, not the newly solved body's current transform. |
| `0x82c01744` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:169` | //82C01744 publishes this during toolkit preparation, not PhysOut fill. |
| `0x82c01900` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:31` | //Original82C01900: Globals260 layout4, graphs0/80; |
| `0x82c01bf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:33` | //variant82C01BF8 supplies Basic96 and128; neither is the Skeleton forward. |
| `0x82c01bf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:245` | //82592A00 -> SkateboardReckoning64 ->82C01BF8: solved deck, with bit20 flip. |
| `0x82c01bf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/world_types.rs:362` | /// Processed2468 bit20: GetEffectiveTransform82C01BF8 negates the deck's X and Z. |
| `0x82c01bf8` | TU3? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:204` | // GetEffectiveTransform82C01BF8: the deck as the rider faces it. |
| `0x82c01f10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/probes.rs:60` | /// PostPhysics82C02138 calls82C01F10 before updating board contact output. |
| `0x82c02138` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/probes.rs:60` | /// PostPhysics82C02138 calls82C01F10 before updating board contact output. |
| `0x82c02158` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics.rs:429` | //Skateboard::UpdatePostPhysics82C02158 prepares the wall probe for |
| `0x82c02388` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/dynamic_normal.rs:1` | //! Stock bindings for original82C02388. Hashes were matched to the actual |
| `0x82c02840` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/ground_position.rs:2` | //! 82C02840. The four wheel rigid-body centres are expressed in Reckoning752, |
| `0x82c02840` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:153` | fn calculate_ground_position_82c02840( |
| `0x82c02a80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:358` | ///82C02A80 publishes actual body rates with filtered normal1216. |
| `0x82c02ad8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:459` | // Board Fill82C02AD8..2B1C publishes physical part6 Z to |
| `0x82c02ad8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:32` | //Board Fill82C02AD8/B14 copies the unadjusted part transform. Its effective |
| `0x82c02d0c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:365` | //82C02D0C: physical deck body+16, not geometric part translation. |
| `0x82c02d0c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:37` | //Basic144 comes from the dynamics COM at82C02D0C, not part translation48. |
| `0x82c03304` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:316` | // Board FillPhysOut82C03304/3318: CollisionInfo+16 ->Ground80. |
| `0x82c03304` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:161` | //82C03304..3340 copies actual BoardBody80 into Ground80 and96. |
| `0x82c03318` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/mod.rs:126` | // Board Fill82C03318 and common ProcessOutput82DB7598. |
| `0x82c03638` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:837` | // wobble 0x82C03638 from the centre of mass's distance to the deck (Skeleton+11008). |
| `0x82c03718` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:31` | /// The Y offset Skateboard::ApplyForces 82C03718 adds to every queued board force's point. The |
| `0x82c0373c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:34` | /// DeckForceYOffset from it (82C0373C..82C03778). The stock debug collection has no such |
| `0x82c03778` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:34` | /// DeckForceYOffset from it (82C0373C..82C03778). The stock debug collection has no such |
| `0x82c03af8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_materials.rs:46` | //activation or wiping-out state. They are not full SetStandard82C03AF8. |
| `0x82c03af8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/entry.rs:135` | /// Board-owned portion of SetStandard82C03AF8. 82C090D4/90EC write |
| `0x82c03f98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/board.rs:1` | //! Concrete CURRENT BoardRuntime adaptation of AddWorldForce82C03F98. |
| `0x82c040f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:18` | //82C040F0(0): shared board steering helper, not Ground locomotion. |
| `0x82c040f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:180` | //82C040F0(0): zero linear drag on every board body. |
| `0x82c04168` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/launch.rs:1` | //! Concrete SetVelocity82C04168 continuation; retain main's jumper owner. |
| `0x82c04168` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/mod.rs:142` | ///82C04168 publishes the requested linear velocity to all seven real body |
| `0x82c04168` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:220` | fn set_animated_velocity_82c04168(&mut self, v: [f32; 4]) -> Result<(), String> { |
| `0x82c041c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:64` | ///82C041C0: publish the effective matrix to the separate deck constraint |
| `0x82c04270` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_effects.rs:107` | //82C04270 reads Body+16 (mass-frame position), not GetPartTransform. |
| `0x82c04270` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:149` | //82C04270 uses the deck BODY COM, not Part6's mass-offset transform. |
| `0x82c04368` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/nonspecific.rs:54` | //82D42F50 starts with steering0 then82C04368, before angular/support. |
| `0x82c04368` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/skeleton.rs:40` | ///Nonspecific performs82C04368 in its subsequent board helper, after steering. |
| `0x82c04368` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:109` | //82BDF530 then82C04368: CURRENT wrapper also retains physics error. |
| `0x82c05f50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:59` | /// Typed portion of Skateboard Reset82C05F50, alongside the coordinator's |
| `0x82c05f50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/mod.rs:59` | //SkateboardReset82C05F50 explicitly clears192..208, then sets196=1. |
| `0x82c06048` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:68` | ground.steering.deck_tilt = 0.0; //82C06048, wrapper256. |
| `0x82c06060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:69` | ground.steering.targets = [0.0; 2]; //82C06060/64, body7680/7684. |
| `0x82c0606c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:73` | self.skeleton_air.reset_board(); //Board Reset82C0606C. |
| `0x82c0606c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:59` | ///82C0606C; preserves the source's independently retained blending flag. |
| `0x82c06070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:71` | life.board_animated_290 = 0; //82C06070. |
| `0x82c060b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/mod.rs:50` | /// Skateboard Reset82C060B0 publishes [0,1,0,0] to the retained |
| `0x82c067f4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_feedback.rs:170` | //owners. Board82C067F4 and Skeleton82BE4620 both store this actor. |
| `0x82c0680c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics.rs:132` | /// Toolkit ctor82C0680C clears8384bit7; wipeout entry/exit owns changes. |
| `0x82c07000` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:282` | fn apply_vector_82c07000(&mut self, v: [f32; 4]) -> Result<(), String> { |
| `0x82c075b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/nonspecific.rs:81` | //82C075B8 adds torque acceleration, not angular velocity/orientation. |
| `0x82c075b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:286` | fn apply_angular_displacement_82c075b8(&mut self, v: [f32; 4]) -> Result<(), String> { |
| `0x82c07788` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:316` | /// StartSkateboardLineTests82DB6310 ->82C07788: four current wheel positions, |
| `0x82c07788` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:349` | // TU3 Start/EndSkeletonLineTests82DB63C0/6580 and board probe82C07788 |
| `0x82c07d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:356` | ///UpdatePostPhysics82C07D20 consumes current solved reports. |
| `0x82c07d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:370` | // groups (the pre-82C07D20 behaviour), for A/B comparisons. |
| `0x82c08198` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/surface.rs:4` | /// Wheel surface IDs come from line queries. The report loop82C08198 updates |
| `0x82c08370` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_effects.rs:127` | ///82C08370..8428 classifies retained per-part normals against the controller's |
| `0x82c0837c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:268` | /// alignment test is82C0837C..8428 and publishes the actual release bit872. |
| `0x82c08450` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skateboard_controller.rs:9` | //82C08450..8494 counts Board844..850 into byte868. This is not a |
| `0x82c08494` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout/observations.rs:57` | //82C08494/82C08640 publish the actual counts to868/869. |
| `0x82c08634` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:398` | // S3 82C08634 writes inertia+36; S2 82B372E0 names this |
| `0x82c08640` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout/observations.rs:57` | //82C08494/82C08640 publish the actual counts to868/869. |
| `0x82c08818` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/surface.rs:1` | //! Native wheel surface vote82C08818 on live wheel lines and contact reports. |
| `0x82c08818` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:159` | ///82C08818 votes only four wheel surfaces; physical contact counts four, |
| `0x82c08818` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/possession.rs:58` | //82C08818 produces840 by voting the four retained wheel surfaces. |
| `0x82c08818` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics.rs:521` | /// Board surface vote (82C08818), including water surface 12. |
| `0x82c08818` | TU3 | `crates/skate2-core/src/physics/board/collision/skateboard_contact_surface.rs:280` | /// (0x82c08818 in its title update 3) and its `SurfacePhysics::Dispatch` (0x82d86018) rides |
| `0x82c08968` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:110` | ///82C08968 observes the solved COM frames, not geometric part frames. |
| `0x82c08c28` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_materials.rs:2` | //! Init82C08C28 clears all three grind materials at82C09084..A4; S2 Init |
| `0x82c08d04` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:94` | //82C08D04..18 copies layout16 to SkateboardBody8380. |
| `0x82c09084` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_materials.rs:2` | //! Init82C08C28 clears all three grind materials at82C09084..A4; S2 Init |
| `0x82c090c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_materials.rs:1` | //!82D89DC0/82C090C0 material modes, applied to the live board collider slots. |
| `0x82c090d4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/entry.rs:135` | /// Board-owned portion of SetStandard82C03AF8. 82C090D4/90EC write |
| `0x82c091f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:35` | // Existing82C091F8 standard setting, already scaled by822F860C. |
| `0x82c091f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/entry.rs:31` | //82C091F8 restores the standard angular drag with literal822F860C. |
| `0x82c0b560` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:53` | //the skeleton's part bitmap. Board82C0B560 culls all7x7 self pairs. |
| `0x82c0d680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:61` | /// by either this reset or ResetBoardBody82C0D680, so preserve activation_time. |
| `0x82c0d680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/probes.rs:38` | ///ResetBoardBody82C0D680 clears published probe records, preserving |
| `0x82c0d680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:63` | ///Board82C0D680 resets CollisionInfo and both probes, preserving7692. |
| `0x82c0d680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:72` | //Original82C0D680 preserves query handles203C..2050. EndBoard |
| `0x82c0d79c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:72` | *board_wiping_out = false; //82C0D79C..7A0 clears body8384 high2bits. |
| `0x82c0e308` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:35` | ///82C0E308 ->82BC1B68: clear actual ActionGraph intent map only. |
| `0x82c11b88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/condition_nodes.rs:140` | // Node::GetParentState82C11B88 climbs through expressions/transitions |
| `0x82c11b88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:451` | // GetParentState82C11B88 climbs expression nodes to the |
| `0x82c126f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_riding_conditions.rs:68` | // Factories82BC3258/82BC31B8 use NumericCondition82C126F0. |
| `0x82c13728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/condition_nodes.rs:145` | *target = graph.binding.find_state(state, name, true); //82C13728 |
| `0x82c13820` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:293` | //ancestor membership test as CurrentState82C13820. |
| `0x82c174f8` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:635` | /// `SNDAEMSI_updatedestroy` 0x82c174f8 for a triggered DESTROY: unlink, unsubscribe the states |
| `0x82c18b78` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:528` | /// The `FunctionClient` callbacks: `SNDAEMSI_SetGlobalVariable` 0x82c18b78, |
| `0x82c18b98` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:529` | /// `SNDAEMSI_SetClassData` 0x82c18b98 and `CsisFunctionCallback` 0x82c18bd8. |
| `0x82c18bd8` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:529` | /// `SNDAEMSI_SetClassData` 0x82c18b98 and `CsisFunctionCallback` 0x82c18bd8. |
| `0x82c18c20` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:563` | /// `SNDAEMSI_CreateModuleInstance` 0x82c18c20: copy the template, fill the destroy state, |
| `0x82c18e80` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:402` | /// posted `RemoveModuleBankHandler` 0x82c18e80 runs on the next audio frame. -10 when no |
| `0x82c18e80` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:430` | /// `Snd9::RemoveModuleBankHandler` 0x82c18e80 for each posted removal: the bank leaves the |
| `0x82c18f50` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:400` | /// `Snd9::Aems::BeginRemoveModuleBank` 0x82c18f50: each module's constructor client leaves |
| `0x82c19090` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:424` | /// `Snd9::Aems::IsModuleBankRemoved` 0x82c19090: no bank in the list has the handle. |
| `0x82c19118` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:691` | /// 0x82c19118), every instance in timer-list order. |
| `0x82c1b3f0` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:690` | /// `SNDAEMSI_timerupdate` 0x82c1b3f0: every `n`-th call (`CalculateAemsTimerFrequency` |
| `0x82c1b478` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:307` | /// `SNDAEMS_addmodulebank` 0x82c1b7d0 and `SNDAEMSI_resolvemodulebank` 0x82c1b478: bind the |
| `0x82c1b7d0` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:307` | /// `SNDAEMS_addmodulebank` 0x82c1b7d0 and `SNDAEMSI_resolvemodulebank` 0x82c1b478: bind the |
| `0x82c1e170` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:107` | ///the last handle length.82C1E170 removes only positive up projection. |
| `0x82c1e568` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/provider.rs:193` | /// each primitive in the box (82C1E568: D -> (A+B)+(C+D)), in [`Self::query`] order. The |
| `0x82c1e568` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:1` | //! User-authorized ZIP spline adapter, checked against original TU3 82C1E568, |
| `0x82c1e568` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:15` | /// 82C1E568 computed near-vertical high bit; newly allocated lower bits zero. |
| `0x82c1e568` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:128` | // Original 82C1E568 / 82C1F098 emit every segment. Do not drop tiny or |
| `0x82c1e6c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:2` | //! 82C1E6C0, 82C1EEF0 and 82C1F098 in default.patched.xex (431b8eba...). |
| `0x82c1ead8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/provider.rs:172` | /// S3 82C1EAD8 static pass only: query gate, ordered asset overlap and |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/provider.rs:192` | /// `GrindData::Implementation::BoxTest` (82C1EAD8) as its edge users read it: the chord of |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:83` | //82C1EAD8 preserves world order and caps the broadphase result at40. |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:21` | /// The grind data `GrindData::BoxTest` (82C1EAD8) walks for the edge collection. |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:340` | /// 82D81610 bounds and 82C1EAD8: the grind data's static pass first, then any authored |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query/edges.rs:31` | //82C1EAD8: the grind data's static pass comes first and shares the cap of forty. |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query.rs:1` | //! Borrowed scene adapter for original Biped query82764AB0/82770650/82C1EAD8. |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query.rs:64` | /// The grind data `GrindData::BoxTest` (82C1EAD8) walks before its moving providers: the |
| `0x82c1ead8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:375` | //82C1EAD8: the grind data's static pass, then any authored static edges, at most |
| `0x82c1eef0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:2` | //! 82C1E6C0, 82C1EEF0 and 82C1F098 in default.patched.xex (431b8eba...). |
| `0x82c1eef0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:24` | // TU3 82C1EEF0 reads the rail count with lhz +2, not a full 32-bit load. |
| `0x82c1f098` | TU3 | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:2` | //! 82C1E6C0, 82C1EEF0 and 82C1F098 in default.patched.xex (431b8eba...). |
| `0x82c1f098` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/spline.rs:128` | // Original 82C1E568 / 82C1F098 emit every segment. Do not drop tiny or |
| `0x82c205d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:21` | ///82C205D8/82C20728: descriptor5 has radius .001; the others have radius0. |
| `0x82c20728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:25` | ///82C20728/82764AB0: caller-sized lines, poolmask7, facing3, no mesh or |
| `0x82c20728` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:1` | //! Static host for original S3 82C20728 and 82E0AAD0 line batches. |
| `0x82c20728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:21` | ///82C205D8/82C20728: descriptor5 has radius .001; the others have radius0. |
| `0x82c209e4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:35` | //82C209E4 supplies the stock Air clearance parameter, not guessed geometry. |
| `0x82c209e4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_selector/settings.rs:9` | ///82C209E4: physics_grinds+636, not measured board geometry. |
| `0x82c209e4` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_geometry.rs:21` | //82C209E4 and Skate2 GetGeometryCollisionOffset82CFABD8: |
| `0x82c20bf0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_geometry.rs:53` | //82C20BF0 starts the real batch during Sync. Host execution may be |
| `0x82c228c8` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:66` | /// `iSNDrandom` 0x82c228c8: the six-word add-with-carry generator over `SNDIrandseed`, which |
| `0x82c22aa0` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:123` | /// `iSNDsin` 0x82c22aa0: a 1024-step period from the quarter table. |
| `0x82c37458` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:385` | /// (0x82e268c8), vehicles 10 and 8 (0x82d28eb8, 0x82c37458), dynamic objects 14/16 and 12/14 |
| `0x82c4c4e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query/edges.rs:55` | //82C4C4E0 checks individual segment bounds, no asset bound gate. |
| `0x82c4e7b8` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:386` | /// (0x82d3aa28, 0x82c4e7b8). Under this map the two `InitCullingTable` pair tables (Skate 2 |
| `0x82c52b30` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:6` | //! (DynamicObject::ApplyForce `0x82c52b30` -> Body::AddForce); Skate 3's pushing attributes (max |
| `0x82c8a610` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:492` | /// `FootControl` ctor 82c92958 and Reset 82c8a610. Folded vector ctor |
| `0x82c8b880` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:526` | /// 82c92a50 constructs four input then four physical feet. Reset 82c8b880 |
| `0x82c92958` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:492` | /// `FootControl` ctor 82c92958 and Reset 82c8a610. Folded vector ctor |
| `0x82c92a50` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:526` | /// 82c92a50 constructs four input then four physical feet. Reset 82c8b880 |
| `0x82c96d58` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_session.rs:3` | //! `GetPhysUpdateData82c96d58` remains a later consumer: its trajectory and |
| `0x82ca80f8` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:8` | //! constructor-bound `TrickHeight` attribute. Skate 2 `0x82CA80F8` is only |
| `0x82ca9b38` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1244` | /// `SkaterAnim::PurgeFacePose` 0x82ca9b38 for a skater whose face pose was set up. |
| `0x82ca9f70` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:586` | /// `SkaterAnim::ToggleSkaterStates82ca9f70`; lookup presence, not scalar truth. |
| `0x82cb7530` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:412` | /// answer when `GetIsRidingFakie` 0x82cb7530 is true. The first getter |
| `0x82cb75d0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:93` | /// Interface setter `SkaterAnim::SetOpacity82cb75d0`: +52e0 interface |
| `0x82cb7668` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:111` | /// Actor825f4264..42c0. Concrete vtable+D0 is `IsMounting82cb7668`; |
| `0x82cb7690` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:949` | /// `SkaterAnim::SetLanding82cb7690` writes only bit 0x8000 of +0xe4c. |
| `0x82cb7728` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:112` | /// +D4 is `SetIsHoldingBoard82cb7728`, not the decompiler's grab setter. |
| `0x82cb7738` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:929` | /// DoAirDismountRevert82cb7738: frames store precedes request bit. |
| `0x82cb7750` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:957` | /// `SkaterAnim::EnableAccelDecel` 0x82cb7750 writes only bit 0x10 of +0xe4c. |
| `0x82cb7798` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1001` | /// `SkaterAnim::SetGrabType82cb7798` stores the full word at +0xee8. |
| `0x82cb77a0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:992` | /// `SkaterAnim::IsCurrentGrabType82cb77a0` compares the word at +0xee8. |
| `0x82cb7868` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:400` | // whose own vtable +0xc0 is SetSpawnFootForwardStance 0x82cb7868 |
| `0x82cb7868` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:965` | /// `SkaterAnim::SetSpawnFootForwardStance`82cb7868 (Animatable-base vtable |
| `0x82cb78a0` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:411` | /// SkaterAnim's `GetFootForward` 0x82cb78a0, then flips its zero/one |
| `0x82cb78a0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:983` | /// GetFootForward82cb78a0: stance before Actor's separate fakie modifier. |
| `0x82cb7c38` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:173` | /// `SkaterAnim::SetAttributes`82cb7c38: shared by Actor phase2 and |
| `0x82cb7c38` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:180` | // SkaterAnim82cb7c38..7d1c: exact literals and distinct sequence IDs. |
| `0x82cbf700` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1129` | /// Selected loaded/non-null `SetupFacePose` 82cbf700. The original recipe |
| `0x82cbf728` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1249` | /// `SetupFacePose` 82cbf728/72c branches straight to its epilogue for an |
| `0x82cbfb50` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1236` | // SetupFacePose82cbfb50 writes mInitFace only after all fixed frames. |
| `0x82cc0a90` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:3` | //! `0x82cc0a90` constructs `Animatable`, `ChannelAnimatable` and `ISkaterAnim` on |
| `0x82cc0b44` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:236` | /// Empty `PoolStack` constructor at 82d7b70c..b778 / 82cc0b44..b74. |
| `0x82cc0e24` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:562` | /// `SkaterAnim` fields through 82cc0e24; the Actor player query follows. |
| `0x82cc0e44` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:883` | // 82cc0e44/54 preserve only 0x039fffff from the unknown old word. |
| `0x82cc0e4c` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:884` | // 82cc0e4c inserts the bool in bit27. No other unknown bit becomes zero. |
| `0x82cc0ef4` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:5` | //! `0x82cc0ef4`; the continuation stops before face setup at `0x82cc1190`. |
| `0x82cc0ef4` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1010` | /// Actual FindCollection/Instance 82cc0ef4/0f04, through fields preceding |
| `0x82cc0f94` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:175` | /// Actual `kUseSprintButton` target byte, read at 82cc0f94/0f98, after |
| `0x82cc0fb0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:665` | /// Actual suffix fields at 82cc0fb0..118c. The collision response turn is NOT |
| `0x82cc1190` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:5` | //! `0x82cc0ef4`; the continuation stops before face setup at `0x82cc1190`. |
| `0x82cc1190` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:1011` | /// `SetupFacePose` 82cc1190. The loaded Vault is the existing real service; |
| `0x82ce1320` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:3` | //! TestForLWRespawnContraints 0x82ce1320, GetLastResortBreadCrumb 0x82ce2598. |
| `0x82ce135c` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:83` | /// crumb (0x82ce135c -> 0x82ce14ac `li r3, 1`). Offline, vehicle sphere, |
| `0x82ce1478` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:136` | // VMX 0x82ce1478/147c: VA63 of the second add is the |
| `0x82ce14ac` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:83` | /// crumb (0x82ce135c -> 0x82ce14ac `li r3, 1`). Offline, vehicle sphere, |
| `0x82ce189c` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_postupdate.rs:131` | // _TestForPhysicsConstraints 0x82ce189c..190c. Its orientation copy |
| `0x82ce1cb8` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:2` | //! GetWipeoutSpotBreadCrumb 0x82ce1cb8, PopBreadCrumb 0x82ce2d80, |
| `0x82ce1e30` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:3` | //! 0x82ce1fd0, ring push_back 0x82ce1e30, and selected pop 0x82ce2d80. |
| `0x82ce1fc0` | Skate 2? | `crates/skate2-core/src/session/selected_session_marker.rs:313` | /// `IsPostionOk`82ce1fc0 on the local Actor's breadcrumbs: |
| `0x82ce1fd0` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:3` | //! 0x82ce1fd0, ring push_back 0x82ce1e30, and selected pop 0x82ce2d80. |
| `0x82ce1fd0` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:99` | /// position, and an empty ring that `DropBreadCrumb` (`0x82ce1fd0`) gives the spawn crumb. |
| `0x82ce2230` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_postupdate.rs:1` | //! SkaterManager::PostUpdate 0x825f8ba4..8c08 and breadcrumb Update82ce2230. |
| `0x82ce22c8` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_postupdate.rs:105` | // Update 0x82ce22c8..2300: signed >=; the ring distance test is |
| `0x82ce2428` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:261` | /// and GetBreadCrumb 0x82ce2428 scans newest to oldest without erasing |
| `0x82ce2598` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:3` | //! TestForLWRespawnContraints 0x82ce1320, GetLastResortBreadCrumb 0x82ce2598. |
| `0x82ce2698` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:260` | /// GetSuggestedRespawnPosition 0x82ce2698, the const query: no override, |
| `0x82ce2d80` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_live_selection.rs:2` | //! GetWipeoutSpotBreadCrumb 0x82ce1cb8, PopBreadCrumb 0x82ce2d80, |
| `0x82ce2d80` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:3` | //! 0x82ce1fd0, ring push_back 0x82ce1e30, and selected pop 0x82ce2d80. |
| `0x82ce2ef8` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:237` | /// PopSuggestedRespawnPosition 0x82ce2ef8. The source must answer every |
| `0x82ce3060` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:236` | /// GetSuggestedBreadCrumb 0x82ce3060 delegates to the override or to |
| `0x82ce30f8` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:346` | /// SerializeBreadCrumbs82ce30f8 copies newest first, up to the 32 live slots. |
| `0x82ce33b8` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:2` | //! Source: BreadCrumbManager::Implementation 0x82ce33b8, DropBreadCrumb |
| `0x82ce33b8` | Skate 2? | `crates/skate2-core/src/actors/actor_breadcrumb_manager.rs:97` | /// (`0x82600630`) makes another: the constructor's (`0x82ce33b8`) spawn position, override |
| `0x82ce7f50` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:336` | // Manager.Init's WaypointMan::Init (0x82ce7f50) empties the spectator queue. |
| `0x82cfabd8` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_geometry.rs:21` | //82C209E4 and Skate2 GetGeometryCollisionOffset82CFABD8: |
| `0x82cfb538` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:259` | /// 82cfb538, `SplineEval_One`82e49908 corrected raw VMX operands. |
| `0x82cfb640` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:1` | //! Retained `GrindData` asset lifecycle,82cfb640/82cfd190/82cfb8b8. |
| `0x82cfb8b8` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:1` | //! Retained `GrindData` asset lifecycle,82cfb640/82cfd190/82cfb8b8. |
| `0x82cfb8e8` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:445` | /// Raw82cfb8e8 LD + b8ec CMPLD proves full64 GUID equality. |
| `0x82cfcfb8` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:419` | /// Selected successful first Init82cfcfb8 allocation. Native repeated Init |
| `0x82cfd190` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:1` | //! Retained `GrindData` asset lifecycle,82cfb640/82cfd190/82cfb8b8. |
| `0x82cfd1fc` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:442` | records.push(record); //82cfd1fc..204, after complete construction; duplicates remain. |
| `0x82d088a8` | Skate 2? | `crates/skate2-core/src/assets/character.rs:325` | /// last writer's. `BuildTInstanceArray` 0x82d088a8 builds that order by |
| `0x82d0a234` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:682` | /// `CharacterRecipeLoader::OnComplete`'s face (0x82d0a234..274): the CAC gender face, not |
| `0x82d14308` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:47` | /// `cAssetRegistry82d65428` constructs an empty map; `CharacterAssetManager82d14308` |
| `0x82d14b90` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:101` | /// `UpdateRequest`'s `RequestState_LoadSchemaFromFile` (0x82d14b90): a schema the registry |
| `0x82d14b90` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:751` | // A schema the registry holds is shared (`RequestState_LoadSchemaFromFile` 0x82d14b90) |
| `0x82d15318` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:791` | // goes straight to `FinalizeStructure` (0x82d15318): `RebindToMaterials` and |
| `0x82d15554` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:458` | /// The unload request, then `UpdateRequest`'s `RequestState_UnloadRequested` (0x82d15554): |
| `0x82d155d8` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:459` | /// the entity loses this load's reference, and only the last one's `Unload` (0x82d155d8) |
| `0x82d164f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/stock_clip_query.rs:14` | //82D16680 requests Attribute::Init82D164F8 at time0, irrespective of |
| `0x82d164f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing_settings.rs:72` | // Attribute::Init82D164F8 copies lane0 for types0/1/3. Type2 |
| `0x82d164f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings/metrics.rs:2` | //!82D164F8 multiplies the authored end by that length, including end=-1. |
| `0x82d1659c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/stock_clip_query.rs:20` | _ => Ok(0.0), //82D1659C: no payload write for other kinds. |
| `0x82d16680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/stock_clip_query.rs:1` | //! Direct named-bank query82D16680, not the currently playing tree/channel. |
| `0x82d16680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/stock_clip_query.rs:14` | //82D16680 requests Attribute::Init82D164F8 at time0, irrespective of |
| `0x82d16680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings/metrics.rs:1` | //!82D16680 uses first matching record, status4, length=frames/fps. |
| `0x82d16680` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings.rs:1` | //! Stock Biped settings82D7B630 and metric queries82D7AFD8/82D16680. |
| `0x82d17370` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:388` | /// Andale::TransitionStore (ctor 0x82D17370): mMaxNumTransitionBlends 3, mMaxNumTrees 4. |
| `0x82d17718` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:543` | // ProcessLineTestBatch82d1add0, ReceiveGroundTests82d17718. All three |
| `0x82d17cdc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:10` | /// Base animator14652: ctor82D17CDC sets zero; virtual40/82531200 writes it. |
| `0x82d180e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:40` | ///82B98050 +82D1CAE8 +82D180E8: masks/stance bit, channel teardown, |
| `0x82d181d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation.rs:156` | /// 82D181D0 stores it at13484, and82531050 emits that pose command. |
| `0x82d18658` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:428` | /// ForceTransitionRelease 0x82D18658: the transition's parent (or the root) takes its |
| `0x82d186f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:578` | //82D186F8's no-current branch calls Play with start0. |
| `0x82d18780` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:409` | /// TU3 Andale::Animatable::TransitionAnim 0x82D18780..7CC, before a new tree is made: a blend |
| `0x82d18870` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:42` | Ok(()) // GetPedestrian returned null, as in UpdateHornState82d18870. |
| `0x82d188a8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:593` | // TU3 TransitionAnim 0x82D18950/0x82D188A8: with a live transition, a transition |
| `0x82d18950` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation.rs:593` | // TU3 TransitionAnim 0x82D18950/0x82D188A8: with a live transition, a transition |
| `0x82d18f98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:63` | //82D18F98, then v48 supplies the tree for GetLength(v20). |
| `0x82d19010` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_conditions.rs:390` | //82BA6378 queries the FIRST cached tree record via82D19010. |
| `0x82d19010` | TU3? | `crates/skate2-core/src/physics/modes/skate3/skater_animation/state.rs:49` | ///82B98980 calls82D19010's first-match query. No scalar/status test: |
| `0x82d19100` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing.rs:53` | /// kind0/status6, as published by82D19100. `normalized` is the control flag |
| `0x82d19648` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/selection_space_host.rs:14` | //TU3 vtable8231E050+76 ->82E32328 ->82D19648 constructs a |
| `0x82d1add0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:543` | // ProcessLineTestBatch82d1add0, ReceiveGroundTests82d17718. All three |
| `0x82d1b9b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/tree_builder.rs:85` | //82D1B9B4..BAD8 resolves the first matching construction value at |
| `0x82d1cae8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:40` | ///82B98050 +82D1CAE8 +82D180E8: masks/stance bit, channel teardown, |
| `0x82d1cf70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:72` | //82D1CF70..D018: greater priority lies above lesser; a new equal |
| `0x82d1d030` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:201` | //82BAAED8 -> channel virtual20=82D1D030 chooses embedded Sequence, |
| `0x82d1d030` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:96` | // Vtable8231E128: +16 = 82D1D090 (blend), +20 = 82D1D030 |
| `0x82d1d090` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:257` | //Actual vtable8231E128+16 is TransitionTo82D1D090. |
| `0x82d1d090` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:163` | //82BBB164/270 call IChannelAnimatablev16=82D1D090. Stack57=false |
| `0x82d1d090` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:96` | // Vtable8231E128: +16 = 82D1D090 (blend), +20 = 82D1D030 |
| `0x82d1d090` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:177` | //Vtable16 is TransitionTo82D1D090. Stack booleans at87/95/103 are1: |
| `0x82d1d0f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board.rs:97` | // (sequence). Both preserve the channel owner via82D1D0F0. |
| `0x82d1d390` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:129` | /// 82d1d390. The callback returns true, so only the FIRST contact of this |
| `0x82d1d4b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:53` | ///82D1D4B8: clamp the actual child time to [0, child length]. |
| `0x82d1d5f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:43` | ///82D1D5F8: missing channel returns zero; clamp remaining child time. |
| `0x82d1d6a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:137` | //Virtual44 is EndChannelPrematurely82D1D6A8, preserving fade timing. |
| `0x82d1d938` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:627` | /// AddGroundTestsToBatch82d1d938: HiFi only, three corners, vertical +/-2m. |
| `0x82d1dbc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_channels.rs:123` | //CompleteTransition82D1DBC0 unconditionally keeps the prior target |
| `0x82d1e0c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:181` | // TU3 decodes the second key for both (Vector_ExtractPackedNVBR 82D1E0C0..E0, |
| `0x82d21cf0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:356` | /// UpdateLOSStates82d21cf0, separated from execution so the Manager can |
| `0x82d21cf0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:363` | // UpdateLOSStates82d21cf0; collision OnLOSimSwitch82e50580 returns true. |
| `0x82d23bf0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:46` | /// OnCullRequested82d23bf0 then DetachFromManagers82d26f70. Physics bodies |
| `0x82d24078` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/selection_space_host.rs:21` | // Type6 SetAttributes82D24078 consumes its own parameters only. |
| `0x82d25138` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:338` | // Vehicle destructor82d25138 repeats road removal, then collision, |
| `0x82d25e30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/pushing_settings.rs:61` | // GetAttributes82D25E30 appends in stored order. Untimed records |
| `0x82d26f70` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:46` | /// OnCullRequested82d23bf0 then DetachFromManagers82d26f70. Physics bodies |
| `0x82d26f88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/selection_space_host.rs:13` | //82D26F88 calls IAnimatable+76, not SkaterAnim::GetAnimTree. |
| `0x82d28c00` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:438` | /// `KDTree::FindNear`82d28c00 through the same typed EntityMan40 tree. |
| `0x82d28c00` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:472` | /// -> `KDTree::FindNear` 0x82d28c00. This uses the SAME selected vehicle |
| `0x82d28eb8` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:385` | /// (0x82e268c8), vehicles 10 and 8 (0x82d28eb8, 0x82c37458), dynamic objects 14/16 and 12/14 |
| `0x82d29030` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:471` | /// `GetVehicleMan` -> `LWVehicleMan::GetVehiclesWithinRange` 0x82d29030 |
| `0x82d29034` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:492` | max_entities.min(24), // 0x82d29034..40, before FindNear. |
| `0x82d2bf18` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:185` | /// 0x82d2bf18): the bank arena's `tSplineData` object whose inline header |
| `0x82d2cdc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime/rotation.rs:82` | // 82D2CDC0 builds an orthonormal frame from reckoning up/deck Z. |
| `0x82d2d860` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:2` | //! Ground/Air/KnownAir vtable24 ->82D34DA8 ->82D2D860 predicts from the actual deck part |
| `0x82d2d860` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:11` | //Six grind vtables call82D2D860 directly;701 calls the82D34DA8 wrapper. |
| `0x82d2d988` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:110` | let contact = hit.contact_position; //Admission uses GetPosition82D2D988. |
| `0x82d2db98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/sync.rs:31` | //104 is uninitialized by82D2DB98 and overwritten on every produce path. |
| `0x82d2ddd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/pre_input.rs:1` | //! Original82D81610 inactive-query branch and82D2DDD0 result reset. Pending |
| `0x82d2e250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:67` | /// `0x82D2E250` with argument7 true: the frame's own axes, unlike the off-board board box (the |
| `0x82d2e328` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:71` | // 0x82D2E328..330: the stacked right axis takes a zero height before the normalize. |
| `0x82d2e5c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:1` | //! Distinct State501 lifecycle: original S3 82D2E5C0/EF38/FFF8/30300/EF20. |
| `0x82d2ef38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:1` | //! Ordered82D2EF38, including actual skeleton/selector/landing consumers. |
| `0x82d2eff0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:25` | //82D2EFF0/82D2F014 load processed+0xB40 (AnimEndCOM), not |
| `0x82d2f014` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:25` | //82D2EFF0/82D2F014 load processed+0xB40 (AnimEndCOM), not |
| `0x82d2fff8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/post.rs:1` | //! State501 Post82D2FFF8, after completed physical skeleton feedback. |
| `0x82d30018` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/post.rs:5` | //82D30018..30: synchronize the SAME manager state503 will consume. |
| `0x82d30300` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/publication.rs:6` | //82D30300 writes OffBoard, not the overlapping on-board Air packet. |
| `0x82d305d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings/board.rs:1` | //! Ground ctor82D305D8 passes state36 to82B97060, whose full64 class/key |
| `0x82d30808` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/runtime.rs:179` | /// this operation in82D30808 and must be performed by the state adapter. |
| `0x82d30848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/entry.rs:1` | //! Effective animation root82BE3650, called by Ground Enter at82D30848. |
| `0x82d30848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:215` | //82D30848 calls GetEffectiveRoot82BE3650 BEFORE Ground placement. |
| `0x82d30b44` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:246` | //82D30B44: actual state56 ConsiderReset, after placement and before mode4. |
| `0x82d30b54` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:248` | //82D30B54 requests the shared skeleton collision controller's mode4. |
| `0x82d30b5c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:253` | //82D30B5C..B64: state52 grab manager, NOT SkateboardController60. |
| `0x82d30b64` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:197` | ///GroundEnter82D30B64 additionally clears interactable publication readiness. |
| `0x82d30bd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:99` | //82D30BD0 is a Ground-state reset, not Biped82D7B1C0. |
| `0x82d30cc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:262` | //82D30CC0 releases the ground query. It does NOT reset shared Biped |
| `0x82d30cd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:259` | //82D30CD8 refreshes, then30CE4..CF8 clears readiness/history before |
| `0x82d30d30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_input.rs:1` | //! PreUpdate82D30D30 inputs borrowed from their completed production owners. |
| `0x82d30dbc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_input.rs:13` | //82D30DBC tests byte1092 of the third line record, then copies1056. |
| `0x82d310f8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings.rs:17` | ///82D310F8 full key DF759B46440F16E9. |
| `0x82d310f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings.rs:19` | ///82D310F8 full key2DD95B399BAE313E. |
| `0x82d31ce8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:418` | //82D31CE8..D04 is Ground FeetIK, not Air's timer-reset wrapper. |
| `0x82d31d14` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:420` | //82D31D14 calls the shared board steering owner with zero input before |
| `0x82d31d1c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/ground_board.rs:1` | //! Ground carried-board Update82D31D1C..DB0. State60 is SkateboardController; |
| `0x82d31d1c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:428` | //82D31D1C..DB0 operates the EXISTING carried-board controller. |
| `0x82d31d78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/ground_board.rs:32` | //82D31D78: 82D75AC8 keeps the board animated where it rests. |
| `0x82d31d80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/ground_board.rs:44` | //82D31D80/DB0 assigns state only AFTER the physical action. |
| `0x82d31e64` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:441` | // Sync82D31E64..82D32070: physical contact correction and the Enter |
| `0x82d32070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:441` | // Sync82D31E64..82D32070: physical contact correction and the Enter |
| `0x82d32084` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:445` | //82D32084 passes state1056 (GroundResult208), not processed752. |
| `0x82d32088` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/services.rs:103` | ///82D32088..32108, after Skeleton has published its real processed fields. |
| `0x82d32088` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:500` | //82D32088..32108: real grab-spline host executes grab::sync here. |
| `0x82d32130` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:502` | //82D32130..164: surface-frame axes, NOT physical-frame axes. |
| `0x82d32164` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:245` | //Sync82D32164 submits AFTER Skeleton; both query batches become |
| `0x82d321cc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_geometry.rs:58` | //82D321CC: no selected edge means no submit, not a fabricated query. |
| `0x82d32338` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/sync.rs:12` | ///82D32338. This writes state146 on every call, including the no-launch path. |
| `0x82d32338` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:430` | //82D32338 precedes contact correction and angular unwind. Submit through |
| `0x82d324b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab.rs:1` | //! Concrete Ground grab decision82D324B0. Query readiness belongs to ONE owner. |
| `0x82d324b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/ground_board.rs:2` | //! state52 in Sync82D324B0 is a DIFFERENT grab-spline manager. |
| `0x82d32aa0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/lifecycle.rs:1` | //! Ground Post82D32AA0 and complete typed Fill82D32D38/82D785F8. |
| `0x82d32d38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/lifecycle.rs:1` | //! Ground Post82D32AA0 and complete typed Fill82D32D38/82D785F8. |
| `0x82d33448` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/launch.rs:1` | //! Wall-jump launch packet;82D33448 constructs it,82BE33D0 fills it. |
| `0x82d33448` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/launch.rs:67` | ///82D33448 layout. Preserve both velocities and all override flags. |
| `0x82d33448` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:228` | fn build_animated_pose_82d33448(&mut self) -> Result<GroundLaunchInfo, String> { |
| `0x82d335f8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:2` | //! Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70. |
| `0x82d33678` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:2` | //! Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70. |
| `0x82d336f8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:2` | //! Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70. |
| `0x82d33928` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:2` | //! Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70. |
| `0x82d33a70` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:2` | //! Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70. |
| `0x82d33b38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:142` | //82D33B38..BC4 modifies only the deck body velocity after wipeout checks. |
| `0x82d33bf8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation.rs:1` | //! GroundAnimation103 lifecycle82D33BF8/33DB8/33C70/34150, original Skate3TU3. |
| `0x82d33db8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation.rs:53` | //82D33DB8 calls UpdateGroundStatesFixedHeading82D8C8F0 without the Ground job's |
| `0x82d33e00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/skeleton.rs:1` | //!82D33E00 calls SkeletonUpdateAnimated82BDDA10 with fast blending enabled. |
| `0x82d33e30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/board.rs:1` | //!82D33E30: jump launch or ordered foot/brake/friction/collision forces. |
| `0x82d34150` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation.rs:20` | ///82D34150 writes only its active fields after the common PhysOut reset. |
| `0x82d34388` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:1` | //! PhysicsAir200 production lifecycle82D34388/82D346C0/82D346A0. |
| `0x82d34570` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:123` | //COM helper82D34570 repeats Fill, then offsets start/board and |
| `0x82d346a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:1` | //! PhysicsAir200 production lifecycle82D34388/82D346C0/82D346A0. |
| `0x82d346c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:1` | //! PhysicsAir200 production lifecycle82D34388/82D346C0/82D346A0. |
| `0x82d34a28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:112` | ///position (82D34A28 r5), not a predicted board position or skeleton root. |
| `0x82d34ab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase/board.rs:1` | //!82D34AB0 consumes the completed Air reckoning's collision normal1216. |
| `0x82d34da8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_trick.rs:3` | //! FillPhysOut is empty; PredictFutureOfDeck shares82D34DA8. |
| `0x82d34da8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:2` | //! Ground/Air/KnownAir vtable24 ->82D34DA8 ->82D2D860 predicts from the actual deck part |
| `0x82d34da8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:10` | //the same82D34DA8. Its target-following trajectory is not this deck sample. |
| `0x82d34da8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:11` | //Six grind vtables call82D2D860 directly;701 calls the82D34DA8 wrapper. |
| `0x82d34da8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:29` | // Skitching vtable 0x82327398 slot 6 is the same 0x82D34DA8. |
| `0x82d34dd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:235` | ///82D34DD8. The common post-physics coordinator immediately follows this |
| `0x82d35130` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:9` | //KnownAir ctor82D35130 installs82327210; vslot24 at82327228 is |
| `0x82d352d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:1` | //! Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880. |
| `0x82d35508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/settings.rs:1` | //! Original KnownAir82D35508/82D35F40/82D360B0/82D36650 stock reads. |
| `0x82d35920` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:1` | //! Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880. |
| `0x82d3597c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:81` | ///Only KnownAir Update82D35D30/Exit82D3597C subsequently write this flag. |
| `0x82d35998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/math.rs:94` | fn restore_velocity_geometry_82d35998(&mut self, v: V, n: V) -> RestoreVelocityGeometry { |
| `0x82d35c58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:1` | //! Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880. |
| `0x82d35d30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:81` | ///Only KnownAir Update82D35D30/Exit82D3597C subsequently write this flag. |
| `0x82d35f40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/settings.rs:1` | //! Original KnownAir82D35508/82D35F40/82D360B0/82D36650 stock reads. |
| `0x82d360b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/settings.rs:1` | //! Original KnownAir82D35508/82D35F40/82D360B0/82D36650 stock reads. |
| `0x82d36590` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:1` | //! Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880. |
| `0x82d36590` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:164` | ///82D36590: apex observation, shared air-wipeout(true), then upside-down request. |
| `0x82d36650` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/settings.rs:1` | //! Original KnownAir82D35508/82D35F40/82D360B0/82D36650 stock reads. |
| `0x82d366d0` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:343` | // DMO EntityMan ctor 0x82d366d0/Init 0x82d36748 and the CensusMan |
| `0x82d36728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:1` | //! Original82D6FD60 ->82D6F5F0 ->82D6F910, invoked by82D36728. |
| `0x82d36748` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:343` | // DMO EntityMan ctor 0x82d366d0/Init 0x82d36748 and the CensusMan |
| `0x82d36880` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:129` | //Consume current output, including KnownAir82D36880's predicted |
| `0x82d36880` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/math.rs:103` | fn transform_point_82d36880(&mut self, m: AffineTransform, p: V) -> V { |
| `0x82d36880` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/output.rs:1` | //! Fill82D36880 overwrites every field below except the explicitly retained pair. |
| `0x82d36880` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:1` | //! Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880. |
| `0x82d37538` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:1` | //! Production Ground entry82D37538 and update82D37C88/82D38800. |
| `0x82d37538` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/entry.rs:1` | //! Actual Ground entry82D37538. Numerical operations touch live board bodies; |
| `0x82d37560` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:83` | //82D37560: full SetStandard, after SetPhysicsState's possession Stop and |
| `0x82d375c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:462` | // Ground Enter clears this owner before publication, as in82D375C8. |
| `0x82d37b20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_exit.rs:1` | //!Complete PhysicsGround Exit82D37B20, on the real board and controller owners. |
| `0x82d37c50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:218` | //PhysicsGround::PreUpdate82D37C50 clears the previous push suppression |
| `0x82d37c50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:188` | ///Ground preupdate82D37C50 ->82D38018 submits82D4E250. The host computes |
| `0x82d37c88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:1` | //! Production Ground entry82D37538 and update82D37C88/82D38800. |
| `0x82d37c88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/update.rs:1` | //! Ordinary Ground82D37C88 executes persistent controller updates and actual |
| `0x82d37c88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:155` | /// 82D4E250 on that copy, so Update 82D37C88 (the world's 3-State pass) still reads the |
| `0x82d37d34` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/pumping.rs:1` | //! Actual Ground82D37D34..7EAC pumping preparation and stock bindings. |
| `0x82d37e74` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/pumping.rs:108` | /// Ground82D37E74 reads the current Processed2548 mode, not the startup mode. |
| `0x82d37f04` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:100` | /// CheckForGrabbableObject, the tail of `PhysicsGround::Update` (`0x82D37F04`), or the |
| `0x82d37f04` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:286` | //82D37F04..F10: the Update's tail checks for a grabbable object on the same |
| `0x82d37f04` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/update.rs:154` | //82D37F04/82D37F10 (CheckForGrabbableObject or InvalidateQuery) follow in ground_phase. |
| `0x82d37f10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:101` | /// owner's InvalidateQuery (`0x82D37F10`) without the intent. Without the skitching |
| `0x82d37f10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/update.rs:154` | //82D37F04/82D37F10 (CheckForGrabbableObject or InvalidateQuery) follow in ground_phase. |
| `0x82d37f38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:133` | ///Ground82D37F38 consumes the candidate query before its shared skeleton update. |
| `0x82d37f38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:195` | ///Ground82D38430 submits the candidate before82D37F38 consumes its investigation. |
| `0x82d37f38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:179` | /// The world's Post 3-State pass (PhysicsGround slot 3, 82D37F38) waits on the job and copies |
| `0x82d38000` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:176` | //Ground82D38000 completes GeneralUpdate;82D38008 then retains the actual |
| `0x82d38000` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:50` | ///Original82D38000 GroundUpdate ->82D38008 UpdatePhysicsError. Call after |
| `0x82d38008` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:176` | //Ground82D38000 completes GeneralUpdate;82D38008 then retains the actual |
| `0x82d38008` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_air.rs:50` | ///Original82D38000 GroundUpdate ->82D38008 UpdatePhysicsError. Call after |
| `0x82d38018` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:154` | /// PreUpdate 82D38018 copies the live Reckoning to PhysicsGround+896 (82D380E8) and runs |
| `0x82d38018` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:188` | ///Ground preupdate82D37C50 ->82D38018 submits82D4E250. The host computes |
| `0x82d380e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:154` | /// PreUpdate 82D38018 copies the live Reckoning to PhysicsGround+896 (82D380E8) and runs |
| `0x82d38430` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/update.rs:3` | //!82D38430 is dispatched by ground_phase through the retained Handplant owner. |
| `0x82d38430` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/update.rs:60` | //GrabWorld (2476 bit22) requests Handplant::ground_query82D38430, |
| `0x82d38430` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:195` | ///Ground82D38430 submits the candidate before82D37F38 consumes its investigation. |
| `0x82d387a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state.rs:65` | // Post vtable+40 is the same82D387A8 as Ground. The shared coordinator invokes |
| `0x82d38800` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:1` | //! Production Ground entry82D37538 and update82D37C88/82D38800. |
| `0x82d38800` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/input.rs:1` | //! UpdateSkateboard82D38800 inputs from the actual processed packet, animation |
| `0x82d38838` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:194` | fn center_of_mass_height_82d38838(&mut self) -> Result<f32, String> { |
| `0x82d389dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/mod.rs:106` | /// branch selection at82D389DC. This changes the solver's actual deck. |
| `0x82d389dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:216` | fn update_body_accumulator_82d389dc(&mut self) -> Result<(), String> { |
| `0x82d38cf4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/launch.rs:95` | ///Ground82D38CF4..D10 overrides both velocity records and byte268. |
| `0x82d38e18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:275` | fn collision_force_dot_velocity_82d38e18( |
| `0x82d3983c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:162` | //82D3983C..54 writes byte32 (reason12), value104 and increments200. |
| `0x82d39bb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:1` | //! PhysicsGround::CheckForGrabbableObject (`0x82D39BB8`) and ConsiderGrabSplineQueryResults |
| `0x82d39d8c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:121` | // 0x82D39D8C: AddDeferredGrabSplineQuery(0xFF, 5, centre of mass, box, 0.25, 60, 30 degrees). |
| `0x82d39d98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:2` | //! (`0x82D39D98`): while the world-grab intent (ProcessedPhysIn+2476 bit 22) is held on board, |
| `0x82d39e68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:151` | // where the full box's is (0x82D39E68..EC4). |
| `0x82d3a04c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_grab.rs:180` | // 0x82D3A04C..070: while the reskitch delay runs, only another vehicle may be taken. |
| `0x82d3a388` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:2` | //! Reset82DE53F0 runs before the actual producers82DB6EC0/82D3A388. |
| `0x82d3a700` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state.rs:16` | /// Enter82D3A700. Body652 points to the deck's real dynamics; its+48 is angular |
| `0x82d3a828` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state.rs:42` | /// Exit82D3A828 restores the board's cached standard wheel material8280. |
| `0x82d3a890` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:269` | /// Slide82D3A890 seeds1136/1200 from Processed528/96 and calls82D8C8F0 |
| `0x82d3a890` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state.rs:48` | /// Update82D3A890: complete Reckoning, Skeleton Ground, then Slide board. |
| `0x82d3a900` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/update.rs:1` | //! Ordered UpdateSkateboard82D3A900: steering, manual, wall launch or Slide forces. |
| `0x82d3a924` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/update.rs:19` | // Re-capture at Slide's own82D3A924 call; no tick elapses between these writes. |
| `0x82d3aa28` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:386` | /// (0x82d3aa28, 0x82c4e7b8). Under this map the two `InitCullingTable` pair tables (Skate 2 |
| `0x82d3aa74` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/mod.rs:95` | /// Slide82D3AA74..AA9C passes a fresh zero previous vector each update. |
| `0x82d3ab10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/update.rs:128` | //82D3AB10 branches directly to the epilogue. No ordinary force tail. |
| `0x82d3ab30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/settings.rs:1` | //! Direct original bindings82D94EC8..4F48,82D3AB30/AD3C/AE68 and stock schema. |
| `0x82d3aec0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/update.rs:156` | //82D3AEC0..AF10 preserves the strict scalar branches and fsel signs. |
| `0x82d3b010` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:215` | //State Fill82D3B010 is a single byte from this retained Slide owner. |
| `0x82d3b1ec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/mod.rs:29` | //Ctor82D3B1EC..B25C full64 handle hashes, verified against the stock keys. |
| `0x82d3b5e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/wipeout_output.rs:15` | // Wipeout Enter82D3B5E8 caches them, Update82D3BFEC applies deck force. |
| `0x82d3b5e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/lifecycle.rs:1` | //! Original Wipeout Enter82D3B5E8 and Exit82D3B990. |
| `0x82d3b990` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/lifecycle.rs:1` | //! Original Wipeout Enter82D3B5E8 and Exit82D3B990. |
| `0x82d3b9c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/update.rs:1` | //! Complete Wipeout300 update order82D3B9C8, on the game body/pose owners. |
| `0x82d3bfec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/wipeout_output.rs:15` | // Wipeout Enter82D3B5E8 caches them, Update82D3BFEC applies deck force. |
| `0x82d3c02c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/prediction.rs:1` | //! Original Wipeout Update82D3C02C..C288; completed query precedes next submit. |
| `0x82d3d6d0` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:359` | /// `LWDynamicObjectMan::AddStreamedLocatorData` 0x82d3d6d0: persistence and |
| `0x82d3e0cc` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:353` | // LWDynamicObjectMan::Init 0x82d3e0cc..e124, after its LWComponentMan Init. |
| `0x82d3ec08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/contact.rs:1` | //! Wipeout300's material corrections and collision-policy requests82D3EC08. |
| `0x82d3eee8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/wipeout_output.rs:1` | //! Wipeout300 Fill82D3EEE8 writes the shared completed physical packet. |
| `0x82d3eee8` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:795` | /// Skate 3 physics: PhysOut_State+68/+69 from WipeoutGround FillPhysOut 82D3EEE8 (the |
| `0x82d3ef84` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:88` | //82D3EF84 and Footplant Fill82D7112C publish that actual byte. |
| `0x82d3f210` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/state.rs:1` | //! Separate native state instances: common ctor82D3F210 and Enter82D3F318. |
| `0x82d3f318` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:1` | //! Physical side effects of Enter82D3F318/Exit82D3F430, all six families. |
| `0x82d3f318` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/state.rs:1` | //! Separate native state instances: common ctor82D3F210 and Enter82D3F318. |
| `0x82d3f430` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:1` | //! Physical side effects of Enter82D3F318/Exit82D3F430, all six families. |
| `0x82d3f448` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/phase.rs:61` | // Common Exit82D3F448 clears the retained leaving byte96. |
| `0x82d3f4e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:1` | //! Native common82D3F4E8 and Nonspecific82D42DF0 Update order. |
| `0x82d3f614` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:103` | //82D3F614 sets r4=1; steering leaf preserves it to the82D3F6E0 store. |
| `0x82d3f6e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:103` | //82D3F614 sets r4=1; steering leaf preserves it to the82D3F6E0 store. |
| `0x82d3f710` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:48` | ///82D3F710. These are actual wheel collision bytes844..847. |
| `0x82d3f7d0` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:56` | ///82D3F7D0/S2 82D85228 SetPartAngularDrag; ZIP's linear_drag write was wrong. |
| `0x82d3fd88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/forces.rs:9` | ///82D3FD88. The family owns its three strengths and position/velocity snapshot. |
| `0x82d405c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/rng.rs:26` | ///82D405C8/E0/F0: exactly three successive unsigned draws, in call order. |
| `0x82d40890` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/contact.rs:99` | //82D40890 blends from the current physical board, not the |
| `0x82d40a98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/contact.rs:100` | //preceding target;82D40A98 adds noise after that blend. |
| `0x82d40af8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:1` | //! Original82D40AF8: collision -> pop -> engagement -> derived substate. |
| `0x82d40bc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:30` | //82D40BC8 queues force15; optional velocity is prediction-only here. |
| `0x82d40c84` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:75` | //82D40C84 SetVelocity updates all seven real board parts. |
| `0x82d40cfc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:102` | //82D40CFC stores16112 and82D40D0C sets16416. No guessed trajectory. |
| `0x82d40d0c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:102` | //82D40CFC stores16112 and82D40D0C sets16416. No guessed trajectory. |
| `0x82d40d28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/skeleton.rs:1` | //!82D40D28 calls82BDDA10 with fast_blend=false only for a pop THIS update. |
| `0x82d40d60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:63` | //Virtual64:82D40D60 common (including Darkslide),82D41CE8 board, |
| `0x82d40d90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:63` | // Common UserTriggeredExit82D40D90 is processed2476 bit29. Slides' |
| `0x82d40ea0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/output.rs:1` | //! Common82D40EA0 and six family FillOut overrides. Post/selector are separate. |
| `0x82d41100` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/state.rs:58` | // Derived Enter82D41100 clears bytes240/241, not impulse224. |
| `0x82d41194` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/contact.rs:220` | // Darkslide contact82D41194..1234 alone writes classification104 here. |
| `0x82d41250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/involuntary.rs:40` | //82D41250: rail branches request10 and publish direction*136 to |
| `0x82d41320` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:77` | //82D41320 reads processed1464, returning literal820997B8 or82165A10. |
| `0x82d41788` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:64` | //82D41788 applies it only to ledges (kind2), preserving the native gate. |
| `0x82d41ce8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:63` | //Virtual64:82D40D60 common (including Darkslide),82D41CE8 board, |
| `0x82d41fb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:87` | //82D41FB0 literal8209975C; shared50-50/tip/5-O virtual60. |
| `0x82d420ec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/involuntary.rs:101` | //82D420EC/F0: r8=1 (across), r7=0 (no normal lift). |
| `0x82d421b0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/tip_nudge.rs:1` | //! S3 tipslide dropping-in nudge82D421B0, separate from common exit nudge. |
| `0x82d422a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/tip_nudge.rs:16` | //82D422A0..C4 multiplies original820996DC=45 by8206D110=deg-to-rad, |
| `0x82d42720` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:64` | //82D42720 tip. No strength clamp exists in these scalar getters. |
| `0x82d42788` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_grinds.rs:68` | /// Skate 3 physics: PhysOut_Grinds+0x146, Tipslide FillPhysOut 82D42788's third flag |
| `0x82d42948` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/involuntary.rs:65` | // Raw82D42948/4C passes computed force-across in r8 and lift in r7. |
| `0x82d42ae8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/settings.rs:85` | //82D42AE8 literal820997B8. |
| `0x82d42d80` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/phase.rs:7` | // Original82D42D80: no feet/drag changes; reset the shared spin only here. |
| `0x82d42df0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:1` | //! Native common82D3F4E8 and Nonspecific82D42DF0 Update order. |
| `0x82d42df0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/nonspecific.rs:1` | //!82D42DF0 through board helper82D42F50; caller owns final grind reckoning. |
| `0x82d42e8c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/nonspecific.rs:39` | //82D42E8C reapplies velocity AFTER animated board/skeleton update. |
| `0x82d42f50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/nonspecific.rs:1` | //!82D42DF0 through board helper82D42F50; caller owns final grind reckoning. |
| `0x82d42f50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/nonspecific.rs:54` | //82D42F50 starts with steering0 then82C04368, before angular/support. |
| `0x82d430a0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/phase.rs:71` | // Original82D430A0 only writes Air442/128 on this retained latch. |
| `0x82d431f0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:273` | //VT823272FC slot8=82D431F0; slots12/40 are original empty leaves. |
| `0x82d431f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:90` | /// Teleporting82D431F0 calls the Actor; it answers after the source tick. |
| `0x82d431f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:88` | /// Update82D431F0 calls the actor only without Processed2468 bit1. |
| `0x82d43488` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:1` | //! RevertGround102, TU3 vtable82327330: Enter82D43488, Update82D43518, |
| `0x82d43518` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:1` | //! RevertGround102, TU3 vtable82327330: Enter82D43488, Update82D43518, |
| `0x82d435fc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/pumping.rs:30` | ///Revert82D435FC..36A0 uses the processed timestep and flag2472 bit16. |
| `0x82d43800` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:54` | ///82D43800: directed remaining angle, stock speed curve and rate damping. |
| `0x82d438b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:60` | //Preserve the two native self-scaled projections82D438B4..D0. |
| `0x82d43b10` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:2` | //! empty Exit, Ground Post82D4C070, Fill82D43B10. Stock layout018E8A2E5028AB3F. |
| `0x82d442d0` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:2` | //! Enter (`0x82d442d0`), UpdateSpline (`0x82d444a0`), hand IK targets (`0x82d45d30`, |
| `0x82d442d0` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:209` | // Enter 0x82d442d0: after Reset, Pushing+0x1A0 = Skeleton+0x3E00. |
| `0x82d44300` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:207` | /// PhysState_OffBoard_Pushing::Enter 0x82d44300..44364: a board in hand (SkateboardController |
| `0x82d444a0` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:2` | //! Enter (`0x82d442d0`), UpdateSpline (`0x82d444a0`), hand IK targets (`0x82d45d30`, |
| `0x82d44a28` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:252` | // Update 0x82D44A28..AA8: a collision pose error (ProcessedPhysIn+736) past 0.2 m is an |
| `0x82d44aac` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:259` | // 0x82D44AAC..AE0: how long ProcessedPhysIn+2484 bit 26 has been set. |
| `0x82d44aec` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:315` | // Pushing::Update 0x82D44AEC..44B14 opens with the off-board feet, then keeps the hands |
| `0x82d44bf8` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:265` | // 0x82D44BF8..C28: the grab box on the effective animation transform, pitch off. |
| `0x82d44c3c` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:274` | // 0x82D44C3C..D38: the held spline, live from its object, and whether the retained one |
| `0x82d44de4` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:286` | // 0x82D44DE4..E14: the intent released, the bit-26 time past 0.4 s or an emergency exit |
| `0x82d44f58` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:324` | // 0x82D44F58..68 tests the OLD retained spline's data and body. |
| `0x82d4500c` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:381` | // Update 0x82d4500c..45088: the hand IK blend steps 0.2 toward the targets' validity. |
| `0x82d45090` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:392` | // 82d45090..45180: the follow position moves toward the push normal's point kDistComToBoard |
| `0x82d4514c` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:394` | // (ProcessedPhysIn+0xF0, the y the vrlimi at 0x82d4514c puts in), by the object's speed |
| `0x82d45184` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:430` | // 0x82D45184..451A0: Pushing+0x120, +0x464 and the new +0x1A0. |
| `0x82d45218` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:320` | // skeleton through UpdateOffBoardPushing with no motion or turn (0x82D45218..45260). |
| `0x82d45218` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:241` | // toward its follow position (0x82D45218..45260). |
| `0x82d45318` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:3` | //! `0x82d46218`, `0x82d463d8`) and MoveObject (`0x82d45318`) are Skate 2's `OffBoardPushing` with |
| `0x82d45d30` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:2` | //! Enter (`0x82d442d0`), UpdateSpline (`0x82d444a0`), hand IK targets (`0x82d45d30`, |
| `0x82d46218` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:3` | //! `0x82d46218`, `0x82d463d8`) and MoveObject (`0x82d45318`) are Skate 2's `OffBoardPushing` with |
| `0x82d463d8` | TU3 | `crates/skate2-core/src/physics/skate3_pushing.rs:3` | //! `0x82d46218`, `0x82d463d8`) and MoveObject (`0x82d45318`) are Skate 2's `OffBoardPushing` with |
| `0x82d46fd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_moving_object.rs:61` | /// The pushing state's FillPhysOut (0x82d46fd0) record the behaviour reads (PhysOut+0x48 |
| `0x82d46fd0` | TU3? | `crates/skate2-core/src/physics/skate3_player.rs:689` | // FillPhysOut of PhysState_OffBoard_Pushing (0x82d46fd0) publishes its own grab, |
| `0x82d46fd0` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:123` | /// FillPhysOut 0x82d46fd0's moving-object record: movement and force along the blended hands' |
| `0x82d46fd0` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:139` | /// The state's mGrabObject (Pushing+0x4B0 bit 5), which its FillPhysOut (`0x82d46fd0`) |
| `0x82d47278` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:1` | //! PhysState_Skitching (104), TU3 vtable 0x82327398: Enter 0x82D47278, Update 0x82D477C0 (its |
| `0x82d47278` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:542` | /// Enter 0x82D47278: Reset, DisableBoardAnimation, then the processed GrabSpline and its |
| `0x82d47318` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:1` | //! Skitch Reset82d887f0 (Skate 3 0x82d47318), with its late live attribute read. |
| `0x82d47318` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:92` | // Skate 3 Reset 0x82d47318: +0x330 zero (0x82d47760), +0x3E4 0.0 (0x82d4771c) and the |
| `0x82d4764c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:93` | // +0x541 0x10/0x08 bits cleared (0x82d4764c). |
| `0x82d4771c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:92` | // Skate 3 Reset 0x82d47318: +0x330 zero (0x82d47760), +0x3E4 0.0 (0x82d4771c) and the |
| `0x82d47760` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:92` | // Skate 3 Reset 0x82d47318: +0x330 zero (0x82d47760), +0x3E4 0.0 (0x82d4771c) and the |
| `0x82d477c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:1` | //! PhysState_Skitching (104), TU3 vtable 0x82327398: Enter 0x82D47278, Update 0x82D477C0 (its |
| `0x82d477c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:562` | /// Update 0x82D477C0. |
| `0x82d477e4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:576` | // 0x82D477E4..EC: +0x540 bit 7 is ProcessedPhysIn+2480 bit 22, the first GrabSpline |
| `0x82d477f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:90` | /// Update 0x82D477F0 disables the hand collision for five frames every tick. |
| `0x82d47810` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:646` | /// The block of Update 0x82D47810..47960 that runs with a published GrabSpline. |
| `0x82d4781c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:662` | // 0x82D4781C..B0: ProcessedPhysIn+2676 (turn) against +2672 (spin). |
| `0x82d478b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:664` | // 0x82D478B4..C0: the ground normal is ProcessedPhysIn+464. |
| `0x82d47900` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:2` | //! `sub_82D47BD8` and `sub_82D479C8`, run by Update 0x82d47900/0x82d47908 after |
| `0x82d47900` | TU3 | `crates/skate2-core/src/physics/skate3_player.rs:1572` | /// Skitching's grab manager calls (TU3 Update 0x82D47900..5C), handled by the Skate 2 world in |
| `0x82d47908` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:2` | //! `sub_82D47BD8` and `sub_82D479C8`, run by Update 0x82d47900/0x82d47908 after |
| `0x82d47918` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:719` | // 0x82D47918..5C: GrabSpline::Init, RequestGrabSplineData and the grabbing notification. |
| `0x82d47968` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:624` | // 0x82D47968..A4: the Ground job's input packet with Reckoning+1136 from ProcessedPhysIn+528 |
| `0x82d479b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:640` | // 0x82D479B0: the Skeleton's ground update. |
| `0x82d479c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:2` | //! `sub_82D47BD8` and `sub_82D479C8`, run by Update 0x82d47900/0x82d47908 after |
| `0x82d479c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:93` | /// `sub_82D479C8`. Clears +0x541 0x08; with a nonzero `Brake` record (Processed+0xAA8) and |
| `0x82d479c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:692` | // sub_82D47BD8 (PushContact, ProcessedPhysIn+2488 bit 29) and sub_82D479C8 (the Brake |
| `0x82d479fc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:102` | // 0x82d479fc beq: an unordered record also brakes. |
| `0x82d47a9c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:113` | // 0x82d47a9c ble: an unordered speed also brakes. |
| `0x82d47bd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:2` | //! `sub_82D47BD8` and `sub_82D479C8`, run by Update 0x82d47900/0x82d47908 after |
| `0x82d47bd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:25` | /// `sub_82D47BD8`. Until the PushContact record (Processed+0x9B8 0x2000_0000) latches |
| `0x82d47bd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:692` | // sub_82D47BD8 (PushContact, ProcessedPhysIn+2488 bit 29) and sub_82D479C8 (the Brake |
| `0x82d47e40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:70` | // 0x82d47e40..e60: only the Y lane of the torque is kept. |
| `0x82d47ecc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/contact.rs:81` | // 0x82d47ecc bge: an unordered timer also pushes back. |
| `0x82d47fc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:655` | // UpdateLocalPhysicalParameters 0x82D47FC0: the board's wheel height and time without |
| `0x82d4a0c0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/deck_offset.rs:1` | //! UpdateDeckAnimOffset, Skate 3 0x82d4a0c0 (Skate 2 0x82d96430). At or below the threshold no |
| `0x82d4a0c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:676` | // UpdateDeckAnimOffset 0x82D4A0C0: ProcessedPhysIn+2488 bit 23 is the PushSpeed record. |
| `0x82d4a1d4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/deck_offset.rs:23` | // 0x82d4a1d4..f4: the PushSpeed record (Processed+0x9B8 0x00800000) aims at zero. |
| `0x82d4a2b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/deck_offset.rs:29` | // 0x82d4a2b4 ble: an unordered angle still sets the offset. |
| `0x82d4a310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/deck_offset.rs:34` | // 0x82d4a310..4c: the X and Z rows keep the splatted cos/sin in W. |
| `0x82d4a350` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:92` | /// UpdateDeckAnimOffset 0x82D4A350..54: the orientation channel only, refreshed for 15 frames. |
| `0x82d4a378` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:754` | /// UpdateHandIK 0x82D4A378, then Skeleton::SetLeft/RightHandIKTargetPosition (0x82BD9728 and |
| `0x82d4ac38` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:2` | //! UpdateSkateboard 0x82D4AC38) and FillPhysOut 0x82D4C078; Exit is empty. The state's own |
| `0x82d4ac38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:791` | /// UpdateSkateboard 0x82D4AC38. |
| `0x82d4adc4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:884` | // 0x82D4ADC4..AE50: a manual's heading (ProcessedPhysIn+2720 balance and +2672 spin) |
| `0x82d4afb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:595` | // CheckForExit 0x82D4AFB8: the world-grab intent is ProcessedPhysIn+2476 bit 22. |
| `0x82d4b034` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:287` | // 0x82d4b034 bgt: an unordered delay also passes. |
| `0x82d4b064` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:296` | // 0x82d4b064 bgt: an unordered delay also reloads. |
| `0x82d4b0c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:51` | /// Skate 3 0x82d4b0c8..b124 adds the push-back force, then the push-forward force. |
| `0x82d4b500` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/forces.rs:43` | /// Skate 3 CalcSpeedForce 0x82d4b500. |
| `0x82d4b5a4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/forces.rs:64` | // 0x82d4b5a4..b604: the deck distance is first advanced one step by |
| `0x82d4b870` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/forces.rs:94` | // 0x82d4b870: ClampVectorWithinMaxLength(force, 500.0). |
| `0x82d4c070` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/revert_state.rs:2` | //! empty Exit, Ground Post82D4C070, Fill82D43B10. Stock layout018E8A2E5028AB3F. |
| `0x82d4c078` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:2` | //! UpdateSkateboard 0x82D4AC38) and FillPhysOut 0x82D4C078; Exit is empty. The state's own |
| `0x82d4c078` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:504` | /// FillPhysOut 0x82D4C078's animation and ground values. |
| `0x82d4c078` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:939` | /// FillPhysOut 0x82D4C078: the grab (OffBoard+0x130) holds while the spline is published and |
| `0x82d4c078` | TU3? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:240` | // Skitching FillPhysOut 0x82D4C078: the camera reads the grab position, the HUD and |
| `0x82d4c0c4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:512` | // 0x82D4C0C4..D4: the hands' delta times 59.999996. |
| `0x82d4c104` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:515` | // 0x82D4C104..34: relative to ProcessedPhysIn's effective position (+0xB0). |
| `0x82d4c3d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:277` | //82D4C3D8 ->82D91298: keep planted hands out of collision response. |
| `0x82d4c530` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:136` | //HandPlant vtable823273CC slot40 ->82D4C530 ->82D90358(false). |
| `0x82d4c5a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/transition.rs:165` | PhysicalStateId::FootPlant => skater.footplant.reset(), //Exit82D4C5A8 |
| `0x82d4c6d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:183` | ///82D4C6D8, after the shared plant collision check82D8FDC0. |
| `0x82d4c6d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:137` | //FootPlant's82D4C6D8 alone uses82D8FDC0. Its low trick-impact |
| `0x82d4c900` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:51` | ///82D4C900: anchor the last completed physical toe position. |
| `0x82d4c9b4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/transition.rs:166` | PhysicalStateId::Boneless => {}, //empty82D4C9B4 |
| `0x82d4c9b8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:63` | ///82D4C9B8. Authored FootJump controls every launch request; no host timer. |
| `0x82d4cb18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:78` | ///82D4CB18/82D4CC18, four PointNegGraphData8 values from the stock schema. |
| `0x82d4cb18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:104` | ///82D4CB18/82D4CC18, independent of either player's body/solver ownership. |
| `0x82d4cc18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:78` | ///82D4CB18/82D4CC18, four PointNegGraphData8 values from the stock schema. |
| `0x82d4cc18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/boneless.rs:104` | ///82D4CB18/82D4CC18, independent of either player's body/solver ownership. |
| `0x82d4d150` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime/selection.rs:1` | //! Ground mode0 consumer of the original core82D4D150 implementation. |
| `0x82d4d150` | TU3? | `crates/skate2-core/src/physics/skate3_pushing.rs:301` | // sub_82D4D150 with the held spline excluded, then CanGrabSpline from the reach base. |
| `0x82d4d418` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck.rs:1` | //! State503 lifecycle from original TU3 82D4D418..82D4E100. |
| `0x82d4d830` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck.rs:188` | ///82D4D830 resets only the shared manager; outgoing state data is not another trajectory. |
| `0x82d4df08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck.rs:175` | ///82D4DF08: synchronize actual query completion before rejection/collision tests. |
| `0x82d4dfc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck/publication.rs:1` | //! State503 FillPhysOut82D4DFC8. No output is inferred from render transforms. |
| `0x82d4e0a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:153` | //Completed State503 Fill82D4E0A8, consumed as Ground322 by the camera. |
| `0x82d4e0e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:98` | // State63 is set only by LandingOnDeckManager::Fill82D4E0E8..E0F4 |
| `0x82d4e100` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck.rs:1` | //! State503 lifecycle from original TU3 82D4D418..82D4E100. |
| `0x82d4e250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation.rs:54` | //1136/1200 seed (82D4E250): the heading is retained while the deck is animated. |
| `0x82d4e250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:39` | /// The Reckoning fields that PhysicsGround's job 82D4E250 computes on its own copy. |
| `0x82d4e250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:155` | /// 82D4E250 on that copy, so Update 82D37C88 (the world's 3-State pass) still reads the |
| `0x82d4e250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:188` | ///Ground preupdate82D37C50 ->82D38018 submits82D4E250. The host computes |
| `0x82d4e250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:201` | // Ordinary ground riding follows the physical deck (82D4E250). |
| `0x82d608a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:120` | //arc (82D608A0 takes sqrt(2*g*drop)). Keep searching; if none is |
| `0x82d61040` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:1` | //! TU3 HandPlantManager82D61040 and physical state600 (823273CC). |
| `0x82d61268` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:245` | //82D61268 clears the active output when submission begins. |
| `0x82d613b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:1` | //! Handplant Launch82D613B0, trajectory selector82D66318/66948 and COM82D625C0. |
| `0x82d61458` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:244` | ///82D61458/64 call standalone Cos/Sin. Permute mask822FBAC0 followed |
| `0x82d61b60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/rotation.rs:1` | //! CreateMtxFromHeadingAndUp82D61B60 and axis-angle frame blend82BD3150. |
| `0x82d61d88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:24` | //82D61D88: ascending, above minimum speed, slope over45deg and \|Ny\|<=.95. |
| `0x82d62408` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:164` | //82D62408: secant handles one fixed tick apart, scaled by6. |
| `0x82d625c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:1` | //! Handplant Launch82D613B0, trajectory selector82D66318/66948 and COM82D625C0. |
| `0x82d62f20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:84` | ///82D62F20 preserves the new-position bit and the stored trajectories. |
| `0x82d63280` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:112` | //82D63280 averages four authored time-warp samples. |
| `0x82d633b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/ik.rs:1` | //! HandPlantManager82D633B0 and Skeleton82BD9728/97D0, shared four-limb IK. |
| `0x82d63d40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:1` | //! PositionSelector82D63D40, velocity window82D64420 and edge clipping82D651E0. |
| `0x82d64218` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:100` | } //82D64218 |
| `0x82d64420` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:1` | //! PositionSelector82D63D40, velocity window82D64420 and edge clipping82D651E0. |
| `0x82d651e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:1` | //! PositionSelector82D63D40, velocity window82D64420 and edge clipping82D651E0. |
| `0x82d65428` | Skate 2? | `crates/skate2-core/src/session/player_mounted_recipe_load.rs:47` | /// `cAssetRegistry82d65428` constructs an empty map; `CharacterAssetManager82d14308` |
| `0x82d657d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:102` | //82D657D0/658A0 retains -depth<localZ<0. |
| `0x82d658a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:211` | //82D658A0 excludes coplanar segments and avoids dividing near-parallel Z. |
| `0x82d65bd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/contact.rs:140` | //82D65BD8: extend both ends across neighbours within .05m and30degrees. |
| `0x82d66318` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:1` | //! Handplant Launch82D613B0, trajectory selector82D66318/66948 and COM82D625C0. |
| `0x82d66b64` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:115` | //82D66B64..98/82D66BC0..BF8 evaluate the query trajectory at |
| `0x82d66bc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:115` | //82D66B64..98/82D66BC0..BF8 evaluate the query trajectory at |
| `0x82d66d70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:139` | //82D66D70 rebase the outgoing parabola to the incoming time origin. |
| `0x82d67228` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:96` | //TrajectorySelector::Reset82D67228 clears9658, preserving9657. |
| `0x82d67848` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:244` | fn update_external_player_82d67848( |
| `0x82d678dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/board.rs:93` | //Launch82D678DC stores packet at1920;128 is selector2048. Update's |
| `0x82d67948` | Skate 2? | `crates/skate2-audio/src/system.rs:174` | /// The music state `UpdateFEAudioState` 0x82d67948 hands `ChangeFEMusicState` 0x82656f78: the |
| `0x82d68800` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/mod.rs:22` | /// resolve in the world step after their launch (Update82D68800 waits on mFutureResult), so a |
| `0x82d68800` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:255` | fn commit_external_player_82d68800(&mut self) -> Result<(), String> { |
| `0x82d68800` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:67` | fn update_trajectory_selector_82d68800(&mut self) -> u8 { |
| `0x82d69d68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/grind.rs:103` | //82D69D68: retain the last real query for non-acquisition candidates. |
| `0x82d6ca58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/services.rs:134` | ///82D6CA58 copies the actual gravity vector and submits; consumption remains |
| `0x82d6ca58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_selector.rs:42` | ///82D6CA58 submits; results stay staged until consume_launch at native Sync. |
| `0x82d6cc50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_selector.rs:69` | ///82D6CC50/D020/D4D0. The current authored level has no dynamic edge |
| `0x82d6d4d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_trajectory.rs:92` | //82D6D4D0 queries the real edge AABB before entering its ledge adjustment loop. |
| `0x82d6de00` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:559` | // Sequence82d6de00 reads FROM until PlayingNewAnim; Transition reads TO. |
| `0x82d6ded0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:430` | // Sequence.SetAttributes82d6ded0: TO above, then FROM until PlayingNewAnim. |
| `0x82d6e658` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_dispatch.rs:1` | //! `DispatchBodyAnim825f1040` / `EvalCurrAnim82d6e658` and the later body ACS Sync. |
| `0x82d6e798` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_selector.rs:137` | ///82D6E798 permits both pending8492 and8499. Its destination is the same |
| `0x82d6f178` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_body_dispatch.rs:723` | /// `ChannelBlend::Eval82d6f178`; the base evaluates before the channel even |
| `0x82d6f334` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:407` | // ChannelBlend82d6f334 then34c: base first; channel always, ignoring return flags. |
| `0x82d6f440` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/footplant.rs:1` | //! FootPlantManager state, original TU3 inline ctor82DB217C and Reset82D6F440. |
| `0x82d6f440` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant.rs:101` | ///82D6F440. Does not cancel the query or erase its last delivered result, |
| `0x82d6f5f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/math.rs:1` | //! Original vector operation order from82D6FD60/82D6F5F0/82D70070. |
| `0x82d6f5f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/settings.rs:1` | //! Cached physics_footplantmanager layout used by82D6FD60/82D6F5F0/82D70070. |
| `0x82d6f5f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:1` | //! Original82D6FD60 ->82D6F5F0 ->82D6F910, invoked by82D36728. |
| `0x82d6f7a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:85` | //82D6F7A8's second pure limiter result is discarded; v125 retains direction. |
| `0x82d6f910` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:1` | //! Original82D6FD60 ->82D6F5F0 ->82D6F910, invoked by82D36728. |
| `0x82d6fa80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:175` | self.contact_time = 0.0; //82D6FA80/82D6FAC4, distinct from Reset's -1. |
| `0x82d6fac4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:175` | self.contact_time = 0.0; //82D6FA80/82D6FAC4, distinct from Reset's -1. |
| `0x82d6fbb0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:179` | ///82D6FBB0: query the real edge geometry within the native two-metre box. |
| `0x82d6fd60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/math.rs:1` | //! Original vector operation order from82D6FD60/82D6F5F0/82D70070. |
| `0x82d6fd60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/settings.rs:1` | //! Cached physics_footplantmanager layout used by82D6FD60/82D6F5F0/82D70070. |
| `0x82d6fd60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:1` | //! Original82D6FD60 ->82D6F5F0 ->82D6F910, invoked by82D36728. |
| `0x82d70070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/math.rs:1` | //! Original vector operation order from82D6FD60/82D6F5F0/82D70070. |
| `0x82d70070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/pose.rs:1` | //! Footplant82D70070 publishes real FootIK targets and temporary collision disables. |
| `0x82d70070` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/settings.rs:1` | //! Cached physics_footplantmanager layout used by82D6FD60/82D6F5F0/82D70070. |
| `0x82d704b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:1` | //! FootPlant601, vtable82327400 and manager82D704B0/82D70E40. |
| `0x82d70ce0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:106` | ///82D70CE0: authored BodyAdjustX/Z move the release point, retaining |
| `0x82d70e40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:1` | //! FootPlant601, vtable82327400 and manager82D704B0/82D70E40. |
| `0x82d71040` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant.rs:137` | /// Common ProcessOutput82DB71B0 calls82D71040 before selected state Fill. |
| `0x82d7112c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:88` | //82D3EF84 and Footplant Fill82D7112C publish that actual byte. |
| `0x82d71160` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:82` | ///288, ctor82D71160; activated289 is a separate retained flag. |
| `0x82d7116c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:80` | ///GrindAirAdjust ctor82D7116C initializes activated289=false. |
| `0x82d712e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_runtime.rs:173` | //82D712E0 clears result48/49 first and always writes adjusting290. |
| `0x82d72238` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:451` | /// `Animatable::ProcessTreeAttrs82d72238`: clear first, `GetCurrAnim`, actual pool. |
| `0x82d72b40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_air_settings.rs:1` | //! Stock GrindAirAdjust settings82D72B40; lengths from the actual deck/truck collections. |
| `0x82d73fc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:1` | //! PlayerGrabSpline82D73FC0/74270/740F8/749D0, distinct from possession. |
| `0x82d740f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:203` | ///82D740F8 consumes completion, not retained candidate existence. |
| `0x82d740f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:113` | //82DB573C ->82D740F8 consumes the one live owner's completed results. |
| `0x82d74270` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:111` | ///82D74270: old validation is consumed BEFORE new query results spawn lines. |
| `0x82d74270` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:51` | //82DB60EC calls82D74270 AFTER sticky reset and before state PreUpdate. |
| `0x82d74740` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:227` | ///82D74740, raw747B8/747F4/7481C..74874. S2 has an extra outer hit-index |
| `0x82d74888` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:126` | //82D74888 refreshes geometry by descriptor from newest results. |
| `0x82d749d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:187` | ///82D749D0 cancels spline queries/validation and clears only native readiness. |
| `0x82d749d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/trajectory.rs:1` | //! Ground ordinary tail82D749D0 cancels pending trajectory work and clears |
| `0x82d749d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:125` | //82DB93E8/93EC resets Player1840 through82D749D0, not ballistic work. |
| `0x82d74dd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:219` | //for every selected state. The constructor82D74DD8 seeds448=0; use the |
| `0x82d74fd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager.rs:3` | //! TU3 82D74FD8 creates two hand/deck drives; 82DB6150 updates possession |
| `0x82d74fd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/hand_drives.rs:1` | //! Native SkateboardController drives82D74FD8 /82D751D8 in the shared island. |
| `0x82d74fd8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/solve.rs:136` | //82D74FD8: persistent hand drives share the deck and skeleton reactions. |
| `0x82d751d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/hand_drives.rs:1` | //! Native SkateboardController drives82D74FD8 /82D751D8 in the shared island. |
| `0x82d75ac8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/ground_board.rs:5` | /// Executes the original 82D75AC8/Hold effects on the existing possession/solver owner. |
| `0x82d75ac8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/ground_board.rs:32` | //82D31D78: 82D75AC8 keeps the board animated where it rests. |
| `0x82d75ac8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager.rs:80` | ///82D75AC8: the board left on the ground, animated, while BipedBoardOnGround is set. |
| `0x82d761f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:140` | //82D761F0 reads Collision1232 + part*112 +99 (group byte1331). |
| `0x82d76d20` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:1` | //! Live bindings for TU3 SkateboardController, 82DB6150 and 82D76D20. |
| `0x82d76d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager.rs:125` | /// 82D76D20: pass the real Skeleton::GetBoneTransform(11), not a hand, |
| `0x82d76d20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:218` | //ProcessOutput82DB71C4 calls SkateboardController::FillPhysOut82D76D20 |
| `0x82d770d8` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/services.rs:10` | ///82D770D8; S2 OffBoardFeetIK::ConsiderReset82DCD110 corroborates the owner. |
| `0x82d770d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_feet.rs:79` | ///82D770D8: retain state56 across500/501/502; clear actual IK reset fields otherwise. |
| `0x82d77330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/feet.rs:1` | //! Ground-only conditioning82D77330 and history82D775D8 around the shared |
| `0x82d77330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/feet.rs:11` | //82D77330 only conditions when BOTH incoming line tests are valid. |
| `0x82d773f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/services.rs:26` | ///82D773F8 reads real line tests and82BE3220 world animation-record bones. |
| `0x82d77558` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/feet.rs:23` | //82D77558 applies IK before Ground correction/history82D775D8. |
| `0x82d775d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/feet.rs:1` | //! Ground-only conditioning82D77330 and history82D775D8 around the shared |
| `0x82d775d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/feet.rs:23` | //82D77558 applies IK before Ground correction/history82D775D8. |
| `0x82d78038` | TU3 | `crates/skate2-core/src/animation/runtime/skate3_gesture_streams.rs:118` | /// (Blend82d78038), as the native body tree's EvalBlendPose supplies them: at `weight` one |
| `0x82d785f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/lifecycle.rs:1` | //! Ground Post82D32AA0 and complete typed Fill82D32D38/82D785F8. |
| `0x82d785f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_feet.rs:97` | ///82D785F8: only these two output bytes belong to the feet service. |
| `0x82d78b28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:1` | //! Offboard host for original LandingOnDeckManager82D78B28..82D79E30. |
| `0x82d78b28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:42` | ///82D78B28 deliberately retains proposed trajectory AND pending completion. |
| `0x82d78d30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:47` | ///82D78D30. Query failure propagates without committing the speculative state. |
| `0x82d79170` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:26` | /// GetTreeAttr82d79170: first matching cached row, five packed name words. |
| `0x82d792d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:97` | ///82D792D0. mapped_position_12608 is the PRE-adjustment mapped board |
| `0x82d79420` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck/output.rs:1` | //! Complete82D79420 writes into caller-owned physical output fields. |
| `0x82d7948c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:96` | //82D7948C/98: destination is PhysOut24 collision, NOT Skeleton20. |
| `0x82d794a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:76` | ///82D794A0: returns EVERY numeric output. A returned query has already run; |
| `0x82d79948` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck/query.rs:1` | //! Real accelerated trajectory query and ordered completion of82D79948/79B40. |
| `0x82d79e18` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:261` | /// Constructor plus Clear, 82d7b6f0 / 82d79e18. Free-list entries reference |
| `0x82d79e30` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck.rs:1` | //! Offboard host for original LandingOnDeckManager82D78B28..82D79E30. |
| `0x82d7a080` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:304` | /// Constructor plus Clear, 82d7b7a8 / 82d7a080. Clear resets lists only; |
| `0x82d7a330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:103` | fn reset_player_probe_82d7a330( |
| `0x82d7a330` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:2` | //! `Update1_ProcessInput` copies it to Processed+1792 and resets it (0x82d7a330). While the shove |
| `0x82d7a440` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:1` | //! TU3 `PhysicalPlayerHiLOD` 0x82db3ce8's shove probe (0x82d7a440), the pass before |
| `0x82d7a440` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:5` | //! (0x82d7a770). Every other skater is a type 1 target first (0x82d7a440's physicals loop), with |
| `0x82d7a440` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:42` | /// Another skater the probe tests (0x82d7a440: its PhysOut system reckoning +64 and +0). |
| `0x82d7a440` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:100` | /// 0x82db3ce8 -> 0x82d7a440, before this frame's input phase copies the probe. The animation |
| `0x82d7a440` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:42` | /// TU3's shove probe (0x82d7a440): its inputs from the previous output, the host's pedestrians |
| `0x82d7a670` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:86` | /// The pedestrian query's centre (0x82d7a670: position + Z axis * reach) while the probe runs. |
| `0x82d7a770` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:5` | //! (0x82d7a770). Every other skater is a type 1 target first (0x82d7a440's physicals loop), with |
| `0x82d7a770` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:13` | /// 0x82d7a770's look-ahead time (0x82099280) for the anticipation zone. |
| `0x82d7a770` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:140` | /// 0x82d7a770: the target in the skater's frame now (strike zone) and 0.2 s ahead (anticipation |
| `0x82d7afd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings.rs:1` | //! Stock Biped settings82D7B630 and metric queries82D7AFD8/82D16680. |
| `0x82d7b1c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:99` | //82D30BD0 is a Ground-state reset, not Biped82D7B1C0. |
| `0x82d7b630` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings.rs:1` | //! Stock Biped settings82D7B630 and metric queries82D7AFD8/82D16680. |
| `0x82d7b6f0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:261` | /// Constructor plus Clear, 82d7b6f0 / 82d79e18. Free-list entries reference |
| `0x82d7b70c` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:236` | /// Empty `PoolStack` constructor at 82d7b70c..b778 / 82cc0b44..b74. |
| `0x82d7b7a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:72` | //82D7B7A0 is the common Biped placement, not Ground Enter/Update. |
| `0x82d7b7a8` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:304` | /// Constructor plus Clear, 82d7b7a8 / 82d7a080. Clear resets lists only; |
| `0x82d7b8e8` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:351` | /// Actual retained fields established by `Animatable::Animatable` 82d7b8e8. |
| `0x82d7f4bc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:285` | // Temporary first-producer trace. Native Ground movement82D7F4BC enters |
| `0x82d7f624` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:286` | // the directed branch and82D7F624 divides target distance by AnimTime; |
| `0x82d809f0` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:558` | // ChannelBlend82d809f0: channel first, regardless of pose blend coefficient. |
| `0x82d80b04` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:681` | /// `ChannelBlend::GetAttributes82d80b04..d30`: sorted union, channel wins equal names. |
| `0x82d8108c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/pre_input.rs:49` | //82D81100 clears pending316; result reset called by ctor82D8108C. |
| `0x82d81100` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/pre_input.rs:49` | //82D81100 clears pending316; result reset called by ctor82D8108C. |
| `0x82d811c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/pre_input.rs:62` | "Pending82D811C8 trajectory batch requires its complete82D81610 geometry consumer" |
| `0x82d81610` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:340` | /// 82D81610 bounds and 82C1EAD8: the grind data's static pass first, then any authored |
| `0x82d81610` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/pre_input.rs:1` | //! Original82D81610 inactive-query branch and82D2DDD0 result reset. Pending |
| `0x82d81610` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/pre_input.rs:62` | "Pending82D811C8 trajectory batch requires its complete82D81610 geometry consumer" |
| `0x82d81610` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:83` | fn update_pre_input_manager_82d81610( |
| `0x82d83cf8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:298` | /// Reference: `TrajectoryLaunchInfo::TrajectoryLaunchInfo` at `0x82d83cf8` |
| `0x82d83cf8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:322` | /// Reference: `TrajectoryLaunchInfo::TrajectoryLaunchInfo` at `0x82d83cf8`. |
| `0x82d84050` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:240` | /// `PhysState_PhysicsAir::GetComTrajectoryLaunchInfo` at `0x82d84050`. |
| `0x82d85228` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:56` | ///82D3F7D0/S2 82D85228 SetPartAngularDrag; ZIP's linear_drag write was wrong. |
| `0x82d85d08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/surface.rs:16` | /// SurfacePhysics constructor82D85D08 binds these exact five collection keys |
| `0x82d86018` | TU3 | `crates/skate2-core/src/physics/board/collision/skateboard_contact_surface.rs:280` | /// (0x82c08818 in its title update 3) and its `SurfacePhysics::Dispatch` (0x82d86018) rides |
| `0x82d86c88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:37` | ///82D86C88 passes processed2952 into82E0AAD0, not actor identity2948. |
| `0x82d87358` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/settings.rs:38` | //physics_wipeout layout256,82D87358; not trajectory grind speed. |
| `0x82d87460` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_camera.rs:2` | //! Tangent selection82D87460 belongs to the manager, not to this conditioner. |
| `0x82d875a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/history.rs:37` | ///82D875A8; accepted families RETAIN previous proximity. Only fallback |
| `0x82d87e18` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/pre.rs:121` | //82D87E18/34/3C: contact+128 and SECOND primitive+208/+224. |
| `0x82d886f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:137` | // IsDeckOffsetWithinEdgeRegion82d886f0 calls the limit helper |
| `0x82d887f0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:1` | //! Skitch Reset82d887f0 (Skate 3 0x82d47318), with its late live attribute read. |
| `0x82d887f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/reset.rs:4` | /// Reset82d887f0 is also called on each Enter. Unlike construction it |
| `0x82d88c58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:1` | //! Skitch scalar state transitions:82d88c58/82d88dc8/82d95ce0/82d9da00/82d9e6d0. |
| `0x82d88c58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:62` | ///82d88c58: both height thresholds and the wheel-contact timer are read. |
| `0x82d88dc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:1` | //! Skitch scalar state transitions:82d88c58/82d88dc8/82d95ce0/82d9da00/82d9e6d0. |
| `0x82d88dc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:74` | ///82d88dc8: opposing spin does not cancel turn; otherwise use the greater |
| `0x82d891c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/geometry.rs:175` | /// CalcFakeCentripedalAcceleration82d891c0: raw PPC fixes the decompiler's |
| `0x82d8939c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/pre.rs:92` | //82D8939C/A0 writes both truck hits. FiveO82D89CB8/9D5C |
| `0x82d89cb8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/pre.rs:92` | //82D8939C/A0 writes both truck hits. FiveO82D89CB8/9D5C |
| `0x82d89dc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_materials.rs:1` | //!82D89DC0/82C090C0 material modes, applied to the live board collider slots. |
| `0x82d89dc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/history.rs:67` | ///82D89DC0: use the PRECEDING KnownAir target on ground, then update it. |
| `0x82d89dc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind.rs:58` | ///82D89DC0 leaves materials and its air-target history untouched. |
| `0x82d89dc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind.rs:60` | ///Restore the initialized standard material triplet (82D89DC0). |
| `0x82d8a638` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind.rs:169` | ///82D8A638 resets the manager, not the separately constructed child objects. |
| `0x82d8a648` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:878` | /// Reference: `PredictionResults::GetCollisionPosition` at `0x82d8a648`. |
| `0x82d8a828` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/history.rs:4` | ///82D8A828/82D8ABD8; preserved current permission/counter arithmetic. |
| `0x82d8a828` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/pre.rs:5` | ///82D8A828: permission -> bounded static query -> materials -> investigator |
| `0x82d8a828` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind.rs:1` | //! Host manager82D8A828/82D8AB08, using CURRENT player-input ownership. |
| `0x82d8a828` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:187` | fn update_grind_manager_82d8a828( |
| `0x82d8a8a0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1780` | /// Reference: `0x82d8a8a0..0x82d8a9cc`. All calculations intentionally run |
| `0x82d8a96c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1680` | /// `0x82d8a96c..0x82d8a9bc`. |
| `0x82d8a9bc` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1680` | /// `0x82d8a96c..0x82d8a9bc`. |
| `0x82d8a9c4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1773` | /// Reference: the `SetLinearDrag` call at `0x82d8a9c4`. |
| `0x82d8a9cc` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1780` | /// Reference: `0x82d8a8a0..0x82d8a9cc`. All calculations intentionally run |
| `0x82d8aa54` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/pre.rs:139` | //82D8AA54..82D8AAF8 publishes previous velocity and friction before |
| `0x82d8aaf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/pre.rs:139` | //82D8AA54..82D8AAF8 publishes previous velocity and friction before |
| `0x82d8ab08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/post.rs:5` | ///82D8AB08's meaningful order. Geometry consumes the pre-submitted hits; |
| `0x82d8ab08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind.rs:1` | //! Host manager82D8A828/82D8AB08, using CURRENT player-input ownership. |
| `0x82d8ab08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind.rs:43` | ///Fresh physical/animation values read by82D8AB08 after intervening producers. |
| `0x82d8ab08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:51` | fn update_grind_manager_82d8ab08(&mut self) { |
| `0x82d8abd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:76` | //82D8ABD8 reads the state selector's retained signed counter+40, |
| `0x82d8abd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/history.rs:4` | ///82D8A828/82D8ABD8; preserved current permission/counter arithmetic. |
| `0x82d8acf0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/observation.rs:40` | /// S3 gravity-relief producer82D8ACF0; NOT UpdateFrictionVsTime. |
| `0x82d8ade8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/selection.rs:1` | //! CalcSuggestedState82D8ADE8 gets actual contacts, line distances and pose. |
| `0x82d8af08` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:233` | /// `PhysState_PhysicsAir::CalcVelFromJump(int)` at `0x82d8af08`. |
| `0x82d8af08` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1836` | /// `0x82d8af08`. |
| `0x82d8af78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:77` | //before this tick's CalcSuggestedState advances it at82D8AF78..94. |
| `0x82d8bd50` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:64` | //82D8BD50 reads Globals260; its layout392/396/400 is physics_airstates. |
| `0x82d8be38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:40` | //Original82D8BE38 copies layouts160,240,320,480,400,80,0. |
| `0x82d8bf58` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/spin_trace.rs:5` | //! - BodySpin (TU3 0x82D8BF58) state: `in` (+120), `f` (+128 flick this tick), `sp` (+140 speed), |
| `0x82d8c8f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation.rs:53` | //82D33DB8 calls UpdateGroundStatesFixedHeading82D8C8F0 without the Ground job's |
| `0x82d8c8f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:269` | /// Slide82D3A890 seeds1136/1200 from Processed528/96 and calls82D8C8F0 |
| `0x82d8da84` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air.rs:41` | //82D8DA84 binds physics_reckoning handle1600, layout pointer1604. |
| `0x82d8dbd8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning.rs:1` | //! Production stock settings and live-input adapter for Reckoning82D8DBD8. |
| `0x82d8e3e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/update.rs:123` | //82BDDFFC..82BDE010 calls the SAME82D8E3E0 as Ground. |
| `0x82d8e3e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground.rs:38` | ///Only the shared82D8E3E0 callee, not Ground movement/lifecycle. |
| `0x82d8e3e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground.rs:27` | ///Terminal82D8E3E0 arguments, emitted after GeneralUpdate and trajectory clear. |
| `0x82d8e5c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:270` | /// directly. Its data block is exactly the Processed snapshot82D8E5C0. |
| `0x82d8e790` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/ground_input.rs:19` | //82D8E790: Processed464 -> input720. |
| `0x82d8e79c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/ground_input.rs:21` | //82D8E79C: Processed528 -> input736. Board112 is a retained |
| `0x82d8e7c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/ground_input.rs:24` | speed: input.scalar_2652,          //82D8E7C0 -> input772. |
| `0x82d8e7d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/ground_input.rs:25` | absolute_speed: input.scalar_2616, //82D8E7D8 -> input780. |
| `0x82d8e924` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/ground_input.rs:26` | wheel_count: input.wheel_count_2556 as i32, //82D8E924 -> input804. |
| `0x82d8e9e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/flip.rs:1` | //! Original Reckoning ResetBodyFlip82D8E9E0 / BeginBodyFlip82D8EAB0. |
| `0x82d8eab0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/flip.rs:1` | //! Original Reckoning ResetBodyFlip82D8E9E0 / BeginBodyFlip82D8EAB0. |
| `0x82d8f9e0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:1` | //! Shared Ground/Air wipeout requests from original TU3 82D8F9E0/82D90358. |
| `0x82d8fdc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:183` | ///82D4C6D8, after the shared plant collision check82D8FDC0. |
| `0x82d8fdc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:137` | //FootPlant's82D4C6D8 alone uses82D8FDC0. Its low trick-impact |
| `0x82d8fee8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/settings.rs:1` | //! Stock physics_wipeout fields read by82D8FEE8/82D90000 only. |
| `0x82d90000` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/settings.rs:1` | //! Stock physics_wipeout fields read by82D8FEE8/82D90000 only. |
| `0x82d90148` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/collision_settings.rs:1` | //! Stock physics_wipeout fields consumed by Biped collision check82D90148. |
| `0x82d90358` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:1` | //! Shared Ground/Air wipeout requests from original TU3 82D8F9E0/82D90358. |
| `0x82d90358` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:136` | //HandPlant vtable823273CC slot40 ->82D4C530 ->82D90358(false). |
| `0x82d90898` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/settings.rs:46` | // Original82D90898 loads164/168. Stock schema proves Ground |
| `0x82d90d40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout/settings.rs:19` | //82D90D40..94 constructs full64 D978550D6DB4E6F7. |
| `0x82d911a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_controller.rs:2` | //! Requests82D911A8 and ascending/falling update82D91660 precede the solve. |
| `0x82d911d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_controller.rs:48` | //82D911D8..1264: unsigned requested-4 indexes the eight cases. |
| `0x82d91298` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/handplant.rs:277` | //82D4C3D8 ->82D91298: keep planted hands out of collision response. |
| `0x82d91330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:14` | //82D91330 sets the foot/toe volume group and clears their disable counters. |
| `0x82d91330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/lifecycle.rs:16` | //82D91330 disables these four volumes permanently (counter0), not the |
| `0x82d913c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll.rs:1` | //! Physical-owner adapter for selector82D913C0 requests8..11 and82BE7280. |
| `0x82d91660` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_controller.rs:2` | //! Requests82D911A8 and ascending/falling update82D91660 precede the solve. |
| `0x82d91660` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_controller.rs:70` | ///Original82D91660 changes the collision request only when byte18 changes. |
| `0x82d92440` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching.rs:836` | // sub_82D92440 with no push or damped-turn retention, then (on the ground) the speed |
| `0x82d933f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/pumping.rs:26` | /// Selected physics_mode layout+8, consumed by CalcPumpForce82D933F0. |
| `0x82d93df0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:203` | fn contact_response_82d93df0( |
| `0x82d944e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate.rs:115` | ///Shared82D944E8 caller for common grind and Nonspecific. The late-false |
| `0x82d944e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/mod.rs:112` | /// Native82D944E8 output publication. Do not discard an existing ground |
| `0x82d944e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/services.rs:267` | fn collision_force_82d944e8( |
| `0x82d94760` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/geometry.rs:1` | //! UpdateGrabSpline82d94760: retained current/future frames and chord geometry. |
| `0x82d94c10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/settings.rs:1` | //!82D94C10 copies physics_jump layout0..456 into the board toolkit cache. |
| `0x82d94c10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:2` | //! Toolkit copies follow82D94C10; selected surface/mode fields stay explicit. |
| `0x82d94ec8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/settings.rs:1` | //! Direct original bindings82D94EC8..4F48,82D3AB30/AD3C/AE68 and stock schema. |
| `0x82d94f80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:165` | //Lookup8 DF474D0A659A7AE3, direct82D94F80..FC0 binding. |
| `0x82d950a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/geometry.rs:133` | //82d950a0..b4 uses last-current, not current-last. Preserve its sign. |
| `0x82d95330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/geometry.rs:158` | //82d95330 loads this+0x230: AimSpace.zAxis, NOT the curve delta |
| `0x82d95468` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/follow.rs:1` | //! UpdateFollowPoint82d95468. Original fixed-step grip/centrifugal controller. |
| `0x82d95ce0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:1` | //! Skitch scalar state transitions:82d88c58/82d88dc8/82d95ce0/82d9da00/82d9e6d0. |
| `0x82d95ce0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:98` | ///82d95ce0. Keep before UpdateFollowPoint and UpdateSkitchState. |
| `0x82d960e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/hands.rs:1` | //! UpdateHands82d960e0, preserving the source's scalar FMA boundaries. |
| `0x82d96430` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/deck_offset.rs:1` | //! UpdateDeckAnimOffset, Skate 3 0x82d4a0c0 (Skate 2 0x82d96430). At or below the threshold no |
| `0x82d966a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/forces.rs:1` | //! Skitch speed/release force and heading helpers82d966a8..97088/82d9e8d8. |
| `0x82d98d7c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2109` | /// `0x82d98d7c`. |
| `0x82d99010` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:254` | /// Reference: `0x82d99010..0x82d990b4`. The signed frame gate uses strict |
| `0x82d99010` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:299` | /// and `PhysState_PhysicsAir::LaunchTrajectorySelector` at `0x82d99010`. |
| `0x82d99050` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:230` | /// Live `kPhysicsAirJumpFixFrames` value read at `0x82d99050`. |
| `0x82d990b4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:254` | /// Reference: `0x82d99010..0x82d990b4`. The signed frame gate uses strict |
| `0x82d990b8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1846` | /// `0x82d990b8`, specifically `0x82d990dc..0x82d9919c`. Steering is always |
| `0x82d990dc` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1846` | /// `0x82d990b8`, specifically `0x82d990dc..0x82d9919c`. Steering is always |
| `0x82d9919c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1846` | /// `0x82d990b8`, specifically `0x82d990dc..0x82d9919c`. Steering is always |
| `0x82d9cd60` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/board.rs:16` | //BodySetLinearDrag82D9CD60 multiplies by native59.999996, for all seven parts. |
| `0x82d9cff8` | Skate 2 | `crates/skate2-core/src/physics/skate3_pushing.rs:377` | // The object hears it is grabbed, by the local player (Skate 2's Update82d9cff8 order). |
| `0x82d9da00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:1` | //! Skitch scalar state transitions:82d88c58/82d88dc8/82d95ce0/82d9da00/82d9e6d0. |
| `0x82d9da00` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:190` | ///82d9da00, including the hand update and fixed-step clocks. |
| `0x82d9dc58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/ik.rs:1` | //! UpdateHandIK82d9dc58. Uses existing Skeleton hand-target services at the caller. |
| `0x82d9e6d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:1` | //! Skitch scalar state transitions:82d88c58/82d88dc8/82d95ce0/82d9da00/82d9e6d0. |
| `0x82d9e6d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/state.rs:271` | ///82d9e6d0 through the force selection. Caller applies the returned force |
| `0x82d9e8d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skitching/forces.rs:1` | //! Skitch speed/release force and heading helpers82d966a8..97088/82d9e8d8. |
| `0x82d9f4b0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2016` | /// Reference: `0x82d9f4b0..0x82d9f824`. The wrapper preserves the native |
| `0x82d9f4cc` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1913` | /// `SkeletonController::UpdateAirCollision` at `0x82d9f4cc`. |
| `0x82d9f4d4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2025` | // addic./bgt at 0x82d9f4d4: title code runs in 32-bit mode, so CR0 compares |
| `0x82d9f514` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1925` | /// `PhysState_PhysicsAir::LaunchTrajectorySelector` at `0x82d9f514`. |
| `0x82d9f524` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1930` | /// `TrajectorySelector::Update` at `0x82d9f524`. |
| `0x82d9f528` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1933` | /// Snapshots `Reckoning::mUpVector` (`+0x1b0`) at `0x82d9f528..0x82d9f544`. |
| `0x82d9f544` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1933` | /// Snapshots `Reckoning::mUpVector` (`+0x1b0`) at `0x82d9f528..0x82d9f544`. |
| `0x82d9f548` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1936` | /// `TrajectorySelector::GetSuggestedLandingNormal` at `0x82d9f548`. |
| `0x82d9f568` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1940` | /// `0x82d9f568..0x82d9f5d4`. Its VMX rounding remains unrecovered. |
| `0x82d9f580` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1878` | /// `c052d34b6b052f44` at `0x82d9f580..0x82d9f600`. |
| `0x82d9f5d4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1940` | /// `0x82d9f568..0x82d9f5d4`. Its VMX rounding remains unrecovered. |
| `0x82d9f600` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1878` | /// `c052d34b6b052f44` at `0x82d9f580..0x82d9f600`. |
| `0x82d9f60c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1882` | /// `724c206c94a73043` at `0x82d9f60c..0x82d9f63c`. |
| `0x82d9f63c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1882` | /// `724c206c94a73043` at `0x82d9f60c..0x82d9f63c`. |
| `0x82d9f670` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1959` | /// Reads `kbDrawPhysicsAirDebugInfo` at `0x82d9f670..0x82d9f684`. |
| `0x82d9f684` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1959` | /// Reads `kbDrawPhysicsAirDebugInfo` at `0x82d9f670..0x82d9f684`. |
| `0x82d9f698` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1965` | /// Represents the debug-build invariant failure at `0x82d9f698..0x82d9f6ac`. |
| `0x82d9f6ac` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1965` | /// Represents the debug-build invariant failure at `0x82d9f698..0x82d9f6ac`. |
| `0x82d9f6b8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1968` | /// `PhysState_PhysicsAir::CalculateBodySpinSpeed` at `0x82d9f6b8`. |
| `0x82d9f6d0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1971` | /// `Reckoning::UpdateAirStates` at `0x82d9f6d0`. |
| `0x82d9f6d4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1886` | /// `fd9ce5e1f36ccf2e` at `0x82d9f6d4..0x82d9f704`. |
| `0x82d9f6d4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1981` | /// `fd9ce5e1f36ccf2e` at `0x82d9f6d4..0x82d9f704`. |
| `0x82d9f704` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1886` | /// `fd9ce5e1f36ccf2e` at `0x82d9f6d4..0x82d9f704`. |
| `0x82d9f704` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1981` | /// `fd9ce5e1f36ccf2e` at `0x82d9f6d4..0x82d9f704`. |
| `0x82d9f714` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1984` | /// `PhysState_PhysicsAir::AddCOMCorrection` at `0x82d9f714`. |
| `0x82d9f724` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1991` | /// The VMX multiply/add order at `0x82d9f724..0x82d9f7a0` remains opaque. |
| `0x82d9f734` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1874` | /// trajectory at `0x82d9f734..0x82d9f79c`. |
| `0x82d9f79c` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1874` | /// trajectory at `0x82d9f734..0x82d9f79c`. |
| `0x82d9f7a0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1991` | /// The VMX multiply/add order at `0x82d9f724..0x82d9f7a0` remains opaque. |
| `0x82d9f7b4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1994` | /// `Skeleton::UpdateKnownAir` at `0x82d9f7b4`. |
| `0x82d9f7c8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1997` | /// `Skeleton::UpdateAnimatedSkateboard` at `0x82d9f7c8`. |
| `0x82d9f7d0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2001` | /// `PhysState_PhysicsAir::UpdateSkateboard` wrapper at `0x82d9f7d0`. |
| `0x82d9f824` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:2016` | /// Reference: `0x82d9f4b0..0x82d9f824`. The wrapper preserves the native |
| `0x82da1b98` | Skate 2? | `crates/skate2-audio/src/system.rs:1023` | /// `PlayMissionSpeech` 0x82504ed0 (+0x3C, `ChallengeAudioManager::PlaySpeech` 0x82da1b98): |
| `0x82da1c48` | Skate 2? | `crates/skate2-audio/src/system.rs:1056` | /// 0x82da1c48): a loaded contest (`mContestId`) resets the emitters' contest state; |
| `0x82da32f0` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_finish.rs:103` | // TU3 RewardAirSequence82DA32F0..3304 applies the authored clean |
| `0x82da3438` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:273` | // TU3 ScoreModule::Update 0x82da3438 reads PhysOut_State +0x4E (flag 78) as |
| `0x82da3520` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_update.rs:73` | // TU3 ScoreModule::Update 82DA3520: with the per-tick holder clears, the air output's |
| `0x82da3de4` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:280` | // (CalculateScoreCollectorType 0x82da3de4), IsSafeToUseSessionMarker and cMsgWipeout read it. |
| `0x82da3fdc` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_grinds.rs:69` | /// (state +0x63). TU3 CalculateScoreCollectorType 82DA3FDC runs no Grind collector while set. |
| `0x82da5f4c` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:133` | // TU3 82DA5F4C: not recorded while ScoreHolder+0x1C5D is set. |
| `0x82da6034` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:154` | // TU3 82DA6034: the sequence skips it while ScoreHolder+0x1C5D is set. |
| `0x82da6198` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:77` | // TU3 EndTrickGeneric 82DA6198: no history while ScoreHolder+0x1C5D is set; the |
| `0x82da629c` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:98` | // TU3 EndAirTrick 82DA629C: nothing is banked while ScoreHolder+0x1C5D is set. |
| `0x82da7888` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:436` | // World8275F430 -> AirCollector82DA7888 publishes the complete mask. |
| `0x82da78d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:157` | //AirCollector82DA78D0 publishes the world conditioner capability mask. |
| `0x82da81e4` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:113` | // TU3 AirCollector82DA81E4 projects velocity against reckoning |
| `0x82da83b0` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_update.rs:142` | // TU3 FillAirPeriodData 82DA83B0 tests the filtered state for a wipeout. |
| `0x82da8a70` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_update.rs:24` | // TU3 Update2_TrickAnalysis 82DA8A70: while the air output's trajectory is locked |
| `0x82da8aa4` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_air_collector.rs:85` | /// TU3 AirCollector byte set by each Update2 (82DA8AA4): the air output's locked trajectory. |
| `0x82da8be0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime/rotation.rs:1` | //! Skate 3 TU3 82DA8BE0, 82DAC880 and 82DAC9D0. |
| `0x82da8be0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime.rs:1` | //! TU3 RotationTracker (82DAC880/82DAC9D0), as AirCollector::UpdateSpinMetric 82DA8BE0 drives it. |
| `0x82da8e88` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:85` | // TU3 82DA8E88 scales by the in-air scalar, then by the repeat penalty. |
| `0x82da8e90` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:98` | // TU3 UpdateSpinMetric 82DA8E90: held while ScoreHolder+0x1C5D is set. |
| `0x82da8fd8` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:136` | // TU3 UpdateFlipMetric 82DA8FD8: held while ScoreHolder+0x1C5D is set. |
| `0x82da9160` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:173` | // TU3 UpdateTrajectoryMetrics 82DA9160/917C: both held while ScoreHolder+0x1C5D is set. |
| `0x82da976c` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_tricks.rs:194` | // TU3 UpdateTricks 82DA976C: no hold or hippy-flip points while the air trajectory is |
| `0x82da9890` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_tricks.rs:226` | // TU3 82DA9890: held while ScoreHolder+0x1C5D is set. |
| `0x82da992c` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_output.rs:13` | // TU3 UpdateScoreThresholds 82DA992C: nothing while the trajectory is locked. |
| `0x82da9e34` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/database.rs:59` | // scoring_trick record (82DA9E34). |
| `0x82da9fd8` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/grind.rs:133` | // TU3 82DA9FD8: held while ScoreHolder+0x1C5D is set. |
| `0x82daa840` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:396` | // TU3 82DAA840: held while ScoreHolder+0x1C5D is set. |
| `0x82daa8e0` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:453` | // TU3 UpdateManual 82DAA8E0 has no speed condition: a slow manual scores. |
| `0x82daaafc` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:498` | // TU3 82DAAAFC: held while ScoreHolder+0x1C5D is set. |
| `0x82daac10` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:543` | // UpdateRevert 0x82dfa598 (TU3 82DAAC10) starts a revert whatever side was last seen (an |
| `0x82dab284` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:694` | // TU3 82DAB284: held while ScoreHolder+0x1C5D is set. |
| `0x82dabe98` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/offboard.rs:27` | /// TU3 82DABE98: a hippy jump starts at a fifth of its points unless the system moves |
| `0x82dac068` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/offboard.rs:61` | // TU3 UpdateHippyJumps 82DAC068: the count is the physics' landing half turns |
| `0x82dac880` | TU3 | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime/rotation.rs:1` | //! Skate 3 TU3 82DA8BE0, 82DAC880 and 82DAC9D0. |
| `0x82dac880` | TU3 | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime.rs:1` | //! TU3 RotationTracker (82DAC880/82DAC9D0), as AirCollector::UpdateSpinMetric 82DA8BE0 drives it. |
| `0x82dac9d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime/rotation.rs:1` | //! Skate 3 TU3 82DA8BE0, 82DAC880 and 82DAC9D0. |
| `0x82dac9d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime.rs:1` | //! TU3 RotationTracker (82DAC880/82DAC9D0), as AirCollector::UpdateSpinMetric 82DA8BE0 drives it. |
| `0x82dacb20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/scoring_runtime/rotation.rs:35` | // 82DACB20 rejects a discontinuity beyond a quarter turn per tick. |
| `0x82dad0a8` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:152` | /// Reset 82DAD0A8: Skate 2's reset set, without touching the wipeout flag, plus the TU3 |
| `0x82dad528` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:582` | /// 82DAD528: entity hits in tweak 2, tweak 1's hold, and tweaks 3 and 0 in whole spins. |
| `0x82dad640` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:441` | /// 82DAD640: the window scores rotation on its own curve (+2260), the wipeout on +2900. |
| `0x82dad728` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:515` | /// UpdateBodyTweaks 82DAD728: a held tweak also gathers its time and rotation. |
| `0x82dada30` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:592` | /// OutputBailData 82DADC48 into the shared bail record; bonus 82DADA30. |
| `0x82dadb48` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:172` | /// 82DADB48: heavy landing foot damage on entering the wipeout. |
| `0x82dadc48` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:592` | /// OutputBailData 82DADC48 into the shared bail record; bonus 82DADA30. |
| `0x82dae4f0` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:1` | //! TU3 Hall of Meat for Skate 3 physics: WipeoutScorer::Update 82DAE4F0 and the TU3 helpers |
| `0x82dae4f0` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:78` | /// Update 82DAE4F0. |
| `0x82dae6f0` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:185` | /// DamageBone 82DAE6F0: a decaying running total per bone, the damage its maximum. |
| `0x82dae7a8` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:216` | /// PreUpdateBones 82DAE7A8 and UpdateBones 82DAE9A8 (CheckForDamage inlined: contact force |
| `0x82dae9a8` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:216` | /// PreUpdateBones 82DAE7A8 and UpdateBones 82DAE9A8 (CheckForDamage inlined: contact force |
| `0x82daebc0` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:491` | /// EndAir 82DAEBC0. |
| `0x82daecb8` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:466` | /// 82DAECB8: apex, airtime and drop height scores, then the body tweaks. |
| `0x82daedc8` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:358` | /// 82DAEDC8: heavy landing memory, then the pre-wipeout window's run-up tracking. |
| `0x82daf1d8` | TU3? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_skate3.rs:250` | /// 82DAF1D8. |
| `0x82daf6d4` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:472` | /// Meat sums (UpdateInWipeout 82DAF6D4). |
| `0x82db0588` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/timed.rs:16` | /// TU3 TimedArcLengthScorable::Update 82DB0588: the step's length, or for a manual its |
| `0x82db18a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/initial.rs:1` | //! Original82DB18A0 initialization of the represented input owner. Native |
| `0x82db18a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:2` | //! Board construction precedes Skeleton construction in82DB18A0. Preserve |
| `0x82db19c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/initial.rs:12` | //82DB19C0: the external packet's ninth vector is the -1 sentinel. |
| `0x82db1a9c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/initial.rs:7` | //82DB1A9C inserts E0080000; active human actor adds bit18 at307C. |
| `0x82db1aac` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:21` | //Original Player ctor82DB1AAC=1000,1ACC=0,3028 clears1264. |
| `0x82db1b08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:20` | //Player ctor82DB1B08..1B78 installs these exact physics_mode keys. |
| `0x82db1b08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:33` | //Original82DB1B08..1B78 initializes these variant slots in this order. |
| `0x82db1b08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/initial.rs:18` | //Original full64 keys at82DB1B08..1B78; stock XML layout+60. |
| `0x82db1e88` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_input.rs:69` | // AnimOut construction82DB1E88..1EFC initializes the null name, |
| `0x82db1f3c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve.rs:49` | // Skateboard 82DB1F3C precedes Skeleton::Init 82DB2080) share one job and its |
| `0x82db2080` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve.rs:49` | // Skateboard 82DB1F3C precedes Skeleton::Init 82DB2080) share one job and its |
| `0x82db217c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/footplant.rs:1` | //! FootPlantManager state, original TU3 inline ctor82DB217C and Reset82D6F440. |
| `0x82db2498` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_controller.rs:15` | //Original inline constructor82DB2498..24B0. |
| `0x82db3008` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:60` | //Ctor82DB3008 enters Ground without ProcessOutput. Keep the constructed |
| `0x82db3008` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:52` | //82DB3008 selects the owned Sleeping object before SetPhysicsState100. |
| `0x82db3024` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/wipeout.rs:32` | state.initialize_player(); //Player82DB3024, after the component ctor. |
| `0x82db3c78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:27` | ///82DB3C78 at the early fixed-step phase, before current ProcessInput. |
| `0x82db3c78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/biped_air.rs:29` | ///slot16 atA0D0. Actual vtable82328688+16 points to82DB3C78. |
| `0x82db3c78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:67` | //Player slot16=82DB3C78 (vtable82328688). Consume with retained input |
| `0x82db3c78` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_selector.rs:179` | ///82DB3C78: clear ready before launch/requery consumption, then rearm. |
| `0x82db3ce8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:1` | //! TU3 `PhysicalPlayerHiLOD` 0x82db3ce8's shove probe (0x82d7a440), the pass before |
| `0x82db3ce8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:100` | /// 0x82db3ce8 -> 0x82d7a440, before this frame's input phase copies the probe. The animation |
| `0x82db3ce8` | TU3 | `crates/skate2-core/src/session/selected_player_tick.rs:952` | // TU3's shove: the probe's pedestrian query before the source tick (0x82db3ce8). |
| `0x82db4048` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:1` | //! Host ownership for the original 82DB4048 input phase. Every numerical |
| `0x82db4094` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:53` | //82DB4094 completes the preceding off-board batch BEFORE input reset. |
| `0x82db409c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:55` | //82DB409C clears the actual shared manager latch;82DB4348 publishes its IK offset. |
| `0x82db4330` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:35` | // TU3 82DB4330 calls 825903C8: 82590414 reads action-table +0x104 |
| `0x82db4348` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:55` | //82DB409C clears the actual shared manager latch;82DB4348 publishes its IK offset. |
| `0x82db4ccc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/wipeout_output.rs:14` | // ProcessInput82DB4CCC/4CDC publishes bit2472.11 and vector816; existing |
| `0x82db5588` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:1` | //! Original PostInput82DB5588 with the live ballistic trajectory selector. |
| `0x82db5588` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:150` | //82DB5588 consumes the preceding state's actual jump publication. |
| `0x82db56b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:68` | //82DB56B0 runs the real selector after GrindManager. That manager |
| `0x82db573c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:145` | //Complete preceding-state submissions before PostInput82DB573C publishes |
| `0x82db573c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:113` | //82DB573C ->82D740F8 consumes the one live owner's completed results. |
| `0x82db5be0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase_packet.rs:162` | ///Actor825926F8 reply ->10704/10768/10784, then82DB5BE0 maps physical input. |
| `0x82db5be0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:129` | // 0x825926f8, no adjust) a marker target takes; 82593640 and 82DB5BE0 copy it raw to the |
| `0x82db5e10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/post_input.rs:84` | fn calculate_scalar_2740_82db5e10(&mut self) -> f32 { |
| `0x82db5e10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:144` | ///Original82DB5E10 reads current Processed464.y and2612 after Skeleton. |
| `0x82db5f78` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:627` | /// Reference: `TrajectorySelector::Reset` at `0x82db5f78`. |
| `0x82db5ff4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/selection.rs:61` | //82DB5FF4 restores the actual counter also used by ground-history. |
| `0x82db6000` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/selection.rs:65` | //SystemLogic82DB6000 clears the per-frame Wipeout request accumulator. |
| `0x82db6050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:1` | //! Update3_State82DB6050 prefix through grab-spline query synchronization. |
| `0x82db6050` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:147` | ///Original82DB6050 prefix; coordinator calls the selected state pre-update next. |
| `0x82db60dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:49` | //82DB60DC clears sticky3184, not this-update3185. |
| `0x82db60ec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:147` | //yet, preserving next-tick visibility and later PreState82DB60EC consumption. |
| `0x82db60ec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/pre_state.rs:51` | //82DB60EC calls82D74270 AFTER sticky reset and before state PreUpdate. |
| `0x82db6120` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:286` | // Source State82DB6120 submits the queue once, then advances state time. |
| `0x82db6150` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:294` | //82DB6150: update the real possession owner after state movement, before solve. |
| `0x82db6150` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:1` | //! Live bindings for TU3 SkateboardController, 82DB6150 and 82D76D20. |
| `0x82db6150` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:188` | /// before state_timer_1344 and solve. TU3 82DB6150..61B0. |
| `0x82db6150` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager.rs:3` | //! TU3 82D74FD8 creates two hand/deck drives; 82DB6150 updates possession |
| `0x82db6310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/probes.rs:61` | /// Its saved endpoints and enabled291 feed the NEXT StartBoard82DB6310. |
| `0x82db6310` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs.rs:316` | /// StartSkateboardLineTests82DB6310 ->82C07788: four current wheel positions, |
| `0x82db63c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:1` | //! PhysicalPlayerHiLOD StartSkeletonLineTests82DB63C0 / result publication |
| `0x82db63c0` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:349` | // TU3 Start/EndSkeletonLineTests82DB63C0/6580 and board probe82C07788 |
| `0x82db63ec` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:64` | //82DB63EC..647C selects hips23,lefttoe15,righttoe19 from the |
| `0x82db6580` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:2` | //!82DB6580. These are radius-bearing trajectory tests from actual physical parts. |
| `0x82db6580` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:46` | /// EndSkeletonLineTests82DB6580 publishes the completed tests in the |
| `0x82db65e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:18` | /// Result0 and32, copied to the physical query record by82DB65E0/F0. |
| `0x82db6698` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:1` | //! Production adapter for the original physical-pose job82DB6698/82DB95E8. |
| `0x82db6db8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/post_physics.rs:1` | //! Offboard state slot40 at82DB6DCC, after Skeleton feedback82DB6DB8 and |
| `0x82db6dcc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/post_physics.rs:1` | //! Offboard state slot40 at82DB6DCC, after Skeleton feedback82DB6DB8 and |
| `0x82db6ddc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/render_pose.rs:13` | //82DB6DDC/6DF8 passes IsWipeout82DB9100 after state postphysics, not the |
| `0x82db6df8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/post_physics.rs:2` | //! before PostWipeoutCheck82DB6DF8 consumes the same request owner. |
| `0x82db6e78` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:830` | /// and `tTrajectory::AdjustTrajectory` at `0x82db6e78`. |
| `0x82db6e78` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:897` | /// Reference: `tTrajectory::AdjustTrajectory` at `0x82db6e78`. |
| `0x82db6ec0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:2` | //! Reset82DE53F0 runs before the actual producers82DB6EC0/82D3A388. |
| `0x82db6ec0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:270` | //82DB6EC0 publishes the same processed foot record to both fields. |
| `0x82db6ee8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:327` | //ProcessOutput82DB6EE8 resets the packet before its component publishers. |
| `0x82db6fd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/world_types.rs:373` | /// The pumping state Update5 publishes (TU3 82DB6FD0). |
| `0x82db6fd0` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:337` | // TU3 Update5_ProcessOutput 82DB6FD0 inlines Pumping.Fill: four skeleton |
| `0x82db703c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind_output.rs:21` | // Common ProcessOutput82DB703C copies selector9653 unconditionally. |
| `0x82db7044` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:164` | //Common ProcessOutput82DB7044/7048 and70C0/70C4; these precede state Fill. |
| `0x82db70a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:203` | //82DB70A0..70FC publishes Reckoning752, not the physical deck frame. |
| `0x82db7104` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:182` | //ProcessOutput82DB7104..7128: selector57 requests State69 here. |
| `0x82db7130` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:223` | //ProcessOutput82DB7130..7180 copies the HandPlantManager animation |
| `0x82db7198` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:885` | /// Reference: `tTrajectory::_GetTimeAtHighestPos` at `0x82db7198`. |
| `0x82db71b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant.rs:137` | /// Common ProcessOutput82DB71B0 calls82D71040 before selected state Fill. |
| `0x82db71c4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:218` | //ProcessOutput82DB71C4 calls SkateboardController::FillPhysOut82D76D20 |
| `0x82db71c8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/world_types.rs:427` | /// This frame's heavy landing (Wipeout request 6) as TU3 Update5 82DB71C8 publishes it: |
| `0x82db7218` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:13` | //82DB7218..7258 runs for every state, before the selected state's Fill. |
| `0x82db7250` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:42` | // ProcessOutput82DB7250/7258 publishes shared Biped720/716 as |
| `0x82db7288` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:384` | // and attaches OB_BipedWorldX/Z unchanged. ProcessOutput82DB7288..729C |
| `0x82db7480` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:43` | //Common82DB7480 copies Processed2468 bit21 to Air439 on every frame. |
| `0x82db74d8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:210` | //Original82DB74D8, not the similarly numbered Air output byte. |
| `0x82db7580` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:1` | //! Selected-state FillPhysOut and common state82DB7580..7790. |
| `0x82db7598` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:369` | //82DB7598 publishes Motion273 from Processed2468 bit20. |
| `0x82db7598` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/mod.rs:126` | // Board Fill82C03318 and common ProcessOutput82DB7598. |
| `0x82db7678` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/controls.rs:83` | //82DB7678/7694 writes category and State75 from the same category==500. |
| `0x82db76e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:162` | //82DB76E0..76F4 publishes this independently of selected-state304/311. |
| `0x82db78a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:115` | //82DB78A8..78DC: the processed off-board request refreshes three |
| `0x82db78a8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:59` | // three publications after state702's (ProcessOutput 82DB78A8..78DC, the PhysOut |
| `0x82db7948` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:13` | //Common82DB7948..7968 derives this after state Fill, not from category400. |
| `0x82db7a58` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:357` | // Original82DB7A58..7AA4 copies AnimOut10374/10371 respectively. |
| `0x82db7a90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:63` | //Common82DB7A90 copies the completed animation packet's byte10372. |
| `0x82db7da4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:33` | /// Update5 publishes them (82DB7DA4, 82DB7DB8: PhysOut_Intents +0x3D and +0x3E), kept past |
| `0x82db7da4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/world_types.rs:413` | /// PhysOut_Intents +0x3D and +0x3E (Update5 82DB7DA4, 82DB7DB8): a drop-in's tail locking on |
| `0x82db7da4` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:166` | /// Skate 3 physics: TU3 `PhysOut_Intents` +0x3D and +0x3E (Update5 82DB7DA4, 82DB7DB8), |
| `0x82db7db8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:33` | /// Update5 publishes them (82DB7DA4, 82DB7DB8: PhysOut_Intents +0x3D and +0x3E), kept past |
| `0x82db7db8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/world_types.rs:413` | /// PhysOut_Intents +0x3D and +0x3E (Update5 82DB7DA4, 82DB7DB8): a drop-in's tail locking on |
| `0x82db7db8` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:166` | /// Skate 3 physics: TU3 `PhysOut_Intents` +0x3D and +0x3E (Update5 82DB7DA4, 82DB7DB8), |
| `0x82db7dcc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_input.rs:124` | ///82DB7DCC..7E0C runs AFTER publishing AnimOut into the physical output. |
| `0x82db7dcc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:34` | /// the clear that follows (82DB7DCC) for the frame's output. |
| `0x82db7e38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:40` | ///PhysOutInteraction (bundle52)+56; ProcessOutput82DB7E38 owns this. |
| `0x82db7e38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_shove.rs:173` | // strike window ProcessOutput82DB7E38 requires) reaches physics only through it, and |
| `0x82db7e38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:217` | //Original ProcessOutput82DB7E38. Probe+72 is copied to Processed1848; |
| `0x82db7fc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:236` | // target and hands the game a type 2 (pedestrian) hit with 0x82db7fc0's velocity. |
| `0x82db7fc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:51` | /// `ProcessOutput` (0x82db7fc0) sends. |
| `0x82db7fc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/shove_probe.rs:166` | /// `ProcessOutput` 0x82db7fc0: the shove's velocity, 7 m/s from the skater's system position to |
| `0x82db8120` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/world.rs:205` | // State-specific handling belongs to ProcessOutput82DB8120. |
| `0x82db8120` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:281` | // ProcessOutput calls 82DB8120 after component and selected-state Fill. |
| `0x82db8540` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/transition.rs:1` | //! Native SetPhysicsState82DB8540 publication, followed by owned Enter/Exit. |
| `0x82db8540` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/transition.rs:157` | //Native82DB8540 publishes Processed state/history BEFORE old Exit. The |
| `0x82db88c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:112` | fn check_teleport_82db88c8( |
| `0x82db8998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:63` | //input reset82DB8998, which would immediately clear that mode again. |
| `0x82db8998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:1` | //! Ordinary local-player teleport82DB8998, operating on the live assemblies. |
| `0x82db8998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:234` | ///82DB8998 brackets board/skeleton teleport with this operation. Call before |
| `0x82db8998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/services.rs:53` | /// Complete 82DB8998 reset including board, skeleton, Reckoning, current |
| `0x82db8998` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_teleport.rs:49` | ///82DB8998 brackets82BDF7C0 with Skeleton16508=true/false after the real |
| `0x82db8a10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:167` | ///82DB8A10..8BB4 flattens At before normalization, rejects length<=.001, |
| `0x82db8a10` | TU3 | `crates/skate2-core/src/physics/player/skate3_spawn.rs:2` | //! then the shared S3 board reset82DB8A10 adds .1m. A standing biped must |
| `0x82db8ce4` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:80` | // TU3 82DB8CE4..8D1C stops possession before moving either assembly. |
| `0x82db8ce4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager.rs:89` | ///82DB8CE4..8D1C and82DB8D6C..8DA4: clear countdown even when |
| `0x82db8d6c` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/input_phase.rs:146` | // TU3 82DB8D6C..8DA4 repeats the controller reset after Skeleton. |
| `0x82db8d6c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager.rs:89` | ///82DB8CE4..8D1C and82DB8D6C..8DA4: clear countdown even when |
| `0x82db8db4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:150` | self.wipeout.state.teleport(); //82DB8DB4, separate from ResetSystems. |
| `0x82db8dd4` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:140` | //82DB8DD4..8DE4 always selects100 after resetting the assemblies. |
| `0x82db8e10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/runtime.rs:252` | ///82DB8E10..8E24, AFTER teleport's SetPhysicsState(Ground). Standard-board |
| `0x82db8e40` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:135` | //82DB8E40 calls SetUpNormal after pose reset, state100 and board reset. |
| `0x82db9018` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:757` | /// Reference: `TrajectorySelector::InitTrajectoryInfo` at `0x82db9018`. |
| `0x82db9100` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/render_pose.rs:13` | //82DB6DDC/6DF8 passes IsWipeout82DB9100 after state postphysics, not the |
| `0x82db92d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:101` | //ResetSystems82DB92D0 preserves Reckoning filter histories. Seed the |
| `0x82db92d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:79` | ///ResetSystems82DB92D0 resets the selector and physical-output filter; |
| `0x82db92d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state.rs:85` | //ResetSystems82DB92D0 clears20..44, sets48=10 and52=0. |
| `0x82db92d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_input_teleport.rs:18` | ///82BE3508, after board reset and before player ResetSystems82DB92D0. |
| `0x82db93b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:123` | //82DB93B0..CC: complete pending queries and clear contact history. |
| `0x82db93e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/input_teleport.rs:125` | //82DB93E8/93EC resets Player1840 through82D749D0, not ballistic work. |
| `0x82db95e8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skeleton_output.rs:1` | //! Production adapter for the original physical-pose job82DB6698/82DB95E8. |
| `0x82db98a0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:650` | /// `IsSecondPassRequired` at `0x82db98a0`. |
| `0x82db9dd8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1571` | /// Reconstructs `TrajectorySelector::CalculateSurfaceScore` at `0x82db9dd8`. |
| `0x82db9ee0` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1519` | /// Reconstructs `TrajectorySelector::CalculateGrindPenalty` at `0x82db9ee0`. |
| `0x82dc02f8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:829` | /// Reference: `TrajectorySelector::CalculateCOMTrajectory` at `0x82dc02f8` |
| `0x82dc0558` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1368` | /// Reconstructs `TrajectorySelector::CalculateWallRideScore` at `0x82dc0558`. |
| `0x82dc2800` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:597` | /// by its private dispatch at `0x82dc2800`. |
| `0x82dc2840` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:23` | /// board + skater pipeline with 28 joints / 53 drives): 50 on every solve. Setup `82DC2840` |
| `0x82dc3ae4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:261` | // Serialized primitive82DC3AE4/82DC3B84, source literal82165A00. |
| `0x82dc3b84` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground.rs:261` | // Serialized primitive82DC3AE4/82DC3B84, source literal82165A00. |
| `0x82dc3bd0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:36` | //82DC3BD0/82DC4260 copy Part92 into VolumeData212 |
| `0x82dc4098` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:247` | /// `TrajectorySelector::Launch` at `0x82dc4098`. |
| `0x82dc4098` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:596` | /// Reference: public `TrajectorySelector::Launch` at `0x82dc4098` followed |
| `0x82dc4260` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:36` | //82DC3BD0/82DC4260 copy Part92 into VolumeData212 |
| `0x82dc5c38` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1473` | /// at `0x82dc5c38` around the still-separate grind spatial index and trajectory |
| `0x82dc5e00` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1604` | /// `0x82dc5e00`. |
| `0x82dc62c4` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1316` | /// `TrajectorySelector::CalculateTrajectoryScores` at `0x82dc62c4`. |
| `0x82dc64c8` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:1635` | // Preserve the reference's scalar addition order at 0x82dc64c8. |
| `0x82dc6528` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:649` | /// Reference: `TrajectorySelector::Update` at `0x82dc6528` and |
| `0x82dc8ed8` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_update1_prefix.rs:119` | // LandingOnDeckManager.Tick82dc8ed8 has exactly this one store. |
| `0x82dcc0fc` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime.rs:228` | ///increment at82DCC0FC; S3 does not. Do not group hits into chunks of three. |
| `0x82dcd110` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/services.rs:10` | ///82D770D8; S2 OffBoardFeetIK::ConsiderReset82DCD110 corroborates the owner. |
| `0x82dd1fa0` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_update1_prefix.rs:95` | // Tick82dd1fa0 does not clear sync_allowed when the pointer is null. |
| `0x82dd5660` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_update1_prefix.rs:107` | // then a pending line batch completes through CalculateResults82dd5660. |
| `0x82dd5f98` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_update1_prefix.rs:106` | // Tick82dd5f98 decrements with native signed wrapping before the signed clamp, |
| `0x82ddbd58` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:3107` | /// `PhysicalPlayerStateChanger::CalcSuggestedState` at `0x82ddbd58`. |
| `0x82de2ef8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:10` | //82DE2EF8 clears these three represented Air324 bits; preserve the rest. |
| `0x82de2f70` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:15` | //82DE2F70..78 loads original world-up into144.82DE304C |
| `0x82de304c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:15` | //82DE2F70..78 loads original world-up into144.82DE304C |
| `0x82de3518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:35` | }, //82DE3518 |
| `0x82de3728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:168` | //Ground288 resets82DE3728; selected ordinary Ground Fill leaves it. |
| `0x82de3728` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:31` | }, //82DE3728 |
| `0x82de3be8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:61` | //State/FilteredState zeros (82DE3BE8/82DE5588) until the completed tick. |
| `0x82de3be8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:81` | //State template82DE3BE8 clears bytes52..87. Only source-owned writes follow. |
| `0x82de3cc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:24` | }, //82DE3CC8 |
| `0x82de3f38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:131` | // PhysicsConditioner Reset82DE3F38, not the ordinary per-frame packet. |
| `0x82de4468` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:155` | //Scoring2 reset82DE4468; HoM duration stays inactive in this host. |
| `0x82de4940` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:2` | //! built by82DE4940; scalar zero does not replace any active producer. |
| `0x82de53f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:2` | //! Reset82DE53F0 runs before the actual producers82DB6EC0/82D3A388. |
| `0x82de53f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/output_reset.rs:1` | //! Represented PhysOut reset values. Original82DE53F0 copies the templates |
| `0x82de53f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/skater.rs:295` | // PhysOut reset82DE53F0 clears these observations before first output. |
| `0x82de53f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/teleport_state.rs:60` | // template cleared each tick by 82DE53F0). The ActionGraph takes it through |
| `0x82de5588` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_grind.rs:46` | // FilteredState reset82DE5588: category0, null names and crouch0. |
| `0x82de5588` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/frame.rs:61` | //State/FilteredState zeros (82DE3BE8/82DE5588) until the completed tick. |
| `0x82de5588` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/runtime.rs:84` | ///82DE5588/82DE5608 initializes196 to0;82DE5BA0/82DE5E10 copies it. |
| `0x82de5608` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/runtime.rs:84` | ///82DE5588/82DE5608 initializes196 to0;82DE5BA0/82DE5E10 copies it. |
| `0x82de56b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:1` | //! Completed PhysOut -> chromosome82DEE508, before filtered82DE56B0. |
| `0x82de56b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/runtime.rs:86` | ///accumulator is present in the reviewed82DE56B0 update chain. |
| `0x82de5858` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/runtime.rs:85` | ///82DE5858 measures velocity/acceleration, not grind distance. No distance |
| `0x82de5858` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/respawn.rs:94` | /// The conditioner's sample count since the last reset (EmpiricalMeasurements82DE5858). |
| `0x82de5858` | TU3? | `crates/skate2-core/src/physics/modes/skate3/world_types.rs:359` | /// last reset (EmpiricalMeasurements82DE5858). |
| `0x82de5ba0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/runtime.rs:84` | ///82DE5588/82DE5608 initializes196 to0;82DE5BA0/82DE5E10 copies it. |
| `0x82de5e10` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/runtime.rs:84` | ///82DE5588/82DE5608 initializes196 to0;82DE5BA0/82DE5E10 copies it. |
| `0x82de6078` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_state/publication.rs:256` | //82DE6078..60C0: state501 is Offboard or OffboardAir from output320. |
| `0x82de61d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:448` | // CalcLandingQuality82DE61D0 retains its conditioner fields on nonlanding |
| `0x82de61d0` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:227` | // TU3 CalcLandingQuality82DE61D0 already ran in the selected solver. |
| `0x82de6480` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/landing_quality.rs:1` | //! Stock PhysicsAnimation cache +560/+576 and +592/+608, TU3 82DE6480. |
| `0x82de6678` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_names.rs:1` | //! ZIP short names verified entry-for-entry from original S3 initializer82DE6678. |
| `0x82de8090` | Skate 2? | `crates/skate2-core/src/physics/assembly/parts/dynamic_assembly_part_transform.rs:298` | /// `Body::SetPosition` 0x82de8090: an attached Part's frozen body is activated and its |
| `0x82de8e88` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/offboard.rs:187` | // GetRawBaseScore 0x82de8e88 returns the base score whether or not the |
| `0x82de9a70` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/database.rs:1` | //! TrickDatabase::ConstructFromAttribSys82dfb490 and TrickScoringSettings::Set82de9a70. |
| `0x82de9f80` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_finish.rs:1` | //! ScoreModule runout82de9f80, gestures82dea060, banking82df61e8/2888/2b00. |
| `0x82dea060` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_finish.rs:1` | //! ScoreModule runout82de9f80, gestures82dea060, banking82df61e8/2888/2b00. |
| `0x82dea488` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/attributes.rs:179` | // Raw lis/ori/insrdi operands:82df25f0/2640,82dedd68/ddb8,82dea488/668. |
| `0x82dea800` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:20` | /// CheckForAirSequenceReward82dea800. Landing quality can arrive during |
| `0x82deacb8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_output.rs:1` | //! AirCollector FillAirPeriodData82deacb8 / thresholds82def4d0 / Fill82defaf0. |
| `0x82deb2d8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:95` | // IsPumping82deb2d8 / VMX-corrected IsWallRiding82df0048. |
| `0x82deb608` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_dispatch.rs:224` | /// ManageCurrentCollector82dfca20, including Nursary.ResetSequence82deb608. |
| `0x82deb658` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/analysis.rs:10` | /// Nursary::CheckForHippyFlip82deb658: the existing ground probes straddle |
| `0x82ded098` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:16` | // Histogram82ded098 uses signed char and saturates to [-127,127]. |
| `0x82deda2c` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_wipeout_score.rs:18` | // Raw82deda2c..44, default key at82df5624..40. |
| `0x82dedd68` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/attributes.rs:179` | // Raw lis/ori/insrdi operands:82df25f0/2640,82dedd68/ddb8,82dea488/668. |
| `0x82dedfc8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners.rs:76` | /// `Sk8::Score::GetTrickType` (`0x82dedfc8`) of a valid scorable. |
| `0x82dee0a0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners.rs:80` | /// `Sk8::Score::GetScorableIDConsumed` (`0x82dee0a0`) of a valid scorable. |
| `0x82dee128` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners.rs:84` | /// `gScorableInfo[id].mBaseScorable`, which `Sk8::Score::GetBaseScorableID` (`0x82dee128`) checks. |
| `0x82dee248` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_score_holder.rs:2` | //! allocation, then perform Reset82dee248 against that same Sequence. |
| `0x82dee2e8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:1` | //! ScoreHolder announcement, repeat history and banking; native82dee2e8..82df3e38. |
| `0x82dee358` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:68` | ///82DEE358 leaves byte144 unwritten. The false storage below is opaque |
| `0x82dee358` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:86` | /// S3 ctor82DEE358 explicitly initializes the saved axes/position but does |
| `0x82dee508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:1` | //! Completed PhysOut -> chromosome82DEE508, before filtered82DE56B0. |
| `0x82dee508` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:1` | //! ZIP chromosome port corrected from S3 82DEE918/82DEF518/82DEE508. |
| `0x82dee508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:21` | ///82DEE508's actual PhysOut publication, after update and only while grinding. |
| `0x82dee508` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:209` | //82DEE508 tail, independent of the category. |
| `0x82dee508` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_names.rs:2` | //!82F92AC0 splits384 records;82DEE508 publishes skating IDs and scoring IDs |
| `0x82dee918` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:1` | //! ZIP chromosome port corrected from S3 82DEE918/82DEF518/82DEE508. |
| `0x82deeaf8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome/pose.rs:5` | /// Consumed subset of the144-byte record copied by82DEEAF8. Ground80 and |
| `0x82deeaf8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:131` | //82DEEB90/82DEEAF8, in this order even if both conditions hold. |
| `0x82deeb90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:131` | //82DEEB90/82DEEAF8, in this order even if both conditions hold. |
| `0x82deed28` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:114` | //82DEED28: fewer than30 samples uses saved, NOT newest ground pose. |
| `0x82deed98` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome/pose.rs:36` | ///82DEED98: twist uses Basic128, not Basic32 or a substituted deck-forward. |
| `0x82deeee0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome/pose.rs:61` | ///82DEEEE0: sign the Basic32 axis from Basic48 minus contact. |
| `0x82def4d0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_output.rs:1` | //! AirCollector FillAirPeriodData82deacb8 / thresholds82def4d0 / Fill82defaf0. |
| `0x82def518` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:1` | //! ZIP chromosome port corrected from S3 82DEE918/82DEF518/82DEE508. |
| `0x82def518` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:170` | //82DEF518: orientation, tilt, then twist, using actual Basic output |
| `0x82defaa0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_feedback.rs:24` | //82DEFAA0..FAC8 zeros the retained vector96. |
| `0x82defaa0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_feedback.rs:33` | //82DEFAA0 resets histories to zero. Every82DEFF38 update refreshes |
| `0x82defaa0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_phase.rs:495` | /// PhysOutAnimation reset82DEFAA0 and the initial represented body outputs. |
| `0x82defaf0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_output.rs:1` | //! AirCollector FillAirPeriodData82deacb8 / thresholds82def4d0 / Fill82defaf0. |
| `0x82defb6c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_feedback.rs:60` | //82DEFB6C..FB7C publishes the preceding Animation0 at Animation16, |
| `0x82defc10` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/database.rs:50` | /// IsGrindGrab82defc10 reads the original trick layout byte148. |
| `0x82deff38` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/animation_feedback.rs:33` | //82DEFAA0 resets histories to zero. Every82DEFF38 update refreshes |
| `0x82df0048` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:95` | // IsPumping82deb2d8 / VMX-corrected IsWallRiding82df0048. |
| `0x82df00e0` | Skate 2 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:46` | /// IsActivitySequenceWorthy82df00e0. Counts are the native current wheel |
| `0x82df0370` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:124` | /// GroundCollector::FillPhysOut82df0580 and UpdateScoreThresholds82df0370. |
| `0x82df0580` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:124` | /// GroundCollector::FillPhysOut82df0580 and UpdateScoreThresholds82df0370. |
| `0x82df0640` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_camera.rs:1` | //! ZIP contact conditioner82DF0640, corroborated against S2 82E2D000. |
| `0x82df0a68` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_dispatch.rs:182` | // Shared Enter82df0a68 calls the concrete Reset virtually. Air has its |
| `0x82df1950` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_wipeout_score.rs:30` | ///82df1950 explicitly initializes every payload field despite noncleared allocation. |
| `0x82df2130` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:9` | ///82DF2130 installs chromosome at slot1 and filtered at slot5;82DF2710 |
| `0x82df2130` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/timed.rs:1` | //! TimedArcLengthScorable::Update82df2130. Extend the retained native tracker. |
| `0x82df22f8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/timed.rs:81` | // Raw82df22f8 fsel(-change,0,change), followed by TWO fmadds. |
| `0x82df2514` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_camera.rs:68` | // S3 constructor82DF2514:0.95 per physical update, not dt-based smoothing. |
| `0x82df25f0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/attributes.rs:179` | // Raw lis/ori/insrdi operands:82df25f0/2640,82dedd68/ddb8,82dea488/668. |
| `0x82df26f0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_score_module.rs:1` | //! Complete nested `ScoreModule` constructor82dfb508 and Reset82df26f0. |
| `0x82df2710` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/condition.rs:9` | ///82DF2130 installs chromosome at slot1 and filtered at slot5;82DF2710 |
| `0x82df3290` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/collision.rs:57` | // 82DF3290 initializes both vectors, fraction and surface to zero on a miss. |
| `0x82df35c8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_score_holder.rs:1` | //! `ScoreHolder82df35c8`: retain the holder before its nested Sim Sequence |
| `0x82df35dc` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_score_holder.rs:61` | // Raw828b14b4..14cc class;82df35dc..35f8 default collection. |
| `0x82df3e38` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/holder.rs:1` | //! ScoreHolder announcement, repeat history and banking; native82dee2e8..82df3e38. |
| `0x82df44d8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_air_collector.rs:32` | /// Set82df44d8 with the original constructor's NULL takeoff argument. This |
| `0x82df4720` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_air_collector.rs:1` | //! `AirCollector82df8e18` and Reset82df4720 read the same constructed `PhysOut`. |
| `0x82df49d8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_lifecycle.rs:1` | //! AirCollector Enter82df49d8 / Exit82dfb898. Extend the existing reset owner. |
| `0x82df4a90` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_conditions.rs:1` | //! Camera condition leaves82DF4A90..82DF61DC, bound to the stock binary graph. |
| `0x82df4e60` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:1` | //! AirCollector spin82df4e60, body-flip82df5188 and trajectory82df5340. |
| `0x82df4f44` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:19` | // Raw82df4f44/5020: fused accumulation when switching modes. |
| `0x82df5188` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:1` | //! AirCollector spin82df4e60, body-flip82df5188 and trajectory82df5340. |
| `0x82df5340` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:1` | //! AirCollector spin82df4e60, body-flip82df5188 and trajectory82df5340. |
| `0x82df54dc` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_metrics.rs:179` | // not add it to the holder here (82df54dc..551c). |
| `0x82df5588` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_conditions.rs:73` | // Constructor82DF5588 uses case-sensitive comparison and keeps0 |
| `0x82df55a0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_wipeout_score.rs:53` | /// Constructor82df55a0 loads actual keyf54cf3579c22165b via generic lookup. |
| `0x82df5618` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_wipeout_score.rs:1` | //! `WipeoutScorer82df5618` owns three scoring children even in offline startup. |
| `0x82df5624` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_wipeout_score.rs:18` | // Raw82deda2c..44, default key at82df5624..40. |
| `0x82df56d0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_wipeout_score.rs:163` | // Raw82df56d0/d8/e0 stores pitch2, yaw0, roll1. |
| `0x82df5bc0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_lifecycle.rs:1` | //! WipeoutScorer Update82df6cb8 / UpdateInWipeout82df5bc0 and native lifecycle. |
| `0x82df61dc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_conditions.rs:1` | //! Camera condition leaves82DF4A90..82DF61DC, bound to the stock binary graph. |
| `0x82df61e8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_finish.rs:1` | //! ScoreModule runout82de9f80, gestures82dea060, banking82df61e8/2888/2b00. |
| `0x82df6294` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:48` | // Native preferences constructor82DF6294/629C initializes both |
| `0x82df69c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:1` | //! Game owners -> normal camera subject82DF69C0. Call after physical output. |
| `0x82df69c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/subject.rs:1` | //! Typed physics/animation publication boundary for normal subject82DF69C0. |
| `0x82df69c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:1` | //! Physical-output publications consumed by gameplay camera82DF69C0. |
| `0x82df6a44` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:311` | special_effect: 0, //82DF6A44..6A60 clears observer mode for this subject. |
| `0x82df6a68` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:189` | ///82DF6A68..6B8C copies the distinct OffBoard trajectory when byte331 is set. |
| `0x82df6b74` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:199` | //82DF6B74..6B8C supplies OffBoard160 to setter52 at82DF6CA0. |
| `0x82df6ca0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:199` | //82DF6B74..6B8C supplies OffBoard160 to setter52 at82DF6CA0. |
| `0x82df6cb8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/wipeout_lifecycle.rs:1` | //! WipeoutScorer Update82df6cb8 / UpdateInWipeout82df5bc0 and native lifecycle. |
| `0x82df6ea4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:36` | /// Actor's actual SkaterAnim component virtual28, copied at82DF6EA4. |
| `0x82df70cc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:162` | //82DF80D8 selects the subject pose separately.82DF70CC/70E8 still |
| `0x82df726c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:281` | subject_flag_328: 0, //82DF726C clears this every publication. |
| `0x82df7474` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:210` | //82DF7474..7490 combines this byte with the dropping-in event. |
| `0x82df74a8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/analysis.rs:1` | //! Nursary::ProcessAnimData82dfc358 / StartScorable82df74a8. |
| `0x82df74e4` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:206` | //82DF74E4 reads308; the camera packet's historical name is misleading. |
| `0x82df7a90` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_tricks.rs:1` | //! AirCollector trick lifecycle82df7a90/7c18/8f98/9080. |
| `0x82df7e20` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:269` | /// StartNewPowerslide/Manual/Revert82df7e20/7f30/8040 share StartScorable, |
| `0x82df80d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/publication.rs:162` | //82DF80D8 selects the subject pose separately.82DF70CC/70E8 still |
| `0x82df8110` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:270` | /// but grab uses the actual analyzed animation name82df8110. |
| `0x82df8400` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/offboard.rs:1` | //! OffboardCollector82df8400/c250/c308. Uses actual skeleton and deck frames. |
| `0x82df852c` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/offboard.rs:96` | // Native fmsubs82df852c, not two rounded products then SUB. |
| `0x82df8e18` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_air_collector.rs:1` | //! `AirCollector82df8e18` and Reset82df4720 read the same constructed `PhysOut`. |
| `0x82df8e80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/camera_output.rs:49` | // fields to zero.82DF8E80 only updates its air-side bytes37..39. |
| `0x82df90c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_conditions.rs:92` | //82DF90C0 compares exactly strlen(requested) bytes with82990158. |
| `0x82df9270` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/trajectory.rs:1` | //! Camera trajectory requests82DF9270/82DF9338/82DF9340. Rust executes the |
| `0x82df92bc` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/trajectory.rs:31` | // 82DF92BC/82DF9304 set both maximum-error fractions to1. |
| `0x82df9304` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/trajectory.rs:31` | // 82DF92BC/82DF9304 set both maximum-error fractions to1. |
| `0x82df9338` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/trajectory.rs:1` | //! Camera trajectory requests82DF9270/82DF9338/82DF9340. Rust executes the |
| `0x82df9340` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/trajectory.rs:1` | //! Camera trajectory requests82DF9270/82DF9338/82DF9340. Rust executes the |
| `0x82df9858` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/grind.rs:1` | //! GrindCollector82df9858/9600/9790; score the existing filtered grind output. |
| `0x82df98d4` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/grind.rs:84` | // Raw82df98d4..98e0: decompiler incorrectly drops this assignment. |
| `0x82df9f78` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:404` | /// EndCurrentManual 0x82df9f78: a manual that never validated ends as -1 (the debug build |
| `0x82dfa260` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:458` | // Raw lis/ori/insrdi82dfa260..278, not the truncated decompiler key. |
| `0x82dfa598` | TU3 | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:543` | // UpdateRevert 0x82dfa598 (TU3 82DAAC10) starts a revert whatever side was last seen (an |
| `0x82dfa788` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:578` | /// EndCurrentGrab 0x82dfa788: only the announcement (the debug build asserts a valid grab |
| `0x82dfa88c` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:593` | // GroundCollector82dfa88c permits only coffin (catalog60). |
| `0x82dfb130` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/offboard.rs:228` | // EndCurrentHippyJump 0x82dfb130: a jump that never validated ends as -1, |
| `0x82dfb490` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/database.rs:1` | //! TrickDatabase::ConstructFromAttribSys82dfb490 and TrickScoringSettings::Set82de9a70. |
| `0x82dfb508` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners_score_module.rs:1` | //! Complete nested `ScoreModule` constructor82dfb508 and Reset82df26f0. |
| `0x82dfb898` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_lifecycle.rs:1` | //! AirCollector Enter82df49d8 / Exit82dfb898. Extend the existing reset owner. |
| `0x82dfbcb0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/air_update.rs:1` | //! AirCollector::Update2_TrickAnalysis82dfbcb0, preserving native metric order. |
| `0x82dfbe08` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:151` | /// Exit82dfbe08. Revert exit announces its end without the regular |
| `0x82dfbfe8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/ground.rs:1` | //! GroundCollector trick analysis82dfbfe8, using the existing retained owners. |
| `0x82dfc358` | Skate 2 | `crates/skate2-core/src/physics/skate3_player.rs:1671` | // from ScoringTrick. S2 ProcessAnimData82dfc358 admits that |
| `0x82dfc358` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/analysis.rs:1` | //! Nursary::ProcessAnimData82dfc358 / StartScorable82df74a8. |
| `0x82dfca20` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_dispatch.rs:224` | /// ManageCurrentCollector82dfca20, including Nursary.ResetSequence82deb608. |
| `0x82dfcd18` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_update.rs:1` | //! ScoreModule.Update82dfcd18. Inputs borrow the already-conditioned PhysOut; |
| `0x82dfcddc` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/module_dispatch.rs:367` | /// Update82dfcddc..cee0. Line prevention lasts a bounded number of held |
| `0x82dfd0f8` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/database.rs:24` | // AnimScorableIDInitializer82dfd0f8 inserts all269 catalog names using |
| `0x82dfe688` | TU3 | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:386` | /// Actual mAnimOutPhysIn+2b4, assigned by `SetPhysUpdateData82dfe688`. |
| `0x82dfee80` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/runtime.rs:175` | // Present the recovered CameraMan82DFEE80 result directly. |
| `0x82e00988` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_selected_animation_conditioning.rs:176` | // TeleportPlayerNow82e00988 resets the retained ScoreModule. Relay |
| `0x82e00988` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners.rs:383` | /// PhysOutConditioners::ResetAll at TeleportPlayerNow 0x82e00988. |
| `0x82e00988` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/conditioner.rs:28` | /// that native TeleportPlayerNow82e00988 reaches through ResetAll. |
| `0x82e01368` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1493` | /// before board (82e01368/1394), and `SkeletonDrives` before `SkeletonBody`. |
| `0x82e01db8` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_update1_prefix.rs:313` | /// Actual `CheckForTeleport82e01db8` selected branch and assertion order. |
| `0x82e01f98` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_update1_prefix.rs:1` | //! Actual `Update1_ProcessInput82e01f98..2174`, through its teleport check. |
| `0x82e03060` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/graph_conditions.rs:113` | //82E03060: signed velocity projected on the subject's forward axis. |
| `0x82e03ae8` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out.rs:375` | /// Exact late Update1 store 0x82e03ae8; no partial-copy helper invokes this. |
| `0x82e03f20` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:863` | // HiLOD82e03f20 completes PhysOut and publishes +2ac after ProcessedPhysIn, |
| `0x82e03f58` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:877` | // HiLOD82e03f58: all conditioners bind this same, completed PhysOut. |
| `0x82e03f58` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners.rs:2` | //!82e03f58 after `PhysOut` and before `SurfacePhysics`. This owner retains partial |
| `0x82e03f90` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:907` | // HiLOD82e03f90: retain partial SurfacePhysics before initializing its materials. |
| `0x82e03fc4` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:932` | // HiLOD82e03fc4: source allocation is retained across InitTargets failure. |
| `0x82e04008` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:864` | // before Reckoning82e04008 and either physical Body. This selected host |
| `0x82e04030` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:973` | // Native calls82e04030 and82e04068, before Pumping and either Body. |
| `0x82e04068` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:973` | // Native calls82e04030 and82e04068, before Pumping and either Body. |
| `0x82e040ec` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1040` | // Actual AnimOut allocation/ctor82e040ec/40f8 precedes Skateboard. |
| `0x82e04204` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1278` | // Native82e04204 follows the completed Skeleton child constructors; |
| `0x82e042bc` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1415` | // Native GrindManager82e042bc follows Wipeout and precedes FootPlant. |
| `0x82e042f4` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1446` | // Selected FootPlant ctor82e042f4. |
| `0x82e0432c` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_assembly.rs:1459` | // Actual PlayerGrabSpline82e0432c follows FootPlant, before controller. |
| `0x82e05dc0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/shake_data.rs:1` | //! Owned .shk parser; numerical sequence centering follows82E05DC0 in core. |
| `0x82e099a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/mod.rs:31` | ///Full82E099A0 query shared by trajectory and Footplant callers. |
| `0x82e09a14` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/known_air/runtime.rs:66` | //82E09A14/2C splats the request horizon into all four final-vector lanes. |
| `0x82e0aad0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:1` | //! Static host for original S3 82C20728 and 82E0AAD0 line batches. |
| `0x82e0aad0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/world.rs:37` | ///82D86C88 passes processed2952 into82E0AAD0, not actor identity2948. |
| `0x82e0b460` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:85` | //82E0B4E0 -> TestBatchManager backend virtual28. Its ctor82E0B460 |
| `0x82e0b4e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/foot_ik_queries.rs:85` | //82E0B4E0 -> TestBatchManager backend virtual28. Its ctor82E0B460 |
| `0x82e0b690` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/solve/assembly_contacts.rs:38` | //82E0B690 names this producer part.m_userData. Biped assigns |
| `0x82e0bb10` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:15` | //! KD bounds. The base `EndUpdate` 0x82e0bb10 and the serialized volume update |
| `0x82e0bb10` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:317` | /// job and `EndUpdate` 0x82e0bb10. The returned ordinals are EndUpdate's |
| `0x82e0bb10` | Skate 2? | `crates/skate2-core/src/physics/assembly/simulation_dmo_dynamic_pool_update.rs:327` | /// `EndUpdate` 0x82e0bb10 for the frame whose `BeginUpdate` rebuilt the |
| `0x82e0bde4` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1827` | /// island-label stores (`0x82e0bde4..0x82e0be14`). Numeric rows are not read. |
| `0x82e0be14` | Skate 2? | `crates/skate2-core/src/physics/solver/bodies/simulation_rigid_body_pool.rs:1827` | /// island-label stores (`0x82e0bde4..0x82e0be14`). Numeric rows are not read. |
| `0x82e0c324` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:947` | // Native manager +0x50, initialized at 0x82e0c324. Shared by all |
| `0x82e0dbd0` | Skate 2? | `crates/skate2-core/src/physics/solver/contacts/simulation_contact_jacobian.rs:1670` | /// `AssemblyPool::Insert` 0x82e0dbd0..dc: consume old u32 BEFORE Add. |
| `0x82e0e7b0` | TU3 | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:23` | /// (`cLivingWorldCollisionBody::GetContacts` 0x82e0e7b0). |
| `0x82e17bb0` | Skate 2? | `crates/skate2-core/src/assets/character.rs:238` | /// (`AddPegasusMaterialToScope` 0x82e17bb0 binds it for a channel without a raster). |
| `0x82e24ce0` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:274` | /// Trajectory constructor82e2a300 and reset82e24ce0: vector time is splatted -1. |
| `0x82e24ce0` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:289` | /// `PhysOut_Air` constructor82e2a300 invokes reset82e24ce0. |
| `0x82e24f6c` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out.rs:348` | // 0x82e24f6c, before nested SkeletonCollisionInfo::Reset. |
| `0x82e25028` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:856` | /// `PhysOut_Audio` reset82e25028. |
| `0x82e250c8` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:544` | /// Native `Ground` reset82e250c8; Ground excludes existing projected members. |
| `0x82e253a8` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:411` | /// Native `Skeleton` reset82e253a8; Ground excludes existing projected members. |
| `0x82e25550` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:750` | /// `PhysOut_State` reset82e25550. |
| `0x82e255f8` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:620` | /// Animation reset82e255f8; time starts zero, wipeout TTL starts `f32::MAX`. |
| `0x82e268c8` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:385` | /// (0x82e268c8), vehicles 10 and 8 (0x82d28eb8, 0x82c37458), dynamic objects 14/16 and 12/14 |
| `0x82e29908` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/conditioner.rs:88` | // UpdateAudioScore82e29908 keeps a positive award for four further |
| `0x82e2a300` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:274` | /// Trajectory constructor82e2a300 and reset82e24ce0: vector time is splatted -1. |
| `0x82e2a300` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:289` | /// `PhysOut_Air` constructor82e2a300 invokes reset82e24ce0. |
| `0x82e2a370` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_simple.rs:927` | /// Their initial zero stores belong to constructor82e2a370 alone. |
| `0x82e2a3f8` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_scoring.rs:1` | //! Complete retained Scoring2 constructor fields (82e2c768) and Reset82e2a3f8. |
| `0x82e2b438` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/conditioner.rs:1` | //! PhysOutConditioner_Scoring.Update82e2b438 after Animation.Update. |
| `0x82e2c768` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_scoring.rs:1` | //! Complete retained Scoring2 constructor fields (82e2c768) and Reset82e2a3f8. |
| `0x82e2d000` | Skate 2 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_camera.rs:1` | //! ZIP contact conditioner82DF0640, corroborated against S2 82E2D000. |
| `0x82e2d380` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_selected_animation_conditioning.rs:136` | // GameCam82e2d380 is the last UpdateAll child. |
| `0x82e2d388` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_grinds.rs:31` | /// `PhysOut_Grinds` reset82e2d388; GUID reset stores sign-extended li -1. |
| `0x82e2d618` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_grinds.rs:114` | /// `PhysOut_Filtered` reset82e2d618; GUID reset stores sign-extended li -1. |
| `0x82e2dee4` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:432` | // The dormant Physics conditioner's grind stores (Update82e2f970, 82e2dee4..dff0): while |
| `0x82e2e9e0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:2` | //! S2 82E2E9E0/82E2EBE8/82E2F9D0 confirms history/publication structure; |
| `0x82e2ebe8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:2` | //! S2 82E2E9E0/82E2EBE8/82E2F9D0 confirms history/publication structure; |
| `0x82e2f2a8` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out.rs:1` | //! Selected components of the single `HiLOD` `PhysOut` owner (0x82e2f2a8). |
| `0x82e2f730` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out.rs:305` | // 0x82e2f730, after all component constructor operations. |
| `0x82e2f75c` | Skate 2? | `crates/skate2-core/src/physics/player/output/physical_player_phys_out.rs:310` | /// 0x82e2f75c..2f8dc: exact component order. Omitted stages can fail after |
| `0x82e2f970` | TU3 | `crates/skate2-core/src/physics/player/output/physical_player_phys_out_skate3.rs:432` | // The dormant Physics conditioner's grind stores (Update82e2f970, 82e2dee4..dff0): while |
| `0x82e2f9d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome.rs:2` | //! S2 82E2E9E0/82E2EBE8/82E2F9D0 confirms history/publication structure; |
| `0x82e2f9d0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_names.rs:3` | //! separately. S2 82E2F9D0 uses160 records (two orientations, five families). |
| `0x82e316f0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/physical_player_phys_out_conditioners.rs:1` | //! Complete eleven-child conditioner constructor82e316f0, called by `HiLOD` at |
| `0x82e31970` | Skate 2? | `crates/skate2-core/src/physics/player/lifecycle/physical_player_selected_animation_conditioning.rs:86` | /// Selected native Animation.Update82e31970. Other `UpdateAll` children remain |
| `0x82e31970` | Skate 2? | `crates/skate2-core/src/physics/player/conditioning/physical_player_conditioner_animation_update.rs:1` | //! Animation.Update82e31970 after Physics.Update; not full `UpdateAll`. |
| `0x82e32328` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/selection_space_host.rs:14` | //TU3 vtable8231E050+76 ->82E32328 ->82D19648 constructs a |
| `0x82e34960` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:109` | /// caller, `GetCameraSkaterOpacity82e34960` returns 0.0 for an empty CameraMan |
| `0x82e363e0` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:95` | /// 0x82e363e0: another skater leaves the cameras alone. The camera's own skater would take |
| `0x82e37c08` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:1` | //! Retained notification fields of `GameCameraManager82e37c08`. This projection |
| `0x82e38cc0` | TU3 | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:3` | //! strike (0x8275f0d8 -> `Pedestrian` 0x82e38cc0), on Skate 2's living world. |
| `0x82e38cc0` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:55` | /// 0x82e38cc0: the shoved pedestrian goes down from the side the shove came from, its collision |
| `0x82e38dd8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_shove.rs:80` | /// 0x82e38dd8: the shove's angle from the pedestrian's facing about world up, wrapped to a half |
| `0x82e3a758` | Skate 2? | `crates/skate2-core/src/camera/frame/camera_session.rs:82` | /// `OnLocalSkaterChanged82e3a758` first creates cameras, then `SetLocalSkater`. |
| `0x82e49908` | Skate 2? | `crates/skate2-core/src/physics/grind_data_runtime.rs:259` | /// 82cfb538, `SplineEval_One`82e49908 corrected raw VMX operands. |
| `0x82e4b868` | Skate 2? | `crates/skate2-core/src/physics/physics.rs:399` | /// Inputs queued by `Predictor::AddQuery` at `0x82e4b868` after asynchronous |
| `0x82e4c6c8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:272` | /// part bits (`LWCollideableObject::ResetCollisionFlags` 0x82e4c6c8), then, |
| `0x82e4c6c8` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:511` | /// ResetCollisionFlags82e4c6c8: vehicle's part-report virtuals at |
| `0x82e4c808` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:530` | // cLivingWorldCollision.UpdateCollisions82e4c808 calls the shared drive. |
| `0x82e4f078` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:437` | /// `GetPedestrianMan`82e4f078 -> `GetPedestriansWithinRange`82e9ab88 -> |
| `0x82e4fec0` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:299` | // GetLength82e4fec0 reads the model's Z dimension (zero without a model). |
| `0x82e50580` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:363` | // UpdateLOSStates82d21cf0; collision OnLOSimSwitch82e50580 returns true. |
| `0x82e50e60` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:384` | /// Island.ResolveContactSpyList827b3a90 -> collision Body.GetContacts82e50e60. |
| `0x82e50e60` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:128` | /// cLivingWorldCollisionBody.GetContacts82e50e60 -> Vehicle.OnEntityCollision |
| `0x82e51708` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:513` | //LWModel.GetRadius82e51708: half dimensions, fsel(X-Z,X,Z). |
| `0x82e62858` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:413` | // exclusively the optional debug-text history82e62858. |
| `0x82e69ac0` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:333` | // LWPropMan.Init82e69ac0: component memory/entities/factory precede |
| `0x82e6b070` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:295` | // LWPedestrianMemoryMan.Init82e6b070 precedes the manager's entity |
| `0x82e6e0f8` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:157` | /// `PedestrianMan::ReleaseNavMeshTile` (`0x82e6e0f8`) for each tile the MeshStore removed |
| `0x82e6e550` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:271` | /// `Pedestrian::ResetCollisionFlags` 0x82e6e550: the collideable object's |
| `0x82e808d8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:1` | //! DetermineOverallCollisionType82e808d8. Facts come from actual contact and |
| `0x82e808d8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:89` | ///82e808d8: part flags deliberately precede several ignore branches. |
| `0x82e80cac` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:460` | //82e80cac calls GetChasee82e86220, which resolves the live |
| `0x82e83580` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:376` | /// `LWPedestrianMan::ResetCollisionFlags` 0x82e83580, the first collision |
| `0x82e86220` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:460` | //82e80cac calls GetChasee82e86220, which resolves the live |
| `0x82e89bd0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:182` | // DecompressTwoConsecFrames 82E89BD0..BF4): the pose holds frame 8k+8. |
| `0x82e90390` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_construction.rs:749` | /// `cPedestrianAnim82e90390` shares Animatable/ChannelAnimatable storage. |
| `0x82e91c88` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:302` | // Pedestrian82e91c88 uses Colliding; BigBlack82e92508 forces a stop. |
| `0x82e92508` | Skate 2? | `crates/skate2-core/src/living_world/vehicles/living_world_vehicle_frame.rs:302` | // Pedestrian82e91c88 uses Colliding; BigBlack82e92508 forces a stop. |
| `0x82e94670` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:319` | //Entity vslot20 GetTransform82e94670 reads core+16 (actual), not+80 target. |
| `0x82e94ca8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:227` | //Entity vslot184 is SetAudioState82e94ca8, not an event callback. |
| `0x82e95158` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:158` | /// All branches of OnEntityCollision82e95158. A false native callback |
| `0x82e951ac` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:235` | /// OnEntityCollision82e951ac..953d4. Called after the sole existing |
| `0x82e957d8` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:285` | /// Full response of OnEntityCollision82e957d8..95d74. The caller has |
| `0x82e95938` | Skate 2? | `crates/skate2-core/src/living_world/pedestrians/pedestrian_collision_contacts.rs:314` | //Raw bgt/ble82e95938/4c, including unordered angular fallthrough. |
| `0x82e99df0` | TU3 | `crates/skate2-core/src/physics/assembly/skate3_world.rs:384` | /// skater bodies are 11 and 13 in Skate 2 (`InitCollisionBodies` 0x82e99df0) but 9 and 11 in TU3 |
| `0x82e9ab88` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:437` | /// `GetPedestrianMan`82e4f078 -> `GetPedestriansWithinRange`82e9ab88 -> |
| `0x82e9ef00` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:305` | // LWPedestrianMan.Init82e9ef00 captures its three execution rates |
| `0x82e9ef90` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:309` | // Init82e9ef90..eff4 binds the global group after execution rates. |
| `0x82e9eff0` | Skate 2? | `crates/skate2-core/src/living_world/interaction/living_world_grind_services.rs:315` | "LWPedestrianMan global perceptions assertion82e9eff0".into()));} |
| `0x82ee5680` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:2` | //! 0x82ee5680-0x82ee5938): the event databases (`VoxData`, the `<group>Output_Events.evt` |
| `0x82ee5680` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:524` | /// `SPCH_ClearMatchParmSettings` 0x82ee5680: the channel's global match parameters to 0xFF. |
| `0x82ee5700` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:464` | /// `SPCH_AddEventDB` 0x82ee57e8 / `iSPCH_BindData` 0x82ee5700: a revision 3.21 database |
| `0x82ee57e8` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:464` | /// `SPCH_AddEventDB` 0x82ee57e8 / `iSPCH_BindData` 0x82ee5700: a revision 3.21 database |
| `0x82ee58b0` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:512` | /// `iSPCH_CsisCb` 0x82ee58b0: the event's spec and its `numParms` parameter words added |
| `0x82ee5938` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:2` | //! 0x82ee5680-0x82ee5938): the event databases (`VoxData`, the `<group>Output_Events.evt` |
| `0x82ee5938` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:97` | /// The Csis table `iSPCH_InitCsis` 0x82ee5938 and `ContextSystemImpl::AddCsisEvents` 0x82ab3dd0 |
| `0x82ee5938` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:466` | /// channel's global match parameters are cleared either way. Then `iSPCH_InitCsis` 0x82ee5938 |
| `0x82f6b2d0` | Skate 2? | `crates/skate2-core/src/physics/player/output/conditioning/scoring/hud_frame.rs:70` | // kIsHoMHUDEnabled initializer82f6b2d0 defaults true. |
| `0x82f75f60` | TU3? | `crates/skate2-core/src/physics/modes/skate3/input/gesture_input.rs:60` | // by82F75F60 from820641A8, separately from cInputMap's own deadzones. |
| `0x82f82138` | Skate 2? | `crates/skate2-core/src/animation/runtime/actor_animation_frame_phase.rs:588` | // Dynamic initializers82f82138/68/80 provide these exact source names. |
| `0x82f825d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:17` | //82F825D0/F0 initialize830BD320/+4A0 from8231A844(+1)/8216DEE0(-1). |
| `0x82f825f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:278` | // Original830BD4A0 initializer82F825F0 splats8216DEE0=-1. |
| `0x82f825f0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames.rs:48` | //830BD4A0 initializer82F825F0 splats8216DEE0=-1 across all lanes. |
| `0x82f82610` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:76` | //82F826F8 broadcasts82181A88 into830BD350.82F82610 similarly |
| `0x82f82690` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:50` | //82F82690 splats original821647E0 into830BD300. |
| `0x82f82690` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:52` | //830BD300 is initialized by82F82690 from821647E0. |
| `0x82f826b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_phase.rs:15` | ///Original initializer82F826B0/822F8B40. This COM trajectory acceleration is |
| `0x82f826b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:96` | //830BD330 initializer82F826B0 uses822F8B40, independently read original bytes. |
| `0x82f826f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:171` | // 82F826F8 loads82181A88 with lvlx128, splats lane0, and stores |
| `0x82f826f8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:76` | //82F826F8 broadcasts82181A88 into830BD350.82F82610 similarly |
| `0x82f838b0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:32` | // 82F838B0 loads 82098E30 = BECC_CCCD into 830BDD20: -0.4. |
| `0x82f847c0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_intent_filter.rs:65` | // 82BA0A48 and original initializers 82F847C0..4850. |
| `0x82f84850` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action_nodes.rs:62` | // TU382F847C0..82F84850 initialize the names consumed by82BA0A48. |
| `0x82f84bc8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:67` | //Original830BE1E0 initializer82F84BC8 identifies KickTurn. HasIntent's |
| `0x82f84da8` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:390` | // TU3 initializers82F84DA8/82F84DC0 bind these exact keys. |
| `0x82f84dc0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/action.rs:390` | // TU3 initializers82F84DA8/82F84DC0 bind these exact keys. |
| `0x82f858d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:76` | //82F85A08/82F858D0 identify the two original names and this order. |
| `0x82f85a08` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:76` | //82F85A08/82F858D0 identify the two original names and this order. |
| `0x82f87410` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_toggle_board.rs:9` | const CHANNEL: &str = "RetrieveBoard"; //82F87410 |
| `0x82f87578` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/air.rs:99` | const CHANNEL: &str = "AirBodyTweak"; //82F87578 |
| `0x82f87578` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:13` | const CHANNEL: &str = "AirBodyTweak"; // initializer82F87578 ->830BE028 |
| `0x82f88150` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/push_off.rs:1` | //! IsPushOffEnabled: original TU3 registration82F88150 -> ctor82BC62F8. |
| `0x82f881d0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:307` | //82F881D0..83C4 constructs this exact37-entry table at830BF5D8. |
| `0x82f883c8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_execute_leaf.rs:207` | const NAMES: [&str; 5] = ["PUMP0", "PUMP1", "PUMP2", "PUMP3", "PUMP4"]; //82F883C8 |
| `0x82f88440` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:1` | //! Original registered reset handlers:82F88440/82F89600. |
| `0x82f89600` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/reset.rs:1` | //! Original registered reset handlers:82F88440/82F89600. |
| `0x82f8a340` | TU3 | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hippy_jump.rs:3` | //! Skate 3 TU3 IDA verifies the complete producer path: factory `0x82F8A340` |
| `0x82f92ac0` | TU3 | `crates/skate2-core/src/physics/modes/skate3/physics/grind_names.rs:2` | //!82F92AC0 splits384 records;82DEE508 publishes skating IDs and scoring IDs |
| `0x82fd8158` | Skate 2? | `crates/skate2-audio/src/world/emitter_system.rs:54` | /// `Sk8::Audio::verbHash` 0x82fd8158: the reverb keys `VerifyReverbKey` 0x824f8ec0 accepts. |
| `0x82fd8e88` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:103` | /// `spliceTypeMap` 0x82fd8e88: per terrain row (flat, ramp and stair terrains first) and surface |
| `0x82fd8f68` | Skate 2? | `crates/skate2-audio/src/player/offboard.rs:100` | /// `types` 0x82fd8f68: the grass/metal heel splices by `((speed * 2 + contact) * 2 + sound) * 2` |
| `0x82fd9410` | Skate 2? | `crates/skate2-audio/src/system.rs:392` | /// `delayIdle`, `startCount` and `endCount` (0x82fd9410-0x82fd9418). |
| `0x82fd9418` | Skate 2? | `crates/skate2-audio/src/system.rs:392` | /// `delayIdle`, `startCount` and `endCount` (0x82fd9410-0x82fd9418). |
| `0x82ff2790` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:23` | /// `gClearCycle` (0x82ff2790). |
| `0x82ff2794` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:21` | /// `iSPCH_OrderSentences`' `multiple` (0x82ff2794). |
| `0x82ff27b4` | Skate 2? | `crates/skate2-audio/src/speech/spch.rs:1809` | /// Seed 0 gives `seedX`'s static words (0x82ff27b4). |
| `0x83016248` | Skate 2? | `crates/skate2-audio/src/aems/mod.rs:102` | /// `sndsintbl` 0x83016248: a quarter sine, 0..65535 over 257 entries. |
| `0x83034f34` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:181` | // Ground job8277C5D8 uses context83034F34/38/3C. The |
| `0x83035308` | TU3? | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:216` | // Runtime reciprocals 0x83035308 / 0x8303530C. |
| `0x8303530c` | TU3? | `crates/skate2-core/src/physics/modes/skate3/animation_pose.rs:216` | // Runtime reciprocals 0x83035308 / 0x8303530C. |
| `0x830bd300` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/settings.rs:50` | //82F82690 splats original821647E0 into830BD300. |
| `0x830bd300` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/ground.rs:52` | //830BD300 is initialized by82F82690 from821647E0. |
| `0x830bd320` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/grind/advance.rs:17` | //82F825D0/F0 initialize830BD320/+4A0 from8231A844(+1)/8216DEE0(-1). |
| `0x830bd320` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:30` | //830BD4A0=-1 and830BD320=1; signs follow native vsel order. |
| `0x830bd330` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/footplant/update.rs:96` | //830BD330 initializer82F826B0 uses822F8B40, independently read original bytes. |
| `0x830bd350` | TU3? | `crates/skate2-core/src/physics/modes/skate3/camera/settings.rs:172` | // all four lanes at830BD350. This is not mapped BSS zero data. |
| `0x830bd350` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:76` | //82F826F8 broadcasts82181A88 into830BD350.82F82610 similarly |
| `0x830bd380` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/settings.rs:77` | //broadcasts82165A10 (zero) into830BD380 for speed override direction. |
| `0x830bd4a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/ground_phase.rs:278` | // Original830BD4A0 initializer82F825F0 splats8216DEE0=-1. |
| `0x830bd4a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/handplant/trajectory.rs:30` | //830BD4A0=-1 and830BD320=1; signs follow native vsel order. |
| `0x830bd4a0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/frames.rs:48` | //830BD4A0 initializer82F825F0 splats8216DEE0=-1 across all lanes. |
| `0x830bdd20` | TU3? | `crates/skate2-core/src/physics/modes/skate3/grind_world/octree.rs:32` | // 82F838B0 loads 82098E30 = BECC_CCCD into 830BDD20: -0.4. |
| `0x830be028` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak.rs:13` | const CHANNEL: &str = "AirBodyTweak"; // initializer82F87578 ->830BE028 |
| `0x830be1e0` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_kickturn.rs:67` | //Original830BE1E0 initializer82F84BC8 identifies KickTurn. HasIntent's |
| `0x830be900` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_sliding_execute.rs:69` | //82BB34D8 uses Manual830BE900; only a negative present value emits. |
| `0x830bf5d8` | TU3? | `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_character_gesture.rs:307` | //82F881D0..83C4 constructs this exact37-entry table at830BF5D8. |
| `0x830d0850` | TU3? | `crates/skate2-core/src/physics/modes/skate3/physics/settings.rs:35` | /// entry and no parent, so GetData fails and the shared Attrib default 830D0850 (zero) |
