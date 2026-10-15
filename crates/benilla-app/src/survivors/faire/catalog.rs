//! The Faire's prizes: every collectable the Prize Booth can pull and the Prize Counter sells, each
//! with its kind, name, rarity and `Interface\Icons` icon. The companion looks come from the
//! companion's own list ([`crate::survivors::rewards::all_looks`]); the rest are seeded here. A
//! prize is owned when the store that keeps its kind holds its key ([`owned`]), and [`grant`]
//! writes it there: looks to the rewards, mounts, hats and tabards to the quests' unlocks (so a
//! quest chain's reward and a pulled one are the same unlock), titles, toys and cards to the
//! Faire's own save until their features land.

use std::sync::LazyLock;

use bevy::prelude::*;

use super::super::cards::{COMMON, EPIC, LEGENDARY, RARE, UNCOMMON};
use super::super::profile::{slug, Profile};
use super::super::rewards::{all_looks, offered, RewardsSave};

/// A prize's rarity, which sets its odds, its duplicate Favor and its Counter price.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::survivors) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub(in crate::survivors) const ALL: [Self; 5] = [
        Self::Common,
        Self::Uncommon,
        Self::Rare,
        Self::Epic,
        Self::Legendary,
    ];

    pub(in crate::survivors) fn label(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }

    /// WoW's item-quality colour.
    pub(in crate::survivors) fn color(self) -> Color {
        match self {
            Self::Common => COMMON,
            Self::Uncommon => UNCOMMON,
            Self::Rare => RARE,
            Self::Epic => EPIC,
            Self::Legendary => LEGENDARY,
        }
    }

    /// A pull's chance of this rarity, in hundredths of a percent: 60%, 25%, 11%, 3.5%, 0.5%.
    pub(in crate::survivors) fn odds(self) -> u32 {
        match self {
            Self::Common => 6_000,
            Self::Uncommon => 2_500,
            Self::Rare => 1_100,
            Self::Epic => 350,
            Self::Legendary => 50,
        }
    }

    /// The odds as the screen prints them: "60%", "3.5%".
    pub(in crate::survivors) fn odds_label(self) -> String {
        let bp = self.odds();
        if bp.is_multiple_of(100) {
            format!("{}%", bp / 100)
        } else {
            format!("{}.{}%", bp / 100, (bp % 100) / 10)
        }
    }

    /// Faire Favor for a duplicate of this rarity.
    pub(in crate::survivors) fn favor(self) -> u32 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 3,
            Self::Rare => 8,
            Self::Epic => 25,
            Self::Legendary => 100,
        }
    }

    /// Silas Darkmoon's price at the Prize Counter, in Favor, before any featured discount.
    pub(in crate::survivors) fn price(self) -> u32 {
        match self {
            Self::Common => 15,
            Self::Uncommon => 40,
            Self::Rare => 100,
            Self::Epic => 300,
            Self::Legendary => 1_000,
        }
    }
}

/// What a prize is, and so which store keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::survivors) enum Kind {
    CompanionLook,
    Mount,
    MountHat,
    Tabard,
    Title,
    /// Recorded only: toys are a feature still to come.
    Toy,
    /// Recorded only: decks and their trinkets are still to come.
    DarkmoonCard,
}

impl Kind {
    pub(in crate::survivors) const ALL: [Self; 7] = [
        Self::CompanionLook,
        Self::Mount,
        Self::MountHat,
        Self::Tabard,
        Self::Title,
        Self::Toy,
        Self::DarkmoonCard,
    ];

    /// One of the kind: "Mount hat".
    pub(in crate::survivors) fn label(self) -> &'static str {
        match self {
            Self::CompanionLook => "Companion",
            Self::Mount => "Mount",
            Self::MountHat => "Mount hat",
            Self::Tabard => "Tabard",
            Self::Title => "Title",
            Self::Toy => "Toy",
            Self::DarkmoonCard => "Darkmoon Card",
        }
    }

    /// The shelf's heading: "Mount Hats".
    pub(in crate::survivors) fn plural(self) -> &'static str {
        match self {
            Self::CompanionLook => "Companions",
            Self::Mount => "Mounts",
            Self::MountHat => "Mount Hats",
            Self::Tabard => "Tabards",
            Self::Title => "Titles",
            Self::Toy => "Toys",
            Self::DarkmoonCard => "Darkmoon Cards",
        }
    }

    /// The id prefix: `mount:brown_horse`.
    fn prefix(self) -> &'static str {
        match self {
            Self::CompanionLook => "look",
            Self::Mount => "mount",
            Self::MountHat => "hat",
            Self::Tabard => "tabard",
            Self::Title => "title",
            Self::Toy => "toy",
            Self::DarkmoonCard => "card",
        }
    }
}

