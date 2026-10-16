//! Class resurrections, in a party only, on top of an ally's channel beside the downed
//! ([`super::super::party::revive_downed`]).
//!
//! - **Resurrection, Redemption, Ancestral Spirit**: a priest, paladin or shaman standing casts
//!   its own on an ally down at least [`RES_WAIT`] within [`RES_RANGE`]: a [`RES_CAST`] cast with
//!   the spell's precast and its sound, then up at half health. [`RES_COOLDOWN`] between.
//! - **Rebirth**: a druid's, the same but on an ally just down too, [`REBIRTH_COOLDOWN`] between.
//! - **Soulstone Resurrection**: once a run a warlock puts a soulstone on an ally in reach (a
//!   healer first). Downed, that ally gets up with it: the local player by pressing
//!   [`SELF_RES_KEY`], anyone [`SELF_RES_AUTO`] after falling. Even with the whole party down.
//! - **Reincarnation**: the ankh, a shaman's own: the same rising, once every [`ANKH_COOLDOWN`].
//!
//! Each shows as combat text over the raised: "Player 1 casts Resurrection on Player 2".

use bevy::prelude::*;

use super::super::data::ClassDef;
use super::super::hero::{self, Hero};
use super::super::party::{self, is_healer};
use super::super::partyui::MemberName;
use super::super::units::{self, Fx, Marks, COLOR_HEAL};
use super::super::{HeroState, LocalHero};
use super::{coop, label, spell, SpellRef};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// How far a resurrection or a soulstone reaches, in yards.
pub(in crate::survivors) const RES_RANGE: f32 = 30.0;
/// How long an ally lies down before all but Rebirth reach it, so its allies' channel comes first.
pub(in crate::survivors) const RES_WAIT: f32 = 4.0;
/// A resurrection's cast, in seconds.
pub(in crate::survivors) const RES_CAST: f32 = 2.0;
pub(in crate::survivors) const RES_COOLDOWN: f32 = 90.0;
pub(in crate::survivors) const REBIRTH_COOLDOWN: f32 = 120.0;
/// A soulstone or an ankh raises its hero this long after it fell, unless pressed for sooner.
pub(in crate::survivors) const SELF_RES_AUTO: f32 = 2.0;
pub(in crate::survivors) const ANKH_COOLDOWN: f32 = 300.0;
/// The local player's key to rise with a soulstone or the ankh.
pub(in crate::survivors) const SELF_RES_KEY: KeyCode = KeyCode::Space;

pub(in crate::survivors) const SOULSTONE: SpellRef =
    spell(20707, "Soulstone Resurrection", "Spell_Shadow_SoulGem");
pub(in crate::survivors) const REINCARNATION: SpellRef =
    spell(21169, "Reincarnation", "Spell_Nature_Reincarnation");

pub(in crate::survivors) struct ResDef {
    pub class: u8,
    pub spell: SpellRef,
    pub cooldown: f32,
    /// Reaches an ally just down (Rebirth), not only one down [`RES_WAIT`].
    pub fresh: bool,
}

pub(in crate::survivors) static RES: &[ResDef] = &[
    ResDef {
        class: 5,
        spell: spell(2006, "Resurrection", "Spell_Holy_Resurrection"),
        cooldown: RES_COOLDOWN,
        fresh: false,
    },
    ResDef {
        class: 2,
        spell: spell(7328, "Redemption", "Spell_Holy_Resurrection"),
        cooldown: RES_COOLDOWN,
        fresh: false,
    },
    ResDef {
        class: 7,
        spell: spell(2008, "Ancestral Spirit", "Spell_Nature_Regenerate"),
        cooldown: RES_COOLDOWN,
        fresh: false,
    },
    ResDef {
        class: 11,
        spell: spell(20484, "Rebirth", "Spell_Nature_Reincarnation"),
        cooldown: REBIRTH_COOLDOWN,
        fresh: true,
    },
];

/// Every spell this module plays, for the data tests.
#[cfg(test)]
pub(super) fn all_spells() -> impl Iterator<Item = SpellRef> {
    RES.iter()
        .map(|r| r.spell)
        .chain([SOULSTONE, REINCARNATION])
}

