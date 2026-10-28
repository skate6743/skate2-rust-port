# Audio implementation log

Started 2026-10-05, for menu, skating and the other sounds as close to 1:1 with the IDA
reference as possible. Every fact below was checked in
IDA (debug `sk82_na_m.xex`, read-only), the PDB layouts or the shipped data. Addresses are
debug-xex VAs. Decisions D1-D4 from [README.md](README.md#6-design-decisions):
D1 = pure-Rust XMA decoder in its own LGPL-2.1+ crate (2026-10-05); D2-D4 as recommended.

## Reading the decompiler output

* Xbox 360 ABI: every argument takes a parameter slot. A float argument goes in an FPR **and
  consumes the matching GPR**, so Hex-Rays' names for integer arguments after a float are shifted
  by one. `ReChannelGainMix(dst r3, src r4, gain f1, dstChannels r6, srcChannels r7, numSamples r8)`.
* The IDB's types for RWAC classes (`Mixer`, `System`, `SndPlayer1`, `Param`) do not match the PDB
  layouts (fields shifted). Map offsets with the PDB layout, not the Hex-Rays member names.
* `py_eval` runs code with separate globals/locals: wrap scripts in a function.

## XMA (crate `skate2-xma`, LGPL-2.1-or-later)

* Port of FFmpeg's `wmaprodec.c` XMA2 path (decode flags 0x10d6, 512 samples/frame, 2048-byte
  packets, 15-bit frame length prefix). Validated against ImageMagick's ffmpeg 4.2.3 `xma2` on 12
  bank samples (22050/24000/36000/44100/48000 Hz, mono, stereo, a 4-channel pair): identical
  length and alignment, max |diff| <= 1.8e-7 (float rounding).
* **The hardware emits frame 0.** FFmpeg's `skip_frame` is not applied: 3,573 of 3,595 RAM sample
  pairs hold exactly `ceil((numSamples + 384) / 512)` frames, matching `EaXmaDec` skipping 384.
* `EaXmaDec::StartPair` 0x82c14200: sub-block word `w`: `w >> 2` = sub-block bytes incl. `w`,
  `w & 3` = `XMACONTEXTINIT::DecoderTable` passed to `Reset` 0x82c0e650 (two-bit rate class:
  0 = 24k, 1 = 32k, 2 = 44.1k, 3 = 48k; data: 22050/24000 -> 0, 36000/44100 -> 2, 48000 -> 3).
  Frames to skip `decoderSkip / 512`, or `(decoderSkip + 384) / 512` when `feedType == 0`;
  `ParseToNextFrame` 0x82c0e8a0 walks 15-bit frame lengths across packets (headers skipped).
* `DecodeEvent` 0x82c169f8: per request, skip `(decoderSkip(+384)) % 512 + mNumPaddingSamples`
  samples, deliver `numSamples - decoderSkip`; at the end `mNumPaddingSamples =
  512 - ((numSamples(+384)) & 511)` (if non-zero) for the next request.
* `Service` 0x82c161b0 feeds 2048-byte pieces (the last may be short); a seek sets
  `InputBufferReadOffset = seekFrameOffsetBits + 32`. Context: 4 subframes/decode, 1-packet input
  buffers, 6144-byte output ring.
* `CopySamplesOut` 0x82c0e9c0: hardware s16 -> float `x * (1/32768)` (`vcsxwfp128 .., 15`).

## Menu sounds (path)

* `AudioSystem::PlayMenuSoundImp(u64 key, bool loop)` 0x8250c738: vault class `fe` at `key`:
  layout +4 float vol, +8 int menu splice (bank 26 `sk8_menu.bnk`, played if > 0), +0xC int FE
  splice (bank 25 `Sk82_FE.bnk`, played if != 0); attribute 0x845A10052522A2FB = Hall of Meat
  splice (bank 7, only if `mNextHOMTime <= 0` and the HOM bank is loaded; then
  `mNextHOMTime = HOMExpireTime * 0.001`); 0xA4080FB65880E3C2 = logo type (killed/owned it
  Csis classes and speech); 0xBF45D439FAC71A2E = bool flange. Then `CheckMenuKey`.
* `PlayFEHudSplice(key, bank, splice, vol, loop, flange)` 0x824e7b28: if in-game data loaded, take
  the first of 10 `tFEHudSplice` slots with `!play && !pSplice`, fill it, `play = 1`.
* `UpdateFEHudSplices(dt)` 0x824fea50: a slot with `play`: new `Splicer::Splice(bank, splice,
  flange ? mpMenuFlangePlugInPtrs[0] : AudioCore::GetMasterSubMix())`, `Play({vol = slot.vol *
  (bank 26 or 7 ? mMenuVol : mFESFXVol) / 32767, timestretch 1, pitch 1, az 0, delta 0,
  spread 1})`. Playing: `Update` with `delta = dt`. Finished: delete; loop -> `PlayFEHudSplice`
  again, else `NotifyMenuSoundFinished(key)`. Then `UpdateMenuFlangeVoice`.

## Splicer (`splicer` lib, 0x82a7aa90-0x82a7d438)

* Bank `SPLC` v3: `SpliceBankHeader` (60 B: magic, version, SizeOfMetaData, NumberOfSplices,
  NumberOfSpliceClusters, NumberOfSpliceLoops, NumberOfSamples, BankIdentifies[32]); metadata at
  +60 = `SPLICE_Data[n]` (36 B), `SPLICE_SpliceClusters[n]` (72 B), loops (24 B each), then per
  splice its sample clusters (12 B) each followed by `SPLICE_SampleRef[n]` (72 B). TOC at
  `60 + SizeOfMetaData` (4-aligned): `{sampleDataOffset, seekTableOffset, hash}` per sample; sample
  data follows (4-aligned). Parser: scratch `splc.py`; `sk8_menu.bnk` = 131 splices, 131 splice
  clusters, 110 samples (all EA-XMA mono 48 kHz RAM).
* Voice per splice sample (`CreateVoiceInstance` 0x82a7ba80), processing stage 0:
  `SndPlayer1(1 ctor param = 1.0, out = stereo ? 2 : 1)`; LFE-only refs: `Gain`, `Route`; else
  `Resample`, `TimeStretch` only if allowed or `TimeStretch != 1`, `Gain`, user plug-ins (VoiceType
  != 255), `Pan2D1(6)` for mono, `Send(mono ? 6 : 2)`. Stereo = `Az == -127`.
* `StartVoice` 0x82a7c6e0 (System lock held): SndPlayer1 event 5 old-param buffer =
  `{f64 startTime 0, f64 streamFileOffset 0, f64 seekTime = Pitch/TimeStretch*SeekPoint (if != 0),
  char* streamPath 0, void* ramSample, void* seekTable (if seeking), u32 streamPool 0,
  f32 expelMode 1.0}`; Resample attr0 = localPitch * update.pitch; Send event 0 (connect to submix)
  + attr0 = 1.0; Route event 0 `{submix, 0.0, 5.0, 1.0}`; Gain attr0 = localRandomVolume * Volume *
  update.vol * envelope; Pan2D1 attr0 = update.spread * Az + update.az; TimeStretch attr0 =
  clamp(update.ts * TimeStretch, 0.5, 2).

## RWAC core

* Voice list ordered by `mProcessingStage` ascending, a new voice inserted before the first voice
  with stage >= its own (`Voice::CreateInstanceHandler` 0x82be38e8). Stages: splices 0, music and
  speech masters 200, mastering voice 255.
* Commands (`System` command buffer, executed by `ExecuteCommands` 0x82bea680 on the mixer thread):
  `Voice::CreateInstance` adds the voice, `Release`, `SetPriority`, `PlugIn::SetAttribute`
  (writes float attribute slot), plug-in specific handlers. `PlugIn::Event` runs `EventEvent` at
  once on the caller's thread; plug-ins queue commands from there.
* Frame: lock, pre callbacks + `TIMERSTAGE_BEFORECOMMANDS` timers (AEMS), unlock;
  `UpdateExpellingVoices`; lock, run commands, unlock; lock, `TIMERSTAGE_BEFOREMIXER` timers
  (SndPlayer1 streaming), unlock; `Mixer::Execute` (unlocked). 256 samples, 48 kHz, 6 channels.
* Channel order inside RWAC: FL, C, FR, SL, SR, LFE (fold-down maps below).
* `FoldDownMapper` tables: `sFoldDownGroups[src-1][dst-1] = (start, end)` into `sFoldDownMaps`
  bytes `gain << 6 | src << 3 | dst`, `sFoldDownGains = {1.0, 0.707, 0.5}`. Mono->6 = C;
  stereo->6 = FL, FR; quad->6 = FL, FR, SL, SR; 6->2: L = FL + .707 C + SL, R = FR + .707 C + SR.
* `MixWithGainRamp` 0x82be15d8: per-sample linear gain for the first 64 samples, then hold
  `gain + 64 * inc`. `ReChannelGainMixRamp` 0x82be27a0 uses `inc = (to - from) / 65` (Send);
  `Gain::Process` 0x82bec150 uses `/64` via `CopyWithGainRamp`.
* `DeClick` 0x82bdc978: adds a DC offset decaying over 16 samples (factors 16/17 ... 1/17).
* Plug-in descriptors (`sPlugInDescRunTime`): type (0 generator, 1 rechannel, 2 resample,
  3 time-stretch, 4 processor), constructor params, attributes (defaults/ranges), events. Dumped in
  the session; e.g. Resample attr0 pitch def 1 [0, 32]; Gain def 1; Pan2D1 ctor (45, 135, 2) and
  9 attributes; LowPassIir2 def 96000; HighPassIir2 def 0; SndPlayer1 event 5 = 9 params.

## EaXmaDec mechanics (decoder.rs)

* Per stereo pair: an XMA context with two 2048-byte input buffers and a 0x18-block (6144-byte)
  output ring of 16-bit samples (`Reset` 0x82c0e650: read offset 32 bits, 4 subframes per
  decode, `DecoderTable = w & 3`). Contexts come from a 256-entry free list
  (`CreateInstanceEvent` 0x82c13f88, all or nothing).
* `Service` 0x82c161b0 is the only code that copies packets: `min(bytesRemaining, 0x800)` into
  the free buffer named by `nextBufferToService` (FIFO), `XMASetInputBufferReadOffset(seek + 32)`
  only when `seekFrameOffsetBits > 0` (then cleared). When `totalBytesRemainingThisFeedDesc`
  reaches 0 it prepares the next fed slot (`Reset` on the first one, `StartPair` per pair).
  Called from `FeedEvent` 0x82c16478 (= `b Service`), the end of each `DecodeEvent` loop, a
  stall and `EnterRecoveryMode`.
* `DecodeEvent` 0x82c169f8 per pair: available = write offset * 256 - read byte offset (wrapped;
  0 with the output buffer invalid means full), skip `mNumSamplesToSkip` from what is there,
  copy at index 0 of the sample buffer if enough, else `Service` and a stall: the stall timer
  starts at `GetTickCount() - 1` against `sXmaStallTimeoutThresholdMs` = 4, so any stall enters
  `EnterRecoveryMode` 0x82c16480 at once (silence for the rest of the request, contexts reset,
  decoding resumes at `mDecodeSlot + 1`). The read offset handed back is rounded down to whole
  frames (`(byte / (ch * 1024)) * ch * 4` blocks).
* `ParseHeaderData` 0x82be3220: a version-1 seek table carries the sample header at +12, else
  NULL (then `UnpackHeader(NULL)`: 0 channels, `numSamples = 0x7FFFFFFF`, codec 0xFF, type 1).
* `sDecoderGuidLut`: 2 'P6B0', 3 'EXm0', 4 'Xas1', 5 'EL31', 6 'L32P', 7 'L32S'. Every bank in
  the menu and skateboard archives is codec 3.
* The port models the hardware as infinitely fast: it decodes whenever looked at, limited by the
  two input buffers and the ring. Validated: a menu sample played through SndPlayer1, Resample,
  Gain, Send, master SubMix and Dac equals the direct decode bit for bit (max |diff| 0), starting
  257 samples in (one frame of `RwacTimerClient`'s 1/187.5 s start delay plus one sample of
  resample history), and the voice expels itself at the end.

## SndPlayer1 (snd_player.rs)

* `EventEvent` 0x82c01420: events 0/5 bump `*mpRequestHandle` (wraps above 4194304 to 1) and
  `*mpNumUnexecutedPlayHandlers`, then queue `PlayHandler` (event 5 params: f64 start, f64 file
  offset, f64 seek time, path, RAM sample, seek table, stream pool, f32 expel mode, out handle);
  1 = stop; 2 = "request finished" query; 4 = modify start time.
* `PlayHandler` 0x82c00b28, `StartRequest` 0x82bfb718, `SubmitChunk` 0x82bf2218 (format 0 size =
  first word, else its low 24 bits; feed type = !resetDecoder; decoder skip and seek data only
  when seeking), `RwacTimerClient` 0x82bff0e0 (feeds while a feed slot is free, EA-XMA requests
  start `systemTime + 0x3f75d867c3ece2a5`), `Process` 0x82bfba90 (`WaitForStartTime`, skip loop in
  256-sample `Decode` calls, the next feed slot's remaining samples asked of `mpLoadedDecoder`, the
  current request's decoder after any advance), `Declick` 0x82bf2798 (16 samples, step = value /
  remaining count), `RemoveRequest` 0x82bfb978 (expel mode 1 -> `Voice::ExpelAfterDecay`).
* Streams (play types 1/2 with a path) need `StreamPool` and are not ported; such a request ends as
  when `AcquireStream` fails.

## RWAC system order (core.rs)

* `ExecuteCommands` 0x82bea680: lock; `mTimerList` callbacks with `mSystemTimerPeriod`;
  `TIMERSTAGE_BEFORECOMMANDS` timers; unlock; `UpdateExpellingVoices` 0x82be92c8 (an expelled
  voice shifts the list, the next one waits a frame); lock; commands; `mCommandTimeStamp++`;
  unlock; lock; `TIMERSTAGE_BEFOREMIXER` timers; `Defragment`; unlock.
* Timers run newest first: `Collection::AddItem` 0x82be7c28 pushes to the head.
* `Dac::Mix` 0x82beb778: `Mixer::Execute` returns the last voice's status; `XenonThread`
  0x82beb8d8 sends silence when it is not available. `mSystemTime += mSystemTimerPeriod`, which
  `Dac::CreateInstance` sets to `256.0f / 48000.0f`.

## Splicer runtime (splicer.rs)

* The game uses the base `SpliceManager` (constructed in `AudioCore::InitializeAudioCore`
  0x82a7f094); vtable slots 1-3 (`AddUserPluginsConfigs` = `li r3,0`, `InitializeUserPlugins`,
  `UpdateUserPlugins` = `blr`) do nothing, so no user plug-ins exist although most refs have
  VoiceType 0.
* `Construct` 0x82a7c2f8: a ref that fails its probability roll (`rand() * 2^-15 >
  ProbabilityOfPlaying`) leaves the cluster pointer unadvanced, so the next sample cluster picks
  from the same refs.
* `GenerateRandomIndex` 0x82a7ade8: type 0 `(int)(rand * 2^-15 * n)`; type 2 keeps a 16-bit mask
  of unused entries in the high half of the history (read with `lhz`, big-endian) and the current
  half in bit 0, start `(int)(rand * 2^-15 * (size + 1))`, refilled with
  `(int)pow(2, size') - 1` when empty; other types `(history + 1) % n`. Banks start every
  history at 0x10000.
* `SpliceSample::Play` 0x82a7c520: `x = fmsubs(r, 2, 1)`; `x <= 0`: `fnmsubs(-x, 1 - rv, 1)`;
  `rv == 0`: 4.0; else `fmadds(1 / (1 - (1 - rv)) - 1, x, 1)`. Pitch and trigger offsets use
  `fmadds`. `Update` 0x82a7ca00: time advances by the previous update's pitch times delta
  (`fmadds`), pan `fmadds(Az, spread, az)`, stop once `(Pitch / localPitch) * Duration + 0.16 <
  time` or the voice is expelled (Resample and Gain 0, SndPlayer1 stop event).
* `Curve::Get` 0x82a7b058 uses `SPLICER_HALF_PI` = 0x3fc90fd0 (1.5707951) and `XMVectorCos`
  0x824b0130: angle minus whole turns (`vrfin`, `vnmsubfp`), then the even polynomial to x^22 with
  `g_XMCosCoefficients0-2` and fused `vmaddfp`. IDA prints VMX 4-operand ops as
  (vD, vA, vB, vC) with vD = vA * vC + vB.
* `ProcessQueued` 0x82a7d1d8 / `Partition` 0x82a7aa90: signed priority compare, descending
  quicksort with the left element as pivot; beyond 60 voices the lowest-priority actives are
  culled for higher queued ones.
* Fused operations are emulated exactly: `math::fmadd` rounds the exact f64 product sum to odd
  before narrowing, which equals a single rounding (tested against `f32::mul_add` on 2 million
  operand triples).

## FE sound request path

* Apt `Audio.PlaySound(name, loop)` = `AudioCmdTranslator::sMethod_PlaySound` 0x8261dfb8 ->
  `Attrib::StringToLowerCaseKey(name)` -> `GlobalFEPlaySound` 0x826150b0: the `fe` collection
  must exist and have a menu splice, FE splice, HOM splice (0x2522A2FB) or logo type (0x5880E3C2)
  -> `AudioSystem::PlayMenuSound` 0x82504dd0 posts `cMsgPlayMenuSound` to the audio event
  broadcaster -> `PlayMenuSoundMsg` 0x8250d428 -> `PlayMenuSoundImp` 0x8250c738.

## Nicotine dynamic mixer (nicotine/)

* `AudioSystem::InitializeMixer` 0x824ed830 (called from `EnterWorld` 0x82504450 only) creates the
  thirteen state managers, runs their `EnterGamePlay`, then `CreateMixer` (13 states) and
  `InitMap` with `data\audio\MixMapSK8.mxb`. `GetStateCount` 0x824edb90 returns the manager's
  `m_CurNumStates` (+0x14): Main 1, Player 2, Environment 1, Collision 10, TrafficCar 4,
  Pedestrian 15, Emitter 5, Crowd 5, DynamicObject 5, Speaker 4 (first entry only,
  `mbCreateStates`), Whoosh 3, ObjectInstance 5, NISCharacter 5 (Emitter, Crowd and NISCharacter
  share ObjectInstance's `EnterGamePlay`).
* Build order (`NFSMixMaster::InitMixMap` 0x829caf30): `PreProcessMixMap` 0x829cf288 (curve totals
  per curve table, deduplicated by raw input id within a state; scale counts), per state and copy
  `CreateMainMapState` 0x829cc128 (`CreateMixCtls`, `Create3DMixCtls`, `CreateEvtMixCtls`), per state
  and copy `CreateSubMixChannels` and `CreateMasterMixChannels`, `SetupStateProcArrays`
  (`InitializeSubChannels`, `InitializeMasterChannels`), `ConnectMixMap` 0x829cc518. Instances of a
  state are consecutive `NFSMixMapState` objects; `m_pFirstInstance[n]` past a state's copies reads
  the next state's first instance.
* Records are patched in place: `AddScaleIDs` writes the expanded scale count into the top byte of
  `nUScaleCntSwing`; `CreateEvtMixCtls` sets bit 0 of zero 12-bit stage lengths (type 0 and 2: p0,
  p2; type 1: p0, p1, p2; type 3 none). Event type is `(nEVTCTLID >> 24) & 0xF` everywhere.
* `GetObjectPtr` 0x829cc1f0 by the top three id bits: 0 mix control (curve Q15, or `CmpdBOut` for
  dB), 1 channel (bit 28: sub, else master output), 2/3 DMixIO input word `id & 0xF` of the block
  keyed `id & 0xE0FFFFF0` (created on first use, `ConnectDMixIO` callback), 4 3D control (the proc
  itself for master 3D lists), 5 event (`output` or `qoutput`), 6/7 `DUMMYINPUT` (0, never written).
  Game objects find their block by `0x40000000 | objectID` (objects) or `0x60000000 | objectID`
  (controls), `objectID = state << 16 | instance << 11 | sfx << 4`.
* `ProcessMixMap` 0x829d0df8: curves (`GetCurveOutput` of the input, `Q15todB`), mix controls
  (`q = 0x7FFF - (((0x7FFF - curve) * nRatio) >> 15)`, `CmpdBOut = (scale * (nOffset +
  Q15todB(q))) >> 15`, scale product starting at 0x7FFF), 3D controls, events, sub channels (sum,
  clamp to the record's swing), master channels (`(i16)(MixData >> 16)` plus inputs while output
  word 15 bit 0 is set, else -10000), `MixMasterChannels` (one packed 16-bit output per preset entry).
* Events: idle when stage Off and trigger 0 (output -10000 with id bit 9, else 0). Otherwise elapsed
  += ms, AR/ASR/ADSR (0x829cd7c8 / 0x829cd4a8 / 0x829cd058) and the output from `qoutput`; bit 9
  outputs dB directly, a positive swing adds `nOffset`. Stage lengths are frames times 16.66667
  (attack p0, decay p1 high, sustain p1 low, release p2, 12 bits each), "instant" below 16.666; AR
  releases with `q - fctiwz(curve * q)`, ASR/ADSR with `1 - curve`; ADSR's sustain level is
  `p2 >> 16`. Bit 8 holds while triggered, bit 10 retriggers from release.
* `MixMasterChannels`: with a 3D input a Doppler preset clamps above 2400 and maps below -4800 to 0
  (without one it clamps to -4800). `GetAzimShapeOutput` uses `etable1` for both distances.
* `IDynamicMixer::miCamState` stays -1 (nothing writes it): the first update switches every 3D
  control to its camera-0 parameters. `SnapshotMixer` is never initialised (no snapshot header is
  loaded), so it is inert and not ported.
* Tables (`g_dBToQ15XForm`, `g_Q15todBXForm`, `g_nArrayCosTable`, pitch tables) are copied word for
  word into `nicotine/tables.rs`. With the SFX slider at 1.0 and no music, `mFESFXVol` and `mMenuVol`
  are 7209 (checked with a temporary probe; an independent evaluation agrees).

## AudioSystem menu sounds (system.rs)

* `AudioSystem::Update` 0x82507b88 is called once per rendered frame from `GameModule::Render`
  0x827320f0 with the frame dT; RenderMan's 30-frame flag forces both groups, as does
  `kCompensateForSlowFrameRate` (1) when dT > 0.02. In world: group A dispatches the audio event
  broadcaster, updates the splice manager and runs the state managers' `UpdateParams`; group B runs
  `ProcessMixMap` and `ProcessUpdate`; group A then updates the HUD splices, `mNextHOMTime` and the
  countdown flag. The port steps it once per 1/60 s simulation tick.
* `LoadInGameData` 0x824fdfe8 loads `Sk82_FE.bnk` (splice type 25) and `sk8_menu.bnk` (26) from
  `splicerfe.big`, creates the menu flange voice and sets `mInGameDataLoaded`.
* `FrontEndManager::RegisterAudioEnums` 0x8261b820 turns every leaf collection of class `fe`
  (0x5831CB95F3E90598, `skater.vlt`) into the Apt global `FE_SOUND_<NAME>` = name (layout word 0).
  `Audio.PlaySound(name, loop)` (`AudioCmdTranslator::sMethod_PlaySound` 0x8261dfb8) keys it with
  `StringToLowerCaseKey`; `GlobalFEPlaySound` 0x826150b0 posts it when the collection names a sound.
  `AudioCmdTranslator` has no other sound methods: `Audio.PlaySignupOut` and the like are no-ops.
* `fe` layout: +4 volume, +8 menu splice (played when > 0), +0xC FE splice (when non-zero);
  attributes 0x845A1005_2522A2FB HOM splice, 0xA4080FB6_5880E3C2 logo type (Csis, not ported),
  0xBF45D439_FAC71A2E flange, 0x9DAD777A_FEA4E73C `HOMExpireTime`.
* `PlayFEHudSplice` 0x824e7b28 fills the first of ten `tFEHudSplice` slots that is neither waiting
  nor playing; `UpdateFEHudSplices` creates the splice on the master (or flange) SubMix with volume
  `slot volume * mFESFXVol / 32767` (`mMenuVol` for banks 26 and 7), updates it with the frame time,
  and on finish replays loops (without the flange flag) or notifies. `StopMenuSoundImp` compares the
  Hall of Meat slot against the menu splice index, as natively.
* Only `SFXObj_Master` (inputs 0-13), `SFXObj_VU` (0, 1), `SFXObj_Challenge`, `SFXObj_Menu`
  (1: Create-a-Character HUD) and `SFXObj_NIS` (2: photo outro) feed `SFXObj_Master`'s outputs 0, 1
  and 10. The SFX slider comes from `ProfileBackEnd::tSettings` (+0x78, default 1.0).
* Output: the mastering Dac's WAVE-order frame folds to stereo by `FoldDownMapper`'s 6-to-2 map
  (`sFoldDownMaps` 28..33): L = C * 0.707 + FL + SL, R = C * 0.707 + FR + SR, in that order.

## Skater contacts and collisions (player/, collision.rs)

* In world, group A runs `MicrophoneMan::UpdateCameras` 0x824f2728 and then the thirteen state
  managers' `UpdateParams` (MAIN 0, PLAYER 1, COLLISION 3, ...); group B runs `ProcessMixMap` and
  every manager's `ProcessUpdate`. With the time controller's frame at 0x1FFFFFF (no world) only
  MAIN updates, and the HUD splices update in group B with group B's time.
* `CSTATEMGR_Player::UpdateParams` 0x82556c60: the states update first; a change of the local
  skater's index (the loop keeps the last local) detaches every state; then each skater that is not
  ghosted and is local or within 30 m of the camera microphone (`IsInRange` 0x825561f8) keeps its
  state or takes a free one, and a local skater steals the head state when none is free.
  `CSTATE_Player::Attach` copies `SkaterAudioStateData` and sets `mWheelType` hard;
  `GetNPCWheelType` 0x8254fee0 is the opposite of the first local state's wheel type (soft without
  one), read whenever `SFXCTL_PlayerPhysics::GetWheelType` serves a non-local state.
* `CSTATE_Base::UpdateParams` 0x82a7e0e0 sets `t_DeltaTime`/`t_CurTime` always but runs the
  controllers (sorted by id) and objects only while attached; `ProcessUpdate` runs every object,
  and each object tests `bIsAttached` itself. `Detach` calls each object's `Detach`, zeroes its
  output word 15 and its 16 input words, then the controllers' `Detach` and inputs.
* `SFXObj_Contacts` (488 bytes) is ported whole except the speech calls (`CheckForImpactSpeech`,
  `PlayGrunt`, the speech of `CheckForCollisionSpeech`) and `CheckForScore`'s crowd; their state
  is kept. The foot drag's `Class_foot_drag` data is kept current; AEMS plays it, so it is silent
  until AEMS is ported. Details that matter:
  * `mFeet[eFeet_Both].mTimeInAir` is the storage of `mNumFeetScraping`: a both-feet catch writes
    its time there as float bits and clears it to 0.0 afterwards.
  * `Destroy` (also `Detach`) leaves the air-dismount splice playing.
  * `GetSpliceIndexForOllie` turns a concrete index 0 into 5; `GetSpliceIndexForLand` uses the
    concrete-hard records for two wheels or fewer at level 3.
  * `PlayWheelHit` reduces level 3 to 2 and adds the level-3 sweetener (`kEnableSweetener`) unless
    `check` and `mL3Triggered`; `CheckForWheelHits` sets inputs 6 and 3 (level 2) or 6 and 4
    (level 3) for four-wheel landings.
  * `LandGround` scales the wheel material's volume (computed against the land material) by
    0x31DEEF8F_A219950F into the land material's slot, as natively.
  * `AddToBailAverage` is one `fmadds`; its maxima compare with `ble` (NaN stores).
  * `LandRail` falls back to `eAudioMaterialType_Metal_Solid_Round_1` (10).
  * `CanPlayCollisionSound` reads `tHOMPriority` from `aud_general` "hom" (0x34049ABC_E23C10AC).
* Vault: the `aud_*` classes are in `skater.vlt`; Gen accessors look attributes up by
  `string_hash64(name)`. `aud_speech` is 0xB29C3B2C_13D96482, `aud_splicer` "footcatch"
  0xF1647BFB_782BE97F and "handgrab" 0xE0C3B44A_B44F7B90, `aud_general` "kickup"
  0x109E63A9_F198B562.
