# Raid mode: Molten Core

Status: the boss mechanics library (`survivors/raid/`) and all ten Molten Core bosses are built, each as its own raid stage (map 409): Lucifron, Magmadar, Gehennas, Garr, Shazzrah, Baron Geddon, Sulfuron Harbinger, Golemagg, Majordomo Executus and Ragnaros. Each boss walks out at 6:00 and its kill wins the stage (Majordomo's surrender wins his). Ragnaros's lair opens only once Majordomo's camp is won. The raid shell (segments, lockout, wipes), T1 loot, Need/Greed for raid items and the social layer are still design only. This is the plan for the first raid. The ROADMAP's "Raid night" section points here.

Ground rules, as for the rest of the fork:

- **Names come from the real 1.12 client.** The names, spells, art, models and sounds are the real 1.12 ones. The numbers are our own.
- **Ids are unverified.** Every spell id below is from memory and is marked "verify against Spell.dbc". Data tests must check each id's name in Spell.dbc before anything uses it. Item, NPC and quest ids are checked the same way against the cores' `item_template` / `creature_template` / `quest_template`, as `gear/catalog.rs` and `jewelry/catalog.rs` already do for display ids.
- **No real money.** Wagers and rewards are Faire tickets only.

---

## 1. Pitch and session shape

**Pitch.** Five friends zone into Molten Core for one raid night. Between boss rooms they push through trash hordes in the lava tunnels, levelling and picking cards as they would in any run. Each boss is an arena fight with WoW's real mechanics, rebuilt so that heroes who never press a spell button can read them and answer them by moving. The bosses come in order, Lucifron to Ragnaros, with loot rolls, side bets and recriminations in between. A clean clear takes about 50 minutes, and a typical night with a few wipes takes 60-75.

### The night at a glance

Trash segments are horde runs on a "pull list": two or three named packs (elites) inside a stream of normal trash. Clearing the pull list opens the next boss room. Boss fights are arena fights. The horde drops to a trickle of themed adds there, so the mechanics stay readable.

| # | Segment | Content | Target |
|---|---|---|---|
| T1 | The entrance | Molten Giants (Stomp knockback), Molten Destroyers, Lava Surgers (line charges) | 2:30 |
| B1 | **Lucifron** | Curses, Impending Doom, Dominate Mind | 2:30 |
| T2 | Magmadar's lair | Core Hound packs (kill all 5 within 10 s or they rise again), an Ancient Core Hound | 2:00 |
| B2 | **Magmadar** | Panic, Lava Bomb, Frenzy | 3:00 |
| T3 | Lava tunnels | Lava Annihilators, Firewalkers, Flameguards; douse a rune | 2:00 |
| B3 | **Gehennas** | Healing curse, Rain of Fire | 2:30 |
| T4 | Firelord hall | Firelords splitting into Lava Spawn; douse a rune | 2:00 |
| B4 | **Garr** | Firesworn adds that erupt | 3:00 |
| T5 | Lava packs | Lava Elementals, Lava Reavers; douse a rune | 2:00 |
| B5 | **Shazzrah** | Blinks, curse, Counterspell | 2:30 |
| T6 | Geddon's approach | Firewalkers, Lava Surgers; douse a rune | 2:00 |
| B6 | **Baron Geddon** | Living Bomb, Inferno | 3:00 |
| T7 | Flamewaker camp | Flamewaker packs (Fist of Ragnaros knockdowns); douse a rune | 2:00 |
| B7 | **Sulfuron Harbinger** | Priests that heal, Flame Spear | 2:30 |
| T8 | Golemagg's ridge | Molten Destroyers, Lava Reavers; douse a rune | 2:00 |
| B8 | **Golemagg the Incinerator** | Magma Splash tank swap, Core Ragers, Pyroblast | 3:00 |
| T9 | The Lava Pool | Douse the last rune; Majordomo's camp appears | 1:30 |
| B9 | **Majordomo Executus** | Reflect shields, Teleport; "I submit!" | 3:00 |
| C | The summoning | Ready check, then the Ragnaros cutscene | 0:30 |
| B10 | **Ragnaros** | Knockback, melee check, Sons of Flame, submerge | 6:00 |

The fights add up to about 29 minutes and the trash to about 18. Ten loot breaks of about 45 s each add another 8. That makes about 55 minutes with no wipes.

**The runes (flavour that is also an objective).** In 1.12, Majordomo appears only after seven Runes of Warding are doused with Aqual Quintessence from Duke Hydraxis. We keep that. After each rune boss, the next trash segment includes "Douse the Rune of X", which uses the quest system's existing hold-a-marked-spot objective (5 s). The runes are Kress (Magmadar), Mohn (Gehennas), Blaz (Garr), Mazj (Shazzrah), Zeth (Geddon), Koro (Sulfuron) and Theri (Golemagg). Verify the rune-to-boss pairing against the cores' gameobject data.

### Lockout and wipes

- **The raid save is a lockout for the night.** The host's profile keeps a `RaidSave` (bosses killed, runes doused, attempts per boss, each player's hero snapshot, the loot log and open bets). It is written at every checkpoint (each pull and each break), so a crash or disconnect resumes from the last one with "Resume raid" in the lobby. The lockout lasts one raid night. There is no weekly lockout, because the friends' schedules are irregular; one could come later.
- **A wipe** happens when every hero is down at once (the existing `party::all_down`). A 10 s screen plays with the real "Release Spirit" dialog and sound. The party respawns at the room entrance at full health; kills are kept and the boss resets. Every build is restored to its pull snapshot, so XP from a failed attempt is lost and wipes can't farm XP. The boss frame shows "Attempt 3".
- A wipe in a trash segment restarts that segment from its own snapshot.
- **Reviving in a boss fight.** The ordinary ally revive still works but takes twice as long. Battle-resurrections work as in 1.12: Druid Rebirth once per boss, a Warlock Soulstone placed before the pull, Shaman Reincarnation once per boss. Other resurrections work only in breaks, where every downed hero is raised automatically.

### Attunement (the Molten Core fragment)

The quest system already gives every zone a three-step chain (`quests/chain.rs`). Attunement is a new zone chain whose reward is "Attunement to the Core":

- **New zone: Blackrock Mountain** (map 0, the chained lava hall between Blackrock Depths and Blackrock Spire), which we need anyway as a fallback arena (section 8). Foes: Dark Iron dwarves, Fireguard elementals, Blackrock orcs.
- **Steps:** (1) turn in three of its in-run quests for Lothos Riftwaker; (2) kill its last boss, Overmaster Pyron, the Fireguard elemental at the Blackrock Depths gate; (3) win there to recover the Core Fragment (item 18412, verify) and turn in "Attunement to the Core" (quest 7848, verify).
- **Non-blocking.** In 1.12, players who weren't attuned could still enter by jumping through the window in Blackrock Depths. So only the host must be attuned to open the raid. Attuned heroes arrive by Lothos's teleport; the others arrive by a short camera fall, the window jump, played for laughs.

### How the survivors loop carries across the raid

**Decision: levels and builds persist across the whole raid night.** They do not reset per boss.

Why:

- The best part of a survivors run is the build snowballing. A raid night should be one long build story ("our Molten Core build"), not ten re-drafts of the same opening cards.
- Resetting per boss would make every fight's first minute the same weak start, and would make late bosses no harder to build for than early ones.
- The lockout snapshot already exists for wipes. It also gives persistence for free across crashes and disconnects.
- The cost is that power runs away. We control it with par levels (below) rather than with resets.

How it works:

- **Par levels.** Each boss is tuned for a party at its par level. Trash XP follows a raid curve, stretched so the party reaches par as it finishes each segment:

  | Boss | Lucifron | Magmadar | Gehennas | Garr | Shazzrah | Geddon | Sulfuron | Golemagg | Majordomo | Ragnaros |
  |---|---|---|---|---|---|---|---|---|---|---|
  | Par level | 15 | 24 | 32 | 40 | 47 | 54 | 60 | 66 | 72 | 78 |

- **Catch-up and cap.** A party under par at a pull gets the gap as banked picks ("raid catch-up"). XP more than 5 levels over par turns into gold.
- **No level-up screens mid-fight.** XP earned during a boss fight is banked. The banked picks, plus a fixed 2-level kill reward, open at the loot break, using the no-pause co-op level-up's banked picks and Breather. So each boss tests the build you brought in.
- **Raid card pool.** On top of the normal pool, raid mode adds Fire Resistance cards (Fire Resistance Aura, Fire Resistance Totem, Greater Fire Protection Potion; a new bonus that cuts fire damage taken), the Restorative Potion (item 9030, verify: removes one curse, magic, poison or disease effect every 5 s for 30 s), and the co-op design's role cards (+threat, heal the lowest ally, taunt on mobility).
- **Jewelry, companion and mounts** work as in any run. Heroes ride their Stable mounts between pulls.

