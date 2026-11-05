# File index (generated)

Generated from the source by reading each file's own module documentation (`//!`) and its
type names; files without module docs list their first functions. Paths are relative to the
repository root. *(tests)* marks unit tests, probes and fixtures.

- Skate 3 physics module: 428 files
- Integration layer: 42 files
- Skate 2 systems carrying Skate 3 behaviour: 84 files

## 1. The Skate 3 physics module (`crate::skate3_native`)

The captured Skate 3 controller/physics code and its additions, compiled inside `skate2-core` with the `skate3-runtime` feature (default on).

### `crates/skate2-core/src/physics/modes/skate3/`

- **`animation.rs`** (10 lines)
  - fns: `native_matrix`
- **`animation_pose.rs`** (217 lines): Production animation command evaluator. File IO and buffer ownership are host services; frame selection, trajectory and pose math live in core.
  - types: `PoseEvaluator`
- **`camera.rs`** (24 lines): Normal gameplay camera rendering endpoint. Simulation owns the cadence of CameraRuntime::advance after its complete physical/animation publication.
- **`crash_context.rs`** (19 lines): Infrequent, allowlisted snapshots: no player names, addresses or lobby credentials.
- **`custom_difficulty.rs`** (191 lines): User-owned difficulty values. Original collection hashes remain asset identities; player-facing names describe verified consumers, not guessed original names.
  - types: `OptionSpec`, `Tuning`
- **`difficulty.rs`** (66 lines): Host preference selects the native physics_mode collection, never a multiplier.
  - types: `Difficulty`, `Saved`
- **`graph_runtime.rs`** (330 lines): Lossless bridge from the stock on-disk graph binding to the recovered TU3 dynamic-controller runtime.
  - types: `GraphCompileError`, `OperationRemap`, `CompiledGraph`, `LoadedGraph`, `StockGraphs`
- **`grind_world.rs`** (108 lines): Authored test-course grind paths. Collision/visuals share these dimensions. This asset owner does not claim a nearby spline is an acquired grind.
  - types: `GrindGeometry`, `Rail`
- **`input.rs`** (49 lines): Platform input adapter; no animation or physics state mutation here.
  - types: `PublishedTickInput`
- **`mod.rs`** (127 lines): Skate 3 controllers and calculations ported into Skate 2's physical player. No standalone game, Bevy App, window, renderer, updater or device polling.
  - types: `Span`, `Transform`
- **`modding.rs`** (32 lines)
  - types: `JointOverride`, `NativeBodyRef`, `PartOverride`
- **`physics.rs`** (519 lines): Host game physics ownership and schedule. Physical calculations stay in core.
  - types: `Skate3PhysicsState`, `SimulationExchange`
- **`scoring_runtime.rs`** (5 lines): TU3 RotationTracker (82DAC880/82DAC9D0), as AirCollector::UpdateSpinMetric 82DA8BE0 drives it. The Skate 2 ScoreModule scores Skate 3 mode; this keeps only its rotation frames.
- **`skate_world.rs`** (439 lines): Collision calculation port; Skate 2 owns all world rendering.
- **`skater_animation.rs`** (399 lines): The skater's stock graph -> tree -> pose -> physics-packet owner. Physical observations come from the preceding completed simulation output.
  - types: `AnimationSource`, `AnimationPhysical`, `SkaterAnimation`
- **`world_types.rs`** (679 lines): Host boundary for the original production tick, with caller-owned scene data.
  - types: `RawController`, `SceneTriangle`, `SceneTriangleFeatures`, `SceneRail`, `HostGrindSegment`, `HostGrindAsset`, `Scene`, `BodyOutput`, `SkeletonPartOutput`, `FrameOutput`

### `crates/skate2-core/src/physics/modes/skate3/animation_pose/`

- **`authored_clips.rs`** (240 lines): Optional authored body clips. Stock graph timing, board, trajectory and attributes remain authoritative.
  - types: `Replacements`, `File`, `AuthoredClip`

### `crates/skate2-core/src/physics/modes/skate3/camera/`

- **`angle.rs`** (38 lines): Camera graph selection from upstream. Skate 2 owns persistence.
  - types: `CameraAngle`
- **`collision.rs`** (67 lines): Host camera query adapter. The six source fat lines are preserved, and each candidate uses the shared recovered TU3 thin/fat triangle dispatcher.
  - types: `CameraCollision`
- **`graph.rs`** (128 lines): Stock default camera graph, using the same recovered dynamic controller as the action and motion graphs. Shot behavior side effects occur at Begin.
  - types: `CameraGraph`, `Behavior`, `CameraHost`
- **`graph_conditions.rs`** (136 lines): Camera condition leaves82DF4A90..82DF61DC, bound to the stock binary graph.
  - types: `Condition`, `Boolean`, `Value`
- **`graph_subject.rs`** (35 lines): Remaining physical subject getters consumed by the stock camera graph. These are publications from physics/animation, not deductions from the shot.
  - types: `CameraGraphSubject`, `CameraGraphEnvironment`
- **`publication.rs`** (386 lines): Game owners -> normal camera subject82DF69C0. Call after physical output. Missing publications are explicit inputs, never inferred from render pose.
  - types: `CameraStateOutput`, `CameraAnimationOutput`, `CameraAirOutput`, `CameraOffboardOutput`, `CameraGrindOutput`, `CameraEventsOutput`, `CameraPreferences`, `CameraPublicationInputs`
- **`runtime.rs`** (180 lines): Complete normal camera owner: subject publications, stock graph, CameraMan, swept world queries and retained query history. The simulation calls advance once per completed physical/animation publication; rendering only presents it.
  - types: `CameraRuntime`
- **`settings.rs`** (220 lines): Stock normal-camera rig settings, bound by TU3 8252E288/8252EDD0. Runtime offsets in camera kernels are relative to settings-object +16.
  - types: `RigSettingsSource`
- **`shake_data.rs`** (20 lines): Owned .shk parser; numerical sequence centering follows82E05DC0 in core.
- **`shot_data.rs`** (264 lines): Immutable stock shot definitions. Field order/units follow TU382CA4660.
  - types: `CameraShotTuning`, `StockShots`
- **`stock_names.rs`** (43 lines): Host decoding of the collection-name hash stored in stock RefSpec values. Same lookup8 byte format as the existing private XML extraction tools.
- **`subject.rs`** (69 lines): Typed physics/animation publication boundary for normal subject82DF69C0. The simulation supplies current physical outputs; camera history stays here.
  - types: `CameraSubjectSnapshot`, `SubjectPublisher`
- **`trajectory.rs`** (48 lines): Camera trajectory requests82DF9270/82DF9338/82DF9340. Rust executes the world work synchronously; PathEvaluator still consumes it on its next poll.
  - types: `TrajectoryResult`, `CameraTrajectory`
- **`world_query.rs`** (46 lines): Camera triangle leaf82771D08 stores the authored face normal, clamps every fraction and retains the first equal-distance hit in world traversal order.

### `crates/skate2-core/src/physics/modes/skate3/camera/tests/`

- **`runtime.rs`** (33 lines) *(tests)*
  - fns: `private_stock_camera_loads_every_graph_operation_and_shot_dependency`, `a_minimized_window_cannot_poison_the_aspect_ratio`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/`

- **`action.rs`** (469 lines): Typed ActionGraph registration and persistent operation instances. The schedule and concrete gameplay sinks are owned by the caller.
  - types: `PhysicalConditions`, `ActionHost`
- **`action_board_adjust.rs`** (40 lines): TU3 BoardAdjust instance+8/+12, Begin82BA2BA8, Update82BA2BC8.
  - types: `State`
- **`action_conditions.rs`** (94 lines): Original ActionGraph leaves that consume the prior animation/state output.
  - types: `ActionCondition`
- **`action_nodes.rs`** (229 lines): Authored ActionGraph intent behavior configuration.
  - types: `Parameter`, `ActionOperation`, `ActionInstance`, `ActionFactoryError`, `ActionFactory`, `UnsupportedOperation`, `ActionInstances`
- **`condition_nodes.rs`** (152 lines): Stock condition attributes bound to core evaluations. Values use the binary float/bool encodings directly, including constructor precedence and defaults.
- **`crouching_settings.rs`** (18 lines): anim_motion/crouching (global300) and auto_pump (global348), TU3 schema.
- **`mod.rs`** (63 lines): Production graph host building blocks. Root owns schedule and registration.
- **`motion.rs`** (439 lines)
  - types: `MotionPhysical`, `MotionHost`; fns: `end_gesture_channels`, `from_graph`, `accept_action_graph`, `run`, `condition_activation`, `context`, `allocate`, `begin`
- **`motion_air_dismounting.rs`** (64 lines): TU3 AirDismounting Begin82BB93E0, Update82BB94C0, empty End82B61BB8.
  - types: `State`
- **`motion_air_execute.rs`** (101 lines): Original airborne MotionGraph lifecycles on the production host.
- **`motion_air_leg.rs`** (47 lines): Stock ControlAirLegExtension configuration82BAF450 and settings Globals336.
  - types: `Operation`
- **`motion_animation.rs`** (550 lines): Stock tree ownership and persistent playback used directly by MotionHost.
  - types: `MotionAnimation`
- **`motion_bump.rs`** (32 lines): SetBumpCoefficients Begin82BB0660, original Skate 3 TU3.
  - types: `Settings`
- **`motion_channels.rs`** (295 lines): Owned counterpart of ChannelAnimatable's ordered persistent overlays.
  - types: `Channel`, `MotionChannels`, `FakieHead`
- **`motion_character_gesture.rs`** (347 lines): Original CharacterGesture82BAA320, instance82BAADD0 and helpers 82BAAFB0/82BAB080/82BAB258/82BAAED8. Allocation and channel names are owned here; the original entry/cycle/exit decisions and channel timing are kept.
  - types: `CharacterGestureInputs`, `GesturePublication`, `CharacterGesture`
- **`motion_conditions.rs`** (455 lines): Concrete MotionGraph condition leaves and stock attributes.
  - types: `MotionCondition`
- **`motion_execute.rs`** (142 lines)
  - fns: `execute`
- **`motion_execute_leaf.rs`** (691 lines)
  - fns: `execute`
- **`motion_fakie_hold.rs`** (83 lines): Trainer gates for the stock automatic fakie-to-switch animation transitions. FakieTurn is player steering and must never be suppressed by this option.
  - types: `Tree`
- **`motion_filter_execute.rs`** (40 lines): Live stock intent filtering shares the existing MotionAnimation maps.
- **`motion_finger_flip.rs`** (82 lines): FingerFlipOut: TU3 Begin82BB3198, Update82BB7C40, empty End82B61BB8.
  - types: `Settings`, `State`
- **`motion_gameplay_conditions.rs`** (276 lines): Original TU3 physical-state condition leaves used by the stock MotionGraph.
  - types: `GameplayConditions`, `GameplayCondition`
- **`motion_grind.rs`** (159 lines): Original S3 grind graph lifecycle, using the current animation owner. No animation choice, physical-state transition or synthetic observation.
  - types: `Operation`, `State`, `Physical`
- **`motion_ground_slope.rs`** (35 lines): GroundSlopeType: TU3 ctor82BA7BD8 and condition82BA7E80.
  - types: `GroundSlopeType`
- **`motion_hand_services.rs`** (96 lines): TU3 HandBusy and MaintainShove lifecycle, independently recovered from IDA. HandBusy: factory82BC8B08, Begin82BAB4D0, End82BAB588. MaintainShove: factory82BCB1C8, Update82BBD690, End82BBD6E8.
  - types: `Operation`, `Hand`, `HandServices`
- **`motion_hippy_jump.rs`** (101 lines): Owned implementation of `Sk8::Behaviours::HippyJumpAntic::Update`.
  - types: `Settings`, `State`
- **`motion_hooks.rs`** (33 lines): Native MotionGraph transition hooks, dispatched by the stock controller.
  - types: `MotionHook`
- **`motion_instances.rs`** (118 lines): Per-node allocation retained by the production MotionHost.
  - types: `Instance`
- **`motion_intent_filter.rs`** (73 lines): Stock FilterMotionGraphIntent constructor and map lifecycle adapter.
  - types: `Operation`
- **`motion_kickturn.rs`** (81 lines): Original TU3 low-speed KickTurn behaviors, connected to MotionHost.
  - types: `Operation`
- **`motion_landing.rs`** (114 lines): Original TU3 landing graph configuration and persistent instance state.
  - types: `Operation`, `Condition`, `Flags`, `Physical`, `State`
- **`motion_landing_execute.rs`** (116 lines): Original landing Begin/Update/End effects, called by the production graph.
- **`motion_manual.rs`** (16 lines): Stock manual animation data. Physical controller state remains in Ground.
- **`motion_moving_object.rs`** (397 lines): TU3 MovingObject (ctor 0x82bb64b8, Begin 0x82bb6860, Update 0x82bb6a20, ClampAngle 0x82bb79d8), InitMovingObjects::Begin 0x82bb63d8 and AngleAdjustByAnimationEndLessThan (factory 0x82bc6398, Activation 0x82ba7598) over SkaterAnim's 0x28-byte MovingObjectsCoefficients. Skate 2's MovingObject is the same code reading the Animation_MovingObjects cvars; TU3 folds their defaults into constants.
  - types: `Coefficients`, `Physical`, `Behaviour`, `Settings`, `MovingObjects`
- **`motion_native.rs`** (143 lines): Small original MotionGraph leaves used by the live riding ancestry.
  - types: `Physical`, `GesturePhysical`, `Operation`, `ScorePacket`, `LandingData`
- **`motion_nodes.rs`** (301 lines): Stock MotionGraph configuration. Native constructor defaults are kept here; lifecycle execution and persistent state are owned by MotionHost.
  - types: `MotionOperation`, `MotionFactory`
- **`motion_riding.rs`** (241 lines): Native riding packet and timer leaves. The graph host owns these states; physics/pose callers consume the resulting values in actor update order.
  - types: `ForceMode`, `RidingOperation`, `RidingState`
- **`motion_riding_conditions.rs`** (168 lines): Original TU3 random, COM velocity and slope conditions for stock riding.
  - types: `RidingConditionInputs`, `VelocityAxis`, `MotionRidingCondition`, `MotionRandom`
- **`motion_runout.rs`** (58 lines): TU3 AddRunoutAttribs: factory82BC9BA8, vtable82320BDC, Begin82BBA4A8, Update82BBA770, End82B61BB8.
  - types: `Physical`, `Parameters`
- **`motion_score_execute.rs`** (24 lines): Stateless score and bump lifecycles using the same MotionHost publication.
- **`motion_scoring_trick.rs`** (39 lines): ScoringTrick lifecycle from original Skate 3 TU3 82BCC030/82BBFA88. Update publishes the same authored encoded name to both score-name slots; Begin and End are empty. No animation, force, or trick-selection side effect.
  - types: `Operation`, `Names`
- **`motion_shove.rs`** (234 lines): Original TU3 Shove constructor82BBCCF0, Begin82BBCDD0, Update82BBCDF0 and End82BBD540. The host owns channels and graph values.
  - types: `ShoveOperation`, `ShovePhysical`, `ShoveState`
- **`motion_sliding.rs`** (252 lines): PowerSliding Update82BB28E8, instance82BB2D70 and helper82595930.
  - types: `Settings`, `Operation`, `State`
- **`motion_sliding_execute.rs`** (140 lines): Production lifecycle for the authored slide family; no detached state copies.
- **`motion_spin.rs`** (298 lines): BodySpin82BAB640, including its native five-state channel controller.
  - types: `Settings`, `PrelandingPhysical`, `State`
- **`motion_stock_conditions.rs`** (80 lines): Recovered stock predicates using completed physical publications. Unsupported archive guesses are deliberately left to the graph's diagnostic path.
  - types: `Condition`
- **`motion_stock_execute.rs`** (89 lines): Imported grab/tweak and moving-object registry behavior dispatch.
- **`motion_stock_gameplay.rs`** (177 lines): Typed bindings for the remaining stock MotionGraph gameplay operations.
  - types: `GrabType`, `Operation`
- **`motion_toggle_board.rs`** (224 lines): TU3 ToggleBoard82BA8FD8/82BA8FE8, retrieval82BA9310/82BA9E20, drop82BA9990. Instance and channel clocks remain owned by the graph.
  - types: `Physical`, `ToggleBoard`
- **`motion_toggle_execute.rs`** (27 lines): Persistent ToggleBoard instance and shared animation-owner dispatch.
- **`motion_tricks.rs`** (226 lines): Original trick leaves. Registration strings, not community symbol guesses, identify SetTrickAttr82BC7848 and ScoringTrick82BCC030.
  - types: `Operation`, `Requests`
- **`motion_wipeout.rs`** (125 lines): Stock Wipeout graph leaves, using completed physical output.
  - types: `Operation`, `Controls`, `State`, `Settings`, `Physical`, `Condition`
- **`motion_wipeout_execute.rs`** (196 lines): Original82BBFF88/82BC01C0; graph ownership is ours, filter order is stock.
- **`outputs.rs`** (306 lines): Explicit graph-phase transport.
  - types: `AuthoredSignals`, `WipeoutControls`, `BoardControls`, `ActionControls`, `MotionEffects`, `ActionInput`, `PriorMotionState`, `GraphDiagnostics`, `GraphCapabilityReport`, `TurningOutput`
- **`pumping_settings.rs`** (9 lines): Stock anim_motion/anim_pump; TU3 global308 layout offsets1152/1708..1784.
- **`pushing.rs`** (249 lines): MotionGraph pushing operations. The stock graph owns activation and order; these handlers only mutate native push properties and animation attributes.
  - types: `PushOperation`, `SettableAttribute`, `PushAnimationSink`, `PushContext`, `PushInstance`
- **`pushing_settings.rs`** (189 lines): Stock pushing settings and authored clip metadata (TU382B953E8..82B9584C).
  - types: `PushTreeAttribute`, `PushTreeMetadata`, `PushTreeSource`, `PushingSettings`
- **`turning_settings.rs`** (36 lines): SetTurning82BB3830/TurnRemap82BB4248 stock layout binding. anim_carving/default's schema is752 bytes; native global slot372 selects it.

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_animation/`

