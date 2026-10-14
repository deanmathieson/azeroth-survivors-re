//! Which battleground bosses drop which gear: a row per boss, in [`MAPS`](super::super::data::MAPS)
//! and roster order. The rarity follows the zone's level band:
//!
//! - Common: the starting zones (Elwynn Forest, Durotar, Teldrassil, Dun Morogh, Mulgore,
//!   Tirisfal Glades).
//! - Uncommon: the low-level zones (The Barrens, Westfall, Duskwood, Redridge Mountains).
//! - Rare: the mid zones (Gurubashi Arena, The Dark Portal, Fire Plume Ridge).
//! - Epic: the 55+ zones (Gates of Ahn'Qiraj, Mount Hyjal, Naxxramas).
//! - Legendary: only the last bosses of the two hardest, Ossirian the Unscarred and Kel'Thuzad,
//!   who split the nine slots between them.
//!
//! Apart from those two, each zone's bosses drop all nine slots at its rarity between them, so any
//! zone of a band can finish a set. The chance climbs with the boss's place in the roster, from
//! 0.40 to 0.60.

use super::Rarity::{self, *};
use super::Slot::{self, *};
use super::{BossDrop, ZoneDrops};

const fn boss(name: &'static str, slots: &'static [Slot], rarity: Rarity, chance: f32) -> BossDrop {
    BossDrop {
        boss: name,
        slots,
        rarity,
        chance,
    }
}