/// One prize.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Prize {
    /// `<kind>:<key>`, unique across the registry; banners name their featured prizes by it.
    pub id: String,
    pub kind: Kind,
    /// The key its store keeps: the name's slug, as the quest chains key theirs; a look's own key.
    pub key: String,
    pub name: String,
    pub rarity: Rarity,
    /// `Interface\Icons\<icon>.blp`.
    pub icon: &'static str,
    /// A companion look's `CreatureDisplayInfo` row, which [`RewardsSave::unlocked_looks`] keeps.
    pub look: Option<u32>,
}

use Rarity::{Common as C, Epic as E, Legendary as L, Rare as R, Uncommon as U};

/// The companion looks' rarities, by the look's key; a look the companion adds later is Rare.
const LOOK_SEEDS: &[(&str, Rarity, &str)] = &[
    ("black_kingsnake", U, "Ability_Hunter_Pet_WindSerpent"),
    ("undercity_cockroach", U, "Ability_Hunter_Pet_Spider"),
    ("emerald_whelpling", R, "INV_Misc_Head_Dragon_Green"),
];
const LOOK_DEFAULT: (Rarity, &str) = (R, "INV_Box_PetCarrier_01");

/// The Faire's own vanity pets, `(name, CreatureDisplayInfo row, rarity, icon)`, each display
/// checked against the 1.12 client's data by the companion workstream: rarer by fame, the cats and
/// birds Common or Uncommon, the engineered ones Rare, the whelplings and the Oozeling Epic. A
/// display the companion's own list already has is that look, not a second prize.
const FAIRE_PETS: &[(&str, u32, Rarity, &str)] = &[
    ("Silver Tabby", 5555, C, "INV_Box_PetCarrier_01"),
    ("Bombay", 5556, C, "INV_Box_PetCarrier_01"),
    ("Cornish Rex", 5586, C, "INV_Box_PetCarrier_01"),
    ("Cockatiel", 6191, C, "INV_Box_Birdcage_01"),
    ("Senegal", 6190, U, "INV_Box_Birdcage_01"),
    ("Great Horned Owl", 4615, U, "Ability_Hunter_Pet_Owl"),
    ("Hawk Owl", 6300, U, "Ability_Hunter_Pet_Owl"),
    ("Tree Frog", 6297, U, "INV_Box_01"),
    ("Crimson Snake", 6303, U, "Ability_Hunter_Pet_WindSerpent"),
    ("Pet Bombling", 8909, R, "INV_Misc_Bomb_04"),
    ("Lil' Smoky", 8910, R, "INV_Gizmo_05"),
    ("Smolderweb Hatchling", 9934, R, "Ability_Hunter_Pet_Spider"),
    ("Crimson Whelpling", 6290, E, "INV_Misc_Head_Dragon_Red"),
    ("Dark Whelpling", 6288, E, "INV_Misc_Head_Dragon_Black"),
    ("Disgusting Oozeling", 15436, E, "INV_Potion_92"),
];