- **`selection_space_host.rs`** (44 lines): Deferred raw-tree callback in SelectionSpace::SetAttributes.
- **`stock_clip_query.rs`** (24 lines): Direct named-bank query82D16680, not the currently playing tree/channel.
- **`tree_builder.rs`** (104 lines): Stock authored tree construction; runtime selection remains in the tree.

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/`

- **`animation.rs`** (50 lines): Begin82BB0A30: live set/apply/query, including partial output on a miss.
  - types: `Animation`
- **`animation_owner.rs`** (38 lines): Compile as a child of motion_animation, NOT motion_grind, so the real owner's private current tree stays private. See the module line in handoff.
- **`animation_owner_tests.rs`** (85 lines) *(tests)*
  - fns: `owner`, `actual_motion_animation_adapter_uses_live_queue_root_and_packet_owner`, `actual_owner_missing_tree_is_an_error_not_fabricated_endpoints`
- **`conditions.rs`** (61 lines): Original S3 graph predicates. Producers and shared dispatch are external.
  - types: `Condition`, `Physical`
- **`settings.rs`** (27 lines)
  - types: `Settings`; fns: `load`
- **`tests.rs`** (209 lines) *(tests)*
  - fns: `attributes`, `settings`, `physical`, `parser_preserves_authored_crouch_name_and_rejects_missing_fade_attributes`, `creates_ordered_packets_not_intents_or_settable_parameters`, `crouch_begins_from_physical_height_updates_authored_parameter_and_ends_silently`, `fade_begin_applies_height_before_endpoints_and_keeps_live_tree_effects`, `endpoint_miss_keeps_original_zero_or_partial_output`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_grind/tests/`

- **`conditions.rs`** (51 lines) *(tests)*
  - fns: `physical`, `conditions_use_their_actual_fields_not_a_shared_grinding_proxy`, `approach_case_folds_facing_but_trick_out_type_uses_exact_strings`
- **`fixture.rs`** (105 lines) *(tests)*: Independent synthetic data, using the genuine core playback pipeline.
  - types: `Owner`
- **`routing.rs`** (74 lines) *(tests)*
  - fns: `production_factory_routes_three_handlers_and_all_five_conditions_to_grind_owner`, `production_factory_preserves_required_grind_attribute_errors`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_hand_services/`

- **`tests.rs`** (90 lines) *(tests)*
  - fns: `nested_hand_lifetimes_preserve_other_owners`, `retail_counter_stores_wrap_without_saturating`, `maintain_shove_is_an_update_latch_not_a_reference_count`, `hand_selection_uses_exact_stock_comparison`, `maintain_end_fades_only_antic_through_existing_animation_runtime`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/`

- **`air.rs`** (177 lines): Offboard animation lifecycle:82BB9F48/A070 and82BBACC0/ACE0/B418.
  - types: `Operation`, `State`
- **`body_tweak.rs`** (180 lines): TU3 OffboardBodyTweakBlend: ctor82BBAB98, Begin82BBACC0, Update82BBACE0, End82BBB418, instance82BBB4D0. Original IDA only.
  - types: `Settings`, `Physical`, `State`
- **`cadence.rs`** (101 lines): BipedCadence82BB9E78, MatchCadence82BBA938/82BBAA00 and LocoState82BA7B50. TU3 default.patched.xex SHA256 431b8eba23565affdc10d137df19b06fe286244cefb3e1a13f32693e9600395a.
  - types: `Operation`, `MatchCadence`, `LocoState`
- **`dismount.rs`** (22 lines): DisableDismount condition82BA5368 in the original Skate3 TU3 executable.
  - types: `Condition`
- **`match_air_time.rs`** (75 lines): Native MatchAirTime: factory82BCA308, Begin82BB9F48, Update82BBA070, instance82BBA390. End is the empty82B61BB8.
  - types: `State`
- **`push_off.rs`** (21 lines): IsPushOffEnabled: original TU3 registration82F88150 -> ctor82BC62F8. The ctor installs vtable8231F6D4; predicate slot+48 is8281DD70. Original bytes386000014E800020 are li r3,1; blr. There is no physical flag read, time gate or runtime setting in this leaf. The graph executor remains responsible for the common active/mask/negation semantics.
  - types: `IsPushOffEnabled`
- **`reset.rs`** (46 lines): Original registered reset handlers:82F88440/82F89600. Both have Begin effects; Update and End point to82B61BB8 (raw blr).
  - types: `Operation`, `ResetOwners`
- **`runout.rs`** (99 lines): AddRunoutAttribs: Begin82BBA4A8, Update82BBA770, state82BBA830. End is82B61BB8. Values are sampled once per activation, never on Update.
  - types: `Operation`, `Observation`, `State`
- **`toggle_board.rs`** (144 lines): Original ToggleBoard lifecycle adapter, TU3 82BA8FD8/8FE8 and 82B61BB8.
  - types: `Physical`
- **`twist_lean.rs`** (50 lines): MatchTwistAndLean stock configuration and existing animation queue adapter.
  - types: `Operation`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/body_tweak/`

- **`tests.rs`** (106 lines) *(tests)*
  - fns: `native_air_tweak_release_latch_keeps_signed_axes_and_strict_counter`, `native_air_tweak_begin_preserves_filters_and_center_request_decays_them`, `stock_air_tweak_channels_parameters_pose_requests_and_teardown`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/cadence/`

- **`tests.rs`** (64 lines) *(tests)*
  - fns: `biped_updates_the_existing_phase_only_during_update`, `match_captures_begin_and_republishes_without_consuming_it`, `absent_components_preserve_retained_capture_and_pending_flag`, `native_initial_capture_is_zero_before_any_begin`, `locomotion_comparison_does_not_clamp_unknown_physical_values`, `missing_locomotion_config_is_not_silently_stand`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/match_air_time/`

- **`tests.rs`** (136 lines) *(tests)*
  - fns: `match_air_time_first_update_and_reentry_skip_seek_then_use_remaining_over_total`, `match_air_time_bounds_displacement_by_xyz_length_and_scales_all_lanes`, `match_air_time_publishes_five_selection_parameters_from_correct_owners`, `stock_match_air_time_selects_jump_advances_pose_and_preserves_landing_identity`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/runout/`

- **`tests.rs`** (65 lines) *(tests)*
  - fns: `observation`, `offboard_selection_and_mirror_are_sampled_at_begin`, `update_reuses_capture_and_preserves_queue_order_and_flags`, `absent_begin_does_not_fabricate_speed_and_stationary_begin_is_valid`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_offboard/toggle_board/`

- **`tests.rs`** (98 lines) *(tests)*
  - fns: `animation`, `physical`, `dropping_terminal_tick_publishes_physical_attribute_and_animation_parameter`, `all_toggle_publications_reach_physics_attributes_and_animation_parameters`, `lifecycle_end_leaves_persistent_channel_and_begin_resets_state`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/motion_stock_conditions/`

- **`tests.rs`** (58 lines) *(tests)*
  - fns: `obstacle_distance_uses_named_clip_displacement_and_native_boundary`, `stop_animation_query_uses_database_and_ignores_event_time_gate`, `stock_stop_clips_drive_obstacle_distance_predicate`

### `crates/skate2-core/src/physics/modes/skate3/graph_host/tests/`

- **`animation_jump_into.rs`** (69 lines) *(tests)*
  - fns: `owner`, `clip`, `jump_into_queries_inactive_marker_begin_and_resets_clip_history`, `jump_into_applies_pending_blend_before_reading_marker_time`, `jump_into_missing_marker_preserves_time_without_emitting_packets`
- **`motion_air_dismounting.rs`** (49 lines) *(tests)*
  - fns: `air_dismounting_stock_crash_node_3207_publishes_only_on_clip_marker`
- **`motion_board_adjust.rs`** (72 lines) *(tests)*
  - fns: `board_adjust_stock_release_keeps_the_authored_air_blend`
- **`motion_bump.rs`** (194 lines) *(tests)*
  - fns: `finite`, `stock_bump_crash_regression`
- **`motion_deck_angles.rs`** (49 lines) *(tests)*
  - fns: `stock_deck_angles_copy_completed_output_only_on_begin`
- **`motion_finger_flip.rs`** (66 lines) *(tests)*
  - fns: `stock_finger_flip_crash_node_3080_lifecycle_and_intent_source`
- **`motion_jump_into.rs`** (69 lines) *(tests)*
  - fns: `stock_jump_into_seeks_once_per_entry_and_consumes_misses`
- **`motion_slide.rs`** (278 lines) *(tests)*: Stock bindings plus production Host lifecycle: a split latch breaks this test.
- **`motion_stock.rs`** (160 lines) *(tests)*
  - fns: `stock_motion_host_loads_graph_settings_and_authored_riding_tree`, `count`, `stock_motion_factory_support_inventory`
- **`motion_tricks.rs`** (263 lines) *(tests)*
  - fns: `stock_360_crouch_takeoff_and_air_animation_pipeline`, `check_expression`
- **`pushing.rs`** (388 lines) *(tests)*
  - types: `Trees`; fns: `settings`, `state`, `context`, `authored_defaults_match_tu3_factories`, `first_update_only_blends_once_per_activation_and_publishes_in_source_order`, `target_and_out_factor_select_attributes_using_switch_state`, `push_out_projects_actual_selected_foot_and_publishes_only_on_begin`, `settable_sink_updates_first_matching_name_and_sequence_without_reordering`
- **`pushing_metadata.rs`** (24 lines) *(tests)*
  - fns: `stock_push_metrics_load_through_the_runtime_data_path`

### `crates/skate2-core/src/physics/modes/skate3/grind_world/`

- **`host.rs`** (130 lines): Native host assets through the existing source loose-octree/contact path.
- **`octree.rs`** (151 lines): Original S3 loose octree insertion/query order, 82AD6830/82476158. Host-owned nodes retain native head insertion, split redistribution and LIFO candidate traversal because result capacity is observably bounded.
  - types: `Bounds`, `Bucket`, `Node`, `Octree`
- **`provider.rs`** (204 lines): Static stock assets only. Registration follows package source occurrence order; native octree order is retained within each registered section.
  - types: `SourceIdentity`, `Asset`, `StaticProvider`
- **`spline.rs`** (147 lines): User-authorized ZIP spline adapter, checked against original TU3 82C1E568, 82C1E6C0, 82C1EEF0 and 82C1F098 in default.patched.xex (431b8eba...). Keep the native cubic payload; the game's contact primitive is its chord D -> (A+B)+(C+D), including for retail cubic segments.
  - types: `PrimitiveMetadata`
- **`tests.rs`** (150 lines) *(tests)*
  - fns: `native`, `no_map_keeps_current_flat_course_empty`, `native_tiny_and_duplicate_knots_are_not_dropped`, `same_native_spline_id_does_not_alias_header_instances`, `provider_keeps_both_header_guids_and_source_rail_identity`, `vertical_flag_uses_horizontal_tolerance_not_chord_length`, `native_payload_and_inclusive_link_bounds_survive_conversion`, `octree_resident_head_insertion_and_forty_cap`

### `crates/skate2-core/src/physics/modes/skate3/input/`

- **`controllers.rs`** (163 lines): Persistent platform-input owner. Device collection runs on host frames; this does not choose the skater simulation clock or advance Derived input.
  - types: `ControllerStatus`, `ControllerInput`, `RawInput`
- **`gesture_catalog.rs`** (150 lines): Original TU3 gesture membership,82BA10C0 ->82BA07F0. The catalog is constructed by82B98DB8 ->82B98F70, independently checked against all270 raw map-insertion calls. PAT paths do not define these groups.
  - types: `Group`
- **`gesture_input.rs`** (210 lines): GameInputManager82696030/826962D8 -> listener8259B878/8259B9D0 -> Fill8259B1F0..B7D8. Runs on the existing 60 Hz gameplay input boundary.
  - types: `GestureInput`
- **`gesture_mapping.rs`** (126 lines): GestureTrickMapping82BA07F0 and CreateTrickIntentFromGesture82BA1C30..2228.
  - types: `State`
- **`gesture_mapping_data.rs`** (414 lines): TU3 82B98F70 records, re-extracted from original bytes and all 270 map inserts.
- **`platform.rs`** (20 lines): Windows device transport. Raw signed axes/trigger bytes reach the TU3 converter without Bevy/gilrs deadzones or normalized-axis reconstruction.
  - types: `DevicePacket`, `DeviceError`

### `crates/skate2-core/src/physics/modes/skate3/input/tests/`

- **`controllers.rs`** (147 lines) *(tests)*
  - fns: `sample_raw_for_test`, `packet`, `raw_packets_reach_stock_actions_without_losing_device_identity`, `unchanged_packet_numbers_do_not_suppress_publication_and_empty_drain_does_not_advance`, `disconnect_publishes_zero_count_and_reconnect_reinitializes_pad_records`, `alternate_guitar_subtype_uses_verified_conversion_mask_and_errors_stay_visible`, `tick_input_is_the_single_fixed_tick_snapshot`, `unavailable_controllers_publish_zero_gameplay_actions`

### `crates/skate2-core/src/physics/modes/skate3/modding/`

- **`moving_queries.rs`** (141 lines): Solid dual-world boundary. Native local bones participate in BoardStep; remote actor mirrors participate in Rapier. Never solve the local actor in both worlds, and never feed reactions back to another peer's body.
  - types: `MeshQuery`, `MovingQueries`
- **`player_physics.rs`** (641 lines): Native skater contact observations and temporary, mod-owned joint overrides.
  - types: `ContactFrame`, `PartRestore`

### `crates/skate2-core/src/physics/modes/skate3/physics/`

- **`air_phase.rs`** (242 lines): PhysicsAir200 production lifecycle82D34388/82D346C0/82D346A0. Uses the live player, board, skeleton and selector; no alternate air solver.
- **`air_reckoning.rs`** (95 lines): Production stock settings and live-input adapter for Reckoning82D8DBD8.
  - types: `AirReckoning`
