//! The mode's tables: classes, abilities and their legendary transformations, passives, enemies
//! and the wave clock. Spell ids name `Spell.dbc` rows only for their icon, name and visual chain;
//! every number here is the mode's.

use super::bonus::{Bonus, School, StatKey};
use super::mechanics::{Aim, Cc, CcKind, Effect as Mech, MechanicSpec as M, Of, Trigger};

/// A weapon on the hero's virtual item slots: `(slot, display, class, subclass, material, inv,
/// sheath)`, the creature form of `UNIT_VIRTUAL_ITEM_SLOT_DISPLAY` and `_INFO`.
pub(crate) type Weapon = (u16, u32, u8, u8, u8, u8, u8);

pub(crate) struct ClassDef {
    pub id: u8,
    pub name: &'static str,
    /// `RAID_CLASS_COLORS`, 0xRRGGBB.
    pub color: u32,
    /// `UNIT_FIELD_BYTES_0` byte 3.
    pub power: u8,
    pub start: &'static str,
    pub pool: &'static [&'static str],
    /// Class-trainer `CreatureDisplayInfo` ids, deduped by race and sex at the menu.
    pub looks: &'static [u32],
    pub weapons: &'static [Weapon],
    /// The sheath state the hero fights in: 1 melee, 2 ranged.
    pub sheath: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Kind {
    /// A missile at the nearest enemies (or instant if the spell has no speed).
    Bolt,
    /// An instant hit on the nearest enemies in range.
    Strike,
    /// A close hit on the nearest enemies in melee reach.
    Melee,
    /// Everything in a radius around the hero.
    Nova,
    /// Everything in a cone toward the nearest enemy.
    Cone,
    /// A ground area at the densest nearby enemy, ticking for `duration`.
    Ground,
    /// A ground area under the hero, ticking for `duration`.
    GroundSelf,
    /// Jumps from enemy to enemy.
    Chain,
    /// A damage-over-time on several enemies.
    Dot,
    /// Always on: pulses every `cooldown` around the hero, with the spell's state visual on him.
    Aura,
    /// Always on: zaps one enemy close to the hero every `cooldown`.
    Orbit,
    /// Absorbs `damage` for `duration`.
    Shield,
    /// Immune for `duration`.
    Immune,
    /// Returns `damage` to every attacker for `duration`.
    Thorns,
    /// Heals `damage` over `duration`.
    Heal,
    /// Drops a totem that casts `sub` for `duration`.
    Totem,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Effect {
    None,
    /// Speed multiplier for seconds.
    Slow(f32, f32),
    Root(f32),
    Stun(f32),
    Fear(f32),
    /// Outgoing damage multiplier for seconds.
    Weaken(f32, f32),
    /// Heal the hero by this fraction of damage dealt.
    Drain(f32),
    /// Kill outright below this health fraction.
    Execute(f32),
}

pub(crate) struct AbilityDef {
    pub key: &'static str,
    pub spell: u32,
    pub kind: Kind,
    /// Direct damage; per tick for areas and pulses; absorb, heal or reflect for the self kinds.
    pub damage: f32,
    /// Damage over time, per second, for `duration`.
    pub dot: f32,
    pub cooldown: f32,
    pub range: f32,
    /// Splash for single hits, area for the area kinds, the jump for a chain.
    pub radius: f32,
    pub count: u32,
    pub duration: f32,
    pub effect: Effect,
    /// The spell whose state kit marks an affected enemy, 0 for none.
    pub mark: u32,
    /// A totem's display, and the spell it casts.
    pub totem: (u32, u32),
    /// The area's own visual hangs overhead and would hide the field from the camera, so the
    /// area plays the spell's impact on whatever it hits instead.
    pub overhead: bool,
}

const fn ab(key: &'static str, spell: u32, kind: Kind) -> AbilityDef {
    AbilityDef {
        key,
        spell,
        kind,
        damage: 0.0,
        dot: 0.0,
        cooldown: 2.0,
        range: 20.0,
        radius: 0.0,
        count: 1,
        duration: 0.0,
        effect: Effect::None,
        mark: 0,
        totem: (0, 0),
        overhead: false,
    }
}

macro_rules! ability {
    ($key:literal, $spell:literal, $kind:ident { $($f:ident: $v:expr),* $(,)? }) => {
        AbilityDef { $($f: $v,)* ..ab($key, $spell, Kind::$kind) }
    };
}

pub(crate) static ABILITIES: &[AbilityDef] = &[
    // --- Mage ---
    ability!(
        "frostbolt",
        116,
        Bolt {
            damage: 12.0,
            cooldown: 1.3,
            range: 24.0,
            effect: Effect::Slow(0.5, 2.5),
            mark: 116
        }
    ),
    ability!(
        "fireball",
        133,
        // WoW's Fireball leaves a short burn; each cast's stacks on the last, and feeds Ignite.
        Bolt {
            damage: 18.0,
            dot: 0.75,
            duration: 4.0,
            cooldown: 2.4,
            range: 26.0,
            radius: 3.5
        }
    ),
    ability!(
        "arcane_missiles",
        7268,
        Bolt {
            damage: 6.0,
            cooldown: 1.9,
            range: 22.0,
            count: 3
        }
    ),
    ability!(
        "frost_nova",
        122,
        Nova {
            damage: 8.0,
            cooldown: 7.0,
            radius: 9.0,
            effect: Effect::Root(2.5),
            mark: 122
        }
    ),
    ability!(
        "arcane_explosion",
        1449,
        Nova {
            damage: 10.0,
            cooldown: 2.8,
            radius: 7.0
        }
    ),
    ability!(
        "blizzard",
        10,
        Ground {
            damage: 7.0,
            cooldown: 9.0,
            range: 20.0,
            radius: 7.0,
            duration: 6.0,
            effect: Effect::Slow(0.6, 1.5)
        }
    ),
    ability!(
        "flamestrike",
        2120,
        Ground {
            damage: 14.0,
            cooldown: 8.0,
            range: 20.0,
            radius: 5.0,
            duration: 4.0
        }
    ),
    ability!(
        "fire_blast",
        2136,
        Strike {
            damage: 26.0,
            cooldown: 3.5,
            range: 18.0
        }
    ),
    ability!(
        "cone_of_cold",
        120,
        Cone {
            damage: 14.0,
            cooldown: 4.5,
            radius: 11.0,
            effect: Effect::Slow(0.5, 3.0),
            mark: 120
        }
    ),
    ability!(
        "blast_wave",
        11113,
        Nova {
            damage: 18.0,
            cooldown: 6.5,
            radius: 8.0,
            effect: Effect::Slow(0.6, 2.0)
        }
    ),
    ability!(
        "pyroblast",
        11366,
        Bolt {
            damage: 70.0,
            dot: 6.0,
            duration: 6.0,
            cooldown: 6.0,
            range: 30.0
        }
    ),
    // --- Warrior ---
    ability!(
        "cleave",
        845,
        Melee {
            damage: 14.0,
            cooldown: 1.1,
            range: 5.5,
            count: 3
        }
    ),
    ability!(
        "heroic_strike",
        78,
        Melee {
            damage: 28.0,
            cooldown: 1.6,
            range: 5.5
        }
    ),
    ability!(
        "whirlwind",
        1680,
        Nova {
            damage: 16.0,
            cooldown: 4.0,
            radius: 7.0
        }
    ),
    ability!(
        "thunder_clap",
        6343,
        Nova {
            damage: 10.0,
            cooldown: 5.0,
            radius: 8.0,
            effect: Effect::Slow(0.5, 3.0),
            mark: 6343
        }
    ),
    ability!(
        "sweeping_strikes",
        12292,
        Aura {
            damage: 6.0,
            cooldown: 0.6,
            radius: 5.5
        }
    ),
    ability!(
        "demoralizing_shout",
        1160,
        Nova {
            damage: 4.0,
            cooldown: 8.0,
            radius: 11.0,
            effect: Effect::Weaken(0.5, 5.0),
            mark: 1160
        }
    ),
    ability!(
        "execute",
        5308,
        Strike {
            damage: 40.0,
            cooldown: 2.0,
            range: 8.0,
            count: 2,
            effect: Effect::Execute(0.3)
        }
    ),
    ability!(
        "mortal_strike",
        12294,
        Melee {
            damage: 48.0,
            cooldown: 3.0,
            range: 6.0
        }
    ),
    ability!(
        "retaliation",
        20230,
        Thorns {
            damage: 14.0,
            cooldown: 18.0,
            duration: 8.0
        }
    ),
    // --- Rogue ---
    ability!(
        "sinister_strike",
        1752,
        Melee {
            damage: 15.0,
            cooldown: 0.85,
            range: 5.5
        }
    ),
    ability!(
        "eviscerate",
        2098,
        Melee {
            damage: 42.0,
            cooldown: 2.8,
            range: 5.5
        }
    ),
    ability!(
        "blade_flurry",
        13877,
        Nova {
            damage: 11.0,
            cooldown: 3.0,
            radius: 6.0
        }
    ),
    ability!(
        "rupture",
        1943,
        Dot {
            dot: 5.0,
            cooldown: 3.5,
            range: 10.0,
            count: 3,
            duration: 6.0
        }
    ),
    ability!(
        "gouge",
        1776,
        Melee {
            damage: 8.0,
            cooldown: 4.0,
            range: 6.0,
            count: 2,
            effect: Effect::Stun(2.0),
            mark: 1776
        }
    ),
    ability!(
        "kidney_shot",
        408,
        Melee {
            damage: 12.0,
            cooldown: 6.0,
            range: 6.0,
            effect: Effect::Stun(3.5),
            mark: 408
        }
    ),
    // --- Priest ---
    ability!(
        "smite",
        585,
        Strike {
            damage: 14.0,
            cooldown: 1.2,
            range: 20.0
        }
    ),
    ability!(
        "shadow_word_pain",
        589,
        Dot {
            dot: 4.0,
            cooldown: 3.0,
            range: 18.0,
            count: 3,
            duration: 9.0
        }
    ),
    ability!(
        "holy_fire",
        14914,
        Strike {
            damage: 22.0,
            dot: 3.0,
            duration: 6.0,
            cooldown: 3.5,
            range: 20.0
        }
    ),
    ability!(
        "mind_blast",
        8092,
        Strike {
            damage: 32.0,
            cooldown: 3.5,
            range: 22.0
        }
    ),
    ability!(
        "holy_nova",
        15237,
        Nova {
            damage: 11.0,
            cooldown: 3.2,
            radius: 9.0,
            effect: Effect::Drain(0.1)
        }
    ),
    ability!(
        "psychic_scream",
        8122,
        Nova {
            damage: 2.0,
            cooldown: 12.0,
            radius: 8.0,
            effect: Effect::Fear(3.5),
            mark: 8122
        }
    ),
    ability!(
        "power_word_shield",
        17,
        Shield {
            damage: 30.0,
            cooldown: 14.0,
            duration: 10.0
        }
    ),
    // --- Warlock ---
    ability!(
        "shadow_bolt",
        686,
        Bolt {
            damage: 15.0,
            cooldown: 1.4,
            range: 24.0
        }
    ),
    ability!(
        "corruption",
        172,
        Dot {
            dot: 4.0,
            cooldown: 3.0,
            range: 20.0,
            count: 3,
            duration: 10.0,
            mark: 172
        }
    ),
    ability!(
        "curse_of_agony",
        980,
        Dot {
            dot: 3.5,
            cooldown: 2.5,
            range: 20.0,
            count: 4,
            duration: 12.0
        }
    ),
    ability!(
        "immolate",
        348,
        Strike {
            damage: 12.0,
            dot: 4.0,
            duration: 9.0,
            cooldown: 2.4,
            range: 20.0,
            count: 2,
            mark: 348
        }
    ),
    ability!(
        "rain_of_fire",
        5740,
        Ground {
            damage: 8.0,
            cooldown: 9.0,
            range: 20.0,
            radius: 7.0,
            duration: 6.0
        }
    ),
    ability!(
        "hellfire",
        5857,
        Aura {
            damage: 6.0,
            cooldown: 1.0,
            radius: 7.0
        }
    ),
    ability!(
        "death_coil",
        6789,
        Bolt {
            damage: 26.0,
            cooldown: 5.0,
            range: 22.0,
            effect: Effect::Drain(0.5)
        }
    ),
    ability!(
        "searing_pain",
        5676,
        Strike {
            damage: 18.0,
            cooldown: 1.8,
            range: 20.0
        }
    ),
    ability!(
        "shadowburn",
        17877,
        Strike {
            damage: 46.0,
            cooldown: 5.0,
            range: 20.0
        }
    ),
    // --- Hunter ---
    ability!(
        "arcane_shot",
        3044,
        Bolt {
            damage: 13.0,
            cooldown: 1.0,
            range: 28.0
        }
    ),
    ability!(
        "multi_shot",
        2643,
        Bolt {
            damage: 10.0,
            cooldown: 2.0,
            range: 26.0,
            count: 3
        }
    ),
    ability!(
        "serpent_sting",
        1978,
        Bolt {
            damage: 2.0,
            dot: 4.0,
            duration: 9.0,
            cooldown: 2.6,
            range: 26.0,
            count: 2
        }
    ),
    ability!(
        "volley",
        1510,
        Ground {
            damage: 8.0,
            cooldown: 9.0,
            range: 24.0,
            radius: 7.0,
            duration: 5.0
        }
    ),
    ability!(
        "explosive_trap",
        13812,
        Ground {
            damage: 22.0,
            dot: 4.0,
            cooldown: 8.0,
            range: 14.0,
            radius: 5.0,
            duration: 0.0
        }
    ),
    ability!(
        "concussive_shot",
        5116,
        Bolt {
            damage: 8.0,
            cooldown: 3.0,
            range: 28.0,
            effect: Effect::Slow(0.5, 4.0),
            mark: 5116
        }
    ),
    ability!(
        "aimed_shot",
        19434,
        Bolt {
            damage: 48.0,
            cooldown: 4.0,
            range: 32.0
        }
    ),
    // --- Shaman ---
    ability!(
        "lightning_bolt",
        403,
        Bolt {
            damage: 15.0,
            cooldown: 1.3,
            range: 24.0
        }
    ),
    ability!(
        "chain_lightning",
        421,
        Chain {
            damage: 18.0,
            cooldown: 3.0,
            range: 20.0,
            radius: 9.0,
            count: 3
        }
    ),
    ability!(
        "earth_shock",
        8042,
        Strike {
            damage: 22.0,
            cooldown: 2.5,
            range: 20.0
        }
    ),
    ability!(
        "flame_shock",
        8050,
        Strike {
            damage: 10.0,
            dot: 4.0,
            duration: 9.0,
            cooldown: 3.0,
            range: 20.0,
            count: 2
        }
    ),
    ability!(
        "frost_shock",
        8056,
        Strike {
            damage: 16.0,
            cooldown: 3.0,
            range: 20.0,
            effect: Effect::Slow(0.5, 4.0)
        }
    ),
    ability!(
        "searing_totem",
        3599,
        Totem {
            damage: 9.0,
            cooldown: 14.0,
            range: 20.0,
            duration: 14.0,
            totem: (4589, 3606)
        }
    ),
    ability!(
        "magma_totem",
        8190,
        Totem {
            damage: 8.0,
            cooldown: 16.0,
            radius: 7.0,
            duration: 14.0,
            totem: (4683, 8349)
        }
    ),
    ability!(
        "lightning_shield",
        324,
        Orbit {
            damage: 12.0,
            cooldown: 0.9,
            radius: 4.0
        }
    ),
    // --- Paladin ---
    ability!(
        "hammer_of_wrath",
        24275,
        Bolt {
            damage: 16.0,
            cooldown: 1.5,
            range: 24.0
        }
    ),
    ability!(
        "consecration",
        26573,
        GroundSelf {
            damage: 7.0,
            cooldown: 9.0,
            radius: 7.0,
            duration: 8.0
        }
    ),
    ability!(
        "holy_shock",
        25912,
        Strike {
            damage: 24.0,
            cooldown: 2.5,
            range: 20.0
        }
    ),
    ability!(
        "exorcism",
        879,
        Strike {
            damage: 36.0,
            cooldown: 5.0,
            range: 22.0
        }
    ),
    ability!(
        "holy_wrath",
        2812,
        Nova {
            damage: 14.0,
            cooldown: 5.0,
            radius: 11.0
        }
    ),
    ability!(
        "hammer_of_justice",
        853,
        Strike {
            damage: 8.0,
            cooldown: 6.0,
            range: 12.0,
            count: 2,
            effect: Effect::Stun(3.0),
            mark: 853
        }
    ),
    ability!(
        "divine_shield",
        642,
        Immune {
            cooldown: 28.0,
            duration: 4.0
        }
    ),
    ability!(
        "judgement",
        20271,
        Strike {
            damage: 26.0,
            cooldown: 4.0,
            range: 14.0,
            count: 2
        }
    ),
    // --- Druid ---
    ability!(
        "wrath",
        5176,
        Bolt {
            damage: 13.0,
            cooldown: 1.2,
            range: 24.0
        }
    ),
    ability!(
        "moonfire",
        8921,
        Strike {
            damage: 10.0,
            dot: 3.0,
            duration: 9.0,
            cooldown: 2.0,
            range: 22.0,
            count: 2
        }
    ),
    ability!(
        "starfire",
        2912,
        Strike {
            damage: 40.0,
            cooldown: 4.0,
            range: 24.0
        }
    ),
    ability!(
        "hurricane",
        16914,
        Ground {
            damage: 7.0,
            cooldown: 10.0,
            range: 20.0,
            radius: 8.0,
            duration: 7.0,
            effect: Effect::Slow(0.6, 1.5),
            overhead: true
        }
    ),
    ability!(
        "insect_swarm",
        5570,
        Dot {
            dot: 4.0,
            cooldown: 3.0,
            range: 20.0,
            count: 3,
            duration: 10.0,
            mark: 5570
        }
    ),
    ability!(
        "entangling_roots",
        339,
        Dot {
            dot: 2.0,
            cooldown: 6.0,
            range: 16.0,
            count: 4,
            duration: 5.0,
            effect: Effect::Root(4.0),
            mark: 339
        }
    ),
    ability!(
        "thorns",
        467,
        Thorns {
            damage: 8.0,
            cooldown: 15.0,
            duration: 15.0
        }
    ),
    ability!(
        "rejuvenation",
        774,
        Heal {
            damage: 30.0,
            cooldown: 15.0,
            duration: 10.0
        }
    ),
    // --- Spec signatures: the 31-point talents' starting spells the pools lacked ---
    ability!(
        "bloodthirst",
        23881,
        Melee {
            damage: 30.0,
            cooldown: 1.6,
            range: 5.5,
            effect: Effect::Drain(0.15)
        }
    ),
    ability!(
        "shield_slam",
        23922,
        Melee {
            damage: 34.0,
            cooldown: 2.6,
            range: 6.0,
            radius: 2.5,
            effect: Effect::Stun(0.8)
        }
    ),
    ability!(
        "crusader_strike",
        2537,
        Melee {
            damage: 22.0,
            cooldown: 1.3,
            range: 6.0,
            count: 2
        }
    ),
    ability!(
        "backstab",
        53,
        Melee {
            damage: 30.0,
            cooldown: 1.5,
            range: 5.0
        }
    ),
    ability!(
        "ambush",
        8676,
        Melee {
            damage: 46.0,
            cooldown: 2.6,
            range: 5.5,
            radius: 2.0
        }
    ),
    ability!(
        "mind_flay",
        15407,
        Strike {
            damage: 13.0,
            cooldown: 1.1,
            range: 20.0,
            effect: Effect::Slow(0.5, 1.5),
            mark: 15407
        }
    ),
    ability!(
        "stormstrike",
        17364,
        Melee {
            damage: 32.0,
            cooldown: 2.2,
            range: 6.0,
            count: 2
        }
    ),
    ability!(
        "shred",
        5221,
        Melee {
            damage: 24.0,
            cooldown: 1.1,
            range: 5.0,
            dot: 3.0,
            duration: 4.0
        }
    ),
];

pub(crate) fn ability(key: &str) -> &'static AbilityDef {
    ABILITIES
        .iter()
        .find(|a| a.key == key)
        .unwrap_or_else(|| panic!("survivors: no ability {key:?}"))
}

