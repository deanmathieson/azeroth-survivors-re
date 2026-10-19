//! What can be gathered where (docs/PROFESSIONS.md, 4.1, 4.4, 4.5): the veins, herbs and fishing
//! pools as real 1.12 game objects, each with its `GameObjectDisplayInfo.dbc` model and its
//! `Lock.dbc` lock, which names the skill and the rank it needs; the zone bands; and what a
//! skinned beast gives.
//!
//! **Verified** by the data test [`tests::every_node_resolves_in_the_client`]: each display's model
//! is the node's (its file name), and each lock's skill slot is the node's skill at its rank.
//! **Not verified:** that these are the rows the cores' `gameobject_template` gives these nodes
//! (the client has no game object names), and the pools' locks, which are matched to a pool by
//! rank alone.

use serde::{Deserialize, Serialize};

use super::items::*;
use super::skills::Line;

/// What a node is gathered with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Kind {
    Vein,
    Herb,
    Pool,
}

impl Kind {
    pub(crate) const fn line(self) -> Line {
        match self {
            Kind::Vein => Line::Mining,
            Kind::Herb => Line::Herbalism,
            Kind::Pool => Line::Fishing,
        }
    }

    /// How near the hero must stand, in yards: on the node, or at a pool's edge.
    pub(crate) const fn reach(self) -> f32 {
        match self {
            Kind::Vein | Kind::Herb => 2.5,
            Kind::Pool => 9.0,
        }
    }

    /// The gather's cast, in seconds.
    pub(crate) const fn cast_secs(self) -> f32 {
        match self {
            Kind::Vein | Kind::Herb => 1.5,
            Kind::Pool => 2.5,
        }
    }
}

/// One gatherable game object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct NodeDef {
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    /// The `GameObjectDisplayInfo.dbc` row.
    pub(crate) display: u32,
    pub(crate) scale: f32,
    /// The `Lock.dbc` row.
    pub(crate) lock: u32,
    /// The skill it needs (the lock's; 1 where the lock says 0).
    pub(crate) req: u16,
    /// What it gives, 1-3 at a time.
    pub(crate) item: u32,
    /// A chance extra: `(item, percent)`, stone from a vein.
    pub(crate) extra: Option<(u32, u32)>,
    /// A rare node spawns this many times less often.
    pub(crate) rarity: u32,
    /// The display's model file name, which the data test checks.
    pub(crate) model: &'static str,
}

const fn node(
    name: &'static str,
    kind: Kind,
    display: u32,
    lock: u32,
    req: u16,
    item: u32,
    model: &'static str,
) -> NodeDef {
    NodeDef {
        name,
        kind,
        display,
        scale: 1.0,
        lock,
        req,
        item,
        extra: None,
        rarity: 1,
        model,
    }
}

const fn vein(
    name: &'static str,
    display: u32,
    lock: u32,
    req: u16,
    ore: u32,
    stone: u32,
    model: &'static str,
) -> NodeDef {
    NodeDef {
        extra: Some((stone, 35)),
        ..node(name, Kind::Vein, display, lock, req, ore, model)
    }
}

const fn herb(
    name: &'static str,
    display: u32,
    lock: u32,
    req: u16,
    item: u32,
    model: &'static str,
) -> NodeDef {
    node(name, Kind::Herb, display, lock, req, item, model)
}

const fn pool(
    name: &'static str,
    display: u32,
    lock: u32,
    req: u16,
    fish: u32,
    model: &'static str,
) -> NodeDef {
    NodeDef {
        scale: 1.5,
        ..node(name, Kind::Pool, display, lock, req, fish, model)
    }
}

