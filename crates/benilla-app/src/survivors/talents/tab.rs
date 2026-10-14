//! The Armory Talents tab's state and input, for the view in `armory/talents.rs`: the chosen
//! talent ([`TalentFocus`]), what a click does to the class's [`TalentSave`] ([`act`]), the words
//! for why it cannot ([`why_text`]), and the client art the view asks for ([`TalentArt`]).
//!
//! Left-click on a talent learns a rank and right-click unlearns one, each choosing it for the
//! Details column, whose Learn and Unlearn act on the chosen talent; Reset is a free respec. Any
//! click rebuilds the tab, and a change is saved.

use bevy::prelude::*;

use super::super::armory::{ArmoryState, ArmoryTab};
use super::super::bonus::Bonus;
use super::super::profile::{slug, Profile};
use super::super::{data, Selection};
use super::progress::{unspent, TalentDevXp};
use super::{
    can_learn, can_unlearn, Allocation, Talent, TalentCatalog, TalentSave, Why, POINTS_PER_TIER,
};
use crate::glue::widgets::GlueDisabled;
use crate::glue::GlueClicks;

/// The talent the Details column describes, chosen by clicking it; kept per class picked.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct TalentFocus {
    chosen: Option<(usize, u32)>,
}

impl TalentFocus {
    /// The talent chosen while `class` (an index into [`data::CLASSES`]) was picked.
    pub fn chosen(&self, class: usize) -> Option<u32> {
        self.chosen.filter(|&(c, _)| c == class).map(|(_, t)| t)
    }
}

/// A talent's button on the tab.
#[derive(Component, Clone, Copy, Debug)]
pub(in crate::survivors) struct TalentSlot(pub u32);

/// What a click asks of the tab: the Details buttons carry one each.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::survivors) enum TalentAction {
    Learn(u32),
    Unlearn(u32),
    Reset,
}

/// An image drawn from the client's archives once they are readable: a talent or tree icon, or a
/// piece of a tree's painting. Its node starts clear, so art the client lacks never shows.
#[derive(Component, Clone, Debug)]
pub(in crate::survivors) struct TalentArt {
    /// The archive path, `.blp` and all.
    pub path: String,
    pub tint: Color,
}

#[derive(Component)]
pub(super) struct TalentArtDone;

/// Whether the class can learn one more rank of `talent` now: the rules allow it, and an earned
/// point is unspent (`dev` is [`TalentDevXp`]).
pub(in crate::survivors) fn check_learn(
    catalog: &TalentCatalog,
    save: &TalentSave,
    dev: u64,
    talent: u32,
) -> Result<(), Why> {
    can_learn(catalog, &save.allocation(), talent)?;
    if unspent(save, dev) == 0 {
        return Err(Why::NoPoints);
    }
    Ok(())
}

/// How the tab colours a talent, as WoW's talent frame does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum TalentShade {
    /// At its last rank: gold.
    Maxed,
    /// Learnable right now: a point to spend, its tier open, its prerequisite met. Green.
    Open,
    /// Anything else, no points left included: grey. With ranks in it, its badge keeps the normal
    /// gold; with none, it is dimmed too.
    Closed,
}

/// `talent`'s [`TalentShade`] for the class's save (`dev` is [`TalentDevXp`]); the Learn button
/// asks the same [`check_learn`].
pub(in crate::survivors) fn shade(
    catalog: &TalentCatalog,
    save: &TalentSave,
    dev: u64,
    talent: &Talent,
) -> TalentShade {
    if save.allocation().rank(talent.id) >= talent.max_rank {
        TalentShade::Maxed
    } else if check_learn(catalog, save, dev, talent.id).is_ok() {
        TalentShade::Open
    } else {
        TalentShade::Closed
    }
}

/// Whether the class can give back one rank of `talent`.
pub(in crate::survivors) fn check_unlearn(
    catalog: &TalentCatalog,
    save: &TalentSave,
    talent: u32,
) -> Result<(), Why> {
    can_unlearn(catalog, &save.allocation(), talent)
}

/// Whether a respec has anything to give back.
pub(in crate::survivors) fn check_reset(save: &TalentSave) -> Result<(), Why> {
    if save.spent() == 0 {
        return Err(Why::NotLearned);
    }
    Ok(())
}

