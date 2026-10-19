//! The account's Jewelry Collection: every piece of Rare-or-better jewelry the account has ever
//! got, for every class, and the loadout it wears into each run, in WoW's own slots: Neck,
//! Finger 1, Finger 2, Trinket 1 and Trinket 2 ([`LoadoutSlot`]).
//!
//! A piece joins the collection the first time it is got: a Rare-or-better piece taken in a run
//! (the local hero wearing it, or a co-op client's pick, [`PickedJewels`]), a completed
//! Darkmoon deck's trinket, or Molten Core's Bound jewelry ([`collect_new`]); each new one raises
//! a "New collection item" toast ([`CollectionToasts`]). Greens are too common to collect.
//!
//! Between runs the Armory's Jewelry tab picks what the loadout wears, one of each piece
//! (unique-equipped) and each in a slot of its kind; [`super::equip_loadout`] puts it on as
//! play begins. A co-op client sends its own in its Hello, and the host holds it to the same
//! rules ([`validate_loadout`]).
//!
//! Saved in the profile's `collection` section ([`JewelryCollection`]), which a save from before
//! it reads as empty. Such a save is migrated once on load ([`migrate`]): its Bound jewelry, its
//! completed decks' trinkets, the Rare-or-better pieces its Codex says were taken in past runs and
//! its old pinned Darkmoon trinket are collected, quietly (no toasts); the pinned trinket goes
//! into Trinket 1, and the Bound jewelry fills the slots left, rarest first.

use std::collections::{BTreeSet, VecDeque};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::auto::Auto;
use super::super::hud::HudFont;
use super::super::profile::Profile;
use super::super::{HeroState, LocalHero};
use super::{catalog, darkmoon_owned, piece, records, Kind, Piece, Rarity, SLOTS};
use crate::glue::art::GOLD;

/// One of the loadout's slots, as the character sheet names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::survivors) enum LoadoutSlot {
    Neck,
    Finger1,
    Finger2,
    Trinket1,
    Trinket2,
}

impl LoadoutSlot {
    /// In the character sheet's order.
    pub(in crate::survivors) const ALL: [Self; 5] = [
        Self::Neck,
        Self::Finger1,
        Self::Finger2,
        Self::Trinket1,
        Self::Trinket2,
    ];

    pub(in crate::survivors) fn label(self) -> &'static str {
        match self {
            Self::Neck => "Neck",
            Self::Finger1 => "Finger 1",
            Self::Finger2 => "Finger 2",
            Self::Trinket1 => "Trinket 1",
            Self::Trinket2 => "Trinket 2",
        }
    }

    /// What the slot holds.
    pub(in crate::survivors) fn kind(self) -> Kind {
        match self {
            Self::Neck => Kind::Necklace,
            Self::Finger1 | Self::Finger2 => Kind::Ring,
            Self::Trinket1 | Self::Trinket2 => Kind::Trinket,
        }
    }

    /// The run's own slot it fills, as an index into [`SLOTS`].
    pub(in crate::survivors) fn index(self) -> usize {
        match self {
            Self::Finger1 => 0,
            Self::Finger2 => 1,
            Self::Trinket1 => 2,
            Self::Trinket2 => 3,
            Self::Neck => 4,
        }
    }

    /// The slots that take `kind`, first first.
    pub(in crate::survivors) fn of_kind(kind: Kind) -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(move |s| s.kind() == kind)
    }
}

/// The loadout as saved: each slot's piece by its key, `None` for an empty slot. A key this build
/// does not know is kept, and worn as nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct Loadout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neck: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finger1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finger2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trinket1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trinket2: Option<String>,
}

impl Loadout {
    pub(in crate::survivors) fn get(&self, slot: LoadoutSlot) -> Option<&str> {
        self.field(slot).as_deref()
    }

    fn field(&self, slot: LoadoutSlot) -> &Option<String> {
        match slot {
            LoadoutSlot::Neck => &self.neck,
            LoadoutSlot::Finger1 => &self.finger1,
            LoadoutSlot::Finger2 => &self.finger2,
            LoadoutSlot::Trinket1 => &self.trinket1,
            LoadoutSlot::Trinket2 => &self.trinket2,
        }
    }