### Roles: what each class does automatically

Players declare a role at the ready check, from what their class can be. The role sets threat multipliers, which role cards are offered and which role abilities are on. **Role abilities fire on their own**, on a cooldown, at the best target in range. The player's job is to be in range, so positioning is the whole skill.

| Class | Roles | Automatic role abilities (spell ids: verify against Spell.dbc) |
|---|---|---|
| Warrior | Tank, Damage | Taunt 355; Pummel 6552 (interrupt); Berserker Rage 18499 (fear immunity) |
| Paladin | Tank, Healer, Damage | Righteous Defense (taunt, as in our co-op design); Cleanse 4987 (magic, poison, disease) |
| Hunter | Damage | Tranquilizing Shot 19801 (removes a boss Frenzy) |
| Rogue | Damage | Kick 1766 (interrupt) |
| Priest | Healer, Damage | Dispel Magic 527 (magic on allies, magic buffs on enemies); Fear Ward 6346 |
| Shaman | Healer, Damage | Purge 370 (enemy magic buffs); Tremor Totem 8143 (fear); Earth Shock 8042 (interrupt) |
| Mage | Damage | Remove Lesser Curse 475; Counterspell 2139 (interrupt); Polymorph 118 (ends a mind control) |
| Warlock | Damage | Banish 710 (elementals); Soulstone Resurrection 20707 |
| Druid | Tank, Healer, Damage | Growl 6795 (taunt); Remove Curse 2782; Rebirth 20484 (battle-resurrection) |

**How a dispel lands.** A dispel pulses every 4 s and removes one debuff of its types from the most urgent afflicted ally within 15 yd. Curses are Mage and Druid only, as in 1.12, so a cursed hero has to run to the decurser. That is the readable part of it.

**Fallbacks for groups that lack a class:** Aqual Quintessence fonts (water pools in boss rooms that cleanse curses and magic from anyone standing in one for 2 s), Restorative Potion cards, and the composition helpers in section 7.

---

## 2. Boss by boss

| # | Boss | Mainly tests | Coordination | Hard enrage (5p) | T1 slots |
|---|---|---|---|---|---|
| 1 | Lucifron | Healers, dispellers | Decurse, Doom huddle, Mind Control spacing | 3:00 | Hands, Feet |
| 2 | Magmadar | Tank, positioning | Panic fear direction, Lava Bomb floor | 3:30 | Legs |
| 3 | Gehennas | Healers | Decurse under heavy damage, Rain of Fire | 3:00 | Hands, Feet |
| 4 | Garr | Damage, add control | Firesworn spacing, eruption chains | 4:00 | Head |
| 5 | Shazzrah | Tank, ranged discipline | Blink pickups, Counterspell range | 3:00 | Hands, Feet |
| 6 | Baron Geddon | Everyone | Living Bomb runs, Inferno | 3:30 | Shoulder |
| 7 | Sulfuron Harbinger | Damage, interrupts | Priest kill order, Flame Spear spread | 3:30 | Shoulder |
| 8 | Golemagg | Tanks | Magma Splash swap, Pyroblast share | 4:00 | Chest, Legs |
| 9 | Majordomo Executus | Everyone | Reflection discipline, Teleport cover | 4:00 | none (cache) |
| 10 | Ragnaros | Everyone | Melee check, knockback, Sons, submerge | 10:00 | T2 legs |

The T1 drop map is the 1.12 one, from memory; verify it. T1 wrists and waists drop from trash.

**Hard enrage (all bosses):** Berserk, +200% damage and +50% speed, with a yell or emote and the raid-warning sound. It is a wipe in practice. Each boss also has a soft enrage, described below.

### 1. Lucifron

The Flamewaker lieutenant, with two Flamewaker Protectors. NPC 12118 (verify).

- **In WoW:** Impending Doom (a magic debuff that hits hard after 10 s unless dispelled), Lucifron's Curse (doubles ability costs) and Shadow Shock; the Protectors cast Dominate Mind and Cleave.
- **In survivors:**
  - **Lucifron's Curse:** every 20 s on 2 random heroes. Their auto-casts run at half rate for 20 s. It is a curse: the Mage or Druid decurse clears it, or a font, or a potion.
  - **Impending Doom:** every 25 s on every hero within 30 yd. A purple countdown shows over each head, and at 0 it deals 35% of max health. It is magic, and the healer's dispel pulse clears one ally per tick. So doomed heroes huddle on the Priest or Paladin, the one moment in MC where stacking is right.
  - **Dominate Mind (Protectors):** every 30 s, one hero for 8 s. The hero turns purple and walks toward the nearest ally, and its spells hit allies at 30% damage. Allies spread away from it. A magic dispel, or the Mage's automatic Polymorph, ends it early.
  - **Shadow Shock:** a 10 yd ring around Lucifron with a 1.5 s wind-up. Only the tank belongs inside.
  - **Protectors:** they chase by threat and Cleave in a cone at their target. Kill them first. A skull marker is set on one automatically.
- **Tests:** healers and dispellers; everyone's spacing.
- **Soft enrage:** after 2:00 the curse targets 3 heroes. **Hard:** 3:00.
- **Loot:** T1 hands or feet; one roll from the shared MC pool (Talisman of Ephemeral Power, Choker of Enlightenment and others).
- **Spells (verify against Spell.dbc):** Impending Doom 19702, Lucifron's Curse 19703, Shadow Shock 19460, Dominate Mind 20604.

### 2. Magmadar

The core hound lord, in his lair. NPC 11982 (verify).

- **In WoW:** Panic (an area fear about every 30 s), Lava Bomb (a fire patch under a random player), Frenzy (Hunters Tranquilizing Shot it off) and Magma Spit (hurts melee).
- **In survivors:**
  - **Panic:** a 2 s wind-up as Magmadar rears up, with a yellow 30 yd ring; every hero inside flees straight away from him for 6 s (forced movement). **The read:** before each Panic, stand where "away from Magmadar" is clear floor, not a pool or the lava edge. Tremor Totem (within 20 yd), Fear Ward, Berserker Rage and Will of the Forsaken make a hero immune. A feared tank drops the boss and he runs for the nearest soft target, so the party makes the tank immune or stacks behind it.
  - **Lava Bomb:** every 8 s, a 6 yd circle under a random hero (1.5 s), leaving a pool that burns for 30 s. The pools build up. Good players drop them at the edges by keeping the fight moving outward.
  - **Frenzy:** every 15 s for 8 s: +60% attack speed and a wider melee cone. The Hunter's automatic Tranquilizing Shot (within 30 yd) removes it. Without one, the healer and the tank's cooldowns have to cover it.
  - **Magma Spit:** a stacking fire damage-over-time on heroes in melee range. Only the tank should be there.
- **Tests:** the tank (fear and Frenzy) and positioning.
- **Soft enrage:** after 2:30, Lava Bomb pools never expire. **Hard:** 3:30.
- **Loot:** T1 legs; Striker's Mark, Earthshaker, Medallion of Steadfast Might, Eskhandar's Right Claw.
- **Spells (verify against Spell.dbc):** Panic 19408, Lava Bomb 19411, Frenzy 19451, Magma Spit 19449.

### 3. Gehennas

A Flamewaker caster with two Flamewaker guards. NPC 12259 (verify).

- **In WoW:** Gehennas' Curse (-75% healing received), Rain of Fire and Shadow Bolt. His Flamewakers cast Sunder Armor and Fist of Ragnaros.
- **In survivors:**
  - **Gehennas' Curse:** every 25 s on all heroes within 30 yd, for 30 s: healing received -75%. Decurse.
  - **Rain of Fire:** every 12 s, three 8 yd circles under random heroes (2 s wind-up) that burn for 6 s.
  - **Shadow Bolt:** a slow purple bolt at a random hero who isn't the tank, for 30% of max health. It is the healers' spike to catch.
  - **Flamewakers:** Sunder Armor stacks on the tank, and Fist of Ragnaros knocks down everyone within 8 yd for 2 s.
