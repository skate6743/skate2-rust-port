# Graphics: original-parity world rendering

Maintainer reference for the map and world renderer, as of 2026-09-27. It
records:

- what is ported;
- where each piece came from (IDA, shader source, manifests, Vault);
- what is still missing;
- every deliberate deviation;
- which claims are still unverified.

Per-batch evidence is in `docs/evidence/render-*-2026-09-2*.md`. This file is
the consolidated view; update it when a batch lands.

## Ground rules for this area

- **Goal:** 1:1 with the Xbox 360 game. The reference is the debug build
  `sk82_na_m.xex` (`.i64`, rich symbols) in IDA (static analysis, not x64dbg).
- **Workflow:**
  1. Read the IDA reference.
  2. Write Rust/WGSL from it.
  3. Re-review against IDA before integrating.
- **Trace every per-frame input to its runtime writer in IDA.** A Vault default
  is not enough. This lesson cost a batch: see "Batch 2" below.
- **Shader maths comes from the shipped shader source,** not from engine
  knowledge. `render.big` ships `data/render/shaders/final/<effect>_<prog>.updb`,
  whose `<file id="0">` is the preprocessed HLSL the programs were compiled
  from. Extract it with `tools/updb_source.py`.
- **Render states come from `data/render/shaders/final/<effect>.xml`**
  (technique, pass states, sampler states).
- **Build and playtest** with `build-release.bat`, then `play-quiet.bat`.
- **WGSL is validated only at runtime,** when pipelines are created (wgpu/naga).
  A WGSL mistake panics at startup, so review shader strings line by line.
- **Label diagnostic output as diagnostic.** A draw that cannot run its original
  effect keeps the diagnostic path, and the reason is logged. Never substitute
  a generic shader and call it the original.

## Status by batch

| Batch | Content | State |
|---|---|---|
| Perf (09-25) | Retained map draw list, clip-volume culling, shared camera, per-texture bind groups | Built and run: 80 -> 165 fps (vsync cap) |
| Phase 1 (09-25) | `defaultenvironment`, `alphatestdefaultenvironment`, `baseenvironment` WGSL ports; Vault lighting inputs | Built and run: lightmaps correct; distant fog wrong, no sky |
| Batch 2 (09-25) | WorldPainter fog/sky lookup + 5 s blends; `sky.fx` port; occlusion cubes skipped | Built and run (log 09-25 23:59: 22 fog / 23 sky collections, no lookup failures). The fog look was not explicitly confirmed |
| Batch 3 (09-26) | `decalenvironment` + `transparentenvironment` ports; loose `DIST_skybox` arenas loaded | Written, rustfmt PASS, **NOT compiled/tested** |
| Cull/bias/hair/decal (09-27) | Symptom: z-fighting everywhere (scalp through hat, walls). Every pass now keeps its manifest CULLMODE and DEPTHBIAS/SLOPESCALEDEPTHBIAS (`raster_state.rs`); `decalzbias` ported; defaulthair draws its 3 original passes with `i_alpha` coverage (`docs/evidence/render-cull-bias-hair-decal-2026-09-27.md`) | Written, rustfmt PASS, **NOT compiled/tested** |
| Sky fix (09-27) | Game log: zero Sky draws. `material_sky`'s `effect` is an inline Vault node (0x40); the app resolver dereferenced it once too often, got literal `"default"`, and the draw was silently Deferred. Fixed (`docs/evidence/render-sky-inline-effect-2026-09-27.md`); also un-defers the tree/decalzbias/glare/visualindicator/baseincandescent-default families | Run (09-27): the sky renders, including its `sky.fx` sun glow |
| Skater ghost (09-27) | Respawn opacity reaches the draw: DoSimRestore `i_params[0].x`/`mIsGhosted` (core) + `ghost` technique pass states for defaultcharacter/ropa, drawn pass-major (`docs/evidence/skater-ghost-render-2026-09-27.md`) | Written, rustfmt PASS, **NOT compiled/tested** |
| Sun, viewport, shadows (09-27) | `g_vLightDir` from the SunLightMan map at the skater hips every frame; widescreen windows keep square pixels around the 16:9 view; the original CSM: skater casters, `Variance` filters, 1024x512 atlas, receiver UV per the shipped microcode (`docs/evidence/render-sun-viewport-shadows-2026-09-27.md`) | Run: shadows confirmed |
| Z-fighting pass (09-27) | Symptom: hair/head, hat/head, sleeves, tree leaves and walls flicker wherever surfaces nearly coincide. Root cause: f32 clip math on city coordinates (~3500 m) gave 11-74 mm depth noise; every GPU position is now camera-relative (`CameraRelativeClip`). Also: 19 world-space effects no longer take the instance matrix; world positions `@invariant`; hair program per its microcode; skater inverse binds in the original component-hash order (`docs/evidence/render-zfighting-worldspace-invariance-2026-09-27.md`) | Built and run (09-27): the flicker is gone |
| PROXY world (09-27) | The low-detail city beyond the streamed chunks: `cProxyWorldManager` load order and proxy map, `AddHighCollection` mapping from the world stream, activation, then per chunk load/unload: a registered high chunk deregisters its proxy, its unload registers it again (`PresThreadMsgMan` arena ref counts). `proxyworld.fx` ported from its shipped microcode (`docs/evidence/proxy-world-rendering-2026-09-27.md`) | Written, rustfmt PASS, **NOT compiled/tested** |
| Hair ghost order (09-27) | Symptom: hair invisible during the respawn fade. The original sorts every mesh by `InstanceSortKey` (draw order first: skater 9, hair 10) before drawing single-pass then multi-pass meshes pass-major; the port drew the ghosted multi-pass group in part order, so head/hat blends covered the hair. Character draws now follow the Vault draw order (`docs/evidence/hair-ghost-draw-order-2026-09-27.md`). Also fixes E0063 (`proxies` initializer) | Written, rustfmt PASS, **NOT compiled/tested** |

## Where the code is

