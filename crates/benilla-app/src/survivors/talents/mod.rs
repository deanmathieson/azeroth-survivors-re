//! The talent catalog: the 27 vanilla trees, three per class, read from the client's own
//! `Talent.dbc` and `TalentTab.dbc` (the [`crate::ui_talent::Talents`] load), with each talent's
//! name and icon from its rank-1 spell. What a rank grants is the mode's own design
//! ([`effects`]); how points may be spent is vanilla's ([`rules`]).

// The talent screen and the stat layer consume this in later workstreams; drop this once they do.
#![allow(dead_code)]

mod effects;
mod rules;

use std::collections::HashMap;

use bevy::prelude::*;

use super::bonus::Bonus;
use super::data;

#[allow(unused_imports)] // The talent screen and the stat layer are the users, still to land.
pub(super) use rules::{
    bonuses, can_learn, can_unlearn, Allocation, Why, MAX_POINTS, POINTS_PER_TIER,
};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        build_catalog.run_if(not(resource_exists::<TalentCatalog>)),
    );
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
        effects::per_rank(self.id).unwrap_or(&[])
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::OnceLock;

    use super::super::bonus::{Bonuses, School, StatKey};
    use super::*;

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
