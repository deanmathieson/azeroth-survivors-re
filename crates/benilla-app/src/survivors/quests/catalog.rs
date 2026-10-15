//! The quests themselves, by zone: who gives them on the battleground, the in-run quests each zone
//! offers (written against that zone's own roster), and the zone's account-wide chain with its
//! reward. All of it is data: [`super::field`] runs the quests, [`super::chain`] judges the chains.

use super::super::data;

/// How long a Defend quest's spot must be held, in all.
const HOLD_SECS: f32 = 30.0;
/// A Collect's item drops from its foe at this chance a kill.
const DROP_CHANCE: f32 = 0.4;

/// What an in-run quest asks of the party. Progress is the party's: whoever lands the kill or
/// picks the item up, it counts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Goal {
    /// Slay `count` of the foe named (any foe at all for `None`).
    Kill {
        foe: Option<&'static str>,
        count: u32,
    },
    /// Pick up `count` of `item`, which the foe named drops at `chance` a kill, picked up as a gem
    /// is.
    Collect {
        item: &'static str,
        look: ItemLook,
        foe: &'static str,
        chance: f32,
        count: u32,
    },
    /// Stand within [`super::field::SPOT_RADIUS`] of a marked spot for `secs` in all, while the
    /// horde comes at it harder.
    Defend { secs: f32 },
    /// Kill the next elite (or guardian) to fall.
    SlayElite,
}

/// How a quest item looks on the ground: a `GameObjectDisplayInfo` row and its scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum ItemLook {
    Candle,
    Sack,
    Scroll,
    Bone,
    Eggs,
}

impl ItemLook {
    #[cfg(test)]
    pub(in crate::survivors) const ALL: [Self; 5] = [
        Self::Candle,
        Self::Sack,
        Self::Scroll,
        Self::Bone,
        Self::Eggs,
    ];

    /// The `GameObjectDisplayInfo` row and the drawn scale.
    pub(in crate::survivors) fn display(self) -> (u32, f32) {
        match self {
            // `World\Generic\PassiveDoodads\Lights\Candle01.mdx`.
            Self::Candle => (100, 3.0),
            // `World\Generic\PassiveDoodads\Furniture\Containers\Sack01.mdx`.
            Self::Sack => (5511, 0.6),
            // `World\Goober\G_Scroll01.mdx`.
            Self::Scroll => (210, 1.6),
            // `World\Azeroth\Elwynn\PassiveDoodads\BattleGladeSkullHuman1\...`.
            Self::Bone => (448, 1.4),
            // `World\Generic\Human\Passive Doodads\SpiderEggsSack\SpiderEggSack04.mdx`.
            Self::Eggs => (364, 0.5),
        }
    }
}

/// One in-run quest.
#[derive(Debug)]
pub(in crate::survivors) struct QuestDef {
    /// Unique across every zone: `WOW_SURVIVORS_QUEST` names a quest by it.
    pub id: &'static str,
    pub name: &'static str,
    /// The giver's one line.
    pub text: &'static str,
    pub goal: Goal,
    /// Seconds from accepting it to finishing it.
    pub limit: f32,
}

/// The NPC who hands out a zone's quests: a `CreatureDisplayInfo.dbc` row and its drawn scale.
#[derive(Debug)]
pub(in crate::survivors) struct Giver {
    pub name: &'static str,
    pub display: u32,
    pub scale: f32,
}