| File | Role |
|---|---|
| `apps/skate2-pc/src/gpu_renderer/environment_shading.rs` | `EnvironmentEffect` enum (channels, scalars, vertex inputs), WGSL ports, pipelines, bind-group layouts, samplers, fresnel + CSM atlas resources |
| `apps/skate2-pc/src/gpu_renderer/shadow_map.rs` | `RenderShadows_CSM` GPU side: Z/HBLUR/ATLAS targets, skater `shadow` technique, `Variance_HBlur`/`VBlur` filters, `ShadowCascades` |
| `apps/skate2-pc/src/render_shadows.rs` | `RenderShadows_CSM` CPU side: light basis, cascade rows, cascade 0 -> 1 transform, `ShadowView::Cull` |
| `apps/skate2-pc/src/gpu_renderer/map_draw.rs` | Retained map draw list: instance records, globals packing, `push_environment` / `push_sky` / diagnostic pushes, culling, per-frame `set_view` |
| `apps/skate2-pc/src/gpu_renderer.rs` | Surface (non-sRGB, Opaque alpha, 4x MSAA), `upload_mesh` (UV0 + lighting stream), texture/cube upload, material bind-group builders |
| `apps/skate2-pc/src/render_environment.rs` | Vault reads: `g_envattributes`, sun vector + sun-direction map, CSM attributes, all `material_fog` / `material_sky` collections, every class's `m_params`, fresnel table |
| `apps/skate2-pc/src/render_world_painter.rs` | `BlendAttrib` / `BlendAttribCollection` for fog and sky (exact fmadds/fsel) |
| `apps/skate2-pc/src/render_inspection.rs` | Per-draw routing: effect selection, channel/texture resolution, fallback accounting, occlusion-cube skip, per-frame blend update, log lines |
| `apps/skate2-pc/src/live_skating_view.rs` | Feeds the live camera clip (widened for non-16:9 windows), `g_vViewPos`, WorldPainter keys, and the camera/skater inputs of the sun map and the cascades |
| `apps/skate2-pc/src/main.rs` | `load_runtime_presentation_scene` + loose sky loader (`LOOSE_PRESENTATION_ASSETS`), material attribute bindings |
| `crates/skate2-core/src/rendering/presentation.rs` | `PresentationScene` with `collections` (stream), `dependencies` (ancestors), `loose` (always resident), `proxies` (registered PROXY collections, shared) |
| `crates/skate2-core/src/streaming/proxy_world_manager.rs` | `cProxyWorldManager` order/maps, `AddHighCollection`, and the proxy Reg/DReg state machine over arena ref counts |
| `apps/skate2-pc/src/proxy_world.rs` | Shared proxy owner: manager read before the world stream, `AddHighCollection` forwarder, registered-proxy snapshot |
| `apps/skate2-pc/src/proxy_texture_world.rs` | PROXY collection load in manager order: textures uploaded once, each spatial collection's geometry as a Presentation collection |
| `apps/skate2-pc/src/city_streaming.rs` | Moving focus; each generation's high unloads/loads drive the proxy registrations |
| `crates/skate2-core/src/rendering/render.rs` | `NativeRenderQueue`: submissions stably sorted by (draw_order, effect scope, pass count, material scope); `default` technique by name |
| `crates/skate2-core/src/world_painter_*.rs` | WorldPainter quadtree/dictionary/palette lookup; `WorldPainterKeys` on `SelectedVisualSnapshot` |
| `apps/skate2-pc/src/gpu_renderer/character.rs` | Character draws: **still diagnostic raw diffuse**; original `ghost` pass states (depth pass, then SRCALPHA blend with alpha test 1) |

## GPU data layout

- **Bind groups:** group 0 = globals (uniform); group 1 = the effect's
  material textures + samplers; group 2 = environment resources (fresnel LUT
  256x1, the live 1024x512 CSM atlas).
- **Vertex slot 0** (20 B): POSITION float3, TEXCOORD0.xy float2.
- **Vertex slot 1** (instance, 160 B): world-from-mesh (locations 2..5),
  colour (6), then `InstanceMaterial`:
  - 7 = `i_monoLightmap_Dot`;
  - 8 = macroOverlayUVScale;
  - 9 = detailNormalUVScale;
  - 10 = macroOverlayOpacity;
  - 11 = `m_params[0]`.

  Sky reuses these lanes: 7.x = "reads the WorldPainter blend",
  8 = `m_params[1]`, 11 = `m_params[0]`.
- **Vertex slot 2** (lighting, 36 B, only when the mesh has TEXCOORD1):
  - 12 = TEXCOORD1 as float4 (`LM`, or `lm_norm`; decoded bits kept, so the
    half-float -0.0 sign encoding survives);
  - 13 = TANGENT float3;
  - 14 = TEXCOORD0.zw (decal UV).

  `GpuMesh` gates admission with `lightmap_components`, `has_tangent` and
  `has_decal_uv`.
