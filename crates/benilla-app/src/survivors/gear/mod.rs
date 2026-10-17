//! Gear: one piece per slot and rarity for every class ([`catalog`]), dropped by the battlegrounds'
//! bosses ([`drops`]). Names and looks are real 1.12 items: each display id is a shipped
//! `ItemDisplayInfo.dbc` row and each icon its inventory icon. The stats are the mode's own, in the
//! shared [`Bonus`] vocabulary, scaled by rarity so a full Legendary set about doubles a hero's
//! output.
//!
//! A slain roster boss's chest rolls its drop as it opens ([`loot`]); new pieces join the class's collection in its
//! profile record ([`save`], through [`store`]) and go on when they beat what the slot wears. What
//! the picked class wears is its share of the meta bonuses ([`MetaParts::gear`]), and its main
//! hand is the hero's weapon. The Armory's Collection, Drop Tables and Character tabs show it all
//! ([`view`], [`ui`]).
//!
//! The hero wears it too: its body is the race's character model dressed in every worn armour
//! piece over its class trainer's outfit ([`outfit`]), with the name and features the class keeps
//! beside its gear ([`looks`]), picked in the Character tab's Appearance panel ([`looks_ui`]).
//!
//! `WOW_SURVIVORS_GEAR=all|<n>` collects every piece of each class, or its first `n` in catalog
//! order (Common first), and wears the best of them, in memory only: nothing of it is saved.

mod catalog;
mod drops;
mod looks;
pub(super) mod looks_create;
mod looks_ui;
mod loot;
mod outfit;
mod save;
mod store;
mod ui;
mod view;

use bevy::prelude::*;

use super::bonus::{Bonus, Bonuses};
pub(super) use super::cards::Rarity;
use super::meta::MetaParts;
use super::{enemies, hud, profile, treasure, Phase, Selection};
pub(super) use looks::results_title;
pub(super) use looks::Features;
pub(super) use looks_ui::{hero_name, looks_panel, saved_body, HeroNameText, HeroNames, LooksDice};
#[cfg_attr(not(feature = "coop"), allow(unused_imports))]
pub(super) use loot::{GroupDrops, WonLoot};
pub(super) use outfit::{wear, Dresser};
pub(super) use save::GearSave;
pub(in crate::survivors) use store::{class_gear, GearRead};
pub(super) use ui::{GearAction, GearView, ItemIcon};
pub(super) use view::{
    boss_rows, clock, percent, progress, rarity_label, sources, summed, zone_rows, zones_dropping,
    BossRow, ZoneRow, RARITIES,
};

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<loot::LootToasts>()
        .init_resource::<loot::GroupDrops>()
        .init_resource::<loot::WonLoot>()
        .init_resource::<GearView>()
        .init_resource::<outfit::Wardrobe>()
        .init_resource::<looks_ui::NameEdit>()
        .init_resource::<looks_ui::LooksDice>()
        .init_resource::<looks_create::CreateSession>()
        .add_systems(Startup, shadow_for_auto)
        // The name box takes its keys before any other reader sees them.
        .add_systems(
            PreUpdate,
            looks_ui::type_name.after(bevy::input::InputSystems),
        )
        .add_systems(
            Update,
            (
                outfit::dress_bodies::<crate::entities::Creatures>,
                looks_create::build,
                looks_ui::add_menu_name,
                (
                    looks_create::input,
                    looks_create::rotate,
                    looks_ui::looks_clicks,
                    looks_ui::remember_body,
                    looks_ui::roll_dice,
                    looks_ui::refresh_looks,
                )
                    .chain(),
            ),
        )
        .add_systems(
            Update,
            (
                store::fill_shadow.run_if(resource_exists::<store::Shadow>),
                (loot::roll_loot, apply_gear, loot::dress_hero).chain(),
            )
                .chain()
                .after(enemies::drop_loot),
        )
        .add_systems(
            Update,
            (
                loot::show_toasts.after(hud::refresh_hud),
                loot::grant_won.before(loot::show_toasts),
                ui::gear_clicks,
                ui::fill_item_icons,
            ),
        )
        .add_systems(OnEnter(Phase::Menu), loot::clear_toasts)
        // The chests the run's end opened roll ahead of its record, so their gold is in it.
        .add_systems(
            OnEnter(Phase::GameOver),
            loot::roll_loot
                .after(treasure::open_leftover_chests)
                .before(profile::record_finished),
        );
    let Some(text) = crate::webenv::var("WOW_SURVIVORS_GEAR") else {
        return;
    };
    match store::dev_count(&text) {
        Some(n) => {
            app.insert_resource(store::Shadow::with_extra(n));
        }
        None => warn!("survivors: WOW_SURVIVORS_GEAR={text:?} (want all or a count)"),
    }
}

