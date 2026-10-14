//! The shared vocabulary for permanent stat bonuses: what a talent, a piece of gear or any other
//! meta-progression source grants ([`Bonus`]), and their summed totals ([`Bonuses`]). Sources are
//! static slices built with the `const fn` constructors; the run folds the totals into its stats.

/// Most armor can remove of incoming damage.
pub(super) const ARMOR_CAP: f32 = 0.60;
/// Most cooldown reduction can remove of a cooldown.
pub(super) const COOLDOWN_CAP: f32 = 0.50;

/// A stat a bonus raises. The serde names are what saves store, so renaming a variant breaks them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum StatKey {
    /// Flat hit points.
    MaxHealth,
    /// Fraction added to the damage multiplier (0.10 = +10%).
    Damage,
    /// Fraction of cooldowns removed (0.09 = 9% shorter); the total is capped at [`COOLDOWN_CAP`].
    Cooldown,
    /// Fraction added to crit chance (0.05 = +5 points).
    CritChance,
    /// Fraction added to the critical damage multiplier (0.10 = +10%).
    CritDamage,
    /// Fraction added to the area multiplier (0.10 = +10%).
    Area,
    /// Flat extra targets or projectiles, in whole numbers.
    Targets,
    /// Fraction added to the movement speed multiplier (0.10 = +10%).
    MoveSpeed,
    /// Flat health per second.
    Regen,
    /// Fraction added to the pickup range multiplier (0.10 = +10%).
    Pickup,
    /// Fraction of incoming damage removed; the total is capped at [`ARMOR_CAP`].
    Armor,
    /// Fraction added to the experience multiplier (0.10 = +10%).
    Experience,
    /// Fraction added to the gold multiplier (0.10 = +10%).
    Gold,
    /// Fraction added to the luck multiplier (0.10 = +10%).
    Luck,
    /// Fraction of damage dealt returned as health.
    Lifesteal,
    /// Flat extra lives.
    Revives,
}

impl StatKey {
    const COUNT: usize = StatKey::Revives as usize + 1;

    /// The stat's tooltip name, plural where it counts something.
    #[allow(dead_code)] // Tooltip text for the talent and gear screens, still to land.
    pub fn label(self) -> &'static str {
        match self {
            StatKey::MaxHealth => "max health",
            StatKey::Damage => "damage",
            StatKey::Cooldown => "cooldowns",
            StatKey::CritChance => "critical strike",
            StatKey::CritDamage => "critical damage",
            StatKey::Area => "area of effect",
            StatKey::Targets => "targets",
            StatKey::MoveSpeed => "movement speed",
            StatKey::Regen => "health per second",
            StatKey::Pickup => "pickup range",
            StatKey::Armor => "damage reduction",
            StatKey::Experience => "experience",
            StatKey::Gold => "gold",
            StatKey::Luck => "luck",
            StatKey::Lifesteal => "lifesteal",
            StatKey::Revives => "extra lives",
        }
    }

    /// Whether the value is a flat amount rather than a fraction shown as a percentage.
    #[allow(dead_code)] // Tooltip text for the talent and gear screens, still to land.
    fn flat(self) -> bool {
        matches!(
            self,
            StatKey::MaxHealth | StatKey::Targets | StatKey::Regen | StatKey::Revives
        )
    }
}

/// A spell school, for damage bonuses scoped to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum School {
    Physical,
    Holy,
    Fire,
    Nature,
    Frost,
    Shadow,
    Arcane,
}

impl School {
    const COUNT: usize = School::Arcane as usize + 1;

    /// The school's display name, capitalised as in tooltips.
    #[allow(dead_code)] // Tooltip text for the talent and gear screens, still to land.
    pub fn label(self) -> &'static str {
        match self {
            School::Physical => "Physical",
            School::Holy => "Holy",
            School::Fire => "Fire",
            School::Nature => "Nature",
            School::Frost => "Frost",
            School::Shadow => "Shadow",
            School::Arcane => "Arcane",
        }
    }
}

/// One stat raised by `value`, in the units [`StatKey`] documents.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bonus {
    pub stat: StatKey,
    pub value: f32,
    /// Scopes a [`StatKey::Damage`] bonus to one school; ignored for every other stat.
    pub school: Option<School>,
}

impl Bonus {
    pub const fn new(stat: StatKey, value: f32) -> Self {
        Self {
            stat,
            value,
            school: None,
        }
    }

    /// Damage scoped to `school`.
    pub const fn school(school: School, value: f32) -> Self {
        Self {
            stat: StatKey::Damage,
            value,
            school: Some(school),
        }
    }

    /// The school a damage bonus is scoped to, if it is one.
    fn damage_school(&self) -> Option<School> {
        self.school.filter(|_| self.stat == StatKey::Damage)
    }