- **Tests:** healers healing through the curse, and decursers prioritising it.
- **Soft enrage:** after 2:00, Rain of Fire drops five circles. **Hard:** 3:00.
- **Loot:** T1 hands or feet; one shared-pool roll.
- **Spells (verify against Spell.dbc):** Gehennas' Curse 19716, Rain of Fire 19717, Shadow Bolt 19728, Fist of Ragnaros 20277.
- **Status: built** (stage "Molten Core: Gehennas' Cavern", the spike's centre cavern). Gehennas' Curse (every 25 s on everyone within 30 yd, healing received x0.25 for 30 s, a curse), Rain of Fire (8 yd circles on the curse count with a 2 s wind-up, burning 6 s; five from 2:00), Shadow Bolt (a 3 yd circle under a hero off the tank, 30%), two Flamewakers as plain adds, Berserk at 3:00. Display 13030; aggro and death are the salamander's `NagaAggro`/`NagaDeath` kits. Not built: the Flamewakers' Sunder Armor and Fist of Ragnaros.

### 4. Garr

The core giant and his Firesworn. NPC 12057 (verify).

- **In WoW:** Firesworn adds that Immolate, erupt when they die and enrage (Separation Anxiety) if dragged too far from Garr; Antimagic Pulse strips buffs; Magma Shackles slows everyone nearby.
- **In survivors:**
  - **Firesworn:** 8 at 5 players. When one dies it erupts 1 s later, a 10 yd fire nova marked by a red ring. An eruption hurts other Firesworn too, so killing two side by side chains them. That is fast but deadly if friends are close. Kill them one at a time, away from the group, or chain them deliberately inside the horde.
  - **Separation Anxiety:** a Firesworn more than 40 yd from Garr enrages (x2 damage). A tether line shows the distance.
  - **Banish:** the Warlock's automatic Banish holds one Firesworn for 8 s, which is how a 5-player group spaces the kills.
  - **Antimagic Pulse:** every 15 s, a 20 yd blue ring that strips shields and buffs. Step out to keep them.
  - **Magma Shackles:** heroes within 15 yd are slowed by 50% for 6 s (telegraphed), so running from an eruption gets harder.
- **Tests:** the damage dealers' add control and spacing.
- **Soft enrage:** from 2:30, Garr detonates one living Firesworn every 15 s (Massive Eruption, 20 yd). **Hard:** 4:00.
- **Loot:** T1 head. **Bindings of the Windseeker (left)** at a low chance (section 4). Gutgore Ripper, Aurastone Hammer, Drillborer Disk, Brutality Blade.
- **Spells (verify against Spell.dbc):** Antimagic Pulse 19492, Magma Shackles 19496, Eruption 19497, Immolate 20294, Separation Anxiety 23492, Massive Eruption 20483.
- **Status: built** (stage "Molten Core: Garr's Cavern", the west cavern). Separation Anxiety is built too: a Firesworn more than 40 yd from Garr deals double damage and wears 23492. Firesworn 3/4/5/6/8 by party size; each erupts where it falls (a red 10 yd ring, 1.5 s, 25% to heroes and 60% of their health to the other Firesworn inside, so neighbours chain) and gives Garr a stack of Enrage 19516 (+15% damage). Antimagic Pulse (a blue 20 yd ring that strips shields and timed buffs), Magma Shackles (a yellow 15 yd ring, half pace for 6 s, magic), Massive Eruption detonating the nearest Firesworn every 15 s from 2:30, Berserk at 4:00. Display 12110, Firesworn 12164. Not built: Banish, Immolate.

### 5. Shazzrah

The Flamewaker arcanist. NPC 12264 (verify).

- **In WoW:** Arcane Explosion, Shazzrah's Curse (+100% magic damage taken), Counterspell, Magic Grounding (a buff to purge), and blinks to a random player, wiping his threat.
- **In survivors:**
  - **Gate of Shazzrah:** every 20 s a purple sigil marks a random hero who isn't the tank. 1.5 s later he blinks there with his threat wiped, and casts Arcane Explosion straight away. The marked hero runs toward the tank, everyone else steps off the sigil, and the tank's automatic taunt picks him up when he arrives.
  - **Arcane Explosion:** every 6 s, a 12 yd ring around him.
  - **Shazzrah's Curse:** every 25 s on all heroes within 30 yd: magic damage taken +100%. Decurse.
  - **Counterspell:** a 15 yd ring with a 1.5 s wind-up. Heroes inside are silenced for 4 s, so their auto-casts stop. The ranged stay at range.
  - **Magic Grounding:** a shimmering shield that absorbs the next big hit. The Shaman's Purge or the Priest's Dispel Magic removes it automatically.
- **Tests:** the tank (re-picking him up) and ranged discipline.
- **Soft enrage:** after 2:00 he blinks every 12 s. **Hard:** 3:00.
- **Loot:** T1 hands or feet; one shared-pool roll.
- **Spells (verify against Spell.dbc):** Arcane Explosion 19712, Shazzrah's Curse 19713, Magic Grounding 19714, Counterspell 19715, Gate of Shazzrah 23138.
- **Status: built** (stage "Molten Core: Shazzrah's Cavern", the north-west cavern). Arcane Explosion (12 yd ring every 6 s), Shazzrah's Curse (magic damage taken x2 for 20 s, a curse), Counterspell (a purple 15 yd ring; heroes inside cast nothing for 2 s), Gate of Shazzrah (a purple sigil under a hero off the tank; 1.5 s later he stands there with his taunt dropped and an Arcane Explosion ring winds up; every 20 s, every 12 s from 2:00), Berserk at 3:00. Gate uses 23139, the blink with a visual (23138 has none; this client's Spell.dbc has no spell named Magic Grounding). Display 13032. Not built: Magic Grounding.

### 6. Baron Geddon

The fire elemental baron. NPC 12056 (verify).

- **In WoW:** Living Bomb (explodes after 8 s, hurting and knocking up everyone near the target), Inferno (a pulsing nova everyone runs from), Ignite Mana (a dispellable mana burn) and Armageddon at the very end ("performs one last service for Ragnaros").
- **In survivors:**
  - **Living Bomb:** every 15 s on a random hero (two at once on Heroic, when it comes). An 8 s countdown with a 12 yd fire ring on the carrier and an arrow toward the emptiest direction. At 0, allies in the ring take 60% of max health and are knocked up; the carrier takes 25%; enemies take a huge hit. **Run away from your friends and into the horde**: it is the best area damage of the night. Healers top up the carrier first.
  - **Inferno:** every 30 s, a 2 s glow, then an 8 s channel of 20 yd pulses that grow stronger. Everyone leaves, the tank included; Geddon roots himself while channelling, then the tank picks him back up. A bomb carrier escaping through the Inferno ring is the classic clash.
  - **Ignite Mana:** every 20 s on 3 heroes: auto-cast cooldowns +50% while it lasts. Magic, so dispel it.
  - **Armageddon:** at 2%, Geddon goes immune and channels for 8 s, then blasts everyone within 35 yd. Run to the walls. It uses the real emote.
- **Tests:** everyone.
- **Soft enrage:** after 2:30 the bomb comes every 10 s. **Hard:** 3:30.
- **Loot:** T1 shoulders. **Bindings of the Windseeker (right)** at a low chance.
- **Spells (verify against Spell.dbc):** Living Bomb 20475, Inferno 19695, Ignite Mana 19659, Armageddon 20478.
- **Status: built** (stage "Molten Core: Baron Geddon's Cavern", the north-west cavern). Living Bomb every 15 s (10 s from 2:30) on a random hero: a red 12 yd ring rides on the carrier and fills over the 8 s fuse, with the seconds counted over its head; at 0 the allies inside take 50%, the carrier 20%, and the horde inside (not bosses) 50% of its health. Party bots carrying it run 18 yd straight out from their leader. Inferno (rooted, a 2 s glow, then 8 pulses of a 20 yd ring a second apart, each 25% harder), Ignite Mana (cooldowns a third slower for 10 s, magic, on the curse count), the Armageddon emote at 2%, Berserk at 3:30. Display 12129. Armageddon is built: at 2% he can't lose health for 8 s while a 35 yd ring winds up, then it blasts for 35%.

### 7. Sulfuron Harbinger

Ragnaros's herald and his four Flamewaker Priests. NPC 12098 (verify).

- **In WoW:** Hand of Ragnaros (an area stun), Inspire (buffs his adds), Flame Spear (hits the target and those near it) and Demoralizing Shout; the Priests cast Dark Mending heals, Shadow Word: Pain and Immolate.
- **In survivors:**
  - **Dark Mending:** a 2.5 s cast bar over a Priest that heals Sulfuron or another Priest for 15%. Any stun, root or fear from the heroes' own spells interrupts it, and so do the automatic Kick, Pummel, Earth Shock and Counterspell within range. Priests die first, marked with skull and cross.
  - **Flame Spear:** an orange 6 yd ring marks a random hero for 2 s, then hits them and everyone in the ring. Spread out.
  - **Hand of Ragnaros:** a 12 yd ring around Sulfuron (1.5 s), then a 2 s stun.
  - **Demoralizing Shout:** heroes within 20 yd deal 20% less damage for 10 s (the existing `Weaken` effect).
  - **Inspire:** below 50% health, the Priests cast 50% faster.
