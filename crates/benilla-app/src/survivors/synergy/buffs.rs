//! Group buffs: each class's signature buff on every ally in reach, in a party of two or more.
//!
//! A cast buff (Fortitude, Arcane Intellect, Mark of the Wild, a blessing, Battle Shout) lasts
//! [`CAST_SECS`] and is cast again on everyone in reach once anyone there lacks it or has under
//! [`RECAST_LEFT`] left. An aura (Devotion and Retribution Aura, Trueshot Aura, Aspect of the Pack,
//! Blood Pact) holds on everyone in reach, a moment after they leave it. A shaman drops a Strength
//! of Earth, a Windfury and a Mana Spring Totem beside itself, real totem models, each holding its
//! buff on everyone within [`TOTEM_RANGE`]; it drops another as it strays from one or the totem's
//! time runs out, and its totems fall with it.
//!
//! The same buff never stacks: a hero holds one of each, the strongest cast on it. A caster's is
//! stronger by [`RANK_STRENGTH`] for each rank of the level-up passive of the same spell it has
//! taken (Power Word: Fortitude, Arcane Intellect, Mark of the Wild, the blessings, Aspect of the
//! Pack), as WoW's ranks and Improved talents made one priest's stronger than another's.
//!
//! A paladin brings one, by build: the stat of its highest-ranked passive card picks it. Armor,
//! health or regeneration: Devotion Aura; damage or critical strike: Blessing of Might; cooldowns
//! or experience: Blessing of Wisdom; anything else, or no card yet: Retribution Aura.

use bevy::prelude::*;

use super::super::bonus::{Bonus, Bonuses, School, StatKey};
use super::super::data::Stat;
use super::super::hero::{self, Hero};
use super::super::mechanics::level_scale;
use super::super::units::{self, Fx, UnitSpec};
use super::super::{HeroState, Run};
use super::{coop, spell, SpellRef};

/// How far a caster's buff or aura reaches, in yards.
pub(in crate::survivors) const BUFF_RANGE: f32 = 40.0;
/// How far a totem's reaches, and how far its shaman strays before dropping another, in yards.
pub(in crate::survivors) const TOTEM_RANGE: f32 = 20.0;
/// A cast buff's seconds, and what is left when it is cast again.
pub(in crate::survivors) const CAST_SECS: f32 = 60.0;
const RECAST_LEFT: f32 = 15.0;
/// How long an aura or a totem's buff holds once its bearer leaves its reach.
pub(in crate::survivors) const AURA_LINGER: f32 = 2.0;
/// How long a totem stands.
const TOTEM_SECS: f32 = 120.0;
/// How much stronger a caster's buff is for each rank of its passive.
pub(in crate::survivors) const RANK_STRENGTH: f32 = 0.2;
/// Retribution Aura's holy damage back on each blow, at run level 1.
const RETRIBUTION: f32 = 5.0;
/// Strengths closer than this are the same.
const EPS: f32 = 1e-4;

/// How a buff reaches its bearers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum BuffKind {
    Cast,
    Aura,
    /// From a totem in `display` (whose model path holds `model`), dropped by `summon`; `slot` is
    /// its place round the shaman.
    Totem {
        display: u32,
        model: &'static str,
        summon: SpellRef,
        slot: u8,
    },
}

#[derive(Debug)]
pub(in crate::survivors) struct BuffDef {
    pub key: &'static str,
    /// The buff on its bearers: name, icon, and the visual its cast and its aura slot play.
    pub spell: SpellRef,
    pub class: u8,
    pub kind: BuffKind,
    pub bonuses: &'static [Bonus],
    /// Holy damage dealt back on every blow its bearer takes, at run level 1.
    pub reflect: f32,
    /// The level-up passive whose ranks strengthen the caster's.
    pub passive: Option<&'static str>,
}

const fn buff(key: &'static str, spell: SpellRef, class: u8, kind: BuffKind) -> BuffDef {
    BuffDef {
        key,
        spell,
        class,
        kind,
        bonuses: &[],
        reflect: 0.0,
        passive: None,
    }
}

