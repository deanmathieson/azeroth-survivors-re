//! What the Armory's gear tabs show, as plain data built from the catalog, the drop tables and a
//! class's [`GearSave`], so it is tested without a screen.

use super::super::bonus::Bonus;
use super::super::data::BOSS_EVERY_SECS;
use super::{drops, items_for, piece, GearSave, Item, Rarity};

/// Every rarity, lowest first.
pub(in crate::survivors) const RARITIES: [Rarity; 5] = [
    Rarity::Common,
    Rarity::Uncommon,
    Rarity::Rare,
    Rarity::Epic,
    Rarity::Legendary,
];

/// A rarity's name, as the item tooltip's quality reads.
pub(in crate::survivors) fn rarity_label(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Common => "Common",
        Rarity::Uncommon => "Uncommon",
        Rarity::Rare => "Rare",
        Rarity::Epic => "Epic",
        Rarity::Legendary => "Legendary",
    }
}

/// One boss that drops a piece.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Source {
    /// The battleground's name.
    pub zone: &'static str,
    pub boss: &'static str,
    /// Per kill, before the rewards multiplier and luck.
    pub chance: f32,
}

/// Every boss that drops `item`, in battleground and roster order; a raid piece's Molten Core
/// bosses, each with its odds for a solo kill.
pub(in crate::survivors) fn sources(item: &Item) -> Vec<Source> {
    if super::raid_piece(item.id).is_some() {
        return super::raid::sources(item.id)
            .into_iter()
            .map(|(boss, chance)| Source {
                zone: "Molten Core",
                boss,
                chance,
            })
            .collect();
    }
    drops::ZONES
        .iter()
        .flat_map(|z| z.bosses.iter().map(move |d| (z.map, d)))
        .filter(|(_, d)| d.rarity == item.rarity && d.slots.contains(&item.slot))
        .map(|(zone, d)| Source {
            zone,
            boss: d.boss,
            chance: d.chance,
        })
        .collect()
}

/// The battlegrounds with a boss that drops `rarity`, in their order.
pub(in crate::survivors) fn zones_dropping(rarity: Rarity) -> Vec<&'static str> {
    drops::ZONES
        .iter()
        .filter(|z| z.bosses.iter().any(|d| d.rarity == rarity))
        .map(|z| z.map)
        .collect()
}

/// How many of `pieces` `gear` has collected, and how many there are.
fn tally<'a>(gear: &GearSave, pieces: impl Iterator<Item = &'a Item>) -> (usize, usize) {
    pieces.fold((0, 0), |(have, of), p| {
        (have + usize::from(gear.has(p.id)), of + 1)
    })
}

/// Class `class`'s pieces collected and in all, of `rarity` alone or of every rarity.
pub(in crate::survivors) fn progress(
    class: usize,
    gear: &GearSave,
    rarity: Option<Rarity>,
) -> (usize, usize) {
    tally(
        gear,
        items_for(class)
            .iter()
            .filter(|i| rarity.is_none_or(|r| i.rarity == r)),
    )
}

/// One battleground in the Drop Tables' list.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::survivors) struct ZoneRow {
    /// An index into [`super::super::data::MAPS`].
    pub map: usize,
    pub name: &'static str,
    /// What most of its bosses drop; a last boss may drop better.
    pub rarity: Rarity,
    /// The class's distinct pieces its bosses drop: collected, and in all.
    pub collected: usize,
    pub total: usize,
}

/// Every battleground, for class `class`.
pub(in crate::survivors) fn zone_rows(class: usize, gear: &GearSave) -> Vec<ZoneRow> {
    drops::ZONES
        .iter()
        .enumerate()
        .map(|(map, zone)| {
            let mut pieces: Vec<&Item> = zone
                .bosses
                .iter()
                .flat_map(|d| d.slots.iter().filter_map(|&s| piece(class, s, d.rarity)))
                .collect();
            pieces.sort_by_key(|p| p.id);
            pieces.dedup_by_key(|p| p.id);
            let (collected, total) = tally(gear, pieces.into_iter());
            ZoneRow {
                map,
                name: zone.map,
                rarity: zone.bosses.first().map_or(Rarity::Common, |d| d.rarity),
                collected,
                total,
            }
        })
        .collect()
}

/// One piece a boss drops.
#[derive(Clone, Debug)]
pub(in crate::survivors) struct PieceRow {
    pub item: &'static Item,
    /// Per kill, before the rewards multiplier and luck.
    pub chance: f32,
    pub collected: bool,
}

/// One boss of a battleground, for the Drop Tables.
#[derive(Clone, Debug)]
pub(in crate::survivors) struct BossRow {
    pub name: &'static str,
    /// Run seconds it walks in at: one every [`BOSS_EVERY_SECS`], the first after the first wait.
    pub secs: u32,
    /// The battleground's final boss.
    pub last: bool,
    pub pieces: Vec<PieceRow>,
}

/// Battleground `map`'s bosses in roster order, with class `class`'s pieces; empty past the
/// battlegrounds.
pub(in crate::survivors) fn boss_rows(map: usize, class: usize, gear: &GearSave) -> Vec<BossRow> {
    let Some(zone) = drops::ZONES.get(map) else {
        return Vec::new();
    };
    let n = zone.bosses.len();
    zone.bosses
        .iter()
        .enumerate()
        .map(|(i, d)| BossRow {
            name: d.boss,
            // `spawn_enemies` brings boss `i` once `elapsed / BOSS_EVERY_SECS` reaches `i + 1`.
            secs: ((i + 1) as f32 * BOSS_EVERY_SECS).round() as u32,
            last: i + 1 == n,
            pieces: d
                .slots
                .iter()
                .filter_map(|&s| piece(class, s, d.rarity))
                .map(|item| PieceRow {
                    item,
                    chance: d.chance,
                    collected: gear.has(item.id),
                })
                .collect(),
        })
        .collect()
}

