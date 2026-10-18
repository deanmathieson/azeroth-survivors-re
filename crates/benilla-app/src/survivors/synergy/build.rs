//! Build synergies: a hero's chosen spells make a school package, independently of party class
//! synergy.  These are deliberately thresholds, not multiplicative stat stacks.

use super::super::bonus::{Bonus, School, StatKey};
use super::super::data::AbilityDef;
#[cfg(test)]
use super::super::data::Kind;
use super::super::mechanics::{self, Aim, Effect, MechanicId, MechanicSpec, Of, Trigger};
use super::super::HeroState;

/// A spell's gameplay role, used by the build sheet and future offer filters.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Projectile,
    Nova,
    Dot,
    Summon,
    Heal,
}

#[cfg(test)]
pub(super) fn role(def: &AbilityDef) -> Role {
    match def.kind {
        Kind::Bolt | Kind::Strike | Kind::Melee | Kind::Chain | Kind::Orbit => Role::Projectile,
        Kind::Nova | Kind::Cone | Kind::Ground | Kind::GroundSelf | Kind::Aura => Role::Nova,
        Kind::Dot => Role::Dot,
        Kind::Totem => Role::Summon,
        Kind::Heal | Kind::Shield | Kind::Immune | Kind::Thorns => Role::Heal,
    }
}

/// The 1.12 spell school's build tag.  Physical is the intentional conservative fallback for
/// weapon attacks and the few hybrid spells, rather than leaving an ability untagged.
pub(crate) fn school(def: &AbilityDef) -> School {
    match def.key {
        k if k.contains("fire")
            || matches!(
                k,
                "pyroblast" | "flamestrike" | "immolate" | "hellfire" | "searing_totem"
            ) =>
        {
            School::Fire
        }
        k if k.contains("frost") || matches!(k, "ice_lance" | "blizzard" | "cone_of_cold") => {
            School::Frost
        }
        k if k.contains("shadow")
            || matches!(
                k,
                "mind_blast" | "mind_flay" | "corruption" | "drain_life" | "curse_of_agony"
            ) =>
        {
            School::Shadow
        }
        k if k.contains("arcane")
            || matches!(k, "starfire" | "arcane_missiles" | "arcane_explosion") =>
        {
            School::Arcane
        }
        k if k.contains("nature")
            || matches!(
                k,
                "lightning_bolt" | "chain_lightning" | "wrath" | "insect_swarm"
            ) =>
        {
            School::Nature
        }
        k if k.contains("holy")
            || matches!(k, "smite" | "exorcism" | "holy_nova" | "judgement") =>
        {
            School::Holy
        }
        _ => School::Physical,
    }
}

/// A school's three-piece set, by its WoW name.
pub(super) fn set_name(school: School) -> &'static str {
    match school {
        School::Fire => "Combustion Mastery",
        School::Frost => "Shatter Lord",
        School::Shadow => "Shadow Weaving",
        School::Holy => "Divine Favor",
        School::Nature => "Storm Caller",
        School::Arcane => "Arcane Potency",
        School::Physical => "Blood Frenzy",
    }
}

/// What a school's three-piece set does, as its line reads.
pub(super) fn set_text(school: School) -> &'static str {
    match school {
        School::Fire => "25% on kill: burns spread to 2 foes near",
        School::Frost => "Frozen foes take 30% more from your blows",
        School::Shadow => "+15% Shadow damage",
        School::Holy => "20% on crit: heal allies near for 8",
        School::Nature => "25% on crit: lightning chains to 2 foes",
        School::Arcane => "Cooldowns 10% faster",
        School::Physical => "30% on kill: 10% faster cooldowns for 4 sec",
    }
}

/// The school's three-piece set `hero` holds, and what it does: "Shatter Lord: frozen foes ...".
/// `None` below three of the school.
pub(crate) fn set_line(hero: &HeroState, school: School) -> Option<String> {
    (tier(count(hero, school)) >= 3).then(|| format!("{}: {}", set_name(school), set_text(school)))
}

/// The set tier `count` abilities (and jewelry) of a school reach: none, 2 (Attunement), 3 (its
/// named set) or 4 (Mastery).
pub(super) fn tier(count: u8) -> u8 {
    match count {
        0 | 1 => 0,
        2 => 2,
        3 => 3,
        _ => 4,
    }
}

/// Each of Attunement (2) and Mastery (4): this much more damage of the school.
pub(super) const TIER_DAMAGE: f32 = 0.06;