pub(in crate::survivors) fn res_of(class: &ClassDef) -> Option<&'static ResDef> {
    RES.iter().find(|r| r.class == class.id)
}

/// One hero's resurrections.
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::survivors) struct ResState {
    /// Seconds before its class resurrection is ready again.
    pub cd: f32,
    /// The ally its resurrection is on, and the cast's seconds left.
    pub casting: Option<(Entity, f32)>,
    /// A warlock has given the run's soulstone.
    pub soulstone_given: bool,
    /// It carries a soulstone.
    pub soulstone: bool,
    /// Seconds before a shaman's ankh is ready again.
    pub ankh_cd: f32,
    /// Its player pressed [`SELF_RES_KEY`] on another machine (a co-op client's, sent to the
    /// host): it gets up as the local player's key would get it up.
    pub asked: bool,
}

/// A downed hero up at [`party::REVIVED_HP`], standing again on screen.
fn raise(
    e: Entity,
    guid: &Guid,
    store: &mut ObjectStore,
    marks: &mut Marks,
    hero: &mut HeroState,
    fields: &mut MessageWriter<FieldChanged>,
) {
    party::get_up(hero);
    units::raise_unit(store);
    marks.clear();
    marks.flush(e, guid.0, store, fields);
}

/// A hero's body and state, as the resurrections raise it.
type Body = (
    Entity,
    &'static Guid,
    &'static Transform,
    &'static mut ObjectStore,
    &'static mut Marks,
    Option<&'static MemberName>,
    &'static mut HeroState,
);

/// Once a run, a warlock in a party puts a soulstone on an ally in reach: a healer first, else
/// the nearest.
pub(super) fn soulstone(mut heroes: Query<Body, With<Hero>>, mut fx: Fx) {
    if !coop(heroes.iter().count()) {
        return;
    }
    let party: Vec<(Entity, Vec3, bool, bool)> = heroes
        .iter()
        .map(|(e, _, tf, .., h)| (e, tf.translation, h.alive(), is_healer(h.class)))
        .collect();
    let warlocks: Vec<(Entity, Vec3)> = heroes
        .iter()
        .filter(|(.., h)| h.class.id == 9 && h.alive() && !h.synergy.res.soulstone_given)
        .map(|(e, _, tf, ..)| (e, tf.translation))
        .collect();
    for (warlock, at) in warlocks {
        let pick = party
            .iter()
            .filter(|&&(e, p, up, _)| e != warlock && up && hero::reach(at, p) <= RES_RANGE)
            .min_by(|a, b| {
                (!a.3)
                    .cmp(&!b.3)
                    .then(hero::reach(at, a.1).total_cmp(&hero::reach(at, b.1)))
            })
            .map(|p| p.0);
        let Some(target) = pick else { continue };
        let names =
            [warlock, target].map(|e| heroes.get(e).map(|(.., name, h)| label(h, name)).unwrap());
        if let Ok((.., mut h)) = heroes.get_mut(target) {
            h.synergy.res.soulstone = true;
        }
        if let Ok((.., mut h)) = heroes.get_mut(warlock) {
            h.synergy.res.soulstone_given = true;
        }
        fx.cast(warlock, SOULSTONE.id, &[target], None);
        fx.text(
            target,
            format!("{} casts {} on {}", names[0], SOULSTONE.name, names[1]),
            COLOR_HEAL,
        );
    }
}