pub(in crate::survivors) static BUFFS: &[BuffDef] = &[
    BuffDef {
        bonuses: &[Bonus::new(StatKey::MaxHealth, 15.0)],
        passive: Some("fortitude"),
        ..buff(
            "fortitude",
            spell(1243, "Power Word: Fortitude", "Spell_Holy_WordFortitude"),
            5,
            BuffKind::Cast,
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Cooldown, 0.04)],
        passive: Some("intellect"),
        ..buff(
            "arcane_intellect",
            spell(1459, "Arcane Intellect", "Spell_Holy_MagicalSentry"),
            8,
            BuffKind::Cast,
        )
    },
    BuffDef {
        bonuses: &[
            Bonus::new(StatKey::MaxHealth, 5.0),
            Bonus::new(StatKey::Damage, 0.02),
            Bonus::new(StatKey::Armor, 0.02),
            Bonus::new(StatKey::MoveSpeed, 0.02),
        ],
        passive: Some("wild"),
        ..buff(
            "mark_of_the_wild",
            spell(1126, "Mark of the Wild", "Spell_Nature_Regeneration"),
            11,
            BuffKind::Cast,
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Armor, 0.06)],
        ..buff(
            "devotion_aura",
            spell(465, "Devotion Aura", "Spell_Holy_DevotionAura"),
            2,
            BuffKind::Aura,
        )
    },
    BuffDef {
        reflect: RETRIBUTION,
        ..buff(
            "retribution_aura",
            spell(7294, "Retribution Aura", "Spell_Holy_AuraOfLight"),
            2,
            BuffKind::Aura,
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Damage, 0.04)],
        passive: Some("might"),
        ..buff(
            "blessing_of_might",
            spell(19740, "Blessing of Might", "Spell_Holy_FistOfJustice"),
            2,
            BuffKind::Cast,
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Experience, 0.05)],
        passive: Some("wisdom"),
        ..buff(
            "blessing_of_wisdom",
            spell(19742, "Blessing of Wisdom", "Spell_Holy_SealOfWisdom"),
            2,
            BuffKind::Cast,
        )
    },
    BuffDef {
        bonuses: &[Bonus::school(School::Physical, 0.06)],
        ..buff(
            "battle_shout",
            spell(6673, "Battle Shout", "Ability_Warrior_BattleShout"),
            1,
            BuffKind::Cast,
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Damage, 0.03)],
        ..buff(
            "trueshot_aura",
            spell(19506, "Trueshot Aura", "Ability_TrueShot"),
            3,
            BuffKind::Aura,
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::MoveSpeed, 0.06)],
        passive: Some("pack"),
        ..buff(
            "aspect_of_the_pack",
            spell(13159, "Aspect of the Pack", "Ability_Mount_WhiteTiger"),
            3,
            BuffKind::Aura,
        )
    },
    BuffDef {
        bonuses: &[Bonus::school(School::Physical, 0.06)],
        ..buff(
            "strength_of_earth",
            spell(8076, "Strength of Earth", "Spell_Nature_EarthBindTotem"),
            7,
            BuffKind::Totem {
                display: 4588,
                model: "EarthElementalTotem",
                summon: spell(
                    8075,
                    "Strength of Earth Totem",
                    "Spell_Nature_EarthBindTotem",
                ),
                slot: 0,
            },
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Cooldown, 0.03)],
        ..buff(
            "windfury",
            spell(8516, "Windfury Totem", "Spell_Nature_Windfury"),
            7,
            BuffKind::Totem {
                display: 4590,
                model: "AirElementalTotem",
                summon: spell(8512, "Windfury Totem", "Spell_Nature_Windfury"),
                slot: 1,
            },
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::Regen, 0.5)],
        ..buff(
            "mana_spring",
            spell(5677, "Mana Spring", "Spell_Nature_ManaRegenTotem"),
            7,
            BuffKind::Totem {
                display: 4587,
                model: "WaterElementalTotem",
                summon: spell(5675, "Mana Spring Totem", "Spell_Nature_ManaRegenTotem"),
                slot: 2,
            },
        )
    },
    BuffDef {
        bonuses: &[Bonus::new(StatKey::MaxHealth, 10.0)],
        ..buff(
            "blood_pact",
            spell(6307, "Blood Pact", "Spell_Shadow_BloodBoil"),
            9,
            BuffKind::Aura,
        )
    },
];

