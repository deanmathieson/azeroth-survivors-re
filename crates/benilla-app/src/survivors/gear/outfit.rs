//! The hero's body, dressed as the client dresses a character: the race and sex character model,
//! its skin, face, hair and facial hair composited live from `CharSections.dbc`, and every worn
//! armour piece painted into the atlas, its geosets and its helm and shoulder models, from
//! `ItemDisplayInfo` ids.
//!
//! No wire carries a local hero's look, so the dress is registered as a creature display
//! ([`crate::entities::Creatures::insert_dressed`]): the class trainer's `CreatureDisplayInfo` row
//! (its character model, scale, voice and blood) under a `CreatureDisplayInfoExtra` row of our own
//! with no baked atlas. The attach then builds it through the character path every humanoid body
//! takes (`resolve_char_look`, the `skin_composite` atlas, `equip_geosets`, the helm and shoulder
//! attach in `resolve_equipment`), and the hero stays a unit, so its animation driver, ground
//! clamp, spell visuals and mobility casts are the ones it had. The main hand is the creature
//! weapon slot [`super::loot::dress_hero`] fills.
//!
//! What a slot wears: the gear's piece, else the trainer's own column, so an empty slot keeps the
//! class's look; a trainer with no appearance row falls back to the race and class
//! `CharStartOutfit`. A [`Dress`] is registered once and its display id kept ([`Wardrobe`]); a hero
//! changes display only when its dress does, and the attach composites once per change.

use std::collections::HashMap;

use benilla_formats::{CharCreateCatalog, NpcAppearance, StartOutfitItem};
use benilla_protocol::messages::ObjectFields;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::data::CLASSES;
use super::super::hero::Hero;
use super::super::Selection;
use super::looks::{feature_ranges, worn_features, Features};
use super::store::GearRead;
use super::{GearSave, Slot};
use crate::entities::{CharCreate, Creatures};
use crate::net::{NetEntity, ObjectStore};
use crate::survivors::faire::catalog::equipped_tabard;
use crate::survivors::profile::Profile;

/// `UNIT_FIELD_DISPLAYID`: a change swaps the standing body (`entities::live_display`).
const FIELD_DISPLAYID: u16 = 131;

/// The first id a dress is registered under; `CreatureDisplayInfo` and
/// `CreatureDisplayInfoExtra` ids stay far below it.
const FIRST_DRESS_ID: u32 = 0x7D00_0000;

/// [`NpcAppearance::equipment`]'s columns: head, shoulder, shirt, chest, belt, pants, boots,
/// wrist, gloves, tabard.
pub(super) const COLUMNS: usize = 10;
const SHIRT: usize = 2;
const CHEST: usize = 3;
const TABARD: usize = 9;

/// The column gear slot `slot` dresses; `None` for the main hand, a held weapon.
pub(super) fn column(slot: Slot) -> Option<usize> {
    Some(match slot {
        Slot::Head => 0,
        Slot::Shoulder => 1,
        Slot::Chest => CHEST,
        Slot::Waist => 4,
        Slot::Legs => 5,
        Slot::Feet => 6,
        Slot::Wrist => 7,
        Slot::Hands => 8,
        Slot::MainHand => return None,
    })
}

/// A `CharStartOutfit` set by column, its held and unrendered items dropped.
pub(super) fn outfit_columns(items: &[StartOutfitItem]) -> [u32; COLUMNS] {
    let mut out = [0; COLUMNS];
    for it in items {
        // The `EQUIPMENT_SLOT_*` index, then its column: head, shoulders, shirt to hands, tabard.
        let col = match crate::entities::equip_slot(it.inv_type) {
            Some(0) => 0,
            Some(2) => 1,
            Some(s @ 3..=9) => s - 1,
            Some(18) => TABARD,
            _ => continue,
        };
        out[col] = it.display_id;
    }
    out
}

/// The class's own outfit: its trainer's columns, else the race and class starting outfit.
pub(super) fn starter(
    trainer: Option<&NpcAppearance>,
    outfit: &[StartOutfitItem],
) -> [u32; COLUMNS] {
    trainer.map_or_else(|| outfit_columns(outfit), |npc| npc.equipment)
}