- **Tests:** the damage dealers' kill order, and the interrupters.
- **Soft enrage:** Inspire also triggers at 2:30. **Hard:** 3:30.
- **Loot:** T1 shoulders; one shared-pool roll.
- **Spells (verify against Spell.dbc):** Demoralizing Shout 19778, Inspire 19779, Hand of Ragnaros 19780, Flame Spear 19781, Dark Mending 19775, Shadow Word: Pain 19776, Dark Strike 19777.
- **Status: built** (stage "Molten Core: Sulfuron's Cavern", the centre cavern). Four Flamewaker Priests cast Dark Mending (a blue ring fills over the 2.5 s cast; a stun or fear on the Priest breaks it; done, it heals the most hurt of Sulfuron and the Priests within 40 yd 15%). Flame Spear (an orange 6 yd ring under a hero, 20%), Hand of Ragnaros (a yellow 12 yd ring, then a 2 s stun), Inspire (Sulfuron and the Priests within 40 yd act 50% faster) below half health and at 2:30, Berserk at 3:30. Display 12161, Priests 13029. The Priests also cast Shadow Word: Pain (2% a second for 12 s, magic). Not built: Demoralizing Shout, the role interrupts.

### 8. Golemagg the Incinerator

The core giant and his two Core Ragers. NPC 11988 (verify).

- **In WoW:** Magma Splash (a stacking debuff on his target), Pyroblast at random players, Earthquake at low health, and Core Ragers that can't die while he lives and grow stronger near him (Golemagg's Trust).
- **In survivors:**
  - **Magma Splash (tank swap):** each swing adds a stack to his target (+8% fire damage taken and a small burn); stacks fall off one every 3 s while you're not his target. At 6 stacks the tank's frame flashes SWAP: the off-tank holding the Ragers walks over and its taunt fires, the first tank walks to the Ragers, and the two cross the room and trade targets.
  - **Core Ragers:** they Mangle (a bleed) their target and heal to full at 50%, so they can't die until Golemagg does; then they fall with him. Within 20 yd of him they deal double damage (Golemagg's Trust), so keep them apart; a line shows the distance.
  - **Pyroblast (stack to split):** every 12 s, a big orange circle (8 yd) on a random hero who isn't the tank. After 3 s it lands for 80% of max health, split evenly among the heroes inside. Two or three share it, and a lone hero dies.
  - **Earthquake:** below 10% health, every 6 s, heavy damage to everyone within 15 yd. Melee steps out and the ranged finish him.
- **Tests:** the tanks, and the stack.
- **Soft enrage:** Earthquake. **Hard:** 4:00.
- **Loot:** T1 chest and legs; a **Sulfuron Ingot** (for Sulfuras); one shared-pool roll.
- **Spells (verify against Spell.dbc):** Magma Splash 13879, Pyroblast 20228, Earthquake 19798, Golemagg's Trust 20553, Mangle 19820.
- **Status: built** (stage "Molten Core: Golemagg's Ridge", the south-centre cavern). Magma Splash stacks on his tank (13880, the debuff with a visual; 1.2% a second a stack, SWAP at 6, falling off solo), two Core Ragers (display 11997) that Mangle in a cone and can't die while he stands (a killing blow leaves a sliver, below half they heal to full with "refuses to die while its master is in trouble"; they fall with him), Earthquake (a 15 yd ring every 6 s, 20%) below 10%, Berserk at 4:00. Pyroblast is built as a stack-to-split: an orange 8 yd circle under a hero off the tank, 3 s, 80% shared among the heroes inside (solo, the Companion soaks a share). Golemagg's Trust too: a Core Rager within 20 yd of him deals double damage. Display 11986.

### 9. Majordomo Executus

Ragnaros's steward, with 4 Flamewaker Elites and 4 Flamewaker Healers. NPC 12018 (verify).

- **In WoW:** alternating shields on his adds (Magic Reflection, Damage Shield), and Teleport, which sends his target into the hot coals. The fight ends when the adds are dead and he submits.
- **In survivors:**
  - **Majordomo himself can't die.** He takes damage, but Aegis of Ragnaros keeps him above 20%. You win by killing all eight adds.
  - **Adds:** the Elites cast Blast Wave (a 10 yd knockback) and Fire Blast. The Healers cast Shadow Shock and an interruptible Great Heal.
  - **Shields, every 30 s for 10 s, alternating**, named by a big icon over each add and a raid warning: **Magic Reflection** ("Casters out!", a blue dome) reflects spell damage at 50%; **Damage Shield** ("Melee out!", orange spikes) burns melee attackers. Heroes can't stop casting, but auto-casts only fire at targets in range, so **stepping out of range is the "stop attacking" button**.
  - **Teleport:** every 20 s, Majordomo sends his current target to the hot coals at the room's edge (5% of max health a second while on them). That hero walks back, and a second taunter or a fast kite holds the room until then.
- **Tests:** everyone's discipline; the tanks.
- **Soft enrage:** after 2:30 the shields last 15 s. **Hard:** 4:00, when the shields stay up for good.
- **Loot:** the Cache of the Firelord chest, with no T1: Core Hound Tooth, Fireproof Cloak, Sash of Whispered Secrets, Wild Growth Spaulders, Gloves of the Hypnotic Flame, Cauterizing Band; Elementium Ore at a low chance, for Thunderfury.
- **Spells (verify against Spell.dbc):** Magic Reflection 20619, Damage Shield 21075, Teleport 20618, Aegis of Ragnaros 20620, Blast Wave 20229.
- **Status: built** (stage "Molten Core: Majordomo's Camp", now a raid stage). Majordomo can't be brought below 20%; four Flamewaker Elites (Blast Wave, a yellow 10 yd ring that throws) and four Flamewaker Healers (Shadow Shock circles, an interruptible Dark Mending) come with him. Every 30 s for 10 s his adds take a ward in turn, Magic Reflection then Damage Shield, with a raid warning; a ward answers a hero's matching blow with half of it, at most 4% of that hero's health a blow. Every 20 s Teleport throws his target 25 yd onto hot coals (a ring burning 5% a second for 10 s; it is a knockback, so no wire change). Once the eight adds are gone he says "Impossible! Stay your attack, mortals... I submit! I submit!" (`A_ExecutusDefeat01`) and the stage is won; winning it opens Ragnaros's lair. Aggro line "Reckless mortals!..." (`A_ExecutusAggro01`). Displays: Majordomo 12029, Elites 12002, Healers 12391. Not built: Fire Blast, Great Heal, the summoning cutscene, the Cache of the Firelord.

**The summoning.** This is the night's set piece.

1. **The surrender.** The last add dies, Majordomo cries "I submit!" and vanishes in fire. The Cache of the Firelord appears and the loot break runs.
2. **The ready check is the summon trigger.** Everyone hitting Ready walks the party into Ragnaros's lair.
3. **The cutscene** (about 25 s, WoW camera): Majordomo calls up his master ("Behold Ragnaros, the Firelord!"); Ragnaros rises from the lava ("TOO SOON! YOU HAVE AWAKENED ME TOO SOON, EXECUTUS!"), burns Majordomo to ash for failing him, and turns on the party ("NOW FOR YOU, INSECTS."). The host can skip it after the first kill of the night, or everyone can vote to skip.

### 10. Ragnaros

The Firelord, waist-deep in his lava pool. NPC 11502 (verify).

- **In WoW:** he never moves. Wrath of Ragnaros knocks melee back, Elemental Fire burns the tank, Magma Blast punishes the raid when nobody is in melee range, Lava Burst erupts, Melt Weapon; after 3 minutes he submerges for 90 s and the Sons of Flame rise.
- **In survivors:**
  - **The arena:** a platform ring around his pool, with Lava Spawn trickling in as the horde. His melee ring (14 yd) is always drawn on the ground.
  - **Melee check (Magma Blast):** if no hero stands in the melee ring for 3 s, he hurls Magma Blast at a random hero for 90% of max health. Someone always has to hold the ring.
  - **Wrath of Ragnaros:** every 25 s he raises Sulfuras and the melee ring glows yellow for 2 s; everyone inside is thrown 25 yd and stunned for 1 s, to "BY FIRE BE PURGED!" The 3 s melee-check timer starts, and a ranged hero must step in while the tank walks back. That hand-off is the fight's heartbeat.
  - **Elemental Fire:** a damage-over-time on the tank; healer throughput. **Lava Burst:** 6 yd lava bubbles under random spots (1.5 s) that knock up anyone on them.
  - **Submerge** at 3:00 and 6:00, to "COME FORTH, MY SERVANTS!": he sinks, and 8 Sons of Flame (at 5 players) climb out at the pool's edge. They fixate on random heroes with a mana-burn aura (auto-casts within 8 yd slow by 30%) and are this phase's horde, so area damage pays. He comes back when they are dead or after 90 s, opening with a Wrath of Ragnaros; Sons still alive stay.
  - **"DIE, INSECT!"** when he kills a hero.
