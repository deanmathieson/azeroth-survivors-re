//! Side bets. Before the ready check each player may stake 0 to [`MAX_STAKE`] Darkmoon Prize
//! Tickets that a player will be the run's **Top damage**, **First to fall**, still standing at
//! **15:00**, or bring in the **Most gold**. Each market is its own pari-mutuel pool: the winning
//! bets share everything staked on it in proportion to their stakes, and when no bet on a market
//! wins, every stake on it comes back. Stakes leave each player's own profile as the run starts;
//! the host settles from the run (its heroes' damage and gold, who fell first, who stood at 15:00)
//! and each machine pays its own player ([`Payout`]), on the results screen, to the coin sound.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::super::data::VICTORY_SECS;
use super::super::super::hud::{NoteKind, ResultsNotes};
use super::super::super::profile::Profile;
use super::super::super::{HeroState, Phase, Run};
use super::super::wire::NetHero;
use super::super::Session;
use super::lobby::{BookShown, HostBook};
use super::to_all;
use super::wire::{Heard, Payout};
use crate::sound::SoundCue;
use bevy_replicon::prelude::*;

/// The most a player may stake.
pub(super) const MAX_STAKE: u32 = 50;
/// Stakes move in steps of this many tickets.
pub(super) const STEP: u32 = 5;
/// The coins of a payout.
pub(super) const COIN: &str = "LOOTWINDOWCOINSOUND";

/// What a bet is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Market {
    TopDamage,
    FirstToFall,
    Survive,
    MostGold,
}

impl Market {
    pub(super) const ALL: [Self; 4] = [
        Self::TopDamage,
        Self::FirstToFall,
        Self::Survive,
        Self::MostGold,
    ];

    pub(super) fn of(i: u8) -> Self {
        Self::ALL[usize::from(i) % Self::ALL.len()]
    }

    #[cfg(test)]
    pub(super) fn index(self) -> u8 {
        Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as u8
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::TopDamage => "Top damage",
            Self::FirstToFall => "First to fall",
            Self::Survive => "Survive to 15:00",
            Self::MostGold => "Most gold",
        }
    }
}

/// One bet: `stake` tickets that player `on` takes market `market` (a [`Market::index`]).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct Wager {
    pub market: u8,
    pub on: u8,
    pub stake: u32,
}

/// One hero's share of the run, for the settling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Standing {
    pub player: u8,
    pub damage: f64,
    pub gold: u32,
}

/// How the run went, for each market: who took it (ties all take it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Outcome {
    pub top_damage: Vec<u8>,
    pub first_to_fall: Option<u8>,
    pub survived: Vec<u8>,
    pub most_gold: Vec<u8>,
}

impl Outcome {
    /// From the heroes' standings, who fell first, and who stood at 15:00. A top with nothing to
    /// show (no damage, no gold) takes nothing.
    pub(super) fn of(standings: &[Standing], fell: Option<u8>, survived: &[u8]) -> Self {
        let top = |key: &dyn Fn(&Standing) -> f64| {
            let best = standings.iter().map(key).fold(0.0, f64::max);
            if best <= 0.0 {
                return Vec::new();
            }
            standings
                .iter()
                .filter(|s| key(s) >= best)
                .map(|s| s.player)
                .collect()
        };
        Self {
            top_damage: top(&|s| s.damage),
            first_to_fall: fell,
            survived: survived.to_vec(),
            most_gold: top(&|s| f64::from(s.gold)),
        }
    }

    /// The players `market` went to.
    pub(super) fn winners(&self, market: Market) -> Vec<u8> {
        match market {
            Market::TopDamage => self.top_damage.clone(),
            Market::FirstToFall => self.first_to_fall.into_iter().collect(),
            Market::Survive => self.survived.clone(),
            Market::MostGold => self.most_gold.clone(),
        }
    }
}