/// A priest's, paladin's, shaman's or druid's resurrection in a party: begun on the nearest
/// downed ally it reaches, landed once its cast is through, broken off should the ally get up
/// first, the caster fall, or the ally be left out of reach.
pub(super) fn resurrect(
    time: Res<Time>,
    mut heroes: Query<Body, With<Hero>>,
    mut fields: MessageWriter<FieldChanged>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    if !coop(heroes.iter().count()) {
        return;
    }
    // Who lies down where, and for how long.
    let downed: Vec<(Entity, Vec3, f32)> = heroes
        .iter()
        .filter_map(|(e, _, tf, .., h)| h.dead_t.map(|t| (e, tf.translation, t)))
        .collect();
    let mut claimed: Vec<Entity> = heroes
        .iter()
        .filter_map(|(.., h)| h.synergy.res.casting.map(|c| c.0))
        .collect();
    let casters: Vec<(Entity, Vec3)> = heroes
        .iter()
        .filter(|(.., h)| res_of(h.class).is_some())
        .map(|(e, _, tf, ..)| (e, tf.translation))
        .collect();
    let mut landed: Vec<(Entity, Entity, &'static ResDef)> = Vec::new();
    for (caster, at) in casters {
        let Ok((.., mut h)) = heroes.get_mut(caster) else {
            continue;
        };
        let def = res_of(h.class).expect("a resurrection");
        let res = &mut h.synergy.res;
        res.cd = (res.cd - dt).max(0.0);
        let up = h.dead_t.is_none();
        let res = &mut h.synergy.res;
        if let Some((target, left)) = res.casting {
            let lying = downed
                .iter()
                .find(|d| d.0 == target)
                .filter(|d| hero::reach(at, d.1) <= RES_RANGE + 5.0);
            if !up || lying.is_none() {
                res.casting = None;
                fx.fail(caster, def.spell.id);
                continue;
            }
            let left = left - dt;
            if left > 0.0 {
                res.casting = Some((target, left));
            } else {
                res.casting = None;
                res.cd = def.cooldown;
                landed.push((caster, target, def));
            }
            continue;
        }
        if !up || res.cd > 0.0 {
            continue;
        }
        let pick = downed
            .iter()
            .filter(|d| d.0 != caster && !claimed.contains(&d.0))
            .filter(|d| def.fresh || d.2 >= RES_WAIT)
            .filter(|d| hero::reach(at, d.1) <= RES_RANGE)
            .min_by(|a, b| hero::reach(at, a.1).total_cmp(&hero::reach(at, b.1)));
        if let Some(&(target, ..)) = pick {
            res.casting = Some((target, RES_CAST));
            claimed.push(target);
            fx.start(caster, def.spell.id);
        }
    }
    for (caster, target, def) in landed {
        let Some(name) = heroes
            .get(caster)
            .ok()
            .map(|(.., member, h)| label(h, member))
        else {
            continue;
        };
        let Ok((e, guid, _, mut store, mut marks, member, mut h)) = heroes.get_mut(target) else {
            continue;
        };
        if h.alive() {
            continue;
        }
        raise(e, guid, &mut store, &mut marks, &mut h, &mut fields);
        fx.cast(caster, def.spell.id, &[target], None);
        fx.text(
            target,
            format!("{name} casts {} on {}", def.spell.name, label(&h, member)),
            COLOR_HEAL,
        );
        // The results' Necromancer counts it.
        if let Ok((.., mut c)) = heroes.get_mut(caster) {
            c.revives_given += 1;
        }
    }
}

/// What a downed hero can rise with by itself: a soulstone first, else a ready ankh.
fn self_res(hero: &HeroState) -> Option<SpellRef> {
    if hero.synergy.res.soulstone {
        Some(SOULSTONE)
    } else if hero.class.id == 7 && hero.synergy.res.ankh_cd <= 0.0 {
        Some(REINCARNATION)
    } else {
        None
    }
}

/// A downed hero in a party with a soulstone or a ready ankh gets up: the local player when it
/// presses [`SELF_RES_KEY`], anyone [`SELF_RES_AUTO`] after falling. Ankhs cool down meanwhile.
#[allow(clippy::type_complexity)]
pub(super) fn self_revive(
    time: Res<Time>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut heroes: Query<(Body, Has<LocalHero>), With<Hero>>,
    mut fields: MessageWriter<FieldChanged>,
    mut fx: Fx,
) {
    let dt = time.delta_secs();
    let party = coop(heroes.iter().count());
    let pressed = keys.is_some_and(|k| k.just_pressed(SELF_RES_KEY));
    for ((e, guid, _, mut store, mut marks, member, mut h), local) in &mut heroes {
        let res = &mut h.synergy.res;
        res.ankh_cd = (res.ankh_cd - dt).max(0.0);
        // A remote player's key counts once, now or never.
        let asked = std::mem::take(&mut res.asked);
        let Some(t) = h.dead_t.filter(|_| party) else {
            continue;
        };
        let Some(spell) = self_res(&h) else { continue };
        if !(t >= SELF_RES_AUTO || (local && pressed) || asked) {
            continue;
        }
        if spell == SOULSTONE {
            h.synergy.res.soulstone = false;
        } else {
            h.synergy.res.ankh_cd = ANKH_COOLDOWN;
        }
        raise(e, guid, &mut store, &mut marks, &mut h, &mut fields);
        fx.cast(e, spell.id, &[e], None);
        fx.text(
            e,
            format!("{} uses {}", label(&h, member), spell.name),
            COLOR_HEAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::testing::{down, hero, heroes, texts, world};
    use super::*;

    fn is_down(w: &World, e: Entity) -> bool {
        !hero(w, e).alive()
    }

    fn lie(w: &mut World, e: Entity, secs: f32) {
        down(w, e);
        w.get_mut::<HeroState>(e).unwrap().dead_t = Some(secs);
    }

    #[test]
    fn a_priest_resurrects_a_downed_ally_in_reach_then_cools_down() {
        let mut w = world(1.0);
        let hs = heroes(
            &mut w,
            &[("Priest", 0.0), ("Warrior", 20.0), ("Mage", 25.0)],
        );
        w.entity_mut(hs[0]).insert(MemberName("Anduin".to_string()));
        w.entity_mut(hs[1])
            .insert(MemberName("Valeera".to_string()));
        lie(&mut w, hs[1], 0.0);
        w.run_system_once(resurrect).unwrap();
        assert!(
            hero(&w, hs[0]).synergy.res.casting.is_none(),
            "not on the just-fallen"
        );
        lie(&mut w, hs[1], RES_WAIT);
        w.run_system_once(resurrect).unwrap();
        assert_eq!(
            hero(&w, hs[0]).synergy.res.casting.map(|c| c.0),
            Some(hs[1])
        );
        w.run_system_once(resurrect).unwrap();
        assert!(is_down(&w, hs[1]), "a second into the cast");
        w.run_system_once(resurrect).unwrap();
        assert!(!is_down(&w, hs[1]), "up after the cast");
        assert_eq!(
            hero(&w, hs[1]).hp,
            hero(&w, hs[1]).stats().max_hp * party::REVIVED_HP
        );
        assert!(texts(&w).contains(&"Anduin casts Resurrection on Valeera".to_string()));
        let cd = hero(&w, hs[0]).synergy.res.cd;
        assert_eq!(cd, RES_COOLDOWN);
        assert_eq!(hero(&w, hs[0]).revives_given, 1, "credited to the caster");
        // Another falls: the cooldown holds it off until it is through.
        lie(&mut w, hs[2], 10.0);
        for _ in 0..80 {
            w.run_system_once(resurrect).unwrap();
        }
        assert!(hero(&w, hs[0]).synergy.res.casting.is_none());
        for _ in 0..12 {
            w.run_system_once(resurrect).unwrap();
        }
        assert!(!is_down(&w, hs[2]), "ready again after 90 s");
    }

    #[test]
    fn a_resurrection_reaches_thirty_yards_and_breaks_off() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Paladin", 0.0), ("Rogue", 31.0)]);
        lie(&mut w, hs[1], 10.0);
        w.run_system_once(resurrect).unwrap();
        assert!(
            hero(&w, hs[0]).synergy.res.casting.is_none(),
            "out of reach"
        );
        w.get_mut::<Transform>(hs[1]).unwrap().translation.x = 29.0;
        w.run_system_once(resurrect).unwrap();
        assert!(hero(&w, hs[0]).synergy.res.casting.is_some());
        // Up another way mid-cast: the cast breaks off, the cooldown unspent.
        party::get_up(&mut w.get_mut::<HeroState>(hs[1]).unwrap());
        w.run_system_once(resurrect).unwrap();
        let res = hero(&w, hs[0]).synergy.res;
        assert!(res.casting.is_none() && res.cd == 0.0);
    }

    #[test]
    fn rebirth_reaches_the_just_fallen_and_cools_down_longer() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Druid", 0.0), ("Mage", 5.0)]);
        lie(&mut w, hs[1], 0.0);
        for _ in 0..3 {
            w.run_system_once(resurrect).unwrap();
        }
        assert!(!is_down(&w, hs[1]));
        assert_eq!(hero(&w, hs[0]).synergy.res.cd, REBIRTH_COOLDOWN);
        assert!(texts(&w).contains(&"Player 1 casts Rebirth on Player 2".to_string()));
    }

    #[test]
    fn no_resurrection_alone() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Shaman", 0.0)]);
        lie(&mut w, hs[0], 0.0);
        for _ in 0..5 {
            w.run_system_once(resurrect).unwrap();
            w.run_system_once(self_revive).unwrap();
        }
        assert!(is_down(&w, hs[0]), "alone, down is dead: no ankh");
    }

    #[test]
    fn a_warlock_s_soulstone_raises_its_ally_once() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Mage", 0.0), ("Warlock", 5.0), ("Priest", 25.0)]);
        w.run_system_once(soulstone).unwrap();
        assert!(hero(&w, hs[2]).synergy.res.soulstone, "the healer first");
        assert!(!hero(&w, hs[0]).synergy.res.soulstone);
        w.run_system_once(soulstone).unwrap();
        assert!(!hero(&w, hs[0]).synergy.res.soulstone, "once a run");
        // Down: up again two seconds on, the stone spent.
        lie(&mut w, hs[2], 1.0);
        w.run_system_once(self_revive).unwrap();
        assert!(is_down(&w, hs[2]));
        lie(&mut w, hs[2], SELF_RES_AUTO);
        w.run_system_once(self_revive).unwrap();
        assert!(!is_down(&w, hs[2]));
        assert!(texts(&w).contains(&"Player 3 uses Soulstone Resurrection".to_string()));
        lie(&mut w, hs[2], 5.0);
        w.run_system_once(self_revive).unwrap();
        assert!(is_down(&w, hs[2]), "the stone is spent");
    }

    #[test]
    fn the_local_player_rises_on_its_key() {
        let mut w = world(0.1);
        let hs = heroes(&mut w, &[("Mage", 0.0), ("Warlock", 5.0)]);
        w.run_system_once(soulstone).unwrap();
        assert!(hero(&w, hs[0]).synergy.res.soulstone);
        lie(&mut w, hs[0], 0.1);
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(SELF_RES_KEY);
        w.insert_resource(keys);
        w.run_system_once(self_revive).unwrap();
        assert!(!is_down(&w, hs[0]), "pressed, at once");
    }

    #[test]
    fn a_remote_player_rises_on_its_own_key_sent_over() {
        let mut w = world(0.1);
        let hs = heroes(&mut w, &[("Warlock", 0.0), ("Mage", 5.0)]);
        w.run_system_once(soulstone).unwrap();
        assert!(hero(&w, hs[1]).synergy.res.soulstone);
        lie(&mut w, hs[1], 0.1);
        // The host's own key is no one else's.
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(SELF_RES_KEY);
        w.insert_resource(keys);
        w.run_system_once(self_revive).unwrap();
        assert!(is_down(&w, hs[1]));
        // Its own player's, from its machine.
        w.get_mut::<HeroState>(hs[1]).unwrap().synergy.res.asked = true;
        w.run_system_once(self_revive).unwrap();
        assert!(!is_down(&w, hs[1]), "asked, at once");
        assert!(!hero(&w, hs[1]).synergy.res.asked, "spent");
    }

    #[test]
    fn a_shaman_s_ankh_raises_it_once_every_five_minutes() {
        let mut w = world(1.0);
        let hs = heroes(&mut w, &[("Shaman", 0.0), ("Rogue", 50.0)]);
        lie(&mut w, hs[0], SELF_RES_AUTO);
        w.run_system_once(self_revive).unwrap();
        assert!(!is_down(&w, hs[0]));
        assert_eq!(hero(&w, hs[0]).synergy.res.ankh_cd, ANKH_COOLDOWN);
        lie(&mut w, hs[0], SELF_RES_AUTO);
        w.run_system_once(self_revive).unwrap();
        assert!(is_down(&w, hs[0]), "the ankh cools down");
        w.get_mut::<HeroState>(hs[0]).unwrap().synergy.res.ankh_cd = 0.5;
        w.run_system_once(self_revive).unwrap();
        assert!(!is_down(&w, hs[0]), "ready again");
        // Another class has no ankh.
        lie(&mut w, hs[1], 5.0);
        w.run_system_once(self_revive).unwrap();
        assert!(is_down(&w, hs[1]));
    }
}