* Banks: `LoadInGameData` loads the splicer banks of `LoadSkateboardData` 0x824ee4b8 from
  `splicerskateboard.big` and `LoadCollisionData` 0x824ee348's `Skate_Collisions.bnk` (5) and
  `Skate_Metal.bnk` (6) from `splicercollision.big`. Names match case-insensitively
  (`SK8_Ollies_Wood.bnk` is `Sk8_Ollies_Wood.bnk` in the archive).
* COLLISION (ten states of `SFXObj_Collision` + `SFXCTL_3DColPos`): `PlayCollisionSound`
  0x824ef148 takes `GetFreeState` 0x8254fad0's first free state, else detaches the one with the
  smallest `Simulation` +0x10 tick (`Attach` stores it with the info). The first `ProcessUpdate`
  runs `StartCollision`: per side, `GetSplicerInfo` gives bank, index, pitch and volume, and
  `CreateCollisionVoice` builds a mono "Collision SubMix" voice (stage 4) with a reverb send at the
  material's wet level and a send to the master mix. Later updates set volume
  `fctiwz(fctiwz(info.vol / 32767 * matVol) * data.vol / 32767)` and pitch
  `fctiwz(data.pitch / 4096 * mixPitch)`; once both splices end the state detaches and `Destroy`
  frees the voices. `SFXObj_Collision`'s `Detach` slot tail-calls `Destroy`.
* `GetCollisionLevel` returns 3 when force < L3 (`bge`: a NaN force continues);
  `CalculateCollisionVolume` takes high when `!(cur < high)`, returns the maximum when
  `!(high > low)` and interpolates with `fmadds`.
* `GlobalEffects` (the reverb and environment sends) is not ported: the splicer reverb voices and
  the collision voices keep only their dry send to the master mix.
* Host: skate2-core's `SelectedLiveSession::audio_phys_out` copies the `PhysOut` fields
  `FillSkaterData` reads, and `audio_camera` gives `CameraPresentation::GetMatrix(1)`'s Z axis and
  position with the completed frame count. The app keeps the last published skater on frames that
  publish none, as `AudioPresentation` keeps its skater while the simulation is paused.

## Live menu sounds (skate2-core ui/, app)

* The original plays menu sounds from two places, and the core menu controllers record both
  (`take_sounds`; `SelectedLiveSession::take_menu_sounds` drains them once per tick and the app
  plays them before `AudioSystem::Update`):
  * The pickers' ActionScript. `controls/menu_picker.apt` `NextValue`/`PrevValue` play
    `FE_SOUND_CORE_NAV_DOWN`/`_UP` only when the value changed, and `OnMenuSelect` plays
    `FE_SOUND_CORE_A_BUTTON` before `Game.OnMenuSelect`. The main menu (`mainmenu.apt`), the
    pause menu (`starthubmenu` -> `challenge_info` -> `mMenuPicker`) and `sk8popup` all use
    `menu_picker` with its defaults (wrap on, GUI on). `hud_menu_picker` plays its nav sound
    before computing the move, so it sounds even when nothing moves; no ported menu uses it.
    `options.apt` plays `FE_SOUND_CORE_NAV_UP`/`_DOWN` on every up/down press, before moving.
  * `Sk8::FE::GlobalFEPlaySound(key, false)` in the front-end states:
    `StateManagerFrontEnd::CheckForOptions` 0x82656968 (Start opens the pause menu) and
    `FrontEndState_StartHub::ReceiveInput` 0x8265e778 (Start or B closes it) play
    `pause_menu_press`; `FrontEndState_Popup::OnEnter` 0x82658474 plays `core_popup`;
    `FrontEndState_Options::ReceiveInput` 0x82686978 plays `core_b_button` on B and
    `core_nav_left`/`core_nav_right` by key before a toggle (the volume sliders play
    `core_*_volume_left/right`); `OnMenuSelect` 0x826871a0 plays `core_a_button` entering a
    category and nothing for a toggle.
* The keys are `fe` collection keys (`Attrib::StringToKey` of the name; the class has 169
  collections). `FE_SOUND_PAUSE_MENU` (starthubmenu's outro frame) has volume 0 in the vault;
  the audible pause sound is `pause_menu_press`.
* The movies' timeline `Audio.PlaySound` calls (letterbox in/out, page-title flicker, core
  fade, HUD) are moved out at the end of every UI frame (`MarkerUi::collect_sounds`): the
  movies clear their lists on their next update, and frames can outnumber ticks.
* Not wired yet: map navigation (`map_nav`), Create-a-Skater (`cas_*`), the phone list, and the
  unlock overlay's and sign-up poster's intro audio.

## XMA hardware timing (rwac/decoder.rs)

* The 360's XMA hardware writes every sample 192 samples earlier than the WMA Pro decoder
  (`skate2-xma`, bit-exact with FFmpeg 4.2.3). `EaXmaDec` assumes the hardware's timing: it
  skips 384 samples at a stream's start (`StartPair` 0x82c14200 and `DecodeEvent` 0x82c169f8
  add 0x180 for feed type 0) and pads each chunk to whole frames.
* Evidence (all 110 samples of `sk8_menu.bnk`): in the decoder's output, audio starts at
  sample ~576 and, for every sample whose sound fades out before its end, stops exactly at
  576 + numSamples. With the 384 skip alone, each sample started with 192 samples of silence
  and lost its last 192 samples; sounds that play their source to the end ended in a click
  (the menu A/B/popup/letterbox sounds; the nav sounds fade before the end).
* The context model therefore drops the decoder's first 192 samples after
  `XMAInitializeContext`, and when a stream's data is exhausted (no input queued, no bytes or
  requests left) and the ring holds fewer samples than the request needs, it drains the
  decoder's final overlap once: those are the samples the hardware had written with its last
  frame. A temporary probe checked the RWAC output against that timing.

## Letterbox and page-title sounds (apps/skate2-pc marker_ui/letterbox.rs)

* The front end has one `_global.LetterBox` (letterbox APT, constructor 0xbd67).
  `EnableLetterBox(true)` plays `niceintro` (`FE_SOUND_LETTERBOX_IN`) only while the movie is
  hidden; `EnableLetterBoxInfo` plays `infointro` (`FE_SOUND_PAGETITLE_FLICKER`) or `infooutro`
  only when `mLetterBoxInfo` changes; `SetTitle` sets the text alone. mcLetterbox frame 58
  calls `EnableLetterBoxInfo(true)`; frame 86 calls `OutroDone`, which hides the movie. Native
  code never calls `EnableLetterBoxInfo` (no `AptIntegration::CallFunction` with
  `eASCall_EnableLetterBoxInfo`).
* Main menu: its constructor enables the letterbox; its frame 13 and the letterbox's frame 58
  both turn the title on, so booting plays one `LETTERBOX_IN` and one flicker.
* Options from the main menu: `OnMenuSelectGame` 0x82681604 pushes state 73 with
  `mbKeepPrevious` set, so `ExecutePushState` only hides the main menu screen
  (`ShowScreen(false)` sets `_visible`). Options' constructor (`ShowLetterbox`) and frame 4
  find the bars and the title already shown and replay nothing: A plays the picker's A sound
  and the dimmer's `core_fade` only. Subpages (`OnMenuSelect` 0x826871a0, `LoadMainMenu`
  0x826714d0: `SetScreenState(1)`) only retitle. B pops back: `ExecutePopState` 0x82d672f8
  shows the main menu, whose `OnGainFocus` 0x8264a018 sends `SetScreenState(4)` (retitle).
  `OnLeave` 0x8266af70 sends `SetScreenState(2)` (title out, letterbox out with
  `LETTERBOX_OUT`) only outside the front end.
* The port had replayed `infointro` on every Options page change and again on Options'
  frame 4, so entering Options played the flicker twice, 4 frames apart, and every subpage
  change played it after the A/B sound. `LetterBox` now models the class: the main menu owns
  it and Options drives that instance in the front end (its own one in game). The
  `settings_movie` test checks the sound sequences on both paths.

## Skateboard rolling (player/skateboard.rs, grain_player.rs, utils.rs)

* `SFXObj_SkateBoard` is PLAYER object 0. Its constructor 0x825427f0 loads each surface's
  `GrainFile` (`aud_skateboard` layout word 16, e.g. `asphalt_rough_hard.grain`) from
  `grains.big` into two `pBoardData` slots; `LoadDataForSurface` changes
  `mSkateboardAttrib[mFrontLeft]` each time, so slot 0 starts on the last surface. Its
  `SetupLoadData` (vtable 6) tail-calls `InitSFX`, so every attach creates four `AudBoard`s;
  `Detach` (vtable 11) tail-calls `Destroy`.
* `UpdateSurfaceType` 0x8254ca10 runs per wheel slot (both for the local skater). `GetSurfaceType`
  gives none for a lifted wheel in a manual or on a rail, else the vault override, else
  `SurfaceMapping::GetRollingMaterial` of wheel 0 or 3 (smooth asphalt past the table). Surfaces
  0-6 and 9 (plywood, asphalts, concretes, wood ramp, aggregate, metal) roll on grains; 7, 8 and
  10-13 through AEMS. A change tears the slot's sound down, or for the local skater hands it to an
  idle other slot and swaps `mFrontLeft`; a grain surface gets two voice chains and two grain
  players with the collection's two `GrainParams`.
* `CreateSkateBoardVoice` 0x825239e8 (from the asm; `PlugInConfig1` is {params, handle,
  channels}): [SubMix, HighPassIir2, LowPassIir2, FrequencyShiftSsb(0.0), Send, Gain, Send] at
  stage 2 into [SubMix, Send (flange, level 0), Send (reverb), Pan2D1 (6 ch), Send (master)] at
  stage 5; the local skater's [SubMix, DistortionClip, Gain, HighShelfIir2, Send] at stage 3 takes
  the first Send at `mCurrentDistortion`. `GlobalEffects` is not ported: the reverb and flange
  sends stay unconnected.
* `ProcessUpdate` 0x8254d0b0: the rolling player's intensity is the cubic Bezier of
  `Speed_Ramp`'s second column over `clamp(speed * push scale / MaxSpeed * 3.6)`; its volume
  `(1 - max(turn, brake)) * output 1` (times `Manual_Volume` in a manual, times the pattern duck).
  The carving player gets the intensity minus 0.1 and `turn * output 2` plus the bowl term (at
  most 1). Both pitch at output 3 / 4096. The grain branch is chosen by `mCurrentRollingType`.
* `UpdateVoices` 0x825240b0: outputs 12 and 11 are the high and low pass, output 0 the pan
  (times 360/65535), output 13 the reverb send, 21 and 22 the flange sends; the frequency shift
  is the manual shift, push frequency and bowl term (rolling) or `Carve_FrequencyShift`, push
  frequency and bowl term (carving).
* Mixer inputs: 0 pulses on a surface change, 1 is `IsWheelSlide` (whose `mAddToAngle` only moves
  while the skid instance exists), 2 and 3 the
  bowl and hill compression, 4 pushing, 5 the azimuth rate, 6 metal under wheel 0.
* `UpdateRolling` 0x82542fc8: a push start rebuilds the push scale (start, a speed-dependent
  level, 1) and push frequency (0, a level, 0) paths; all their stages are linear (the
  decompiler shows the stage lengths as curve types: the 360's ABI gives a float argument a GPR
  slot). Foot braking compares the normalized acceleration and velocity (`vrsqrtefp` plus two
  Newton steps); a NaN dot product brakes, as natively.
* The grain players run as `TIMERSTAGE_BEFORECOMMANDS` timers: `ExecuteCommands` runs them
  first, newest first, with the timer period, sharing `rw::math::Random` with the modulation and
  ducker draws.
* Kept as natively: `UpdateVolumeModulation` checks one last value per modulation for both
  slots, so only slot 0's voices receive the modulation gains.
* Not ported: the non-local whoosh (`CheckForWhoosh`).

## AEMS and Csis (csis.rs, aems/)

* `.csi` files are MOIR: a 40-byte `SystemDesc`, then function descs (12 bytes: clients,
  `pStringId`, `crc` s16, `key` s16), class descs (12) and globals (16: clients, current value,
  `pStringId`, crc and key). Keys come from `gUniqueKeyId` (s16, wrapping to 1); systems are
  head-inserted. `SetHandle` searches the systems with the handle's system CRC first, then all,
  matching the desc CRC and the name; a handle is `{desc + 1, crc << 16 | key}`. Errors: -1
  alloc, -3 stale (the handle becomes `{0, -3}`), -4 no clients, -5 not found, -6 unbound.
* `CreateInstanceFast`: ClassData with refcount 1, the constructor clients, `SendParameters`.
  `ReleaseFast`: the destructor clients set `triggered`, then refcount - 1; subscriptions add and
  drop references, the class frees at 0. `GlobalVariable::SetFast` notifies only a change;
  `SubscribeFast` calls the new client at once.
* ABKC banks (no function fixups in any shipped bank): interface refs are
  `{handleOffset, IDOffset, type}` (0 global, 1 class, else function), IDs `{s16 system crc, s16
  interface crc, name}`. Module records are `0x3C + 4 * (players + control classes)` bytes. An
  instance copies the module template (0x18-byte header); its state subscriptions go destructor
  (20 bytes), globals (28 each), class data (20 + 4n), functions (28 + 4n). `DESTROY` is every
  shipped module's last op (all 263).
* The timer (n = 6, 32 ms) interprets instances newest first. `iSNDrandom` starts from a zero seed
  and carries when the new value is below the old. `iSNDsin` is the 1024-step table at 0x83016248.
* Opcodes (raw PPC of `InterpretModule` 0x82c19c20): ENVELOPE ends its last segment at 0; TABLE's
  8-bit entries are signed and its interpolation index `round(x * res - 0.5)` can be -1;
  OSCILLATOR scales the sine by 2^-16; RAMP steps `(target - current) / duration * period / 4096`
  and clamps by the step's sign; DELAYLINE writes 0 back over a negative delay; DEMUX writes 0 at
  `4 * (previous + 2)`; ControlClass is unused by the shipped banks.
* `SkateSamplePlayer` voices: SndPlayer1, Rechannel, Resample, high pass, low pass, gain, Pan2D1
  (six channels, 45/135/0), Send to the Master submix. Inputs: 0 pitch (/4096), 2 volume, 3
  azimuth (x 360/65536), 5 wet (default 0), 6 and 7 low and high pass, 8 dry (default 1.0). The
  wet, flange, notch and moveable patches are not ported.
* `LoadFoleyData` 0x824f0bf0 ends with `mpUtility = new C_foley_utility` (no members): its module
  (`Foley_Cloth.abk` #228) answers each `cloth_*_counter_msg` call with a RANDOMSHUFFLE into the
  `cloth_*_counter_snd` globals the cloth trick module (#290) picks its variation from.
* The board's grit is a "grab" every few seconds (attack about 224 ms, hold 54 ms, release 260 ms,
  then `GritHoldTime`, 4000 - speed * 0.35 ms plus shuffle and oscillator); rocks are rare random
  bounces gated by a 14.9 s state generator. Neither plays continuously.

## Tricks (player/tricks.rs)

* `SFXObj_Tricks` is PLAYER object 5 (TypeInfo 0x00010050, mix IO 0x40010050); `NewSFXObj`
  appends, so the objects update in sfx order. It runs for the local skater only.
* `Class_Flips` (`Sk8_Air_Flip_Tricks.abk`): created in the air while a trick is performed, or on
  a dismount contact (trick 0x22); not for tricks -1, 0x23 (grab) and 0x24 (coffin). Members 7-9
  are the deck's local angular speed per axis (Z, Y, X) as `|w| / MaxValue * 1000` (1000 at
  most, 0 below `MinInput`), 10 game speed x 500, 11 the trick, 12 the jump fraction x 10000, 13
  the fun meter, 15 `mChallengeType == 0`, 17 `mbInOwnTheSpot`, 18 not rewinding, 19-21
  `BoardFlipsVolZ/Y/X`, 23 `TrickVol`, 25 local, 26 mixer volume 6 (flange). The constructor
  0x825137a0 clamps every member; its stores are authoritative (IDA shifts the stack argument
  names by four).
* The mixer getters (vtable +0x34 to +0x40) are `GetDMixOutput` with preset types 3 (azimuth), 1
  (pitch), 0 (volume) and 2 (frequency); azimuth and pitch read 0 without a player state.
* `cloth_trick` (`Foley_Cloth.abk`, named in lower case) plays per trick, and on the trick's end
  with `mTrickDoneType`, kept for the trick's length. `Destroy` releases the flip and the trick
  foley only; the trick-done foley outlives a detach.
* `UpdateFunMeter` moves toward the multiplier's target (aud_general 0x47EC76B4F9FC79F6) at the
  fall and rise rates. The `mbIsTimesOneFive/Two/Three` flags are `GlobalData`'s
  `timesOneFive/Two/Three` (`DoSimSubmit` 0x827fba0c: the local skater's
  `PhysOut_Scoring2::mMomentumMeterMultiplier` at 3, 2 or 1.5 and up), copied by
  `UpdatePresentationData` after the state managers' `UpdateParams`.

## Cracks (player/cracks.rs)

* `SFXObj_Cracks` is PLAYER object 4 (mix IO 0x40010040): a `Class_Seams` instance per wheel
  (`Seams_Bank.abk`). `PlaySeam` alternates the trigger between 1 and 2 (it is cleared every
  `UpdateParams`), sets the angle flag, the seam surface and the wheel type, and pulses mixer
  input 0.
* Surface patterns 1-15 name `aud_cracks` collections (`SurfacePatternToKey` 0x824e6858); the
  pattern is wheel 0's, or wheel 3's in a manual before wheel 0 lands. Crack type 1: each wheel's
  position turned by `GridAngle` falls in a cell of `Grid_Length` along and `Grid_Width` across
  (both x `Wood_Sidewalk_Mult` for wood seams on the sidewalk pattern); a changed cell is a crack,
  `MinFramesBetweenLeftRight` apart within a pair, with the angle flag unless both wheels cross.
  Crack type 2 (slats): `speed x 100 x game speed (2 at most) x t` against `Slat_Width`, the back
  wheels following 50 behind (`SlatReset` 999).
* A material change on the ground plays a transition seam (`GetTransitionType` + 5) instead.

## Audio PhysOut conditioner (skate2-core physical_player_conditioner_audio_update.rs)

* `PhysOutConditioner_Audio::Update` 0x827c6848 is `UpdateAll` slot 3 (after Grinds). Without it
  `PhysOut_Audio` stayed at its reset values: no wheel landing (the `Sk8_Lands_*` impacts never
  played), no deck angular speed (flips), wheel slide, carving, jump fraction, grind data,
  averaged collisions, stairs or walking. Pedestrian perception reads the same wheel slide,
  grinding and wipeout values.
* Steps: wheel slide `|deck velocity . x axis|` past 1.38 over 15 (at most 1), carving the rise of
  its four-frame average; a wheel landing after more than `kMinAirFrames` (5) in the air with
  `max(-normal . previous velocity, 0) / 9` (at most 1); the jump fraction between
  `kSmallJumpDV` (0) and `kLargeJumpDV` (2.65); the deck angular velocity in the deck's frame and
  the torso angular velocity delta; the grind type and surface latched while grinding; contact
  forces maxed over four frames; the fall speed over four frames against 3.1/3.3/3.7 (off-board
  landing 1-4); hippy jump strength against 0.45/0.75; feet touchdowns, stairs and walking up or
  down (`kUpDownTolerance` 0.07).
* Kept as natively: both stairs flags come from the left foot's surface ID.
* The core vault now selects `aud_general` and `aud_contacts`; the float arrays read as
  `EA::Reflection::Float` (0xE22228FBB3C209D8) elements.

## Front-end sound requests (skate2-core ui::FeSoundRequest, app game_audio.rs)

* Front-end sounds reach the audio system as `Play`, `Loop` or `Stop` of an `FE_SOUND_*` global:
  `FE::GlobalFEPlaySound` 0x826150b0 (one-shot or looping) and `FE::GlobalFEStopSound`
  0x826151a0. Both play or stop only a key whose `fe` collection names a splice, a Hall of Meat
  splice or a logo, through `IAudioInterface` +0x2C (`PlayMenuSound`) and +0x30
  (`StopMenuSound`). The movies' `Audio.PlaySound` is a one-shot `Play`.

## Edit Skater sounds (skate2-core ui/edit_skater.rs, session/edit_skater_frontend.rs)

* createskater `UpdateInput`: in list mode (state 0) its `menu_picker` (wrapping, disabled items
  selectable) sounds Up/Down only when the value changes and A on select, and nothing with no
  items. The thumbnail grid (state 1) sounds every direction, moved or not, while it has items;
  A goes to the native `OnThumbnailSelect` 0x8268aee0 (owned, remove and stamp parts sound
  core_a_button). Style and morph (states 2 and 3) sound Up, Down and A every time.
* `FrontEndState_CaC::ReceiveInput` 0x8268d818: B sounds core_b_button before
  `PopScreenState` or `Finish`; Start sounds cas_finish. Left/Right sound core_nav_left/right on
  trucks, wheels, the four gestures, posture, stance, skin tone, hair/facial hair/brow style,
  ethnicity presets and wear style, and on colour rows only in a type 3 (morph) menu.
* The GenderSwitch popup (`OnMenuSelect` 0x8268b408) is a default `sPopupInfo`: not
  cancellable, default selection Yes. `FrontEndState_Popup::OnEnter` 0x82658474 sounds
  core_popup; sk8popup hands every key to its wrapping `menu_picker`, so only Up, Down and A
  act and sound.
* The morph slider (`OnUpdate` 0x82670400): past `input_threshold` the value steps and
  `cas_slider` starts looping (flag +0x12EE); a clamped step or a released stick stops it, as do
  `OnMenuHilight` (every style/morph Up/Down via createskater `SetSelection`),
  `SetScreenState` and `OnLeave` 0x82680570 (after the exit fade).

## Pause (system.rs SFXObj_Pause, skate2-core FrontendAudioState)

* `SFXObj_Pause` is MAIN SFX 7 (IO 0x40000070). `UpdateParams` 0x82532f48: paused means
  `PauseMan::IsPaused(1)` with a `Presentation`, outside replays, sample capture and NIS scenes,
  and not loading. Paused raises input 0 when `mCurrentMenu` (`+0x618`) is the pause menu, else
  input 2 from the second paused frame (`+0x1C`). `IsExitingToMainMenu` also raises input 0,
  and input 1 is `mSk8HUDState == 0x20`. In `MixMapSK8.mxb` input 0 fades the player, collision,
  pedestrian and reverb outputs out over about ten frames, and input 2 drops them at once.
* `StateManagerFrontEndBase::UpdateFEAudioState` 0x82d67948 makes `mSk8HUDState` the top
  front-end state and sends its `gFEStateInfo` 0x82fded80 row's music state; `mbPause` rows push
  `kPAUSE_STATE_FRONTEND`. Rows used: StartHub 7 (music 3), CaC 0x13 (1), ChallengeMap 0x15 (9),
  jacked 0x2A (8), Options 0x49 (11). Popping the last state restores the HUD stack's free-skate
  state 1 (music 4). A popup reports nothing: `OpenPopup` 0x82d6aaa0 enters FE state 34 without
  `UpdateOnEnter`, so neither its row's pause nor its music state applies.
* `paused` is the session's `audio_paused` (the NIS's `kPAUSE_STATE_NIS` from `RunBasicNIS` to
  `EndNIS`, the challenge pause and every pausing front-end state, the career's included), and
  `mCurrentMenu`/`mFEMusicState` follow the state `ChangeFEMusicState` posted, so an Ignore row
  (the map) keeps the pause menu's fade. Nothing on the skater side checks the pause: in
  a NIS the board is silenced by Pause 2 before the scene starts and after it ends, and by the
  scene's NIS input while it plays. Evidence: docs/evidence/nis-pause-board-audio-2026-10-10.md.
* Measured in an audio dump: the board fades out about 0.1 s after pausing, the pause is silent,
  and rolling returns about 0.1 s after unpausing.

## Wheels (player/wheels.rs, rwac/plugins/iir2.rs PeakingIir2)

* `SFXObj_Wheels` is PLAYER object 2 (TypeInfo 0x00010020, IO 0x40010020) and has no
  `UpdateParams` (its vtable slot is the empty base). `ProcessUpdate` 0x8254f990 runs
  `ProcessWheels` 0x8254d908 for the three `eWheelsType`s: in the air (`mInAir`), in a manual
  (`mbIsManual`), and holding the board (the local skater only, `mIsOffBoard && mIsHoldingBoard`
  or `mIsHandPlant`), then `UpdateSubmix` 0x82527d68.
* The constructor 0x82544158 loads the Jump and Man spin-downs, `data\audio\Whls_spins_Jump_1.snr`
  and `Whls_spins_Man_1.snr` (RAM samples, 48 kHz mono, about 14.8 s), with their `.sek` seek
  tables. The names are `char*` attributes of `aud_general` "wheels" (0x03B710C8_0E1AC13E), and
  `mSampleLength` is 14.0 for both. `InitSFX` 0x825151b0 sets `mbPlayData1`, so the first start
  after an attach plays the Jump spin. Every start then alternates between Jump and Man, whatever
  the kind.