/// Real 1.12 mounts: the riding mounts Uncommon, the battleground and faction ones Rare, the swift
/// ones Epic, and the raid and dungeon drops Legendary.
const MOUNTS: &[(&str, Rarity, &str)] = &[
    ("Brown Horse", U, "Ability_Mount_RidingHorse"),
    ("Chestnut Mare", U, "Ability_Mount_RidingHorse"),
    ("Pinto", U, "Ability_Mount_RidingHorse"),
    ("Gray Kodo", U, "Ability_Mount_Kodo_01"),
    ("Brown Ram", U, "Ability_Mount_MountainRam"),
    ("Striped Frostsaber", U, "Ability_Mount_WhiteTiger"),
    ("Red Mechanostrider", U, "Ability_Mount_MechaStrider"),
    ("Red Skeletal Horse", U, "Ability_Mount_Undeadhorse"),
    ("Horn of the Dire Wolf", U, "Ability_Mount_BlackDireWolf"),
    (
        "Horn of the Frostwolf Howler",
        R,
        "Ability_Mount_WhiteDireWolf",
    ),
    ("Stormpike Battle Charger", R, "Ability_Mount_MountainRam"),
    ("Black War Tiger", R, "Ability_Mount_BlackPanther"),
    ("Swift Palomino", E, "Ability_Mount_RidingHorse"),
    (
        "Swift White Mechanostrider",
        E,
        "Ability_Mount_MechaStrider",
    ),
    (
        "Reins of the Winterspring Frostsaber",
        E,
        "Ability_Mount_PinkTiger",
    ),
    ("Swift Stormsaber", E, "Ability_Mount_BlackPanther"),
    (
        "Horn of the Black War Wolf",
        L,
        "Ability_Mount_BlackDireWolf",
    ),
    ("Deathcharger's Reins", L, "Ability_Mount_Undeadhorse"),
    ("Swift Razzashi Raptor", L, "Ability_Mount_Raptor"),
    ("Swift Zulian Tiger", L, "Ability_Mount_JungleTiger"),
];

const HATS: &[(&str, Rarity, &str)] = &[
    ("Tiny Party Hat", C, "INV_Helmet_52"),
    ("Pirate Hat", C, "INV_Helmet_66"),
    ("Red Winter Hat", C, "INV_Helmet_67"),
    ("Green Winter Hat", U, "INV_Helmet_68"),
    ("Dapper Top Hat", U, "INV_Helmet_69"),
    ("Tiny Pith Helmet", U, "INV_Helmet_50"),
    ("Wizard Hat", R, "INV_Helmet_53"),
    ("Jack-o'-Lantern Hat", R, "INV_Helmet_70"),
    ("Crown of the Fire Festival", E, "INV_Crown_02"),
];

/// `(name, rarity, icon, ItemDisplayInfo)`.  Unlike most Faire prizes, tabards are worn by the
/// character compositor, so their display rows are part of the catalog too.
const TABARDS: &[(&str, Rarity, &str, u32)] = &[
    ("Stormwind Tabard", C, "INV_Misc_TabardPVP_01", 23914),
    ("Ironforge Tabard", C, "INV_Misc_TabardPVP_03", 23916),
    ("Darnassus Tabard", C, "INV_Misc_TabardPVP_01", 23917),
    ("Gnomeregan Tabard", C, "INV_Misc_TabardPVP_03", 23918),
    ("Orgrimmar Tabard", C, "INV_Misc_TabardPVP_02", 23913),
    ("Thunder Bluff Tabard", C, "INV_Misc_TabardPVP_04", 23915),
    ("Undercity Tabard", C, "INV_Misc_TabardPVP_02", 23919),
    ("Darkspear Tabard", C, "INV_Misc_TabardPVP_04", 23920),
    (
        "People's Militia Tabard",
        U,
        "INV_Shirt_GuildTabard_01",
        15198,
    ),
    ("Nethergarde Tabard", U, "INV_Shirt_GuildTabard_01", 15198),
    ("Arathor Battle Tabard", U, "INV_Misc_TabardPVP_03", 23916),
    (
        "Battle Tabard of the Defilers",
        U,
        "INV_Misc_TabardPVP_04",
        23915,
    ),
    ("Stormpike Battle Tabard", R, "INV_Misc_TabardPVP_01", 15198),
    ("Frostwolf Battle Tabard", R, "INV_Misc_TabardPVP_02", 15198),
    (
        "Tabard of the Argent Dawn",
        R,
        "INV_Shirt_GuildTabard_01",
        15198,
    ),
    (
        "Tabard of the Cenarion Circle",
        R,
        "INV_Shirt_GuildTabard_01",
        15198,
    ),
    (
        "Tabard of the Darkmoon Faire",
        E,
        "INV_Shirt_GuildTabard_01",
        15198,
    ),
];

