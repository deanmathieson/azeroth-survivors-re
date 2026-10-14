# Multiplayer (co-op) design

Status: planned. It starts after wave 3 (talents, gear, achievements and rewards, camera) and the zones and racials work are merged, because co-op touches nearly every survivors system.

## Goal

2–4 players survive one horde together, either on one PC (couch co-op) or over the network (online co-op). Friends-only co-op: no matchmaking and no anti-cheat.

## Rules of a co-op run (defaults, open to change)

- **Shared:** the clock, the horde, bosses, elites, caches, gold, and the XP bar and level. The whole party levels up together.
- **Per player:**
  - hero (class, race, looks), abilities, passives and stats;
  - health and shields;
  - talents, gear and perks from that player's own profile;
  - rerolls and banishes.
- **Level-up:** every player picks their own card. The world stays paused until all have picked.
- **Down, not dead:** a hero at 0 health is downed. An ally standing next to them for 3 s revives them at 50%. Revive perks and talents still work first. The run is lost when every hero is down at once.
- **Enemies** chase the nearest living hero. Enemy count and health scale with party size (×1.0 / 1.6 / 2.2 / 2.8 health and density for 1–4 players).
- **Rewards:** each player's own profile records the run (talent XP, achievements, loot drops for their own class).

## Architecture

1. **Split `Run`.**
   - `RunShared` resource: elapsed, kills, XP, level, gold, elites, caches, bosses, reward multiplier, recorded flag.
   - `HeroState` component on each hero: player id, class, abilities, passives, hp, shield, immune, revives, bonuses, rerolls, banishes, banished.
   - Every `hero.single()` becomes a query over heroes, or a lookup by player id. That covers targeting, pickups, the camera, the HUD, cards and the hand.
2. **Local co-op.** Player 1 uses keyboard and mouse; players 2–4 use gamepads (`gilrs` is already a dependency). The camera follows the party's centre and zooms out to keep everyone framed. There's a split HUD (one health bar per hero) and level-up cards per player in turn.
3. **Online co-op.**
   - `bevy_replicon` 0.40.x with `bevy_replicon_renet2` 0.17.x (the newest versions supporting Bevy 0.18).
   - The host is authoritative for enemies, damage, drops, XP, gold and bosses.
   - Each client is authoritative for its own hero's movement, which feels instant. The host trusts it, since this is friends-only.
   - Hero, enemy, gem and cache entities replicate.
   - Inputs and choices travel as client→server events: card pick, reroll, banish, skip, mobility cast.
   - Menu: **Host** (with a port) and **Join** (with an address). Desktop uses renet2's native UDP transport; the browser build uses WebTransport/WebSocket later.
4. **Saving:** each machine saves only its own player's profile.

## Milestones

1. **Groundwork.** The `RunShared`/`HeroState` split, still one player. No behaviour change; every test passes.
2. **Couch co-op.** 2–4 local heroes, the revive rule, party scaling, the party camera and HUD.
3. **Online co-op on desktop.** Replicon and renet2, host/join, replicated world, client-owned hero movement.
4. **Browser transport and polish.** Reconnect, a lobby showing who has joined, and ping display.

## Risks

- The `Run` split is a large mechanical refactor. Do it alone, with no other feature work in flight.
- Replicating hundreds of enemies: replicate position, health and death only; the visuals are rebuilt client-side from the display id.
- RNG and level-up offers must be rolled on the host, then sent to each player.