- **`animated_skeleton.rs`** (278 lines): Stock skeleton-to-physics ownership. Animation globals and real physical observations enter here; native calculations and history stay in core.
  - types: `AnimatedSkeleton`
- **`animation_feedback.rs`** (76 lines): Persistent physical-to-animation conditioning with stock collection values.
  - types: `AnimationFeedback`
- **`animation_feedback_settings.rs`** (61 lines): PhysOutConditioner_Animation turn settings, TU3 global204/physics_animation.
- **`animation_grind.rs`** (125 lines): Completed physical publication -> both stock graphs. No state selection.
- **`animation_grind_tests.rs`** (83 lines) *(tests)*: Independent publication tests; these do not simulate physical producers.
- **`animation_input.rs`** (174 lines): Animation attributes become owned physical inputs through the actual Skeleton dispatcher. This adapter loads stock settings and retains caches; it does not choose animations or manufacture a push from controller input.
  - types: `AudioEvents`, `AnimationInput`
- **`animation_phase.rs`** (511 lines): The production physical-output -> stock graph -> next physical-input phase. Native actor phases82592FD8/82593128/82593230/82593640 preserve this order.
- **`animation_phase_packet.rs`** (199 lines): Owned PlayerInput packet storage; original actor publication825937EC.
  - types: `AnimationProfile`, `AnimationPhaseOutput`
- **`biped_air.rs`** (105 lines): Distinct State501 lifecycle: original S3 82D2E5C0/EF38/FFF8/30300/EF20.
  - types: `BipedAir`
- **`biped_ground.rs`** (528 lines)
  - types: `PushingFollow`, `Owner`; fns: `finish_reckoning`, `air_settings`, `turn_vs_speed`, `load`, `reset`, `follow`, `follow_result`, `run`
- **`boneless.rs`** (140 lines): Physical state602, TU3 vtable82327434. Stock no-comply/boneless/fastplant animations select this state with AnimSkateboard and launch with FootJump.
  - types: `Boneless`, `LaunchInput`, `LaunchOutput`
- **`camera_output.rs`** (222 lines): Physical-output publications consumed by gameplay camera82DF69C0. Reset82DE53F0 runs before the actual producers82DB6EC0/82D3A388.
  - types: `StaticWorld`
- **`camera_output_tests.rs`** (46 lines) *(tests)*
  - fns: `offboard_camera_consumes_the_distinct_completed_output_fields`, `reset_offboard_output_does_not_invent_a_valid_trajectory`
- **`camera_tricks.rs`** (81 lines)
  - fns: `tricks_select_air_camera_without_chasing_the_board_upward`, `replay`
- **`clock.rs`** (71 lines): Offline tick cadence. Original TU3 Simulation::OnRequestSetSimRate82857EC8 sets a wall-clock timer; it does not change the physical integration step.
  - types: `SimulationClock`
- **`colliders.rs`** (150 lines): Bind stock child geometry to the live solver part transforms.
- **`collision_groups.rs`** (52 lines): Original TU3 assembly/part collision groups shared by world and actor queries.
- **`controls.rs`** (306 lines): Controller-derived state is evaluated on the same fixed gameplay tick.
  - types: `PlayerControls`, `SimulationActions`
- **`dev_trace.rs`** (63 lines): Diagnostic observations only: never repair, reset or clamp simulation values.
  - types: `TickGuard`
- **`dev_trace_buffer.rs`** (59 lines): Bounded development-only flight recorder. No disk writes during healthy ticks.
  - types: `Buffer`
- **`foot_ik.rs`** (225 lines): Physical skeleton IK data binding. This is the production skeleton's IK owner, using authored physics volumes and the real reparented animation targets.
  - types: `FootIk`, `PhysicalInput`, `DriveOutput`
- **`foot_ik_queries.rs`** (186 lines): PhysicalPlayerHiLOD StartSkeletonLineTests82DB63C0 / result publication 82DB6580. These are radius-bearing trajectory tests from actual physical parts.
  - types: `SkeletonLineHit`, `SkeletonLineTests`
- **`foot_physical_output.rs`** (38 lines): Persistent original physical-foot output owner. Call after final physical record correction, at Skeleton::FillPhysOut82BE2138/2140 publication time.
  - types: `FootPhysicalOutputs`
- **`footplant.rs`** (158 lines): FootPlantManager state, original TU3 inline ctor82DB217C and Reset82D6F440. Query results are retained separately from pending query ownership.
  - types: `Footplant`
- **`frame.rs`** (406 lines): One production tick: controller graphs, physical input/state, shared solve, physical animation output and the normal gameplay camera.
  - types: `PreparedFrame`
- **`grind.rs`** (56 lines): ZIP grind runtime split into source-closed components against CURRENT APIs. Physical acquisition remains with the input manager. The retained runtime owns family state, body operations and publication, never candidate selection.
  - types: `Family`
- **`grind_air_settings.rs`** (68 lines): Stock GrindAirAdjust settings82D72B40; lengths from the actual deck/truck collections.
- **`grind_camera.rs`** (84 lines): ZIP contact conditioner82DF0640, corroborated against S2 82E2D000. Tangent selection82D87460 belongs to the manager, not to this conditioner.
  - types: `Contact`, `GrindCamera`
- **`grind_chromosome.rs`** (251 lines): ZIP chromosome port corrected from S3 82DEE918/82DEF518/82DEE508. S2 82E2E9E0/82E2EBE8/82E2F9D0 confirms history/publication structure; only S3 has four-way orientation for5050/5O and the sixth darkslide family.
  - types: `Components`, `Publication`, `Chromosome`
- **`grind_host.rs`** (41 lines): Bind the manager to the current board materials and authored world queries.
  - types: `LiveHost`
- **`grind_materials.rs`** (49 lines): 82D89DC0/82C090C0 material modes, applied to the live board collider slots. Init82C08C28 clears all three grind materials at82C09084..A4; S2 Init 82B374E8 confirms their identities. Grind resistance is a separate force.
  - types: `GrindMaterials`
- **`grind_names.rs`** (50 lines): ZIP short names verified entry-for-entry from original S3 initializer82DE6678. 82F92AC0 splits384 records;82DEE508 publishes skating IDs and scoring IDs separately. S2 82E2F9D0 uses160 records (two orientations, five families).
  - types: `Name`
- **`grind_trick.rs`** (201 lines): TU3 PHYSICS_STATE_GRIND_TRICK202, vtable82327174, owner PhysicalPlayer+1716. Enter82D335F8, Exit82D33678, Update82D336F8, board82D33928, post82D33A70. FillPhysOut is empty; PredictFutureOfDeck shares82D34DA8.
  - types: `GrindTrick`
- **`ground.rs`** (274 lines): Our level geometry, queried with the recovered physics/world calculations.
  - types: `Terrain`
- **`ground_animation.rs`** (90 lines): GroundAnimation103 lifecycle82D33BF8/33DB8/33C70/34150, original Skate3TU3.
  - types: `GroundAnimationRuntime`
- **`ground_exit.rs`** (24 lines): Complete PhysicsGround Exit82D37B20, on the real board and controller owners.
- **`ground_phase.rs`** (302 lines): Production Ground entry82D37538 and update82D37C88/82D38800. State selection belongs to the player coordinator. This phase consumes its current processed packet and changes the same board that the solver advances.
  - types: `GroundEdge`, `GroundLifecycle`
- **`handplant.rs`** (316 lines): TU3 HandPlantManager82D61040 and physical state600 (823273CC).
  - types: `Handplant`
- **`input_phase.rs`** (300 lines): Connect the animation packet to the one physical player and skeleton. PlayerInput retains the original ordering; callbacks split host ownership.
  - types: `Callbacks`
- **`input_teleport.rs`** (269 lines): Ordinary local-player teleport82DB8998, operating on the live assemblies.
- **`input_transition_tests.rs`** (35 lines) *(tests)*: Exercise the production vert-input adapter from raw controller samples.
- **`known_air.rs`** (202 lines): Native KnownAir201 lifecycle82D352D0/82D35C58/82D35920/82D36590/82D36880. All phase adapters borrow the production owners; the state copy below is a stack-local value while those owners are borrowed, then published once.
  - types: `KnownAir`
- **`landing_on_deck.rs`** (193 lines): State503 lifecycle from original TU3 82D4D418..82D4E100.
  - types: `Runtime`
- **`landing_quality.rs`** (18 lines): Stock PhysicsAnimation cache +560/+576 and +592/+608, TU3 82DE6480.
- **`offboard_audit_trace.rs`** (99 lines): TEMPORARY bottom-up audit observations; not native gameplay behavior. Opt in with SKATE3_AUDIT_OFFBOARD_TICKS=start:end (inclusive simulation ticks). Disabled by default. No gameplay state, guards, poses or forces are changed.
- **`onboard_correction_tests.rs`** (136 lines) *(tests)*: Real owner/helper integration with private stock construction. Synthetic requests isolate latch semantics; these are not a recorded failure replay.
- **`plant_skeleton.rs`** (92 lines): Skeleton82BDEC48 (anchored foot) and82BDF090 (plant COM).
- **`player_state.rs`** (181 lines): PhysicalPlayer startup, PostInput, state selection and state publication. Native control decisions use original TU3 functions and stock collections; host ownership replaces the original player component pointers.
  - types: `PlayerState`
- **`render_pose.rs`** (97 lines): Finish the actual physical pose and publish it to the single render skin. Call after collision/error/COM output and before the next animation update.
- **`respawn.rs`** (477 lines): TU3 actor checkpoint manager, bound to the single-player static scene.
  - types: `Runtime`, `Settings`, `RespawnHost`, `LivingWorldRadii`, `Scene`
- **`revert_state.rs`** (270 lines): RevertGround102, TU3 vtable82327330: Enter82D43488, Update82D43518, empty Exit, Ground Post82D4C070, Fill82D43B10. Stock layout018E8A2E5028AB3F.
  - types: `RevertState`
- **`riding_outputs.rs`** (392 lines): Live board/query/solver outputs used by the riding gameplay owner. Physical calculations stay in core; this owns stock settings and scheduling state. Solved contacts and line probes remain distinct native inputs.
  - types: `RidingPoseInputs`, `RidingOutputs`
- **`settings.rs`** (160 lines): Stock skater XML values bound to the recovered physical calculations.
  - types: `PhysicsSettings`
- **`shove_probe.rs`** (186 lines) *(tests)*: TU3 `PhysicalPlayerHiLOD` 0x82db3ce8's shove probe (0x82d7a440), the pass before `Update1_ProcessInput` copies it to Processed+1792 and resets it (0x82d7a330). While the shove behaviour holds state flag 86 it finds a pedestrian within reach of the hands (`LivingWorld::Manager` 0x826bed80, answered by the host) and tests it against the strike zone (0x82d7a770). Every other skater is a type 1 target first...
  - types: `ShoveFrame`, `ShovePedestrian`, `ShoveSkater`, `ShoveHit`, `ShoveProbe`
- **`skateboard_controller.rs`** (13 lines): Actual hand-controller lifecycle and board contact producer binding.
- **`skater.rs`** (385 lines): The game's single skater: stock graph animation and its physical skeleton. Loading creates the real bodies from the animator's initial evaluated pose.
  - types: `Skate3ControllerState`
- **`skeleton_air.rs`** (179 lines): Production animated-board and COM-controlled air Skeleton updates. Original TU3 82BDDA10 / 82BDE600. All owners are the same objects used by ProcessData, GeneralUpdate, the shared solve and final pose publication.
  - types: `SkeletonAir`
- **`skeleton_body.rs`** (324 lines): Stock physical body construction. Initial mapped pose and spawn are real actor/physics inputs supplied by the coordinator; no pose or COM fallback.
- **`skeleton_body_collision_settings.rs`** (82 lines): Original82BE7860/82BE7280 and exact locations in stock physics_skeleton XML.
- **`skeleton_body_drive_settings.rs`** (57 lines): Original BoneDrives constructor82BEC4A0/82BEC7B0 collection bindings. 82BEC50C..544 builds local=00132D1E84F0BC32;82BEC5B4..D4 builds root=550B23128E0C3838. The12 transition keys preserve native offsets232..276.
- **`skeleton_colliders.rs`** (114 lines): Bind original physical skater shapes to the bodies receiving solver reactions. The collision owner selects enabled parts and materials; shapes and local-Z capsule axes come from the same stock definition used to construct each part.
- **`skeleton_controller.rs`** (85 lines): Shared collision-policy controller used by Ground and both Air states. Requests82D911A8 and ascending/falling update82D91660 precede the solve.
  - types: `SkeletonControllerState`
- **`skeleton_feedback.rs`** (374 lines): Post-solver contact reports and physical/animation errors for the live skater. Engine report storage is ours; spy arithmetic and report order are native.
  - types: `HostContact`
- **`skeleton_grind_air.rs`** (48 lines): 82BDCFB0 at the existing ProcessData boundary; never a physical teleport.
- **`skeleton_input_general.rs`** (152 lines): Movement-phase continuation of the original Skeleton input owner.
- **`skeleton_input_runtime.rs`** (269 lines): The production Skeleton::ProcessData82BD8918 boundary. All borrows point at the same owners subsequently used by Ground, the solve and pose output. GeneralUpdate belongs to the later movement-state phase, not PlayerInput.
  - types: `CollisionInput`, `SkeletonOwners`, `SkeletonPoseInput`, `SkeletonInputRuntime`
- **`skeleton_input_teleport.rs`** (92 lines): Actual teleport skeleton stages82BE3508 and82BDF7C0. The player owns the intervening ResetSystems and ProcessData calls; this is the same skeleton.
- **`skeleton_output.rs`** (178 lines): Production adapter for the original physical-pose job82DB6698/82DB95E8.
  - types: `SkeletonOutput`
- **`slide_state.rs`** (67 lines): PhysicsSlideGround101 production lifecycle, original TU3 vtable823285B4.
  - types: `SlideState`
- **`solid_contacts.rs`** (230 lines): Solid Rapier/native solver boundary. Native bone/board primitives remain native; convex objects are tested with their actual Parry support shapes.
- **`solve.rs`** (185 lines): One physical solve for the board, skater and original animation targets. The caller publishes forces and drive targets before entering this phase.
- **`startup_check.rs`** (113 lines): Map startup checks for `--validate-maps` (see crate::skate3_native::map_validation). Mirrors the private `map_startup` test: spawn support, deck pose and a short neutral startup must reach supported wheels with finite state. Results are reported, not asserted.
  - types: `StartupReport`
- **`teleport_state.rs`** (128 lines): Host checkpoint response to original actor reset callback82592518. Shared scheduling preserves request -> next input reply ->702 output -> reset.
  - types: `Checkpoint`, `Runtime`
- **`wipeout.rs`** (167 lines): Shared Ground/Air wipeout requests from original TU3 82D8F9E0/82D90358.
  - types: `Wipeout`
- **`world_proxies.rs`** (119 lines): The Skate 2 world's moving bodies as extra bodies of the Skate 3 player's solve: each joins at its centre of mass as a finite-mass shadow, and only a body this game moves keeps the reaction the solve gives it.
  - types: `Proxies`

### `crates/skate2-core/src/physics/modes/skate3/physics/air_phase/`

- **`board.rs`** (59 lines): 82D34AB0 consumes the completed Air reckoning's collision normal1216.
- **`input.rs`** (119 lines): Actual Processed/animation/stock owners read by PhysicsAir and its launch.
  - types: `AirSettings`
- **`skeleton.rs`** (48 lines): Same live Skeleton owners as Ground; Air chooses its original update method.

### `crates/skate2-core/src/physics/modes/skate3/physics/air_reckoning/`

- **`settings.rs`** (86 lines): Stock classes are verified against original8289D180 and828A0054..00A0: physics_reckoning7CE1B6B581709C6F, physics_bodyspin05EC9CB03A5B2EEE.
  - types: `Mode`

### `crates/skate2-core/src/physics/modes/skate3/physics/air_trajectory/`