const TITLES: &[(&str, Rarity, &str)] = &[
    ("the Faire-Goer", C, "INV_Misc_Ticket_Darkmoon_01"),
    ("Carnie", C, "INV_Misc_Gift_01"),
    ("Ticket Hoarder", U, "INV_Misc_Ticket_Darkmoon_01"),
    ("the Lucky", U, "INV_Misc_Coin_02"),
    ("of the Shattered Hand", R, "Ability_Rogue_Eviscerate"),
    ("the Patient", R, "Spell_Nature_TimeStop"),
    ("Champion of the Faire", E, "INV_Misc_Ticket_Tarot_Stack_01"),
    ("Scourgebane", E, "INV_Misc_Head_Scourge_01"),
    ("Jenkins", E, "Ability_Warrior_BattleShout"),
    ("the Insane", L, "Spell_Shadow_Charm"),
    ("Scarab Lord", L, "INV_Misc_QirajiCrystal_01"),
    ("the Flamewaker", L, "Spell_Fire_Fire"),
];

const TOYS: &[(&str, Rarity, &str)] = &[
    ("Noggenfogger Elixir", C, "INV_Potion_83"),
    ("Goblin Gumbo", C, "INV_Misc_Food_63"),
    ("Snowball", C, "INV_Ammo_Snowball"),
    ("Savory Deviate Delight", U, "INV_Misc_Fish_14"),
    (
        "Darkmoon Fireworks",
        U,
        "INV_Misc_MissileLargeCluster_Purple",
    ),
    ("Toy Train Set", R, "INV_Gizmo_02"),
    ("Discombobulator Ray", R, "INV_Gizmo_08"),
    ("Sayge's Fortune", R, "INV_Misc_Note_02"),
    ("Orb of Deception", E, "INV_Misc_Orb_02"),
    ("Dartol's Rod of Transformation", E, "INV_Wand_07"),
];

/// The four decks of Darkmoon cards, each Ace to Eight; the Ace is Epic, the rest Rare.
const SUITS: &[(&str, &str)] = &[
    ("Beasts", "INV_Misc_Ticket_Tarot_Beasts_01"),
    ("Elementals", "INV_Misc_Ticket_Tarot_Elemental_01"),
    ("Portals", "INV_Misc_Ticket_Tarot_Portal_01"),
    ("Warlords", "INV_Misc_Ticket_Tarot_Warlord_01"),
];
const RANKS: [&str; 8] = [
    "Ace", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight",
];

/// The completed deck's genuine 1.12 trinket.  The suit names are the classic cards' sets.
const DECKS: &[(&str, &str, u32, &str)] = &[
    (
        "beasts",
        "Darkmoon Card: Blue Dragon",
        19288,
        "INV_Misc_Ticket_Tarot_BlueDragon_01",
    ),
    (
        "elementals",
        "Darkmoon Card: Maelstrom",
        19289,
        "INV_Misc_Ticket_Tarot_Maelstrom_01",
    ),
    (
        "portals",
        "Darkmoon Card: Twisting Nether",
        19290,
        "INV_Misc_Ticket_Tarot_TwistingNether_01",
    ),
    (
        "warlords",
        "Darkmoon Card: Heroism",
        19287,
        "INV_Misc_Ticket_Tarot_Heroism_01",
    ),
];

/// The deck a suit completes, including its 1.12 item id and client icon.
pub(in crate::survivors) fn deck(suit: &str) -> Option<(&'static str, u32, &'static str)> {
    DECKS
        .iter()
        .find(|(key, ..)| *key == suit)
        .map(|&(_, name, item, icon)| (name, item, icon))
}

/// The equipped tabard's compositing display, only while the prize remains owned.
pub(in crate::survivors) fn equipped_tabard(profile: &Profile) -> Option<u32> {
    let key = profile.faire.wardrobe.tabard.as_deref()?;
    profile.quests.tabards.contains(key).then(|| {
        TABARDS
            .iter()
            .find(|(name, ..)| slug(name) == key)
            .expect("saved tabard key belongs to the Faire catalog")
            .3
    })
}