    fn field_mut(&mut self, slot: LoadoutSlot) -> &mut Option<String> {
        match slot {
            LoadoutSlot::Neck => &mut self.neck,
            LoadoutSlot::Finger1 => &mut self.finger1,
            LoadoutSlot::Finger2 => &mut self.finger2,
            LoadoutSlot::Trinket1 => &mut self.trinket1,
            LoadoutSlot::Trinket2 => &mut self.trinket2,
        }
    }

    /// The slot wearing `key`, if one does.
    pub(in crate::survivors) fn slot_of(&self, key: &str) -> Option<LoadoutSlot> {
        LoadoutSlot::ALL
            .into_iter()
            .find(|s| self.get(*s) == Some(key))
    }
}

/// Why a piece could not go into the loadout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Unequippable {
    /// Not in the collection (or no collectable piece at all).
    Locked,
    /// The slot is not of its kind.
    WrongSlot,
    /// Unique-Equipped: another slot wears it.
    Unique,
}

/// The account's collection, `Profile::collection`. Every field must read a save without it as
/// its default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct JewelryCollection {
    /// Every piece collected, by [`Piece::key`].
    pub unlocked: BTreeSet<String>,
    /// What is worn into each run.
    pub loadout: Loadout,
    /// The one-time [`migrate`] has run on this save.
    pub migrated: bool,
}

impl JewelryCollection {
    /// A new account's: nothing collected, and nothing from before the collection to migrate.
    pub(in crate::survivors) fn new_account() -> Self {
        Self {
            migrated: true,
            ..Self::default()
        }
    }

    pub(in crate::survivors) fn has(&self, key: &str) -> bool {
        self.unlocked.contains(key)
    }

    /// Collect `piece`, when it is collectable: whether it is new.
    pub(in crate::survivors) fn unlock(&mut self, piece: &Piece) -> bool {
        collectable(piece) && self.unlocked.insert(piece.key.to_string())
    }

    /// Wear collected piece `key` in `slot`, over whatever it holds.
    pub(in crate::survivors) fn equip(
        &mut self,
        slot: LoadoutSlot,
        key: &str,
    ) -> Result<(), Unequippable> {
        let piece = piece(key)
            .filter(|p| collectable(p) && self.has(p.key))
            .ok_or(Unequippable::Locked)?;
        if piece.kind != slot.kind() {
            return Err(Unequippable::WrongSlot);
        }
        match self.loadout.slot_of(piece.key) {
            Some(s) if s == slot => Ok(()),
            Some(_) => Err(Unequippable::Unique),
            None => {
                *self.loadout.field_mut(slot) = Some(piece.key.to_string());
                Ok(())
            }
        }
    }

    /// Empty `slot`.
    pub(in crate::survivors) fn unequip(&mut self, slot: LoadoutSlot) {
        *self.loadout.field_mut(slot) = None;
    }

    /// A click on collected piece `key`: off when it is worn, else on into the first empty slot of
    /// its kind, or over the first of its kind when all are full. The slot it went into, if on.
    pub(in crate::survivors) fn toggle(
        &mut self,
        key: &str,
    ) -> Result<Option<LoadoutSlot>, Unequippable> {
        if let Some(slot) = self.loadout.slot_of(key) {
            self.unequip(slot);
            return Ok(None);
        }
        let kind = piece(key).ok_or(Unequippable::Locked)?.kind;
        let slot = LoadoutSlot::of_kind(kind)
            .find(|s| self.loadout.get(*s).is_none())
            .or_else(|| LoadoutSlot::of_kind(kind).next())
            .ok_or(Unequippable::WrongSlot)?;
        self.equip(slot, key).map(|()| Some(slot))
    }

    /// The loadout by key, in the run's slot order ([`SLOTS`]): what a co-op client's Hello
    /// carries.
    pub(in crate::survivors) fn wire(&self) -> [Option<String>; SLOTS.len()] {
        let mut out: [Option<String>; SLOTS.len()] = Default::default();
        for slot in LoadoutSlot::ALL {
            out[slot.index()] = self.loadout.get(slot).map(String::from);
        }
        out
    }

    /// What the loadout wears, in the run's slot order: each piece collected, in a slot of its
    /// kind, and none twice.
    pub(in crate::survivors) fn worn(&self) -> [Option<&'static Piece>; SLOTS.len()] {
        let mut out = validate_loadout(&self.wire());
        for p in &mut out {
            if p.is_some_and(|p| !self.has(p.key)) {
                *p = None;
            }
        }
        out
    }
}

