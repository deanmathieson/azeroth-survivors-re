//! The Prize Booth and the Prize Counter: the pulls, their odds and pity, the featured banners and
//! Silas Darkmoon's Favor prices. Pure over the [`Profile`]: the screen calls [`pull`] and [`buy`]
//! and saves what they change.
//!
//! A pull rolls a rarity at [`Rarity::odds`], then a prize of it at even odds; on the featured
//! banner the banner's prizes of that rarity share half of it ([`FEATURED_SHARE`]). Pity is kept
//! per banner type: the [`EPIC_PITY`]th pull without an Epic or better is one, the
//! [`LEGENDARY_PITY`]th without a Legendary is one, and a hit resets its counter. Ten pulls at once
//! cost [`TEN_PULL_COST`] and always hold a Rare or better. A prize already owned turns into Faire
//! Favor ([`Rarity::favor`]), which the Counter takes for any prize ([`Rarity::price`]), the
//! current banner's for [`FEATURED_DISCOUNT`]% less.

use super::super::profile::{AccountRecord, Profile};
use super::catalog::{catalog, grant, owned, Prize, Rarity};
use super::save::FaireSave;
use super::tickets::RUNS_PER_STAY;

/// Tickets for one pull, and for ten at once.
pub(in crate::survivors) const PULL_COST: u32 = 10;
pub(in crate::survivors) const TEN_PULL_COST: u32 = 90;
/// The pull that is always Epic or better, counted from the last; and always Legendary.
pub(in crate::survivors) const EPIC_PITY: u32 = 30;
pub(in crate::survivors) const LEGENDARY_PITY: u32 = 90;
/// The share of a rarity's odds the featured prizes of it split, on the featured banner.
pub(in crate::survivors) const FEATURED_SHARE: f64 = 0.5;
/// Percent off the current banner's featured prizes at the Prize Counter.
pub(in crate::survivors) const FEATURED_DISCOUNT: u32 = 20;

/// A themed week on the booth: its featured prizes, by id.
#[derive(Debug)]
pub(in crate::survivors) struct Banner {
    pub name: &'static str,
    pub blurb: &'static str,
    /// The banner's emblem, `Interface\Icons\<emblem>.blp`.
    pub emblem: &'static str,
    /// The banner's colour, `0xRRGGBB`.
    pub tint: u32,
    pub featured: &'static [&'static str],
}

/// The banners, in their order on the booth; each holds [`RUNS_PER_STAY`] runs.
pub(in crate::survivors) static BANNERS: &[Banner] = &[
    Banner {
        name: "Scourge Week",
        blurb: "The Argent Dawn rides against the Scourge. So does a very fast skeleton horse.",
        emblem: "Spell_Shadow_RaiseDead",
        tint: 0x5fc46a,
        featured: &[
            "mount:deathcharger_s_reins",
            "title:scourgebane",
            "tabard:tabard_of_the_argent_dawn",
            "mount:red_skeletal_horse",
            "look:undercity_cockroach",
            "look:disgusting_oozeling",
            "tabard:undercity_tabard",
        ],
    },
    Banner {
        name: "Ahn'Qiraj Week",
        blurb: "The Scarab Gong has rung. The gates are open, and the tanks are for the taking.",
        emblem: "INV_Misc_QirajiCrystal_05",
        tint: 0xd9a64a,
        featured: &[
            "mount:horn_of_the_black_war_wolf",
            "title:scarab_lord",
            "mount:swift_stormsaber",
            "tabard:tabard_of_the_cenarion_circle",
            "hat:tiny_pith_helmet",
        ],
    },
    Banner {
        name: "Westfall Harvest",
        blurb: "Pumpkins, plough horses and the People's Militia: the Faire's homely week.",
        emblem: "INV_Misc_Food_Wheat_01",
        tint: 0xf08c26,
        featured: &[
            "mount:swift_palomino",
            "hat:jack_o_lantern_hat",
            "tabard:people_s_militia_tabard",
            "mount:brown_horse",
            "mount:chestnut_mare",
            "mount:pinto",
            "hat:tiny_party_hat",
            "look:tree_frog",
            "toy:goblin_gumbo",
        ],
    },
    Banner {
        name: "Elemental Fury",
        blurb: "Fire, earth, wind and water, and the Lord of the Firelands' own retinue.",
        emblem: "Spell_Fire_ElementalDevastation",
        tint: 0xff5a26,
        featured: &[
            "title:the_flamewaker",
            "hat:crown_of_the_fire_festival",
            "card:ace_of_elementals",
            "look:crimson_whelpling",
            "card:two_of_elementals",
            "card:three_of_elementals",
            "card:four_of_elementals",
            "card:five_of_elementals",
            "card:six_of_elementals",
            "card:seven_of_elementals",
            "card:eight_of_elementals",
        ],
    },
    Banner {
        name: "Stranglethorn Fever",
        blurb: "Zul'Gurub's beasts and Booty Bay's pirates, all under one tent.",
        emblem: "INV_ZulGurubTrinket",
        tint: 0x33bf73,
        featured: &[
            "mount:swift_razzashi_raptor",
            "mount:swift_zulian_tiger",
            "mount:black_war_tiger",
            "look:senegal",
            "look:crimson_snake",
            "toy:savory_deviate_delight",
            "hat:pirate_hat",
        ],
    },
    Banner {
        name: "Darkmoon Jubilee",
        blurb: "Silas Darkmoon throws open the vaults: the Faire's own finest, and its Aces.",
        emblem: "Spell_Nature_CrystalBall",
        tint: 0xa659f2,
        featured: &[
            "title:the_insane",
            "title:champion_of_the_faire",
            "tabard:tabard_of_the_darkmoon_faire",
            "look:dark_whelpling",
            "look:pet_bombling",
            "look:lil_smoky",
            "card:ace_of_beasts",
            "card:ace_of_elementals",
            "card:ace_of_portals",
            "card:ace_of_warlords",
            "toy:sayge_s_fortune",
            "toy:darkmoon_fireworks",
            "title:ticket_hoarder",
            "title:the_faire_goer",
        ],
    },
];