/// One row per [`MAPS`](super::super::data::MAPS) entry, in its order.
pub(super) static ZONES: &[ZoneDrops] = &[
    ZoneDrops {
        map: "The Barrens",
        bosses: &[
            boss("Echeyakee", &[Hands, Feet], Uncommon, 0.40),
            boss("Takk the Leaper", &[Wrist, Waist], Uncommon, 0.44),
            boss("Lakota'mani", &[Shoulder], Uncommon, 0.48),
            boss("Humar the Pridelord", &[Legs, Head], Uncommon, 0.52),
            boss("Agathelos the Raging", &[Chest], Uncommon, 0.56),
            boss("Hezrul Bloodmark", &[MainHand, Chest], Uncommon, 0.60),
        ],
    },
    ZoneDrops {
        map: "The Dark Portal",
        bosses: &[
            boss("Akubar the Seer", &[Wrist, Hands], Rare, 0.40),
            boss("Lord Azrethoc", &[Waist, Feet], Rare, 0.44),
            boss("Lord Hel'nurath", &[Shoulder], Rare, 0.48),
            boss("Teremus the Devourer", &[Legs, Chest], Rare, 0.52),
            boss("Lord Banehollow", &[Head], Rare, 0.56),
            boss("Lord Kazzak", &[MainHand, Chest], Rare, 0.60),
        ],
    },
    ZoneDrops {
        map: "Gates of Ahn'Qiraj",
        bosses: &[
            boss("Kurinnaxx", &[Wrist, Feet], Epic, 0.40),
            boss("General Rajaxx", &[Waist, Hands], Epic, 0.44),
            boss("Moam", &[Legs, Shoulder], Epic, 0.48),
            boss("Buru the Gorger", &[Chest, MainHand], Epic, 0.52),
            boss("Princess Huhuran", &[Head, Hands], Epic, 0.56),
            boss(
                "Ossirian the Unscarred",
                &[Waist, Feet, Wrist, Hands],
                Legendary,
                0.60,
            ),
        ],
    },
    ZoneDrops {
        map: "Fire Plume Ridge",
        bosses: &[
            boss("Ravasaur Matriarch", &[Feet, Hands], Rare, 0.40),
            boss("Uhk'loc", &[Wrist, Waist], Rare, 0.44),
            boss("Blazerunner", &[Shoulder, Head], Rare, 0.48),
            boss("Baron Charr", &[Legs], Rare, 0.52),
            boss("Tyrant Devilsaur", &[Chest], Rare, 0.56),
            boss("King Mosh", &[MainHand, Legs], Rare, 0.60),
        ],
    },
    ZoneDrops {
        map: "Gurubashi Arena",
        bosses: &[
            boss("King Bangalash", &[Hands, Wrist], Rare, 0.40),
            boss("Mokk the Savage", &[Feet, Waist], Rare, 0.44),
            boss("Bloodlord Mandokir", &[Shoulder, MainHand], Rare, 0.48),
            boss("High Priest Thekal", &[Legs], Rare, 0.52),
            boss("Jin'do the Hexxer", &[Head], Rare, 0.56),
            boss("Hakkar", &[Chest, MainHand], Rare, 0.60),
        ],
    },
    ZoneDrops {
        map: "Elwynn Forest",
        bosses: &[
            boss("Mother Fang", &[Wrist, Feet], Common, 0.40),
            boss("Narg the Taskmaster", &[Hands, Waist], Common, 0.44),
            boss("Hogger", &[MainHand], Common, 0.48),
            boss("Thuros Lightfingers", &[Shoulder], Common, 0.52),
            boss("Fedfennel", &[Legs, Head], Common, 0.56),
            boss("Gruff Swiftbite", &[Chest], Common, 0.60),
        ],
    },
    ZoneDrops {
        map: "Durotar",
        bosses: &[
            boss("Geolord Mottle", &[Feet, Hands], Common, 0.40),
            boss("Razormane Dustrunner", &[Wrist], Common, 0.44),
            boss("Razormane Battleguard", &[Waist, Shoulder], Common, 0.48),
            boss("Burning Blade Neophyte", &[Legs], Common, 0.52),
            boss("Death Flayer", &[Head, Chest], Common, 0.56),
            boss("Burning Blade Cultist", &[MainHand], Common, 0.60),
        ],
    },
    ZoneDrops {
        map: "Teldrassil",
        bosses: &[
            boss("Threggil", &[Hands, Wrist], Common, 0.40),
            boss("Uruson", &[Feet], Common, 0.44),
            boss("Gnarlpine Defender", &[Waist, Legs], Common, 0.48),
            boss("Duskstalker", &[Shoulder], Common, 0.52),
            boss("Oakenscowl", &[Chest, Head], Common, 0.56),
            boss("Gnarlpine Augur", &[MainHand], Common, 0.60),
        ],
    },
    ZoneDrops {
        map: "Dun Morogh",
        bosses: &[
            boss("Edan the Howler", &[Wrist], Common, 0.40),
            boss("Timber", &[Feet, Hands], Common, 0.44),
            boss("Great Father Arctikus", &[Head], Common, 0.48),
            boss("Old Icebeard", &[Shoulder, Waist], Common, 0.52),
            boss("Vagash", &[Legs], Common, 0.56),
            boss("Bjarn", &[Chest, MainHand], Common, 0.60),
        ],
    },
    ZoneDrops {
        map: "Mulgore",
        bosses: &[
            boss("Prairie Stalker", &[Feet], Common, 0.40),
            boss("Mazzranache", &[Hands, Wrist], Common, 0.44),
            boss("Snagglespear", &[MainHand], Common, 0.48),
            boss("Swoop", &[Shoulder], Common, 0.52),
            boss("Elder Plainstrider", &[Waist, Legs], Common, 0.56),
            boss("Ghost Howl", &[Head, Chest], Common, 0.60),
        ],
    },
    ZoneDrops {
        map: "Tirisfal Glades",
        bosses: &[
            boss("Farmer Solliden", &[Hands, Waist], Common, 0.40),
            boss("Tormented Spirit", &[Wrist], Common, 0.44),
            boss("Bayne", &[Feet], Common, 0.48),
            boss("Ressan the Needler", &[Shoulder, Legs], Common, 0.52),
            boss("Fellicent's Shade", &[Head], Common, 0.56),
            boss("Gorefang", &[Chest, MainHand], Common, 0.60),
        ],
    },
    ZoneDrops {
        map: "Westfall",
        bosses: &[
            boss("Brainwashed Noble", &[Wrist, Hands], Uncommon, 0.40),
            boss("Marisa du'Paige", &[Waist, Feet], Uncommon, 0.44),
            boss("Sergeant Brashclaw", &[Shoulder], Uncommon, 0.48),
            boss("Klaven Mortwake", &[Head], Uncommon, 0.52),
            boss("Lupos", &[Legs], Uncommon, 0.56),
            boss("Vultros", &[Chest, MainHand], Uncommon, 0.60),
        ],
    },
    ZoneDrops {
        map: "Duskwood",
        bosses: &[
            boss("Lupos", &[Feet, Wrist], Uncommon, 0.40),
            boss("Naraxis", &[Hands, Waist], Uncommon, 0.44),
            boss("Nightbane Vile Fang", &[Legs], Uncommon, 0.48),
            boss("Nightbane Tainted One", &[Shoulder], Uncommon, 0.52),
            boss("Fenros", &[Head, Chest], Uncommon, 0.56),
            boss("Nefaru", &[MainHand], Uncommon, 0.60),
        ],
    },
    ZoneDrops {
        map: "Redridge Mountains",
        bosses: &[
            boss("Blackrock Hunter", &[Wrist, Feet], Uncommon, 0.40),
            boss("Singe", &[Hands], Uncommon, 0.44),
            boss("Tharil'zun", &[Waist, Shoulder], Uncommon, 0.48),
            boss("Blackrock Gladiator", &[Legs], Uncommon, 0.52),
            boss("Rohh the Silent", &[Head], Uncommon, 0.56),
            boss("Kazon", &[Chest, MainHand], Uncommon, 0.60),
        ],
    },
    ZoneDrops {
        map: "Mount Hyjal",
        bosses: &[
            boss("Prince Xavalis", &[Wrist, Waist], Epic, 0.40),
            boss("Lady Hederine", &[Hands, Feet], Epic, 0.44),
            boss("Taerar", &[Shoulder, Legs], Epic, 0.48),
            boss("Lethon", &[Chest], Epic, 0.52),
            boss("Emeriss", &[Head, Legs], Epic, 0.56),
            boss("Ysondre", &[MainHand, Head], Epic, 0.60),
        ],
    },
    ZoneDrops {
        map: "Naxxramas",
        bosses: &[
            boss("Anub'Rekhan", &[Wrist, Hands], Epic, 0.40),
            boss("Grand Widow Faerlina", &[Waist, Feet], Epic, 0.44),
            boss("Patchwerk", &[Legs, MainHand], Epic, 0.48),
            boss("Gluth", &[Shoulder, Waist], Epic, 0.52),
            boss("Thaddius", &[Head, Chest], Epic, 0.56),
            boss(
                "Kel'Thuzad",
                &[Head, Shoulder, Chest, Legs, MainHand],
                Legendary,
                0.60,
            ),
        ],
    },
];
