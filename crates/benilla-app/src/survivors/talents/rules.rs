//! Vanilla's point rules over one class's [`Allocation`]: [`MAX_POINTS`] in all, a tier opening at
//! [`POINTS_PER_TIER`] points per tier already in its tree, a prerequisite at the rank it names,
//! and no rank past the talent's last. An unlearn may not leave the allocation breaking any of
//! them. [`bonuses`] folds an allocation into the shared stat vocabulary.

use std::collections::BTreeMap;

use super::super::bonus::{Bonus, Bonuses};
use super::{effects, TalentCatalog};

/// The points a level-60 character has (levels 10 to 60).
pub(in crate::survivors) const MAX_POINTS: u32 = 51;
/// Points per tier: tier `n` opens at `n` × this many points spent in its tree.
pub(in crate::survivors) const POINTS_PER_TIER: u32 = 5;

/// Talent id to rank, for one class: the rules never mix trees across classes, so neither should
/// a caller.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct Allocation {
    ranks: BTreeMap<u32, u8>,
}

/// Why a rank cannot be learned or unlearned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Why {
    /// No talent with that id in the catalog.
    Unknown,
    /// All [`MAX_POINTS`] are spent; the Talents tab also says so when every earned point is.
    NoPoints,
    /// The talent is at its last rank.
    MaxRank,
    /// The tier needs `need` points in the tree, which has `have`.
    TierLocked { need: u32, have: u32 },
    /// The prerequisite `talent` must reach `rank` first.
    Prereq { talent: u32, rank: u8 },
    /// Unlearning a talent with no ranks.
    NotLearned,
    /// Talent `by` needs this one at its current rank.
    Required { by: u32 },
    /// Talent `by`, deeper in the tree, would lose the points that opened its tier.
    Holds { by: u32 },
}

impl Allocation {
    pub fn rank(&self, talent: u32) -> u8 {
        self.ranks.get(&talent).copied().unwrap_or(0)
    }

    /// Points spent in all trees.
    pub fn spent(&self) -> u32 {
        self.ranks.values().map(|&r| u32::from(r)).sum()
    }

    /// Points spent in one tree.
    pub fn spent_in(&self, catalog: &TalentCatalog, tree: u32) -> u32 {
        catalog
            .talents_in(tree)
            .map(|t| u32::from(self.rank(t.id)))
            .sum()
    }

    /// Learned talents and their ranks, by id.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u8)> + '_ {
        self.ranks.iter().map(|(&id, &rank)| (id, rank))
    }

    /// One more rank, if [`can_learn`] allows it.
    pub fn learn(&mut self, catalog: &TalentCatalog, talent: u32) -> Result<(), Why> {
        can_learn(catalog, self, talent)?;
        *self.ranks.entry(talent).or_default() += 1;
        Ok(())
    }

    /// One rank fewer, if [`can_unlearn`] allows it.
    pub fn unlearn(&mut self, catalog: &TalentCatalog, talent: u32) -> Result<(), Why> {
        can_unlearn(catalog, self, talent)?;
        self.lower(talent);
        Ok(())
    }

    fn lower(&mut self, talent: u32) {
        if let Some(rank) = self.ranks.get_mut(&talent) {
            *rank -= 1;
            if *rank == 0 {
                self.ranks.remove(&talent);
            }
        }
    }
}

/// Ranks by talent id as a save lists them, unchecked: a later pair for an id replaces an earlier
/// one, and rank 0 is no rank.
impl FromIterator<(u32, u8)> for Allocation {
    fn from_iter<I: IntoIterator<Item = (u32, u8)>>(pairs: I) -> Self {
        let mut ranks = BTreeMap::new();
        for (id, rank) in pairs {
            if rank == 0 {
                ranks.remove(&id);
            } else {
                ranks.insert(id, rank);
            }
        }
        Self { ranks }
    }
}