/// Settle `bets` (each player's one) on `outcome`, market by market: a market's pool goes to the
/// bets that won it, shared by stake (rounded down, the odd tickets to the biggest winning stake,
/// the earlier on a tie); a market no bet won hands every stake back. Bets of nothing are no bets.
pub(super) fn settle(bets: &[(u8, Wager)], outcome: &Outcome) -> Vec<Payout> {
    let mut out = Vec::new();
    for market in Market::ALL {
        let on: Vec<(u8, Wager)> = bets
            .iter()
            .copied()
            .filter(|(_, w)| w.stake > 0 && Market::of(w.market) == market)
            .collect();
        if on.is_empty() {
            continue;
        }
        let winners = outcome.winners(market);
        let pool: u64 = on.iter().map(|(_, w)| u64::from(w.stake)).sum();
        let won: Vec<bool> = on.iter().map(|(_, w)| winners.contains(&w.on)).collect();
        let winning: u64 = on
            .iter()
            .zip(&won)
            .filter(|(_, &w)| w)
            .map(|((_, w), _)| u64::from(w.stake))
            .sum();
        if winning == 0 {
            out.extend(on.iter().map(|&(player, wager)| Payout {
                player,
                wager,
                paid: wager.stake,
                won: false,
                refunded: true,
            }));
            continue;
        }
        let mut paid: Vec<u32> = on
            .iter()
            .zip(&won)
            .map(|((_, w), &win)| {
                if win {
                    (pool * u64::from(w.stake) / winning) as u32
                } else {
                    0
                }
            })
            .collect();
        let left = pool - paid.iter().map(|&p| u64::from(p)).sum::<u64>();
        if left > 0 {
            let biggest = (0..on.len())
                .filter(|&i| won[i])
                .max_by(|&a, &b| on[a].1.stake.cmp(&on[b].1.stake).then(b.cmp(&a)));
            if let Some(i) = biggest {
                paid[i] += left as u32;
            }
        }
        out.extend(
            on.iter()
                .zip(won)
                .zip(paid)
                .map(|(((player, wager), won), paid)| Payout {
                    player: *player,
                    wager: *wager,
                    paid,
                    won,
                    refunded: false,
                }),
        );
    }
    out
}

/// What the host saw of the run for the bets: who fell first, and who stood at 15:00.
#[derive(Resource, Default, Debug)]
pub(super) struct Tracker {
    fell: Option<u8>,
    survived: Option<Vec<u8>>,
}

/// The tickets this player staked as the run began.
#[derive(Resource, Default)]
pub(super) struct Escrow(pub u32);

/// The bets as they were settled, for the results screen.
#[derive(Resource, Default)]
pub(super) struct Settled(pub Option<Vec<Payout>>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Tracker>()
        .init_resource::<Escrow>()
        .init_resource::<Settled>()
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            (reset_tracker, take_stakes),
        )
        .add_systems(
            Update,
            (track.run_if(super::hosting).run_if(in_run), hear_settled),
        )
        .add_systems(OnEnter(Phase::GameOver), settle_run.run_if(super::hosting))
        .add_systems(OnEnter(Phase::Menu), forget_settled);
}

fn in_run(session: Res<Session>) -> bool {
    session.in_run
}

fn reset_tracker(mut tracker: ResMut<Tracker>, mut settled: ResMut<Settled>) {
    *tracker = Tracker::default();
    settled.0 = None;
}

/// As a co-op run begins, this player's stake leaves its own tickets.
fn take_stakes(
    session: Res<Session>,
    book: Res<BookShown>,
    mut profile: ResMut<Profile>,
    mut escrow: ResMut<Escrow>,
) {
    escrow.0 = 0;
    if !matches!(
        session.link,
        super::super::Link::Hosting { .. } | super::super::Link::Joined { .. }
    ) {
        return;
    }
    let Some(w) = book.bet(session.me).filter(|w| w.stake > 0) else {
        return;
    };
    let take = w.stake.min(profile.faire.tickets);
    if take > 0 {
        profile.faire.tickets -= take;
        profile.mark_dirty();
        escrow.0 = take;
    }
}

/// Who fell first, and who still stood as the clock reached 15:00.
fn track(run: Res<Run>, heroes: Query<(&NetHero, &HeroState)>, mut tracker: ResMut<Tracker>) {
    if tracker.fell.is_none() {
        if let Some((h, _)) = heroes.iter().find(|(_, s)| !s.alive()) {
            tracker.fell = Some(h.player);
        }
    }
    if tracker.survived.is_none() && run.elapsed >= VICTORY_SECS {
        tracker.survived = Some(
            heroes
                .iter()
                .filter(|(_, s)| s.alive())
                .map(|(h, _)| h.player)
                .collect(),
        );
    }
}

