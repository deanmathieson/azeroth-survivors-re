//! Party synergy, WoW's way: what each class brings to a party of two or more (bots counted).
//! Everything here is the host's simulation, so it plays the same online.
//!
//! - **Group buffs** ([`buffs`]): each class's signature buff on every ally within
//!   [`buffs::BUFF_RANGE`] of the caster (a shaman's totems: of the totem), refreshed as it runs
//!   low, its real visual on the cast. Power Word: Fortitude, Arcane Intellect, Mark of the Wild,
//!   a paladin's aura or blessing by build, Battle Shout, Trueshot Aura and Aspect of the Pack,
//!   Strength of Earth, Windfury and Mana Spring Totems, Blood Pact. A buff never stacks with
//!   itself: the strongest holds. They are bonuses in the stat vocabulary, the C panel's "Party".
//!   A lone hero gets none.
//! - **Shared debuffs** ([`debuffs`]): a class's blows put its debuff on the enemy (Sunder Armor,
//!   Expose Armor, Faerie Fire, Curse of the Elements and of Shadow, Hunter's Mark, Winter's
//!   Chill, Fire Vulnerability, Shadow Weaving's Shadow Vulnerability, Judgement of the Crusader),
//!   and every hero's blows of its school deal more to it. Solo too, kept small.
//! - **Resurrections** ([`res`]): Resurrection, Redemption, Ancestral Spirit and Rebirth raise a
//!   downed ally from range; a warlock's Soulstone Resurrection and a shaman's Reincarnation let a
//!   hero get itself up. In a party only, on top of the ally's channel ([`super::party`]).
//! - **Threat** ([`threat`]): a tank class's stance or form (Defensive Stance, Righteous Fury,
//!   Bear Form) raises its threat in a party, and its taunts force enemies onto it.
//!
//! Spells are real 1.12 rows, checked by name and icon against `Spell.dbc` in the data tests;
//! every number is the mode's own.

mod buffs;
mod debuffs;
mod res;
mod threat;

use bevy::prelude::*;

use super::{abilities, enemies, hero, party, HeroState, Phase};

pub(super) use buffs::{bonuses as buff_bonuses, reflect, PartyBuff};
pub(super) use debuffs::{afflict, Debuffs};
#[cfg(test)]
pub(super) use threat::{taunts, TAUNT_SECS};

/// A real 1.12 spell: its `Spell.dbc` id, and the name and icon the data tests know it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SpellRef {
    pub id: u32,
    pub name: &'static str,
    pub icon: &'static str,
}

pub(super) const fn spell(id: u32, name: &'static str, icon: &'static str) -> SpellRef {
    SpellRef { id, name, icon }
}

/// Whether `heroes` play as a party: two or more, bots counted. A lone hero's run is as it was.
pub(super) fn coop(heroes: usize) -> bool {
    heroes >= 2
}

/// A hero as the party's combat text names it.
pub(super) fn label(hero: &HeroState) -> String {
    format!("Player {}", u32::from(hero.player) + 1)
}

/// One hero's part in a party's play.
#[derive(Clone, Debug, Default)]
pub(in crate::survivors) struct HeroSynergy {
    /// The group buffs on it, one of each at most.
    pub(in crate::survivors) buffs: Vec<PartyBuff>,
    /// Its resurrection's cooldown and cast, its soulstone and its ankh.
    pub(in crate::survivors) res: res::ResState,
    /// A tank's stance is up, and its taunt's cooldown.
    pub(in crate::survivors) stance: bool,
    pub(in crate::survivors) taunt_cd: f32,
}

/// The spells `hero` shows as its auras after its own: its stance, a soulstone, the group buffs.
pub(super) fn auras(hero: &HeroState, shown: &mut Vec<u32>) {
    if hero.synergy.stance {
        if let Some(stance) = threat::stance(hero.class) {
            shown.push(stance.spell.id);
        }
    }
    if hero.synergy.res.soulstone {
        shown.push(res::SOULSTONE.id);
    }
    shown.extend(hero.synergy.buffs.iter().map(|b| b.def.spell.id));
}

pub(super) fn plugin(app: &mut App) {
    // The host's simulation: a co-op client shows its results ([`party::simulates`]).
    let playing = || {
        in_state(Phase::Playing)
            .and(party::simulates)
            .and(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering())
    };
    app.add_systems(
        Update,
        (
            threat::stances,
            buffs::party_buffs,
            buffs::party_totems,
            threat::taunts,
        )
            .chain()
            .after(enemies::spawn_enemies)
            .before(enemies::move_enemies)
            .run_if(playing()),
    )
    .add_systems(
        Update,
        debuffs::tick_debuffs
            .after(abilities::apply_hits)
            .before(hero::hero_vitals)
            .run_if(playing()),
    )
    .add_systems(
        Update,
        (res::soulstone, res::resurrect, res::self_revive)
            .chain()
            .after(hero::hero_vitals)
            .before(party::revive_downed)
            .run_if(playing()),
    );
}

#[cfg(test)]
pub(super) mod testing {
    //! A world the party systems run in, a second passing each run.
    use bevy::prelude::*;