- **`grind.rs`** (283 lines): Real static University provider -> native trajectory admission -> landing orientation.
  - types: `GrindContext`, `Settings`
- **`mod.rs`** (164 lines): Real stock-settings and BoardWorld adapter for the player trajectory selector.
  - types: `AirTrajectoryRuntime`
- **`settings.rs`** (123 lines): Stock class hashes429A8F4385DD539D(trajectory),7CE1B6B581709C6F (reckoning),5338958C408B6009(feet), defaultD7EDBD362D7D2152. Layout names/offsets were checked against original TU3 loads and stock XML.
- **`world.rs`** (155 lines): Original trajectory collision leaf82771D08 and nearby triangle collector 82772028, across the host's world-format boundary. BoardWorld stores static world-space triangles, so their source geometry transform is identity.

### `crates/skate2-core/src/physics/modes/skate3/physics/biped_air/`

- **`input.rs`** (105 lines): Direct processed/physical inputs for the distinct BipedAir lifecycle.
- **`post.rs`** (26 lines): State501 Post82D2FFF8, after completed physical skeleton feedback.
- **`publication.rs`** (26 lines): Recovered output adapter; switches with the complete lifecycle migration.
- **`settings.rs`** (22 lines): Stock physics_wipeout fields read by82D8FEE8/82D90000 only.
- **`update.rs`** (144 lines): Ordered82D2EF38, including actual skeleton/selector/landing consumers.

### `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/`

- **`entry.rs`** (16 lines): Effective animation root82BE3650, called by Ground Enter at82D30848.
- **`feet.rs`** (72 lines): Ground-only conditioning82D77330 and history82D775D8 around the shared FeetIK evaluator. Air's per-update timer resets do NOT belong here.
- **`grab.rs`** (99 lines): Concrete Ground grab decision82D324B0. Query readiness belongs to ONE owner.
  - types: `Input`
- **`grab_runtime.rs`** (234 lines): PlayerGrabSpline82D73FC0/74270/740F8/749D0, distinct from possession. Scene execution is concrete; retained results become visible only at Sync.
  - types: `Owner`, `Publication`
- **`ground_board.rs`** (48 lines): Ground carried-board Update82D31D1C..DB0. State60 is SkateboardController; state52 in Sync82D324B0 is a DIFFERENT grab-spline manager.
- **`lifecycle.rs`** (116 lines): Ground Post82D32AA0 and complete typed Fill82D32D38/82D785F8. Parent publishes every returned field through the shared output records.
  - types: `StatePublication`
- **`services.rs`** (159 lines): Concrete, stateless Ground bindings to the single Skate3ControllerState owners. Grab-spline queries/actions are a separate missing host, not board possession.
- **`sync.rs`** (56 lines): Ground-owned Sync stages. The caller executes returned launch on its ONE persistent AirSelector before frame correction; no speculative transition.
  - types: `LaunchInput`

### `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/entry/`

- **`tests.rs`** (24 lines) *(tests)*
  - fns: `ordinary_entry_preserves_the_complete_root`, `mirrored_entry_flips_only_x_and_z_including_fourth_lanes`

### `crates/skate2-core/src/physics/modes/skate3/physics/biped_ground/grab_runtime/`

- **`selection.rs`** (6 lines): Ground mode0 consumer of the original core82D4D150 implementation.
- **`tests.rs`** (99 lines) *(tests)*
  - fns: `record`, `s3_validation_reprocesses_swap_with_next_hit_not_next_triplet`, `nearest_ineligible_suppresses_farther_eligible`, `both_height_boundaries_are_exclusive`, `invalidation_preserves_retained_candidates_and_interactable_completion`, `publication_consumes_ready_not_retained_data`

### `crates/skate2-core/src/physics/modes/skate3/physics/climbing/`

- **`approach.rs`** (209 lines): Airborne reach is a presentation overlay. Stock graphs, gravity and collisions retain ownership until both arm targets are physically within reach.
  - types: `Approach`
- **`clip.rs`** (161 lines): Optional authored clips, separate from the SHA-bound stock animation banks.
  - types: `File`, `Clip`, `Clips`
- **`contacts.rs`** (115 lines): Two-bone IK over the authored pose. Preserve limb lengths and align both palms to fixed surface contacts with independent wrist rotation.
- **`ledge.rs`** (125 lines): Static collision probes for the first authored jump/hang/mantle set.
  - types: `Ledge`
- **`mod.rs`** (428 lines): Hybrid ledge climbing. This extension owns movement while attached; the stock action/motion graphs resume on release. It does not forge stock states or animation-bank records, and uses the same skeleton and render snapshots.
  - types: `Runtime`, `Phase`, `Attached`
- **`tests.rs`** (432 lines) *(tests)*
  - fns: `world`, `reachable_ledge_needs_width_support_and_headroom`, `default_course_has_a_reachable_test_block`, `hybrid_climb_holds_for_second_press_and_resumes_walking`, `climbing_with_a_dropped_board_preserves_free_board_physics`, `turning_away_during_reach_fades_out_and_lands_normally`, `run_climb`, `capture`

### `crates/skate2-core/src/physics/modes/skate3/physics/controls/`

- **`bail_tests.rs`** (69 lines) *(tests)*: PC bail input regression; the core native producer remains unmodified.
  - types: `Sticks`
- **`object_settings.rs`** (29 lines): The original inputlistener_default fields fetched by8259C650..C75C.
- **`offboard_tests.rs`** (138 lines) *(tests)*: Numerical and adapter regressions, not original-executable parity tests.
  - types: `Packet`

### `crates/skate2-core/src/physics/modes/skate3/physics/footplant/`

- **`ground.rs`** (192 lines): FootPlant601, vtable82327400 and manager82D704B0/82D70E40.
- **`math.rs`** (78 lines): Original vector operation order from82D6FD60/82D6F5F0/82D70070. Estimate seeds use independent PC math; Xenon bit equality is unverified.
- **`observations.rs`** (31 lines): Completed body/animation observations for the source footplant candidate. The manager owns the source calculation; this record owns no controller, bodies, clocks, queries or solver. Either native character can supply it.
  - types: `CandidatePose`, `PoseTarget`, `GroundStep`
- **`pose.rs`** (81 lines): Footplant82D70070 publishes real FootIK targets and temporary collision disables.
- **`settings.rs`** (47 lines): Cached physics_footplantmanager layout used by82D6FD60/82D6F5F0/82D70070.
  - types: `Settings`
- **`update.rs`** (212 lines): Original82D6FD60 ->82D6F5F0 ->82D6F910, invoked by82D36728.
  - types: `FootplantFrame`

### `crates/skate2-core/src/physics/modes/skate3/physics/grind/`

- **`advance.rs`** (137 lines): Native common82D3F4E8 and Nonspecific82D42DF0 Update order.
- **`arithmetic.rs`** (5 lines): Same scalar dot ordering as CURRENT core's private native arithmetic helper.
- **`board.rs`** (49 lines): Concrete CURRENT BoardRuntime adaptation of AddWorldForce82C03F98.
- **`condition.rs`** (72 lines): Completed PhysOut -> chromosome82DEE508, before filtered82DE56B0.
- **`contact.rs`** (267 lines): Six native substate1 dispatches. Manager query/family selection is upstream.
  - types: `Op`
- **`forces.rs`** (84 lines): Original force leaves wired to CURRENT's immediate deck accumulator.
- **`involuntary.rs`** (114 lines): Native family substate2 dispatch; no generic tipslide substitution.
- **`launch.rs`** (28 lines): Concrete SetVelocity82C04168 continuation; retain main's jumper owner.
- **`lifecycle.rs`** (62 lines): Physical side effects of Enter82D3F318/Exit82D3F430, all six families. State latches/frame initialization are distinct from these body operations.
- **`observation.rs`** (58 lines): Required producer inputs. None implements Default: absence of a producer is not permission to use world-up, metal friction, geometry zero or full energy.
  - types: `ManagerObservation`, `GeometryObservation`, `SurfaceObservation`, `ControlObservation`, `EngagementObservation`
- **`output.rs`** (161 lines): Common82D40EA0 and six family FillOut overrides. Post/selector are separate.
  - types: `State`, `PhysicalOutput`, `SideEffects`
- **`phase.rs`** (118 lines): CURRENT Skate3PhysicsState/Skate3ControllerState adapters. Shared scheduling belongs to main.
- **`post.rs`** (13 lines): Common grind Post using CURRENT collision observations and request owner.
- **`rng.rs`** (58 lines): Host-owned orientation stream using recovered82970628 arithmetic. Draw ownership/seed bookkeeping is host policy; do not claim original cross-subsystem interleaving. No random draw occurs until AddNoise asks.
  - types: `OrientationRandom`
- **`runtime.rs`** (108 lines): Host lifetime for the six native physical-state instances and chromosome.
  - types: `Runtime`
- **`settings.rs`** (65 lines): Stock runtime data. No map/query settings or manager defaults live here.
  - types: `Settings`
- **`state.rs`** (64 lines): Separate native state instances: common ctor82D3F210 and Enter82D3F318.
  - types: `State`
- **`substate.rs`** (154 lines): Original82D40AF8: collision -> pop -> engagement -> derived substate. All writes target current board, manager jumper and skeleton owners.
- **`tip_nudge.rs`** (29 lines): S3 tipslide dropping-in nudge82D421B0, separate from common exit nudge. Scalar reconstruction of original operand flow; ordinary PC floating point, not a claim of bit-exact VMX128 execution or an instruction emulator.
- **`wipeout_output.rs`** (18 lines): Complete the grind FillOut write to native PhysOut pointer+12's 48-byte block. CURRENT already represents it as PhysicalPlayerInput::physics.

### `crates/skate2-core/src/physics/modes/skate3/physics/grind/substate/`

- **`nonspecific.rs`** (95 lines): 82D42DF0 through board helper82D42F50; caller owns final grind reckoning.
- **`settings.rs`** (91 lines): Stock mode endpoints and constructor-vtable-resolved jump getters.
  - types: `Settings`
- **`skeleton.rs`** (65 lines): 82D40D28 calls82BDDA10 with fast_blend=false only for a pop THIS update.

### `crates/skate2-core/src/physics/modes/skate3/physics/grind_camera/`

- **`tests.rs`** (43 lines) *(tests)*
  - fns: `output`, `contact`, `ordinary_motion_is_not_smoothed_but_family_discontinuity_is`, `primitive_change_uses_xyz_midpoint_and_inactive_does_not_erase_output`

### `crates/skate2-core/src/physics/modes/skate3/physics/grind_chromosome/`

- **`pose.rs`** (81 lines)
  - types: `ApproachPose`, `Input`; fns: `straight`, `low`, `add`, `sub`, `signed`
- **`tests.rs`** (93 lines) *(tests)*
  - fns: `input`, `family_switch_does_not_fake_new_category_or_skip_scoring_delay`, `air_snapshot_clears_history_and_short_ground_history_does_not_replace_saved`, `twist_uses_128_not_forward32_and_names_keep_both_id_domains`, `frontlip_and_frontboard_publish_distinct_native_scorables`

### `crates/skate2-core/src/physics/modes/skate3/physics/grind_names/`

- **`table0.rs`** (292 lines)
- **`table1.rs`** (292 lines)
- **`table2.rs`** (292 lines)
- **`table3.rs`** (292 lines)
- **`table4.rs`** (292 lines)
- **`table5.rs`** (292 lines)
- **`table6.rs`** (292 lines)
- **`table7.rs`** (292 lines)

### `crates/skate2-core/src/physics/modes/skate3/physics/ground_animation/`

- **`board.rs`** (214 lines): 82D33E30: jump launch or ordered foot/brake/friction/collision forces.
- **`settings.rs`** (67 lines): 82D94C10 copies physics_jump layout0..456 into the board toolkit cache.
  - types: `GroundAnimationSettings`
- **`skeleton.rs`** (28 lines): 82D33E00 calls SkeletonUpdateAnimated82BDDA10 with fast blending enabled.

### `crates/skate2-core/src/physics/modes/skate3/physics/ground_runtime/`

- **`corrections.rs`** (102 lines): Native numerical callbacks for the ordinary ground-state branch.
- **`entry.rs`** (165 lines): Actual Ground entry82D37538. Numerical operations touch live board bodies; the physical skeleton/air modifier remain calls to their separate owners.
  - types: `EntrySettings`, `GroundEntryTargets`, `ManualBodies`, `Projection`
- **`entry_restoration_tests.rs`** (65 lines) *(tests)*: Independent field-ownership regression, not a stock gameplay/parity fixture.
- **`input.rs`** (345 lines): UpdateSkateboard82D38800 inputs from the actual processed packet, animation attributes, physical pose and source board toolkit. No raw button-to-force or guessed contact flags are introduced at this boundary.
  - types: `GroundInputObservations`
- **`launch.rs`** (125 lines): Wall-jump launch packet;82D33448 constructs it,82BE33D0 fills it. The native selector consumes this packet after body velocity publication.
  - types: `GroundLaunchInfo`, `GroundLaunchPhysical`
- **`mod.rs`** (163 lines): Owned native ground contact/force services for the live BoardRuntime. The coordinator supplies the processed physical packet in native phase order.
  - types: `GroundRuntime`
- **`pumping.rs`** (166 lines): Actual Ground82D37D34..7EAC pumping preparation and stock bindings.
  - types: `GroundPumping`, `GroundPumpingMode`
- **`services.rs`** (294 lines): Ground's physical operations execute against the live board, in core's recovered order. Detached queue/inertia ownership is published before the solver runs; no force, drag or steering output is left in a test-only object.
  - types: `GroundControllers`, `GroundPhysicalFrame`, `BoardServices`
- **`settings.rs`** (322 lines): Stock bindings used by the complete Ground::UpdateSkateboard call. Toolkit copies follow82D94C10; selected surface/mode fields stay explicit.
  - types: `GroundProfiles`, `GroundSettings`
- **`state.rs`** (96 lines): Native Ground controller ownership. Source entry/update and output retain the same instances; constructing this does not mark Ground as entered.
  - types: `GroundState`
- **`surface.rs`** (31 lines): Native wheel surface vote82C08818 on live wheel lines and contact reports.
- **`trajectory.rs`** (23 lines): Ground ordinary tail82D749D0 cancels pending trajectory work and clears published validity. Rust owns the request lifetime; no guest pool is copied.
  - types: `GroundTrajectoryState`
- **`update.rs`** (167 lines): Ordinary Ground82D37C88 executes persistent controller updates and actual body forces before prediction and IK publication. Ground's handplant query 82D38430 is dispatched by ground_phase through the retained Handplant owner.
  - types: `GroundUpdateFrame`, `GroundUpdateTargets`

### `crates/skate2-core/src/physics/modes/skate3/physics/handplant/`

- **`contact.rs`** (243 lines): PositionSelector82D63D40, velocity window82D64420 and edge clipping82D651E0.
  - types: `Candidate`
- **`ik.rs`** (91 lines): HandPlantManager82D633B0 and Skeleton82BD9728/97D0, shared four-limb IK.
- **`rotation.rs`** (51 lines): CreateMtxFromHeadingAndUp82D61B60 and axis-angle frame blend82BD3150.
- **`settings.rs`** (97 lines): TU3 cached layouts55B5940075F78842 and0F38084037CB9175.
  - types: `Settings`
- **`trajectory.rs`** (313 lines): Handplant Launch82D613B0, trajectory selector82D66318/66948 and COM82D625C0.

### `crates/skate2-core/src/physics/modes/skate3/physics/known_air/`

- **`flip.rs`** (49 lines): Original Reckoning ResetBodyFlip82D8E9E0 / BeginBodyFlip82D8EAB0.
- **`math.rs`** (107 lines): Original KnownAir VMX geometry; independent PC estimate seeds.
- **`output.rs`** (69 lines): Fill82D36880 overwrites every field below except the explicitly retained pair.
- **`runtime.rs`** (373 lines): Concrete KnownAir boundary. All callbacks mutate the same live owners. Fallible host queries/pose checks retain their first error for the phase caller.
  - types: `Runtime`
