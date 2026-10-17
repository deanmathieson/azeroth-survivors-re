# Spike: can the engine load Molten Core (map 409)?

Status: spike done (RAIDS.md build plan, M0). Static checks only: the game was not launched, so there are no screenshots yet.
Checks: `crates/benilla-formats/tests/molten_core.rs` (data-gated; `cargo test -p benilla-formats --test molten_core`).

## Verdict: go

Use map 409 for the raid. The engine already streams WMO-only maps from start to finish. Molten Core's data is complete in the chain, and its caverns are as roomy as the arenas we already ship (Fire Plume Ridge has a leash of 32). The rooms are actually roomier than the MC-themed fallbacks on map 0, apart from the open Burning Steppes fields.

A small amount of survivors-side work is needed before it's playable (see "What's missing"). None of it touches the renderer.

## What loads

- **Map.dbc 409** is "Molten Core" in directory `MoltenCore`, with instance type 2 (raid).
- **`MoltenCore.wdt`** has MPHD flags `0x1` (WMO-only) and no ADT tiles. It has one global WMO, `world\wmo\dungeon\az_blackrock\blackrock_lower_guild.wmo`, at position (0, 0, 0) and rotation (0, 0, 0), with doodad set 0 and name set 0.
- **The WMO** has 12 groups, all interior, and 12 portals (24 MOPR refs). It has 10 MFOG fogs, 252 doodads in 1 set and no skybox. It builds into 84 render submeshes with 33,573 triangles, and has 15,735 walking-collision triangles. Every render texture and every doodad model is present in the chain.
  - Five groups (0, 3, 6, 9, 10) carry MLIQ magma, which is the same `LiquidKind::Magma` path that Fire Plume Ridge's lava already renders through.
  - This is a small building: Stormwind is far heavier. Frame time should be a non-issue (still to be measured in a play build).
- **Placement:** the global WMO goes through the same `placement_rotation` as any MODF. At rotation 0 that is a half-turn about up, so world = (−x, −y, z) of model space. The server's MC entrance areatrigger lands at (1096, −467, −104.6), which maps into group 0, on its floor at −104.4. So our world coordinates match the server's.
- **The runtime path already exists:**
  - `benilla-world/src/terrain_stream.rs` (around line 531) registers a WDT's global WMO as one placement (`GLOBAL_WMO_UID`). It exempts WMO-only maps from the "off the tile grid" warning, and gates the loading cover on that one placement and its doodads.
  - `benilla-assets/src/wmo.rs` builds the WMO collider for any placement, so the avian walking colliders cover MC too.
  - Survivors sets `CurrentMap` from `MapDef.map` (`survivors/hero.rs` `follow_map`), and the streamer resolves the directory through Map.dbc. So **a `MapDef` row with `map: 409` is the whole switch**, and no formats or world change was needed.
- **The tile loaders** (`load_tiles_around`, `MapTiles`) correctly find no terrain for "MoltenCore". There is no map-by-name tile load to fix: the streamer's WDT branch, not the ADT loader, is what brings in WMO-only maps. This is pinned by test (c).

## Stage centres (map 409, raw WoW world coordinates)

Each centre was found and checked by flood-filling the collision floors. A floor is walkable within 50°, may step at most 2 yd per 2 yd, must have a roof over it, and must have no lava over it. "Open" is the radius the flood covers in every direction, and each leash is a few yards inside it. "Clearance" is the height from the floor to the ceiling.

Only Ragnaros (838.5, −829.7) and Majordomo (758, −1177) are anchored to remembered 1.12 spawns, and the entrance is anchored to its areatrigger. The other room names are geographic guesses. Check them against the core's `creature` spawns before tuning.

| Stage | Centre (x, y, z) | Open | Leash | Clearance |
|---|---|---|---|---|
| Raid start (entrance) | 1080, −480, −108.2 | 16 | 14 | 21 |
| Lucifron's cavern (T1/B1) | 1096, −732, −149.1 | 52 | 45 | 54 |
| Magmadar's lair (T2/B2) | 1112, −1008, −187.3 | 27 | 24 | 27 |
| South-centre cavern (Golemagg?) | 868, −1008, −196.4 | 38 | 34 | 61 |
| West cavern (Garr?) | 732, −520, −217.0 | 44 | 40 | 30 |
| North-west cavern (Geddon / Shazzrah?) | 644, −816, −208.5 | 49 | 45 | 40 |
| Centre cavern (Sulfuron / Gehennas?) | 892, −600, −203.6 | 38 | 34 | 34 |
| Majordomo's camp (B9) | 736, −1168, −120.4 | 34 | 30 | 58 |
| Ragnaros, dry ledge north of his pool | 844, −772, −226.0 | 18 | 16 | 113 |
| **Ragnaros, lava walkable (B10)** | 828, −852, −229.0 | 32 | 30 | 114 |

