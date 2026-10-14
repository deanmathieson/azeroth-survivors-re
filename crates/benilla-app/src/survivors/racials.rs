//! Racial traits: two passives per race, named for its racials in the game, granted to every run
//! the race begins. [`apply_race`] keeps [`MetaParts::race`] on the menu's pick.

use bevy::prelude::*;

use super::bonus::{Bonus, Bonuses, School, StatKey};
use super::meta::MetaParts;
use super::Selection;

/// One trait: its name and what it grants.
pub(super) struct Racial {
    pub name: &'static str,
    pub bonuses: &'static [Bonus],
}

impl Racial {
    /// What the trait grants, as tooltips say it: "+8% gold".
    pub fn effect(&self) -> String {
        let parts: Vec<String> = self.bonuses.iter().map(Bonus::describe).collect();
        parts.join(", ")
    }
}

/// A race's two traits.
pub(super) struct RaceTraits {
    /// The `ChrRaces` id, as the menu's looks carry it.
    pub race: u8,
    pub traits: [Racial; 2],
}

const fn racial(name: &'static str, bonuses: &'static [Bonus]) -> Racial {
    Racial { name, bonuses }
}

/// Every playable race's traits, each worth about one talent of three to five points.
pub(super) static RACIALS: &[RaceTraits] = &[
    RaceTraits {
        race: 1,
        traits: [
            racial("Diplomacy", &[Bonus::new(StatKey::Gold, 0.08)]),
            racial("The Human Spirit", &[Bonus::new(StatKey::Experience, 0.05)]),
        ],
    },
    RaceTraits {
        race: 2,
        traits: [
            racial("Blood Fury", &[Bonus::new(StatKey::Damage, 0.06)]),
            racial("Hardiness", &[Bonus::new(StatKey::MaxHealth, 25.0)]),
        ],
    },
    RaceTraits {
        race: 3,
        traits: [
            racial("Stoneform", &[Bonus::new(StatKey::Armor, 0.06)]),
            racial(
                "Gun Specialization",
                &[Bonus::new(StatKey::CritDamage, 0.10)],
            ),
        ],
    },
    RaceTraits {
        race: 4,
        traits: [
            racial("Quickness", &[Bonus::new(StatKey::MoveSpeed, 0.05)]),
            // Striking from hiding.
            racial("Shadowmeld", &[Bonus::new(StatKey::CritChance, 0.03)]),
        ],
    },
    RaceTraits {
        race: 5,
        traits: [
            racial("Cannibalize", &[Bonus::new(StatKey::Lifesteal, 0.02)]),
            racial("Shadow Resistance", &[Bonus::school(School::Shadow, 0.06)]),
        ],
    },
    RaceTraits {
        race: 6,
        traits: [
            racial("Endurance", &[Bonus::new(StatKey::MaxHealth, 40.0)]),
            racial("War Stomp", &[Bonus::new(StatKey::Area, 0.06)]),
        ],
    },
    RaceTraits {
        race: 7,
        traits: [
            racial("Expansive Mind", &[Bonus::new(StatKey::Cooldown, 0.06)]),
            racial("Escape Artist", &[Bonus::new(StatKey::MoveSpeed, 0.05)]),
        ],
    },
    RaceTraits {
        race: 8,
        traits: [
            racial("Berserking", &[Bonus::new(StatKey::Cooldown, 0.05)]),
            racial("Regeneration", &[Bonus::new(StatKey::Regen, 1.0)]),
        ],
    },
];

pub(super) fn traits(race: u8) -> Option<&'static RaceTraits> {
    RACIALS.iter().find(|r| r.race == race)
}

/// Both traits' bonuses summed; none for a race without traits.
pub(super) fn bonuses(race: u8) -> Bonuses {
    let mut out = Bonuses::default();
    for t in traits(race).into_iter().flat_map(|r| &r.traits) {
        out.add_all(t.bonuses);
    }
    out
}

/// The race of the look picked at the menu, once there is one.
pub(super) fn picked_race(selection: &Selection) -> Option<u8> {
    selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, race, _)| race)
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        apply_race
            .run_if(resource_changed::<Selection>)
            .after(super::menu::menu_clicks),
    );
}

/// The picked race's traits become [`MetaParts::race`], ready for the next run to start with.
fn apply_race(selection: Res<Selection>, mut parts: ResMut<MetaParts>) {
    let want = picked_race(&selection).map(bonuses).unwrap_or_default();
    if parts.race != want {
        parts.race = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_race_has_two_traits() {
        for race in 1..=8u8 {
            let r = traits(race).unwrap_or_else(|| panic!("race {race} has no traits"));
            for t in &r.traits {
                assert!(!t.name.is_empty() && !t.bonuses.is_empty(), "race {race}");
                assert!(!t.effect().is_empty());
            }
            assert_ne!(r.traits[0].name, r.traits[1].name);
            assert_ne!(bonuses(race), Bonuses::default());
        }
        assert_eq!(RACIALS.len(), 8);
        assert!(traits(0).is_none() && traits(9).is_none());
        assert_eq!(bonuses(9), Bonuses::default());
    }

    #[test]
    fn effects_read_as_tooltips() {
        let effects = |race| traits(race).unwrap().traits.each_ref().map(|t| t.effect());
        assert_eq!(effects(1), ["+8% gold", "+5% experience"]);
        assert_eq!(effects(3), ["+6% damage reduction", "+10% critical damage"]);
        assert_eq!(effects(5), ["+2% lifesteal", "+6% Shadow damage"]);
        assert_eq!(effects(7), ["-6% cooldowns", "+5% movement speed"]);
        assert_eq!(effects(8), ["-5% cooldowns", "+1 health per second"]);
    }

    #[test]
    fn the_picked_race_sets_meta_parts() {
        let mut app = App::new();
        app.init_resource::<Selection>()
            .init_resource::<MetaParts>()
            .add_systems(Update, apply_race);
        app.update();
        assert_eq!(app.world().resource::<MetaParts>().race, Bonuses::default());

        for race in 1..=8u8 {
            {
                let mut sel = app.world_mut().resource_mut::<Selection>();
                sel.looks = vec![(100, 1, 0), (200, race, 1)];
                sel.look = Some(1);
            }
            app.update();
            let parts = app.world().resource::<MetaParts>();
            assert_eq!(parts.race, bonuses(race), "race {race}");
            // The race's part alone: no other source is touched.
            assert_eq!(parts.talents, Bonuses::default());
            assert_eq!(parts.total(), bonuses(race));
        }
        // The last pick was the Troll.
        let troll = app.world().resource::<MetaParts>();
        assert_eq!(troll.race.get(StatKey::Regen), 1.0);

        // No look picked, no traits.
        app.world_mut().resource_mut::<Selection>().look = None;
        app.update();
        assert_eq!(app.world().resource::<MetaParts>().race, Bonuses::default());
    }
}