/// An auto-runner run the profile does not record ([`super::profile::record_finished`]) keeps
/// its drops in memory too.
fn shadow_for_auto(
    mut commands: Commands,
    auto: Option<Res<super::auto::Auto>>,
    shadow: Option<Res<store::Shadow>>,
) {
    if auto.is_some()
        && shadow.is_none()
        && std::env::var("WOW_SURVIVORS_RECORD").as_deref() != Ok("1")
    {
        commands.insert_resource(store::Shadow::default());
    }
}

/// [`MetaParts::gear`] is what the picked class wears, kept in step as the pick or the gear
/// changes.
fn apply_gear(selection: Res<Selection>, gear: store::GearRead, mut parts: ResMut<MetaParts>) {
    if !selection.is_changed() && !gear.is_changed() {
        return;
    }
    let want = selection.class.map_or_else(Bonuses::default, |c| {
        bonuses_of(&gear.get(c).equipped_ids())
    });
    if parts.gear != want {
        parts.gear = want;
    }
}

/// Where a piece is worn: the nine slots the catalog fills for every class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Slot {
    Head,
    Shoulder,
    Chest,
    Waist,
    Legs,
    Feet,
    Wrist,
    Hands,
    MainHand,
}

impl Slot {
    pub(super) const ALL: [Slot; 9] = [
        Slot::Head,
        Slot::Shoulder,
        Slot::Chest,
        Slot::Waist,
        Slot::Legs,
        Slot::Feet,
        Slot::Wrist,
        Slot::Hands,
        Slot::MainHand,
    ];

    /// The slot's tooltip name, as the 1.12 item tooltip prints it.
    pub(super) fn label(self) -> &'static str {
        match self {
            Slot::Head => "Head",
            Slot::Shoulder => "Shoulder",
            Slot::Chest => "Chest",
            Slot::Waist => "Waist",
            Slot::Legs => "Legs",
            Slot::Feet => "Feet",
            Slot::Wrist => "Wrist",
            Slot::Hands => "Hands",
            Slot::MainHand => "Main Hand",
        }
    }

    /// The slot's key in a save's [`GearSave::equipped`]; never rename one.
    pub(super) fn key(self) -> &'static str {
        match self {
            Slot::Head => "head",
            Slot::Shoulder => "shoulder",
            Slot::Chest => "chest",
            Slot::Waist => "waist",
            Slot::Legs => "legs",
            Slot::Feet => "feet",
            Slot::Wrist => "wrist",
            Slot::Hands => "hands",
            Slot::MainHand => "main_hand",
        }
    }
}

/// One piece of gear.
#[derive(Debug)]
pub(super) struct Item {
    /// Stable across releases: saves will store it, so a piece keeps its id for good.
    pub id: u16,
    /// An index into [`super::data::CLASSES`].
    pub class: usize,
    pub slot: Slot,
    pub rarity: Rarity,
    pub name: &'static str,
    /// The `ItemDisplayInfo.dbc` row it is drawn with.
    pub display_id: u32,
    /// The display's inventory icon: an `Interface\Icons\` basename without extension.
    pub icon: &'static str,
    pub bonuses: &'static [Bonus],
}

/// What one boss drops. A kill rolls each of `slots` on its own at `chance` (raised by the
/// rewards multiplier and luck, [`loot::drop_odds`]); each hit drops that slot's piece at
/// `rarity` for the hero's own class ([`piece`]).
#[derive(Debug)]
pub(super) struct BossDrop {
    /// The boss's roster name, kept beside its index so a reordered roster fails the tests.
    pub boss: &'static str,
    pub slots: &'static [Slot],
    pub rarity: Rarity,
    /// In `(0, 1]`.
    pub chance: f32,
}

