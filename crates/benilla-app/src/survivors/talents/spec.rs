//! Spec identity: each tree's 31-point talent (its tier-7 capstone) makes the hero that spec for
//! every run ([`SPECS`]). The spec swaps the class's run-start ability for its signature spell, and
//! brings a signature summon ([`Summon::Guardian`], run by [`super::guardian`]) or form
//! ([`Summon::Form`]) for the whole run.
//!
//! A form shows its spell's state kit on the hero, a druid's shapeshift wears its form's model,
//! and its bonuses and mechanics are granted under [`SPEC_MECHANIC`]. A form named as its own
//! capstone talent (Shadowform, Moonkin Form) carries no bonuses of its own: the talent's aura is
//! the effect, and the spec only shows it.
//!
//! Spell ids, display ids and names are real 1.12 rows (the data tests check them); the numbers
//! are the mode's own, at run level 1.

use bevy::prelude::*;

use super::super::bonus::{Bonus, School, StatKey};
use super::super::data;
use super::super::mechanics::{
    self as game_mechanics, Aim, Cc, CcKind, Effect as Mech, MechanicId, MechanicSpec as M, Of,
    Trigger,
};
use super::super::party::HeroState;
use super::mechanics::PERMANENT;

/// Whom a spec's form mechanics are granted under.
pub(in crate::survivors) const SPEC_MECHANIC: MechanicId = MechanicId::Other("spec");

/// One tree's spec: what its 31-point talent makes the hero.
#[derive(Debug)]
pub(in crate::survivors) struct SpecDef {
    /// The `TalentTab.dbc` tree, which the data tests check the capstone against.
    #[allow(dead_code)]
    pub tree: u32,
    /// The tree's tier-7 capstone, the `Talent.dbc` row that grants the spec.
    pub talent: u32,
    /// "Frost Mage".
    pub title: &'static str,
    /// The run-start ability, by its key in `data::ABILITIES`.
    pub start: &'static str,
    /// Its `Spell.dbc` name, which the data tests check: "Power Word: Shield".
    pub start_name: &'static str,
    pub summon: Summon,
}

/// A spec's signature companion for the run.
#[derive(Debug)]
pub(in crate::survivors) enum Summon {
    Guardian(GuardianDef),
    Form(FormDef),
}

/// What a guardian does every [`GuardianDef::every`] seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Act {
    /// Runs to the nearest enemy near its owner and strikes it.
    Melee,
    /// Casts its spell at the nearest enemy in range, from its owner's side.
    Bolt,
    /// Heals its owner while it stands within reach. Planted, as a totem or a well is: it is set
    /// down afresh beside its owner once the owner has gone past its reach.
    Heal,
}

/// A summoned ally that follows its hero all run: a host-simulated unit, as a companion is.
#[derive(Debug)]
pub(in crate::survivors) struct GuardianDef {
    /// The creature's 1.12 name: "Water Elemental".
    pub name: &'static str,
    /// Its `CreatureDisplayInfo` row; 0 wears its owner's own display (a shadow of the hero).
    pub display: u32,
    pub scale: f32,
    /// How many come.
    pub count: u8,
    pub act: Act,
    /// The spell whose cast (a bolt's missile) or impact plays as it acts.
    pub spell: u32,
    pub school: School,
    /// Seconds between acts.
    pub every: f32,
    /// Damage, or healing for [`Act::Heal`], per act at run level 1.
    pub amount: f32,
    /// How far it reaches: a bolt's range, a melee guardian's hunt from its owner, a healer's
    /// reach, past which it is planted afresh beside its owner.
    pub range: f32,
    /// A `GameObjectDisplayInfo` row rather than a creature's: a planted thing (the Lightwell).
    pub object: bool,
    /// Its signature spell beside its act: the Doomguard's Rain of Fire, the Infernal's
    /// Immolation, the Treants' roots.
    pub special: Option<Special>,
}

