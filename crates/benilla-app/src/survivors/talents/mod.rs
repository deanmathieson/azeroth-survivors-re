//! The talent catalog: the 27 vanilla trees, three per class, read from the client's own
//! `Talent.dbc` and `TalentTab.dbc` (the [`crate::ui_talent::Talents`] load), with each talent's
//! name and icon from its rank-1 spell. What a rank grants is the mode's own design
//! ([`effects`]); how points may be spent is vanilla's ([`rules`]).
//!
//! Each recorded run pays its class talent XP, which buys points ([`progress`]); the Armory's
//! Talents tab spends them ([`tab`]); and the picked class's learned ranks become
//! [`MetaParts::talents`], which every run starts with ([`apply_talents`]).
//!
//! A tree's 31-point capstone makes the hero its spec for every run ([`spec`]): a new start spell
//! and a signature summon ([`guardian`]) or form. A talent that is also a spell or passive the run
//! offers teaches it instead of doubling it ([`mechanics::Teach`]; the audit is
//! `docs/TALENTS.md`).

mod effects;
mod guardian;
mod mechanics;
mod progress;
mod rules;
mod save;
mod spec;
mod tab;

use std::collections::HashMap;

use bevy::prelude::*;

use super::bonus::{Bonus, Bonuses};
use super::mechanics::{self as game_mechanics, MechanicId};
use super::meta::MetaParts;
use super::party::{HeroState, LocalHero};
use super::profile::{slug, Profile};
use super::{data, hud, Selection};

pub(super) use progress::{add_xp, earned_points, xp_for_points, TalentDevXp};
use rules::{bonuses, can_learn, can_unlearn};
pub(super) use rules::{Allocation, Why, MAX_POINTS, POINTS_PER_TIER};
pub(super) use save::TalentSave;
pub(super) use tab::{
    check_learn, check_reset, check_unlearn, describe_rank, locked_reasons, shade, unmet, why_text,
    TalentAction, TalentArt, TalentFocus, TalentShade, TalentSlot,
};

pub(in crate::survivors) fn mechanic_description(talent: u32, rank: u8) -> Vec<String> {
    mechanics::describe(talent, rank)
}

pub(in crate::survivors) use mechanics::milestones;
pub(in crate::survivors) use spec::HeroSpec;

pub(in crate::survivors) fn is_headline(talent: u32) -> bool {
    mechanics::entry(talent).is_some_and(|entry| entry.headline)
}

/// The spec lines of a tree's 31-point capstone, empty for any other talent: "Becomes: Frost
/// Mage.", "Starts with Frostbolt.", "Summons a Water Elemental."
pub(in crate::survivors) fn spec_lines(talent: u32) -> Vec<String> {
    spec::by_talent(talent)
        .map(spec::SpecDef::tooltip)
        .unwrap_or_default()
}

/// Whether the class spell or passive `key` may be offered to `hero`: one a talent teaches is
/// offered only to the heroes who learned it, if it is a spell.
pub(in crate::survivors) fn offered(hero: &HeroState, key: &str) -> bool {
    match mechanics::teacher_of(key).and_then(|t| t.teaches) {
        Some(mechanics::Teach::Spell(_)) => hero.mechanics.taught(key),
        _ => true,
    }
}

/// The rank `hero` takes the spell or passive `key` at: 2 when a learned talent teaches it.
pub(in crate::survivors) fn first_rank(hero: &HeroState, key: &str) -> u8 {
    if hero.mechanics.taught(key) {
        2
    } else {
        1
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<TalentFocus>().add_systems(
        Update,
        (
            build_catalog.run_if(not(resource_exists::<TalentCatalog>)),
            progress::award_xp,
            apply_talents,
            grant_talents,
            (
                guardian::spawn_guardians,
                guardian::run_guardians,
                guardian::wear_forms,
            )
                .chain()
                .after(grant_talents)
                .run_if(in_state(super::Phase::Playing))
                .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            tab::talent_input,
            tab::catalog_arrived,
            // After the Armory has (re)built the tab, so new art fills the same frame.
            tab::fill_art.after(hud::show_panels),
        ),
    );
    if let Some(dev) = TalentDevXp::from_env() {
        app.insert_resource(dev);
    }
}

/// Give the local hero the learned talents' active mechanics, the spells and passives they teach,
/// and its spec, after `start_run` replaces its state.
#[allow(clippy::type_complexity)]
fn grant_talents(
    mut commands: Commands,
    profile: Option<Res<Profile>>,
    selection: Res<Selection>,
    phase: Res<State<super::Phase>>,
    mut heroes: Query<
        (Entity, &mut HeroState, Option<&HeroSpec>),
        (With<LocalHero>, Changed<HeroState>),
    >,
) {
    if *phase.get() != super::Phase::Playing {
        return;
    }
    let (Some(profile), Some(class)) = (profile, selection.class) else {
        return;
    };
    let ranks = saved(&profile, class).map_or(&[][..], |s| s.ranks.as_slice());
    for (e, mut hero, had) in &mut heroes {
        let spec = grant_learned(&mut hero, ranks);
        match (spec, had) {
            (Some(spec), Some(had)) if std::ptr::eq(spec, had.0) => {}
            (Some(spec), _) => {
                commands.entity(e).insert(HeroSpec(spec));
            }
            (None, Some(_)) => {
                commands.entity(e).remove::<HeroSpec>();
            }
            (None, None) => {}
        }
    }
}

/// Put every learned active talent on a freshly created hero, what the learned talents teach,
/// and the spec its capstone makes it, which it returns. Kept separate from the ECS query so the
/// exact run-start grant set remains small and directly testable.
fn grant_learned(hero: &mut HeroState, ranks: &[(u32, u8)]) -> Option<&'static spec::SpecDef> {
    for (id, rank) in ranks {
        let specs = mechanics::at_rank(*id, *rank);
        if !specs.is_empty() {
            game_mechanics::grant(hero, MechanicId::Talent(*id), specs);
        }
        if *rank > 0 {
            if let Some(mechanics::Teach::Spell(key) | mechanics::Teach::Passive(key)) =
                mechanics::entry(*id).and_then(|m| m.teaches)
            {
                hero.mechanics.teach(key);
            }
        }
    }
    let spec = spec::learned(ranks)?;
    spec::grant(hero, spec);
    Some(spec)
}

