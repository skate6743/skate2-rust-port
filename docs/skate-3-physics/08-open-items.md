# Open items and known gaps

What is not ported, not verified, or knowingly different. TU3 addresses.

## Not ported

| Item | TU3 | Note |
|---|---|---|
| Skitching (104), FollowPath (105), Sleeping (700) states | | `physics/frame.rs` returns an error if one is ever selected; the port never selects them |
| `Skeleton::UpdateRootTransformsOffBoard` (pushing-only root update with the push turn) | 0x82BE1058 | the pushing follow carries the BipedGround biped to the clamped COM and turns it instead (`skate3_player.rs` → `skater.pushing_follow`) |
| Pushing: emergency exit, partial ragdoll and collision response while holding a prop; grabbable re-check | PhysState_OffBoard_Pushing (vtable 0x82327364) | the gameplay part (grab, move by forces, follow, hand IK) is ported |
| Skate 3 per-district emitter volumes (`reverb_<district>.ems`, `sfx_`, `music_`, `speakers_`, `crowds_`) | SFXObj_Reverb::UpdateParams 0x824de548 takes them first | Skate 2's city volumes are switched off in districts; districts use Skate 3's painted reverb layer and the default key |
| Darkcatch effect values driven by the jitter mixer (effects whose "use attributes" flag is 0) | SFXObj_Jitter at AudioSystem+0x2D4, 0x824EF458 | those effects stay at plugin defaults (DistortionClip 100 = bypass, IIR gain 1 = flat) |
| Reverb mixer inputs | UpdateNicotine 0x824df468 has 7 inputs | the port keeps Skate 2's 4-input mix data |
| Class_Flips hippy special cases | TU3 +0x1F6 byte / +mNumBails byte → type 3 | the port keeps its `was_hippy` levels |
| Score output records with no consumer | ScoreHolder::FillPhysOut 0x82da6630, Sequence::FillPhysOut 0x82da4a48, TrickScoringSettings::LookupTrick 0x82da25b8, TrickDatabase::CollectTrickData 0x82da1bf0 | not yet diffed (`20-scoring-audit.md`) |

## Knowingly different (timing)

| Item | TU3 | Port |
|---|---|---|
| Off-board (BipedAir) trajectory completion | completes at the next tick start (0x82DB3C78 → 0x82D6CC50 → 0x82D6D020); a pending tick's packet 0x82D6E3F8 uses the launch arc and does not advance the blend timer | completes at launch: BipedAir enter/relaunch ticks differ by one blend tick (PhysicsAir's selector already completes next tick, the big-air fix) |

## Not verified in play

- Online wipeout limit with a real online bail (offline stand-in verified: reset at 4.0 s).
- Tiny hop (2488 bit 28 = OB_DropIn) cannot stick: the per-frame ProcessedPhysIn reset (0x82BF9EF0)
  clears it; the trajectory selector scores misses 0 and any positive landing wins (0x82D68FE8).
  Needs a launch trace.
- Auto push riding switch: the motion graph entered Riding.Turning.Switch before any input and
  never took the anticipation in one scripted cycle; not shown to differ from TU3.
- Caveman landings on banks (no clean native comparison yet; flat runs match: 5.76–6.57 vs native
  5.78–6.54 m/s).
- Prop pushing distances against TU3 (scripted pull: 2 m at ~2 m/s; Skate 2 physics stops on the
  same scenery).

## Resolved questions

- Heavy-landing input to the Hall of Meat: `export_frame`
  publishes the wipeout request 6 as `GameplayOutput::heavy_landing` (kind 1/2/0, speed; TU3 Update5
  0x82DB71C8), and `physical_player_phys_out_skate3.rs` passes it on as the special wipeout info.
- The A-press reply's stance conversion: checkpoint replies take
  `host_teleport`'s conversion; TU3 sends markers and checkpoints through the same Actor teleport
  0x825926f8, and TeleportPlayerNow 0x82DB8998 applies no mirror.