Ragnaros's lair is a ring of rock broken up by lava channels, so its largest dry patch is only 18 yd. If magma counts as a hazard rather than a hole, the ring and the channels make one 30 yd arena, 25 yd from his spawn. That is the fight RAIDS.md describes, and it needs lava damage (below).

To list more rooms: `cargo test -p benilla-formats --test molten_core -- --ignored --nocapture survey`. Optional settings: `MC_NEAR=x,y,r` limits the search to one area, and `MC_WADE=1` counts lava as walkable.

## What's missing to render and walk it in survivors

1. **Ground probes start above the ceiling.** `enemies.rs:255` `ground_on_collision` casts down from `max(y, floor) + 64`. That is above MC's 21–61 yd ceilings, so spawns, treasure, companions and party placement (16 call sites) would land on the cave's roof. The fix is a ceiling-aware probe: cast from the hero's height plus a step, and fall back to the current probe outdoors. That is about 1 day with tests. `body.rs` is already safe, because it searches down from the feet.
2. **Lava is only decoration.** Survivors has no environmental damage. Magma pools need to become a damage zone for heroes, read from the WMO MLIQ the liquid query already exposes, and foes should be kept or paced out of them. Ragnaros's arena depends on this. About 1–2 days.
3. **Camera under a roof.** The WoW-style camera collides with geometry. Check the classic top-down view against 27–30 yd clearances (Magmadar, Garr): it may need a lower pitch or a camera-only ceiling fade. About 0.5–1 day.
4. **Terrain heightfield is absent.** WMO-only maps have no ADT height, so `MapDef.floor` (set to each centre's z above) is the only fallback until colliders spawn, and the loading cover waits for the placement anyway. Nothing to do beyond setting `floor` correctly.
5. **Lighting and fog** come from WMO MOCV and the 10 MFOG records on the interior path. `Light.dbc` for 409 falls back to the map's global row, or to record 1 if there is none. Look at it in a play build. The midday lock (`MINUTE_OF_DAY`) doesn't matter indoors.
6. **Unverified until a play build:** portal culling across all 12 groups, the minimap (WMO-only maps have no ADT minimap tiles, so expect it to be blank), and frame time with a full horde.
7. **Moving between rooms** stays on map 409. Changing stage repositions the hero with no worldport, so there is no loading screen between pulls.

## Camera check still required

Do not change the camera before a play build. Check the classic top-down view in Magmadar's Lair
(27 yd clearance), West Cavern / Garr's room (30 yd), Centre Cavern (34 yd), and Lucifron's
Cavern (54 yd). The low-clearance rooms may need a lower pitch or a camera-only ceiling fade.

**Estimate: about 3–5 days** to MC arenas you can play: `MapDef` rows (0.5 day), the ground probe (1 day), lava damage (1–2 days), the camera check (0.5–1 day), and an M0 play build with screenshots plus spawn verification (0.5 day). The map-0 fallback needs only the rows (about 1 day), but it gives worse rooms (below).

## The fallback on map 0, checked against the loaded ADT tiles and WMOs

Each fallback was checked by the same flood, on terrain plus every WMO that its tiles place.

| Arena | Centre (x, y, z) | Open | Leash | Notes |
|---|---|---|---|---|
| Burning Steppes, lava field below Blackrock Stronghold | −7692, −1404, 133.9 | 40 | 36 | ADT magma beside it; something 20 yd overhead |
| Burning Steppes, open field below the Stronghold | −7704, −1464, 133.0 | 58 | 50 | little lava in view |
| Searing Gorge, the Cauldron's floor | −6724, −1368, 181.8 | 20 | 18 | the Cauldron's lava is WMO liquid, not ADT |
| Searing Gorge, the plateau east of the Cauldron | −6758, −1224, 247.0 | 90 | 80 | open sky, no lava in reach |
| Blackrock Mountain (`blackrock.wmo`), floor below the lava hall | −7355, −1000, 173.0 | 28 | 24 | interior |
| Blackrock Mountain, the chained lava hall | −7595, −1012, 244.0 | 16 | 14 | interior, cramped |

The Blackrock Mountain hall and the Cauldron floor, which RAIDS.md planned to use for six bosses, are smaller than every MC cavern but the entrance. The fallback also needs the same lava-damage work if the lava is to mean anything. So the fallback saves only items 1 and 4 from the list above, and costs authenticity. Keep the Burning Steppes field as plan B for Ragnaros if the lava-walkable ring plays badly.