pub(in crate::survivors) fn by_key(key: &str) -> &'static BuffDef {
    BUFFS.iter().find(|b| b.key == key).expect("a group buff")
}

/// One group buff on a hero: whose, how strong, and its seconds left.
#[derive(Clone, Copy, Debug)]
pub(in crate::survivors) struct PartyBuff {
    pub def: &'static BuffDef,
    pub strength: f32,
    pub left: f32,
    pub caster: Entity,
}

/// What `caster` brings to its party: its class's buffs, a paladin's by build.
pub(in crate::survivors) fn brings(caster: &HeroState) -> Vec<&'static BuffDef> {
    if caster.class.id == 2 {
        return vec![by_key(paladin_pick(caster))];
    }
    BUFFS
        .iter()
        .filter(|b| b.class == caster.class.id)
        .collect()
}

/// A paladin's one buff, by the stat of its highest-ranked passive (the first taken on a tie).
pub(in crate::survivors) fn paladin_pick(paladin: &HeroState) -> &'static str {
    let top = paladin
        .passives
        .iter()
        .enumerate()
        .max_by(|a, b| (a.1).1.cmp(&(b.1).1).then(b.0.cmp(&a.0)))
        .map(|(_, (p, _))| p.stat);
    match top {
        Some(Stat::Armor | Stat::MaxHp | Stat::Regen) => "devotion_aura",
        Some(Stat::Might | Stat::Crit) => "blessing_of_might",
        Some(Stat::Haste | Stat::Growth) => "blessing_of_wisdom",
        _ => "retribution_aura",
    }
}

/// How strong `caster`'s `def` is: 1, and [`RANK_STRENGTH`] more for each rank of its passive.
pub(in crate::survivors) fn strength(def: &BuffDef, caster: &HeroState) -> f32 {
    let rank = def.passive.map_or(0, |key| {
        caster
            .passives
            .iter()
            .find(|(p, _)| p.key == key)
            .map_or(0, |(_, level)| *level)
    });
    1.0 + RANK_STRENGTH * f32::from(rank)
}

/// `def` at `strength` from `caster` lands on a hero holding `buffs`, for `secs`: it never stacks
/// with itself, and a stronger one already there holds. Whether it went up fresh or replaced a
/// weaker one.
pub(in crate::survivors) fn apply(
    buffs: &mut Vec<PartyBuff>,
    def: &'static BuffDef,
    strength: f32,
    caster: Entity,
    secs: f32,
) -> bool {
    match buffs.iter_mut().find(|b| b.def.key == def.key) {
        None => {
            buffs.push(PartyBuff {
                def,
                strength,
                left: secs,
                caster,
            });
            true
        }
        Some(b) if b.strength > strength + EPS => false,
        Some(b) => {
            let fresh = b.strength < strength - EPS;
            if fresh {
                b.left = secs;
            } else {
                b.left = b.left.max(secs);
            }
            b.strength = strength;
            b.caster = caster;
            fresh
        }
    }
}

/// Whether a caster's `def` at `strength` is due on a hero holding `buffs`: it lacks it, holds a
/// weaker one, or its own runs low. A stronger one is its caster's to keep up.
fn due(buffs: &[PartyBuff], def: &BuffDef, strength: f32) -> bool {
    match buffs.iter().find(|b| b.def.key == def.key) {
        None => true,
        Some(b) => {
            b.strength < strength - EPS || (b.strength <= strength + EPS && b.left < RECAST_LEFT)
        }
    }
}