/// What finishing a chain unlocks. Mounts, mount hats and tabards are features still to come: the
/// chain records their keys ([`Reward::key`]) for them to read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Reward {
    /// Talent XP for the class that finished the chain.
    TalentXp(u64),
    /// A companion look, by its `CreatureDisplayInfo` row
    /// ([`super::super::rewards::earned_look`]).
    CompanionLook(u32),
    Mount(&'static str),
    MountHat(&'static str),
    Tabard(&'static str),
}

impl Reward {
    /// What kind of reward this is, for the Armory.
    pub(in crate::survivors) fn kind(self) -> &'static str {
        match self {
            Self::TalentXp(_) => "Talent XP",
            Self::CompanionLook(_) => "Companion look",
            Self::Mount(_) => "Mount",
            Self::MountHat(_) => "Mount hat",
            Self::Tabard(_) => "Tabard",
        }
    }

    /// The reward's own name: "Brown Horse", "+2000 talent XP".
    pub(in crate::survivors) fn name(self) -> String {
        match self {
            Self::TalentXp(xp) => format!("+{xp} talent XP"),
            Self::CompanionLook(display) => super::super::rewards::earned_look(display)
                .map_or_else(|| format!("Look {display}"), |l| l.name.to_string()),
            Self::Mount(name) | Self::MountHat(name) | Self::Tabard(name) => name.to_string(),
        }
    }

    /// The key a mount, hat or tabard is saved under: its name's slug.
    pub(in crate::survivors) fn key(self) -> Option<String> {
        match self {
            Self::Mount(name) | Self::MountHat(name) | Self::Tabard(name) => {
                Some(super::super::profile::slug(name))
            }
            Self::TalentXp(_) | Self::CompanionLook(_) => None,
        }
    }
}

/// A zone's account-wide chain: [`super::chain::CHAIN_QUESTS`] of its quests, then its last boss,
/// then a win at [`super::chain::CHAIN_WIN_MULT`] rewards or more.
#[derive(Debug)]
pub(in crate::survivors) struct ChainDef {
    pub name: &'static str,
    pub text: &'static str,
    pub reward: Reward,
}

/// One zone's quest content, by its [`data::MapDef::name`].
#[derive(Debug)]
pub(in crate::survivors) struct ZoneQuests {
    pub zone: &'static str,
    pub giver: Giver,
    pub quests: &'static [QuestDef],
    pub chain: ChainDef,
}

/// The quest content of `map` ([`data::MAPS`]), if it has any.
pub(in crate::survivors) fn for_map(map: usize) -> Option<&'static ZoneQuests> {
    let name = data::MAPS.get(map)?.name;
    ZONES.iter().find(|z| z.zone == name)
}

/// The quest `id` names, with the zone it belongs to.
pub(in crate::survivors) fn quest(id: &str) -> Option<(&'static ZoneQuests, &'static QuestDef)> {
    ZONES
        .iter()
        .find_map(|z| z.quests.iter().find(|q| q.id == id).map(|q| (z, q)))
}

const fn q(
    id: &'static str,
    name: &'static str,
    text: &'static str,
    goal: Goal,
    limit: f32,
) -> QuestDef {
    QuestDef {
        id,
        name,
        text,
        goal,
        limit,
    }
}

const fn kill(foe: &'static str, count: u32) -> Goal {
    Goal::Kill {
        foe: Some(foe),
        count,
    }
}

const fn kill_any(count: u32) -> Goal {
    Goal::Kill { foe: None, count }
}

const fn collect(item: &'static str, look: ItemLook, foe: &'static str, count: u32) -> Goal {
    Goal::Collect {
        item,
        look,
        foe,
        chance: DROP_CHANCE,
        count,
    }
}

const HOLD: Goal = Goal::Defend { secs: HOLD_SECS };

const fn giver(name: &'static str, display: u32, scale: f32) -> Giver {
    Giver {
        name,
        display,
        scale,
    }
}

const fn chain(name: &'static str, text: &'static str, reward: Reward) -> ChainDef {
    ChainDef { name, text, reward }
}

