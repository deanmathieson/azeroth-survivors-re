//! What a talent does beyond its stats: the headline talents' abilities and the proc talents'
//! procs, built from the shared [`super::super::mechanics`] effects and granted on the hero at run
//! start (`grant_talents` in the parent module).
//!
//! - **Milestones.** Each tree's three headline talents for its 10, 20 and 30 points: the
//!   1-point ability talents of its tiers 3, 5 and 7, or the nearest one where a tier has none
//!   ([`milestones`]). Each is a real ability echoing what it did in WoW, under its own spell's
//!   visual.
//! - **Headlines.** The milestones and the other 1-point abilities and iconic talents: gold-framed
//!   on the Talents tab.
//! - **Procs.** Multi-rank talents whose WoW effect was a proc or a crowd control proc for real,
//!   their chance per rank ([`PerRank::Chance`]); one whose WoW proc listened to a school or a
//!   spell listens to the class's abilities of it ([`Listen`]).
//!
//! - **Teachers.** A talent that is also a spell or passive the run offers is never a second,
//!   separate effect under the same name ([`Teach`]): it teaches that spell, which only its
//!   learners are offered and which they take at rank 2, or upgrades the passive to start at rank
//!   2; a capstone whose name is its spec's summon is that summon ([`Teach::Spec`]).
//!
//! A talent here grants its mechanics instead of the stat bonus it had (`rules::bonuses` excludes
//! its old `effects` row); plain stat talents keep theirs. Numbers are the mode's own, at run
//! level 1: flat damage and healing grow with the run's level as every mechanic's does.

use super::super::bonus::{Bonus, School, StatKey};
use super::super::mechanics::{Aim, Cc, CcKind, Effect as Mech, MechanicSpec as M, Of, Trigger};

/// A burst this long lasts the run: set going once, a second in, it never runs out.
pub(super) const PERMANENT: f32 = 86_400.0;
/// Clearcasting, the buff Arcane Concentration and Omen of Clarity leave: the mage's and the
/// druid's `Spell.dbc` rows. A Clearcasting proc can come at most this often, in seconds.
pub(super) const CLEARCASTING: u32 = 12536;
pub(super) const CLEARCASTING_DRUID: u32 = 16870;
const CLEARCASTING_GAP: f32 = 3.0;
/// Shatter's visual: the frost shattering on a frozen target.
pub(super) const SHATTER: u32 = 116;
/// Ignite's burn, the effect the talent's passive sets going (the talent's own row has no
/// visual).
pub(super) const IGNITE: u32 = 12654;

/// How a talent's mechanics grow with its rank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PerRank {
    /// The same at every rank: a 1-point talent's.
    Same,
    /// Each hit, crit or kill trigger's chance is per rank, up to certain.
    Chance,
}

/// Which of the hero's abilities a talent's hit, crit, kill and cast triggers listen to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Listen {
    /// Every ability: the specs' own [`Of`].
    Own,
    /// Only these, by their keys in `data::ABILITIES`, named in the tooltip as `label` ("Fire",
    /// "Shadow Bolt"): each listening trigger is granted once per key.
    #[allow(dead_code)]
    To(&'static str, &'static [&'static str]),
}

/// What a talent that is also one of the run's spells or passives does instead of a mechanic of
/// its own, so one name is one thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Teach {
    /// The class spell of this key (in `data::ABILITIES`): offered only to heroes who learned the
    /// talent, and taken at rank 2.
    Spell(&'static str),
    /// The passive of this key (in `data::PASSIVES`), offered to all: its learners take it at
    /// rank 2.
    Passive(&'static str),
    /// The capstone's spec summon of the same name is its effect ([`super::spec`]).
    Spec,
}

/// One talent's mechanics.
pub(super) struct TalentMechanics {
    pub id: u32,
    /// The rank-1 spell's name, which the data tests check against `Spell.dbc`.
    pub name: &'static str,
    /// Gold-framed on the tab: a milestone, a 1-point ability or an iconic talent.
    pub headline: bool,
    pub per_rank: PerRank,
    pub listen: Listen,
    pub specs: &'static [M],
    /// What it teaches in place of `specs`, which are then empty.
    pub teaches: Option<Teach>,
}

/// `talent`'s mechanics, if the table has it.
pub(super) fn entry(talent: u32) -> Option<&'static TalentMechanics> {
    MECHANICS.iter().find(|m| m.id == talent)
}

/// The talent that teaches spell or passive `key`, if one does.
pub(super) fn teacher_of(key: &str) -> Option<&'static TalentMechanics> {
    MECHANICS.iter().find(|m| match m.teaches {
        Some(Teach::Spell(k) | Teach::Passive(k)) => k == key,
        _ => false,
    })
}

/// Every talent that teaches, for the audit.
#[cfg(test)]
pub(super) fn teachers() -> impl Iterator<Item = &'static TalentMechanics> {
    MECHANICS.iter().filter(|m| m.teaches.is_some())
}

/// Every talent with mechanics, for the audit.
#[cfg(test)]
pub(super) fn all() -> &'static [TalentMechanics] {
    MECHANICS
}

/// The spell whose sound a landmark talent's moment rings when the talent's own rank-one row has
/// no visual (a passive): the 1.12 spell it sets off, or the one it is known by.
/// The first of a talent's that rings is its sound.
pub(super) static SOUND_SPELLS: &[(u32, u32)] = &[
    (137, 29131),       // Anger Management: Bloodrage
    (153, 12976),       // Last Stand's own effect
    (1391, 5118),       // Bestial Swiftness: Aspect of the Cheetah
    (1391, 19574),      // or Bestial Wrath
    (1325, 24131),      // Wyvern Sting's poison
    (1325, 19503),      // or Scatter Shot
    (281, 2098),        // Relentless Strikes: Eviscerate
    (281, 14177),       // or Cold Blood
    (382, 2983),        // Vigor: Sprint
    (382, 14177),       // or Cold Blood
    (381, 1784),        // Premeditation: Stealth
    (381, 14177),       // or Cold Blood
    (1561, 27827),      // Spirit of Redemption's spirit
    (1561, 15237),      // or Holy Nova
    (574, 16246),       // Elemental Focus: its Clearcasting
    (565, 403),         // Elemental Fury: Lightning Bolt
    (565, 16166),       // or Elemental Mastery
    (582, 8075),        // Totemic Mastery: Strength of Earth Totem
    (582, 5394),        // or Healing Stream Totem
    (75, CLEARCASTING), // Arcane Concentration: its Clearcasting
    (967, 686),         // Ruin: Shadow Bolt
    (967, 17877),       // or Shadowburn
    (789, 2912),        // Nature's Grace: Starfire
    (789, 16864),       // or Omen of Clarity
    (809, 24932),       // Leader of the Pack's own aura
    (809, 16857),       // or Faerie Fire (Feral)
];