/// Whether `piece` can be collected: Rare or better. The greens are a run's simple stat finds.
pub(in crate::survivors) fn collectable(piece: &Piece) -> bool {
    piece.rarity as u8 >= Rarity::Rare as u8
}

/// Every collectable piece, rings then trinkets then necklaces, Rare first, then the Darkmoon
/// trinkets.
pub(in crate::survivors) fn collection_pieces() -> impl Iterator<Item = &'static Piece> {
    catalog::CATALOG
        .iter()
        .chain(catalog::DARKMOON)
        .filter(|p| collectable(p))
}

/// A loadout by key in the run's slot order, as it may be worn: only collectable pieces, each in
/// a slot of its kind, and the first of any piece named twice. The host holds a co-op client's
/// Hello to this; whether the account collected each is that player's own profile's to say.
pub(in crate::survivors) fn validate_loadout(
    keys: &[Option<String>; SLOTS.len()],
) -> [Option<&'static Piece>; SLOTS.len()] {
    let mut out: [Option<&'static Piece>; SLOTS.len()] = [None; SLOTS.len()];
    for (i, key) in keys.iter().enumerate() {
        let Some(p) = key.as_deref().and_then(piece) else {
            continue;
        };
        let taken = out.iter().flatten().any(|q| q.key == p.key);
        if collectable(p) && SLOTS[i] == p.kind && !taken {
            out[i] = Some(p);
        }
    }
    out
}

/// Where a piece is got, for the Armory's locked pieces.
pub(in crate::survivors) fn source(piece: &Piece) -> String {
    if let Some(suit) = super::darkmoon_suit(piece.key) {
        let mut deck = suit.to_string();
        if let Some(first) = deck.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        return format!("Darkmoon Faire: complete the {deck} deck");
    }
    let bosses = super::super::gear::raid::bound_sources(piece.key);
    if !bosses.is_empty() {
        return format!("Drops from {}", bosses.join(" and "));
    }
    "Found in runs: caches and quest turn-ins".to_string()
}

/// The pieces the profile has got outside a run: its Bound raid jewelry and its completed decks'
/// trinkets.
fn owned_outside_runs(profile: &Profile) -> Vec<&'static Piece> {
    let bound = profile.raid.bound_keys().into_iter().filter_map(piece);
    bound.chain(darkmoon_owned(profile)).collect()
}

/// A save from before the collection, once: its Bound jewelry, completed decks' trinkets, the
/// pieces its Codex logged as taken in past runs (the Rare-or-better ones) and its old pinned
/// Darkmoon trinket are collected, with no toast; the pinned trinket goes into Trinket 1 and the
/// Bound jewelry fills the empty slots, rarest first. The pinned field is cleared, as the loadout
/// holds it now.
pub(in crate::survivors) fn migrate(profile: &mut Profile) {
    if profile.collection.migrated {
        return;
    }
    let pinned = profile
        .faire
        .wardrobe
        .pinned_trinket
        .take()
        .and_then(|k| catalog::DARKMOON.iter().find(|p| p.key == k));
    let mut owned = owned_outside_runs(profile);
    // Taken in a past run: a key this build does not know is skipped, a green is not collectable.
    owned.extend(
        profile
            .codex
            .taken
            .iter()
            .filter(|(_, n)| **n > 0)
            .filter_map(|(k, _)| piece(k)),
    );
    let mut bound: Vec<&'static Piece> = profile
        .raid
        .bound_keys()
        .into_iter()
        .filter_map(piece)
        .collect();
    bound.sort_by_key(|p| std::cmp::Reverse(p.rarity as u8));
    let c = &mut profile.collection;
    for p in pinned.iter().copied().chain(owned) {
        c.unlock(p);
    }
    if let Some(p) = pinned {
        if c.loadout.get(LoadoutSlot::Trinket1).is_none() {
            let _ = c.equip(LoadoutSlot::Trinket1, p.key);
        }
    }
    for p in bound {
        if c.loadout.slot_of(p.key).is_some() {
            continue;
        }
        if let Some(slot) = LoadoutSlot::of_kind(p.kind).find(|s| c.loadout.get(*s).is_none()) {
            let _ = c.equip(slot, p.key);
        }
    }
    c.migrated = true;
}