/// What `buffs` grant, each at its strength.
pub(in crate::survivors) fn bonuses(buffs: &[PartyBuff]) -> Bonuses {
    let mut out = Bonuses::default();
    for b in buffs {
        for bonus in b.def.bonuses {
            out.add(&Bonus {
                value: bonus.value * b.strength,
                ..*bonus
            });
        }
    }
    out
}

/// The holy damage a blow on `hero` draws back at run `level`, and whose it is: Retribution Aura's.
pub(in crate::survivors) fn reflect(hero: &HeroState, level: u32) -> Option<(Entity, f32)> {
    hero.synergy
        .buffs
        .iter()
        .filter(|b| b.def.reflect > 0.0)
        .map(|b| (b.caster, b.def.reflect * b.strength * level_scale(level)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Every hero's group buffs run down, and a lone hero holds none.
#[allow(clippy::type_complexity)]
fn tick(
    heroes: &mut Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    dt: f32,
    party: bool,
) {
    for (.., mut h) in heroes.iter_mut() {
        if h.synergy.buffs.is_empty() {
            continue;
        }
        // WoW's buffs fall away with their bearer.
        if !party || !h.alive() {
            h.synergy.buffs.clear();
            continue;
        }
        h.synergy.buffs.retain_mut(|b| {
            b.left -= dt;
            b.left > 0.0
        });
    }
}

/// The casters' buffs and auras on everyone standing in reach, with their visuals as they are
/// cast or go up. Max health follows: a hero's health rises with it and is held under it.
#[allow(clippy::type_complexity)]
pub(super) fn party_buffs(
    time: Res<Time>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    let party = coop(heroes.iter().count());
    let before: Vec<(Entity, f32)> = heroes
        .iter()
        .map(|(e, _, h)| (e, h.stats().max_hp))
        .collect();
    tick(&mut heroes, dt, party);
    if party {
        let standing: Vec<(Entity, Vec3)> = heroes
            .iter()
            .filter(|(.., h)| h.alive())
            .map(|(e, tf, _)| (e, tf.translation))
            .collect();
        let casts: Vec<(Entity, Vec3, &'static BuffDef, f32)> = heroes
            .iter()
            .filter(|(.., h)| h.alive())
            .flat_map(|(e, tf, h)| {
                brings(h)
                    .into_iter()
                    .filter(|d| !matches!(d.kind, BuffKind::Totem { .. }))
                    .map(move |d| (e, tf.translation, d, strength(d, h)))
            })
            .collect();
        for (caster, at, def, strength) in casts {
            let reach: Vec<Entity> = standing
                .iter()
                .filter(|(_, p)| hero::reach(at, *p) <= BUFF_RANGE)
                .map(|(e, _)| *e)
                .collect();
            match def.kind {
                BuffKind::Cast => {
                    let wanted = reach.iter().any(|&e| {
                        heroes
                            .get(e)
                            .is_ok_and(|(.., h)| due(&h.synergy.buffs, def, strength))
                    });
                    if !wanted {
                        continue;
                    }
                    for &e in &reach {
                        if let Ok((.., mut h)) = heroes.get_mut(e) {
                            apply(&mut h.synergy.buffs, def, strength, caster, CAST_SECS);
                        }
                    }
                    fx.cast(caster, def.spell.id, &reach, None);
                }
                BuffKind::Aura => {
                    let mut up = false;
                    for &e in &reach {
                        if let Ok((.., mut h)) = heroes.get_mut(e) {
                            let fresh =
                                apply(&mut h.synergy.buffs, def, strength, caster, AURA_LINGER);
                            up |= fresh && e == caster;
                        }
                    }
                    if up {
                        fx.cast(caster, def.spell.id, &reach, None);
                    }
                }
                BuffKind::Totem { .. } => {}
            }
        }
    }
    follow_max_health(&mut heroes, &before);
}

/// A hero's health rises with its max health and is held under it.
#[allow(clippy::type_complexity)]
fn follow_max_health(
    heroes: &mut Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    before: &[(Entity, f32)],
) {
    for (e, _, mut h) in heroes.iter_mut() {
        let Some(&(_, was)) = before.iter().find(|(b, _)| *b == e) else {
            continue;
        };
        if !h.alive() {
            continue;
        }
        let now = h.stats().max_hp;
        if now > was {
            h.hp += now - was;
        } else if h.hp > now {
            h.hp = now;
        }
    }
}

/// A shaman's totem, holding its buff on everyone near.
#[derive(Component)]
pub(in crate::survivors) struct PartyTotem {
    pub def: &'static BuffDef,
    pub owner: Entity,
    pub left: f32,
    pub strength: f32,
}

/// Where a totem in `slot` stands beside a shaman at `at`.
fn totem_spot(at: Vec3, slot: u8) -> Vec3 {
    let a = f32::from(slot) / 3.0 * std::f32::consts::TAU + 0.5;
    at + Vec3::new(a.cos(), 0.0, a.sin()) * 2.0
}

/// Totems hold their buffs on everyone standing near; one whose shaman fell, strayed or stands
/// alone, or whose time ran out, fades. A shaman in a party drops each totem it lacks.
#[allow(clippy::type_complexity)]
pub(super) fn party_totems(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    mut totems: Query<(Entity, &Transform, &mut PartyTotem), Without<Hero>>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    let party = coop(heroes.iter().count());
    let everyone: Vec<(Entity, Vec3, bool)> = heroes
        .iter()
        .map(|(e, tf, h)| (e, tf.translation, h.alive()))
        .collect();
    let mut standing: Vec<(&'static str, Entity)> = Vec::new();
    for (e, tf, mut totem) in &mut totems {
        totem.left -= dt;
        let pos = tf.translation;
        let keep = party
            && totem.left > 0.0
            && everyone
                .iter()
                .any(|&(h, at, up)| h == totem.owner && up && hero::reach(at, pos) <= TOTEM_RANGE);
        if !keep {
            commands
                .entity(e)
                .remove::<PartyTotem>()
                .insert(benilla_world::model_fade::DespawnFade::default());
            continue;
        }
        standing.push((totem.def.key, totem.owner));
        for &(h, at, up) in &everyone {
            if up && hero::reach(at, pos) <= TOTEM_RANGE {
                if let Ok((.., mut ally)) = heroes.get_mut(h) {
                    let (def, strength, owner) = (totem.def, totem.strength, totem.owner);
                    apply(&mut ally.synergy.buffs, def, strength, owner, AURA_LINGER);
                }
            }
        }
    }
    if !party {
        return;
    }
    for (shaman, tf, h) in heroes.iter() {
        if !h.alive() {
            continue;
        }
        for def in brings(h) {
            let BuffKind::Totem {
                display,
                summon,
                slot,
                ..
            } = def.kind
            else {
                continue;
            };
            if standing.contains(&(def.key, shaman)) {
                continue;
            }
            let pos = totem_spot(tf.translation, slot);
            fx.cast(shaman, summon.id, &[], None);
            let serial = run.guid();
            let totem = units::spawn_unit(
                &mut commands,
                &UnitSpec {
                    serial,
                    display,
                    scale: 1.0,
                    level: 60,
                    faction: units::FACTION_FRIENDLY,
                    bytes0: 0,
                    weapons: &[],
                    sheath: 0,
                },
                pos,
                0.0,
            );
            commands.entity(totem).insert(PartyTotem {
                def,
                owner: shaman,
                left: TOTEM_SECS,
                strength: strength(def, h),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::super::charpanel::{breakdown, Source};
    use super::super::super::data;
    use super::super::super::meta::MetaParts;
    use super::super::testing::{class, hero, heroes, world};
    use super::*;

    fn passive(key: &str) -> &'static data::PassiveDef {
        data::PASSIVES.iter().find(|p| p.key == key).unwrap()
    }

    fn keys(world: &World, e: Entity) -> Vec<&'static str> {
        hero(world, e)
            .synergy
            .buffs
            .iter()
            .map(|b| b.def.key)
            .collect()
    }

    #[test]
    fn buffs_land_in_a_party_on_allies_in_reach_only() {
        let mut w = world(1.0);
        let hs = heroes(
            &mut w,
            &[("Priest", 0.0), ("Warrior", 30.0), ("Rogue", 60.0)],
        );
        let base = hero(&w, hs[1]).stats().max_hp;
        w.run_system_once(party_buffs).unwrap();
        assert_eq!(keys(&w, hs[0]), ["fortitude", "battle_shout"]);
        assert_eq!(keys(&w, hs[1]), ["fortitude", "battle_shout"]);
        // The rogue is past the priest's 40 yd, but in the warrior's.
        assert_eq!(keys(&w, hs[2]), ["battle_shout"]);
        let warrior = hero(&w, hs[1]);
        assert_eq!(warrior.stats().max_hp, base + 15.0);
        assert_eq!(warrior.hp, warrior.stats().max_hp, "health rises with it");
        assert!(
            (warrior.school_mult(School::Physical) - 1.06).abs() < 1e-5,
            "Battle Shout"
        );
        // Downed, a hero loses its buffs and gets none.
        super::super::testing::down(&mut w, hs[1]);
        w.run_system_once(party_buffs).unwrap();
        assert!(keys(&w, hs[1]).is_empty());
    }

    #[test]
    fn a_lone_hero_gets_no_buffs() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Priest", 0.0)]);
        let before = hero(&w, hs[0]).stats().max_hp;
        for _ in 0..3 {
            w.run_system_once(party_buffs).unwrap();
            w.run_system_once(party_totems).unwrap();
        }
        assert!(keys(&w, hs[0]).is_empty());
        assert_eq!(hero(&w, hs[0]).stats().max_hp, before);
        let mut w = world(1.0);
        heroes(&mut w, &[("Shaman", 0.0)]);
        w.run_system_once(party_totems).unwrap();
        let totems = w.query::<&PartyTotem>().iter(&w).count();
        assert_eq!(totems, 0, "no totems alone");
    }

    #[test]
    fn a_buff_never_stacks_and_the_strongest_holds() {
        let def = by_key("fortitude");
        let (weak, strong) = (
            Entity::from_raw_u32(1).unwrap(),
            Entity::from_raw_u32(2).unwrap(),
        );
        let mut buffs = Vec::new();
        assert!(apply(&mut buffs, def, 1.0, weak, CAST_SECS));
        assert!(
            apply(&mut buffs, def, 1.4, strong, CAST_SECS),
            "a stronger one replaces it"
        );
        assert!(
            !apply(&mut buffs, def, 1.0, weak, CAST_SECS),
            "a weaker one does not"
        );
        assert_eq!(buffs.len(), 1);
        assert_eq!((buffs[0].strength, buffs[0].caster), (1.4, strong));
        let b = bonuses(&buffs);
        assert!((b.get(StatKey::MaxHealth) - 21.0).abs() < 1e-4);
        // The weaker caster has nothing to recast; the stronger one keeps it up.
        assert!(!due(&buffs, def, 1.0));
        buffs[0].left = RECAST_LEFT - 1.0;
        assert!(!due(&buffs, def, 1.0) && due(&buffs, def, 1.4));
    }

    #[test]
    fn the_strongest_priest_s_fortitude_holds_in_the_world() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Priest", 0.0), ("Priest", 5.0), ("Mage", 10.0)]);
        w.get_mut::<HeroState>(hs[1]).unwrap().passives = vec![(passive("fortitude"), 2)];
        w.run_system_once(party_buffs).unwrap();
        let mage = hero(&w, hs[2]);
        let forts: Vec<&PartyBuff> = mage
            .synergy
            .buffs
            .iter()
            .filter(|b| b.def.key == "fortitude")
            .collect();
        assert_eq!(forts.len(), 1);
        assert_eq!(forts[0].caster, hs[1]);
        assert!((forts[0].strength - 1.4).abs() < 1e-5);
    }

    #[test]
    fn a_paladin_brings_its_aura_or_blessing_by_build() {
        let mut p = HeroState::new(0, class("Paladin"), 1, Bonuses::default());
        assert_eq!(paladin_pick(&p), "retribution_aura");
        p.passives = vec![(passive("wild"), 1)];
        assert_eq!(paladin_pick(&p), "devotion_aura");
        p.passives = vec![(passive("wild"), 1), (passive("might"), 2)];
        assert_eq!(paladin_pick(&p), "blessing_of_might");
        p.passives = vec![(passive("wisdom"), 2), (passive("might"), 2)];
        assert_eq!(
            paladin_pick(&p),
            "blessing_of_wisdom",
            "the first taken on a tie"
        );
        assert_eq!(brings(&p).len(), 1);
        let rogue = HeroState::new(0, class("Rogue"), 1, Bonuses::default());
        assert!(brings(&rogue).is_empty(), "no rogue buff in 1.12");
        let hunter = HeroState::new(0, class("Hunter"), 1, Bonuses::default());
        assert_eq!(brings(&hunter).len(), 2);
    }

    #[test]
    fn retribution_aura_answers_blows_for_the_paladin() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Paladin", 0.0), ("Mage", 5.0)]);
        w.run_system_once(party_buffs).unwrap();
        let (paladin, dmg) = reflect(hero(&w, hs[1]), 1).unwrap();
        assert_eq!((paladin, dmg), (hs[0], RETRIBUTION));
        assert!(
            reflect(hero(&w, hs[1]), 11).unwrap().1 > dmg,
            "it grows with the run"
        );
    }

    #[test]
    fn a_shaman_drops_its_totems_and_they_buff_the_party_near() {
        let mut w = world(1.0);
        let hs = heroes(
            &mut w,
            &[("Shaman", 0.0), ("Warrior", 10.0), ("Mage", 35.0)],
        );
        w.run_system_once(party_totems).unwrap();
        let mut totems: Vec<u32> = w
            .query::<(&PartyTotem, &crate::net::NetEntity)>()
            .iter(&w)
            .map(|(_, n)| n.display_id.unwrap())
            .collect();
        totems.sort();
        assert_eq!(totems, [4587, 4588, 4590], "water, earth and air");
        w.run_system_once(party_totems).unwrap();
        assert_eq!(
            keys(&w, hs[1]),
            ["strength_of_earth", "windfury", "mana_spring"]
        );
        assert!(keys(&w, hs[2]).is_empty(), "out of the totems' reach");
        // The shaman falls: its totems with it.
        super::super::testing::down(&mut w, hs[0]);
        w.run_system_once(party_totems).unwrap();
        assert_eq!(w.query::<&PartyTotem>().iter(&w).count(), 0);
    }

    #[test]
    fn the_character_panel_shows_the_buffs_under_party() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Druid", 0.0), ("Priest", 5.0)]);
        w.run_system_once(party_buffs).unwrap();
        let priest = hero(&w, hs[1]);
        let rows = breakdown(priest, &MetaParts::default());
        let k = Source::ALL
            .iter()
            .position(|&s| s == Source::Party)
            .unwrap();
        let health = rows
            .iter()
            .find(|r| r.line == super::super::super::charpanel::Line::Health)
            .unwrap();
        assert!(
            (health.shares[k] - 20.0).abs() < 1e-3,
            "{:?}",
            health.shares
        );
        assert_eq!(Source::Party.label(), "Party");
        let sum: f32 = health.shares.iter().sum();
        assert!((sum - health.total).abs() < 1e-3);
    }
}