/// A guardian's signature spell, beside its [`Act`]. Damage is at run level 1, and grows as its
/// act's does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Special {
    /// Every `every` seconds, `spell`'s burning ground on the thickest pack within its range,
    /// `radius` yards across: `dps` of its school a second for `secs`.
    Rain {
        spell: u32,
        every: f32,
        radius: f32,
        dps: f32,
        secs: f32,
    },
    /// Every `every` seconds, `damage` of its school to every enemy within `radius` of it, under
    /// `spell`'s visual: an aura it wears.
    Aura {
        spell: u32,
        every: f32,
        radius: f32,
        damage: f32,
    },
    /// Its blows root what they strike for `secs`, `chance` of the time, under `spell`'s roots.
    Root { spell: u32, chance: f32, secs: f32 },
}

/// A stance, aura or shapeshift held all run.
#[derive(Debug)]
pub(in crate::survivors) struct FormDef {
    /// The spell's 1.12 name: "Cat Form".
    pub name: &'static str,
    /// The spell whose state kit stays on the hero, if it has one worth showing.
    pub aura: Option<u32>,
    /// What it grants for the run; empty for a form named as its own capstone.
    pub bonuses: &'static [Bonus],
    /// Its other mechanics (a trap laid every so often).
    pub specs: &'static [M],
    /// A shapeshift's model: the Night Elf and Tauren `CreatureDisplayInfo` rows.
    pub shape: Option<(u32, u32)>,
}

impl SpecDef {
    /// The summon or form's name.
    #[cfg(test)]
    pub fn summon_name(&self) -> &'static str {
        match &self.summon {
            Summon::Guardian(g) => g.name,
            Summon::Form(f) => f.name,
        }
    }

    /// The talent tooltip's spec lines: "Becomes: Frost Mage.", "Starts with Frostbolt.",
    /// "Summons a Water Elemental."
    pub fn tooltip(&self) -> Vec<String> {
        let summon = match &self.summon {
            Summon::Guardian(g) if g.count > 1 => {
                format!("Summons {} {}.", count_word(g.count), plural(g.name))
            }
            Summon::Guardian(g) => format!("Summons {} {}.", article(g.name), g.name),
            Summon::Form(f) if f.shape.is_some() => format!("Shifts into {}.", f.name),
            Summon::Form(f) => format!("Holds {} all run.", f.name),
        };
        vec![
            format!("Becomes: {}.", self.title),
            format!("Starts with {}.", self.start_name),
            summon,
        ]
    }
}

/// The spec `talent` grants, if it is a tree's capstone.
pub(in crate::survivors) fn by_talent(talent: u32) -> Option<&'static SpecDef> {
    SPECS.iter().find(|s| s.talent == talent)
}

/// The spec a class's learned ranks make: the capstone learned, if any. 51 points reach only one
/// tree's 31.
pub(in crate::survivors) fn learned(ranks: &[(u32, u8)]) -> Option<&'static SpecDef> {
    ranks
        .iter()
        .filter(|(_, rank)| *rank > 0)
        .find_map(|(id, _)| by_talent(*id))
}

/// Make `hero` `spec` for the run: its class's start ability becomes the spec's (keeping its rank),
/// and a form's bonuses and mechanics are granted. Idempotent, so it may run on every change.
pub(in crate::survivors) fn grant(hero: &mut HeroState, spec: &'static SpecDef) {
    let start = data::ability(spec.start);
    let class_start = hero.class.start;
    let owns_start = hero.abilities.iter().any(|o| o.def.key == spec.start);
    if let Some(first) = hero.abilities.first_mut() {
        if first.def.key == class_start && !owns_start {
            first.def = start;
        }
    }
    if let Summon::Form(form) = &spec.summon {
        let mut specs: Vec<M> = form.specs.to_vec();
        if !form.bonuses.is_empty() {
            let burst = M::new(
                Trigger::Periodic { every: 1.0 },
                Mech::Burst {
                    duration: PERMANENT,
                    bonuses: form.bonuses,
                },
            );
            specs.push(match form.aura {
                Some(spell) => burst.visual(spell),
                None => burst,
            });
        }
        if !specs.is_empty() && !game_mechanics::has(hero, SPEC_MECHANIC) {
            game_mechanics::grant(hero, SPEC_MECHANIC, specs);
        }
    }
}