/// The three talent ids named by the tree's 10/20/30-point footer, if it is a class tree.
pub(in crate::survivors) fn milestones(tree: u32) -> Option<&'static [u32; 3]> {
    MILESTONES
        .iter()
        .find_map(|(id, talents)| (*id == tree).then_some(talents))
}

/// The three landmark talents, at 10, 20, and 30 points, for every TalentTab id.  This is kept
/// explicit instead of inferred from DBC layout: several vanilla rows at those tiers are passive
/// ranks, whereas this chooses the tree's memorable ability or proc.
pub(super) static MILESTONES: &[(u32, [u32; 3])] = &[
    (161, [137, 133, 135]),
    (164, [160, 165, 167]),
    (163, [153, 152, 148]),
    (382, [1435, 1433, 1502]),
    (383, [1442, 1431, 1430]),
    (381, [1481, 1409, 1441]),
    (361, [1391, 1387, 1386]),
    (363, [1345, 1353, 1361]),
    (362, [1308, 1312, 1325]),
    (182, [281, 280, 382]),
    (181, [301, 223, 205]),
    (183, [303, 284, 381]),
    (201, [348, 351, 322]),
    (202, [442, 1561, 1637]),
    (203, [501, 484, 521]),
    (261, [574, 565, 573]),
    (263, [617, 616, 901]),
    (262, [582, 591, 590]),
    (81, [75, 86, 87]),
    (41, [29, 32, 36]),
    (61, [69, 72, 71]),
    (302, [1061, 1041, 1022]),
    (303, [1226, 1281, 1282]),
    (301, [963, 967, 968]),
    (283, [788, 789, 793]),
    (281, [804, 1162, 809]),
    (282, [827, 831, 844]),
];

/// What `rank` ranks of `talent` grant, ready to grant: nothing at rank 0 or for a talent without
/// mechanics.
pub(super) fn at_rank(talent: u32, rank: u8) -> Vec<M> {
    let Some(entry) = entry(talent).filter(|_| rank > 0) else {
        return Vec::new();
    };
    let _ = entry.name;
    let mut out = Vec::new();
    for spec in entry.specs {
        let spec = scaled(*spec, entry.per_rank, rank);
        match (entry.listen, listens_to(spec.trigger)) {
            (Listen::To(_, keys), Some(_)) => {
                out.extend(keys.iter().map(|&k| with_of(spec, Of::Ability(k))));
            }
            _ => out.push(spec),
        }
    }
    out
}

/// `spec` at `rank`, as `per_rank` grows it.
fn scaled(spec: M, per_rank: PerRank, rank: u8) -> M {
    if per_rank == PerRank::Same {
        return spec;
    }
    let n = f32::from(rank);
    let up = |c: f32| (c * n).min(1.0);
    let trigger = match spec.trigger {
        Trigger::OnHit { of, chance } => Trigger::OnHit {
            of,
            chance: up(chance),
        },
        Trigger::OnCrit { of, chance } => Trigger::OnCrit {
            of,
            chance: up(chance),
        },
        Trigger::OnKill { of, chance } => Trigger::OnKill {
            of,
            chance: up(chance),
        },
        other => other,
    };
    M { trigger, ..spec }
}

/// The ability a trigger listens to, for the triggers that listen to one.
fn listens_to(trigger: Trigger) -> Option<Of> {
    match trigger {
        Trigger::OnCast(of)
        | Trigger::OnHit { of, .. }
        | Trigger::OnCrit { of, .. }
        | Trigger::OnKill { of, .. } => Some(of),
        _ => None,
    }
}

fn with_of(spec: M, of: Of) -> M {
    let trigger = match spec.trigger {
        Trigger::OnCast(_) => Trigger::OnCast(of),
        Trigger::OnHit { chance, .. } => Trigger::OnHit { of, chance },
        Trigger::OnCrit { chance, .. } => Trigger::OnCrit { of, chance },
        Trigger::OnKill { chance, .. } => Trigger::OnKill { of, chance },
        other => other,
    };
    M { trigger, ..spec }
}

// ── Tooltips ────────────────────────────────────────────────────────────────────────────────────

/// What `rank` ranks of `talent`'s mechanics do, one line each, in WoW-like words: "Every 60 sec:
/// +30% damage and 30% shorter cooldowns for 15 sec." Empty for a talent without mechanics.
pub(super) fn describe(talent: u32, rank: u8) -> Vec<String> {
    let Some(entry) = entry(talent) else {
        return Vec::new();
    };
    match entry.teaches {
        Some(Teach::Spell(_)) => {
            return vec![format!(
                "Teaches {}: your runs offer it, and you take it at rank 2.",
                entry.name
            )];
        }
        Some(Teach::Passive(_)) => {
            return vec![format!(
                "Your runs' {} starts at rank 2 when you take it.",
                entry.name
            )];
        }
        Some(Teach::Spec) => return Vec::new(),
        None => {}
    }
    let label = match entry.listen {
        Listen::To(label, _) => Some(label),
        Listen::Own => None,
    };
    entry
        .specs
        .iter()
        .map(|s| {
            describe_spec(
                &scaled(*s, entry.per_rank, rank.max(1)),
                label,
                Some(proc_name(entry)),
            )
        })
        .collect()
}

/// The name a talent's proc goes by in the tooltip: Clearcasting for the talents that leave it,
/// else the talent's own.
fn proc_name(entry: &TalentMechanics) -> &'static str {
    let clearcasting = entry.specs.iter().any(|s| {
        matches!(
            s.effect,
            Mech::NextCast { aura, .. } if aura == CLEARCASTING || aura == CLEARCASTING_DRUID
        )
    });
    if clearcasting {
        "Clearcasting"
    } else {
        entry.name
    }
}

