//! The hero's looks, kept per class beside its gear ([`super::GearSave::looks`]): its name, and the
//! five features the character-create screen offers for each race and sex: skin colour, face,
//! hair style, hair colour, and facial hair or the race's own equivalent (horns, tusks, earrings,
//! markings, features). The counts are the create screen's, read from `CharSections.dbc`,
//! `CharHairGeosets.dbc` and `CharacterFacialHairStyles.dbc` ([`CharCreateCatalog::ranges`]), and
//! the labels are its own `GlueStrings` captions, keyed by the race's `ChrRaces` tokens.

use std::collections::BTreeMap;

use benilla_formats::{CharCreateCatalog, NpcAppearance};
use serde::{Deserialize, Serialize};

use super::super::Rng;
use crate::glue_strings::GlueStrings;

/// How many features a body has: skin colour, face, hair style, hair colour, facial hair.
pub(in crate::survivors) const FEATURES: usize = 5;
/// The facial-hair feature's index, the one a race may not offer.
pub(in crate::survivors) const FACIAL: usize = 4;

/// A hero name's length bounds, the create screen's (`ObjectMgr::CheckPlayerName`).
pub(in crate::survivors) const NAME_MIN: usize = 2;
pub(in crate::survivors) const NAME_MAX: usize = 12;

/// One body's features, as `PLAYER_BYTES` and `CreatureDisplayInfoExtra` both hold them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct Features {
    pub skin: u8,
    pub face: u8,
    pub hair_style: u8,
    pub hair_color: u8,
    pub facial_hair: u8,
}

impl Features {
    pub fn from_array(v: [u8; FEATURES]) -> Self {
        Self {
            skin: v[0],
            face: v[1],
            hair_style: v[2],
            hair_color: v[3],
            facial_hair: v[4],
        }
    }

    pub fn to_array(self) -> [u8; FEATURES] {
        [
            self.skin,
            self.face,
            self.hair_style,
            self.hair_color,
            self.facial_hair,
        ]
    }

    /// An NPC's own features, its class trainer's for a hero.
    pub fn of(npc: &NpcAppearance) -> Self {
        Self {
            skin: npc.skin,
            face: npc.face,
            hair_style: npc.hair_style,
            hair_color: npc.hair_color,
            facial_hair: npc.facial_hair,
        }
    }

    /// Each feature brought into `0..count`; 0 for a feature with no choices.
    pub fn clamped(self, counts: [u8; FEATURES]) -> Self {
        let mut v = self.to_array();
        for (f, &n) in v.iter_mut().zip(&counts) {
            *f = (*f).min(n.saturating_sub(1));
        }
        Self::from_array(v)
    }

    /// Feature `i` one step left (`dir < 0`) or right, wrapping within its count.
    pub fn step(self, i: usize, dir: i8, counts: [u8; FEATURES]) -> Self {
        let mut v = self.clamped(counts).to_array();
        if let (Some(f), Some(&n)) = (v.get_mut(i), counts.get(i)) {
            let n = i32::from(n.max(1));
            *f = (i32::from(*f) + i32::from(dir.signum())).rem_euclid(n) as u8;
        }
        Self::from_array(v)
    }

    /// Every feature at random within its count.
    pub fn random(counts: [u8; FEATURES], rng: &mut Rng) -> Self {
        Self::from_array(counts.map(|n| rng.index(usize::from(n.max(1))) as u8))
    }
}

/// A class hero's looks: its name, and its features per race and sex.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(in crate::survivors) struct HeroLooks {
    /// The hero's name, [`valid_name`]; empty until one is given.
    pub name: String,
    /// The features each body wears, by [`body_key`]. A body not here wears its class trainer's.
    pub features: BTreeMap<String, Features>,
}

impl HeroLooks {
    /// No name and the trainers' features, for a class without a record.
    pub const EMPTY: Self = Self {
        name: String::new(),
        features: BTreeMap::new(),
    };

    /// The hero's name, if it has a valid one.
    pub fn name(&self) -> Option<&str> {
        valid_name(&self.name).then_some(self.name.as_str())
    }