* A start seeks to `length x (1 - clamp(boardSpeed / 50 x 3.6, 0, 1))` seconds (`fsel` pair; a
  held board's fraction stops at 0.26), so a fast board hears the spin from its beginning.
  `StartWheelVoice` 0x82544630 builds SndPlayer1, Rechannel, Resample, Gain and Send (stage 1),
  plays event 5 with the seek table, mutes it, and sends it into the sub mix. For the local
  skater, mixer output 4 also sets the flange send.
* While a kind lasts, its voice takes volume output 1 (or 5 in a manual, 6 holding the board, 7 in
  a hand plant) and pitch output 2 (`UpdateWheelsVoice` 0x82528188). The voice is freed once
  expelled (`Voice::mState` 2) and does not restart until the kind ends. A kind that ends frees
  its voice (`FreeWheelVoice` 0x82528268). `Destroy` frees the voices but keeps the playing flags.
* `CreateWheelsSubmix` 0x825279e0 (stage 2): SubMix, PeakingIir2, HighShelfIir2, the flange Send
  (`GlobalEffects::GetFlangeSubMix(0x2000)`, starting at 0), the reverb Send (`GetReverbSubMix`),
  Pan2D1 to six channels, and the master Send. Natively the constructor builds it; the port builds
  it on the first attach, because objects are constructed without the audio core. The flange and
  reverb sends go to `GlobalEffects` (see Reverb and flange).
* `UpdateSubmix`, while any voice exists, sets the pan from output 0 (azimuth). For the local
  skater it also sets:
  * the peaking filter: frequency `speed x (4500 + (800 - 4500) x occlusion)`, gain 0.1, Q 20;
  * the high shelf: frequency `4000 x speed`, gain `3 + (0.5 - 3) x occlusion`;
  * the flange send (output 4).

  Here `speed` is `mGameSpeed` capped at 1 and `occlusion` is `mWheelOcclusion`. For any skater
  it sets the reverb send (output 3).
* `PeakingIir2` (`PI20`, defaults 96000, 1, 3): `CalculateFilterCoefficients` 0x82bf04f8 is the
  RBJ peaking filter on the root of the linear gain. `Process` 0x82bf85f8 clamps w to the IIR2
  range and Q to 0.2..20, and passes unity gain through (resetting the histories once). It has
  1500 decay samples.

## Rail (player/rail.rs)

* `SFXObj_Rail` (64 bytes, PLAYER object 3, IO 0x40010030) plays the grind and slide loop
  `Class_grind` (`SK8_AEMS_skateboard.csi`, `GRINDS.abk`; 16 members: volume, dry, wet, azimuth,
  pitch, lowPass, highPass, speed, load, surface, parts, swt_volume, enter_Volume, exit_Volume,
  local_Skater, flange_Wet). `mpRailSound[0]` is the grind; a 50-50 adds `mpRailSound[1]` for the
  other truck. The constructor 0x82514508 sets `mCurrentGrindType` to `GRIND_TYPE_NONE` (-1).
  Attaching runs the shared `InitSFX` 0x82515d80 through `SetupLoadData` (output on); `Destroy`
  0x825217f8 releases both instances.
* `UpdateParams` 0x82541978: input 0 = grinding, input 1 = grinding last frame but not now. With
  an instance and no grind, both instances go and the type returns to none. Without one, a grind's
  first frame on a tagged surface builds it: volume 0, dry 0x7FFF, wet/azimuth/pitch 0, lowPass
  25000, highPass 0, load 1024, then speed, surface (0-14), parts (0 for boardslide, tipslide and
  backslash, else 2), the sweetener, enter and exit volumes, the local flag and the local flange
  (mixer output 6). A 50-50 (type 0) adds parts 1.
* Speed: `(mBoardSpeed - 0.5) / aud_rails::MaxSpeed (45) * 3.6`, limited to 0..1 by two `fsel`s
  (NaN becomes 1), times 10000, at most 9000.
* Surface: `SurfaceMapping::GetGrindMaterial` of `mGrindMaterial`. An untagged surface
  (`eAudioMaterialType_Max`, 132) grinds as `eGrindMaterial_Rail1` (4), or not at all under
  `kNoRailWhenUntagged` (`Audio_Debug`, default off); `eGrindMaterial_Untagged` (14) starts nothing.
  `ProcessUpdate` maps the same way but uses 14 instead of returning.
* `aud_rails` "default" keeps four 14-element per-surface arrays for each of parts 0, 1 and 2:
  the rail volume multiplier (`GetRailVolumeMultiplier` 0x82521850) and the sweetener
  (`GetSweetenerVolume` 0x82521b50), enter (`GetEnterVolume` 0x82521e68) and exit
  (`GetExitVolume` 0x82522180) volumes, the last three times 32767. The surface stops at the parts
  0 array's last element (so untagged reads element 13); parts past 2 give 1.
* `ProcessUpdate` 0x82542028, while grinding with an instance: the grind type is
  `aud_rails` 0x6710834AD5E0847F unless that is none (-1, as shipped), else the skater's. A change to
  a non-50-50 drops the second truck; a change to 50-50 adds it (parts 1). Both instances then take
  volume 0x7FFF, dry = mixer output 1 x the rail volume multiplier, wet = output 5, flange = output
  6 (local only), lowPass/highPass = outputs 3/4, azimuth = output 0, pitch = output 2 and the
  speed; the first takes the parts, the second 1 while the type is 50-50.
* `Attrib::Attribute::GetLength` reaches the audio code as `AttribSource::length` (the app answers
  with `AttribDb`'s `Attribute::len`).
* Verified offline (`aems_probe`): a 50-50 on Rail1 holds about -20 dB after its enter transient,
  a boardslide on rough wood about -26 dB, a second truck about -21 dB.

## Clothing (player/clothing.rs)

* `SFXObj_Clothing` (PLAYER object 6, 68 bytes, mix IO 0x40010060) plays three things. All
  values come from `SFXCTL_PlayerPhysics`, `GlobalAttributeCollection::aud_general_clothing`
  (+0x58) and `aud_collisions_default` (+0x28).
  * Wipeout cloth falls: `c_cloth_falls` (`Foley_Cloth.abk`).
  * Body slides: `c_body_slide` (`Bodyslide.abk`).
  * Push leg foley: `sk82_cloth` (`SPLICE_TYPE` 23) splice 32 (push up) and 33 (push down), on
    the master submix.
* `UpdateParams` 0x8254e0f0 runs `HandleFalls`, `HandleLegFoley`, then `CheckForBodySlides`.
  `ProcessUpdate` 0x82530670 runs `UpdateBodySlides`, `UpdateFalls`, then `UpdateLegFoley`.
* No call is gated on the local skater.
* Attach runs the shared `InitSFX` 0x82515d80 (mixer outputs on). Detach runs `Destroy`
  0x8252f8b8, which releases both instances and both splices; the slide duration resets only when
  a slide existed. The `was` flags and the fall speed survive a detach.
* `HandleFalls` 0x8252fc20 (switch 0xE1EA210F3CB3F8B8):
  * The torso spin and the limb speed are each scaled to thousandths of `Max_Tumble_Speed`
    (0x0A9A9BD1150FE838) as `fctiwz(1/max x speed x 1000)`. The larger one is
    `mMaxClothFallSpeed`.
  * A falls instance starts on the first wipeout frame, with the torso value as its speed.
  * It is released when the wipeout is over or ends.
* `UpdateFalls` 0x82516c78 sets these members of the falls instance:
  * volume = 0x7FFF;
  * azimuth = output 0 (at most 0xFFFF);
  * pitch = output 5;
  * speed = `mMaxClothFallSpeed` (at most 1000);
  * dry = output 6.
* `HandleLegFoley` 0x8252f968 (switch 0x91E69D3AC71F1D7F):
  * Push up starts when general pushing begins; the next push start or a wipeout cuts it.
  * Push down restarts on every push start; a wipeout cuts it.
  * Both play silent with the update's `t`, and their spread is 1 for the local skater and 0
    otherwise.
* `UpdateLegFoley` 0x825303b8: while a splice plays, it is updated with:
  * volume = output 8 / 32767;
  * pitch = output 7 / 4096;
  * azimuth = output 0 in degrees;
  * the state's `t_DeltaTime`.

  A finished splice is released.
* `GetBodySlideInfo` 0x8252fea8 checks only the first six of the eight skeleton contacts:
  * A contact slides when `!(|slideSpeed| <= 0)`, so NaN counts as sliding.
  * Its surface is contact 1's if contact 1 has one, else the first non-zero surface.
  * The speed is the player speed in thousandths of 0x787171ECD02DBBC3.
  * Surface: rough concrete (2) without a mapped material; face (4) on a face contact; else
    `SurfaceMapping::GetBodySlideMaterial` 0x8251c030 (mapping record word 10).
  * `kSlideSurface` (default -1) overrides the surface.
* `CheckForBodySlides` 0x82546710 (switch 0x76BE385BAB3182B7):
  * A slide starts while in contact once the speed exceeds `Slide_MinSpeedForSlideStart`
    (0xB021DB338B89D0F2).
  * It stops at or below 0x746EA8EF187E1571, or out of contact.
  * At creation, `ground_slide` is the wipeout flag.
* `UpdateBodySlides` 0x825300d0:
  * tumble = the torso spin in thousandths of 0x5D2F244E82CF7255;
  * the duration grows by `Slide_TimeIncrement` (0x4435464D195C3526) each update, at most 7000;
  * pitch = output 10, dry = output 9.
* Not the same sound as Tricks' `cloth_trick`: Tricks plays trick foley from `aud_general`
  tricks/clothing; Clothing plays falls, slides and pushes.

## Treatments (player/treatments.rs)

* `SFXObj_Treatments` (PLAYER object 8, 40 bytes, mix IO 0x40010080) owns one
  `Class_Treatment` (`Treatments.abk`).
  * `UpdateParams` 0x82530c08 creates it once, for the local skater only.
  * The constructor 0x82513bd8 clamps each of the 38 members.
  * Initial values: pitch 0x1000, low pass 25000, `game_Speed` 500, and members 31-37 =
    `aud_general` treatments (+0x60, `string_hash64("treatments")`) `volume_prelandwhoosh`,
    `volume_bigvrb`, `volume_outofbound`, `volume_wind`, `volume_bail`, `volume_ownspot`,
    `volume_rumble`.
  * The 31 stack arguments fill members 7-37 in order. The stack area starts at `sp+0x50`;
    IDA's argument names there are shifted.
* `ProcessUpdate` 0x82530da0 sets these members:
  * volume = output 2 when `kEnableTreatments`, else 0;
  * dry = 0x7FFF, wet = 0;
  * azimuth = output 0, pitch = output 1;
  * game speed x 500;
  * `replay_Forward` = not rewinding;
  * air time and time to land x 1000 (at most 10000);
  * jump height x 166.66667, `fsel`-clamped to 0..1000 (NaN gives 1000);
  * board speed x 0.08, `fsel`-clamped to 0..1, x 10000 → `speed` (at most 10000) and
    `landing_Velocity` (at most 1000);
  * `fun_Meter` = 0.

  `event_Type`: a bail gives 5 (`bail_Meter` = severity x 1000); then out of bounds gives 1
  (`out_Of_Bound`, proximity = amount x 10000); then Own the Spot gives 4 (`own_The_Spot`).
* The module's `ClassData` reads members at a 24-byte header offset:
  * read: volume through low pass, `event_Type`, the pB members, `landing_Velocity`,
    `time_In_Air`, `jump_Height`, `own_The_Spot`, `fun_Meter`, `bail_Meter`, `out_Of_Bound`,
    `speed`, `volume_New_Trick`, `volume_Big_Vrb`, `volume_Out_of_Bound`;
  * not read: time remaining, game speed, OOB proximity and the other volumes.
* `VDPresentationState::mOutData` bail and out-of-bounds values come from `WorldAudioState`
  (see Presentation inputs).

## Sense of speed (player/sense_of_speed.rs)

* `SFXObj_SenseOfSpeed` is PLAYER object 9 (TypeInfo 0x00010090, mix IO 0x40010090), for the
  local skater only. Its layers run from `mBoardSpeed` x 3.6 (km/h) against `aud_senseofspeed`
  "default" ranges, which the constructor 0x82549588 reads (`CheckAttribVariables` 0x825361d8).
  It also loads `x_jet_rolling.grain` (`AudioAssetManager::SyncLoad`, bank 0x10, sfx 1).
* `SenseOfSpeed_rattle`, `SenseOfSpeed_tone` and `SenseOfSpeed_wind` (`sense_of_speed.abk`), all
  in `SK8_AEMS_skateboard.csi`. `UpdateParams` 0x82535700 releases a layer below its minimum.
  From the minimum it creates the instance when it is absent and the layer's cvar is set.
  `kPlayRattle`, `kPlayWind` and `kPlayRocket` default on; `kPlayTone` defaults off, and the tone
  class has no module instance in the bank.
* Member layout: Volume, Azimuth, Pitch, Speed, 25000, 0, WetFX, DryFX, then the class's own
  volumes. The rattle adds its normal, metal and high-tone volumes; the wind adds low,
  buffering 1, buffering 2 and whistle. A new instance starts at 0, 0, 4096 and the speed's
  fraction of its range x 1000; those volumes are clamped to 0..32767.
* `ProcessUpdate` 0x82535ce0 writes the master volume and the mixer values. The rattle takes
  azimuth 0, pitch 2 and dry volume 1. The tone takes pitch 3 and dry 4; the wind takes pitch 3
  and dry 5. WetFX is 0, then the fraction is recomputed with a fused `speed * 3.6 - min`.
  `UpdateParams` multiplies and subtracts separately.
* The fraction is `fsel(-x, 0, x)` then `fsel(1 - x, x, 1)`, so NaN reads 1.
* The rocket `AudBoard` is allocated by `InitSFX` 0x82517fe8 and runs while
  `!(km/h <= RocketMinSpeed)` and `kPlayRocket`. It starts on the master sub mix with the
  attribute's five grain floats. Its update is volume `RocketVol x (mixer 6 / 32767) / 32767`,
  pitch mixer 3 / 4096, azimuth 0 and the fraction as intensity. Below the range it is destroyed
  (twice natively).
* Its timer ids are 8 + the state instance, after the skateboards' eight.

## Hand grabs (player/hand_grabs.rs)

* `SFXObj_HandGrabs` is PLAYER object 11 (TypeInfo 0x000100b0, mix IO 0x400100b0). Its checks and
  splice updates run for the local skater only. Mixer input 0 is 32767 while skitching, written
  for every skater.
* Starting to skitch, plant a hand or grab a dynamic object (`CheckForSkitch` 0x8253a620,
  `CheckForHandPlants` 0x8253a7a0, `CheckForGrab` 0x8253aa28) does three things. It replaces the
  grab and material splices, plays the hand grab (bank 5, splice 0x302), and then plays the
  surface's grab splice. For skitching that is bank 6, splice 0x1C0. For a hand plant it is the
  planted surface; for an object it is the `aud_dynamicobject` (0xB05F7C74C63ACFB8) collection
  `mDMOKey` names, at layout word 90.
* Lifting a hand plant or letting go of an object plays the surface's grab-off splice. Material
  132 plays nothing. The flags survive `Destroy`, which only deletes the splices.
* `AudioSystem::GetGrabCollision` 0x824efc40 picks the bank from the material's
  `tAudioSurfaceMapping` type, reading material 0 outside the table. The index is:
  * type 5: grab layout word 16, grab-off `Grab_Off`;
  * type 6: grab word 20, grab-off `Metal_Grab_Off`;
  * type 7: grab `HOM_Soft_L1`, grab-off `Grab_Off`.
* `UpdateSplices` 0x8253ad00 updates each splice until it stops: volume outputs 1, 2 and 3,
  pitch 4, azimuth 0. The material and release volumes are also scaled by the grabbed object's
  `GetGrabVolume` and `GetGrabOffVolume` (0x9149686ADC031389, 0x397202D0AA482601), and only when
  `mDMOKey` names a collection, whatever caused the splice.
* `mDMOKey` comes from `GetGrabbedObjectKey` (see Presentation inputs). It stays 0 while the host
  feeds no dynamic objects, so objects never play yet.

## Off-board footsteps (player/offboard.rs)

* `SFXObj_OffBoard` is PLAYER object 10 (TypeInfo 0x000100a0, mix IO 0x400100a0; constructor
  0x82537038, `UpdateParams` 0x82549910, `ProcessUpdate` 0x82549d20, `Destroy` 0x825379a0). It
  runs for every skater; the splicer voices and the heel and foot-push splices are the local
  skater's only. Mixer input 0 is 0x7FFF off the board, else 0.
* `UpdateParams` maps `mPlayerSpeed`, each foot's slide speed and angular Y speed through the
  `aud_general` footstep graphs (0x90 bytes: 16 x at +0x10, 16 y at +0x50; `fctiwz`). The heel
  touches with `m*FootTouchingOffBoard`; the toe never does. Leaving a foot plant arms a
  10-update jump timer on the foot that was down (right first). Then `CheckForFootSteps`,
  `CheckForCloth`, `CheckForJumps`; `CheckForOffBoardSpeech` (non-local skaters back on the board,
  `Sk8SpeechSystem`) is not ported.
* A step is a heel that was up and is down. `CheckForFootSteps` (gated by `kEnablePCFootSteps`)
  takes each foot's off-board surface (Concrete_Rough when none; `kFootStepOverride` replaces
  both), creates the `playercharacter_footstep` instances (`SK8_AEMS_Foley.csi`, left first) and,
  for the local skater, four "FootStep SubMix" voices per foot. Each step draws new notch
  frequencies and gains (`AdjustFilterParameters`, `rw::math::Random`) for every skater.
* Local steps: run when the speed graph exceeds `0x729D6290FB6A8E3B` (500). The heel's primary
  splice is the material's walk collision when its `aud_material` has `Add_In_Walk` (+0x7D):
  `Soft_L1` walking, `Soft_L3` running, from the collisions, metal or HOM bank; footstep grass and
  metal surfaces use the `types` table (collisions bank 764-767, metal 439-444, primary and
  secondary) instead. `GetSpliceTuningKey` finds the splice's `aud_cluster` (high-pass, low-pass,
  notch frequency/gain/Q, volume factor) in the collisions or the other banks' tuning table, else
  "default"; a missing collection keeps the previous cluster layout (`Instance::Change`). The
  primary's volume factor is `Walk_Volume / 32767` when the material has a walk collision, and the
  left foot reads the right foot's material for it (native). The foot pushes come from
  `spliceTypeMap` by terrain row (1, 3, 5 vs the rest) and footstep surface: foot_pushes,
  smoothconc, woodfoot, dirtfeet, grassfeet or collisions 768/769.
* The voice is `SubMix`, `HighPassIir2`, `LowPassIir2`, three `PeakingIir2`s and the environment
  `Send` in one channel, `Pan2D1` (45/135 degrees, norm 0) and the master `Send` in six. The
  foot-push voices also take the random notches (gain / 32767). Send 6 carries the foot's wet
  level (mixer output 8) into `GetReverbSubMix`. Pan is the mixer
  azimuth x 360/65536, set when a step starts.
* `UpdateFootSteps`: both instances get mixer azimuth, pitch, low/high-pass (outputs 4, 5), wet
  (6) and dry (2) FX, heel/toe, the foot's AEMS speeds, the jump flag (timer or a jump-up cloth
  allocated), `mOffBoardLanding`, the speed graph, the shoe (`kFootStepShoeType`, else 1), the
  footstep surface, the terrain, the six volumes, local and the notches, clamped to the member
  ranges (landing 1-4, surface 1-7, terrain 1-5). Local splices play at output 7 (11 when the
  right foot's material has a walk volume) times each splice's volume factor; one parameter block
  serves all eight splices, so `Splice::Update`'s pitch scaling compounds.
* Cloth: each step plays `sk82_cloth` walk/run/sprint by `mPlayerSpeed` against the clothing
  collection's run (3.3) and sprint (4.7) speeds, and while the local skater carries the board a
  `sk8_foley_rattle` (3/4/5). Jumps: an off-board jump (24), a hippy jump (short 26, normal 28,
  high 30) or leaving a foot plant plays the jump-up cloth; when the velocity turns downward while
  it exists the jump-down cloth (25 after a jump, else 27/29/31) replaces both. The jump-up cloth
  updates only while playing and stays allocated, so it keeps the AEMS jump flag set until
  replaced. Mixer volumes: cloth 3, rattle 12, jump up 9, jump down 10.

## Reverb and flange (global_effects.rs, reverb.rs, rwac/delay_line.rs, rwac/plugins/delay.rs, rwac/plugins/reverb_model1.rs)

* `GlobalEffects` (constructor 0x82505c10, created with the audio singletons) builds two
  environment reverbs and two flanges. "EnvSendSub" (stage 199) is the bus every wet send feeds
  (`GetReverbSubMix` 0x824e8668): its Sends 1 and 2 carry each instance's `fxlevel` into the
  instance's "EnvSub" (200: Gain `fxgain`, PeakingIir2 notch, three Sends). Those feed "RvrbSub"
  (201: Delay pre-delay, ReverbModel1, Gain, two Sends into a left and a right "rvrbfiltsub" at
  202) and two "rvrbdelaysub" (201: Delay, high and low pass, Gain, Pan2D1). Filters and delays
  pan into the master mix at 270 and 90 degrees. "FlangeSub" (150) x2 (`GetFlangeSubMix`
  0x824e8678: index 1 for patch 0x4000): Delay, PeakingIir2, a wet Send back into "EnvSendSub",
  a dry Gain, Pan2D1 to the master mix.
* The LFO task (`RwAudioCoreTimer` 0x824e86e0, a `TIMERSTAGE_BEFORECOMMANDS` timer) sweeps 14
  attributes every mixer frame: `out = maxrange * depth * sin(2 pi t / cycle) + base`, the time
  wrapped with the VMX reciprocal and a fused subtract. Per instance: the notch frequency, both
  delay times and both delay pans (90 degrees around 270 and 90, the second half a cycle on);
  per flange its delay time and notch frequency (`SetFlangeParms` 0x824f4220).
* Every Delay the game creates reads its constructor one word past the float it stores (the
  debug `Param` is {tag, value}); the word there is a string pointer, a tiny negative float, so
  each line starts at 256 samples and its timer grows it to the longest delay asked for.
* `SFXObj_Reverb` (MAIN 5, IO 0x40000050). `InitSFX` 0x82546b68 turns its output on, puts the
  first `aud_reverb` collection in both slots (instance 0 at level 1, instance 1 at 0;
  `VerbInit` 0x82531560 to `SetupReverb` 0x824f2ef8) and gives the first and next `aud_flange`
  collections patches 0x3000 and 0x4000 (`InitializeFlange` 0x82531330).
* Collection order is `Attrib::Class::GetFirstCollection` / `GetNextCollection` over the class's
  `VecHashMap` (size `collection_reserve`, slot = low key word mod size, linear probing). For
  `skater.vlt`: the first `aud_reverb` is 0x9572128E_FDA5C188 (the only key in slot 0 of 33);
  the `aud_flange` pair is 0xDFDFFAD6_7CCBA322 then 0xC6290FFC_B9E84CD9.
* `UpdateParams` 0x8254e1e0, with a `Presentation`: inputs 0-3 are `GetReverbCategory`
  0x82517018's categories 1-4 (`UpdateNicotine` 0x825170c0). A non-zero `GetReverbKey`
  0x824f1df8 different from the active one starts `SetupWPVerb` 0x82531988: the other slot takes
  it at level 0, and `ProcTimeTrans` 0x82531a68 cross-fades linearly over one second
  (`VerbUpdate` 0x82531870 / `UpdateReverb` 0x824f2c20 each frame).
* `ProcessUpdate` 0x82532000: outputs 0-3 are flange 0's dry and wet and flange 1's
  (`SetFlangeDry`/`Wet`, `UpdateFlange` 0x824f43b0), output 4 the global reverb volume that
  scales the three reverb sends (`UpdateGain` 0x824f2db0, with the last `SetupReverb`'s levels
  for both instances). In `MixMapSK8.mxb` output 4 idles at 32730 and drops to 1 under
  `SFXObj_Pause` input 0; the flange levels idle at 1.
* Wet routing (each as the native creator does it): AEMS sample players get a Send after the
  low pass into "EnvSendSub" at 0 until input 5; collision voices' send 1 and the splicer
  wet voices' send 1; the skateboard stage-2 send 2 (reverb) and send 1 (flange 0);
  `SFXObj_SkateBoard::ProcessUpdate` sets both flanges' pan (`SetFlangeAzimuth` 0x824e86a0,
  azimuth / 65536 * 360); the wheels sub mix's sends 4 and 3; footstep voices' send 6; the menu
  flange voice's send 1 into flange 1.
* `Delay` (0x82bfd438): attribute 0 seconds (rounded to samples), 1 feedback (|fb| <= 0.99);
  out = the tap, written = input + fb * tap. A changed delay cross-fades both taps over 128
  samples, the feedback from each. A delay longer than the history waits (state 2).
* `ReverbModel1` (0x82bf99e8): space size clamped to 2..83.3 m; six combs at 0.8..1.2 x size,
  lengths the next unused primes above the travel time at 343 m/s; low-pass gain g1 from the
  distance and rate table divided by the brightness (at least the last g1 + 0.001); feedback
  g2 = (1 - g1)(1 - 0.366 / time). Comb: w = x + g1 w[-1] + g2 w[-M], out (w[-M] - g1 w[-M-1]) / 6,
  summed; one all-pass per output pair (mono: 288 samples, 0.7, scale 2). A changed attribute
  takes three frames to apply.
* `DelayLine` (0x82be7f68): taps behind the samples written since `Reset` read 0; a tap shorter
  than the frame reads this frame's writes.
* Not ported: the AEMS flange and notch patches (0x2000/0x4000/4096, 1024).
  `GlobalData.reverbKey` is fed from WorldPainter layer 10 (see Presentation inputs). The emitter
  system's reverb volumes and their transitions are under Reverb volume transitions.
* Checked offline against float64 models of the asm: ReverbModel1 impulse response (time 1,
  size 45, brightness 0.5) within 1.3e-8 of a 0.23 peak; the Delay with its delay stepped every
  frame (299 cross-fades) within 5.7e-8. The whole chain's impulse response is finite and falls
  about 33 dB per half second.

## Presentation inputs (skate2-core session/selected_session_public_facade.rs, physical_player_phys_out_audio.rs; app game_audio.rs)

* Reverb key: `AudioPresentation::DoSimSubmit` 0x827fb2f8 starts from a zero vector. For the
  skater whose `SkaterData` has the local bit (0x80000000), it loads that skater's
  `PhysOut_Skeleton::mSkeletonHipsPos` (+0x40), and `VectorPermuteConstant<0,6,0,0>` makes it
  (x, z). `LayerMan::GetAttribData` reads layers 8, 7 and 10 at that point into `GlobalData` +0,
  +8 and +0x10 (`reverbKey`). `SelectedLiveSession::audio_reverb_key` does the same with
  `try_attrib_key(10, (x, z))`; layer 10 keeps its case. Without a local skater it samples (0, 0).
* The key is used as the `aud_reverb` collection key as-is. The layer's palette regions are
  `reverb01` to `reverb24`, and `skater.vlt`'s first `aud_reverb` collection,
  0x9572128E_FDA5C188, is `StringHash64("reverb04")`.
* `AudioSystem::GetReverbKey` 0x824f1df8 returns, in order of precedence:
  * `mAdditionalReverbKey` (+0xC98) while `mbUseAdditionalKeys` (+0xC90) is set;
  * otherwise the `Presentation`'s `GlobalData.reverbKey`;
  * otherwise 0.
* `SetAdditionalLocationKey` 0x824f1b30 is called from `GameModule::GameLoad` 0x8273c200 with
  `GameInfo.additionalWorldLocation` and reads `fe_locations(key)`.
  * When bool 0x9FD0A05E_9C39B47D is set, it stores the RefSpec keys and sets the flag: reverb
    0xE486236A_56B7A93B at +0xC98, ambience 0xE2040B41_081A056D at +0xCA0, emitter
    0xE0817948_EE509E0E at +0xCA8, and the data key at +0xCB0.
  * Otherwise it clears all of them.
  * The port loads no additional world, so the flag stays clear.
* Bail: `VDWriter::WriteData` 0x827ff8c8 calls `_PrepareBailData` 0x827fcd40 only for the local
  actor. It writes a record unless `PhysOut_Scoring2::mHoM_BailData.mOverallWipeoutDurationInSeconds`
  <= 0, so a NaN duration still records. The record holds:
  * +0x190: the time in wipeout;
  * +0x194: `mNormalizedBailValue`, which is `PhysOut_Collision`'s skeleton
    `mContactForceTotalDuringWipeout` (Collision +0xA0);
  * +0x198: the head impact.
* `VDReader::ReadData` 0x827fdd78 sets `bBail` from the record count and `fBailSeverity` from
  +0x194. `SkaterAudioPhysOut` now carries `overall_wipeout_duration` and
  `contact_force_total_during_wipeout`. The host sets `bail` = !(duration <= 0) and takes the
  severity from the latter.
* Out of bounds: `ReceiveOutOfBoundaryAmountMsg` 0x827fc718 keeps the amount at writer +0x38.
  `WriteData` records it unless it is negative (0x827ffb1c), then resets it to -1 (0x827fff90),
  so each message makes a one-frame record. `ReadData` sets `bOutOfBounds` and
  `fOutOfBoundsAmount` from that record. The session's `out_of_boundary_amount` is that record
  (`take_out_of_boundary_amount` on the marker UI).
* `ReadData` writes the severity and the amount only when their record exists, keeping the last
  value otherwise. `SFXObj_Contacts` and `SFXObj_Treatments` read them only under their flags,
  so the host passes 0 when the record is absent.
* Both `SfxCtx` sites (`world_update_params`, `process_update`) take the four values from
  `WorldAudioState`. `PhysicsInputs.visual_bail` (mixer input 11) is the same `bBail`.
* Grabbed object key: `SFXCTL_PlayerPhysics::UpdateParams` clears `mGrabbingDMO` (+0x2F9) each
  update. While the local skater grabs (flag 0x40), it stores
  `AudioSystem::GetGrabbedObjectKey` 0x824e69e8 (call at 0x8254ad18) as the 64-bit `mDMOKey`
  (+0x300), and sets `mGrabbingDMO` when that key is non-zero.
* `GetGrabbedObjectKey` forwards to `CSTATEMGR_DynamicObject::GetGrabbedObjectKey` 0x82550430
  (AudioSystem +0x4E8, state manager 8; 0 when absent). That returns the first state in list
  order with `bIsAttached` (+0x34) set and a non-zero key at +0x48.
* `SFXCTL_DynamicObjectPhysics::UpdateParams` 0x8253e4c8 writes that key each update: the
  controller's `mAttribKey` (the `aud_dynamicobject` sliding or rolling key from the object's
  `dynamicType`) while `DynamicData.grabbedObject` is set, else 0.
* `CSTATEMGR_DynamicObject::UpdateParams` 0x825544a0 attaches the first five `DynamicData`
  entries. When none of those five is grabbed, a grabbed entry further down takes the fifth slot.
* `DoSimSubmit` sets `DynamicData.grabbedObject` (+0x20) when the local
  `PhysOut_OffBoard::mIsGrabbingObject` holds and
  `LWGrabbableObjectMan::GetGrabSplineOwnerObjectID` of `PhysOut_State::mGrabSplineDesc.mObjectID`
  equals the entry's `objectID`. `SelectedLiveSession::audio_grabbed_dynamic_object` returns that
  owner (`DmoObjectMan::grab_spline_owner`, 0x82e57dd0) while the skater grabs.
* PLAYER (state 1) updates before DYNAMICOBJECT (state 8), so the player reads the key from the
  previous frame's DMO update. `PhysicsInputs` is built before any manager updates, which
  preserves that order. `grabbed_object_key` stays 0 while `WorldAudioState::dynamic_objects`
  is empty (see Dynamic objects).

## Environment ambience (ENVIRONMENT state)