    /// Short tooltip text: "+10% damage", "+5% Fire damage", "+126 max health", "-9% cooldowns".
    #[allow(dead_code)] // Tooltip text for the talent and gear screens, still to land.
    pub fn describe(&self) -> String {
        // Cooldown reduction shortens cooldowns, so it reads as a minus.
        let shown = if self.stat == StatKey::Cooldown {
            -self.value
        } else {
            self.value
        };
        let sign = if shown < 0.0 { '-' } else { '+' };
        let size = shown.abs();
        if self.stat.flat() {
            let amount = tenths(size);
            let label = match self.stat {
                StatKey::Targets if amount == "1" => "target",
                StatKey::Revives if amount == "1" => "extra life",
                stat => stat.label(),
            };
            return format!("{sign}{amount} {label}");
        }
        let percent = size * 100.0;
        let amount = if percent < 1.0 {
            tenths(percent)
        } else {
            percent.round().to_string()
        };
        match self.damage_school() {
            Some(school) => format!("{sign}{amount}% {} damage", school.label()),
            None => format!("{sign}{amount}% {}", self.stat.label()),
        }
    }
}

/// `v` to one decimal place, without the decimal when it rounds to a whole number.
#[allow(dead_code)] // Tooltip text for the talent and gear screens, still to land.
fn tenths(v: f32) -> String {
    let t = (v * 10.0).round() / 10.0;
    if t.fract() == 0.0 {
        t.to_string()
    } else {
        format!("{t:.1}")
    }
}

/// Bonuses summed per stat, with school-scoped damage kept apart from unscoped damage.
#[derive(Default, Clone, Debug, PartialEq)]
pub(super) struct Bonuses {
    totals: [f32; StatKey::COUNT],
    school_damage: [f32; School::COUNT],
}

impl Bonuses {
    pub fn add(&mut self, bonus: &Bonus) {
        match bonus.damage_school() {
            Some(school) => self.school_damage[school as usize] += bonus.value,
            None => self.totals[bonus.stat as usize] += bonus.value,
        }
    }

    pub fn add_all(&mut self, bonuses: &[Bonus]) {
        for bonus in bonuses {
            self.add(bonus);
        }
    }

    pub fn merge(&mut self, other: &Bonuses) {
        for (total, more) in self.totals.iter_mut().zip(other.totals) {
            *total += more;
        }
        for (total, more) in self.school_damage.iter_mut().zip(other.school_damage) {
            *total += more;
        }
    }

    /// The unscoped total for `stat`, uncapped.
    pub fn get(&self, stat: StatKey) -> f32 {
        self.totals[stat as usize]
    }

    /// Damage scoped to `school` only; unscoped damage is [`Bonuses::get`]`(StatKey::Damage)`.
    pub fn school_damage(&self, school: School) -> f32 {
        self.school_damage[school as usize]
    }

    /// Armor, capped at [`ARMOR_CAP`].
    pub fn armor(&self) -> f32 {
        self.get(StatKey::Armor).min(ARMOR_CAP)
    }

    /// Cooldown reduction, capped at [`COOLDOWN_CAP`].
    pub fn cooldown(&self) -> f32 {
        self.get(StatKey::Cooldown).min(COOLDOWN_CAP)
    }
}

/// Reads comma-separated `name=value` entries, as the `WOW_SURVIVORS_BONUSES` test aid writes
/// them: a stat by its [`StatKey`] name (`crit_chance=0.2`), or a school's name for that school's
/// damage (`fire=0.3`). Returns the sum and the entries it could not read.
pub(super) fn parse(text: &str) -> (Bonuses, Vec<&str>) {
    let mut out = Bonuses::default();
    let mut skipped = Vec::new();
    for entry in text.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        match parse_entry(entry) {
            Some(bonus) => out.add(&bonus),
            None => skipped.push(entry),
        }
    }
    (out, skipped)
}

fn parse_entry(entry: &str) -> Option<Bonus> {
    let (name, value) = entry.split_once('=')?;
    let value = value.trim().parse::<f32>().ok().filter(|v| v.is_finite())?;
    let name = name.trim();
    match by_name::<StatKey>(name) {
        Some(stat) => Some(Bonus::new(stat, value)),
        None => by_name::<School>(name).map(|school| Bonus::school(school, value)),
    }
}