/// Whether all eight cards in `suit` are owned.
pub(in crate::survivors) fn deck_complete(profile: &Profile, suit: &str) -> bool {
    RANKS.iter().all(|rank| {
        profile
            .faire
            .cards
            .contains(&slug(&format!("{rank} of {suit}")))
    })
}

/// Every supported completed-deck key.
pub(in crate::survivors) fn deck_keys() -> impl Iterator<Item = &'static str> {
    DECKS.iter().map(|(key, ..)| *key)
}

/// Decorate `name` as WoW displays this title.  Articles and prepositions read after the name;
/// honorifics and achievement names read before it.
pub(in crate::survivors) fn titled_name(name: &str, title: Option<&str>) -> String {
    let Some(title) = title else {
        return name.into();
    };
    if matches!(
        title,
        "Carnie"
            | "Ticket Hoarder"
            | "Champion of the Faire"
            | "Scourgebane"
            | "Jenkins"
            | "Scarab Lord"
    ) {
        format!("{title} {name}")
    } else {
        format!("{name} {title}")
    }
}

/// The selected title's display text, only while it is still owned.
pub(in crate::survivors) fn equipped_title(profile: &Profile) -> Option<&'static str> {
    let key = profile.faire.wardrobe.title.as_deref()?;
    profile.faire.titles.contains(key).then(|| {
        by_id(&format!("title:{key}"))
            .expect("saved title key belongs to the Faire catalog")
            .name
            .as_str()
    })
}

fn prize(kind: Kind, key: String, name: String, rarity: Rarity, icon: &'static str) -> Prize {
    Prize {
        id: format!("{}:{key}", kind.prefix()),
        kind,
        key,
        name,
        rarity,
        icon,
        look: None,
    }
}

fn build() -> Vec<Prize> {
    let mut all = Vec::new();
    // The companion's starting looks are every account's already: only the earned ones are prizes.
    let starting = RewardsSave::default();
    let none = super::super::achievements::AchievementSave::default();
    for look in all_looks().filter(|l| !offered(l, &starting, &none)) {
        let (rarity, icon) = LOOK_SEEDS
            .iter()
            .find(|(key, ..)| *key == look.key)
            .map(|&(_, r, i)| (r, i))
            .or_else(|| {
                FAIRE_PETS
                    .iter()
                    .find(|p| p.1 == look.display)
                    .map(|&(_, _, r, i)| (r, i))
            })
            .unwrap_or(LOOK_DEFAULT);
        all.push(Prize {
            look: Some(look.display),
            ..prize(
                Kind::CompanionLook,
                look.key.to_string(),
                look.name.to_string(),
                rarity,
                icon,
            )
        });
    }
    for &(name, display, rarity, icon) in FAIRE_PETS {
        if all_looks().any(|l| l.display == display) {
            continue;
        }
        all.push(Prize {
            look: Some(display),
            ..prize(
                Kind::CompanionLook,
                slug(name),
                name.to_string(),
                rarity,
                icon,
            )
        });
    }
    for (kind, seeds) in [
        (Kind::Mount, MOUNTS),
        (Kind::MountHat, HATS),
        (Kind::Title, TITLES),
        (Kind::Toy, TOYS),
    ] {
        for &(name, rarity, icon) in seeds {
            all.push(prize(kind, slug(name), name.to_string(), rarity, icon));
        }
    }
    for &(name, rarity, icon, _) in TABARDS {
        all.push(prize(
            Kind::Tabard,
            slug(name),
            name.to_string(),
            rarity,
            icon,
        ));
    }
    for &(suit, icon) in SUITS {
        for (i, rank) in RANKS.iter().enumerate() {
            let name = format!("{rank} of {suit}");
            let rarity = if i == 0 { E } else { R };
            all.push(prize(Kind::DarkmoonCard, slug(&name), name, rarity, icon));
        }
    }
    all
}

static CATALOG: LazyLock<Vec<Prize>> = LazyLock::new(build);

/// Every prize, looks first, then by kind as [`Kind::ALL`] orders them.
pub(in crate::survivors) fn catalog() -> &'static [Prize] {
    &CATALOG
}

/// The prize with `id`.
pub(in crate::survivors) fn by_id(id: &str) -> Option<&'static Prize> {
    catalog().iter().find(|p| p.id == id)
}

