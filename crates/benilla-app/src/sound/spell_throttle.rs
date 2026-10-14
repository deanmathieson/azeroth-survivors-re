//! A spell-sound budget for a mode that casts far more than the 1.12 client ever did: the
//! survivors mode's heroes fire dozens of spells a second late in a run, every one through the
//! spell-visual kits' sounds ([`super::spell`], [`super::missile`]). Opt-in: the budget exists only
//! when that mode inserts [`SpellSfxThrottle`], so the client's own plays pass untouched.
//!
//! Each kit start asks [`SpellSfxThrottle::admit`]: a kit starts at most [`PER_KIT_MAX`] times
//! within [`RING_SECS`] and never twice within [`MIN_GAP`], all spell kits together at most
//! [`GLOBAL_MAX`] times within [`RING_SECS`], and a kit repeated within [`REPEAT_WINDOW`] plays
//! quieter. A [`SpellSfxGain`] on the unit a kit plays on scales it: a boss's or elite's hits ring
//! louder, a party extra's casts quieter. The engine's own gates (`SAME_KIT_MAX`, the
//! `NO_DUPLICATES` flag, the voice ceiling) still apply after this one.

use std::collections::VecDeque;

use bevy::prelude::*;

/// At most this many starts of one kit still ringing.
pub(crate) const PER_KIT_MAX: usize = 3;
/// The least time between two starts of one kit, in seconds.
pub(crate) const MIN_GAP: f64 = 0.08;
/// Spell sounds ringing at once, every kit together.
pub(crate) const GLOBAL_MAX: usize = 24;
/// How long a start counts as ringing: about a spell one-shot's length.
pub(crate) const RING_SECS: f64 = 1.0;
/// Earlier starts of the same kit inside this window make the next one quieter.
pub(crate) const REPEAT_WINDOW: f64 = 0.5;
/// Each such repeat takes this much off, down to [`REPEAT_FLOOR`].
const REPEAT_STEP: f32 = 0.15;
const REPEAT_FLOOR: f32 = 0.4;
/// Every budgeted play's level before its gain: the headroom a crowd of spells needs, which a
/// boss's gain spends.
const BASE: f32 = 0.8;

/// A boss's hits: back to full level.
pub(crate) const GAIN_BOSS: f32 = 1.25;
/// An elite's or guardian's hits.
pub(crate) const GAIN_ELITE: f32 = 1.12;
/// A party extra's casts, under the local hero's.
pub(crate) const GAIN_EXTRA: f32 = 0.55;

/// How loud the spell kits played on this unit are against the rest.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct SpellSfxGain(pub(crate) f32);

/// The budget: recent starts by kit, oldest first.
#[derive(Resource, Default, Debug)]
pub(crate) struct SpellSfxThrottle {
    starts: VecDeque<(u32, f64)>,
}

impl SpellSfxThrottle {
    /// May `kit` start at `now` (seconds, monotonic)? The volume multiplier to play it at, or
    /// `None` to drop it. An admitted start is counted.
    pub(crate) fn admit(&mut self, kit: u32, now: f64, gain: f32) -> Option<f32> {
        let keep = RING_SECS.max(REPEAT_WINDOW);
        while self.starts.front().is_some_and(|&(_, t)| now - t >= keep) {
            self.starts.pop_front();
        }
        let ringing = |t: f64| now - t < RING_SECS;
        let same = || self.starts.iter().filter(|&&(k, _)| k == kit);
        if same().any(|&(_, t)| now - t < MIN_GAP) {
            return None;
        }
        if same().filter(|&&(_, t)| ringing(t)).count() >= PER_KIT_MAX {
            return None;
        }
        if self.starts.iter().filter(|&&(_, t)| ringing(t)).count() >= GLOBAL_MAX {
            return None;
        }
        let repeats = same().filter(|&&(_, t)| now - t < REPEAT_WINDOW).count() as f32;
        let repeat = (1.0 - REPEAT_STEP * repeats).max(REPEAT_FLOOR);
        self.starts.push_back((kit, now));
        Some((BASE * gain * repeat).clamp(0.0, 1.0))
    }
}

/// The multiplier for a play on `unit` under `throttle`, or `None` to drop it; every play passes
/// at 1.0 with no throttle.
pub(super) fn budget(
    throttle: Option<&mut SpellSfxThrottle>,
    kit: u32,
    now: f64,
    gain: Option<&SpellSfxGain>,
) -> Option<f32> {
    match throttle {
        Some(t) => t.admit(kit, now, gain.map_or(1.0, |g| g.0)),
        None => Some(1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn a_kit_waits_its_gap_and_rings_at_most_three_times() {
        let mut t = SpellSfxThrottle::default();
        assert!(t.admit(7, 0.0, 1.0).is_some());
        assert_eq!(t.admit(7, 0.05, 1.0), None, "inside the gap");
        assert!(t.admit(9, 0.05, 1.0).is_some(), "another kit is its own");
        assert!(t.admit(7, 0.1, 1.0).is_some());
        assert!(t.admit(7, 0.2, 1.0).is_some());
        assert_eq!(t.admit(7, 0.3, 1.0), None, "three still ringing");
        // The first has rung out.
        assert!(t.admit(7, 1.01, 1.0).is_some());
    }

    #[test]
    fn every_kit_together_rings_at_most_the_global_cap() {
        let mut t = SpellSfxThrottle::default();
        for kit in 0..GLOBAL_MAX as u32 {
            assert!(t.admit(kit, 0.0, 1.0).is_some());
        }
        assert_eq!(t.admit(999, 0.0, 1.0), None);
        assert!(t.admit(999, RING_SECS, 1.0).is_some(), "once they ring out");
    }

    #[test]
    fn repeats_play_quieter_down_to_the_floor_and_recover() {
        let mut t = SpellSfxThrottle::default();
        let first = t.admit(7, 0.0, 1.0).unwrap();
        let second = t.admit(7, 0.1, 1.0).unwrap();
        let third = t.admit(7, 0.2, 1.0).unwrap();
        assert!(close(first, BASE));
        assert!(second < first && third < second);
        // Many repeats bottom out at the floor.
        let mut t = SpellSfxThrottle::default();
        let mut last = 1.0;
        for i in 0..40 {
            if let Some(v) = t.admit(7, f64::from(i) * 0.09, 1.0) {
                last = v;
            }
        }
        assert!(last >= BASE * REPEAT_FLOOR - 1e-5);
        // Past the window, full level again.
        assert!(close(t.admit(7, 100.0, 1.0).unwrap(), BASE));
    }

    #[test]
    fn bosses_ring_louder_and_extras_quieter() {
        let mut t = SpellSfxThrottle::default();
        let plain = t.admit(1, 0.0, 1.0).unwrap();
        let elite = t.admit(2, 0.0, GAIN_ELITE).unwrap();
        let boss = t.admit(3, 0.0, GAIN_BOSS).unwrap();
        let extra = t.admit(4, 0.0, GAIN_EXTRA).unwrap();
        assert!(extra < plain && plain < elite && elite < boss);
        assert!(boss <= 1.0);
    }

    #[test]
    fn with_no_throttle_every_play_passes_at_full_level() {
        assert_eq!(
            budget(None, 7, 0.0, Some(&SpellSfxGain(GAIN_BOSS))),
            Some(1.0)
        );
        let mut t = SpellSfxThrottle::default();
        assert!(close(
            budget(Some(&mut t), 7, 0.0, Some(&SpellSfxGain(GAIN_EXTRA))).unwrap(),
            BASE * GAIN_EXTRA
        ));
    }
}