/// The stat bonuses a school's set grants at `tier`, all its tiers so far together: +6% of the
/// school's damage at 2 and again at 4, and the stat-only three-piece sets (Arcane's cooldowns,
/// Shadow's damage).
pub(super) fn tier_bonuses(school: School, tier: u8) -> Vec<Bonus> {
    let mut out = Vec::new();
    if tier >= 2 {
        out.push(Bonus::school(school, TIER_DAMAGE));
    }
    if tier >= 3 {
        match school {
            School::Arcane => out.push(Bonus::new(StatKey::Cooldown, 0.10)),
            School::Shadow => out.push(Bonus::school(School::Shadow, 0.15)),
            _ => {}
        }
    }
    if tier >= 4 {
        out.push(Bonus::school(school, TIER_DAMAGE));
    }
    out
}

/// The mechanic a school's three-piece set carries, through the shared toolkit; none for the
/// stat-only sets ([`tier_bonuses`]).
pub(super) fn set_mechanic(school: School) -> &'static [MechanicSpec] {
    const COMBUSTION: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnKill {
            of: Of::Any,
            chance: 0.25,
        },
        Effect::Dot {
            school: School::Fire,
            dps: 6.0,
            secs: 4.0,
        },
    )
    .at(Aim::Spread { radius: 6.0, n: 2 })
    .visual(348)
    .cooldown(1.5)];
    const SHATTER: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnHit {
            of: Of::Any,
            chance: 1.0,
        },
        Effect::Shatter { bonus: 0.30 },
    )];
    const DIVINE_FAVOR: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnCrit {
            of: Of::Any,
            chance: 0.20,
        },
        Effect::HealPulse {
            radius: 10.0,
            amount: 8.0,
        },
    )
    .at(Aim::Hero)
    .visual(20216)
    .cooldown(2.0)];
    const STORM_CALLER: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnCrit {
            of: Of::Any,
            chance: 0.25,
        },
        Effect::Chain {
            jumps: 2,
            falloff: 0.6,
        },
    )
    .visual(421)
    .cooldown(1.5)];
    const BLOOD_FRENZY: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnKill {
            of: Of::Any,
            chance: 0.30,
        },
        Effect::Burst {
            duration: 4.0,
            bonuses: &[Bonus::new(StatKey::Cooldown, 0.10)],
        },
    )
    .visual(16257)
    .cooldown(4.0)];
    match school {
        School::Fire => &COMBUSTION,
        School::Frost => &SHATTER,
        School::Holy => &DIVINE_FAVOR,
        School::Nature => &STORM_CALLER,
        School::Physical => &BLOOD_FRENZY,
        School::Arcane | School::Shadow => &[],
    }
}

/// The schools, in [`School`]'s order: a set's place in the hero's applied sets.
pub(super) const SCHOOLS: [School; 7] = [
    School::Physical,
    School::Holy,
    School::Fire,
    School::Nature,
    School::Frost,
    School::Shadow,
    School::Arcane,
];

/// The mechanic id a school's set grants under.
fn set_id(school: School) -> MechanicId {
    MechanicId::Other(match school {
        School::Physical => "set_physical",
        School::Holy => "set_holy",
        School::Fire => "set_fire",
        School::Nature => "set_nature",
        School::Frost => "set_frost",
        School::Shadow => "set_shadow",
        School::Arcane => "set_arcane",
    })
}

/// `hero`'s school sets brought in line with its build: each school whose tier changed has its old
/// tier's bonuses taken back and its new tier's put on, and its three-piece mechanic granted or
/// revoked. Called wherever the build's spells or jewelry change; the host's alone, as the
/// bonuses travel with the build. Whether anything changed.
pub(crate) fn sync_sets(hero: &mut HeroState) -> bool {
    let mut changed = false;
    for (i, school) in SCHOOLS.into_iter().enumerate() {
        let want = tier(count(hero, school));
        let had = hero.synergy.sets[i];
        if want == had {
            continue;
        }
        for b in tier_bonuses(school, had) {
            hero.bonuses.add(&Bonus {
                value: -b.value,
                ..b
            });
        }
        for b in tier_bonuses(school, want) {
            hero.bonuses.add(&b);
        }
        let specs = set_mechanic(school);
        if want >= 3 && !specs.is_empty() {
            mechanics::grant(hero, set_id(school), specs.iter().copied());
        } else {
            mechanics::revoke(hero, set_id(school));
        }
        hero.synergy.sets[i] = want;
        changed = true;
    }
    changed
}

/// Every school set's mechanic, for the data test of their visuals.
#[cfg(test)]
pub(crate) fn every_set_mechanic() -> Vec<(&'static str, &'static [MechanicSpec])> {
    SCHOOLS
        .into_iter()
        .map(|s| (set_name(s), set_mechanic(s)))
        .filter(|(_, specs)| !specs.is_empty())
        .collect()
}