/// The index in [`catalog`] of the prize with `id`.
pub(in crate::survivors) fn index_of(id: &str) -> Option<usize> {
    catalog().iter().position(|p| p.id == id)
}

/// Whether the account has `prize`, in whichever store keeps its kind.
pub(in crate::survivors) fn owned(profile: &Profile, prize: &Prize) -> bool {
    match prize.kind {
        Kind::CompanionLook => prize
            .look
            .is_some_and(|d| profile.rewards.unlocked_looks.contains(&d)),
        Kind::Mount => profile.quests.mounts.contains(&prize.key),
        Kind::MountHat => profile.quests.mount_hats.contains(&prize.key),
        Kind::Tabard => profile.quests.tabards.contains(&prize.key),
        Kind::Title => profile.faire.titles.contains(&prize.key),
        Kind::Toy => profile.faire.toys.contains(&prize.key),
        Kind::DarkmoonCard => profile.faire.cards.contains(&prize.key),
    }
}

/// Unlock `prize` in the store that keeps its kind; whether it was new.
pub(in crate::survivors) fn grant(profile: &mut Profile, prize: &Prize) -> bool {
    let key = prize.key.clone();
    match prize.kind {
        Kind::CompanionLook => prize
            .look
            .is_some_and(|d| profile.rewards.unlocked_looks.insert(d)),
        Kind::Mount => profile.quests.mounts.insert(key),
        Kind::MountHat => profile.quests.mount_hats.insert(key),
        Kind::Tabard => profile.quests.tabards.insert(key),
        Kind::Title => profile.faire.titles.insert(key),
        Kind::Toy => profile.faire.toys.insert(key),
        Kind::DarkmoonCard => {
            let fresh = profile.faire.cards.insert(key);
            for suit in deck_keys() {
                if deck_complete(profile, suit) {
                    profile.faire.darkmoon_trinkets.insert(suit.into());
                }
            }
            fresh
        }
    }
}

