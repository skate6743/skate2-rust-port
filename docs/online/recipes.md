# Recipes (skater appearance)

Every player's look (created skater parts, or a pro's marquee recipe) travels as a *recipe*: a
packed `Sk8::CAC::RecipeData`. Skate 2 exchanges recipes through the RB over the game
connection (Skate 1 used Plasma's blob service instead, per Arcadia). The flow state is
`Sk8::Flow::StateOnlineRecipe`; addresses are `sk82_na_m.xex`. Messages:
`GameRecipeRequest` 23, `GameRecipeHead` 24, `GameRecipeData` 25 ([messages.md](messages.md)).

## 1. Uploading your own

`DoUploadRecipe` 0x8275ee18, run when the RB asks (`GameRecipeRequest` reaching
`StateOnlineGame`, [game-flow.md](game-flow.md) §3):

- a **created skater**: copy player 1's CAC parts record (`CACBackEnd::GetPlayerPartsRecord`,
  marked dirty) → `UploadRecipe`;
- a **pro skater**: `LoadMarqueeRecipe` 0x827554b8 loads the marquee recipe file
  (`Character::GetRecipeFileName(name, Category_Marquee)`) into the same record, setting
  `stance` from the marquee's layout byte 16 and `hatType` from its attributes, then
  `UploadRecipe`. No loaded player falls back to marquee 0x2275B90050166969.

`UploadRecipe` 0x8274d6e8: `RecipeData::Pack` into an 8240-byte buffer, send
`GameRecipeHead(local peer id, size, Sk8::GetCRC(data))`, then `ceil(size / 1024)`
`GameRecipeData(local peer id, chunk size <= 1024, chunk index from 0)`, all to the RB.

After uploading, the client downloads its own recipe back (`DoDownLoadRecipe(local peer)` →
`LoadLocalRecipe`), so it renders itself from exactly what its peers receive.

## 2. Downloading a player's

`DoDownLoadRecipe(peer)` 0x82775c40:

- if the peer has a recipe queued for removal (they left and came back), unload that one
  first ("Player reconnected, deleting old recipe!");
- if six recipes are cached (`mRecipeMap`), unload the oldest queued for removal first;
- then `RequestRemoteRecipe` 0x827739a8: send `GameRecipeRequest(peer)` to the RB and wait for
  the head. Size 0 means no recipe: load the default skater. Otherwise expect
  `ceil(size / 1024)` chunks.

`StateOnlineRecipe::OnMessage` 0x8276e0d0 keeps the head and appends chunks, asserting that each
chunk is for the requested peer and numbered one after the previous. `DownloadRemoteRecipe`
0x827723d8 joins them when all have arrived, `RecipeData::Unpack`, → `LoadingRemoteRecipe`
0x8276fb40: a valid record → `CreateSerializedLoader`, otherwise the default marquee loader;
the loader is registered in `mRecipeMap[peer]` and, when loaded, optionally morphed
(`kbMorphRecipes`, `cMsgPresCharacterMorph`). The CRC in the head is not checked here.

`StateOnlineGame::UpdateGameInfo` applies each slot's loaded recipe
(`CharacterRecipeLoader::ApplyAttributes(slot)`) when a reset builds the players.

## 3. When downloads happen

- On joining: every player already in the game (`OE_GameJoined` queues them).
- When a player joins: `OE_PlayerJoin` queues the newcomer.
- `LobbySkate` downloads the queue one at a time and only then answers the RB's load sequence
  (`GameLoadRequest`) and applies the join reset ([game-flow.md](game-flow.md) §5). This is the
  bidirectional "recipe mesh" barrier Arcadia rebuilt: the joiner must have everyone's recipe
  and everyone the joiner's before the reset that adds the joiner to the lockstep.
- On leaving: `HandlePlayerLeave` queues the leaver's recipe for removal;
  `LeaveGameFinal` unloads them all.

## 4. For the port

Recipes are plain data and independent of the lockstep. The same exchange (upload on join,
fetch per player, cache six, default when missing) fits any transport, and the port already
reads and writes the same record: `skate2_formats::skater_profile::RecipeData`
(`crates/skate2-formats/src/skater_profile.rs`, after `RecipeData::Pack` 0x82b657b0).

A look this console cannot build whole, such as one with parts of a linked Skate 3 data folder
it lacks, shows as the default skater: the marquee `0x2275B90050166969` that
`StartLoadingDefaultRecipe` 0x82748fa0 loads for a player without a recipe, which is also the
skater of a save with no created skater. The look is the marquee's and the pose stays the
remote player's; the skeleton comes from the local skater, as every skater's is alike
(`Character::default_skater`, `crates/skate2-core/src/session/online/remote_skater.rs`).