/// What each column wears: the starter's, each slot the gear wears over its own column. A worn
/// chest also takes the starter's tabard and shirt off, so the piece shows whole.
pub(super) fn worn_columns(starter: [u32; COLUMNS], gear: &GearSave) -> [u32; COLUMNS] {
    let mut out = starter;
    for slot in Slot::ALL {
        if let (Some(col), Some(piece)) = (column(slot), gear.worn(slot)) {
            out[col] = piece.display_id;
        }
    }
    if gear.worn(Slot::Chest).is_some() {
        out[TABARD] = 0;
        out[SHIRT] = 0;
    }
    out
}

/// One hero body: the trainer display it is built on, its race and sex, its features and what
/// each column wears. Equal dresses are one display.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) struct Dress {
    pub base: u32,
    pub race: u8,
    pub sex: u8,
    pub features: Features,
    pub equipment: [u32; COLUMNS],
}

impl Dress {
    /// The dress as an appearance row: composited live, never a baked atlas.
    pub fn appearance(&self) -> NpcAppearance {
        let f = self.features;
        NpcAppearance {
            race: self.race,
            sex: self.sex,
            skin: f.skin,
            face: f.face,
            hair_style: f.hair_style,
            hair_color: f.hair_color,
            facial_hair: f.facial_hair,
            equipment: self.equipment,
            bake_name: None,
        }
    }
}

/// The dress of class `class`'s hero in `look`, `(trainer display, race, sex)`: its picked
/// features or the trainer's, and its gear over the trainer's outfit.
pub(super) fn plan(
    class: usize,
    look: (u32, u8, u8),
    gear: &GearSave,
    trainer: Option<&NpcAppearance>,
    cc: Option<&CharCreateCatalog>,
    tabard: Option<u32>,
) -> Dress {
    let (base, race, sex) = look;
    let counts = feature_ranges(cc, race, sex);
    let class_id = CLASSES.get(class).map_or(0, |c| c.id);
    let outfit = cc.map_or(&[][..], |c| c.start_outfit(race, class_id, sex));
    Dress {
        base,
        race,
        sex,
        features: worn_features(&gear.looks, race, sex, trainer, counts),
        equipment: {
            let mut equipment = worn_columns(starter(trainer, outfit), gear);
            if gear.worn(Slot::Chest).is_none() {
                if let Some(tabard) = tabard {
                    equipment[TABARD] = tabard;
                }
            }
            equipment
        },
    }
}

/// Where a dress is registered: [`Creatures`] in the game.
pub(super) trait Catalog {
    /// A display's appearance row, the class trainer's look and outfit.
    fn npc_look(&self, display: u32) -> Option<NpcAppearance>;
    /// Register `id` as `base` worn with `look`; false when `base` has no row.
    fn register(&mut self, id: u32, base: u32, look: NpcAppearance) -> bool;
}

impl Catalog for Creatures {
    fn npc_look(&self, display: u32) -> Option<NpcAppearance> {
        self.npc_appearance(display)
    }

    fn register(&mut self, id: u32, base: u32, look: NpcAppearance) -> bool {
        self.insert_dressed(id, base, look)
    }
}

/// Every dress registered so far, with its display id. Kept for the session, so a look worn
/// before costs a lookup and its composite is still cached.
#[derive(Resource, Default)]
pub(super) struct Wardrobe {
    ids: HashMap<Dress, u32>,
}

impl Wardrobe {
    /// The display of a dress already registered.
    pub fn cached(&self, dress: &Dress) -> Option<u32> {
        self.ids.get(dress).copied()
    }

    /// Register `dress` under the next free id; `None` when its base has no row.
    pub fn register(&mut self, dress: &Dress, catalog: &mut impl Catalog) -> Option<u32> {
        let id = FIRST_DRESS_ID + self.ids.len() as u32;
        if !catalog.register(id, dress.base, dress.appearance()) {
            return None;
        }
        self.ids.insert(*dress, id);
        Some(id)
    }
}