/// The banner on the booth for the run after `runs` recorded ones.
pub(in crate::survivors) fn banner_at(runs: u32) -> &'static Banner {
    &BANNERS[(runs / RUNS_PER_STAY) as usize % BANNERS.len()]
}

/// The next `n` banners after the current one, each with the runs until it opens.
pub(in crate::survivors) fn upcoming(runs: u32, n: u32) -> Vec<(u32, &'static Banner)> {
    let start = runs - runs % RUNS_PER_STAY;
    (1..=n)
        .map(|k| {
            let at = start + k * RUNS_PER_STAY;
            (at - runs, banner_at(at))
        })
        .collect()
}

impl Banner {
    /// Whether `prize` is one of this banner's.
    pub(in crate::survivors) fn features(&self, prize: &Prize) -> bool {
        self.featured.contains(&prize.id.as_str())
    }
}

/// The booth's two banner types, each with its own pity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(in crate::survivors) enum BannerKind {
    /// The week's themed banner.
    #[default]
    Featured,
    /// The Darkmoon Prize Booth's standing stock, every prize of a rarity at even odds.
    Standard,
}

impl BannerKind {
    pub(in crate::survivors) const ALL: [Self; 2] = [Self::Featured, Self::Standard];

    /// The pity counter's key in [`FaireSave::pity`].
    pub(in crate::survivors) fn key(self) -> &'static str {
        match self {
            Self::Featured => "featured",
            Self::Standard => "standard",
        }
    }
}

// ── The dice ────────────────────────────────────────────────────────────────────────────────────

/// SplitMix64: one step of the booth's dice.
fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Uniform in `[0, 1)`.
fn unit(state: &mut u64) -> f64 {
    (next_u64(state) >> 11) as f64 / (1u64 << 53) as f64
}

/// A first seed from the account's record: deterministic, and different for every account that
/// has played at all.
pub(in crate::survivors) fn seed(account: &AccountRecord) -> u64 {
    let mut s = 0x0DAC_F00D_D4C3_u64;
    for v in [
        u64::from(account.runs),
        account.kills,
        account.bosses,
        account.gold_earned,
        account.seconds_played.to_bits(),
        account.damage_done.to_bits(),
    ] {
        s ^= v;
        next_u64(&mut s);
    }
    s
}

/// The dice as saved, seeded from `account` on first use.
fn dice(save: &FaireSave, account: &AccountRecord) -> u64 {
    save.dice.map_or_else(|| seed(account), |d| d as u64)
}

/// A rarity at the booth's odds, for `r` uniform in `[0, 1)`.
pub(in crate::survivors) fn roll_rarity(r: f64) -> Rarity {
    at_least(Rarity::Common, r)
}