/// One mechanic as a tooltip line; `label` names what its trigger listens to, when that is not
/// its own [`Of`]. A proc of a talent `name`d reads as WoW words one: "Your hits have a 10% chance
/// to proc Impact: stuns the target for 1.5 sec."
pub(super) fn describe_spec(spec: &M, label: Option<&str>, name: Option<&str>) -> String {
    match (spec.trigger, spec.effect) {
        (Trigger::Periodic { .. }, Mech::Burst { duration, bonuses }) if duration >= PERMANENT => {
            return format!("For the whole run: {}.", bonus_list(bonuses));
        }
        (_, Mech::TotemReach { mult }) => {
            return format!(
                "Your totems reach {} farther, their reach ringed on the ground.",
                pct(mult - 1.0)
            );
        }
        _ => {}
    }
    let proc = matches!(
        spec.trigger,
        Trigger::OnHit { .. } | Trigger::OnCrit { .. } | Trigger::OnKill { .. }
    );
    let once = (spec.cooldown > 0.0 && !matches!(spec.trigger, Trigger::Periodic { .. }))
        .then(|| format!(" (once every {})", secs(spec.cooldown)));
    let mut when = match name.and_then(|n| proc_text(spec.trigger, label, n)) {
        Some(named) => named,
        None => trigger_text(spec.trigger, label),
    };
    if let Some(once) = once {
        when.push_str(&once);
    }
    format!("{when}: {}.", effect_text(spec.effect, spec.at, proc))
}

/// A proc trigger as WoW names a talent's proc: "Your critical strikes proc Ignite", "Your Fire
/// hits have a 6% chance to proc Impact". `None` for any other trigger.
fn proc_text(trigger: Trigger, label: Option<&str>, name: &str) -> Option<String> {
    let (of, c, what) = match trigger {
        Trigger::OnHit { of, chance } => (of, chance, "hits"),
        Trigger::OnCrit { of, chance } => (of, chance, "critical strikes"),
        Trigger::OnKill { of, chance } => (of, chance, "killing blows"),
        _ => return None,
    };
    let subject = match (label, of) {
        (Some(l), _) => format!("Your {l} {what}"),
        (None, Of::Ability(key)) => format!("Your {} {what}", title(key)),
        (None, Of::Any) => format!("Your {what}"),
    };
    Some(if c >= 1.0 {
        format!("{subject} proc {name}")
    } else {
        format!("{subject} have a {} chance to proc {name}", pct(c))
    })
}

fn trigger_text(trigger: Trigger, label: Option<&str>) -> String {
    let named = |of: Of| match (label, of) {
        (Some(l), _) => Some(l.to_string()),
        (None, Of::Ability(key)) => Some(title(key)),
        (None, Of::Any) => None,
    };
    let chance = |c: f32, what: &str, of: Of| {
        let what = match named(of) {
            Some(n) => format!("{n} {what}"),
            None => what.to_string(),
        };
        if c >= 1.0 {
            format!("On every {what}")
        } else {
            format!("{} chance on {} {what}", pct(c), article(&what))
        }
    };
    match trigger {
        Trigger::Periodic { every } => format!("Every {}", secs(every)),
        Trigger::OnCast(of) => match named(of) {
            Some(n) => format!("When you cast {n}"),
            None => "When you cast".to_string(),
        },
        Trigger::OnHit { of, chance: c } => chance(c, "hit", of),
        Trigger::OnCrit { of, chance: c } => chance(c, "critical strike", of),
        Trigger::OnKill { of, chance: c } => chance(c, "kill", of),
        Trigger::OnLethal => "When a blow would kill you".to_string(),
        Trigger::OnLowHealth { threshold } => {
            format!("When your health falls below {}", pct(threshold))
        }
    }
}

