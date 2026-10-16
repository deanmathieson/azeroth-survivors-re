# Roadmap and nightly plan

This is the working plan for this fork (`progression` branch). Our content and numbers are our own; the names, spells, art and sounds come from the real 1.12 client. The gacha is for fun only: no real money, ever.

## Where we are (15 Oct 2026)

On `progression` (pushed, 391 survivors tests, clippy clean):

- **Meta-progression:** profile saves, run options, talents (51 points) with Talents 2.0 (each tree's 10/20/30-point talents are real effects), real WoW gear that shows on the hero, achievements and perks, zone unlocks and racials.
- **In-run:** quests from field givers, Companion 2.0, jewelry choices and the C character panel, legendary skill transforms, Overcharge picks once a build is maxed, and an XP curve that reaches about level 81 by 15:00.
- **Feel:** the WoW camera with collision, spell sounds, zone music, boss frames, boss chests, instant replay, and the Gurubashi pit fills the whole arena.
- **Collection:** the Darkmoon Faire (tickets only), the Stable (mounts and hats you ride in runs) and the Wardrobe (tabards, titles, a toy bar, Darkmoon decks that become trinkets).
- **Co-op rules (solo with bots for now):** downed heroes revived by allies, party scaling, spawns around each player, per-hero results. Try it with `WOW_SURVIVORS_PARTY=3`.

In progress (paused, resumed tonight):

| Work | State |
|---|---|
| Online networking (host/join, replicated world, per-player hands) | built on `net`, mid-merge into `progression` |
| No-pause co-op level-ups (banked picks, Tab strip, Focus, slow-mo, Breather, drop-in catch-up) | being built during that merge |
| WoW party synergy (party buffs, shared debuffs, class resurrections, taunts) | branch `wowparty` |
| WoW party UI (party frames, nameplates, ally arrows, Recount-style meter, MVP awards) | branch `wowui` |

## Tonight's run

Each step only starts when the one before is verified. Nothing is pushed unless the full gate is green.

0. **Party size is 5** (the group is 4-5 friends): the host accepts 4 clients; party scaling gains a 5th step (×3.4); the party frames show 4 allies.
1. **Resume the three paused workers** together: networking (in the main repo, its own `target/`), `wowparty` and `wowui` (each with its own target dir).
2. **Networking lands.** Verify (rustfmt, clippy, survivors tests with and without `coop`, play build), then push.
3. **Merge `wowparty` and `wowui`** on top. One integration worker wires them into networking:
   - clients fill the party frames from replicated heroes;
   - party buffs, debuff icons, downed state and resurrection visuals replicate;
   - the meter counts remote players.
4. **WoW social layer** (networked; two workers in parallel):
   - **Pings and raid markers:** skull, cross, square, moon, triangle, diamond, circle and star on a spot or an enemy, shown on the minimap; "Help!", "Boss" and "Chest here" pings.
   - **Emote wheel** with the real voice emotes (/wave, /cheer, /dance, /charge…).
   - **Ready check** in the lobby, with the real ready-check pop-up and sound; the lobby shows each player's class, title and mount.
   - **Spectate while downed:** cycle the camera through living allies.
   - **Personal boss loot:** each player rolls for their own class; everyone sees the party's loot banners.
5. **Launch night kit** (so friends don't build anything):
   - a prebuilt zip of the game exe and config, shared privately; friends bring their own WoW 1.12.1 data;
   - a version check in the Hello/Welcome handshake that refuses mismatched builds with a clear message;
   - a one-page friend guide: get the data, unzip, Tailscale, Join.
6. **Two-window soak test:** with the PC free overnight, launch the game twice (one hosting, one joining on 127.0.0.1), take screenshots of the lobby and a shared run, and play a full 15-minute co-op run, and check they connect, see each other, level up without pausing, and hold frame rate with a 5-player-sized horde.
7. **Docs and page:** "How to play online" in `docs/MULTIPLAYER.md`, this roadmap, the README, and the toast.house page. Leave a morning summary.

**Rules for the night:**
- one build folder per worktree;
- check free disk before each wave (stop if under 15 GB);
- the orchestrator re-runs every gate itself before pushing;
- no force-pushes;
- a worker that fails twice is parked and written up rather than retried forever.

**Morning deliverable:** `progression` with online co-op and the party features, a fresh play build, a short list of things to try with friends, and an honest list of what's untested.

## How we'll test co-op

- Automated: in-process host + client over a real UDP loopback (already in the test suite).
- On one PC: two windows, one hosting (`WOW_SURVIVORS_HOST=5000`) and one joining (`WOW_SURVIVORS_JOIN=127.0.0.1:5000`).
- With friends: Tailscale on everyone's PC (easiest), or forward UDP 5000 on the host's router.

## Tomorrow evening (before the group arrives)

Small, social, high on laughs:

- **Need / Greed / Pass** on boss loot, with the real roll window and dice sound, plus `/roll`.
- **Side bets** before a run (top damage, first to die, survives to 15:00), wagered in Faire tickets and settled from the results.
- **Faire pulls broadcast** to the lobby, with a fanfare for Legendary pulls, and ticket gifting.
- **Fashion vote** after a run: best-dressed wins tickets.

## Raid night: Molten Core

The first raid is Molten Core. Its design lives in docs/RAIDS.md. Order of work:

1. **Spike:** does the engine load and render the Molten Core instance map (map 409)? If not, what is missing?
2. **Boss mechanics library** built on the mechanics toolkit:
   - telegraphed ground effects;
   - spread and stack;
   - Living Bomb;
   - dispellable curses;
   - fear;
   - taunt swaps;
   - enrage timers;
   - adds and phases.
3. **Raid mode:** pull trash, then bosses in order, ending with Ragnaros.
4. **Tier 1 sets** with real 2/4/8-piece set bonuses through Need/Greed, and raid achievements and titles.
5. **Transmog collection,** so raid looks can be shown off.

## Campfire intermissions (the user loves this; after launch night)

A hybrid of continuous survival and waves, built for co-op.

**When it happens.** Every boss kill, roughly every 2:15, turns the 8 s Breather into a 45-60 s **camp**:
- the horde backs off;
- a campfire spawns (the real campfire model and crackle);
- everyone gathers, and WoW's Well Rested "Zzz" shows.

**What you do at camp:**
- Spend banked picks with full-size cards.
- Visit a goblin vendor selling rerolls, Healthstones and temporary buffs for gold.
- Settle side bets for the next stretch.
- Play mini games, each a few seconds, rewarding small buffs or tickets:
  - **Fishing:** a bobber; click when it dips; the catch is buff food.
  - **Cooking:** Spiced Wolf Meat gives the party Well Fed.
  - **Darkmoon Whack-a-Gnoll.**
  - **Duels:** quick, with no consequences ("X has defeated Y in a duel").
  - **/roll-offs** for a bonus chest.

**Leaving camp.** A ready check or a 60 s timer, then the war horn and the next stretch. Solo: a shorter camp, or Enter skips it.

**Inspiration.** Raid pull-and-rest rhythm: Need/Greed, the fashion vote and Faire pulls all live naturally at the fire.

## Backlog (after co-op)

- Role cards in co-op (heals that also hit the lowest ally, +threat, taunt on mobility).
- Faire phase 3: transmog.
- Online polish: reconnect, a ping/latency display, a browser transport.
- Clients see enemy debuff marks, hero auras and mounts.
- The hero blinks for a few frames on a gear change.
- Better hat icons for the Dapper Top Hat and Jack-o'-Lantern Hat; double-check the Gray Kodo, Striped/Winterspring Frostsaber and Swift White Mechanostrider skins.