/// The spec a hero is this run, on its entity: its guardians and its form follow it.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::survivors) struct HeroSpec(pub &'static SpecDef);

impl HeroSpec {
    /// The form's spell whose state kit the hero shows, for a form that has one.
    pub fn form_aura(&self) -> Option<u32> {
        match &self.0.summon {
            Summon::Form(f) => f.aura,
            Summon::Guardian(_) => None,
        }
    }
}

fn article(word: &str) -> &'static str {
    match word.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn count_word(n: u8) -> String {
    match n {
        2 => "two".into(),
        3 => "three".into(),
        n => n.to_string(),
    }
}

/// "Spirit Wolf" → "Spirit Wolves", "Treant" → "Treants".
fn plural(name: &str) -> String {
    match name.strip_suffix("Wolf") {
        Some(stem) => format!("{stem}Wolves"),
        None => format!("{name}s"),
    }
}

const fn form(
    name: &'static str,
    aura: Option<u32>,
    bonuses: &'static [Bonus],
    specs: &'static [M],
    shape: Option<(u32, u32)>,
) -> Summon {
    Summon::Form(FormDef {
        name,
        aura,
        bonuses,
        specs,
        shape,
    })
}

#[allow(clippy::too_many_arguments)]
const fn guardian(
    name: &'static str,
    display: u32,
    scale: f32,
    count: u8,
    act: Act,
    spell: u32,
    school: School,
    every: f32,
    amount: f32,
    range: f32,
) -> Summon {
    Summon::Guardian(GuardianDef {
        name,
        display,
        scale,
        count,
        act,
        spell,
        school,
        every,
        amount,
        range,
        object: false,
        special: None,
    })
}

/// `summon`, a guardian, with its signature `special`.
const fn with_special(summon: Summon, special: Special) -> Summon {
    match summon {
        Summon::Guardian(g) => Summon::Guardian(GuardianDef {
            special: Some(special),
            ..g
        }),
        form => form,
    }
}

