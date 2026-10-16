# Talents: the double-dip audit and the 31-point specs

Code: `crates/benilla-app/src/survivors/talents/` (`mechanics.rs` for the audit's resolutions,
`spec.rs` for the specs, `guardian.rs` for the summons in the world).

## Party talent pass

- **Lightwell** and **Healing Stream Totem** now pulse every living hero inside the guardian's
  stated range; the host credits the owner and emits the spell visual on every recipient.
- **Trueshot Aura**, **Divine Spirit**, and **Leader of the Pack** are true 40-yard party auras.
  Their short refresh window means the benefit follows range, including for remote heroes.
- **Power Infusion** is a modest, visible ally burst: every 45 seconds it picks the living
  in-range party member with the most damage done (the priest in solo) for 8% damage for 10 sec.
- **Deep Wounds** is a stacking Physical bleed, **Seal of Command** is a holy-strike proc, and
  **Concussion Blow** is a stunning blow rather than a stat row.

## The rule: one name, one thing

A talent that is also a spell (or passive) the run offers never grants a second, separate effect
under the same name. It *teaches* that spell instead (`Teach` in `talents/mechanics.rs`):

- **Spell.** The spell leaves the class pool for heroes without the talent. A hero who learned it
  is offered it as any other spell, and takes it at **rank 2**. The talent grants no mechanic of
  its own. Tooltip: "Teaches Pyroblast: your runs offer it, and you take it at rank 2."
- **Passive.** A passive every class may take stays offered to all; the talent's learners take it
  at **rank 2**.
- **Spec.** A capstone whose name is its spec's summon (Lightwell) has the summon as its effect.
- A spec form named as its own capstone (Shadowform, Moonkin Form) adds no bonuses of its own:
  the talent's aura is the effect, and the spec only shows it (state kit, shapeshift model).
- A legendary transformation never shares a talent's or another offered spell's name.

The tests that hold this: `no_talent_and_offered_spell_share_a_name_unresolved` (pure: every
talent mechanic against every class spell, passive and legendary, failing on a shared name
without the `Teach` flag) and `no_talent_shares_a_client_name_with_an_offered_spell` (data-gated:
all 27 trees' talents by `Spell.dbc` name against the offered spells' `Spell.dbc` names).

## Overlaps and their resolutions

| Talent (id) | Overlap in the run | Resolution |
|---|---|---|
| Pyroblast (29) | Mage spell `pyroblast`; Fireball's legendary was "Pyroblast" | Teaches `pyroblast` (rank 2, gated); Fireball's legendary renamed **Hot Streak** |
| Blast Wave (32) | Mage spell `blast_wave` | Teaches `blast_wave` |
| Combustion (36) | Fire Blast's legendary was "Combustion" | Talent keeps its burst; legendary renamed **Inferno Blast** |
| Sweeping Strikes (133) | Warrior spell `sweeping_strikes` | Teaches `sweeping_strikes` |
| Mortal Strike (135) | Warrior spell `mortal_strike` | Teaches `mortal_strike`; also the Arms spec's start |
| Bloodthirst (167) | New Warrior spell `bloodthirst` | Teaches `bloodthirst`; also the Fury spec's start |
| Shield Slam (148) | New Warrior spell `shield_slam` | Teaches `shield_slam`; also the Protection spec's start |
| Last Stand (153) | Retaliation's legendary was "Last Stand" | Talent keeps its burst; legendary renamed **Die by the Sword** |
| Consecration (1435) | Paladin spell `consecration` | Teaches `consecration` |
| Holy Shock (1502) | Paladin spell `holy_shock` | Teaches `holy_shock`; also the Holy spec's start |
| Blessing of Kings (1442) | Passive "Blessing of Kings" (`kings`), offered to all | The passive stays for all; the talent's learners take it at rank 2 (no separate max-health aura) |
| Aimed Shot (1345) | Hunter spell `aimed_shot` | Teaches `aimed_shot`; also the Marksmanship spec's start |
| Scatter Shot (1353) | Concussive Shot's legendary was "Scatter Shot" | Talent keeps its strike; legendary renamed **Binding Shot** |
| Blade Flurry (223) | Rogue spell `blade_flurry` | Teaches `blade_flurry` |
| Hemorrhage (talent, stat row) | Sinister Strike's legendary was "Hemorrhage" | Legendary renamed **Crimson Edge** |
| Holy Nova (442) | Priest spell `holy_nova` | Teaches `holy_nova`; also the Holy spec's start |
| Lightwell (1637) | The Holy spec's summon | The Lightwell guardian is the talent's effect (no separate nova) |
| Mind Flay (501) | New Priest spell `mind_flay` | Teaches `mind_flay`; also the Shadow spec's start |
| Shadowform (521) | The Shadow spec's form | The talent's aura is the effect; the spec shows Shadowform's state kit |
| Stormstrike (901) | New Shaman spell `stormstrike` | Teaches `stormstrike`; also the Enhancement spec's start |
| Shadowburn (963) | Warlock spell `shadowburn` | Teaches `shadowburn` (no separate execute proc) |
| Conflagrate (968) | Immolate's legendary was "Conflagrate" | Talent keeps its nova; legendary renamed **Chaos Bolt** (Incinerate and Barrage are 1.12 talents too) |
| Moonkin Form (793) | The Balance spec's form | The talent's aura is the effect; the spec wears the Moonkin model |
| Insect Swarm (827) | Druid spell `insect_swarm` | Teaches `insect_swarm`; also the Restoration spec's start |
| Nature's Grasp (talent, stat row) | Entangling Roots' legendary was "Nature's Grasp" | Legendary renamed **Earthen Grasp** |
| (none) | Multi-Shot's legendary "Volley" and the spell `volley` | Legendary renamed **Chimera Shot** |
| Searing Light (403) | Holy Fire's legendary was "Searing Light" | Legendary renamed **Sacred Flame** |