fn effect_text(effect: Mech, at: Aim, proc: bool) -> String {
    match effect {
        Mech::Nova {
            school,
            radius,
            damage,
            knockback,
            cc,
        } => {
            let mut s = format!(
                "a {} blast {} hits enemies within {} yd",
                school.label(),
                area_at(at, proc),
                num(radius)
            );
            if damage > 0.0 {
                s.push_str(&format!(" for {} damage", num(damage)));
            }
            let mut then = Vec::new();
            if knockback > 0.0 {
                then.push("knocking them back".to_string());
            }
            if let Some(cc) = cc {
                then.push(cc_participle(cc));
            }
            if !then.is_empty() {
                s.push_str(", ");
                s.push_str(&then.join(" and "));
            }
            s
        }
        Mech::Burst { duration, bonuses } => {
            format!("{} for {}", bonus_list(bonuses), secs(duration))
        }
        Mech::BurningGround {
            school,
            radius,
            dps,
            duration,
        } => format!(
            "the ground {} burns for {} {} damage a second within {} yd for {}",
            area_at(at, proc),
            num(dps),
            school.label(),
            num(radius),
            secs(duration)
        ),
        Mech::Chain { jumps, falloff } => format!(
            "the blow leaps on to {} at {} power",
            count(jumps, "more enemy", "more enemies"),
            pct(falloff)
        ),
        Mech::Split { count: n, .. } => {
            format!(
                "the blow splits into {} at half power",
                count(n, "shard", "shards")
            )
        }
        Mech::Pierce { n } => format!(
            "the blow passes through up to {} behind",
            count(n, "enemy", "enemies")
        ),
        Mech::Boomerang => "the blow flies back to you, striking all in its path".to_string(),
        Mech::Orbit { n, duration, .. } => format!(
            "{} circle you for {}, striking with the blow",
            count(n, "mote", "motes"),
            secs(duration)
        ),
        Mech::Echo { extra_casts: 1 } => "the ability casts again".to_string(),
        Mech::Echo { extra_casts } => format!("the ability casts {extra_casts} more times"),
        Mech::Execute { threshold, mult } => {
            let times = match mult {
                2.0 => "double".to_string(),
                3.0 => "triple".to_string(),
                m => format!("{}×", num(m)),
            };
            format!(
                "{} below {} health takes {times} damage",
                target_at(at, proc),
                pct(threshold)
            )
        }
        Mech::HealPulse { radius, amount } if radius < 3.0 => {
            format!("heals you for {}", num(amount))
        }
        Mech::HealPulse { radius, amount } => format!(
            "heals you and allies within {} yd for {}",
            num(radius),
            num(amount)
        ),
        Mech::PartyAura {
            radius,
            duration,
            bonuses,
        } => format!(
            "you and allies within {} yd gain {} for {}",
            num(radius),
            bonus_list(bonuses),
            secs(duration)
        ),
        Mech::BestAllyAura {
            radius,
            duration,
            bonuses,
        } => format!(
            "the highest-damage ally within {} yd gains {} for {}",
            num(radius),
            bonus_list(bonuses),
            secs(duration)
        ),
        Mech::Shield { amount, duration } => format!(
            "a shield absorbs {} damage for {}",
            num(amount),
            secs(duration)
        ),
        Mech::CheatDeath { immune, heal_frac } => format!(
            "you live on at {} health, immune for {}",
            pct(heal_frac),
            secs(immune)
        ),
        Mech::ResetCooldowns => "every ability is ready at once".to_string(),
        Mech::ApplyCc { kind, secs: t } => {
            let target = target_at(at, proc);
            match kind {
                CcKind::Stun => format!("stuns {target} for {}", secs(t)),
                CcKind::Chill(m) => {
                    format!("slows {target} to {} speed for {}", pct(m), secs(t))
                }
                CcKind::Root => format!("roots {target} for {}", secs(t)),
                CcKind::Fear => format!("fears {target} for {}", secs(t)),
            }
        }
        Mech::Dot {
            school,
            dps,
            secs: t,
        } => format!(
            "{} takes {} {} damage a second for {}",
            target_at(at, proc),
            num(dps),
            school.label(),
            secs(t)
        ),
        Mech::NextCast {
            instant,
            crit,
            cooldown,
            ..
        } => {
            let mut parts = Vec::new();
            if instant {
                parts.push("your slowest ability is ready at once".to_string());
            }
            if crit {
                parts.push("your next cast strikes critically with every blow".to_string());
            }
            if cooldown < 1.0 {
                let kept = if (cooldown - 0.5).abs() < 1e-3 {
                    "halved".to_string()
                } else {
                    format!("{} shorter", pct(1.0 - cooldown))
                };
                parts.push(format!("your next cast's cooldown is {kept}"));
            }
            parts.join(", and ")
        }
        Mech::Ignite {
            dps,
            secs: t,
            per_burn,
        } => format!(
            "{} burns for {} Fire damage a second for {}, {} more for each Fire burn already on it",
            target_at(at, proc),
            num(dps),
            secs(t),
            pct(per_burn)
        ),
        Mech::Shatter { bonus } => format!(
            "a frozen or rooted target takes the blow again for {} of it",
            pct(bonus)
        ),
        Mech::ConsumeDot {
            school,
            burst,
            spread,
            radius,
            secs: t,
        } => format!(
            "your {s} burn on the nearest enemy is consumed for {} of its remaining damage at once, \
             and spreads to {} within {} yd for {}",
            pct(burst),
            count(spread, "enemy", "enemies"),
            num(radius),
            secs(t),
            s = school.label(),
        ),
        Mech::ShareHeal {
            school,
            share,
            radius,
        } => format!(
            "{} of the {} damage you dealt heals you and allies within {} yd",
            pct(share),
            school.label(),
            num(radius)
        ),
        Mech::TotemReach { mult } => format!("your totems reach {} farther", pct(mult - 1.0)),
    }
}

/// Where an area lands, after "a blast …".
fn area_at(at: Aim, proc: bool) -> String {
    match at {
        Aim::Trigger if proc => "around the target".to_string(),
        Aim::Trigger | Aim::Hero => "around you".to_string(),
        Aim::Follow => "under you".to_string(),
        Aim::Spread { n, .. } => format!("on {}", nearest(n, proc)),
        Aim::Random { .. } => "on a random enemy".to_string(),
        Aim::Densest { .. } => "on the thickest pack".to_string(),
    }
}

/// Whom a single-target effect lands on.
fn target_at(at: Aim, proc: bool) -> String {
    match at {
        Aim::Trigger if proc => "the target".to_string(),
        Aim::Trigger | Aim::Hero | Aim::Follow => "the nearest enemy".to_string(),
        Aim::Spread { n, .. } => nearest(n, proc),
        Aim::Random { .. } => "a random enemy".to_string(),
        Aim::Densest { .. } => "an enemy in the thickest pack".to_string(),
    }
}

/// "the nearest enemy", "the 2 nearest other enemies".
fn nearest(n: u8, proc: bool) -> String {
    let other = if proc { "other " } else { "" };
    if n == 1 {
        format!("the nearest {other}enemy")
    } else {
        format!("the {n} nearest {other}enemies")
    }
}

fn cc_participle(cc: Cc) -> String {
    let t = secs(cc.secs);
    match cc.kind {
        CcKind::Stun => format!("stunning them for {t}"),
        CcKind::Chill(m) => format!("slowing them to {} speed for {t}", pct(m)),
        CcKind::Root => format!("rooting them for {t}"),
        CcKind::Fear => format!("fearing them for {t}"),
    }
}