/// The display of the dress of class `class`'s hero in `look`, registering it on first wear. The
/// catalog is written only then: [`Creatures`] changing re-resolves every unit's equipment.
fn dressed<C: Catalog + Resource>(
    wardrobe: &mut Wardrobe,
    catalog: &mut ResMut<C>,
    class: usize,
    look: (u32, u8, u8),
    gear: &GearSave,
    cc: Option<&CharCreateCatalog>,
    tabard: Option<u32>,
) -> Option<u32> {
    let trainer = catalog.npc_look(look.0);
    let dress = plan(class, look, gear, trainer.as_ref(), cc, tabard);
    match wardrobe.cached(&dress) {
        Some(id) => Some(id),
        None => wardrobe.register(&dress, &mut **catalog),
    }
}

/// What a hero spawns wearing: [`Dresser::display`].
#[derive(SystemParam)]
pub(in crate::survivors) struct Dresser<'w> {
    gear: GearRead<'w>,
    profile: Option<Res<'w, Profile>>,
    wardrobe: ResMut<'w, Wardrobe>,
    creatures: Option<ResMut<'w, Creatures>>,
    char_create: Option<Res<'w, CharCreate>>,
}

impl Dresser<'_> {
    /// The display a hero of class `class` in `look`, `(trainer display, race, sex)`, spawns
    /// with: its dressed body, else the trainer's own display.
    pub(in crate::survivors) fn display(&mut self, class: usize, look: (u32, u8, u8)) -> u32 {
        let Some(creatures) = self.creatures.as_mut() else {
            return look.0;
        };
        let cc = self.char_create.as_deref().map(|c| &c.0);
        dressed(
            &mut self.wardrobe,
            creatures,
            class,
            look,
            self.gear.get(class),
            cc,
            self.profile.as_deref().and_then(equipped_tabard),
        )
        .unwrap_or(look.0)
    }
}

/// Put display `id` on a standing hero: the descriptor's display field, which swaps a built body
/// in place (`refresh_live_display`), and the entity's, which an unbuilt one is built from. True
/// when it changed.
pub(super) fn wear(id: u32, net: &mut Mut<NetEntity>, store: &mut Mut<ObjectStore>) -> bool {
    if net.display_id == Some(id) {
        return false;
    }
    net.display_id = Some(id);
    store
        .0
        .merge(ObjectFields::from_pairs(&[(FIELD_DISPLAYID, id)]));
    true
}

