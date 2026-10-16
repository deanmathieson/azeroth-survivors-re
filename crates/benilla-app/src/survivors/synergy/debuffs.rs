//! Shared debuffs: a class's own blows put its debuff on the enemy struck, and every hero's blows
//! of the schools it opens deal more to that enemy, whoever put it there. Each shows in the
//! enemy's debuff slots (Hunter's Mark floats its red mark overhead) and plays its visual as it
//! first lands, where 1.12 gave it one.
//!
//! Sunder Armor and Expose Armor open physical damage, and as in 1.12 they do not stack: only the
//! larger counts. Faerie Fire opens it too. Curse of the Elements opens fire and frost, Curse of
//! Shadow shadow and arcane, Fire Vulnerability fire, Shadow Weaving's Shadow Vulnerability
//! shadow, Judgement of the Crusader holy, Hunter's Mark every school; Winter's Chill raises the
//! critical strike chance of frost blows. A warrior's Demoralizing Shout (an ability) weakens the
//! enemy's blows on everyone.
//!
//! The numbers are small: what one class's own debuffs add to its own blows alone stays at or
//! under [`SOLO_MOST`], so a solo run barely moves; a party's debuffs add up across classes.

use bevy::prelude::*;

use super::super::bonus::School;
use super::super::data::ClassDef;
use super::super::enemies::Enemy;
use super::super::mechanics::chance;
use super::super::units::{Fx, Marks};
use super::super::Rng;
use super::{spell, SpellRef};

/// The most one class's own debuffs raise its own blows' damage taken.
#[cfg(test)]
const SOLO_MOST: f32 = 0.05;

const PHYSICAL: &[School] = &[School::Physical];
const ANY: &[School] = &[];