/// What an ability becomes at its top rank, the legendary card's transformation: its mechanics
/// are granted on the hero for the rest of the run ([`super::mechanics::sync_legendaries`]).
pub(crate) struct LegendaryDef {
    /// The ability it transforms, by key.
    pub ability: &'static str,
    pub name: &'static str,
    /// One line, for the card and the tooltip.
    pub text: &'static str,
    pub mechanics: &'static [M],
    /// A real 1.12 spell: its icon on the card, its cast on the hero as the transformation takes,
    /// and, when it and the ability both fly a missile, the missile every cast flies after.
    pub visual_spell: u32,
}

/// The legendary of the ability `key`, if it has one (every ability does).
pub(crate) fn legendary(key: &str) -> Option<&'static LegendaryDef> {
    LEGENDARIES.iter().find(|l| l.ability == key)
}

const fn hit(key: &'static str) -> Trigger {
    hit_by(key, 1.0)
}

const fn hit_by(key: &'static str, chance: f32) -> Trigger {
    Trigger::OnHit {
        of: Of::Ability(key),
        chance,
    }
}

const fn crit(key: &'static str) -> Trigger {
    Trigger::OnCrit {
        of: Of::Ability(key),
        chance: 1.0,
    }
}

const fn kill(key: &'static str) -> Trigger {
    kill_by(key, 1.0)
}

const fn kill_by(key: &'static str, chance: f32) -> Trigger {
    Trigger::OnKill {
        of: Of::Ability(key),
        chance,
    }
}

const fn cast(key: &'static str) -> Trigger {
    Trigger::OnCast(Of::Ability(key))
}

const fn nova(school: School, radius: f32, damage: f32, knockback: f32) -> Mech {
    Mech::Nova {
        school,
        radius,
        damage,
        knockback,
        cc: None,
    }
}

const fn nova_cc(school: School, radius: f32, damage: f32, kind: CcKind, secs: f32) -> Mech {
    Mech::Nova {
        school,
        radius,
        damage,
        knockback: 0.0,
        cc: Some(Cc { kind, secs }),
    }
}

const fn ground(school: School, radius: f32, dps: f32, duration: f32) -> Mech {
    Mech::BurningGround {
        school,
        radius,
        dps,
        duration,
    }
}

const fn dot(school: School, dps: f32, secs: f32) -> Mech {
    Mech::Dot { school, dps, secs }
}

const fn cc(kind: CcKind, secs: f32) -> Mech {
    Mech::ApplyCc { kind, secs }
}

const fn spread(radius: f32, n: u8) -> Aim {
    Aim::Spread { radius, n }
}