- **Tests:** everyone.
- **Soft enrage:** after his second emergence the lava rises, and a ring telegraph shrinks the safe platform by 2 yd every 15 s. **Hard:** 10:00 of total fight time.
- **Loot:** the Eye of Sulfuras, Band of Accuria, Choker of the Fire Lord, Onslaught Girdle, Bonereaver's Edge, Perdition's Blade, the class's existing Epic (T2) legs from `gear/catalog.rs`, the Essence of the Firelord for a hero on the Thunderfury quest, and Elementium Ore at a low chance (section 4).
- **Spells (verify against Spell.dbc):** Elemental Fire 20564, Magma Blast 20565, Wrath of Ragnaros 20566, Lava Burst 21158, Submerge 21107, Melt Weapon 21388. Sons of Flame NPC 12143 (verify).
- **Status: built** (stage "Molten Core: Ragnaros' Lair"). He stands rooted at the room's centre with his 14 yd melee ring always drawn. "TOO SOON! YOU HAVE AWAKENED ME TOO SOON, EXECUTUS!" (`A_RagnarosArrival01`) as he rises. Wrath of Ragnaros every 25 s (a yellow ring on the melee ring, 2 s, thrown 25 yd, "BY FIRE BE PURGED!" with `A_RagnarosSpecialAttack01`), Elemental Fire on the tank (3% a second for 8 s), Lava Burst (6 yd circles on the curse count that throw 8 yd; 21158 has no visual of its own), Magma Blast (with nobody in the melee ring for 3 s, a 4 yd circle under a hero for 50%). At 3:00, then every 4:30, he submerges for up to 90 s ("COME FORTH, MY SERVANTS!", `A_RagnarosReinforcements01`): he can't be hurt or act, Sons of Flame rise by party size (3-8), and he comes back once they are gone (after 3 s at least) or the time is up, opening with a Wrath. Berserk at 10:00. A huge boss frame. His death plays `RagnarosDeath` and the banner "Ragnaros is banished to the Elemental Plane! The Molten Core is cleared!". Loot: Sulfuron Ingot (17203, 50%), Eye of Sulfuras (17204, 8%), Bonereaver's Edge (17076, 15%) and Band of Accuria (17063, 15%) are rolled as he falls and listed on the results screen. The gear catalog's class pieces and Need/Greed have no place for named items yet, so nothing is kept; the item ids are not checked (the client has no item names). Display 11121, Sons 14508. Not built: Melt Weapon, "DIE, INSECT!", the rising lava soft enrage.

**Voices.** Each built boss roars its creature's own aggro kit as it walks out and gives its death cry as it falls, both from `CreatureSoundData` through its display (checked in the raid tests).

**Yells.** In 1.12 only Majordomo and Ragnaros are voiced (verify). The other bosses use their emote text and the raid-warning sound. Every line's exact text and sound must come from the client's `SoundEntries.dbc` and the cores' `script_texts`, not from this doc. Which line plays at which moment is our staging:

| Moment | Line |
|---|---|
| Majordomo pulled | "Reckless mortals!" |
| Majordomo's adds dead | "I submit!" |
| Ragnaros rises | "TOO SOON!" |
| Fight starts | "NOW FOR YOU, INSECTS." |
| Wrath of Ragnaros | "BY FIRE BE PURGED!" |
| Submerge | "COME FORTH, MY SERVANTS!" |
| A hero dies | "DIE, INSECT!" |
| Geddon's Armageddon | the emote: "...performs one last service for Ragnaros." |

---

## 3. Boss mechanics library

The existing toolkit (`mechanics/mod.rs`) belongs to heroes: a `MechanicSpec` is a hero's trigger, effect and aim, set off by its own play. Bosses need the opposite: the boss's script picks a time and a hero, warns everyone, then lands. So we add a boss-side layer. It reuses the toolkit where it can:

- the `Effect` blows: `Nova`, `BurningGround`, `Dot`, `ApplyCc`;
- `School`, `CcKind` (Fear already exists) and `data::Effect` (`Weaken`, `Slow`);
- the `validate()` pattern.

It adds what bosses need: triggers on the clock and on health, hero picks, telegraphs, debuffs and phases. The rough shape:

```rust
pub(crate) struct BossAbility {
    pub name: &'static str,
    pub spell: Option<u32>,   // real 1.12 spell: its name, icon and visual (verify against Spell.dbc)
    pub when: BossTrigger,    // Every { secs, jitter } | AtSecs | AtHealth(f32) | OnPhase | OnAddDeath
    pub pick: Pick,           // Tank | RandomHero | RandomNonTank | Farthest | AllWithin(r) | Boss | Point
    pub tell: Tell,           // a ground shape and its wind-up, or a debuff icon with a countdown
    pub does: BossEffect,     // a mechanics::Effect where one fits, or one of the blocks below
    pub yell: Option<Yell>,
    pub min_party: u8,        // switched off below this party size (section 7)
}
```

**Readability rules for every block:**

- **Warning first.** Every blow over 20% of max health has a telegraph or a counted-down debuff icon.
- **Wind-ups:** at least 1.5 s for circles, 2.0 s for cones and lines, and 2.5 s for donuts and stack circles, because the horde is in the way.
- **Movement alone is enough.** Every mechanic can be answered by moving. A role ability makes it easier, never possible.
- **Colours:** red for damage, purple for a debuff or curse, yellow for knockback or fear, blue for a safe spot or one to share, orange for split damage. The fill grows from the centre over the wind-up.