/// The 27 specs, class by class in `data::CLASSES` order and each class's trees in tab order.
pub(in crate::survivors) static SPECS: &[SpecDef] = &[
    // --- Warrior ---
    SpecDef {
        tree: 161,
        talent: 135,
        title: "Arms Warrior",
        start: "mortal_strike",
        start_name: "Mortal Strike",
        summon: form(
            "Battle Stance",
            Some(2457),
            &[
                Bonus::new(StatKey::Damage, 0.10),
                Bonus::new(StatKey::CritDamage, 0.20),
            ],
            &[],
            None,
        ),
    },
    SpecDef {
        tree: 164,
        talent: 167,
        title: "Fury Warrior",
        start: "bloodthirst",
        start_name: "Bloodthirst",
        summon: form(
            "Berserker Stance",
            Some(2458),
            &[
                Bonus::new(StatKey::CritChance, 0.10),
                Bonus::new(StatKey::Cooldown, 0.10),
            ],
            &[],
            None,
        ),
    },
    SpecDef {
        tree: 163,
        talent: 148,
        title: "Protection Warrior",
        start: "shield_slam",
        start_name: "Shield Slam",
        summon: form(
            "Defensive Stance",
            Some(71),
            &[
                Bonus::new(StatKey::Armor, 0.15),
                Bonus::new(StatKey::MaxHealth, 30.0),
            ],
            &[],
            None,
        ),
    },
    // --- Paladin ---
    SpecDef {
        tree: 382,
        talent: 1502,
        title: "Holy Paladin",
        start: "holy_shock",
        start_name: "Holy Shock",
        summon: form(
            "Concentration Aura",
            Some(19746),
            &[Bonus::new(StatKey::Regen, 1.5)],
            &[M::new(
                Trigger::Periodic { every: 12.0 },
                Mech::HealPulse {
                    radius: 15.0,
                    amount: 20.0,
                },
            )
            .at(Aim::Hero)
            .visual(635)],
            None,
        ),
    },
    SpecDef {
        tree: 383,
        talent: 1430,
        title: "Protection Paladin",
        start: "judgement",
        start_name: "Judgement",
        summon: form(
            "Devotion Aura",
            Some(465),
            &[
                Bonus::new(StatKey::Armor, 0.15),
                Bonus::new(StatKey::MaxHealth, 25.0),
            ],
            &[],
            None,
        ),
    },
    SpecDef {
        tree: 381,
        talent: 1441,
        title: "Retribution Paladin",
        start: "crusader_strike",
        start_name: "Crusader Strike",
        summon: form(
            "Retribution Aura",
            Some(7294),
            &[Bonus::new(StatKey::Damage, 0.10)],
            &[M::new(
                Trigger::Periodic { every: 2.0 },
                Mech::Nova {
                    school: School::Holy,
                    radius: 5.0,
                    damage: 6.0,
                    knockback: 0.0,
                    cc: None,
                },
            )
            .at(Aim::Hero)
            .visual(7294)],
            None,
        ),
    },
    // --- Hunter ---
    SpecDef {
        tree: 361,
        talent: 1386,
        title: "Beast Mastery Hunter",
        start: "multi_shot",
        start_name: "Multi-Shot",
        summon: guardian(
            "Frostsaber",
            SPIRIT_BEAST,
            1.3,
            1,
            Act::Melee,
            17253,
            School::Physical,
            1.2,
            22.0,
            16.0,
        ),
    },
    SpecDef {
        tree: 363,
        talent: 1361,
        title: "Marksmanship Hunter",
        start: "aimed_shot",
        start_name: "Aimed Shot",
        summon: form(
            "Aspect of the Hawk",
            Some(13165),
            &[
                Bonus::new(StatKey::Damage, 0.10),
                Bonus::new(StatKey::CritChance, 0.05),
            ],
            &[],
            None,
        ),
    },
    SpecDef {
        tree: 362,
        talent: 1325,
        title: "Survival Hunter",
        start: "explosive_trap",
        start_name: "Explosive Trap",
        summon: form(
            "Frost Trap",
            None,
            &[],
            &[M::new(
                Trigger::Periodic { every: 9.0 },
                Mech::Nova {
                    school: School::Frost,
                    radius: 7.0,
                    damage: 10.0,
                    knockback: 0.0,
                    cc: Some(Cc {
                        kind: CcKind::Chill(0.4),
                        secs: 4.0,
                    }),
                },
            )
            .at(Aim::Densest { range: 18.0 })
            .visual(13809)],
            None,
        ),
    },
    // --- Rogue ---
    SpecDef {
        tree: 182,
        talent: 382,
        title: "Assassination Rogue",
        start: "backstab",
        start_name: "Backstab",
        summon: form(
            "Deadly Poison",
            None,
            &[],
            &[M::new(
                Trigger::OnHit {
                    of: Of::Any,
                    chance: 0.3,
                },
                Mech::Dot {
                    school: School::Nature,
                    dps: 4.0,
                    secs: 6.0,
                },
            )
            .visual(2818)],
            None,
        ),
    },
    SpecDef {
        tree: 181,
        talent: 205,
        title: "Combat Rogue",
        start: "eviscerate",
        start_name: "Eviscerate",
        summon: form(
            "Slice and Dice",
            Some(5171),
            &[Bonus::new(StatKey::Cooldown, 0.15)],
            &[],
            None,
        ),
    },
    SpecDef {
        tree: 183,
        talent: 381,
        title: "Subtlety Rogue",
        start: "ambush",
        start_name: "Ambush",
        summon: guardian(
            "Shadow Double",
            0,
            1.0,
            1,
            Act::Melee,
            16511,
            School::Shadow,
            1.0,
            16.0,
            14.0,
        ),
    },
    // --- Priest ---
    SpecDef {
        tree: 201,
        talent: 322,
        title: "Discipline Priest",
        start: "holy_fire",
        start_name: "Holy Fire",
        summon: form(
            "Inner Fire",
            Some(588),
            &[
                Bonus::new(StatKey::Armor, 0.10),
                Bonus::new(StatKey::Damage, 0.10),
            ],
            &[],
            None,
        ),
    },
    SpecDef {
        tree: 202,
        talent: 1637,
        title: "Holy Priest",
        start: "holy_nova",
        start_name: "Holy Nova",
        summon: Summon::Guardian(GuardianDef {
            name: "Lightwell",
            display: LIGHTWELL,
            scale: 1.0,
            count: 1,
            act: Act::Heal,
            spell: 7001,
            school: School::Holy,
            every: 4.0,
            amount: 14.0,
            range: 15.0,
            object: true,
            special: None,
        }),
    },
    SpecDef {
        tree: 203,
        talent: 521,
        title: "Shadow Priest",
        start: "mind_flay",
        start_name: "Mind Flay",
        summon: form("Shadowform", Some(15473), &[], &[], None),
    },
    // --- Shaman ---
    SpecDef {
        tree: 261,
        talent: 573,
        title: "Elemental Shaman",
        start: "chain_lightning",
        start_name: "Chain Lightning",
        summon: guardian(
            "Air Elemental",
            5187,
            0.7,
            1,
            Act::Bolt,
            403,
            School::Nature,
            2.0,
            18.0,
            22.0,
        ),
    },
    SpecDef {
        tree: 263,
        talent: 901,
        title: "Enhancement Shaman",
        start: "stormstrike",
        start_name: "Stormstrike",
        summon: guardian(
            "Spirit Wolf",
            SPIRIT_WOLF,
            1.0,
            2,
            Act::Melee,
            17253,
            School::Physical,
            1.0,
            12.0,
            14.0,
        ),
    },
    SpecDef {
        tree: 262,
        talent: 590,
        title: "Restoration Shaman",
        start: "earth_shock",
        start_name: "Earth Shock",
        summon: guardian(
            "Healing Stream Totem",
            4587,
            1.0,
            1,
            Act::Heal,
            5394,
            School::Nature,
            3.0,
            8.0,
            15.0,
        ),
    },
    // --- Mage ---
    SpecDef {
        tree: 81,
        talent: 87,
        title: "Arcane Mage",
        start: "arcane_missiles",
        start_name: "Arcane Missiles",
        summon: guardian(
            ARCANE_NAME,
            ARCANE_DISPLAY,
            ARCANE_SCALE,
            1,
            Act::Bolt,
            7268,
            School::Arcane,
            1.4,
            10.0,
            22.0,
        ),
    },
    SpecDef {
        tree: 41,
        talent: 36,
        title: "Fire Mage",
        start: "pyroblast",
        start_name: "Pyroblast",
        summon: guardian(
            "Fire Elemental",
            1070,
            0.6,
            1,
            Act::Bolt,
            133,
            School::Fire,
            2.2,
            20.0,
            22.0,
        ),
    },
    SpecDef {
        tree: 61,
        talent: 71,
        title: "Frost Mage",
        start: "frostbolt",
        start_name: "Frostbolt",
        summon: guardian(
            "Water Elemental",
            525,
            0.8,
            1,
            Act::Bolt,
            116,
            School::Frost,
            1.8,
            16.0,
            22.0,
        ),
    },
    // --- Warlock ---
    SpecDef {
        tree: 302,
        talent: 1022,
        title: "Affliction Warlock",
        start: "corruption",
        start_name: "Corruption",
        summon: guardian(
            "Felhunter",
            FELHUNTER,
            1.0,
            1,
            Act::Melee,
            17253,
            School::Shadow,
            1.2,
            16.0,
            16.0,
        ),
    },
    SpecDef {
        tree: 303,
        talent: 1282,
        title: "Demonology Warlock",
        start: "death_coil",
        start_name: "Death Coil",
        summon: with_special(
            guardian(
                "Doomguard",
                1912,
                0.8,
                1,
                Act::Melee,
                0,
                School::Shadow,
                1.6,
                30.0,
                16.0,
            ),
            Special::Rain {
                spell: RAIN_OF_FIRE,
                every: 10.0,
                radius: 6.0,
                dps: 3.0,
                secs: 4.0,
            },
        ),
    },
    SpecDef {
        tree: 301,
        talent: 968,
        title: "Destruction Warlock",
        start: "immolate",
        start_name: "Immolate",
        summon: with_special(
            guardian(
                "Infernal",
                169,
                0.7,
                1,
                Act::Melee,
                0,
                School::Fire,
                1.8,
                34.0,
                16.0,
            ),
            Special::Aura {
                spell: IMMOLATION,
                every: 2.0,
                radius: 5.0,
                damage: 4.0,
            },
        ),
    },
    // --- Druid ---
    SpecDef {
        tree: 283,
        talent: 793,
        title: "Balance Druid",
        start: "starfire",
        start_name: "Starfire",
        summon: form("Moonkin Form", Some(24858), &[], &[], Some((15374, 15375))),
    },
    SpecDef {
        tree: 281,
        talent: 809,
        title: "Feral Druid",
        start: "shred",
        start_name: "Shred",
        summon: form(
            "Cat Form",
            Some(768),
            &[
                Bonus::new(StatKey::MoveSpeed, 0.10),
                Bonus::new(StatKey::CritChance, 0.05),
            ],
            &[],
            Some((892, 8571)),
        ),
    },
    SpecDef {
        tree: 282,
        talent: 844,
        title: "Restoration Druid",
        start: "insect_swarm",
        start_name: "Insect Swarm",
        summon: with_special(
            guardian(
                "Treant",
                TREANT,
                TREANT_SCALE,
                3,
                Act::Melee,
                0,
                School::Nature,
                1.4,
                10.0,
                14.0,
            ),
            Special::Root {
                spell: ENTANGLING_ROOTS,
                chance: 0.3,
                secs: 2.0,
            },
        ),
    },
];