/// One class's saved talents (an index into [`data::CLASSES`]), if it has a record.
pub(super) fn saved(profile: &Profile, class: usize) -> Option<&TalentSave> {
    let class = data::CLASSES.get(class)?;
    profile.classes.get(&slug(class.name)).map(|r| &r.talents)
}

/// What a class's learned talents grant, nothing for a class with none.
fn class_bonuses(catalog: &TalentCatalog, profile: &Profile, class: usize) -> Bonuses {
    saved(profile, class).map_or_else(Bonuses::default, |s| bonuses(catalog, &s.allocation()))
}

/// Keeps [`MetaParts::talents`] at the picked class's learned talents, rechecked whenever the
/// pick, the profile or the catalog changes; written only when the bonuses do.
fn apply_talents(
    catalog: Option<Res<TalentCatalog>>,
    profile: Option<Res<Profile>>,
    selection: Res<Selection>,
    mut parts: ResMut<MetaParts>,
) {
    let changed = selection.is_changed()
        || profile.as_ref().is_some_and(|p| p.is_changed())
        || catalog.as_ref().is_some_and(|c| c.is_changed());
    if !changed {
        return;
    }
    let want = match (&catalog, &profile, selection.class) {
        (Some(catalog), Some(profile), Some(class)) => class_bonuses(catalog, profile, class),
        _ => Bonuses::default(),
    };
    if parts.talents != want {
        parts.talents = want;
    }
}

/// One class's talent tree, a `TalentTab.dbc` row.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Tree {
    pub id: u32,
    /// An index into [`data::CLASSES`].
    pub class: usize,
    pub name: String,
    /// 0..3 within the class, the client's tab order. That is file order: the `OrderIndex`
    /// column repeats itself (Arcane and Fire are both 0).
    pub order: u8,
    /// The extensionless `Interface\Icons\…` path.
    pub icon: Option<String>,
    /// The `Interface\TalentFrame\<background>-…` art base ("MageFire").
    pub background: String,
}

/// One talent, a `Talent.dbc` row.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Talent {
    pub id: u32,
    /// The owning [`Tree::id`].
    pub tree: u32,
    /// 0-based; tier `n` opens at `n` × [`POINTS_PER_TIER`] points in the tree.
    pub tier: u8,
    /// 0-based, of 4.
    pub column: u8,
    pub max_rank: u8,
    /// The spell each rank teaches, rank 1 first; exactly `max_rank` long.
    pub spells: Vec<u32>,
    /// A talent needed first, and the rank it needs (always its full rank in the shipped data).
    pub prereq: Option<(u32, u8)>,
    /// The rank-1 spell's name and icon path.
    pub name: String,
    pub icon: Option<String>,
}

impl Talent {
    /// What one rank grants; rank `n` grants `n` times this.
    pub fn effect(&self) -> &'static [Bonus] {
        if mechanics::entry(self.id).is_some() {
            &[]
        } else {
            effects::per_rank(self.id).unwrap_or(&[])
        }
    }
}

/// Every tree and talent, built once the client's talent and spell tables are up.
#[derive(Resource, Debug)]
pub(super) struct TalentCatalog {
    /// In file order.
    trees: Vec<Tree>,
    /// Grouped by tree in [`Self::trees`] order, each tree's in native row order.
    talents: Vec<Talent>,
    by_id: HashMap<u32, usize>,
}

impl TalentCatalog {
    pub fn new(trees: Vec<Tree>, talents: Vec<Talent>) -> Self {
        let by_id = talents.iter().enumerate().map(|(i, t)| (t.id, i)).collect();
        Self {
            trees,
            talents,
            by_id,
        }
    }