| Block | Parameters | Used by |
|---|---|---|
| **Ground circle** | radius, wind-up, damage, school, pick, optional lingering pool (dps, secs) | Lava Bomb, Rain of Fire, Lava Burst |
| **Cone** | angle, length, wind-up, damage, facing (the boss's target or a hero) | Protector Cleave, trash breath |
| **Line** | width, length, wind-up, damage, from/to (a charge path) | Lava Surger charge, Magma Blast path |
| **Ring / donut** | inner and outer radius, wind-up, damage, inward or outward | Wrath of Ragnaros (melee ring), Shadow Shock, rising lava |
| **Pulsing area** | radius, pulses, interval, growth per pulse, boss roots while channelling | Inferno, Earthquake, Arcane Explosion |
| **Debuff bomb** | pick, count, fuse secs, radius, damage to allies, damage to carrier, hits enemies (bool), "run" arrow | Living Bomb, Flame Spear (no fuse, short ring) |
| **Stack to split** | pick, radius, wind-up, total damage, split among heroes inside, minimum soakers | Golemagg's Pyroblast, Impending Doom huddle (variant: stack on a healer) |
| **Dispellable debuff** | kind (Curse, Magic, Poison, Disease), duration, stacks, payload: healing received x, damage taken x by school, auto-cast rate x, delayed blow (secs, damage), mind control; fonts can cleanse (bool) | Lucifron's Curse, Impending Doom, Gehennas' Curse, Shazzrah's Curse, Ignite Mana |
| **Dispellable boss buff** | buff (Frenzy, Grounding), duration, removed by (Tranquilizing Shot, Purge, offensive Dispel Magic) | Magmadar's Frenzy, Magic Grounding |
| **Fear** | radius, wind-up, secs, direction (away from boss or random), run speed, ignored by (Tremor Totem area, Fear Ward, Berserker Rage, Will of the Forsaken) | Panic |
| **Knockback** | radius, distance, from (boss or point), stun secs, upward (bool) | Wrath of Ragnaros, Stomp, Blast Wave, Living Bomb |
| **Melee check** | ring radius, grace secs, punish ability | Ragnaros's Magma Blast |
| **Mind control** | secs, walk to (nearest ally), damage to allies x, ended by (magic dispel, Polymorph, a stun) | Dominate Mind |
| **Silence** | radius, wind-up, secs (auto-casts stop) | Counterspell |
| **Threat wipe and blink** | to (a random non-tank), sigil wind-up, then an ability | Gate of Shazzrah |
| **Teleport a hero** | to (point), hazard (dps) | Majordomo's Teleport |
| **Interruptible cast** | cast secs, interrupted by (any crowd control, role interrupts), on finish | Dark Mending, Great Heal |
| **Enemy shield** | kind (reflect spells, damage melee attackers), secs, every, alternating, applies to (adds) | Magic Reflection, Damage Shield |
| **Add waves** | foes (NPC, display, count by party size), spawn (ring, points, around boss, pool edge), every, max alive, on death (eruption), tether (radius, enrage), linked (dies with boss, can't die before boss), fixate | Firesworn, Core Ragers, Priests, Sons of Flame |
| **Phase transition** | trigger (health below, time, adds dead), on enter (abilities on and off, yell, emote), invulnerable secs | Golemagg below 10%, Geddon's Armageddon, Majordomo's submission |
| **Submerge / intermission** | duration, untargetable, ends when (adds dead or time), adds, re-emerge ability | Ragnaros |
| **Hard enrage** | at secs, damage x, speed x, yell, sound | every boss |
| **Soft enrage** | starts at, kind: damage ramp per 10 s, faster abilities, more targets, shrinking arena (ring step, interval), floor that never expires | per boss (section 2) |
| **Tank-swap stacks** | debuff, added by (the boss's swing on its target), per-stack effect, decay when not targeted (secs per stack), warn at, cap | Magma Splash, Sunder Armor |
| **Yell / emote** | text, sound kit (`SoundEntries.dbc`), channel (yell, emote, raid warning), on (ability, phase, a hero dying, pull, kill), cooldown | Majordomo, Ragnaros, all emotes |
| **Cleansing font** | position, radius, stand secs, removes (curse, magic), per-hero cooldown | every boss room (fallback dispel) |

Built so far (`survivors/raid/mod.rs`): ground circles, cones, lines and donuts with pools; blows that also silence, stun, slow or strip buffs, and that can hurt the horde; curses and magic for the dispel pulses (cooldown rate, healing received, magic damage taken, delayed doom, mind control); the debuff bomb with its riding ring; tank-swap stacks; fear and knockback; add waves (party-scaled with a count of 0), an add's eruption on death and its master's enrage, adds that can't die while their master stands; the blink to a sigil; the rooted pulsing Inferno; the interruptible heal and Inspire; phases on health and time; Frenzy; the hard enrage; yells with a sound kit or the creature's own voice. In co-op the host runs every script; telegraphs replicate as entities, forced moves and slows as each hero's `NetRaid`, and yells as `NetYell` (co-op PROTOCOL 15).

**Engine seams this uses or adds:**

- **Taunt and threat.** `enemies.rs` already has a `taunted: Option<(Entity, f32)>` seam and `party::pick_target` (threat over distance). Taunts, tank threat multipliers and threat wipes go through those.
- **Debuffs on heroes** are a new `HeroState::debuffs` list, beside the shield and thorns timers. Party frames show the icons. The `wowparty` branch's shared-debuff work is the place to start.
- **Forced movement belongs to the client.** Fear, knockback, mind control and teleport all move a hero, but each client owns its hero's movement (MULTIPLAYER.md). So the host sends a forced-movement event (kind, direction, secs), and the owning client plays it out locally.
- **Telegraphs** are replicated entities (shape, position, rotation, size, start, land time), so every screen draws the same warning at the same time.
- **The host runs every boss script and rolls all the dice.**
- **Bots must play the mechanics.** `WOW_SURVIVORS_PARTY` bots only follow the leader today. They need a basic dodge: leave telegraphs, carry the bomb away, stack on orange, step out of range on reflection. Without it, solo testing of a 5-player fight means nothing.

---

## 4. Loot

### Tier 1 sets

All nine T1 sets have 8 pieces. Our gear system already has exactly those 8 armour slots, plus a main hand that T1 doesn't use.

- **Item ids:** item ids are from memory (verify against `item_template`). The sets' ItemSet.dbc rows sit around 201-209 (look them up by name and verify).
- **Stats and new id block:** T1 pieces get our Epic stat budget and set bonuses, which nothing else in the catalog has yet. They go in a new id block (for example 2000+), as stable keys that are never computed.

| Class | Set | Item ids (verify) | Head | Shoulder | Chest | Wrist | Hands | Waist | Legs | Feet |
|---|---|---|---|---|---|---|---|---|---|---|
| Warrior | Battlegear of Might | 16861-16868 | Helm of Might | Pauldrons of Might | Breastplate of Might | Bracers of Might | Gauntlets of Might | Belt of Might | Legplates of Might | Sabatons of Might |
| Paladin | Lawbringer Armor | 16853-16860 | Lawbringer Helm | Lawbringer Spaulders | Lawbringer Chestguard | Lawbringer Bracers | Lawbringer Gauntlets | Lawbringer Belt | Lawbringer Legplates | Lawbringer Boots |
| Hunter | Giantstalker Armor | 16845-16852 | Giantstalker's Helmet | Giantstalker's Epaulets | Giantstalker's Breastplate | Giantstalker's Bracers | Giantstalker's Gloves | Giantstalker's Belt | Giantstalker's Leggings | Giantstalker's Boots |
| Rogue | Nightslayer Armor | 16820-16827 | Nightslayer Cover | Nightslayer Shoulder Pads | Nightslayer Chestpiece | Nightslayer Bracelets | Nightslayer Gloves | Nightslayer Belt | Nightslayer Pants | Nightslayer Boots |
| Priest | Vestments of Prophecy | 16811-16819 | Circlet of Prophecy | Mantle of Prophecy | Robes of Prophecy | Vambraces of Prophecy | Gloves of Prophecy | Girdle of Prophecy | Pants of Prophecy | Boots of Prophecy |
| Shaman | The Earthfury | 16837-16844 | Earthfury Helmet | Earthfury Epaulets | Earthfury Vestments | Earthfury Bracers | Earthfury Gauntlets | Earthfury Belt | Earthfury Legguards | Earthfury Boots |
| Mage | Arcanist Regalia | 16795-16802 | Arcanist Crown | Arcanist Mantle | Arcanist Robes | Arcanist Bindings | Arcanist Gloves | Arcanist Belt | Arcanist Leggings | Arcanist Boots |
| Warlock | Felheart Raiment | 16803-16810 | Felheart Horns | Felheart Shoulder Pads | Felheart Robes | Felheart Bracers | Felheart Gloves | Felheart Belt | Felheart Pants | Felheart Slippers |
| Druid | Cenarion Raiment | 16828-16836 | Cenarion Helm | Cenarion Spaulders | Cenarion Vestments | Cenarion Bracers | Cenarion Gloves | Cenarion Belt | Cenarion Leggings | Cenarion Boots |

**Which bosses drop which slots** (the 1.12 map, from memory; verify):

| Slot | Drops from |
|---|---|
| Head | Garr |
| Shoulder | Baron Geddon, Sulfuron Harbinger |
| Chest | Golemagg |
| Legs | Magmadar, Golemagg |
| Hands, Feet | Lucifron, Gehennas, Shazzrah |
| Wrist, Waist | trash: each trash segment has a 30% chance of one |

**Smart loot.** A T1 drop rolls its class only from the classes in the party, weighted toward players who still lack that slot. A night with 5 players drops about 18 T1 pieces (2 per tier boss, plus trash), so a group fills its sets over roughly three nights.

### Set bonuses (3 / 5 / 8)

The bonuses trigger at 3, 5 and 8 pieces, as T1's did in 1.12. Data tests check the thresholds against ItemSet.dbc. The original texts are paraphrased from memory as the flavour source; the real ones can be read from ItemSet.dbc.

Each bonus is built from mechanics blocks the toolkit already has (`Burst`, `Nova`, `HealPulse`, `Shield`, `CheatDeath`, `ResetCooldowns`, `Chain`, `Split`, `Dot`). The bonuses apply everywhere, in raids and in normal runs. Gear is meta-progression, so raid loot has to matter in every run.

| Set | 1.12 flavour | 3 pieces | 5 pieces | 8 pieces |
|---|---|---|---|---|
| Might | armour, threat, Sunder Armor | +15% armour, +25% threat | Your hits stack Sunder Armor on bosses: +2% damage taken from the whole party per stack, up to 5 | Below 30% health: Shield Wall, -75% damage taken for 6 s, and a taunt on everything within 15 yd (90 s cooldown) |
| Lawbringer | holy damage, party heals on hit | +holy damage, aura radius +25% | Every 10 s your next hit Judges its target: the party deals +5% holy damage to it | Your hits have a 5% chance to heal every hero within 15 yd |
| Giantstalker | pet, Multi-Shot | Companion +30% damage and health | Your shots split into 1 more target | Every 12 s, Volley rains on the densest pack; Tranquilizing Shot cooldown halved |
| Nightslayer | Vanish, healed by Vanish | +crit chance | Below 25% health: Vanish, dropping all threat, untouchable for 2 s (60 s cooldown) | Vanish also heals 30%, and your next hit after it crits |
| Prophecy | Flash Heal, Prayer of Healing | +healing done | Heals' cooldowns -15% | Every third heal also heals everyone within 15 yd of its target |
| Earthfury | totems, Lightning Shield, Reincarnation | Totem radius +25% | Lightning Shield: nature thorns on whatever hits you | Reincarnation: once per boss fight, get back up at 50% (once per run elsewhere) |
| Arcanist | spell damage, lower threat | +spell damage | -30% threat; Blink cooldown -25% | Every 45 s, Evocation: every ability is ready at once, and you get a shield |
| Felheart | Drain Life, Soulstone | Leech: heal for 5% of damage dealt | Once per boss fight (once per run elsewhere), a lethal blow is turned away (Soulstone) | Your damage-over-time spells jump to 3 nearby enemies when their target dies |
| Cenarion | Thorns, Tranquility, Hurricane | Thorns on every hero within 20 yd | Rebirth: +1 battle-resurrection per boss fight | Party below 50% health: Tranquility, a strong area heal over time (60 s cooldown); Hurricane cooldown halved |

### Notable non-set drops

Non-set raid items name which classes can use them, a field the gear system doesn't have yet. Rings, trinkets and necklaces already exist in `jewelry/catalog.rs` as in-run picks (Band of Accuria, Talisman of Ephemeral Power, Choker of the Fire Lord, Quick Strike Ring, Ring of Spell Power). A raid-looted copy is **Bound**:

- in raids it is worn from the start;
- in normal runs it is guaranteed in your first jewelry offer.

| Item (id, verify) | From | Our version |
|---|---|---|
| Bindings of the Windseeker, left (18563) | Garr, 4% | Thunderfury quest item; a legendary-quality banner when it drops |
| Bindings of the Windseeker, right (18564) | Baron Geddon, 4% | Thunderfury quest item |
| Eye of Sulfuras (17204) | Ragnaros, 8% | Sulfuras quest item |
| Sulfuron Ingot (17203) | Golemagg | Sulfuras quest item (4 needed) |
| Essence of the Firelord (19017) | Ragnaros, while on the quest | Thunderfury quest item |
| Elementium Ore (18562) | Ragnaros and Majordomo's cache, 5% | Thunderfury quest item; stands in for Blackwing Lair's drop and moves there when that raid exists |
| Band of Accuria (17063) | Ragnaros | Bound jewelry (already Legendary in the catalog) |
| Choker of the Fire Lord (18814) | Ragnaros | Bound jewelry |
| Onslaught Girdle (19137) | Ragnaros | Waist for melee: damage, crit and armour |
| Bonereaver's Edge (17076) | Ragnaros | Main hand: hits stack an armour break on bosses (as the real item does) |
| Perdition's Blade (18816) | Ragnaros | Main hand: hits have a chance of a small fire blast |
| Core Hound Tooth (18805) | Majordomo's cache | Main hand: crit chance and crit damage |
| Talisman of Ephemeral Power (18820) | shared pool | Bound trinket: every 90 s, +40% spell damage for 15 s (its real on-use) |
| Striker's Mark (17069) | Magmadar | Hunter main hand: shot speed |
| Earthshaker (17073) | Magmadar | Main hand: hits have a chance to knock down |
| Drillborer Disk (17066) | Garr | Tank main hand: armour and thorns |
| Shard of the Flame, Essence of the Pure Flame | Ragnaros | Bound trinkets: regeneration and fire thorns |

The shared pool covers the epics that drop from several bosses, such as Choker of Enlightenment, Medallion of Steadfast Might, Fire Runed Grimoire, Wristguards of Stability, Helm of the Lifegiver, Sorcerous Dagger and Azuresong Mageblade. Each kill rolls one item from it. Every item in this table is a candidate; the implementer verifies each id and source before adding it.

### Legendary quest lines (long-term goals)

They are tracked in the Armory's Quests tab with progress per item, and can span many nights.

- **Thunderfury, Blessed Blade of the Windseeker** (19019): Warrior, Paladin, Hunter, Rogue, Mage, Warlock.
  1. Loot both Bindings. Highlord Demitrian, in Silithus, examines them.
  2. Bring the Essence of the Firelord from Ragnaros, plus 10 Elementium Ore. In 1.12 the ore comes from Blackwing Lair. Until a BWL raid exists, it drops at a low chance (5%) from Ragnaros and Majordomo's cache, and it moves to BWL once that raid exists.
  3. Turn everything in to Demitrian, who forges the blade. Later, a "Rise, Thunderfury!" run at Gates of Ahn'Qiraj (Silithus is already a zone), with Prince Thunderaan as its final boss, becomes the quest's finale.
  
  **Reward:** a main hand whose hits have a 20% chance of a Chain of lightning over 4 jumps that slows everything it hits. Plus the achievement "Did someone say [Thunderfury]?"
- **Sulfuras, Hand of Ragnaros** (17182): Warrior, Paladin, Shaman, Druid.
  1. Loot the Eye of Sulfuras.
  2. Bring 4 Sulfuron Ingots from Golemagg, to forge the Sulfuron Hammer (in 1.12 the Thorium Brotherhood does this).
  
  **Reward:** Sulfuras. `gear/catalog.rs` already has it as the Paladin's Legendary main hand; the quest extends it to the other mace classes. Hits have a chance to fire Hand of Ragnaros (a fire nova that stuns for 2 s), and a burning aura follows the wielder.

---

## 5. Need / Greed / Pass

- **The real roll window:** the 1.12 group-loot frame, with item icon, name in its quality colour, a timer bar, and Need (dice), Greed (coin) and Pass buttons. The timer is 20 s, not 60, and running out passes. Several items stack as several frames, as in 1.12.
- **Chat lines in the real 1.12 wording**, taken from the client's `GlobalStrings.lua` (for example `LOOT_ROLL_NEED`, `LOOT_ROLL_WON`, `LOOT_ROLL_ALL_PASSED`, `RANDOM_ROLL_RESULT`; verify the names). For example: "Need Roll - 87 for [Lawbringer Helm] by Kaz", then "Kaz won: [Lawbringer Helm]".
- **"Won by" banner.** A big centre-screen announcement in raid-warning style follows. Epic items get the loot sound, and Legendary or quest items (Eye, Bindings) get a fanfare plus a ten-second pause in the action for the reactions.
- **Need is by class only:** T1 to the piece's class, and other items to the classes that can use them. Roles don't come into it. Need is greyed out when it doesn't apply, so nobody can ninja.
- **Greed: anyone.** A Greed win unlocks the item's look in the Wardrobe, or can be sharded into Faire tickets (a Nexus Crystal animation).
- **All pass:** "Everyone passed on: [item]". It is sharded for the party.
- **Ties** re-roll between the tied players.
- **/roll:** typed `/roll` or `/roll 1-50` prints "Kaz rolls 47 (1-100)" to the party. A Roll button in the loot window does the same, for off-rolls the group settles by hand.
- **The host rolls the dice.** Clients send their choice as events, the same way card picks travel.

**DKP-lite: no.** DKP is bookkeeping built for 40-person guilds over months. For five friends it adds admin and takes away the dice drama they love. Instead we use one visible, automatic rule:

- **Pity.** Each Need roll you lose adds +5 to your future Need rolls (shown as "1-100 +10" next to the dice), up to +25, and it resets when you win one. It is fair over a night and costs nothing to run.

---

## 6. Social and gambling hooks

**Side bets** (Faire tickets only; never real money):

- **Placing bets.** The bet board opens at each loot break and closes at the pull. The pool is shared (pari-mutuel): all stakes go in and the winners split the pot, with no house cut. The host settles each bet from the raid log, and payouts show in chat.
- **Limits:** 1-10 tickets a bet, at most 50 tickets a night per player, no lending, no going below zero.
- **Markets:** who dies to Living Bomb first; wipes before Ragnaros (over/under 2.5); will a Binding drop tonight; who tops the meter on Garr; who wins the next T1 Need roll; first to "DIE, INSECT!".
- **Sayge.** Sayge's Dark Fortune from the Faire can be bought with tickets as a pre-raid buff.

**Wipe recap ("Floor Inspector"):** a 10 s card after every wipe.

- **Awards:** **Floor Inspector** (first to die, with the killing blow: "killed by Living Bomb (Kaz's)"), **Bomb Hugger** (most damage from a friend's Living Bomb), **Fire Tester** (seconds standing in pools), and a Recount-style ranking of avoidable damage taken.
- **Repair bill.** A cosmetic running total for the night, after 1.12's 10% durability loss per death ("Tonight's repair bill: 37g 12s").
- **"Fifty DKP minus!"** goes to whoever is Floor Inspector three pulls running.

**Loot-council banter:**

- **Loot Ledger:** who won what tonight, plus each player's dry streak ("6 bosses, no loot").
- **Upgrade preview:** each candidate's percentage gain, so everyone can argue about it. A "Gz" button sends /congratulate, and "Shard it" handles passed items.
- **A fake loot council:** before rolls, everyone votes with a thumbs emote on who "deserves" it. The vote counts for nothing but is shown with the results.

**Transmog:**

- **Every T1 piece** you win (Need or Greed) unlocks its look in the Wardrobe; Greed wins on other classes' pieces build the collection. A complete set unlocks a "Set collected" entry and its title.
- **Phase 3 of the Faire (transmog)** shows the looks off, and a post-raid Fashion vote (best-dressed wins tickets) reuses the planned post-run vote.

**Raid titles and achievements** (Wardrobe titles, Armory achievements):

| Achievement | How | Reward |
|---|---|---|
| Firelord's Bane | Kill Ragnaros | Title "Firelord's Bane" |
| Attuned | Finish Attunement to the Core | Lothos's teleport |
| Core Cleared | Kill all ten bosses in one night | Title "of the Molten Core" |
| Full Clear, No Wipes | All ten bosses with zero wipes | Title "the Unscorched" |
| Didn't Stand in Fire | Kill Ragnaros with nobody hit by Lava Burst or a Lava Bomb pool | Tickets |
| No Bomb Hugs | Kill Geddon with no ally hurt by a Living Bomb | Tickets |
| Hunters Have One Job | Remove 10 Frenzies in one Magmadar kill | Tickets |
| Rune Dousing | Douse all seven runes | Hydraxian tabard |
| Did someone say [Thunderfury]? | Loot a Binding of the Windseeker | Tickets and a server-wide (party) shout |
| Blessed Blade / Hand of Ragnaros | Finish a legendary quest | Title "the Windseeker" / "Hand of Ragnaros" |
| Tier 1 Complete | Collect all 8 pieces of your class's set | Title by set ("of Might", "the Lawbringer"...) |
| Heroic Firelord (with Heroic, later) | Kill Ragnaros on Heroic | Title "the Molten" |

---

## 7. Scaling for 1-5 players, and difficulty

Tuned for 5. Trash uses the existing party scale. Bosses get a steeper health curve, because the whole party focuses them.

| Players | Trash density and health | Boss health | Firesworn / Sons | Curse targets | T1 per tier boss | Other rolls per boss |
|---|---|---|---|---|---|---|
| 1 | x1.0 | x1.0 | 3 / 3 | 1 | 1 | 1 |
| 2 | x1.6 | x1.7 | 4 / 4 | 1 | 1 | 1 |
| 3 | x2.2 | x2.4 | 5 / 5 | 2 | 2 | 1 |
| 4 | x2.8 | x3.1 | 6 / 6 | 2 | 2 | 1 |
| 5 | x3.4 | x3.8 | 8 / 8 | 3 | 2 | 1 (Ragnaros 3) |

Boss damage per hit doesn't scale with players: every hero takes the same blows, so mechanics feel the same at any size.

**Mechanics that change with group size.** They switch on and off with each ability's `min_party`:

- **Solo:**
  - Living Bomb and Pyroblast hurt only you, and your Companion counts as a soaker for Pyroblast.
  - Dominate Mind is replaced by a second Shadow Shock.
  - Magma Splash stacks fall off on their own at 6.
  - Your Companion satisfies Ragnaros's melee check while you're knocked back.
  - Fonts are always up.
- **2 players:** no mind control. Stacks fall off at 6 unless both players can taunt.

**Composition helpers.** These rules let any group of friends raid. Heroic switches them off.

| Missing | Helper |
|---|---|
| A tank (no taunt) | Boss melee -40%; the boss's target goes to the highest-threat hero; +threat role cards are more common |
| A healer | Boss damage -25%; healing potions drop from trash |
| A curse dispeller (Mage, Druid) | Two extra fonts; curses last half as long |
| A magic dispeller (Priest, Paladin) | Impending Doom deals half damage; fonts cleanse it |
| A Hunter | Frenzy lasts 4 s instead of 8 |
| Fear immunity | Panic lasts 4 s instead of 6 |

**Difficulty:**

- **Normal:** as described. The first version ships Normal only.
- **Heroic (later, not in the first version):** unlocks after the first Ragnaros kill.
  - **Harder:** +1 to every mechanic count (two Living Bombs, three submerges), enrages 20% sooner, no composition helpers, no fonts.
  - **Rewards:** +1 T1 roll per tier boss, and the "Heroic Firelord" title.
- **The Run Options rewards multiplier** applies on top, as for any run.

---

## 8. Build plan

Each milestone can be played on its own, and none starts until the one before passes the usual gate: rustfmt, clippy, the survivors tests with and without `coop`, and a play build.

**M0. Engine spike: can map 409 load and render?** (time-box: 1 day)

- **Why it's a real question.** Every current battleground is on map 0 or 1 (even Naxxramas is the outdoor necropolis area), so no instance map has been tried.
- **What to check:** map 409 in `Map.dbc`; its WDT (instances are usually one global WMO with no terrain tiles); WMO group rendering and portal culling across all of MC; lava liquid inside WMO groups; lighting and fog (`Light.dbc`); collision through the existing WMO collider; frame time with a 5-player-sized horde.
- **How:** a test aid, `WOW_SURVIVORS_MAP=409 <x y z>`, drops a hero in MC. **Output:** screenshots of the entrance, Magmadar's lair, Garr's room, the Lava Pool and Ragnaros's lair; each room's measured centre and leash; a list of what's missing.
- **Decision gate.** If it works, raid arenas are `MapDef` rows with `map: 409` (the shape already takes any map id). If not, use the fallback and keep building: every boss is data, and an arena is just a `MapDef` row. The fallback is approved for launch.
- **Fallback: MC-themed arenas on map 0**, so one continent stays streamed: Blackrock Mountain's chained lava hall (Lucifron, Gehennas, Sulfuron; also the attunement zone); Searing Gorge's Cauldron (Magmadar, Garr, Golemagg, Majordomo); the Burning Steppes lava fields below Blackrock Stronghold (Shazzrah, Geddon); and the Cauldron's floor for Ragnaros, with Fire Plume Ridge's lava lake (map 1, a shipped arena whose lava already renders) as plan B.

**M1. Raid shell: a boss rush with no mechanics.** Raid mode in the lobby, the attunement check, role declaration at the ready check, and the segment state machine (trash, boss, loot break, next). Par-level XP and catch-up; `RaidSave` with checkpoints, wipes, attempts and Resume; real names and models on plain chase-and-hit bosses. **Playable when:** you can play trash and ten bosses in order, wipe, and resume after killing the process.

**M2. Mechanics library and test room.** The section 3 blocks, the hero debuff list, role abilities (dispel pulse, interrupt, taunt, Tranquilizing Shot, battle-resurrections), forced-movement events, and bot dodging. A "Raid Target Dummy" room fires each block on demand. Data tests check that every spell id in a boss script exists in `Spell.dbc` under that name. **Playable when:** each block can be dodged, dispelled or answered there, solo and with bots.

**M3. The first two bosses: Lucifron and Magmadar, with segments T1 and T2.** Curses, Impending Doom, Dominate Mind, Panic, Lava Bomb, Frenzy, Core Hound packs and fonts. A raid-sim test proves the mechanics matter: 5 bots that follow them must kill each boss before the enrage at par level, and 5 bots that stand still must wipe. **Playable when:** friends can play "MC: the first wing" online.

**M4. The rest, in order:** Gehennas, Garr, Shazzrah and Baron Geddon; then Sulfuron, Golemagg, Majordomo with the summoning cutscene, and Ragnaros; plus the rune dousing and the remaining trash. Each boss lands with its raid-sim test. **Playable when:** a full clear works online with 5.

**M5. Loot: T1 sets and Need/Greed.** The 72 T1 pieces (ids, display ids and icons verified against the cores, as the catalog already does), set bonuses as mechanics, smart loot and the slot map; non-set drops with class lists and Bound jewelry; the roll window with real strings and sounds, /roll, pity and sharding; stub steps for the Thunderfury and Sulfuras quests. **Playable when:** loot drops, gets rolled for, and is worn the next night.

**M6. Social, polish and feel.** Side bets, the wipe recap, achievements and titles, transmog unlocks, the attunement chain and the Blackrock Mountain zone. Yells with sound, boss frames with attempt counts, debuff icons on party frames, raid-warning text. A tuning pass from real nights' raid logs. Then the README, the ROADMAP and the toast.house page.

---

## 9. Decisions

1. **Levels and builds persist across the whole raid night**, with par levels and catch-up to stop runaway power (section 1).
2. **Lockout: one raid night, resumable.** "Resume raid" picks up from the last checkpoint after a crash or disconnect. There is no weekly lockout, because the friends' schedules are irregular; one could come later.
3. **Set thresholds are 1.12's real 3/5/8** (section 4).
4. **Smart loot for T1:** a T1 piece only rolls classes in the party, weighted toward players missing that slot.
5. **Need is by class only.** No role restriction, to keep it simple.
6. **Fallback arenas on map 0** (Blackrock Mountain, Searing Gorge, Burning Steppes) are fine for launch if map 409 fails the spike.
7. **Thunderfury: substitute now.** Elementium Ore drops at a low chance from Ragnaros and Majordomo's cache until a Blackwing Lair raid exists, then it moves there. The Prince Thunderaan fight comes later, as the quest's finale.
8. **Side-bet limits as written:** 1-10 tickets a bet, at most 50 a night per player.
9. **Set bonuses apply everywhere**, in raids and normal runs. Gear is meta-progression, so loot matters.
10. **Heroic comes later,** not in the first version.