/// A Winterspring frostsaber, the white tiger hunters prize.
const SPIRIT_BEAST: u32 = 748;
/// The shaman's own Ghost Wolf, its ghostly wolf.
const SPIRIT_WOLF: u32 = 1236;
/// `G_HolyLightWell`, the well the priest's Lightwell raises.
const LIGHTWELL: u32 = 6671;
/// The Felhunter's felhound.
const FELHUNTER: u32 = 850;
/// A young ancient protector: the treant.
const TREANT: u32 = 860;
const TREANT_SCALE: f32 = 0.45;
/// The Doomguard's Rain of Fire (rank 1), the Infernal's Immolation (the burn its aura deals; the
/// aura's own row has no visual), and the Treants' Entangling Roots (rank 1).
pub(in crate::survivors) const RAIN_OF_FIRE: u32 = 5740;
pub(in crate::survivors) const IMMOLATION: u32 = 20153;
pub(in crate::survivors) const ENTANGLING_ROOTS: u32 = 339;
const ARCANE_NAME: &str = "Mana Fiend";
const ARCANE_DISPLAY: u32 = 14252;
const ARCANE_SCALE: f32 = 0.5;

#[cfg(test)]
mod tests {
    use super::super::super::bonus::Bonuses;
    use super::*;

    fn class(name: &str) -> &'static data::ClassDef {
        data::CLASSES.iter().find(|c| c.name == name).unwrap()
    }

    #[test]
    fn every_tree_has_one_spec_with_a_real_start_and_summon() {
        assert_eq!(SPECS.len(), 27);
        let trees: std::collections::HashSet<u32> = SPECS.iter().map(|s| s.tree).collect();
        assert_eq!(trees.len(), 27, "one spec per tree");
        for spec in SPECS {
            assert_eq!(
                super::super::mechanics::milestones(spec.tree).map(|m| m[2]),
                Some(spec.talent),
                "{}: the capstone is the tree's 30-point landmark",
                spec.title
            );
            let start = data::ability(spec.start);
            assert!(
                start.damage > 0.0 || start.dot > 0.0,
                "{} starts dealing damage",
                spec.title
            );
            assert!(!spec.summon_name().is_empty());
            match &spec.summon {
                Summon::Guardian(g) => {
                    assert!(g.count >= 1 && g.every > 0.0 && g.amount > 0.0 && g.range > 0.0);
                }
                Summon::Form(f) => {
                    assert!(
                        !f.bonuses.is_empty() || !f.specs.is_empty() || f.aura.is_some(),
                        "{} does something or shows something",
                        f.name
                    );
                    for m in f.specs {
                        m.validate().unwrap_or_else(|e| panic!("{}: {e}", f.name));
                    }
                }
            }
        }
    }

    #[test]
    fn each_class_starts_each_spec_differently_from_its_class() {
        for (i, def) in data::CLASSES.iter().enumerate() {
            let starts: Vec<&str> = SPECS[i * 3..i * 3 + 3].iter().map(|s| s.start).collect();
            for s in &starts {
                assert!(
                    def.pool.contains(s),
                    "{}: {s} is a {} spell",
                    def.name,
                    def.name
                );
                assert_ne!(*s, def.start, "{}: a spec swaps the start", def.name);
            }
            assert!(starts[0] != starts[1] && starts[1] != starts[2] && starts[0] != starts[2]);
        }
    }

    #[test]
    fn thirty_one_points_swap_the_start_and_grant_the_form() {
        let mage = class("Mage");
        let frost = by_talent(71).unwrap();
        let ranks = [(37, 5), (71, 1)];
        assert_eq!(learned(&ranks).map(|s| s.title), Some("Frost Mage"));
        assert_eq!(learned(&[(37, 5), (71, 0)]).map(|s| s.title), None);
        let mut hero = HeroState::new(0, mage, 1, Bonuses::default());
        assert_eq!(hero.abilities[0].def.key, "fireball");
        grant(&mut hero, frost);
        assert_eq!(hero.abilities[0].def.key, "frostbolt");
        assert_eq!(hero.abilities.len(), 1);
        // Idempotent: a second grant changes nothing.
        grant(&mut hero, frost);
        assert_eq!(hero.abilities[0].def.key, "frostbolt");
        assert_eq!(
            frost.tooltip(),
            [
                "Becomes: Frost Mage.",
                "Starts with Frostbolt.",
                "Summons a Water Elemental."
            ]
        );

        let druid = class("Druid");
        let feral = by_talent(809).unwrap();
        let mut hero = HeroState::new(0, druid, 4, Bonuses::default());
        grant(&mut hero, feral);
        assert_eq!(hero.abilities[0].def.key, "shred");
        assert!(
            game_mechanics::has(&hero, SPEC_MECHANIC),
            "Cat Form is held"
        );
        assert_eq!(
            feral.tooltip()[2],
            "Shifts into Cat Form.",
            "and wears its model"
        );
        assert_eq!(HeroSpec(feral).form_aura(), Some(768));
        assert_eq!(HeroSpec(frost).form_aura(), None);
    }

    #[test]
    fn a_learned_start_keeps_its_rank_and_a_later_pick_is_not_swapped() {
        let mage = class("Mage");
        let fire = by_talent(36).unwrap();
        let mut hero = HeroState::new(0, mage, 1, Bonuses::default());
        hero.abilities[0].level = 3;
        grant(&mut hero, fire);
        assert_eq!(
            (hero.abilities[0].def.key, hero.abilities[0].level),
            ("pyroblast", 3)
        );
    }
}