    /// The catalog from the client's tables: a tab whose class mask names none of
    /// [`data::CLASSES`] is left out, with its talents.
    pub fn from_client(
        client: &benilla_formats::TalentCatalog,
        spells: &benilla_formats::SpellCatalog,
    ) -> Self {
        let mut trees = Vec::new();
        let mut talents = Vec::new();
        let mut per_class = [0u8; 32];
        for tab in client.tabs() {
            let Some(class) = data::CLASSES
                .iter()
                .position(|c| tab.class_mask & (1u32 << (c.id - 1)) != 0)
            else {
                continue;
            };
            trees.push(Tree {
                id: tab.id,
                class,
                name: tab.name.clone(),
                order: per_class[class],
                icon: tab.icon.clone(),
                background: tab.background.clone(),
            });
            per_class[class] += 1;
            for t in client.talents_in_tab(tab.id) {
                let max_rank = t.max_rank() as u8;
                let spell = spells.get(t.ranks[0]);
                talents.push(Talent {
                    id: t.id,
                    tree: tab.id,
                    tier: t.row as u8,
                    column: t.col as u8,
                    max_rank,
                    spells: t.ranks[..usize::from(max_rank)].to_vec(),
                    prereq: (t.prereq_talent != 0)
                        .then(|| (t.prereq_talent, t.prereq_rank as u8 + 1)),
                    name: spell.map(|d| d.name.clone()).unwrap_or_default(),
                    icon: spell.and_then(|d| d.icon.clone()),
                });
            }
        }
        Self::new(trees, talents)
    }

    /// Every tree, in file order.
    #[allow(dead_code)] // The data tests read the whole catalog; the game reads it by class.
    pub fn trees(&self) -> &[Tree] {
        &self.trees
    }

    /// A class's trees (an index into [`data::CLASSES`]), in tab order.
    pub fn trees_for(&self, class: usize) -> impl Iterator<Item = &Tree> {
        let mut trees: Vec<&Tree> = self.trees.iter().filter(|t| t.class == class).collect();
        trees.sort_by_key(|t| t.order);
        trees.into_iter()
    }

    pub fn tree(&self, id: u32) -> Option<&Tree> {
        self.trees.iter().find(|t| t.id == id)
    }

    #[allow(dead_code)] // As [`Self::trees`]: the game reads talents by tree or id.
    pub fn talents(&self) -> &[Talent] {
        &self.talents
    }

    /// A tree's talents, in native row order: the shipped data authors them by tier, then column.
    pub fn talents_in(&self, tree: u32) -> impl Iterator<Item = &Talent> {
        self.talents.iter().filter(move |t| t.tree == tree)
    }

    pub fn talent(&self, id: u32) -> Option<&Talent> {
        self.by_id.get(&id).map(|&i| &self.talents[i])
    }
}

/// Builds the catalog once both client tables have loaded; without client data it never does.
fn build_catalog(
    mut commands: Commands,
    talents: Option<Res<crate::ui_talent::Talents>>,
    spells: Option<Res<crate::ui_action::Spells>>,
) {
    let (Some(talents), Some(spells)) = (talents, spells) else {
        return;
    };
    let catalog = TalentCatalog::from_client(&talents.catalog, &spells.catalog);
    info!(
        "survivors: {} talent trees, {} talents",
        catalog.trees.len(),
        catalog.talents.len()
    );
    commands.insert_resource(catalog);
}

/// A made-up catalog for the pure tests, no client data needed.
#[cfg(test)]
pub(super) mod fixture {
    use super::{Talent, TalentCatalog, Tree};

    /// Real ids, so the bonuses come from the real table: Toughness and Fire Power.
    pub const TOUGH: u32 = 140;
    pub const FIRE_POWER: u32 = 35;

    fn talent(id: u32, tree: u32, tier: u8, max_rank: u8, prereq: Option<(u32, u8)>) -> Talent {
        Talent {
            id,
            tree,
            tier,
            column: (id % 4) as u8,
            max_rank,
            spells: (1..=u32::from(max_rank)).map(|r| id * 10 + r).collect(),
            prereq,
            name: format!("t{id}"),
            icon: None,
        }
    }

