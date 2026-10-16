//! Tanks, in a party: a warrior, a paladin or a druid fights in its stance or form (Defensive
//! Stance, Righteous Fury, Bear Form), which raises its threat [`TANK_THREAT`] times, so the horde
//! picks it over an ally a little nearer ([`super::super::party::pick_target`]); the stance shows
//! as its aura and plays its visual as it goes up. Its taunt forces enemies onto it for a few
//! seconds (`Enemy::taunted`): a warrior's Taunt and a druid's Growl pull one enemy, a boss or an
//! elite first, every [`TAUNT_COOLDOWN`].
//!
//! A paladin's taunt is an area one, as the brief's Righteous Defense would be; that spell came
//! with The Burning Crusade and has no 1.12 row, so the paladin's pulse is its 1.12 threat spell,
//! Righteous Fury: every [`PULSE_COOLDOWN`] it pulls every enemy within [`PULSE_RADIUS`].

use bevy::prelude::*;

use super::super::data::ClassDef;
use super::super::enemies::Enemy;
use super::super::hero::{self, Hero};
use super::super::units::Fx;
use super::super::{HeroState, Run};
use super::{coop, spell, SpellRef};

/// How much a tank's stance multiplies its threat.
pub(in crate::survivors) const TANK_THREAT: f32 = 3.0;
/// A single taunt's reach, cooldown and hold, in yards and seconds.
pub(in crate::survivors) const TAUNT_RANGE: f32 = 15.0;
pub(in crate::survivors) const TAUNT_COOLDOWN: f32 = 8.0;
pub(in crate::survivors) const TAUNT_SECS: f32 = 4.0;
/// The paladin's area pulse: radius, cooldown and hold.
pub(in crate::survivors) const PULSE_RADIUS: f32 = 10.0;
pub(in crate::survivors) const PULSE_COOLDOWN: f32 = 10.0;
pub(in crate::survivors) const PULSE_SECS: f32 = 3.0;

pub(in crate::survivors) struct StanceDef {
    pub class: u8,
    pub spell: SpellRef,
}

pub(in crate::survivors) static STANCES: &[StanceDef] = &[
    StanceDef {
        class: 1,
        spell: spell(71, "Defensive Stance", "Ability_Warrior_DefensiveStance"),
    },
    StanceDef {
        class: 2,
        spell: spell(25780, "Righteous Fury", "Spell_Holy_SealOfFury"),
    },
    StanceDef {
        class: 11,
        spell: spell(5487, "Bear Form", "Ability_Racial_BearForm"),
    },
];

pub(in crate::survivors) struct TauntDef {
    pub class: u8,
    pub spell: SpellRef,
    /// Every enemy within [`PULSE_RADIUS`], not one within [`TAUNT_RANGE`].
    pub area: bool,
}

pub(in crate::survivors) static TAUNTS: &[TauntDef] = &[
    TauntDef {
        class: 1,
        spell: spell(355, "Taunt", "Spell_Nature_Reincarnation"),
        area: false,
    },
    TauntDef {
        class: 11,
        spell: spell(6795, "Growl", "Ability_Physical_Taunt"),
        area: false,
    },
    TauntDef {
        class: 2,
        spell: spell(25780, "Righteous Fury", "Spell_Holy_SealOfFury"),
        area: true,
    },
];

/// Every spell this module plays, for the data tests.
#[cfg(test)]
pub(super) fn all_spells() -> impl Iterator<Item = SpellRef> {
    STANCES
        .iter()
        .map(|s| s.spell)
        .chain(TAUNTS.iter().map(|t| t.spell))
}

/// A tank class's stance or form.
pub(in crate::survivors) fn stance(class: &ClassDef) -> Option<&'static StanceDef> {
    STANCES.iter().find(|s| s.class == class.id)
}

fn taunt_of(class: &ClassDef) -> Option<&'static TauntDef> {
    TAUNTS.iter().find(|t| t.class == class.id)
}

/// A hero's threat: a tank's stance's in a party, else 1.
pub(in crate::survivors) fn threat(class: &ClassDef, party: bool) -> f32 {
    if party && stance(class).is_some() {
        TANK_THREAT
    } else {
        1.0
    }
}