/// Ability count for `school`; jewelry may contribute one thematic school through its proc key.
pub(super) fn count(hero: &HeroState, school: School) -> u8 {
    let spells = hero
        .abilities
        .iter()
        .filter(|a| self::school(a.def) == school)
        .count();
    let jewelry = hero
        .jewelry
        .worn()
        .filter(|p| jewelry_school(p.key) == Some(school))
        .count();
    (spells + jewelry).min(u8::MAX as usize) as u8
}

/// A school's set progress, as a card and the Character panel show it: the next tier and what it
/// brings, or the sets held.
pub(crate) fn progress(hero: &HeroState, school: School) -> String {
    let n = count(hero, school);
    let label = school.label();
    match tier(n) {
        0 => format!("{label} {n}/2 \u{2192} {label} Attunement"),
        2 => format!("{label} 2/3 \u{2192} {}", set_name(school)),
        3 => format!(
            "{label} 3/4 \u{2192} {label} Mastery ({})",
            set_name(school)
        ),
        _ => format!("{label} {n}: {} and Mastery", set_name(school)),
    }
}

fn jewelry_school(key: &str) -> Option<School> {
    match key {
        "talisman_of_ephemeral_power" | "zandalarian_hero_charm" | "ring_of_spell_power" => {
            Some(School::Arcane)
        }
        "choker_of_the_fire_lord" => Some(School::Fire),
        "eye_of_moam" => Some(School::Nature),
        "mark_of_the_champion" => Some(School::Holy),
        "painweaver_band" => Some(School::Shadow),
        "hand_of_justice" | "band_of_accuria" | "blackhands_breadth" => Some(School::Physical),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::Owned;
    use super::*;
    use crate::survivors::data;

    #[test]
    fn every_ability_has_a_school_and_role() {
        for ability in data::ABILITIES {
            let _ = (school(ability), role(ability));
        }
    }

    fn of_school(school: School) -> Vec<&'static AbilityDef> {
        data::ABILITIES
            .iter()
            .filter(|a| self::school(a) == school)
            .collect()
    }

    #[test]
    fn school_sets_come_and_go_with_the_build() {
        let fire = of_school(School::Fire);
        assert!(fire.len() >= 4, "enough Fire spells to test the sets");
        let mut hero = HeroState::default();
        hero.abilities.clear();
        let base = hero.school_mult(School::Fire);
        let id = set_id(School::Fire);
        let add = |hero: &mut HeroState, def: &'static AbilityDef| {
            hero.abilities.push(Owned {
                def,
                level: 1,
                cd: 0.0,
            });
            sync_sets(hero)
        };
        assert!(!add(&mut hero, fire[0]), "one is no set");
        assert!(add(&mut hero, fire[1]), "two: Attunement");
        assert!((hero.school_mult(School::Fire) - (base + TIER_DAMAGE)).abs() < 1e-4);
        assert!(!mechanics::has(&hero, id));
        assert!(add(&mut hero, fire[2]), "three: Combustion Mastery");
        assert!(mechanics::has(&hero, id));
        assert!(add(&mut hero, fire[3]), "four: Mastery");
        assert!((hero.school_mult(School::Fire) - (base + 2.0 * TIER_DAMAGE)).abs() < 1e-4);
        assert!(!sync_sets(&mut hero), "nothing more to do");
        // Losing them takes it all back.
        hero.abilities.truncate(1);
        assert!(sync_sets(&mut hero));
        assert!((hero.school_mult(School::Fire) - base).abs() < 1e-4);
        assert!(!mechanics::has(&hero, id));
        assert_eq!(hero.synergy.sets, [0; 7]);
    }

    #[test]
    fn arcane_potency_is_ten_percent_faster_cooldowns() {
        assert!(tier_bonuses(School::Arcane, 2)
            .iter()
            .all(|b| b.stat == StatKey::Damage));
        assert!(tier_bonuses(School::Arcane, 3)
            .iter()
            .any(|b| b.stat == StatKey::Cooldown && (b.value - 0.10).abs() < 1e-6));
        assert_eq!(tier_bonuses(School::Fire, 4).len(), 2);
        assert!(tier_bonuses(School::Holy, 1).is_empty());
        // Every school's set says what it does, and the card shows where the build stands.
        for school in SCHOOLS {
            assert!(!set_text(school).is_empty() && !set_name(school).is_empty());
        }
        let hero = HeroState::default();
        assert!(progress(&hero, School::Frost).contains("Attunement"));
    }
}
