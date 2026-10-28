# Downloadable content worlds: focus selection and render location (2026-10-27)

Reference: debug `sk82_na_m.xex`, the shipped `skater.vlt`, and the six Skate 2 content packages.

## 1. Simulation focus over several stream files

- `cAssetStreamSystem::FindCollections` 0x824dfea8: with the focus's own stream XML
  (`mUseStreamXml`) it takes that collection list. Otherwise it runs
  `cStreamFile::FindCollections` 0x824df068 on the first stream file of the type and merges
  each later file's entries. An entry already held is replaced; a new one joins while fewer
  than 16 are held. The whole is then sorted by `FocusLODDistanceCompare`, and offline
  without focus XML the Sim count is capped at `kSimMaxFocusCollections`.
- `cStreamFile::FindCollections`: with its own `mXmlCollectionMap` and `kEnableStreamXml`, it
  takes the nearest collection inside the small box and selects that collection's XML
  neighbourhood. When that yields nothing, it falls back to the octree radius query.
- The content packages ship no `data\stream\<world>_Sim.xml`, so an additional world's file
  always takes the radius query. BAM's file uses its XML.
- Port: `SelectedSessionTerrain::spatial_selection` runs BAM's file through its XML (when
  enabled) and merges the loaded additional world's radius query. With BAM alone the result is
  the previous one.
- Every content global locator and `_Sim.loc` start lies inside its world's Sim hulls; BAM
  selects nothing there.

## 2. World render location

- `StateLoading::LoadAdditionalWorldStream` 0x82772680, `TeleportLoadState` 0x8275fdb8 and
  `GameModule::GameLoad` 0x8273be50 send `cMsgSetWorldRenderLocation` with
  `sGameInfo.additionalWorldLocation`.
- `LooseWorldPresentation::SetWorldRenderLocation` 0x827d53a8 runs
  `FindCollection(render_locations, key)` into +0x4454.
- `GetWorldRenderLocation` 0x827d0cc8 returns, in order:
  1. for `mLooseWorld` 1 or 2 (Trick Guide, CAS), `6568f34643066684` or `664b0d048078fe31`;
  2. the collection's key;
  3. without a collection, the marquee viewer's `f17b6509256ee58b` or the default
     `ec763034105b866b`.
- `RenderMan::UpdateRenderParams` 0x82852780 changes `m_RenderLocation` when the key differs,
  then calls `SHLightingMan::UpdateDefaultLightProbe`.
- `render_locations` layout bytes: `use_worldpainter` 0, `use_world_sh_probes` 1,
  `use_world_onto_object_shadow` 2, the sun-direction-map flag 3.
- `UpdateWorldPainterAttribs` 0x82848ba8 takes the WorldPainter keys only with
  `bWorldPainterInfo` and `use_worldpainter`. Otherwise it takes the location's `material_fog`,
  `material_sky`, `auto_exposure` (`rendering`), `colour_cube` and `bloom` (`post_fx`)
  references.
- The content vaults define `render_locations` for `dlc_parkade`, `dlc_elementary_school`,
  `dlc_community_center` and `dlc_rob_fantasy_park`. Each has parent `dlc_default`, a root
  collection of `skater.vlt`. The Maloof Money Cup has none and keeps the default location.
- `dlc_parkade`:
  - sun (0x3ed09a24, 0x3f12f976, 0x3f35cfff);
  - sun map off, WorldPainter off;
  - fog `be871da589aae0e9`, from the content vault.
- The city's default location keeps the WorldPainter and the sun map on.
- The `fe_locations` RefSpecs of the content locations are audio collections (reverb,
  emitters, ambiences), not rendering ones.
- Port:
  - the session's `world_render_location`;
  - `Inspection::set_world_render_location` reads the location through an `AttribDb` over
    `skater.vlt` and the content vaults;
  - `EnvironmentLighting::with_vault_collections` adds the content's fog, sky, bloom,
    exposure and colour cube collections;
  - `with_world_render_location` applies the location;
  - the `use_worldpainter` gate is in `render_inspection`.
