//! What every vanilla talent grants in the mode: our own design echoing what it did in WoW, in
//! the shared stat vocabulary. Each entry is one rank's bonuses; rank `n` grants `n` times them.
//!
//! The scale per rank, by the talent's rank count (5 / 3 / 2): school damage +3 / 4 / 5%,
//! unscoped damage +2 / 3 / 4%, crit chance +1 / 1.5 / 2 points, cooldowns -2 / 3 / 4% (half that
//! for a mana or rage cost), area +3 / 4 / 5%, health +3 / 4 / 6, regen +0.2 / 0.3 / 0.4 per
//! second, damage reduction +1 / 1.5 / 2% (Toughness +2%). One-point talents in the deep tiers and
//! the 31-point capstones grant something notable: a target, an extra life, +10% of a school.
//!
//! Mana has no place in the mode: cost talents read as cooldowns, mana pools as health and mana
//! regeneration as health regeneration. A pet's talent is the hero's, a threat talent damage
//! reduction, a range talent pickup range, and a proc's chance luck.

use super::super::bonus::{Bonus, School, StatKey};

/// `(talent id, the rank-1 spell's name, one rank's bonuses)`; the name guards against a mistyped
/// id, which the data tests check against `Spell.dbc`.
pub(super) type Effect = (u32, &'static str, &'static [Bonus]);

/// One rank's bonuses for `talent`, if the table has it.
pub(super) fn per_rank(talent: u32) -> Option<&'static [Bonus]> {
    EFFECTS
        .iter()
        .find(|(id, _, _)| *id == talent)
        .map(|&(_, _, per_rank)| per_rank)
}

const fn hp(v: f32) -> Bonus {
    Bonus::new(StatKey::MaxHealth, v)
}
const fn dmg(v: f32) -> Bonus {
    Bonus::new(StatKey::Damage, v)
}
const fn cdr(v: f32) -> Bonus {
    Bonus::new(StatKey::Cooldown, v)
}
const fn crit(v: f32) -> Bonus {
    Bonus::new(StatKey::CritChance, v)
}
const fn critd(v: f32) -> Bonus {
    Bonus::new(StatKey::CritDamage, v)
}
const fn area(v: f32) -> Bonus {
    Bonus::new(StatKey::Area, v)
}
const fn targets(v: f32) -> Bonus {
    Bonus::new(StatKey::Targets, v)
}
const fn speed(v: f32) -> Bonus {
    Bonus::new(StatKey::MoveSpeed, v)
}
const fn regen(v: f32) -> Bonus {
    Bonus::new(StatKey::Regen, v)
}
const fn pickup(v: f32) -> Bonus {
    Bonus::new(StatKey::Pickup, v)
}
const fn armor(v: f32) -> Bonus {
    Bonus::new(StatKey::Armor, v)
}
const fn xp(v: f32) -> Bonus {
    Bonus::new(StatKey::Experience, v)
}
const fn gold(v: f32) -> Bonus {
    Bonus::new(StatKey::Gold, v)
}
const fn luck(v: f32) -> Bonus {
    Bonus::new(StatKey::Luck, v)
}
const fn leech(v: f32) -> Bonus {
    Bonus::new(StatKey::Lifesteal, v)
}
const fn revive(v: f32) -> Bonus {
    Bonus::new(StatKey::Revives, v)
}
const fn phys(v: f32) -> Bonus {
    Bonus::school(School::Physical, v)
}
const fn holy(v: f32) -> Bonus {
    Bonus::school(School::Holy, v)
}
const fn fire(v: f32) -> Bonus {
    Bonus::school(School::Fire, v)
}
const fn nature(v: f32) -> Bonus {
    Bonus::school(School::Nature, v)
}
const fn frost(v: f32) -> Bonus {
    Bonus::school(School::Frost, v)
}
const fn shadow(v: f32) -> Bonus {
    Bonus::school(School::Shadow, v)
}
const fn arcane(v: f32) -> Bonus {
    Bonus::school(School::Arcane, v)
}