    use super::super::bonus::Bonuses;
    use super::super::data::{self, ClassDef};
    use super::super::hero::Hero;
    use super::super::units::Marks;
    use super::super::{HeroState, LocalHero, Rng, Run};
    use crate::net::{FieldChanged, Guid, ObjectStore};

    pub(in crate::survivors) fn class(name: &str) -> &'static ClassDef {
        data::CLASSES.iter().find(|c| c.name == name).unwrap()
    }

    pub(in crate::survivors) fn world(secs: f32) -> World {
        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(secs));
        world.insert_resource(time);
        world.init_resource::<Run>();
        world.insert_resource(Rng(7));
        world.init_resource::<Messages<FieldChanged>>();
        world.init_resource::<crate::creature_anim::PlaySeq>();
        world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
        world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
        world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
        world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
        world
    }

    /// Heroes of these classes at these x positions; the first is the local one.
    pub(in crate::survivors) fn heroes(world: &mut World, heroes: &[(&str, f32)]) -> Vec<Entity> {
        heroes
            .iter()
            .enumerate()
            .map(|(i, &(name, x))| {
                let state = HeroState::new(i as u8, class(name), 1, Bonuses::default());
                let store = ObjectStore(Default::default());
                let body = (Guid(i as u64 + 1), Transform::from_xyz(x, 0.0, 0.0), store);
                let mut e = world.spawn((Hero::default(), body, Marks::buffs(), state));
                if i == 0 {
                    e.insert(LocalHero);
                }
                e.id()
            })
            .collect()
    }

    pub(in crate::survivors) fn hero(world: &World, e: Entity) -> &HeroState {
        world.get::<HeroState>(e).unwrap()
    }

    pub(in crate::survivors) fn down(world: &mut World, e: Entity) {
        let mut h = world.get_mut::<HeroState>(e).unwrap();
        h.dead_t = Some(0.0);
        h.hp = 0.0;
    }

    /// The combat text written so far.
    pub(in crate::survivors) fn texts(world: &World) -> Vec<String> {
        let msgs = world.resource::<Messages<crate::combat_text::CombatTextSpawn>>();
        msgs.get_cursor()
            .read(msgs)
            .map(|m| m.text.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every spell the party plays, with what it is for.
    fn all_spells() -> Vec<(String, SpellRef)> {
        let mut out = Vec::new();
        for b in buffs::BUFFS {
            out.push((b.key.to_string(), b.spell));
            if let buffs::BuffKind::Totem { summon, .. } = b.kind {
                out.push((format!("{} totem", b.key), summon));
            }
        }
        for d in debuffs::DEBUFFS {
            out.push((d.key.to_string(), d.spell));
        }
        out.extend(res::all_spells().map(|s| (format!("res {}", s.name), s)));
        out.extend(threat::all_spells().map(|s| (format!("threat {}", s.name), s)));
        out
    }

    /// Every spell whose visual plays: the buffs and casts, and the debuffs that have one.
    fn played() -> Vec<u32> {
        let mut out: Vec<u32> = all_spells()
            .iter()
            .map(|(_, s)| s.id)
            .filter(|&id| !debuffs::DEBUFFS.iter().any(|d| d.spell.id == id))
            .collect();
        out.extend(debuffs::DEBUFFS.iter().filter_map(|d| d.visual));
        out
    }

    /// Against the real `Spell.dbc`, `SpellVisual.dbc` and creature displays; skips without client
    /// data. Every spell named is the 1.12 row of that name and icon, every visual played has a
    /// kit, and every totem stands in its element's model.
    #[test]
    fn every_party_spell_is_real_with_its_name_and_icon() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let visuals =
            benilla_formats::load_spell_visual_catalog(&mut chain).expect("SpellVisual.dbc");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let mut bad = Vec::new();
        for (what, s) in all_spells() {
            let Some(d) = spells.get(s.id) else {
                bad.push(format!("{what}: {} has no row", s.id));
                continue;
            };
            if d.name != s.name {
                bad.push(format!(
                    "{what}: {} is {:?}, not {:?}",
                    s.id, d.name, s.name
                ));
            }
            let icon = d
                .icon
                .as_deref()
                .map(|p| p.rsplit('\\').next().unwrap_or(p));
            if !icon.is_some_and(|i| i.eq_ignore_ascii_case(s.icon)) {
                bad.push(format!(
                    "{what}: {}'s icon is {icon:?}, not {}",
                    s.id, s.icon
                ));
            }
        }
        for id in played() {
            let kit = spells
                .get(id)
                .and_then(|d| visuals.stages(d.visual))
                .is_some_and(|s| s.precast != 0 || s.cast != 0 || s.impact != 0 || s.state != 0);
            if !kit {
                bad.push(format!("{id} plays no visual"));
            }
        }
        for b in buffs::BUFFS {
            if let buffs::BuffKind::Totem { display, model, .. } = b.kind {
                let path = creatures.model(display).map(|m| m.model_path);
                if !path.as_deref().is_some_and(|p| p.contains(model)) {
                    bad.push(format!("{}: display {display} is {path:?}", b.key));
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn a_party_is_two_or_more() {
        assert!(!coop(0) && !coop(1));
        assert!(coop(2) && coop(5));
    }
}
