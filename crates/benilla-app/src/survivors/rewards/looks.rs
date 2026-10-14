//! The companion's looks: real 1.12 vanity pets, each checked against the client's
//! `CreatureDisplayInfo.dbc` and `CreatureModelData.dbc` (the model and skin are named beside each).
//! Some are worn from the start; the rest open as the pet levels ([`super::pet`]), as an
//! achievement is earned ([`super::super::achievements`]), or as a quest chain awards them
//! ([`super::super::quests`], kept in [`RewardsSave::unlocked_looks`]).

use super::super::achievements::AchievementSave;
use super::pet;
use super::save::RewardsSave;

/// What opens a look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Unlock {
    /// Worn from the start.
    Start,
    /// The pet reaching this level.
    PetLevel(u32),
    /// Earning this achievement, by its id: the table of achievement rewards lives here, so the
    /// achievements' catalog stays as it is.
    Achievement(&'static str),
    /// A quest chain's reward, kept by display in [`RewardsSave::unlocked_looks`].
    Quest,
}

/// One look the companion can wear.
pub(in crate::survivors) struct Look {
    /// The key a save stores.
    pub(in crate::survivors) key: &'static str,
    pub(in crate::survivors) name: &'static str,
    /// The `CreatureDisplayInfo.dbc` row.
    pub(in crate::survivors) display: u32,
    /// `OBJECT_FIELD_SCALE_X`, which alone sizes a unit: about 1.8 times the display's own
    /// scale, so the critter reads under the raised camera.
    pub(in crate::survivors) scale: f32,
    pub(in crate::survivors) unlock: Unlock,
}

const fn look(
    key: &'static str,
    name: &'static str,
    display: u32,
    scale: f32,
    unlock: Unlock,
) -> Look {
    Look {
        key,
        name,
        display,
        scale,
        unlock,
    }
}

/// The looks bought with the perk or earned with the pet and the achievements, in the Companion
/// tab's order: the starting ones, then by pet level, then the achievements'. Each names its model
/// and skin; the scale is the display's own times about 1.8.
pub(in crate::survivors) static LOOKS: &[Look] = &[
    // `Creature\Squirrel\Squirrel.mdx`, display scale 1.3.
    look("squirrel", "Squirrel", 134, 2.3, Unlock::Start),
    // `Creature\Rabbit\Rabbit.mdx`, `RabbitSkin`.
    look("rabbit", "Rabbit", 328, 1.8, Unlock::Start),
    // `Creature\Frog\Frog.mdx`, `FrogBrownSpots`.
    look("frog", "Frog", 901, 1.8, Unlock::Start),
    // `Creature\Cat\Cat.mdx`, `OrangeTabby`, display scale 0.5: the Orange Tabby's carrier cat.
    look("cat", "Orange Tabby", 5554, 0.9, Unlock::Start),
    // `Creature\Chicken\Chicken.mdx`, `ChickenSkin`.
    look("chicken", "Chicken", 304, 1.8, Unlock::Start),
    // `Creature\PrarieDog\PrarieDog.mdx` (sic).
    look("prairie_dog", "Prairie Dog", 1072, 1.8, Unlock::Start),
    // `Creature\Rabbit\Rabbit.mdx`, the vanity pets' rabbit row.
    look(
        "snowshoe_rabbit",
        "Snowshoe Rabbit",
        6302,
        1.8,
        Unlock::Start,
    ),
    // `Creature\Frog\Frog.mdx`, `FrogGreen`.
    look("wood_frog", "Wood Frog", 6295, 1.8, Unlock::Start),
    // `Creature\snake\snake.mdx`, `SnakeSkinBrown`.
    look("brown_snake", "Brown Snake", 2957, 1.8, Unlock::Start),
    // `Creature\Cat\Cat.mdx`, `Birman`, display scale 0.5: the colour-pointed Siamese.
    look("siamese", "Siamese", 5585, 0.9, Unlock::PetLevel(3)),
    // `Creature\Parrot\Parrot.mdx`, `ParrotSkinBlue`, display scale 0.25.
    look(
        "hyacinth_macaw",
        "Hyacinth Macaw",
        6192,
        0.45,
        Unlock::PetLevel(6),
    ),
    // `Creature\Squirrel\Squirrel.mdx`, `squirrel_Robo`, display scale 1.3.
    look(
        "mechanical_squirrel",
        "Mechanical Squirrel",
        7937,
        2.3,
        Unlock::PetLevel(9),
    ),
    // `Creature\DragonWhelp\DragonWhelp.mdx`, `DragonWhelpSkinBlue`, display scale 0.5.
    look(
        "azure_whelpling",
        "Azure Whelpling",
        6293,
        0.9,
        Unlock::PetLevel(12),
    ),
    // `Creature\FaerieDragon\FaerieDragon.mdx`, display scale 0.3.
    look(
        "sprite_darter",
        "Sprite Darter",
        6294,
        0.55,
        Unlock::PetLevel(15),
    ),
    // `Creature\DireWolf\DireWolf.mdx`, `DireWolfSkinDarkBlack`, display scale 0.4.
    look("worg_pup", "Worg Pup", 9563, 0.7, Unlock::PetLevel(18)),
    // `Creature\panda\PandaCub.mdx`, display scale 0.25.
    look(
        "panda_cub",
        "Panda Cub",
        10990,
        0.45,
        Unlock::Achievement("wins_5"),
    ),
    // `Creature\zergling\ZerglingPet.mdx`, display scale 0.25.
    look(
        "zergling",
        "Zergling",
        10993,
        0.45,
        Unlock::Achievement("kills_10000"),
    ),
    // `Creature\diablo\DiabloFunSized.mdx`, display scale 0.25.
    look(
        "mini_diablo",
        "Mini Diablo",
        10992,
        0.45,
        Unlock::Achievement("bosses_100"),
    ),
];