/// The run is over: the host settles the book and tells everyone, and the book is cleared.
fn settle_run(
    run: Res<Run>,
    heroes: Query<(&NetHero, &HeroState)>,
    tracker: Res<Tracker>,
    mut book: ResMut<HostBook>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    let bets: Vec<(u8, Wager)> = book.bets.iter().map(|(&p, &w)| (p, w)).collect();
    book.bets.clear();
    book.dirty = true;
    if bets.iter().all(|(_, w)| w.stake == 0) {
        return;
    }
    let standings: Vec<Standing> = heroes
        .iter()
        .map(|(h, s)| Standing {
            player: h.player,
            damage: s.damage_done,
            gold: s.gold,
        })
        .collect();
    // A win at 15:00 may end the run before the tracker saw the clock.
    let survived = tracker.survived.clone().unwrap_or_else(|| {
        if run.won || run.elapsed >= VICTORY_SECS {
            heroes
                .iter()
                .filter(|(_, s)| s.alive())
                .map(|(h, _)| h.player)
                .collect()
        } else {
            Vec::new()
        }
    });
    let outcome = Outcome::of(&standings, tracker.fell, &survived);
    out.write(to_all(Heard::Settled(settle(&bets, &outcome))));
}

/// The settling heard: this player's payout goes into its own tickets, noted and chimed.
#[allow(clippy::too_many_arguments)]
fn hear_settled(
    mut heard: MessageReader<Heard>,
    session: Res<Session>,
    mut profile: ResMut<Profile>,
    mut notes: ResMut<ResultsNotes>,
    mut settled: ResMut<Settled>,
    mut escrow: ResMut<Escrow>,
    mut cues: MessageWriter<SoundCue>,
) {
    for h in heard.read() {
        let Heard::Settled(payouts) = h else { continue };
        settled.0 = Some(payouts.clone());
        let Some(mine) = payouts.iter().find(|p| p.player == session.me) else {
            continue;
        };
        // Never more back than was taken when the stake could not all be paid in.
        let paid = if mine.refunded { escrow.0 } else { mine.paid };
        escrow.0 = 0;
        if paid > 0 {
            profile.faire.tickets = profile.faire.tickets.saturating_add(paid);
            profile.faire.tickets_earned =
                profile.faire.tickets_earned.saturating_add(u64::from(paid));
            profile.mark_dirty();
            cues.write(SoundCue::flat(COIN));
        }
        let line = if mine.refunded {
            format!("Side bet refunded: {paid} tickets back")
        } else if mine.won {
            format!("Side bet won: +{paid} Darkmoon Prize Tickets!")
        } else {
            format!("Side bet lost: {} tickets gone", mine.wager.stake)
        };
        notes.push(
            line,
            if mine.won || mine.refunded {
                NoteKind::Good
            } else {
                NoteKind::Info
            },
        );
    }
}

fn forget_settled(mut settled: ResMut<Settled>) {
    settled.0 = None;
}

/// "Jaina: 20 on Thrall, First to fall".
pub(super) fn bet_line(name: &str, on: &str, w: &Wager) -> String {
    format!(
        "{name}: {} on {on}, {}",
        w.stake,
        Market::of(w.market).label()
    )
}

