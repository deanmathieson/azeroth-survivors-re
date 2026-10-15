//! Perks: account-wide boosts bought rank by rank with reward points, each rank dearer than the
//! last. Stat perks reach every run as [`bonuses`]; the reroll and banish perks add to the run's
//! hand ([`extra_rerolls`], [`extra_banishes`]); the Companion perk brings
//! [`super::companion`] along.
//!
//! A purchase is not final at once: a rank bought since the Rewards tab opened (the
//! [`PerkSession`]) refunds one at a time, and anything older comes back only through a reset, of
//! which an account has [`super::save::PERK_RESETS`].

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::super::bonus::{Bonus, Bonuses, StatKey};
use super::save::RewardsSave;

/// What a perk's ranks do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Effect {
    /// Each rank adds this bonus to every run.
    Stat(Bonus),
    /// Each rank adds a reroll to every run's level-up hand.
    Reroll,
    /// Each rank adds a banish.
    Banish,
    /// The one rank brings the companion on every run.
    Companion,
}

/// One perk.
pub(in crate::survivors) struct Perk {
    /// The stable key a save keeps its ranks under.
    pub(in crate::survivors) id: &'static str,
    pub(in crate::survivors) name: &'static str,
    pub(in crate::survivors) desc: &'static str,
    /// Each rank's cost in turn: as many ranks as costs.
    pub(in crate::survivors) costs: &'static [u32],
    pub(in crate::survivors) effect: Effect,
}

/// Every perk, in the Rewards tab's order.
pub(in crate::survivors) static PERKS: &[Perk] = &[
    Perk {
        id: "extra_reroll",
        name: "Extra Reroll",
        desc: "Deal a level-up hand again once more a run.",
        costs: &[15, 30, 50],
        effect: Effect::Reroll,
    },
    Perk {
        id: "extra_banish",
        name: "Extra Banish",
        desc: "Strike one more choice from a run's offers.",
        costs: &[20, 40],
        effect: Effect::Banish,
    },
    Perk {
        id: "lucky",
        name: "Lucky",
        desc: "Rarer cards turn up more often.",
        costs: &[10, 20, 35],
        effect: Effect::Stat(Bonus::new(StatKey::Luck, 0.05)),
    },
    Perk {
        id: "prosperous",
        name: "Prosperous",
        desc: "Every coin you pick up is worth more.",
        costs: &[10, 20, 35],
        effect: Effect::Stat(Bonus::new(StatKey::Gold, 0.10)),
    },
    Perk {
        id: "hardy",
        name: "Hardy",
        desc: "Start every run with more health.",
        costs: &[10, 20, 35],
        effect: Effect::Stat(Bonus::new(StatKey::MaxHealth, 20.0)),
    },
    Perk {
        id: "fleet",
        name: "Fleet",
        desc: "Run a little faster.",
        costs: &[10, 20, 35],
        effect: Effect::Stat(Bonus::new(StatKey::MoveSpeed, 0.03)),
    },
    Perk {
        id: "studious",
        name: "Studious",
        desc: "Learn more from every experience gem.",
        costs: &[10, 20, 35],
        effect: Effect::Stat(Bonus::new(StatKey::Experience, 0.05)),
    },
    Perk {
        id: "companion",
        name: "Companion",
        desc: "A critter follows you, fetches pickups, and levels to learn skills.",
        costs: &[25],
        effect: Effect::Companion,
    },
];

impl Perk {
    pub(in crate::survivors) fn max_rank(&self) -> u32 {
        self.costs.len() as u32
    }

    /// The ranks `save` holds, never past the top.
    pub(in crate::survivors) fn rank(&self, save: &RewardsSave) -> u32 {
        save.perks
            .get(self.id)
            .copied()
            .unwrap_or(0)
            .min(self.max_rank())
    }

    /// The next rank's cost; `None` at the top rank.
    pub(in crate::survivors) fn next_cost(&self, save: &RewardsSave) -> Option<u32> {
        self.costs.get(self.rank(save) as usize).copied()
    }