/// Whether `talent` can take one more rank.
pub(in crate::survivors) fn can_learn(
    catalog: &TalentCatalog,
    allocation: &Allocation,
    talent: u32,
) -> Result<(), Why> {
    let t = catalog.talent(talent).ok_or(Why::Unknown)?;
    if allocation.rank(talent) >= t.max_rank {
        return Err(Why::MaxRank);
    }
    if allocation.spent() >= MAX_POINTS {
        return Err(Why::NoPoints);
    }
    let need = u32::from(t.tier) * POINTS_PER_TIER;
    let have = allocation.spent_in(catalog, t.tree);
    if have < need {
        return Err(Why::TierLocked { need, have });
    }
    if let Some((p, rank)) = t.prereq {
        if allocation.rank(p) < rank {
            return Err(Why::Prereq { talent: p, rank });
        }
    }
    Ok(())
}

/// Whether `talent` can give back one rank and leave every other rank learnable: no talent still
/// needs it at this rank, and every deeper tier keeps its points in the tiers above it.
pub(in crate::survivors) fn can_unlearn(
    catalog: &TalentCatalog,
    allocation: &Allocation,
    talent: u32,
) -> Result<(), Why> {
    let t = catalog.talent(talent).ok_or(Why::Unknown)?;
    let rank = allocation.rank(talent);
    if rank == 0 {
        return Err(Why::NotLearned);
    }
    let mut after = allocation.clone();
    after.lower(talent);
    for other in catalog.talents_in(t.tree) {
        if after.rank(other.id) == 0 {
            continue;
        }
        if let Some((p, need)) = other.prereq {
            if p == talent && rank - 1 < need {
                return Err(Why::Required { by: other.id });
            }
        }
        // Learning checks the whole tree's points, but a learned rank stays valid only while
        // the tiers above it hold enough: every learn order that reaches it put them there.
        let above: u32 = catalog
            .talents_in(t.tree)
            .filter(|x| x.tier < other.tier)
            .map(|x| u32::from(after.rank(x.id)))
            .sum();
        if above < u32::from(other.tier) * POINTS_PER_TIER {
            return Err(Why::Holds { by: other.id });
        }
    }
    Ok(())
}