/// Every ability's transformation, class by class as [`ABILITIES`] runs. Each is built from the
/// shared [`super::mechanics`] effects, so none needs code of its own.
pub(crate) static LEGENDARIES: &[LegendaryDef] = &[
    // --- Mage ---
    LegendaryDef {
        ability: "frostbolt",
        name: "Shatterbolt",
        text: "Bursts into three icy shards and a chilling nova where it hits.",
        mechanics: &[
            M::new(
                hit("frostbolt"),
                Mech::Split {
                    count: 3,
                    spread: 8.0,
                },
            )
            .visual(116),
            M::new(
                hit("frostbolt"),
                nova_cc(School::Frost, 3.5, 6.0, CcKind::Chill(0.5), 2.0),
            )
            .visual(122)
            .cooldown(0.4),
        ],
        visual_spell: 122,
    },
    LegendaryDef {
        ability: "fireball",
        name: "Hot Streak",
        text: "Swells into a Pyroblast that bursts wide and leaves the ground ablaze.",
        mechanics: &[
            M::new(hit("fireball"), nova(School::Fire, 4.5, 10.0, 1.5)).visual(2136),
            M::new(hit("fireball"), ground(School::Fire, 4.0, 8.0, 4.0))
                .visual(2120)
                .cooldown(1.0),
        ],
        visual_spell: 11366,
    },
    LegendaryDef {
        ability: "arcane_missiles",
        name: "Arcane Cascade",
        text: "Every missile arcs on to a second enemy, and kills may loose another volley.",
        mechanics: &[
            M::new(
                hit("arcane_missiles"),
                Mech::Chain {
                    jumps: 1,
                    falloff: 0.8,
                },
            )
            .visual(7268),
            M::new(
                kill_by("arcane_missiles", 0.3),
                Mech::Echo { extra_casts: 1 },
            )
            .cooldown(1.5),
        ],
        visual_spell: 12042,
    },
    LegendaryDef {
        ability: "frost_nova",
        name: "Deep Freeze",
        text: "Pulses twice, and frozen enemies below a third of their health shatter.",
        mechanics: &[
            M::new(cast("frost_nova"), Mech::Echo { extra_casts: 1 }),
            M::new(
                hit("frost_nova"),
                Mech::Execute {
                    threshold: 0.33,
                    mult: 3.0,
                },
            )
            .visual(116),
        ],
        visual_spell: 12472,
    },
    LegendaryDef {
        ability: "arcane_explosion",
        name: "Arcane Shockwave",
        text: "A second, wider ring of arcane force hurls the crowd back.",
        mechanics: &[M::new(
            cast("arcane_explosion"),
            nova(School::Arcane, 11.0, 6.0, 4.0),
        )
        .at(Aim::Hero)
        .visual(1449)],
        visual_spell: 12043,
    },
    LegendaryDef {
        ability: "blizzard",
        name: "Endless Winter",
        text: "A second storm follows you, freezing some of what it catches.",
        mechanics: &[
            M::new(cast("blizzard"), ground(School::Frost, 6.0, 6.0, 6.0))
                .at(Aim::Follow)
                .visual(10),
            M::new(hit_by("blizzard", 0.1), cc(CcKind::Root, 1.5)).visual(116),
        ],
        visual_spell: 10,
    },
    LegendaryDef {
        ability: "flamestrike",
        name: "Twin Pillars",
        text: "Strikes twice, and enemies it burns to death explode.",
        mechanics: &[
            M::new(cast("flamestrike"), Mech::Echo { extra_casts: 1 }),
            M::new(kill("flamestrike"), nova(School::Fire, 4.0, 12.0, 1.0)).visual(2136),
        ],
        visual_spell: 2120,
    },
    LegendaryDef {
        ability: "fire_blast",
        name: "Inferno Blast",
        text: "Each blast erupts in a ring of fire; a critical blast readies all your spells.",
        mechanics: &[
            M::new(hit("fire_blast"), nova(School::Fire, 5.0, 12.0, 2.0)).visual(8349),
            M::new(crit("fire_blast"), Mech::ResetCooldowns)
                .visual(11129)
                .cooldown(10.0),
        ],
        visual_spell: 11129,
    },
    LegendaryDef {
        ability: "cone_of_cold",
        name: "Polar Gale",
        text: "Leaves a field of frost on the pack and freezes some of it solid.",
        mechanics: &[
            M::new(cast("cone_of_cold"), ground(School::Frost, 5.0, 5.0, 4.0))
                .at(Aim::Densest { range: 11.0 })
                .visual(10)
                .cooldown(2.0),
            M::new(hit_by("cone_of_cold", 0.3), cc(CcKind::Root, 2.0)).visual(116),
        ],
        visual_spell: 120,
    },
    LegendaryDef {
        ability: "blast_wave",
        name: "Firestorm",
        text: "Hurls enemies far back and leaves a ring of flame around you.",
        mechanics: &[
            M::new(cast("blast_wave"), nova(School::Fire, 8.0, 0.0, 6.0)).at(Aim::Hero),
            M::new(cast("blast_wave"), ground(School::Fire, 6.0, 7.0, 4.0))
                .at(Aim::Hero)
                .visual(2120),
        ],
        visual_spell: 8349,
    },
    LegendaryDef {
        ability: "pyroblast",
        name: "Meteor",
        text: "Crashes down in a stunning blast and leaves the ground molten.",
        mechanics: &[
            M::new(
                hit("pyroblast"),
                nova_cc(School::Fire, 6.0, 24.0, CcKind::Stun, 1.0),
            )
            .visual(24340),
            M::new(hit("pyroblast"), ground(School::Fire, 5.0, 10.0, 5.0)).visual(2120),
        ],
        visual_spell: 24340,
    },
    // --- Warrior ---
    LegendaryDef {
        ability: "cleave",
        name: "Whirling Blades",
        text: "Every few swings you whirl, cutting down everything around you.",
        mechanics: &[
            M::new(cast("cleave"), nova(School::Physical, 6.0, 12.0, 0.0))
                .at(Aim::Hero)
                .visual(1680)
                .cooldown(1.5),
        ],
        visual_spell: 1680,
    },
    LegendaryDef {
        ability: "heroic_strike",
        name: "Shockwave",
        text: "Each strike sends a shockwave through the enemies behind, and crits stun.",
        mechanics: &[
            M::new(hit("heroic_strike"), Mech::Pierce { n: 3 }).visual(6343),
            M::new(crit("heroic_strike"), cc(CcKind::Stun, 1.5)).visual(12809),
        ],
        visual_spell: 12809,
    },
    LegendaryDef {
        ability: "whirlwind",
        name: "Bladestorm",
        text: "You keep spinning: every Whirlwind turns three times.",
        mechanics: &[M::new(cast("whirlwind"), Mech::Echo { extra_casts: 2 })],
        visual_spell: 9632,
    },
    LegendaryDef {
        ability: "thunder_clap",
        name: "Earthshaker",
        text: "An aftershock rolls out wider, and some are stunned where they stand.",
        mechanics: &[
            M::new(
                cast("thunder_clap"),
                nova_cc(School::Nature, 12.0, 6.0, CcKind::Chill(0.5), 3.0),
            )
            .at(Aim::Hero)
            .visual(6343),
            M::new(hit_by("thunder_clap", 0.25), cc(CcKind::Stun, 1.2)).visual(853),
        ],
        visual_spell: 19798,
    },
    LegendaryDef {
        ability: "sweeping_strikes",
        name: "Whirling Steel",
        text: "Two blades circle you, slicing through all they cross.",
        mechanics: &[M::new(
            cast("sweeping_strikes"),
            Mech::Orbit {
                n: 2,
                radius: 4.5,
                duration: 4.0,
            },
        )
        .visual(12292)
        .cooldown(4.0)],
        visual_spell: 12328,
    },
    LegendaryDef {
        ability: "demoralizing_shout",
        name: "Intimidating Roar",
        text: "Sends some fleeing in terror and rallies you: +20% damage for 5 sec.",
        mechanics: &[
            M::new(hit_by("demoralizing_shout", 0.4), cc(CcKind::Fear, 2.5)).visual(1160),
            M::new(
                cast("demoralizing_shout"),
                Mech::Burst {
                    duration: 5.0,
                    bonuses: &[Bonus::new(StatKey::Damage, 0.2)],
                },
            )
            .visual(6673),
        ],
        visual_spell: 5246,
    },
    LegendaryDef {
        ability: "execute",
        name: "Sudden Death",
        text: "A kill swings again at once, and wounded foes take double.",
        mechanics: &[
            M::new(kill("execute"), Mech::Echo { extra_casts: 1 }).cooldown(0.4),
            M::new(
                hit("execute"),
                Mech::Execute {
                    threshold: 0.5,
                    mult: 2.0,
                },
            )
            .visual(12809),
        ],
        visual_spell: 1719,
    },
    LegendaryDef {
        ability: "mortal_strike",
        name: "Colossus Smash",
        text: "Splits into two more blows and leaves a deep, bleeding wound.",
        mechanics: &[
            M::new(
                hit("mortal_strike"),
                Mech::Split {
                    count: 2,
                    spread: 6.0,
                },
            )
            .visual(12294),
            M::new(hit("mortal_strike"), dot(School::Physical, 8.0, 6.0)).visual(772),
        ],
        visual_spell: 772,
    },
    LegendaryDef {
        ability: "retaliation",
        name: "Die by the Sword",
        text: "Every 90 sec, a killing blow leaves you standing; each use raises a shield.",
        mechanics: &[
            M::new(
                Trigger::OnLethal,
                Mech::CheatDeath {
                    immune: 2.0,
                    heal_frac: 0.3,
                },
            )
            .visual(12976)
            .cooldown(90.0),
            M::new(
                cast("retaliation"),
                Mech::Shield {
                    amount: 40.0,
                    duration: 8.0,
                },
            )
            .visual(871),
        ],
        visual_spell: 12976,
    },
    // --- Rogue ---
    LegendaryDef {
        ability: "sinister_strike",
        name: "Crimson Edge",
        text: "Every strike opens a bleeding wound, and critical strikes swing again.",
        mechanics: &[
            M::new(hit("sinister_strike"), dot(School::Physical, 5.0, 6.0)).visual(16511),
            M::new(crit("sinister_strike"), Mech::Echo { extra_casts: 1 }).cooldown(0.5),
        ],
        visual_spell: 16511,
    },
    LegendaryDef {
        ability: "eviscerate",
        name: "Coup de Grace",
        text: "Wounded foes take it three times over, and kills hurl blades at two more.",
        mechanics: &[
            M::new(
                hit("eviscerate"),
                Mech::Execute {
                    threshold: 0.35,
                    mult: 3.0,
                },
            )
            .visual(2098),
            M::new(
                kill("eviscerate"),
                Mech::Split {
                    count: 2,
                    spread: 8.0,
                },
            )
            .visual(2098),
        ],
        visual_spell: 14177,
    },
    LegendaryDef {
        ability: "blade_flurry",
        name: "Blade Dance",
        text: "Three blades keep whirling around you after each flurry.",
        mechanics: &[M::new(
            cast("blade_flurry"),
            Mech::Orbit {
                n: 3,
                radius: 5.0,
                duration: 2.5,
            },
        )
        .visual(13877)
        .cooldown(2.5)],
        visual_spell: 13750,
    },
    LegendaryDef {
        ability: "rupture",
        name: "Crimson Tempest",
        text: "Each rupture tears open the two enemies nearest its victim too.",
        mechanics: &[M::new(hit("rupture"), dot(School::Physical, 5.0, 6.0))
            .at(spread(6.0, 2))
            .visual(1943)],
        visual_spell: 703,
    },
    LegendaryDef {
        ability: "gouge",
        name: "Blinding Powder",
        text: "Gouged enemies, and the two beside them, stagger off blind.",
        mechanics: &[
            M::new(hit("gouge"), cc(CcKind::Fear, 3.0)).visual(1776),
            M::new(hit("gouge"), cc(CcKind::Fear, 3.0))
                .at(spread(5.0, 2))
                .visual(1776),
        ],
        visual_spell: 2094,
    },
    LegendaryDef {
        ability: "kidney_shot",
        name: "Crippling Blow",
        text: "The stun ripples out to everything around the target.",
        mechanics: &[M::new(
            hit("kidney_shot"),
            nova_cc(School::Physical, 5.0, 8.0, CcKind::Stun, 1.5),
        )
        .visual(1833)],
        visual_spell: 1833,
    },
    // --- Priest ---
    LegendaryDef {
        ability: "smite",
        name: "Chastise",
        text: "Holy light leaps on to two more enemies, and kills restore your health.",
        mechanics: &[
            M::new(
                hit("smite"),
                Mech::Chain {
                    jumps: 2,
                    falloff: 0.8,
                },
            )
            .visual(585),
            M::new(
                kill("smite"),
                Mech::HealPulse {
                    radius: 10.0,
                    amount: 4.0,
                },
            )
            .at(Aim::Hero)
            .visual(635)
            .cooldown(1.0),
        ],
        visual_spell: 10060,
    },
    LegendaryDef {
        ability: "shadow_word_pain",
        name: "Vampiric Plague",
        text: "Pain spreads to the two nearest enemies, and its victims feed you.",
        mechanics: &[
            M::new(hit("shadow_word_pain"), dot(School::Shadow, 4.0, 9.0))
                .at(spread(6.0, 2))
                .visual(589),
            M::new(
                hit_by("shadow_word_pain", 0.25),
                Mech::HealPulse {
                    radius: 12.0,
                    amount: 3.0,
                },
            )
            .at(Aim::Hero)
            .visual(15286)
            .cooldown(1.0),
        ],
        visual_spell: 15286,
    },
    LegendaryDef {
        ability: "holy_fire",
        name: "Sacred Flame",
        text: "Sets the ground ablaze with holy fire where it strikes.",
        mechanics: &[
            M::new(hit("holy_fire"), ground(School::Holy, 4.0, 7.0, 4.0))
                .visual(26573)
                .cooldown(1.0),
        ],
        visual_spell: 14914,
    },
    LegendaryDef {
        ability: "mind_blast",
        name: "Mind Shatter",
        text: "Shatters into three lesser blasts, and may send its victim fleeing.",
        mechanics: &[
            M::new(
                hit("mind_blast"),
                Mech::Split {
                    count: 3,
                    spread: 8.0,
                },
            )
            .visual(8092),
            M::new(hit_by("mind_blast", 0.5), cc(CcKind::Fear, 1.5)).visual(5782),
        ],
        visual_spell: 15473,
    },
    LegendaryDef {
        ability: "holy_nova",
        name: "Divine Hymn",
        text: "Rings out twice, and heals your party as it does.",
        mechanics: &[
            M::new(cast("holy_nova"), Mech::Echo { extra_casts: 1 }),
            M::new(
                cast("holy_nova"),
                Mech::HealPulse {
                    radius: 15.0,
                    amount: 5.0,
                },
            )
            .at(Aim::Hero),
        ],
        visual_spell: 724,
    },
    LegendaryDef {
        ability: "psychic_scream",
        name: "Psychic Horror",
        text: "The scream hurls enemies back and leaves their minds bleeding.",
        mechanics: &[
            M::new(cast("psychic_scream"), nova(School::Shadow, 8.0, 10.0, 4.0)).at(Aim::Hero),
            M::new(hit("psychic_scream"), dot(School::Shadow, 5.0, 6.0)).visual(589),
        ],
        visual_spell: 15407,
    },
    LegendaryDef {
        ability: "power_word_shield",
        name: "Guardian Spirit",
        text: "Every 2 min, a killing blow leaves you standing; each shield heals your party.",
        mechanics: &[
            M::new(
                Trigger::OnLethal,
                Mech::CheatDeath {
                    immune: 3.0,
                    heal_frac: 0.4,
                },
            )
            .visual(6346)
            .cooldown(120.0),
            M::new(
                cast("power_word_shield"),
                Mech::HealPulse {
                    radius: 15.0,
                    amount: 10.0,
                },
            )
            .at(Aim::Hero)
            .visual(635),
        ],
        visual_spell: 6346,
    },
    // --- Warlock ---
    LegendaryDef {
        ability: "shadow_bolt",
        name: "Corrupting Bolt",
        text: "Corrupts its victim and spreads Corruption to the three enemies nearest it.",
        mechanics: &[
            M::new(hit("shadow_bolt"), dot(School::Shadow, 4.0, 10.0)).visual(172),
            M::new(hit("shadow_bolt"), dot(School::Shadow, 4.0, 10.0))
                .at(spread(6.0, 3))
                .visual(172),
        ],
        visual_spell: 172,
    },
    LegendaryDef {
        ability: "corruption",
        name: "Seed of Corruption",
        text: "Some victims burst at once, blasting shadow into their neighbours.",
        mechanics: &[M::new(
            hit_by("corruption", 0.3),
            nova(School::Shadow, 5.0, 12.0, 0.0),
        )
        .visual(17877)],
        visual_spell: 18265,
    },
    LegendaryDef {
        ability: "curse_of_agony",
        name: "Curse of Doom",
        text: "Some victims are marked for doom: a second, crushing curse.",
        mechanics: &[M::new(
            hit_by("curse_of_agony", 0.25),
            dot(School::Shadow, 10.0, 6.0),
        )
        .visual(603)],
        visual_spell: 603,
    },
    LegendaryDef {
        ability: "immolate",
        name: "Chaos Bolt",
        text: "Leaves the ground burning, and critical hits detonate in a fiery blast.",
        mechanics: &[
            M::new(hit("immolate"), ground(School::Fire, 3.5, 6.0, 4.0))
                .visual(5740)
                .cooldown(1.0),
            M::new(crit("immolate"), nova(School::Fire, 5.0, 16.0, 1.0)).visual(17962),
        ],
        visual_spell: 17962,
    },
    LegendaryDef {
        ability: "rain_of_fire",
        name: "Hellstorm",
        text: "A second rain of fire follows you wherever you go.",
        mechanics: &[
            M::new(cast("rain_of_fire"), ground(School::Fire, 7.0, 7.0, 6.0))
                .at(Aim::Follow)
                .visual(5740),
        ],
        visual_spell: 1122,
    },
    LegendaryDef {
        ability: "hellfire",
        name: "Infernal Aura",
        text: "Anything that dies in the flames explodes.",
        mechanics: &[M::new(kill("hellfire"), nova(School::Fire, 4.0, 12.0, 1.5)).visual(5857)],
        visual_spell: 706,
    },
    LegendaryDef {
        ability: "death_coil",
        name: "Soul Siphon",
        text: "Bounces through three more enemies, draining each.",
        mechanics: &[M::new(
            hit("death_coil"),
            Mech::Chain {
                jumps: 3,
                falloff: 0.85,
            },
        )
        .visual(6789)],
        visual_spell: 18708,
    },
    LegendaryDef {
        ability: "searing_pain",
        name: "Soul Fire",
        text: "Burns straight through the two enemies behind its target.",
        mechanics: &[M::new(hit("searing_pain"), Mech::Pierce { n: 2 }).visual(5676)],
        visual_spell: 6353,
    },
    LegendaryDef {
        ability: "shadowburn",
        name: "Soul Harvest",
        text: "Kills shatter into shards that seek out three more; the wounded take double.",
        mechanics: &[
            M::new(
                kill("shadowburn"),
                Mech::Split {
                    count: 3,
                    spread: 10.0,
                },
            )
            .visual(17877),
            M::new(
                hit("shadowburn"),
                Mech::Execute {
                    threshold: 0.3,
                    mult: 2.0,
                },
            )
            .visual(17877),
        ],
        visual_spell: 1120,
    },
    // --- Hunter ---
    LegendaryDef {
        ability: "arcane_shot",
        name: "Arcane Lance",
        text: "Lances through up to three enemies behind its target.",
        mechanics: &[M::new(hit("arcane_shot"), Mech::Pierce { n: 3 }).visual(3044)],
        visual_spell: 19506,
    },
    LegendaryDef {
        ability: "multi_shot",
        name: "Chimera Shot",
        text: "Arcane arrows, and every few seconds a volley rains on the thickest pack.",
        mechanics: &[
            M::new(cast("multi_shot"), ground(School::Arcane, 6.0, 8.0, 3.0))
                .at(Aim::Densest { range: 26.0 })
                .visual(1510)
                .cooldown(4.0),
        ],
        visual_spell: 1510,
    },
    LegendaryDef {
        ability: "serpent_sting",
        name: "Wyvern Venom",
        text: "Venom spreads to the two nearest enemies, and some victims fall asleep.",
        mechanics: &[
            M::new(hit("serpent_sting"), dot(School::Nature, 4.0, 9.0))
                .at(spread(6.0, 2))
                .visual(1978),
            M::new(hit_by("serpent_sting", 0.2), cc(CcKind::Stun, 2.0)).visual(24131),
        ],
        visual_spell: 19386,
    },
    LegendaryDef {
        ability: "volley",
        name: "Arrow Storm",
        text: "Rains twice, pinning some of what it strikes.",
        mechanics: &[
            M::new(cast("volley"), Mech::Echo { extra_casts: 1 }),
            M::new(hit_by("volley", 0.08), cc(CcKind::Root, 1.5)).visual(1510),
        ],
        visual_spell: 3045,
    },
    LegendaryDef {
        ability: "explosive_trap",
        name: "Immolation Trap",
        text: "Leaves the ground burning where it goes off.",
        mechanics: &[
            M::new(hit("explosive_trap"), ground(School::Fire, 5.0, 7.0, 5.0))
                .visual(2120)
                .cooldown(1.0),
        ],
        visual_spell: 13795,
    },
    LegendaryDef {
        ability: "concussive_shot",
        name: "Binding Shot",
        text: "Ricochets through two more enemies, and may daze its victim.",
        mechanics: &[
            M::new(
                hit("concussive_shot"),
                Mech::Chain {
                    jumps: 2,
                    falloff: 0.8,
                },
            )
            .visual(5116),
            M::new(hit_by("concussive_shot", 0.3), cc(CcKind::Stun, 1.0)).visual(19503),
        ],
        visual_spell: 19503,
    },
    LegendaryDef {
        ability: "aimed_shot",
        name: "Kill Shot",
        text: "Punches through four behind, and wounded targets take it three times over.",
        mechanics: &[
            M::new(hit("aimed_shot"), Mech::Pierce { n: 4 }).visual(19434),
            M::new(
                hit("aimed_shot"),
                Mech::Execute {
                    threshold: 0.3,
                    mult: 3.0,
                },
            )
            .visual(19434),
        ],
        visual_spell: 19574,
    },
    // --- Shaman ---
    LegendaryDef {
        ability: "lightning_bolt",
        name: "Elemental Overload",
        text: "Bolts may strike twice, and critical bolts fork to two more enemies.",
        mechanics: &[
            M::new(
                hit_by("lightning_bolt", 0.35),
                Mech::Echo { extra_casts: 1 },
            )
            .cooldown(0.6),
            M::new(
                crit("lightning_bolt"),
                Mech::Chain {
                    jumps: 2,
                    falloff: 0.75,
                },
            )
            .visual(421),
        ],
        visual_spell: 16166,
    },
    LegendaryDef {
        ability: "chain_lightning",
        name: "Storm Lash",
        text: "Every link sparks one more jump, and some of them stun.",
        mechanics: &[
            M::new(
                hit("chain_lightning"),
                Mech::Chain {
                    jumps: 1,
                    falloff: 0.8,
                },
            )
            .visual(421),
            M::new(hit_by("chain_lightning", 0.25), cc(CcKind::Stun, 1.2)).visual(853),
        ],
        visual_spell: 17364,
    },
    LegendaryDef {
        ability: "earth_shock",
        name: "Earthquake",
        text: "Each shock cracks the ground, briefly stunning everything around.",
        mechanics: &[M::new(
            hit("earth_shock"),
            nova_cc(School::Nature, 5.0, 10.0, CcKind::Stun, 0.8),
        )
        .visual(8042)],
        visual_spell: 19798,
    },
    LegendaryDef {
        ability: "flame_shock",
        name: "Lava Burst",
        text: "Leaves pools of lava where it strikes, and critical shocks erupt.",
        mechanics: &[
            M::new(hit("flame_shock"), ground(School::Fire, 3.5, 7.0, 4.0))
                .visual(5740)
                .cooldown(1.0),
            M::new(crit("flame_shock"), nova(School::Fire, 4.0, 14.0, 1.0)).visual(8349),
        ],
        visual_spell: 8050,
    },
    LegendaryDef {
        ability: "frost_shock",
        name: "Glacial Shock",
        text: "Splits into two icy shards and may freeze its victim solid.",
        mechanics: &[
            M::new(
                hit("frost_shock"),
                Mech::Split {
                    count: 2,
                    spread: 8.0,
                },
            )
            .visual(8056),
            M::new(hit_by("frost_shock", 0.3), cc(CcKind::Root, 2.0)).visual(8056),
        ],
        visual_spell: 8056,
    },
    LegendaryDef {
        ability: "searing_totem",
        name: "Totemic Fury",
        text: "The totem's bolts split on impact into two more.",
        mechanics: &[M::new(
            hit("searing_totem"),
            Mech::Split {
                count: 2,
                spread: 6.0,
            },
        )
        .visual(3606)],
        visual_spell: 1535,
    },
    LegendaryDef {
        ability: "magma_totem",
        name: "Molten Core",
        text: "Its pulses leave pools of lava and set some enemies burning.",
        mechanics: &[
            M::new(hit("magma_totem"), ground(School::Fire, 4.0, 6.0, 4.0))
                .visual(5740)
                .cooldown(2.0),
            M::new(hit_by("magma_totem", 0.3), dot(School::Fire, 4.0, 4.0)).visual(348),
        ],
        visual_spell: 8187,
    },
    LegendaryDef {
        ability: "lightning_shield",
        name: "Static Charge",
        text: "Each zap arcs to two more, and falling low unleashes a stunning discharge.",
        mechanics: &[
            M::new(
                hit("lightning_shield"),
                Mech::Chain {
                    jumps: 2,
                    falloff: 0.75,
                },
            )
            .visual(403),
            M::new(
                Trigger::OnLowHealth { threshold: 0.35 },
                Mech::Nova {
                    school: School::Nature,
                    radius: 8.0,
                    damage: 20.0,
                    knockback: 6.0,
                    cc: Some(Cc {
                        kind: CcKind::Stun,
                        secs: 1.5,
                    }),
                },
            )
            .at(Aim::Hero)
            .visual(421)
            .cooldown(20.0),
        ],
        visual_spell: 324,
    },
    // --- Paladin ---
    LegendaryDef {
        ability: "hammer_of_wrath",
        name: "Avenger's Hammer",
        text: "Hammers fly back to you, striking everything on the way.",
        mechanics: &[M::new(hit("hammer_of_wrath"), Mech::Boomerang).visual(24275)],
        visual_spell: 1022,
    },
    LegendaryDef {
        ability: "consecration",
        name: "Hallowed Ground",
        text: "A second consecration moves with you, burning wherever you walk.",
        mechanics: &[
            M::new(cast("consecration"), ground(School::Holy, 7.0, 6.0, 8.0))
                .at(Aim::Follow)
                .visual(26573),
        ],
        visual_spell: 26573,
    },
    LegendaryDef {
        ability: "holy_shock",
        name: "Radiant Shock",
        text: "Arcs on to another enemy, and every shock heals your party.",
        mechanics: &[
            M::new(
                hit("holy_shock"),
                Mech::Chain {
                    jumps: 1,
                    falloff: 0.8,
                },
            )
            .visual(25912),
            M::new(
                cast("holy_shock"),
                Mech::HealPulse {
                    radius: 15.0,
                    amount: 6.0,
                },
            )
            .at(Aim::Hero)
            .visual(635),
        ],
        visual_spell: 635,
    },
    LegendaryDef {
        ability: "exorcism",
        name: "Purifying Light",
        text: "Detonates in a holy blast that stuns everything near the target.",
        mechanics: &[M::new(
            hit("exorcism"),
            nova_cc(School::Holy, 6.0, 16.0, CcKind::Stun, 1.0),
        )
        .visual(2812)],
        visual_spell: 2878,
    },
    LegendaryDef {
        ability: "holy_wrath",
        name: "Avenging Wrath",
        text: "Each wrath fills you with fury, +25% damage for 5 sec, and may stun.",
        mechanics: &[
            M::new(
                cast("holy_wrath"),
                Mech::Burst {
                    duration: 5.0,
                    bonuses: &[Bonus::new(StatKey::Damage, 0.25)],
                },
            )
            .visual(20216),
            M::new(hit_by("holy_wrath", 0.15), cc(CcKind::Stun, 1.0)).visual(853),
        ],
        visual_spell: 20216,
    },
    LegendaryDef {
        ability: "hammer_of_justice",
        name: "Righteous Hammers",
        text: "Splits into three more hammers, each one stunning.",
        mechanics: &[M::new(
            hit("hammer_of_justice"),
            Mech::Split {
                count: 3,
                spread: 8.0,
            },
        )
        .visual(853)],
        visual_spell: 853,
    },
    LegendaryDef {
        ability: "divine_shield",
        name: "Divine Intervention",
        text: "Every 90 sec, a killing blow leaves you standing; each shield heals you.",
        mechanics: &[
            M::new(
                Trigger::OnLethal,
                Mech::CheatDeath {
                    immune: 3.0,
                    heal_frac: 0.5,
                },
            )
            .visual(642)
            .cooldown(90.0),
            M::new(
                cast("divine_shield"),
                Mech::HealPulse {
                    radius: 12.0,
                    amount: 25.0,
                },
            )
            .at(Aim::Hero)
            .visual(635),
        ],
        visual_spell: 19752,
    },
    LegendaryDef {
        ability: "judgement",
        name: "Judgement of Command",
        text: "Arcs on to two more enemies, and wounded foes take double.",
        mechanics: &[
            M::new(
                hit("judgement"),
                Mech::Chain {
                    jumps: 2,
                    falloff: 0.85,
                },
            )
            .visual(20271),
            M::new(
                hit("judgement"),
                Mech::Execute {
                    threshold: 0.3,
                    mult: 2.0,
                },
            )
            .visual(20271),
        ],
        visual_spell: 20375,
    },
    // --- Druid ---
    LegendaryDef {
        ability: "wrath",
        name: "Wrath of Cenarius",
        text: "Bursts into three motes of nature on impact.",
        mechanics: &[M::new(
            hit("wrath"),
            Mech::Split {
                count: 3,
                spread: 8.0,
            },
        )
        .visual(5176)],
        visual_spell: 29166,
    },
    LegendaryDef {
        ability: "moonfire",
        name: "Starfall",
        text: "Stars rain down around you wherever you go, and moonfire calls more down.",
        mechanics: &[
            M::new(
                Trigger::Periodic { every: 8.0 },
                ground(School::Arcane, 8.0, 8.0, 7.0),
            )
            .at(Aim::Follow)
            .visual(20687),
            M::new(hit_by("moonfire", 0.3), nova(School::Arcane, 3.0, 8.0, 0.0))
                .at(Aim::Random { range: 14.0 })
                .visual(2912),
        ],
        visual_spell: 20687,
    },
    LegendaryDef {
        ability: "starfire",
        name: "Eclipse",
        text: "Explodes in a stunning starburst; critical hits empower you, +30% damage for 6 sec.",
        mechanics: &[
            M::new(
                hit("starfire"),
                Mech::Nova {
                    school: School::Arcane,
                    radius: 5.0,
                    damage: 14.0,
                    knockback: 2.0,
                    cc: Some(Cc {
                        kind: CcKind::Stun,
                        secs: 1.0,
                    }),
                },
            )
            .visual(2912),
            M::new(
                crit("starfire"),
                Mech::Burst {
                    duration: 6.0,
                    bonuses: &[Bonus::new(StatKey::Damage, 0.3)],
                },
            )
            .visual(24858),
        ],
        visual_spell: 24858,
    },
    LegendaryDef {
        ability: "hurricane",
        name: "Tempest",
        text: "Whirls up a second storm, and its gusts may stun what they catch.",
        mechanics: &[
            M::new(cast("hurricane"), Mech::Echo { extra_casts: 1 }),
            M::new(hit_by("hurricane", 0.1), cc(CcKind::Stun, 1.0)).visual(16914),
        ],
        visual_spell: 16914,
    },
    LegendaryDef {
        ability: "insect_swarm",
        name: "Locust Swarm",
        text: "Each swarm spreads to the three enemies nearest its victim.",
        mechanics: &[M::new(hit("insect_swarm"), dot(School::Nature, 4.0, 10.0))
            .at(spread(6.0, 3))
            .visual(5570)],
        visual_spell: 5570,
    },
    LegendaryDef {
        ability: "entangling_roots",
        name: "Earthen Grasp",
        text: "Roots spread to the two beside each victim and crush all they hold.",
        mechanics: &[
            M::new(hit("entangling_roots"), cc(CcKind::Root, 3.0))
                .at(spread(5.0, 2))
                .visual(16689),
            M::new(hit("entangling_roots"), dot(School::Nature, 5.0, 5.0)).visual(16689),
        ],
        visual_spell: 16689,
    },
    LegendaryDef {
        ability: "thorns",
        name: "Barkskin",
        text: "Thorns also shields you, and falling low hardens your skin.",
        mechanics: &[
            M::new(
                cast("thorns"),
                Mech::Shield {
                    amount: 30.0,
                    duration: 15.0,
                },
            )
            .visual(22812),
            M::new(
                Trigger::OnLowHealth { threshold: 0.4 },
                Mech::Burst {
                    duration: 6.0,
                    bonuses: &[Bonus::new(StatKey::Armor, 0.3)],
                },
            )
            .visual(20655)
            .cooldown(20.0),
        ],
        visual_spell: 22812,
    },
    LegendaryDef {
        ability: "rejuvenation",
        name: "Tranquility",
        text: "Heals your whole party, and falling low calls a great surge of healing.",
        mechanics: &[
            M::new(
                cast("rejuvenation"),
                Mech::HealPulse {
                    radius: 20.0,
                    amount: 15.0,
                },
            )
            .at(Aim::Hero)
            .visual(774),
            M::new(
                Trigger::OnLowHealth { threshold: 0.3 },
                Mech::HealPulse {
                    radius: 20.0,
                    amount: 40.0,
                },
            )
            .at(Aim::Hero)
            .visual(740)
            .cooldown(45.0),
        ],
        visual_spell: 740,
    },
    // --- Spec signatures ---
    LegendaryDef {
        ability: "bloodthirst",
        name: "Rampage",
        text: "Critical strikes swing again, and every kill drives you on to more damage.",
        mechanics: &[
            M::new(crit("bloodthirst"), Mech::Echo { extra_casts: 1 }).cooldown(0.5),
            M::new(
                kill("bloodthirst"),
                Mech::Burst {
                    duration: 6.0,
                    bonuses: &[Bonus::new(StatKey::Damage, 0.15)],
                },
            )
            .visual(23881)
            .cooldown(1.0),
        ],
        visual_spell: 23881,
    },
    LegendaryDef {
        ability: "shield_slam",
        name: "Devastate",
        text: "Each slam splits into two more and raises a shield that soaks the blows.",
        mechanics: &[
            M::new(
                hit("shield_slam"),
                Mech::Split {
                    count: 2,
                    spread: 6.0,
                },
            )
            .visual(23922),
            M::new(
                cast("shield_slam"),
                Mech::Shield {
                    amount: 20.0,
                    duration: 6.0,
                },
            )
            .visual(23922)
            .cooldown(6.0),
        ],
        visual_spell: 23922,
    },
    LegendaryDef {
        ability: "crusader_strike",
        name: "Divine Storm",
        text: "Each strike leaps on to two more foes and heals you as it lands.",
        mechanics: &[
            M::new(
                hit("crusader_strike"),
                Mech::Chain {
                    jumps: 2,
                    falloff: 0.8,
                },
            )
            .visual(2537),
            M::new(
                cast("crusader_strike"),
                Mech::HealPulse {
                    radius: 2.0,
                    amount: 5.0,
                },
            )
            .at(Aim::Hero)
            .visual(2537)
            .cooldown(1.0),
        ],
        visual_spell: 2537,
    },
    LegendaryDef {
        ability: "backstab",
        name: "Mutilate",
        text: "Every stab poisons its victim, and critical stabs strike twice.",
        mechanics: &[
            M::new(hit("backstab"), dot(School::Nature, 5.0, 6.0)).visual(53),
            M::new(crit("backstab"), Mech::Echo { extra_casts: 1 }).cooldown(0.5),
        ],
        visual_spell: 53,
    },
    LegendaryDef {
        ability: "ambush",
        name: "Shadow Dance",
        text: "Kills from the shadows make every ability ready at once.",
        mechanics: &[
            M::new(kill_by("ambush", 0.5), Mech::ResetCooldowns)
                .visual(8676)
                .cooldown(8.0),
            M::new(
                hit("ambush"),
                Mech::Split {
                    count: 2,
                    spread: 6.0,
                },
            )
            .visual(8676),
        ],
        visual_spell: 8676,
    },
    LegendaryDef {
        ability: "mind_flay",
        name: "Mind Sear",
        text: "Flays two more minds at once, and its victims may flee in terror.",
        mechanics: &[
            M::new(
                hit("mind_flay"),
                Mech::Split {
                    count: 2,
                    spread: 8.0,
                },
            )
            .visual(8092),
            M::new(hit_by("mind_flay", 0.15), cc(CcKind::Fear, 1.5)).visual(5782),
        ],
        visual_spell: 15407,
    },
    LegendaryDef {
        ability: "stormstrike",
        name: "Maelstrom",
        text: "Every strike calls lightning that leaps through three more foes.",
        mechanics: &[M::new(
            hit("stormstrike"),
            Mech::Chain {
                jumps: 3,
                falloff: 0.7,
            },
        )
        .visual(421)],
        visual_spell: 17364,
    },
    LegendaryDef {
        ability: "shred",
        name: "Mangle",
        text: "Shreds tear at two more foes and leave them bleeding.",
        mechanics: &[
            M::new(
                hit("shred"),
                Mech::Split {
                    count: 2,
                    spread: 6.0,
                },
            )
            .visual(5221),
            M::new(hit("shred"), dot(School::Physical, 6.0, 6.0)).visual(1822),
        ],
        visual_spell: 5221,
    },
];