/// A rarity of `min` or better, at their odds relative to each other.
fn at_least(min: Rarity, r: f64) -> Rarity {
    let tiers: Vec<Rarity> = Rarity::ALL.into_iter().filter(|&t| t >= min).collect();
    let total: u32 = tiers.iter().map(|t| t.odds()).sum();
    let mut x = r * f64::from(total);
    // Rarest first, so float dust lands on the common end.
    for &t in tiers.iter().rev() {
        let w = f64::from(t.odds());
        if x < w {
            return t;
        }
        x -= w;
    }
    min
}

// ── Pulls ───────────────────────────────────────────────────────────────────────────────────────

/// One prize pulled.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct Pull {
    /// An index into [`catalog`].
    pub prize: usize,
    /// New to the account; otherwise it paid [`Pull::favor`].
    pub new: bool,
    pub favor: u32,
    /// One of the banner's featured prizes.
    pub featured: bool,
}

impl Pull {
    pub(in crate::survivors) fn prize(&self) -> &'static Prize {
        &catalog()[self.prize]
    }
}

/// Why the booth or the counter said no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Refusal {
    /// Short this many tickets, or this much Favor.
    Short(u32),
    /// The Counter does not sell what the account has.
    Owned,
    /// No such prize.
    Unknown,
}

/// Tickets `count` pulls cost: ten at once at [`TEN_PULL_COST`], otherwise [`PULL_COST`] each.
pub(in crate::survivors) fn cost(count: u32) -> u32 {
    if count == 10 {
        TEN_PULL_COST
    } else {
        count.saturating_mul(PULL_COST)
    }
}

/// Pull `count` prizes (1 or 10) on `kind`'s banner (`banner` the week's), paying from `dev`'s
/// in-memory tickets first and then the saved ones. Every prize is unlocked or turned to Favor as
/// it lands, and the pity and dice are saved with it.
pub(in crate::survivors) fn pull(
    profile: &mut Profile,
    dev: &mut u32,
    kind: BannerKind,
    banner: &Banner,
    count: u32,
) -> Result<Vec<Pull>, Refusal> {
    let price = cost(count);
    let have = profile.faire.tickets.saturating_add(*dev);
    if have < price {
        return Err(Refusal::Short(price - have));
    }
    let from_dev = price.min(*dev);
    *dev -= from_dev;
    profile.faire.tickets -= price - from_dev;

    let mut state = dice(&profile.faire, &profile.account);
    let mut pity = profile.faire.pity(kind.key());
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        let mut rarity = roll_rarity(unit(&mut state));
        if pity.since_legendary + 1 >= LEGENDARY_PITY {
            rarity = Rarity::Legendary;
        } else if pity.since_epic + 1 >= EPIC_PITY && rarity < Rarity::Epic {
            rarity = at_least(Rarity::Epic, unit(&mut state));
        }
        let ten_short = count == 10
            && i == count - 1
            && out.iter().all(|p: &Pull| p.prize().rarity < Rarity::Rare);
        if ten_short && rarity < Rarity::Rare {
            rarity = at_least(Rarity::Rare, unit(&mut state));
        }
        let featured = (kind == BannerKind::Featured).then_some(banner);
        let prize = choose(rarity, featured, &mut state);
        out.push(land(profile, prize, featured));
        pity.since_epic = if rarity >= Rarity::Epic {
            0
        } else {
            pity.since_epic + 1
        };
        pity.since_legendary = if rarity == Rarity::Legendary {
            0
        } else {
            pity.since_legendary + 1
        };
    }
    profile.faire.pity.insert(kind.key().into(), pity);
    profile.faire.dice = Some(state as i64);
    profile.faire.pulls = profile.faire.pulls.saturating_add(count);
    Ok(out)
}

/// A prize of `rarity`: with a banner, its featured prizes of the rarity share
/// [`FEATURED_SHARE`] of the pick and the rest share what is left; otherwise all at even odds.
fn choose(rarity: Rarity, banner: Option<&Banner>, state: &mut u64) -> usize {
    let pool: Vec<usize> = (0..catalog().len())
        .filter(|&i| catalog()[i].rarity == rarity)
        .collect();
    let (hot, cold): (Vec<usize>, Vec<usize>) = pool
        .iter()
        .partition(|&&i| banner.is_some_and(|b| b.features(&catalog()[i])));
    let side = if hot.is_empty() {
        &cold
    } else if cold.is_empty() || unit(state) < FEATURED_SHARE {
        &hot
    } else {
        &cold
    };
    side[(unit(state) * side.len() as f64) as usize % side.len()]
}