`CSTATEMGR_Environment::EnterGamePlay` 0x8254fc18 creates one `CSTATE_Environment` with `Setup(1)`:
`SFXObj_Ambience` (TypeInfo 0x00020000, objectID 0x40020000), the location's ambience bed
(`world/environment.rs`). `GameModule::OneOffLoad` 0x8273add8 calls `AudioSystem::EnterWorld` once
at boot, whose `InitializeMixer` 0x824ed830 enters every state manager's gameplay; only `Shutdown`
leaves it. The port creates the state on its first update and updates it with or without a world,
so the front end has its bed.

* Data. The constructor 0x82545168 runs `LoadResidentData` 0x8252a530, which `SyncLoad`s
  `data\audio/ambienceresident.big` (16 `.snr` headers) into `AmbienceInfo`s: the header, the entry
  name with its last letter made 's', and `atol` of its first two characters as the id. It then creates
  the stream pool 'Amb0' (one stream, 0x80000 buffer bytes). The beds are looped 5-channel 48 kHz
  EA-XMA streams (format 0, play type 1, loop start 0) in `data\audio\ambience.big`, 3.8 to 7.3 M
  samples each.
* Key. `GetAmbienceKey` 0x8252a058 takes the front end's ambience (`soho_alley`,
  0xF646CF81_5FDFE13B) while `mbIsFE`, then for 30 more updates, and for as long as `IsLoading` holds
  at the last one. Otherwise it takes `AudioSystem::GetAmbienceKey` 0x824f1cb8: the additional
  location's key, else `GlobalData.ambienceKey`, which keeps its last value through updates without
  a world. A set `aud_general_test_ambience` (+0x68 of
  `GlobalAttributeCollection`, RefSpec 0x51BAD6E2_DB03B4AE) replaces either.
  `AudioPresentation::DoSimSubmit` 0x827fb2f8 reads `GlobalData.ambienceKey` from WorldPainter layer
  7 (`eAudio_Ambience`) at the local skater's hips (x, z); layer 8 is emitters and layer 10 reverb.
  Each region name hashes (`StringHash64`, case kept) to an `aud_wp_ambiences` collection. All 21
  collections match a palette name: `svm_south` is id 1, `soho_alley` (the FE) is id 15, `spillway`
  is id 14.
* `mbIsFE` (AudioSystem +0x7C8) is set by the constructor and by `ExitGamePlay` 0x824ed990 (from
  `GameModule::GameUnload`), and cleared by `ExitFE` 0x825087c0, which `FrontEndManager::Render2D`
  calls once `StateLoading::GameLoadState` sets FE state 3. The session's `audio_is_fe` is the main
  menu being up. `IsLoading` is the load screen (FE state 0x51, `FrontEndState_TipLoader`, pushed by
  `StateLoading::RequestLoadScreen`). The port has none, so the front-end bed lasts the 30 updates
  after the menu closes. `mbIsFE` is also `SFXObj_Menu`'s mixer input 2.
* `aud_wp_ambiences` layout: `volume` f32, `id` i32, `fadeOut` f32, `fadeIn` f32. The vault has
  volumes from 0.55 to 1.0 and fades from 1 to 3 seconds.
* `UpdateParams` 0x8254dd30 calls `UpdateAmbience` while the key holds and `ChangeAmbience`
  0x825452d0 when it moves. It then sets mixer input 0 to the fade:

  | State | Input 0 |
  |---|---|
  | Fading out | `fctiwz(t / fadeOut * 32767)` |
  | Fading in | `0x7FFF - fctiwz(t / fadeIn * 32767)` |
  | Playing | 0 |
  | None | 0x7FFF |

  The value is clamped to 0..0x7FFF.
* `ChangeAmbience` takes one step per update:
  1. A playing bed starts fading out. With a new key, it also creates `c_main_ambience_crossfade`
     with the members `[0, 0x7FFF, 0, 0, 0x1000, 25000, 0, 0, 0, id]`. The id comes from
     `GetAmbienceCrossFadeID` 0x82515a40, clamped to 1..25. That lookup gives 25 to or from
     `spillway`, the pair's row of the 24-row `ambienceCrossFade` table 0x82024d08 in either order,
     and 6 otherwise.
  2. The fade-out ends after `fadeOut` seconds with `StopAmbience`: the stop event, then
     `Voice::Release`.
  3. With no bed, the new bed starts fading in and the key is taken.

  `UpdateAmbience` 0x8252a220 ends a fade-in after `fadeIn` seconds, releasing the crossfade instance.
  A fade-out whose key comes back turns into a fade-in at `fadeIn - fadeOut/fadeOutDur * fadeIn`
  (`fnmsubs`).
* `ProcessUpdate` 0x8252a310 sets three plug-in attributes:

  | Plug-in | Attribute 0 |
  |---|---|
  | Gain | `volume * GetVolume(0) / 32767` |
  | Resample | `GetPitch(2) / 4096` |
  | LowPassIir2 | `GetFrequency(3)` |

  During a fade, crossfade members 0 and 8 follow `GetVolume(1)` through `SetMemberData`.
* `PlayAmbience` 0x82529c90 builds a stage 0 voice of `SndPlayer1`, `Rechannel`, `Resample`, `Gain`,
  `LowPassIir2` and `Send`, all with 6 channels. It plays event 0 with these parameters:
  * path `data\audio\ambience.big|NN_name_amb.sns`
  * the resident header
  * pool 'Amb0'
  * expel mode 1

  The Send goes to `GetMasterSubMix`. `Rechannel` turns the 5 channels into 6
  (`ReChannelGainWrite`, channel for channel, LFE silent).
* `PlayAmbience` sets no gain, so the bed plays at the Gain's default of 1.0 until the next
  `ProcessUpdate`, which on the 360 is one update at the default profile's full volume. The port
  sets the Gain to 0 before the play event (PC port change): the level the fade-in starts from.
  Without it, a profile saved at no volume heard the bed at full volume at boot, for as long as
  the first frames took (a mix dump at boot with all volumes at 0: peak 0.41 for one 256-sample
  block, then silence).
* The destructor 0x82529b60 releases the voice (no stop event) and the pool, leaving a running
  crossfade alone. It runs only at `Shutdown`, which the port does not have.

### Streams: `SndPlayer1` play types 1/2 and `StreamPool`

These now follow the native branches:

* `PlayHandler` 0x82c00b28 takes `GetInstance` and `AcquireStream`. It queues the file from the
  seek chunk, then the loop twice from `loopStartStreamOffset`.
* `GetNextChunk` 0x82bf2658 reads `Stream::GetChunk` and releases skipped 'E'/'U'/'H' chunks.
* `StreamNextChunk` 0x82bfb150 ends a waiting request whose first file request went free.
* `StartRequest` 0x82bfb718 reads the header from the stream for format 1.
* `HandleLoopStart` 0x82bfb298 and `HandleSampleEnd` 0x82bfb440 queue one more loop and feed the
  next chunk with a decoder reset. Play type 2 handles the RAM part, then the streamed part, with
  the Format0 parser.
* `FeedCleanup` 0x82bfafc0 and `RemoveRequest` 0x82bfb978 release the chunks and the stream.

The parsers are ported: `Format0ChunkParser` 0x82bf2140 uses the top bit as the last-chunk flag and
clears it in the buffer; `Format1ChunkParser` 0x82bf21c8 ends on an 'E' chunk.

`rwac/stream_pool.rs` ports the pool and the stream:

* `AcquireStream` 0x82be3288 returns the context's own stream with one more reference, else the
  first free one.
* `ReleaseStream` 0x82be3570 kills the stream on its last reference.

Structural difference: each pool stream reads its queued files in order on its own thread. It holds
at most `bufferBytes` of unreleased chunks, and `GetChunk` gives nothing while the next chunk is
loading. A chunk's readiness time therefore differs from the 360's asynchronous reads; chunk order
and decoder resets are the same. Stream stealing by priority (`StreamLostCallback` =
`RemoveAllRequests`) is not ported, because each pool the port makes has one user at a time.

The host provides the files through `StreamFiles`. `GameStreamFiles` resolves game paths under the
game root and reads `archive|entry` paths in place through the archive's named table.

### Verification

A temporary example built the `PlayAmbience` voice on a bare `AudioCore` with the 'Amb0' pool and
`GameStreamFiles` over the game directory. The example has since been deleted.

* **Resident table.** The 16 entries from `01_svm_south_amb.snr` to `16_wf_main2_amb.snr` parse,
  and their `.sns` names resolve in `ambience.big`.
* **Pool reuse.** A first voice played, then was stopped and released. A second voice got the pool's
  one stream back.
* **Bit-exact decode.** All 5 source channels matched a direct decode of the stream bit for bit,
  starting 514 samples in. Beds 01, 14 and 15 were checked. Native starts one frame later for the
  XMA start delay; the rest of the 514 is the read thread's first chunk. The channels land in RWAC
  FL, C, FR, SL, SR, and LFE stays silent.
* **Loop.** Bed 14 (3,829,240 samples) was rendered for 82 seconds. After the loop it restarts from
  the file's start with no gap and is bit-exact, through the queued loop request and a decoder reset.
* **Mix map.** DMixIO 0x40020000 exists in `MixMapSK8.mxb`. With the SFX slider at full:

  | Fade input 0 | Volume output 0 | Crossfade output 1 |
  |---|---|---|
  | 0 (playing) | 10290 | 0 |
  | 0x7FFF (no bed) | 0 | 7284 |

  Between those the outputs are linear. Pitch is 4096 and frequency 24971 throughout.
* **Vault.** `aud_wp_ambiences` has 21 collections, each with a volume, an id from 1 to 16, and
  fades from 1 to 3 seconds.
* **Tests.** `cargo test --release -p skate2-audio` passes (30 tests).
* **In game.** A main-menu mix dump holds the front-end bed at about -37 dB RMS throughout. In the
  push demo the world input carries ambience key 0x27E5DE92_D436CCBC, and the emitter key follows
  the skater.

## Traffic (world/traffic.rs, traffic_engine.rs, traffic_horn.rs, traffic_skids.rs)

* TRAFFICCAR is state 4. `CSTATEMGR_TrafficCar::EnterGamePlay` 0x8254ff90 makes four
  `CSTATE_TrafficCar` states with `Setup(15)`.
  * Objects, in id order: 0 `SFXObj_TrafficEngine`, 1 `SFXObj_TrafficSkids`,
    2 `SFXObj_TrafficHorn`, 3 `SFXObj_TrafficWoosh`. The mix IO of each is
    `0x40040000 | instance << 11 | sfx << 4`.
  * Controllers, which `SortSFXCtl` sorts by index: 0 `SFXCTL_TrafficCarPhysics`, then the car,
    engine and exhaust positions (1-3). The positions are plain `SFXCTL_3DObjPos`, ported as
    `Obj3d` and sharing `Pos3dShared`. The mix IO of each is `0x60040000 | instance << 11 | ctl << 4`.
* The vehicle feed comes from `AudioPresentation::DoSimSubmit` 0x827fb2f8.
  * It calls `LWVehicleMan::GetVehiclesWithinRange` at the camera's (x, z), 40 m, at most ten
    cars, in the query's order. `GetVehicle` skips a missing id.
  * Each car becomes a 32-byte `VehicleData` (`WorldAudioState::vehicles`):
    * `mAcceleration` and `mSpeed` of `tVehicleStateOut` (`Vehicle`+0xD48);
    * the query distance;
    * `GetID`;
    * `mHornState`, kept as a signed 4-bit field (`insrwi`, read back with `srawi 28`);
    * `IsSkidding` (flag bit 3);
    * `GetEngineKey` 0x82d1e6d8. This is the collection key at layout byte 8 of the car's
      `livingworld_vehicle_characteristics`. That key is the `aud_traffic_engine` RefSpec.
  * The core facade `SelectedLiveSession::audio_vehicles` builds this list. It adds the
    `cPresVehicle::GetLocalToWorldMatrix` rows, which are `None` until `CreatePresVehicle` has
    the car's recipe; this mirrors the null `GetPresVehicle`. `DoSimRestore` 0x827f5c60 only
    re-points `mVehicle`/`mVehicleCount`.
* `CSTATEMGR_TrafficCar::UpdateParams` 0x82552df0 works as follows.
  * `mLoadBuffer += t`. Nothing runs while it is below 4 (`blt`). It is never reset after
    `EnterGamePlay`.
  * `mNumVehiclesAround` is `mVehicleCount`. The first `IntClamp(count, 0, 4)` ids are collected.
  * An attached state whose `mTrafficCarStateID` is among them clears that slot. Otherwise it
    detaches.
  * Each remaining non-zero id attaches the first free state.
  * Then `CSTATEMGR_Base::UpdateParams` runs. Every state gets `t_DeltaTime = t`. An attached
    state then updates its physics, the three positions, and the engine, skids, horn and woosh
    objects, in that order.
* Attach (0x825516d0) runs `InitSFX` on every controller, then `LoadData`. `LoadData` runs each
  object's `InitSFX` (0x82515d80), which turns that object's output on.
* Detach (0x825516e0 / `CSTATE_Base::Detach` 0x82a7e350):
  * Each object runs `Destroy`, which releases its Csis instances. Its output goes off and its
    inputs are cleared.
  * Each controller only has its inputs cleared. Its output stays on.
* `ProcessUpdate` runs for every state, attached or not: the engine, the skids, then the horn.
  The woosh has none. A detached state holds no instances, so these do nothing.
* `SFXCTL_TrafficCarPhysics`: `InitSFX` 0x82510df8 turns the output on and zeroes every vector.
  `UpdateParams` 0x82519d70 does the following.
  * It finds the car with `FindVehicleByID` 0x82552d58 among the first `mNumVehiclesAround`. It
    needs the presentation matrix too; without the car or the matrix it returns.
  * Vectors:
    * forward = the unguarded normalize of matrix row 2;
    * position = row 3;
    * engine = position + forward, exhaust = position - forward (fused `vmaddfp` with ±1);
    * velocity = forward × speed.
  * It copies acceleration, speed, distance, horn state, skidding and the engine key.
  * Input 0 is the speed relative to the camera microphone, |velocity - mic0 velocity|. The
    `aud_general` 0x3213338D2D2C817F values bound it:
    * at most `max_relative_velocity_vehicle` (35);
    * changing by at most 0xAB85397C101B0752 (100/s) × t from `mLastRelativeSpeed`, which only
      the constructor clears.
  * The input is then `fctiwz(rel / max × 32767)`, clamped to 0..32767.
* `SFXObj_TrafficEngine` (`TRAFFIC_CAR` in `AEMS_TRAFFIC.csi`, 19 members):
  * `UpdateParams` 0x8252b1f8 runs on the first update after an attach.
    * It loads `aud_traffic_engine` (class 0x259095163B974174) from the engine key.
    * `car_ID` is layout i16 at byte 8. A 1 becomes 7 if `Random() % 100 < 33`, or 8 if it is
      below 66. A 3 becomes 6 if it is below 50.
    * The instance is created as {pitch_Offset 4096, loPAss 25000, fX_Dry 32767,
      car_ID clamped to 0..9}.
    * Min and max RPM come from layout floats 0 and 1, plus 15 accessors. `mDownShift` and
      `mUpShift` survive re-attaches.
  * `ProcessUpdate` 0x8252b548 runs in this order.
    1. Input 0 is `unit(speed × 0.05) × 32767`.
    2. `dts = DeltaTimeScaler × t_DeltaTime`. `kAddDeltaTime` is 0, so the old delta is not
       accumulated.
    3. `mRpmModulate` wobbles by ±`ModulationAmplitude`.
    4. The gear is the first i ≤ `NumGears` with speed < i × `SpeedDeltaPerGear`.
    5. The target is speed × max / gear speed + modulate, clamped to min..max with `!(>)`/`!(<)`.
    6. Rate limit by `RpmDropsPerSecond` × dts. Falling uses the upshift scaler, sets the gas
       target to `GasOffScaler` and `mUpShift = 3`. Rising uses the downshift scaler and sets
       `mDownShift = 3`.
    7. Gas follows its target at `GasSpeedScaler` × dts.
    8. Members:
       * rPM is 0..10000.
       * Volumes 3, 4 and 5 are multiplied by gas when `UseGasVolScaler` is set.
       * The camera-facing bias is `EngExhBias` × dot(norm(car - mic0 pos), norm(forward)).
         `volume_EXH = v4 - fctiwz(v4 × bias × -1/32767)`.
         `volume_ENG = v3 - fctiwz(v3 × -bias × -1/32767)`.
         The two by volumes are v5. All four are 0 unless `kEngineAudio` and `VolumeOn`.
       * fX_WET is output 9. The azimuths are outputs 0 and 1 (0..65536) and 2 (0..65535).
       * pitch_Offset is output 6 (0..16383) and loPAss output 7, each × gas when its flag is
         set. hiPass is output 8.
       * accel_Decel = accel × 3000. shift = +1 / -1 while the counters run down.
       * car_Speed = speed × 2000. rel_Speed = |mic1 vel - velocity| × 2000. Note mic1, the
         skater; this is native.
* `SFXObj_TrafficSkids` (`TRAFFIC_SKID`, 14 members):
  * It is created as {pitch 4096, LoPass 25000, Dry_FX 32767}.
  * `ProcessUpdate` 0x825161f0 sets these members:
    * skid volume (output 1) and squeak volume (output 2), when `kSkidAudio` is set;
    * wet (6), azimuth (0), pitch (3, 0..20000), low pass (4) and high pass (5);
    * brake = skidding;
    * speed × 2000 and accel × 3000.
* `SFXObj_TrafficHorn` (`TRAFFIC_HORN`, 12 members):
  * `UpdateParams` 0x82515ea0 creates the horn with `horn_ID = Random() % 9`.
  * While the horn state is 6 (`Alarm`) it creates `c_car_alarm` (`Sk8_Emitters_Project.csi`)
    with `car_alarm_ID = Random() & 3`. Otherwise it releases the alarm.
  * `ProcessUpdate` 0x8252c070, horn state 6:
    * input 0 = 0 and horn_On 0;
    * the alarm gets volume 32767, dry (6), wet (10), azimuth (5), pitch (7, 0..8192) and low
      pass (9).
  * `ProcessUpdate`, any other horn state:
    * A new non-zero state reloads `mEngineAttrib` from the engine key. A layout `car_ID` of 0
      silences the horn.
    * input 0 = 32767 or 0.
    * Members:
      * volume 32767, dry (1, when `kHornAudio`), wet (8) and horn_On clamped to 0..6;
      * distance = |car - mic1 pos| × 1000, speed × 2000 and rel speed |mic1 vel - vel| × 2000;
      * azimuth (0), pitch (2), low pass (3) and high pass (4).
* `SFXObj_TrafficWoosh`, `CheckForWhooshes` 0x8252c868, uses the camera microphone.
  * It needs a distance of at most layout float 0 of `aud_whooshby` "vehicle" 0x39FC10A1DEBD34F8
    (6 m), and a relative speed of at least element 2 of 0x79B8671F26F9A0DD (18.5).
  * Then it calls `PlayWhooshSound` with that key and the car's position (`WhooshManager::play`).
  * After a whoosh plays, `mPlayedWhoosh` holds while the distance stays below the radius. The
    flag survives a detach.
* The `Audio_Traffic` cvars keep their registered values: `kAddDeltaTime` 0, `kEngineAudio`,
  `kHornAudio` and `kSkidAudio` 1.
* Loading is `AudioSystem::LoadVehiclesData` 0x824fd868.
  * `SecondaryLoad` asks for it, and `Update` loads it once no crowd banks are loaded. The call
    itself unloads crowds first.
  * The banks, in `AsyncLoadAt` order, are all from `vehicle.big` except the last:
    * `C00_heavy01`, `C01_family01`, `C03_sports01`, `C04_taxi01`, `C05_truck01`,
      `C06_sports02`, `C07_family02` and `C08_family03` `.abk`;
    * `Traffic_Horn.abk` and `Traffic_Skid.abk`;
    * `car_alarms.abk`, from `emitters2.big`.
  * The port loads them at `EnterWorld` after the other banks (app `VEHICLE_BANKS`).
* The crowd/vehicle bank exclusivity is ported (`UnloadCrowdData`, `LoadCrowdsData` and both
  load statuses, `system_contest.rs`).
* Not ported: `DEBUG_TRAFFIC_CAR_CONNECTIONS`; the `xcore` asserts.

## Pedestrians (world/pedestrian.rs, world/pedestrian_sfx.rs)

* PEDESTRIAN is `eMAINMAPSTATES` 5. `CSTATEMGR_Pedestrian::EnterGamePlay` 0x825500c8 creates fifteen
  `CSTATE_Pedestrian`s with `Setup(3)`: objects 0 (`SFXObj_PedestrianSpeech`) and 1
  (`SFXObj_PedestrianSFX`, CreateObject 0x82545700), and the controllers they ask for, sorted by
  index: `SFXCTL_PedestrianPhysics` (0, CreateObject 0x8251a1c0) and `SFXCTL_3DPedestrianPos` (1, a
  plain `SFXCTL_3DObjPos`, CreateObject 0x8253e330). Mix IOs: objects 0x4005xxx0, controllers
  0x6005xxx0 (instance << 11, index << 4). The speech object is not created (see the end).
* Input: `AudioPresentation::DoSimSubmit` 0x827fb2f8 asks `LWPedestrianMan::GetPedestriansWithinRange`
  0x82e9ab88 for at most 30 pedestrians within 50 m of the game camera's (x, z) (`BasicCamera::GetPosition`,
  permuted `<0,6,0,0>`), in the KD-tree's order (`KDTree::FindNear` 0x82d28c00, clamped to 40). A
  pedestrian whose core is not initialized (`LWCommonCoreDataProxy::IsInitialized`) is skipped. Each
  16-byte `PedestrianData` packs `PedStateOut` (`mState`, `mLength`, `mTopic`,
  `mTargetGreetCharacter`, the flag byte at 0x14: right foot 0x10, left 0x20, player collision 0x40,
  stopped 0x80), `PedStateOut::GetAudioCharacter` 0x82e78490 (`livingworld_models` layout+4), the
  distance and the entity id; both foot surfaces are `GetAudioMaterial(eAudioSurfaceType_AsphaltSmooth)`
  (0). The core's `SelectedLiveSession::audio_pedestrians` reproduces this; the app packs it with
  `PedestrianData::from_state_out` into `WorldAudioState::pedestrians`, with the pres pedestrian's
  `GetLocalToWorldMatrix` rows 2 and 3 (`core.transform`; `None` without a presentation).
* `CSTATEMGR_Pedestrian::UpdateParams` 0x825530d0: `kNumPedestrians` (-1: all) pedestrians, none in
  Point2Point, Death and Spot races; rows for the first 15 (clamped). A row needs an id and a
  character below 100; it plays SFX while its index is below `aud_speech` "default"
  0x53364DFA09A499DD (3). `type` 0x492964E71634DA6D and `shoetype` 0x871BDC669F2B1844 come from the
  character's `aud_pedestrian` (class 0x2AAAAB336154DF15) collection, whose key is byte 40 of the
  character's 0x38-byte entry in `aud_speech` "default" 0x60CA5F2E959EC361
  (`Sk8SpeechSystem::GetSpeechCharacteristicsKey` 0x824ec7a8; `mSpeechAttrib` is "default").
  Attached states in list order: a row with the same id gives it the state, feet, surfaces and SFX
  flag (and attaches nothing); otherwise `CSTATE_Base::Detach`. Each remaining row takes the first
  free state (`GetFreeState`); a security guard (`type` 0x40) first takes guard character
  `75 + rw::math::Random() % 5` and its `type`/`shoetype` (work boots, 3). Then
  `CSTATEMGR_Base::UpdateParams` updates every state: physics, position, then the SFX object.
* Attach (`CSTATE_Pedestrian::Attach` 0x82551728, `CSTATE_Base::Attach`): the physics controller's
  `InitSFX` 0x82511108 turns its output on and clears its vectors, but keeps the speed, the feet and the
  terrain of the state's previous pedestrian; the position controller and the SFX object turn their
  outputs on. Detach: the SFX object's `Detach` is `Destroy` 0x8252d418 (both `livingword_footstep`
  instances right first, then the cloth splices left first), output off and inputs cleared; the
  controllers' inputs are cleared.
* `SFXCTL_PedestrianPhysics::UpdateParams` 0x8251a2d8 (nothing without a pres pedestrian): the
  matrix's position and normalised facing (unguarded), velocity `(pos - prev) * recip(t)` (zero for
  `t <= 0`), speed the mean of this and the last velocity's lengths. A foot touching down records the
  height it touched at; terrain is flat unless `|right - left| * 100` exceeds
  `aud_general_footstep` 0xFEA50103D9637FAA (1 cm): the touching foot (right first) reports up when
  it is the higher one, stairs or incline from `rw::math::Random() & 1`, drawn every update.
* `SFXObj_PedestrianSFX::UpdateParams` 0x82545820: AEMS speed = the footstep speed graph
  0x90B47430C4ED2CCC (16 points, `fctiwz`) of the physics speed; heels from the state (toes never).
  While it plays SFX: `CheckForFootSteps` 0x8252d4f0 (surface Concrete_Rough when none; with
  `kEnableFootSteps` the missing `livingword_footstep` instances, left first, `Speed` 0 on the left
  and 1000 on the right, notch Q from 0x1C237A44FB9722C8; a new step's `AdjustFilterParameters`,
  right first), `CheckForCloth` 0x8252d978 (a new step replaces that foot's `sk82_cloth_foley`
  splice, walk/run/sprint by `aud_general_clothing` speeds 4.7/3.3, on the master mix, spread 0) and
  `CheckForWhooshes` 0x8252e6f8 (within the `aud_whooshby_pedestrian` layout distance, 5 m, of the
  camera microphone and at least its speed element 2, 17, relative to it: `PlayWhooshSound` with key
  0xAD8DE7E5DA126B8A at the pedestrian; armed again beyond the distance).
* `ProcessUpdate` 0x82545978: `ProcessFootSteps` 0x8252dd20 (while playing SFX) writes both feet's
  members: dry volume output 2 for a guard else 1, wet 9, low/high pass 7/8, azimuth 0, pitch 5, side
  step in the avoid states (4, 5), stumble after a knockdown (6, 7), the shoe type, footstep surface,
  terrain, volumes 0x62A2E64238934734 and the notches; then `UpdateCloth` 0x8252db58: volume output 4
  for a guard else 3, pitch 5, azimuth 0, shared parameters that `Splice::Update` scales in place.
* The `SFXCTL_3DObjPos` statics are the manager's own copy: `Generate3DParams` writes every value it
  reads in the same call. `UpdateMixerOutputs` and the physics controller's `Detach` are no-ops.
* Data: `LoadFoleyData` 0x824f0bf0 (`Bodyslide.abk`, `Foley_Cloth.abk`, `fstep_livingworld.abk`,
  `fstep_skateshoe1_sm.abk`), `SK8_AEMS_Foley.csi` and `sk82_cloth.bnk` were already loaded.
* Not ported: `SFXObj_PedestrianSpeech` (`UpdateSpeech` 0x82545eb0, `UpdatePedVoiceInfo` 0x825459c8,
  `ProximitySpeech` 0x8252ed88, `UpdateCellPhoneRing` 0x8252ead0) needs `Sk8SpeechSystem`. It reads the
  `PedAudioState` fields only it uses: `grunt_index`/`charSpchId` (aud_pedestrian layout words 0/2),
  `playSpeech` (distance below `aud_speech` 0x7F0FFDF7A90577C6, 50 m), `lw_class` (the greet target's
  0x02D9BFDA94A8B35E), the `mc_`/`cm_`/`an_` ids, `topic` (`1 << (topic - 1)`), `wasPlayerCollision`,
  `distance`, `volume_factor` 0x2087A3290483BB4F and `near_far_distance` 0xA27215A909135B62.

## Emitters (world/emitter.rs, world/emitter_system.rs, world/emitter_assets.rs)

* `Sk8EmitterSystem` (`AudioSystem::mpEmitterSystem`) over the base `EmitterSystem`
  (0x82a81100-0x82a83ae8). The `AudioSystem` constructor gives it five 0x64000-byte swap slots
  and one 0x7800-byte DLC slot. `Initialize` loads it after `emitter_utility.abk` and before
  `LoadInGameData`'s banks: `EnterGamePlay` creates the `C_emitter_utility`, `LoadGameFiles`
  0x824e9e00 reads `sfx.ems`, `reverb.ems`, `music.ems`, `speakers.ems` and `crowds.ems` (654,
  419, 80, 128 and 36 volumes), then `EnableEmitterSwap(true)` runs.