- **`settings.rs`** (51 lines): Original KnownAir82D35508/82D35F40/82D360B0/82D36650 stock reads.

### `crates/skate2-core/src/physics/modes/skate3/physics/landing_on_deck/`

- **`publication.rs`** (36 lines): State503 FillPhysOut82D4DFC8. No output is inferred from render transforms.
- **`skeleton.rs`** (99 lines): Skeleton82BDE428: old-root COM, new root, shared drives, shared reckoning.

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/`

- **`air_diagnostics.rs`** (93 lines): Bounded numeric history for user-reproduced off-board bails. No per-frame IO.
  - types: `Sample`, `History`
- **`air_feet.rs`** (108 lines): State56 manager updates real foot targets before Skeleton GeneralUpdate.
- **`air_selector.rs`** (203 lines): Concrete BipedAir selector host over the canonical BoardWorld. Parent owns the Air facade and phase schedule. Native completion is explicit, so a synchronous fixed-step backend never adds an automatic frame delay.
  - types: `AirSelector`
- **`air_state.rs`** (247 lines): Live BipedAir501 Enter/Update/Exit and FillPhysOut.
- **`air_trajectory.rs`** (224 lines): Biped trajectory launch/completion through the actual scene query owner.
- **`board_effects.rs`** (143 lines): Board-controller effects applied to the existing board and hook bodies.
  - types: `Policy`, `BoardEffects`
- **`board_manager.rs`** (181 lines): Persistent SkateboardController adapter, not the grab-spline query manager.
  - types: `Owner`, `Transition`
- **`board_possession.rs`** (165 lines): Live board controller adapter. The caller borrows the existing board and state448 owners; this module never advances a second physics island.
  - types: `VolumeFlags`, `Materials`, `Effects`
- **`collision_settings.rs`** (19 lines): Stock physics_wipeout fields consumed by Biped collision check82D90148. Retain this once with the physical player's off-board settings.
- **`contact_queries.rs`** (103 lines): Scene boundary for the recovered BipedToolkit probe layout. The pending packet owns observations; the world's geometry and acceleration tree stay shared.
  - types: `Probe`, `Hit`
- **`contact_toolkit.rs`** (9 lines): Static canonical-world adapter for OffboardGroundAnalyzer. The runtime must supply actual mesh metadata; neither a board contact nor a synthetic ground ray can provide it.
- **`feet.rs`** (72 lines): State56 manager updates real foot targets before Skeleton GeneralUpdate.
- **`grab_scene.rs`** (93 lines): Stateless offboard grab-scene execution over explicitly authored objects. Ground owns pending/completion, validation and selection; this owns no ticks.
  - types: `MeshAssembly`, `HostLines`, `Registry`, `Scene`
- **`ground_geometry.rs`** (83 lines): Ground Sync submits edge geometry; the next PreUpdate consumes its result.
  - types: `State`
- **`ground_input.rs`** (76 lines): PreUpdate82D30D30 inputs borrowed from their completed production owners.
- **`ground_query.rs`** (83 lines): Borrowed scene adapter for original Biped query82764AB0/82770650/82C1EAD8. All geometry and live transforms belong to the canonical collision scene.
  - types: `Bounds`, `Mesh`, `Segment`, `EdgeBody`, `AlternateRecord`, `PrimaryEdges`, `IndexedEdgeBody`, `Scene`
- **`ground_state.rs`** (283 lines): BipedGround's live Enter, ground job, Sync and Exit owners.
  - types: `Services`
- **`ground_sync.rs`** (60 lines): Host side scene service for the recovered BipedGround synchronizer.
  - types: `SceneService`
- **`hand_drives.rs`** (154 lines): Native SkateboardController drives82D74FD8 /82D751D8 in the shared island. Factory82AE64F8 stores argument5 at drive16 (A), argument4 at20 (B): hand bodies3/7 are A and the deck is B, despite the factory argument order.
- **`landing_deck.rs`** (120 lines): Offboard host for original LandingOnDeckManager82D78B28..82D79E30. One retained manager and real scene-query completion; no physical-state facade.
  - types: `Owner`
- **`mod.rs`** (24 lines): Off-board game adapters over the shared physical and scene owners.
- **`mod_solid_ground.rs`** (263 lines): Mod solid providers for biped ground queries and moving-object classification.
  - types: `VehicleEdgeCache`, `EdgeBodyStorage`, `VehicleCandidate`
- **`pose_adjust.rs`** (31 lines): Original82BD8F10 writes the existing AnimatedSkeleton board-offset owner.
- **`possession.rs`** (118 lines): Native hand controller over the player's live pose and board observations.
  - types: `Actions`
- **`post_physics.rs`** (51 lines): Offboard state slot40 at82DB6DCC, after Skeleton feedback82DB6DB8 and before PostWipeoutCheck82DB6DF8 consumes the same request owner.
- **`runtime.rs`** (235 lines): Persistent native Biped/controller/contact owners shared across physical states.
  - types: `Runtime`
- **`settings.rs`** (91 lines): Stock Biped settings82D7B630 and metric queries82D7AFD8/82D16680.
  - types: `Settings`
- **`skeleton_air.rs`** (82 lines): BipedAir Skeleton82BDDD70 with the existing skeleton, drives and solver. The caller supplies the shared reckoning service; no second physics owner.
  - types: `Input`
- **`skeleton_ground.rs`** (227 lines): Original Biped Skeleton update82BDE060 with the existing physical owners.
  - types: `Input`, `ReckoningUpdate`, `Push`

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/air_selector/`

- **`settings.rs`** (38 lines): Selector-owned stock settings. Missing authored fields are errors.
  - types: `Settings`
- **`tests.rs`** (204 lines) *(tests)*
  - fns: `world`, `context`, `owner`, `packet`, `six_real_lines_keep_native_pool_ties_and_exact_surfaces`, `conditional_pool_requires_exact_island_three`, `line_rejects_nonfinite_input_without_synthesizing_a_hit`, `synchronous_results_remain_staged_until_native_consume`

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_manager/`

- **`runtime.rs`** (302 lines): Live bindings for TU3 SkateboardController, 82DB6150 and 82D76D20.
  - types: `LiveState`
- **`tests.rs`** (320 lines) *(tests)*: Adapter tests use synthetic physical parameters, not stock parity fixtures. They check writes to actual BoardRuntime bodies/drives rather than a call log.

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/board_possession/`

- **`drives.rs`** (75 lines): 74FD8 creates both persistent hand drives. AddDrive reverses argument bodies: child/internal A is skeleton part3/7; parent/internal B is deck6.
- **`settings.rs`** (48 lines): Constructor74DD8 class7DEA05DC70A1ED7D/defaultD7EDBD362D7D2152.

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/contact_toolkit/`

- **`tests.rs`** (259 lines) *(tests)*
  - fns: `plane`, `input`, `actual_world_hit_preserves_mesh_identity_and_packed_surface`, `nonfinite_unused_lane_preserves_sweep_hits_but_invalid_xyz_still_fails`, `mesh_filter_is_not_a_board_flag_or_material_tag`, `canonical_world_reaches_completed_analyzer_without_substitute_support`, `toolkit_refuses_world_without_authored_query_metadata`, `embedded_static_rwcm_hits_distinct_actor_query_ids`
- **`world.rs`** (418 lines): 82770B40/82771D08 mesh traversal and82770E00/82772028 nearby collection. Authored map pools plus the host's immutable solid moving-geometry provider.
  - types: `StaticScene`

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/grab_scene/`

- **`collision.rs`** (113 lines): Actual static scene swept triangles. Assembly identity comes exclusively from registered physical associations, never the mesh/triangle numeric ID.

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/ground_query/`

- **`edges.rs`** (72 lines)
  - fns: `append`, `body`, `candidates`
- **`lines.rs`** (108 lines)
  - fns: `query`, `query_pools`
- **`tests.rs`** (288 lines) *(tests)*
  - fns: `indexed_edges_sort_ids_and_preserve_shared_capacity`, `decoded_face_uses_world_vertices_and_inclusive_bounds`, `triangle`, `packet`, `mesh`, `actual_triangle_queries_preserve_pool_ties_group_filter_and_conditional_gate`, `canonical_world_bridge_requires_metadata_and_queries_authored_surfaces`, `indexed_world_bridge_retains_group_pool_and_conditional_selection`
- **`transform.rs`** (67 lines)
  - fns: `add`, `sub`, `scale`, `madd`, `point`, `bounds`, `overlaps`, `matches`
- **`world.rs`** (127 lines): Ephemeral adapter over the canonical BoardWorld metadata. Geometry is borrowed.
  - types: `WorldScene`

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/landing_deck/`

- **`input.rs`** (28 lines): Actual completed PlayerInput/BoardToolkit fields read by82D78xxx.
- **`output.rs`** (24 lines): Complete82D79420 writes into caller-owned physical output fields.
  - types: `PublishTargets`