/// Looks the quest chains award ([`crate::survivors::quests`]), offered once their display is in
/// [`RewardsSave::unlocked_looks`]. Checked as [`LOOKS`] are.
pub(in crate::survivors) static EARNED_LOOKS: &[Look] = &[
    // `Creature\snake\snake.mdx`, black, the Black Kingsnake's.
    look(
        "black_kingsnake",
        "Black Kingsnake",
        1206,
        1.8,
        Unlock::Quest,
    ),
    // `Creature\cockroach\Cockroach.mdx`.
    look(
        "undercity_cockroach",
        "Undercity Cockroach",
        2177,
        2.4,
        Unlock::Quest,
    ),
    // `Creature\DragonWhelp\DragonWhelp.mdx`, green, display scale 0.5.
    look(
        "emerald_whelpling",
        "Emerald Whelpling",
        6291,
        0.9,
        Unlock::Quest,
    ),
];

/// Every look, [`LOOKS`] then [`EARNED_LOOKS`]: what a look pick indexes.
pub(in crate::survivors) fn all_looks() -> impl Iterator<Item = &'static Look> {
    LOOKS.iter().chain(EARNED_LOOKS)
}

/// Whether `save` may dress a companion in `look`, with the achievements `earned`: a starting
/// look always, the rest once what unlocks them is held.
pub(in crate::survivors) fn offered(
    look: &Look,
    save: &RewardsSave,
    earned: &AchievementSave,
) -> bool {
    match look.unlock {
        Unlock::Start => true,
        Unlock::PetLevel(level) => pet::level(save.pet_xp) >= level,
        Unlock::Achievement(id) => earned.has(id),
        Unlock::Quest => save.unlocked_looks.contains(&look.display),
    }
}

/// The earned look wearing `display`.
pub(in crate::survivors) fn earned_look(display: u32) -> Option<&'static Look> {
    EARNED_LOOKS.iter().find(|l| l.display == display)
}