    /// What `rank` ranks grant, as the Rewards tab words it: "+10% luck", "+2 rerolls a run".
    /// `None` at rank 0.
    pub(in crate::survivors) fn effect_at(&self, rank: u32) -> Option<String> {
        if rank == 0 {
            return None;
        }
        let count =
            |one: &str, many: &str| format!("+{rank} {} a run", if rank == 1 { one } else { many });
        Some(match self.effect {
            Effect::Stat(bonus) => Bonus {
                value: bonus.value * rank as f32,
                ..bonus
            }
            .describe(),
            Effect::Reroll => count("reroll", "rerolls"),
            Effect::Banish => count("banish", "banishes"),
            Effect::Companion => "A companion on every run".into(),
        })
    }
}

/// Points left to spend: the earned achievements' points and any test-aid points, less what
/// perks cost. Never below zero, even if `spent` outgrew them (test-aid points spent and gone).
pub(in crate::survivors) fn available(earned: u32, extra: u32, spent: u32) -> u32 {
    earned.saturating_add(extra).saturating_sub(spent)
}

/// Every stat perk's bonus at the ranks `save` holds.
pub(in crate::survivors) fn bonuses(save: &RewardsSave) -> Bonuses {
    let mut out = Bonuses::default();
    for perk in PERKS {
        let rank = perk.rank(save);
        if let (Effect::Stat(bonus), true) = (perk.effect, rank > 0) {
            out.add(&Bonus {
                value: bonus.value * rank as f32,
                ..bonus
            });
        }
    }
    out
}

/// The ranks `save` holds of every perk with `effect`.
fn ranks_of(save: &RewardsSave, effect: Effect) -> u32 {
    PERKS
        .iter()
        .filter(|p| p.effect == effect)
        .map(|p| p.rank(save))
        .sum()
}

/// Rerolls each run starts with on top of the hand's own.
pub(in crate::survivors) fn extra_rerolls(save: &RewardsSave) -> u32 {
    ranks_of(save, Effect::Reroll)
}

/// Banishes each run starts with on top of the hand's own.
pub(in crate::survivors) fn extra_banishes(save: &RewardsSave) -> u32 {
    ranks_of(save, Effect::Banish)
}

/// Whether the companion comes along.
pub(in crate::survivors) fn companion_unlocked(save: &RewardsSave) -> bool {
    ranks_of(save, Effect::Companion) > 0
}

/// The Rewards tab's editing session: the ranks bought since it opened, which may still be
/// refunded one at a time. Memory only; emptied as the tab closes, which commits them.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct PerkSession {
    bought: BTreeMap<&'static str, u32>,
}

impl PerkSession {
    pub(in crate::survivors) fn is_empty(&self) -> bool {
        self.bought.is_empty()
    }

    pub(in crate::survivors) fn clear(&mut self) {
        self.bought.clear();
    }

    fn bought(&self, perk: &Perk) -> u32 {
        self.bought.get(perk.id).copied().unwrap_or(0)
    }

    /// Buy `perk`'s next rank out of `points` to spend: what it cost, or `None` at the top rank or
    /// short of points.
    pub(in crate::survivors) fn buy(
        &mut self,
        save: &mut RewardsSave,
        perk: &'static Perk,
        points: u32,
    ) -> Option<u32> {
        let cost = perk.next_cost(save).filter(|&cost| cost <= points)?;
        let rank = perk.rank(save) + 1;
        save.perks.insert(perk.id.into(), rank);
        save.spent = save.spent.saturating_add(cost);
        *self.bought.entry(perk.id).or_default() += 1;
        Some(cost)
    }

    /// Whether `perk`'s top rank was bought this session, so it may be refunded.
    pub(in crate::survivors) fn can_refund(&self, save: &RewardsSave, perk: &Perk) -> bool {
        self.bought(perk) > 0 && perk.rank(save) > 0
    }

    /// Give back `perk`'s top rank, bought this session: what it cost, or `None`.
    pub(in crate::survivors) fn refund(
        &mut self,
        save: &mut RewardsSave,
        perk: &Perk,
    ) -> Option<u32> {
        if !self.can_refund(save, perk) {
            return None;
        }
        let rank = perk.rank(save) - 1;
        let cost = perk.costs[rank as usize];
        if rank == 0 {
            save.perks.remove(perk.id);
        } else {
            save.perks.insert(perk.id.into(), rank);
        }
        save.spent = save.spent.saturating_sub(cost);
        if let Some(n) = self.bought.get_mut(perk.id) {
            *n -= 1;
            if *n == 0 {
                self.bought.remove(perk.id);
            }
        }
        Some(cost)
    }