/// One battleground's bosses, in roster order.
pub(super) struct ZoneDrops {
    /// The [`super::data::MapDef::name`] it belongs to.
    pub map: &'static str,
    pub bosses: &'static [BossDrop],
}

/// Every piece of class `class` (an index into [`super::data::CLASSES`]), 45 of them: one per
/// slot and rarity. Empty for an index past the classes.
pub(super) fn items_for(class: usize) -> &'static [Item] {
    catalog::CLASS_ITEMS.get(class).copied().unwrap_or(&[])
}

/// The piece with id `id`, from any class.
pub(super) fn item(id: u16) -> Option<&'static Item> {
    catalog::CLASS_ITEMS
        .iter()
        .flat_map(|items| items.iter())
        .find(|i| i.id == id)
}

/// The piece a drop resolves to: class `class`'s for `slot` at `rarity`.
pub(super) fn piece(class: usize, slot: Slot, rarity: Rarity) -> Option<&'static Item> {
    items_for(class)
        .iter()
        .find(|i| i.slot == slot && i.rarity == rarity)
}

/// What boss `boss` (an index into the battleground's roster) of [`super::data::MAPS`] entry
/// `map` drops; `None` past either end.
pub(super) fn drops_for(map: usize, boss: usize) -> Option<&'static BossDrop> {
    drops::ZONES.get(map)?.bosses.get(boss)
}