pub(crate) const MAX_ABILITY_LEVEL: u8 = 8;
pub(crate) const MAX_PASSIVE_LEVEL: u8 = 5;
pub(crate) const MAX_ABILITIES: usize = 6;
pub(crate) const MAX_PASSIVES: usize = 6;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Stat {
    Might,
    MaxHp,
    Speed,
    Regen,
    Haste,
    Area,
    Magnet,
    Growth,
    Armor,
    Crit,
}

pub(crate) struct PassiveDef {
    pub key: &'static str,
    pub name: &'static str,
    /// Icon source, and a state visual on the hero where the spell has one.
    pub spell: u32,
    pub stat: Stat,
    pub per_level: f32,
}

pub(crate) static PASSIVES: &[PassiveDef] = &[
    PassiveDef {
        key: "might",
        name: "Blessing of Might",
        spell: 19740,
        stat: Stat::Might,
        per_level: 0.10,
    },
    PassiveDef {
        key: "fortitude",
        name: "Power Word: Fortitude",
        spell: 1243,
        stat: Stat::MaxHp,
        per_level: 0.20,
    },
    PassiveDef {
        key: "pack",
        name: "Aspect of the Pack",
        spell: 13159,
        stat: Stat::Speed,
        per_level: 0.10,
    },
    PassiveDef {
        key: "renew",
        name: "Renew",
        spell: 139,
        stat: Stat::Regen,
        per_level: 0.5,
    },
    PassiveDef {
        key: "intellect",
        name: "Arcane Intellect",
        spell: 1459,
        stat: Stat::Haste,
        per_level: 0.08,
    },
    PassiveDef {
        key: "amplify",
        name: "Amplify Magic",
        spell: 1008,
        stat: Stat::Area,
        per_level: 0.10,
    },
    PassiveDef {
        key: "detect",
        name: "Detect Lesser Invisibility",
        spell: 132,
        stat: Stat::Magnet,
        per_level: 0.40,
    },
    PassiveDef {
        key: "wisdom",
        name: "Blessing of Wisdom",
        spell: 19742,
        stat: Stat::Growth,
        per_level: 0.08,
    },
    PassiveDef {
        key: "wild",
        name: "Mark of the Wild",
        spell: 1126,
        stat: Stat::Armor,
        per_level: 1.0,
    },
    PassiveDef {
        key: "kings",
        name: "Blessing of Kings",
        spell: 20217,
        stat: Stat::Crit,
        per_level: 0.05,
    },
];