/// Unlock `index`'s prize, or pay its Favor if the account has it.
fn land(profile: &mut Profile, index: usize, banner: Option<&Banner>) -> Pull {
    let prize = &catalog()[index];
    let featured = banner.is_some_and(|b| b.features(prize));
    if grant(profile, prize) {
        Pull {
            prize: index,
            new: true,
            favor: 0,
            featured,
        }
    } else {
        let favor = prize.rarity.favor();
        profile.faire.favor = profile.faire.favor.saturating_add(favor);
        Pull {
            prize: index,
            new: false,
            favor,
            featured,
        }
    }
}

// ── The Prize Counter ───────────────────────────────────────────────────────────────────────────

/// `prize`'s Favor price while `banner` is up: [`FEATURED_DISCOUNT`]% off its featured prizes.
pub(in crate::survivors) fn price(prize: &Prize, banner: &Banner) -> u32 {
    let full = prize.rarity.price();
    if banner.features(prize) {
        full * (100 - FEATURED_DISCOUNT) / 100
    } else {
        full
    }
}

/// Buy the prize `id` for Favor while `banner` is up; the Favor spent.
pub(in crate::survivors) fn buy(
    profile: &mut Profile,
    id: &str,
    banner: &Banner,
) -> Result<u32, Refusal> {
    let prize = super::catalog::by_id(id).ok_or(Refusal::Unknown)?;
    if owned(profile, prize) {
        return Err(Refusal::Owned);
    }
    let cost = price(prize, banner);
    if profile.faire.favor < cost {
        return Err(Refusal::Short(cost - profile.faire.favor));
    }
    profile.faire.favor -= cost;
    grant(profile, prize);
    Ok(cost)
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{by_id, Kind};
    use super::*;

    fn rich(tickets: u32, dice: u64) -> Profile {
        let mut p = Profile::default();
        p.faire.tickets = tickets;
        p.faire.dice = Some(dice as i64);
        p
    }

    #[test]
    fn every_featured_prize_exists_and_banners_rotate() {
        for b in BANNERS {
            assert!(!b.featured.is_empty(), "{}", b.name);
            for id in b.featured {
                assert!(by_id(id).is_some(), "{}: {id}", b.name);
            }
        }
        // Five runs a banner, in order, round again.
        for runs in 0..5 {
            assert_eq!(banner_at(runs).name, "Scourge Week");
        }
        assert_eq!(banner_at(5).name, "Ahn'Qiraj Week");
        assert_eq!(banner_at(14).name, "Westfall Harvest");
        assert_eq!(banner_at(5 * BANNERS.len() as u32).name, "Scourge Week");
        let next: Vec<(u32, &str)> = upcoming(7, 3).iter().map(|(n, b)| (*n, b.name)).collect();
        assert_eq!(
            next,
            [
                (3, "Westfall Harvest"),
                (8, "Elemental Fury"),
                (13, "Stranglethorn Fever")
            ]
        );
    }

    #[test]
    fn rarities_roll_at_their_odds() {
        let mut state = 0x5EED_u64;
        let n = 200_000;
        let mut counts = [0u32; 5];
        for _ in 0..n {
            counts[roll_rarity(unit(&mut state)) as usize] += 1;
        }
        for r in Rarity::ALL {
            let got = f64::from(counts[r as usize]) / f64::from(n);
            let want = f64::from(r.odds()) / 10_000.0;
            // Within a tenth of the odds, or 0.1 points for the rare end.
            let tol = (want * 0.1).max(0.001);
            assert!((got - want).abs() < tol, "{r:?}: {got} vs {want}");
        }
        // The forced tiers keep their relative odds.
        assert_eq!(at_least(Rarity::Epic, 0.0), Rarity::Legendary);
        assert_eq!(at_least(Rarity::Epic, 0.99), Rarity::Epic);
        assert_eq!(at_least(Rarity::Rare, 0.5), Rarity::Rare);
    }

    #[test]
    fn many_seeded_pulls_land_near_the_odds_and_never_break_pity() {
        let mut p = rich(u32::MAX / 2, 42);
        let banner = banner_at(0);
        let mut counts = [0u32; 5];
        let (mut dry_epic, mut dry_legend) = (0u32, 0u32);
        let (mut worst_epic, mut worst_legend) = (0u32, 0u32);
        let n = 6_000;
        for _ in 0..n {
            let pulls = pull(&mut p, &mut 0, BannerKind::Standard, banner, 1).unwrap();
            let r = pulls[0].prize().rarity;
            counts[r as usize] += 1;
            dry_epic = if r >= Rarity::Epic { 0 } else { dry_epic + 1 };
            dry_legend = if r == Rarity::Legendary {
                0
            } else {
                dry_legend + 1
            };
            worst_epic = worst_epic.max(dry_epic);
            worst_legend = worst_legend.max(dry_legend);
        }
        assert!(worst_epic < EPIC_PITY, "{worst_epic}");
        assert!(worst_legend < LEGENDARY_PITY, "{worst_legend}");
        let share = |r: Rarity| f64::from(counts[r as usize]) / f64::from(n);
        // Pity lifts the rare end a little; the common end stays near its odds.
        assert!((share(Rarity::Common) - 0.6).abs() < 0.03);
        assert!((share(Rarity::Uncommon) - 0.25).abs() < 0.03);
        assert!(share(Rarity::Epic) >= 0.03);
        assert!(share(Rarity::Legendary) >= 0.005);
        assert_eq!(p.faire.pulls, n);
        // The same dice pull the same prizes.
        let mut a = rich(100, 7);
        let mut b = rich(100, 7);
        assert_eq!(
            pull(&mut a, &mut 0, BannerKind::Featured, banner, 10),
            pull(&mut b, &mut 0, BannerKind::Featured, banner, 10)
        );
    }

    #[test]
    fn ten_pulls_cost_ninety_and_hold_a_rare() {
        let banner = banner_at(0);
        for seed in 0..400u64 {
            let mut p = rich(90, seed);
            let pulls = pull(&mut p, &mut 0, BannerKind::Standard, banner, 10).unwrap();
            assert_eq!(pulls.len(), 10);
            assert!(
                pulls.iter().any(|x| x.prize().rarity >= Rarity::Rare),
                "seed {seed}"
            );
            assert_eq!(p.faire.tickets, 0);
        }
        let mut p = rich(89, 1);
        assert_eq!(
            pull(&mut p, &mut 0, BannerKind::Standard, banner, 10),
            Err(Refusal::Short(1))
        );
        assert_eq!(p.faire.tickets, 89, "a refused pull costs nothing");
        // The test aid's tickets go first, and only the rest come off the save.
        let mut p = rich(50, 1);
        let mut dev = 45;
        pull(&mut p, &mut dev, BannerKind::Standard, banner, 10).unwrap();
        assert_eq!((dev, p.faire.tickets), (0, 5));
    }

    #[test]
    fn pity_lands_on_the_thirtieth_and_ninetieth_pull() {
        let banner = banner_at(0);
        for seed in 0..200u64 {
            let mut p = rich(10, seed);
            p.faire.pity.insert(
                "standard".into(),
                super::super::save::Pity {
                    since_epic: EPIC_PITY - 1,
                    since_legendary: 10,
                },
            );
            let x = pull(&mut p, &mut 0, BannerKind::Standard, banner, 1).unwrap()[0];
            assert!(x.prize().rarity >= Rarity::Epic, "seed {seed}");
            let pity = p.faire.pity("standard");
            assert_eq!(pity.since_epic, 0, "an Epic+ resets its counter");
            // The other banner type's pity is its own.
            assert_eq!(p.faire.pity("featured"), Default::default());
        }
        for seed in 0..50u64 {
            let mut p = rich(10, seed);
            p.faire.pity.insert(
                "featured".into(),
                super::super::save::Pity {
                    since_epic: 3,
                    since_legendary: LEGENDARY_PITY - 1,
                },
            );
            let x = pull(&mut p, &mut 0, BannerKind::Featured, banner, 1).unwrap()[0];
            assert_eq!(x.prize().rarity, Rarity::Legendary, "seed {seed}");
            assert_eq!(p.faire.pity("featured"), Default::default());
        }
        // A pull short of pity counts up.
        let mut p = rich(10, 3);
        p.faire.pity.insert(
            "standard".into(),
            super::super::save::Pity {
                since_epic: 5,
                since_legendary: 5,
            },
        );
        let x = pull(&mut p, &mut 0, BannerKind::Standard, banner, 1).unwrap()[0];
        if x.prize().rarity < Rarity::Epic {
            assert_eq!(p.faire.pity("standard").since_epic, 6);
        }
    }

    #[test]
    fn the_featured_banner_favours_its_prizes() {
        let banner = BANNERS.iter().find(|b| b.name == "Ahn'Qiraj Week").unwrap();
        // Two of the seven Legendaries are featured: half the Legendary pulls go to them.
        let mut state = 99u64;
        let n = 20_000;
        let hits = (0..n)
            .filter(|_| {
                banner.features(&catalog()[choose(Rarity::Legendary, Some(banner), &mut state)])
            })
            .count();
        let got = hits as f64 / f64::from(n);
        assert!((got - FEATURED_SHARE).abs() < 0.02, "{got}");
        // The standard banner has none.
        let hits = (0..n)
            .filter(|_| banner.features(&catalog()[choose(Rarity::Legendary, None, &mut state)]))
            .count();
        let legendaries = catalog()
            .iter()
            .filter(|p| p.rarity == Rarity::Legendary)
            .count();
        let even = 2.0 / legendaries as f64;
        assert!((hits as f64 / f64::from(n) - even).abs() < 0.03);
    }

    #[test]
    fn duplicates_turn_into_favor() {
        let banner = banner_at(0);
        let mut p = rich(10_000, 11);
        // Own every prize: whatever lands is a duplicate.
        for prize in catalog() {
            grant(&mut p, prize);
        }
        let pulls = pull(&mut p, &mut 0, BannerKind::Standard, banner, 10).unwrap();
        let favor: u32 = pulls.iter().map(|x| x.favor).sum();
        assert!(pulls
            .iter()
            .all(|x| !x.new && x.favor == x.prize().rarity.favor()));
        assert_eq!(p.faire.favor, favor);
        assert_eq!(
            Rarity::ALL.map(|r| r.favor()),
            [1, 3, 8, 25, 100],
            "the Favor table"
        );
        // And on an empty account, a first pull is new and pays none.
        let mut fresh = rich(10, 11);
        let x = pull(&mut fresh, &mut 0, BannerKind::Standard, banner, 1).unwrap()[0];
        assert!(x.new && x.favor == 0 && owned(&fresh, x.prize()));
    }

    #[test]
    fn the_counter_sells_for_favor_and_discounts_the_banner() {
        let scourge = banner_at(0);
        let aq = banner_at(5);
        let mut p = Profile::default();
        p.faire.favor = 1_500;
        let tank = by_id("mount:horn_of_the_black_war_wolf").unwrap();
        assert_eq!(price(tank, scourge), 1_000);
        assert_eq!(price(tank, aq), 800, "20% off on its banner");
        assert_eq!(
            Rarity::ALL.map(|r| r.price()),
            [15, 40, 100, 300, 1000],
            "the price table"
        );
        assert_eq!(buy(&mut p, &tank.id, aq), Ok(800));
        assert_eq!(p.faire.favor, 700);
        assert!(p.quests.mounts.contains("horn_of_the_black_war_wolf"));
        assert_eq!(buy(&mut p, &tank.id, aq), Err(Refusal::Owned));
        let raptor = by_id("mount:swift_razzashi_raptor").unwrap();
        assert_eq!(buy(&mut p, &raptor.id, aq), Err(Refusal::Short(300)));
        assert_eq!(p.faire.favor, 700, "a refused sale costs nothing");
        let card = by_id("card:three_of_portals").unwrap();
        assert_eq!(card.kind, Kind::DarkmoonCard);
        assert_eq!(buy(&mut p, &card.id, aq), Ok(100));
        assert!(p.faire.cards.contains("three_of_portals"));
        assert_eq!(buy(&mut p, "mount:hippogryph", aq), Err(Refusal::Unknown));
    }

    #[test]
    fn a_first_pull_seeds_the_dice_from_the_account() {
        let banner = banner_at(0);
        let mut a = Profile::default();
        a.faire.tickets = 10;
        a.account.kills = 12_345;
        let mut b = a.clone();
        b.account.kills = 12_346;
        pull(&mut a, &mut 0, BannerKind::Standard, banner, 1).unwrap();
        pull(&mut b, &mut 0, BannerKind::Standard, banner, 1).unwrap();
        assert!(a.faire.dice.is_some());
        assert_ne!(a.faire.dice, b.faire.dice);
        assert_eq!(seed(&a.account), seed(&a.account.clone()));
    }
}