/// `(owned, of)` over the prizes `filter` keeps.
pub(in crate::survivors) fn completion(
    profile: &Profile,
    filter: impl Fn(&Prize) -> bool,
) -> (usize, usize) {
    catalog()
        .iter()
        .filter(|p| filter(p))
        .fold((0, 0), |(have, of), p| {
            (have + usize::from(owned(profile, p)), of + 1)
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn ids_are_unique_and_every_kind_and_rarity_is_stocked() {
        let ids: HashSet<&str> = catalog().iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids.len(), catalog().len());
        for kind in Kind::ALL {
            assert!(catalog().iter().any(|p| p.kind == kind), "{kind:?}");
        }
        for rarity in Rarity::ALL {
            assert!(catalog().iter().any(|p| p.rarity == rarity), "{rarity:?}");
        }
        assert_eq!(Rarity::ALL.iter().map(|r| r.odds()).sum::<u32>(), 10_000);
        assert_eq!(Rarity::Epic.odds_label(), "3.5%");
        assert_eq!(Rarity::Common.odds_label(), "60%");
        // 32 cards, the Aces Epic.
        let cards: Vec<&Prize> = catalog()
            .iter()
            .filter(|p| p.kind == Kind::DarkmoonCard)
            .collect();
        assert_eq!(cards.len(), 32);
        assert!(cards
            .iter()
            .all(|c| (c.rarity == Rarity::Epic) == c.name.starts_with("Ace")));
        assert_eq!(
            by_id("card:eight_of_warlords").map(|p| p.rarity),
            Some(Rarity::Rare)
        );
    }

    #[test]
    fn the_quest_chain_unlocks_are_the_same_keys() {
        // What the quest chains award ([`crate::survivors::quests`]) is a prize here too, under the
        // key the chain records.
        for id in [
            "mount:brown_horse",
            "mount:gray_kodo",
            "mount:horn_of_the_black_war_wolf",
            "mount:swift_razzashi_raptor",
            "hat:tiny_party_hat",
            "hat:tiny_pith_helmet",
            "hat:jack_o_lantern_hat",
            "tabard:darnassus_tabard",
            "tabard:nethergarde_tabard",
            "tabard:people_s_militia_tabard",
            "look:black_kingsnake",
            "look:undercity_cockroach",
            "look:emerald_whelpling",
        ] {
            assert!(by_id(id).is_some(), "{id}");
        }
        // The starting looks are everyone's: never a prize.
        assert!(by_id("look:squirrel").is_none());
        // The Faire's own pets are, each once.
        for &(name, display, rarity, _) in FAIRE_PETS {
            let looks: Vec<&Prize> = catalog()
                .iter()
                .filter(|p| p.look == Some(display))
                .collect();
            assert_eq!(looks.len(), 1, "{name}");
            assert_eq!(
                (looks[0].kind, looks[0].rarity),
                (Kind::CompanionLook, rarity)
            );
            let look = all_looks().find(|look| look.display == display).unwrap();
            assert_eq!(looks[0].name, name);
            assert_eq!(look.name, name);
        }
        assert_eq!(
            by_id("look:disgusting_oozeling").map(|p| p.rarity),
            Some(Rarity::Epic)
        );
        assert_eq!(by_id("look:black_kingsnake").unwrap().look, Some(1206));
    }

    #[test]
    fn grants_land_in_the_existing_stores() {
        let mut p = Profile::default();
        let get = |id: &str| by_id(id).unwrap();
        for id in [
            "look:emerald_whelpling",
            "look:lil_smoky",
            "mount:deathcharger_s_reins",
            "hat:wizard_hat",
            "tabard:stormwind_tabard",
            "title:the_insane",
            "toy:orb_of_deception",
            "card:ace_of_portals",
        ] {
            assert!(!owned(&p, get(id)), "{id}");
            assert!(grant(&mut p, get(id)), "{id} is new");
            assert!(owned(&p, get(id)), "{id}");
            assert!(!grant(&mut p, get(id)), "{id} again is no news");
        }
        assert!(p.rewards.unlocked_looks.contains(&6291));
        assert!(p.rewards.unlocked_looks.contains(&8910));
        assert!(p.quests.mounts.contains("deathcharger_s_reins"));
        assert!(p.quests.mount_hats.contains("wizard_hat"));
        assert!(p.quests.tabards.contains("stormwind_tabard"));
        assert!(p.faire.titles.contains("the_insane"));
        assert!(p.faire.toys.contains("orb_of_deception"));
        assert!(p.faire.cards.contains("ace_of_portals"));
        // A chain's reward reads as owned here too.
        p.quests.mounts.insert("brown_horse".into());
        assert!(owned(&p, get("mount:brown_horse")));
        let (have, of) = completion(&p, |prize| prize.kind == Kind::Mount);
        assert_eq!((have, of), (2, MOUNTS.len()));
    }

    #[test]
    fn a_complete_deck_awards_its_trinket_once() {
        let mut profile = Profile::default();
        for rank in RANKS {
            let prize = by_id(&format!("card:{}", slug(&format!("{rank} of Beasts")))).unwrap();
            assert!(grant(&mut profile, prize));
        }
        assert!(deck_complete(&profile, "beasts"));
        assert!(profile.faire.darkmoon_trinkets.contains("beasts"));
        let ace = by_id("card:ace_of_beasts").unwrap();
        assert!(!grant(&mut profile, ace));
        assert_eq!(profile.faire.darkmoon_trinkets.len(), 1);
    }

    #[test]
    fn titles_use_wow_prefixes_and_suffixes() {
        assert_eq!(titled_name("Thrall", Some("Carnie")), "Carnie Thrall");
        assert_eq!(
            titled_name("Thrall", Some("the Faire-Goer")),
            "Thrall the Faire-Goer"
        );
    }

    #[test]
    fn icons_are_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let chain = benilla_formats::open_chain(&data).expect("open chain");
        let mut icons: Vec<&str> = catalog().iter().map(|p| p.icon).collect();
        icons.push(LOOK_DEFAULT.1);
        for icon in icons {
            let path = format!("Interface\\Icons\\{icon}.blp");
            assert!(chain.contains(&path), "{path}");
        }
    }

    #[test]
    fn tabard_displays_are_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let displays =
            benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        for &(name, _, _, display) in TABARDS {
            assert!(
                displays.get(display).is_some(),
                "{name}: no display {display}"
            );
        }
    }
}