/// Every zone's quests. The givers' displays are checked against the 1.12 client's
/// `CreatureDisplayInfo.dbc` (the NPC is named beside each); a quest's foe is its zone's.
pub(in crate::survivors) static ZONES: &[ZoneQuests] = &[
    ZoneQuests {
        zone: "The Barrens",
        // `OrcMale.mdx`.
        giver: giver("Mankrik", 3855, 1.15),
        quests: &[
            q(
                "barrens_razormane_raiders",
                "Razormane Raiders",
                "The Razormane keep raiding the Crossroads' supply carts. Thin out the quilboar.",
                kill("Razormane Quilboar", 12),
                120.0,
            ),
            q(
                "barrens_centaur_bracers",
                "Centaur Bracers",
                "Every Kolkar you drop is one less raider. Bring back their bracers as proof.",
                collect("Centaur Bracers", ItemLook::Sack, "Kolkar Wrangler", 8),
                150.0,
            ),
            q(
                "barrens_hold_the_waystone",
                "Hold the Waystone",
                "Stand by the old waystone and keep the herds from trampling the road.",
                HOLD,
                120.0,
            ),
            q(
                "barrens_thunder_on_the_plains",
                "Thunder on the Plains",
                "Something huge is stirring in the grass. Bring it down before it reaches the road.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Gold Road",
            "Keep the road from the Crossroads to Ratchet open, and see the plains' last terror fall.",
            Reward::Mount("Gray Kodo"),
        ),
    },
    ZoneQuests {
        zone: "The Dark Portal",
        // A Nethergarde Officer, `HumanMale.mdx`.
        giver: giver("Nethergarde Officer", 6752, 1.15),
        quests: &[
            q(
                "portal_snickerfang_jowls",
                "Snickerfang Jowls",
                "Nethergarde's bloodmage wants hyena jowls for his brews. Best not to ask why.",
                collect("Snickerfang Jowl", ItemLook::Bone, "Snickerfang Hyena", 8),
                150.0,
            ),
            q(
                "portal_the_shadowsworn",
                "The Shadowsworn",
                "Cultists chant at the foot of the Portal. Silence them before they finish.",
                kill("Shadowsworn Cultist", 10),
                120.0,
            ),
            q(
                "portal_hold_the_line",
                "Hold Nethergarde's Line",
                "Plant the keep's banner on the basin floor and stand by it.",
                HOLD,
                120.0,
            ),
            q(
                "portal_heralds_of_the_legion",
                "Heralds of the Legion",
                "The Legion is sending a champion through. Be there to meet it.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "Watchers of the Portal",
            "Nethergarde Keep stands between Azeroth and the Legion. Earn its colours.",
            Reward::Tabard("Nethergarde Tabard"),
        ),
    },
    ZoneQuests {
        zone: "Gates of Ahn'Qiraj",
        // `NightElfMale.mdx`.
        giver: giver("Cenarion Hold Infantry", 15304, 1.15),
        quests: &[
            q(
                "aq_securing_the_supply_lines",
                "Securing the Supply Lines",
                "Skitterers swarm Cenarion Hold's wagons. Clear the sands around them.",
                kill("Sand Skitterer", 12),
                120.0,
            ),
            q(
                "aq_encrypted_twilight_texts",
                "Encrypted Twilight Texts",
                "The Twilight's Hammer carry their orders in cipher. The Hold's scholars want them.",
                collect(
                    "Encrypted Twilight Text",
                    ItemLook::Scroll,
                    "Twilight Stonecaller",
                    8,
                ),
                150.0,
            ),
            q(
                "aq_hold_the_scarab_wall",
                "Hold the Scarab Wall",
                "The wall groans. Stand at the mark while the swarm throws itself at you.",
                HOLD,
                120.0,
            ),
            q(
                "aq_champions_of_the_swarm",
                "Champions of the Swarm",
                "A Qiraji champion leads the next wave. End it before it reaches the wall.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Scepter of the Shifting Sands",
            "Prove yourself before the Scarab Wall, and its gates will remember your name.",
            Reward::Mount("Horn of the Black War Wolf"),
        ),
    },
    ZoneQuests {
        zone: "Fire Plume Ridge",
        // `TaurenMale.mdx`.
        giver: giver("Torwa Pathfinder", 8832, 1.25),
        quests: &[
            q(
                "ungoro_raptor_mastery",
                "Raptor Mastery",
                "Torwa wants to see how you fare against the crater's quickest hunters.",
                kill("Ravasaur", 12),
                120.0,
            ),
            q(
                "ungoro_bloodpetal_sprouts",
                "Bloodpetal Sprouts",
                "The flayers carry sprouts tangled in their roots. Pull them free for study.",
                collect("Bloodpetal Sprout", ItemLook::Sack, "Bloodpetal Flayer", 8),
                150.0,
            ),
            q(
                "ungoro_fire_plume_forged",
                "Fire Plume Forged",
                "Hold the vent on the summit while the crater's beasts come for its heat.",
                HOLD,
                120.0,
            ),
            q(
                "ungoro_ruthless_predators",
                "Ruthless Predators",
                "The biggest predator in the crater is on the hunt. Hunt it first.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Fires of Un'Goro",
            "Tame the summit of Fire Plume Ridge, all the way up to its king.",
            Reward::MountHat("Tiny Pith Helmet"),
        ),
    },
    ZoneQuests {
        zone: "Gurubashi Arena",
        // `GoblinMale.mdx`.
        giver: giver("Short John Mithril", 14561, 1.1),
        quests: &[
            q(
                "arena_panther_mastery",
                "Panther Mastery",
                "The handlers let the panthers out first. Show the crowd how it's done.",
                kill("Shadowmaw Panther", 12),
                120.0,
            ),
            q(
                "arena_bloodsail_buccaneers",
                "Bloodsail Buccaneers",
                "The Bloodsail have money riding on the fights. Their orders say how much.",
                collect("Bloodsail Orders", ItemLook::Scroll, "Bloodsail Sea Dog", 8),
                150.0,
            ),
            q(
                "arena_hold_the_floor",
                "Hold the Arena Floor",
                "Stand on the mark in the middle of the sand, and don't step off it.",
                HOLD,
                120.0,
            ),
            q(
                "arena_blood_in_the_sand",
                "Blood in the Sand",
                "The crowd wants a slaughter. Give them sixty, and quickly.",
                kill_any(60),
                90.0,
            ),
        ],
        chain: chain(
            "Blood of the Gurubashi",
            "Win the arena crowd's favour first, then win the arena itself.",
            Reward::Mount("Swift Razzashi Raptor"),
        ),
    },
    ZoneQuests {
        zone: "Elwynn Forest",
        // `HumanMale.mdx`.
        giver: giver("Marshal Dughan", 1985, 1.15),
        quests: &[
            q(
                "elwynn_fargodeep_mine",
                "The Fargodeep Mine",
                "Kobolds are digging their way out of the Fargodeep Mine. Send them back down.",
                kill("Kobold Tunneler", 10),
                120.0,
            ),
            q(
                "elwynn_kobold_candles",
                "Kobold Candles",
                "The miners light their tunnels with stolen wax. Take the candles back.",
                collect("Large Candle", ItemLook::Candle, "Kobold Miner", 8),
                150.0,
            ),
            q(
                "elwynn_princess_must_die",
                "Princess Must Die!",
                "Something big and angry is loose on the Brackwell farm. Put it down before supper.",
                Goal::SlayElite,
                150.0,
            ),
            q(
                "elwynn_pumpkin_patch",
                "Defend the Pumpkin Patch",
                "Stand by the scarecrow and keep the horde off the Brackwell pumpkins.",
                HOLD,
                120.0,
            ),
        ],
        chain: chain(
            "Hogger's Bounty",
            "Marshal Dughan pays for a safe forest, from the kobold mines to the beasts that rule \
             the woods.",
            Reward::Mount("Brown Horse"),
        ),
    },
    ZoneQuests {
        zone: "Durotar",
        // A Razor Hill Grunt, `OrcMale.mdx`.
        giver: giver("Razor Hill Grunt", 9798, 1.15),
        quests: &[
            q(
                "durotar_vile_familiars",
                "Vile Familiars",
                "Imps from the Burning Blade's rituals are loose in the canyons. Stamp them out.",
                kill("Vile Familiar", 12),
                120.0,
            ),
            q(
                "durotar_sting_of_the_scorpid",
                "Sting of the Scorpid",
                "Razor Hill's apothecary needs stingers, and the armored ones carry the best.",
                collect("Scorpid Stinger", ItemLook::Sack, "Armored Scorpid", 8),
                150.0,
            ),
            q(
                "durotar_hold_razor_hill",
                "Hold Razor Hill",
                "Stand at the watch post and hold the line for the Horde.",
                HOLD,
                120.0,
            ),
            q(
                "durotar_dark_storms",
                "Dark Storms",
                "A champion of the Burning Blade walks the valley. Make an example of it.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Burning Blade",
            "Root out the cult hiding in Durotar's canyons, down to its last master.",
            Reward::CompanionLook(1206),
        ),
    },
    ZoneQuests {
        zone: "Teldrassil",
        // `NightElfFemale.mdx`.
        giver: giver("Sentinel Shaya", 12476, 1.15),
        quests: &[
            q(
                "teldrassil_balance_of_nature",
                "The Balance of Nature",
                "Too many young nightsabers stalk the glades. Restore the balance.",
                kill("Young Nightsaber", 12),
                120.0,
            ),
            q(
                "teldrassil_webwood_venom",
                "Webwood Venom",
                "The druids need venom from the Webwood spiders to brew an antidote.",
                collect("Webwood Venom Sac", ItemLook::Eggs, "Webwood Lurker", 8),
                150.0,
            ),
            q(
                "teldrassil_moonwell_watch",
                "The Moonwell's Watch",
                "Keep watch over the moonwell while the corrupted creatures close in.",
                HOLD,
                120.0,
            ),
            q(
                "teldrassil_gnarlpine_corruption",
                "Gnarlpine Corruption",
                "A fel-touched brute leads the next charge. Cleanse the glade of it.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "Shadows in the Canopy",
            "Find what corrupts the Gnarlpine and cut it out of Teldrassil.",
            Reward::Tabard("Darnassus Tabard"),
        ),
    },
    ZoneQuests {
        zone: "Dun Morogh",
        // An Ironforge Mountaineer, `DwarfMale.mdx`.
        giver: giver("Ironforge Mountaineer", 1598, 1.2),
        quests: &[
            q(
                "dunmorogh_boar_hunter",
                "The Boar Hunter",
                "The crag boars are tearing up the passes. Thin the herd.",
                kill("Crag Boar", 12),
                120.0,
            ),
            q(
                "dunmorogh_boar_ribs",
                "Beer Basted Boar Ribs",
                "Ribs for the Thunderbrew grill. The bigger the boar, the better the rib.",
                collect("Crag Boar Rib", ItemLook::Bone, "Large Crag Boar", 8),
                150.0,
            ),
            q(
                "dunmorogh_hold_the_brewery",
                "Hold the Brewery",
                "Stand guard at the brewery door. Nobody touches the stout.",
                HOLD,
                120.0,
            ),
            q(
                "dunmorogh_troll_cave",
                "The Troll Cave",
                "A brute has come down from the Frostmane caves. Send it back in pieces.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Thunderbrew Run",
            "Make the road to Kharanos safe enough for a barrel to roll home on its own.",
            Reward::MountHat("Tiny Party Hat"),
        ),
    },
    ZoneQuests {
        zone: "Mulgore",
        // `TaurenFemale.mdx`.
        giver: giver("Brave Windfeather", 3770, 1.25),
        quests: &[
            q(
                "mulgore_hunt_begins",
                "The Hunt Begins",
                "Every young brave's first hunt. Show the plainstriders your spear.",
                kill("Plainstrider", 12),
                120.0,
            ),
            q(
                "mulgore_prairie_wolf_paws",
                "Prairie Wolf Paws",
                "The wolves grow bold near the herds. Their paws tell the elders how many.",
                collect("Prairie Wolf Paw", ItemLook::Sack, "Prairie Wolf", 8),
                150.0,
            ),
            q(
                "mulgore_hold_the_red_rocks",
                "Hold the Red Rocks",
                "Stand by the sacred stones while the plains rush at you.",
                HOLD,
                120.0,
            ),
            q(
                "mulgore_rite_of_strength",
                "Rite of Strength",
                "The Earthmother asks for a worthy foe. The next great beast will do.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "Rites of the Earthmother",
            "Walk Mulgore's rites: the hunt, the vigil and the greatest beast of the plains.",
            Reward::TalentXp(1000),
        ),
    },
    ZoneQuests {
        zone: "Tirisfal Glades",
        // `ScourgeMale.mdx`.
        giver: giver("Executor Zygand", 1649, 1.15),
        quests: &[
            q(
                "tirisfal_mindless_ones",
                "The Mindless Ones",
                "Mindless dead wander the fields. Put them to rest for good.",
                kill("Rotting Dead", 12),
                120.0,
            ),
            q(
                "tirisfal_rot_hide_ichor",
                "Rot Hide Ichor",
                "The apothecaries want the Rot Hide's tainted blood. Fill these vials.",
                collect("Rot Hide Ichor", ItemLook::Eggs, "Rot Hide Gnoll", 8),
                150.0,
            ),
            q(
                "tirisfal_hold_brill",
                "Hold Brill",
                "Stand at Brill's gate and let nothing living through.",
                HOLD,
                120.0,
            ),
            q(
                "tirisfal_scarlet_crusade",
                "The Scarlet Crusade",
                "A zealot champion leads the next charge. The Dark Lady wants it dead.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Forsaken Watch",
            "Hold Tirisfal for the Forsaken against the dead, the gnolls and the Crusade.",
            Reward::CompanionLook(2177),
        ),
    },
    ZoneQuests {
        zone: "Westfall",
        // `HumanMale.mdx`.
        giver: giver("Gryan Stoutmantle", 1690, 1.15),
        quests: &[
            q(
                "westfall_defias_brotherhood",
                "The Defias Brotherhood",
                "The Defias smuggle stolen grain out of Westfall. Stop them in the fields.",
                kill("Defias Smuggler", 10),
                120.0,
            ),
            q(
                "westfall_red_leather_bandanas",
                "Red Leather Bandanas",
                "Every bandana is a Defias who won't be back. Bring them to Sentinel Hill.",
                collect("Red Leather Bandana", ItemLook::Sack, "Defias Trapper", 8),
                150.0,
            ),
            q(
                "westfall_hold_sentinel_hill",
                "Hold Sentinel Hill",
                "Stand where the militia can see you and hold the hill.",
                HOLD,
                120.0,
            ),
            q(
                "westfall_patrolling_westfall",
                "Patrolling Westfall",
                "Something big is coming over the hills toward the farms. Stop it.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "Justice for Westfall",
            "The People's Militia needs a champion to break the Defias hold on Westfall.",
            Reward::Tabard("People's Militia Tabard"),
        ),
    },
    ZoneQuests {
        zone: "Duskwood",
        // `HumanMale.mdx`.
        giver: giver("Watcher Dodds", 2384, 1.15),
        quests: &[
            q(
                "duskwood_night_watch",
                "The Night Watch",
                "The dead are rising from Raven Hill's graves. Lay them back down.",
                kill("Skeletal Warrior", 12),
                120.0,
            ),
            q(
                "duskwood_gooey_spider_legs",
                "Gooey Spider Legs",
                "Darkshire's cook swears by them. The Night Watch is too scared to ask why.",
                collect("Gooey Spider Leg", ItemLook::Eggs, "Venom Web Spider", 8),
                150.0,
            ),
            q(
                "duskwood_hold_darkshire",
                "Hold Darkshire",
                "Stand by the lantern post and keep the dark off Darkshire.",
                HOLD,
                120.0,
            ),
            q(
                "duskwood_worgen_in_the_woods",
                "Worgen in the Woods",
                "The Nightbane pack hunts closer to town every night. Cull it.",
                kill("Nightbane Worgen", 10),
                120.0,
            ),
        ],
        chain: chain(
            "Lanterns of Darkshire",
            "Keep the lanterns of Darkshire lit through Duskwood's longest night.",
            Reward::MountHat("Jack-o'-Lantern Hat"),
        ),
    },
    ZoneQuests {
        zone: "Redridge Mountains",
        // `HumanMale.mdx`.
        giver: giver("Magistrate Solomon", 1753, 1.15),
        quests: &[
            q(
                "redridge_goulash",
                "Redridge Goulash",
                "Lakeshire's inn is out of goulash, and the goretusk snouts are the secret.",
                collect("Great Goretusk Snout", ItemLook::Sack, "Great Goretusk", 8),
                150.0,
            ),
            q(
                "redridge_murloc_poachers",
                "Murloc Poachers",
                "Murlocs are poaching Lake Everstill dry. Drive them back into the water.",
                kill("Murloc Shorestriker", 12),
                120.0,
            ),
            q(
                "redridge_hold_the_bridge",
                "Hold Lakeshire Bridge",
                "Stand on the bridge and hold it while the hills empty out at you.",
                HOLD,
                120.0,
            ),
            q(
                "redridge_blackrock_menace",
                "Blackrock Menace",
                "The Blackrock send one of their best down from the peaks. Show it the way back.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "Blackrock Invasion",
            "Lakeshire stands alone against the Blackrock orcs. Stand with it.",
            Reward::TalentXp(2000),
        ),
    },
    ZoneQuests {
        zone: "Mount Hyjal",
        // `KeeperOfTheGrove.mdx`, whose display scale is 2.
        giver: giver("Keeper Remulos", 11906, 1.6),
        quests: &[
            q(
                "hyjal_scourge_at_the_roots",
                "The Scourge at the Roots",
                "Ghouls are gnawing at the World Tree's roots. Scatter them.",
                kill("Cannibal Ghoul", 12),
                120.0,
            ),
            q(
                "hyjal_felbound_shards",
                "Felbound Shards",
                "The satyrs carry shards of corrupted souls. Remulos will cleanse them.",
                collect(
                    "Corrupted Soul Shard",
                    ItemLook::Bone,
                    "Jadefire Felsworn",
                    8,
                ),
                150.0,
            ),
            q(
                "hyjal_battle_for_mount_hyjal",
                "The Battle for Mount Hyjal",
                "Stand at the ancients' circle and hold the mountain once more.",
                HOLD,
                120.0,
            ),
            q(
                "hyjal_nightmare_corruption",
                "Nightmare Corruption",
                "A Nightmare-touched champion leads the attack. Wake it for good.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "Guardians of the World Tree",
            "Defend Nordrassil as the ancients once did, against the Legion, the Scourge and the \
             Nightmare.",
            Reward::CompanionLook(6291),
        ),
    },
    ZoneQuests {
        zone: "Naxxramas",
        // `HumanMale.mdx`.
        giver: giver("Commander Eligor Dawnbringer", 16015, 1.15),
        quests: &[
            q(
                "naxx_clearing_the_plaguewood",
                "Clearing the Plaguewood",
                "The necropolis drops soldiers on the Plaguewood. Break their ranks.",
                kill("Scourge Soldier", 12),
                120.0,
            ),
            q(
                "naxx_invaders_scourgestones",
                "Invader's Scourgestones",
                "The Argent Dawn wants every scourgestone the cult carries.",
                collect(
                    "Invader's Scourgestone",
                    ItemLook::Bone,
                    "Death Cultist",
                    8,
                ),
                150.0,
            ),
            q(
                "naxx_under_the_necropolis",
                "Under the Necropolis",
                "Hold the consecrated ground beneath Naxxramas while the dead pour down.",
                HOLD,
                120.0,
            ),
            q(
                "naxx_dread_citadel",
                "The Dread Citadel",
                "One of Kel'Thuzad's lieutenants takes the field. Strike it down.",
                Goal::SlayElite,
                150.0,
            ),
        ],
        chain: chain(
            "The Fall of Kel'Thuzad",
            "Bring Naxxramas down out of the sky, one plague at a time.",
            Reward::TalentXp(5000),
        ),
    },
];

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn every_zone_has_its_quests_and_one_chain() {
        for (i, map) in data::MAPS.iter().enumerate() {
            let zone = for_map(i).unwrap_or_else(|| panic!("{} has no quests", map.name));
            assert!(
                zone.quests.len() >= 3,
                "{}: {}",
                map.name,
                zone.quests.len()
            );
            assert!(!zone.chain.name.is_empty() && !zone.chain.text.is_empty());
            assert!(!zone.giver.name.is_empty() && zone.giver.scale > 0.0);
        }
        // One entry per zone, and no zone that is not on the menu.
        assert_eq!(ZONES.len(), data::MAPS.len());
        for zone in ZONES {
            assert!(
                data::MAPS.iter().any(|m| m.name == zone.zone),
                "{}",
                zone.zone
            );
        }
    }

    #[test]
    fn every_foe_a_quest_names_is_on_its_zones_roster() {
        for (i, map) in data::MAPS.iter().enumerate() {
            let roster = data::roster(i);
            let zone = for_map(i).unwrap();
            for quest in zone.quests {
                let foe = match quest.goal {
                    Goal::Kill { foe: Some(f), .. } | Goal::Collect { foe: f, .. } => f,
                    _ => continue,
                };
                assert!(
                    roster.enemies.iter().any(|e| e.name == foe),
                    "{}: {} names {foe}, not on the roster",
                    map.name,
                    quest.id
                );
            }
        }
    }

    #[test]
    fn quests_are_well_formed_and_their_ids_unique() {
        let mut ids = HashSet::new();
        for zone in ZONES {
            for def in zone.quests {
                assert!(ids.insert(def.id), "{} twice", def.id);
                assert!(!def.name.is_empty() && !def.text.is_empty(), "{}", def.id);
                assert!(def.limit >= 60.0, "{}", def.id);
                match def.goal {
                    Goal::Kill { count, .. } => assert!(count > 0),
                    Goal::Collect { count, chance, .. } => {
                        assert!(count > 0 && chance > 0.0 && chance <= 1.0)
                    }
                    Goal::Defend { secs } => assert!(secs > 0.0 && secs < def.limit),
                    Goal::SlayElite => {}
                }
                assert_eq!(quest(def.id).map(|(z, _)| z.zone), Some(zone.zone));
            }
        }
        assert!(quest("no_such_quest").is_none());
        // Every kind of objective is written somewhere, and "any foe" too.
        let goals: Vec<Goal> = ZONES
            .iter()
            .flat_map(|z| z.quests.iter().map(|q| q.goal))
            .collect();
        assert!(goals
            .iter()
            .any(|g| matches!(g, Goal::Kill { foe: None, .. })));
        assert!(goals
            .iter()
            .any(|g| matches!(g, Goal::Kill { foe: Some(_), .. })));
        assert!(goals.iter().any(|g| matches!(g, Goal::Collect { .. })));
        assert!(goals.iter().any(|g| matches!(g, Goal::Defend { .. })));
        assert!(goals.iter().any(|g| matches!(g, Goal::SlayElite)));
    }

    #[test]
    fn chain_rewards_are_spread_and_their_unlocks_distinct() {
        let mut kinds: HashMap<&str, usize> = HashMap::new();
        let mut keys = HashSet::new();
        let mut names = HashSet::new();
        for zone in ZONES {
            let r = zone.chain.reward;
            *kinds.entry(r.kind()).or_default() += 1;
            assert!(names.insert(zone.chain.name), "{} twice", zone.chain.name);
            if let Some(key) = r.key() {
                assert!(keys.insert(key.clone()), "{key} twice");
                assert!(!key.is_empty());
            }
            if let Reward::CompanionLook(display) = r {
                assert!(
                    super::super::super::rewards::earned_look(display).is_some(),
                    "{}: look {display} is no companion look",
                    zone.zone
                );
            }
        }
        // The companion keeps every look apart, the earned ones too.
        let looks = super::super::super::rewards::all_looks;
        let distinct: HashSet<&str> = looks().map(|l| l.key).collect();
        assert_eq!(distinct.len(), looks().count());
        // Five kinds over sixteen chains: none left out, none crowding the rest.
        assert_eq!(kinds.len(), 5, "{kinds:?}");
        assert!(kinds.values().all(|&n| (3..=4).contains(&n)), "{kinds:?}");
        assert_eq!(
            Reward::Mount("Brown Horse").key().as_deref(),
            Some("brown_horse")
        );
        assert_eq!(Reward::TalentXp(9).key(), None);
    }

    #[test]
    fn every_display_the_quests_use_is_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let displays: HashMap<u32, u32> = creatures.display_models().collect();
        let models: HashMap<u32, String> = creatures
            .sized_models()
            .map(|(id, path, _)| (id, path.to_ascii_lowercase().replace('/', "\\")))
            .collect();
        let model_of = |display: u32| {
            displays
                .get(&display)
                .and_then(|m| models.get(m))
                .unwrap_or_else(|| panic!("creature display {display} has no model"))
                .clone()
        };
        for zone in ZONES {
            let model = model_of(zone.giver.display);
            // A person or a keeper of the grove: no giver is a beast.
            assert!(
                model.starts_with("character\\") || model.contains("keeperofthegrove"),
                "{}: {} is {model}",
                zone.zone,
                zone.giver.name
            );
            if let Reward::CompanionLook(display) = zone.chain.reward {
                let model = model_of(display);
                assert!(model.starts_with("creature\\"), "{display} is {model}");
            }
        }
        let objects = benilla_formats::load_gameobject_catalog(&mut chain).expect("objects");
        let mut used: Vec<u32> = ItemLook::ALL.iter().map(|l| l.display().0).collect();
        used.extend(super::super::field::SPOT_DISPLAYS);
        for display in used {
            assert!(
                objects.model_path(display).is_some(),
                "GameObjectDisplayInfo has no row {display}"
            );
        }
    }
}