/// A settled bet's line for the results: what it paid.
pub(super) fn payout_line(name: &str, on: &str, p: &Payout) -> String {
    let bet = bet_line(name, on, &p.wager);
    if p.refunded {
        format!("{bet}: refunded")
    } else if p.won {
        format!("{bet}: won {}", p.paid)
    } else {
        format!("{bet}: lost")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bet(market: Market, on: u8, stake: u32) -> Wager {
        Wager {
            market: market.index(),
            on,
            stake,
        }
    }

    fn standing(player: u8, damage: f64, gold: u32) -> Standing {
        Standing {
            player,
            damage,
            gold,
        }
    }

    #[test]
    fn the_run_decides_each_market_ties_and_all() {
        let s = [
            standing(0, 900.0, 40),
            standing(1, 1200.0, 40),
            standing(2, 1200.0, 10),
        ];
        let o = Outcome::of(&s, Some(2), &[0, 1]);
        assert_eq!(
            o.winners(Market::TopDamage),
            vec![1, 2],
            "a tie shares the top"
        );
        assert_eq!(o.winners(Market::FirstToFall), vec![2]);
        assert_eq!(o.winners(Market::Survive), vec![0, 1]);
        assert_eq!(o.winners(Market::MostGold), vec![0, 1]);
        let nothing = Outcome::of(&[standing(0, 0.0, 0)], None, &[]);
        assert!(Market::ALL.iter().all(|&m| nothing.winners(m).is_empty()));
    }

    #[test]
    fn a_pool_goes_to_its_winners_by_stake() {
        let o = Outcome {
            first_to_fall: Some(2),
            ..Default::default()
        };
        let bets = [
            (0, bet(Market::FirstToFall, 2, 30)),
            (1, bet(Market::FirstToFall, 2, 10)),
            (3, bet(Market::FirstToFall, 0, 20)),
        ];
        let p = settle(&bets, &o);
        assert_eq!(p.iter().map(|x| x.paid).collect::<Vec<_>>(), [45, 15, 0]);
        assert_eq!(
            p.iter().map(|x| x.won).collect::<Vec<_>>(),
            [true, true, false]
        );
        assert_eq!(
            p.iter().map(|x| x.paid).sum::<u32>(),
            60,
            "the pool, no more, no less"
        );
    }

    #[test]
    fn odd_tickets_go_to_the_biggest_winning_stake() {
        let o = Outcome {
            top_damage: vec![1],
            ..Default::default()
        };
        // 35 in the pool, 10 and 5 winning: 23.33 and 11.67, rounded down to 23 and 11; the odd
        // ticket to the 10.
        let bets = [
            (0, bet(Market::TopDamage, 1, 10)),
            (1, bet(Market::TopDamage, 1, 5)),
            (2, bet(Market::TopDamage, 0, 20)),
        ];
        let p = settle(&bets, &o);
        assert_eq!(p.iter().map(|x| x.paid).collect::<Vec<_>>(), [24, 11, 0]);
    }

    #[test]
    fn no_winner_hands_every_stake_back() {
        let o = Outcome {
            survived: vec![],
            most_gold: vec![3],
            ..Default::default()
        };
        let bets = [
            (0, bet(Market::Survive, 1, 50)),
            (1, bet(Market::Survive, 0, 15)),
            (2, bet(Market::MostGold, 3, 5)),
            (3, bet(Market::MostGold, 3, 0)),
        ];
        let p = settle(&bets, &o);
        assert_eq!(p.len(), 3, "a bet of nothing is no bet");
        let survive: Vec<_> = p.iter().filter(|x| x.wager.market == 2).collect();
        assert!(survive.iter().all(|x| x.refunded && !x.won));
        assert_eq!(survive.iter().map(|x| x.paid).collect::<Vec<_>>(), [50, 15]);
        let gold = p.iter().find(|x| x.player == 2).unwrap();
        assert!(
            gold.won && gold.paid == 5,
            "alone on the winner: its own stake back as winnings"
        );
        assert!(settle(&[], &o).is_empty());
    }

    #[test]
    fn markets_settle_apart() {
        let o = Outcome {
            top_damage: vec![0],
            first_to_fall: Some(1),
            ..Default::default()
        };
        let bets = [
            (0, bet(Market::TopDamage, 0, 10)),
            (1, bet(Market::FirstToFall, 0, 40)),
            (2, bet(Market::FirstToFall, 1, 10)),
        ];
        let p = settle(&bets, &o);
        let paid = |player: u8| p.iter().find(|x| x.player == player).unwrap().paid;
        assert_eq!((paid(0), paid(1), paid(2)), (10, 0, 50));
        assert_eq!(
            payout_line("Jaina", "Thrall", &p[2]),
            "Jaina: 10 on Thrall, First to fall: won 50"
        );
    }
}
