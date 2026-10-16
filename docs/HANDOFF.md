# Handoff: finish launch day (16 Oct 2026)

For a fresh Claude Code session on another machine. Read this file first, then docs/ROADMAP.md, docs/MULTIPLAYER.md and docs/FRIENDS.md.

## Where things stand

- `progression` (pushed, d772654) has online co-op for up to 5 players:
  - host/join and a lobby;
  - no-pause level-ups (banked picks, Tab strip, Focus, slow-mo, breather, catch-up);
  - a build-version check;
  - party synergy (buffs, shared debuffs, class resurrections, taunts);
  - the party UI (frames, nameplates, ally arrows, M meter, MVP awards), all replicated;
  - the launch kit (scripts/package-friends.ps1, docs/FRIENDS.md);
  - docs/RAIDS.md (the Molten Core design).
- Gate at d772654:
  - 444 survivors tests pass without `coop` and 460 with it;
  - clippy is clean;
  - the play build is OK.
- Nothing has been played over a real network yet. The in-process UDP loopback test passes.

## Remaining work today, in order

1. **Merge three pushed branches into `progression`.** Each branched from 48de3f0 (before networking), so expect conflicts in hud.rs, party.rs, mod.rs and gear/.
   - `fixes` (7bc27a8): mount hats render (seated on the mount's head bone; per-mount offsets in survivors/stable.rs), companion look scales, a hardened survivors-pick restore. Small; merge first.
   - `specs` (9303540): 31-point spec identity (a starting-spell swap and a summon/form per tree; talents/spec.rs, talents/guardian.rs), the talent/spell double-dip fix (docs/TALENTS.md), create-screen looks.
     - KNOWN GAP: specs are granted only to the local hero. Make them apply to remote heroes on the host. Clients send their talent points in Hello/lobby like stat bonuses, so the host spawns their guardians. Guardians are host-simulated and must replicate like companions.
   - `social` (5ea18b8): Need/Greed/Pass, chat with /roll and emotes, raid-marker pings (middle mouse), ready check, spectate while downed, side bets, Faire pull broadcasts, fashion vote. Lives in survivors/coop/social/.
     - It bumped `PROTOCOL` to 3. The `progression` integration ALSO bumped it to 3, so make it 4 after merging.
2. **Gate after each merge.** Set WOW_DATA to the WoW 1.12.1 client's Data folder; data-gated tests skip without it.
   - `rustfmt --edition 2021 --check <touched files>` (never `cargo fmt --all`; upstream isn't fmt-clean)
   - `cargo clippy -p benilla-app -p warcraft-survivors --all-targets -- -D warnings`
   - `cargo test -p benilla-app --no-default-features --lib survivors`
   - `cargo test -p benilla-app --no-default-features --features coop --lib survivors`
   - `cargo build --profile play -p warcraft-survivors`
3. **Check 7 test failures outside survivors** seen in `cargo test -p benilla-app --lib` (all features): glue_strings, pipe_warm::menagerie, ui_script (minimap, surface_gate, verb_event_gate ×2), ui_session. Find out whether they predate this week's work: check out 48de3f0, or upstream `main`, and run them. Fix them if ours.
4. **Push `progression`.**
5. **The friends' zip must be built on the WINDOWS PC.** Friends run Windows, and cross-compiling this Bevy app from macOS is not worth it today.
   - Build without the `dev` feature, so the exe doesn't carry the builder's checkout paths. For example `cargo build --profile play -p warcraft-survivors --no-default-features --features <the non-dev defaults incl. coop>`; read crates/warcraft-survivors/Cargo.toml for the feature list. Confirm `coop` is still on.
   - Then `scripts/package-friends.ps1 -Exe <that exe>`.
   - The launchers set BENILLA_HOME to the kit folder.
6. **Smoke test** on one PC: one window hosting, one joining 127.0.0.1:5000 (see "How to play online" in docs/MULTIPLAYER.md). Check the lobby, ready check, a shared run, a level-up strip with no pause, a death and revive, and the results with MVP awards.
7. **Update the status page** (toast.house/azeroth-survivors-re). It lives on the `gh-pages` branch (index.html plus jpgs). Mark online co-op as landed and add the social features.

## Environment notes

- Rust 1.98.1 (pinned in rust-toolchain).
- `CARGO_INCREMENTAL=0` keeps target/ small.
- If parallel worktrees build at once, give each its own CARGO_TARGET_DIR. Worktrees sharing one target overwrite each other's test binaries.
- After deleting a worktree: `cargo clean -p lua-src -p mlua-sys` (it caches absolute paths).
- Display ids written from memory were often wrong. Verify ids with data-gated tests that list ALL mismatches.
- Commits end with "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>".

## Taste (from the user)

- Authentic WoW 1.12: real names, spells, icons, sounds and UI art from the MPQs, with our own numbers.
- No real-money anything; the Faire runs on tickets.
- Co-op must never make people wait on each other.
- The group is 4-5 friends ("Toast House"). Their first session is tonight.
- Next big thing after launch: the Molten Core raid (docs/RAIDS.md). Step 1 is a spike: does map 409 load and render?