* An `.ems` file is a big-endian count followed by 72-byte `tEmitterVolume`s: identifier, game
  state mask, pos, size[3], innerRadius, orientation, the `aud_emitter` key and a debug colour.
  `AddEmitterVolume` 0x82a81828 boxes each volume as a cube of its largest size in
  `cManagedOctTree<tEmitterVolume*,1500>`. `GetCollisions` sorts the overlapping entries, so the
  port tests the boxes in entry order. An entry is the file order, or the last freed index.
* `Update` 0x82a83598 runs after the state managers' `ProcessUpdate` (group A) at microphone
  0's position. The random emitters update first, then the volume unloads. Every status is
  untouched, then `SearchOctTree` looks inside the camera's one-unit box. `CheckVolume`
  0x82a82db0 skips volumes of inactive game states. A sphere (equal sizes) uses
  distance/radius. Any other volume is an ellipsoid: along `orientation`, along
  `normalize(cross(up, o))` (size[2]) and along their normalised cross product (size[1]). Both
  axes come from the native `vnmsubfp` permute sequence with no zero guard, so an upright
  orientation gives NaN and is never inside. NaN or >= 1 is outside. `innerRadius` maps
  [inner, 1) to [0, 1) (`fmadds`), and inside it the distance is 0.
* `UpdateOrAddToVolumeList` 0x82a82910 touches a known status, except an AEMS one while the swap
  is not enabled. A new status gets a key: `GetKeyFromVolume` 0x824f88e8 draws one of the
  volume's `RefSpec` alternatives (draw 0 keeps the volume's own key). A new AEMS status shares
  the slot of a status with the same key, or loads `emitters.big|<filename>` into a free slot
  (`FindFreeSlot` searches backwards and prefers an unloaded slot that last held the file). If
  no status is created, the touch lands on the last status the list search read: that status
  stays and takes this volume's distance (native register reuse).