/// Every hero wears the picked class's dress, re-planned only when the pick, the gear or the
/// looks may have changed, or a hero arrives; a dress it already wears changes nothing. Party
/// extras share the pick, so they share the dress.
#[allow(clippy::type_complexity)]
pub(super) fn dress_bodies<C: Catalog + Resource>(
    selection: Res<Selection>,
    gear: GearRead,
    profile: Option<Res<Profile>>,
    char_create: Option<Res<CharCreate>>,
    catalog: Option<ResMut<C>>,
    mut wardrobe: ResMut<Wardrobe>,
    mut heroes: Query<(&mut NetEntity, &mut ObjectStore), With<Hero>>,
    arrived: Query<(), Added<Hero>>,
) {
    let Some(mut catalog) = catalog else { return };
    if !selection.is_changed()
        && !gear.is_changed()
        && !profile.as_ref().is_some_and(|p| p.is_changed())
        && arrived.is_empty()
    {
        return;
    }
    let (Some(class), Some(&look)) = (
        selection.class,
        selection.look.and_then(|i| selection.looks.get(i)),
    ) else {
        return;
    };
    let cc = char_create.as_deref().map(|c| &c.0);
    let Some(id) = dressed(
        &mut wardrobe,
        &mut catalog,
        class,
        look,
        gear.get(class),
        cc,
        profile.as_deref().and_then(equipped_tabard),
    ) else {
        return;
    };
    for (mut net, mut store) in &mut heroes {
        wear(id, &mut net, &mut store);
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::profile::{slug, Profile};
    use super::super::{class_gear, piece, Rarity};
    use super::*;

    /// A trainer in a full outfit: every column its own id, 1001 for the head up to 1010.
    fn trainer() -> NpcAppearance {
        NpcAppearance {
            race: 1,
            sex: 0,
            skin: 2,
            face: 1,
            hair_style: 3,
            hair_color: 4,
            facial_hair: 0,
            equipment: std::array::from_fn(|i| 1001 + i as u32),
            bake_name: Some("HumanMaleTrainer.blp".into()),
        }
    }

    /// The catalog stand-in: one trainer row, and every registration counted.
    #[derive(Resource, Default)]
    struct Fake {
        registered: Vec<(u32, u32, NpcAppearance)>,
    }

    impl Catalog for Fake {
        fn npc_look(&self, display: u32) -> Option<NpcAppearance> {
            (display == 3000).then(trainer)
        }
        fn register(&mut self, id: u32, base: u32, look: NpcAppearance) -> bool {
            if base != 3000 {
                return false;
            }
            self.registered.push((id, base, look));
            true
        }
    }

    #[test]
    fn every_armour_slot_has_a_column_and_the_weapon_none() {
        let mut seen = Vec::new();
        for slot in Slot::ALL {
            match column(slot) {
                Some(c) => {
                    assert!(c < COLUMNS && c != SHIRT && c != TABARD, "{slot:?}");
                    assert!(!seen.contains(&c), "{slot:?} shares a column");
                    seen.push(c);
                }
                None => assert_eq!(slot, Slot::MainHand),
            }
        }
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn worn_gear_overrides_its_column_and_empty_slots_keep_the_starter() {
        let start = trainer().equipment;
        let mut gear = GearSave::default();
        assert_eq!(worn_columns(start, &gear), start, "nothing worn");
        // Class 4's head and boots, collected and worn.
        let head = piece(4, Slot::Head, Rarity::Rare).unwrap();
        let feet = piece(4, Slot::Feet, Rarity::Common).unwrap();
        gear.collect(head);
        gear.collect(feet);
        let worn = worn_columns(start, &gear);
        assert_eq!(worn[0], head.display_id);
        assert_eq!(worn[6], feet.display_id);
        for col in [1, 2, 3, 4, 5, 7, 8, 9] {
            assert_eq!(worn[col], start[col], "column {col} keeps the trainer's");
        }
        // A worn chest shows whole: no starter tabard or shirt over or under it.
        let chest = piece(4, Slot::Chest, Rarity::Epic).unwrap();
        gear.collect(chest);
        let worn = worn_columns(start, &gear);
        assert_eq!(worn[CHEST], chest.display_id);
        assert_eq!((worn[SHIRT], worn[TABARD]), (0, 0));
        // A main hand is a held weapon, never a column.
        gear.collect(piece(4, Slot::MainHand, Rarity::Epic).unwrap());
        assert_eq!(worn_columns(start, &gear), worn);
        // Every slot worn: all eight armour columns are the gear's.
        for slot in Slot::ALL {
            gear.collect(piece(4, slot, Rarity::Legendary).unwrap());
        }
        let worn = worn_columns([0; COLUMNS], &gear);
        for slot in Slot::ALL {
            if let Some(col) = column(slot) {
                assert_eq!(worn[col], gear.worn(slot).unwrap().display_id, "{slot:?}");
            }
        }
    }

    #[test]
    fn a_trainer_without_a_row_falls_back_to_the_start_outfit() {
        let outfit = [
            StartOutfitItem {
                display_id: 9891,
                inv_type: 4, // shirt
            },
            StartOutfitItem {
                display_id: 9892,
                inv_type: 7, // legs
            },
            StartOutfitItem {
                display_id: 10141,
                inv_type: 8, // feet
            },
            StartOutfitItem {
                display_id: 1542,
                inv_type: 21, // a main-hand sword: held, not worn
            },
            StartOutfitItem {
                display_id: 555,
                inv_type: 20, // a robe: the chest column
            },
            StartOutfitItem {
                display_id: 777,
                inv_type: 19, // tabard
            },
        ];
        let cols = starter(None, &outfit);
        assert_eq!(cols, [0, 0, 9891, 555, 0, 9892, 10141, 0, 0, 777]);
        assert_eq!(starter(Some(&trainer()), &outfit), trainer().equipment);
    }

    #[test]
    fn a_dress_is_registered_once_and_reused() {
        let mut fake = Fake::default();
        let mut wardrobe = Wardrobe::default();
        let gear = GearSave::default();
        let dress = plan(0, (3000, 1, 0), &gear, Some(&trainer()), None, None);
        assert_eq!(dress.equipment, trainer().equipment);
        assert_eq!(
            dress.features,
            Features::of(&trainer()),
            "the trainer's own"
        );
        let id = wardrobe.register(&dress, &mut fake).unwrap();
        assert_eq!(wardrobe.cached(&dress), Some(id));
        let (_, base, look) = &fake.registered[0];
        assert_eq!(*base, 3000);
        assert!(
            look.bake_name.is_none(),
            "composited, never the baked atlas"
        );
        assert_eq!(look.equipment, dress.equipment);
        // A trainer with no row registers nothing.
        let orphan = Dress { base: 1, ..dress };
        assert_eq!(wardrobe.register(&orphan, &mut fake), None);
        assert_eq!(wardrobe.ids.len(), 1);
    }

    /// From the client: every playable race and sex counts its features from the tables, with a
    /// caption for each, and a dress on its body display resolves its character model; every
    /// class's trainer looks give an outfit to start from and a dress that resolves.
    #[test]
    fn every_body_counts_its_features_and_resolves_from_the_data() {
        use super::super::looks::{feature_counts, feature_labels, has_facial};
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let mut creatures =
            benilla_formats::load_creature_catalog(&mut chain).expect("creature displays");
        let cc = CharCreateCatalog::load(&mut chain).expect("char-create tables");
        let strings = crate::glue_strings::table_from_chain(&mut chain);
        let mut id = FIRST_DRESS_ID;
        for race in 1..=8u8 {
            for sex in 0..2u8 {
                let counts = feature_counts(Some(&cc), race, sex);
                let r = cc.ranges(race, sex).expect("ranges");
                assert_eq!(
                    counts,
                    [r.skin, r.face, r.hair_style, r.hair_color, r.facial_hair],
                    "race {race} sex {sex}"
                );
                assert!(
                    counts[0] > 1 && counts[1] > 1 && counts[2] > 1 && counts[3] > 1,
                    "race {race} sex {sex}: {counts:?}"
                );
                let labels = feature_labels(Some(&strings), Some(&cc), race, sex);
                assert!(labels.iter().all(|l| !l.is_empty()), "{labels:?}");
                eprintln!(
                    "race {race} sex {sex}: {counts:?} {labels:?} facial {}",
                    has_facial(Some(&cc), race, sex)
                );
                let body = cc.body_display(race, sex).expect("body display");
                let model = creatures.model(body).expect("body model");
                assert!(
                    model
                        .model_path
                        .to_ascii_lowercase()
                        .starts_with("character\\"),
                    "race {race} sex {sex}: {}",
                    model.model_path
                );
                let dress = Dress {
                    base: body,
                    race,
                    sex,
                    features: Features::from_array(counts.map(|n| n - 1)),
                    equipment: [0; COLUMNS],
                };
                assert!(creatures.insert_dressed(id, body, dress.appearance()));
                let worn = creatures.model(id).expect("the dress resolves");
                assert_eq!(worn.model_path, model.model_path);
                let npc = worn.npc_appearance.expect("with its appearance");
                assert_eq!((npc.race, npc.sex, npc.skin), (race, sex, counts[0] - 1));
                assert!(npc.bake_name.is_none());
                id += 1;
            }
        }
        for (c, class) in CLASSES.iter().enumerate() {
            let mut dressed = 0;
            for &trainer in class.looks {
                let Some((race, sex)) = creatures.display_race_sex(trainer) else {
                    continue;
                };
                let npc = creatures.model(trainer).and_then(|m| m.npc_appearance);
                let dress = plan(
                    c,
                    (trainer, race, sex),
                    &GearSave::default(),
                    npc.as_ref(),
                    Some(&cc),
                    None,
                );
                assert!(
                    dress.equipment.iter().any(|&d| d != 0),
                    "{} trainer {trainer} starts naked",
                    class.name
                );
                assert!(creatures.insert_dressed(id, trainer, dress.appearance()));
                let m = creatures.model(id).expect("the trainer dress resolves");
                assert!(m.model_path.to_ascii_lowercase().starts_with("character\\"));
                id += 1;
                dressed += 1;
            }
            assert!(dressed > 0, "{} has no trainer look", class.name);
        }
    }

    fn hero_display(world: &World, hero: Entity) -> Option<u32> {
        world.entity(hero).get::<NetEntity>().unwrap().display_id
    }

    fn edit_gear(world: &mut World, f: impl FnOnce(&mut GearSave)) {
        let mut profile = world.resource_mut::<Profile>();
        f(&mut profile
            .classes
            .entry(slug(CLASSES[4].name))
            .or_default()
            .gear);
    }

    /// Through the system: the dress is planned and registered when the gear or looks change and
    /// a hero arrives; nothing else registers or re-dresses, and a dress worn before is reused.
    #[test]
    fn the_dress_is_rebuilt_only_when_gear_or_looks_change() {
        let mut world = World::new();
        world.init_resource::<Fake>();
        world.init_resource::<Wardrobe>();
        world.insert_resource(Profile::default());
        world.insert_resource(Selection {
            class: Some(4),
            looks: vec![(3000, 1, 0)],
            look: Some(0),
            ..default()
        });
        let hero = world
            .spawn((
                Hero::default(),
                NetEntity {
                    kind: benilla_protocol::EntityKind::Unit,
                    display_id: Some(3000),
                    scale: 1.0,
                },
                ObjectStore(ObjectFields::from_pairs(&[])),
            ))
            .id();
        // One system across runs, so its change ticks carry over.
        let mut schedule = Schedule::default();
        schedule.add_systems(dress_bodies::<Fake>);
        let registered = |world: &World| world.resource::<Fake>().registered.len();

        schedule.run(&mut world);
        assert_eq!(registered(&world), 1);
        let first = hero_display(&world, hero).unwrap();
        assert_eq!(first, FIRST_DRESS_ID);
        let store = &world.entity(hero).get::<ObjectStore>().unwrap().0;
        assert_eq!(store.unit_displayid(), Some(first as i32));
        let look = &world.resource::<Fake>().registered[0].2;
        assert_eq!(
            (look.skin, look.face, look.hair_style, look.hair_color),
            (2, 1, 3, 4),
            "the trainer's features"
        );

        // Nothing changed, or only a profile part that is neither gear nor looks: no new row.
        schedule.run(&mut world);
        world.resource_mut::<Profile>().account.runs += 1;
        schedule.run(&mut world);
        assert_eq!(registered(&world), 1);
        assert_eq!(hero_display(&world, hero), Some(first));

        // New looks: a new dress.
        let mine = Features::from_array([5, 5, 5, 5, 5]);
        edit_gear(&mut world, |g| g.looks.set_features(1, 0, mine));
        schedule.run(&mut world);
        assert_eq!(registered(&world), 2);
        let second = hero_display(&world, hero).unwrap();
        assert_ne!(second, first);
        assert_eq!(world.resource::<Fake>().registered[1].2.skin, 5);

        // New gear: a worn head is another.
        let head = piece(4, Slot::Head, Rarity::Epic).unwrap();
        edit_gear(&mut world, |g| {
            g.collect(head);
        });
        assert!(class_gear(&world, 4).wears(head.id));
        schedule.run(&mut world);
        assert_eq!(registered(&world), 3);
        assert_eq!(
            world.resource::<Fake>().registered[2].2.equipment[0],
            head.display_id
        );
        assert_ne!(hero_display(&world, hero), Some(second));

        // Taking it off, then the looks, wears each earlier dress again from the wardrobe.
        edit_gear(&mut world, |g| g.equipped.clear());
        schedule.run(&mut world);
        assert_eq!(hero_display(&world, hero), Some(second));
        edit_gear(&mut world, |g| g.looks.features.clear());
        schedule.run(&mut world);
        assert_eq!(hero_display(&world, hero), Some(first));
        assert_eq!(registered(&world), 3);

        // A hero arriving in the same dress is dressed without a new row.
        let extra = world
            .spawn((
                Hero::default(),
                NetEntity {
                    kind: benilla_protocol::EntityKind::Unit,
                    display_id: Some(3000),
                    scale: 1.0,
                },
                ObjectStore(ObjectFields::from_pairs(&[])),
            ))
            .id();
        schedule.run(&mut world);
        assert_eq!(hero_display(&world, extra), Some(first));
        assert_eq!(registered(&world), 3);
    }
}