/// Every node, in band order. Its index is its id on the wire.
pub(crate) static NODES: &[NodeDef] = &[
    // 1-15
    vein(
        "Copper Vein",
        310,
        38,
        1,
        COPPER_ORE,
        ROUGH_STONE,
        "copper_miningnode_01",
    ),
    herb("Peacebloom", 269, 29, 1, PEACEBLOOM, "bush_peacebloom01"),
    herb("Silverleaf", 270, 259, 1, SILVERLEAF, "bush_silverleaf01"),
    pool(
        "School of Fish",
        6291,
        1621,
        25,
        RAW_BRILLIANT_SMALLFISH,
        "tradeskill_fishschool_01",
    ),
    // 15-30
    vein(
        "Tin Vein",
        315,
        39,
        65,
        TIN_ORE,
        COARSE_STONE,
        "tin_miningnode_01",
    ),
    herb("Mageroyal", 268, 9, 50, MAGEROYAL, "bush_magebloom01"),
    herb("Bruiseweed", 358, 11, 100, BRUISEWEED, "bush_bruiseweed01"),
    pool(
        "Deviate Fish School",
        6482,
        1624,
        50,
        DEVIATE_FISH,
        "tradeskill_fishschool_02",
    ),
    // 30-45
    vein(
        "Iron Deposit",
        312,
        41,
        125,
        IRON_ORE,
        HEAVY_STONE,
        "iron_miningnode_01",
    ),
    vein(
        "Mithril Deposit",
        313,
        379,
        175,
        MITHRIL_ORE,
        SOLID_STONE,
        "mithril_miningnode_01",
    ),
    herb("Kingsblood", 320, 45, 125, KINGSBLOOD, "bush_crownroyal01"),
    herb("Liferoot", 677, 27, 150, LIFEROOT, "bush_liferoot01"),
    herb("Fadeleaf", 697, 47, 160, FADELEAF, "bush_fadeleaf01"),
    herb("Goldthorn", 698, 521, 170, GOLDTHORN, "bush_goldthorn01"),
    herb(
        "Khadgar's Whisker",
        701,
        49,
        185,
        KHADGARS_WHISKER,
        "bush_khadgarswhisker01",
    ),
    herb(
        "Purple Lotus",
        2314,
        439,
        210,
        PURPLE_LOTUS,
        "bush_purplelotus",
    ),
    pool(
        "Oily Blackmouth School",
        6485,
        1625,
        100,
        OILY_BLACKMOUTH,
        "tradeskill_fishschool_oil",
    ),
    // 45-55
    vein(
        "Truesilver Deposit",
        314,
        380,
        230,
        TRUESILVER_ORE,
        SOLID_STONE,
        "truesilver_miningnode_01",
    ),
    vein(
        "Small Thorium Vein",
        3951,
        400,
        245,
        THORIUM_ORE,
        DENSE_STONE,
        "thorium_miningnode_01",
    ),
    herb("Sungrass", 2315, 441, 230, SUNGRASS, "bush_sungrass"),
    herb("Gromsblood", 2313, 444, 250, GROMSBLOOD, "bush_gromsblood"),
    herb(
        "Golden Sansam",
        4652,
        1119,
        260,
        GOLDEN_SANSAM,
        "bush_sansam",
    ),
    pool(
        "Stonescale Eel Swarm",
        6742,
        1626,
        175,
        STONESCALE_EEL,
        "tradeskill_fishschool_eelsyellow",
    ),
    // 55-60
    vein(
        "Rich Thorium Vein",
        3952,
        939,
        275,
        THORIUM_ORE,
        DENSE_STONE,
        "richthorium_miningnode_01",
    ),
    herb("Dreamfoil", 4635, 1120, 270, DREAMFOIL, "bush_dreamfoil"),
    herb(
        "Mountain Silversage",
        4633,
        1121,
        280,
        MOUNTAIN_SILVERSAGE,
        "bush_mountainsilversage",
    ),
    herb(
        "Plaguebloom",
        4632,
        1122,
        285,
        PLAGUEBLOOM,
        "bush_plaguebloom",
    ),
    herb("Icecap", 4634, 1123, 290, ICECAP, "bush_icecap"),
    NodeDef {
        rarity: 8,
        ..herb(
            "Black Lotus",
            4636,
            1124,
            300,
            BLACK_LOTUS,
            "bush_blacklotus",
        )
    },
];

/// A zone band (4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Band {
    B1,
    B15,
    B30,
    B45,
    B55,
    /// Molten Core: no herbs, no water, and no vein yet (Dark Iron has no verified display).
    Raid,
}

/// The battleground's band, by its name (4.1's table).
pub(crate) fn band_of(map: &str) -> Band {
    match map {
        "Elwynn Forest" | "Durotar" | "Teldrassil" | "Dun Morogh" | "Mulgore"
        | "Tirisfal Glades" => Band::B1,
        "The Barrens" | "Westfall" | "Redridge Mountains" | "Duskwood" => Band::B15,
        "Gurubashi Arena" => Band::B30,
        "Fire Plume Ridge" | "The Dark Portal" => Band::B45,
        "Gates of Ahn'Qiraj" | "Mount Hyjal" | "Naxxramas" => Band::B55,
        _ => Band::Raid,
    }
}

/// The node indices a band spawns. A band shares the one below's best vein, as 4.1's table has it.
pub(crate) fn band_nodes(band: Band) -> &'static [usize] {
    match band {
        Band::B1 => &[0, 1, 2, 3],
        Band::B15 => &[0, 4, 5, 6, 7],
        Band::B30 => &[8, 9, 10, 11, 12, 13, 14, 15, 16],
        Band::B45 => &[9, 17, 18, 19, 20, 21, 22],
        Band::B55 => &[18, 23, 24, 25, 26, 27, 28, 22],
        Band::Raid => &[],
    }
}

/// A skinned beast's yield by its level, and the Skinning it needs: vmangos's rule, 1 to level
/// 10, `(level - 10) * 10` to 20, `level * 5` above.
pub(crate) fn skin(level: u32) -> (u32, u16) {
    let item = match level {
        0..=10 => RUINED_LEATHER_SCRAPS,
        11..=20 => LIGHT_LEATHER,
        21..=32 => MEDIUM_LEATHER,
        33..=44 => HEAVY_LEATHER,
        45..=55 => THICK_LEATHER,
        _ => RUGGED_LEATHER,
    };
    let req = match level {
        0..=10 => 1,
        11..=19 => (level - 10) * 10,
        _ => level * 5,
    };
    (item, req.min(300) as u16)
}