    /// The features picked for the `(race, sex)` body; `None` keeps the trainer's.
    pub fn features(&self, race: u8, sex: u8) -> Option<Features> {
        self.features.get(&body_key(race, sex)).copied()
    }

    pub fn set_features(&mut self, race: u8, sex: u8, features: Features) {
        self.features.insert(body_key(race, sex), features);
    }
}

/// A body's key in [`HeroLooks::features`]: `"<race>-<sex>"`, a bare TOML key.
pub(in crate::survivors) fn body_key(race: u8, sex: u8) -> String {
    format!("{race}-{sex}")
}

/// Whether `name` is a hero name as saved: 2 to 12 ASCII letters, the first upper case and the
/// rest lower, as the game capitalises a character's name.
pub(in crate::survivors) fn valid_name(name: &str) -> bool {
    normalize_name(name).is_some_and(|n| n == name)
}

/// `typed` as a hero name, capitalised like a character's ("aRTHAS" is "Arthas"); `None` unless
/// it is 2 to 12 letters and nothing else.
pub(in crate::survivors) fn normalize_name(typed: &str) -> Option<String> {
    let len = typed.len();
    if !(NAME_MIN..=NAME_MAX).contains(&len) || !typed.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let lower = typed.to_ascii_lowercase();
    Some(lower[..1].to_ascii_uppercase() + &lower[1..])
}

/// A race's name syllables: openings, then male and female endings, all letters only, each
/// opening at most 6 letters and each ending at most 6, so every pair is a valid name.
struct Syllables {
    starts: &'static [&'static str],
    male: &'static [&'static str],
    female: &'static [&'static str],
}

/// Per ChrRaces id 1 to 8, in the flavour of each race's own lore names.
const SYLLABLES: [Syllables; 8] = [
    // Human
    Syllables {
        starts: &[
            "Al", "Bran", "Ced", "Dar", "Ed", "Gar", "Hal", "Jor", "Lor", "Mar", "Os", "Rol",
            "Tor", "Wil",
        ],
        male: &["ric", "win", "mund", "bert", "an", "ard", "ias", "dor"],
        female: &["a", "ela", "wyn", "ine", "issa", "ra", "eth", "ia"],
    },
    // Orc
    Syllables {
        starts: &[
            "Gro", "Thra", "Dur", "Kar", "Gul", "Mog", "Naz", "Ug", "Zug", "Krag", "Rok", "Gar",
        ],
        male: &["ash", "gar", "nak", "thar", "dan", "rok", "mash", "tok"],
        female: &["ka", "gra", "sha", "na", "ra", "za", "gul", "ga"],
    },
    // Dwarf
    Syllables {
        starts: &[
            "Bal", "Bran", "Dur", "Grim", "Mag", "Mur", "Thor", "Hur", "Brom", "Kar", "Dol", "Fal",
        ],
        male: &["din", "dan", "rim", "nar", "grim", "li", "ik", "ar"],
        female: &["na", "ra", "li", "da", "wyn", "dra", "gi", "ka"],
    },
    // Night Elf
    Syllables {
        starts: &[
            "Ty", "Mal", "Ely", "Shan", "Fan", "Ara", "Il", "Ten", "Sil", "Ly", "Ash", "Del",
        ],
        male: &[
            "furion", "dris", "andris", "thas", "anar", "ion", "ael", "oric",
        ],
        female: &["rande", "dris", "ara", "nis", "wen", "elle", "ira", "ana"],
    },
    // Undead
    Syllables {
        starts: &[
            "Mor", "Sylv", "Gor", "Bel", "Var", "Dra", "Nec", "Mal", "Rot", "Gri", "Vor", "Lur",
        ],
        male: &["dred", "mus", "goth", "eth", "ius", "ven", "ock", "rik"],
        female: &["ra", "vane", "tris", "ith", "ess", "wyn", "elle", "is"],
    },
    // Tauren
    Syllables {
        starts: &[
            "Bai", "Ham", "Tah", "Mul", "Kan", "Tor", "Nar", "Hau", "Ska", "Bru", "Mak", "Har",
        ],
        male: &["ne", "uul", "ka", "tor", "mu", "nar", "dan", "rak"],
        female: &["la", "na", "ra", "wa", "ya", "sha", "ka", "ma"],
    },
    // Gnome
    Syllables {
        starts: &[
            "Gel", "Fiz", "Mek", "Tink", "Bin", "Sprock", "Wiz", "Nix", "Pip", "Gim", "Fen", "Mill",
        ],
        male: &["bin", "zle", "kle", "wick", "bolt", "nik", "gle", "dle"],
        female: &["sy", "bit", "kle", "li", "zy", "ny", "pa", "ette"],
    },
    // Troll
    Syllables {
        starts: &[
            "Zul", "Vol", "Sen", "Jin", "Ra", "Zal", "Kaz", "Mak", "Ty", "Ven", "Ro", "Za",
        ],
        male: &["jin", "zan", "tal", "rah", "ji", "kar", "zek", "mun"],
        female: &["ja", "zi", "ra", "la", "ya", "sha", "ti", "ka"],
    },
];