- **`query.rs`** (79 lines): Real accelerated trajectory query and ordered completion of82D79948/79B40.

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/settings/`

- **`board.rs`** (32 lines): Ground ctor82D305D8 passes state36 to82B97060, whose full64 class/key are492026E0F096F283/D7EDBD362D7D2152 (physics_state_offboard/default). 82B6CC78 stores the returned collection layout at wrapper4 = state40.
- **`curves.rs`** (42 lines): PointNegGraphData has four bounds followed by N X values and N Y values.
- **`metrics.rs`** (46 lines): 82D16680 uses first matching record, status4, length=frames/fps. 82D164F8 multiplies the authored end by that length, including end=-1.
- **`possession.rs`** (25 lines): Stock physics_skatecontroller/default data used by the board controller.
- **`tests.rs`** (89 lines) *(tests)*
  - fns: `clip`, `query_uses_full_frame_duration_and_negative_untimed_end`, `first_matching_record_wins_without_a_playback_status_filter`, `actual_attribute_absence_is_distinct_from_malformed_data`, `graph_bounds_are_separate_and_duplicate_knots_preserve_endpoint_jump`, `actual_stock_settings_and_metrics_load_together`

### `crates/skate2-core/src/physics/modes/skate3/physics/offboard/skeleton_ground/`

- **`frames.rs`** (66 lines)
  - fns: `prepare_pose`, `prepare`
- **`frames_tests.rs`** (130 lines) *(tests)*
  - fns: `biped_ik_bindings_keep_adjacent_foot_volumes_distinct`, `disabling_feet_target_ik_fades_board_weight_without_disabling_the_pose`, `frozen_board_uses_retained_pose_and_flips_complete_axes`, `biped_com_and_skate_root_use_old_basis_without_overwriting_physical_board`
- **`pose_audit.rs`** (43 lines): Read-only fixture diagnostics; never modifies simulation or acceptance limits.
- **`state.rs`** (75 lines): Persistent animation-space board16016 and the Biped reckoning callback.
  - types: `State`

### `crates/skate2-core/src/physics/modes/skate3/physics/player_input/`

- **`dynamic_normal.rs`** (21 lines): Stock bindings for original82C02388. Hashes were matched to the actual collection names; XML layoutC85F0-C8190 verifies curve offset1120.
- **`grind.rs`** (204 lines): Host manager82D8A828/82D8AB08, using CURRENT player-input ownership. Static authored-world scope. Gameplay leaves live in skate-core; world filtering, primitive provenance and board material application stay explicit.
  - types: `PreContext`, `PostContext`, `MaterialMode`, `PrimitiveMetadata`, `Host`, `Pending`, `GeometryWork`, `PostResult`, `GrindInputState`
- **`grind_output.rs`** (29 lines): Completed common/Skeleton output needed by the stock grind graph.
- **`ground_position.rs`** (71 lines): Ground-position observation from TU3 Skateboard::CalculateGroundPosition 82C02840. The four wheel rigid-body centres are expressed in Reckoning752, averaged laterally/longitudinally, and placed at the lowest wheel centre.
- **`initial.rs`** (29 lines): Original82DB18A0 initialization of the represented input owner. Native opaque allocation flags are replaced by zero-initialized host storage.
- **`mod.rs`** (202 lines): Concrete original Skate3 player input phase for the authored riding world.
  - types: `InputStage`, `PlayerInputRuntime`
- **`output_reset.rs`** (45 lines): Represented PhysOut reset values. Original82DE53F0 copies the templates built by82DE4940; scalar zero does not replace any active producer.
- **`pre_input.rs`** (69 lines): Original82D81610 inactive-query branch and82D2DDD0 result reset. Pending geometry is explicit; this owner never treats an unimplemented query as a miss.
  - types: `PreInputResult`, `PreInputManager`
- **`reset.rs`** (61 lines): Selective reset of the represented ProcessedPhysIn fields, TU3 82BF9EF0. Packet buffers and retained flag lanes deliberately survive this reset.
- **`services.rs`** (214 lines): Host ownership for the original 82DB4048 input phase. Every numerical preparation is performed here; the two callbacks address the shared skeleton/whole-player owners that cannot be borrowed twice by the host.
  - types: `InputHostFrame`, `PlayerInputCallbacks`, `Services`

### `crates/skate2-core/src/physics/modes/skate3/physics/player_input/grind/`

- **`history.rs`** (94 lines)
  - fns: `permission`, `advance_history`, `material_mode`, `decrement`
- **`post.rs`** (203 lines)
  - fns: `post_update`, `entry_kind`
- **`pre.rs`** (160 lines)
  - fns: `pre_update`
- **`publication.rs`** (71 lines)
  - fns: `surface`, `observation`, `family`
- **`settings.rs`** (58 lines)
  - types: `Settings`; fns: `load`, `graph4`
- **`tests.rs`** (244 lines) *(tests)*
  - types: `TestHost`; fns: `state`, `context`, `apply_material_mode`, `surface_probe`, `force_exit_line`, `material_selection_uses_previous_air_target_and_preserves_skipped_history`, `permission_keeps_flicker_weights_strict_threshold_and_signed_decrement`, `constructor_reset_does_not_reinitialize_child_history_or_jump_energy`
- **`world.rs`** (186 lines): Static host for original S3 82C20728 and 82E0AAD0 line batches. Both use scene mask7, matching ID2952, side flags3, rejection mask0 and a null surface-exclusion table. Metadata is mandatory, never render tags.
- **`world_tests.rs`** (307 lines) *(tests)*
  - fns: `triangle`, `world`, `down`, `exact_packed_surface_not_render_tag_and_zero_rejection_mask`, `pool_order_precedes_mesh_order_and_nearest_still_wins`, `conditional_pool_requires_exact_island_state_three`, `contact_fatness_does_not_move_the_query_hit`, `rounded_edge_hit_publishes_authored_face_not_sweep_normal`

### `crates/skate2-core/src/physics/modes/skate3/physics/player_state/`

- **`post_input.rs`** (244 lines): Original PostInput82DB5588 with the live ballistic trajectory selector. The authored world has no grind edges or grab-spline descriptors.
  - types: `PostInputState`, `Services`
- **`pre_state.rs`** (63 lines): Update3_State82DB6050 prefix through grab-spline query synchronization. Ground/Air/KnownAir vtable24 ->82D34DA8 ->82D2D860 predicts from the actual deck part transform and its physical body's linear velocity, with a single VMADD.
- **`publication.rs`** (286 lines): Selected-state FillPhysOut and common state82DB7580..7790.
- **`registry.rs`** (137 lines): Coordinator-owned physical state registry.
  - types: `StateCapability`, `StateRegistry`
- **`selection.rs`** (69 lines): CalcSuggestedState82D8ADE8 gets actual contacts, line distances and pose.
- **`transition.rs`** (259 lines): Native SetPhysicsState82DB8540 publication, followed by owned Enter/Exit.
  - types: `Calls`
- **`wipeout_output.rs`** (50 lines): Wipeout300 Fill82D3EEE8 writes the shared completed physical packet.

### `crates/skate2-core/src/physics/modes/skate3/physics/riding_outputs/`

- **`ground_input.rs`** (34 lines): Snapshot inputs copied by original TU382D8E5C0 for the Ground job.
  - types: `GroundPacketInputs`
- **`ground_input_tests.rs`** (26 lines) *(tests)*
  - fns: `ground_reckoning_keeps_retained_dynamic_up_separate_from_wheel_support`
- **`probes.rs`** (121 lines) *(tests)*: The fifth/sixth board probes, kept in the same query/publication phase as the wheel probes. Geometry is the world's recovered triangle dispatcher.
  - types: `BoardProbes`, `Pending`

### `crates/skate2-core/src/physics/modes/skate3/physics/slide_state/`

- **`settings.rs`** (62 lines): Direct original bindings82D94EC8..4F48,82D3AB30/AD3C/AE68 and stock schema.
- **`update.rs`** (244 lines): Ordered UpdateSkateboard82D3A900: steering, manual, wall launch or Slide forces.
  - types: `Angle`

### `crates/skate2-core/src/physics/modes/skate3/physics/solve/`

- **`assembly_contacts.rs`** (110 lines): Original single-player SkaterSkaterCollisionPipeline8276556C. Board construction precedes Skeleton construction in82DB18A0. Preserve that assembly order: board self (all culled), board/rider, rider self.
- **`assembly_contacts_tests.rs`** (105 lines) *(tests)*
  - fns: `mode`, `material`, `volume`, `board_pairs_use_actual_biped_part_groups`, `assembly_gate_and_world_query_group_are_distinct`, `original_single_player_table_keeps_special_groups_and_symmetry`
- **`diagnostics.rs`** (36 lines): Locate invalid physical values before they propagate into a rendered pose.

### `crates/skate2-core/src/physics/modes/skate3/physics/tests/`

- **`clock.rs`** (83 lines) *(tests)*
  - fns: `camera_rate_requests_expire_and_preserve_message_order`, `camera_slow_motion_waits_for_a_positive_air_prediction`, `host_timer_matches_skate3_rate_and_countdown_at_every_tick`
- **`flat_ground.rs`** (39 lines) *(tests)*
  - fns: `flat_render_vertices_and_actual_queries_agree_across_entire_surface`

### `crates/skate2-core/src/physics/modes/skate3/physics/wipeout/`

- **`observations.rs`** (96 lines): Borrow the actual completed collision and physical owners; no default contacts.
  - types: `Observations`
- **`settings.rs`** (80 lines): Globals264 -> original8289D270 class EB844CFDBF928EFD/default. Fields are independently mapped through the stock physics_wipeout schema.

### `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/`

- **`contact.rs`** (62 lines): Wipeout300's material corrections and collision-policy requests82D3EC08.
- **`drive_settings.rs`** (47 lines): Native82BE7860's per-part records and82BEB0C8/82BEB790 settings.
- **`lifecycle.rs`** (131 lines): Original Wipeout Enter82D3B5E8 and Exit82D3B990.
- **`mod.rs`** (146 lines): Original physical WipeoutGround300. Off-board recovery destinations are separate states; this owner retains the native request and timing fields.
  - types: `WipeoutState`
- **`output.rs`** (19 lines)
  - fns: `fill`
- **`prediction.rs`** (87 lines): Original Wipeout Update82D3C02C..C288; completed query precedes next submit.
  - types: `Prediction`
- **`ragdoll.rs`** (72 lines): Physical-owner adapter for selector82D913C0 requests8..11 and82BE7280.
  - types: `RagdollSetup`
- **`settings.rs`** (101 lines): Stock vault bindings for WipeoutGround300 and its authored control profiles.
  - types: `Settings`
- **`skeleton.rs`** (195 lines): Original Skeleton::UpdateWipeout82BDFA88 on the live physical owners.
- **`update.rs`** (241 lines): Complete Wipeout300 update order82D3B9C8, on the game body/pose owners.

### `crates/skate2-core/src/physics/modes/skate3/physics/wipeout_states/ragdoll/`

- **`settings.rs`** (98 lines): Authored inputs to normal82BE58A8 and ragdoll82BE59A8/6D60.
  - types: `Settings`

### `crates/skate2-core/src/physics/modes/skate3/scoring_runtime/`

- **`rotation.rs`** (142 lines): Skate 3 TU3 82DA8BE0, 82DAC880 and 82DAC9D0. Track the skater root while a trick animates; otherwise track the board projected against reckoning up. Deck flicks are not skater rotations.
  - types: `Tracker`, `Rotation`

### `crates/skate2-core/src/physics/modes/skate3/skater_animation/`

- **`state.rs`** (88 lines): State shared by graph construction, stance events and physics publication.
  - types: `AnimationState`

## 2. Integration layer (Skate 2 host <-> Skate 3 physics)

Files that create, drive, feed or read the Skate 3 player from Skate 2's player, assembly, scoring, animation, camera, input and session code.

### `crates/skate2-core/src/`

- **`lib.rs`** (885 lines): Platform-neutral contracts for reconstructed Skate 2 systems.

### `crates/skate2-core/src/actors/`

- **`actor_breadcrumb_live_selection.rs`** (329 lines): Reached local Actor breadcrumb selector over the original mounted owners. GetWipeoutSpotBreadCrumb 0x82ce1cb8, PopBreadCrumb 0x82ce2d80, TestForLWRespawnContraints 0x82ce1320, GetLastResortBreadCrumb 0x82ce2598.
  - types: `LiveBreadCrumbSelectionError`, `LiveBreadCrumbSelectionSource`
- **`actor_input_generation.rs`** (300 lines): SAME Actor/action-map `GenerateActionGraphIntents`, reference 0x825fda28.
  - types: `ActorInputPhysics`, `ActorInputGenerationError`

### `crates/skate2-core/src/actors/skater/`

- **`skater_manager_observers.rs`** (511 lines): Actual selected observer order and first-player callback prefix.
  - types: `SkaterObserverError`, `CensusNotificationFields`, `Observer`, `SkaterObserverState`

### `crates/skate2-core/src/animation/runtime/`

- **`actor_skate3_gestures.rs`** (167 lines): PC gesture catalog and movement-base admission on the retained Actor.
- **`skate3_gesture_playback.rs`** (374 lines): Skate 3 CharacterGesture82BAA320 on the existing Skate 2 Actor's ACS channels. Only animation is owned here. The selected game's physical player, root, character recipe, hands/face writers and save owner remain authoritative.
  - types: `ChannelPose`, `Context`, `Playback`
- **`skate3_gesture_streams.rs`** (202 lines): Authored S3 gesture finger/expression cues on the retained S2 hands/face ACS. Body tree sampling and native clip key/token arithmetic share the same time.
  - types: `HandPoses`, `GestureStreams`

### `crates/skate2-core/src/camera/frame/`

- **`camera_skate3.rs`** (66 lines): Source Skate 3 camera result published to the retained host camera transport.

### `crates/skate2-core/src/living_world/pedestrians/`

- **`pedestrian_collision_shove.rs`** (109 lines): PC port of TU3's shove on a pedestrian (Skate 3 physics): the host's answer to the probe's pedestrian query (`LivingWorld::Manager` vslot 0x44, 0x826bed80) and the game's response to a strike (0x8275f0d8 -> `Pedestrian` 0x82e38cc0), on Skate 2's living world.

### `crates/skate2-core/src/physics/`

- **`skate3_bridge.rs`** (159 lines): Asset-independent Skate 3 physics over actual Skate 2 world geometry.
  - types: `Skate3GroundWorld`
- **`skate3_grabs.rs`** (404 lines): Native Skate2 authored grab providers and immutable admitted-tick mesh lines. Spline IDs remain the actual LivingWorld IDs. Assembly IDs retain the allocation handle occurrence, independent of entity, mesh and body slots.
  - types: `HostGrabs`, `MeshLines`
- **`skate3_grinds.rs`** (422 lines): Retained Skate 2 spline payloads and source occurrences for the S3 runtime.
  - types: `HostGrinds`, `CachedProp`, `CachedSource`, `ReferenceGrinds`
- **`skate3_player.rs`** (1837 lines): Live-player owner for the explicitly requested PC simulation mod.
  - types: `PreparedPlayerTick`, `PlayerAssets`, `HostTeleport`, `PlayerEntry`, `Skate3Player`
- **`skate3_pushing.rs`** (356 lines): PC port: TU3 PhysState_OffBoard_Pushing (vtable `0x82327364`) for the Skate 3 physics. Its Enter (`0x82d442d0`), UpdateSpline (`0x82d444a0`), hand IK targets (`0x82d45d30`, `0x82d46218`, `0x82d463d8`) and MoveObject (`0x82d45318`) are Skate 2's `OffBoardPushing` with the pushing cvars at their defaults, so the host runs Skate 2's over the world's DMO grab splines, whose messages the world applies in Update1. TU3...
  - types: `Inputs`, `HandTargets`, `Pushing`
- **`skate3_terrain.rs`** (242 lines): Immutable Skate 3 triangle preparation retained by actual Skate 2 arena occurrence. Streaming still chooses every source and its order in Skate 2.
  - types: `PreparedMesh`, `PreparedTerrain`

### `crates/skate2-core/src/physics/assembly/`

- **`skate3_player_backend.rs`** (40 lines): Narrow access to the retained body pool for the explicitly selected PC mod.
- **`skate3_world.rs`** (753 lines): Retained host body identities and actual authored collision geometry for the captured Skate 3 solver. No proxy body is integrated by both solvers.
  - types: `MeshOwner`, `LocalTriangle`, `Skate3MeshBody`, `Skate3Bodies`, `PlacedMesh`, `CachedMesh`, `HostBodies`

### `crates/skate2-core/src/physics/assembly/parts/`

- **`skate3_part_mirror.rs`** (18 lines): Use the retained host Part mass frame when mirroring an alternate solver.

### `crates/skate2-core/src/physics/player/`

- **`skate3_actor_input.rs`** (154 lines): Semantic S2 Actor policy at the native S3 input boundary. S2 GenerateActionGraphIntents825FDA28 and Actor.Update825F10F0 own the requests; S3 input-filter bit numbers are deliberately not reused.
  - types: `HostActorInput`
- **`skate3_camera_rate.rs`** (138 lines): Optional S3 camera time effects; physical integration keeps its fixed step.
- **`skate3_exit.rs`** (147 lines): One-time native solver handoff. Actor reset generations and physical body occurrences prevent a mode change from affecting a later marker/career reset.
  - types: `PlayerExit`, `BodyRate`
- **`skate3_spawn.rs`** (26 lines): PC offboard reset placement. S2 AdjustSpawnPosition825F1B8C adds .15m, then the shared S3 board reset82DB8A10 adds .1m. A standing biped must start on its actual nearby support, not fall from that combined clearance.

### `crates/skate2-core/src/physics/player/lifecycle/`

- **`physical_player_assembly.rs`** (1591 lines): Source-ordered physical-player assembly projection. It retains the actual supplied physics inputs shared by board and skeleton; it is not an Actor, UCOM object, processed-input implementation or full `HiLOD` constructor.
  - types: `PhysicalPlayerAssemblyServices`, `PhysicalPlayerAssemblyError`, `PhysicalPlayerSkeletonFinishError`, `PhysicalPlayerAssemblyInputs`, `PhysicalPlayerAssemblyOwner`, `PhysicalPlayerAssemblyPhase`, `PhysicalPlayerAssemblyConstruction`, `PhysicalPlayerAssemblyConstructionState`, `PhysicalConstructionBindError`
- **`physical_player_selected_animation_conditioning.rs`** (198 lines): Selected retained Animation conditioner after the seven Physics callbacks.
  - types: `SelectedGroundAnimationComplete`, `SelectedAnimationFinishError`
- **`physical_player_update1_prefix.rs`** (339 lines): Actual `Update1_ProcessInput82e01f98..2174`, through its teleport check.
  - types: `Update1AfterTeleportCheck`, `Update1PrefixError`

### `crates/skate2-core/src/physics/player/output/`

- **`physical_player_phys_out_skate3.rs`** (544 lines): Explicit PC mod observer publication. These are completed source-runtime facts for Skate 2 world consumers, never a certificate of a Skate 2 Fill.
  - types: `Skate3Publication`

### `crates/skate2-core/src/physics/player/output/conditioning/`

- **`physical_player_phys_out_conditioners_air_collector.rs`** (272 lines): `AirCollector82df8e18` and Reset82df4720 read the same constructed `PhysOut`.
  - types: `AirGroundData`, `AirGapData`, `AirCollector`, `AirResetError`

### `crates/skate2-core/src/physics/player/output/conditioning/scoring/`

- **`air_metrics.rs`** (186 lines): AirCollector spin82df4e60, body-flip82df5188 and trajectory82df5340.
- **`attributes.rs`** (195 lines): Scoring's original generated attribute access paths. Tuning remains in Vault.
  - types: `ScoreError`, `ScoringAttributes`
- **`skate3_catalog.rs`** (305 lines): Native mod extension: resolve Skate 3 TU3's names into the existing Skate 2 scoring/HUD pipeline. Shared names keep their original Skate 2 identities.
- **`skate3_hom.rs`** (96 lines): TU3 Hall of Meat authored bones and curves. S2 caller offsets are translated by stable field hashes; S3 added curves and moved the shared fields.
  - types: `Tweak`, `Data`
- **`wipeout_skate3.rs`** (650 lines): TU3 Hall of Meat for Skate 3 physics: WipeoutScorer::Update 82DAE4F0 and the TU3 helpers it calls, over the shared scorer's bones, trackers, air drops and entity scorers. Skate 3 scores the run-up into a bail (air, drop and rotation while airborne), decays repeated bone hits, and adds body-tweak spins and the wipeout response to the bonus.
  - types: `Skate3HoM`

### `crates/skate2-core/src/physics/solver/bodies/`

- **`external_player_driver.rs`** (246 lines): PC mod ownership boundary. The original body state/list membership stays intact while an alternate solver owns an occurrence-qualified set of bodies.
  - types: `ModeBodyView`

### `crates/skate2-core/src/session/`

- **`selected_player_skate3.rs`** (294 lines): Explicit host selection, without any second player controller or solver.
- **`selected_player_tick.rs`** (1323 lines): Selected host tick over retained native input, Actor and physical owners. This does not implement the whole Simulation/camera/locomotion schedule.
  - types: `SelectedPlayerTick`, `SelectedTickStage`, `SelectedTickNumerics`, `SelectedTickRequest`, `SelectedQueuedTickInput`, `TickInputMode`, `AdmittedTickInput`, `SelectedTickInputDelivery`, `SelectedQueuedTickOutcome`, `SelectedTickSkater`
- **`skate3_input_capture.rs`** (68 lines): Observe the same hardware poll used by Skate 2, before conversion/remapping.
  - types: `CapturePad`, `IdlePad`
- **`skate3_input_focus.rs`** (36 lines): Gameplay control does not require permission to open the front end. In particular Net::ResumeGame masks simulation pauses independently of the top PauseMan state used by CheckForPauseState826477b0.
- **`skate3_offboard_marker_corpus.rs`** (81 lines) *(tests)*
- **`skate3_offboard_marker_drift_corpus.rs`** (78 lines) *(tests)*
- **`skate3_online_reset_corpus.rs`** (60 lines) *(tests)*
- **`skate3_player_corpus.rs`** (2027 lines) *(tests)*: Headless executable-embedded player over original Skate 2 world and rig owners.
  - types: `StreamCounts`, `StreamObservations`, `PlayerProbe`
- **`skate3_prop_lip_corpus.rs`** (120 lines) *(tests)*

## 3. Skate 2 systems that carry Skate 3 (TU3) behaviour

Shared Skate 2 code with Skate 3 branches or TU3 values: scoring, wipeouts, audio, breadcrumbs, collision, pedestrians. Each file names its Skate 3 parts in its own comments.

### `crates/skate2-audio/src/`

- **`global_effects.rs`** (605 lines): `Sk8::Audio::GlobalEffects` (0x43C bytes, constructor 0x82505c10): the two environment reverbs every wet send feeds, the two flanges, and the LFO task that sweeps them.
  - types: `Skate3Effects`, `LfoParms`, `LfoShape`, `EnvRvrbPresetParms`, `FilterParms`, `DelayParms`, `EnvRvrbRuntimeParms`, `FlangePresetParms`, `FlangeRuntimeParms`, `GlobalEffects`
- **`presentation.rs`** (512 lines): `Sk8::AudioPresentation` for the skater: the `PhysOut` values `FillSkaterData` 0x827f4c48 reads (supplied by the host as a [`PhysOutView`]) and the `SkaterData` it builds. The bit words of the native structure are separate fields here; their widths are kept where the native truncates.
  - types: `CollisionInfo`, `PhysOutView`, `SkaterData`
- **`reverb.rs`** (677 lines): `Sk8::Audio::SFXObj_Reverb` (MAIN SFX 5, 0x350 bytes, DMixIO 0x40000050): sets up `GlobalEffects`' two reverb instances from `aud_reverb` collections, cross-fades from one to the other over a second when the location's reverb key changes, and feeds the flange levels and the global reverb volume from its mixer outputs.
  - types: `RvrbParms`, `Reverb`, `Rig`
- **`system.rs`** (2196 lines): `Sk8::Audio::AudioSystem` (0x824e64b8-0x8250a300): the `Update` order, the HUD and menu sound path (`PlayMenuSound` 0x82504dd0 -> `cMsgPlayMenuSound` -> `PlayMenuSoundImp` 0x8250c738 -> `PlayFEHudSplice` 0x824e7b28 -> `UpdateFEHudSplices` 0x824fea50), the menu flange voice, the MAIN mixer state objects that produce the menu volumes, `GlobalEffects` (`crate::global_effects`), and the PLAYER and COLLISION state...
  - types: `SceneInfo`, `FeSound`, `GameAudioState`, `WorldAudioState`, `FeHudSplice`, `AssetStatus`, `MissionMusic`, `AudioMessage`, `MasterObject`, `Host`
- **`system_contest.rs`** (581 lines): The contests' side of `Sk8::Audio::AudioSystem`: the mission sound effects (`PlayMissionSFXImp` 0x824fecb8), the crowd banks of `data\audio\crowds.big` (`LoadCrowdsData` 0x824fdcc8, `GetCrowdsLoadStatus` 0x824f0818, `UnloadCrowdData` 0x824f0918) and the traffic banks they share their memory with (`LoadVehiclesData` 0x824fd868, `GetVehiclesLoadStatus` 0x824f0718, `UnloadVehicleData` 0x824fdbc0), the contest's...
- **`vmx.rs`** (137 lines): The VMX128 operation sequences the audio game code compiles its vector math to.

### `crates/skate2-audio/src/aems/`

- **`mod.rs`** (752 lines): Snd9's AEMS runtime: compiled sound "modules" in `.abk` (`ABKC`) banks, instantiated when the Csis class they subscribe to is created and interpreted by the "Aems Modules" timer (`TIMERSTAGE_BEFORECOMMANDS`, every sixth mix frame). PLAYER nodes start Skate's sample players (`SkateSamplePlayerFactory`), which mix to the mastering sub mix.
  - types: `SndRandom`, `Module`, `Bank`, `Instance`, `TimerState`, `Aems`, `ClassObject`

### `crates/skate2-audio/src/player/`

- **`contacts.rs`** (1868 lines): `Sk8::Audio::SFXObj_Contacts` (488 bytes): pushes, ollies, landings, wheel hits, manuals, foot catches and scrapes, hand grabs, kick-ups, air dismounts, board and body collisions.
  - types: `VoicedSplice`, `FootPush`, `FootInfo`, `HandGrab`, `FootDragData`, `Contacts`
- **`mod.rs`** (554 lines): The PLAYER state (`eMAINMAPSTATES` 1): `CSTATEMGR_Player`, `CSTATE_Player`, the `SFXCTL_PlayerPhysics` / `SFXCTL_3DPlayerPos` / `SFXCTL_3DBoardPos` controllers and the skater SFX objects. `Setup(0xFFF)` creates objects 0-11 in id order (`CreateSFXObjs` 0x82a7f130); `SFXObj_PlayerSpeech` (7) needs the speech system and is not ported.
  - types: `Cvars`, `CollisionInfo`, `SfxCtx`, `SkaterAudioStateData`, `PlayerState`, `PlayerFrame`, `PlayerManager`
- **`offboard.rs`** (1037 lines): `Sk8::Audio::SFXObj_OffBoard` (592 bytes, PLAYER object 10, `CreateObject` 0x82549858) and the `FootStepUtils` it drives. Off the board each foot has a `playercharacter_footstep` AEMS instance (`SK8_AEMS_Foley.csi`); for the local skater a foot touching down also plays its heel (walk collision, or the grass/metal table) and foot-push splices through four "FootStep SubMix" voices per foot, filtered by the splice's...
  - types: `SplicerFootStep`, `FootStep`, `OffBoard`, `Footsteps`
- **`physics.rs`** (753 lines): `Sk8::Audio::SFXCTL_PlayerPhysics` (800 bytes): copies the skater's `SkaterData` each update, derives the values the skater SFX objects read, and writes the controller's Nicotine inputs.
  - types: `PhysicsInputs`, `PlayerPhysics`
- **`tricks.rs`** (448 lines): `Sk8::Audio::SFXObj_Tricks` (76 bytes, PLAYER object 5): the local skater's board flip whoosh (`Class_Flips`, `Sk8_Air_Flip_Tricks.abk`) and the trick and trick-done cloth foley (`cloth_trick`, `Foley_Cloth.abk`), with the fun meter the flip instance reads.
  - types: `Tricks`

### `crates/skate2-audio/src/rwac/plugins/`

- **`pan2d.rs`** (442 lines): `rw::audio::core::Pan2D1` and `Pan2DKernel`/`Pan2DKernelBase`: pans 1, 2, 4 or 5 emitters over 2, 4 or 5 speakers (plus LFE) from an angle, distance, size and twist.
  - types: `Speaker`, `Emitter`, `Kernel`, `Pan2D1`

### `crates/skate2-audio/src/speech/`

- **`spch.rs`** (1901 lines): The EA `spch` speech library (`SPCH_*` / `iSPCH_*`, 0x829b3180-0x829b6ef0 and 0x82ee5680-0x82ee5938): the event databases (`VoxData`, the `<group>Output_Events.evt` files), the sample banks (`VOXBANKHDR`, the `.hdr` files), the queue of pending events per channel, sentence and sample choice, and the sample requests handed to the speech system.
  - types: `EventSpec`, `CsisRow`, `SampleRequestData`, `EventRef`, `VoxEventItem`, `VoxInGame`, `EventDatInfo`, `VoxBankInfo`, `PhraseChoice`, `EventChoice`

### `crates/skate2-audio/src/speech/context/`

- **`mod.rs`** (759 lines): The context systems (`Contexts`, the contextsystem 1.01.00 package at 0x82aaf270-0x82ab89a0, and `Sk8::Audio::Sk8ContextSysMan`): the cameraman's, the announcer's and the main cast's rules for when their speech projects speak.
  - types: `Group`, `SystemInfo`, `Source`, `Getter`, `Interface`, `Trigger`, `Condition`, `Sender`, `Context`, `ValidContext`

### `crates/skate2-audio/src/world/`

- **`emitter_system.rs`** (1479 lines): `Sk8::Audio::Sk8EmitterSystem` (`AudioSystem::mpEmitterSystem`, 0x220 bytes) over its base `Sk8::Audio::EmitterSystem` (0x82a81100-0x82a83ae8): the sound volumes of the `.ems` data files and the random location emitters.
  - types: `EmitterVolume`, `EmitterFiles`, `VolumeRef`, `VolumeTree`, `VolumeStatus`, `EmitterStatus`, `MusicVolumeInfo`, `VolumeStatusId`, `VolumeStatusView`, `EmitterIo`
- **`environment.rs`** (475 lines): The ENVIRONMENT state (`eMAINMAPSTATES` 2): `CSTATEMGR_Environment`, whose `EnterGamePlay` 0x8254fc18 creates one `CSTATE_Environment` with `Setup(1)`: `Sk8::Audio::SFXObj_Ambience` (0x58 bytes, TypeInfo 0x00020000, `CreateObject` 0x8254dc78), the location's ambience bed. The beds are looped 5-channel EA-XMA streams in `data\audio\ambience.big`, played through `SndPlayer1` with the resident `.snr` headers of...
  - types: `AmbienceInfo`, `AmbienceState`, `Ambience`, `EnvironmentManager`

### `crates/skate2-core/src/actors/`

- **`actor_breadcrumb_manager.rs`** (549 lines): Actor-owned skater breadcrumb storage, separate from camera breadcrumbs. Source: BreadCrumbManager::Implementation 0x82ce33b8, DropBreadCrumb 0x82ce1fd0, ring push_back 0x82ce1e30, and selected pop 0x82ce2d80. Collision, navmesh, and living-world answers must come from their owners.
  - types: `SkaterBreadCrumb`, `ActorBreadCrumbManager`, `BreadCrumbSelectionSource`, `ActorBreadCrumbMountError`, `ActorAppropriateTeleportError`, `Accepting`
- **`actor_breadcrumb_postupdate.rs`** (259 lines): SkaterManager::PostUpdate 0x825f8ba4..8c08 and breadcrumb Update82ce2230.
  - types: `ActorBreadCrumbPostUpdateError`
- **`actor_construction.rs`** (783 lines): Retained selected `Actor` constructor prefix, 0x825fe850..0x825fec20. Includes owned listener and unloaded graph construction; not a complete Actor.
  - types: `ActorConstructionError`, `ActorChaseFormationOwner`, `ActorIdentity`, `ActorPrefixPhase`, `ActorConstructorPrefix`, `ActorInitialFields`, `ActorGraphRegistrationError`, `ActorGraphRegistration`, `ActorConstruction`
- **`actor_teleport.rs`** (229 lines): Actor::Teleport 0x825ed1c8..0x825ed220, after a separate source-selected respawn transform has been chosen. This only publishes the Actor's reset.
  - types: `ActorTeleportError`

### `crates/skate2-core/src/animation/hands_face/`

- **`actor_animation_hands_face_runtime.rs`** (376 lines): Actor825f87b8 source dispatch order; manager owns outer hand command sets.
  - types: `HandFaceDispatchSource`, `HandFaceRuntimeError`, `Target`

### `crates/skate2-core/src/animation/runtime/`

- **`actor_animation_body_dispatch.rs`** (828 lines): `DispatchBodyAnim825f1040` / `EvalCurrAnim82d6e658` and the later body ACS Sync. A source-null current tree selects the bind node; retained nonnull trees produce fetch/SQT commands. Dependency-cycle batch splits remain checked.
  - types: `BodyAnimationDispatchSource`, `BodyDispatchError`, `PendingBodyDispatch`, `BodyPoseWork`, `BodyPoseWorkState`
- **`actor_animation_body_session.rs`** (330 lines): Same Actor and Simulation owners for asynchronous body evaluation. References: `DispatchBodyAnim825f1040` and `UpdatePhysics825ffd98`. `GetPhysUpdateData82c96d58` remains a later consumer: its trajectory and post-evaluation producers must run before its two pose copies.
  - types: `BodySessionError`, `ActorBodyAnimationServices`
- **`actor_animation_construction.rs`** (1275 lines): Retained `SkaterAnim` constructor prefix and Vault/cvar continuation.
  - types: `ActorAnimationInterface`, `ActorAnimationRegistry`, `AnimationLifetime`, `ActorAnimationInputs`, `ActorAnimationPhase`, `ActorAnimationConstructionError`, `ActorAnimationCvars`, `AnimationPointerState`, `AnimationFixedStorage`, `AnimationSettableAttribute`
- **`actor_animation_frame_phase.rs`** (794 lines): Actor animation phase2/3 on the actual retained `SkaterAnim` and tree owners. Action and Motion run between phase1 and phase2 in the outer Actor caller.
  - types: `AnimationPhaseError`, `AnimationPhaseDispatchError`

### `crates/skate2-core/src/assets/`

- **`character.rs`** (861 lines): Runtime-owned character graph assembled from a decoded recipe and RX2 assets.
  - types: `CharacterHierarchy`, `CharacterSkinningPart`, `CharacterSkinningState`, `CharacterSkinningError`, `CharacterScene`, `CharacterComponentAsset`, `CharacterModelAsset`, `CharacterRopaBinding`, `CharacterLodAsset`, `CharacterTextureAsset`

### `crates/skate2-core/src/camera/frame/`

- **`camera_session.rs`** (146 lines): Retained notification fields of `GameCameraManager82e37c08`. This projection does not claim completed Subject, `IrisCore`, rig or camera construction.
  - types: `CameraStartupError`, `CameraStartupFields`

### `crates/skate2-core/src/diagnostics/`

- **`test_log.rs`** (46 lines): PC Skate 3 physics trace lines (`SKATE2_TRACE_SKATE3=1`): the Skate 3 player's stages and events on stderr. Nothing is recorded otherwise; every call returns after one cached flag.
  - types: `StageGuard`, `TimedStageGuard`

### `crates/skate2-core/src/input/`

- **`stick_settings.rs`** (250 lines): PC port addition (user request): RPCS3's stick settings for each analog stick, applied to a pad's sample before the game reads it, as RPCS3 applies its own before the emulated pad (`PadHandlerBase::get_mapping`): each axis scaled by the multiplier and kept to its range (`TranslateButtonPress`'s `NormalizeStickInput`, `MultipliedInput`), then a radial deadzone whose inside ramps up to the anti-deadzone and whose...
  - types: `StickSettings`, `ControllerSettings`

### `crates/skate2-core/src/input/recognition/`

- **`juice_input_boot_tests.rs`** (267 lines) *(tests)*
  - types: `Platform`; fns: `new`, `observe_ready_without_reentry_lock`, `initialize_network`, `debug_address`, `console_name`, `connect`, `sleep_milliseconds`, `cvars`

### `crates/skate2-core/src/living_world/interaction/`

- **`living_world_grind_services.rs`** (642 lines): Selected query projection of ONE Simulation-owned `LivingWorld` occurrence. Root retains this owner across queries. A future complete Manager must retain the same occurrence; this does not execute or certify its unrelated Init.
  - types: `LivingWorldInitError`, `LivingWorldGrindServices`, `VehicleNearQueryError`, `PedestrianNearQueryError`, `DmoLocatorRouteError`

### `crates/skate2-core/src/living_world/pedestrians/`

- **`pedestrian_collision_contacts.rs`** (517 lines): DetermineOverallCollisionType82e808d8. Facts come from actual contact and entity/interface owners; this module performs no replacement entity lookup.
  - types: `CollisionContact`, `Skate3PedestrianContact`, `AnyContact`, `PedestrianCollisionType`, `OtherPedestrian`, `OtherCollisionOwner`, `CollisionClassificationFacts`, `ContactEntity`

### `crates/skate2-core/src/physics/`

- **`grind_data_runtime.rs`** (537 lines): Retained `GrindData` asset lifecycle,82cfb640/82cfd190/82cfb8b8. Arena-backed sources feed the independently reviewed native octree.
  - types: `GrindDataNumeric`, `HostGrindDataNumeric`, `GrindDataError`, `GrindDataSourceError`, `GrindSplineSource`, `GrindSplineOwner`, `GrindDataPrimitive`, `GrindAssetRecord`, `GrindDataQueryResult`, `DynamicGrindSplineProvider`
- **`physics.rs`** (6615 lines): Native animation-to-physics handoff recovered from Skate 2.
  - types: `PointGraph`, `JumpPhysicsCurves`, `GroundJumpTuning`, `GroundJumpVerticalInput`, `GroundJumpVerticalSolution`, `GroundJumpTangentialInput`, `GroundJumpTangentialSolution`, `GroundJumpInput`, `GroundJumpSolution`, `PhysicsAirVector3`

### `crates/skate2-core/src/physics/assembly/`

- **`simulation_assembly_backend.rs`** (2723 lines): Concrete safe owner for the three recovered `rw::physics::Simulation` pools.
  - types: `SimulationAssemblyJointEndpoint`, `SimulationAssemblyBackendAddFailure`, `SimulationAssemblyBackendInitError`, `SerialisedAssemblySimulationBackendInitError`, `SerialisedAssemblySimulationBindError`, `SerialisedAssemblyPartAttachmentBindError`, `SerialisedAssemblySimulationAddFailure`, `SerialisedAssemblyArenaRouteToken`, `SerialisedAssemblyRoutedInertia`, `SerialisedAssemblyRoutedBodyHandle`
- **`simulation_dmo_dynamic_pool_update.rs`** (607 lines): Per-frame DMO pool update and the DMO line collection.
  - types: `DmoMeshSlot`, `MeshArenaOwner`, `SimulationDmoPoolUpdateError`, `MeshWalk`, `DmoMeshGeometry`

### `crates/skate2-core/src/physics/assembly/parts/`

- **`dynamic_assembly_part_transform.rs`** (407 lines): Owner-qualified live Part get/set, 0x826c3480 / 0x82b25480. Raw Part cache inputs survive; live body getters retain the existing finite XYZ/canonical-W boundary rather than manufacturing native packed pointers.
  - types: `DynamicAssemblyPartTransformError`, `DynamicAssemblyPartDisplacementError`

### `crates/skate2-core/src/physics/board/collision/`

- **`skateboard_contact_surface.rs`** (317 lines): Retained surface-only projection of board `PostPhysics` 0x82b36a58 and `ChoosePhysicsSurfaceType` 0x82b24878. Contact production is shared, not repeated.
  - types: `SkateboardContactFrameError`, `CompletedSkateboardContacts`, `SkateboardContactSurfaceOwners`, `SkateboardContactSurfaceCvars`, `SkateboardPartSurface`, `SkateboardSurfaceWinner`, `SkateboardContactSurfaceError`, `SkateboardContactSurface`

### `crates/skate2-core/src/physics/board/runtime/`

- **`skateboard_ground_frame.rs`** (867 lines): One retained authored SkaterPool/static-ground collision component frame. Real board and skeleton membership supplies the full publication range. Flags3 adjacent board/skeleton pairs and DMO pairs share the native island partition. Collision workers execute in a documented serial host order.
  - types: `SkateboardGroundFrameSchedule`, `SkaterSkaterPairs`, `SkateboardGroundFrameInput`, `SkateboardGroundFrameError`

### `crates/skate2-core/src/physics/collision/`

- **`static_ground_export.rs`** (368 lines): Read-only host geometry for the alternate simulation, separate from native trajectory queries and their 64-triangle result capacity.
  - types: `ResidentGroundFeatures`, `ResidentGroundExportError`, `ResidentGroundGeometry`, `ResidentGroundIdentity`, `ResidentGroundJob`
- **`static_ground_pool.rs`** (2381 lines): Owned static-ground publication for processed Environment instances.
  - types: `StaticEnvironmentSource`, `StaticEnvironmentRoot`, `StaticGroundSourceError`, `StaticGroundPoolCreateError`, `StaticGroundCapacityError`, `StaticGroundSlot`, `StaticGroundAssemblyMesh`, `StaticGroundSlotSnapshot`, `StaticGroundPool`

### `crates/skate2-core/src/physics/player/conditioning/`

- **`physical_player_conditioner_animation_update.rs`** (128 lines): Animation.Update82e31970 after Physics.Update; not full `UpdateAll`.
  - types: `AnimationUpdateComplete`, `AnimationUpdateError`

### `crates/skate2-core/src/physics/player/output/`

- **`physical_player_movement_pose.rs`** (206 lines): Movement animation publication into the existing Skate 2 Actor pose owner. The normal Skate 2 writer still merges its hand/face buffers, records replay data, runs character corrections and skins the selected Skate 2 recipe.
  - types: `MovementRig`
- **`physical_player_phys_out.rs`** (530 lines): Selected components of the single `HiLOD` `PhysOut` owner (0x82e2f2a8). Constructor/reset completion describes this projection and its collaborators, never completion of a native frame or an implicit buffer swap.
  - types: `PhysOutOrigin`, `PhysOutValue`, `PhysOutComponent`, `PhysOutRemainderStage`, `PhysOutRemainder`, `PhysOutAccessError`, `PhysOutLifecycleError`, `PhysOutPublicationError`, `PhysOutSurfaceError`, `PhysOutMotionVector`
- **`physical_player_phys_out_audio.rs`** (321 lines): The retained `PhysOut` values `AudioPresentation::FillSkaterData` 0x827f4c48 reads, copied for the audio host. Direct const component reads, no write latch, like the presentation getters.
  - types: `SkaterAudioCollision`, `SkaterAudioPhysOut`
- **`physical_player_phys_out_grinds.rs`** (152 lines): Complete field values for the native grind outputs; mounted only by the actual nineteen-component constructor and reset in its original order.
  - types: `GrindChromosome`, `Grinds`, `Filtered`
- **`physical_player_phys_out_scoring.rs`** (1100 lines): Complete retained Scoring2 constructor fields (82e2c768) and Reset82e2a3f8. Native empty lists retain all four constructed Scorable elements. Reset clears counts, preserving inactive elements and context UID storage exactly.
  - types: `IntentTuple`, `TrickInput`, `TrickSettings`, `Scorable`, `ContextUid`, `ContextUids`, `ContextData`, `ContextGroups`, `Threshold`, `Thresholds`
- **`physical_player_phys_out_simple.rs`** (929 lines): Concrete primitive component constructors inside the retained `PhysOut` owner. These values are not an independently constructed output owner. Motion and System remainders exclude fields already retained by the existing projection. Mount only through the complete nineteen-component constructor sequence.
  - types: `MotionRemainder`, `SystemRemainder`, `SkateboardReckoning`, `Physics`, `Camera`, `Intents`, `OffBoard`, `Trajectory`, `HandPlant`, `Air`

### `crates/skate2-core/src/physics/player/output/conditioning/`

- **`physical_player_phys_out_conditioners.rs`** (616 lines): Complete eleven-child conditioner constructor82e316f0, called by `HiLOD` at 82e03f58 after `PhysOut` and before `SurfacePhysics`. This owner retains partial allocations separately from the native published pointers.
  - types: `AllocationSite`, `ConstructionError`, `ResetAllError`, `Retained`, `Bound`, `Scoring`, `PathRecording`, `Conditioners`
- **`physical_player_phys_out_conditioners_score_holder.rs`** (122 lines): `ScoreHolder82df35c8`: retain the holder before its nested Sim Sequence allocation, then perform Reset82dee248 against that same Sequence.
  - types: `ScoreHolder`, `MissingSequence`
- **`physical_player_phys_out_conditioners_score_module.rs`** (323 lines): Complete nested `ScoreModule` constructor82dfb508 and Reset82df26f0.
  - types: `ResetSuffix`, `CurrentCollector`, `ScoreModule`
- **`physical_player_phys_out_conditioners_wipeout_score.rs`** (213 lines): `WipeoutScorer82df5618` owns three scoring children even in offline startup. This is constructor ownership only; it does not admit or spawn DMO gameplay.
  - types: `EntityScorer`, `DmoScorer`, `DmoScorerError`, `CharacterArea`, `BoneInfo`, `WipeoutScorer`

### `crates/skate2-core/src/physics/player/output/conditioning/scoring/`

- **`air_lifecycle.rs`** (252 lines): AirCollector Enter82df49d8 / Exit82dfb898. Extend the existing reset owner.
- **`air_output.rs`** (91 lines): AirCollector FillAirPeriodData82deacb8 / thresholds82def4d0 / Fill82defaf0.
- **`air_tricks.rs`** (233 lines): AirCollector trick lifecycle82df7a90/7c18/8f98/9080.
  - types: `AirTrickFrame`
- **`air_update.rs`** (68 lines): AirCollector::Update2_TrickAnalysis82dfbcb0, preserving native metric order.
- **`analysis.rs`** (123 lines): Nursary::ProcessAnimData82dfc358 / StartScorable82df74a8.
- **`catalog.rs`** (2173 lines): Original gScorableInfo823671b0: IDs, consumed/base tricks, category, type and name.
  - types: `ScorableInfo`
- **`conditioner.rs`** (134 lines): PhysOutConditioner_Scoring.Update82e2b438 after Animation.Update.
  - types: `ScoringUpdateError`
- **`database.rs`** (208 lines): TrickDatabase::ConstructFromAttribSys82dfb490 and TrickScoringSettings::Set82de9a70.
  - types: `TrickDatabase`
- **`frame.rs`** (227 lines): Native collector inputs sampled from the original retained PhysOut fields.
- **`grind.rs`** (214 lines): GrindCollector82df9858/9600/9790; score the existing filtered grind output.
  - types: `GrindFrame`
- **`ground.rs`** (726 lines): GroundCollector trick analysis82dfbfe8, using the existing retained owners. Inputs are samples of the original PhysOut/GlobalMetrics/cvar fields; the live conditioner must provide these, not infer tricks from controller input.
  - types: `GroundFrame`, `GroundScorable`
- **`holder.rs`** (282 lines): ScoreHolder announcement, repeat history and banking; native82dee2e8..82df3e38.
- **`hud_frame.rs`** (179 lines): PhysOutModule_Hud827c79b8 reads completed Scoring2/Animation output.
- **`module_dispatch.rs`** (460 lines): Original retained collector dispatch, vtables8236c2b8..8236c3ac. No additional collectors are allocated by this path.
  - types: `CollectorFrame`
- **`module_finish.rs`** (257 lines): ScoreModule runout82de9f80, gestures82dea060, banking82df61e8/2888/2b00.
  - types: `SequenceFinishFrame`
- **`module_update.rs`** (172 lines): ScoreModule.Update82dfcd18. Inputs borrow the already-conditioned PhysOut; the existing Update5 reset owns clearing output events before this stage.
  - types: `ModuleFrame`, `ModuleTuning`
- **`offboard.rs`** (270 lines): OffboardCollector82df8400/c250/c308. Uses actual skeleton and deck frames.
  - types: `OffboardFrame`
- **`timed.rs`** (103 lines): TimedArcLengthScorable::Update82df2130. Extend the retained native tracker.
  - types: `ArcMeasure`
- **`wipeout_lifecycle.rs`** (267 lines): WipeoutScorer Update82df6cb8 / UpdateInWipeout82df5bc0 and native lifecycle.
  - types: `WipeoutFrame`, `WipeoutTuning`

### `crates/skate2-core/src/physics/solver/bodies/`

- **`simulation_dynamic_update.rs`** (544 lines): Ordinary-finite host reconstruction of `RigidBody::DynamicUpdate`.
  - types: `SimulationDynamicUpdateBody`, `SimulationDynamicUpdateConfiguration`, `Fixture`
- **`simulation_rigid_body_pool.rs`** (6935 lines): Safe semantic model of the preallocated `rw::physics::Simulation` body pool.
  - types: `SimulationMutableInertia`, `MutableInertiaState`, `RetainedInertia`, `SimulationRigidBodyFrameIdentity`, `SimulationRigidBodyDynamicUpdatePhaseIdentity`, `SimulationRigidBodySolverDeltaRow`, `SimulationRigidBodySolverDeltaEndpoint`, `SimulationRigidBodySolverDeltaTransactionError`, `SimulationRigidBodyIndexedSolverDeltaError`, `SimulationRigidBodyIndexedSolverDeltaPlan`

### `crates/skate2-core/src/physics/solver/contacts/`

- **`simulation_contact_jacobian.rs`** (9883 lines): Evidence-bounded value layout for `rw::physics::ContactJacobian`.
  - types: `SolverDeltaRowToken`, `ContactBodyToken`, `SimulationContactBuildWordVector`, `SimulationContactBuildInput`, `SimulationContactCommonBodyInput`, `SimulationNetworkContactSelectedBodyInput`, `SimulationNetworkContactDrainInput`, `SimulationNetworkContactDrainEstimateSite`, `SimulationNetworkContactDrainNumericBackend`, `SimulationAssemblyGroundMaterialInput`

### `crates/skate2-core/src/physics/solver/drives/`

- **`simulation_drive_jacobian.rs`** (9640 lines): Evidence-bounded foundation for `rw::physics::DriveJacobian` construction.
  - types: `SolverDeltaRowToken`, `DriveSlotToken`, `SimulationDriveJacobian`, `SimulationDriveHorusRows`, `SimulationDriveHorusAccumulatorStoreOrder`, `SimulationDriveHorusAccumulator`, `SimulationDriveBodyGeometryInput`, `SimulationDriveGeometryHost`, `SimulationDrivePredictionBodyInput`, `SimulationDrivePointPredictionHost`

### `crates/skate2-core/src/physics/solver/joints/`

- **`simulation_joint_build_stage1.rs`** (813 lines): Evidence-bounded first stage of `rw::physics::JointJacobian::Build`.
  - types: `SimulationJointBuildStage1Body`, `SimulationJointStage1Rqd`, `SimulationJointBuildStage1`, `HostBackend`, `InjectedError`, `FailingBackend`
- **`simulation_joint_build_twist_prediction.rs`** (732 lines): Recovered typed continuation for `JointJacobian::Build` 0x82ae6170..0x82ae6304.
  - types: `JointRqdRows`, `JointAngularBodyValues`, `JointTwistStage1Values`, `FinalSwingPackets`, `JointAngularControlStack`, `JointPostPredictionBoundary`, `Event`, `Failure`, `OracleBackend`

### `crates/skate2-core/src/session/`

- **`player_mounted_recipe_load.rs`** (939 lines): Mounted first-local-marquee completion into the SAME supplied player loader. Synchronous host IO fulfills source asset stages; no native thread scheduling or registration completion is implied. All partial owners survive errors.
  - types: `PreparedRecipe`, `RegisteredRecipe`, `RegisteredAsset`, `MountedRecipeRegistry`, `MountedRecipePhase`, `MountedPlayerRecipeLoad`, `MountedRecipeError`
- **`selected_location_manager.rs`** (241 lines): `Sk8::cLocationManager` over the registered arenas: `RegArena` (`0x82a8eff8`) adds every named `tLocationDesc` of an arena's kind-21 `tLocationDescData` (`AddLocationData` `0x82a8e980`) under `GetHashValue32(name)`, after the entries already held for that hash, and `DRegArena` (`0x82a8e430`) removes them again. The global locator arenas (`cGlobalFileLocationManager::LoadGlobalLocations` `0x82a8f3b0`, every...
  - types: `Location`, `LocationManager`
- **`selected_production_input_owner.rs`** (670 lines): One-shot selected input composition. The application supplies real pad and Juice platforms, mounted configuration, scheduler, and Simulation fields. Draft module awaiting root integration into the registered application.
  - types: `SelectedInputBootstrap`, `ProductionInputStartError`, `ProductionInputBindError`, `ProductionInputBindFailure`, `SelectedProductionInputOwner`, `ProductionInputFrameError`, `SessionMarkerStateError`, `PlayerUiTeleportError`
- **`selected_session_frontend_world.rs`** (278 lines): The world behind the main menu. `GameModule::OneOffLoad` 0x8273b1a0 boots at the non-streamed `Z_TrainingStart_Fixed` location, and the simulation runs there for the ambient AI skaters the front end camera watches (`AISkaterManager` in its minimal model set, front-end distances). The first player is there too: `ResetGame` 0x8273a680's `PlayerUI::OnReset` 0x828aadd0 asks for it and `PlayerUI::Update` 0x828af9f8,...
  - types: `Phase`, `FrontendWorld`
- **`selected_session_marker.rs`** (550 lines): Production owners of `Sk8::SessionMarker`, `PlayerUI::UpdateSessionMarker` and the teleport command they request, over the selected session's Actor, World, terrain, cvars and input. Game-layer owners the port has no producer for answer with the state their native constructors leave.
  - types: `SelectedSessionMarkerError`, `SelectedSessionMarker`, `SelectedSessionMarkerSources`, `SkaterManagerCommandError`, `SelectedSkaterManagerCommands`
- **`selected_session_party_play.rs`** (268 lines): `FrontEndState_PartyPlay` over the main menu: its character and location lists, the `partyplay` viewer, and the game it hands over.
  - types: `PartyPlay`