* `RemoveUntouched` 0x82a81fd8 frees non-AEMS statuses. An AEMS one detaches its state, then
  leaves if another status shares its key and slot. Otherwise it unloads the slot without
  unresolving: the asset empties at once and the bank stays added to AEMS. `UpdateVolumeUnloads`
  frees it once the slot is empty. `UpdateVolumeList` 0x82a818f0 plays each loaded AEMS volume
  on a free EMITTER state (`PlayVolume` 0x824f89d0: `aemsID`, volume x
  `ConvertDistanceToVolume`, quadratic or linear by attribute 0xF209C093_F40A4CCC, Nicotine
  azimuth, at the volume's position). After that, `UpdateVolume` 0x824f8ba8 keeps the level
  current.
* Random emitters (`ChangeEmitters` 0x82a82708) use `AudioSystem::GetEmitterKey`: the
  presentation's `GlobalData.emitterKey`, from WorldPainter layer 8, and none in races. The
  `aud_wp_emitters` list gives `RefSpec`s and weights (default 10). Two stay loaded, drawn by
  weight among the unloaded ones. Each plays when `Random()%1000` thousandths of
  `min_time..max_time` have passed (`fmadds`). The emitter that has waited longest plays at a
  random azimuth, with its volume drawn in 1/scale steps between `min_volume` and
  `max_volume`, and expires after `duration`. A detached state unloads its emitter with
  unresolve.
* `SFXObj_Emitter` (5 states, IO 0x40060000 | instance << 11, controller 0x6006...): `InitSFX`
  0x82516e58 creates `C_emitter` {0x7FFF, 0, 0, 0, 4096, 25000, 0, 0, aemsID clamped 0..500}.
  `ProcessUpdate` 0x825307b0 reads mixer outputs 4/8/5/6 (volume) or 2/7/1/3 (random), scaled by
  the info's volume. The azimuth is external or comes from output 0. The `C_emitter` module of
  every added `emitters.big` bank is instanced, and only the one whose ID matches plays. This was
  checked with an AEMS probe on `bird_calls_crows.abk` and `boat_horns.abk`: IDs 87 and 150 play,
  each with one silent instance of the other bank's module.
* `AudioAssetManager` swap slots (`emitter_assets.rs`) keep native semantics:
  `AsyncLoadToSwapSlot` re-marks a resolved slot with the same file as Loaded without a read. A
  different file first runs `BeginRemoveModuleBank` on the old bank, and the read waits for
  `IsModuleBankRemoved`. Unresolving unloads go through `mUnloadList`. In the port a read
  completes on the `Update` after it starts. The bank removal (`RemoveModuleBankHandler`) runs
  on the next audio frame (`Aems::timer_update`).
* `GetCurrentReverbVolume` 0x82a81c80 returns the first reverb status other than the excluded
  one, provided its `reverbType` is in `verbHash` (24 keys). `UpdateMusicVolumes` keeps
  `curMusicVolume` for `SFXObj_Music`, which is not ported.
* Not ported or idle: the game states (0x4000 `damActive` in single player and 0x8000
  `monsterConstructionActive` come from `GlobalData`; the rest come from challenges), so only
  state-0 volumes are active. Also not ported: the DLC data (`AsyncLoadDLCData` etc. are ported
  but no additional location key exists), `SetDynamicCameraMode`/`GetRenderAudioSpheres`
  (debug), `ExitGamePlay` (no exit path) and the chase-music silencing of emitter keys.

## Reverb volume transitions (reverb.rs)

* `UpdateParams` 0x8254e1e0 with no transition running:
  * Outside the reverb volumes, a new location key starts a time transition.
  * Entering a volume starts a distance transition (`RVRB_TRANS_DIST`) to its `reverbType`, with
    the sphere slot as the target.
  * With two volumes, a sphere-to-sphere transition starts (same key: the target becomes the
    active slot).
  * Leaving the centre of the only volume starts a distance transition back to the location key.
* A running transition advances by `mrvrb[target].type`: time, `ProcDistTrans` 0x82546c68, or
  `ProcSprhSphrTrans` 0x82546e18 (>= 3 cancels).
* `ProcDistTrans`: the followed volume stays current, or is replaced by one of the same key.
  Its distance becomes the sphere slot's `current`. At 0 the sphere slot wins, otherwise
  `PanTransition` 0x82531ca0 runs. Losing the volume runs `ResetDistTrans` 0x82531b88: the
  sphere slot's side wins at once and the volumes are dropped.
* `PanTransition`: the target fades to `1 - current` and the other slot to `current`. The
  target's filters pan by the volume's bearing from the local board (`mLocal` boardPos/boardLook
  x-z): `acos` of the normalised dot (clamped, x 180 x 1/pi), signed by the cross product. Left
  is `current * (angle - 270 or angle + 90) + 270` and right is `90 - current * (90 - angle)`,
  both fused and wrapped to 0..360. With no local skater it runs `InitSFX`.
* `mpevs` can outlive a status that the emitter system frees. Natively the freed memory is still
  read, and the port reads the last values of the 16 most recently freed statuses.

## Crowds and speakers (world/crowd.rs, world/speaker.rs)

* Both are driven by mission audio:
  * `PlayMissionSFXImp` 0x824fedbc loads the crowd banks (`LoadCrowdsData`, after disabling the
    emitter swap whose memory they share) and sets the multi-speaker mode.
  * `TriggerMissionMusicImp` 0x824ff234 starts the crowds.
  * Crowd and speaker volumes need challenge game states.
* Crowds are ported (`world/crowd.rs`, `system_contest.rs`):
  * `PlayMissionSFX` contest load and unload, the crowd banks of `crowds.big` (asset bank 14),
    `mbPlayArenaOnLoad`, `TriggerMissionMusic` (contest and race) and `StartCrowds`.
  * The five `CSTATE_Crowd`s with `SFXObj_Crowd` and `SFXCTL_3DCrowdPos`, attached to the
    nearest spectator groups (`CacheCrowdPodInfos`, thirty within 1000).
  * `PlayCrowdSound` from `CheckForScore` (the local skater's score change and bails).
* With no challenge, `CSTATEMGR_Crowd`'s key is 0 (`PlayCrowdSound` returns at once) and
  `mStartCrowds` is false.
* Speakers are ported (`world/speaker.rs`). A contest load sets `CSTATEMGR_Speaker`'s
  multi-speaker mode off line; its next `UpdateParams` 0x82557d58:
  * frees the speech characters' delay voices and builds two `MultiSpeaker`s (speech, music),
    each a `SubMix` sending to four `Speaker` voices (gain, high pass, delay, low pass, notch,
    pan, send to the speech or music master; `aud_general` "multispeaker" sets the delays);
  * connects the speech voices instance's stream 0 to the speech `MultiSpeaker`;
  * then matches the first four speaker volumes to the four `CSTATE_Speaker`s by identifier.
* Each attached speaker: `SFXCTL_SpeakerPhysics` delays each type by the camera's distance over
  the speed of sound (held for "multispeaker"'s update count) and sets its pan distance and
  nicotine distance; `SFXObj_Speaker` sets the gain, filters (a large arena's own outputs) and
  azimuth notch from the mixer and the volume's `aud_emitter` graphs.
* The contest's unload clears the mode: the speech stops and returns to the speech master, the
  states detach (their speakers silenced), the voices are freed and the delay voices return.
* A released `SubMix` unlinks every `Send` and `Route` still connected to it
  (`SubMix::ReleaseEvent` 0x82bfc2d8), as when the music still routes to a freed `MultiSpeaker`.

## Emitter host inputs

* `WorldAudioState::emitter_key`: `SelectedLiveSession::audio_emitter_key` (WorldPainter layer
  8 at the local skater's hips x-z, as for layers 7 and 10).
* `AemsFiles::emitters`: the five `.ems` files and every `emitters.big` bank.

## Dynamic objects, whooshes, world instances and NIS characters

`world/dynamic_object.rs` (+ `dynamic_object/dynamic.rs`, `dynamic_object/moving.rs`),
`world/whoosh.rs`, `world/object_instance.rs`, `world/nis_character.rs`: states 8, 10, 11 and 12.

### DYNAMICOBJECT (8)

* `CSTATEMGR_DynamicObject::UpdateParams` 0x825544a0 takes the first five entries of
  `WorldAudioState::dynamic_objects` (`AudioPresentation::mDynamicObject`). When none of them is
  grabbed, a grabbed object further down the list replaces the fifth. States whose object is no
  longer listed detach. Free states attach to the new objects. Then every state updates
  (`CSTATEMGR_Base::UpdateParams`).
* State update order is `SFXCTL_3DDynamicObjectPos`, then `SFXCTL_DynamicObjectPhysics`, then
  `SFXObj_Dynamic`, then `SFXObj_Moving`. `SortSFXCtl` 0x82a7de80 orders the controllers by
  index, so the position controller reads last update's physics position and velocity. On the
  first update after an attach it reads the zeros `InitSFX` left.
* `SFXCTL_DynamicObjectPhysics::UpdateParams` 0x8253e4c8:
  * Position comes from the presentation bounds centre, or from the local skater while grabbed.
  * Speed is |velocity|. While grabbed, it is raised to |angular velocity y|.
  * `mbIsBeingMoved` uses a 0.2 m/s threshold and is dropped while another DMO is grabbed.
  * Move type is push, or pull when velocity . skater look < 0.
  * Primary and secondary `aud_dynamicobject` keys and volumes come from the
    `livingworld_dynamicobject_characteristics` collection. The sliding layout is at 0x40/0x28/0x60/0x58;
    otherwise they come from the `RefSpec`/float attributes. Sliding is off when the no-slide bool
    is set, or when layout byte 0x7C is set and the object is on its side.
  * The moving speed is smoothed by `aud_dynamicobject` word 76 and clamped with fsel.
  * `onGround` is debounced over `(int)(10 / max(gameSpeed, 0.1))` updates.
  * The state key is the primary key while grabbed.
  * Mixer input 0 is `CalcRotSpeed`.
* `SFXObj_Dynamic`:
  * Plays `c_dynamic_sliding_objects` / `c_dynamic_rolling_objects` once `CanPlayDynamic` sees the
    same non-zero key on two updates on the ground at a speed over 0.2. The sliding instance is
    only created at a non-zero AEMS speed.
  * Impacts go to `PlayCollisionSound` with `dmoCollision`: the collision type against the surface,
    the L3 duck of an L4, then the rattle. A rattle below its threshold keeps the type's force range,
    as the native out-parameters do. The manager's impacts attach collision states in the same
    update (`system.rs` plays `ctx.collisions` right after state 8).
  * Whoosh: see below.
* `SFXObj_Moving` handles a hand-moved object on the ground:
  * Sets the manager's key (`SetMoveableObjectKey`: notch/volume/pitch modulation attributes),
    move type (frequency shift) and km/h.
  * Starts `c_sliding_start` / `c_rolling_start`.
  * Asks the swap banks for `data\audio\splicerdmo.big|<Moveable_*.abk>` (AEMS) and the
    `aud_dynamicobjectrandom` `.bnk` (splicer bank 22).
  * Once loaded, plays `c_moveable_*_objects` and random splices.
  * `nicotine_input_1` gets the first angle crossfade value, as the native argument list does.
* The manager's two moveable voices are created on the first update: stage 3 `SubMix`, `Gain`,
  `FrequencyShiftSsb(0)`, 2x `PeakingIir2`, `Pan2D1` to 6 ch, then a `Send` to master. Their
  modulation runs in `ProcessUpdate`.
  * `UpdateNotchFrequency` is called with `MainNotch2` for both chains, as natively, so
    `mNotchAttrib[3].mCurFreqRand` stays 0.
  * `XMVectorSin` 0x824b0060 is ported bit-exact (`xm_sin`).
* `GetGrabbedObjectKey` 0x82550430 feeds `PhysicsInputs.grabbed_object_key`, read before state 8
  updates, so it is last update's key, as natively.

### WHOOSH (10), OBJECTINSTANCE (11)

* `CheckForWhoosh` (0x82533e30, 0x8253b250) uses the camera microphone:
  * Distance below `aud_whooshby` layout word 0, and speed at least the slowest of the three
    speeds, calls `PlayWhooshSound` 0x824f0b30, which attaches one of three whoosh states.
  * Leaving the distance re-arms it.
  * `SFXObj_ObjectInstance::mPlayedWhoosh` is only cleared by its constructor.
* `SFXObj_Whoosh`:
  * The speed band picks the splice index and delay (`GetWhooshIndex`).
  * After the delay it plays bank 21 (`Sk82_Whsh_Bys.bnk`) at `Volume` × output 1, pitch 2,
    azimuth 0, and detaches when the splice ends.
  * `InitSFX` runs at the next whoosh update. For DMO-triggered whooshes that is still the same
    update; for instance-triggered ones nothing reads the object in between.
* `CSTATEMGR_ObjectInstance::UpdateParams` 0x82558300 matches held instances by exact position and
  low key word; it only attaches instances with a key. The position controller never gets vectors.

### NISCHARACTER (12)

* Speech only: `SFXObj_NISCharacter` fills a `VoiceInfo` for `Sk8SpeechSystem`.
* `CSTATEMGR_NISCharacter::UpdateParams` 0x82558930 attaches on a change to `SCENE_TYPE_RACE_START`
  from `ScenePlayer` entities and speech character keys. Neither is ported, so no state attaches.
  The per-object code (voice info, position controller) is ported.

### Inputs and idle values

* `WorldAudioState::dynamic_objects` (`DynamicData`): the app has to fill it from `DoSimSubmit`'s
  DMO block:
  * `GetDynamicObjectsWithinRange(local skater x/z, 30)`;
  * `grabbed_object` = `session.audio_grabbed_dynamic_object() == Some(id)`;
  * velocity, characteristic key, part-0 angular velocity, part-0 strongest contact force,
    `GetAudioMaterial(surface)` and on-ground, |transform row 1 . y| < 0.3, and the pres bounds centre.

  Core has no `DynamicObjectStateOut` contact list yet. Until then, feed no contacts: force 0,
  `onGround` false. This silences the rolling, sliding and moving loops and the impacts. Empty
  (the default) keeps the state idle.
* `WorldAudioState::spacial_instances`: `SpatialInstanceManager::BoundingBoxTest` (±10 m, 10 results)
  is not ported in core. It stays empty.
* `WorldAudioState::dmo_swap_files`: `AudioAssetManager` path resolution. In this build
  `splicerdmo.big` holds only the eight `SK82_*_Randoms.bnk`. The `splicerdmo.big|Moveable_*.abk`
  paths `SFXObj_Moving` asks for do not resolve (`BigHandler::Open` searches only the named BIG),
  so natively the moveable loops never load. The starts and random splices do. The app fills it
  with `splicerdmo.big`'s entries by bare name, and loads the four `dynamicobject.big` banks of
  `SFXObj_Dynamic`'s constructor at `EnterWorld` (`DMO_BANKS`).
* Core accessor still to write (facade), per `DoSimSubmit` 0x827fb2f8:
  * Range query: `DmoObjectMan::try_objects_within_range([x, z], 30.0, 50)` at the local skater's
    `PhysOut` system-reckoning position (the camera position without one).
  * Per id, from `object(id)`: `velocity`, `model.characteristic_collection` (`dynamic_type`), and
    `on_side` = |transform row 1 . (0,1,0)| < 0.30000001.
  * `grabbed_object`: `audio_grabbed_dynamic_object() == Some(id)`.
  * Unported here, so idle: part-0 angular velocity (0); contacts (force 0, material 132,
    `on_ground` false); the bounds centre (`None`). `PresentationEntity::GetBoundsCenter`
    0x82e3dd30 returns a stored member whose writer is not traced.
  * Even partial, this list makes `GetGrabbedObjectKey` → HandGrabs (the grab material) native.
* Not ported:
  * The `MusicSystem` chase mode in `GetIsBeingMoved` (treated as not chasing).
  * `SkateSamplePlayerFactory` patches 512/2048, which route AEMS samples into the moveable voices
    and write `mSendInfo` (the voices' pan, 0 here).
  * `SNDAEMS_removemodulebank` on swap unload.

## MAIN objects: Bloom, HOM, Jitter, NIS volumes and moments (main_objects.rs, system.rs)

* MAIN `UpdateParams` order (creation order, `CSTATEMGR_Base::UpdateParams`): Master, Reverb,
  NIS, Pause, Bloom (9), VU, Challenge, HOM (12), Menu, Jitter (14). `ProcessUpdate`: Master,
  Reverb, NIS, Bloom; the others have none. Announcer (0), Music (1), CameraMan (3) and
  Speech (8) are the music and speech systems'; Speech's `ProcessUpdate` comes between NIS and
  Bloom.
* `SFXObj_Bloom` (IO 0x40000090). `UpdateParams` 0x82549fa0 branches to `CheckForBloom`
  0x8253b698:
  * The fade is `VDPresentationState::mOutData.sFadeInfo.mNormalizedFadeValue` when
    `bFadeInfo` is set and no scene plays (`mCurrentSceneInfo.sceneType == 0`), else 0. It is
    raised to `AudioSystem::mNISFadeAmount`.
  * It is stored as `mFadeAmount`; input 0 is `fctiwz(fade * 32767)` clamped to 0..0x7FFF.
  * A positive input creates `fade_to_white` once: {0, 0, 4096, 25000, 0, 0, 0x7FFF,
    clamp(fctiwz(fade * 100), 0, 100)}. Zero releases it.
  * `ProcessUpdate` 0x8253b690 branches to `UpdateBloom` 0x825186d8: member 0 (`Volume`)
    becomes output 0's volume and member 7 (`bloom_amount`) the fade, then `SetMemberData`.
  * `bFadeInfo`: `VDWriter::ReceiveFadeAmountMsg` 0x827fc758 keeps `cMsgFadeAmount` at writer
    +0x40, `WriteData` 0x827ff8c8 records it when it is not negative and resets it to -1, and
    `VDReader::ReadData` 0x827fdd78 sets `bFadeInfo` from that record (record 14). The senders
    are `cChallengeEffectsManager::Update` 0x82daa1a0 and challenge Lua. The session's
    `audio_fade_amount` is the career's `effects.fade_amount`.
  * `mNISFadeAmount` is written only by `BloomToWhiteTransitionEffect::Update` 0x827d55c0,
    with the value it gives the post-fx blocks' `bloomToWhiteValue`. The session's
    `audio_nis_fade_amount` is the career NIS transition's `bloom_to_white`.
* `SFXObj_HOM` (IO 0x400000C0). `UpdateParams` 0x8253c598:
  * input 0 is `mIsHallOfMeat`;
  * input 1 is `mSk8HUDState == eSk8HUDState_HOMResults` (9);
  * input 2 is `mCurrentSceneInfo.specialFlags == NIS_SPECIALFLAG_HALLOFMEAT` (3).

  In `MixMapSK8.mxb` input 0 triggers MAIN events 76 to 84 and 125 to 128.
* `SFXObj_Jitter` (IO 0x400000E0). `InitSFX` 0x8253ca38 turns the output on and reads
  `aud_general` 0x6F058488_8913307B's attribute 0xB66AAD95_7873A8B3: up to ten 20-byte
  jitterers {midpoint, range, max delta, min delta, multiplier}. Each starts at its midpoint
  with no delta.
  * `skater.vlt` has six: midpoint 16384, range 16383, multiplier 1, with max/min deltas
    29000/21000, 30000/20000, 31000/19000 and so on.
  * `Jitterer::Update` 0x82518858 draws one `Random()` per jitterer per update:
    1. step = (max - min) * ((Random() % 2001 - 1000) * 0.001f), pushed away from zero by min
       (`step - min` below zero, else `min + step`; -0.0 counts as positive);
    2. the delta gains the step and is clamped to ±max with `fsel`;
    3. a position past mid + range reflects (over = pos - upper, pos = upper - over, delta
       negated), and NaN takes this branch; one below mid - range reflects up;
    4. the position is clamped to the range with `fsel`.
  * `UpdateParams` 0x8254a0b0 sets input i to `fmadds(pos - mid, mult, mid)`, clamped to the
    range, then to 0..32767 (`fsel`; a NaN position gives the upper bound), `fctiwz`.
  * In `MixMapSK8.mxb` inputs 0 to 2 scale PLAYER mix controls 59, 61 and 73. Only 73 reaches a
    channel (player sub 2, 0xD0090102). With the inputs at 0, that control was scaled to 0.
* `SFXObj_NIS::ProcessUpdate` 0x82517348 keeps sixteen mixer values:
  * volumes: 0 scripted speech, 1 other, 2 story, 3 photo, 4 pro, 5 race, 6 flythrough, 7 OTS,
    8 contest, 9 film, 10 photographer, 11 cell phone, 12 NIS pro, 14 tutorial, 15 tutorial
    success;
  * pitch 13: scripted speech.

  `GetNISVol` 0x82547e80 picks the volume of `GetNISType`'s type (`mOtherNISVol` for none).
  `Sk8SpeechSystem::UpdateVoiceInfo` reads it through `AudioSystem::nis_vol`. `InitSFX` 0x82532ad0
  turns the output on and registers the object.
* NIS input 1 is `mbPausedForMovie` (+0x529), an `AudioSystem` field now.
  * `PauseForMovie` 0x824f0a18 sets it, and `ResumeAfterMovie` 0x824f0a98 clears it.
  * The movie player's `FrontEndState_MoviePlayer::OnEnter` and `OnLeave` call them, as do
    `FrontEndState_Credits`. Their `XMPOverrideBackgroundMusic` and `PATH_pause` of the music
    stream are the music system's.
  * `GetMovieVolume` 0x824e6808 is `mMovieVolume` (master output 8, stored by
    `SFXObj_Master::ProcessUpdate` through `SetMovieVolume`) times 1/32767.
  * Master output 9 is `SetLoadingScreenVolume` 0x824e6e60 (`mLoadScreenVolume` = value / 32767).
* `NotifyPickPhotoScreen` 0x824e6b24 sets `SFXObj_Challenge`'s +0x1C. Challenge input 5 then
  holds while the game speed is at most 0; a positive speed clears the flag.
* Audit of the existing MAIN objects against IDA:
  * Master, NIS (`GetNISType`'s mapping included), Pause, Challenge and Menu match.
  * Idle until the music and speech systems are ported: Master inputs 7 and 8
    (`MusicSystem::GetCurrentState` mode 5: 0x7FFF and 0x3FFF), 9 (mode 6), 10
    (`IsAMainCastCharacterSpeaking`) and 11 (`IsALivingWorldCharacterSpeaking`); VU inputs 0
    and 1 (`AudioCore::GetMusicVU`/`GetSpeechVU`).

### Hall of Meat bank (main_objects.rs HomBank, system.rs)

* `UpdateHOMBankStatus` 0x824f1650 runs in the world branch of `AudioSystem::Update`:
  * after `UpdatePresentationData` and `UpdateFEHudSplices`;
  * before `mNextHOMTime` counts down and the 3-2-1-go counter.
* Empty: `GetHOMBank` 0x824e6a08 picks one of `sHOMBankNames`
  (`data\audio\splicercollision.big|HOM_Set_1..5.bnk`). It counts the banks not yet loaded,
  resets all five when none is left, and takes the `Random() % count`-th unloaded one
  (`sHOMBankNames[0]` if the walk finds none). It marks it loaded and starts
  `AsyncLoadToSwapSlot` into a new 0x40000-byte swap bank as splice type 7; the state becomes
  Loading.
* Loading becomes Loaded once the read completes. In the port this happens on the next update,
  when the bank installs as `SPLICE_HOM`; a bank that cannot install stays Loading. Loaded
  records `mWasHallOfMeat`. Nothing writes Unloading.
* `PlayMenuSoundImp` plays an `fe` sound's HOM splice only with the bank loaded (the `hom_*` bone
  sounds, splices 0x18 to 0x1A). `UpdatePresentationData` 0x82504848 derives `mIsHallOfMeat`:
  * it needs the bank loaded, `GlobalData` flag bit 22, not in a replay under `kNoHOMInReplay`,
    and `mGameSpeed <= aud_general HOM_MinGameSpeed` or already Hall of Meat;
  * this derivation is not ported, so `WorldAudioState::hall_of_meat` stays the host's input.

### Moments: blooms and logos (`Sk8_moments.csi`, moments.big)

* `SecondaryLoad` 0x824fe178 ends with `LoadVehiclesData(1)` and `LoadMomentsData` 0x824f0e50.
  The latter `SyncLoad`s `fade_to_white.abk` (sfx 0xB), `killed_it.abk` (0xC) and
  `owned_it.abk` (0xD). The app loads them after the DMO banks and before the vehicle banks.
  `Sk8_moments.csi` declares `killedit_logo`, `fade_to_white`, `ownedit_logo` and
  `killedit_bloom`. Every class starts {Volume, Azimuth 0, Pitch 4096, LowPass 25000, HighPass 0,
  WetFX 0, DryFX 0x7FFF}; all but `ownedit_logo` add a value.
* `PlayNISSfx` 0x82504540 is called from `ICEUserChannelAudioEffect::FireEvent` 0x826d6648 for
  command 2:
  * id 1 replaces `mpKilledIt` with a `killedit_bloom` at `mKilledBloomVol` (master output 5);
  * id 4 replaces `mpOwnedIt` with a `killedit_bloom` (the same class) at `mOwnedBloomVol`
    (output 7);
  * ids 2 and 3 (killed) and 5 (owned) set the running bloom's `bloom_value`, and
    `SetMemberData`;
  * the value is always the id clamped to 1..5;
  * ids from 100 go to `PlayLoadScreenData` (see below).

  `NotifyNISPlaybackFinish` 0x824fe558 (from `Scene::Purge` and `Scene::UnBindToGame`) releases
  both blooms.
* `PlayMenuSoundImp` 0x8250c738: an `fe` sound with a logo type (attribute
  0xA4080FB6_5880E3C2) creates `ownedit_logo` for type 4, else `killedit_logo` with `logo_value`
  = the type clamped to 1..5.
  * The volume is `fctiwz(mOwnedLogoVol or mKilledLogoVol * sound volume)`, clamped.
  * The instance is released before the function returns; the modules play out and destroy
    themselves.
  * The mission speech it then asks for (1202 owned, 1207 killed, through
    `Sk8SpeechSystem::CheckForRaceWinner` and the cameraman, announcer and main-cast paths)
    belongs to the speech system.
  * Logo types in `skater.vlt`: `challenge_ots` 4, `killedit_intro` 1, `killedit_flourish` 2,
    `killedit_outro` 3. The killed-it and own-the-spot movies already send these sounds.
* `ICEUserChannelAudioEffect::UpdateFrameBegin` 0x826ce9f8 is the freeze channel's walk over
  channel 30:
  * every interval crossed fires `FireEvent(GetValueInt(132, i), GetValueInt(133, i))` once;
  * nothing fires on the first interval after a take change;
  * it shares `OnTakeStart` 0x826cb400 (previous interval 0xFFFF) and `OnTakeChange` 0x826d6e78.

  The core NIS manager keeps the fired ids, and the app passes them to
  `AudioSystem::play_nis_sfx`.
* Checked with `aems_probe` on the real `moments.big`, all four classes instanced from their
  banks:
  * `killedit_bloom` values 1 and 4: RMS 0.46, 6 and 4 players;
  * `killedit_logo` 1 and 3: RMS 0.098 and 0.021, the instance destroying itself within 2 s;
  * `ownedit_logo`: RMS 0.035;
  * `fade_to_white` at full volume: RMS 0.05.

  With create-then-release, as `play_logo` does, logos 1 to 4 still played for 1.69, 0.66, 0.73
  and 3.23 s, and no instance was left.

### Not ported

* Load screen audio. `FrontEndState_TipLoader` (FE state 0x51) is the trick-book load screen.
  * `SceneManagerImpl::PreloadTrickbookLoadScreenAnimsAndAudio` calls `LoadLoadScreenData`
    0x824fe648 with three ids and a callback. `InitLoadScreenData` 0x824e6b40 names 20 slots
    after `data\audio/loadscreen.big`'s 15 `.snr` entries, each with `atol` of its first three
    characters as the id (102 and 104 to 117). `LoadLoadScreenBank` 0x824f1938 `ASyncLoad`s the
    id's entry by name. `UpdateLoadScreenLoads` and `UpdateUnloadLoadScreenData` run in group A
    after the splice manager.
  * `PlayNISSfx` ids from 100 (`GetAudioIDFromTrickbookLoadScreenCameraTrack`) reach
    `PlayLoadScreenData` 0x824fe720: `StopLoadScreenData`, then a stage-5 voice (SndPlayer1,
    Rechannel, Resample, Gain at `mLoadScreenVolume`, Send to the master mix) plays the sample.
  * The port has no load screen, so none of this has a caller.
* NIS post. `QueueNISPost` 0x824f1158 and `PlayNISPost` 0x824f11c0 are
  `Sk8SpeechSystem::QueueNISSpeech`/`PlayNISSpeech` for `eSk8Character_SFXPost` (`post.big`
  and `fepost.big` streams); post id 1 also overrides the background music. That is the speech
  system's.
* `NotifyNISPlaybackStart` 0x824fe540 sets `mCurrentSceneInfo` and runs `CheckForRoundStart`.
  The host still owns `GameAudioState`'s scene type, special flags and replay flag; no caller
  of the start notification was found by vtable slot.

## Dynamic-object feed (skate2-core `audio_dynamic_objects`, living_world_dmo_objects.rs, living_world_scoring.rs; app game_audio.rs)

* `AudioPresentation::DoSimSubmit` 0x827fb2f8 calls `ILWDynamicObjectMan` vtable +0x10
  (`GetDynamicObjectsWithinRange` 0x82d3be78) with 30 m and at most 20 (`li r6, 0x14`; the
  callee's own clamp is 50).
  * The point starts as the camera position. A local skater replaces it with
    `PhysOut_SystemReckoning::mCentreOfMass` (`PhysOut` +0x24, +0x30). It is then permuted to
    (x, z).
  * `SelectedLiveSession::audio_dynamic_objects` makes the same query through
    `DmoObjectMan::try_objects_within_range`.
* Per id, in the query's order:
  * A null `GetDynamicObject` (vtable +0x18) skips the id, and so does a false
    `LWCommonCoreDataProxy::IsInitialized`. Every member of the port's entity map is initialized.
  * The 80-byte `DynamicData` record:

    | Offset | Field | Source |
    |---|---|---|
    | +0x00 | id | `GetID` |
    | +0x10 | velocity | `GetVelocity`: core data +0x90 |
    | +0x20 | `grabbedObject` | see below |
    | +0x28 | `dynamicType` | `GetCharacteristic()->GetCollection()` |
    | +0x30 | angular velocity | `GetPartAngularVelocity(0)` |
    | +0x40 | force | strongest contact |
    | +0x44 | `onGround` | strongest contact |
    | +0x48 | material | strongest contact |
    | +0x4C | `onSide` | transform |

  * `dynamicType`: the constructor sets `mpCharacteristic` to `ModelMan::GetModel(key)` + 0xCC,
    which is the model's `characteristic_collection`.
  * Angular velocity: the HiLOD world collision body's Part 0, `Part::m_body` (+0x4C), then
    `RigidBody::mOmega` (+0x30). The W lane is `mLeft`; the audio only takes 3-lane lengths, so
    the port leaves it 0.
  * `grabbedObject`: set when the local `PhysOut_OffBoard::mIsGrabbingObject` holds and
    `GetGrabSplineOwnerObjectID(mGrabSplineDesc.mObjectID)` is the record's id.
* Strongest contact: Part 0's list is copied and searched for the greatest `mCollisionForce`.
  * A candidate replaces the best unless it is at most the best (`fcmpu`/`ble`), so a NaN
    replaces.
  * With a best contact: its force, `GetAudioMaterial(mContactAudioSurfaceType)` (the app's
    `audio_material`) and `onGround` = 1.
  * Without one: force `f31` (0.0) and `onGround` = 0. The material stays unwritten; the app
    passes 0, which `SFXObj_Dynamic` reads only for an impact.
* `onSide` = `!(|row 1 . (0, 1, 0)| >= 0.3)`, using `vmsum3fp`, the sign cleared and `bge`. A NaN
  therefore counts as on its side.
* The contact list:
  * `DynamicObjectStateOut` (0x330 bytes) is `mObjectContactList` (a `fixed_vector` of per-Part
    `fixed_vector<tDynamicObjectContactInfo, 12>`) followed by `mpDynamicObject`.
  * `cLivingWorldCollisionBody::GetContacts` 0x82e50e60 calls `OnEntityCollision` 0x82d3ce88 for
    every contact in the body's contact-spy list. There is no filter, and the DMO callback returns
    0, so the walk never stops early.
* `OnEntityCollision` 0x82d3ce88:
  * Every owner first gets `NotifyMonitoringEntitiesOfCollision`.
  * An actor sets its scoring flag; that part was already ported. Groups 3, 6 and 4 also run
    `HandleSkeletonContact`.
  * Any other contact builds a `tDynamicObjectContactInfo`:
    * position from `pos`;
    * the normal;
    * force from the contact spy's +0x40;
    * `mCollisionForce` = |`relativeVel` . `normal`| (`vmsum3fp`, sign cleared);
    * the surface left undefined (0) and the entity id taken from the pedestrian, vehicle or
      DMO.
  * A contact without an owner instead takes the surface `surfaceID & 0x7f`, and surface class
    6 or 9 (`surfaceID >> 7 & 0x1f`) sets flag 4.
  * `AddContact` 0x82d35f38 keeps the first 12 contacts of each Part.
* In the port, `resolve_dmo_scoring_contacts` now runs both branches over the island contact list.
  `DynamicObject::part0_contacts` keeps Part 0's force and surface; nothing reads the other fields
  or Parts.
* `LWDynamicObjectMan::ResetCollisionFlags` 0x82d303b0 empties only Part 0's list
  (`part0.mpEnd = part0.mpBegin`, 0x82d30460..68). `reset_collision_flags` now clears
  `part0_contacts`.
* Bounds centre: `PresentationEntity::GetBoundsCenter` 0x82e3dd30 returns `m_globalBC` (+0x110).
  * `CalculateBoundsInfo` 0x827f2708 (from `InitFromModelInstances`/`InitFromRecipe`) sets
    `m_localBC = (max + min) * 0.5` and forces its W to 1. The RModel boxes store W = 0; all 93
    DMO render models were checked.
  * `SetLocalToWorldMatrix` 0x827efd08 computes `m_globalBC = row0 * c.x`, then fused
    `row1 * c.y`, `row2 * c.z` and `row3 * 1`.
  * `UpdateLivingWorldPresentationEntities` 0x828133e0 passes it the SLerp of the previous and
    current replay transforms. The port uses the core `mTransform`, the frame's end transform, as
    the renderer's `bounds_center()` does.
* Physics is unchanged:
  * `part0_contacts` and flag 4 are outputs only.
  * No port code reads flag 4; the readers test 0x02, 0x08, 0x10, 0x20 and 0x80.
  * The scoring flags are set as before.
* Structural differences:
  * An object whose collision body cannot be read is skipped. Natively,
    `GetPartAngularVelocity` asserts.
  * The bounds centre is always supplied. Natively it is absent until the pres entity exists.
* `WorldAudioState::spacial_instances`: after the DMO block, `DoSimSubmit` runs
  `SpatialInstanceManager::BoundingBoxTest` over the local skater's hips ±10 m, with 10 results.
  Core has no `SpatialInstanceManager`, so the list stays empty.

## EALayer3 (rwac/ealayer3/: mod.rs, core.rs, tables.rs, generated_transforms.rs; rwac/decoder.rs `CodecDecoder`)

* Codec selection: `SndPlayer1::StartRequest` 0x82bfb718 takes the factory GUID from
  `sDecoderGuidLut` 0x822af934, indexed by the 4-bit SNR codec: `[0, 0, 'P6B0', 'EXm0', 'Xas1',
  'EL31', 'L32P', 'L32S']`. Codecs 5, 6 and 7 are `EaLayer31Dec`, `EaLayer32PcmDec` and
  `EaLayer32SpikeDec`, whose `CreateInstanceEvent`s (0x82c170b0/b8/c0) call
  `EaLayer3DecBase::CreateInstance` 0x82c167e8 with version 0, 1 or 2. `rwac::decoder::CodecDecoder`
  is that choice: 3 is the XMA `Decoder`, 5 to 7 are `EaLayer3Decoder`, and any other codec fails
  the factory as before (PCM 'P6B0' and XAS 'Xas1' are not ported). `RwacTimerClient` 0x82bff0e0
  gives only codec 3 the one-frame start delay (`cmplwi r11, 3`).
* Game data: codec 5 only, mono. English, French and German main-cast speech at 44.1 kHz (MPEG-1
  granules) and French announcer speech at 22.05 kHz (MPEG-2): 22,074 `.sth` records. The NIS,
  post and load-screen streams are XMA.
* `DecoderDesc` 0x83012ea4: the last word's high half is the block size, 576 (XMA's is 0), so
  `Decoder::Decode` 0x82bdccb0 runs its block-based branch:
  * what is left of the last block is copied first (`AdvanceDecodeState` 0x82bdcc28);
  * then one `DecodeEvent` per block into the decoder's "Decoder block storage" (576 samples per
    channel), copied out up to the request.
  * `mDecodedSamplesAvailable` is a u16, so a failed `DecodeEvent` (-1) becomes 65535.
  * `Decoder::Feed` 0x82bdcb68 calls `Decoder::vftable`'s feed event, which is the empty `blr`
    0x825187c0 (`EaXmaDec` has its own).
* `EaLayer3DecBase` (0x60 bytes plus the core pointers): one `EALayer3Core` per channel pair (an
  odd count's last one is mono), each with "PolySynthHistoryF" and "MP3HybridHistory" (2304 bytes
  per channel, zeroed).
  * `DecodeEvent` 0x82c15e00 takes the next prepared request when the last is used up. A new feed
    (feed type 0) sets `mLatency` 1105 and `mNewFeed`. A `decoderSkip` runs `SkipBlocks` 0x82c12e18.
  * It then decodes one granule per core until one has samples past the latency and skip, and
    scales them by `__real_38000000` (2^-15).
  * `SkipBlocks` drops whole granules by the seek data's 16-bit sizes: version 0 has one per
    granule; versions 1 and 2 have one per ten blocks, then the block headers' sizes. The latency
    rules depend on `mNewFeed` and version 1.
* `DecodeGranule` 0x82c15aa8, version 0:
  * a flag byte, then the granule (`Open` on a new feed, else only the bit reader moves);
  * the latency, then the skip, taken off the front (`memmove`);
  * a flag of 0xEE puts a PCM block after the granule (`DecodeSpecialBlock` 0x82c08230: u32 count,
    u32 offset, planar big-endian s16 per channel, written at latency + offset).
* `DecodeGranule`, versions 1 and 2: `EaLayer32Block::Read` 0x82c08418.
  * The first u16: bit 15 is the PCM header, bit 14 the channels, 12 bits the size.
  * The PCM header word: offset mode (2 bits), offset samples (10), PCM samples (10), granule
    size (10).
  * Samples by offset mode, with a granule / without one:
    * ignore: 576 - offset / PCM samples;
    * preserve: 576 / PCM samples;
    * mute: 576 / 2 × PCM samples.
  * `DecodePcm` 0x82c07ef0 writes from the offset: interleaved s16 for two channels, otherwise
    the first channel only.
* `EALayer3Core` (core.rs, `CMpegBase` / `CMpegLayer3Base` code):
  * `Open` 0x82c13a30: `CMpegBase::ProcessHeader` 0x82c0de98 gets the granule's first byte, which
    never carries the 0xFFE sync, so it returns at once. The synthesis is reset and the band
    offsets set to 1.
  * `Decode` 0x82c13aa8 reads, in order:
    * the header byte (`ProcessEALayer3Header` 0x82c09880: version 2 bits, rate 2, mode 2,
      extension 2);
    * the granule bit, `GetSideInfo` 0x82c084f0, then per channel the scale factors, Huffman and
      `Dequantize`;
    * then `Stereo`, and per channel `Reorder` (short blocks) or a buffer swap, `AntiAlias`,
      `ReorderForVectoring` 0x82c070d0, `Hybrid`, `FrequencyInversionX4` 0x82c07838,
      `HELPER_ReorderForFPolySynth` 0x82b93510 and 18 `PolySynth` calls.
* Scale factors: `GetScaleFactors` 0x82c089a8 (MPEG-1, `scfsi` keeps the last granule's for
  granule 1), `GetLsfScaleData` 0x82c093c0 and `GetLsfScaleFactors` 0x82c096f0.
  * LSF side info always fills granule 0, but `Decode` uses `[ch][gr]` with the stream's granule
    bit, so an LSF granule must say 0.
* `DecodeHuffman` 0x82c12f98 dequantises as it decodes. Each value is ±2^((globalGain - 210)/4)
  times |x|^(4/3). Values of 32 and more go 32 at a time through `SToPowerOf4over3` 0x82c09bf0, a
  9-segment linear fit by bit length (`fmuls` then `fadds`; 1000 gives 9694.8 instead of 10000), so
  the output is not a reference MP3 decoder's.
  * Refills stop where the held bits, counted from the pair's start, reach the part 2/3 end.
  * A count1 overrun rewinds to the end, keeping the stale register when the rest is byte
    aligned.
  * A skip of 32 bits or more loses up to 7 bits: `slw`/`srw` by 32 or more give 0.
* `Dequantize` 0x82c0a050 scales each band by `gTwoToNegativeQuarterPower + 45` (2^((45 - n)/4)),
  skipping gains of 1.
* `Stereo` 0x82c0a350 is dist10's `III_stereo` with M/S `(l ± r) × HELPER_cos4_0` and MPEG-1
  intensity `t = l / (ratio + 1)`, `r = t`, `l = ratio × t` (`fdivs`).
  * Band 12's copy writes band 11's lines from band 10's start, and the long-block scan starts
    from line 0.
  * `TAN12_0` is followed by `io_0`, so an `is_pos` of 16 to 31 (a 5-bit LSF scale factor that an
    MPEG-1 granule kept) reads `io_0`.
* `Hybrid` 0x82c0ccf0 works in X4 bands (four subband lanes of 18 rows):
  * `Imdct36X1` 0x82c0c540 with `win_0[blockType]`, or `Imdct12X1` 0x82c07208;
  * then `OverlapAddX4` 0x82c0cb00;
  * a mixed block's first band has lanes 0 and 1 long (`win_0[0]`) and lanes 2 and 3 short.
* `PolySynth` 0x82c0dba8 is mpg123's `synth_1to1` over `dct64` 0x82c0ce90 and `decwin_0`. The
  fused order follows the asm, and the backward half's last term reads `window[0]`, not
  `window[-16]`.
* Float semantics are the asm's instruction for instruction, fused where fused. `fnmsubs` is
  `-(a·c - b)` negated after rounding, so an exact zero gives -0, unlike `crate::math::fnmsub`
  (`c - a·b`, +0). The module uses `-fmadd(a, c, -b)`. `dct64`, `Imdct36X1` and `Imdct12X1` have
  no branches; `generated_transforms.rs` is emitted from their operation traces, one line per
  instruction.
* Structural differences:
  * `Decode`'s stack buffers (two 576-float spectra and the 600-float work buffer, contiguous) are
    the decoder's scratch, shared by its cores. A big-values run past 576 lands in the next
    buffer, as natively.
  * Lines `Reorder` leaves unwritten keep the port's last values: 8 kHz short blocks (540-575) and
    mixed blocks whose `3 × s[3]` passes 36. Natively they are stack memory, which the frames of
    the calls between granules also overwrite, so their values are arbitrary. No game data is
    8 kHz.
  * Invalid streams: native writes past its buffers (a stereo granule on a mono core, region
    counts past band 22 giving a negative `XMemSet`, PCM past the block storage). The port drops
    those writes, reads past the data as 0, and clamps a sample rate index of 3.
  * `ScaleSamples` 0x82be2e28's VMX path (`vmulfp128`, which flushes denormals) is the plain
    multiply, as in `kernels.rs`.
  * NaN payloads follow the host; only overflowed corrupt data makes NaNs.

### Verification

* A PPC interpreter ran the debug executable's own decoder on the image's code and tables: the
  `DecoderFactory` layout, then `Decoder::Feed`, `GetSamplesRemaining` and `Decode`, with
  `memset`/`memmove`/`XMemCpy`/`ScaleSamples` stubbed. Its output was compared with the port's,
  bit for bit, under the `SndPlayer1` schedule (one request per chunk, at most 256 samples per
  `Decode`). All bit-exact:
  * two whole clips: English main cast `1002_37_burn_ambchat` (102,664 samples, 54 chunks) and
    French announcer `300_35_roll` (88,878 samples, MPEG-2 LSF);
  * 12 random records from the English, French and German speech;
  * 82 random streams: 1, 2 and 3 channels; versions 0, 1 and 2; MPEG-1, MPEG-2 and MPEG-2.5 at
    11/12 kHz; joint stereo with intensity and M/S; long, start/stop, short and mixed blocks; PCM
    blocks.
* The comparisons found and fixed the signed zero of `fnmsubs` and the `TAN12_0`/`io_0` adjacency.
* All 22,074 game records decode to their header's sample count with finite output.
* Through a voice: a codec 5 RAM sample (an English main-cast record with play type 0) played on
  SndPlayer1, Resample, Gain and Send into the mastering voice is audible for 111,730 of the
  expected 111,743 output samples (102,664 at 44.1 kHz), on the centre channel, then expels.
* Unit tests pin two synthetic streams (two-channel version 0 with PCM blocks, mono version 1) to
  the native output hashes.

## Music (music/, formats/pathfinder.rs)

`Sk8::Audio::MusicSystem` (over `MusicCore`, `musiccore\2.00.00`) drives EA Pathfinder 5.3
(`path\5.03.00-sk8`), which plays the `.mpf` node graphs out of the `*_Stream.mus` streams through
`PathTrackRWAudioCoreStream`. `Sk8AppMan::CreateAudioSingletons` 0x8272e4c0 constructs the music
system after the speech system and calls `MusicSystem::Initialize` 0x824e8c98:
`MusicCore::Initialize` 0x82a806e0 sets up the library, adds the `"pathfinder timer"`, installs
`PathLibProgressCB` 0x82a80690 as the song-progress callback (the event and action callbacks are the
empty `AIProcess::OnEnd`), and seeds Pathfinder with `rw::math::Random()`.

### Data (formats/pathfinder.rs)

* `data\audio/music/{world,game,ipod}.mpf` (`PATHFINDHEADER`, `'PFDx'`, version 5.3). Each holds one
  project:

  | File | projectID | Tracks | Events | Nodes (audio/logic) | Named variables |
  |---|---|---|---|---|---|
  | world.mpf | 0 | 1 | 9 | 4026 (3984/42) | resumenode, djsection, dj |
  | game.mpf | 1 | 1 | 25 | 2674 (2208/466) | numchasers, chaser, chasesection, resumenode, numracers, racer, racesection, nodeoffset, crashing, mainpart, currentnode, numreplays, replay, replaysection, bar0, savecontroller, onboardresume |
  | ipod.mpf | 2 | 2 | 2 | 1728 (1520/208) | ipodsection, ipod, replay |

  Bit-fields are allocated from the least significant bit of each big-endian word. A node is 16
  bytes (`index:16` is a 1-based sample, or a logic node: 0 entry, -1 repeat, -2 random control,
  -3 event), followed by 4-byte branches (`controlmin`, `controlmax` signed bytes, `dstnode`).
  Events are a 20-byte header and 12-byte actions.
* `*_Stream.mus` (`PATHMUSHEADER`, 40 bytes, then 28-byte `PATHNODEOFFSETS`). The stream cache is
  the first `maxaram` bytes (`PATHTRACKINFO`: world 173344, game 86768, ipod 45824).
  `FindSampleOffsets` 0x829ba4d0 maps a node's `PATHFINDSAMPLE.offset` to the sample's EAAC header
  in the cache (16-byte units) and its chunks in the file (128-byte units). The samples are 2-channel
  48 kHz EA-XMA (codec 3, play type 1) on the Format0 chunk parser.

### Pathfinder runtime (music/path.rs)

Ported against the decompile of the whole library (`PATH_*`, `PATHI_*`):

* Projects and handles: `PATH_addmapfile` 0x829bb2b0, `PATHI_switchproject` 0x829b84e0,
  `PATHI_switchvoice` 0x829b8560, `PATHI_gettrackptr` 0x829b8640. Handle bits 0-23 are tracks, 24-27
  projects (`0x01000000 << projectID`), 28-31 voices; the music system's tracks are 0x11000001
  (world), 0x12000001 (game), 0x14000001 (iPod), all three 0x17000001.
* Tracks: `PATH_createstreamtrack` 0x829c3a78 (`PATHI_createtrack` 0x829c2950, `PATHI_inittrack`
  0x829bca30, including its 11-character name), `PATH_createstreamimp` 0x829c2948
  (`PATH_createstreamimplementation` 0x829bc550, `PathToRWAudioCore::CreateStreamTrack` 0x829c2cb0,
  stream pool `'pth0'` of 0x40000 bytes), `PATH_attachstreamimp` 0x829bc730,
  `PATH_detachstreamimp` 0x829bc920, `PATH_cachestreamdata` 0x829bb5f8.
* Control: `PATH_gain` 0x829bce20, `PATH_setvolume` 0x829bd030, `PATH_pitchmult` 0x829bd1f8,
  `PATH_channelsvolume` 0x829bd388, `PATH_pause` 0x829c1d58, `PATH_stop` 0x829c1ef8 (`PATHI_stop`
  0x829bac80), `PATH_latency` 0x829bac08, `PATH_control` 0x829c1c18 (`PATHI_channelbranch`
  0x829bbf38), `PATH_setnamedvalue` 0x829b7c90, `PATH_pitchmultfade` 0x829c2bb8,
  `PATH_channelsvolumefade` 0x829c2bd0 (`PATHI_rampfunc` 0x829c2a20), `PATH_seedrandom` 0x829b8428
  and `PATHI_random` 0x829b8348.
* Events: `PATH_event` 0x829c2060, `PATH_clearallevents` 0x829c22a8, `PATHI_getevent` 0x829b6fd0,
  `PATHI_copyevent` 0x829b77f0 (the 4096-byte event heap), `PATHI_addevent` 0x829b7690 (action
  indents, the `GetMilliseconds` stamp), `PATHI_releaseevent` 0x829bad58, `PATHI_eventtakespriority`
  0x829baf10, `PATHI_serviceeventqueue` 0x829c4908, `PATHI_serviceevent` 0x829c4840,
  `PATHI_serviceaction` 0x829c3c48 (all 18 action types: conditions, wait time, wait beat, branch,
  fades, set value, event, event action, math, pause/resume, bank load), `PATHI_conditiondone`
  0x829b7030, `PATHI_restoretolastwhile` 0x829b70c0, `PATHI_getvalue` 0x829c30c8 (28 special
  values), `PATHI_setvalue` 0x829c1420, `PATHI_mainvoice` 0x829b8938.
* Nodes: `PATHI_nextnode` 0x829bbc28 (the repeat count is the control while the track repeats the
  node), `PATHI_pickclosestbranch` 0x829b82c8, `PATHI_routenode` 0x829bbea0, `PATHI_enternode`
  0x829c2418 (logic nodes act on the first node's track; every step restarts from the given
  control), `PATHI_queuenode` 0x829bc000, `PATHI_seeknextnode` 0x829c27c8, `PATHI_sampleoffset`
  0x829bb7b0, `PATHI_getchannelset` 0x829ba5f0, `PATHI_beatinfo` 0x829bb898 and
  `PATHI_calcwaitbeat` 0x829b8120.
* Status and fades: `PATHI_trackstatus` 0x829ba668 (`PATHI_getfadestats` 0x829b8cb0 answers with a
  free slot, so a playing track reads `PATHTRACK_FADING`), `PATHI_timeremaining` 0x829b8268,
  `PATHI_getmastertrack` 0x829b86d8, `PATHI_startfade` 0x829bd728, `PATHI_calcfade` 0x829bd5b8
  (`fnmsubs` and `fmadds` kept fused), `PATHI_updatefade` 0x829bda48, `PATHI_volume` 0x829bd528.
* Service: `PATHI_service` 0x829c4d10 (`PATHI_statusall` 0x829b8a88, `PATHI_serviceproject`
  0x829c4a50). `RWAudioCoreTimer::Update` 0x829c50e0 runs `PATHI_servicetimer` 0x829c4df8 (bank
  service 66) when the time of this call passes the last one by more than 0.01 s (compared in
  double), and `ServiceTaskThreadFunc` 0x829c4e90's task service (32) runs every 50 ms. Both skip
  while `Path::paused`.

Differences: `GetMilliseconds` (`PathToRWFileSystem` 0x829b8e38, a stopwatch) is the audio core's
clock, and the task service runs every 50 ms of that clock from the same timer rather than on its
own thread. Events are copied per project rather than shared through the heap; with one project per
map file the result is the same. When a branch or seek fails, `pfstate->eventindex` drops the
events the action queued while their stale queue slots stay until the next queue service; the port
drops them outright.

### Stream implementation (music/path_track.rs)

`PathTrackRWAudioCoreStream` (constructor 0x829c0bb0) and `PathTrackRWAudioCore`: `SetupPluginsConfig`
0x829be510 makes one stage 0 voice of `SndPlayer1` (3 requests), `Rechannel`, `Resample`, one `Route`
per channel set, the FX `Send` and the dry `Send`. `Play` 0x829c0f08 queues the node's sample (start
1 ms, the `.sns` offset, the cached header, the pool). `CheckStatus` 0x829c0e90/0x829bf418,
`GetPathStatus` 0x829bf7a0, `ReadyForNewRequest` 0x829b99e0, `StreamCache` 0x829ba3a8,
`GetStreamCache` 0x829ba450, `SetGain` 0x829b9a28, `SetVolume` 0x829b9b18, `SetDryLevel` 0x829b9c00,
`SetFXSendLevel` 0x829c0010, `SetPitchMult` 0x829c0288, `Pause` 0x829b9dd0, `StopPlaying`
0x829c1250, `SetChannelsVolume` 0x829bfc48, `SetChannelsPosition` 0x829bfe08, `AddChannelSet`
0x829b97d8, `RemoveChannelSet` 0x829b96c0, `RouteChannels` 0x829be930/0x829beff8 and
`SetupChannelVoice` 0x829beb28 (stage 1 `SubMix`, `Pan2D1`, `Send` per set). `SetStretchMult`
0x829b9d78 is the native stub (built without time stretch, returns -8).

### Music system (music/mod.rs)

* Modes: INIT 1, NONE 2, FE 3, ENVIRONMENT 4, CHASE 5, CHALLENGE 6, REPLAY 7, IPOD 8. Triggers:
  READY, INIT, RESET, PAUSE, RESUME, RETURN, STOP. A target state is set by the triggers and
  `UpdateState` 0x824f6318 moves the current state to it.
* `MusicSystem::Update` 0x82508400 runs `MusicCore::Update` 0x82a80920: `CheckConfigVars`
  0x82505db8, `UpdateMusicState` 0x82a7fc20 (`OnInit` 0x824f6760, `OnReset` 0x824f6840, `OnReturn`
  0x824e9610, `OnStop` 0x824e9618), the pause while the game state is replay or the pause menu, then
  `CheckPendingTasks` 0x82a80820 or `UpdateState`, then `UpdatePosition` 0x82507400, `UpdateVolume`
  0x82503540 and `UpdateDsp` 0x82506968. The first update with a stream implementation sets
  `kResetMusic` (`mbFirstInit`).
* Loading: `PrepareStream` 0x82a80ab0 queues `MusicAsyncLoadTask` (the map file through the asset
  manager, then `AddMapFile` 0x824e9248), `MusicCreateStreamTask` (`CreateStreamTrack` 0x82a7ff60 with
  `GetTrackInfo` 0x824e9038: latency `musicTweaks` word 3, word 2 for the game track; 3.5 s buffer;
  the first track makes the six-channel stream implementation) and `MusicAcquireStreamTask`
  (`SwitchStreamTrack` 0x82a80148: the implementation moves to the track). `StopStream` 0x82a7fa68,
  `ResetStream` 0x82a7fb08, `ResetDsp` 0x82500fd0. The map loads complete on the next update, as the
  asset manager's do.
* States: `CheckStateReady` 0x8250d1b0, `IsStateReallyReady` 0x824e8be8, `SaveReturnState`
  0x824e9348, `RestoreState` 0x824e9510, `MusicState::SetMode` 0x824e41f0,
  `MusicWorldData::Equals` 0x824e4180, `SetDefaultMode` 0x824f61e0.
* Triggers: `FrontEndTrigger` 0x82506408, `IPodTrigger` 0x824f6d98, `EnvironmentTrigger`
  0x824e97c8. `UpdateIPodState` 0x82506aa8 starts an iPod song with named values `ipod` and
  `resumenode` and event 0x0416D750; `UpdateEnvironmentState` 0x8250b100 starts a world playlist's
  song by its preset (layout word 7): 1 and 4 play the iPod song with `ipod` and event 0x0416D750,
  or `replay` and event 0x0437F4CB under the contest replay channel map; 3 plays the DJ song with
  `dj`, `resumenode` and event 0x015EEFA7. It then runs the interactive music
  (`InitializeInteractiveMusic` 0x825022f8, `UpdateInteractiveMusic` 0x82502390,
  `GetInteractiveLevel` 0x825028c0, `GetInteractiveKey` 0x824f7010), whose level is the world
  track's `PATH_control`.
* Playlists (`aud_playlists`): `SetPlayList` 0x82501148, `ChooseSong` 0x824f6530 (in order, shuffled,
  or shuffled on start; the 16 `PlayListHistory` entries), `ChooseStartSong` 0x82a7fe40,
  `ChooseNextSong` 0x82a7fe58, `GetPlayHistory` 0x82a7fe70, `GetCurrentSongID` 0x824e9178,
  `GetNumSongsInPlayList` 0x824e8d30, `FindSongIndex` 0x82a7f850 over `sPartIDs_ipod` (53) and
  `sPartIDs_dj_interactive` (22). The front end's playlists are the main menu (0xDE63C26D_A486B5C0,
  the constructor's `mCurrentPlayList`), Create A Skater (0x82BE3BCC_575FA538) and the music
  player's (0xFB6F3EEE_B355D7E6).
* World music: `UpdatePosition` keeps `mWorldSoundSource` on microphone 0 outside the environment.
  In it, `CheckEnvironmentZones` 0x82503168 takes the current music volume of `music.ems`
  (`Sk8EmitterSystem::GetCurrentMusicVolume`, `mAttribKeyMusicVolume`), its `aud_emitter` `dsp_info`
  and its playlist (layout word 7 is the channel map preset, words 2-3 the `aud_music`
  collection, `aux_dsp_info`), maps the stream channels and triggers the environment, then holds
  for 240 updates. `UpdateTrackPosition` 0x82502bf8 places channel sets 0 and 1 by the music
  object's azimuth and the volume distance and sets the three set volumes from the `aud_emitter`
  gains and curves (square roots by `vrsqrtefp` and two Newton steps, as `length_from_squared`).
* Channels: `MapStreamChannels` 0x82501310. The first call makes the two `MusicDspVoice`s and the
  channel sets 0-2 (six channels) and 3 (one). Every call removes the sets, sets the dry level
  (`kStraightToDac`, `kApplyDSP`), routes set 3 to `GlobalEffects`' "EnvSendSub" at volume 0
  (`kGlobalReverb`), then the preset: stereo (dry 127 into DSP voice 0), contest iPod (set 2 to
  the music master on line, else `CSTATEMGR_Speaker::GetMultiSpeakerSubmix(eSpeakerType_Music)`), DJ location (set 0 into
  DSP voice 0, set 1 with channels 0/2 to 4/3 into DSP voice 1, set 2 to the music master). Each
  set's send goes to the submix `SetDrySubmixPlugin` holds at the time.
* DSP (music/dsp.rs): `MusicDspVoice` `CreateDspSubMix` (stage 2: `SubMix`, `Delay` for voice 1,
  `PeakingIir2`, `HighPassIir2`, `LowPassIir2`, `Send`), `CheckAttribulator` (send, Q, peak gain and
  frequency, low pass with the nicotine LPF, `kTestLowPass` and `kGameSpeedLowPassScalar`, high pass,
  delay time, feedback), `SetDspInfoKey`, `SetReverbChannelSetGain`.
* Volume: `CalcVolume` 0x824f7dd8 (the music object's volume for `GetMusicVolType` 0x824e9cb0,
  times the song's, times the music volume's for a DJ environment), `CalcWetVolume` 0x824f7ff0,
  `GetNicotineLPF` 0x824f8078; `PATH_gain` follows in thousandths.
* `GetPitchRampTime` 0x824e8e88: PAUSE and RESUME ramp the pitch to 0 and back over it.
* `UpdateChyron` 0x825069f0 records a new iPod song in `MusicSystem::chyron`.
* `PauseForMovie` 0x824f0a18 and `ResumeAfterMovie` 0x824f0a98 pause the stream (resuming only if
  the music is on); there is no user music for `XMPOverrideBackgroundMusic`.
  `NotifyNISPlaybackFinish` 0x824fe558 starts with `MusicTrigger(ALL_RESUME)`.

### Front end

`AudioSystem::ReceiveFEMusicStateChangeMsg` 0x82508378 calls `FrontEndTrigger(FE_BEGIN, state)` for
the state `StateManagerFrontEnd::ChangeFEMusicState` 0x82656f78 posts. That remaps a screen's
`gFEStateInfo` 0x82fded80 music state: MainMenu then InGame is ignored; CAC is Off while the trick
tutorial state's +0x38 is 1; Options is MainMenu in the front end, else PauseMenu; InGame is
PauseMenu while exiting to the main menu; Loading is Off, or ignored when `FEBackEnd::Data.mFEState`
is `eFE_STATE_IN_GAME`; MusicPlayer is InGame. Leaving Loading or Off for InGame first calls
`MusicPlayerManager::SwitchMusicMode` 0x82629fc8 with the profile's music mode, here the default
world mode (`SetDefaultMode(MUSIC_ENVIRONMENT)`). `mFEMusicState` starts at Ignore (base
constructor 0x82d68230).

The port's front end has no FE state machine, so `MusicSystem::front_end` makes the messages from
`GameAudioState`: entering the front end (`is_fe`) is `EnterFE` 0x824fd320 (`MusicTrigger(ALL_INIT)`)
and the next updates request MainMenu (the main menu rows hold 0); leaving it is the loading
screen's Loading with `eFE_STATE_EXIT_FE`; in the game the HUD stack's `fe_music_state` goes
through. `AudioSystem::enter_world` ends with `MusicSystem::boot`: the front end's `EnterFE` and the
music updates of the boot screens until the first reset has run, so the main menu's message comes
after it, as natively.

### Mixer objects (music/sfx.rs, system.rs)

* `SFXObj_Music` is MAIN object 1 (DMixIO 0x40000010), created before `SFXObj_Master`;
  `SFXCTL_MusicPos` is MAIN control 0 (0x60000000), an `SFXCTL_3DObjPos` on `mWorldSoundSource`.
  `ConnectDMixIO` now connects the MAIN control. `InitSFX` 0x82528a00 turns the output on.
* `UpdateParams` 0x82528a88: input 0 the music volume distance, 1 and 2 inside the inner sphere
  (now and since entering), 3 times three, 4 game music disabled, 5 music playing, 6 the rockstar
  level (the x1.5/x2/x3 levels of `aud_general` "music", rising and falling at its rates), 8
  challenge loading. `ProcessUpdate` 0x82515408 reads the nine volumes, wet volumes and low-pass
  frequencies by music type, the azimuth, and the controller's distance and twist.
* `SFXObj_Master::UpdateParams` 0x8252a918 inputs 7 and 8 are the current mode CHASE (0x7FFF and
  0x3FFF), input 9 CHALLENGE.
* `AudioCore::CreateMusicMasterVoice` 0x82a7d658 (in `InitializeAudioCore` after the mastering
  voice): stage 200 `SubMix` "Music Master" and a `Send` to the master; its `VuMeter` is left out.
* Update order: `MusicSystem::Update` runs in group A after the event dispatch and before the speech
  system (`AudioSystem::Update` 0x82507b88); the Pathfinder timer is a `TIMERSTAGE_BEFORECOMMANDS`
  timer of `render_stereo`. New emitter volumes are also refused while the music targets CHASE.

### Signal chain

Stream voice (stage 0) -> dry send -> `MusicDspVoice` 0 (stage 2: peaking, high pass, low pass) ->
"Music Master" (stage 200) -> master. In a DJ location the sets add DSP voice 1 (with delay) and a
direct route to the music master; set 3 feeds the reverb send. The volume is `PATH_gain` on the
stream (`CalcVolume` from the music object's mixer volume).

### Verification

A temporary probe outside the workspace (the app's `load_system`, `GameStreamFiles` over
`<game folder>`) ran `AudioSystem` for the scenarios below and has been deleted.

* **Main menu.** After boot the iPod track plays a shuffled main-menu song from about 0.5 s
  (e.g. node 706, song 0x1DEB2B7C), node to node (706, 707, 708), at about -27 dB RMS.
* **Bit-exact decode.** With `kApplyDSP` and `kGlobalReverb` off and the SFX and speech sliders at 0
  (which silences the front-end ambience bed), both output channels equal a direct EA-XMA decode
  of node 706's sample (`Ipod_Stream.mus`, header 0x9A0, chunks 0xE3A5F, 485714 samples) times a
  constant 0.125278, correlation 1.000000, residual 1e-9 RMS (float rounding) over seven seconds.
* **World.** Leaving the front end runs Loading -> Off -> STOP, then InGame -> world mode -> RESET.
  With the skater in `music.ems` volume 0x49 (`aud_emitter` 0x38FDEA28_3A52CC98, playlist
  0xD019DF2A_A5A867C0, type 4, 52 songs), `CheckEnvironmentZones` triggers the playlist and the iPod
  track plays it through the DJ location channel map. In volume 0x4E (playlist 0xC1FD2D2E_5C2E75E8,
  type 3, 21 DJ songs) the world track plays the interactive DJ song from node 1003 on, one node per
  bar group.
* **Return.** Quitting to the front end (`is_fe` again) restarts the main menu playlist with a new
  song.
* **Tests.** `cargo test --release -p skate2-audio` passes (the calcfade curves and the random
  generator steps among them).

### Not ported

* Chase, race, challenge and replay music (`ChaseTrigger`, `RaceTrigger`, `ChallengeTrigger`,
  `ReplayTrigger`, `StopChaseMusic`, `CheckChaseState` 0x824f7088, `CheckReplayState` 0x824f7340,
  `CheckForContestReplay` 0x824f79f8): no chase, contest or replay input exists, so the modes 5-7
  are never targeted (`game.mpf` still loads).
* The interactive score: `mCondScore - mCondPrevScore` is 0, so the DJ level stays at its minimum.
* The chyron display (`FrontEndManager::mChyron.Display`), the music player (`MusicPlayerManager`,
  custom playlists, the profile music mode) and user (XMP) music.
* `VuMeter` and `GetMusicVU`, so `SFXObj_VU` input 0 stays 0.
* Pathfinder RAM (bank) tracks: sub-bank loading, `PATHI_choosesynchtime` beat synchronisation and
  the bank actions, which the stream tracks do not use.

### Observation

The front end starts (and restarts on re-entry) with a 15 ms transient peaking at 0.41 about 10 ms
in. It is not music: it is there with the music disabled. The front-end ambience bed underneath the
music measures about -37 dB RMS.

## Speech (speech/mod.rs, speech/spch.rs, speech/instance.rs, speech/pedestrian.rs; world/pedestrian.rs, system.rs)

Plan phase A10. Living-world pedestrian speech plays end to end. The cameraman, announcer and
main cast are reached natively only through the context systems, which are not ported (see the
end), so those projects load but nothing triggers them.

### The `spch` library (speech/spch.rs, 0x829b3180-0x829b6ef0, 0x82ee5680-0x82ee5938)

* Event database (`VoxData`, `<project>Output_Events.evt`, revision 3.21):
  * Header 0x18 bytes, then the event offset table (`u16 * 4` per event).
  * The global match parameters follow at `align_up(2 * numEvents)`.
* `VoxEvent` (12-byte header), followed by:
  * sentence offsets (`u16 * 4`, `align_up(2 * ns)`);
  * 3-byte rules (`round_down(3 * (nr + 1))`);
  * two rule-bit arrays of `((nr + 7) / 8) * ns` bytes (the block `align_up`);
  * row contexts (`round_down(3 * ((nrc >> 4) + 1))`; byte 1 is the parameter index);
  * valid-parameter codes: a `u32` of 2-bit codes, plus a `u16` per sentence when `flags & 8`, the
    block `(2 * ns + 7) & ~3`;
  * follow data when `flags & 0x10`: a `u8` count and `u16` event ids.
* Sentence: 8 bytes, `bitField = numPhrases << 2 | filterMode`.
  * Then phrase offsets (`u8 * 4`), context masks (`u32`) and the mutable match-parameter IO
    words (`u32`).
* Phrase: 8 bytes (bank id, bank id index, bank type 0/1/2, filter count).
  * Each 8-byte filter is `{u32 mask, parm (0 / 254 post / 255 IO / n), rule, match parm (0x80 =
    sentence IO), pad}`.
* Bank header (`VOXBANKHDR`, 16 bytes, one `.hdr` of `<project>hdr.big`):
  * samples of `np + 2` bytes `{u16 block offset, parms}`, `T = ((np + 2) * ns + 19) & ~3`;
  * `np` `u32` masks;
  * cycle bits when `parmFlags & 0x80`;
  * the repeat queue when `sampleRepeat`;
  * sub-bank bits.
* Sample offsets:
  * A sample's offset is `blockOffset * ((blockSize + 1) << 8)`; a sub bank adds
    `((bs + 1) * bankBlocks * sub) << 8`.
  * The `.sth` record at `offset / 256` is a `u32` `.dat` offset, then the 8-byte SNR header.
* Random numbers:
  * `iSPCH_InitRandom` 0x829b61c8 seeds six words from `seed * 65537`, then makes `32 - x % 32`
    warm-up draws.
  * `iSPCH_EACrandom` 0x829b5f70 is an add-with-carry generator.
  * `iSPCH_Rand` 0x829b6050 redraws up to 32 times against the handle's history, returns the last
    draw and records the best.
  * The game seed is the low word of `time64` (`speech::time64_seed`).
* `SPCH_Init` clears every rule callback and nothing sets them again. So `iSPCH_GetRuleSettings`
  0x829b6cb0 puts every rule in the user mask:
  * The bit is `slw 1, 31 - i`: none once the shift's 0x20 bit is set, wrapping from rule 64.
  * `iSPCH_RuleSet` and `iSPCH_ConstantRuleSet` return at once.
  * `gFilterSetting` is 0, `gClearCycle` is 1, and the data rate is
    `SPCH_GetSampleDataRate(22050, 16)` (44100).
* `SPCH_AddSmartEventV` 0x829b3b88 takes `(callbackData, spec, args)`; the speech system passes the
  pedestrian id. `SPCH_Play` 0x829b3fb0 chooses and plays one sentence per channel.
* A sample request (`SpeechSampleRequestCallback`) goes to the instance project with the
  request's channel. The project keeps only the latest (`PlayRequest` 0x82d3e5f8).

### `Sk8SpeechSystem` and the speech instances (speech/mod.rs, speech/instance.rs)

* `CreateAudioSingletons` 0x8272e4c0 constructs it after `GlobalEffects` and before `MusicSystem`
  (`AudioSystem::new` keeps that order).
  * The speech master voice is `CreateSpeechMasterVoice` 0x82a7d820: stage 200, `SubMix "Speech
    Master"`, VU, `Send` -> master. The VU plug-in is not ported.
  * `SpeechSystem::Initialize` 0x82d40ce8 creates three `SpeechInstance`s of two streams, the 'Spch'
    stream pool (0x53706368, 3 streams, 0x40000 bytes) and `SPCH_Init`.
  * `AudioSystem::Initialize` 0x82509c18 calls it (0x82509fbc). The port makes the instances and
    `SPCH_Init` in `Sk8SpeechSystem::new` and the pool in `AudioSystem::enter_world`, because the
    pool's streams read the host's files, which the host sets after `AudioSystem::new`.
* `GetVoiceStyle` 0x824eb3b0: instance 0 (characters) is distance based on both streams; instance 1
  is stereo on stream 0; instance 2 is 6-channel on stream 0. Every other stream is disabled.
* `CreateVoice` 0x82d40368 builds each style's voice. The `mplugInIndexs` slots are `SndP1, Rch,
  Rsp, notch, delay send, Gain, HPF, LPF, wet Send, Pan, Send`.

  | Style | Plug-ins | Slots | Main send | Setup |
  | --- | --- | --- | --- | --- |
  | Stereo | `SndPlayer1, Rechannel, Resample, Gain, HighPassIir2, LowPassIir2, Send (wet), Pan2D1 {45, 135, 0} 6ch, Send 6ch` | `[0,1,2,-1,-1,3,4,5,6,7,8]` | speech master | - |
  | 6Channel | the stereo chain at 6 channels | as stereo | master | pan attributes 1 = 0, 2 = 1000 |
  | DistanceBased | `SndPlayer1, Rechannel, Resample, PeakingIir2 (notch, attribute 2 = 3.0), Send (delay), Gain, HighPassIir2, LowPassIir2, Send (wet), Pan2D1, Send` | 0..10 | master | delay voice |

  * The wet send goes to `GetReverbVoice` 0x824fbad0, `GlobalEffects::mpEnvSendSubPlugInPtrs[0]`.
  * The delay voice (`CreateDelayVoice` 0x82d3fb40) is stage 2: `SubMix, HighPassIir2, Delay (max
    0.15 s), LowPassIir2, Pan2D1 6ch, Send 6ch -> master`.
* Loading: `SecondaryLoad` 0x824fe178 (English branch) runs `SpeechInstance::Load` 0x82d3e958 for:
  * instance 0: `maincast` on channel 0 and `livingworld` on channel 1;
  * instance 1: `cameraman` on channel 2 and `announcer` on channel 3.
* What `Load` reads, from `data\audio\english\<project>speech.big|`:
  * `<project>Output_Events.evt` goes to `SPCH_AddEventDB`.
  * Each `.hdr` of `<project>hdr.big` goes to `SPCH_AddBank`, with handles rising.
  * The `.sth` at the same index of `<project>sth.big` is paired with it.
  * A bank's `.dat` is the `.hdr` name with a `dat` extension, in the same archive.
* `enter_world` loads the projects through the host's stream files (`rw::core::filesys`), so the app
  needs no change. The `.csi` subscription is left out because only the context systems call it.
* `AudioSystem::Update` 0x82507b88, group A runs, in order:
  1. `AudioAssetManager::Update`
  2. the event dispatch
  3. `MusicSystem::Update`
  4. `Sk8SpeechSystem::Update` 0x8250d438
  5. `Sk8ContextSysMan::Update`
  6. `UpdateSpliceManager`

  `Update` sets the `spch` tick from `Simulation::mTick` (`GetTick` 0x824fa4e0) and runs
  `SpeechSystem::Update` 0x82d40fd8. Its `UpdateVoiceInfo` part fills only the announcer, cameraman
  and NIS voice infos, and those objects are not ported.
* `SpeechInstance::Update` 0x82d40be0 runs each stream:
  * A playing stream follows its speaker (`UpdateParams` 0x82d3f0d0) until `UpdatePlaying`
    0x82d3f6f0 finds its request done.
  * An idle stream runs `SPCH_Play` for each project and plays `ChooseSampleRequestData` 0x82d3e528's
    pick. The priority is `2 - project slot`.
* `PlayRequestedSample` 0x82d409b8 and `PlayData` 0x82d3f798 play the request. They need a free
  speaker from `GetVoiceInfoForSample` 0x8250be58. The `SndPlayer1` event 0 is:
  * start: core time + the event tuning's `Delay`, or +1000 s while `mQueueSpeech`;
  * stream path: project archive + `.dat` name, at the `.sth` record's `.dat` offset;
  * RAM sample: the record's SNR header;
  * stream pool 'Spch', expel 0.
* "Request done" is `SndPlayer1::EventEvent` event 2 (`SndPlayer1::request_done`). Attribute 0 is the
  current request handle and attribute 2 the current request's length:
  * a handle below attribute 0 is done;
  * a handle equal to it is done when attribute 2 is 0;
  * a handle above it is not done when it is above `last_request_handle_processed`, or at most
    `last_request_handle_successfully_processed`;
  * otherwise it is done when attribute 2 is 0.
* `UpdateParams` maps the `VoiceInfo` onto the plug-ins:
  * gain = vol / 32767 (0 while `mDisableVolume`);
  * pan = azimuth * 360 / 65535;
  * resample = pitch / 4096, with a pitch of 256 giving 0;
  * high pass and low pass in Hz, wet send = wet vol / 32767;
  * the notch frequency, gain and Q;
  * the delay send, its filters, and the delay time (only when it changed, capped at 0.15 s).

  Without a speaker the voice runs at unity (low pass 25000). Once `SndPlayer1` attribute 0 equals
  the stream's handle, the stream stops queueing.
* Speaking queries (all on instance 0's streams, where actually playing means `+0x306` playing and
  not `+0x348` queueing):
  * `IsALivingWorldCharacterSpeaking` 0x824eab78 and `IsAMainCastCharacterSpeaking` 0x824eaa78 test
    `mCurChannel` (`+0x308`) for 1 and 0. They drive `SFXObj_Master` inputs 11 and 10.
  * `SecuritySpeaking` 0x824ebbd0 tests `IsCharacterRangeSpeaking` 0x824eb3f8, whose range check is
    `character >= low || character <= high`. That is always true, so it means any voice playing.
  * `IsProVoPlaying` 0x824eb490: a main-cast voice-over playing.
* `SFXObj_Speech` (MAIN 8, `UpdateParams` 0x825333a0, vtable 0x820298f0) sets its inputs after
  `SFXObj_Pause` and before `SFXObj_Bloom`:
  * inputs 0-5 are cleared;
  * 1 = 0x7FFF while `SecuritySpeaking`;
  * 2 = `ScriptedSpeechPlaying`, always false;
  * 3 = 0x7FFF while `IsProVoPlaying`;
  * 0 and 4 come from a getter that returns 0.

  Its `InitSFX` (0x82515d80, shared) only turns the output on, which `connect_mix_map` already does.
  Its `ProcessUpdate` slot is the shared no-op 0x825187c0.

### Living-world speech (speech/mod.rs)

* `PlayPedestrianSpeechSample` 0x82503848 maps the pedestrian audio state (`kEnableLWSpeech` 1) to
  an event:

  | Audio state | Event (and side effect) |
  | --- | --- |
  | 1, 3 | `Radio` for a security guard; for a bum (`0x4000`) one of `Shout` / `PanHandle` / `RandomBum` by `Random() % 3`; anyone else returns |
  | 2 | `Shout` |
  | 4, 5 | `Gasp` |
  | 6 | `ImpactReact`, plus `T_715_40_collide_lw` on a player collision |
  | 9 | `T_811_NPC_Knck` on a player collision, otherwise nothing |
  | 10 | `Concern` / `SpecCollision` by `Random() & 1` |
  | 11, 66 | `ChaseTerminate` |
  | 12 | `CoolDown` |
  | 14 | `ChaseStart` / `ChaseJoin` |
  | 15 | `ChaseJoin` |
  | 16 | `ChaseTerminate`, plus `T_600_GetAway` |
  | 17 | `Lunge` |
  | 18 | `HotPursuit` |
  | 19 | `Catch` |
  | 20-22 | `Flee` / `Help` |
  | 23, 24 | `Pos` |
  | 25 | `SpecSlam` |
  | 26 | `Repeat` |
  | 33-40 | the conversation events |
  | 48 | `ConvCell` |
  | 50 | `CellBye` |
  | 51 | `Grunt` |
  | 59, 60 | `Warn`; a guard plays 0x2000 or 0x2001 by the chase no-skate flag |
  | 62 | `ClsInteract` |
  | 65 | `Block` |
  | 69 | `ClsReact` |
  | 70 | `CellGreet` |
  | 71 | `Catch`, plus `T_609_Catch` |

  `mChaseAudioData` is not fed, so the chase flags (0x40 warn, 0x80 no-skate zone) read 0.
* `PlayLivingWorldSpeech` 0x824fc198:
  * `CanPlayLivingWorldSpeech` 0x824fc0a8 needs a live, non-replay presentation and an offline game.
  * The tuning comes from `GetSpeechTuningKey` 0x824ec840, the character's 0x38-byte entry in
    `aud_speech` "default" 0x60CA5F2E959EC361 at +16.
  * `GetSpeechEventKey` 0x824fb138 searches the channel's array of the `aud_speechtuning` collection
    (MC 0x355D04659873929C, LW 0xC3E2421FFA830BB6, CM 0x28226E0CC3C03F13, AN 0x10A8410E40307891).
    * Entries are 0x20 bytes, matched on the **low** half of word 0 (`clrlwi 16`), with the
      `aud_speechevent` key at +16.
    * A miss falls back to the channel default (LW 0xF97B9086CAA50C53).
  * The `aud_speechevent` layout supplies the delay, interruptable / interrupt / interrupt-stream
    flags, priority, frequency, minimum time between events and voice-over.
* Then, in order:
  1. `CheckTimeBetweenEvents` 0x824ec8d8: the f32 `mLWTimeBetweenEvents[character][index]`, indexed
     by `ConvertSpeechEventToIndex` 0x824ea888 over the 61-event table 0x820159e0 and zeroed at
     construction, must be older than the minimum.
  2. `ShouldPlayEvent` 0x824eb328: `Random() % 1000 * 0.1` below the frequency, or at least 99.9.
  3. `CheckForInterrupt` 0x824fb538: the current tuning is read flat at `channel * 2 + stream`.
  4. `PlayLWSEDEvent` 0x824eb930 (project 1, dat 0, args `char type, char id`, plus a third: near/far
     for 0x2000/1/2/A/C/F, 0x2012 and 0x201C; class for 0x2014/5; topic for 0x204D-0x2054).
  5. `UpdateEventTime` 0x824ec990.
* `GetVoiceInfoForSample` 0x8250be58 (living world):
  * A voice-over event speaks as the NIS photographer; otherwise the attached pedestrian state whose
    id is the event's callback data (at least 0x1000001) speaks.
  * `mSpeakingPedInfo[stream]` takes the id with distance 0.
  * An incoming call overrides with the cell-phone voice.
  * `ParseEventFileName` 0x824eb5d0 on the bank name (`EEE_CC_..._f.dat`) sets the character, the
    common event and far.
  * The speaker must be neither playing nor queueing; `UpdateTuningData` 0x824fafe0 caps priority
    and frequency at 100.

### `SFXObj_PedestrianSpeech` (speech/pedestrian.rs; world/pedestrian.rs)

* PEDESTRIAN object 0 (`CreateObject` 0x8252e920, 0xA4 bytes), mix IO `0x4005xxx0`. The
  constructor sets previous state 72 (Max), event 8279 (none) and delay count -1.
  * `InitSFX` 0x82516810: output on; the voice takes the pedestrian's id and character, attached, on
    stream 2 (none).
  * `Detach` 0x82516908: detached; low-volume count and proximity flag cleared; the ring deleted.
    Then `CSTATE_Base::Detach` turns the output off and clears the inputs.
* `UpdateParams` 0x8254dfe0 is `UpdateSpeech` 0x82545eb0, then `ProximitySpeech` 0x8252ed88.
* `UpdateSpeech` 0x82545eb0 speaks when the state changes, or while still `PictureTaking` (29); near
  means distance <= `near_far_distance`.
  * 49 (`CellHello`) plays the ring.
  * 7 and 8 become 30 after picture taking, else 51.
  * An announcer character (`an_char_id`) goes to `PlayAnnouncerSpeechSample` 0x824fc020: 28 is
    `T_601_greet`; 51 is `T_201_grunt` or `T_202_hit_react` by `Random() & 1`.
  * A living-world character (`mc_lw_class`) goes to `PlayPedestrianSpeechSample`.
  * Anyone else goes to `PlayMainCastSpeechSample` 0x82503ee8, which first stops a speaking stream
    for 19, 30 and 51. Its triggers: 10 `T_205`, 14/53/54/65 `T_663`, 17 `T_609_Lunge`, 19 `T_624`,
    28 `T_601_ai_greet`, 29 `T_1002`, 59/60 `T_501_Warn`; 30/51 is `T_626_BBHit` for Big Black hit
    by the player, else `T_201`/`T_202` by `Random() & 1`.
  * The `Random` draws are made even though nothing receives the triggers.
* `ProximitySpeech` 0x8252ed88: within `CameramanSpeechTriggerDistance` (0x8EA50947E8463836, 15 m)
  the cameraman's `T_4999_In_Prox` fires once (Big Black also sends `T_621_Arrival`).
* `ProcessUpdate` 0x82545e98 runs `UpdatePedVoiceInfo` 0x825459c8 while attached:
  * Azimuth is output 0, pitch output 1. Dry and wet volumes (each `* volume_factor`):

    | Pedestrian | Dry output | Wet output |
    | --- | --- | --- |
    | `mc_pro_id` set | 9 + far | 19 |
    | photographers (Atiba 128, Burnette 256) | 11 + far | 20 |
    | other `mc_ip_id` or `mc_ip_class` | 7 + far | 18 |
    | guard playing `Radio` | 6 | 17 |
    | other guard | 4 + far | 16 |
    | anyone else | 2 + far | 15 |

  * Delay send is output 21 (`* volume_factor`), its high pass 22 and low pass 23.
  * While speaking, a volume at or below 0x6995C510258C9AF6 (200) for more than 0x3D8CD05C962FF399
    (60) updates stops the stream (`StopSpeechByChannel` 0x824fb668). `mSpeakingPedInfo` takes the
    volume and distance.
  * Notch gain, frequency and Q come from 8-point `PointGraphEval` graphs of the `aud_speech`
    "default" layout at +80/+160/+0 (x at +16, y at +48) over the azimuth folded at 32767:
    `GetNotchGain` 0x8252f0c0, `GetNotchFreq` 0x8252f010, `GetNotchQ` 0x8252f170. The Q graph is
    flat 3.0.
  * Delay: when the count is at most 0, the delay is `min(GetDistanceFactor * distance / 344,
    0.15)`. `GetDistanceFactor` is 0x8252ea48, key 0xBF48032DB145C5B4 = 1, and the min is an `fsel`.
    The count then steps down, and a negative count reloads 0x510BAFA32A76B340 (4).
  * High pass is output 13, low pass 14; then `UpdateCellPhoneRing`.
* `PlayCellRing` 0x8252eca8 plays `Splice(27 = CellPhone_Rings.bnk, 5, master)` at volume 0.
  `UpdateCellPhoneRing` 0x8252ead0:
  * While the ring plays it follows the voice (vol / 32767, pitch / 4096, azimuth * 360 / 65535,
    spread 0).
  * When the ring ends, the splice is deleted and state 70 (`CellGreetSpeech`) speaks.
* `PedAudioState` speech fields (`CSTATEMGR_Pedestrian::UpdateParams` 0x825530d0). Unless noted they
  come from the character's `aud_pedestrian` collection, and `PedAudioStateInit` 0x82550000 zeroes
  everything but `character` 100 and `volume_factor` 1.0:

  | Field | Source |
  | --- | --- |
  | `grunt_index`, `charSpchId` | layout words 0 and 2 |
  | `volume_factor` | 0x2087A3290483BB4F |
  | `near_far_distance` | 0xA27215A909135B62 |
  | `mc_pro_id`, `mc_ip_id` | 0x161E7AD743C83C3C, 0x14FD437D190677C8 |
  | `mc_pro_class`, `mc_ip_class`, `mc_lw_class` | 0x6F2933E977CF40DD, 0xD6EA428C2B43E23A, 0xAF7F088CDBEAA429 |
  | `cm_lw_type`, `cm_lw_gender` | 0x0CE4A5717E9DB02C, 0x68BB61E508841729 |
  | `cm_pro_id`, `cm_ip_id` | 0x877795E7633B441E, 0xBDF5EAF6AC880F6A |
  | `an_char_id` | 0x6F9C8A27E4CD37DC |
  | `lw_class` | 0x02D9BFDA94A8B35E of the greet target's collection, when the target is below 100 |
  | `topic` | `1 << (topic - 1)` for topics 1-17, else 0 |
  | `playSpeech` | distance < `aud_speech` 0x7F0FFDF7A90577C6 (50); written, never read |
  | `wasPlayerCollision`, `distance` | the `PedestrianData` |

  * A refresh also copies `lw_class`, `topic`, `distance`, `wasPlayerCollision` and `playSpeech`.
  * The guard re-pick (75 + `Random() % 5`) re-reads every characteristic.
  * After the update the manager reports each state's (attached, id, character) to the speech
    system (`GetPedestrianVoiceInfoFromState` 0x82553ee0).

### Data (skater.vlt and english/*speech.big, verified)

* Codecs per project (`.sth` SNR byte 4):

  | Project | Codec | Channels | Banks |
  | --- | --- | --- | --- |
  | maincast | 5 (EALayer3 'EL31') | mono | 1393 |
  | livingworld | 3 (EA-XMA) | mono | 1050 |
  | cameraman | 3 (EA-XMA) | channel config 4 | 281 |
  | announcer | 3 (EA-XMA) | mono | 270 |

  EALayer3 decodes through `SndPlayer1`'s `CodecDecoder` (fork w3eal3); the speech code needs
  nothing for it. Main cast is reached natively only through the context systems, NIS and mission
  speech.
* Characters by `aud_pedestrian` id:
  * 1-24, 27, 28: `mc_pro_id` (one bit each).
  * 26, 29-33, 36-38: `mc_ip_id` (Big Black 26 = 1, Atiba 38 = 128, Burnette 37 = 256).
  * 35, 36: `an_char_id`.
  * 41-79, 82-86, 89-96: `mc_lw_class` (living world). Guards are 75-79 (`type` 0x40, volume
    factor 0.794); the bum is 86 (0x4000, 0.5).
* `near_far_distance` is 20 for the living world and 30 for the main cast.
* Character 41's living-world tuning, as `event: priority / frequency / minimum seconds`:
  * `Pos` 0x200A: 45 / 10 / 5
  * `Shout` 0x2016: 20 / 15 / 200
  * `Grunt` 0x200E: 70 / 100 / 0
  * `ImpactReact` 0x200F: 80 / 100 / 0, with a 1.5 s delay
  * `Gasp` 0x2010: 65 / 100 / 100
  * `Warn` 0x2012: 100 / 100 / 0

### Verification

* Unit tests in `skate2-audio` (60 pass):
  * `spch`: the static seed words, carries, rand bounds, bank sample sizes and queues, bank order,
    event-spec words.
  * `speech`: bank-name parsing, the event index.
  * `world::pedestrian`: rows, speech fields, guard re-pick.
* Offline renders against the real data, using temporary probes (deleted) with
  `GameStreamFiles` on `<game folder>` and the real `skater.vlt`:
  * The four projects load: 4 event databases, 2994 banks.
  * `101_pos` (0x200A) resolves to `101_41_adtm1_SpecPos_f.dat`; the `.sth` record is codec 3,
    44100 Hz. It renders through the instance-0 distance voice at about -20 dB.
  * A pedestrian object (character 41) driven through states 3, 6, 4, 51, 23, 2 and the real tuning
    played:
    * `ImpactReact` 1.5 s after the knockdown;
    * `Grunt` at once;
    * `Pos` once its 10 % draw passed.
  * In the same run, `Gasp` (minimum 100 s) and `Shout` (200 s) were held back, and each event
    started only after the previous one freed the stream.
  * Near/far banks follow `near_far_distance`: at 3 m the near/far events picked banks without
    `_f`, at 25 m the `_f` banks.
  * The main cast `101_15_Buse_pos.dat` (codec 5) rendered through the speech chain at about -25 dB
    for 1.2 s, then finished.

### Not ported

* The context systems (`Sk8ContextSysMan`, `ContextBuilderXMLCallback`, `script.css`,
  `Trigger_Events.csi/.evt` in `data\audio\context\*.big`). These are the only way the cameraman,
  announcer, main-cast and skater (`SFXObj_PlayerSpeech`) speech starts.
  * `trigger_context_event` stands for `PlayCameraManCsisEvent`, `PlayMainCastCsisEvent` and
    `PlayAnnouncerCsisEvent`.
  * The objects that would speak are not ported either: `SFXObj_Announcer` (MAIN 0),
    `SFXObj_CameraMan` (MAIN 3) and `SFXObj_PlayerSpeech`.
* NIS, post and mission speech:
  * `QueueNISSpeech` 0x824ead68 and `PlayNISSpeech` 0x824eafe0, reached through `QueueNISPost`
    0x824f1158 and `PlayNISPost` 0x824f11c0 with `eSk8Character_SFXPost`.
  * `LoadResidentData` 0x824fa7a8 (`nisresident.big`, `post.big`).
  * `FinishNIS` 0x824faec0.
  * The speech side of `SFXObj_NISCharacter`.
  * `PlayMissionSpeech` 0x82504ed0.
  * The core has no audio-speech ICE channel to drive them:
    `ICEUserChannelAudioSpeech::FireEvent`, `ICELITEUserChannelAudioPost::FireEvent` and
    `ScenePlayer::AudioQueueForStart`.
* The VU meter plug-in (`rw::audio::core::VuMeter` 0x82bf3a68-0x82bfcb88) in the speech and music
  master voices, and `GetSpeechVU` 0x82a7d8f0 / `GetMusicVU` 0x82a7d728 for `SFXObj_VU`.
* `SndPlayer1` events 3 (queued) and 4 (`ModifyStartTime`), used only by queued (NIS) speech.
* `mChaseAudioData`, the watched-event messages in `EventFinished` 0x825077f0, and the
  `mSketchyLand` countdown.

## Music player and volume settings (music/mod.rs, system.rs)

This covers the parts that live in `crates/skate2-audio`: `src/music/mod.rs` and `src/system.rs`. All addresses come from the Jan-17-2009 debug xex in IDA.

### Native enums (from the IDA type library)

| Enum | Values | Rust |
|---|---|---|
| `MusicSystem::eMusicMode` | INIT 1, NONE 2, FE 3, ENVIRONMENT 4, CHASE 5, CHALLENGE 6, REPLAY 7, IPOD 8 | `music::mode` |
| `MusicSystem::eIPodTrigger` | START 1, START_PLAYLIST 2, RESTART 3, STOP 4 | `music::ipod_trigger` |
| `ProfileBackEnd::eMusicMode` (`tMusicPlayer::mMusicMode`) | Off 0, World 1, MusicPlayer 2 | `music::music_mode` |

The front end always calls `IPodTrigger` with **playlist key 0** (r6) and **resume node -1** (r7). That holds for `Play` 0x82639b00, `PlayPlaylist` 0x82629cd0, `PlayNextSong` 0x82629d68, `PlayPrevSong` 0x82629dd8 and `sMethod_Stop` 0x82637ea0. Where the decompiler shows `HIDWORD(v8) = -1`, that is the resume node, not the playlist.

### Public `AudioSystem` API (`system.rs`)

```rust
pub fn set_music_default_mode(&mut self, mode: i32)                 // SetDefaultMode 0x824f61e0, music::mode values
pub fn ipod_trigger(&mut self, trigger: i32, song: u32)             // IPodTrigger 0x824f6d98 (playlist 0, resume -1)
pub fn set_music_player_playlist(&mut self, songs: &[u32], current_song_index: i32)
pub fn music_player_song_index(&self) -> i32                        // tMusicPlayer::mCurrentSongIndex after the update
pub fn current_music_player_song_id(&self) -> u32                   // GetCurrentMusicPlayerSongID 0x824e9628
pub fn music_player_song_ids(&self) -> Vec<u32>                     // sMethod_GetDefaultSongList 0x826381a0
pub fn music_player_song_info(song: u32, localize: impl FnMut(u32) -> Option<String>) -> (String, String, String)
```

`GameAudioState` has two new fields, which must be filled each frame:

- `music_mode: i32`: the profile's `tMusicPlayer::mMusicMode`. It defaults to `music_mode::WORLD`, as `ResetProfile` 0x8256d8f0 does.
- `new_profile: bool`: the profile's `tProfileData::mNewProfile` (ProfileBackEnd +0x68).

`MusicSystem`, reachable through `AudioSystem::music()`, also exposes:

- `switch_music_mode(profile_mode, io)`
- `num_music_player_songs()`
- `MusicSystem::music_player_song_id_from_index(i)`

The free function `music::music_player_song_info` is what `AudioSystem::music_player_song_info` calls.

### Native functions ported

| Function | Address | Port |
|---|---|---|
| `MusicPlayerManager::SwitchMusicMode` (+ its `PlayPlaylist(current id, current index)`) | 0x82629fc8 / 0x82629cd0 | `MusicSystem::switch_music_mode` |
| `StateManagerFrontEnd::ChangeFEMusicState`, Loading/Off → InGame calling `SwitchMusicMode(GetMusicMode())` | 0x82656f78 | `change_fe_music_state` now calls `switch_music_mode(io.music_mode)`. It previously hard-coded `SetDefaultMode(MUSIC_ENVIRONMENT)`. |
| `MusicSystem::FrontEndTrigger`, MainMenu/CAC | 0x82506408 | The `profile_music = false` placeholder is gone; see below. |
| `MusicPlayerManager::GetNextSongInCurrentPlaylist` / `GetNextSong` | 0x826191b0 / 0x826078f0 | `next_music_player_song`, used by `update_ipod_state`'s end-of-song branch. |
| `MusicSystem::UpdateIPodState`, end-of-song fallback | 0x82506aa8 | `(GetNextSongInCurrentPlaylist(song), collection 0)` in place of `(0, 0)` |
| `MusicSystem::GetCurrentMusicPlayerSongID` (= `MusicPlayerManager::GetCurrentSongID` 0x82629e48) | 0x824e9628 | `current_music_player_song_id` |
| `MusicCore::GetNumMusicPlayerSongs` | 0x82a803a0 | `num_music_player_songs` |
| `MusicCore::GetMusicPlayerSongIDFromIndex` | 0x82a80408 | `music_player_song_id_from_index` |
| `MusicCore::FindSongStringID` | 0x82a7f8c0 | `find_song_string_id` |
| `MusicCore::GetSongInfo` | 0x82a80480 | `music_player_song_info` |
| `sPartNames_ipod` | 0x82fd7ef8 | `IPOD_SONG_NAMES` (53 entries, beside `IPOD_SONGS` = `sPartIDs_ipod` 0x82fd7fd0) |

#### FrontEndTrigger, MainMenu/CAC (0x82506408)

- **MainMenu:**
  - If the default mode is IPOD: StopStream, `mEnableMusic = 1`, then `IPodTrigger(START, MusicPlayerManager::GetCurrentSongID(), 0, -1)`. This keeps the music-player song across the menu.
  - Otherwise: the main-menu playlist.
- **CAC:**
  - If `mNewProfile` is set: set `is_new_profile`. If the default mode is also IPOD, take the same iPod path as the main menu; otherwise use the CAC playlist.
  - If `mNewProfile` is not set: use the CAC playlist. The playlist path returns early when the default mode is NONE or IPOD.

#### GetNextSong (0x826078f0)

1. Start with `idx = mCurrentSongIndex`.
2. If `song` is among the first `mNumSongs` entries, set `idx` to its position.
3. Compute `next = idx + 1`; if `next >= mNumSongs` (signed), set `next = 0`.
4. Write `mCurrentSongIndex = next` and return `playlist[next]`.

The port writes the result into the music system's copy of `mCurrentSongIndex`. The app reads it back with `music_player_song_index()` and stores it in the profile.

### What the app must do

#### Every frame, before `AudioSystem::update`

1. Fill these `GameAudioState` fields in the per-frame literal. `game_audio.rs` does `system.state = state` every frame, so a setter would be overwritten; that is why these are state fields.
   - `sfx_volume`, `speech_volume`, `music_volume` from `tSettings` `mSFXVolume`, `mDialogueVolume`, `mMusicVolume` (0..1).
   - `music_mode` from `tMusicPlayer::mMusicMode`.
   - `new_profile` from `mNewProfile`.
2. Call `set_music_player_playlist(&playlist.mSongs[..mNumSongs], mCurrentSongIndex)` for `mPlaylists[mCurrentPlaylistId]`.
3. Apply any music-player commands queued this frame (see the events list below).

#### Every frame, after the update

- Write `music_player_song_index()` back to `tMusicPlayer::mCurrentSongIndex`.

#### On events (FE side, `MusicPlayerManager` / `MusicPlayerCmdTranslator`)

- **`SwitchMusicMode(m)`**, from `sMethod_SetMusicMode` 0x82637d68, `FrontEndState_MusicPlayer::UpdateMusicMode` 0x8265c2f0, or `OnLoaded` 0x8266a968 when the mode is not 1 or 2:
  1. Store `m` in the profile.
  2. Call `set_music_default_mode(m == 1 ? ENVIRONMENT : m == 0 ? NONE : IPOD)`. Mode 2 maps to IPOD; any other value leaves the default mode alone.
  3. For `m == 2`, also do PlayPlaylist: `idx = (0 <= cur < n) ? cur : 0`, store `idx`, then `ipod_trigger(START, songs[idx])`.
- **`RevertMusicMode`** 0x8262a088: `set_music_default_mode(...)` for the profile mode. It does no PlayPlaylist.
- **`Play(song)`** 0x82639b00:
  - If `song == current_music_player_song_id()`: `ipod_trigger(RESTART, song)` and show the chyron (`Chyron::Display(uint)` 0x82639a08).
  - Otherwise: `ipod_trigger(START, song)`.
  - Then set `mCurrentSongIndex` to the song's position in the current playlist, or -1 if it is not there.
- **`PlayNextSong` / `PlayPrevSong`**: run `GetNextSong` / `GetPrevSong` 0x826079c8 on the current playlist, from `current_music_player_song_id()`, then `ipod_trigger(START, result)`.
  - `GetPrevSong`: `idx - 1`; if that is below 0, use `n - 1`.
- **`sMethod_PlayPlaylist(id)`** 0x82637ee0: `SetCurrentPlaylist(id)` (index 0), then `ipod_trigger(START, playlist[0])`.
- **`sMethod_Stop`**: `ipod_trigger(STOP, 0)`.
- **Song list** (`sMethod_GetDefaultSongList`): `music_player_song_ids()` returns the 52 catalogue ids in order, without the leading `IPOD_INVALID`.
- **Song info** (`sMethod_GetSongInfo` 0x82638048): `music_player_song_info(id, |sid| localized.get(&sid).map(utf16 → String))`.
  - It returns `(artist, title, album)`, which is native out-param order v6, v7, v8. Apt reads [0] as artistText, [1] as songText, [2] as album.
  - Every string is a `#`-prefixed literal; strip the `#` for display, as SkateAptString does.
  - The string id is `ICELITE::StringHash` (`skate2_formats::localization::string_hash`) of `IPOD_<partname>_<ARTIST|TITLE|ALBUM>`, uppercased with the '#' omitted.
  - Fallbacks:
    - artist or title missing from the localizer: the `#IPOD_…_ARTIST` / `_TITLE` label;
    - song not in the list: `#Unknown Artist` / `#Unknown Title`;
    - album missing: `# `.

#### Volume settings

- `tSettings` at ProfileBackEnd +0x78: SFX +0, Dialogue +4, Music +8.
- They reach audio through `SFXObj_Master::UpdateParams` 0x8252a918. Its reads are `lfs` +0x80 → input 1 (music), +0x78 → input 2 (SFX), +0x7C → input 3 (dialogue). Each value is `fctiwz(v * 32767)` clamped to 0..0x7FFF.
- This is already ported as `system.rs` `update_master` from `GameAudioState.music_volume`, `sfx_volume` and `speech_volume`.
- `mVolumeMode` (Full/Midnight) has no reader among the `mBEModules` users in audio.
- The FE music-player volume (`sMethod_SetMusicVolume` 0x82604e38) writes the same `mMusicVolume`.

#### Profile defaults (`ResetProfile` 0x8256d8f0)

- All 5 playlists hold the full catalogue in order (`GetMusicPlayerSongIDFromIndex` 0..51), with `mNumSongs = 52`.
- `mCurrentPlaylistId = 0`, `mCurrentSongIndex = 0`, `mMusicMode = World`, `mIsShuffle = 0`, `mNewProfile = 1`.

### Verification

- `cargo build --release -p skate2-audio`: clean, no warnings.
- `cargo test --release -p skate2-audio`: 69 passed.
- A temporary probe, now deleted, checked:
  - the GetNextSong wrap and search with index write-back, through `update_ipod_state`;
  - Loading → InGame with profile mode 2: IPOD default plus `playlist[index]`, with an out-of-range or -1 index falling back to 0; mode 1 gives ENVIRONMENT;
  - FrontEndTrigger MainMenu, CAC with a new profile and CAC without one;
  - the catalogue count and indices;
  - song info against the real English tables (`miscboot.big`): all 52 artists and titles resolve; China Creeps, Cut Chemist, ELO and Suicidal Tendencies have no album string, so they give `# ` as native does.

### Open items and differences

- **Empty playlist:** native `GetNextSong` / `PlayPlaylist` with `mNumSongs == 0` returns the stale raw `mSongs[0]`. A truncated slice returns 0.
- **Chyron:** `Play`'s RESTART `Chyron::Display(uint)` is on the FE side. `MusicSystem::chyron` only carries `UpdateChyron`'s song.
- **Disabled music:** the "IPOD" song list is assumed registered (`MusicSystem::Initialize` 0x824e8c98 registers it only when the music system is enabled). With music disabled, string ids would be 0 natively.

## Options and the music player front end (skate2-core ui/options.rs, ui/music_player.rs, session/selected_session_music_player.rs; app marker_ui/settings_movie.rs, marker_ui/music_player_movie.rs)

* `FrontEndState_Options` (FE state 73, screen 120): `gGameOptionsScreens` 0x82fdfa70 rows the port
  drives. In the front end the list is Audio Settings and the PC Video Settings page; in game it
  leads with the Music Player (`OnMenuSelect` 0x826871a0 `PushState(30)`).
  * Audio Settings: SFX, dialogue and music sliders over `tSettings` (ProfileBackEnd +0x78/+0x7C/
    +0x80). `ReceiveInput` 0x82686978 steps 0.1 per left/right, clamped to 0..1, sounding
    `core_sfx|speech|music_volume_left|right`, and marks `mIsDirty`. The row shows
    `GetIntegerValue` 0x82643b10, `floor(v * 10 + 0.5)`, as ten bars and `ID_COMMON_PERCENT` of ten
    times it (options.apt `UpdateItemValue`).
  * options.apt `Prev/NextValueCalc` wrap past either end; `SetState(1)` reselects row 0.
  * `RequestExit` 0x82681700 folds the music player state's `mbIsDirty` into `mIsDirty` and saves
    the profile (`eBeMemcardSave_Profile`) unless a main challenge runs. `OnEnter` 0x8265c468 clears
    `mIsDirty`, so popping back from the music player reopens Options clean.
* `FrontEndState_MusicPlayer` (FE state 30, screen 35, `musicplayer/musicplayertest`; `gFEStateInfo`
  pause, dimmer, music state 12): `OnLoaded` 0x8266a968 opens on the profile's mode (MusicPlayer: the
  player screen and a playlist refresh; otherwise ambient, switching an Off profile to World).
  `ReceiveInput` 0x8266aa00 handles B, A (`core_b_button` for both), X (`core_x_button`, edit only,
  `ToggleSongInPlayList` 0x8265c0d8), left/right over `gMusicPlayerScreens` 0x82fdf95c
  ({Volume} / {PlayList, Shuffle, Volume}) and LT/RT (`AptLShift2`/`AptRShift2`, `core_tab`);
  keys it returns false for reach the movie's `UpdateInput` (up/down, AptMisc).
  `EnableEditMode` 0x8265c370 reorders the edited playlist (shuffle or sort) on leaving edit.
* `MusicPlayerManager`: `PlayPlaylist`, `Play`, `SwitchMusicMode`, `ShufflePlaylist` 0x82607aa0
  (a hundred global `rw::math::Random` swaps), `SortPlaylist` 0x8262fb40 (catalogue order),
  `SetIsShuffle` 0x8262fc28 (all five playlists). The session queues `MusicRequest`s
  (`SetDefaultMode`, `IPodTrigger`) that the app applies before the audio update, and takes back the
  playing song and `mCurrentSongIndex` after it.
* Saving: `ProfileBackEnd::SaveData` writes Settings, ProfileData and MusicPlayer; the career save
  now carries all three (`EditSkaterProfileStore::save_memcard`), Options' exit saves the first and
  last (`save_audio_profile`). `LoadSettings` 0x8255b950 v3 and `LoadMusicPlayer` 0x825645a8 v2 read
  them back; a missing leaf is `tSettings::Default` / `ResetProfile`'s playlists.
* The movie: `mMusicPlayer`'s `ambient`/`musicplayer`/`editplaylist` labels, `UpdateCurrentPlayList`'s
  seven-row window (the speaker on the current song), the eleven-row edit list with checkboxes and
  the basic_slider scroller. Apt member lookup is case-insensitive (`AptNativeHash::HashFindKey`
  0x829da970, `EqualNoCaseHash`), so the script's `SetSubTitle` is the selector's `SetSubtitle`.
* No Music Player in the start menu: its Options (`mScreenMode` 0 while `mFEState` is
  `eFE_STATE_IN_FE`, `LoadMainMenu` 0x826714d0) lists Audio, Video, Control and Online settings (the
  360 build adds Device Selection); only the in-game list (screen 1) leads with the Music Player, and
  `OnMenuSelect` is the sole `PushState(30)`. The PS3 retail EBOOT's table (file 0x131fde0) agrees.
* The chyron (skate2-core ui/chyron.rs; app marker_ui/chyron_movie.rs): `FrontEndManager::mChyron`,
  main.swf's `gChyron` layer (source/screens/main/chyron, above the screen manager's screens and the
  letterbox). `UpdateOnEnter` 0x82d663f0 enables it for a state whose `gFEStateInfo` row has
  `mbEnableChyron` (MainMenu, FreeSkateOption, Options, the music player, CaC, ChallengeMap and
  others; never StartHub or the HUD states). `Display` 0x82618ff0 shows `GetSongInfo`'s artist,
  title and album through the movie's `FixUpString` (cut with `...` to the artist field's width);
  `MusicSystem::UpdateChyron` 0x825069f0 calls it for each new iPod song and
  `MusicPlayerManager::Play` 0x82639b00 when it restarts the playing one. `ChyronIntroDone` makes it
  Showing; `FrontEndManager::Update` (16 ms per update) plays the outro after 5000 ms. In the start
  menu it shows the song the Music Player mode starts there.
* Leaving the game (`StateGameUnload::UnloadGameState` 0x82757cc8): `HUDManager2::ExitToFE`
  0x82cee828 unloads every HUD component (the app rebuilds its HUD movies,
  `MarkerUi::exit_to_front_end`), the challenge system unloads the career, and
  `MusicPlayerManager::RevertMusicMode` 0x8262a088 sets the profile's mode back.
* The front end's audio: `GameModule::Update` 0x8273edc8 keeps the simulation running while
  `eFE_STATE_IN_FE`, so `AudioSystem::Update` stays in its world branch with a presentation that has
  no skaters, cars, pedestrians or dynamic objects (`SelectedLiveSession::audio_front_end`: the main
  menu's ICE camera, the WorldPainter keys at (0, 0)). The PLAYER manager's `mPreviousLocal` change
  detaches every player state (`CSTATEMGR_Player::UpdateParams` 0x82556c60), freeing the wheels and
  board voices, and the other world managers drop what is no longer submitted.
  `MicrophoneMan::UpdateCameras` returns without a local skater, so the microphones keep the last
  game position. `ExitGamePlay`'s `StopChaseMusic` has nothing to stop: no chase music starts yet.