## The full audit: every talent mechanic against the run's pool

Every talent with a mechanic in `talents/mechanics.rs`, its overlap with a spell, passive or
legendary the run offers, and what it does now. "Own mechanic" is the talent's own effect, which
nothing in the run shares a name with.

| Talent (id) | Overlap | Resolution |
|---|---|---|
| Arcane Concentration (75) | none | Own mechanic (echo) |
| Presence of Mind (86) | none | Own mechanic (echo) |
| Arcane Power (87) | none | Own mechanic (burst); Arcane capstone |
| Blast Wave (32) | spell | Teaches the spell |
| Combustion (36) | legendary name | Own mechanic; legendary renamed; Fire capstone |
| Ice Block (72) | none | Own mechanic (cheat death) |
| Ice Barrier (71) | none | Own mechanic (shield); Frost capstone |
| Cold Snap (69) | none | Own mechanic (cooldown reset) |
| Pyroblast (29) | spell, legendary name | Teaches the spell; legendary renamed |
| Impact (30) | none | Own mechanic (stun proc) |
| Ignite (34) | none | Own mechanic (burn proc) |
| Improved Blizzard (63) | none | Own mechanic (chill proc) |
| Sweeping Strikes (133) | spell | Teaches the spell |
| Last Stand (153) | legendary name | Own mechanic; legendary renamed |
| Anger Management (137) | none | Own mechanic (aura) |
| Mortal Strike (135) | spell | Teaches the spell; Arms capstone |
| Piercing Howl (160) | none | Own mechanic (nova) |
| Death Wish (165) | none | Own mechanic (aura) |
| Bloodthirst (167) | spell | Teaches the spell; Fury capstone |
| Concussion Blow (152) | none | Own mechanic (nova) |
| Shield Slam (148) | spell | Teaches the spell; Protection capstone |
| Improved Hamstring (129) | none | Own mechanic (root proc) |
| Consecration (1435) | spell | Teaches the spell |
| Divine Favor (1433) | none | Own mechanic (aura) |
| Holy Shock (1502) | spell | Teaches the spell; Holy capstone |
| Blessing of Kings (1442) | passive | Passive at rank 2 for its learners |
| Blessing of Sanctuary (1431) | none | Own mechanic (aura) |
| Holy Shield (1430) | none | Own mechanic (nova); Protection capstone |
| Seal of Command (1481) | none | Own mechanic (nova) |
| Sanctity Aura (1409) | none | Own mechanic (aura) |
| Repentance (1441) | none | Own mechanic (nova); Retribution capstone |
| Bestial Swiftness (1391) | none | Own mechanic (aura) |
| Intimidation (1387) | none | Own mechanic (nova) |
| Bestial Wrath (1386) | none | Own mechanic (aura); Beast Mastery capstone |
| Aimed Shot (1345) | spell | Teaches the spell |
| Scatter Shot (1353) | legendary name | Own mechanic; legendary renamed |
| Trueshot Aura (1361) | none | Own mechanic (aura); Marksmanship capstone |
| Deterrence (1308) | none | Own mechanic (aura) |
| Counterattack (1312) | none | Own mechanic (nova) |
| Wyvern Sting (1325) | none | Own mechanic (nova); Survival capstone |
| Relentless Strikes (281) | none | Own mechanic (aura) |
| Cold Blood (280) | none | Own mechanic (aura) |
| Vigor (382) | none | Own mechanic (aura); Assassination capstone |
| Riposte (301) | none | Own mechanic (nova) |
| Blade Flurry (223) | spell | Teaches the spell |
| Adrenaline Rush (205) | none | Own mechanic (aura); Combat capstone |
| Preparation (284) | none | Own mechanic (cooldown reset) |
| Ghostly Strike (303) | none | Own mechanic (nova) |
| Premeditation (381) | none | Own mechanic (nova); Subtlety capstone |
| Inner Focus (348) | none | Own mechanic (aura) |
| Divine Spirit (351) | none | Own mechanic (aura) |
| Power Infusion (322) | none | Own mechanic (aura); Discipline capstone |
| Holy Nova (442) | spell | Teaches the spell |
| Spirit of Redemption (1561) | none | Own mechanic (cheat death) |
| Lightwell (1637) | spec summon | The Lightwell guardian; Holy capstone |
| Blackout (464) | none | Own mechanic (stun proc) |
| Mind Flay (501) | spell | Teaches the spell |
| Vampiric Embrace (484) | none | Own mechanic (aura) |
| Shadowform (521) | spec form | Own aura, shown by the spec; Shadow capstone |
| Elemental Focus (574) | none | Own mechanic (aura) |
| Elemental Fury (565) | none | Own mechanic (aura) |
| Elemental Mastery (573) | none | Own mechanic (aura); Elemental capstone |
| Two-Handed Axes and Maces (617) | none | Own mechanic (aura) |
| Parry (616) | none | Own mechanic (aura) |
| Stormstrike (901) | spell | Teaches the spell; Enhancement capstone |
| Totemic Mastery (582) | none | Own mechanic (aura) |
| Nature's Swiftness (591) | none | Own mechanic (aura) |
| Mana Tide Totem (590) | none | Own mechanic (nova); Restoration capstone |
| Amplify Curse (1061) | none | Own mechanic (nova) |
| Siphon Life (1041) | none | Own mechanic (nova) |
| Dark Pact (1022) | none | Own mechanic (nova); Affliction capstone |
| Fel Domination (1226) | none | Own mechanic (aura) |
| Demonic Sacrifice (1281) | none | Own mechanic (aura) |
| Soul Link (1282) | none | Own mechanic (aura); Demonology capstone |
| Shadowburn (963) | spell | Teaches the spell |
| Ruin (967) | none | Own mechanic (aura) |
| Conflagrate (968) | legendary name | Own mechanic; legendary renamed; Destruction capstone |
| Omen of Clarity (788) | none | Own mechanic (aura) |
| Nature's Grace (789) | none | Own mechanic (aura) |
| Moonkin Form (793) | spec form | Own aura, shown and worn by the spec; Balance capstone |
| Feral Charge (804) | none | Own mechanic (nova) |
| Faerie Fire (Feral) (1162) | none | Own mechanic (nova) |
| Leader of the Pack (809) | none | Own mechanic (aura); Feral capstone |
| Insect Swarm (827) | spell | Teaches the spell |
| Nature's Swiftness (831) | none | Own mechanic (aura) |
| Swiftmend (844) | none | Own mechanic (heal); Restoration capstone |

