# Skate 3 scoring audit (TU3 vs the Skate 2 port in Skate 3 mode)

Goal: scoring 1:1 with real Skate 3 (flatground, vert, held vert tricks,
multipliers, landings). Skate 3 mode runs the Skate 2 ScoreModule port with `use_skate3`:
`ScoringAttributes` serves Skate 3 values by Skate 2 layout offsets (`skate3_catalog`), and Skate 3
branches sit behind `skate3_enabled()`. Each TU3 function below is diffed against the Skate 2
original (Skate 2's executable) and our port, and every difference ported. Status: `-` not yet diffed, `same`
no logic difference, `ported` difference brought in, `open` difference found and not yet ported.

Graph checks: every collector curve base the port passes (320, 400, 800, 880, 960, 1040, 1120) is a TU3 base in `scoring_fields.rs`; the ground trackers' TU3 graph indices (0,1), (3,2), (7,6) match `TimedArcLength::new`; scalars resolve by attribute hash and a missing Skate 3 key is an error, not a zero.

Cross-cutting TU3 differences:
- ScoreHolder +0x1C5D = PhysOut_Air+452 (locked trajectory), copied by Update 82DA3520 (ported:
  `skate3_air_reckoning`). Read by 14 functions (see their rows).
- ScoreHolder +0x1C5C: the air reward-pending flag (Skate 2 has it); no Skate 3 difference.
- Scorable count 332 (`SCORABLE_CAPACITY`) and the Skate 3 catalog.

| TU3 | Function | S2 match | Status |
|---|---|---|---|
| 0x82da33e0 | ScoreModule::Update | callees 0.669 | ported +0x1C5D copy; the rest matches (constructor flag +0x88 to Scoring2, runout memory 30, takeoff normal, collector order, meters, gestures with Skate 2's id 35 because the published gesture is the Skate 2 actor's, distance from damped COM) |
| 0x82da8a70 | AirCollector::Update2_TrickAnalysis | gap 0.491 | ported (+452 gate, holder gap gate) |
| 0x82da8be0 | AirCollector::UpdateSpinMetric | data 0.689 | ported: +0x1C5D holder gate; points = (curve(880) x in-air scalar) x repeat penalty in that order. The old engine's `Rotation` matches the TU3 branches (player tracker while a valid trick, else the board projected on the reckoning up +96 via 82D2CDC0, banking at each switch); the up is `PhysOutSystemVector::Up` = reckoning +96 in Skate 3 mode. Allowance +0x63C (Hash_25B11E405C140E61) matches. Rounding-level only: TU3 banks degrees per switch, the port banks radians and converts once |
| 0x82da8eb8 | AirCollector::UpdateFlipMetric | gap 0.466 | ported (+0x1C5D gate; rest matches Skate 2: category 2, states +1600) |
| 0x82da8ff8 | AirCollector::UpdateTrajectoryMetrics | order 0.651 | ported (+0x1C5D gate on both holder additions; curves +976/+1056/+816 via skate3_catalog to verify) |
| 0x82da93d8 | AirCollector::UpdateTricks | data 0.700 | ported: the hold/hippy section also needs the locked flag clear (returns 0), and the holder add is gated by +0x1C5D. The rest matches: type 5, conversion, grab-to-grab chain (+1688), hold curve (+1120 object, +1136/+1168), hippy bonus (+1536). Open: verify skate3_catalog maps those Skate 2 offsets to the same TU3 fields |
| 0x82da9d68 | GrindCollector::Update2_TrickAnalysis | data 0.755 | ported: holder add gated by +0x1C5D; the timed scorable measures step length past 0.005 m (TimedArcLength flag 1). The grind-grab id swap (layout byte 148, type 5) matches |
| 0x82daa5e8 | GroundCollector::UpdatePowerslide | callers 0.824 | ported: holder add gated by +0x1C5D; step length past 0.005 m. Start/end/max-score flow matches |
| 0x82daa8e0 | GroundCollector::UpdateManual | callees 0.788 | ported: TU3 has no speed condition (slow manuals score; S2's kAllowSlowManuals and speed gates are gone); travel is the step along the board's velocity direction (reckoning +160, TimedArcLength flag 0) past 0.005 m; holder add gated by +0x1C5D. Revert-count limit (6), connector rule and bit 24 match |
| 0x82daaec8 | GroundCollector::UpdateGrabs | callees 0.821 | ported: holder add gated by +0x1C5D; step length past 0.005 m. Coffin-only rule and grab change flow match |
| 0x82dab8b0 | HandplantCollector::Update2_TrickAnalysis | order 0.777 | ported: height points add gated by +0x1C5D |
| 0x82daba10 | HandplantCollector::UpdateHandplants | callers 0.892 | ported: holder add gated by +0x1C5D; type 6, start-tick inheritance and curve (+320, frames/60) match |
| 0x82dabfb0 | OffboardCollector::UpdateHippyJumps | callees 0.564 | ported: the 180 count is the physics' landing half turns (PhysOut_Ground +0x138, `landing_half_turns_312`), not S2's spin trackers; its points are curve(+880) x 0.5; holder add gated by +0x1C5D; the flip bonus has no validity check. Start 82DABE98: point scalar 0.2 when the damped system velocity's part across the up vector (reckoning +96) is under 4 m/s, else 1.0 |
| 0x82da5ef0 | ScoreHolder::UpdateActiveScorable | callees 0.774 | ported: the active-scorable record also needs +0x1C5D clear |
| 0x82da6198 | ScoreHolder::EndTrickGeneric | callees 0.837 | ported: no line/sequence/type history while +0x1C5D is set; the callers' sequence-score additions stay unconditional (TimedArcLength 82DB0780) |
| 0x82da6260 | ScoreHolder::EndAirTrick | data 0.769 | ported: the air-sequence banking (category 5 / 3 pending) is skipped while +0x1C5D is set |
| 0x82da5f98 | ScoreHolder::AnnounceTrickEnding (sub_82DA5F98) | - | ported: the tricks-ended record stays; the sequence add (pattern matcher, recent-four ring) is skipped while +0x1C5D is set |
| 0x82da6468 | ScoreHolder::RewaredAirSequence | callees 0.773 | same. +0x1C5C is the reward-pending flag Skate 2 has; +0x1C4E (cleared on a default landing) is `skate3_clean_landing`, already reset at air start, so equivalent |
| 0x82da6538 | ScoreHolder::EndSequence | order 0.638 | same (holder +0x1C line, +0x20 sequence, +0x24 banked, +0x28/+0x2C air). Output-only extra: holder +0x12A8 (sequence grind length, added by GrindCollector::Exit 82DA9B74, published and zeroed here, zeroed by CheckSequence/Reset); no consumer in the port |
| 0x82da37b0 | ScoreModule::CheckSequence | callers 0.638 | same: wipeout/runout branches, scale +85, inline momentum tiers (88/89, 90/91, 95/96, allow-drop +93) = `MomentumMeter::end_sequence`, line stoke over 1.01. Output-only extra: Scoring2 +0x14 = line score on a wipeout; no consumer |
| 0x82da4010 | ScoreModule::ManageCurrentCollector | callers 0.822 | same (exit scoring per kind, wipeout-interrupted flag, offboard runout / in-air loss) |
| 0x82da3dd8 | ScoreModule::CalculateScoreCollectorType | callers 0.866 | ported: SmoothFilteredPhysicsState is inlined unchanged; TU3 adds one gate, no Grind collector while PhysOut_Grinds+0x146 is set (Tipslide FillPhysOut 82D42788 copies its state +0x63 there). Published from our tipslide_326 as `skate3_tipslide_blocks_scoring`, read in select_collector |
| 0x82da3c68 | ScoreModule::DigestExternalInput | callees 0.751 | same (TrickPatternMatcherGroup::ActivatePattern inlined) |
| 0x82da4238 | ScoreModule::FillPhysOut | callers 0.793 | same (collector and holder fill, line/momentum seconds and points, multiplied HUD score, gesture flags, tapped-trick threshold, offboard/onboard transitions); decompiler names are shifted Skate 2 labels |
| 0x82da3070 | ScoreModule::Reset | callees 0.765 | same. TU3 resets mCurrentGesture to 37 (its gesture count); our Skate 3 gestures publish in Skate 2 numbering through native_gesture_id, whose "none" is 35, so the sentinel stays 35 |
| 0x82da8078 | AirCollector::Enter | gap 0.457 | same: reward, reset, most-recent type 8/5 scalars (+419/+420), stationary = (states+0x66C)^2 > speed across reckoning up (+96), then switch/fakie (+0x9D/+0x9B, cleared together) scalars (+417/+421). The decompiler's offboard/pushing names are Skate 2 labels on TU3's animation layout |
| 0x82da89a0 | AirCollector::UpdateGapDetection | gap 0.519 | same: its inner skip is the collector's locked byte, already handled by Update2's `locked` |
| 0x82da88e8 | AirCollector::CalculateGapScore | constants 0.862 | same (curve bases 1440/1280/1360/1200, summed in order) |
| 0x82da8360 | AirCollector::FillAirPeriodData | order 0.819 | same fields; wiped_out from the filtered state == Wipeout in Skate 3 mode (ported; Skate 2 keeps is_wiping_out) |
| 0x82da92b8 | AirCollector::StartNewTrick | data 0.790 | same |
| 0x82da9218 | AirCollector::EndCurrentTrick | callers 0.738 | same (UpdateScorableList 82DA91C0 = `remember_first`) |
| 0x82da9920 | AirCollector::UpdateScoreThresholds | callers 0.857 | ported: skipped while the collector's locked byte is set (`skate3_locked`, stored by Update2). Open: the threshold table is read from Skate 2's general layout in Skate 3 mode (big-trick flags only, no points) |
| 0x82da9a18 | AirCollector::FillPhysOut | order 0.664 | same (in-air flag, thresholds, FillAirPeriodData) |
| 0x82daac10 | GroundCollector::UpdateRevert | callers 0.820 | ported: TU3 starts a revert whatever side was last seen (none seen = id 5) and ends an invalid one without banking; replaced Smokey's non-native wait for a typed revert (his held-darkcatch legacy) and S2's errors in Skate 3 mode. Side tracking and raw started/finished match |
| 0x82daae60 | GroundCollector::StartNewGrab | callees 0.713 | same: StartScorable(name, id, 1.0, grab scorable, 0.0), then AttemptToAnnounceTrickBeginning |
| 0x82dab2a8 | GroundCollector::IsActivitySequenceWorthy | data 0.832 | same (TU3 literals 3 and 8 = the Skate 2 cvar defaults 3 and 3+5). Found a feed gap: TU3 reads Ground+0x10C = the pumping acceleration (Update5 82DB6FD0 inlines Pumping.Fill from the pumping object +0x30), which Skate 3 mode never published, so pumping never kept a ground sequence alive. Now published with the four skeleton absorptions (`FrameOutput::pumping`) |
| 0x82dab588 | GroundCollector::UpdateScoreThresholds | callees 0.831 | same structure (two fsel maxima of current vs finished lengths, two threshold updates); same open note on the Skate 2 threshold table |
| 0x82da9c88 | GrindCollector::EndCurrentTrick | callers 0.831 | same: TU3 ends through EndTrickGeneric 82DA6198 (histograms, skipped while +0x1C5D is set; our record_finished) where S2 calls EndTrick(id, 0.0), which adds no points |
| 0x82dab7d0 | HandplantCollector::UpdateHeight | order 0.861 | same: below-hips minus below-deck drop, running maximum, states curve +104/+112 |
| 0x82dac498 | Nursary::ProcessAnimData | callers 0.628 | same: raw id for bits 0x03000000, scoring_trick lookup and trick-type enable (+0x70), ground grab ids gnd_bsgrab/dbl/fs (67..69 by name in both catalogs) dropped from the grab bit |
| 0x82dac698 | Nursary::StartScorable | constants 0.791 | same result: TU3 (sub_82DAC620) scales penalty x ExternalInput+0xAE0 x scalar; +0xAE0 is a Skate 3 field that only the input reset (8275D4A0) writes, as 1.0 (no other store in 0x82300000..0x82A00000), so it equals penalty x scalar exactly |
| 0x82dac780 | Nursary::CheckForHippyFlip | constants 1.000 | same |
| 0x82dac880 | RotationTracker::Update | callees 0.919 | same: per-tick angle wrapped to ±pi, accumulated, matrix kept (old engine `Tracker::update`) |
| 0x82dac930 | RotationTracker::UpdateWithBanking | constants 0.860 | same: TU3 hard-codes 30 degrees where S2 passes kMinAngleToBankInDegrees, whose default is 30.0 |
| 0x82dac9d0 | RotationTracker::CalculateRotation | constants 0.853 | equivalent: Z projected on the previous Y plane, a per-tick discontinuity rejected (dot test), AngleBetweenVectors; the port's atan2 form differs only in rounding |
| 0x82da5de0 | ScoreHolder::AttemptToAnnounceTrickBeginning | callees 0.485 | same: inlined IsReadyToBeAnnounced + AnnounceBeginning (tap scalar general +348 = key 2577DF0FCE5251E4) |
| 0x82da60f0 | ScoreHolder::AnnounceTrickCancelled | callees 0.511 | same: inlined Scorable::AnnounceEnding (an unannounced trick becomes a tap trick) |
| 0x82da5d18 | ScoreHolder::CalcPointPenalty | data 0.866 | same (penalty unless category 5; general curve +0x104 over the signed line history +0x1000) |
| 0x82da6630 | ScoreHolder::FillPhysOut | callees 0.728 | - |
| 0x82da48b8 | Sequence::End | order 0.751 | same, plus one TU3-only write: Scoring2+0x70 = count of the unique dynamic objects (up to 8) GlobalMetrics::AddDMOContextData 82DA6F64 records over/under during the line (Sequence+0x6F8 vector). Open, no effect on points: not published |
| 0x82da4a48 | Sequence::FillPhysOut | callers 0.709 | - |
| 0x82da2340 | TrickScoringSettings::Set | callees 0.553 | same layout offsets as Skate 2 (+136 id, +160 base, +112 type, +152 fakie, +116 difficulty, +124/+128/+140/+144, intents from +16 with count at +10); TU3 skips +156. In Skate 3 mode shared names take Skate 3 points, completion delay and variant (`lookup_key`), Skate 3-only names come from `skate3_catalog::lookup` |
| 0x82da25b8 | TrickScoringSettings::LookupTrick | constants 0.787 | - |
| 0x82da1bf0 | TrickDatabase::CollectTrickData | gap 0.424 | - |
| 0x82db0588 | TimedArcLengthScorable::Update | callees 0.748 | ported (`ArcMeasure::Skate3`): flag 1 = step length, flag 0 = step along reckoning +160; both count past a literal 0.005 m (S2: the kMinDistancePerFrameToAccumulate cvar); time accrues only while eligible; score fsel/fmadd order matches |
| 0x82da7888 | GlobalMetrics::DigestExternalInput | callees 0.685 | same order (vehicle, pedestrian, skater, DMO, static, over-DMO ring of 8). TU3 copies the whole external flags word 2780 into Scoring2 where S2 copies its HoM bit, and resolves static-object types through scoring_entities::EntityType; context data only, no points |
| 0x82da7b50 | AirGapData::Update | order 0.667 | same (horizontal distance from the gap start, count/accumulate/maximum on a scoring end). AirGroundData::Set 82DA7A38 is constants 1.000: grass 0x7F==10, stairs 0xF80==0x400 over the raw Skate 3 surface id |
| HoM | WipeoutScorer::* (0x82dad0a8..0x82daf810) | - | ported: TU3 scorer in wipeout_skate3.rs (window run-up, decaying bone damage, TU3 bonus/output); see `21-hall-of-meat.md` |

The remaining functions in 0x82DA14A0..0x82DB0E00 (constructors, containers, context data) are
lower risk; diff them after the rows above.

Also decide: the old engine's `scoring_runtime` (physics/modes/skate3/scoring_runtime.rs) runs as a
second scorer each tick. Once the ScoreModule path is complete, keep one authoritative scorer.