/// Whether a foe is a beast, by its name: survivors' foes carry no creature type. Insects,
/// demons, the dead and the humanoids are not.
pub(crate) fn is_beast(name: &str) -> bool {
    const BEASTS: &[&str] = &[
        "Plainstrider",
        "Kodo",
        "Scorpid",
        "Boar",
        "Ravager",
        "Widow",
        "Spider",
        "Tarantula",
        "Recluse",
        "Duskbat",
        "Taillasher",
        "Basilisk",
        "Core Hound",
        "Condor",
        "Coyote",
        "Devilsaur",
        "Diemetradon",
        "Duskstalker",
        "Gorilla",
        "Lizard",
        "Cougar",
        "Fleshripper",
        "Wolf",
        "Goretusk",
        "Pterrordax",
        "Thunderhawk",
        "Hyena",
        "Bear",
        "Jungle Stalker",
        "Leopard",
        "Lashtail",
        "Screecher",
        "Scytheclaw",
        "Huntress",
        "Prowler",
        "Prairie Stalker",
        "Nightsaber",
        "Panther",
        "Tiger",
        "Raptor",
        "Ravasaur",
        "Stegodon",
        "Owl",
        "Swoop",
        "Zhevra",
        "Echeyakee",
        "Humar",
        "Bangalash",
        "Mother Fang",
        "Lupos",
        "Swine",
        "Snickerfang",
        "Magmadar",
        "Thunderhead",
        "Stormsnout",
        "Thrashtail",
        "Snapjaw",
    ];
    BEASTS.iter().any(|b| name.contains(b))
}

/// The fish a camp's bobber lands, by band (4.5).
pub(crate) fn camp_fish(band: Band) -> (u32, u16) {
    match band {
        Band::B1 => (RAW_BRILLIANT_SMALLFISH, 1),
        Band::B15 => (RAW_LONGJAW_MUD_SNAPPER, 25),
        Band::B30 => (OILY_BLACKMOUTH, 100),
        Band::B45 | Band::B55 | Band::Raid => (STONESCALE_EEL, 175),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survivors_nodes_tables_are_consistent() {
        for (i, n) in NODES.iter().enumerate() {
            let item = super::super::items::template(n.item)
                .unwrap_or_else(|| panic!("{}: {} has no template", n.name, n.item));
            assert!(item.stack >= 1);
            if let Some((extra, pct)) = n.extra {
                assert!(super::super::items::template(extra).is_some(), "{}", n.name);
                assert!(pct <= 100);
            }
            assert!(n.req >= 1 && n.req <= 300, "{i}");
        }
        for band in [Band::B1, Band::B15, Band::B30, Band::B45, Band::B55] {
            assert!(!band_nodes(band).is_empty());
            for &i in band_nodes(band) {
                assert!(i < NODES.len());
            }
        }
        for map in super::super::super::data::MAPS {
            if !map.name.starts_with("Molten Core") {
                assert_ne!(band_of(map.name), Band::Raid, "{} has a band", map.name);
            }
        }
        let item = |l: u32| skin(l).0;
        assert_eq!(item(8), RUINED_LEATHER_SCRAPS);
        assert_eq!(skin(15), (LIGHT_LEATHER, 50));
        assert_eq!(skin(40).1, 200);
        assert!(super::super::items::template(item(60)).is_some());
        assert!(is_beast("Hecklefang Hyena") && is_beast("Core Hound"));
        assert!(!is_beast("Razormane Quilboar") && !is_beast("Felhound"));
    }

    /// The nodes are real: each display's model is the node's, and each lock's skill slot is the
    /// node's skill at the node's rank. Every mismatch is listed.
    /// The `LockType.dbc` index a kind's lock names: Herbalism 2, Mining 3, Fishing 19.
    fn lock_type(kind: Kind) -> u32 {
        match kind {
            Kind::Herb => 2,
            Kind::Vein => 3,
            Kind::Pool => 19,
        }
    }

    #[test]
    fn every_node_resolves_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let go = benilla_formats::load_gameobject_catalog(&mut chain).expect("GO displays");
        let locks = benilla_formats::load_lock_catalog(&mut chain).expect("Lock.dbc");
        let mut bad = Vec::new();
        for n in NODES {
            match go.model_path(n.display) {
                Some(path) if path.to_ascii_lowercase().contains(n.model) => {}
                other => bad.push(format!("{}: display {} is {other:?}", n.name, n.display)),
            }
            let slot = locks.slots(n.lock).and_then(|s| {
                s.iter().find(|s| {
                    s.key_type == benilla_formats::LOCK_KEY_SKILL && s.index == lock_type(n.kind)
                })
            });
            match slot {
                Some(s) if s.skill.max(1) == u32::from(n.req) => {}
                other => bad.push(format!("{}: lock {} has {other:?}", n.name, n.lock)),
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