/// Seconds as the run clock reads them, `m:ss`.
pub(in crate::survivors) fn clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// A chance as a whole percentage.
pub(in crate::survivors) fn percent(chance: f32) -> String {
    format!("{}%", (chance * 100.0).round() as u32)
}

/// The pieces `ids`' bonuses with like ones (same stat and school) summed, in the order each
/// first appears.
pub(in crate::survivors) fn summed(ids: &[u16]) -> Vec<Bonus> {
    let mut out: Vec<Bonus> = Vec::new();
    for b in ids
        .iter()
        .filter_map(|&id| super::item(id))
        .flat_map(|i| i.bonuses)
    {
        match out
            .iter_mut()
            .find(|o| o.stat == b.stat && o.school == b.school)
        {
            Some(o) => o.value += b.value,
            None => out.push(*b),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::super::bonus::{Bonuses, StatKey};
    use super::super::super::data::{roster, CLASSES, MAPS};
    use super::super::{bonuses_of, Slot};
    use super::*;

    #[test]
    fn boss_rows_time_and_mark_the_bosses() {
        let naxx = MAPS.iter().position(|m| m.name == "Naxxramas").unwrap();
        let rows = boss_rows(naxx, 6, &GearSave::default());
        assert_eq!(rows.len(), roster(naxx).bosses.len());
        assert_eq!(rows[0].name, "Anub'Rekhan");
        assert_eq!(clock(rows[0].secs), "2:15");
        assert_eq!(clock(rows[1].secs), "4:30");
        assert_eq!(clock(rows[5].secs), "13:30");
        assert!(rows[5].last && rows[..5].iter().all(|r| !r.last));
        let kt = &rows[5];
        assert_eq!(kt.name, "Kel'Thuzad");
        assert_eq!(kt.pieces.len(), 5);
        assert!(kt
            .pieces
            .iter()
            .all(|p| p.item.rarity == Rarity::Legendary && p.item.class == 6 && !p.collected));
        assert_eq!(percent(kt.pieces[0].chance), "60%");
        assert!(boss_rows(MAPS.len(), 0, &GearSave::default()).is_empty());
    }

    #[test]
    fn zone_rows_count_distinct_pieces_collected() {
        let class = 3;
        let mut gear = GearSave::default();
        let rows = zone_rows(class, &gear);
        assert_eq!(rows.len(), MAPS.iter().filter(|m| m.map != 409).count());
        for (m, row) in rows.iter().enumerate() {
            assert_eq!((row.map, row.name), (m, MAPS[m].name));
            assert_eq!(row.collected, 0);
        }
        let elwynn = rows.iter().find(|r| r.name == "Elwynn Forest").unwrap();
        assert_eq!((elwynn.rarity, elwynn.total), (Rarity::Common, 9));
        // Mount Hyjal drops Legs and Head twice: still nine pieces.
        let hyjal = rows.iter().find(|r| r.name == "Mount Hyjal").unwrap();
        assert_eq!((hyjal.rarity, hyjal.total), (Rarity::Epic, 9));
        // Naxxramas: Epic in all nine slots, and Kel'Thuzad's five Legendaries.
        let naxx = rows.iter().find(|r| r.name == "Naxxramas").unwrap();
        assert_eq!((naxx.rarity, naxx.total), (Rarity::Epic, 14));

        // Collecting Elwynn's chest counts in every Common zone, and in no other.
        gear.collect(piece(class, Slot::Chest, Rarity::Common).unwrap());
        for row in zone_rows(class, &gear) {
            let want = usize::from(row.rarity == Rarity::Common);
            assert_eq!(row.collected, want, "{}", row.name);
        }
        assert_eq!(progress(class, &gear, None), (1, 45));
        assert_eq!(progress(class, &gear, Some(Rarity::Common)), (1, 9));
        assert_eq!(progress(class, &gear, Some(Rarity::Epic)), (0, 9));
    }

    #[test]
    fn sources_and_zones_follow_the_drop_tables() {
        for class in 0..CLASSES.len() {
            for i in items_for(class) {
                assert!(!sources(i).is_empty(), "{}", i.name);
            }
        }
        let thunderfury = piece(0, Slot::MainHand, Rarity::Legendary).unwrap();
        let from = sources(thunderfury);
        assert!(from.iter().all(|s| s.chance > 0.0));
        assert_eq!(
            zones_dropping(Rarity::Legendary),
            vec!["Gates of Ahn'Qiraj", "Naxxramas"]
        );
        assert_eq!(zones_dropping(Rarity::Common).len(), 6);
    }

    #[test]
    fn summed_bonuses_match_the_bonus_totals() {
        let ids: Vec<u16> = items_for(4).iter().map(|i| i.id).collect();
        let lines = summed(&ids);
        let mut again = Bonuses::default();
        again.add_all(&lines);
        assert_eq!(again, bonuses_of(&ids));
        // Like bonuses are folded into one line.
        for (i, a) in lines.iter().enumerate() {
            assert!(lines[i + 1..]
                .iter()
                .all(|b| (b.stat, b.school) != (a.stat, a.school)));
        }
        assert!(summed(&[]).is_empty());
        assert!(lines.iter().any(|b| b.stat == StatKey::Damage));
    }
}
