# Professions and the WoW inventory

Status: P0 (the bags spike) and most of P1 (gathering, skills, the purse, the camp's merchant, trainer and Smelting, tracking) are built; the rest is design. This covers a full WoW-style inventory and all twelve 1.12 professions for survivors: Mining, Herbalism, Skinning, Fishing, Alchemy, Cooking, First Aid, Enchanting, Blacksmithing, Leatherworking, Tailoring and Engineering.

Ground rules, as for the rest of the fork:

- **Take as much from the base game as we can.** The bags, the tradeskill and craft windows, the merchant, the loot window, the skills pane, the trainer, the bank, the trade window and every item tooltip are the stock 1.12 FrameXML, which the engine already runs. The names, icons, models, spell visuals and sounds are the real ones. The numbers are our own.
- **Ids are unverified.** Every item, spell, game object and lock id below is from memory. Data tests check each one before anything uses it:
  - spell and skill-line ids against `Spell.dbc` and `SkillLine.dbc`;
  - display ids and icons against `ItemDisplayInfo.dbc`, as `gear/catalog.rs` already does;
  - item and game-object entries and names against the cores' `item_template` and `gameobject_template` (the client has no item names).
- **No real money.** Gold here is in-game gold, and it never converts to or from Faire tickets.

---

## 1. Pitch and the loop

Runs now drop more than XP and gear. Ore veins and herbs sprout across the battleground, beasts leave hides and fish rise in pools. Stand on a node for a moment and the hero swings a pick while their spells keep firing, and the ore goes into real WoW bags. At the campfire after each boss, players brew, cook, bandage and enchant. Between runs they smelt, forge, sew and tinker in the Armory, and they carry potions, bandages and gadgets into the next run on a small bar.

| When | What professions do |
|---|---|
| In a run (fighting) | Gather nodes, skin beasts, fish pools; use satchel items (potions, bandages, bombs, gadgets) on keys 4-6 |
| At camp (45-60 s) | Cook for the party, brew, bandage, enchant; fish the camp bobber; buy and sell at the goblin; trade with friends |
| At home (the Armory, between runs) | Everything else: smelt, forge, sew, tinker, disenchant, train, bank, sort bags |

**What it is for.** Deterministic progress beside gear's random drops. A reason to replay zones (Thorium only grows in the high ones). A reason for friends to trade. And a supply chain for the raid (Greater Fire Protection Potions for Ragnaros).

---

## 2. Architecture: a hybrid UI

### 2.1 Which UI draws what

| Surface | Drawn by | Opened by |
|---|---|---|
| Bags (Backpack plus four bags) | **FrameXML** `ContainerFrame` | **B** (all bags), or the bag buttons |
| Item tooltips everywhere (FrameXML and Bevy) | **FrameXML** `GameTooltip` | hover |
| Tradeskill window (every profession but Enchanting) | **FrameXML** `Blizzard_TradeSkillUI` | the profession book, or a station |
| Enchanting | **FrameXML** `Blizzard_CraftUI` (1.12 Enchanting uses the Craft window) | the profession book |
| Merchant (the camp goblin, home vendors) | **FrameXML** `MerchantFrame` | clicking the NPC |
| Loot window (fishing catch, opened containers) | **FrameXML** `LootFrame` | the event |
| Skills pane | **FrameXML** `SkillFrame` in `CharacterFrame`, other tabs hidden | **K** |
| Profession book | **FrameXML** `SpellBookFrame`, one General tab of profession spells | **P** |
| Trainers (ranks and recipes) | **FrameXML** `ClassTrainerFrame` (`Blizzard_TrainerUI`) | the Armory's Trainers row |
| Bank (home only) | **FrameXML** `BankFrame` | the Armory's Bank button |
| Trade between players | **FrameXML** `TradeFrame` | drop an item on an ally, or the party-frame menu |
| Casting bar for gathering and crafting | **FrameXML** `CastingBarFrame` | `SPELLCAST_START` / `_DELAYED` / `_STOP` |
| Survivors HUD, satchel bar, cards, hand strip, party frames, minimap | **Bevy** (unchanged) | |
| Armory (talents, gear, Faire, Stable, Wardrobe, Character tab) | **Bevy** (unchanged) | |

**Decision: a curated manifest, not the whole `FrameXML.toc`.** Survivors loads only what these frames need, in the shape `tests/tradeskill_frame.rs`'s `FILES` list already proves:

- the base: `GlobalStrings`, `Fonts`, `UIParent`, `GameTooltip`, `MoneyFrame`, `StaticPopup`, `UIDropDownMenu`, `ItemButtonTemplate`;
- the windows: `ContainerFrame`, `MerchantFrame`, `LootFrame`, `BankFrame`, `TradeFrame`, `CastingBarFrame`, `CharacterFrame` with `SkillFrame`, `SpellBookFrame`, `ClassTrainerFrameTemplates`;
- the load-on-demand addons `Blizzard_TradeSkillUI`, `Blizzard_CraftUI` and `Blizzard_TrainerUI`.

The stock action bars, unit frames, chat, minimap, buff frame and `Bindings.xml` never load, so none of them can fight the HUD and none of their key bindings exist. A small survivors Lua shim hides `CharacterFrameTab1/3/4/5` and the spellbook's class tabs.

### 2.2 The opt-in

Today `SurvivorsPlugin` inserts `run_mode::CaptureMode` (`survivors/mod.rs:80`), and under it `ui_wanted` (`ui_script/lifecycle.rs:125`) loads no UI. CaptureMode also switches off the stock player camera and movement, which survivors still needs off. So the UI gets its own flag instead of losing CaptureMode:

- a new marker resource, `run_mode::SurvivorsUi`, which `SurvivorsPlugin` inserts beside CaptureMode;
- `ui_wanted` becomes `!CaptureMode || capture_ui_opted_in() || SurvivorsUi`, and `ui_script/mod.rs`'s `capture_ui_active` gains the same third arm so the shared feeds run;
- the manifest step picks the survivors manifest when `SurvivorsUi` is present;
- **kill switch:** `WOW_SURVIVORS_FRAMEXML=0` leaves `SurvivorsUi` out, which is today's behaviour.

The load runs once at the world entry, sliced across frames as now, and stays up across runs.

### 2.3 The source of truth: local resources

`survivors/professions/inventory.rs` holds a `Bags` resource, the player's containers in WoW numbering:

| Id | Container |
|---|---|
| 0 | Backpack, 16 slots |
| 1-4 | the worn bags (size from the bag item) |
| -1 | the bank's 24 slots (home only) |
| 5-10 | bank bags |

Each slot holds `{ item, count, creator }`. Every rule lives here, in plain Rust with unit tests: stack limits; move, swap, merge and split; bag families (no bag inside itself); autostore, destroy, consume and add, with "Inventory is full." on overflow. The UI only mirrors it.

A `Professions` resource sits beside it: the purse (copper), skill ranks, known recipes, cooldowns and the satchel. Both load from the profile and save through `Profile::mark_dirty` (section 8).

### 2.4 Pushing snapshots (feeds)

Survivors has no `SelfPlayer` or `ObjectStore`, and the stock feeds already treat that as "no source" and push nothing (`ui_items/feed.rs`: "an absent self player is no source"). So survivors adds its own feeds and reuses the stock push helpers:

| What | Pushed with | Notes |
|---|---|---|
| Containers | `ui_items::feed::apply_container_source` (`feed.rs:1211`), with a `HashMap<i64, ContainerState>` built from `Bags` | Reuses the per-bag diff, `BAG_UPDATE`, `ITEM_LOCK_CHANGED` and `BAG_OPEN`. Synthetic slot guids keep the close-on-swap logic working |
| Item tooltips | `set_item_template(id, view)`, answering `take_item_stat_asks` | From the authored item table (2.6) |
| Requirements | `set_player_req_state` | The picked class, race and level, so a wrong "Classes:" line turns red |
| Money | `set_money(copper)` (`unit/mod.rs:347`) | The profile purse at home, the party's run purse in a run (5.4) |
| Tradeskill book | `set_trade_skill(TradeSkillState)` | Recipes from `SkillLineAbility` and `Spell.dbc`; colours from `ui_tradeskill::difficulty` (`ui_tradeskill.rs:181`). `resolve_recipe` is split so its item lookups take a closure, and survivors passes the authored table instead of `Items` |
| Enchanting | `set_craft(CraftState)` | As above |
| Skills | `set_skills(SkillsState)` through `ui_char::skills_row` | "Professions" and "Secondary Skills" headers from `SkillLineCategory.dbc` |
| Merchant, loot, trainer, bank, trade | `set_merchant`, `set_loot` (`LootState::fishing` for the bobber), `set_trainer`, `set_bank`, `set_trade` | One feed each, run only while that window is open |
| Casting bar | `fire_event("SPELLCAST_START", [name, ms])`, then `_DELAYED`, `_STOP`, `_INTERRUPTED` | Gathering and crafting casts |

Each feed runs in the `UiFeed` set with `.run_if(survivors::active)` and pushes only on change, through a generation counter on `Bags` and `Professions` (the `FeedMemory` gate pattern).

### 2.5 Handling drained intents

Every `take_*` queue has exactly one drainer. **The stock net drains get `.run_if(not(survivors::active))`**: `ui_items/drain.rs`, `ui_tradeskill::drain_trade_skill`, and the merchant, loot and trade drains. Otherwise they would empty the queues and send packets to no server. Survivors' drains apply each intent to `Bags` and `Professions` in the same frame:

| Intent (queue) | Survivors handler |
|---|---|
| `take_container_moves` (move, swap, merge, split with `count`) | `Bags::move_item` |
| `take_bag_autostores` | `Bags::autostore` |
| `take_container_destroys` | `Bags::destroy`, after the stock `DELETE_ITEM` popup has asked |
| `take_container_uses` (right-click) | Use a consumable (at camp or home); learn a recipe; equip a bag into a free bag slot; "Equip" crafted gear (3.4); open a container item (Battered Chest, clams) into the loot window |
| `take_container_autoequips` | "Equip" for crafted gear and bags |
| `take_item_picks` while targeting | Disenchant a bag item; an enchant's bag target |
| `take_trade_skill_dos(spell, count)`, `take_craft_dos` | Start a craft cast: after the cast time, consume reagents and add products; decrement `repeat_count` for Create All |
| `take_merchant_buys`, `_slot_buys`, `_cursor_sells`, `_buybacks` | Purse and `Bags`, with a 12-row buyback list in memory |
| `take_loot_picks`, `take_loot_close` | Move the rows into `Bags` |
| `take_trainer_buys` | Spend the purse; raise the cap or learn the recipe |
| `take_bank_purchase` | Buy a bank bag slot at `BankBagSlotPrices.dbc`'s price |
| `take_trade_*` | Relay to the host (section 7) |
| `take_skill_abandons` | Unlearn, after the stock confirm popup; the skill and recipes go, the materials stay |

### 2.6 Authored items

WoW keeps item templates on the server, so survivors authors them in `survivors/professions/items.rs`. Each row has the real entry, name, quality, class and subclass, display id, stack size, sell price, bonding and bag size, plus a survivors effect: a `Bonus` list for gear, or a `Use` effect for a consumable.

The `ItemTemplateView` is built from the row. A survivors bonus prints as a real-looking green line through `spell_triggers`, so tooltips read like 1.12. For example:
- `(1, 0, "Equip: Improves your chance to get a critical strike by 2%.")`;
- `(0, 0, "Use: Restores 25% of your health.")`.

**The id set is closed under `Spell.dbc`.** A curated recipe's reagents and products come from its spell's `Reagent[8]` and `SPELL_EFFECT_CREATE_ITEM`. A data test fails if a recipe names an item the table lacks, or if the table carries an item that no recipe, node or drop table uses.

### 2.7 Input and focus

- **Pointer.** The player UI's hover already feeds `PointerOverUi` ahead of `wowcam::hud_takes_pointer`. A click on a frame starts no mouselook, and right-drag off the frames still turns the camera.
- **Keyboard.** While an edit box has focus (`UiKeyboardCapture`), every survivors key system stands down: WASD, Q/E, 1-6, B, R, X, F, G, M, V, Tab and Esc. The split-stack box and trade money take digits.
- **B** opens all bags. While the hand strip is open, B still banishes (`hand.rs:217`) and the bags don't open.
- **Esc** first runs `CloseSpecialWindows()` and `CloseAllBags()`. Only if nothing was open does it open the survivors menu, as in WoW.
- **Nothing pauses** for bags, skills or the profession book, alone or in co-op, and you can walk with bags open. The tradeskill, craft, merchant, trainer, bank and trade windows open only at camp or home.
- **Modal survivors screens win.** A solo level-up hand, the jewelry choice, the results and the menu close every FrameXML window as they open. Tooltips stay topmost.
- **One UI scale.** The settings' UI scale drives both the Bevy HUD and `UIParent:SetScale`.
- **Anchors.** Bags stack bottom-right above the HUD's bars (the container offset is raised by the HUD's height). The casting bar sits above the XP bar. Windows dock left through `UIParent`'s panel manager.