/// A name in race `race`'s flavour for a hero of `sex`: an opening and an ending of its syllables,
/// always [`valid_name`].
pub(in crate::survivors) fn random_name(race: u8, sex: u8, rng: &mut Rng) -> String {
    let s = &SYLLABLES[usize::from(race.clamp(1, 8) - 1)];
    let ends = if sex == 1 { s.female } else { s.male };
    let typed = format!(
        "{}{}",
        s.starts[rng.index(s.starts.len())],
        ends[rng.index(ends.len())]
    );
    normalize_name(&typed).unwrap_or_else(|| "Hero".to_string())
}

/// The five feature counts of a `(race, sex)` body, each at least 1; `None` without the tables.
pub(in crate::survivors) fn feature_ranges(
    cc: Option<&CharCreateCatalog>,
    race: u8,
    sex: u8,
) -> Option<[u8; FEATURES]> {
    cc.and_then(|c| c.ranges(race, sex))
        .map(|r| [r.skin, r.face, r.hair_style, r.hair_color, r.facial_hair].map(|n| n.max(1)))
}

/// [`feature_ranges`], one choice each without the tables.
pub(in crate::survivors) fn feature_counts(
    cc: Option<&CharCreateCatalog>,
    race: u8,
    sex: u8,
) -> [u8; FEATURES] {
    feature_ranges(cc, race, sex).unwrap_or([1; FEATURES])
}

/// Whether the body has a facial-hair feature at all: the create screen hides the dial when the
/// race's token is `NONE`.
pub(in crate::survivors) fn has_facial(cc: Option<&CharCreateCatalog>, race: u8, sex: u8) -> bool {
    cc.and_then(|c| c.facial_hair_customization(race, sex))
        .is_none_or(|t| !t.eq_ignore_ascii_case("NONE"))
}

/// The five features' captions for a `(race, sex)` body, as the create screen labels its dials:
/// `CHAR_CUSTOMIZATION1_DESC` and `2_DESC`, then `HAIR_<token>_STYLE` / `_COLOR` and
/// `FACIAL_HAIR_<token>` by the race's `ChrRaces` tokens ("Horns", "Tusks", "Markings").
pub(in crate::survivors) fn feature_labels(
    strings: Option<&GlueStrings>,
    cc: Option<&CharCreateCatalog>,
    race: u8,
    sex: u8,
) -> [String; FEATURES] {
    let hair = cc
        .and_then(|c| c.hair_customization(race))
        .unwrap_or("NORMAL")
        .to_ascii_uppercase();
    let facial = cc
        .and_then(|c| c.facial_hair_customization(race, sex))
        .unwrap_or("NORMAL")
        .to_ascii_uppercase();
    // Without the strings, the token in words: "HORNS" is "Horns".
    let word = |token: &str| {
        let lower = token.to_ascii_lowercase();
        let mut chars = lower.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_ascii_uppercase().to_string() + chars.as_str()
        })
    };
    let (hair_style, hair_color) = if hair == "NORMAL" {
        ("Hair Style".to_string(), "Hair Color".to_string())
    } else {
        let w = word(&hair);
        (format!("{w} Style"), format!("{w} Color"))
    };
    let facial_hair = if facial == "NORMAL" {
        "Facial Hair".to_string()
    } else {
        word(&facial)
    };
    let text = |key: &str, fallback: &str| {
        strings
            .and_then(|s| s.get(key))
            .unwrap_or(fallback)
            .to_string()
    };
    [
        text("CHAR_CUSTOMIZATION1_DESC", "Skin Color"),
        text("CHAR_CUSTOMIZATION2_DESC", "Face"),
        text(&format!("HAIR_{hair}_STYLE"), &hair_style),
        text(&format!("HAIR_{hair}_COLOR"), &hair_color),
        text(&format!("FACIAL_HAIR_{facial}"), &facial_hair),
    ]
}