/// A unit variant by its serde name, so the names read here are the ones saves store.
fn by_name<'a, T: serde::Deserialize<'a>>(name: &'a str) -> Option<T> {
    use serde::de::IntoDeserializer;
    let de: serde::de::value::StrDeserializer<'a, serde::de::value::Error> =
        name.into_deserializer();
    T::deserialize(de).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_STATS: [StatKey; StatKey::COUNT] = [
        StatKey::MaxHealth,
        StatKey::Damage,
        StatKey::Cooldown,
        StatKey::CritChance,
        StatKey::CritDamage,
        StatKey::Area,
        StatKey::Targets,
        StatKey::MoveSpeed,
        StatKey::Regen,
        StatKey::Pickup,
        StatKey::Armor,
        StatKey::Experience,
        StatKey::Gold,
        StatKey::Luck,
        StatKey::Lifesteal,
        StatKey::Revives,
    ];

    // Written the way data authors will write theirs.
    static TALENT: &[Bonus] = &[
        Bonus::new(StatKey::Damage, 0.05),
        Bonus::school(School::Fire, 0.10),
        Bonus::new(StatKey::MaxHealth, 20.0),
    ];

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn add_sums_per_stat() {
        let mut b = Bonuses::default();
        b.add(&Bonus::new(StatKey::Damage, 0.10));
        b.add(&Bonus::new(StatKey::Damage, 0.05));
        b.add(&Bonus::new(StatKey::Regen, 1.5));
        assert!(close(b.get(StatKey::Damage), 0.15));
        assert!(close(b.get(StatKey::Regen), 1.5));
        assert_eq!(b.get(StatKey::Luck), 0.0);
    }

    #[test]
    fn school_damage_is_kept_apart_from_unscoped() {
        let mut b = Bonuses::default();
        b.add_all(TALENT);
        assert!(close(b.get(StatKey::Damage), 0.05));
        assert!(close(b.school_damage(School::Fire), 0.10));
        assert_eq!(b.school_damage(School::Frost), 0.0);
        assert!(close(b.get(StatKey::MaxHealth), 20.0));
    }

    #[test]
    fn school_on_other_stats_is_ignored() {
        let mut b = Bonuses::default();
        b.add(&Bonus {
            stat: StatKey::Area,
            value: 0.2,
            school: Some(School::Fire),
        });
        assert!(close(b.get(StatKey::Area), 0.2));
        assert_eq!(b.school_damage(School::Fire), 0.0);
    }

    #[test]
    fn merge_sums_both_tables() {
        let mut a = Bonuses::default();
        a.add_all(TALENT);
        let mut b = Bonuses::default();
        b.add_all(&[
            Bonus::new(StatKey::Damage, 0.10),
            Bonus::school(School::Fire, 0.02),
            Bonus::school(School::Shadow, 0.03),
            Bonus::new(StatKey::Revives, 1.0),
        ]);
        a.merge(&b);
        assert!(close(a.get(StatKey::Damage), 0.15));
        assert!(close(a.school_damage(School::Fire), 0.12));
        assert!(close(a.school_damage(School::Shadow), 0.03));
        assert!(close(a.get(StatKey::MaxHealth), 20.0));
        assert!(close(a.get(StatKey::Revives), 1.0));
        let mut twice = Bonuses::default();
        twice.add_all(TALENT);
        twice.add_all(TALENT);
        let mut merged = Bonuses::default();
        merged.add_all(TALENT);
        let copy = merged.clone();
        merged.merge(&copy);
        assert_eq!(twice, merged);
    }

    #[test]
    fn armor_and_cooldown_are_capped() {
        let mut b = Bonuses::default();
        b.add_all(&[
            Bonus::new(StatKey::Armor, 0.45),
            Bonus::new(StatKey::Armor, 0.30),
            Bonus::new(StatKey::Cooldown, 0.40),
            Bonus::new(StatKey::Cooldown, 0.20),
        ]);
        assert_eq!(b.armor(), ARMOR_CAP);
        assert_eq!(b.cooldown(), COOLDOWN_CAP);
        // The raw totals stay uncapped.
        assert!(close(b.get(StatKey::Armor), 0.75));
        assert!(close(b.get(StatKey::Cooldown), 0.60));

        let mut low = Bonuses::default();
        low.add(&Bonus::new(StatKey::Armor, 0.10));
        low.add(&Bonus::new(StatKey::Cooldown, 0.09));
        assert!(close(low.armor(), 0.10));
        assert!(close(low.cooldown(), 0.09));
    }

    #[test]
    fn describe_percentages() {
        let d = |stat, v| Bonus::new(stat, v).describe();
        assert_eq!(d(StatKey::Damage, 0.10), "+10% damage");
        assert_eq!(
            Bonus::school(School::Fire, 0.05).describe(),
            "+5% Fire damage"
        );
        assert_eq!(d(StatKey::Cooldown, 0.09), "-9% cooldowns");
        assert_eq!(d(StatKey::Cooldown, -0.05), "+5% cooldowns");
        assert_eq!(d(StatKey::CritChance, 0.07), "+7% critical strike");
        assert_eq!(d(StatKey::Armor, 0.05), "+5% damage reduction");
        assert_eq!(d(StatKey::MoveSpeed, -0.10), "-10% movement speed");
        // Rounded to whole percent from 1% up, one decimal below.
        assert_eq!(d(StatKey::Gold, 0.125), "+13% gold");
        assert_eq!(d(StatKey::Luck, 0.005), "+0.5% luck");
        assert_eq!(d(StatKey::Lifesteal, 0.01), "+1% lifesteal");
    }

    #[test]
    fn describe_flat_values() {
        let d = |stat, v| Bonus::new(stat, v).describe();
        assert_eq!(d(StatKey::MaxHealth, 126.0), "+126 max health");
        assert_eq!(d(StatKey::Regen, 1.3), "+1.3 health per second");
        assert_eq!(d(StatKey::Regen, 2.0), "+2 health per second");
        assert_eq!(d(StatKey::Targets, 1.0), "+1 target");
        assert_eq!(d(StatKey::Targets, 2.0), "+2 targets");
        assert_eq!(d(StatKey::Revives, 1.0), "+1 extra life");
        assert_eq!(d(StatKey::Revives, 2.0), "+2 extra lives");
        assert_eq!(d(StatKey::MaxHealth, -15.0), "-15 max health");
    }

    #[test]
    fn labels_are_distinct() {
        for (i, a) in ALL_STATS.iter().enumerate() {
            assert_eq!(*a as usize, i);
            assert!(!a.label().is_empty());
            for b in &ALL_STATS[i + 1..] {
                assert_ne!(a.label(), b.label());
            }
        }
        assert_eq!(School::Fire.label(), "Fire");
    }

    #[test]
    fn parse_reads_stats_and_schools() {
        let (b, skipped) = parse("damage=0.5, crit_chance=0.2,fire=0.3,targets=1,revives=1");
        assert!(skipped.is_empty(), "{skipped:?}");
        assert!(close(b.get(StatKey::Damage), 0.5));
        assert!(close(b.get(StatKey::CritChance), 0.2));
        assert!(close(b.get(StatKey::Targets), 1.0));
        assert!(close(b.get(StatKey::Revives), 1.0));
        // A school's name is that school's damage, apart from unscoped damage.
        assert!(close(b.school_damage(School::Fire), 0.3));
        assert_eq!(b.school_damage(School::Frost), 0.0);
        // Repeats sum, as two sources would.
        let (twice, _) = parse("max_health=20,max_health=5,shadow=0.1,shadow=0.1");
        assert!(close(twice.get(StatKey::MaxHealth), 25.0));
        assert!(close(twice.school_damage(School::Shadow), 0.2));
    }

    #[test]
    fn parse_skips_malformed_entries() {
        let (b, skipped) = parse("bogus=1,damage,armor=lots,area=inf,MaxHealth=5,luck=0.25,,");
        assert_eq!(
            skipped,
            ["bogus=1", "damage", "armor=lots", "area=inf", "MaxHealth=5"]
        );
        assert!(close(b.get(StatKey::Luck), 0.25));
        assert_eq!(b, {
            let mut only = Bonuses::default();
            only.add(&Bonus::new(StatKey::Luck, 0.25));
            only
        });
        assert_eq!(parse("").0, Bonuses::default());
    }

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Saved {
        stat: StatKey,
        school: School,
        stats: Vec<StatKey>,
        schools: Vec<School>,
    }

    #[test]
    fn serde_names_round_trip() {
        let saved = Saved {
            stat: StatKey::MaxHealth,
            school: School::Fire,
            stats: ALL_STATS.to_vec(),
            schools: vec![School::Physical, School::Arcane],
        };
        let text = toml::to_string(&saved).unwrap();
        assert!(text.contains("stat = \"max_health\""), "{text}");
        assert!(text.contains("school = \"fire\""), "{text}");
        for name in [
            "\"crit_chance\"",
            "\"crit_damage\"",
            "\"move_speed\"",
            "\"lifesteal\"",
            "\"revives\"",
            "\"physical\"",
            "\"arcane\"",
        ] {
            assert!(text.contains(name), "{name} missing from {text}");
        }
        let back: Saved = toml::from_str(&text).unwrap();
        assert_eq!(back, saved);
        let hand: Saved = toml::from_str(
            "stat = \"cooldown\"\nschool = \"shadow\"\nstats = [\"pickup\"]\nschools = []\n",
        )
        .unwrap();
        assert_eq!(hand.stat, StatKey::Cooldown);
        assert_eq!(hand.school, School::Shadow);
        assert_eq!(hand.stats, [StatKey::Pickup]);
    }
}