### 2.8 Co-op ownership (detail in section 7)

Each player's bags, purse, skills and recipes are local to their machine and saved in their own profile, as gear and talents already are. The host owns the shared world: node spawns and depletion, each player's yield (it rolls them, so a node can't be double-dipped), skinnable corpses, and the effects of party-affecting items (a Target Dummy, a Battle Chicken, a served meal). It tells each player what they got, and that player's machine puts it in their bags.

### 2.9 Module layout

`crates/benilla-app/src/survivors/professions/`: `mod.rs` (the plugin), `items.rs` (the authored table), `inventory.rs` (`Bags`), `frames.rs` (the opt-in shim and feeds), `drains.rs`, `gather.rs` (nodes, skinning, pools), `skills.rs` (ranks, skill-ups, trainers), `recipes.rs` (the curation over `SkillLineAbility`), `effects.rs` (item uses to mechanics), `satchel.rs` (the run bar), `econ.rs` (purse, vendors, buyback), `net.rs` (co-op) and `save.rs`.

---

## 3. Inventory model

### 3.1 What exists, and when

| Store | Lives | Persists | Holds |
|---|---|---|---|
| **Bags** (Backpack plus 4 bag slots) | always, in and out of runs | yes, per profile | materials, consumables, recipes, unequipped crafted gear, duplicate gear, bags |
| **Bank** (24 slots plus up to 6 bank bags) | home only | yes | anything |
| **Satchel** (3 run-bar slots, keys 4-6) | the loadout, set at home or camp | yes (which items) | references to usable items in the bags; a use consumes from the bags |
| **Purse** | always | yes (profile gold) | copper; in a run the party's shared run purse shows instead (5.4) |
| **Class gear collection** | as today | yes | worn and collected pieces per class, plus their enchants |
| **Jewelry** | in a run, as today | no | unchanged; a declined or replaced piece can be disenchanted (5.2) |

**Decision: the satchel is a bar, not a second container.** The bags travel with you anyway, so the satchel limits what you can *use* in a run, not what you carry. That keeps one inventory and one source of truth, and it is Megabonk-shaped: three choices you commit to.

**Decision: the haul is kept, win or lose.** Everything gathered goes straight into the bags and stays, like talent XP. Casual friends shouldn't lose an evening's ore to one bad Ragnaros pull.

### 3.2 Bags

You start with the 16-slot Backpack and four empty bag slots. Bags come from caches and quest rewards, the home vendor, and Tailoring.

| Bag | Entry (verify) | Slots | Source |
|---|---|---|---|
| Linen Bag | 4238 | 6 | Tailoring 45; the home vendor |
| Woolen Bag | 4240 | 8 | Tailoring 80 |
| Small Silk Pack | 4245 | 10 | Tailoring 150 |
| Mageweave Bag | 10050 | 12 | Tailoring 225 |
| Runecloth Bag | 14046 | 14 | Tailoring 260 |
| Mooncloth Bag | 14155 | 16 | Tailoring 285 (Mooncloth) |
| Bottomless Bag | 14156 | 18 | Tailoring 300 (Mooncloth, Felcloth) |
| Core Felcloth Bag | 21342 | 20 | Tailoring 300, an MC pattern |

When the bags are full, the real "Inventory is full." error and sound play, and the item stays on the ground for 30 s. P4 adds a mailbox at home, the stock `MailFrame`, that collects overflow.

### 3.3 Item ids

Materials, recipes and crafted items use **real 1.12 entries, names, icons and display ids**, with authored templates (2.6). Examples, all to verify:

| Kind | Items (entry) | Stack (verify) |
|---|---|---|
| Ore | Copper 2770, Tin 2771, Iron 2772, Silver 2775, Gold 2776, Mithril 3858, Truesilver 7911, Thorium 10620, Dark Iron 11370 | 10 |
| Bars | Copper 2840, Bronze 2841, Silver 2842, Iron 3575, Tin 3576, Gold 3577, Steel 3859, Mithril 3860, Truesilver 6037, Dark Iron 11371, Thorium 12359, Arcanite 12360 | 20 |
| Stone | Rough 2835, Coarse 2836, Heavy 2838, Solid 7912, Dense 12365 | 20 |
| Herbs | Silverleaf 765, Mageroyal 785, Peacebloom 2447, Earthroot 2449, Briarthorn 2450, Bruiseweed 2453, Kingsblood 3356, Liferoot 3357, Khadgar's Whisker 3358, Fadeleaf 3818, Goldthorn 3821, Firebloom 4625, Purple Lotus 8831, Sungrass 8838, Gromsblood 8846, Dreamfoil 13463, Golden Sansam 13464, Mountain Silversage 13465, Plaguebloom 13466, Icecap 13467, Black Lotus 13468 | 20 |
| Cloth | Linen 2589, Wool 2592, Silk 4306, Mageweave 4338, Runecloth 14047, Felcloth 14256, Mooncloth 14342 | 20 |
| Leather | Ruined Leather Scraps 2934, Light 2318, Medium 2319, Heavy 4234, Thick 4304, Rugged 8170, Devilsaur 15417, Core 17012 | 10-20 |
| Enchanting | Strange Dust 10940 to Illusion Dust 16204; Lesser/Greater Magic to Eternal essences; Small Glimmering Shard 10978 to Large Brilliant Shard 14344; Nexus Crystal 20725 | 5-20 |
| Fish | Raw Brilliant Smallfish 6291, Raw Longjaw Mud Snapper 6289, Deviate Fish 6522, Oily Blackmouth 6358, Firefin Snapper 6359, Winter Squid 13757, Raw Nightfin Snapper 13759, Stonescale Eel 13422 | 20 |
| Raid materials | Lava Core 17011, Fiery Core 17010, Sulfuron Ingot 17203, Elemental Fire 7068, Essence of Fire 7078, Arcane Crystal 12363 | 10-20 |

Recipe items (Recipe:, Plans:, Pattern:, Schematic:, Formula:) are real entries with a trigger-6 "teaches" line. Using one learns its spell, and the stock "Already known" red works.

### 3.4 Gear in the bags

Survivors gear stays what it is: a per-class collection of catalog pieces (`gear/`) that the Bevy Armory shows. Two kinds of gear now also exist as bag items:

1. **Crafted gear** (Lionheart Helm, the Devilsaur set, Robe of the Archmage…) is a real bag item, **Binds when equipped**, so it can be traded or sold until then. "Equip" (right-click, or the Armory) checks the class can wear it (the tooltip's class line is red otherwise), adds it to that class's collection under a catalog id in a new crafted range (`2000+`, `u16`, stable save keys), and removes the bag item. From then on it is ordinary collected gear.
2. **Duplicate boss drops**, today "Already had it", now land in the bags as a **Soulbound** copy to sell or disenchant, so every drop is worth something.

Crafted catalog rows take `gear/catalog.rs`'s shape (`Item`: slot, rarity, display id, icon, bonuses) plus a `classes` mask. The existing `displays_and_icons_exist_in_the_client` test covers them.

---

## 4. Gathering in runs

### 4.1 Nodes by zone band

Nodes are real game objects:
- **models** from `GameObjectDisplayInfo.dbc`;
- **entry, display id and `lockId`** from the cores' `gameobject_template` (authored, verify);
- **the skill needed**, read at load from `Lock.dbc` (`lock.rs`: the `LOCK_KEY_SKILL` slot whose `LockType` is Herbalism or Mining, and its `Skill`).

The numbers in brackets below are only a guide.

| Band | Zones | Veins (skill) | Herbs | Skins | Water |
|---|---|---|---|---|---|
| 1-15 | Elwynn, Durotar, Teldrassil, Dun Morogh, Mulgore, Tirisfal | Copper (1) | Peacebloom, Silverleaf, Earthroot | Ruined Leather Scraps, Light Leather, Light Hide | most |
| 15-30 | The Barrens, Westfall, Redridge, Duskwood | Copper, Tin (65), Silver (75) | Mageroyal, Briarthorn, Bruiseweed, Stranglekelp | Light and Medium Leather | yes; Deviate Fish in the Barrens |
| 30-45 | Gurubashi Arena | Iron (125), Gold (155), Mithril (175) | Kingsblood, Liferoot, Fadeleaf, Goldthorn, Khadgar's Whisker, Purple Lotus | Heavy Leather | the sea around the pit |
| 45-55 | Fire Plume Ridge, The Dark Portal | Mithril, Truesilver (230), Small Thorium (245) | Sungrass, Blindweed, Golden Sansam, Gromsblood | Thick Leather; **Devilsaur Leather** from Un'Goro's devilsaurs | Un'Goro only |
| 55-60 | Gates of Ahn'Qiraj, Mount Hyjal, Naxxramas | Small and Rich Thorium (245, 275) | Dreamfoil, Mountain Silversage, Plaguebloom, Icecap; **Black Lotus** (rare) | Rugged Leather | Hyjal only |
| Raid | Molten Core | Dark Iron Deposit (230) | none | **Core Leather** from Core Hounds | none (lava) |

- **Spawning.** The host spawns 6-10 nodes per 15-minute run, weighted to the zone band and the party's skills (no Black Lotus for a party that can't pick it): a few at the start, then one every 60-90 s, away from the heroes and inside the leash, and none in boss waves or camps. Each lasts 90 s, then sinks.
- **Tracking.** Find Minerals (2580) and Find Herbs (2383), toggled in the profession book, put the real tracking blips on the survivors minimap.
- **Racials** (1.12): Tauren Cultivation (+15 Herbalism) and Gnome Engineering Specialist (+15 Engineering).

### 4.2 Gathering while fighting

- **Stand within 2 yd for 1.5 s.** The gather starts on its own and the hero keeps auto-casting. It plays the real gather spell, whose visuals and sounds come from `Spell.dbc` and `SpellVisual`: Mining 2575, Herb Gathering 2366, Skinning 8613.
- **The casting bar.** The stock `CastingBarFrame` shows the gather. As in 1.12, **a hit pushes it back 0.25 s** (`SPELLCAST_DELAYED`, at most three times), and **moving off the node interrupts it**.
- **Too low a skill:** the node's tooltip shows a red "Requires Mining (175)" and nothing starts.
- **Yield:** 1-3 of the node's material, more with skill over the requirement. Extras: stone from veins; gems (Malachite to Huge Emerald, and Arcane Crystal from Thorium); Black Lotus and Elemental Fire at low odds.
- **Delivery:** straight into the bags, with the real "You receive loot: [Copper Ore]x2." line (GlobalStrings' `LOOT_ITEM_SELF_MULTIPLE`) as a toast.

### 4.3 Shared party nodes

A node is the party's, not the first tapper's. Every hero who finishes a gather on it gets **their own yield**, rolled by the host with that player's skill, once per hero. The node depletes 5 s after its first harvest, or when every hero with the skill has harvested it. So a group crowding a vein is good, and nobody steals anybody's ore.

### 4.4 Skinning

- **Horde beasts.** Each beast killed within 30 yd of a skinner has a small chance (about 3%) to give that skinner leather or a hide directly. The horde dies too fast to stand on corpses.
- **Great beasts, elite beasts and beast bosses** (devilsaurs, Core Hounds, Magmadar, Felbeasts). The corpse stays 15 s with a sparkle and is skinned like a node (4.2), each skinner getting their own yield.
- **Data.** Survivors foes have no creature type, so `data.rs`'s `Foe` gains `skin: Option<SkinTier>`, set per roster row.

### 4.5 Fishing

- **In runs: fishing pools.** Pools are real game objects (entries to verify): School of Deviate Fish, Oily Blackmouth School, Firefin Snapper School, Floating Wreckage, Stonescale Eel Swarm, Patch of Elemental Water. They spawn only in zones whose `MapDef` gains `water: true` (each checked by eye). Standing within 3 yd of a pool casts Fishing (7620) for 2.5 s with the bobber visual, and lands the pool's fish or Elemental Water; Floating Wreckage gives a Battered Chest to open into the `LootFrame`. A pool serves each player once and lasts 60 s.
- **At camp: the existing bobber, now with real loot.** A click while it is under opens the `LootFrame` in fishing mode (`LootState::fishing`) with the zone's fish. Fishing skill widens the bite window. Rare catches: a Big-mouth Clam (a Black or Golden Pearl), Deviate Fish, a Message in a Bottle.
- **Until Cooking lands (P2),** a camp fish still gives Well Fed, so the camp never loses its buff.

### 4.6 Skill-ups

As in WoW, with the bands taken from vmangos's `SkillGainChance` (verify):

| | Gathering (node requirement R) | Crafting (`SkillLineAbility` bands) |
|---|---|---|
| Orange | below R+25: 100% | below `trivial_low`: 100% |
| Yellow | R+25 to R+50: 75% | `trivial_low` to the midpoint: high |
| Green | R+50 to R+100: 25% | the midpoint to `trivial_high`: low |
| Grey | R+100 and up: none | `trivial_high` and up: none |

- **Colours and odds agree.** Crafting uses `ui_tradeskill::difficulty`, the same function that colours the window.
- **The skill-up line.** A skill-up shows "Your skill in Mining has increased to 76." as a toast, with the real sound.
- **Caps.** Ranks stop at the trainer level bought (5.4).

---

## 5. Crafting

### 5.1 Recipes

- **Source data.** Each recipe is a `Spell.dbc` spell with `SPELL_ATTR_IS_TRADESKILL`, joined to its profession through `SkillLineAbility.dbc`. Together they give the skill needed and the trivial bands, the reagents, the tools (`totems`: Blacksmith Hammer, Mining Pick, Runed Copper Rod, Arclight Spanner), the spell focus (Anvil, Forge, Cooking Fire, Moonwell) and the created item.
- **Curated, not exhaustive.** Each profession gets about 25-45 recipes in `recipes.rs`: a levelling ladder of 1.12's own filler (Copper Chain Belt, Rough Copper Bomb, Linen Bandage…), whose products sell and disenchant, plus every recipe that matters in survivors. The rest of 1.12's ~1,500 recipes are left out, so every authored item earns its place.
- **Learning.** The trainer teaches most recipes automatically at each rank. Home vendors sell a few, in limited stock that refreshes each run. The best are **rare recipe drops** from caches, elites and bosses, by zone band: Plans: Lionheart Helm, Pattern: Robe of the Archmage, Schematic: Arcanite Dragonling, Recipe: Flask of the Titans.
- **Stations.** Spell foci are honoured. **The camp** is a Cooking Fire, and Alchemy, First Aid, Enchanting, Leatherworking, Tailoring and Engineering need no focus. **Home** adds the Forge, the Anvil and the Moonwell, so smelting, Blacksmithing and Mooncloth happen between runs.
- **Cast times** are 1.12's. At camp, time spent crafting competes with spending banked picks.
- **Cooldowns.** Transmute: Arcanite (17187) and Mooncloth (18560) have a real-time cooldown of 20 hours, shown by the stock window. They are the long chase for Arcanite Reapers and Bottomless Bags.

### 5.2 What each profession is for in survivors

Every effect runs through the existing toolkit: `mechanics::timed_buff`, `party::heal` and `party::shield`, `party::pick_target`'s forced target for taunts, and the companion and summon code for pets. Every gear bonus is a `Bonus`.

**Alchemy** (Tradeskill window; camp and home)

| Kind | Examples (entry, verify) | In survivors |
|---|---|---|
| Healing potions (satchel) | Minor 118 to Major 13446 | Restore 15% → 40% of max health. **A shared 60 s potion cooldown** |
| Combat potions (satchel) | Swiftness 2459, Free Action 5634, Limited Invulnerability 3387, Mighty Rage 13442, Greater Fire Protection 13457, Restorative 9030 | +50% speed 15 s; immune to roots and slows 30 s; immune to physical 6 s; +25% damage 20 s; absorb fire (raid); cleanse a debuff every 5 s. Same shared cooldown |
| Elixirs (at the run's start or at camp) | Mongoose 13452, Giants 9206, Fortitude 3825, Greater Firepower 21546, Shadow Power 9264, Greater Arcane 13454, Superior Defense 13445 | Crit and speed; damage; max health; Fire, Shadow or Arcane damage; damage reduction. They last the run; **any two at once** |
| Flasks | Titans 13510, Supreme Power 13512, Distilled Wisdom 13511, Chromatic Resistance 13513 | A big max-health, spell-damage or cooldown buff, or resist all (raid). It lasts the run and **survives a down. One flask** |
| Transmutes | Arcanite, Iron to Gold, Mithril to Truesilver, Elemental Fire | Feed Blacksmithing and Engineering; the Philosopher's Stone (9149) is the tool |

The camp goblin keeps selling Mongoose and Fortitude at a premium, so a party with no alchemist still has them.

**Cooking** (a Cooking Fire: the camp's fire, or home)

- **Eating.** 3 s seated at the fire. Well Fed lasts until the next camp, so **every camp is a meal choice**, one food buff at a time.
- **Serve the party.** Cooking at camp hands one serving of each batch to every ally within 15 yd, relayed by the host as a `Gift`. The cook becomes the party's buff source.
- **Recipes:** Spiced Wolf Meat and Monster Omelet (max health), Smoked Sagefish (regen), Grilled Squid (crit), Nightfin Soup (cooldowns), Runn Tum Tuber Surprise (spell damage), Smoked Desert Dumplings (damage), **Dragonbreath Chili** (a chance on hit to breathe fire) and **Savory Deviate Delight** (a pirate or ninja costume until the next camp, for laughs).

**First Aid** (no focus; camp and home)

- **Bandages** (satchel). Linen Bandage 1251 to Heavy Runecloth Bandage 14530 heal 20% → 60% of max health over 6 s. Damage interrupts them as in 1.12, softened: **each hit removes 1 s of what is left**, so a bandage rewards kiting. Recently Bandaged (11196) blocks another for 60 s; bandages don't share the potion cooldown.
- **Anti-Venom and Strong Anti-Venom** cure a poison, for Naxxramas and raid mechanics.
- **Passive:** First Aid skill speeds up reviving a downed ally, +1% per 10 skill up to +30%. A co-op role that needs no crafting.
- **Ranks:** Expert from the vendor book, as in 1.12. Artisan from **Triage**, a camp mini-game in which you bandage wounded soldiers in order of need, as the 1.12 quest did.

**Enchanting** (the Craft window; camp and home; the Runed rods are tools)

- **Disenchant** (13262) works on any Uncommon or better bag item: crafted gear, duplicate boss copies, filler. It uses the stock item-pick cursor (`set_item_pick_armed`, then `take_item_picks`).
- **Disenchant in a run.** Enchanters get a third choice in the jewelry offer, **Disenchant**, beside "Take gold". A replaced piece can be disenchanted too.
- **Yields** follow 1.12's table with our odds: Uncommon gives dust (75%), an essence (20%) or a shard (5%); Rare a shard; Epic 1-2 Large Brilliant Shards or a Nexus Crystal. The tier follows the item's zone band.
- **Enchanting survivors gear.** The Craft window's Enchant asks for a target, and the Armory's Character tab (Bevy) highlights the worn pieces that fit, like WoW's targeting glow. The enchant is stored per piece (section 8), joins the piece's bonuses, and shows as the tooltip's green enchant line (`EnchantView`).

| Slot | Enchant (spell, verify) | Survivors effect |
|---|---|---|
| Main hand | Crusader 20034 | Chance on hit: heal 5% and +15% damage for 15 s |
| Main hand | Fiery Weapon 13898 | Chance on hit: a Fire bolt at the target |
| Main hand | Icy Chill 20029 | Chance on hit: slow the target's pack |
| Main hand | Lifestealing 20032 | Chance on hit: a drain that heals you |
| Main hand | Unholy Weapon 20033 | Chance on hit: Shadow damage over time |
| Main hand | Spell Power 22749 / Agility 23800 / Strength 23799 | +school damage / +crit / +damage |
| Chest | Greater Stats 20025 / Major Health 20026 | A small all-round bonus / +max health |
| Feet | Minor Speed 13890 / Greater Stamina 20020 | **+8% movement speed** / +max health |
| Hands | Greater Agility 20012 / Minor Haste 13948 / Riding Skill 13947 | +crit / −2% cooldowns / +pickup range |
| Wrist | Superior Strength 20010 / Healing 23802 | +damage / +healing done |

Head and legs take no enchant, as in 1.12. Wizard Oil and Brilliant Wizard Oil (20749/20750, verify) are satchel weapon coatings: +spell damage until the next camp.

**Blacksmithing** (home: Forge and Anvil)

- **Run consumables:** Sharpening Stones (Rough 2862 to Dense 12404, and Elemental 18262) and Weightstones (Rough 3239 to Dense 12643) give +Physical damage until the next camp. The Iron Shield Spike damages attackers when you are hit.
- **Gear,** up to Epic: Lionheart Helm 12640 (head, plate, crit), Arcanite Reaper 12784 (an epic main hand), the Imperial Plate pieces (a Rare set), and Dark Iron Plate and the Fiery Chain Girdle (MC, from Lava Core and Fiery Core).
- **The one Legendary.** The **Sulfuron Hammer** needs the Sulfuron Ingot from Golemagg. With the Eye of Sulfuras from Ragnaros it becomes **Sulfuras, Hand of Ragnaros**, the one Legendary craft.
- **Specialisation:** Armorsmith or Weaponsmith at 250. A Weaponsmith then picks Master Swordsmith, Axesmith or Hammersmith at 275 (1.12's three).

**Leatherworking** (camp and home)

- **Run consumables:** Armor Kits (Light 2304 to Rugged 15564, and the Core Armor Kit 18251) give +damage reduction until the next camp.
- **Gear:** Devilsaur Gauntlets 15063 and Leggings 15062 (a Rare two-piece set), Black Dragonscale (mail, MC), the Molten Helm and Corehound Boots (MC), and the Volcanic pieces (Fire Resistance for Ragnaros).
- **Specialisation:** Dragonscale, Elemental or Tribal at 225 (1.12's three).

**Tailoring** (camp and home; Mooncloth at home's Moonwell)

- **Bags** (3.2): the storage path for everyone, and the best thing to trade.
- **Cloth:** bolts, Mooncloth (on a cooldown) and Felcloth.
- **Gear:** Robe of the Archmage 14152 (Mage; Use: refresh your cooldowns, once a stretch), Robe of the Void 14153 (Warlock), Truefaith Vestments 14154 (Priest), Cindercloth and Shadoweave pieces (Fire and Shadow damage), Flarecore (MC, Fire Resistance) and Glacial (Naxxramas).
- **No 1.12 specialisation** (they came in TBC). Its late goals are the Bottomless Bag, the Core Felcloth Bag and the epic robes.

**Engineering** (camp and home; the Arclight Spanner and Blacksmith Hammer are tools)

- **Gadgets go on the satchel bar.** Each has its own cooldown, plus a shared 10 s engineering lockout so a bar of bombs can't be spammed.
- **Malfunctions** are rolled on each use and announced in their real flavour.

| Gadget (entry, verify) | Effect | Malfunction |
|---|---|---|
| Rough Copper Bomb 4360 → Thorium Grenade 15993, Dense Dynamite 18641 | Thrown at the densest pack: Fire damage; grenades stun 3 s | none (bombs are honest) |
| Goblin Sapper Charge 10646 | Huge damage around you | **always hurts you too** (as in 1.12) |
| Target Dummy 4366 → Masterwork 16023 | The horde chases it for 10-15 s (a forced target), and it soaks the hits | none |
| Goblin Rocket Boots 7189 | +60% speed 20 s | 5%: they explode, a 3 s knockdown and Fire damage |
| Gnomish Battle Chicken 10725 | A chicken pet that fights for 90 s and sometimes Battle Squawks (party −cooldowns) | none |
| Mechanical, Mithril and **Arcanite Dragonling** 4396/10576/16022 | A dragonling pet that breathes fire for 45-60 s | the older ones may turn on you |
| Gnomish Net-o-Matic Projector 10720 | Roots a cone of enemies for 10 s | 10%: it nets you instead |
| Gnomish Death Ray 10645 | A huge Shadow beam at the strongest enemy | drains your health; sometimes hits you |
| Gnomish Mind Control Cap 10726 | Controls an elite for 20 s | 5%: you walk into the horde for 5 s |
| Explosive Sheep 4384 | A sheep runs at the nearest pack and explodes | none |
| Goblin Jumper Cables 7148 / XL 18587 | **Revive a downed ally at once** | 33% / 25%: they fail (the real "shock") |
| Discombobulator Ray 4388 | Shrinks a boss: −20% damage for 10 s | 10%: it shrinks you |

**Specialisation at 200** (1.12's two):
- **Gnomish:** the Net-o-Matic, the Death Ray, the Battle Chicken and the Mind Control Cap. Quirky, and the malfunctions land on you.
- **Goblin:** the Sapper Charge, the Mortar, the Rocket Boots and the Dragon Gun. Big damage that hits you too.

### 5.3 The satchel bar

- **Three slots on keys 4, 5 and 6,** on the Bevy HUD beside the toy bar (7-9). Each shows the HUD's cooldown sweep (`SweepUiMaterial`) and the item's count across your bags.
- **Filling it.** Drag an item from the FrameXML bags onto a slot (a small bridge lets the cursor payload drop on a Bevy target), or use the Armory's loadout panel.
- **On entry.** The flask, elixirs and armor kit or stone you mark are applied as the run starts, so nobody spends the opening seconds clicking.
- **Cooldown categories:** potions 60 s, shared; bandages, Recently Bandaged; gadgets their own plus the 10 s engineering lockout; elixirs, flasks, food and stones only at the run's start or at camp.
- **More slots** may come later from achievements or perks (open question 7).

### 5.4 Ranks, trainers and money

- **Ranks.** Apprentice (cap 75), Journeyman (150), Expert (225) and Artisan (300) are bought from the trainer in the Armory's real `ClassTrainerFrame`, which also sells recipes. The prices are our own, about 1, 3, 8 and 20 runs' banked gold.
- **Profile gold (the purse) is new.** At the end of every run, win or lose, each player banks **25% of the gold the party collected**. It counts collected gold, not leftovers, so a friend's camp shopping never shrinks your bank.
- **Gold in runs.** The shared run purse is unchanged: the camp goblin's `MerchantFrame` shows and spends it. At home, the frames show the profile purse.
- **Other gold sources:** selling materials, crafted products, filler and duplicate gear at their authored `sell_price`.

---

## 6. Balance and economy

### 6.1 How crafted power fits

| Source | Scope | Budget (a full set, or a full use, against a fresh hero) |
|---|---|---|
| Talents | per class, permanent | as today |
| Boss gear | per class, permanent | a full Legendary set ≈ ×2 output (as today) |
| **Crafted gear** | the same slots and rarity ladder | **never above boss gear of the same rarity.** It is targeted (you pick the slot) and carries a flavour line (Lionheart crit, the Devilsaur pair). It tops out at Epic, plus the one Legendary, Sulfuras |
| **Enchants** | per piece, permanent | about +15-20% output for a fully enchanted set |
| **Flask, 2 elixirs, food, a stone or kit** | per run | about +15% together |
| **Potions, bandages, gadgets** | per use, on cooldowns | situational: recovery, escape, control, burst |

**Guardrails:** three satchel slots and shared cooldowns; buffs last a stretch (food, stones, kits) or a run (elixirs, flasks), never across runs; no consumable raises XP or loot; MC is tuned assuming Fire Resistance consumables exist, as the real raid was. A data test sums every crafted bonus at its best and asserts these budgets, as the gear tests already assert the Legendary doubling.

### 6.2 Gold sinks and faucets

- **Sinks:** trainer ranks and recipes; limited-stock recipe vendors; vendor reagents (vials, thread, flux, spices and salt, Runed rods, Elemental Flux for MC plate); bank bag slots at `BankBagSlotPrices.dbc`'s real prices; respecialising, 1.12's unlearn (about 10 runs' gold); later, cosmetics such as Fireworks.
- **Faucets:** banked run gold and selling.

The target is a mild surplus for a casual player. Gold should never block fun, but an Artisan rank should still feel earned.

### 6.3 How many professions per player?

**Recommendation: all twelve, account-wide, with no slot limit.**

- **Survivors' progression is already account-wide.** The profile is the account, one per machine. A per-class limit would mean levelling Mining nine times.
- **Casual friends play irregularly.** With a 2-primary limit, someone picks wrong, or the party has no alchemist on the night the raid needs Fire Protection Potions.
- **Shared nodes work best when everyone can harvest.** With every gathering skill, a crowded vein gives everyone ore.
- **Identity still forms.** Levelling all twelve to 300 is long (6.4), so friends specialise early and trade.

What is lost is WoW's interdependence. Three things keep some of it:
- **specialisations are exclusive** (Goblin or Gnomish; Armorsmith or Weaponsmith; Dragonscale, Elemental or Tribal), and switching costs gold;
- the **20-hour cooldowns** gate Arcanite and Mooncloth, so the party's supply comes from whoever keeps them rolling;
- **crafted gear is BoE and bags are tradeable,** so the best crafter is still the party's crafter.

### 6.4 Pace

Targets, tuned by a `SKILL_UP_STEP` constant and the node counts:

| Goal | Target |
|---|---|
| A gathering profession, 1 to 300 | about 10 runs (2.5 hours) |
| A crafting profession, 1 to 300 | about 15 runs' materials, plus the gold for its ranks |
| Everything | months of casual play |

---

## 7. Co-op

### 7.1 Who sees what

| Thing | Seen by | Decided by |
|---|---|---|
| Nodes, pools and skinnable corpses | everyone (a replicated `NetNode`: def, position, depletion) | host |
| A node you lack the skill for | everyone, greyed, with a red requirement line | |
| Someone gathering | everyone sees the animation; only you see your casting bar | |
| Your yield | **you** get a toast; the others see "Thrall receives loot: [Thorium Ore]." in the party feed | the host rolls it; your machine stores it |
| Your bags, purse, skills and recipes | you; others only through trade and tooltips | your machine |
| Party-affecting uses (Target Dummy, pets, Sapper Charge, Jumper Cables, served food) | everyone | the host spawns or applies them |
| Self-only uses (potions, bandages, elixirs) | everyone sees the buff (`NetBuffs`) | your machine applies them, and the host mirrors the buff on your hero |

### 7.2 Wire additions

These follow `coop/wire.rs`'s pattern, with `CampAction` as the model.

**`Hello`** gains `skills: Vec<(u16, u16)>` (skill line, rank) and the player's specialisations, and a `SkillsChanged` message follows each skill-up. That lets the host gate nodes and roll yields.

| Direction | Messages |
|---|---|
| Client to host | `GatherAction { node, Start \| Cancel \| Finish }`; `UseItem { item, at }` (party-affecting items only; the client has already consumed it); `Serve { item, count }`; `TradeAction(…)` |
| Host to client | `Yield { node, items }`; `Gift { item, count, from }`; `TradeUpdate(…)`; `TradeCommit` |
| Replicated | `NetNode`. Pets, dummies and dragonlings are ordinary units through `Look`, `NetPos` and `NetLife` |

The host checks range, depletion, and "already harvested by this player". As elsewhere, it trusts each client about its own machine: this is friends-only.

### 7.3 Crafting at camp, and shared materials

**Decision: no shared material pool.** Each player crafts from their own bags, and sharing happens three ways:

1. **Serve the party** (Cooking): one player's materials buff everyone.
2. **The real `TradeFrame`,** at camp and in the lobby, never while fighting. Drop an item on an ally's party frame (1.12's `DropItemOnUnit` opens a trade) or use the party-frame menu; six slots plus gold, with the seventh "Will not be traded" slot left out. The host relays each side's offer to the other screen (`set_trade`). When both accept, it sends `TradeCommit` to both, and each side removes its offered items (locked while offered) and adds the other's. A disconnect mid-commit can lose or duplicate an item, which is accepted for friends and logged.
3. **BoE crafted gear and bags** make natural gifts.

---

## 8. Save format

`PROFILE_VERSION` goes from 4 to **5**. As before, every field is new and defaults to empty: a version 4 save loads unchanged, and a version 4 build opens a version 5 save read-only rather than dropping data. New sections:

```toml
version = 5

[purse]
copper = 152340

[inventory]
bags = [4238, 10050, 0, 0]          # worn bag entries in containers 1-4; 0 is empty

[[inventory.items]]
bag = 0                             # 0 backpack, 1-4 worn bags, -1 bank, 5-10 bank bags
slot = 1
item = 2770
count = 10

[[inventory.items]]
bag = 1
slot = 3
item = 12640                        # Lionheart Helm, unequipped
count = 1
creator = "Thrall"                  # the tooltip's <Made by Thrall>

[bank]
purchased = 2
bags = [14046, 0, 0, 0, 0, 0]

[professions]
specs = { engineering = "gnomish", blacksmithing = "weaponsmith/hammersmith" }
recipes = [12640, 13510, 16022]     # learned beyond the trainer's automatic ones (spell ids)
tracking = "herbs"

[professions.skills]
mining = { rank = 182, max = 225 }  # keyed by slug; the code maps slugs to SkillLine ids
first_aid = { rank = 300, max = 300 }

[professions.cooldowns]
17187 = 1760912345                  # Transmute: Arcanite, ready at (unix seconds)

[satchel]
slots = [13446, 14530, 10646]
on_entry = [13510, 13452]           # applied as the run starts

[classes.mage.gear]
enchants = { "1611" = 1900 }        # piece id → SpellItemEnchantment id
```

- **Robustness.** Slots are sparse and keyed by (bag, slot). An unknown item entry is skipped with a warning and never fails the profile, as with unknown gear slots.
- **Test aid.** `WOW_SURVIVORS_BAGS=test|full|<n>` fills the bags in memory only, through `gear/store.rs`'s `Shadow` pattern, so screenshots never touch the save.
- **Two copies on one PC** share a profile (MULTIPLAYER.md). For inventory testing, give each copy its own `BENILLA_HOME`, or they will overwrite each other's bags.

---

## 9. Build plan

Each phase is playable on its own, and must pass the usual gate before the next starts:
- rustfmt on the touched files, and clippy;
- the survivors tests with and without `coop`;
- `benilla-formats`' data tests with `WOW_DATA`;
- a play build and a smoke launch.

Every phase that touches the UI also gets a **screenshot check at 1920x1080 and 1280x720**: `WOW_SURVIVORS_OPEN=<frame>` with `WOW_SURVIVORS_SHOTS=<dir>` (an extension of the existing run-options aid), over a live `WOW_SURVIVORS_AUTO` run. Someone actually looks at the shots.

### P0. Spike: the bags open over the HUD (time-box: 2 days)

**Build:**
- `SurvivorsUi`, the `ui_wanted` arm and the curated manifest;
- the `Bags` resource, with `WOW_SURVIVORS_BAGS=test`: ten items (ore, herbs, a potion, a bag, a crafted helm) and their authored templates;
- the container feed through `apply_container_source`;
- the move, split, swap, autostore and destroy drains, with the stock drains gated off;
- B to open all bags, Esc to close them, and the keyboard stand-down.

**It answers:**
- Does the curated `UIParent` load cleanly?
- What does it cost, at start-up and per frame (`UiFrameCost`)?
- Do the two UIs layer correctly?
- Does the web build still load in an acceptable time?

**Verify:**
- unit tests for every `Bags` operation: stack limits, a split into an occupied slot, a bag into itself, overflow;
- an engine test, `tests/survivors_bags.rs`, in the shape of `tradeskill_frame.rs`: load the manifest, push a `ContainerState`, drive `PickupContainerItem` and `SplitContainerItem`, and assert the queued intents;
- `ui_wanted` is true under survivors and false in a plain capture;
- **screenshot:** all five bags open bottom-right over a running horde, with an item tooltip on hover. The HUD's bars, ability icons, toy bar and minimap must be uncovered and nothing clipped at 1280x720; by hand, dragging an item must start no mouselook.

**Go / no-go.** If the curated `UIParent` can't load cleanly within the time-box, fall back to the full FrameXML with the stock frames hidden, and write down why.

**Result (2026-10-18): go, curated.** No fallback was needed.

- **Built:** `run_mode::SurvivorsUi` and the `ui_wanted` arm; the curated load in `ui_script/survivors_ui.rs`, which the sliced world-entry load and `ReloadUI()` both take; `survivors/professions/` with `Bags` (`inventory.rs`), the authored templates (`items.rs`) and the feed, drains and keys (`frames.rs`); the stock bag drains and `feed_item_stats` gated off under `SurvivorsUi`; the kill switch `WOW_SURVIVORS_FRAMEXML=0`; the screenshot aid `WOW_SURVIVORS_OPEN=bags` (with `WOW_SURVIVORS_SHOTS=<dir>`, it saves `bags.png`).
- **Load time:** 17 of the 89 `FrameXML.toc` rows plus `ContainerFrameAdapters.xml`. Measured by `survivors_ui_loads_the_curated_framexml_without_lua_errors` on the Mac (M1 Max, test profile: dependencies at O3, our crates at O1), three runs: curated **103 / 112 / 107 ms**, against **527 / 483 / 506 ms** for the whole interface (`load_default_ui`) in the same runs. That is about a fifth. The in-app figure is the `in-game UI up in N ms` log line, which still needs reading on the PC.
- **What worked:** the load raises no Lua error and builds no stock bar, unit frame, chat, minimap or buff frame. No `Bindings.xml` loads, so `GetNumBindings()` is 0 and no stock key does anything. B runs the stock `OpenAllBags`, and the backpack sits bottom-right above `CONTAINER_OFFSET_Y` 70. A drag, a split, the delete popup and an autostore all queue the stock intents, and the survivors drains apply them to `Bags`. Item tooltips render from the authored templates. Esc closes popups, the stack split and the bags before the survivors menu sees it. A click on a bag reaches neither the HUD buttons under it nor the wowcam.
- **What the plan got wrong:**
  - **The manifest:** the list in 2.1 was not enough. The bags also need the rows `BasicControls`, `MoneyFrame.lua`, `UIPanelTemplates.lua`, `UIMenu`, `UIErrorsFrame`, `StackSplitFrame` and `Cooldown`.
  - **Stand-ins:** the stock Lua indexes several frames from files we leave out, unguarded, so a stand-in shim runs after `UIParent.xml`. It hides a stand-in for each of these:
    - the bag-bar buttons;
    - `ChatFrameEditBox`;
    - `MerchantFrame`;
    - the options windows;
    - a 1-unit `BankFrame` on the left edge, because `updateContainerFrameAnchors` does arithmetic on `BankFrame:GetRight()`.

    The shim also sets `NUM_BAG_SLOTS`, `NUM_BANKBAGSLOTS` and `KEYRING_CONTAINER`, and stubs the missing callees as no-ops. It replaces `UIParent_ManageFramePositions` with a pass that only seats the bags, under `SURVIVORS_CONTAINER_OFFSET_Y`.
  - **`capture_ui_active`:** it is not extended (2.2). It gates only the capture's synthetic demo player, which must not run in survivors.
- **Not done or not measured in P0:**
  - **Per-frame cost:** `UiFrameCost` (`WOW_UI_COST=1`) is not measured.
  - **The web build:** not loaded.
  - **The screenshots:** the 1920x1080 and 1280x720 check is still to do on the PC.
  - **Right-click use:** `UseContainerItem` still goes to the stock applier, which does nothing without a server. It is P1/P2 work.
  - **The keyboard stand-down:** it covers only key presses. A key held while typing keeps acting, as in the reference.
  - **Item ids:** the entries are still unverified, since the repo has no `Item.dbc` parser. Several `ItemDisplayInfo` rows share each icon, so the display test proves only that the icon and the row agree.

### P1. Inventory, gathering and skills

**Build:**
- profile v5 (purse, inventory, skills) and templates for the gathering materials;
- nodes per band from `Lock.dbc`; gathering with `CastingBarFrame` and pushback; tracking blips;
- shared co-op nodes (`NetNode`, `GatherAction`, `Yield`); skinning (`Foe::skin`);
- run pools, and the camp bobber through `LootFrame`;
- skill-ups, Smelting (at home), the K skills pane, the P profession book, and trainers for the four gathering lines;
- selling at the camp goblin through the real `MerchantFrame` (his existing services become a Services button beside it), and banked run gold.

**Verify:**
- data tests: every node's lock resolves in `Lock.dbc` to the right skill; every authored display id and icon exists; every node, pool and drop item is in the table;
- skill-up band tests against the vmangos numbers, and save round-trips (v4 → v5, v5 → v5);
- **a co-op loopback test** (on the suite's UDP pattern): two players finish the same vein, each gets their own yield, the node depletes, and a third harvest is refused;
- **screenshot, at both sizes:** Copper mined mid-fight with the casting bar above the XP bar; the skills pane; the fishing loot window at camp; the merchant with the bags open.

**Progress (2026-10-19): both halves are built; the profession book and the screenshots are not.**

- **Built:**
  - profile v5, with `[inventory]` and `[professions.skills]`;
  - the gathered materials' templates;
  - nodes per band, whose displays and `Lock.dbc` ranks are checked by a data test;
  - gathering while fighting, with the real gather spell's kits, the stock `CastingBarFrame` (now a curated row), pushback and the walk-off interrupt;
  - shared co-op nodes: the replicated `NetNode`, `GatherAction` with `Yield`, and the loopback test;
  - skinning on beast kills;
  - fishing pools on water, found through the world's liquid query;
  - the camp bobber's catch landing in the bags;
  - skill-ups on vmangos's bands;
  - the potion, drunk with a right-click;
  - K's skills readout.

  The co-op protocol is now 16.
- **Built, the economy half** (`econ.rs`, `stations.rs`, `recipes.rs`, `tracking.rs`):
  - **The purse:** `[purse] copper` in profile v5, shown on the Backpack's stock money line and in every window's. At the end of each recorded run, win or lose, each machine banks 25% of the gold the party collected, as a results note ("You bank 62s 50c of the party's gold.").
  - **The merchant:** the goblin's click opens the stock `MerchantFrame` (now a curated row) beside his own window. It sells Coal, Empty Vials and a Linen Bag for the purse, and buys any bag item at its `sell_price`: a right-click on a bag slot or a stack dropped on the window. It keeps a 12-row buyback tab.
  - **The trainer:** the camp's Train button opens the stock `ClassTrainerFrame` (`Blizzard_TrainerUI`). It sells Journeyman, Expert and Artisan on all four lines: their real rank spells, 1.12's skill requirements (50, 125, 200) and caps (150, 225, 300). Every hero now starts as an Apprentice, capped at 75.
  - **Smelting:** the camp's Smelt button opens the stock `TradeSkillFrame` (`Blizzard_TradeSkillUI`). It lists Copper, Tin, Bronze, Iron, Steel, Mithril, Truesilver and Thorium. A cast takes 3 s under the stock casting bar, consumes the reagents, makes the bar, and rolls Mining on `SkillLineAbility`'s bands. Create All repeats while the ore lasts.
  - **Tracking:** Find Minerals and Find Herbs put the gold resource dot of `ObjectIcons.blp` on each standing vein or herb on the survivors minimap. Their icon sits in a `MiniMapTrackingFrame`-style button on the map's ring.
  - **Data tests:** these check against `Spell.dbc`, `SkillLine`/`SkillLineAbility`, `SpellCastTimes`, `SpellFocusObject` and `ItemDisplayInfo`:
    - each rank spell's name, rank, icon, line and skill step;
    - the tracking spells;
    - each Smelt spell's reagents, product and count, trivial bands, Forge focus and cast;
    - the Minor Healing Potion recipe's reagents, which vouch for Empty Vial;
    - every new item's display and icon.

    An engine test drives all three windows on the real files with no Lua error.
  - **The curated load:** it grows to 20 toc rows, two template files no row names (`CharacterFrameTemplates.xml` and `QuestFrameTemplates.xml`), two layer files and the two addons, loaded eagerly. Measured as in P0, on the Mac, three runs: curated **144 / 136 / 135 ms**, against **488 / 489 / 487 ms** for the whole interface, up from about 105 ms.

  The wire is unchanged: the purse, the shop, the trainer, Smelting and tracking are all each player's own, so the co-op protocol stays at 16.
- **Deviations:**
  - **Skills on the wire:** the client's rank rides on `GatherAction`, rather than in `Hello` plus `SkillsChanged`.
  - **Creature type:** `Foe` gained no `skin` field. Beasts are recognised by name, and a great beast, elite or boss is skinned outright, with no corpse left to work.
  - **The skills pane:** K shows the stock `GameTooltip` as a readout, because `SkillFrame` needs `CharacterFrame`, which is far outside the curated load.
  - **Camp fishing:** the fish goes straight to the bags, not through `LootFrame`.
  - **Silver, Gold and Dark Iron:** no vein, since no verified display.
  - **The Stranglekelp, Briarthorn and Earthroot herbs:** left out.
  - **The exchange rate:** a run's gold piece banks as a silver, so a 250-gold run banks 62s 50c. Each machine counts the party's collected gold as every rise of the replicated run purse, so camp shopping never shrinks it. A pickup and a purchase in the same snapshot would net out, which a camp's lack of spawns makes rare.
  - **Which purse the merchant spends:** the goblin's `MerchantFrame` buys, sells and shows the profile purse, not the run purse (5.4 had it spend the run purse). His run-gold tokens and elixirs stay on his Bevy window, retitled Services and moved beside the stock frame, and still spend the shared run purse through the camp's host-checked `CampAsk::Buy`.
  - **Trainers at camp, not the Armory:** one "Gathering Trainer" lists all four lines, grouped by line, as a tradeskill trainer. Apprentice is known from the start at no cost. The prices are ours, in 1.12's coin: Journeyman 1g, Expert 4g, Artisan 10g. The Smelting recipes are learned at their 1.12 trainer skill (from memory; the client does not carry it) without a visit.
  - **Smelting at camp:** the campfire stands in for the Forge (the window's "Requires: Forge" is met), not home. The Silver, Gold and Dark Iron smelts are left out until their veins exist.
  - **The LoadOnDemand addons:** `Blizzard_TrainerUI` and `Blizzard_TradeSkillUI` load with the rest, since the curated load has no addon registry. The stand-ins make `ClassTrainerFrame_LoadUI` and `TradeSkillFrame_LoadUI` no-ops, and their `ADDON_LOADED`s fire with the bags' first push.
  - **Tracking's switch:** the minimap button cycles off, minerals, herbs, in place of the profession book. One tracking is on at a time, as in 1.12, and it is saved as `[professions] tracking`.
  - **Not verified:** Coal's icon and display row (`INV_Stone_15`, 15805) and Empty Vial's row (18076) are from memory. Their entries are verified (`Spell.dbc`'s Smelt Steel and Minor Healing Potion), and each row is checked to carry its icon.
- **Still to do in P1:**
  - the P profession book;
  - the Armory's (home) trainer and merchant;
  - the screenshot check, now including the merchant beside his Services, the trainer and Smelting at camp, and the tracking button.

### P2. Alchemy, Cooking, First Aid, Enchanting, and the satchel

**Build:**
- the four professions' curated recipes; the tradeskill and craft feeds; the craft drain (casts, Create All);
- `effects.rs`, with the potion, bandage and food rules, and Serve the party;
- Disenchant (in the bags and from the jewelry offer); enchants on gear (the Armory highlight, `GearSave::enchants`);
- the satchel bar on 4-6 with on-entry buffs, and recipe drops.

**Verify:**
- every recipe's reagents and products match `Spell.dbc`; the power-budget test (6.1);
- the shared potion cooldown, Recently Bandaged, the bandage's shortening on hits, and the disenchant tables;
- an enchant changes the worn bonuses and survives a save;
- co-op: served food reaches each ally within 15 yd, and no one else;
- **screenshot:** the Alchemy window at camp (recipe colours, reagent counts, the bags); the Enchanting window beside the Armory highlight; the satchel bar mid-run with a cooldown sweeping; a potion tooltip.

### P3. Blacksmithing, Leatherworking, Tailoring, Engineering, and crafted gear

**Build:**
- the four professions; the crafted gear range and "Equip" from the bags into the collection (BoE); duplicate boss drops as Soulbound copies;
- tailored bags, stones and armor kits;
- the gadgets, with malfunctions and co-op replication (`UseItem`);
- the MC materials, the Fire Resistance crafts and the Sulfuras chain.

**Verify:**
- crafted display ids and icons exist (the gear test, extended); a crafted Rare never beats the boss Rare in the same slot;
- a class can't equip the wrong armour (the tooltip's class line is red); equipping removes the bag item and adds the collection entry;
- each malfunction rolls at its rate over 10,000 seeded uses; the Target Dummy redirects `party::pick_target`;
- co-op: Jumper Cables revive a remote ally, and a Sapper Charge hurts its user on every machine;
- **screenshot:** the Blacksmithing window at home with the Forge met; a Lionheart Helm tooltip in the bags (BoE line, class line, green bonus lines); a Battle Chicken and a Target Dummy mid-run; the bag bar with a Mooncloth Bag.

### P4. Polish: the bank, trade and specialisations

**Build:**
- the `BankFrame` at home (24 slots, bank bag slots bought with gold);
- the `TradeFrame` between players: the host relay, the two-phase commit, and `DropItemOnUnit` on party frames;
- specialisations and unlearning, the 20-hour cooldowns, limited-stock recipe vendors, the overflow mailbox (`MailFrame`), and Triage;
- achievements ("Master of All Trades", "Arcanite Reaper Forged"), the toast.house page and the README.

**Verify:**
- a trade in the two-window soak test (one PC, two `BENILLA_HOME`s), including a cancel from each side and a disconnect mid-trade;
- bank slot prices match `BankBagSlotPrices.dbc`; a specialisation switch unlearns exactly its recipes;
- **screenshot:** the bank with the bags open; a trade window on both screens; a trainer showing a specialisation.

---

## 10. Risks

| Risk | Mitigation |
|---|---|
| The curated `UIParent` references frames we didn't load, and errors | P0's time-box and go/no-go; the full-load-plus-hide fallback; the engine test catches Lua errors |
| FrameXML load time (desktop start-up; about 3 s, sliced, in the browser) | Measured in P0. It loads once per session, and the kill switch exists |
| The two UIs fight over input, scale or layering | One owner per queue; the `UiKeyboardCapture` stand-down; Esc arbitration; screenshots every phase |
| A stock drain and a survivors drain both empty the same queue | Gate every stock drain with `not(survivors::active)`, and test that each queue has exactly one drainer under survivors |
| Wrong ids from memory | The closure under `Spell.dbc`; display and icon tests; the cores' dump for names and game objects. Each test lists every mismatch, as the gear tests do |
| Scope: 12 professions and hundreds of items | Curated recipe lists; phases that each ship playable; filler products share one template shape |
| Power creep trivialises runs | The budget test (6.1), the satchel's limits, buffs that last a stretch, MC tuned with consumables in mind |
| Save growth or corruption | Sparse keyed slots, unknown entries skipped, read-only on older builds, the version bump |
| Co-op duplicates from a trade or gift on a disconnect | The two-phase commit; accepted as friends-only, and logged |
| Real-time cooldowns frustrate weekly players | Only two of them (Arcanite, Mooncloth); open question 8 |

## 11. Decisions

1. **A hybrid UI:** stock FrameXML for bags, tooltips, tradeskill and craft, merchant, loot, skills, profession book, trainer, bank, trade and the casting bar; Bevy for the HUD, the satchel bar and the Armory.
2. **A curated FrameXML manifest** under a new `SurvivorsUi` flag, with no stock bindings and a kill switch.
3. **`Bags` and `Professions` are the source of truth.** Survivors' feeds reuse `apply_container_source`, and the stock net drains are gated off.
4. **Real 1.12 entries, names, icons and display ids** with authored templates; the id set is closed under `Spell.dbc`.
5. **The satchel is a 3-slot bar** (keys 4-6) over the bags, not a second container.
6. **The haul is kept, win or lose.**
7. **Gear stays a collection.** Crafted gear and duplicate drops are bag items until they are equipped (BoE), sold or disenchanted.
8. **Shared party nodes:** the host rolls each player's own yield.
9. **All twelve professions, account-wide;** exclusive specialisations restore some interdependence.
10. **Profile gold:** each player banks 25% of the gold the party collected.
11. **The bank is at home only;** the camp has the merchant, the fire and the bobber.
12. **No shared material pool:** sharing is by Serve the party, the trade window and BoE gifts.

## 12. Open questions

1. **Profession limit:** all twelve, account-wide (recommended), or WoW's 2 primary plus 3 secondary for stronger party identity?
2. **The haul on a loss:** keep it all (recommended), or lose a share of the run's materials on defeat, for stakes?
3. **Banked gold:** is 25% of the party's collected gold right? And should gold and Faire tickets stay strictly separate?
4. **The bank:** at home only (recommended), or a banker at camp too?
5. **Specialisations:** Alchemy, Tailoring and Enchanting have none in 1.12. Allow TBC-style ones (Potion, Elixir or Transmute Master), or stay strictly 1.12?
6. **Keys:** B bags, K skills, P profession book, 4-6 satchel. Any to move? B still banishes while the hand strip is open.
7. **Satchel size:** three slots, growing to four or five through achievements, or fixed?
8. **Cooldowns:** real time (20 h for Arcanite and Mooncloth), or counted in runs (ready again after 2 runs)?
9. **The crafted ceiling:** Epic plus the one Legendary (Sulfuras)? Or should crafting stop at Rare, so bosses stay the best source?
10. **An Auction House between friends** (the engine has the stock `AuctionFrame`): wanted later, or is trade enough?
11. **Pace:** about 10 runs to max a gathering skill and 15 for a craft. Faster for a casual group?
