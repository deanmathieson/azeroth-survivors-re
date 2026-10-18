//! Build synergies: a hero's chosen spells make a school package, independently of party class
//! synergy.  These are deliberately thresholds, not multiplicative stat stacks.

use super::super::bonus::School;
use super::super::data::{AbilityDef, Kind};
use super::super::HeroState;

/// A spell's gameplay role, used by the build sheet and future offer filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Projectile,
    Nova,
    Dot,
    Summon,
    Heal,
}

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

pub(super) fn name(school: School, count: u8) -> Option<&'static str> {
    match (school, count) {
        (School::Fire, 3) => Some("Combustion Mastery"),
        (School::Frost, 3) => Some("Shatter Lord"),
        (School::Shadow, 3) => Some("Shadow Weaving"),
        (School::Holy, 3) => Some("Divine Favor"),
        (School::Nature, 3) => Some("Storm Caller"),
        (School::Arcane, 3) => Some("Arcane Potency"),
        (School::Physical, 3) => Some("Blood Frenzy"),
        (_, 2) => Some("School attunement"),
        (_, 4..) => Some("School mastery"),
        _ => None,
    }
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

pub(crate) fn progress(hero: &HeroState, school: School) -> String {
    let n = count(hero, school);
    match name(school, n.min(3)) {
        Some(name) if n < 3 => format!("{} {n}/3 → {name}", school.label()),
        Some(name) => format!("{} {n}: {name}", school.label()),
        None => format!("{} {n}/2", school.label()),
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
    use super::*;
    use crate::survivors::data;

    #[test]
    fn every_ability_has_a_school_and_role() {
        for ability in data::ABILITIES {
            let _ = (school(ability), role(ability));
        }
    }
}