/// The look `key` names; empty or unknown is the first.
pub(in crate::survivors) fn look_for(key: &str) -> &'static Look {
    all_looks().find(|l| l.key == key).unwrap_or(&LOOKS[0])
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::survivors::achievements::catalog;

    #[test]
    fn looks_have_their_own_keys_and_displays_and_a_default() {
        let keys: HashSet<&str> = all_looks().map(|l| l.key).collect();
        assert_eq!(keys.len(), all_looks().count());
        let displays: HashSet<u32> = all_looks().map(|l| l.display).collect();
        assert_eq!(
            displays.len(),
            all_looks().count(),
            "no two wear one display"
        );
        assert_eq!(look_for("").key, LOOKS[0].key);
        assert_eq!(look_for("dragon").key, LOOKS[0].key);
        assert_eq!(look_for("frog").display, 901);
        assert_eq!(look_for("emerald_whelpling").display, 6291);
        assert!(all_looks().all(|l| l.scale > 0.0 && !l.name.is_empty()));
    }

    #[test]
    fn every_unlock_source_is_where_it_should_be() {
        let start = LOOKS.iter().filter(|l| l.unlock == Unlock::Start).count();
        assert_eq!(start, 9, "the first five and four more");
        // One look at each of these pet levels, and at no other.
        let levels: Vec<u32> = LOOKS
            .iter()
            .filter_map(|l| match l.unlock {
                Unlock::PetLevel(n) => Some(n),
                _ => None,
            })
            .collect();
        assert_eq!(levels, [3, 6, 9, 12, 15, 18]);
        // Three achievements' rewards, each a real achievement.
        let ids: Vec<&str> = LOOKS
            .iter()
            .filter_map(|l| match l.unlock {
                Unlock::Achievement(id) => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 3);
        for id in ids {
            assert!(
                catalog().iter().any(|a| a.id == id),
                "{id} is no achievement"
            );
        }
        // The quest chains' looks are all and only the earned ones.
        assert!(LOOKS.iter().all(|l| l.unlock != Unlock::Quest));
        assert!(EARNED_LOOKS.iter().all(|l| l.unlock == Unlock::Quest));
    }

    #[test]
    fn each_source_offers_its_look_once_held() {
        let mut save = RewardsSave::default();
        let mut earned = AchievementSave::default();
        let offered_keys = |save: &RewardsSave, earned: &AchievementSave| -> Vec<&str> {
            all_looks()
                .filter(|l| offered(l, save, earned))
                .map(|l| l.key)
                .collect()
        };
        assert_eq!(offered_keys(&save, &earned).len(), 9, "the starting looks");
        assert!(offered(look_for("frog"), &save, &earned));
        assert!(!offered(look_for("siamese"), &save, &earned));
        // Pet levels open theirs as they pass.
        save.pet_xp = pet::xp_for_level(3) - 1;
        assert!(!offered(look_for("siamese"), &save, &earned));
        save.pet_xp = pet::xp_for_level(3);
        assert!(offered(look_for("siamese"), &save, &earned));
        assert!(!offered(look_for("hyacinth_macaw"), &save, &earned));
        save.pet_xp = pet::xp_for_level(pet::MAX_LEVEL);
        assert_eq!(offered_keys(&save, &earned).len(), 15);
        // An achievement opens its own.
        assert!(!offered(look_for("mini_diablo"), &save, &earned));
        earned.earned.insert("bosses_100".into(), 40);
        assert!(offered(look_for("mini_diablo"), &save, &earned));
        assert!(!offered(look_for("zergling"), &save, &earned));
        // A quest chain's, by its display.
        assert!(!offered(look_for("black_kingsnake"), &save, &earned));
        save.unlocked_looks.insert(1206);
        assert!(offered(look_for("black_kingsnake"), &save, &earned));
        assert_eq!(earned_look(1206).map(|l| l.key), Some("black_kingsnake"));
        assert!(earned_look(134).is_none(), "a starting look is not earned");
    }

    #[test]
    fn looks_are_the_named_critters_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let catalog = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        // Each look's model, and its skin where the model wears several.
        let want: HashMap<&str, (&str, &str)> = [
            ("squirrel", ("creature\\squirrel\\squirrel.", "squirrel")),
            ("rabbit", ("creature\\rabbit\\rabbit.", "rabbitskin")),
            ("frog", ("creature\\frog\\frog.", "frogbrownspots")),
            ("cat", ("creature\\cat\\cat.", "orangetabby")),
            ("chicken", ("creature\\chicken\\chicken.", "chickenskin")),
            ("prairie_dog", ("creature\\prariedog\\prariedog.", "")),
            (
                "snowshoe_rabbit",
                ("creature\\rabbit\\rabbit.", "rabbitskin"),
            ),
            ("wood_frog", ("creature\\frog\\frog.", "froggreen")),
            ("brown_snake", ("creature\\snake\\snake.", "snakeskinbrown")),
            ("siamese", ("creature\\cat\\cat.", "birman")),
            (
                "hyacinth_macaw",
                ("creature\\parrot\\parrot.", "parrotskinblue"),
            ),
            (
                "mechanical_squirrel",
                ("creature\\squirrel\\squirrel.", "squirrel_robo"),
            ),
            (
                "azure_whelpling",
                ("creature\\dragonwhelp\\dragonwhelp.", "dragonwhelpskinblue"),
            ),
            (
                "sprite_darter",
                ("creature\\faeriedragon\\faeriedragon.", ""),
            ),
            (
                "worg_pup",
                ("creature\\direwolf\\direwolf.", "direwolfskindarkblack"),
            ),
            ("panda_cub", ("creature\\panda\\pandacub.", "")),
            ("zergling", ("creature\\zergling\\zerglingpet.", "")),
            ("mini_diablo", ("creature\\diablo\\diablofunsized.", "")),
            (
                "black_kingsnake",
                ("creature\\snake\\snake.", "snakeskinblack"),
            ),
            (
                "undercity_cockroach",
                ("creature\\cockroach\\cockroach.", ""),
            ),
            (
                "emerald_whelpling",
                (
                    "creature\\dragonwhelp\\dragonwhelp.",
                    "dragonwhelpskingreen",
                ),
            ),
        ]
        .into_iter()
        .collect();
        assert_eq!(want.len(), all_looks().count(), "every look is checked");
        for look in all_looks() {
            let (model, skin) = want[look.key];
            let found = catalog
                .model(look.display)
                .unwrap_or_else(|| panic!("{}: display {} has no model", look.key, look.display));
            let path = found.model_path.to_ascii_lowercase().replace('/', "\\");
            assert!(
                path.starts_with(model),
                "{}: display {} is {path}",
                look.key,
                look.display
            );
            let skins: Vec<String> = found
                .textures
                .iter()
                .flatten()
                .map(|t| t.to_ascii_lowercase())
                .collect();
            assert!(
                skin.is_empty() || skins.iter().any(|t| t.ends_with(skin)),
                "{}: display {} wears {skins:?}",
                look.key,
                look.display
            );
        }
    }
}