/// Carry out `action` on one class's save; on an error the save is untouched.
pub(in crate::survivors) fn act(
    catalog: &TalentCatalog,
    save: &mut TalentSave,
    dev: u64,
    action: TalentAction,
) -> Result<(), Why> {
    let mut alloc = save.allocation();
    match action {
        TalentAction::Learn(id) => {
            check_learn(catalog, save, dev, id)?;
            alloc.learn(catalog, id)?;
        }
        TalentAction::Unlearn(id) => alloc.unlearn(catalog, id)?,
        TalentAction::Reset => {
            check_reset(save)?;
            alloc = Allocation::default();
        }
    }
    save.set_allocation(&alloc);
    Ok(())
}

/// `why` as a tooltip says it, about `talent`.
pub(in crate::survivors) fn why_text(catalog: &TalentCatalog, talent: u32, why: Why) -> String {
    let name = |id: u32| {
        catalog
            .talent(id)
            .map_or_else(|| "another talent".to_string(), |t| t.name.clone())
    };
    match why {
        Why::Unknown => "Unknown talent".into(),
        Why::NoPoints => "No talent points available".into(),
        Why::MaxRank => "Already at its last rank".into(),
        Why::TierLocked { need, .. } => {
            let tree = catalog
                .talent(talent)
                .and_then(|t| catalog.tree(t.tree))
                .map_or("its tree", |t| t.name.as_str());
            format!("Requires {need} points in {tree} Talents")
        }
        Why::Prereq { talent: p, rank } => {
            let s = if rank == 1 { "" } else { "s" };
            format!("Requires {rank} point{s} in {}", name(p))
        }
        Why::NotLearned => "No ranks to unlearn".into(),
        Why::Required { by } => format!("{} needs it", name(by)),
        Why::Holds { by } => format!("{} needs these points to stay open", name(by)),
    }
}