/// An allocation's summed bonuses: rank `n` of a talent grants `n` times its per-rank bonuses.
/// Ids the catalog lacks are skipped, and ranks past a talent's last are not counted.
pub(in crate::survivors) fn bonuses(catalog: &TalentCatalog, allocation: &Allocation) -> Bonuses {
    let mut out = Bonuses::default();
    for (id, rank) in allocation.iter() {
        let Some(t) = catalog.talent(id) else {
            continue;
        };
        let n = f32::from(rank.min(t.max_rank));
        for b in effects::per_rank(id).unwrap_or(&[]) {
            out.add(&Bonus {
                value: b.value * n,
                ..*b
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::super::bonus::{School, StatKey};
    use super::super::{Talent, Tree};
    use super::*;

    // Real ids, so `bonuses` reads the real table: Toughness, Fire Power, Impact.
    const TOUGH: u32 = 140;
    const FIRE_POWER: u32 = 35;
    const IMPACT: u32 = 30;

    fn talent(id: u32, tree: u32, tier: u8, max_rank: u8, prereq: Option<(u32, u8)>) -> Talent {
        Talent {
            id,
            tree,
            tier,
            column: 0,
            max_rank,
            spells: (1..=u32::from(max_rank)).map(|r| id * 10 + r).collect(),
            prereq,
            name: format!("t{id}"),
            icon: None,
        }
    }

    /// Tree 1 has made-up ids `9000 + 10 × tier + column` past every real talent, 57 points
    /// deep: 9012 needs 9011 at 3/3 and 9060 needs 9050 at 1/1. Tree 2 has Toughness, Fire
    /// Power and Impact at tier 0.
    fn catalog() -> TalentCatalog {
        let tree = |id, order| Tree {
            id,
            class: 0,
            name: format!("tree {id}"),
            order,
            icon: None,
            background: String::new(),
        };
        let talents = vec![
            talent(9000, 1, 0, 5, None),
            talent(9001, 1, 0, 5, None),
            talent(9010, 1, 1, 5, None),
            talent(9011, 1, 1, 3, None),
            talent(9012, 1, 1, 2, Some((9011, 3))),
            talent(9020, 1, 2, 5, None),
            talent(9021, 1, 2, 5, None),
            talent(9030, 1, 3, 5, None),
            talent(9031, 1, 3, 5, None),
            talent(9040, 1, 4, 5, None),
            talent(9041, 1, 4, 5, None),
            talent(9050, 1, 5, 1, None),
            talent(9051, 1, 5, 5, None),
            talent(9060, 1, 6, 1, Some((9050, 1))),
            talent(TOUGH, 2, 0, 5, None),
            talent(FIRE_POWER, 2, 0, 5, None),
            talent(IMPACT, 2, 0, 5, None),
        ];
        TalentCatalog::new(vec![tree(1, 0), tree(2, 1)], talents)
    }

    fn learn_n(a: &mut Allocation, cat: &TalentCatalog, id: u32, n: u8) {
        for _ in 0..n {
            a.learn(cat, id)
                .unwrap_or_else(|e| panic!("learn {id}: {e:?}"));
        }
    }

    /// Tree 1 at 47 points through tier 6, learned tier by tier, and Toughness at `tough`.
    fn deep(cat: &TalentCatalog, tough: u8) -> Allocation {
        let mut a = Allocation::default();
        for (id, n) in [
            (9000, 5),
            (9001, 5),
            (9010, 5),
            (9011, 3),
            (9012, 2),
            (9020, 5),
            (9030, 5),
            (9040, 5),
            (9041, 5),
            (9050, 1),
            (9051, 5),
            (9060, 1),
            (TOUGH, tough),
        ] {
            learn_n(&mut a, cat, id, n);
        }
        assert_eq!(a.spent_in(cat, 1), 47);
        a
    }

    #[test]
    fn ranks_stop_at_the_last() {
        let cat = catalog();
        let mut a = Allocation::default();
        learn_n(&mut a, &cat, 9000, 5);
        assert_eq!(a.rank(9000), 5);
        assert_eq!(can_learn(&cat, &a, 9000), Err(Why::MaxRank));
        assert_eq!(can_learn(&cat, &a, 999), Err(Why::Unknown));
    }

    #[test]
    fn a_tier_opens_at_five_points_per_tier_in_its_tree() {
        let cat = catalog();
        let mut a = Allocation::default();
        learn_n(&mut a, &cat, 9000, 4);
        let locked = Err(Why::TierLocked { need: 5, have: 4 });
        assert_eq!(can_learn(&cat, &a, 9010), locked);
        // Points in another tree do not count.
        learn_n(&mut a, &cat, TOUGH, 5);
        assert_eq!(can_learn(&cat, &a, 9010), locked);
        learn_n(&mut a, &cat, 9001, 1);
        assert_eq!(can_learn(&cat, &a, 9010), Ok(()));
        assert_eq!(
            can_learn(&cat, &a, 9040),
            Err(Why::TierLocked { need: 20, have: 5 })
        );
        // Points at any tier count toward the next.
        learn_n(&mut a, &cat, 9010, 5);
        assert_eq!(can_learn(&cat, &a, 9020), Ok(()));
        assert_eq!(
            can_learn(&cat, &a, 9040),
            Err(Why::TierLocked { need: 20, have: 10 })
        );
        assert_eq!(a.spent_in(&cat, 1), 10);
        assert_eq!(a.spent(), 15);
    }

    #[test]
    fn a_prerequisite_needs_its_full_rank() {
        let cat = catalog();
        let mut a = Allocation::default();
        learn_n(&mut a, &cat, 9000, 5);
        learn_n(&mut a, &cat, 9011, 2);
        assert_eq!(
            can_learn(&cat, &a, 9012),
            Err(Why::Prereq {
                talent: 9011,
                rank: 3
            })
        );
        learn_n(&mut a, &cat, 9011, 1);
        assert_eq!(can_learn(&cat, &a, 9012), Ok(()));
    }

    #[test]
    fn fifty_one_points_in_all() {
        let cat = catalog();
        let mut a = deep(&cat, 4);
        assert_eq!(a.spent(), MAX_POINTS);
        assert_eq!(can_learn(&cat, &a, TOUGH), Err(Why::NoPoints));
        assert_eq!(a.learn(&cat, 9001), Err(Why::MaxRank));
        assert_eq!(a.learn(&cat, 9061), Err(Why::Unknown));
        a.unlearn(&cat, TOUGH).unwrap();
        assert_eq!(can_learn(&cat, &a, IMPACT), Ok(()));
    }

    #[test]
    fn unlearning_keeps_the_allocation_valid() {
        let cat = catalog();
        let mut a = deep(&cat, 0);
        assert_eq!(
            can_unlearn(&cat, &a, 9001),
            Ok(()),
            "tier 0 has points to spare"
        );
        assert_eq!(can_unlearn(&cat, &a, 99), Err(Why::Unknown));
        assert_eq!(can_unlearn(&cat, &a, TOUGH), Err(Why::NotLearned));
        assert_eq!(can_unlearn(&cat, &a, 9011), Err(Why::Required { by: 9012 }));
        assert_eq!(can_unlearn(&cat, &a, 9050), Err(Why::Required { by: 9060 }));
        // A leaf gives its point back, and then its prerequisite can follow.
        a.unlearn(&cat, 9060).unwrap();
        a.unlearn(&cat, 9050).unwrap();
        assert_eq!(a.rank(9050), 0);

        // Learnable on the tree's total, but tier 0 must keep the 5 that opened tier 1, and
        // tiers 0-1 the 10 that opened tier 2.
        let mut a = Allocation::default();
        learn_n(&mut a, &cat, 9000, 5);
        learn_n(&mut a, &cat, 9010, 5);
        learn_n(&mut a, &cat, 9020, 1);
        assert_eq!(can_unlearn(&cat, &a, 9000), Err(Why::Holds { by: 9010 }));
        assert_eq!(can_unlearn(&cat, &a, 9010), Err(Why::Holds { by: 9020 }));
        a.unlearn(&cat, 9020).unwrap();
        a.unlearn(&cat, 9010).unwrap();
        // A tier's own points never hold it open.
        assert_eq!(can_unlearn(&cat, &a, 9000), Err(Why::Holds { by: 9010 }));
        learn_n(&mut a, &cat, 9001, 1);
        assert_eq!(can_unlearn(&cat, &a, 9000), Ok(()));
    }

    #[test]
    fn bonuses_scale_with_rank() {
        let cat = catalog();
        let mut a = Allocation::default();
        learn_n(&mut a, &cat, TOUGH, 3);
        learn_n(&mut a, &cat, FIRE_POWER, 2);
        // Tree 1's made-up ids have no effects and add nothing.
        learn_n(&mut a, &cat, 9000, 5);
        let got = bonuses(&cat, &a);
        let mut want = Bonuses::default();
        for (id, n) in [(TOUGH, 3.0), (FIRE_POWER, 2.0)] {
            for b in effects::per_rank(id).unwrap() {
                want.add(&Bonus {
                    value: b.value * n,
                    ..*b
                });
            }
        }
        assert_eq!(got, want);
        assert!(got.get(StatKey::Armor) > 0.0, "Toughness is armor");
        assert!(got.school_damage(School::Fire) > 0.0, "Fire Power is fire");
        assert_eq!(bonuses(&cat, &Allocation::default()), Bonuses::default());
    }
}