/// Every hero's threat; a tank's stance goes up in a party, its visual with it, and comes down
/// with the hero.
pub(super) fn stances(mut heroes: Query<(Entity, &mut HeroState), With<Hero>>, mut fx: Fx) {
    let party = coop(heroes.iter().count());
    for (e, mut h) in &mut heroes {
        let threat = threat(h.class, party);
        if h.threat != threat {
            h.threat = threat;
        }
        let up = party && h.alive() && stance(h.class).is_some();
        if up != h.synergy.stance {
            h.synergy.stance = up;
            if let (true, Some(s)) = (up, stance(h.class)) {
                fx.cast(e, s.spell.id, &[e], None);
            }
        }
    }
}

/// A tank standing in a party taunts as its cooldown comes round: its single taunt the boss or
/// elite in reach first, else the nearest enemy not already its own; the paladin's pulse every
/// enemy near.
#[allow(clippy::type_complexity)]
pub(in crate::survivors) fn taunts(
    time: Res<Time>,
    run: Res<Run>,
    mut heroes: Query<(Entity, &Transform, &mut HeroState), With<Hero>>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy), Without<Hero>>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    if !coop(heroes.iter().count()) {
        return;
    }
    let now = run.elapsed;
    for (tank, tf, mut h) in &mut heroes {
        let Some(def) = taunt_of(h.class) else {
            continue;
        };
        h.synergy.taunt_cd = (h.synergy.taunt_cd - dt).max(0.0);
        if !h.alive() || h.synergy.taunt_cd > 0.0 {
            continue;
        }
        let at = tf.translation;
        let mine = |en: &Enemy| {
            en.taunted
                .is_some_and(|(e, until)| e == tank && until > now)
        };
        let mut near: Vec<(Entity, f32, bool)> = enemies
            .iter()
            .filter(|(_, _, en)| !en.dead && !mine(en))
            .map(|(e, etf, en)| (e, hero::reach(at, etf.translation), en.boss || en.elite))
            .filter(|&(_, d, _)| d <= if def.area { PULSE_RADIUS } else { TAUNT_RANGE })
            .collect();
        if near.is_empty() {
            continue;
        }
        let secs = if def.area {
            PULSE_SECS
        } else {
            near.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.total_cmp(&b.1)));
            near.truncate(1);
            TAUNT_SECS
        };
        let pulled: Vec<Entity> = near.iter().map(|n| n.0).collect();
        for &e in &pulled {
            if let Ok((.., mut en)) = enemies.get_mut(e) {
                en.taunted = Some((tank, now + secs));
            }
        }
        fx.cast(tank, def.spell.id, &pulled, None);
        h.synergy.taunt_cd = if def.area {
            PULSE_COOLDOWN
        } else {
            TAUNT_COOLDOWN
        };
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::testing::{class, hero, heroes, world};
    use super::*;

    #[test]
    fn tanks_raise_their_threat_in_a_party_only() {
        let mut w = world(1.0);
        let hs = heroes(
            &mut w,
            &[
                ("Warrior", 0.0),
                ("Paladin", 1.0),
                ("Druid", 2.0),
                ("Mage", 3.0),
            ],
        );
        w.run_system_once(stances).unwrap();
        let threats: Vec<f32> = hs.iter().map(|&e| hero(&w, e).threat).collect();
        assert_eq!(threats, [TANK_THREAT, TANK_THREAT, TANK_THREAT, 1.0]);
        assert!(hero(&w, hs[0]).synergy.stance && !hero(&w, hs[3]).synergy.stance);
        let mut shown = Vec::new();
        super::super::auras(hero(&w, hs[2]), &mut shown);
        assert_eq!(shown.first(), Some(&5487), "Bear Form shows");
        let mut w = world(1.0);
        let lone = heroes(&mut w, &[("Warrior", 0.0)]);
        w.run_system_once(stances).unwrap();
        assert_eq!(hero(&w, lone[0]).threat, 1.0);
        assert_eq!(threat(class("Rogue"), true), 1.0);
    }
}