/// Every talent in the 27 trees, by class and tree in tab order, each tree in grid order.
pub(super) static EFFECTS: &[Effect] = &[
    // Warrior: Arms.
    (124, "Improved Heroic Strike", &[cdr(0.015)]),
    (130, "Deflection", &[armor(0.01)]),
    (127, "Improved Rend", &[phys(0.04)]),
    (126, "Improved Charge", &[speed(0.04)]),
    (641, "Tactical Mastery", &[cdr(0.01)]),
    (128, "Improved Thunder Clap", &[area(0.04)]),
    (131, "Improved Overpower", &[crit(0.02)]),
    (137, "Anger Management", &[cdr(0.04)]),
    (121, "Deep Wounds", &[critd(0.07)]),
    (136, "Two-Handed Weapon Specialization", &[phys(0.03)]),
    (662, "Impale", &[critd(0.10)]),
    (132, "Axe Specialization", &[crit(0.01)]),
    (133, "Sweeping Strikes", &[targets(1.0)]),
    (125, "Mace Specialization", &[luck(0.02)]),
    (123, "Sword Specialization", &[phys(0.02)]),
    (134, "Polearm Specialization", &[crit(0.01)]),
    (129, "Improved Hamstring", &[speed(0.02)]),
    (135, "Mortal Strike", &[phys(0.10), critd(0.10)]),
    // Warrior: Fury.
    (158, "Booming Voice", &[area(0.03)]),
    (157, "Cruelty", &[crit(0.01)]),
    (161, "Improved Demoralizing Shout", &[armor(0.01)]),
    (159, "Unbridled Wrath", &[cdr(0.01)]),
    (166, "Improved Cleave", &[phys(0.03), area(0.02)]),
    (160, "Piercing Howl", &[area(0.06)]),
    (661, "Blood Craze", &[regen(0.3)]),
    (154, "Improved Battle Shout", &[dmg(0.02)]),
    (1581, "Dual Wield Specialization", &[phys(0.03)]),
    (1542, "Improved Execute", &[critd(0.08)]),
    (155, "Enrage", &[dmg(0.02)]),
    (168, "Improved Slam", &[cdr(0.02)]),
    (165, "Death Wish", &[phys(0.10), armor(-0.03)]),
    (1543, "Improved Intercept", &[speed(0.03)]),
    (1541, "Improved Berserker Rage", &[dmg(0.02)]),
    (156, "Flurry", &[cdr(0.02)]),
    (167, "Bloodthirst", &[phys(0.08), leech(0.02)]),
    // Warrior: Protection.
    (1601, "Shield Specialization", &[armor(0.01)]),
    (138, "Anticipation", &[hp(3.0)]),
    (142, "Improved Bloodrage", &[regen(0.3)]),
    (140, "Toughness", &[armor(0.02)]),
    (141, "Iron Will", &[hp(3.0)]),
    (153, "Last Stand", &[hp(20.0)]),
    (145, "Improved Shield Block", &[armor(0.01)]),
    (147, "Improved Revenge", &[luck(0.03)]),
    (144, "Defiance", &[dmg(0.02)]),
    (146, "Improved Sunder Armor", &[phys(0.03)]),
    (151, "Improved Disarm", &[armor(0.01)]),
    (143, "Improved Taunt", &[cdr(0.02)]),
    (150, "Improved Shield Wall", &[armor(0.015)]),
    (152, "Concussion Blow", &[phys(0.05), luck(0.05)]),
    (149, "Improved Shield Bash", &[luck(0.04)]),
    (702, "One-Handed Weapon Specialization", &[phys(0.03)]),
    (148, "Shield Slam", &[phys(0.06), armor(0.04), hp(10.0)]),
    // Paladin: Holy.
    (1450, "Divine Strength", &[dmg(0.02)]),
    (1449, "Divine Intellect", &[hp(3.0)]),
    (1432, "Spiritual Focus", &[cdr(0.01)]),
    (1463, "Improved Seal of Righteousness", &[holy(0.03)]),
    (1444, "Healing Light", &[regen(0.3)]),
    (1435, "Consecration", &[area(0.08), holy(0.03)]),
    (1443, "Improved Lay on Hands", &[hp(6.0)]),
    (1628, "Unyielding Faith", &[hp(4.0)]),
    (1461, "Illumination", &[cdr(0.01)]),
    (1446, "Improved Blessing of Wisdom", &[xp(0.04)]),
    (1433, "Divine Favor", &[crit(0.04)]),
    (1465, "Lasting Judgement", &[holy(0.03)]),
    (1627, "Holy Power", &[crit(0.01)]),
    (1502, "Holy Shock", &[holy(0.10), regen(0.5)]),
    // Paladin: Protection.
    (1422, "Improved Devotion Aura", &[armor(0.01)]),
    (1421, "Redoubt", &[armor(0.01)]),
    (1630, "Precision", &[dmg(0.02)]),
    (1425, "Guardian's Favor", &[cdr(0.02), speed(0.02)]),
    (1423, "Toughness", &[armor(0.02)]),
    (1442, "Blessing of Kings", &[hp(10.0), dmg(0.03)]),
    (1501, "Improved Righteous Fury", &[dmg(0.02)]),
    (1424, "Shield Specialization", &[armor(0.01)]),
    (1629, "Anticipation", &[hp(3.0)]),
    (1521, "Improved Hammer of Justice", &[cdr(0.03)]),
    (1626, "Improved Concentration Aura", &[cdr(0.015)]),
    (1431, "Blessing of Sanctuary", &[armor(0.04), holy(0.03)]),
    (1426, "Reckoning", &[dmg(0.02)]),
    (1429, "One-Handed Weapon Specialization", &[dmg(0.02)]),
    (1430, "Holy Shield", &[holy(0.08), armor(0.05)]),
    // Paladin: Retribution.
    (1401, "Improved Blessing of Might", &[dmg(0.02)]),
    (1407, "Benediction", &[cdr(0.01)]),
    (1631, "Improved Judgement", &[cdr(0.03)]),
    (1464, "Improved Seal of the Crusader", &[holy(0.04)]),
    (1403, "Deflection", &[armor(0.01)]),
    (1633, "Vindication", &[armor(0.01)]),
    (1411, "Conviction", &[crit(0.01)]),
    (1481, "Seal of Command", &[holy(0.07)]),
    (1634, "Pursuit of Justice", &[speed(0.04)]),
    (1632, "Eye for an Eye", &[armor(0.015)]),
    (1405, "Improved Retribution Aura", &[dmg(0.03)]),
    (1410, "Two-Handed Weapon Specialization", &[dmg(0.03)]),
    (1409, "Sanctity Aura", &[holy(0.10)]),
    (1402, "Vengeance", &[holy(0.03)]),
    (1441, "Repentance", &[dmg(0.08), critd(0.10)]),
    // Hunter: Beast Mastery.
    (1382, "Improved Aspect of the Hawk", &[cdr(0.02)]),
    (1389, "Endurance Training", &[hp(4.0)]),
    (1624, "Improved Eyes of the Beast", &[pickup(0.08)]),
    (1381, "Improved Aspect of the Monkey", &[armor(0.01)]),
    (1395, "Thick Hide", &[armor(0.015)]),
    (1625, "Improved Revive Pet", &[regen(0.4)]),
    (1384, "Pathfinding", &[speed(0.04)]),
    (1391, "Bestial Swiftness", &[speed(0.05)]),
    (1396, "Unleashed Fury", &[dmg(0.02)]),
    (1385, "Improved Mend Pet", &[regen(0.3)]),
    (1393, "Ferocity", &[crit(0.01)]),
    (1388, "Spirit Bond", &[regen(0.5)]),
    (1387, "Intimidation", &[dmg(0.03), luck(0.05)]),
    (1390, "Bestial Discipline", &[cdr(0.02)]),
    (1397, "Frenzy", &[cdr(0.02)]),
    (1386, "Bestial Wrath", &[dmg(0.10)]),
    // Hunter: Marksmanship.
    (1341, "Improved Concussive Shot", &[luck(0.02)]),
    (1342, "Efficiency", &[cdr(0.01)]),
    (1343, "Improved Hunter's Mark", &[dmg(0.02)]),
    (1344, "Lethal Shots", &[crit(0.01)]),
    (1345, "Aimed Shot", &[phys(0.07)]),
    (1346, "Improved Arcane Shot", &[cdr(0.02)]),
    (1352, "Hawk Eye", &[pickup(0.06)]),
    (1348, "Improved Serpent Sting", &[nature(0.03)]),
    (1349, "Mortal Shots", &[critd(0.05)]),
    (1353, "Scatter Shot", &[armor(0.03), speed(0.03)]),
    (1347, "Barrage", &[area(0.03), dmg(0.02)]),
    (1351, "Improved Scorpid Sting", &[armor(0.01)]),
    (1362, "Ranged Weapon Specialization", &[phys(0.03)]),
    (1361, "Trueshot Aura", &[dmg(0.10)]),
    // Hunter: Survival.
    (1623, "Monster Slaying", &[dmg(0.02), critd(0.02)]),
    (1301, "Humanoid Slaying", &[dmg(0.02), gold(0.03)]),
    (1311, "Deflection", &[armor(0.01)]),
    (1304, "Entrapment", &[luck(0.02)]),
    (1621, "Savage Strikes", &[crit(0.02)]),
    (1305, "Improved Wing Clip", &[speed(0.02)]),
    (1306, "Clever Traps", &[fire(0.03), area(0.03)]),
    (1622, "Survivalist", &[hp(4.0)]),
    (1308, "Deterrence", &[armor(0.05)]),
    (1322, "Trap Mastery", &[luck(0.04)]),
    (1310, "Surefooted", &[speed(0.02), dmg(0.01)]),
    (1309, "Improved Feign Death", &[hp(5.0)]),
    (1321, "Killer Instinct", &[crit(0.015)]),
    (1312, "Counterattack", &[phys(0.04), armor(0.03)]),
    (1303, "Lightning Reflexes", &[dmg(0.02)]),
    (1325, "Wyvern Sting", &[nature(0.10)]),
    // Rogue: Assassination.
    (276, "Improved Eviscerate", &[phys(0.04)]),
    (272, "Remorseless Attacks", &[crit(0.02)]),
    (270, "Malice", &[crit(0.01)]),
    (273, "Ruthlessness", &[luck(0.03)]),
    (274, "Murder", &[dmg(0.03)]),
    (277, "Improved Slice and Dice", &[cdr(0.03)]),
    (281, "Relentless Strikes", &[cdr(0.05)]),
    (278, "Improved Expose Armor", &[phys(0.04)]),
    (269, "Lethality", &[critd(0.06)]),
    (682, "Vile Poisons", &[nature(0.03)]),
    (268, "Improved Poisons", &[luck(0.02)]),
    (280, "Cold Blood", &[crit(0.05)]),
    (279, "Improved Kidney Shot", &[dmg(0.03)]),
    (283, "Seal Fate", &[cdr(0.02)]),
    (382, "Vigor", &[dmg(0.06), cdr(0.04)]),
    // Rogue: Combat.
    (203, "Improved Gouge", &[armor(0.01)]),
    (201, "Improved Sinister Strike", &[cdr(0.02)]),
    (186, "Lightning Reflexes", &[armor(0.01)]),
    (202, "Improved Backstab", &[crit(0.01)]),
    (187, "Deflection", &[armor(0.01)]),
    (181, "Precision", &[dmg(0.02)]),
    (204, "Endurance", &[speed(0.03)]),
    (301, "Riposte", &[phys(0.05), armor(0.02)]),
    (222, "Improved Sprint", &[speed(0.03)]),
    (206, "Improved Kick", &[luck(0.04)]),
    (182, "Dagger Specialization", &[crit(0.01)]),
    (221, "Dual Wield Specialization", &[phys(0.03)]),
    (184, "Mace Specialization", &[luck(0.02)]),
    (223, "Blade Flurry", &[targets(1.0)]),
    (242, "Sword Specialization", &[phys(0.02)]),
    (183, "Fist Weapon Specialization", &[crit(0.01)]),
    (1703, "Weapon Expertise", &[dmg(0.04)]),
    (1122, "Aggression", &[phys(0.04)]),
    (205, "Adrenaline Rush", &[cdr(0.10)]),
    // Rogue: Subtlety.
    (241, "Master of Deception", &[gold(0.03)]),
    (261, "Opportunity", &[phys(0.03)]),
    (1700, "Sleight of Hand", &[armor(0.02)]),
    (247, "Elusiveness", &[cdr(0.03)]),
    (244, "Camouflage", &[speed(0.02)]),
    (245, "Initiative", &[luck(0.03)]),
    (303, "Ghostly Strike", &[phys(0.04), armor(0.03)]),
    (263, "Improved Ambush", &[crit(0.015)]),
    (246, "Setup", &[armor(0.01)]),
    (262, "Improved Sap", &[gold(0.05)]),
    (1123, "Serrated Blades", &[phys(0.04)]),
    (1701, "Heightened Senses", &[armor(0.02)]),
    (284, "Preparation", &[cdr(0.06)]),
    (265, "Dirty Deeds", &[cdr(0.02)]),
    (681, "Hemorrhage", &[phys(0.07)]),
    (1702, "Deadliness", &[dmg(0.02)]),
    (381, "Premeditation", &[dmg(0.06), crit(0.03)]),
    // Priest: Discipline.
    (342, "Unbreakable Will", &[hp(3.0)]),
    (345, "Wand Specialization", &[dmg(0.02)]),
    (352, "Silent Resolve", &[armor(0.01)]),
    (344, "Improved Power Word: Fortitude", &[hp(6.0)]),
    (343, "Improved Power Word: Shield", &[armor(0.015)]),
    (321, "Martyrdom", &[armor(0.02)]),
    (348, "Inner Focus", &[crit(0.03), cdr(0.02)]),
    (347, "Meditation", &[regen(0.3)]),
    (346, "Improved Inner Fire", &[armor(0.015)]),
    (341, "Mental Agility", &[cdr(0.01)]),
    (350, "Improved Mana Burn", &[cdr(0.02)]),
    (1201, "Mental Strength", &[hp(3.0)]),
    (351, "Divine Spirit", &[regen(1.0)]),
    (1202, "Force of Will", &[dmg(0.015), crit(0.005)]),
    (322, "Power Infusion", &[dmg(0.10)]),
    // Priest: Holy.
    (410, "Healing Focus", &[regen(0.2)]),
    (406, "Improved Renew", &[regen(0.3)]),
    (401, "Holy Specialization", &[crit(0.01)]),
    (411, "Spell Warding", &[armor(0.01)]),
    (1181, "Divine Fury", &[cdr(0.02)]),
    (442, "Holy Nova", &[area(0.08), holy(0.04)]),
    (1636, "Blessed Recovery", &[regen(0.2)]),
    (361, "Inspiration", &[armor(0.015)]),
    (1635, "Holy Reach", &[area(0.05)]),
    (408, "Improved Healing", &[cdr(0.015)]),
    (403, "Searing Light", &[holy(0.05)]),
    (413, "Improved Prayer of Healing", &[regen(0.2)]),
    (1561, "Spirit of Redemption", &[revive(1.0)]),
    (402, "Spiritual Guidance", &[dmg(0.02)]),
    (404, "Spiritual Healing", &[regen(0.2)]),
    (1637, "Lightwell", &[regen(1.5)]),
    // Priest: Shadow.
    (465, "Spirit Tap", &[regen(0.2)]),
    (464, "Blackout", &[luck(0.02)]),
    (466, "Shadow Affinity", &[armor(0.01)]),
    (482, "Improved Shadow Word: Pain", &[shadow(0.05)]),
    (463, "Shadow Focus", &[shadow(0.02)]),
    (542, "Improved Psychic Scream", &[cdr(0.03)]),
    (481, "Improved Mind Blast", &[cdr(0.02)]),
    (501, "Mind Flay", &[shadow(0.06)]),
    (483, "Improved Fade", &[armor(0.015)]),
    (881, "Shadow Reach", &[pickup(0.06)]),
    (461, "Shadow Weaving", &[shadow(0.03)]),
    (541, "Silence", &[cdr(0.04)]),
    (484, "Vampiric Embrace", &[leech(0.03)]),
    (1638, "Improved Vampiric Embrace", &[leech(0.01)]),
    (462, "Darkness", &[shadow(0.03)]),
    (521, "Shadowform", &[shadow(0.10), armor(0.05)]),
    // Shaman: Elemental.
    (564, "Convection", &[cdr(0.01)]),
    (563, "Concussion", &[nature(0.03)]),
    (572, "Earth's Grasp", &[area(0.04)]),
    (1640, "Elemental Warding", &[armor(0.015)]),
    (561, "Call of Flame", &[fire(0.04)]),
    (574, "Elemental Focus", &[cdr(0.04)]),
    (575, "Reverberation", &[cdr(0.02)]),
    (562, "Call of Thunder", &[crit(0.01)]),
    (567, "Improved Fire Totems", &[area(0.04)]),
    (1642, "Eye of the Storm", &[armor(0.01)]),
    (1645, "Elemental Devastation", &[crit(0.01)]),
    (1641, "Storm Reach", &[pickup(0.08)]),
    (565, "Elemental Fury", &[critd(0.20)]),
    (721, "Lightning Mastery", &[cdr(0.02)]),
    (573, "Elemental Mastery", &[dmg(0.08), crit(0.03)]),
    // Shaman: Enhancement.
    (614, "Ancestral Knowledge", &[hp(3.0)]),
    (612, "Shield Specialization", &[armor(0.01)]),
    (609, "Guardian Totems", &[armor(0.02)]),
    (613, "Thundering Strikes", &[crit(0.01)]),
    (605, "Improved Ghost Wolf", &[speed(0.04)]),
    (607, "Improved Lightning Shield", &[nature(0.04)]),
    (610, "Enhancing Totems", &[dmg(0.04)]),
    (617, "Two-Handed Axes and Maces", &[dmg(0.05)]),
    (601, "Anticipation", &[armor(0.01)]),
    (602, "Flurry", &[cdr(0.02)]),
    (615, "Toughness", &[armor(0.02)]),
    (1647, "Improved Weapon Totems", &[dmg(0.02), fire(0.02)]),
    (611, "Elemental Weapons", &[dmg(0.03)]),
    (616, "Parry", &[armor(0.04)]),
    (1643, "Weapon Mastery", &[dmg(0.02)]),
    (901, "Stormstrike", &[nature(0.10)]),
    // Shaman: Restoration.
    (586, "Improved Healing Wave", &[regen(0.2)]),
    (593, "Tidal Focus", &[cdr(0.01)]),
    (589, "Improved Reincarnation", &[hp(6.0)]),
    (581, "Ancestral Healing", &[armor(0.015)]),
    (595, "Totemic Focus", &[cdr(0.01)]),
    (583, "Nature's Guidance", &[dmg(0.02)]),
    (587, "Healing Focus", &[armor(0.01)]),
    (582, "Totemic Mastery", &[area(0.08)]),
    (1646, "Healing Grace", &[armor(0.01)]),
    (588, "Restorative Totems", &[regen(0.2)]),
    (594, "Tidal Mastery", &[crit(0.01)]),
    (1648, "Healing Way", &[regen(0.2)]),
    (591, "Nature's Swiftness", &[cdr(0.05)]),
    (592, "Purification", &[regen(0.2)]),
    (590, "Mana Tide Totem", &[regen(1.0), cdr(0.04)]),
    // Mage: Arcane.
    (74, "Arcane Subtlety", &[dmg(0.03)]),
    (76, "Arcane Focus", &[arcane(0.02)]),
    (80, "Improved Arcane Missiles", &[arcane(0.03)]),
    (78, "Wand Specialization", &[dmg(0.03)]),
    (1650, "Magic Absorption", &[armor(0.01)]),
    (75, "Arcane Concentration", &[cdr(0.01), luck(0.01)]),
    (82, "Magic Attunement", &[area(0.05)]),
    (81, "Improved Arcane Explosion", &[arcane(0.02), crit(0.01)]),
    (85, "Arcane Resilience", &[armor(0.04)]),
    (83, "Improved Mana Shield", &[hp(6.0)]),
    (88, "Improved Counterspell", &[luck(0.04)]),
    (1142, "Arcane Meditation", &[regen(0.3)]),
    (86, "Presence of Mind", &[cdr(0.06)]),
    (77, "Arcane Mind", &[hp(3.0)]),
    (421, "Arcane Instability", &[dmg(0.02), crit(0.01)]),
    (87, "Arcane Power", &[dmg(0.10)]),
    // Mage: Fire.
    (26, "Improved Fireball", &[cdr(0.02)]),
    (30, "Impact", &[fire(0.03)]),
    (34, "Ignite", &[critd(0.05)]),
    (28, "Flame Throwing", &[pickup(0.08)]),
    (27, "Improved Fire Blast", &[cdr(0.03)]),
    (1141, "Incinerate", &[crit(0.02)]),
    (31, "Improved Flamestrike", &[area(0.02), crit(0.01)]),
    (29, "Pyroblast", &[fire(0.07)]),
    (23, "Burning Soul", &[armor(0.02)]),
    (25, "Improved Scorch", &[fire(0.03)]),
    (24, "Improved Fire Ward", &[armor(0.02)]),
    (1639, "Master of Elements", &[cdr(0.015)]),
    (33, "Critical Mass", &[crit(0.02)]),
    (32, "Blast Wave", &[area(0.08), fire(0.03)]),
    (35, "Fire Power", &[fire(0.03)]),
    (36, "Combustion", &[fire(0.10), crit(0.02)]),
    // Mage: Frost.
    (70, "Frost Warding", &[armor(0.02)]),
    (37, "Improved Frostbolt", &[cdr(0.02)]),
    (1649, "Elemental Precision", &[dmg(0.02)]),
    (73, "Ice Shards", &[critd(0.05)]),
    (38, "Frostbite", &[luck(0.03)]),
    (62, "Improved Frost Nova", &[cdr(0.03)]),
    (65, "Permafrost", &[speed(0.02)]),
    (61, "Piercing Ice", &[frost(0.04)]),
    (69, "Cold Snap", &[cdr(0.06)]),
    (63, "Improved Blizzard", &[frost(0.02), area(0.03)]),
    (741, "Arctic Reach", &[area(0.05)]),
    (66, "Frost Channeling", &[cdr(0.015)]),
    (67, "Shatter", &[crit(0.01)]),
    (72, "Ice Block", &[revive(1.0)]),
    (64, "Improved Cone of Cold", &[frost(0.03), area(0.01)]),
    (68, "Winter's Chill", &[frost(0.03)]),
    (71, "Ice Barrier", &[hp(20.0), armor(0.05)]),
    // Warlock: Affliction.
    (1005, "Suppression", &[shadow(0.02)]),
    (1003, "Improved Corruption", &[cdr(0.02)]),
    (1006, "Improved Curse of Weakness", &[armor(0.015)]),
    (1101, "Improved Drain Soul", &[xp(0.04)]),
    (1007, "Improved Life Tap", &[cdr(0.02)]),
    (1004, "Improved Drain Life", &[leech(0.004)]),
    (1284, "Improved Curse of Agony", &[shadow(0.04)]),
    (1001, "Fel Concentration", &[armor(0.01)]),
    (1061, "Amplify Curse", &[shadow(0.06)]),
    (1021, "Grim Reach", &[pickup(0.08)]),
    (1002, "Nightfall", &[cdr(0.03)]),
    (1121, "Improved Drain Mana", &[shadow(0.03)]),
    (1041, "Siphon Life", &[leech(0.025)]),
    (1081, "Curse of Exhaustion", &[speed(0.05)]),
    (1082, "Improved Curse of Exhaustion", &[speed(0.01)]),
    (1042, "Shadow Mastery", &[shadow(0.03)]),
    (1022, "Dark Pact", &[cdr(0.06), regen(0.5)]),
    // Warlock: Demonology.
    (1221, "Improved Healthstone", &[regen(0.4)]),
    (1222, "Improved Imp", &[fire(0.03)]),
    (1223, "Demonic Embrace", &[hp(4.0)]),
    (1224, "Improved Health Funnel", &[regen(0.3)]),
    (1225, "Improved Voidwalker", &[armor(0.015)]),
    (1242, "Fel Intellect", &[cdr(0.01)]),
    (1243, "Improved Succubus", &[dmg(0.02)]),
    (1226, "Fel Domination", &[cdr(0.05)]),
    (1241, "Fel Stamina", &[hp(3.0)]),
    (1227, "Master Summoner", &[cdr(0.02)]),
    (1262, "Unholy Power", &[dmg(0.02)]),
    (1283, "Improved Enslave Demon", &[luck(0.02)]),
    (1281, "Demonic Sacrifice", &[shadow(0.08)]),
    (1261, "Improved Firestone", &[fire(0.05)]),
    (1244, "Master Demonologist", &[dmg(0.01), armor(0.01)]),
    (1282, "Soul Link", &[armor(0.08), dmg(0.03)]),
    (1263, "Improved Spellstone", &[hp(6.0)]),
    // Warlock: Destruction.
    (944, "Improved Shadow Bolt", &[shadow(0.03)]),
    (941, "Cataclysm", &[cdr(0.01)]),
    (943, "Bane", &[cdr(0.02)]),
    (982, "Aftermath", &[luck(0.02)]),
    (983, "Improved Firebolt", &[fire(0.03)]),
    (984, "Improved Lash of Pain", &[cdr(0.02)]),
    (981, "Devastation", &[crit(0.01)]),
    (963, "Shadowburn", &[shadow(0.07)]),
    (985, "Intensity", &[area(0.04)]),
    (964, "Destructive Reach", &[pickup(0.08)]),
    (965, "Improved Searing Pain", &[fire(0.02)]),
    (986, "Pyroclasm", &[area(0.02), luck(0.03)]),
    (961, "Improved Immolate", &[fire(0.03)]),
    (967, "Ruin", &[critd(0.20)]),
    (966, "Emberstorm", &[fire(0.03)]),
    (968, "Conflagrate", &[fire(0.10)]),
    // Druid: Balance.
    (762, "Improved Wrath", &[cdr(0.02)]),
    (761, "Nature's Grasp", &[armor(0.03)]),
    (921, "Improved Nature's Grasp", &[luck(0.025)]),
    (787, "Improved Entangling Roots", &[armor(0.01)]),
    (763, "Improved Moonfire", &[arcane(0.03)]),
    (791, "Natural Weapons", &[dmg(0.02)]),
    (781, "Natural Shapeshifter", &[speed(0.02)]),
    (782, "Improved Thorns", &[nature(0.04)]),
    (788, "Omen of Clarity", &[cdr(0.05)]),
    (764, "Nature's Reach", &[pickup(0.08)]),
    (792, "Vengeance", &[critd(0.05)]),
    (784, "Improved Starfire", &[arcane(0.01), cdr(0.01)]),
    (789, "Nature's Grace", &[cdr(0.05)]),
    (783, "Moonglow", &[cdr(0.015)]),
    (790, "Moonfury", &[nature(0.02), arcane(0.02)]),
    (793, "Moonkin Form", &[crit(0.03), armor(0.04), dmg(0.04)]),
    // Druid: Feral Combat.
    (796, "Ferocity", &[cdr(0.01)]),
    (795, "Feral Aggression", &[dmg(0.02)]),
    (799, "Feral Instinct", &[armor(0.01)]),
    (797, "Brutal Impact", &[luck(0.04)]),
    (794, "Thick Hide", &[armor(0.02)]),
    (807, "Feline Swiftness", &[speed(0.05)]),
    (804, "Feral Charge", &[speed(0.05)]),
    (798, "Sharpened Claws", &[crit(0.015)]),
    (802, "Improved Shred", &[cdr(0.02)]),
    (803, "Predatory Strikes", &[dmg(0.03)]),
    (800, "Blood Frenzy", &[critd(0.08)]),
    (801, "Primal Fury", &[cdr(0.02)]),
    (805, "Savage Fury", &[dmg(0.03)]),
    (1162, "Faerie Fire (Feral)", &[dmg(0.05)]),
    (808, "Heart of the Wild", &[hp(3.0), dmg(0.01)]),
    (809, "Leader of the Pack", &[crit(0.05), speed(0.05)]),
    // Druid: Restoration.
    (821, "Improved Mark of the Wild", &[armor(0.01)]),
    (822, "Furor", &[cdr(0.01)]),
    (824, "Improved Healing Touch", &[regen(0.2)]),
    (823, "Nature's Focus", &[regen(0.1)]),
    (826, "Improved Enrage", &[dmg(0.03)]),
    (829, "Reflection", &[regen(0.2)]),
    (827, "Insect Swarm", &[nature(0.06), armor(0.02)]),
    (841, "Subtlety", &[armor(0.01)]),
    (843, "Tranquil Spirit", &[cdr(0.01)]),
    (830, "Improved Rejuvenation", &[regen(0.3)]),
    (831, "Nature's Swiftness", &[cdr(0.05)]),
    (828, "Gift of Nature", &[regen(0.2)]),
    (842, "Improved Tranquility", &[armor(0.02)]),
    (825, "Improved Regrowth", &[crit(0.01)]),
    (844, "Swiftmend", &[regen(1.0), hp(10.0)]),
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn ids_are_unique_and_every_entry_grants_something() {
        let mut seen = HashSet::new();
        for &(id, name, per_rank) in EFFECTS {
            assert!(seen.insert(id), "talent {id} ({name}) listed twice");
            assert!(!per_rank.is_empty(), "{name} grants nothing");
            assert!(per_rank.iter().all(|b| b.value != 0.0), "{name} has a zero");
        }
        assert_eq!(EFFECTS.len(), 432, "the 1.12 Talent.dbc's 432 rows");
        assert_eq!(per_rank(140), Some(&[armor(0.02)][..]));
        assert_eq!(per_rank(1), None);
    }
}