/// A co-op client's jewelry picks, which its own hero does not mirror: each is collected as the
/// local hero's worn pieces are ([`collect_new`]).
#[derive(Resource, Default, Debug)]
pub(in crate::survivors) struct PickedJewels(pub Vec<&'static Piece>);

/// New pieces collected, waiting for the toast, and the one on it with its seconds left.
#[derive(Resource, Default, Debug)]
pub(in crate::survivors) struct CollectionToasts {
    queue: VecDeque<&'static Piece>,
    shown: Option<(&'static Piece, f32)>,
}

impl CollectionToasts {
    pub(in crate::survivors) fn push(&mut self, piece: &'static Piece) {
        self.queue.push_back(piece);
    }
}

/// The toast's two lines: "New collection item:" over "[Hand of Justice]".
pub(in crate::survivors) fn toast_lines(piece: &Piece) -> (&'static str, String) {
    ("New collection item:", format!("[{}]", piece.name))
}

/// Collect whatever the account has newly got: the pieces the local hero wears and a co-op
/// client's picks (in a run the profile records), and the Bound jewelry and completed decks'
/// trinkets the profile holds. Each new piece is toasted.
pub(super) fn collect_new(
    profile: Option<ResMut<Profile>>,
    heroes: Query<&HeroState, With<LocalHero>>,
    mut picked: ResMut<PickedJewels>,
    auto: Option<Res<Auto>>,
    mut toasts: ResMut<CollectionToasts>,
) {
    let found_picks = std::mem::take(&mut picked.0);
    let Some(mut profile) = profile else { return };
    let mut found: Vec<&'static Piece> = Vec::new();
    if records(auto.as_deref()) {
        found.extend(heroes.iter().flat_map(|h| h.jewelry.worn()));
        found.extend(found_picks);
    }
    if profile.is_changed() {
        found.extend(owned_outside_runs(&profile));
    }
    // Read first, so a profile that holds them all already is not marked changed.
    if found
        .iter()
        .all(|p| !collectable(p) || profile.collection.has(p.key))
    {
        return;
    }
    for p in found {
        if profile.collection.unlock(p) {
            toasts.push(p);
        }
    }
    profile.mark_dirty();
}

/// How long each toast shows, in real seconds (the jewelry picker pauses the run's clock).
const TOAST_SECS: f32 = 4.0;

#[derive(Component)]
pub(super) struct ToastRoot;
#[derive(Component)]
pub(super) struct ToastBox;

/// The toast's frame, built once the HUD's font is in; hidden while nothing is new.
pub(super) fn spawn_toast(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    built: Query<(), With<ToastRoot>>,
) {
    if !built.is_empty() || font.is_none() {
        return;
    }
    commands
        .spawn((
            ToastRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Percent(22.0),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            // Over the jewelry picker and the Armory, under their tooltips' cursor boxes.
            GlobalZIndex(990),
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|r| {
            r.spawn((
                ToastBox,
                Node {
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::all(GOLD),
                BackgroundColor(Color::srgba(0.04, 0.03, 0.02, 0.92)),
            ));
        });
}

/// The next new piece takes the toast once the one before has had its time.
pub(super) fn show_toast(
    mut commands: Commands,
    time: Res<Time<Real>>,
    font: Option<Res<HudFont>>,
    mut toasts: ResMut<CollectionToasts>,
    mut roots: Query<&mut Node, With<ToastRoot>>,
    mut boxes: Query<(Entity, &mut BorderColor), With<ToastBox>>,
) {
    let (Ok(mut root), Ok((frame, mut edge)), Some(font)) =
        (roots.single_mut(), boxes.single_mut(), font)
    else {
        return;
    };
    if let Some((_, left)) = toasts.shown.as_mut() {
        *left -= time.delta_secs();
        if *left > 0.0 {
            return;
        }
        toasts.shown = None;
    }
    let Some(piece) = toasts.queue.pop_front() else {
        if root.display != Display::None {
            root.display = Display::None;
        }
        return;
    };
    toasts.shown = Some((piece, TOAST_SECS));
    root.display = Display::Flex;
    *edge = BorderColor::all(piece.rarity.color());
    let face = |s: String, size: f32, color: Color| {
        (
            Text::new(s),
            TextFont {
                font: font.0.clone(),
                font_size: size,
                ..default()
            },
            TextColor(color),
            TextShadow {
                offset: Vec2::splat(1.0),
                color: Color::BLACK,
            },
        )
    };
    commands.entity(frame).despawn_related::<Children>();
    commands.entity(frame).with_children(|b| {
        b.spawn((
            Node {
                width: Val::Px(36.0),
                height: Val::Px(36.0),
                flex_shrink: 0.0,
                ..default()
            },
            ImageNode {
                color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                ..default()
            },
            super::super::gear::ItemIcon {
                icon: piece.icon,
                tint: Color::WHITE,
            },
        ));
        b.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|t| {
            let (head, name) = toast_lines(piece);
            t.spawn(face(head.into(), 13.0, GOLD));
            t.spawn(face(name, 16.0, piece.rarity.color()));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::super::super::profile::Profile;
    use super::super::{wear_loadout, Jewelry};
    use super::*;
    use crate::survivors::bonus::Bonuses;
    use crate::survivors::data;

    fn by_key(key: &str) -> &'static Piece {
        piece(key).unwrap_or_else(|| panic!("no piece {key}"))
    }

    fn collected(keys: &[&str]) -> JewelryCollection {
        let mut c = JewelryCollection::default();
        for k in keys {
            assert!(c.unlock(by_key(k)), "{k}");
        }
        c
    }

    #[test]
    fn the_slots_are_wows_and_each_is_a_run_slot_of_its_kind() {
        let labels: Vec<_> = LoadoutSlot::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(
            labels,
            ["Neck", "Finger 1", "Finger 2", "Trinket 1", "Trinket 2"]
        );
        let mut seen = BTreeSet::new();
        for slot in LoadoutSlot::ALL {
            assert_eq!(SLOTS[slot.index()], slot.kind(), "{slot:?}");
            assert!(seen.insert(slot.index()), "{slot:?} shares a run slot");
        }
        assert_eq!(
            LoadoutSlot::of_kind(Kind::Ring).collect::<Vec<_>>(),
            [LoadoutSlot::Finger1, LoadoutSlot::Finger2]
        );
        assert_eq!(
            LoadoutSlot::of_kind(Kind::Trinket).collect::<Vec<_>>(),
            [LoadoutSlot::Trinket1, LoadoutSlot::Trinket2]
        );
        assert_eq!(
            LoadoutSlot::of_kind(Kind::Necklace).collect::<Vec<_>>(),
            [LoadoutSlot::Neck]
        );
        // Finger 1 and Trinket 1 are the run's first slot of their kind, as a find fills them.
        let first = |k| Jewelry::slots_of(k).next();
        assert_eq!(first(Kind::Ring), Some(LoadoutSlot::Finger1.index()));
        assert_eq!(first(Kind::Trinket), Some(LoadoutSlot::Trinket1.index()));
        assert_eq!(first(Kind::Necklace), Some(LoadoutSlot::Neck.index()));
    }

    #[test]
    fn only_rare_or_better_is_collected() {
        let mut c = JewelryCollection::default();
        assert!(!c.unlock(by_key("the_1_ring")), "an Uncommon");
        assert!(!c.unlock(by_key("carrot_on_a_stick")), "an Uncommon");
        assert!(c.unlock(by_key("hand_of_justice")), "a Rare");
        assert!(!c.unlock(by_key("hand_of_justice")), "collected once");
        assert!(c.unlock(by_key("band_of_accuria")), "Bound jewelry");
        assert!(c.unlock(by_key("darkmoon_maelstrom")), "a Darkmoon trinket");
        assert_eq!(c.unlocked.len(), 3);
        // The collection is every Rare-or-better piece and the four Darkmoon trinkets.
        let all: Vec<_> = collection_pieces().collect();
        assert_eq!(all.len(), 33 + 4);
        assert!(all.iter().all(|p| collectable(p)));
        assert!(all.iter().any(|p| p.key == "darkmoon_heroism"));
        assert!(all.iter().all(|p| p.rarity != Rarity::Uncommon));
    }

    #[test]
    fn unique_equipped_and_each_in_a_slot_of_its_kind() {
        let mut c = collected(&[
            "quick_strike_ring",
            "band_of_accuria",
            "naglering",
            "hand_of_justice",
            "onyxia_tooth_pendant",
        ]);
        // Locked, or no such piece.
        assert_eq!(
            c.equip(LoadoutSlot::Finger1, "seal_of_the_archmagus"),
            Err(Unequippable::Locked)
        );
        assert_eq!(
            c.equip(LoadoutSlot::Finger1, "the_1_ring"),
            Err(Unequippable::Locked)
        );
        assert_eq!(
            c.equip(LoadoutSlot::Finger1, "ashbringer"),
            Err(Unequippable::Locked)
        );
        // A ring is no trinket.
        assert_eq!(
            c.equip(LoadoutSlot::Trinket1, "band_of_accuria"),
            Err(Unequippable::WrongSlot)
        );
        assert_eq!(c.equip(LoadoutSlot::Finger1, "band_of_accuria"), Ok(()));
        assert_eq!(c.equip(LoadoutSlot::Finger1, "band_of_accuria"), Ok(()));
        // Unique-Equipped: never in both fingers.
        assert_eq!(
            c.equip(LoadoutSlot::Finger2, "band_of_accuria"),
            Err(Unequippable::Unique)
        );
        assert_eq!(c.loadout.get(LoadoutSlot::Finger2), None);
        // A click fills the first empty slot of the kind, then replaces the first; a worn piece
        // comes off.
        assert_eq!(
            c.toggle("quick_strike_ring"),
            Ok(Some(LoadoutSlot::Finger2))
        );
        assert_eq!(c.toggle("naglering"), Ok(Some(LoadoutSlot::Finger1)));
        assert_eq!(c.loadout.get(LoadoutSlot::Finger1), Some("naglering"));
        assert_eq!(c.toggle("naglering"), Ok(None));
        assert_eq!(c.loadout.get(LoadoutSlot::Finger1), None);
        assert_eq!(c.toggle("hand_of_justice"), Ok(Some(LoadoutSlot::Trinket1)));
        assert_eq!(
            c.toggle("onyxia_tooth_pendant"),
            Ok(Some(LoadoutSlot::Neck))
        );
        assert_eq!(c.toggle("seal_of_the_archmagus"), Err(Unequippable::Locked));
        let worn: Vec<_> = c.worn().iter().map(|p| p.map(|p| p.key)).collect();
        assert_eq!(
            worn,
            [
                None,
                Some("quick_strike_ring"),
                Some("hand_of_justice"),
                None,
                Some("onyxia_tooth_pendant")
            ]
        );
        c.unequip(LoadoutSlot::Neck);
        assert_eq!(c.loadout.get(LoadoutSlot::Neck), None);
    }

    #[test]
    fn a_loadout_is_held_to_legal_slots_and_one_of_each() {
        let keys = |k: [Option<&str>; 5]| k.map(|k| k.map(String::from));
        let out = validate_loadout(&keys([
            Some("band_of_accuria"),
            // The same ring twice: the first stands.
            Some("band_of_accuria"),
            // A ring in a trinket slot.
            Some("quick_strike_ring"),
            Some("darkmoon_heroism"),
            // An Uncommon necklace: not collectable, so never worn.
            Some("medal_of_courage"),
        ]));
        let got: Vec<_> = out.iter().map(|p| p.map(|p| p.key)).collect();
        assert_eq!(
            got,
            [
                Some("band_of_accuria"),
                None,
                None,
                Some("darkmoon_heroism"),
                None
            ]
        );
        let out = validate_loadout(&keys([
            Some("no_such_ring"),
            None,
            Some("darkmoon_maelstrom"),
            Some("darkmoon_maelstrom"),
            Some("choker_of_the_fire_lord"),
        ]));
        let got: Vec<_> = out.iter().map(|p| p.map(|p| p.key)).collect();
        assert_eq!(
            got,
            [
                None,
                None,
                Some("darkmoon_maelstrom"),
                None,
                Some("choker_of_the_fire_lord")
            ]
        );
        // What is worn of a saved loadout is collected too: a hand-edited key is not.
        let mut c = collected(&["hand_of_justice"]);
        c.loadout.trinket1 = Some("hand_of_justice".into());
        c.loadout.trinket2 = Some("kiss_of_the_spider".into());
        assert_eq!(c.worn()[2].map(|p| p.key), Some("hand_of_justice"));
        assert_eq!(c.worn()[3], None);
    }

    #[test]
    fn the_loadout_goes_on_in_its_slots() {
        let mut c = collected(&[
            "band_of_accuria",
            "darkmoon_blue_dragon",
            "kiss_of_the_spider",
            "gem_of_trapped_innocents",
        ]);
        c.equip(LoadoutSlot::Finger2, "band_of_accuria").unwrap();
        c.equip(LoadoutSlot::Trinket1, "darkmoon_blue_dragon")
            .unwrap();
        c.equip(LoadoutSlot::Trinket2, "kiss_of_the_spider")
            .unwrap();
        c.equip(LoadoutSlot::Neck, "gem_of_trapped_innocents")
            .unwrap();
        let mut hero = HeroState::new(0, &data::CLASSES[0], 1, Bonuses::default());
        assert_eq!(wear_loadout(&mut hero, &c), 4);
        assert_eq!(hero.jewelry.slot(0), None);
        assert_eq!(hero.jewelry.slot(1).map(|p| p.key), Some("band_of_accuria"));
        assert_eq!(
            hero.jewelry.slot(2).map(|p| p.key),
            Some("darkmoon_blue_dragon")
        );
        assert_eq!(
            hero.jewelry.slot(3).map(|p| p.key),
            Some("kiss_of_the_spider")
        );
        assert_eq!(
            hero.jewelry.slot(4).map(|p| p.key),
            Some("gem_of_trapped_innocents")
        );
        // Only what is worn grants: the collection itself gives nothing.
        let mut want = Bonuses::default();
        for p in hero.jewelry.worn() {
            want.add_all(p.bonuses);
        }
        assert_eq!(hero.jewelry.bonuses(), want);
    }

    #[test]
    fn the_collection_saves_as_its_own_section_and_old_saves_read_empty() {
        let mut c = collected(&["hand_of_justice", "darkmoon_maelstrom"]);
        c.equip(LoadoutSlot::Trinket2, "darkmoon_maelstrom")
            .unwrap();
        c.migrated = true;
        let mut profile = Profile::default();
        profile.collection = c.clone();
        let text = toml::to_string_pretty(&profile).unwrap();
        assert!(text.contains("[collection]"), "{text}");
        assert!(text.contains("[collection.loadout]"), "{text}");
        let back: Profile = toml::from_str(&text).unwrap();
        assert_eq!(back.collection, c);
        assert_eq!(
            toml::from_str::<JewelryCollection>("").unwrap(),
            JewelryCollection::default()
        );
        // A new account has nothing to migrate; a save from before the section does.
        assert!(Profile::default().collection.migrated);
        let old: Profile = toml::from_str("version = 5\n\n[account]\nruns = 2\n").unwrap();
        assert!(!old.collection.migrated);
    }

    #[test]
    fn an_old_save_migrates_its_bound_jewelry_and_pinned_trinket() {
        let text = "version = 5\n\n[faire]\ncards = [\"ace_of_portals\", \"two_of_portals\", \
                    \"three_of_portals\", \"four_of_portals\", \"five_of_portals\", \
                    \"six_of_portals\", \"seven_of_portals\", \"eight_of_portals\"]\n\n\
                    [faire.wardrobe]\npinned_trinket = \"darkmoon_heroism\"\n\n\
                    [raid]\nbound = [2130, 2131, 2132, 2133, 2135, 2136]\n\n\
                    [codex]\nseen = [\"kiss_of_the_spider\"]\n\n\
                    [codex.taken]\nhand_of_justice = 2\nearthstrike = 1\nthe_1_ring = 3\n\
                    a_piece_from_a_newer_build = 1\n";
        let mut profile: Profile = toml::from_str(text).unwrap();
        migrate(&mut profile);
        let c = &profile.collection;
        assert!(c.migrated);
        assert_eq!(profile.faire.wardrobe.pinned_trinket, None, "folded in");
        for key in [
            "darkmoon_heroism",
            // The completed Portals deck's.
            "darkmoon_twisting_nether",
            "choker_of_enlightenment",
            "quick_strike_ring",
            "talisman_of_ephemeral_power",
            "ring_of_spell_power",
            "band_of_accuria",
            "choker_of_the_fire_lord",
            // Taken in past runs, as the Codex logged them.
            "hand_of_justice",
            "earthstrike",
        ] {
            assert!(c.has(key), "{key}");
        }
        // Not a green taken, a piece only seen, nor a key this build does not know.
        assert!(!c.has("the_1_ring") && !c.has("kiss_of_the_spider"));
        assert!(!c.has("a_piece_from_a_newer_build"));
        assert_eq!(c.unlocked.len(), 10);
        // The backfill is quiet: what a migrated save holds raises no toast.
        {
            use bevy::ecs::system::RunSystemOnce;
            let mut world = World::new();
            world.insert_resource(profile.clone());
            world.init_resource::<PickedJewels>();
            world.init_resource::<CollectionToasts>();
            world.run_system_once(collect_new).unwrap();
            assert!(world.resource::<CollectionToasts>().queue.is_empty());
        }
        let c = &profile.collection;
        // The pinned trinket in Trinket 1; the Bound jewelry in what is left, rarest first:
        // Band of Accuria takes Finger 1, and the first Epic of each kind the rest.
        assert_eq!(
            c.loadout.get(LoadoutSlot::Trinket1),
            Some("darkmoon_heroism")
        );
        assert_eq!(
            c.loadout.get(LoadoutSlot::Trinket2),
            Some("talisman_of_ephemeral_power")
        );
        assert_eq!(c.loadout.get(LoadoutSlot::Finger1), Some("band_of_accuria"));
        assert_eq!(
            c.loadout.get(LoadoutSlot::Finger2),
            Some("quick_strike_ring")
        );
        assert_eq!(
            c.loadout.get(LoadoutSlot::Neck),
            Some("choker_of_enlightenment")
        );
        // Once only: a slot emptied since stays empty.
        let before = profile.collection.clone();
        profile.collection.unequip(LoadoutSlot::Neck);
        migrate(&mut profile);
        assert_eq!(profile.collection.loadout.get(LoadoutSlot::Neck), None);
        assert_eq!(profile.collection.unlocked, before.unlocked);
        // A save with nothing to fold in migrates to an empty loadout.
        let mut bare: Profile = toml::from_str("version = 5\n").unwrap();
        migrate(&mut bare);
        assert!(bare.collection.migrated && bare.collection.unlocked.is_empty());
        assert_eq!(bare.collection.loadout, Loadout::default());
    }

    #[test]
    fn locked_pieces_say_where_they_drop() {
        assert_eq!(source(by_key("band_of_accuria")), "Drops from Ragnaros");
        assert_eq!(
            source(by_key("darkmoon_blue_dragon")),
            "Darkmoon Faire: complete the Beasts deck"
        );
        assert_eq!(
            source(by_key("hand_of_justice")),
            "Found in runs: caches and quest turn-ins"
        );
        let (head, name) = toast_lines(by_key("hand_of_justice"));
        assert_eq!(
            format!("{head} {name}"),
            "New collection item: [Hand of Justice]"
        );
    }

    /// Rare-or-better pieces the local hero puts on, a co-op client's picks, and the profile's
    /// Bound jewelry are collected and toasted once; greens and other heroes' pieces are not.
    #[test]
    fn what_the_account_gets_is_collected_and_toasted() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(Profile::default());
        world.init_resource::<PickedJewels>();
        world.init_resource::<CollectionToasts>();
        let mut me = HeroState::default();
        super::super::wear(&mut me, by_key("hand_of_justice"), None).unwrap();
        super::super::wear(&mut me, by_key("medal_of_courage"), None).unwrap();
        world.spawn((me, LocalHero));
        let mut bot = HeroState::default();
        super::super::wear(&mut bot, by_key("kiss_of_the_spider"), None).unwrap();
        world.spawn(bot);
        world
            .resource_mut::<PickedJewels>()
            .0
            .push(by_key("earthstrike"));
        world.resource_mut::<Profile>().raid.bound.insert(2135);
        world.run_system_once(collect_new).unwrap();
        let c = &world.resource::<Profile>().collection;
        assert!(c.has("hand_of_justice") && c.has("earthstrike") && c.has("band_of_accuria"));
        assert!(!c.has("medal_of_courage"), "a green");
        assert!(!c.has("kiss_of_the_spider"), "another hero's");
        assert_eq!(world.resource::<CollectionToasts>().queue.len(), 3);
        assert!(world.resource::<PickedJewels>().0.is_empty());
        // Again: nothing new, nothing toasted.
        world.run_system_once(collect_new).unwrap();
        assert_eq!(world.resource::<CollectionToasts>().queue.len(), 3);
    }
}