/// What `rank` ranks of `talent` grant, as one line ("+6% Fire damage, +1% critical strike").
pub(in crate::survivors) fn describe_rank(talent: &Talent, rank: u8) -> String {
    let n = f32::from(rank);
    talent
        .effect()
        .iter()
        .map(|b| {
            Bonus {
                value: b.value * n,
                ..*b
            }
            .describe()
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What stands between `alloc` and `talent`'s next rank in its tree, in tooltip words: the
/// tier's points and the prerequisite, each when unmet. Empty once the talent is open.
pub(in crate::survivors) fn unmet(
    catalog: &TalentCatalog,
    alloc: &Allocation,
    talent: &Talent,
) -> Vec<String> {
    let mut out = Vec::new();
    let need = u32::from(talent.tier) * POINTS_PER_TIER;
    let have = alloc.spent_in(catalog, talent.tree);
    if have < need {
        out.push(why_text(catalog, talent.id, Why::TierLocked { need, have }));
    }
    if let Some((p, rank)) = talent.prereq {
        if alloc.rank(p) < rank {
            out.push(why_text(
                catalog,
                talent.id,
                Why::Prereq { talent: p, rank },
            ));
        }
    }
    out
}

/// Talent clicks on the open tab: left-click learns and right-click unlearns, each choosing the
/// talent; the Details buttons act on the chosen one. A change is saved, and any click rebuilds
/// the tab.
#[allow(clippy::too_many_arguments)]
pub(super) fn talent_input(
    clicks: Res<GlueClicks>,
    mouse: Res<ButtonInput<MouseButton>>,
    slots: Query<(Entity, &Interaction, &TalentSlot)>,
    buttons: Query<(Entity, &TalentAction, &GlueDisabled)>,
    catalog: Option<Res<TalentCatalog>>,
    selection: Res<Selection>,
    dev: Option<Res<TalentDevXp>>,
    mut profile: ResMut<Profile>,
    mut focus: ResMut<TalentFocus>,
    mut armory: ResMut<ArmoryState>,
) {
    let (Some(catalog), Some(class)) = (catalog, selection.class) else {
        return;
    };
    let Some(key) = data::CLASSES.get(class).map(|c| slug(c.name)) else {
        return;
    };
    let right = mouse.just_released(MouseButton::Right);
    let on_slot = slots.iter().find_map(|(e, interaction, slot)| {
        if clicks.hit(e) {
            Some(TalentAction::Learn(slot.0))
        } else if right && *interaction != Interaction::None {
            Some(TalentAction::Unlearn(slot.0))
        } else {
            None
        }
    });
    let action = on_slot.or_else(|| {
        buttons
            .iter()
            .find(|(e, _, disabled)| !disabled.0 && clicks.hit(*e))
            .map(|(_, a, _)| *a)
    });
    let Some(action) = action else {
        return;
    };
    if let Some(TalentAction::Learn(id) | TalentAction::Unlearn(id)) = on_slot {
        focus.chosen = Some((class, id));
    }
    // A copy, so a refused click leaves no empty record behind.
    let mut save = profile
        .classes
        .get(&key)
        .map(|r| r.talents.clone())
        .unwrap_or_default();
    if act(&catalog, &mut save, dev.map_or(0, |d| d.0), action).is_ok() {
        profile.classes.entry(key).or_default().talents = save;
        profile.mark_dirty();
    }
    armory.refresh();
}

/// A tab built before the client's talent tables loaded shows a waiting line: rebuild it once the
/// catalog arrives.
pub(super) fn catalog_arrived(
    catalog: Option<Res<TalentCatalog>>,
    mut armory: ResMut<ArmoryState>,
) {
    if catalog.is_some_and(|c| c.is_added()) && armory.open && armory.tab == ArmoryTab::Talents {
        armory.refresh();
    }
}

/// Draws each new [`TalentArt`] once the archives are readable.
pub(super) fn fill_art(
    mut commands: Commands,
    mut slots: Query<(Entity, &mut ImageNode, &TalentArt), Without<TalentArtDone>>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut assets) = assets else {
        return;
    };
    for (e, mut img, art) in &mut slots {
        commands.entity(e).insert(TalentArtDone);
        if let Some(h) = assets.sprite_texture(&art.path, &mut images) {
            img.image = h;
            img.color = art.tint;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{catalog, TOUGH};
    use super::super::progress::xp_for_points;
    use super::*;
    use crate::survivors::profile::ClassRecord;

    #[test]
    fn clicks_learn_and_unlearn_into_the_save() {
        let cat = catalog();
        let mut save = TalentSave {
            xp: xp_for_points(6),
            ..default()
        };
        for _ in 0..3 {
            act(&cat, &mut save, 0, TalentAction::Learn(9001)).unwrap();
        }
        assert_eq!(save.ranks, [(9001, 3)]);
        assert_eq!(
            act(&cat, &mut save, 0, TalentAction::Learn(9010)),
            Err(Why::TierLocked { need: 5, have: 3 })
        );
        act(&cat, &mut save, 0, TalentAction::Learn(9000)).unwrap();
        act(&cat, &mut save, 0, TalentAction::Learn(9000)).unwrap();
        act(&cat, &mut save, 0, TalentAction::Learn(9010)).unwrap();
        assert_eq!(save.ranks, [(9000, 2), (9001, 3), (9010, 1)]);

        // Every earned point is spent: the rules would allow it, the points do not.
        let before = save.clone();
        assert_eq!(
            act(&cat, &mut save, 0, TalentAction::Learn(9000)),
            Err(Why::NoPoints)
        );
        assert_eq!(
            act(&cat, &mut save, 0, TalentAction::Unlearn(9001)),
            Err(Why::Required { by: 9010 })
        );
        assert_eq!(save, before, "a refused click changes nothing");

        act(&cat, &mut save, 0, TalentAction::Unlearn(9010)).unwrap();
        act(&cat, &mut save, 0, TalentAction::Unlearn(9001)).unwrap();
        assert_eq!(save.ranks, [(9000, 2), (9001, 2)]);

        // What the profile keeps is what the tab learned.
        let mut profile = Profile::default();
        profile.classes.insert(
            "mage".into(),
            ClassRecord {
                talents: save.clone(),
                ..default()
            },
        );
        let text = toml::to_string_pretty(&profile).unwrap();
        let back: Profile = toml::from_str(&text).unwrap();
        assert_eq!(back.classes["mage"].talents, save);

        act(&cat, &mut save, 0, TalentAction::Reset).unwrap();
        assert!(save.ranks.is_empty());
        assert_eq!(save.xp, xp_for_points(6), "a respec is free");
        assert_eq!(
            act(&cat, &mut save, 0, TalentAction::Reset),
            Err(Why::NotLearned)
        );
    }

    #[test]
    fn talents_shade_as_wow_colours_them() {
        let cat = catalog();
        let t = |id: u32| cat.talent(id).unwrap();
        // No points at all: even an open tier-0 talent is grey.
        let mut save = TalentSave::default();
        assert_eq!(shade(&cat, &save, 0, t(9000)), TalentShade::Closed);
        assert_eq!(check_learn(&cat, &save, 0, 9000), Err(Why::NoPoints));
        // The dev XP counts as earned.
        assert_eq!(
            shade(&cat, &save, xp_for_points(1), t(9000)),
            TalentShade::Open
        );
        // Points to spend: tier 0 is green, tier 1 grey until its tier and prerequisite are met.
        save.xp = xp_for_points(6);
        assert_eq!(shade(&cat, &save, 0, t(9000)), TalentShade::Open);
        assert_eq!(shade(&cat, &save, 0, t(9010)), TalentShade::Closed);
        // At its last rank, gold, points or none.
        for _ in 0..3 {
            act(&cat, &mut save, 0, TalentAction::Learn(9001)).unwrap();
        }
        assert_eq!(shade(&cat, &save, 0, t(9001)), TalentShade::Maxed);
        for _ in 0..2 {
            act(&cat, &mut save, 0, TalentAction::Learn(9000)).unwrap();
        }
        assert_eq!(shade(&cat, &save, 0, t(9010)), TalentShade::Open);
        act(&cat, &mut save, 0, TalentAction::Learn(9010)).unwrap();
        // Every point spent: the half-learned talent goes grey, and the reason reads as WoW's.
        assert_eq!(shade(&cat, &save, 0, t(9000)), TalentShade::Closed);
        assert_eq!(shade(&cat, &save, 0, t(9010)), TalentShade::Maxed);
        assert_eq!(shade(&cat, &save, 0, t(9001)), TalentShade::Maxed);
        assert_eq!(check_learn(&cat, &save, 0, 9000), Err(Why::NoPoints));
        assert_eq!(
            why_text(&cat, 9000, Why::NoPoints),
            "No talent points available"
        );
    }

    #[test]
    fn the_dev_xp_buys_points_without_touching_the_save() {
        let cat = catalog();
        let mut save = TalentSave::default();
        assert_eq!(
            act(&cat, &mut save, 0, TalentAction::Learn(TOUGH)),
            Err(Why::NoPoints)
        );
        act(
            &cat,
            &mut save,
            xp_for_points(1),
            TalentAction::Learn(TOUGH),
        )
        .unwrap();
        assert_eq!((save.xp, save.ranks.as_slice()), (0, &[(TOUGH, 1)][..]));
    }

    #[test]
    fn reasons_and_ranks_read_as_tooltips() {
        let cat = catalog();
        let mut alloc = Allocation::default();
        let gated = cat.talent(9010).unwrap();
        assert_eq!(
            unmet(&cat, &alloc, gated),
            [
                "Requires 5 points in Tree 1 Talents",
                "Requires 3 points in t9001"
            ]
        );
        for _ in 0..3 {
            alloc.learn(&cat, 9001).unwrap();
        }
        for _ in 0..2 {
            alloc.learn(&cat, 9000).unwrap();
        }
        assert!(unmet(&cat, &alloc, gated).is_empty());
        assert_eq!(
            why_text(&cat, 9001, Why::Required { by: 9010 }),
            "t9010 needs it"
        );
        assert_eq!(
            why_text(
                &cat,
                9000,
                Why::Prereq {
                    talent: 9001,
                    rank: 1
                }
            ),
            "Requires 1 point in t9001"
        );

        let tough = cat.talent(TOUGH).unwrap();
        let per_rank: Vec<String> = tough.effect().iter().map(Bonus::describe).collect();
        assert_eq!(describe_rank(tough, 1), per_rank.join(", "));
        let doubled: Vec<String> = tough
            .effect()
            .iter()
            .map(|b| {
                Bonus {
                    value: b.value * 2.0,
                    ..*b
                }
                .describe()
            })
            .collect();
        assert_eq!(describe_rank(tough, 2), doubled.join(", "));
        assert!(
            describe_rank(gated, 1).is_empty(),
            "made-up ids grant nothing"
        );
    }

    #[test]
    fn the_focus_belongs_to_the_class_it_was_chosen_for() {
        let focus = TalentFocus {
            chosen: Some((2, 35)),
        };
        assert_eq!(focus.chosen(2), Some(35));
        assert_eq!(focus.chosen(3), None);
        assert_eq!(TalentFocus::default().chosen(2), None);
    }
}