    /// Whether a rank predates the session: a reset then uses one of the account's resets. One
    /// that only takes back this session's ranks is free, as refunding them one by one would be.
    pub(in crate::survivors) fn committed(&self, save: &RewardsSave) -> bool {
        PERKS.iter().any(|p| p.rank(save) > self.bought(p))
    }

    /// Whether "Reset perks" would do anything now.
    pub(in crate::survivors) fn can_reset(&self, save: &RewardsSave) -> bool {
        let any = PERKS.iter().any(|p| p.rank(save) > 0);
        any && (save.resets_left > 0 || !self.committed(save))
    }

    /// Refund every rank: the points it gave back, or `None` with nothing to refund or no reset
    /// left for ranks that predate the session.
    pub(in crate::survivors) fn reset(&mut self, save: &mut RewardsSave) -> Option<u32> {
        if !self.can_reset(save) {
            return None;
        }
        if self.committed(save) {
            save.resets_left -= 1;
        }
        let refunded = save.spent;
        save.perks.clear();
        save.spent = 0;
        self.clear();
        Some(refunded)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::save::PERK_RESETS;
    use super::*;

    fn perk(id: &str) -> &'static Perk {
        PERKS.iter().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn the_table_is_well_formed_and_a_full_collection_buys_it_all() {
        let ids: HashSet<&str> = PERKS.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), PERKS.len());
        for p in PERKS {
            assert!(!p.costs.is_empty(), "{}", p.id);
            assert!(
                p.costs.windows(2).all(|w| w[0] < w[1]),
                "{} escalates",
                p.id
            );
        }
        let all: u32 = PERKS.iter().flat_map(|p| p.costs).sum();
        let earnable = crate::survivors::achievements::total_points();
        assert!(
            all <= earnable,
            "perks cost {all}, achievements give {earnable}"
        );
        assert_eq!(perk("extra_reroll").max_rank(), 3);
        assert_eq!(perk("extra_banish").max_rank(), 2);
        assert_eq!(perk("companion").max_rank(), 1);
    }

    #[test]
    fn points_are_earned_less_spent() {
        assert_eq!(available(100, 0, 30), 70);
        assert_eq!(available(100, 50, 30), 120);
        // Test-aid points spent and then gone: never negative.
        assert_eq!(available(10, 0, 40), 0);
    }

    #[test]
    fn buying_escalates_and_never_overspends() {
        let mut save = RewardsSave::default();
        let mut session = PerkSession::default();
        let lucky = perk("lucky");
        assert_eq!(session.buy(&mut save, lucky, 9), None, "short of points");
        assert_eq!(save, RewardsSave::default());
        assert_eq!(session.buy(&mut save, lucky, 10), Some(10));
        assert_eq!(session.buy(&mut save, lucky, 100), Some(20));
        assert_eq!(lucky.next_cost(&save), Some(35));
        assert_eq!(session.buy(&mut save, lucky, 34), None);
        assert_eq!(session.buy(&mut save, lucky, 35), Some(35));
        assert_eq!(lucky.rank(&save), 3);
        assert_eq!(session.buy(&mut save, lucky, 1_000), None, "top rank");
        assert_eq!(save.spent, 65);
        // The points a caller passes are what is left: earned less spent.
        let points = available(70, 0, save.spent);
        assert_eq!(points, 5);
        assert_eq!(session.buy(&mut save, perk("hardy"), points), None);
    }

    #[test]
    fn refunds_take_back_this_sessions_ranks_one_at_a_time() {
        let mut save = RewardsSave::default();
        let mut session = PerkSession::default();
        let fleet = perk("fleet");
        session.buy(&mut save, fleet, 100);
        session.buy(&mut save, fleet, 100);
        assert_eq!(save.spent, 30);
        assert_eq!(
            session.refund(&mut save, fleet),
            Some(20),
            "the top rank's cost"
        );
        assert_eq!((fleet.rank(&save), save.spent), (1, 10));
        assert_eq!(session.refund(&mut save, fleet), Some(10));
        assert_eq!((fleet.rank(&save), save.spent), (0, 0));
        assert!(save.perks.is_empty());
        assert_eq!(session.refund(&mut save, fleet), None, "nothing left");
        assert!(session.is_empty());

        // Closing the tab commits: an older rank does not refund.
        session.buy(&mut save, fleet, 100);
        session.clear();
        assert!(!session.can_refund(&save, fleet));
        assert_eq!(session.refund(&mut save, fleet), None);
        // A rank bought on top of it refunds; the older one stays.
        session.buy(&mut save, fleet, 100);
        assert_eq!(session.refund(&mut save, fleet), Some(20));
        assert_eq!(session.refund(&mut save, fleet), None);
        assert_eq!((fleet.rank(&save), save.spent), (1, 10));
    }

    #[test]
    fn resets_refund_everything_and_are_limited() {
        let mut save = RewardsSave::default();
        let mut session = PerkSession::default();
        assert!(!session.can_reset(&save), "nothing bought");
        assert_eq!(session.reset(&mut save), None);

        // This session's ranks alone: a free reset.
        session.buy(&mut save, perk("hardy"), 100);
        session.buy(&mut save, perk("companion"), 100);
        assert!(!session.committed(&save));
        assert_eq!(session.reset(&mut save), Some(35));
        assert_eq!(save.resets_left, PERK_RESETS);
        assert_eq!((save.spent, save.perks.len()), (0, 0));

        // Committed ranks spend a reset, until none are left.
        for left in (0..PERK_RESETS).rev() {
            session.buy(&mut save, perk("studious"), 100);
            session.clear();
            assert!(session.committed(&save));
            assert_eq!(session.reset(&mut save), Some(10));
            assert_eq!(save.resets_left, left);
            assert_eq!(perk("studious").rank(&save), 0);
        }
        session.buy(&mut save, perk("studious"), 100);
        session.clear();
        assert!(!session.can_reset(&save));
        assert_eq!(session.reset(&mut save), None);
        assert_eq!((perk("studious").rank(&save), save.spent), (1, 10));
    }

    #[test]
    fn stat_perks_become_bonuses() {
        let mut save = RewardsSave::default();
        assert_eq!(bonuses(&save), Bonuses::default());
        save.perks.insert("lucky".into(), 2);
        save.perks.insert("prosperous".into(), 1);
        save.perks.insert("hardy".into(), 3);
        save.perks.insert("fleet".into(), 1);
        save.perks.insert("studious".into(), 3);
        // Past the top rank, and a perk this build does not have.
        save.perks.insert("extra_banish".into(), 9);
        save.perks.insert("wide_hand".into(), 1);
        let b = bonuses(&save);
        let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
        assert!(close(b.get(StatKey::Luck), 0.10));
        assert!(close(b.get(StatKey::Gold), 0.10));
        assert!(close(b.get(StatKey::MaxHealth), 60.0));
        assert!(close(b.get(StatKey::MoveSpeed), 0.03));
        assert!(close(b.get(StatKey::Experience), 0.15));
        assert_eq!(b.get(StatKey::Damage), 0.0);
        assert_eq!(extra_banishes(&save), 2, "capped at the top rank");
        assert_eq!(extra_rerolls(&save), 0);
        assert!(!companion_unlocked(&save));
        save.perks.insert("extra_reroll".into(), 3);
        save.perks.insert("companion".into(), 1);
        assert_eq!(extra_rerolls(&save), 3);
        assert!(companion_unlocked(&save));
    }

    #[test]
    fn effects_read_as_the_tab_shows_them() {
        assert_eq!(perk("lucky").effect_at(0), None);
        assert_eq!(perk("lucky").effect_at(2).as_deref(), Some("+10% luck"));
        assert_eq!(
            perk("hardy").effect_at(3).as_deref(),
            Some("+60 max health")
        );
        assert_eq!(
            perk("fleet").effect_at(1).as_deref(),
            Some("+3% movement speed")
        );
        assert_eq!(
            perk("extra_reroll").effect_at(1).as_deref(),
            Some("+1 reroll a run")
        );
        assert_eq!(
            perk("extra_banish").effect_at(2).as_deref(),
            Some("+2 banishes a run")
        );
    }
}