    /// Class 0's "Tree 1" has made-up ids past every real talent: 9000 (5 ranks) and 9001 (3)
    /// at tier 0, and 9010 (1) at tier 1 needing 9001 at 3/3. "Tree 2" has Toughness and Fire
    /// Power at tier 0.
    pub fn catalog() -> TalentCatalog {
        let tree = |id: u32, order| Tree {
            id,
            class: 0,
            name: format!("Tree {id}"),
            order,
            icon: None,
            background: String::new(),
        };
        TalentCatalog::new(
            vec![tree(1, 0), tree(2, 1)],
            vec![
                talent(9000, 1, 0, 5, None),
                talent(9001, 1, 0, 3, None),
                talent(9010, 1, 1, 1, Some((9001, 3))),
                talent(TOUGH, 2, 0, 5, None),
                talent(FIRE_POWER, 2, 0, 5, None),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::OnceLock;

    use super::super::bonus::{Bonuses, School, StatKey};
    use super::*;

    #[test]
    fn the_picked_classs_talents_become_its_meta_part() {
        use super::super::profile::ClassRecord;
        use fixture::{FIRE_POWER, TOUGH};

        let cat = fixture::catalog();
        let mage = data::CLASSES.iter().position(|c| c.name == "Mage").unwrap();
        let save = TalentSave {
            xp: 0,
            ranks: vec![(FIRE_POWER, 2), (TOUGH, 3)],
        };
        let mut profile = Profile::default();
        profile.classes.insert(
            "mage".into(),
            ClassRecord {
                talents: save.clone(),
                ..default()
            },
        );
        let mut app = App::new();
        app.insert_resource(cat)
            .insert_resource(profile)
            .insert_resource(Selection {
                class: Some(mage),
                ..default()
            })
            .init_resource::<MetaParts>()
            .add_systems(Update, apply_talents);
        app.update();
        let want = bonuses(app.world().resource::<TalentCatalog>(), &save.allocation());
        assert_ne!(want, Bonuses::default());
        assert_eq!(app.world().resource::<MetaParts>().talents, want);
        assert_eq!(app.world().resource::<MetaParts>().gear, Bonuses::default());

        // Another class, with nothing learned, starts runs with nothing.
        app.world_mut().resource_mut::<Selection>().class = Some((mage + 1) % data::CLASSES.len());
        app.update();
        assert_eq!(
            app.world().resource::<MetaParts>().talents,
            Bonuses::default()
        );

        // Back on the mage, and a rank unlearned in the profile: the part follows.
        app.world_mut().resource_mut::<Selection>().class = Some(mage);
        app.world_mut()
            .resource_mut::<Profile>()
            .classes
            .get_mut("mage")
            .unwrap()
            .talents
            .ranks = vec![(TOUGH, 3)];
        app.update();
        let want = bonuses(
            app.world().resource::<TalentCatalog>(),
            &[(TOUGH, 3)].into_iter().collect(),
        );
        assert_eq!(app.world().resource::<MetaParts>().talents, want);
        assert_eq!(
            app.world()
                .resource::<MetaParts>()
                .talents
                .school_damage(School::Fire),
            0.0
        );
    }

    /// The client's raw talent table and the catalog built from it, loaded once for every test.
    fn client() -> Option<&'static (benilla_formats::TalentCatalog, TalentCatalog)> {
        static LOADED: OnceLock<Option<(benilla_formats::TalentCatalog, TalentCatalog)>> =
            OnceLock::new();
        LOADED
            .get_or_init(|| {
                let data = benilla_formats::wow_data_or_skip!(None);
                let mut chain = benilla_formats::open_chain(&data).expect("open chain");
                let raw = benilla_formats::load_talent_catalog(&mut chain).expect("talents");
                let spells = benilla_formats::load_spell_catalog(&mut chain).expect("spells");
                let catalog = TalentCatalog::from_client(&raw, &spells);
                Some((raw, catalog))
            })
            .as_ref()
    }

    #[test]
    fn every_class_has_three_trees() {
        let Some((_, cat)) = client() else { return };
        assert_eq!(cat.trees().len(), 27);
        for (class, def) in data::CLASSES.iter().enumerate() {
            let trees: Vec<&Tree> = cat.trees_for(class).collect();
            assert_eq!(trees.len(), 3, "{} has three trees", def.name);
            assert_eq!(trees.iter().map(|t| t.order).collect::<Vec<_>>(), [0, 1, 2]);
            for t in &trees {
                assert!(t.icon.is_some(), "{} {} has an icon", def.name, t.name);
                assert!(!t.background.is_empty());
                assert!(
                    cat.talents_in(t.id).count() >= 14,
                    "{} {}",
                    def.name,
                    t.name
                );
            }
        }
        let mage = data::CLASSES.iter().position(|c| c.name == "Mage").unwrap();
        assert_eq!(
            cat.trees_for(mage)
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            ["Arcane", "Fire", "Frost"]
        );
        let raw_talents: usize = cat.trees().iter().map(|t| raw_len(t.id)).sum();
        assert_eq!(cat.talents().len(), raw_talents);
    }

    fn raw_len(tree: u32) -> usize {
        client().map_or(0, |(raw, _)| raw.talents_in_tab(tree).len())
    }

    #[test]
    fn every_talent_has_an_effect_and_every_effect_a_talent() {
        let Some((_, cat)) = client() else { return };
        let missing: Vec<String> = cat
            .talents()
            .iter()
            .filter(|t| effects::per_rank(t.id).is_none())
            .map(|t| format!("{} {}", t.id, t.name))
            .collect();
        assert!(missing.is_empty(), "talents without effects: {missing:?}");
        for &(id, name, per_rank) in effects::EFFECTS {
            let talent = cat.talent(id);
            assert!(talent.is_some(), "effect {id} {name} names no talent");
            // The name guards against a mistyped id landing on another talent.
            assert_eq!(talent.unwrap().name, name, "effect {id}");
            assert!(!per_rank.is_empty(), "effect {id} {name} grants nothing");
        }
    }

    #[test]
    fn max_ranks_match_the_rank_spells() {
        let Some((raw, cat)) = client() else { return };
        for t in cat.talents() {
            let row = raw
                .talents_in_tab(t.tree)
                .iter()
                .find(|r| r.id == t.id)
                .unwrap();
            let populated = row.ranks.iter().filter(|&&s| s != 0).count();
            assert_eq!(usize::from(t.max_rank), populated, "{} {}", t.id, t.name);
            assert_eq!(t.spells.len(), populated);
            assert!((1..=5).contains(&t.max_rank));
            assert!(
                !t.name.is_empty(),
                "talent {} has a rank-1 spell name",
                t.id
            );
            if let Some((p, rank)) = t.prereq {
                let p = cat.talent(p).expect("prereq resolves");
                assert_eq!(p.tree, t.tree);
                assert_eq!(rank, p.max_rank, "{} needs {} at full rank", t.name, p.name);
            }
        }
        let ids: HashSet<u32> = cat.talents().iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), cat.talents().len(), "talent ids are unique");
    }