/// The summed bonuses of the pieces `ids`; an unknown id adds nothing.
pub(super) fn bonuses_of(ids: &[u16]) -> Bonuses {
    let mut total = Bonuses::default();
    for i in ids.iter().filter_map(|&id| item(id)) {
        total.add_all(i.bonuses);
    }
    total
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::bonus::StatKey;
    use super::super::data::{roster, CLASSES, MAPS};
    use super::*;

    const RARITIES: [Rarity; 5] = [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Epic,
        Rarity::Legendary,
    ];

    fn all_items() -> impl Iterator<Item = &'static Item> {
        (0..CLASSES.len()).flat_map(items_for)
    }

    /// Every `(map, boss index, drop)`.
    fn all_drops() -> impl Iterator<Item = (usize, usize, &'static BossDrop)> {
        drops::ZONES
            .iter()
            .enumerate()
            .flat_map(|(m, z)| z.bosses.iter().enumerate().map(move |(b, d)| (m, b, d)))
    }

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for i in all_items() {
            assert!(seen.insert(i.id), "id {} is used twice", i.id);
        }
        assert_eq!(seen.len(), CLASSES.len() * 45);
    }

    #[test]
    fn every_class_has_one_piece_per_slot_and_rarity() {
        assert_eq!(catalog::CLASS_ITEMS.len(), CLASSES.len());
        for (class, def) in CLASSES.iter().enumerate() {
            let items = items_for(class);
            assert_eq!(items.len(), 45, "{}", def.name);
            for slot in Slot::ALL {
                for rarity in RARITIES {
                    let n = items
                        .iter()
                        .filter(|i| i.slot == slot && i.rarity == rarity)
                        .count();
                    assert_eq!(n, 1, "{} {slot:?} {rarity:?}", def.name);
                }
            }
            for i in items {
                assert_eq!(i.class, class, "{} sits under the wrong class", i.name);
                assert!(!i.name.is_empty() && !i.icon.is_empty() && i.display_id > 0);
                assert!(!i.bonuses.is_empty(), "{} grants nothing", i.name);
            }
        }
        assert!(items_for(CLASSES.len()).is_empty());
    }

    #[test]
    fn rarer_pieces_grant_more() {
        // Within a class and slot, each rarity carries at least as many bonuses as the one below,
        // and its lead bonus is larger.
        for class in 0..CLASSES.len() {
            for slot in Slot::ALL {
                let tiers: Vec<_> = RARITIES
                    .iter()
                    .map(|&r| piece(class, slot, r).unwrap())
                    .collect();
                for pair in tiers.windows(2) {
                    let (lo, hi) = (pair[0], pair[1]);
                    assert!(hi.bonuses.len() >= lo.bonuses.len(), "{}", hi.name);
                    assert!(hi.bonuses[0].value > lo.bonuses[0].value, "{}", hi.name);
                }
            }
        }
    }

    #[test]
    fn lookups_find_pieces() {
        let first = &items_for(0)[0];
        assert_eq!(item(first.id).map(|i| i.name), Some(first.name));
        assert!(item(0).is_none());
        let p = piece(6, Slot::MainHand, Rarity::Legendary).unwrap();
        assert_eq!(p.class, 6);
        assert_eq!(p.slot, Slot::MainHand);
        assert_eq!(p.rarity, Rarity::Legendary);
    }

    #[test]
    fn bonuses_of_sums_the_pieces() {
        let a = &items_for(0)[0];
        let b = &items_for(0)[1];
        let mut want = Bonuses::default();
        want.add_all(a.bonuses);
        want.add_all(b.bonuses);
        // An unknown id adds nothing.
        assert_eq!(bonuses_of(&[a.id, b.id, 0]), want);
        assert_eq!(bonuses_of(&[]), Bonuses::default());
        // Every Legendary main hand adds a target.
        for class in 0..CLASSES.len() {
            let mh = piece(class, Slot::MainHand, Rarity::Legendary).unwrap();
            assert_eq!(bonuses_of(&[mh.id]).get(StatKey::Targets), 1.0);
        }
    }

    #[test]
    fn meta_gear_is_what_the_picked_class_wears() {
        use super::super::profile::{slug, Profile};
        let mut app = App::new();
        app.init_resource::<MetaParts>()
            .init_resource::<Selection>()
            .insert_resource(Profile::default())
            .add_systems(Update, apply_gear);
        app.update();
        assert_eq!(app.world().resource::<MetaParts>().gear, Bonuses::default());

        let collect = |app: &mut App, class: usize, slot: Slot, rarity: Rarity| {
            let mut profile = app.world_mut().resource_mut::<Profile>();
            let record = profile.classes.entry(slug(CLASSES[class].name));
            record
                .or_default()
                .gear
                .collect(piece(class, slot, rarity).unwrap());
        };
        let worn =
            |app: &App, class: usize| bonuses_of(&class_gear(app.world(), class).equipped_ids());
        let meta = |app: &App| app.world().resource::<MetaParts>().gear.clone();
        collect(&mut app, 5, Slot::Head, Rarity::Rare);
        collect(&mut app, 5, Slot::MainHand, Rarity::Rare);
        collect(&mut app, 6, Slot::Chest, Rarity::Rare);
        app.world_mut().resource_mut::<Selection>().class = Some(5);
        app.update();
        assert_ne!(meta(&app), Bonuses::default());
        assert_eq!(meta(&app), worn(&app, 5));

        // Another pick, then a better piece for it.
        app.world_mut().resource_mut::<Selection>().class = Some(6);
        app.update();
        assert_eq!(meta(&app), worn(&app, 6));
        let before = meta(&app);
        collect(&mut app, 6, Slot::Chest, Rarity::Epic);
        app.update();
        assert_eq!(meta(&app), worn(&app, 6));
        assert_ne!(meta(&app), before);

        // No pick wears nothing.
        app.world_mut().resource_mut::<Selection>().class = None;
        app.update();
        assert_eq!(meta(&app), Bonuses::default());
    }

    #[test]
    fn drops_line_up_with_maps_and_rosters() {
        assert_eq!(
            drops::ZONES.len(),
            MAPS.iter().filter(|m| m.map != 409).count()
        );
        for (m, zone) in drops::ZONES.iter().enumerate() {
            assert_eq!(zone.map, MAPS[m].name, "zone {m} is out of order");
            let bosses = &roster(m).bosses;
            assert_eq!(zone.bosses.len(), bosses.len(), "{}", zone.map);
            for (b, d) in zone.bosses.iter().enumerate() {
                assert_eq!(d.boss, bosses[b].name, "{} boss {b}", zone.map);
                assert!(std::ptr::eq(drops_for(m, b).unwrap(), d));
            }
            assert!(drops_for(m, bosses.len()).is_none());
        }
        assert!(drops_for(MAPS.len(), 0).is_none());
    }

    #[test]
    fn chances_are_in_range() {
        for (_, _, d) in all_drops() {
            assert!(
                d.chance > 0.0 && d.chance <= 1.0,
                "{}: {}",
                d.boss,
                d.chance
            );
            assert!(!d.slots.is_empty(), "{} drops nothing", d.boss);
        }
    }

    #[test]
    fn every_piece_drops_somewhere() {
        for i in all_items() {
            assert!(
                all_drops().any(|(_, _, d)| d.rarity == i.rarity && d.slots.contains(&i.slot)),
                "nothing drops {} ({:?} {:?})",
                i.name,
                i.slot,
                i.rarity
            );
        }
    }

    #[test]
    fn zones_finish_sets_and_legendaries_are_last() {
        let mut legendary_zones = 0;
        for zone in drops::ZONES {
            let (last, rest) = zone.bosses.split_last().unwrap();
            assert!(
                rest.iter().all(|d| d.rarity == rest[0].rarity),
                "{}",
                zone.map
            );
            assert_ne!(rest[0].rarity, Rarity::Legendary, "{}", zone.map);
            let mut slots: Vec<Slot> = rest.iter().flat_map(|d| d.slots.iter().copied()).collect();
            if last.rarity == Rarity::Legendary {
                legendary_zones += 1;
            } else {
                assert_eq!(last.rarity, rest[0].rarity, "{}", zone.map);
                slots.extend(last.slots);
            }
            for slot in Slot::ALL {
                assert!(slots.contains(&slot), "{} drops no {slot:?}", zone.map);
            }
        }
        assert_eq!(legendary_zones, 2);
    }

    /// The looks are real: every display id is an `ItemDisplayInfo.dbc` row whose inventory icon
    /// is the piece's, and every icon ships as `Interface\Icons\<name>.blp`.
    #[test]
    fn displays_and_icons_exist_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut bad = Vec::new();
        for i in all_items() {
            let path = format!("Interface\\Icons\\{}", i.icon);
            match cat.get(i.display_id) {
                None => bad.push(format!("{}: no display {}", i.name, i.display_id)),
                Some(d)
                    if !d
                        .icon
                        .as_deref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(&path)) =>
                {
                    bad.push(format!(
                        "{}: display {} shows {:?}",
                        i.name, i.display_id, d.icon
                    ))
                }
                Some(_) => {}
            }
            if !chain.contains(&format!("{path}.blp")) {
                bad.push(format!("{}: no {path}.blp", i.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// Dev tool, not a check: dumps every `ItemDisplayInfo.dbc` row to
    /// `<temp>/w2-gear/item_display_info.tsv` (id, models, model textures, icon, body-region
    /// textures), to search for a set's looks by texture name when authoring [`catalog`].
    #[test]
    #[ignore = "dev tool: writes a TSV of ItemDisplayInfo.dbc"]
    fn dump_item_display_info() {
        use std::fmt::Write as _;
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut rows: Vec<_> = cat.iter().collect();
        rows.sort_by_key(|(id, _)| *id);
        let mut out = String::from(
            "id\tmodel_l\tmodel_r\ttex_l\ttex_r\ticon\tarm_upper\tarm_lower\thand\ttorso_upper\t\
             torso_lower\tleg_upper\tleg_lower\tfoot\n",
        );
        let s = |v: &Option<String>| v.clone().unwrap_or_default();
        for (id, d) in rows {
            let icon = d.icon.as_deref().unwrap_or("");
            let icon = icon.strip_prefix("Interface\\Icons\\").unwrap_or(icon);
            let _ = write!(
                out,
                "{id}\t{}\t{}\t{}\t{}\t{icon}",
                s(&d.model[0]),
                s(&d.model[1]),
                s(&d.model_texture[0]),
                s(&d.model_texture[1]),
            );
            for t in &d.region_textures {
                let _ = write!(out, "\t{}", s(t));
            }
            out.push('\n');
        }
        let dir = std::env::temp_dir().join("w2-gear");
        std::fs::create_dir_all(&dir).expect("create the dump folder");
        let path = dir.join("item_display_info.tsv");
        std::fs::write(&path, out).expect("write the dump");
        eprintln!("{} rows to {}", cat.len(), path.display());
    }
}