/// A burst's bonuses: "+30% damage and 30% shorter cooldowns".
fn bonus_list(bonuses: &[Bonus]) -> String {
    let words: Vec<String> = bonuses
        .iter()
        .map(|b| match b.stat {
            StatKey::Cooldown if b.value > 0.0 => {
                format!("{} shorter cooldowns", pct(b.value))
            }
            StatKey::CritChance if b.value >= 1.0 => "every blow a critical strike".to_string(),
            _ => b.describe(),
        })
        .collect();
    match words.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// `n` of a thing: "1 more enemy", "3 shards".
fn count(n: u8, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}

/// "a" or "an", as `word` begins.
fn article(word: &str) -> &'static str {
    match word.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

/// A key as a name: "fire_blast" reads "Fire Blast".
fn title(key: &str) -> String {
    key.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `v` whole when it rounds to a whole number, else to one decimal place.
fn num(v: f32) -> String {
    let t = (v * 10.0).round() / 10.0;
    if t.fract() == 0.0 {
        format!("{t:.0}")
    } else {
        format!("{t:.1}")
    }
}

/// A fraction as a percentage: 0.3 reads "30%".
fn pct(v: f32) -> String {
    format!("{}%", num(v * 100.0))
}

/// Seconds as WoW says them: "15 sec", "2 min".
fn secs(v: f32) -> String {
    if v >= 60.0 && (v % 60.0).abs() < 1e-3 {
        format!("{} min", num(v / 60.0))
    } else {
        format!("{} sec", num(v))
    }
}

macro_rules! mechanic {
    ($id:literal, $name:literal, $spell:expr, $trigger:expr, $effect:expr) => {
        TalentMechanics {
            id: $id,
            name: $name,
            headline: true,
            per_rank: PerRank::Same,
            listen: Listen::Own,
            specs: &[M::new($trigger, $effect).visual($spell)],
            teaches: None,
        }
    };
}

/// A multi-rank talent whose chance is stated per rank in vanilla.
macro_rules! proc_mechanic {
    ($id:literal, $name:literal, $spell:expr, $trigger:expr, $effect:expr) => {
        TalentMechanics {
            id: $id,
            name: $name,
            headline: true,
            per_rank: PerRank::Chance,
            listen: Listen::Own,
            specs: &[M::new($trigger, $effect).visual($spell)],
            teaches: None,
        }
    };
}

/// A multi-rank proc talent whose blow lands a crowd control on the struck target: `chance` per
/// rank on a hit, `secs` of `kind`, under `spell`'s impact.
macro_rules! cc_proc {
    ($id:literal, $name:literal, $spell:literal, $chance:literal, $kind:expr, $secs:literal) => {
        TalentMechanics {
            id: $id,
            name: $name,
            headline: false,
            per_rank: PerRank::Chance,
            listen: Listen::Own,
            specs: &[M::new(
                Trigger::OnHit {
                    of: Of::Any,
                    chance: $chance,
                },
                Mech::ApplyCc {
                    kind: $kind,
                    secs: $secs,
                },
            )
            .visual($spell)],
            teaches: None,
        }
    };
}

/// A multi-rank talent whose 1.12 proc was an extra combo point: `chance` per rank on a hit to
/// cut the next cast's cooldown by a quarter, at most once every [`CLEARCASTING_GAP`].
macro_rules! combo_proc {
    ($id:literal, $name:literal, $spell:literal, $chance:literal) => {
        TalentMechanics {
            id: $id,
            name: $name,
            headline: false,
            per_rank: PerRank::Chance,
            listen: Listen::Own,
            specs: &[M::new(
                Trigger::OnHit {
                    of: Of::Any,
                    chance: $chance,
                },
                Mech::NextCast {
                    instant: false,
                    crit: false,
                    cooldown: 0.75,
                    aura: 0,
                },
            )
            .visual($spell)
            .cooldown(CLEARCASTING_GAP)],
            teaches: None,
        }
    };
}

/// A talent that is also a spell or passive of the run, or its spec's summon: it teaches that
/// instead of a mechanic of its own ([`Teach`]).
macro_rules! teaches {
    ($id:literal, $name:literal, $teach:expr) => {
        TalentMechanics {
            id: $id,
            name: $name,
            headline: true,
            per_rank: PerRank::Same,
            listen: Listen::Own,
            specs: &[],
            teaches: Some($teach),
        }
    };
}

/// A landmark active, represented by a short recurring spell burst.  The bespoke entries below
/// use a closer effect where the shared mechanic vocabulary has one (shields, cheat death,
/// chains, cooldown resets, and so on).
macro_rules! active {
    ($id:literal, $name:literal, $spell:literal, $school:expr) => {
        mechanic!(
            $id,
            $name,
            $spell,
            Trigger::Periodic { every: 35.0 },
            Mech::Nova {
                school: $school,
                radius: 6.0,
                damage: 24.0,
                knockback: 0.0,
                cc: None
            }
        )
    };
}

/// A landmark passive or stance, made visible as a permanent run aura rather than another
/// invisible percentage row in the talent sheet.
macro_rules! aura {
    ($id:literal, $name:literal, $spell:literal, $stat:expr, $value:expr) => {
        mechanic!(
            $id,
            $name,
            $spell,
            Trigger::Periodic { every: 1.0 },
            Mech::Burst {
                duration: PERMANENT,
                bonuses: &[Bonus::new($stat, $value)]
            }
        )
    };
}

/// The active talents.  Their rank-one spell is deliberately the visual: this keeps the ability
/// recognisable even where the roguelite effect is only an echo of its WoW counterpart.
static MECHANICS: &[TalentMechanics] = &[
    // Clearcasting: a hit may halve the next cast's cooldown.
    TalentMechanics {
        id: 75,
        name: "Arcane Concentration",
        headline: true,
        per_rank: PerRank::Chance,
        listen: Listen::Own,
        specs: &[M::new(
            Trigger::OnHit {
                of: Of::Any,
                chance: 0.02,
            },
            Mech::NextCast {
                instant: false,
                crit: false,
                cooldown: 0.5,
                aura: CLEARCASTING,
            },
        )
        .visual(11213)
        .cooldown(CLEARCASTING_GAP)],
        teaches: None,
    },
    mechanic!(
        86,
        "Presence of Mind",
        12043,
        Trigger::Periodic { every: 30.0 },
        Mech::NextCast {
            instant: true,
            crit: true,
            cooldown: 1.0,
            aura: 12043
        }
    ),
    mechanic!(
        87,
        "Arcane Power",
        12042,
        Trigger::Periodic { every: 60.0 },
        Mech::Burst {
            duration: 15.0,
            bonuses: &[
                Bonus::new(StatKey::Damage, 0.30),
                Bonus::new(StatKey::Cooldown, 0.30)
            ]
        }
    ),
    teaches!(32, "Blast Wave", Teach::Spell("blast_wave")),
    mechanic!(
        36,
        "Combustion",
        11129,
        Trigger::Periodic { every: 60.0 },
        Mech::Burst {
            duration: 12.0,
            bonuses: &[Bonus::new(StatKey::CritChance, 1.0)]
        }
    ),
    mechanic!(
        72,
        "Ice Block",
        11958,
        Trigger::OnLethal,
        Mech::CheatDeath {
            immune: 3.0,
            heal_frac: 0.30
        }
    ),
    mechanic!(
        71,
        "Ice Barrier",
        11426,
        Trigger::Periodic { every: 30.0 },
        Mech::Shield {
            amount: 45.0,
            duration: 20.0
        }
    ),
    teaches!(133, "Sweeping Strikes", Teach::Spell("sweeping_strikes")),
    mechanic!(
        153,
        "Last Stand",
        12975,
        Trigger::OnLowHealth { threshold: 0.30 },
        Mech::Burst {
            duration: 12.0,
            bonuses: &[Bonus::new(StatKey::MaxHealth, 0.30)]
        }
    ),
    mechanic!(
        284,
        "Preparation",
        14185,
        Trigger::Periodic { every: 60.0 },
        Mech::ResetCooldowns
    ),
    teaches!(963, "Shadowburn", Teach::Spell("shadowburn")),
    mechanic!(
        1561,
        "Spirit of Redemption",
        20711,
        Trigger::OnLethal,
        Mech::CheatDeath {
            immune: 5.0,
            heal_frac: 0.50
        }
    ),
    mechanic!(
        793,
        "Moonkin Form",
        24858,
        Trigger::Periodic { every: 1.0 },
        Mech::Burst {
            duration: PERMANENT,
            bonuses: &[Bonus::new(StatKey::Damage, 0.15)]
        }
    ),
    mechanic!(
        844,
        "Swiftmend",
        18562,
        Trigger::Periodic { every: 20.0 },
        Mech::HealPulse {
            radius: 2.0,
            amount: 32.0
        }
    ),
    proc_mechanic!(
        30,
        "Impact",
        11103,
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.02
        },
        Mech::ApplyCc {
            kind: CcKind::Stun,
            secs: 1.5
        }
    ),
    proc_mechanic!(
        34,
        "Ignite",
        IGNITE,
        Trigger::OnCrit {
            of: Of::Any,
            chance: 1.0
        },
        // Fed by the burns on its target: Fireball's own among them.
        Mech::Ignite {
            dps: 8.0,
            secs: 4.0,
            per_burn: 0.15
        }
    ),
    proc_mechanic!(
        67,
        "Shatter",
        SHATTER,
        Trigger::OnCrit {
            of: Of::Any,
            chance: 0.2
        },
        Mech::Shatter { bonus: 0.5 }
    ),
    proc_mechanic!(
        464,
        "Blackout",
        15268,
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.02
        },
        Mech::ApplyCc {
            kind: CcKind::Stun,
            secs: 2.0
        }
    ),
    proc_mechanic!(
        129,
        "Improved Hamstring",
        12289,
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.03
        },
        Mech::ApplyCc {
            kind: CcKind::Root,
            secs: 2.0
        }
    ),
    proc_mechanic!(
        121,
        "Deep Wounds",
        12834,
        Trigger::OnCrit {
            of: Of::Any,
            chance: 0.33
        },
        // Dots are independently tracked per landed proc (up to the enemy's six-dot cap), so
        // successive weapon criticals build the intended stacking bleed.
        Mech::Dot {
            school: School::Physical,
            dps: 5.0,
            secs: 5.0
        }
    ),
    proc_mechanic!(
        63,
        "Improved Blizzard",
        11185,
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.33
        },
        Mech::ApplyCc {
            kind: CcKind::Chill(0.5),
            secs: 2.0
        }
    ),
    // Talents whose 1.12 effect was a named proc, once a "luck" row: now that proc, under the
    // spell it set off.
    cc_proc!(125, "Mace Specialization", 5530, 0.01, CcKind::Stun, 1.5),
    cc_proc!(147, "Improved Revenge", 12798, 0.03, CcKind::Stun, 1.0),
    cc_proc!(149, "Improved Shield Bash", 18498, 0.03, CcKind::Stun, 1.0),
    cc_proc!(
        1341,
        "Improved Concussive Shot",
        19410,
        0.01,
        CcKind::Stun,
        1.5
    ),
    cc_proc!(1304, "Entrapment", 19185, 0.02, CcKind::Root, 2.0),
    proc_mechanic!(
        268,
        "Improved Poisons",
        2818,
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.02
        },
        Mech::Dot {
            school: School::Nature,
            dps: 3.0,
            secs: 6.0
        }
    ),
    cc_proc!(206, "Improved Kick", 18425, 0.03, CcKind::Stun, 1.0),
    cc_proc!(184, "Mace Specialization", 5530, 0.01, CcKind::Stun, 1.5),
    cc_proc!(88, "Improved Counterspell", 18469, 0.03, CcKind::Stun, 1.0),
    cc_proc!(38, "Frostbite", 12494, 0.03, CcKind::Root, 2.0),
    cc_proc!(982, "Aftermath", 18118, 0.02, CcKind::Chill(0.5), 3.0),
    cc_proc!(986, "Pyroclasm", 18093, 0.03, CcKind::Stun, 1.5),
    cc_proc!(
        921,
        "Improved Nature's Grasp",
        19975,
        0.02,
        CcKind::Root,
        2.0
    ),
    // An extra combo point, here a hastened next cast.
    combo_proc!(273, "Ruthlessness", 14156, 0.03),
    combo_proc!(245, "Initiative", 13976, 0.03),
    // The 27 trees' 10 / 20 / 30-point landmarks.  Keep this list in lock-step with
    // MILESTONES: the test below deliberately checks both the Talent and Spell DBC data.
    aura!(137, "Anger Management", 12296, StatKey::Regen, 1.0),
    teaches!(135, "Mortal Strike", Teach::Spell("mortal_strike")),
    active!(160, "Piercing Howl", 12323, School::Physical),
    aura!(165, "Death Wish", 12328, StatKey::Damage, 0.20),
    teaches!(167, "Bloodthirst", Teach::Spell("bloodthirst")),
    mechanic!(
        152,
        "Concussion Blow",
        12809,
        Trigger::Periodic { every: 35.0 },
        Mech::Nova {
            school: School::Physical,
            radius: 6.0,
            damage: 18.0,
            knockback: 0.0,
            cc: Some(Cc {
                kind: CcKind::Stun,
                secs: 1.5
            })
        }
    ),
    teaches!(148, "Shield Slam", Teach::Spell("shield_slam")),
    teaches!(1435, "Consecration", Teach::Spell("consecration")),
    aura!(1433, "Divine Favor", 20216, StatKey::CritChance, 0.20),
    teaches!(1502, "Holy Shock", Teach::Spell("holy_shock")),
    teaches!(1442, "Blessing of Kings", Teach::Passive("kings")),
    aura!(1431, "Blessing of Sanctuary", 20911, StatKey::Armor, 0.15),
    active!(1430, "Holy Shield", 20925, School::Holy),
    proc_mechanic!(
        1481,
        "Seal of Command",
        20375,
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.20
        },
        Mech::Nova {
            school: School::Holy,
            radius: 2.0,
            damage: 12.0,
            knockback: 0.0,
            cc: None
        }
    ),
    aura!(1409, "Sanctity Aura", 20218, StatKey::Damage, 0.12),
    active!(1441, "Repentance", 20066, School::Holy),
    aura!(1391, "Bestial Swiftness", 19596, StatKey::MoveSpeed, 0.15),
    active!(1387, "Intimidation", 19577, School::Physical),
    aura!(1386, "Bestial Wrath", 19574, StatKey::Damage, 0.25),
    teaches!(1345, "Aimed Shot", Teach::Spell("aimed_shot")),
    active!(1353, "Scatter Shot", 19503, School::Physical),
    mechanic!(
        1361,
        "Trueshot Aura",
        19506,
        Trigger::Periodic { every: 1.0 },
        Mech::PartyAura {
            radius: 40.0,
            duration: 2.0,
            bonuses: &[Bonus::new(StatKey::Damage, 0.06)]
        }
    ),
    aura!(1308, "Deterrence", 19263, StatKey::Armor, 0.20),
    active!(1312, "Counterattack", 19306, School::Physical),
    active!(1325, "Wyvern Sting", 19386, School::Nature),
    aura!(281, "Relentless Strikes", 14179, StatKey::Cooldown, 0.10),
    aura!(280, "Cold Blood", 14177, StatKey::CritChance, 0.25),
    aura!(382, "Vigor", 14983, StatKey::Damage, 0.10),
    active!(301, "Riposte", 14251, School::Physical),
    teaches!(223, "Blade Flurry", Teach::Spell("blade_flurry")),
    aura!(205, "Adrenaline Rush", 13750, StatKey::Cooldown, 0.20),
    active!(303, "Ghostly Strike", 14278, School::Physical),
    active!(381, "Premeditation", 14183, School::Physical),
    aura!(348, "Inner Focus", 14751, StatKey::CritChance, 0.15),
    mechanic!(
        351,
        "Divine Spirit",
        14752,
        Trigger::Periodic { every: 1.0 },
        Mech::PartyAura {
            radius: 40.0,
            duration: 2.0,
            bonuses: &[Bonus::new(StatKey::Regen, 0.6)]
        }
    ),
    mechanic!(
        322,
        "Power Infusion",
        10060,
        Trigger::Periodic { every: 45.0 },
        Mech::BestAllyAura {
            radius: 40.0,
            duration: 10.0,
            bonuses: &[Bonus::new(StatKey::Damage, 0.08)]
        }
    ),
    teaches!(442, "Holy Nova", Teach::Spell("holy_nova")),
    teaches!(1637, "Lightwell", Teach::Spec),
    teaches!(501, "Mind Flay", Teach::Spell("mind_flay")),
    mechanic!(
        484,
        "Vampiric Embrace",
        15286,
        Trigger::Periodic { every: 2.0 },
        Mech::ShareHeal {
            school: School::Shadow,
            share: 0.15,
            radius: 40.0
        }
    ),
    mechanic!(
        521,
        "Shadowform",
        15473,
        Trigger::Periodic { every: 1.0 },
        Mech::Burst {
            duration: PERMANENT,
            bonuses: &[
                Bonus::school(School::Shadow, 0.15),
                Bonus::new(StatKey::Armor, 0.15)
            ]
        }
    ),
    aura!(574, "Elemental Focus", 16164, StatKey::CritChance, 0.12),
    aura!(565, "Elemental Fury", 16089, StatKey::CritDamage, 0.25),
    aura!(573, "Elemental Mastery", 16166, StatKey::Damage, 0.20),
    aura!(
        617,
        "Two-Handed Axes and Maces",
        16269,
        StatKey::Damage,
        0.12
    ),
    aura!(616, "Parry", 16268, StatKey::Armor, 0.10),
    teaches!(901, "Stormstrike", Teach::Spell("stormstrike")),
    mechanic!(
        582,
        "Totemic Mastery",
        16189,
        Trigger::Periodic { every: PERMANENT },
        Mech::TotemReach { mult: 1.5 }
    ),
    aura!(591, "Nature's Swiftness", 16188, StatKey::Cooldown, 0.15),
    active!(590, "Mana Tide Totem", 16190, School::Nature),
    teaches!(29, "Pyroblast", Teach::Spell("pyroblast")),
    mechanic!(
        69,
        "Cold Snap",
        12472,
        Trigger::Periodic { every: 75.0 },
        Mech::ResetCooldowns
    ),
    active!(1061, "Amplify Curse", 18288, School::Shadow),
    active!(1041, "Siphon Life", 18265, School::Shadow),
    active!(1022, "Dark Pact", 18220, School::Shadow),
    aura!(1226, "Fel Domination", 18708, StatKey::Cooldown, 0.20),
    aura!(1281, "Demonic Sacrifice", 18788, StatKey::Damage, 0.15),
    aura!(1282, "Soul Link", 19028, StatKey::Armor, 0.15),
    aura!(967, "Ruin", 17959, StatKey::CritDamage, 0.35),
    mechanic!(
        968,
        "Conflagrate",
        17962,
        Trigger::Periodic { every: 10.0 },
        Mech::ConsumeDot {
            school: School::Fire,
            burst: 1.25,
            spread: 2,
            radius: 8.0,
            secs: 5.0
        }
    ),
    // Clearcasting: a hit may halve the next cast's cooldown.
    TalentMechanics {
        id: 788,
        name: "Omen of Clarity",
        headline: true,
        per_rank: PerRank::Same,
        listen: Listen::Own,
        specs: &[M::new(
            Trigger::OnHit {
                of: Of::Any,
                chance: 0.06,
            },
            Mech::NextCast {
                instant: false,
                crit: false,
                cooldown: 0.5,
                aura: CLEARCASTING_DRUID,
            },
        )
        .visual(16864)
        .cooldown(CLEARCASTING_GAP)],
        teaches: None,
    },
    aura!(789, "Nature's Grace", 16880, StatKey::CritChance, 0.12),
    active!(804, "Feral Charge", 16979, School::Physical),
    active!(1162, "Faerie Fire (Feral)", 16857, School::Nature),
    mechanic!(
        809,
        "Leader of the Pack",
        17007,
        Trigger::Periodic { every: 1.0 },
        Mech::PartyAura {
            radius: 40.0,
            duration: 2.0,
            bonuses: &[Bonus::new(StatKey::CritChance, 0.05)]
        }
    ),
    teaches!(827, "Insect Swarm", Teach::Spell("insect_swarm")),
    aura!(831, "Nature's Swiftness", 17116, StatKey::Cooldown, 0.15),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_words_a_proc_in_wow_like_language() {
        let spec = M::new(
            Trigger::OnHit {
                of: Of::Any,
                chance: 0.15,
            },
            Mech::ApplyCc {
                kind: CcKind::Stun,
                secs: 2.0,
            },
        )
        .at(Aim::Trigger);
        assert_eq!(
            describe_spec(&spec, None, None),
            "15% chance on a hit: stuns the target for 2 sec."
        );
    }

    #[test]
    fn proc_chances_scale_by_rank() {
        let spec = at_rank(30, 5).pop().unwrap();
        let Trigger::OnHit { chance, .. } = spec.trigger else {
            panic!("Impact listens for hits")
        };
        assert!((chance - 0.10).abs() < f32::EPSILON);
    }

    #[test]
    fn party_talents_are_short_range_auras_with_real_combat_effects() {
        for id in [1361, 351, 809] {
            let spec = at_rank(id, 1).pop().unwrap();
            assert!(spec.validate().is_ok(), "{id}: {spec:?}");
            let Mech::PartyAura { radius, .. } = spec.effect else {
                panic!("{id} is not a party aura");
            };
            assert!((radius - 40.0).abs() < f32::EPSILON);
        }
        let Mech::BestAllyAura { radius, .. } = at_rank(322, 1).pop().unwrap().effect else {
            panic!("Power Infusion is not an ally aura");
        };
        assert!((radius - 40.0).abs() < f32::EPSILON);
        assert!(matches!(
            at_rank(121, 3).pop().unwrap().effect,
            Mech::Dot {
                school: School::Physical,
                ..
            }
        ));
        assert!(matches!(
            at_rank(1481, 1).pop().unwrap().trigger,
            Trigger::OnHit { .. }
        ));
        assert!(matches!(
            at_rank(152, 1).pop().unwrap().effect,
            Mech::Nova {
                cc: Some(Cc {
                    kind: CcKind::Stun,
                    ..
                }),
                ..
            }
        ));
    }

    /// Presence of Mind and Clearcasting are one reusable next-cast guarantee.
    #[test]
    fn presence_of_mind_and_clearcasting_are_next_cast_guarantees() {
        let pom = at_rank(86, 1).pop().unwrap();
        assert_eq!(pom.trigger, Trigger::Periodic { every: 30.0 });
        assert!(matches!(
            pom.effect,
            Mech::NextCast {
                instant: true,
                crit: true,
                ..
            }
        ));
        for (id, aura) in [(75, CLEARCASTING), (788, CLEARCASTING_DRUID)] {
            let spec = at_rank(id, 1).pop().unwrap();
            assert!(spec.validate().is_ok(), "{id}");
            assert_eq!(
                spec.effect,
                Mech::NextCast {
                    instant: false,
                    crit: false,
                    cooldown: 0.5,
                    aura,
                }
            );
            assert!(spec.cooldown > 0.0, "{id}: Clearcasting cannot chain");
        }
        let Trigger::OnHit { chance, .. } = at_rank(75, 5).pop().unwrap().trigger else {
            panic!("a hit proc")
        };
        assert!((chance - 0.10).abs() < 1e-6, "2% a rank, as 1.12's");
    }

    /// Fire, frost, warlock, priest and shaman talents do what their 1.12 talents did.
    #[test]
    fn the_signature_talents_are_their_mechanics() {
        assert!(matches!(
            at_rank(34, 5).pop().unwrap().effect,
            Mech::Ignite { per_burn, .. } if per_burn > 0.0
        ));
        let shatter = at_rank(67, 5).pop().unwrap();
        assert!(matches!(shatter.effect, Mech::Shatter { .. }));
        assert_eq!(
            shatter.trigger,
            Trigger::OnCrit {
                of: Of::Any,
                chance: 1.0
            },
            "every crit at 5/5"
        );
        assert!(matches!(
            at_rank(968, 1).pop().unwrap().effect,
            Mech::ConsumeDot {
                school: School::Fire,
                ..
            }
        ));
        let Mech::Burst { bonuses, .. } = at_rank(521, 1).pop().unwrap().effect else {
            panic!("Shadowform is held all run")
        };
        assert!(bonuses.contains(&Bonus::new(StatKey::Armor, 0.15)));
        assert!(bonuses.contains(&Bonus::school(School::Shadow, 0.15)));
        assert!(matches!(
            at_rank(484, 1).pop().unwrap().effect,
            Mech::ShareHeal {
                school: School::Shadow,
                ..
            }
        ));
        assert!(matches!(
            at_rank(582, 1).pop().unwrap().effect,
            Mech::TotemReach { mult } if mult > 1.0
        ));
        for m in MECHANICS {
            for spec in m.specs {
                spec.validate()
                    .unwrap_or_else(|e| panic!("{} {}: {e}", m.id, m.name));
            }
        }
    }

    /// A proc talent's tooltip names its proc, as WoW's do, rank by rank.
    #[test]
    fn proc_tooltips_name_their_procs() {
        assert_eq!(
            describe(30, 5),
            ["Your hits have a 10% chance to proc Impact: stuns the target for 1.5 sec."]
        );
        assert_eq!(
            describe(38, 3),
            ["Your hits have a 9% chance to proc Frostbite: roots the target for 2 sec."]
        );
        assert_eq!(
            describe(34, 1),
            [
                "Your critical strikes proc Ignite: the target burns for 8 Fire damage a second for \
              4 sec, 15% more for each Fire burn already on it."
            ]
        );
        assert_eq!(
            describe(75, 5),
            [
                "Your hits have a 10% chance to proc Clearcasting (once every 3 sec): your next \
              cast's cooldown is halved."
            ]
        );
        assert_eq!(
            describe(86, 1),
            [
                "Every 30 sec: your slowest ability is ready at once, and your next cast strikes \
              critically with every blow."
            ]
        );
        assert_eq!(
            describe(582, 1),
            ["Your totems reach 50% farther, their reach ringed on the ground."]
        );
        assert_eq!(
            describe(484, 1),
            ["Every 2 sec: 15% of the Shadow damage you dealt heals you and allies within 40 yd."]
        );
        for m in MECHANICS {
            for line in describe(m.id, 1) {
                assert!(!line.contains("luck"), "{}: {line}", m.name);
            }
        }
    }
}