    #[test]
    fn every_tree_landmark_is_mapped_to_its_rank_one_spell() {
        let Some((_, cat)) = client() else { return };
        assert_eq!(mechanics::MILESTONES.len(), 27);
        let unique: HashSet<u32> = mechanics::MILESTONES
            .iter()
            .flat_map(|(_, ids)| ids)
            .copied()
            .collect();
        assert_eq!(unique.len(), 81, "three distinct milestones per tree");
        for &(tree, ids) in mechanics::MILESTONES {
            let tree_talents: HashSet<u32> = cat.talents_in(tree).map(|t| t.id).collect();
            for id in ids {
                let talent = cat
                    .talent(id)
                    .unwrap_or_else(|| panic!("milestone {id} exists"));
                assert_eq!(
                    talent.tree, tree,
                    "{} belongs to its landmark tree",
                    talent.name
                );
                assert!(tree_talents.contains(&id));
                let mechanic = mechanics::entry(id)
                    .unwrap_or_else(|| panic!("{} ({id}) has mechanics", talent.name));
                assert_eq!(mechanic.name, talent.name, "milestone {id} name");
                assert!(
                    mechanic
                        .specs
                        .iter()
                        .all(|spec| spec.visual == Some(talent.spells[0])),
                    "{} uses its rank-one Spell.dbc id as its visual",
                    talent.name
                );
            }
        }
    }

    #[test]
    fn learned_mechanics_are_granted_under_their_talent_ids() {
        let mut hero = HeroState::default();
        let ranks = vec![(75, 1), (30, 3), (130, 5)];
        assert!(grant_learned(&mut hero, &ranks).is_none(), "no capstone");
        assert!(game_mechanics::has(&hero, MechanicId::Talent(75)));
        assert!(game_mechanics::has(&hero, MechanicId::Talent(30)));
        assert!(!game_mechanics::has(&hero, MechanicId::Talent(130)));
    }

    /// The 31 points: the spec's start replaces the class's, and the spec rides on the hero.
    #[test]
    fn thirty_one_points_grant_the_spec() {
        let mage = data::CLASSES.iter().find(|c| c.name == "Mage").unwrap();
        let mut hero = HeroState::new(0, mage, 1, Bonuses::default());
        assert_eq!(hero.abilities[0].def.key, "fireball");
        let spec = grant_learned(&mut hero, &[(61, 5), (71, 1)]).expect("Ice Barrier is Frost's");
        assert_eq!(spec.title, "Frost Mage");
        assert_eq!(hero.abilities[0].def.key, "frostbolt");
        assert!(matches!(&spec.summon, spec::Summon::Guardian(g) if g.name == "Water Elemental"));
        assert!(
            game_mechanics::has(&hero, MechanicId::Talent(71)),
            "and Ice Barrier itself"
        );

        // Through the system: the local hero carries its spec, and loses it with the points.
        let mut app = App::new();
        let mut profile = Profile::default();
        profile.classes.insert(
            "mage".into(),
            super::super::profile::ClassRecord {
                talents: TalentSave {
                    xp: 0,
                    ranks: vec![(36, 1)],
                },
                ..default()
            },
        );
        app.add_plugins(bevy::state::app::StatesPlugin)
            .insert_state(super::super::Phase::Playing)
            .insert_resource(profile)
            .insert_resource(Selection {
                class: data::CLASSES.iter().position(|c| c.name == "Mage"),
                ..default()
            })
            .add_systems(Update, grant_talents);
        let e = app
            .world_mut()
            .spawn((HeroState::new(0, mage, 1, Bonuses::default()), LocalHero))
            .id();
        app.update();
        let carried = app.world().get::<HeroSpec>(e).map(|s| s.0.title);
        assert_eq!(carried, Some("Fire Mage"));
        assert_eq!(
            app.world().get::<HeroState>(e).unwrap().abilities[0]
                .def
                .key,
            "pyroblast"
        );
        app.world_mut()
            .resource_mut::<Profile>()
            .classes
            .get_mut("mage")
            .unwrap()
            .talents
            .ranks
            .clear();
        app.world_mut().get_mut::<HeroState>(e).unwrap().hp -= 1.0;
        app.update();
        assert!(app.world().get::<HeroSpec>(e).is_none());
    }