## The 31-point specs

Each tree's capstone (its tier-7 talent, 31 points deep) makes the hero that spec for every run:
the class's run-start ability becomes the spec's signature spell (keeping its rank), and the spec
brings a signature summon or form for the whole run. On the Talents tab the capstone wears a deep
gold **SPEC** frame and banner, and its tooltip reads e.g. "Becomes: Frost Mage. Starts with
Frostbolt. Summons a Water Elemental."

The Mage now starts on Fireball (the class start); Frost's Frostbolt is the spec's.

| Class | Tree | Capstone (id) | Starts with | Summon or form |
|---|---|---|---|---|
| Warrior | Arms | Mortal Strike (135) | Mortal Strike | Battle Stance: +10% damage, +20% critical damage |
| Warrior | Fury | Bloodthirst (167) | Bloodthirst | Berserker Stance: +10% crit, 10% shorter cooldowns |
| Warrior | Protection | Shield Slam (148) | Shield Slam | Defensive Stance: 15% damage reduction, +30 health |
| Paladin | Holy | Holy Shock (1502) | Holy Shock | Concentration Aura: regen, and a Holy Light pulse heals the party every 12 sec |
| Paladin | Protection | Holy Shield (1430) | Judgement | Devotion Aura: 15% damage reduction, +25 health |
| Paladin | Retribution | Repentance (1441) | Crusader Strike | Retribution Aura: +10% damage, a holy pulse every 2 sec |
| Hunter | Beast Mastery | Bestial Wrath (1386) | Multi-Shot | Frostsaber (a big white saber) that hunts beside you |
| Hunter | Marksmanship | Trueshot Aura (1361) | Aimed Shot | Aspect of the Hawk: +10% damage, +5% crit |
| Hunter | Survival | Wyvern Sting (1325) | Explosive Trap | Frost Trap laid on the thickest pack every 9 sec |
| Rogue | Assassination | Vigor (382) | Backstab | Deadly Poison: 30% of hits poison |
| Rogue | Combat | Adrenaline Rush (205) | Eviscerate | Slice and Dice: 15% shorter cooldowns |
| Rogue | Subtlety | Premeditation (381) | Ambush | Shadow Double: a shadow in your own likeness strikes beside you |
| Priest | Discipline | Power Infusion (322) | Holy Fire | Inner Fire: 10% damage reduction, +10% damage |
| Priest | Holy | Lightwell (1637) | Holy Nova | Lightwell, planted beside you, heals you |
| Priest | Shadow | Shadowform (521) | Mind Flay | Shadowform (the talent's aura, shown) |
| Shaman | Elemental | Elemental Mastery (573) | Chain Lightning | Air Elemental casting Lightning Bolt |
| Shaman | Enhancement | Stormstrike (901) | Stormstrike | Two Spirit Wolves (the ghost wolf) |
| Shaman | Restoration | Mana Tide Totem (590) | Earth Shock | Healing Stream Totem, planted beside you |
| Mage | Arcane | Arcane Power (87) | Arcane Missiles | Mana Fiend casting Arcane Missiles |
| Mage | Fire | Combustion (36) | Pyroblast | Fire Elemental casting Fireball |
| Mage | Frost | Ice Barrier (71) | Frostbolt | Water Elemental casting Frostbolt |
| Warlock | Affliction | Dark Pact (1022) | Corruption | Felhunter |
| Warlock | Demonology | Soul Link (1282) | Death Coil | Doomguard |
| Warlock | Destruction | Conflagrate (968) | Immolate | Infernal |
| Druid | Balance | Moonkin Form (793) | Starfire | Moonkin Form (Night Elf or Tauren moonkin model) |
| Druid | Feral | Leader of the Pack (809) | Shred | Cat Form (cat model): +10% speed, +5% crit |
| Druid | Restoration | Swiftmend (844) | Insect Swarm | Three Treants |

Choices where 1.12 had no summon: warriors take their stances, paladins their auras, rogues their
poisons, Slice and Dice or a shadow double; Survival lays traps; the specs whose 1.12 signature is
a heal or shield (Discipline, Restoration) start on a damage spell of their tree so a run can
begin, and keep the healing in their summon. Demonology starts on Death Coil rather than the
class's Shadow Bolt so the swap is felt.

### Summons in the world

Guardians are host-simulated units (`talents/guardian.rs`), spawned like the companions with a run
guid and despawned with the run; nothing about them is local-only. They keep a half ring behind
their owner and every few seconds act: melee guardians run down the nearest enemy near their
owner, casters fly their spell at the nearest in range, healers (the Healing Stream Totem, the
Lightwell, a `GameObject`) stand planted, heal their owner in reach and are set down afresh when
the owner moves past. Every blow and heal is the owner's. Display ids are real 1.12 rows, checked
by `guardians_wear_their_creatures_models`; spell ids, the capstones' tiers and the start spells'
names by `specs_name_real_capstones_spells_and_models`.

New class spells for the specs (each with its own legendary): Bloodthirst (23881), Shield Slam
(23922), Crusader Strike (2537), Backstab (53), Ambush (8676), Mind Flay (15407), Stormstrike
(17364), Shred (5221).

## Character customisation

The menu is the 1.12 create screen (race and gender buttons, class, the Randomize dice, the
glue-screen art); the Armory's Character tab has the Appearance panel. What it covers now:

- **Race and gender**: the menu's buttons. The pick is the class hero's **body**, saved in the
  profile with its looks (`HeroLooks::body`); picking the class again brings it back.
- **Skin colour, face, hair style, hair colour, facial hair / horns / tusks / piercings /
  markings**: one dial each, captioned by the race's `GlueStrings` tokens, with the client's own
  counts from `CharSections.dbc`, `CharHairGeosets.dbc` and `CharacterFacialHairStyles.dbc`
  (pinned for all 16 bodies by `option_counts_per_race_and_gender_are_the_clients`).
- **Name**: 2 to 12 letters, letters only, capitalised (`normalize_name`), with a race-flavoured
  Random.
- **Randomize**: the Appearance panel's button and the menu's dice both roll the features.
- **Preview**: the real model, re-dressed the frame a feature changes, on a slow turntable that a
  left-drag off the menu's frames turns by hand.
- Saved per class in the profile (`classes.<class>.gear.looks`).