- **Globals** (240 B, WGSL `Globals`):
  - clip-from-world;
  - `g_vViewPos`;
  - `g_vLightDir`;
  - `g_FogColour`;
  - `g_FogK1`;
  - `g_envattributes[2]`;
  - `CSM_Mat_Row0[0..2]`, `Row1`, `Row2` (this frame's cascades);
  - the blended `material_sky/default` `m_params[0..2]`.

### Original `GlobalShaderData` (IDA type; offsets from `GfxMan->mGlobalData`, GfxMan +0x1B180)

| Offset | Field | Port |
|---|---|---|
| 0x00/0x40/0x80 | `g_matVP`, `g_matV`, `g_matP` | clip-from-world only |
| 0xC0 | `g_vLightDir` | yes (sun-direction map at the skater hips, every frame) |
| 0xD0 | `g_vViewDir` | no |
| 0xE0 | `g_vViewPos` | yes |
| 0xF0 | `g_ViewDotLight` | **no**. Formula decoded (below) |
| 0x100 | `g_vCubeMapCentre` | no; writer not traced |
| 0x110 | `g_fresnelLookupTexture` | texture resource at group 2 |
| 0x120 | `g_gammapower` | no (`rendering/default` key 0x223727ee2c3d853b = 2.0) |
| 0x130 | `g_FogColour` | yes |
| 0x140 | `g_FogK1` | yes |
| 0x150 | `g_fAnimationTime` | **no**. Writer found (below) |
| 0x160/0x170 | `g_TrafficLightsStatus_1/_2` | **no**; writer not traced |
| 0x180.. | `CSM_MatRow0..2[4]`, `WorldShadow_MatRow[4]` | the elements the world effects read (`Row0[0..1]`, `Row1[0]`, `Row2[0]`); `WorldShadow_MatRow` no |

`GraphicsMan::Init` 0x8273f6e8 registers every name to its field. There are no
direct stores. The writers use base+offset (`li rN, off` + `stvx128`) or a
computed address.

- **`g_ViewDotLight`** (`RenderMan::UpdateRenderParams` 0x82852780, stvx at
  +0xF0) = (dot(normalize(camera zAxis), A) * B.y + B.x, a, b, 0). All inputs
  are from `rendering/default`:
  - A key 0xd900aef7ea4d75a2 = (0.5, 0, -0.879, 0);
  - B key 0x222cba2d46ea6839 = (0.5, 0.2, 0, 0);
  - a key 0x9cdb53d164e5da30 = 0.2;
  - b key 0x0279cce002b46bc4 = 0.4.
- **`g_fAnimationTime`**: the writer is `WorldPresentation::Update` 0x827d3af8.
  It reads the DataSystem time-controller tick `{int frame, float fraction}`:
  - In an NIS, dT = the ScenePlayer's `GetTimeDelta`.
  - Otherwise dT = |(fraction - mPrevDelta) + (frame - mPrevFrame)| / 60, or 0
    when |frame delta| >= 15 (`CLOTH_MAX_FRAME_DELTA`).
  - Then `.x += dT` (yzw kept), then `cPCAWaterAnimationData::Update(dT)`.

## Implemented effects (`default` technique only)

`PresentationEntity::BindConstants` 0x827fb168 resets every world entity to
technique "default" each frame, so only `default` programs matter for the world.

A draw routes to its original effect only when all of these hold. Otherwise
it stays diagnostic, and the reason is counted in `ported-effect draws kept
diagnostic by reason=`:

- every sampler channel resolves through the shader texture table;
- `environment` is a cube map;
- the scalar channels exist;
- an "li*" lightmap selector exists;
- `m_params` resolves;
- the mesh has the effect's vertex inputs.

Material channels follow `AddPegasusMaterialToScope` 0x82e17bb0:

- channels are read from index 2 on;
- a flag-bit-1 channel is an inline float4 bound as `i_<name>`;
- an "li*" name sets `i_monoLightmap_Dot = LightMapDot[image_channel & 3]`.

`m_params` comes from `cMaterialScope::Resolve` 0x82e17a98 (the material
class/variant Vault collection).

| Effect | Channels (bind order) | Vertex inputs | Output / states |
|---|---|---|---|
| `defaultenvironment` | diffuse, lightmap, chromaticity | UV0, TEXCOORD1.xy | alpha forced 0; opaque |
| `alphatestdefaultenvironment` | same | same | alpha = diffuse.a; test GREATEREQUAL 30; COLORWRITE RGB |
| `baseenvironment` | diffuse, specular, normal, detail, macrooverlay, environment (cube), lightmap, chromaticity + 3 scalars | UV0, `lm_norm` float4, TANGENT | alpha 1; opaque |
| `decalenvironment` | baseenvironment's + decal (CLAMP) | + TEXCOORD0.zw | baseenvironment lighting over `lerp(diffuse, decal.rgb², decal.a)`; alpha 0; opaque |
| `transparentenvironment` | diffuse, specular, environment (cube), lightmap, chromaticity | UV0, `lm_norm` | normal *not* renormalized; alpha = diffuse.a²; test GREATEREQUAL 16; SRCALPHA/INVSRCALPHA ADD; Z write on; COLORWRITE RGB |
| `sky` | diffuse (gradient), detail (clouds), specular (sun colour, fetched as 1D at v=0) | POSITION, UV0 | VS adds `g_vViewPos`; gradient desaturate/tint by `m_params[1]`; + clouds - 0.5, squared; sun glow; x `m_params[0].y`; `PS_Bloom_Tone`; alpha 1; never culled |
| `decalzbias` | diffuse, specular, normal, lightmap, chromaticity | UV0, `lm_norm`, TANGENT (baseenvironment's VS) | rgb² diffuse; unnormalized 2·tex−1 normal into `GetTangentLight`; `GetCalculatedSpecularTerm` (no reflection); alpha = diffuse.a; test GREATEREQUAL 16; SRCALPHA/INVSRCALPHA ADD; Z write; RGB; DEPTHBIAS -0.0001, SLOPESCALEDEPTHBIAS 1.0 |
| `proxyworld` | diffuse (pass-reset sampler) | POSITION, UV0 (SHORT2N) | unlit: (rgb², a) × `m_params[0].y` (0.4 in `material_proxyworld/default`), per-vertex fog, `PS_Fog_Bloom_Tone`; all four lanes written; opaque |

**Rasterizer state.** Every row's `default` pass is CULLMODE FRONT, which is
D3DCULL_CW: screen-clockwise triangles are culled. The port uses wgpu `Ccw`
plus cull `Back` (`raster_state.rs`); BACK maps to cull `Front`, NONE to no
culling. DEPTHBIAS is a normalized-depth offset, 2^-24 per Depth32Float unit
over depth [0.5, 1); the slope scale is kept as authored. The chain is
`cullModeTranslation` 0x83024f8c `[0, 2, 1]` -> `D3DDevice_SetRenderState_CullMode`
0x82c34770 (PA_SU_SC_MODE_CNTL bits 0-2), and `D3DDevice_SetRenderState_DepthBias`
0x82c35338 (unscaled) / `SlopeScaleDepthBias` 0x82c35270 (x16, 1/16-pixel unit).

**Shared helpers.** The WGSL helpers are in source order, with the compiled
call-site literals:

- `ComputeFogFactor` (length to `g_vViewPos`; `pow(sat(d*K1.x + K1.y), K1.z)`);
- `tonemap`, and `PS_Fog_Bloom_Tone` followed by `MoveFromLinearSpace` (sqrt):
  output stays gamma-2 on a non-sRGB surface;
- `CSM_Shadow` / `CSM_ShadowMap_PS`, with the atlas V mapping of the shipped
  microcode (y * -0.5 + 0.5), not the `.updb` text's (y * -0.25 + 0.75);
- `GetShadowedLightMap` with the SingleTap lightmap, and its literals:
  multiplier (4.5,4.5,4.5,1.5), shadow value (0.07,0.09,0.1,0.04), ramp 50,
  CSM bias (-0.005,10,25);
- `GetTangentLight` (0.64,0.64,0.420465);
- `ApplyOverlay`, which returns its darken branch;
- `GetSpecularTerm`:
  - fresnel = LUT.g * 5;
  - reflection xy negated;
  - power 10 + ecc*290;
  - tint (18,15,10.5);
  - threshold 0.1;
  - reflection scale 1.5;
  - ramp (0.3,4,1.5).

The specular term lives in the shared `SPECULAR` WGSL snippet (base, decal,
transparent). The base/decal body is `environment_vs` + `environment_tone(surface)`.

**Samplers.**

- The diffuse family is WRAP, anisotropic, LINEAR mip.
- Lightmap, chromaticity and decal are CLAMP.
- The cube is CLAMP, LINEAR min/mag, POINT mip.
- Sky diffuse/detail use WRAP-U and CLAMP-V; the sun texture is CLAMP.
- Samplers without a manifest block get the pass reset (WRAP, LINEAR
  min/mag, POINT mip, anisotropy 1; `cFxParser::EndElement` 0x82e1fb60,
  reset definition at 0x82e1fd70): the fresnel texture and `proxyworld`'s
  `i_diffuse`.
- `shadowTexture_CSM0` gets the named `ShadowTextureSampler`
  (`AddDefaultSamplerStates` 0x82e18990): BORDER u/v with a zero border,
  LINEAR min/mag, no mip.

## World-level systems

### Environment lighting inputs (`render_environment.rs`)

- `g_envattributes` = `material_envattributes/default` (7 float4;
  `GraphicsMan::Init`). [2].x = 1.0 is the exposure.
- The fresnel LUT follows `CreateFresnelLookupTexture` 0x82833a68 and
  `UpdateFresnelLookupTexture` 0x8282a528: 256x1, from `g_envattributes[5]`.
- `g_vLightDir` = the SunLightMan map sample at the local skater's hips,
  normalized, every frame (see Sun direction below). The location's Vault
  vector (-0.41, 0.4, 0.8) is only the map-less fallback.
- The world render location is always default 0xec763034105b866b.
  - `LooseWorldPresentation::GetWorldRenderLocation` 0x827d0cc8.
  - The city never sends `cMsgSetWorldRenderLocation`; `GameModule::GameLoad`
    0x8273be50 sends it only for extra locations.

### Sun direction and dynamic shadows (09-27)

- **Sun.** `UpdateRenderParams` 0x82852780 overwrites the Vault vector with
  `SunLightMan::GetSunPosition` 0x82851f20: `sun_bam` (64x64 linear A8R8G8B8
  in `miscboot.big` `globaltextures.rx2`) over world x (-2840..1774) and z
  (-2462..1345), `SampleBilinear` then `c * 2/255 - 1`, sampled at
  `GetHipsPos`.
- **Cascades.** `RenderShadows_CSM` 0x82855888 runs right after it. Light
  basis from `-g_vLightDir`; cascade half-extents 1.8 m and 7.2 m
  (`ScaleLevel` {1, 4} x `rendering` 0xb9a92367e7ac68b9 attribute 1.8), each
  centred that far along the camera axis; translation snapped to one texel;
  depth = height below the skater's `m_BB` top + 2 m over 5.1 m.
- **Casters.** Only the local skater in the port (all its parts, `shadow`
  technique: skinned `simpleVS`, stencil 255, COLORWRITE 0, CULLMODE BACK),
  culled per cascade by `IsInLightView` and a 36 m camera distance.
- **Filters and atlas.** `Variance_HBlur`/`VBlur` (5-tap cascade 0, 3-tap
  cascade 1): depth = centre or mean of covered neighbours, coverage tent
  blurred; written into one half of the 1024x512 atlas.
- **Receivers.** The world effects' `CSM_ShadowMap_PS`, then
  `GetShadowedLightMap` darkens only where the lightmap is brighter than the
  shadow colour, so dynamic shadows never double-darken baked shadow.
- Details and addresses: `docs/evidence/render-sun-viewport-shadows-2026-09-27.md`.

### WorldPainter fog and sky (batch 2)

- **Where the keys come from.**
  `VisualDirector::VDWriter::_PrepareWorldPainterData` 0x827fd200 samples the
  camera (x, z) through `LayerMan::GetAttribData` 0x82ceaf50 for layers eFog
  15, eColorCube 11, eSky 16, eExposure 17, 18 (bloom) and eDistricts 4. Only
  fog and sky are consumed so far.
- **The lookup chain:**
  - tile (`GetTileHandle` 0x82cead18, 128 units);
  - `DoQuadTreeLookup` 0x82cea8b8 (10-byte nodes, inclusive `vcmpbfp`,
    first-containing child);
  - the dictionary;
  - `WorldPainter::Palette::kaTOC` 0x82312480 (363 entries, compared on the GUID
    high word);
  - `StringHash64(name)`. Layers 5 and 13 lower-case the name first. A miss
    gives key 0.
- **Blending.** `RenderMan::UpdateWorldPainterAttribs` 0x82848ba8 runs
  `BlendAttribCollection` for fog and sky, 5 s each, starting from `default`
  0xd7edbd362d7d2152.
  - `BlendAttrib`: lerp' = fmadds(1/T, dt, lerp), clamped to exactly 1 by fsel;
    out = fmadds(1-l, begin, end*l).
  - Without keys it falls back to the location's `material_fog` /
    `material_sky` references.
- **Output.** `UpdateRenderParams` packs:
  - `g_FogK1` = (1/(far-near), -near/(far-near), power, 0);
  - `g_FogColour` = (rgb*strength, -strength).

  The sky blend writes `material_sky/default`'s `m_params` storage, so only
  materials that share that storage read the blend (lane 7.x).
- **Port details.** dt = presentation frame time. The core samples keys in
  `SelectedVisualSnapshot.world_painter`.

### Sky dome and loose arenas (batches 2 + 3)

- **Loading.**
  - `GameModule::OneOffLoadTexture` 0x82735730 adds, in order:
    - `DIST_skybox_Textures.rx2`
    - `DIST_skybox.rx2`
    - `DIST_Water_Textures.rx2`
    - `DIST_Water.rx2`
    - `material_test.rx2`
    - (`DIST_ProxyWorld*` only when `kEnableProxyWorldStream` is 0)

    All are in `data\content\world\models\`, packed in `big\miscboot.big`.
  - `LooseAssetHelper::LoadAssets` 0x8272cdb8 turns '/' into '\', loads each
    file as an arena and sends `SendPresRegArenaMsg`.
- **What the port does.**
  - It loads the skybox pair as one collection in `PresentationScene.loose`.
    That collection is resident for the session and chained last in
    `resident_collections()`.
  - Standalone `.rx2` files go through `Rw4Arena::split_file_resources`, then
    `own_presentation_arena`.
  - The cache id is `string_hash64(path)`, a host-only identity.
- **Entities.** `WorldPresentation::AddRenderInstance` 0x827e4f00:
  - A new-format instance whose bone-name list starts with "skybox" becomes a
    `SkyPresEntity` 0x827d4258. It marks all islands visible, and its `Update`
    0x827d4480 re-centres on `g_vViewPos`.
  - One starting "testCu" becomes an `OccTestPresEntity`, which is never
    rendered. The port skips those instances.
- **Why no sky showed (09-27 log).** `material_sky` stores `effect` as an
  inline collection node (flags 0x40): the slot's own relocation points at
  `"sky"`. `resolve_collection_entry_location` dereferenced that slot like a
  stored pointer, so `resolve_material_manifest` fell back to `"default"` and
  the sky submission was Deferred, which is never drawn or counted. Inline
  nodes now resolve to their slot, as `Node::GetPointer` and the core
  `AttributeRuntime` do.
- **`DIST_skybox.rx2` contents** (offline check):
  - model `skyboxSphere`: 1 InstanceData, 2 models, 5 optimeshes;
  - materials:
    - `proxymoutains` (`environmentsimple.background`, lightmapped distant
      mountains);
    - two dummy ad signs (`environmentsimple.default`);
    - `skybox1` (`sky.default`).
  - Dome bounds ±7451 x -285..4472; far plane 7700 (`rendering` attribute).
  - All 8 textures are 2D DXT1 or A8R8G8B8 (decodable).

### PROXY world (09-27)

- **Data.** `worldproxy.big`, stream `PROXY_Pres`: 147 spatial collections
  `cPres_{x}_{z}_high_proxy`, each named by its own rounded hull centre,
  plus the parent `cPres_Global_proxy` (93 textures). A spatial collection
  holds one arena with one Environment instance (1-6 optimeshes; 482
  `proxyworld.default` materials in all) and 0-3 texture arenas (35 shared
  with a neighbour). Vertices: FLOAT3 position, SHORT2N UV0; every island is
  a triangle list.
- **Boot** (`GameModule::OneOffLoad`, `kEnableProxyWorldStream` default 1):
  `SyncLoadProxyWorldStream` 0x824df648 keys each XSS row
  `10000*x + z/10` (x, z = floor(hull centre + 0.5)), stable-sorts
  ascending, fills `mProxyCollectionMap` and loads each in that order;
  `SyncActivateProxyWorld` 0x824d9ef8 then registers every one.
- **Mapping.** `cStreamFile::Update` 0x824e0ba8 calls `AddHighCollection`
  0x824dc678 for every LOD-0 Presentation row: `"cPres_%d_%d_high_proxy"`
  from the row's rounded centre, `RwHash64String`, kept only when the proxy
  map holds it. BAM: 284 rows, 147 map (one proxy each), 137 have none.
- **Swap.** `LoadComplete` 0x824dc208 -> `NotifyCollectionPresReg`
  0x82d5c680 registers the high chunk, then deregisters its proxy in the
  same batch; the deferred path (`UpdatePresCollectionDefer` 0x82d5b5f0)
  deregisters it only after the chunk's last arena, so there is never a
  hole. `UnloadComplete` 0x824dc3e8 -> `NotifyCollectionPresDReg`
  0x82d580d0 deregisters the chunk, then re-registers its proxy.
  `ReceiveRegCollectionMsg`/`ReceiveDRegCollectionMsg` ref-count arenas;
  first use reaches `RegArena` -> `AddEnviroInstance` ->
  `WorldPresentation::AddRenderInstance`.
- **Render view.** `AddRenderInstance` puts an entity whose first mesh's
  draw order equals `m_iProxyOrderGuid` (`pass` 0xC9480BAF95B5238F, layout
  field 6, which is also the proxy material's DrawOrder) in `eSceneView`
  only; other world entities also join `eWorldShadowView`. The port has no
  world-shadow pass yet, so nothing is excluded.
- **Port.** The core state machine replays those messages; each generation
  applies its unloads, then its loads; the scene draws the registered
  proxies after the loose arenas, through the same queue sort (draw order
  6).

### Queue order, culling and the diagnostic path

- **Draw order.** `NativeRenderQueue` does a stable sort by the material
  binding's draw_order, then effect scope, pass count and material scope. The
  reference is a stable LSD radix pass. No other re-sorting happens: blended
  draws are not depth-sorted.
- **Culling.** `SceneRenderView::Cull` 0x828500a8 does the entity -> island
  shape (`WorldPresEntityOptimesh::CullMeshes_VMX` 0x827d0ae8,
  `FillMtlSrtKeyListWithCulledKeys` 0x827e3068). It is tested against the exact
  rasterizer clip volume, so culling never changes the image. Sky draws are
  never culled.
- **Multi-pass meshes.** `SceneRenderView::RenderMeshes` 0x82850670 draws
  single-pass meshes in sorted order and collects the rest. It then draws those
  pass-major: pass 0 of every multi-pass mesh, then pass 1, and so on, each
  mesh only for the passes it has. The port's multi-pass draws are the hair
  (3 passes) and ghosted skater parts (2), all after every single-pass object.
- **Diagnostic path.** Effects that are not ported draw raw UV0 diffuse
  (unlit, no fog). Without a diffuse texture they draw a solid palette colour.
  `tree`, `treeanimate` and `alphatestdefaultenvironment` get the manifest
  alpha test on this path. Every diagnostic draw keeps its technique's pass
  CULLMODE and depth bias; a multi-pass technique whose passes differ keeps
  the pass-reset state (no culling, no bias).
- **World space.** An effect whose shipped `defaultVS` writes oPos as the
  `g_matVP` dp4s of the fetched position takes world-space vertices: 29
  effects, listed in `render_inspection.rs` `WORLD_SPACE_EFFECTS` (with
  treeanimate and vertexanimate, whose wave offset is unported). Their
  draws use the identity instance matrix; the rest keep the instance
  transform.
- **Identical depth.** The original's world programs share one oPos
  instruction sequence, so coincident vertices of different effects tie and
  LESSEQUAL keeps the later sorted draw. Every world position output in the
  port is `@invariant` (environment effects through `world_clip`).
- **Camera-relative positions.** `Globals` holds the clip matrix rebased at
  `g_vViewPos` (`CameraRelativeClip`, f64 on the CPU); every GPU position
  is transformed as an offset from the camera (map, sky, skater bone rows,
  shadow casters). City coordinates are thousands of metres out, and the
  direct f32 product cost 11-74 mm of depth order at 3-40 m. CPU culling
  keeps the true matrix.

### Video screens (10-19)

- **Native.**
  - `cModelInstance::Resolve` 0x82853de8 gives every effect scope named
    `videoscreen_default` (the `videoscreen` fx's `default` technique;
    `Technique::Technique` 0x82e1cfb8 names scopes `<fx>_<technique>`)
    `m_PostFX.m_mainRt`'s colour texture as `i_diffuse`. It sets it after the material's
    own channels (0x82853f90..0x82854028).
  - `ApplyFastUber` 0x828469b8 renders the graded frame back into the main target and
    resolves it (0x82846ca4), before `Blit_ToDeviceRT` and the HUD. A screen therefore
    shows the previous frame, graded, without the HUD.
  - The main target is 1152x640 (`PostFX::Init` 0x8284a908..944).
- **Data.** BAM has two `incandescent.videoscreen` materials (manifest `videoscreen`), in
  `cPres_-100_-2100_high`, the stadium interior. Their programs are baseincandescent's.
- **Port.**
  - `gpu_renderer/main_target.rs` holds a 1152x640 texture in the surface format.
  - After the frame's PostFX, the final pass's scene texture is copied into it.
  - The `videoscreen` draws bind that texture as their diffuse
    (`TextureIdentity::MainTarget` in `render_inspection.rs`).
  - Evidence: [video-screens-2026-10-19.md](evidence/video-screens-2026-10-19.md).

## Missing (not ported)

Counts are diagnostic draws from the 09-25 run (two city generations).

Channels and states below are from each manifest's `default` technique. The
shader inputs are from its UPDB source. None of these has had a line-by-line
review yet.

| Effect | Draws | Channels | What it needs |
|---|---|---|---|
| `treeanimate` | 198-225 | diffuse, lightmap, chromaticity | Wind VS: `g_fAnimationTime` (writer known), `m_params[0..2]` (array of 3), `g_ViewDotLight` (formula known), `GetLightMap_SingleTap`. Alpha test GREATEREQUAL 30, COLORWRITE RGB, Z write, no blend |
| `vertexanimate` | 20-51 | same | Same inputs and states as treeanimate |
| `baseincandescent` | 94-190 | lightmap, chromaticity (no diffuse) | `outcol *= lerp(1, GetLightMap(...), m_params[0].x)`. `GetLightmapValue` averages four `tfetch2D` taps at ±0.5-texel offsets (`FetchValidOnly`) before squaring. Only CULLMODE FRONT (opaque) |
| `zbiasincandescent` | 53-76 | same | As above + blend ONE/ONE ADD (alpha too), DEPTHBIAS -0.0001, SLOPESCALEDEPTHBIAS 1, Z write, RGB. The bias conversion is done (`raster_state.rs`); the diagnostic draw already carries it |
| `zbiasincandescentbacklit` | 45-58 | same | As baseincandescent + the same depth bias + alpha test GREATEREQUAL 16, no blend |
| `scrollincandescent` | 12-21 | same | UV += `g_fAnimationTime` * (`uAnimationSpeed`, `vAnimationSpeed`); opaque |
| `building_ceiling_worldspace` | 51-105 | diffuse, specular, environment, normal, diffuse2, lightmap, chromaticity | `floorHeightMeters`, `floorCount`, interior-strip parallax; `m_params[0]`; opaque |
| `baseterrain` | 4-9 | diffuse, specular, normal, diffuse2, normal2, specular2, macrooverlay, lightmap, chromaticity | Two-layer slope blend, `diffuse2UVScale`, `m_params[0..1]`, `GetCalculatedSpecularTerm`; opaque |
| `water`, `flowingwater` | 2-5 | diffuse, specular, environment (ANISOTROPIC), normal, (water: normal2), lightmap, chromaticity | Same PCA normals as the ocean (`g_fPcaMean`, `g_fPcaWeights{R,G,B}_{0,1}`), `g_fAnimationTime`, `g_ViewDotLight`, `diffuse2UVScale`, `m_params[0..3]`; opaque |
| `ocean` (`DIST_Water.rx2`, not loaded) | - | environment, normal, normal2, macrooverlay, lightmap, chromaticity | PCA normals from `cPCAWaterAnimationData` (Init/Update; `Update(dT)` runs right after the animation-time write), `ward_anisotropic`, `m_params[0..2]`; `material_ocean/default` exists in the Vault. Alpha 0, opaque |
| `advertisement` | 31-67 | - | **No UPDB source** (microcode `.fpo`/`.vpo` only); would need microcode translation |
| `trafficlight_one`/`_two` | 4-8 | diffuse | `g_TrafficLightsStatus_1/2`; no blend, no alpha test, RGB. Unverified writer lead: `WorldPresentation::DoSimSubmit` calls `TrafficLight::GetCurrentState` / `GetCurrentPedState` |
| Skater cloth (ROPA) | - | The `*_ropa` parts (tutorial_male's hoodie) become a CPU cloth simulation after `ClothInit` 0x827d7d88: `ROPA::Effect` particles, constraints, bone drivers and sphere/tube/plane collision, gated by `cv_EnableRopa`. The port only draws their GPU branch, so sleeves can intersect arms |
| Character effects (`defaultcharacter`, `cac_*`, `*_ropa`, hair, ...) | - | Character colour is diagnostic. The pass states are original: CULLMODE for every pass, the `ghost` passes, and `defaulthair`'s 3 `default` passes with alpha `i_alpha`(UV1) * `i_params[0].x`. Lighting (`CalculateCharacterLighting`, SH irradiance) is not ported |
| Living world / vehicles (`defaultlivingworld`, `livingworld_stamp`, `vehicle*`) | - | `pedestrian_render.rs`, `vehicle_render.rs` |
| Crowd imposters (`crowdimposter`, livingworld_stamp `crowd`) | - | `gpu_renderer/crowd.rs`. `RenderCrowdTargets` (after the shadow cascades) draws each target's no-sim pedestrian into a 364x310 cell of a 1280x720 atlas with the `crowd` technique (livingworld_stamp's own pass; other effects fall back to `default`), the pass's stencil 255 resolved into a mask. `RenderCrowdImposters` (after the 3D front end, before the particles) draws camera-facing quads: colorized by the texel's alpha, fogged, toned, alpha from the mask, alpha test 30, SRCALPHA/INVSRCALPHA, depth write |

**Systems:**

- **Sun in NIS/front end.** `mManualModeNIS` returns a stored sun (+0xA0);
  the port samples the map at the camera without a skater.
- **PostFX.**
  - Auto-exposure: `PostFX::Tonemap` 0x82828f68 reads the downsampled bloom
    buffer back each frame and rewrites `g_envattributes[2].x` =
    clamp(((target - avgLum)*damping + 1)*exposure, min, max). The blends are 5
    s for exposure (key 0x2b00cf9bb190ce8d) and 2 s for bloom.
  - Bloom, and the colour cube (eColorCube keys are already sampled but unused).
- **Shadows still missing.** The world-shadow pass (`m_pWorldShadowMap`,
  `CalculateWorldShadow`) and character self-shadowing need the character
  lighting port. DMOs, pedestrians and other skaters cast once they are
  rendered (vehicles never cast).
- **Deck AO.** `PostFX::RenderAOBoxes` 0x828465b0 (`gpu_renderer/deck_ao.rs`): after the
  uber or visual FX pass, a box (0.2 x 0.2 x 0.6 of the deck matrix, `GetDeckMatrix`: the
  `skateboard_root` pose, hips and root) under each skater the game state draws multiplies the
  target by `deckao_PS`: the depth's camera-space point in deck space samples
  `board_occlusion.Texture` (x, z times `DeckAO_XLen`/`ZLen`), lightened by the height
  (`DeckAO_HeightFallOff`), the stencil (characters stay unshaded) and a positive first bloom
  coefficient. `g_FOV` comes from the drawn projection, so other window shapes match. The
  bone is found by name in the presentation hierarchy (`InitAnimation` 0x827e1d54). The
  texture is read through `m_pLinearClampSamplerState` (`PostFX::Init` 0x8284a8b8): mip
  filter `D3DTEXF_NONE` (2 on the 360, the base map), so every fetch uses the full-size level
  and a point off the board clamps to the white border; sampling the smaller mips instead
  greys the whole box far away. With MSAA each depth and stencil sample is shaded and the
  pixel takes their mean, so a character's edge pixels are not shaded as all ground.
  PC deviation: the darkening (one minus the original's factor) is 1.2 times the original's
  (`STRENGTH` in `deckao_PS`'s port), and with PostFX off the boxes still draw, into the scene the
  frame drew directly, with the camera planes' depth constants and no bloom term.
  `board_occlusion.Texture` (32x64 R5G6B5, unsigned) is kept resident with the other
  `globaltextures.rx2` PostFX textures; `SetTexturePointers` 0x82828728 leaves it null when
  absent. With the shipped values (XLen 1.6, ZLen 0.9, HeightFallOff 5) and the ground about
  6 cm below `skateboard_root`, a rolling board darkens the ground under its centre to about
  0.6 of its colour, mostly hidden under the deck from the usual camera.
- **Other globals:** `g_vCubeMapCentre`, `g_vViewDir`, `g_gammapower`.
- **Character part order.** Inverse binds fill in `BuildTInstanceArray`'s
  component-name hash order (09-27). Its Beard skip (no Beard when a
  Rostral exists) and the model/LOD order inside one component are not
  ported; recipe order is kept there.

## Deliberate deviations

- **CSM formats and numerics.** G16R16 targets become Rg32Float (blur) and
  Rg16Float (atlas); the D24S8 target is read by aspect; `Inverse` and
  `rsqrte` are exact-math stand-ins (ULP level). The camera is the provisional
  `CameraMan` matrix, as for `g_vViewPos`. Without clamp-to-border the atlas
  clamps to its edge texel.
- **Widescreen.** The original only renders 16:9. Other window shapes widen
  the horizontal (wider window) or vertical (narrower) field around the same
  16:9 view, keeping pixels square; 16:9 stays bit-exact.
- **Video screens.** The main target is not drawn at 1152x640. It is the window-size
  scene's central 16:9 view (the original's view), box-filtered to 1152x640 with four
  bilinear taps a texel.
- **Exposure fixed** at the Vault 1.0 (no auto-exposure).
- **No stencil attachment.** The hair and ghost passes' STENCILPASS REPLACE
  255 writes are dropped; nothing reads the stencil yet.
- **Depth bias precision.** The constant is exact for depths of 0.5 and up
  (farther than twice the near plane); the slope term is per pixel at the
  port's resolution, as the hardware defines it.
- **PC enhancements:**
  - anisotropy 16 instead of the manifests' 4;
  - Resolution Scale (Display Settings, 50% to 200% in steps of 10, saved in the display
    chunk, version 6): the scene's depth, MSAA colour, PostFX and final-pass scene targets are
    that percentage of the render size (the longer side capped at the device's largest
    texture). `Blit_ToDeviceRT` brings it to the window: one sample at or below 100%, above it
    four warped samples a quarter of a window pixel apart (exactly the 2x2 texels at 200%). The
    HUD, movies and fades draw on the window afterwards. A frame without a final blit renders at
    100%, and a scale the GPU cannot allocate falls back to 100% until the setting changes. At
    100% every target and the blit are as before;
  - 4x MSAA (`SKATE2_NO_MSAA` gives 1x); the character programs interpolate their
    attributes at the centroid, so an edge pixel whose centre lies off the triangle does not
    sample past the surface (UVs, normals, the shadow lookup's world position);
  - VSync (Display Settings, display chunk version 7; earlier saves take it from their cap,
    whose Refresh Rate alone used to wait for the refresh): on presents Fifo, off Immediate
    (else Mailbox). The Frame Rate Cap applies either way: 60 and 120 wait in `FrameCap`, and
    Refresh Rate without VSync waits the monitor's interval (`refresh_rate_millihertz`, read
    again each second, 60 Hz when unknown). While the window does not have the focus, VSync
    presents as off and `FrameCap` waits at least the monitor's interval instead: a Fifo
    swapchain whose window another one covers can stop taking frames, and a frame held in
    its present holds the simulation (the SIM thread's next tick waits for the window to
    take the last one), long enough online for the server to drop the game as frozen. Any
    swapchain can stall that way (wgpu's acquire gives up after a second), so an unfocused
    frame that gets no image, or waits 250 ms or more for its image and present, puts the
    window in the background (`render_window.rs`): its client frames run on the minimized
    window's 8.3 ms timer, draw nothing and count as shown, so the simulation keeps
    wall-clock pace and fade-covered loads go on. Focus, or the cursor moving over the
    window, brings the redraws back;
  - `SKATE2_NO_VSYNC` for uncapped presentation (VSync off, no cap).
- **Backends.** Every wgpu backend is allowed (Vulkan first, then DX12), and the DX12 shaders
  compile with FXC, which ships with Windows: wgpu's automatic choice takes any `dxcompiler.dll`
  on the PATH (a Vulkan SDK's, say), and an old one without `dxil.dll` fails every shader, so the
  device never starts. `WGPU_BACKEND`, `WGPU_DX12_COMPILER` and wgpu's other variables override;
  `SKATE2_WGPU_LOG` prints wgpu's warnings and errors, with `WGPU_VALIDATION=1` also the
  validation layer's. A panic in the window thread shows the error, the adapter and the crash
  report's path in a message box.
- **Bundled draws and the stencil reference.** wgpu sends the stencil reference to the driver
  only while a pipeline that reads it is bound, and replaying a render bundle binds its pipelines
  without it. `WorldShadowView::Render` binds the world caster pipeline before setting
  `COVERAGE_STENCIL` and executing its bundle; otherwise every bundled caster draws with an unset
  reference (Vulkan validation: `VUID-vkCmdDrawIndexed-commandBuffer-02701`).
- **Precision and filtering:** `half` is evaluated as f32; there is no
  TRILINEARTHRESHOLD equivalent.
- **Composite alpha:** the surface is non-sRGB and prefers Opaque composite
  alpha, because effects write alpha 0 on purpose.
- **Missing channel texture:** the original keeps the previous draw's texture
  (`DispatchParamsHelper` skips a null param). The port sends that draw down
  the diagnostic path instead, with a logged reason.
- **WorldPainter dt** is presentation frame time, not the original's frame
  dT source.
- **Loose sky load is non-fatal.** It logs `loose presentation assets not
  loaded (...)` and continues; the original loads it unconditionally.
- **PROXY boot timing.** The manager reads the PROXY XSS before the world
  stream starts, as the original does, but the collections load when the
  renderer exists (texture uploads need it), after the initial high
  chunks, and the world stream's `AddHighCollection` calls can arrive after
  those chunks registered. Activation still precedes every high
  registration the port applies, and a late mapping hides its proxy at
  once, the state the original reaches; a registration change alone
  rebuilds the draw list.
- **PROXY swap granularity.** A high chunk registers whole at the
  generation swap (the direct `NotifyCollectionPresReg` batch); the
  deferred per-frame arena registration is not modeled.
- **PROXY textures** stay resident all session. A registered proxy's
  textures are always registered in the original, so the image is the
  same.
- **Not loaded on purpose:**
  - `DIST_Water` pair: `ocean.fx` is unported, so it would be a flat
    placeholder plane.
  - `material_test.rx2`: its only instance is an occlusion test, never
    rendered.

## Unverified assumptions (check in IDA before relying on them)

1. **`SendPresRegArenaMsg` routing.** Does the loose sky arena reach
   `WorldPresentation::AddRenderInstance` (SkyPresEntity) or
   `LooseWorldPresentation::AddRenderInstance` 0x827e10b0 (plain
   LooseWorldPresEntity)? Either way it draws with `sky.fx`; only culling
   differs.
2. **Manifest states as applied at runtime.** Resolved 09-27 for parsing:
   `cFxParser::EndElement("Pass")` 0x82e1f778 resets blend (no blend, no
   alpha test), depth (ZENABLE, ZWRITEENABLE, LESSEQUAL), raster (cull NONE,
   no bias) and every sampler (WRAP, LINEAR, POINT mip) after each pass;
   `DEPTHWRITEENABLE` aliases `ZWRITEENABLE`. `shadowTexture_CSM0`,
   `shadowTexture_World`, `aliased_shadowTexture_CSM0`, `lightmapLUTTexture`
   and `lightingTexture` get named sampler states instead. Draw-time overrides
   beyond `SetCurrentEffect` were not traced.
3. **Transparent pass.** Is there a separate transparent pass, or
   back-to-front sorting, in `SceneRenderView` beyond the sort key?
4. **`environmentsimple.background` shader.** Resolved offline 09-27: both
   `environmentsimple.background` and `.default` resolve through the layout
   pointer to `defaultenvironment` (ported Default).
5. **Sun texture fetch.** `tex1D` of a 2D texture is assumed to fetch row
   v=0 on Xenos (sky sun texture).
6. **Texture fetch-constant swizzle/sign.** Not retained by the decoder. The
   supported DXT1/DXT5/A8R8G8B8/R5G6B5 formats assume the standard swizzle.
7. **Batch 2 scalar channel names** are confirmed by the 09-25 log (Base draws
   route). Batch 3 decal/transparent channel names are unconfirmed until a
   run.
8. **DoSimRestore cadence.** Native restores from a free-running render thread,
   gated by `DataProvider::DoRestore` 0x82995ea8 provider bits. The port
   restores once per completed sim tick. The present interval was not traced.
9. **`.updb` source vs shipped microcode.** The CSM atlas V mapping differs
   between the `.updb` HLSL and the shipped `.fpo` programs; the port follows
   the microcode there. Other literals taken from `.updb` text match the
   `defaultenvironment` microcode's literal table, but the other effects were
   not compared line by line.

## How to check a run

`play-quiet.bat` suppresses these lines (`--quiet`). Capture them with:

```powershell
$env:SKATE2_GAME_ROOT='<game folder>'
target\release\skate2-pc.exe --live-skating 2> render-log.txt
```

| Line | Meaning |
|---|---|
| `original environment lighting: Vault g_vLightDir=... (sun-direction map requested=..., sampled per frame at the skater hips) ... WorldPainter blends over N material_fog / M material_sky ...` | Vault inputs loaded once |
| `original environment shading draws={...}` | Per-effect draws running the original shader (expect Default, AlphaTest, Base, Decal, Transparent, Sky, DecalZBias, ProxyWorld). Ends with `deferred default-effect submissions=N`: effects that resolved to the literal `"default"`, which are never drawn |
| `PROXY world: 147 collections in load order` | The manager read `PROXY_Pres.xss` before the world stream |
| `PROXY retained collections=148 unique texture uploads=145 spatial geometry=147` | Every PROXY collection loaded; textures uploaded once |
| `PROXY world activated: N registered proxies beside M high collections` | Activation plus the initial high chunks: N is at most 147, less the proxies of those chunks whose mappings already exist |
| `ported-effect draws kept diagnostic by reason={...}` | Why a ported effect fell back (missing channel, stream, `m_params`) |
| `diagnostic draws by effect={...}` | Unported effects by manifest; use it to pick the next port |
| `occlusion-test instances not rendered=N` | testCu skips |
| `WorldPainter key lookup failed (...)` | Once; fog/sky then use the location references |
| `loose presentation assets not loaded (...)` | Sky arenas could not be owned or bound |

## Research tools

- `tools/extract_big.py ARCHIVE.big OUTDIR [substring...]`: named BIG +
  RefPack. For example, `miscboot.big` holds the loose models and `render.big`
  the shaders and manifests.
- `tools/updb_source.py`: HLSL from `.updb`. The `.updb` text is not always
  what shipped; check the microcode for anything that matters.
- `tools/xenos_disasm.py`: Xenos microcode of `.fpo`/`.vpo` containers, or of
  shader blobs dumped from the xex, with their literal constants.
- `tools/vault_dump.py VLT BIN show <class> [collection...] [--names file]`:
  Vault classes and collections. The Vault is inside `data/db/db.big`, so
  extract `skater.vlt` / `skater.bin` first.
- `tools/match_attribute_hashes.py`: `string_hash64`, for class/collection
  keys (e.g. `material_ocean` = 0xa91b760195c3feda).
- The 09-25/26 session used scratch helpers, not in the repo:
  - an effect viewer printing manifest states, samplers and the HLSL body;
  - an `.rx2` texture-header reader mirroring `rw4.rs` + `material.rs`
    `Texture::parse` (dictionary at resource-0 +48; entries of 24 B;
    `TEXTURE_TYPE_ID` 0x000200E8; fetch words at object +28).

## Suggested next order

1. A compile/run of the 09-27 batches (sky fix, cull/bias/hair/decal,
   PROXY world).
   Read the log lines above.
2. Close the unverified items 1 to 4 in IDA.
3. Add `g_fAnimationTime` and `g_ViewDotLight` to the globals. Then port
   `treeanimate`/`vertexanimate`, then the incandescent family (its depth
   bias already converts through `raster_state.rs`),
   `building_ceiling_worldspace` and `baseterrain`.
4. `cPCAWaterAnimationData` (shared by `ocean`, `water` and `flowingwater`),
   then those three effects, then load the `DIST_Water` pair.
5. PostFX (auto-exposure, bloom, colour cube, deck AO boxes), character
   shaders (then the world-shadow pass and character self-shadowing), DMO
   rendering (then DMO shadow casters).