    /// A taught spell is offered only to its learners, at rank 2; a taught passive is offered to
    /// all, its learners taking it at rank 2.
    #[test]
    fn talents_teach_their_spells_instead_of_doubling_them() {
        let mage = data::CLASSES.iter().find(|c| c.name == "Mage").unwrap();
        let mut hero = HeroState::new(0, mage, 1, Bonuses::default());
        assert!(
            !offered(&hero, "pyroblast"),
            "no Pyroblast without the talent"
        );
        assert!(offered(&hero, "frostbolt") && offered(&hero, "kings"));
        assert_eq!(first_rank(&hero, "pyroblast"), 1);
        grant_learned(&mut hero, &[(29, 1), (1442, 1)]);
        assert!(offered(&hero, "pyroblast"));
        assert_eq!(first_rank(&hero, "pyroblast"), 2);
        assert_eq!(first_rank(&hero, "kings"), 2);
        assert!(
            !game_mechanics::has(&hero, MechanicId::Talent(29)),
            "the talent is the spell, not a second Pyroblast"
        );
        assert_eq!(
            mechanic_description(29, 1),
            ["Teaches Pyroblast: your runs offer it, and you take it at rank 2."]
        );
    }

    fn title(key: &str) -> String {
        key.split('_')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Every name the run offers as a card: the class spells (by key), the passives and the
    /// legendary transformations, each with the spell or passive key it belongs to.
    fn offered_names() -> Vec<(String, &'static str)> {
        let mut out: Vec<(String, &'static str)> = data::CLASSES
            .iter()
            .flat_map(|c| c.pool.iter())
            .map(|k| (title(k), *k))
            .collect();
        out.extend(data::PASSIVES.iter().map(|p| (p.name.to_string(), p.key)));
        out.extend(
            data::LEGENDARIES
                .iter()
                .map(|l| (l.name.to_string(), l.ability)),
        );
        out
    }

    /// No talent mechanic shares its name with a spell, passive or legendary the run offers,
    /// unless it teaches exactly that one (the resolution flag).
    #[test]
    fn no_talent_and_offered_spell_share_a_name_unresolved() {
        let offered = offered_names();
        let mut clashes = Vec::new();
        for m in mechanics::all() {
            for (name, key) in &offered {
                if !name.eq_ignore_ascii_case(m.name) {
                    continue;
                }
                let resolved = matches!(
                    m.teaches,
                    Some(mechanics::Teach::Spell(k) | mechanics::Teach::Passive(k)) if k == *key
                );
                if !resolved {
                    clashes.push(format!("talent {} {} vs {key}", m.id, m.name));
                }
            }
        }
        assert!(clashes.is_empty(), "{clashes:#?}");
        // A spec's summon or form is never a second effect under a learned talent's name either,
        // but for the capstone whose effect it is.
        for s in spec::SPECS {
            for m in mechanics::all() {
                if m.name == s.summon_name() {
                    let own = m.id == s.talent;
                    let shown_only = matches!(&s.summon, spec::Summon::Form(f) if f.bonuses.is_empty() && f.specs.is_empty());
                    assert!(
                        own && (m.teaches == Some(mechanics::Teach::Spec) || shown_only),
                        "{} vs talent {}",
                        s.title,
                        m.id
                    );
                }
            }
        }
        // Every teacher teaches something real, and only its own class's spell.
        for t in mechanics::teachers() {
            match t.teaches {
                Some(mechanics::Teach::Spell(k)) => {
                    data::ability(k);
                    assert_eq!(title(k).to_ascii_lowercase(), t.name.to_ascii_lowercase());
                }
                Some(mechanics::Teach::Passive(k)) => {
                    let p = data::PASSIVES.iter().find(|p| p.key == k).unwrap();
                    assert_eq!(p.name, t.name);
                }
                Some(mechanics::Teach::Spec) => {
                    assert_eq!(spec::by_talent(t.id).map(|s| s.summon_name()), Some(t.name));
                }
                None => unreachable!(),
            }
        }
    }

    /// The same audit over the client's own names: every talent in the 27 trees (mechanics or
    /// not) against every spell, passive and legendary the run offers, by `Spell.dbc` name.
    #[test]
    fn no_talent_shares_a_client_name_with_an_offered_spell() {
        let Some((_, cat)) = client() else { return };
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("spells");
        let mut offered: Vec<(String, &'static str)> = data::CLASSES
            .iter()
            .flat_map(|c| c.pool.iter())
            .map(|k| {
                let name = spells
                    .get(data::ability(k).spell)
                    .map_or_else(|| title(k), |s| s.name.clone());
                (name, *k)
            })
            .collect();
        offered.extend(data::PASSIVES.iter().map(|p| (p.name.to_string(), p.key)));
        offered.extend(
            data::LEGENDARIES
                .iter()
                .map(|l| (l.name.to_string(), l.ability)),
        );
        let mut clashes = Vec::new();
        for t in cat.talents() {
            for (name, key) in &offered {
                if !name.eq_ignore_ascii_case(&t.name) {
                    continue;
                }
                let resolved = matches!(
                    mechanics::entry(t.id).and_then(|m| m.teaches),
                    Some(mechanics::Teach::Spell(k) | mechanics::Teach::Passive(k)) if k == *key
                );
                if !resolved {
                    clashes.push(format!("talent {} {} vs {key}", t.id, t.name));
                }
            }
        }
        assert!(clashes.is_empty(), "{clashes:#?}");
    }

    /// The specs against the client's tables: each capstone is its tree's tier-7 talent, each
    /// start spell is named as the spec says, and each summon's spell and model is real.
    #[test]
    fn specs_name_real_capstones_spells_and_models() {
        let Some((_, cat)) = client() else { return };
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("spells");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let mut bad = Vec::new();
        for s in spec::SPECS {
            let Some(t) = cat.talent(s.talent) else {
                bad.push(format!("{}: no talent {}", s.title, s.talent));
                continue;
            };
            if t.tree != s.tree || t.tier != 6 {
                bad.push(format!(
                    "{}: {} is not tier 7 of {}",
                    s.title, t.name, s.tree
                ));
            }
            let tree = cat.tree(s.tree).map(|t| t.name.as_str()).unwrap_or("");
            if !s.title.starts_with(tree.split(' ').next().unwrap_or("?")) {
                bad.push(format!("{}: tree {tree}", s.title));
            }
            let start = data::ability(s.start);
            // A start's spell may be its effect row ("Explosive Trap Effect", "Arcane Missile").
            match spells.get(start.spell) {
                Some(d)
                    if d.name.starts_with(s.start_name) || s.start_name.starts_with(&d.name) => {}
                other => bad.push(format!(
                    "{}: {} is {:?}",
                    s.title,
                    start.spell,
                    other.map(|d| &d.name)
                )),
            }
            match &s.summon {
                spec::Summon::Guardian(g) => {
                    if g.spell != 0 && spells.get(g.spell).is_none() {
                        bad.push(format!("{}: spell {}", g.name, g.spell));
                    }
                    if g.display != 0 && !g.object && creatures.model(g.display).is_none() {
                        bad.push(format!("{}: display {} has no model", g.name, g.display));
                    }
                }
                spec::Summon::Form(f) => {
                    if let Some(a) = f.aura {
                        match spells.get(a) {
                            Some(d) if d.name == f.name => {}
                            other => bad.push(format!(
                                "{}: aura {a} is {:?}",
                                f.name,
                                other.map(|d| &d.name)
                            )),
                        }
                    }
                    for m in f.specs {
                        if let Some(v) = m.visual.filter(|v| spells.get(*v).is_none()) {
                            bad.push(format!("{}: visual {v}", f.name));
                        }
                    }
                    if let Some((elf, tauren)) = f.shape {
                        for d in [elf, tauren] {
                            let path = creatures
                                .model(d)
                                .map(|m| m.model_path.to_ascii_lowercase())
                                .unwrap_or_default();
                            if !path.contains("creature\\druid") {
                                bad.push(format!("{}: shape {d} is {path:?}", f.name));
                            }
                        }
                    }
                }
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// Each guardian's model is the creature it is named for.
    #[test]
    fn guardians_wear_their_creatures_models() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let objects = benilla_formats::load_gameobject_catalog(&mut chain).expect("objects");
        let mut bad = Vec::new();
        for s in spec::SPECS {
            let spec::Summon::Guardian(g) = &s.summon else {
                continue;
            };
            if g.display == 0 {
                continue;
            }
            let want = GUARDIAN_MODELS
                .iter()
                .find(|(name, _)| *name == g.name)
                .map(|(_, m)| *m)
                .unwrap_or_else(|| panic!("{} has no expected model", g.name));
            let got = if g.object {
                objects
                    .model_path(g.display)
                    .map(str::to_ascii_lowercase)
                    .unwrap_or_default()
            } else {
                creatures
                    .model(g.display)
                    .map(|m| m.model_path.to_ascii_lowercase())
                    .unwrap_or_default()
            };
            if !got.contains(want) {
                bad.push(format!("{}: display {} is {got:?}", g.name, g.display));
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    /// The model folder each guardian's display must come from.
    const GUARDIAN_MODELS: &[(&str, &str)] = &[
        ("Frostsaber", "creature\\tiger\\"),
        ("Lightwell", "lightwell"),
        ("Air Elemental", "creature\\airelemental\\"),
        ("Spirit Wolf", "creature\\wolf\\wolf_ghost"),
        (
            "Healing Stream Totem",
            "creature\\spells\\waterelementaltotem",
        ),
        ("Mana Fiend", "creature\\manafiend\\"),
        ("Fire Elemental", "creature\\fireelemental\\"),
        ("Water Elemental", "creature\\waterelemental\\"),
        ("Felhunter", "creature\\felhound\\"),
        ("Doomguard", "creature\\doomguard\\"),
        ("Infernal", "creature\\infernal\\"),
        ("Treant", "creature\\ancientprotector\\"),
    ];

    /// Dev dump of every tree with each talent's effect and the tree's all-ranks totals:
    /// `cargo test -p benilla-app --lib survivors::talents::tests::dump -- --ignored --nocapture`.
    #[test]
    #[ignore = "dev dump"]
    fn dump() {
        let Some((_, cat)) = client() else { return };
        for (class, def) in data::CLASSES.iter().enumerate() {
            for tree in cat.trees_for(class) {
                println!("== {} / {} ({})", def.name, tree.name, tree.id);
                let mut all = Bonuses::default();
                for t in cat.talents_in(tree.id) {
                    let per: Vec<String> = t.effect().iter().map(Bonus::describe).collect();
                    println!(
                        "  {:>4} t{} c{} x{} {:<34} {}",
                        t.id,
                        t.tier,
                        t.column,
                        t.max_rank,
                        t.name,
                        per.join(", ")
                    );
                    for b in t.effect() {
                        all.add(&Bonus {
                            value: b.value * f32::from(t.max_rank),
                            ..*b
                        });
                    }
                }
                println!("  every rank: {}", describe_all(&all));
            }
        }
    }

    #[test]
    #[ignore = "scratch"]
    fn scratch_dump() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("spells");
        let visuals = benilla_formats::load_spell_visual_catalog(&mut chain).expect("vis");
        let Some((_, cat)) = client() else { return };
        let vis = |id: u32| -> String {
            match spells.get(id) {
                None => format!("{id}:MISSING"),
                Some(d) => {
                    let st = visuals.stages(d.visual);
                    format!(
                        "{id}:{}[v{} c{} i{} s{} p{} sch{} pas{}]",
                        d.name,
                        d.visual,
                        st.map_or(0, |s| s.cast),
                        st.map_or(0, |s| s.impact),
                        st.map_or(0, |s| s.state),
                        st.map_or(0, |s| s.area_effect),
                        d.school,
                        d.passive as u8
                    )
                }
            }
        };
        for (class, def) in data::CLASSES.iter().enumerate() {
            for tree in cat.trees_for(class) {
                println!("== {} / {} ({})", def.name, tree.name, tree.id);
                for t in cat.talents_in(tree.id) {
                    let s = spells.get(t.spells[0]);
                    let trig: Vec<String> = s
                        .map(|d| {
                            d.effect_trigger_spell
                                .iter()
                                .filter(|&&x| x != 0)
                                .map(|&x| vis(x))
                                .collect()
                        })
                        .unwrap_or_default();
                    let desc = s.and_then(|d| d.description.clone()).unwrap_or_default();
                    println!(
                        "  {:>4} t{} c{} x{} {:<30} spells{:?} {} trig{:?} | {}",
                        t.id,
                        t.tier,
                        t.column,
                        t.max_rank,
                        t.name,
                        t.spells,
                        vis(t.spells[0]),
                        trig,
                        desc.replace('\n', " ")
                            .chars()
                            .take(160)
                            .collect::<String>()
                    );
                }
            }
        }
    }

    fn describe_all(b: &Bonuses) -> String {
        use StatKey::*;
        let stats = [
            MaxHealth, Damage, Cooldown, CritChance, CritDamage, Area, Targets, MoveSpeed, Regen,
            Pickup, Armor, Experience, Gold, Luck, Lifesteal, Revives,
        ];
        let schools = [
            School::Physical,
            School::Holy,
            School::Fire,
            School::Nature,
            School::Frost,
            School::Shadow,
            School::Arcane,
        ];
        let mut out: Vec<String> = stats
            .into_iter()
            .filter(|&s| b.get(s) != 0.0)
            .map(|s| Bonus::new(s, b.get(s)).describe())
            .collect();
        out.extend(
            schools
                .into_iter()
                .filter(|&s| b.school_damage(s) != 0.0)
                .map(|s| Bonus::school(s, b.school_damage(s)).describe()),
        );
        out.join(", ")
    }
}
