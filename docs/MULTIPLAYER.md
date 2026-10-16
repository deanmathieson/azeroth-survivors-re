# Multiplayer (co-op) design

Status: milestone 1 (groundwork) is in. Milestone 3, online co-op on desktop, is playable on `progression`: host, join, lobby, replicated heroes, enemies, pickups and run, per-player level-up hands, spell visuals, and disconnects, under milestone 2's party rules (bots, down and revive, party scaling, the level-up waiting for every player). See "How to play online" and "Online co-op: what is built" below.

## How to play online

Online co-op is in the desktop build (`warcraft-survivors`, cargo feature `coop`, on by default). The browser build has no co-op yet.

**The host**

1. At the main menu, pick your class, race and zone, then click **Multiplayer** (left of Enter World).
2. Leave the port at **5000** (or type another) and click **Host**. The panel shows your LAN address, for example `Same network: friends join 192.168.1.20:5000`.
3. Give your friends an address they can reach:
   - **Same house or LAN:** the LAN address the panel shows.
   - **Over the internet:** your public IP (search "what is my IP"), with **UDP** port 5000 forwarded on your router to this PC, and allowed through the Windows firewall (Windows asks the first time you host; allow it on private networks).
   - **Easiest over the internet:** install [Tailscale](https://tailscale.com) or [ZeroTier](https://www.zerotier.com) on every PC and join the same network. Friends then join your Tailscale address (`100.x.y.z:5000`). No port forwarding, and it works behind most routers.
4. The **Party** list fills as friends join (name, race and class). When everyone is in, click **Start Run**. Everyone enters the run together, on your zone and your Run Options.

**A friend joining**

1. Pick your own class, race and look at the main menu first: that is the hero you bring.
2. Click **Multiplayer**, type the host's address in **Join address** (`192.168.1.20:5000`, `100.64.0.7`, or `[::1]:5000`; a bare address uses port 5000), and click **Join** (or press Enter). Ctrl+V pastes.
3. Wait in the lobby. The run starts when the host clicks Start Run (Enter World does nothing while you are joined).

**During and after a run**

- Each player moves their own hero. Enemies, damage, drops, XP, gold and bosses are the host's.
- The XP bar and level are shared, and **a co-op level-up never pauses the world**. Each level banks a hand for every player (jewelry choices from caches and quests too): the badge over the action bar reads "Level up! ×N (Tab)". **Tab** opens your hand as a strip at the bottom of the screen while you keep moving and casting: 1, 2, 3 (or a click) pick, R rerolls, B banishes, X skips, and Tab hides it with the picks still banked. The next banked hand opens after each pick. The host deals your hands; your choices are yours.
- **Focus:** while your strip is open you take 50% less damage, under Divine Protection's glow, which your allies see.
- **Slow-mo:** if every living player has their strip open at once, the world runs at 40% speed ("Everyone's choosing") until someone closes theirs.
- **Breather:** when a boss falls, nothing spawns for 8 seconds ("Breather — spend your picks").
- A player joining mid-run banks one hand per level the party has made, to catch up.
- **Esc** opens the menu without pausing (so does the Character panel). Playing alone, or alone with test-aid bots, keeps the paused level-up as before.
- A hero at 0 health is downed, not dead: stand beside them to revive them (the host decides; everyone sees the downed pose and the revive glow). A downed player's level-ups wait until they are up. The horde scales with the whole party, remote players included.
- When the run ends, each machine records the run on its own profile: talent XP, achievements, and loot for your own class.
- **Leave** in the Multiplayer panel leaves the party; your hero leaves the host's run. If the host leaves or its connection drops, everyone else goes back to the menu with a message.

**Two instances on one PC (testing)**

Run two copies of `warcraft-survivors`, for example from two terminals. The first hosts and the second joins `127.0.0.1:5000`, from the panel or with the test aids:

```bash
WOW_SURVIVORS_NAME=Host WOW_SURVIVORS_HOST=5000 cargo survivors
WOW_SURVIVORS_NAME=Guest WOW_SURVIVORS_JOIN=127.0.0.1:5000 cargo survivors
```

Both copies share one profile directory on one PC, so they save the same profile files.

## Online co-op: what is built

Code: `crates/benilla-app/src/survivors/coop/` (`net` for the transport, `wire` for what travels, `host`, `client` and `ui`), on `bevy_replicon` 0.40.4 and `bevy_replicon_renet2` 0.17.0 (renet2 netcode over UDP; the newest pair on Bevy 0.18). The integration seams the party-rules work replaces are in `party.rs`: `Remote` (a remote human's hero), `NetRole`/`simulates` (a client simulates nothing), `HandOutbox`/`HandDecision` (a client's cards post choices instead of applying them), and `apply_pick`.

- **Authority.** The host simulates. Every system that decides the run (the spawner, the chase, casting and hits, drops, pickups, caches, quests, companions, jewelry and the vitals) runs only where `party::simulates` holds. Each client owns its hero's walk and sends it 30 times a second (unreliable); the host trusts it.
- **Replication** (30 Hz on the wall clock, so a paused host still syncs):
  - every unit, object and ground-spell area: its `units::Look` (display, scale and weapons, not meshes) and a quantised `NetPos` (centimetres, 1/256 turn, pace and movement flags);
  - heroes: `NetHero` (player, name, class, race, sex, trainer display), `NetVitals` (health, max health, shield, down) and `NetBuild` (spells, talents, bonuses, picks, rerolls, banishes);
  - enemies: `NetEnemy` (roster index and rank, sent once) and `NetLife` (health and death);
  - the run: one `NetRun` entity (clock, kills, XP, level, gold, elites, caches, bosses, won, banner).
  Clients build each body from its look, ease everything but their own hero toward the host's place, and play deaths and corpse fades locally. A client dresses only its own hero in its own gear; other heroes show their class trainer's look.
- **Messages.** Client to host: `Hello` (name, pick, profile bonuses), `MoveInput`, `HandAction` (pick, reroll, banish, skip). Host to client: `Welcome` (player number), `Lobby`, `StartRun` (stage and Run Options), `RunPhase` (results, or back to the menu), `Hand` (that player's hand, with the build its slots index), and `NetFx` (spell GO with targets, other cast edges, melee swings and combat numbers, entity-mapped) so clients see and hear every spell.
- **Co-op level-ups** (`survivors/strip.rs`, any run with two or more non-bot heroes, `strip::CoopRun`): `bank_level_ups` turns a pending LevelUp or Jewelry phase into banked hands (`HeroState::picks`, `jewelry.picks`), bots pick at once, and `wait_for_party` holds nothing in co-op. `party::next_hand` deals a jewelry choice first (as `Choice::Jewel` cards, worn by `jewelry::put_on`), then level-up cards; `party::apply_pick` / `apply_skip` spend them. The host pre-deals each remote player's next hand (`serve_hands`); picks, rerolls, banishes and skips come back through the same paths, and a player who goes down has its hand put away until revived. `FocusInput` tells the host a client's strip is open (`HeroState::focused`, half damage, `strip::FOCUS_MARK`). `strip::slow_mo` sets the host's virtual time to 40% and `NetRun::slow` makes clients follow. The Breather (`Run::breather`) holds `spawn_enemies` and `spawn_elites`. `strip::world_runs` keeps the simulation running under Esc's menu and the Character panel in co-op, and `pause` pauses nothing there.
- **Replicated party state:** `NetVitals` carries down, revive progress and Focus (clients show the downed pose, the revive glow and Focus's glow); `NetBuild` carries Overcharges and banked jewelry; `Hand` carries Overcharge and jewelry cards. Party scaling (`Run::party`) counts remote heroes.
- **Party and build:** up to five players (the host and four friends); the horde scales ×1.0 / 1.6 / 2.2 / 2.8 / 3.4. Everyone must run the same build: the handshake compares the co-op protocol, the release and the commit, and a mismatch is refused on both screens with "<Name> is on a different build; everyone needs the same zip."
- **Not yet:** the browser transport; a client's mobility spell only moves its own hero (its damage and effects do not reach the host); talent mechanics from a client's profile (its stat bonuses do travel); co-op jewelry hands roll from the base catalog (no Darkmoon decks) and replace the plainest worn piece rather than asking which; enemy debuff marks, hero buff auras, ability cooldown sweeps and mounts on clients; reconnecting; ping display.

## Goal

2–5 players survive one horde together, either on one PC (couch co-op) or over the network (online co-op). Friends-only co-op: no matchmaking and no anti-cheat.

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

## Roles: tanks, healers and damage

Co-op should support real party roles, chosen through class and build:

- **Healers** (Priest, Paladin, Druid, Shaman):
  - Heal spells and shields go to an ally, aimed at the most-injured hero in range. Examples: Renew, Power Word: Shield, Prayer of Healing, Holy Light, Rejuvenation, Chain Heal.
  - Healing totems and auras heal every hero inside their area.
  - A downed ally can be revived faster by a healer.
  - Healing done earns XP and shows on the results screen.
- **Tanks** (Warrior, Paladin, Druid):
  - Enemies pick their target by threat, not just distance. A hero's threat score is its threat multiplier divided by the distance.
  - Stances and forms raise the multiplier (Defensive Stance, Righteous Fury, Bear Form).
  - Taunts force nearby enemies onto the tank for a few seconds (Taunt, Growl, Righteous Defense).
  - Tank builds lean on armor, health and damage reduction.
  - Damage taken shows on the results screen.
- **Damage:** every class, and the default for solo play.
- **Cards and talents:**
  - In co-op, role cards join the level-up pool. Examples: "your heals also hit the lowest-health ally", "+threat, +armor", "taunt on your mobility spell".
  - Solo play never offers ally-only cards.
- **Results:** each player's damage done, healing done and damage taken.

Groundwork seams (milestone 1):
- Heals and shields go through `party::heal`/`party::shield` with a target-selection fn. In solo, that target is the caster.
- Enemy targeting goes through `party::pick_target` with a per-hero `threat` field and an optional forced target. In solo, that's the nearest hero.
- Each hero tracks `healing_done` and `damage_taken`.

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
3. **Online co-op on desktop.** Replicon and renet2, host/join, replicated world, client-owned hero movement. *First slice built (`net` branch); see "Online co-op: what is built".*
4. **Browser transport and polish.** Reconnect, a lobby showing who has joined, and ping display.

## Milestone 2 checklist (from the groundwork review)

The groundwork (495398f) leaves single-player unchanged. Fix these before a real second player plays:

1. **Picks for real players:** `roll_offer` and `party::pick_at_random` draw every non-local hero's hands at random, and LevelUp closes when the local hero is done. Add a `Bot` marker that only `fill_party` sets, auto-pick only for bots, and keep LevelUp open until every non-bot hero has picked.
2. **Heals on the downed:** `party::heal`, `heal_over_time`, `shield`, the drain heal and the Healthstone card have no `alive()` guard. Return 0 for downed targets.
3. **Picks while downed:** `deal_hands` deals to fallen heroes. Decision: a downed hero banks its picks and gets them when revived. No pause is offered to a downed local player.
4. **Spawn centre:** rings, elites and bosses centre on the party's average position, so spread-out players get spawns on top of them. Ring around one standing hero per spawn (round-robin) and reject points within ~25 yd of any hero.
5. **Gold multiplier:** decision: every gold gain uses the collecting hero's multiplier, i.e. coins by whoever picks them up, Skip by the skipper and caches by the opener. The host computes it, so clients can't drift.
6. **No-hero start:** make `start_run` return false when there is no hero.

## Risks

- The `Run` split is a large mechanical refactor. Do it alone, with no other feature work in flight.
- Replicating hundreds of enemies: replicate position, health and death only; the visuals are rebuilt client-side from the display id.
- RNG and level-up offers must be rolled on the host, then sent to each player.