/// The features a `(race, sex)` hero wears: those picked for the body, else its class trainer's
/// (`trainer`), each within the body's counts ([`feature_ranges`]) when the tables are in.
pub(in crate::survivors) fn worn_features(
    looks: &HeroLooks,
    race: u8,
    sex: u8,
    trainer: Option<&NpcAppearance>,
    counts: Option<[u8; FEATURES]>,
) -> Features {
    let f = looks
        .features(race, sex)
        .or_else(|| trainer.map(Features::of))
        .unwrap_or_default();
    counts.map_or(f, |n| f.clamped(n))
}

/// The results screen's title: "Thrall the Orc Shaman in Durotar", or without a name "Orc Shaman
/// in Durotar".
pub(in crate::survivors) fn results_title(
    name: Option<&str>,
    race: &str,
    class: &str,
    zone: &str,
) -> String {
    match name {
        Some(name) => format!("{name} the {race} {class} in {zone}"),
        None => format!("{race} {class} in {zone}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_letters_two_to_twelve_and_capitalised() {
        assert_eq!(normalize_name("thrall").as_deref(), Some("Thrall"));
        assert_eq!(normalize_name("aRTHAS").as_deref(), Some("Arthas"));
        assert_eq!(normalize_name("Jo").as_deref(), Some("Jo"));
        assert_eq!(
            normalize_name("Abcdefghijkl").as_deref(),
            Some("Abcdefghijkl")
        );
        for bad in [
            "",
            "A",
            "Abcdefghijklm",
            "Thrall2",
            "Jaina Proudmoore",
            "Zul'jin",
            "Ünal",
        ] {
            assert_eq!(normalize_name(bad), None, "{bad:?}");
            assert!(!valid_name(bad), "{bad:?}");
        }
        assert!(valid_name("Thrall"));
        assert!(!valid_name("thrall"), "saved names are capitalised");
        assert!(!valid_name("ThRall"));
        let looks = HeroLooks {
            name: "bad name".into(),
            ..HeroLooks::default()
        };
        assert_eq!(looks.name(), None);
    }

    #[test]
    fn every_race_names_validly_in_both_sexes() {
        for (r, s) in SYLLABLES.iter().enumerate() {
            for part in s.starts.iter().chain(s.male).chain(s.female) {
                assert!(
                    !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphabetic()),
                    "race {} syllable {part:?}",
                    r + 1
                );
            }
            // Every pairing, not just the ones a seed happens to draw.
            for start in s.starts {
                for end in s.male.iter().chain(s.female) {
                    let name = normalize_name(&format!("{start}{end}"));
                    assert!(name.is_some(), "race {} {start}{end}", r + 1);
                }
            }
        }
        let mut rng = Rng(0x5EED);
        for race in 1..=8u8 {
            for sex in 0..2u8 {
                let names: Vec<String> =
                    (0..64).map(|_| random_name(race, sex, &mut rng)).collect();
                for n in &names {
                    assert!(valid_name(n), "race {race} sex {sex}: {n:?}");
                }
                let distinct: std::collections::HashSet<_> = names.iter().collect();
                assert!(distinct.len() > 16, "race {race} sex {sex} repeats itself");
            }
        }
    }

    #[test]
    fn features_step_wrap_clamp_and_randomise_within_their_counts() {
        let counts = [10, 5, 1, 8, 3];
        let f = Features::from_array([9, 0, 0, 7, 2]);
        assert_eq!(f.step(0, 1, counts).skin, 0, "wraps right");
        assert_eq!(f.step(1, -1, counts).face, 4, "wraps left");
        assert_eq!(f.step(2, 1, counts).hair_style, 0, "a single choice stays");
        assert_eq!(f.step(4, -1, counts).facial_hair, 1);
        assert_eq!(
            Features::from_array([20, 20, 20, 20, 20]).clamped(counts),
            Features::from_array([9, 4, 0, 7, 2])
        );
        let mut rng = Rng(99);
        for _ in 0..200 {
            let r = Features::random(counts, &mut rng).to_array();
            assert!(r.iter().zip(&counts).all(|(v, n)| v < n), "{r:?}");
        }
    }

    #[test]
    fn a_body_wears_its_pick_else_the_trainers_features() {
        let trainer = NpcAppearance {
            race: 2,
            sex: 0,
            skin: 3,
            face: 4,
            hair_style: 12,
            hair_color: 1,
            facial_hair: 2,
            equipment: [0; 10],
            bake_name: Some("x.blp".into()),
        };
        let counts = Some([9, 9, 7, 9, 9]);
        let mut looks = HeroLooks::default();
        assert_eq!(
            worn_features(&looks, 2, 0, Some(&trainer), counts),
            Features::from_array([3, 4, 6, 1, 2]),
            "the trainer's, clamped"
        );
        assert_eq!(
            worn_features(&looks, 2, 0, Some(&trainer), None),
            Features::from_array([3, 4, 12, 1, 2]),
            "no tables, no clamp"
        );
        assert_eq!(
            worn_features(&looks, 2, 0, None, counts),
            Features::default()
        );
        let mine = Features::from_array([1, 2, 3, 4, 5]);
        looks.set_features(2, 0, mine);
        assert_eq!(worn_features(&looks, 2, 0, Some(&trainer), counts), mine);
        // Another body of the class keeps its own.
        assert_eq!(looks.features(2, 1), None);
    }

    #[test]
    fn looks_round_trip_and_old_saves_read() {
        let mut looks = HeroLooks {
            name: "Thrall".into(),
            ..HeroLooks::default()
        };
        looks.set_features(2, 0, Features::from_array([1, 2, 3, 4, 5]));
        looks.set_features(8, 1, Features::from_array([0, 1, 0, 1, 0]));
        let text = toml::to_string_pretty(&looks).unwrap();
        assert!(text.contains("2-0"), "{text}");
        assert_eq!(toml::from_str::<HeroLooks>(&text).unwrap(), looks);
        assert_eq!(toml::from_str::<HeroLooks>("").unwrap(), HeroLooks::EMPTY);
        // A partial body record keeps its other features at 0.
        let part: HeroLooks = toml::from_str("[features.1-0]\nskin = 4\n").unwrap();
        assert_eq!(
            part.features(1, 0),
            Some(Features::from_array([4, 0, 0, 0, 0]))
        );
        assert_eq!(part.name(), None);
    }

    #[test]
    fn labels_follow_the_race_tokens_without_the_strings() {
        let labels = feature_labels(None, None, 1, 0);
        assert_eq!(
            labels,
            [
                "Skin Color",
                "Face",
                "Hair Style",
                "Hair Color",
                "Facial Hair"
            ]
            .map(String::from)
        );
        let strings = GlueStrings::from_map(
            [("FACIAL_HAIR_NORMAL".to_string(), "Beard".to_string())]
                .into_iter()
                .collect(),
        );
        assert_eq!(feature_labels(Some(&strings), None, 3, 0)[FACIAL], "Beard");
    }

    #[test]
    fn the_results_title_names_the_hero() {
        assert_eq!(
            results_title(Some("Thrall"), "Orc", "Shaman", "Durotar"),
            "Thrall the Orc Shaman in Durotar"
        );
        assert_eq!(
            results_title(None, "Orc", "Shaman", "Durotar"),
            "Orc Shaman in Durotar"
        );
    }
}
