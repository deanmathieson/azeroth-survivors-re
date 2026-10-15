//! The Wardrobe's three-slot Darkmoon toy bar, active only while a run is playing.

use bevy::prelude::*;

use super::bonus::{Bonus, School, StatKey};
use super::mechanics::{self, Aim, Cc, CcKind, Effect, MechanicId, MechanicSpec, Trigger};
use super::profile::Profile;
use super::units::Fx;
use super::{HeroState, LocalHero, Phase};

const KEYS: [KeyCode; 3] = [KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9];

#[derive(Clone, Copy)]
struct Toy {
    key: &'static str,
    name: &'static str,
    cooldown: f32,
    active: f32,
    visual: u32,
    effect: Effect,
    at: Aim,
}

const TOYS: &[Toy] = &[
    Toy {
        key: "noggenfogger_elixir",
        name: "Noggenfogger",
        cooldown: 45.0,
        active: 15.0,
        visual: 674,
        effect: Effect::Burst {
            duration: 1.0,
            bonuses: &[Bonus::new(StatKey::Regen, 5.0)],
        },
        at: Aim::Hero,
    },
    Toy {
        key: "goblin_gumbo",
        name: "Goblin Gumbo",
        cooldown: 60.0,
        active: 12.0,
        visual: 17,
        effect: Effect::Burst {
            duration: 1.0,
            bonuses: &[Bonus::new(StatKey::Armor, 0.20)],
        },
        at: Aim::Hero,
    },
    Toy {
        key: "snowball",
        name: "Snowball",
        cooldown: 30.0,
        active: 1.0,
        visual: 10,
        effect: Effect::Nova {
            school: School::Frost,
            radius: 4.0,
            damage: 12.0,
            knockback: 2.0,
            cc: Some(Cc {
                kind: CcKind::Chill(0.55),
                secs: 2.0,
            }),
        },
        at: Aim::Hero,
    },
    Toy {
        key: "savory_deviate_delight",
        name: "Deviate Delight",
        cooldown: 60.0,
        active: 15.0,
        visual: 102,
        effect: Effect::Burst {
            duration: 1.0,
            bonuses: &[Bonus::new(StatKey::MoveSpeed, 0.30)],
        },
        at: Aim::Hero,
    },
    Toy {
        key: "darkmoon_fireworks",
        name: "Fireworks",
        cooldown: 75.0,
        active: 1.0,
        visual: 115,
        effect: Effect::Nova {
            school: School::Fire,
            radius: 8.0,
            damage: 25.0,
            knockback: 5.0,
            cc: None,
        },
        at: Aim::Hero,
    },
    Toy {
        key: "toy_train_set",
        name: "Toy Train",
        cooldown: 90.0,
        active: 8.0,
        visual: 617,
        effect: Effect::Nova {
            school: School::Physical,
            radius: 6.0,
            damage: 8.0,
            knockback: 6.0,
            cc: Some(Cc {
                kind: CcKind::Fear,
                secs: 1.5,
            }),
        },
        at: Aim::Hero,
    },
    Toy {
        key: "discombobulator_ray",
        name: "Discombobulator",
        cooldown: 90.0,
        active: 8.0,
        visual: 406,
        effect: Effect::ApplyCc {
            kind: CcKind::Root,
            secs: 2.5,
        },
        at: Aim::Random { range: 18.0 },
    },
    Toy {
        key: "sayge_s_fortune",
        name: "Sayge's Fortune",
        cooldown: 120.0,
        active: 20.0,
        visual: 237,
        effect: Effect::Burst {
            duration: 1.0,
            bonuses: &[
                Bonus::new(StatKey::Damage, 0.20),
                Bonus::new(StatKey::Luck, 0.20),
            ],
        },
        at: Aim::Hero,
    },
    Toy {
        key: "orb_of_deception",
        name: "Orb of Deception",
        cooldown: 120.0,
        active: 15.0,
        visual: 167,
        effect: Effect::Burst {
            duration: 1.0,
            bonuses: &[Bonus::new(StatKey::CritChance, 0.25)],
        },
        at: Aim::Hero,
    },
    Toy {
        key: "dartol_s_rod_of_transformation",
        name: "Dartol's Rod",
        cooldown: 90.0,
        active: 15.0,
        visual: 553,
        effect: Effect::Burst {
            duration: 1.0,
            bonuses: &[Bonus::new(StatKey::Area, 0.35)],
        },
        at: Aim::Hero,
    },
];

#[derive(Clone, Copy)]
pub(super) struct ToySlot {
    toy: &'static Toy,
    cooldown: f32,
    active: f32,
}