/// What a debuff does to the enemy it is on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Vuln {
    /// Blows of these schools (none: every blow) deal this much more a stack.
    Taken(&'static [School], f32),
    /// Blows of these schools strike critically this much more often a stack.
    CritTaken(&'static [School], f32),
}

#[derive(Debug)]
pub(in crate::survivors) struct DebuffDef {
    pub key: &'static str,
    /// The debuff aura: its name and icon, shown in the enemy's debuff slots.
    pub spell: SpellRef,
    /// The spell whose visual plays as it first lands; `None` where 1.12 showed none.
    pub visual: Option<u32>,
    pub class: u8,
    /// The schools of the class's blows that put it on; none is any.
    pub on: &'static [School],
    pub chance: f32,
    pub stacks: u8,
    pub secs: f32,
    pub vuln: Vuln,
    /// Sunder Armor and Expose Armor: of these, only the larger counts.
    pub armor: bool,
}

pub(in crate::survivors) static DEBUFFS: &[DebuffDef] = &[
    DebuffDef {
        key: "sunder_armor",
        spell: spell(7386, "Sunder Armor", "Ability_Warrior_Sunder"),
        visual: Some(7386),
        class: 1,
        on: PHYSICAL,
        chance: 0.5,
        stacks: 5,
        secs: 15.0,
        vuln: Vuln::Taken(PHYSICAL, 0.01),
        armor: true,
    },
    DebuffDef {
        key: "expose_armor",
        spell: spell(8647, "Expose Armor", "Ability_Warrior_Riposte"),
        visual: Some(8647),
        class: 4,
        on: PHYSICAL,
        chance: 0.25,
        stacks: 1,
        secs: 30.0,
        vuln: Vuln::Taken(PHYSICAL, 0.05),
        armor: true,
    },
    DebuffDef {
        key: "faerie_fire",
        spell: spell(770, "Faerie Fire", "Spell_Nature_FaerieFire"),
        visual: Some(770),
        class: 11,
        on: ANY,
        chance: 0.2,
        stacks: 1,
        secs: 40.0,
        vuln: Vuln::Taken(PHYSICAL, 0.04),
        armor: false,
    },
    DebuffDef {
        key: "curse_of_the_elements",
        spell: spell(1490, "Curse of the Elements", "Spell_Shadow_ChillTouch"),
        visual: Some(1490),
        class: 9,
        on: &[School::Fire],
        chance: 0.3,
        stacks: 1,
        secs: 30.0,
        vuln: Vuln::Taken(&[School::Fire, School::Frost], 0.05),
        armor: false,
    },
    DebuffDef {
        key: "curse_of_shadow",
        spell: spell(17862, "Curse of Shadow", "Spell_Shadow_CurseOfAchimonde"),
        visual: Some(17862),
        class: 9,
        on: &[School::Shadow],
        chance: 0.3,
        stacks: 1,
        secs: 30.0,
        vuln: Vuln::Taken(&[School::Shadow, School::Arcane], 0.05),
        armor: false,
    },
    DebuffDef {
        key: "hunters_mark",
        spell: spell(1130, "Hunter's Mark", "Ability_Hunter_SniperShot"),
        visual: Some(1130),
        class: 3,
        on: ANY,
        chance: 0.2,
        stacks: 1,
        secs: 30.0,
        vuln: Vuln::Taken(ANY, 0.03),
        armor: false,
    },
    DebuffDef {
        key: "winters_chill",
        spell: spell(12579, "Winter's Chill", "Spell_Frost_ChillingBlast"),
        visual: None,
        class: 8,
        on: &[School::Frost],
        chance: 1.0,
        stacks: 5,
        secs: 15.0,
        vuln: Vuln::CritTaken(&[School::Frost], 0.01),
        armor: false,
    },
    DebuffDef {
        key: "fire_vulnerability",
        spell: spell(22959, "Fire Vulnerability", "Spell_Fire_SoulBurn"),
        visual: None,
        class: 8,
        on: &[School::Fire],
        chance: 0.5,
        stacks: 5,
        secs: 30.0,
        vuln: Vuln::Taken(&[School::Fire], 0.01),
        armor: false,
    },
    DebuffDef {
        key: "shadow_weaving",
        spell: spell(15258, "Shadow Vulnerability", "Spell_Shadow_BlackPlague"),
        visual: None,
        class: 5,
        on: &[School::Shadow],
        chance: 0.5,
        stacks: 5,
        secs: 15.0,
        vuln: Vuln::Taken(&[School::Shadow], 0.01),
        armor: false,
    },
    DebuffDef {
        key: "judgement_of_the_crusader",
        spell: spell(21183, "Judgement of the Crusader", "Spell_Holy_HolySmite"),
        visual: Some(20271),
        class: 2,
        on: &[School::Holy],
        chance: 0.25,
        stacks: 1,
        secs: 20.0,
        vuln: Vuln::Taken(&[School::Holy], 0.04),
        armor: false,
    },
];

/// Whether a blow of `school` is one of `schools`; none is every blow, and a blow of no known
/// school is only that.
fn reaches(schools: &[School], school: Option<School>) -> bool {
    schools.is_empty() || school.is_some_and(|s| schools.contains(&s))
}

#[derive(Clone, Copy, Debug)]
struct Active {
    def: &'static DebuffDef,
    stacks: u8,
    left: f32,
}

/// The shared debuffs on one enemy.
#[derive(Clone, Debug, Default)]
pub(in crate::survivors) struct Debuffs(Vec<Active>);

impl Debuffs {
    pub(in crate::survivors) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `def` on, or a stack more of it and its time back: whether it is new.
    pub(in crate::survivors) fn apply(&mut self, def: &'static DebuffDef) -> bool {
        match self.0.iter_mut().find(|a| a.def.key == def.key) {
            Some(a) => {
                a.stacks = (a.stacks + 1).min(def.stacks);
                a.left = def.secs;
                false
            }
            None => {
                self.0.push(Active {
                    def,
                    stacks: 1,
                    left: def.secs,
                });
                true
            }
        }
    }

    pub(in crate::survivors) fn tick(&mut self, dt: f32) {
        self.0.retain_mut(|a| {
            a.left -= dt;
            a.left > 0.0
        });
    }

    #[cfg(test)]
    pub(in crate::survivors) fn stacks(&self, key: &str) -> u8 {
        self.0
            .iter()
            .find(|a| a.def.key == key)
            .map_or(0, |a| a.stacks)
    }

    /// What a blow of `school` is multiplied by: one, and every debuff that opens its school.
    pub(in crate::survivors) fn taken(&self, school: Option<School>) -> f32 {
        let mut more = 0.0;
        let mut armor: f32 = 0.0;
        for a in &self.0 {
            if let Vuln::Taken(schools, per) = a.def.vuln {
                if reaches(schools, school) {
                    let v = per * f32::from(a.stacks);
                    if a.def.armor {
                        armor = armor.max(v);
                    } else {
                        more += v;
                    }
                }
            }
        }
        1.0 + more + armor
    }

    /// The extra chance a blow of `school` strikes critically.
    pub(in crate::survivors) fn crit_taken(&self, school: Option<School>) -> f32 {
        self.0
            .iter()
            .filter_map(|a| match a.def.vuln {
                Vuln::CritTaken(schools, per) if reaches(schools, school) => {
                    Some(per * f32::from(a.stacks))
                }
                _ => None,
            })
            .sum()
    }
}

/// The debuffs `class`'s blow of `school` puts on the enemy `target`, each on its own chance: a
/// fresh one plays its visual there, and every one shows in its debuff slots.
pub(in crate::survivors) fn afflict(
    class: &ClassDef,
    school: Option<School>,
    debuffs: &mut Debuffs,
    marks: &mut Marks,
    rng: &mut Rng,
    fx: &mut Fx,
    target: Entity,
) {
    for def in DEBUFFS
        .iter()
        .filter(|d| d.class == class.id && reaches(d.on, school))
    {
        if !chance(rng, def.chance) {
            continue;
        }
        if debuffs.apply(def) {
            if let Some(visual) = def.visual {
                fx.impact(target, visual);
            }
        }
        marks.add(def.spell.id, def.secs);
    }
}

/// The debuffs run out.
pub(super) fn tick_debuffs(time: Res<Time>, mut enemies: Query<&mut Enemy>) {
    let dt = time.delta_secs();
    for mut en in &mut enemies {
        if !en.debuffs.is_empty() {
            en.debuffs.tick(dt);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::super::abilities::{apply_hits, Hit, Origin, PendingHits};
    use super::super::super::data;
    use super::super::super::enemies::Loot;
    use super::super::super::mechanics::Procs;
    use super::super::super::HeroState;
    use super::super::testing::{class, heroes, world};
    use super::*;
    use crate::net::{Guid, ObjectStore};

    fn by_key(key: &str) -> &'static DebuffDef {
        DEBUFFS.iter().find(|d| d.key == key).unwrap()
    }

    /// A school a debuff opens.
    fn its_school(def: &DebuffDef) -> Option<School> {
        match def.vuln {
            Vuln::Taken(s, _) | Vuln::CritTaken(s, _) => {
                Some(s.first().copied().unwrap_or(School::Shadow))
            }
        }
    }

    /// A world [`apply_hits`] runs in, with a warrior, a rogue and a sturdy foe.
    fn hits_world() -> (World, Vec<Entity>, Entity) {
        let mut w = world(1.0);
        w.init_resource::<PendingHits>();
        w.init_resource::<Procs>();
        w.init_resource::<Messages<Hit>>();
        w.init_resource::<Messages<Loot>>();
        let hs = heroes(&mut w, &[("Warrior", 0.0), ("Rogue", 2.0)]);
        let foe = Enemy {
            def: &data::roster(0).enemies[0],
            hp: 1e9,
            max_hp: 1e9,
            speed: 0.0,
            damage: 0.0,
            radius: 0.6,
            xp: 1,
            boss: false,
            elite: false,
            guardian: false,
            dormant: false,
            attack_cd: 0.0,
            slow: 1.0,
            slow_t: 0.0,
            root_t: 0.0,
            stun_t: 0.0,
            fear_t: 0.0,
            weaken: 1.0,
            weaken_t: 0.0,
            dots: Vec::new(),
            dead: false,
            taunted: None,
            debuffs: Debuffs::default(),
        };
        let body = (
            Guid(99),
            Transform::default(),
            ObjectStore(Default::default()),
            Marks::debuffs(),
        );
        let foe = w.spawn((foe, body)).id();
        (w, hs, foe)
    }

    /// The damage `hits` of `school` for 10 each from `source` deal the foe.
    fn dealt(w: &mut World, source: Entity, foe: Entity, school: Option<School>, n: usize) -> f32 {
        let done = |w: &World| w.get::<HeroState>(source).unwrap().damage_done;
        let before = done(w);
        for _ in 0..n {
            w.write_message(Hit {
                school,
                ..Hit::direct(source, foe, 10.0, false)
            });
        }
        w.run_system_once(apply_hits).unwrap();
        // A fresh reader each run would read them again.
        w.resource_mut::<Messages<Hit>>().clear();
        (done(w) - before) as f32
    }

    #[test]
    fn each_debuff_raises_damage_taken_from_another_hero() {
        for def in DEBUFFS {
            let (mut w, hs, foe) = hits_world();
            // The rogue strikes, never the class that put it on (unless it is the rogue's own).
            let school = its_school(def);
            let n = 400;
            let plain = dealt(&mut w, hs[1], foe, school, n);
            {
                let mut en = w.get_mut::<Enemy>(foe).unwrap();
                for _ in 0..def.stacks {
                    en.debuffs.apply(def);
                }
            }
            let debuffed = dealt(&mut w, hs[1], foe, school, n);
            assert!(
                debuffed > plain * 1.005,
                "{}: {plain} -> {debuffed}",
                def.key
            );
            // A blow of a school it does not open is untouched, unless it opens every one.
            if let Vuln::Taken(schools, _) = def.vuln {
                if !schools.is_empty() {
                    let other = [School::Nature, School::Holy, School::Physical]
                        .into_iter()
                        .find(|s| !schools.contains(s))
                        .unwrap();
                    let same = dealt(&mut w, hs[1], foe, Some(other), 10);
                    assert!((same - 100.0).abs() < 1e-3, "{}: {same}", def.key);
                }
            }
        }
    }

    #[test]
    fn a_class_s_blows_put_its_debuffs_on() {
        let (mut w, hs, foe) = hits_world();
        let def = &data::ABILITIES[0];
        let sunder = || Hit {
            origin: Origin::Ability(def),
            school: Some(School::Physical),
            ..Hit::direct(Entity::PLACEHOLDER, Entity::PLACEHOLDER, 1.0, false)
        };
        for _ in 0..40 {
            w.write_message(Hit {
                source: hs[0],
                target: foe,
                ..sunder()
            });
        }
        w.run_system_once(apply_hits).unwrap();
        let en = w.get::<Enemy>(foe).unwrap();
        assert_eq!(en.debuffs.stacks("sunder_armor"), 5, "stacks to five");
        assert_eq!(
            en.debuffs.stacks("expose_armor"),
            0,
            "the warrior's own only"
        );
        assert!((en.debuffs.taken(Some(School::Physical)) - 1.05).abs() < 1e-5);
        // A mage's frost bolt is no warrior's.
        let mut d = Debuffs::default();
        let mut marks = Marks::debuffs();
        let mut rng = Rng(3);
        w.run_system_once(move |mut fx: Fx| {
            afflict(
                class("Mage"),
                Some(School::Frost),
                &mut d,
                &mut marks,
                &mut rng,
                &mut fx,
                Entity::PLACEHOLDER,
            );
            assert_eq!(d.stacks("winters_chill"), 1);
            assert_eq!(d.stacks("sunder_armor"), 0);
        })
        .unwrap();
    }

    #[test]
    fn sunder_and_expose_armor_do_not_stack() {
        let mut d = Debuffs::default();
        for _ in 0..5 {
            d.apply(by_key("sunder_armor"));
        }
        d.apply(by_key("expose_armor"));
        d.apply(by_key("faerie_fire"));
        // The larger of the two armor debuffs, and Faerie Fire on top.
        assert!((d.taken(Some(School::Physical)) - 1.09).abs() < 1e-5);
        assert_eq!(d.taken(Some(School::Fire)), 1.0);
        assert_eq!(d.taken(None), 1.0, "a blow of no known school");
        d.tick(16.0);
        assert!(
            (d.taken(Some(School::Physical)) - 1.09).abs() < 1e-5,
            "Sunder ran out"
        );
        d.tick(30.0);
        assert!(d.is_empty());
    }

    #[test]
    fn a_class_s_own_debuffs_barely_move_its_solo_damage() {
        let schools = [
            School::Physical,
            School::Holy,
            School::Fire,
            School::Nature,
            School::Frost,
            School::Shadow,
            School::Arcane,
        ];
        for class in data::CLASSES {
            let mut d = Debuffs::default();
            for def in DEBUFFS.iter().filter(|d| d.class == class.id) {
                for _ in 0..def.stacks {
                    d.apply(def);
                }
            }
            for s in schools {
                // A critical strike doubles a blow: its extra chance is that much damage.
                let more = d.taken(Some(s)) - 1.0 + d.crit_taken(Some(s));
                assert!(more <= SOLO_MOST + 1e-5, "{} {s:?}: {more}", class.name);
            }
        }
    }
}