pub(crate) static CLASSES: &[ClassDef] = &[
    ClassDef {
        id: 1,
        name: "Warrior",
        color: 0xC79C6E,
        power: 1,
        start: "cleave",
        pool: &[
            "cleave",
            "heroic_strike",
            "whirlwind",
            "thunder_clap",
            "sweeping_strikes",
            "demoralizing_shout",
            "execute",
            "mortal_strike",
            "retaliation",
            "bloodthirst",
            "shield_slam",
        ],
        looks: &[
            1300, 1374, 1375, 1504, 1578, 1599, 1707, 1721, 1880, 2096, 2103, 2113, 2196, 2198,
            2614, 2620, 2658, 3053, 3054, 3055, 3280, 3287, 3343, 3399, 3431, 3743, 3793, 3794,
            4242, 4556, 6071, 11037,
        ],
        weapons: &[(0, 7483, 2, 7, 1, 13, 3), (1, 1706, 4, 6, 1, 14, 4)],
        sheath: 1,
    },
    ClassDef {
        id: 2,
        name: "Paladin",
        color: 0xF58CBA,
        power: 0,
        start: "hammer_of_wrath",
        pool: &[
            "hammer_of_wrath",
            "consecration",
            "holy_shock",
            "exorcism",
            "holy_wrath",
            "hammer_of_justice",
            "divine_shield",
            "judgement",
            "crusader_strike",
        ],
        looks: &[
            1299, 1499, 1622, 3087, 3088, 3089, 3284, 3289, 3346, 3393, 7356,
        ],
        weapons: &[(0, 8011, 2, 4, 2, 13, 3), (1, 1705, 4, 6, 1, 14, 4)],
        sheath: 1,
    },
    ClassDef {
        id: 3,
        name: "Hunter",
        color: 0xABD473,
        power: 0,
        start: "arcane_shot",
        pool: &[
            "arcane_shot",
            "multi_shot",
            "serpent_sting",
            "volley",
            "explosive_trap",
            "concussive_shot",
            "aimed_shot",
        ],
        looks: &[
            1373, 1703, 1723, 1882, 2066, 2087, 2105, 2112, 2205, 2206, 2251, 3056, 3072, 3073,
            3299, 3309, 3310, 3312, 3395, 3558, 3744, 3810, 3811, 4239, 4241, 4372, 4560, 7538,
            10245,
        ],
        weapons: &[(0, 7480, 2, 6, 1, 17, 2), (2, 6235, 2, 2, 2, 15, 0)],
        sheath: 2,
    },
    ClassDef {
        id: 4,
        name: "Rogue",
        color: 0xFFF569,
        power: 3,
        start: "sinister_strike",
        pool: &[
            "sinister_strike",
            "eviscerate",
            "blade_flurry",
            "rupture",
            "gouge",
            "kidney_shot",
            "backstab",
            "ambush",
        ],
        looks: &[
            1297, 1327, 1328, 1507, 1580, 1603, 1704, 1725, 1886, 2231, 2243, 2252, 2631, 2639,
            2659, 3100, 3101, 3113, 3351, 3407, 3436, 3749, 4360, 5146, 5528, 13171,
        ],
        weapons: &[(0, 7492, 2, 7, 1, 13, 3), (1, 6443, 2, 15, 1, 13, 3)],
        sheath: 1,
    },
    ClassDef {
        id: 5,
        name: "Priest",
        color: 0xFFFFFF,
        power: 0,
        start: "smite",
        pool: &[
            "smite",
            "shadow_word_pain",
            "holy_fire",
            "mind_blast",
            "holy_nova",
            "psychic_scream",
            "power_word_shield",
            "mind_flay",
        ],
        looks: &[
            1295, 1495, 1579, 1602, 1708, 1733, 1897, 2137, 2138, 2139, 2200, 2201, 2202, 2618,
            2626, 3066, 3085, 3086, 3282, 3283, 3344, 3401, 3429, 4068, 4690, 4711, 10473, 10723,
            11044, 11048, 11053,
        ],
        weapons: &[(0, 1926, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
    ClassDef {
        id: 7,
        name: "Shaman",
        color: 0x0070DE,
        power: 0,
        start: "lightning_bolt",
        pool: &[
            "lightning_bolt",
            "chain_lightning",
            "earth_shock",
            "flame_shock",
            "frost_shock",
            "searing_totem",
            "magma_totem",
            "lightning_shield",
            "stormstrike",
        ],
        looks: &[
            1360, 1878, 2082, 2102, 2123, 3746, 3816, 4231, 4552, 10180, 13341,
        ],
        weapons: &[(0, 7477, 2, 4, 2, 13, 3), (1, 1705, 4, 6, 1, 14, 4)],
        sheath: 1,
    },
    ClassDef {
        id: 8,
        name: "Mage",
        color: 0x69CCF0,
        power: 0,
        start: "fireball",
        pool: &[
            "fireball",
            "frostbolt",
            "arcane_missiles",
            "frost_nova",
            "arcane_explosion",
            "blizzard",
            "flamestrike",
            "fire_blast",
            "cone_of_cold",
            "blast_wave",
            "pyroblast",
        ],
        looks: &[
            1294, 1470, 1484, 1592, 1600, 2134, 2135, 2644, 2657, 2810, 3108, 3109, 3292, 3293,
            4522, 4523, 4524, 4526, 4665, 5001, 6058, 6060, 6072, 7669, 10171, 10214, 10215, 10216,
            10548, 10733, 12849,
        ],
        weapons: &[(0, 1927, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
    ClassDef {
        id: 9,
        name: "Warlock",
        color: 0x9482C9,
        power: 0,
        start: "shadow_bolt",
        pool: &[
            "shadow_bolt",
            "corruption",
            "curse_of_agony",
            "immolate",
            "rain_of_fire",
            "hellfire",
            "death_coil",
            "searing_pain",
            "shadowburn",
        ],
        looks: &[
            1324, 1325, 1326, 1469, 1581, 1604, 1884, 1930, 2637, 2646, 2675, 3115, 3116, 3122,
            3271, 3286, 3291, 3345, 3607, 3745, 4567,
        ],
        weapons: &[(0, 5542, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
    ClassDef {
        id: 11,
        name: "Druid",
        color: 0xFF7D0A,
        power: 0,
        start: "wrath",
        pool: &[
            "wrath",
            "moonfire",
            "starfire",
            "hurricane",
            "insect_swarm",
            "entangling_roots",
            "thorns",
            "rejuvenation",
            "shred",
        ],
        looks: &[
            1706, 1732, 2106, 2115, 2121, 2250, 2255, 2261, 3300, 3301, 3302, 3819, 3820, 7357,
            10738, 12053,
        ],
        weapons: &[(0, 22391, 2, 10, 2, 17, 2)],
        sheath: 1,
    },
];

#[derive(Clone, Copy)]
pub(crate) struct EnemyDef {
    pub name: &'static str,
    pub display: u32,
    pub hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub scale: f32,
    pub xp: u32,
    pub level: u32,
    /// A great beast, which takes more room in the crowd.
    pub big: bool,
}

const fn en(
    name: &'static str,
    display: u32,
    hp: f32,
    speed: f32,
    damage: f32,
    xp: u32,
    level: u32,
) -> EnemyDef {
    EnemyDef {
        name,
        display,
        hp,
        speed,
        damage,
        scale: 1.0,
        xp,
        level,
        big: false,
    }
}

/// The Barrens, roughly in the order a character meets them.
pub(crate) static ENEMIES: &[EnemyDef] = &[
    en("Fleeting Plainstrider", 1284, 8.0, 3.6, 2.0, 1, 10),
    en("Zhevra Runner", 6087, 10.0, 4.4, 2.0, 1, 11),
    en("Razormane Quilboar", 1218, 16.0, 3.4, 3.0, 2, 12),
    en("Razormane Hunter", 6094, 18.0, 3.6, 4.0, 2, 12),
    en("Savannah Huntress", 1056, 22.0, 4.8, 8.0, 2, 13),
    en("Sunscale Lashtail", 1744, 26.0, 4.6, 8.0, 3, 13),
    en("Kolkar Wrangler", 9442, 36.0, 4.0, 10.0, 3, 14),
    en("Kolkar Stormer", 9443, 40.0, 4.0, 11.0, 3, 14),
    en("Witchwing Harpy", 3218, 34.0, 5.0, 10.0, 3, 15),
    en("Hecklefang Hyena", 2710, 44.0, 5.2, 12.0, 4, 15),
    en("Razormane Defender", 1253, 70.0, 3.3, 14.0, 5, 16),
    en("Razormane Mystic", 4643, 55.0, 3.8, 12.0, 4, 16),
    en("Sunscale Screecher", 1747, 60.0, 5.0, 13.0, 4, 17),
    en("Kolkar Bloodcharger", 9447, 85.0, 4.6, 16.0, 5, 18),
    en("Savannah Prowler", 1973, 75.0, 5.4, 15.0, 5, 18),
    en("Thunderhawk Hatchling", 1742, 90.0, 4.2, 16.0, 6, 19),
    en("Witchwing Slayer", 2163, 95.0, 5.0, 18.0, 6, 20),
    en("Bristleback Geomancer", 6091, 120.0, 3.6, 20.0, 7, 21),
    huge(en("Stormsnout", 1537, 150.0, 4.0, 22.0, 8, 22)),
    huge(en("Barrens Kodo", 1453, 220.0, 3.2, 28.0, 12, 23)),
    en("Sunscale Scytheclaw", 4442, 160.0, 5.6, 24.0, 8, 24),
    huge(en("Thunderhead", 1538, 180.0, 4.2, 26.0, 10, 25)),
    en("Silithid Swarmer", 2731, 90.0, 6.0, 18.0, 5, 25),
    huge(en("Greater Thunderhawk", 1974, 240.0, 4.4, 30.0, 12, 26)),
    en("Hecklefang Stalker", 2712, 200.0, 6.0, 30.0, 10, 27),
    en("Kolkar Destroyer", 9446, 280.0, 4.6, 34.0, 14, 28),
    en("Kolkar Mauler", 4874, 300.0, 4.6, 36.0, 14, 29),
    huge(en("Greater Barrens Kodo", 1232, 420.0, 3.4, 42.0, 24, 30)),
    huge(en("Elder Thunder Lizard", 2764, 380.0, 4.4, 44.0, 22, 31)),
    huge(en("Silithid Protector", 1307, 360.0, 5.0, 40.0, 20, 32)),
];

/// The great beasts at three quarters size, so a crowd of them still leaves the field readable.
const fn huge(def: EnemyDef) -> EnemyDef {
    EnemyDef {
        scale: 0.75,
        big: true,
        ..def
    }
}

pub(crate) static BOSSES: &[EnemyDef] = &[
    EnemyDef {
        scale: 1.6,
        ..en("Echeyakee", 1934, 700.0, 5.2, 20.0, 60, 16)
    },
    EnemyDef {
        scale: 1.6,
        ..en("Takk the Leaper", 1337, 1800.0, 5.4, 32.0, 120, 19)
    },
    EnemyDef {
        scale: 1.7,
        ..en("Lakota'mani", 1241, 4200.0, 4.0, 45.0, 200, 22)
    },
    EnemyDef {
        scale: 1.7,
        ..en("Humar the Pridelord", 4424, 7000.0, 5.6, 55.0, 300, 26)
    },
    EnemyDef {
        scale: 1.8,
        ..en("Agathelos the Raging", 2450, 12000.0, 5.0, 70.0, 400, 33)
    },
    EnemyDef {
        scale: 1.8,
        ..en("Hezrul Bloodmark", 9448, 20000.0, 4.8, 90.0, 500, 33)
    },
];

/// Survive this long to win.
pub(crate) const VICTORY_SECS: f32 = 900.0;
/// A boss every two and a quarter minutes, the last of six at 13:30, a fight before the bell.
pub(crate) const BOSS_EVERY_SECS: f32 = 135.0;
/// On a raid stage, the last roster boss walks out this long in, after the trash.
pub(crate) const RAID_BOSS_SECS: f32 = 360.0;

/// The roster boss `map` sends out next at `elapsed`, given `out` roster places already used
/// (spawned or passed over), on a roster `count` long. One every [`BOSS_EVERY_SECS`], in order; on
/// a raid stage the bosses before the last keep that beat only until [`RAID_BOSS_SECS`], when the
/// last walks out and any not yet out are passed over.
pub(crate) fn next_boss(map: &MapDef, elapsed: f32, out: u32, count: usize) -> Option<usize> {
    let out = out as usize;
    let on_clock = (elapsed / BOSS_EVERY_SECS) as usize;
    if !map.raid {
        return (out < on_clock.min(count)).then_some(out);
    }
    let last = count.checked_sub(1)?;
    if elapsed >= RAID_BOSS_SECS {
        return (out <= last).then_some(last);
    }
    (out < on_clock.min(last)).then_some(out)
}

/// Whether the run is won: on a raid stage once its last boss has fallen ([`Run::final_boss`]),
/// elsewhere once the clock reaches [`VICTORY_SECS`].
///
/// [`Run::final_boss`]: super::Run::final_boss
pub(crate) fn won(map: &MapDef, elapsed: f32, final_boss_down: bool) -> bool {
    if map.raid {
        final_boss_down
    } else {
        elapsed >= VICTORY_SECS
    }
}

/// Experience per gem: every level of the curve comes at the run's pace.
pub(crate) const XP_RATE: f32 = 2.0;

/// How far through the run, 0 at the start and 1 at [`VICTORY_SECS`].
pub(crate) fn progress(elapsed: f32) -> f32 {
    (elapsed / VICTORY_SECS).max(0.0)
}

/// The pool of roster indices at a point in the run, from a roster `tiers` long.
pub(crate) fn wave_pool(elapsed: f32, tiers: usize) -> [usize; 3] {
    // The roster spread over thirty steps of the run, the last three from step 28: on The
    // Barrens' thirty tiers, one newer every step.
    let step = (progress(elapsed) * 30.0) as usize;
    let span = tiers.saturating_sub(3);
    let lo = (step.saturating_sub(1) * span / 27).min(span);
    [lo, lo + 1, lo + 2]
}

/// How many enemies the spawner keeps alive at a point in the run.
pub(crate) fn target_alive(elapsed: f32) -> usize {
    ((14.0 + progress(elapsed) * 225.0) as usize).min(220)
}

/// Enemy health grows on top of the tiers, so a late kill takes several spells.
pub(crate) fn hp_scale(elapsed: f32) -> f32 {
    1.0 + progress(elapsed) * 0.75
}

/// Enemy damage grows the same way.
pub(crate) fn damage_scale(elapsed: f32) -> f32 {
    1.0 + progress(elapsed) * 0.9
}

/// The experience to go from `level` to the next: cheap early, then a steady climb that leaves a
/// normal-density run time to play between hands while rarity carries a build to its top ranks.
pub(crate) fn xp_to_next(level: u32) -> f32 {
    let l = level as f32;
    if level < 11 {
        10.0 + (l - 1.0) * 17.0
    } else if level < 25 {
        147.0 - (l - 10.0)
    } else if level < 40 {
        let middle = l - 25.0;
        133.0 + middle * 32.0 - middle * middle * 0.84
    } else {
        let late = l - 40.0;
        425.0 + late * 9.0 + late * late * 6.6
    }
}

/// A normal-density run's expected experience at `seconds`: five turns of the opening live horde
/// per minute from the replenisher, plus its scheduled elites and bosses. The pool and every XP
/// value come from the default roster, so the pacing test follows the live spawn table rather than
/// a hand-made curve.
#[cfg(test)]
fn normal_run_xp(seconds: f32, party_size: u8) -> f32 {
    const TARGET_REFRESHES_PER_MINUTE: f32 = 5.0;
    const ELITES_PER_MINUTE: f32 = 60.0 / 75.0;
    let roster = roster(0);
    let horde_kills_per_minute =
        super::party::crowd(target_alive(0.0), party_size) as f32 * TARGET_REFRESHES_PER_MINUTE;
    let horde_xp_scale = 1.0 / super::party::scale(party_size);
    let whole_minutes = (seconds / 60.0).floor() as u32;
    let partial = (seconds / 60.0).fract();
    let minute_xp = |minute: u32| {
        let elapsed = (minute as f32 + 0.5) * 60.0;
        let pool = wave_pool(elapsed, roster.enemies.len());
        let horde_xp = pool
            .iter()
            .map(|&i| roster.enemies[i].xp as f32)
            .sum::<f32>()
            / pool.len() as f32;
        (horde_kills_per_minute * horde_xp_scale + ELITES_PER_MINUTE) * horde_xp * XP_RATE
    };
    let horde = (0..whole_minutes).map(minute_xp).sum::<f32>() + minute_xp(whole_minutes) * partial;
    let bosses = roster
        .bosses
        .iter()
        .enumerate()
        .filter(|(i, _)| (*i as f32 + 1.0) * BOSS_EVERY_SECS <= seconds)
        .map(|(_, boss)| boss.xp as f32 * XP_RATE)
        .sum::<f32>();
    horde + bosses
}

#[cfg(test)]
fn level_at_xp(mut xp: f32) -> u32 {
    let mut level = 1;
    while xp >= xp_to_next(level) {
        xp -= xp_to_next(level);
        level += 1;
    }
    level
}

#[cfg(test)]
#[test]
fn normal_density_keeps_a_build_moving_without_constant_hands() {
    for party_size in [1, 5] {
        for minute in [2.5, 6.0, 10.0, 15.0] {
            let xp = normal_run_xp(minute * 60.0, party_size);
            let level = level_at_xp(xp);
            eprintln!("normal density, {party_size}-player: level {level} at {minute}:00");
        }
    }
    for (seconds, expected) in [(150.0, 10), (360.0, 25), (600.0, 40), (VICTORY_SECS, 54)] {
        let level = level_at_xp(normal_run_xp(seconds, 1));
        assert!(
            (expected - 2..=expected + 2).contains(&(level as i32)),
            "normal density: wanted about {expected} at {seconds}s, got {level}"
        );
        let party_level = level_at_xp(normal_run_xp(seconds, 5));
        assert!(
            (level as i32 - 1..=level as i32 + 1).contains(&(party_level as i32)),
            "five players should stay near solo pace at {seconds}s: solo {level}, party {party_level}"
        );
    }
}

/// A creature on a battleground's roster: who it is, its pace and its size. Its health, damage
/// and experience come from its place on the roster, read off The Barrens' tuned ladder.
pub(crate) struct Foe {
    name: &'static str,
    display: u32,
    level: u32,
    speed: f32,
    /// The drawn scale, chosen against the model's Stand height so the field reads as The
    /// Barrens does: about 2-3.5 yd for the horde, 4-5 for the great beasts, 5-9 for a boss.
    scale: f32,
    big: bool,
}

const fn foe(name: &'static str, display: u32, level: u32, speed: f32, scale: f32) -> Foe {
    Foe {
        name,
        display,
        level,
        speed,
        scale,
        big: false,
    }
}

const fn giant(name: &'static str, display: u32, level: u32, speed: f32, scale: f32) -> Foe {
    Foe {
        big: true,
        ..foe(name, display, level, speed, scale)
    }
}

/// Where a run is fought.
pub(crate) struct MapDef {
    pub name: &'static str,
    /// The zone or continent under the name on the menu.
    pub zone: &'static str,
    /// The `Map.dbc` row.
    pub map: u32,
    /// Where the hero starts (WoW coordinates).
    pub center: [f32; 3],
    /// Ground used while the WMO/terrain collider is still streaming.
    pub floor: f32,
    /// How far from the centre the hero may walk.
    pub leash: f32,
    /// The results screen's line on a win.
    pub victory: &'static str,
    /// Weakest first; empty for The Barrens, which keeps its hand-tuned [`ENEMIES`].
    foes: &'static [Foe],
    /// One every [`BOSS_EVERY_SECS`], in order; empty for The Barrens' [`BOSSES`].
    bosses: &'static [Foe],
    /// A raid stage: its last boss walks out at [`RAID_BOSS_SECS`] and killing it wins the run;
    /// the [`VICTORY_SECS`] clock wins nothing ([`next_boss`], [`won`]).
    pub raid: bool,
}

impl MapDef {
    /// Ragnaros's arena deliberately includes walkable MLIQ magma; the other MC rooms keep their
    /// pools as scenery until their encounter design calls for them.
    pub(crate) fn lava_hazard(&self) -> bool {
        self.name == "Molten Core: Ragnaros' Lair"
    }
}

// CreatureDisplayInfo rows from the 1.12 client: Molten Giant 8269, Firelord 12231, Lava Surger
// 12239, Core Hound 12168, Flamewaker 12030 and Ancient Core Hound 12189.
const MC_FOES: &[Foe] = &[
    giant("Molten Giant", 8269, 62, 3.4, 1.0),
    foe("Firelord", 12231, 62, 4.2, 1.1),
    foe("Lava Surger", 12239, 62, 5.2, 0.9),
    giant("Core Hound", 12168, 61, 4.4, 0.9),
    foe("Flamewaker", 12030, 62, 4.2, 1.1),
    giant("Ancient Core Hound", 12189, 62, 4.0, 0.9),
];

const MC_BOSSES: &[Foe] = &[
    giant("Molten Giant", 8269, 62, 3.8, 1.35),
    foe("Firelord", 12231, 62, 4.4, 2.0),
    giant("Lava Surger", 12239, 62, 5.0, 1.2),
    giant("Ancient Core Hound", 12189, 62, 4.0, 1.1),
    foe("Flamewaker", 12030, 62, 4.2, 2.0),
    giant("Molten Giant", 8269, 62, 3.6, 1.45),
];

/// Lucifron (`CreatureDisplayInfo` 13031, the black Flamewaker salamander) closes his cavern's
/// roster, a scripted raid boss ([`super::raid`]).
const MC_LUCIFRON_BOSSES: &[Foe] = &[
    giant("Molten Giant", 8269, 62, 3.8, 1.35),
    foe("Firelord", 12231, 62, 4.4, 2.0),
    giant("Lava Surger", 12239, 62, 5.0, 1.2),
    giant("Ancient Core Hound", 12189, 62, 4.0, 1.1),
    foe("Flamewaker", 12030, 62, 4.2, 2.0),
    foe("Lucifron", 13031, 63, 4.6, 2.4),
];

/// Magmadar (`CreatureDisplayInfo` 10193, the great core hound) closes his lair's roster.
const MC_MAGMADAR_BOSSES: &[Foe] = &[
    giant("Molten Giant", 8269, 62, 3.8, 1.35),
    foe("Firelord", 12231, 62, 4.4, 2.0),
    giant("Lava Surger", 12239, 62, 5.0, 1.2),
    giant("Ancient Core Hound", 12189, 62, 4.0, 1.1),
    foe("Flamewaker", 12030, 62, 4.2, 2.0),
    giant("Magmadar", 10193, 63, 5.0, 1.8),
];

/// The adds raid scripts call in, by name ([`super::raid::Does::Adds`]): Lucifron's Flamewaker
/// Protectors (`CreatureDisplayInfo` 12029, the larger black salamander).
const MC_ADDS: &[Foe] = &[foe("Flamewaker Protector", 12029, 62, 4.4, 2.0)];

pub(crate) static MAPS: &[MapDef] = &[
    MapDef {
        name: "The Barrens",
        zone: "Kalimdor",
        map: 1,
        // The plains south of the Crossroads.
        center: [-950.0, -2700.0, 92.0],
        floor: 92.0,
        leash: 150.0,
        victory: "The Barrens are yours.",
        foes: &[],
        bosses: &[],
        raid: false,
    },
    MapDef {
        name: "The Dark Portal",
        zone: "Blasted Lands",
        map: 0,
        // The basin before the Portal, ringed by cliffs.
        center: [-11840.0, -3197.0, -30.0],
        floor: -30.0,
        leash: 75.0,
        victory: "The Dark Portal stands silent.",
        foes: &[
            foe("Starving Snickerfang", 8050, 45, 4.6, 1.0),
            foe("Snickerfang Hyena", 2714, 49, 5.0, 1.15),
            foe("Black Slayer", 10824, 47, 5.0, 0.6),
            foe("Redstone Crystalhide", 798, 51, 3.6, 1.3),
            foe("Shadowsworn Cultist", 7838, 51, 3.8, 1.3),
            foe("Felbeast", 7949, 50, 4.8, 0.85),
            foe("Dreadmaul Mauler", 14401, 53, 3.6, 1.1),
            foe("Felhound", 1913, 54, 5.2, 1.0),
            foe("Felguard Sentry", 9017, 55, 4.2, 0.9),
            foe("Manahound", 6173, 60, 5.6, 1.15),
            giant("Lesser Infernal", 10906, 55, 3.4, 1.0),
            foe("Felguard Elite", 7970, 61, 4.4, 1.0),
            giant("Doomguard Commander", 4426, 61, 4.0, 1.1),
        ],
        bosses: &[
            foe("Akubar the Seer", 10920, 54, 4.4, 2.6),
            foe("Lord Azrethoc", 4426, 55, 4.6, 1.35),
            foe("Lord Hel'nurath", 14556, 62, 4.8, 1.65),
            giant("Teremus the Devourer", 6378, 60, 5.0, 0.55),
            foe("Lord Banehollow", 8611, 62, 4.8, 1.95),
            giant("Lord Kazzak", 12449, 63, 4.6, 0.75),
        ],
        raid: false,
    },
    MapDef {
        name: "Gates of Ahn'Qiraj",
        zone: "Silithus",
        map: 1,
        // The sands before the Scarab Wall.
        center: [-8180.0, 1536.0, 5.0],
        floor: 5.0,
        leash: 80.0,
        victory: "The Qiraji retreat behind the Scarab Wall.",
        foes: &[
            foe("Sand Skitterer", 8014, 56, 5.0, 1.0),
            foe("Stonelash Scorpid", 15383, 55, 4.6, 1.6),
            foe("Dredge Striker", 15386, 56, 4.4, 1.0),
            foe("Twilight Avenger", 11811, 58, 4.0, 1.1),
            foe("Hive'Ashi Drone", 2303, 58, 5.4, 0.55),
            foe("Stonelash Flayer", 15385, 57, 4.8, 1.75),
            foe("Hive'Ashi Defender", 12153, 59, 4.2, 1.3),
            foe("Dust Stormer", 8715, 57, 4.6, 1.05),
            foe("Twilight Stonecaller", 11815, 59, 3.8, 1.1),
            foe("Hive'Zora Wasp", 11088, 59, 5.6, 0.6),
            foe("Hive'Zora Reaver", 11085, 60, 4.8, 1.0),
            foe("Hive'Regal Ambusher", 11106, 60, 5.2, 1.05),
            foe("Vekniss Wasp", 15335, 60, 5.6, 0.65),
            foe("Qiraji Gladiator", 15741, 61, 4.4, 0.75),
            foe("Vekniss Warrior", 15334, 61, 4.6, 1.15),
            giant("Obsidian Destroyer", 15343, 61, 3.8, 1.2),
            giant("Qiraji Champion", 15340, 63, 4.6, 0.95),
            giant("Anubisath Sentinel", 15347, 61, 4.0, 0.62),
        ],
        bosses: &[
            giant("Kurinnaxx", 15742, 62, 5.0, 1.15),
            foe("General Rajaxx", 15376, 63, 4.8, 1.15),
            foe("Moam", 15392, 63, 4.2, 1.6),
            giant("Buru the Gorger", 15654, 63, 4.4, 0.85),
            giant("Princess Huhuran", 15739, 63, 5.0, 0.55),
            foe("Ossirian the Unscarred", 15432, 63, 4.6, 0.82),
        ],
        raid: false,
    },
    MapDef {
        name: "Fire Plume Ridge",
        zone: "Un'Goro Crater",
        map: 1,
        // The summit plateau, its lava lake on the west rim; the crater floor's canopy would
        // hide the fight from above.
        center: [-7136.0, -1309.0, -184.0],
        floor: -184.0,
        leash: 32.0,
        victory: "Even King Mosh has fallen.",
        foes: &[
            foe("Bloodpetal Lasher", 11634, 49, 3.6, 0.9),
            foe("Young Diemetradon", 8510, 48, 4.4, 0.55),
            foe("Ravasaur", 5242, 50, 5.4, 0.75),
            foe("Muculent Ooze", 11140, 50, 3.2, 1.4),
            foe("Ravasaur Hunter", 5292, 51, 5.6, 0.85),
            foe("Tar Beast", 1549, 51, 3.4, 1.0),
            foe("Bloodpetal Flayer", 11635, 52, 3.8, 1.05),
            foe("Primal Ooze", 4754, 52, 3.4, 1.6),
            foe("Gorishi Wasp", 11090, 53, 5.4, 0.55),
            foe("Pterrordax", 8411, 53, 5.2, 0.5),
            foe("Diemetradon", 8511, 52, 4.6, 0.7),
            foe("Glutinous Ooze", 1146, 54, 3.6, 1.8),
            foe("Tar Lurker", 9010, 54, 3.6, 1.15),
            foe("Scorching Elemental", 1070, 54, 4.2, 1.05),
            foe("Frenzied Pterrordax", 8412, 55, 5.6, 0.55),
            foe("Elder Diemetradon", 8512, 55, 4.6, 0.8),
            giant("Stegodon", 5241, 52, 3.8, 0.72),
            giant("Stone Guardian", 8395, 52, 3.6, 1.45),
            giant("Plated Stegodon", 5287, 54, 3.8, 0.78),
            giant("Devilsaur", 5239, 56, 5.0, 0.72),
        ],
        bosses: &[
            foe("Ravasaur Matriarch", 11319, 56, 5.6, 1.55),
            foe("Uhk'loc", 8129, 59, 4.8, 2.4),
            foe("Blazerunner", 1204, 57, 4.6, 1.8),
            foe("Baron Charr", 14517, 58, 4.6, 1.95),
            giant("Tyrant Devilsaur", 5240, 60, 5.0, 1.0),
            giant("King Mosh", 5305, 61, 5.0, 1.2),
        ],
        raid: false,
    },
    MapDef {
        name: "Gurubashi Arena",
        zone: "Stranglethorn Vale",
        map: 0,
        // The arena pit: its floor is the open ADT terrain, ringed by the arena WMO's wall 42-48
        // yd out; the leash is the largest circle inside the wall less 2 yd (`arena_probe`).
        center: [-13203.5, 275.5, 21.9],
        floor: 21.9,
        leash: 40.0,
        victory: "The arena roars your name.",
        foes: &[
            foe("Shadowmaw Panther", 11452, 37, 5.6, 1.4),
            foe("Cold Eye Basilisk", 8797, 39, 3.8, 1.0),
            foe("Jungle Stalker", 11317, 40, 5.2, 0.8),
            foe("Elder Mistvale Gorilla", 838, 40, 4.0, 1.1),
            foe("Zanzil Zombie", 1065, 43, 3.4, 1.1),
            foe("Thrashtail Basilisk", 8802, 41, 4.0, 1.15),
            foe("Bloodsail Sea Dog", 796, 44, 4.0, 1.05),
            foe("Gurubashi Axe Thrower", 11074, 60, 4.2, 1.1),
            foe("Razzashi Serpent", 15182, 60, 4.8, 0.85),
            foe("Zulian Panther", 633, 60, 5.6, 1.6),
            foe("Gurubashi Headhunter", 11109, 60, 4.4, 1.15),
            foe("Bloodseeker Bat", 14562, 60, 5.6, 0.55),
            foe("Razzashi Raptor", 2571, 60, 5.6, 0.95),
            foe("Gurubashi Blood Drinker", 11080, 60, 4.2, 1.2),
            foe("Zulian Tiger", 11031, 61, 5.4, 1.8),
            foe("Hakkari Shadowcaster", 11229, 61, 4.0, 1.2),
            foe("Gurubashi Champion", 11100, 61, 4.4, 1.3),
            foe("Gurubashi Bat Rider", 15303, 62, 5.0, 0.6),
            giant("Gurubashi Berserker", 14832, 62, 4.0, 1.7),
        ],
        bosses: &[
            giant("King Bangalash", 616, 43, 5.8, 3.0),
            foe("Mokk the Savage", 840, 44, 4.8, 2.4),
            foe("Bloodlord Mandokir", 11288, 63, 5.0, 2.1),
            foe("High Priest Thekal", 15216, 63, 5.0, 2.3),
            foe("Jin'do the Hexxer", 11311, 63, 4.6, 2.5),
            giant("Hakkar", 15295, 63, 4.4, 2.7),
        ],
        raid: false,
    },
    MapDef {
        name: "Elwynn Forest",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Elwynn Forest's own creatures: no building or big prop near.
        center: [-9548.5, 115.0, 60.0],
        floor: 60.0,
        leash: 150.0,
        victory: "Elwynn Forest is safe once more.",
        foes: &[
            foe("Young Wolf", 447, 1, 5.4, 1.0),
            foe("Timber Wolf", 604, 2, 5.4, 1.0),
            foe("Defias Thug", 5035, 4, 4.2, 1.15),
            foe("Mangy Wolf", 903, 6, 5.4, 1.0),
            foe("Stonetusk Boar", 503, 6, 4.6, 1.0),
            foe("Kobold Tunneler", 139, 6, 4.2, 1.0),
            foe("Defias Cutpurse", 2361, 6, 4.2, 1.15),
            foe("Forest Spider", 382, 6, 4.2, 1.0),
            foe("Murloc", 617, 7, 4.2, 1.0),
            foe("Kobold Miner", 373, 7, 4.2, 1.0),
            foe("Murloc Streamrunner", 527, 7, 4.2, 1.0),
            foe("Rockhide Boar", 389, 8, 4.6, 1.0),
            foe("Gray Forest Wolf", 380, 8, 5.4, 1.0),
            foe("Defias Bandit", 2357, 9, 4.2, 1.15),
            giant("Young Forest Bear", 1006, 9, 4.6, 1.0),
            foe("Defias Rogue Wizard", 2359, 10, 3.8, 1.1),
        ],
        bosses: &[
            foe("Mother Fang", 2541, 10, 4.6, 1.9),
            foe("Narg the Taskmaster", 774, 10, 4.2, 1.9),
            foe("Hogger", 384, 11, 4.2, 1.9),
            foe("Thuros Lightfingers", 3341, 11, 4.2, 2.18),
            foe("Fedfennel", 175, 12, 4.2, 1.9),
            foe("Gruff Swiftbite", 175, 12, 4.2, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Durotar",
        zone: "Kalimdor",
        map: 1,
        // Open, level ground among Durotar's own creatures: no building or big prop near.
        center: [587.5, -4311.4, 19.0],
        floor: 19.0,
        leash: 150.0,
        victory: "Durotar is quiet again.",
        foes: &[
            foe("Vile Familiar", 1185, 4, 4.2, 1.0),
            foe("Felstalker", 850, 4, 4.2, 1.0),
            foe("Clattering Scorpid", 2486, 6, 4.6, 1.5),
            foe("Pygmy Surf Crawler", 1307, 6, 4.6, 1.0),
            foe("Dire Mottled Boar", 381, 7, 4.6, 1.0),
            foe("Razormane Quilboar", 1218, 7, 4.6, 1.0),
            foe("Armored Scorpid", 2487, 8, 4.6, 1.5),
            foe("Bloodtalon Taillasher", 1960, 8, 5.4, 0.85),
            foe("Razormane Scout", 1252, 8, 4.6, 1.0),
            foe("Dustwind Harpy", 3012, 8, 4.2, 1.0),
            foe("Dustwind Pillager", 6814, 8, 4.2, 1.0),
            foe("Razormane Dustrunner", 6088, 9, 4.6, 1.0),
            foe("Elder Mottled Boar", 193, 9, 4.6, 1.0),
            foe("Razormane Battleguard", 1964, 10, 4.6, 1.0),
            foe("Burning Blade Neophyte", 4186, 10, 3.8, 1.0),
            foe("Burning Blade Cultist", 4194, 11, 3.8, 1.0),
        ],
        bosses: &[
            foe("Geolord Mottle", 6113, 9, 4.6, 1.9),
            foe("Razormane Dustrunner", 6088, 9, 4.6, 1.9),
            foe("Razormane Battleguard", 1964, 10, 4.6, 1.9),
            foe("Burning Blade Neophyte", 4186, 10, 3.8, 1.9),
            foe("Death Flayer", 2491, 11, 4.6, 2.85),
            foe("Burning Blade Cultist", 4194, 11, 3.8, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Teldrassil",
        zone: "Kalimdor",
        map: 1,
        // Open, level ground among Teldrassil's own creatures: no building or big prop near.
        center: [9737.5, 649.8, 1298.5],
        floor: 1298.5,
        leash: 150.0,
        victory: "Teldrassil blooms again.",
        foes: &[
            foe("Young Nightsaber", 11454, 1, 5.4, 1.5),
            foe("Young Thistle Boar", 8869, 2, 4.6, 1.0),
            foe("Grell", 6806, 3, 4.2, 1.0),
            foe("Grellkin", 3023, 4, 4.2, 1.0),
            foe("Nightsaber", 6805, 6, 5.4, 1.5),
            foe("Strigid Owl", 10832, 6, 5.2, 1.0),
            foe("Webwood Lurker", 760, 6, 4.2, 1.0),
            foe("Timberling", 3033, 6, 4.2, 1.15),
            foe("Gnarlpine Gardener", 6799, 6, 3.8, 1.0),
            foe("Gnarlpine Warrior", 6800, 7, 4.2, 1.0),
            foe("Gnarlpine Ambusher", 3024, 7, 4.2, 1.0),
            foe("Vicious Grell", 6811, 7, 4.2, 1.0),
            foe("Gnarlpine Defender", 6801, 8, 4.2, 1.0),
            foe("Gnarlpine Shaman", 897, 8, 3.8, 1.0),
            foe("Timberling Bark Ripper", 6817, 8, 4.2, 1.15),
            foe("Gnarlpine Augur", 3024, 9, 3.8, 1.0),
        ],
        bosses: &[
            foe("Threggil", 904, 6, 4.2, 1.9),
            foe("Uruson", 6818, 7, 4.2, 1.9),
            foe("Gnarlpine Defender", 6801, 8, 4.2, 1.9),
            foe("Duskstalker", 11453, 9, 5.4, 2.85),
            foe("Oakenscowl", 2567, 9, 4.2, 2.18),
            foe("Gnarlpine Augur", 3024, 9, 3.8, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Dun Morogh",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Dun Morogh's own creatures: no building or big prop near.
        center: [-5400.8, -739.9, 396.7],
        floor: 396.7,
        leash: 150.0,
        victory: "Dun Morogh thaws in your wake.",
        foes: &[
            foe("Crag Boar", 607, 6, 4.6, 1.0),
            giant("Young Black Bear", 8843, 6, 4.6, 1.0),
            foe("Juvenile Snow Leopard", 748, 6, 5.4, 1.5),
            foe("Young Wendigo", 929, 6, 4.2, 1.0),
            foe("Wendigo", 950, 7, 4.2, 1.0),
            foe("Large Crag Boar", 381, 7, 4.6, 1.0),
            foe("Snow Tracker Wolf", 604, 7, 5.4, 1.0),
            foe("Snow Leopard", 954, 8, 5.4, 1.5),
            foe("Elder Crag Boar", 744, 8, 4.6, 1.0),
            foe("Winter Wolf", 785, 8, 5.4, 1.0),
            giant("Ice Claw Bear", 8840, 8, 4.6, 1.0),
            foe("Frostmane Seer", 5610, 9, 3.8, 1.2),
            foe("Frostmane Snowstrider", 5607, 9, 4.2, 1.2),
        ],
        bosses: &[
            foe("Edan the Howler", 931, 9, 4.2, 1.9),
            foe("Timber", 11422, 10, 5.4, 1.9),
            foe("Great Father Arctikus", 5625, 11, 4.2, 2.28),
            foe("Old Icebeard", 1050, 11, 4.2, 1.9),
            foe("Vagash", 1050, 11, 4.2, 1.9),
            giant("Bjarn", 913, 12, 4.6, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Mulgore",
        zone: "Kalimdor",
        map: 1,
        // Open, level ground among Mulgore's own creatures: no building or big prop near.
        center: [-1809.9, -220.6, -8.4],
        floor: -8.4,
        leash: 150.0,
        victory: "Mulgore's herds roam free.",
        foes: &[
            foe("Plainstrider", 1219, 2, 5.4, 1.0),
            foe("Battleboar", 8869, 4, 4.6, 1.0),
            foe("Prairie Wolf", 1100, 6, 5.4, 1.0),
            foe("Palemane Tanner", 550, 6, 4.2, 1.0),
            foe("Venture Co. Hireling", 7243, 6, 4.2, 1.3),
            foe("Adult Plainstrider", 1220, 7, 5.4, 1.0),
            foe("Wiry Swoop", 1228, 7, 5.2, 0.8),
            foe("Venture Co. Laborer", 7244, 7, 4.2, 1.3),
            foe("Palemane Skinner", 1216, 7, 4.2, 1.0),
            foe("Prairie Stalker", 643, 8, 5.4, 1.0),
            foe("Flatland Cougar", 1059, 8, 5.4, 1.5),
            foe("Palemane Poacher", 1217, 8, 4.2, 1.0),
            foe("Venture Co. Taskmaster", 7246, 8, 4.2, 1.3),
            foe("Swoop", 1229, 9, 5.2, 0.8),
            foe("Elder Plainstrider", 1221, 9, 5.4, 1.0),
        ],
        bosses: &[
            foe("Prairie Stalker", 643, 8, 5.4, 1.9),
            foe("Mazzranache", 1961, 9, 5.4, 1.9),
            foe("Snagglespear", 275, 9, 4.2, 1.9),
            foe("Swoop", 1229, 9, 5.2, 1.52),
            foe("Elder Plainstrider", 1221, 9, 5.4, 1.9),
            foe("Ghost Howl", 720, 12, 5.4, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Tirisfal Glades",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Tirisfal Glades's own creatures: no building or big prop near.
        center: [1993.0, -153.3, 35.4],
        floor: 35.4,
        leash: 150.0,
        victory: "Tirisfal is at peace.",
        foes: &[
            foe("Decrepit Darkhound", 9021, 6, 4.2, 1.4),
            foe("Rotting Dead", 1200, 6, 3.6, 1.0),
            foe("Greater Duskbat", 4734, 7, 5.2, 0.8),
            foe("Ravaged Corpse", 1201, 7, 3.6, 1.0),
            foe("Rot Hide Graverobber", 10853, 7, 4.2, 1.0),
            foe("Rot Hide Gnoll", 10852, 7, 4.2, 1.0),
            foe("Rattlecage Soldier", 200, 7, 4.2, 1.2),
            foe("Cursed Darkhound", 9020, 8, 4.2, 1.4),
            foe("Rot Hide Mongrel", 847, 8, 4.2, 1.0),
            foe("Hungering Dead", 1202, 8, 3.6, 1.0),
            foe("Scarlet Missionary", 2468, 8, 3.8, 1.15),
            foe("Darkeye Bonecaster", 11398, 8, 3.8, 1.0),
            foe("Vampiric Duskbat", 8808, 9, 5.2, 0.8),
            foe("Scarlet Zealot", 2483, 9, 4.2, 1.1),
            foe("Shambling Horror", 1197, 9, 3.6, 1.0),
        ],
        bosses: &[
            foe("Farmer Solliden", 3535, 8, 4.2, 2.18),
            foe("Tormented Spirit", 9534, 9, 4.2, 1.52),
            foe("Bayne", 7892, 10, 4.2, 2.66),
            foe("Ressan the Needler", 9750, 11, 5.2, 1.52),
            foe("Fellicent's Shade", 5430, 12, 4.2, 1.9),
            foe("Gorefang", 11413, 13, 5.4, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Westfall",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Westfall's own creatures: no building or big prop near.
        center: [-10252.2, 1483.5, 41.6],
        floor: 41.6,
        leash: 150.0,
        victory: "Westfall's fields are safe.",
        foes: &[
            foe("Young Fleshripper", 410, 11, 5.2, 0.8),
            foe("Coyote", 643, 11, 5.4, 1.0),
            foe("Defias Smuggler", 4418, 12, 4.2, 1.15),
            giant("Harvest Golem", 367, 12, 3.6, 1.0),
            foe("Young Goretusk", 8871, 13, 4.6, 1.0),
            foe("Defias Trapper", 2331, 13, 4.2, 1.1),
            foe("Fleshripper", 2305, 14, 5.2, 0.8),
            foe("Goretusk", 3027, 15, 4.6, 1.0),
            giant("Harvest Watcher", 378, 15, 3.6, 1.0),
            foe("Riverpaw Brute", 384, 16, 4.2, 1.0),
            foe("Riverpaw Bandit", 383, 17, 4.2, 1.0),
            foe("Great Goretusk", 3035, 17, 4.6, 1.0),
            foe("Riverpaw Taskmaster", 376, 18, 4.2, 1.0),
            foe("Defias Highwayman", 2342, 18, 4.2, 1.15),
            foe("Pygmy Venom Web Spider", 958, 19, 4.6, 1.0),
            foe("Riverpaw Mystic", 502, 19, 3.8, 1.0),
        ],
        bosses: &[
            foe("Brainwashed Noble", 3267, 18, 4.2, 2.18),
            foe("Marisa du'Paige", 2355, 18, 4.2, 2.09),
            foe("Sergeant Brashclaw", 383, 18, 4.2, 1.9),
            foe("Klaven Mortwake", 5805, 23, 4.2, 2.18),
            foe("Lupos", 11412, 23, 5.4, 1.9),
            foe("Vultros", 507, 26, 5.2, 1.52),
        ],
        raid: false,
    },
    MapDef {
        name: "Duskwood",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Duskwood's own creatures: no building or big prop near.
        center: [-10051.8, -792.5, 36.5],
        floor: 36.5,
        leash: 150.0,
        victory: "Duskwood's curse lifts.",
        foes: &[
            foe("Pygmy Venom Web Spider", 958, 19, 4.6, 1.0),
            foe("Venom Web Spider", 955, 20, 4.6, 1.0),
            foe("Skeletal Warrior", 200, 22, 4.2, 1.2),
            foe("Green Recluse", 2541, 22, 4.6, 1.0),
            foe("Skeletal Mage", 9783, 23, 4.2, 1.0),
            foe("Young Black Ravager", 246, 24, 5.4, 1.0),
            foe("Skeletal Horror", 9786, 24, 3.6, 1.0),
            foe("Black Widow Hatchling", 368, 25, 4.6, 1.0),
            foe("Black Ravager", 741, 25, 5.4, 1.0),
            foe("Black Ravager Mastiff", 9562, 26, 5.4, 1.0),
            foe("Nightbane Worgen", 657, 27, 5.4, 1.0),
            foe("Nightbane Shadow Weaver", 203, 28, 5.4, 1.0),
            foe("Nightbane Dark Runner", 736, 29, 5.4, 1.0),
            foe("Nightbane Vile Fang", 522, 30, 5.4, 1.0),
            foe("Fetid Corpse", 559, 30, 3.6, 1.3),
            foe("Nightbane Tainted One", 1098, 31, 5.4, 1.0),
        ],
        bosses: &[
            foe("Lupos", 11412, 23, 5.4, 1.9),
            foe("Naraxis", 963, 27, 4.6, 1.9),
            foe("Nightbane Vile Fang", 522, 30, 5.4, 1.9),
            foe("Nightbane Tainted One", 1098, 31, 5.4, 1.9),
            foe("Fenros", 11179, 32, 5.4, 1.9),
            foe("Nefaru", 11181, 34, 5.4, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Redridge Mountains",
        zone: "Eastern Kingdoms",
        map: 0,
        // Open, level ground among Redridge Mountains's own creatures: no building or big prop near.
        center: [-9199.8, -2767.5, 91.0],
        floor: 91.0,
        leash: 150.0,
        victory: "Redridge is safe once more.",
        foes: &[
            foe("Redridge Thrasher", 489, 15, 4.2, 1.0),
            foe("Redridge Mongrel", 487, 16, 4.2, 1.0),
            foe("Tarantula", 366, 16, 4.2, 1.0),
            foe("Great Goretusk", 3035, 17, 4.6, 1.0),
            foe("Redridge Poacher", 551, 17, 4.2, 1.0),
            foe("Murloc Shorestriker", 1305, 17, 4.2, 1.0),
            foe("Black Dragon Whelp", 387, 18, 4.2, 1.0),
            foe("Redridge Brute", 500, 18, 4.2, 1.0),
            foe("Redridge Mystic", 10789, 19, 3.8, 1.0),
            foe("Dire Condor", 490, 19, 5.2, 0.8),
            foe("Redridge Basher", 609, 20, 4.2, 1.0),
            foe("Greater Tarantula", 520, 20, 4.2, 1.0),
            foe("Redridge Drudger", 551, 21, 4.2, 1.0),
            foe("Blackrock Summoner", 6030, 23, 3.8, 1.0),
            foe("Blackrock Tracker", 6040, 24, 4.2, 1.0),
            foe("Blackrock Champion", 6044, 25, 4.2, 1.0),
        ],
        bosses: &[
            foe("Blackrock Hunter", 6031, 24, 4.2, 1.9),
            foe("Singe", 497, 24, 4.2, 1.9),
            foe("Tharil'zun", 6043, 24, 4.2, 1.9),
            foe("Blackrock Gladiator", 6042, 25, 4.2, 1.9),
            foe("Rohh the Silent", 10792, 26, 4.2, 1.9),
            foe("Kazon", 6041, 27, 4.2, 1.9),
        ],
        raid: false,
    },
    MapDef {
        name: "Mount Hyjal",
        zone: "Kalimdor",
        map: 1,
        // A plateau below the World Tree.
        center: [5478.0, -3729.0, 1593.0],
        floor: 1593.0,
        leash: 70.0,
        victory: "Nordrassil endures.",
        foes: &[
            foe("Cannibal Ghoul", 10626, 56, 4.6, 1.3),
            foe("Jadefire Rogue", 8575, 52, 5.2, 1.0),
            foe("Putrid Gargoyle", 7854, 56, 5.2, 0.85),
            foe("Death Cultist", 10391, 56, 3.8, 1.1),
            foe("Felbeast", 7949, 50, 4.8, 0.85),
            foe("Torn Screamer", 984, 57, 5.0, 0.7),
            foe("Necromancer", 10427, 57, 3.6, 1.15),
            foe("Jadefire Felsworn", 11334, 54, 4.8, 1.1),
            foe("Felhound", 1913, 54, 5.2, 1.0),
            foe("Nerubian Overseer", 14698, 58, 4.4, 0.95),
            foe("Felguard Sentry", 9017, 55, 4.2, 0.9),
            foe("Hate Shrieker", 10751, 58, 5.0, 0.8),
            foe("Hederine Manastalker", 6172, 58, 5.4, 1.15),
            giant("Lesser Infernal", 10906, 55, 3.4, 1.0),
            foe("Felguard Elite", 7970, 61, 4.4, 1.0),
            giant("Stitched Horror", 1693, 58, 3.6, 1.25),
            giant("Doomguard Commander", 4426, 61, 4.0, 1.1),
        ],
        bosses: &[
            foe("Prince Xavalis", 11335, 57, 5.0, 1.85),
            foe("Lady Hederine", 10925, 61, 4.8, 2.7),
            giant("Taerar", 15363, 63, 4.8, 0.5),
            giant("Lethon", 15365, 63, 4.8, 0.5),
            giant("Emeriss", 15366, 63, 4.8, 0.53),
            giant("Ysondre", 15364, 63, 4.8, 0.58),
        ],
        raid: false,
    },
    MapDef {
        name: "Naxxramas",
        zone: "Eastern Plaguelands",
        map: 0,
        // Plaguewood, under the necropolis.
        center: [3010.0, -3790.0, 124.0],
        floor: 124.0,
        leash: 80.0,
        victory: "Kel'Thuzad's necropolis falls dark.",
        foes: &[
            foe("Scourge Soldier", 5231, 55, 4.4, 1.15),
            foe("Plagued Swine", 6121, 53, 5.0, 1.0),
            foe("Cannibal Ghoul", 10626, 56, 4.6, 1.3),
            foe("Cursed Mage", 11397, 55, 3.6, 0.95),
            foe("Putrid Gargoyle", 7854, 56, 5.2, 0.85),
            foe("Scourge Warder", 612, 56, 4.4, 1.2),
            foe("Death Cultist", 10391, 56, 3.8, 1.1),
            foe("Frenzied Plaguehound", 9022, 57, 5.6, 1.4),
            foe("Torn Screamer", 984, 57, 5.0, 0.7),
            foe("Necromancer", 10427, 57, 3.6, 1.15),
            foe("Carrion Devourer", 7900, 57, 4.0, 1.25),
            foe("Crypt Walker", 11178, 58, 4.6, 0.85),
            foe("Putrid Shrieker", 7855, 58, 5.4, 1.0),
            foe("Hate Shrieker", 10751, 58, 5.0, 0.8),
            foe("Ziggurat Protector", 10627, 59, 4.4, 1.6),
            foe("Deathknight", 16508, 60, 4.6, 1.75),
            giant("Stitched Horror", 1693, 58, 3.6, 1.25),
            giant("Slaughterhouse Protector", 7864, 60, 3.6, 1.3),
            giant("Death Lord", 10729, 61, 5.0, 1.05),
        ],
        bosses: &[
            giant("Anub'Rekhan", 15931, 63, 4.6, 0.55),
            foe("Grand Widow Faerlina", 15940, 63, 4.8, 2.9),
            giant("Patchwerk", 16174, 63, 4.0, 1.7),
            giant("Gluth", 16064, 63, 5.0, 0.62),
            giant("Thaddius", 16137, 63, 4.0, 0.98),
            foe("Kel'Thuzad", 15945, 63, 4.4, 2.1),
        ],
        raid: false,
    },
    MapDef {
        name: "Molten Core: Raid Start",
        zone: "Molten Core",
        map: 409,
        center: [1080.0, -480.0, -108.2],
        floor: -108.2,
        leash: 14.0,
        victory: "The Core's entrance is secure.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: Lucifron's Cavern",
        zone: "Molten Core",
        map: 409,
        center: [1096.0, -732.0, -149.1],
        floor: -149.1,
        leash: 45.0,
        victory: "Lucifron's cavern falls silent.",
        foes: MC_FOES,
        bosses: MC_LUCIFRON_BOSSES,
        raid: true,
    },
    MapDef {
        name: "Molten Core: Magmadar's Lair",
        zone: "Molten Core",
        map: 409,
        center: [1112.0, -1008.0, -187.3],
        floor: -187.3,
        leash: 24.0,
        victory: "Magmadar's lair is clear.",
        foes: MC_FOES,
        bosses: MC_MAGMADAR_BOSSES,
        raid: true,
    },
    MapDef {
        name: "Molten Core: South-centre Cavern",
        zone: "Molten Core",
        map: 409,
        center: [868.0, -1008.0, -196.4],
        floor: -196.4,
        leash: 34.0,
        victory: "The south cavern cools.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: West Cavern",
        zone: "Molten Core",
        map: 409,
        center: [732.0, -520.0, -217.0],
        floor: -217.0,
        leash: 40.0,
        victory: "The west cavern is clear.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: North-west Cavern",
        zone: "Molten Core",
        map: 409,
        center: [644.0, -816.0, -208.5],
        floor: -208.5,
        leash: 45.0,
        victory: "The north-west cavern is clear.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: Centre Cavern",
        zone: "Molten Core",
        map: 409,
        center: [892.0, -600.0, -203.6],
        floor: -203.6,
        leash: 34.0,
        victory: "The centre cavern is clear.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: Majordomo's Camp",
        zone: "Molten Core",
        map: 409,
        center: [736.0, -1168.0, -120.4],
        floor: -120.4,
        leash: 30.0,
        victory: "Majordomo's camp is broken.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: Ragnaros's Dry Ledge",
        zone: "Molten Core",
        map: 409,
        center: [844.0, -772.0, -226.0],
        floor: -226.0,
        leash: 16.0,
        victory: "The dry ledge holds.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: false,
    },
    MapDef {
        name: "Molten Core: Ragnaros' Lair",
        zone: "Molten Core",
        map: 409,
        center: [828.0, -852.0, -229.0],
        floor: -229.0,
        leash: 30.0,
        victory: "Ragnaros is driven back into the lava.",
        foes: MC_FOES,
        bosses: MC_BOSSES,
        raid: true,
    },
];

/// A battleground's horde and bosses with their numbers.
pub(crate) struct Roster {
    pub enemies: Vec<EnemyDef>,
    pub bosses: Vec<EnemyDef>,
    /// What raid scripts call in on this battleground ([`super::raid`]), never on the clock.
    pub adds: Vec<EnemyDef>,
}

impl Roster {
    /// The kind at `index` of the horde (`boss` false) or the bosses; past the horde, the adds,
    /// numbered on from it as [`Roster::index_of`] numbers them.
    pub(crate) fn kind(&self, boss: bool, index: usize) -> Option<&EnemyDef> {
        if boss {
            return self.bosses.get(index);
        }
        self.enemies
            .get(index)
            .or_else(|| self.adds.get(index.checked_sub(self.enemies.len())?))
    }

    /// Where `def` stands in the roster, `(boss, index)`, as [`Roster::kind`] reads it back.
    pub(crate) fn index_of(&self, def: &EnemyDef) -> Option<(bool, usize)> {
        let same = |list: &[EnemyDef]| list.iter().position(|d| std::ptr::eq(d, def));
        same(&self.bosses)
            .map(|i| (true, i))
            .or_else(|| same(&self.enemies).map(|i| (false, i)))
            .or_else(|| same(&self.adds).map(|i| (false, self.enemies.len() + i)))
    }
}

static ROSTERS: std::sync::LazyLock<Vec<Roster>> = std::sync::LazyLock::new(|| {
    MAPS.iter()
        .map(|m| {
            if m.foes.is_empty() {
                Roster {
                    enemies: ENEMIES.to_vec(),
                    bosses: BOSSES.to_vec(),
                    adds: Vec::new(),
                }
            } else {
                Roster {
                    enemies: ladder(m.foes, ENEMIES),
                    bosses: ladder(m.bosses, BOSSES),
                    // Numbered at the ladder's first boss rung: an add, not a boss.
                    adds: if m.map == 409 {
                        ladder(MC_ADDS, BOSSES)
                    } else {
                        Vec::new()
                    },
                }
            }
        })
        .collect()
});

pub(crate) fn roster(map: usize) -> &'static Roster {
    &ROSTERS[map.min(ROSTERS.len() - 1)]
}

/// Number a roster off a tuned ladder: each creature takes the health, damage and experience at
/// its own place along it, from the ladder's running peak so no later tier comes out softer.
fn ladder(foes: &[Foe], tuned: &[EnemyDef]) -> Vec<EnemyDef> {
    let mut peak = (0.0f32, 0.0f32, 0.0f32);
    let rungs: Vec<(f32, f32, f32)> = tuned
        .iter()
        .map(|d| {
            peak = (
                peak.0.max(d.hp),
                peak.1.max(d.damage),
                peak.2.max(d.xp as f32),
            );
            peak
        })
        .collect();
    let top = (rungs.len() - 1) as f32;
    foes.iter()
        .enumerate()
        .map(|(i, f)| {
            let t = if foes.len() > 1 {
                i as f32 / (foes.len() - 1) as f32 * top
            } else {
                0.0
            };
            let (lo, hi) = (rungs[t.floor() as usize], rungs[t.ceil() as usize]);
            let k = t.fract();
            let mix = |a: f32, b: f32| (a + (b - a) * k).round();
            EnemyDef {
                name: f.name,
                display: f.display,
                hp: mix(lo.0, hi.0),
                speed: f.speed,
                damage: mix(lo.1, hi.1),
                scale: f.scale,
                xp: mix(lo.2, hi.2) as u32,
                level: f.level,
                big: f.big,
            }
        })
        .collect()
}

#[cfg(test)]
mod arena_probe {
    use std::collections::HashSet;

    use benilla_assets::coords::{placement_rotation, wow_to_bevy};
    use bevy::math::{Affine3A, Vec3};

    use super::MAPS;

    /// The Gurubashi pit measured from the MPQs: its floor is the ADT terrain (the arena WMO has
    /// no floor there) and its wall is the WMO's walking collision. The stage's centre stands on
    /// that floor, and its leash circle plus a 2 yd margin stays inside the wall in 32 directions.
    #[test]
    fn gurubashi_leash_fills_the_pit_inside_its_wall() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let arena = MAPS.iter().find(|m| m.name == "Gurubashi Arena").unwrap();
        let center = wow_to_bevy(arena.center);
        let near = |p: Vec3| (p - center).with_y(0.0).length() < 150.0;
        let mut seen = HashSet::new();
        let (mut wmo, mut terrain) = (Vec::new(), Vec::new());
        let tiles = benilla_formats::load_tiles_around(
            &mut chain,
            "Azeroth",
            arena.center[0],
            arena.center[1],
            1,
        )
        .expect("load arena tiles");
        for (_, tile) in tiles {
            for chunk in &tile.chunks {
                for &tri in chunk.indices.as_chunks::<3>().0 {
                    let tri = tri.map(|i| wow_to_bevy(chunk.positions[i as usize]));
                    if near(tri[0]) {
                        terrain.push(tri);
                    }
                }
            }
            for placed in tile.wmos {
                if !placed
                    .model
                    .to_ascii_lowercase()
                    .contains("stranglegladiatorarena")
                    || !seen.insert(placed.unique_id)
                {
                    continue;
                }
                let world_from_local = Affine3A::from_rotation_translation(
                    placement_rotation(placed.rotation),
                    wow_to_bevy(placed.position),
                );
                let mesh = benilla_formats::load_wmo_collision_tris(&mut chain, &placed.model)
                    .expect("load the arena's walking collision");
                for &tri in mesh.indices.as_chunks::<3>().0 {
                    wmo.push(tri.map(|i| {
                        world_from_local.transform_point3(wow_to_bevy(mesh.positions[i as usize]))
                    }));
                }
            }
        }
        assert!(!wmo.is_empty(), "the Gurubashi Arena WMO is placed here");

        // The floor under the centre and round the leash is the flat terrain, open to the sky.
        for p in std::iter::once(center).chain((0..8).map(|i| {
            let a = i as f32 / 8.0 * std::f32::consts::TAU;
            center + Vec3::new(a.cos(), 0.0, a.sin()) * arena.leash
        })) {
            let top = p.with_y(arena.floor + 200.0);
            let floor = terrain
                .iter()
                .filter_map(|tri| ray_hit(top, Vec3::NEG_Y, tri))
                .map(|d| top.y - d)
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                (floor - arena.floor).abs() < 0.25,
                "the pit floor at {p:?} is {floor}, not the stage's {}",
                arena.floor
            );
            assert!(
                wmo.iter()
                    .all(|tri| ray_hit(top, Vec3::NEG_Y, tri)
                        .is_none_or(|d| top.y - d < floor - 0.5)),
                "the arena WMO covers the pit floor at {p:?}"
            );
        }

        let origin = center.with_y(arena.floor + 1.5);
        let distances = (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * std::f32::consts::TAU;
                let dir = Vec3::new(a.cos(), 0.0, a.sin());
                wmo.iter()
                    .filter_map(|tri| ray_hit(origin, dir, tri))
                    .fold(f32::INFINITY, f32::min)
            })
            .collect::<Vec<_>>();
        eprintln!("Gurubashi pit wall distances: {distances:.2?}");
        for (i, distance) in distances.into_iter().enumerate() {
            assert!(
                distance >= arena.leash + 2.0,
                "leash plus margin crosses the pit wall at direction {i}: {distance} yd"
            );
            assert!(
                distance < 60.0,
                "direction {i} escapes the pit: {distance} yd"
            );
        }
    }

    #[test]
    fn molten_core_roster_uses_client_creature_display_rows() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        for foe in super::MC_FOES
            .iter()
            .chain(super::MC_BOSSES)
            .chain(super::MC_LUCIFRON_BOSSES)
            .chain(super::MC_MAGMADAR_BOSSES)
            .chain(super::MC_ADDS)
        {
            assert!(
                creatures.display_scale(foe.display).is_some(),
                "{} display {} is not a 1.12 CreatureDisplayInfo row",
                foe.name,
                foe.display
            );
        }
        let ragnaros = MAPS
            .iter()
            .find(|m| m.name == "Molten Core: Ragnaros' Lair")
            .expect("MC lava arena");
        assert!(ragnaros.lava_hazard());
    }

    /// MÃ¶ller-Trumbore: the distance along `direction` to `tri`, either face.
    fn ray_hit(origin: Vec3, direction: Vec3, tri: &[Vec3; 3]) -> Option<f32> {
        let [a, b, c] = *tri;
        let edge_ab = b - a;
        let edge_ac = c - a;
        let p = direction.cross(edge_ac);
        let det = edge_ab.dot(p);
        if det.abs() < 1e-7 {
            return None;
        }
        let inv_det = det.recip();
        let to_origin = origin - a;
        let u = to_origin.dot(p) * inv_det;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = to_origin.cross(edge_ab);
        let v = direction.dot(q) * inv_det;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let distance = edge_ac.dot(q) * inv_det;
        (distance >= 0.0).then_some(distance)
    }
}