impl ToySlot {
    pub(super) fn name(self) -> &'static str {
        self.toy.name
    }
    pub(super) fn cooldown(self) -> f32 {
        self.cooldown
    }
}

#[derive(Resource, Default)]
pub(super) struct ToyBar {
    slots: Vec<ToySlot>,
}

impl ToyBar {
    pub(super) fn slots(&self) -> &[ToySlot] {
        &self.slots
    }
}

#[derive(Component, Clone, Copy)]
pub(super) struct ToyButton(pub(super) usize);
#[derive(Component, Clone, Copy)]
pub(super) struct ToyCooldownText(pub(super) usize);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ToyBar>()
        .add_systems(OnEnter(Phase::Playing), load_toys)
        .add_systems(
            Update,
            (use_toys, tick_toys, refresh_buttons)
                .chain()
                .run_if(in_state(Phase::Playing)),
        );
}

fn load_toys(profile: Option<Res<Profile>>, mut bar: ResMut<ToyBar>) {
    bar.slots.clear();
    let Some(profile) = profile else { return };
    for key in &profile.faire.wardrobe.toys {
        let Some(toy) = TOYS.iter().find(|toy| toy.key == key) else {
            continue;
        };
        if profile.faire.toys.contains(key) {
            bar.slots.push(ToySlot {
                toy,
                cooldown: 0.0,
                active: 0.0,
            });
        }
    }
}

fn use_toys(
    keys: Res<ButtonInput<KeyCode>>,
    pressed: Query<(&Interaction, &ToyButton), Changed<Interaction>>,
    mut bar: ResMut<ToyBar>,
    mut heroes: Query<(Entity, &mut HeroState), With<LocalHero>>,
    mut fx: Fx,
) {
    let clicked: Vec<usize> = pressed
        .iter()
        .filter_map(|(interaction, button)| {
            (*interaction == Interaction::Pressed).then_some(button.0)
        })
        .collect();
    let Some(index) = KEYS
        .iter()
        .position(|key| keys.just_pressed(*key))
        .or_else(|| clicked.into_iter().next())
    else {
        return;
    };
    let Some(slot) = bar.slots.get_mut(index) else {
        return;
    };
    if slot.cooldown > 0.0 {
        return;
    }
    let Ok((entity, mut hero)) = heroes.single_mut() else {
        return;
    };
    let toy = slot.toy;
    activate(slot, &mut hero);
    fx.cast(entity, toy.visual, &[], None);
}

fn activate(slot: &mut ToySlot, hero: &mut HeroState) {
    let toy = slot.toy;
    mechanics::grant(
        hero,
        MechanicId::Other(toy.key),
        [
            MechanicSpec::new(Trigger::Periodic { every: 0.1 }, toy.effect)
                .at(toy.at)
                .visual(toy.visual),
        ],
    );
    slot.cooldown = toy.cooldown;
    slot.active = toy.active;
}

fn tick_toys(
    time: Res<Time>,
    mut bar: ResMut<ToyBar>,
    mut heroes: Query<&mut HeroState, With<LocalHero>>,
) {
    let dt = time.delta_secs();
    let Ok(mut hero) = heroes.single_mut() else {
        return;
    };
    for slot in &mut bar.slots {
        slot.cooldown = (slot.cooldown - dt).max(0.0);
        if slot.active > 0.0 {
            slot.active = (slot.active - dt).max(0.0);
            if slot.active == 0.0 {
                mechanics::revoke(&mut hero, MechanicId::Other(slot.toy.key));
            }
        }
    }
}

fn refresh_buttons(bar: Res<ToyBar>, mut texts: Query<(&ToyCooldownText, &mut Text)>) {
    for (slot, mut text) in &mut texts {
        let cooldown = bar.slots.get(slot.0).map_or(0.0, |toy| toy.cooldown());
        **text = if cooldown > 0.0 {
            format!("{cooldown:.0}s")
        } else {
            "Ready".into()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toy_use_starts_its_cooldown() {
        let toy = TOYS.iter().find(|toy| toy.key == "snowball").unwrap();
        let mut slot = ToySlot {
            toy,
            cooldown: 0.0,
            active: 0.0,
        };
        let mut hero = HeroState::default();
        activate(&mut slot, &mut hero);
        assert_eq!(slot.cooldown, 30.0);
        assert!(mechanics::has(&hero, MechanicId::Other("snowball")));
    }

    #[test]
    fn every_catalog_toy_has_a_run_effect() {
        assert_eq!(TOYS.len(), 10);
        assert!(TOYS
            .iter()
            .all(|toy| toy.cooldown >= 30.0 && toy.cooldown <= 120.0));
    }
}
