//! Jewelry: rings, trinkets and necklaces found during a run. Each hero wears up to two rings, two
//! trinkets and a necklace ([`SLOTS`], its [`Jewelry`]), which start empty every run; what a worn
//! piece grants joins the hero's bonuses ([`wear`]), so it reaches [`HeroState::stats`] as every
//! other bonus does.
//!
//! Opening a cache or turning in a quest ([`treasure::open_cache`]) banks a jewelry choice instead
//! of the free level-up hand it gave before ([`Run::pending_jewelry`]). While playing, a banked
//! choice pauses the run on [`Phase::Jewelry`] once any level-up hands are made ([`open_jewelry`]):
//! every hero is dealt one choice for each ([`deal`]), the heroes no one here picks for choose at
//! random, and the local hero is offered three pieces ([`roll_pool`]: weighted to rarity by its
//! luck and the run's level, never one it wears). It takes one, or gold instead; a piece whose
//! slots are full asks which worn one it replaces ([`act`]). With nothing left to offer, the
//! choices become level-up hands. [`offer`] puts it on screen.
//!
//! Every piece ever offered and taken is logged per account in the profile's
//! [`CodexSave`] ([`codex`]), which the Armory's Codex tab shows.

mod catalog;
mod codex;
mod offer;

use std::ops::DerefMut;

use bevy::prelude::*;

use super::bonus::{Bonus, Bonuses, School, StatKey};
pub(super) use super::cards::Rarity;
use super::faire::catalog::deck_complete;
use super::mechanics::{self, Effect, MechanicId, MechanicSpec, Of, Trigger};
use super::profile::Profile;
use super::{hero, party, treasure, HeroState, LocalHero, Phase, Rng, Run, Stats};
pub(super) use codex::{CodexAction, CodexSave, CodexView};

/// The jewelry offer's and the Codex's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    phases(app);
    app.init_resource::<JewelOffer>()
        .init_resource::<CodexView>()
        .add_systems(
            Update,
            open_jewelry
                .after(treasure::open_caches)
                .before(hero::hero_vitals)
                .run_if(super::strip::world_runs)
                .run_if(super::party::simulates),
        )
        .add_systems(
            Update,
            (
                offer::spawn_root,
                offer::jewel_input,
                offer::auto_pick.run_if(resource_exists::<super::auto::Auto>),
                offer::log_seen,
                offer::refresh_offer,
                offer::paint_buttons,
            )
                .chain(),
        )
        .add_systems(OnEnter(Phase::Playing), equip_pinned_darkmoon)
        .add_systems(Update, codex::codex_clicks);
}

/// A pinned completed-deck trinket occupies the first free trinket slot as play begins.
fn equip_pinned_darkmoon(
    profile: Option<Res<Profile>>,
    mut run: ResMut<Run>,
    mut heroes: Query<&mut HeroState, With<LocalHero>>,
) {
    if run.pinned_trinket {
        return;
    }
    let Some(profile) = profile else { return };
    if let Ok(mut hero) = heroes.single_mut() {
        equip_pinned(&profile, &mut hero);
    }
    run.pinned_trinket = true;
}

fn equip_pinned(profile: &Profile, hero: &mut HeroState) -> bool {
    let Some(key) = profile.faire.wardrobe.pinned_trinket.as_deref() else {
        return false;
    };
    let Some(piece) = piece(key) else {
        return false;
    };
    if !darkmoon_pool(profile)
        .iter()
        .any(|owned| owned.key == piece.key)
    {
        return false;
    }
    wear(hero, piece, None).is_ok()
}

/// The jewelry choice pauses the run as the level-up hand does, and deals as it opens.
fn phases(app: &mut App) {
    app.add_systems(
        OnEnter(Phase::Jewelry),
        (super::pause, deal_jewelry).chain(),
    );
}

/// Where a piece is worn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Kind {
    Ring,
    Trinket,
    Necklace,
}

impl Kind {
    pub(super) const ALL: [Kind; 3] = [Kind::Ring, Kind::Trinket, Kind::Necklace];

    /// One of the kind, as a card names its slot.
    pub(super) fn label(self) -> &'static str {
        match self {
            Kind::Ring => "Ring",
            Kind::Trinket => "Trinket",
            Kind::Necklace => "Necklace",
        }
    }

    /// The kind's heading in a list.
    pub(super) fn plural(self) -> &'static str {
        match self {
            Kind::Ring => "Rings",
            Kind::Trinket => "Trinkets",
            Kind::Necklace => "Necklaces",
        }
    }
}

/// A hero's jewelry slots, in order: two rings, two trinkets and a necklace.
pub(super) const SLOTS: [Kind; 5] = [
    Kind::Ring,
    Kind::Ring,
    Kind::Trinket,
    Kind::Trinket,
    Kind::Necklace,
];

/// One piece of jewelry.
#[derive(Debug, PartialEq)]
pub(super) struct Piece {
    /// Stable across releases: the Codex saves it, so a piece keeps its key for good.
    pub key: &'static str,
    pub kind: Kind,
    pub rarity: Rarity,
    pub name: &'static str,
    /// The `ItemDisplayInfo.dbc` row the real item is drawn with, which the icon comes from; kept
    /// for the test that holds the two together, since nothing draws jewelry on the hero.
    #[allow(dead_code)]
    pub display_id: u32,
    /// The display's inventory icon: an `Interface\Icons\` basename without extension.
    pub icon: &'static str,
    pub bonuses: &'static [Bonus],
    /// The shared mechanics library's stable proc key.  Greens are intentionally stat-only.
    pub proc: Option<&'static str>,
}

/// Every piece, rings then trinkets then necklaces, each Uncommon first.
pub(super) fn pieces() -> &'static [Piece] {
    catalog::CATALOG
}

/// The piece with key `key`.
pub(super) fn piece(key: &str) -> Option<&'static Piece> {
    catalog::CATALOG
        .iter()
        .chain(catalog::DARKMOON)
        .find(|p| p.key == key)
}

/// The Darkmoon trinket keyed `key`, and only a Darkmoon trinket: what another player's pinned
/// trinket may be (its deck is its own profile's to check), never an arbitrary catalog piece.
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
pub(super) fn darkmoon_piece(key: &str) -> Option<&'static Piece> {
    catalog::DARKMOON.iter().find(|p| p.key == key)
}

/// Deck trinkets unlocked by this account.  Each appears four times in the selection pool, making
/// a completed deck feel like an earned run-starting reward without displacing normal jewelry.
pub(super) fn darkmoon_pool(profile: &Profile) -> Vec<&'static Piece> {
    ["beasts", "elementals", "portals", "warlords"]
        .into_iter()
        .filter(|suit| deck_complete(profile, suit))
        .filter_map(|suit| {
            catalog::DARKMOON.iter().find(|p| {
                p.key
                    == format!(
                        "darkmoon_{}",
                        match suit {
                            "beasts" => "blue_dragon",
                            "elementals" => "maelstrom",
                            "portals" => "twisting_nether",
                            _ => "heroism",
                        }
                    )
            })
        })
        .flat_map(|piece| std::iter::repeat_n(piece, 4))
        .collect()
}

/// The normal catalog plus the account's completed-deck trinkets.  A trinket occurs four times:
/// the weighted roller removes only the picked entry, making it markedly more likely without
/// permitting duplicate offers of the same piece.
pub(super) fn pool(profile: Option<&Profile>) -> Vec<&'static Piece> {
    let mut out: Vec<_> = catalog::CATALOG.iter().collect();
    if let Some(profile) = profile {
        out.extend(darkmoon_pool(profile));
    }
    out
}

fn darkmoon_specs(key: &str) -> &'static [MechanicSpec] {
    const BLUE: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnCast(Of::Any),
        Effect::Burst {
            duration: 15.0,
            bonuses: &[Bonus::new(StatKey::Cooldown, 0.50)],
        },
    )
    .visual(16618)
    .cooldown(15.0)];
    const HEROISM: [MechanicSpec; 2] = [
        MechanicSpec::new(
            Trigger::OnHit {
                of: Of::Any,
                chance: 0.15,
            },
            Effect::HealPulse {
                radius: 12.0,
                amount: 30.0,
            },
        )
        .visual(23513)
        .cooldown(8.0),
        MechanicSpec::new(
            Trigger::OnHit {
                of: Of::Any,
                chance: 0.15,
            },
            Effect::Burst {
                duration: 8.0,
                bonuses: &[Bonus::new(StatKey::Damage, 0.25)],
            },
        )
        .visual(23513)
        .cooldown(8.0),
    ];
    const MAELSTROM: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.20,
        },
        Effect::Nova {
            school: School::Nature,
            radius: 4.0,
            damage: 28.0,
            knockback: 0.0,
            cc: None,
        },
    )
    .visual(21179)
    .cooldown(3.0)];
    const NETHER: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnLethal,
        Effect::CheatDeath {
            immune: 2.0,
            heal_frac: 0.35,
        },
    )
    .visual(20707)
    .cooldown(99_999.0)];
    match key {
        "darkmoon_blue_dragon" => &BLUE,
        "darkmoon_heroism" => &HEROISM,
        "darkmoon_maelstrom" => &MAELSTROM,
        "darkmoon_twisting_nether" => &NETHER,
        _ => &[],
    }
}

/// The kinds of proc blue-and-better jewelry carries: each has one mechanic ([`proc_specs`]) and
/// the one line its card shows ([`proc_text`]), so the two never drift apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcKind {
    SpellBurst,
    ExtraAttack,
    Crit,
    Shield,
    DecayingPower,
    Reset,
    HolyNova,
    Chain,
    Heal,
}

/// Which proc a catalog piece's key carries, echoing the real item where it had one.
fn proc_kind(key: &str) -> ProcKind {
    match key {
        "talisman_of_ephemeral_power" => ProcKind::SpellBurst,
        "hand_of_justice" | "band_of_accuria" => ProcKind::ExtraAttack,
        "blackhands_breadth" => ProcKind::Crit,
        "royal_seal_of_eldre_thalas" => ProcKind::Shield,
        "zandalarian_hero_charm" => ProcKind::DecayingPower,
        "eye_of_moam" => ProcKind::Reset,
        "mark_of_the_champion" => ProcKind::HolyNova,
        "painweaver_band" | "ring_of_spell_power" | "choker_of_the_fire_lord" => ProcKind::Chain,
        "diamond_flask" | "mark_of_fordring" | "gem_of_trapped_innocents" => ProcKind::Heal,
        // The remaining real pieces keep a quiet, useful on-hit extra strike, on an independent
        // cooldown when two are worn together.
        _ => ProcKind::ExtraAttack,
    }
}

/// The small, distinct procs on blue-and-better jewelry.  Their flat values are deliberately
/// below a rank-one spell: a find changes the build's rhythm, not its whole damage budget.
fn proc_specs(kind: ProcKind) -> &'static [MechanicSpec] {
    const SPELL_BURST: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::Periodic { every: 18.0 },
        Effect::Burst {
            duration: 6.0,
            bonuses: &[Bonus::new(StatKey::Damage, 0.12)],
        },
    )
    .visual(23271)
    .cooldown(18.0)];
    const EXTRA_ATTACK: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.10,
        },
        Effect::Echo { extra_casts: 1 },
    )
    // Hand of Justice's own extra attack.
    .visual(15601)
    .cooldown(4.0)];
    const CRIT: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnCrit {
            of: Of::Any,
            chance: 1.0,
        },
        Effect::NextCast {
            instant: false,
            crit: true,
            cooldown: 0.8,
            aura: 12043,
        },
    )
    // Elemental Mastery's flash: a next-spell-crits guarantee with a cast and an impact kit, where
    // Presence of Mind's (the held aura) has only a cast and a state.
    .visual(16166)
    .cooldown(12.0)];
    const SHIELD: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnLowHealth { threshold: 0.35 },
        Effect::Shield {
            amount: 28.0,
            duration: 5.0,
        },
    )
    .visual(17)
    .cooldown(30.0)];
    const DECAYING_POWER: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::Periodic { every: 24.0 },
        Effect::Burst {
            duration: 8.0,
            bonuses: &[Bonus::new(StatKey::Damage, 0.16)],
        },
    )
    // Zandalarian Hero Charm's own Unstable Power.
    .visual(24658)
    .cooldown(24.0)];
    const RESET: [MechanicSpec; 1] =
        [
            MechanicSpec::new(Trigger::Periodic { every: 35.0 }, Effect::ResetCooldowns)
                .visual(26166)
                .cooldown(35.0),
        ];
    // Mark of the Champion's: its wrath is for the undead and demons alone, a holy smite on the
    // one struck.
    const HOLY_NOVA: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnHit {
            of: Of::Bane,
            chance: 0.30,
        },
        Effect::Nova {
            school: School::Holy,
            radius: 1.5,
            damage: 14.0,
            knockback: 0.0,
            cc: None,
        },
    )
    .visual(23455)
    .cooldown(5.0)];
    const CHAIN: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnHit {
            of: Of::Any,
            chance: 0.12,
        },
        Effect::Chain {
            jumps: 2,
            falloff: 0.65,
        },
    )
    .visual(421)
    .cooldown(3.0)];
    const HEAL: [MechanicSpec; 1] = [MechanicSpec::new(
        Trigger::OnKill {
            of: Of::Any,
            chance: 0.15,
        },
        Effect::HealPulse {
            radius: 10.0,
            amount: 12.0,
        },
    )
    .visual(2054)
    .cooldown(5.0)];
    match kind {
        ProcKind::SpellBurst => &SPELL_BURST,
        ProcKind::ExtraAttack => &EXTRA_ATTACK,
        ProcKind::Crit => &CRIT,
        ProcKind::Shield => &SHIELD,
        ProcKind::DecayingPower => &DECAYING_POWER,
        ProcKind::Reset => &RESET,
        ProcKind::HolyNova => &HOLY_NOVA,
        ProcKind::Chain => &CHAIN,
        ProcKind::Heal => &HEAL,
    }
}

/// The card line for a piece's proc, by its key: a Darkmoon trinket's own mechanic, else its
/// [`ProcKind`]'s. Every number here is the one in [`darkmoon_specs`] / [`proc_specs`].
pub(super) fn proc_text(key: &str) -> &'static str {
    match key {
        "darkmoon_blue_dragon" => "On cast: cooldowns 50% faster for 15 sec",
        "darkmoon_heroism" => "15% on hit: heal allies near for 30 and +25% damage for 8 sec",
        "darkmoon_maelstrom" => "20% on hit: a lightning nova for 28 damage",
        "darkmoon_twisting_nether" => "Once a run: cheat death, back up at 35% health",
        _ => match proc_kind(key) {
            ProcKind::SpellBurst => "Every 18 sec: +12% damage for 6 sec",
            ProcKind::ExtraAttack => "10% on hit: strike again",
            ProcKind::Crit => "On crit: your next cast crits (every 12 sec)",
            ProcKind::Shield => "Below 35% health: absorb 28 damage (every 30 sec)",
            ProcKind::DecayingPower => "Every 24 sec: +16% damage for 8 sec",
            ProcKind::Reset => "Every 35 sec: reset your cooldowns",
            ProcKind::HolyNova => "30% on hit vs undead and demons: a holy smite for 14",
            ProcKind::Chain => "12% on hit: the blow chains to 2 more foes",
            ProcKind::Heal => "15% on kill: heal allies near for 12",
        },
    }
}

/// What a worn piece grants as a mechanic: a Darkmoon trinket its own (never a generic proc),
/// else its proc's, and a green nothing.
fn mechanic_specs(piece: &Piece) -> &'static [MechanicSpec] {
    let darkmoon = darkmoon_specs(piece.key);
    if !darkmoon.is_empty() {
        return darkmoon;
    }
    match piece.proc {
        Some(key) => proc_specs(proc_kind(key)),
        None => &[],
    }
}

/// Every piece's mechanic by its key, the Darkmoon trinkets' too: the data test holds their
/// visuals to the real spells'.
#[cfg(test)]
pub(super) fn every_mechanic() -> Vec<(&'static str, &'static [MechanicSpec])> {
    pieces()
        .iter()
        .chain(catalog::DARKMOON)
        .map(|p| (p.key, mechanic_specs(p)))
        .filter(|(_, specs)| !specs.is_empty())
        .collect()
}

/// Give a worn piece its run mechanic: a Darkmoon trinket's, or a blue-or-better piece's proc.
pub(super) fn grant_darkmoon(hero: &mut HeroState, piece: &Piece) {
    let specs = mechanic_specs(piece);
    if !specs.is_empty() {
        mechanics::grant(
            hero,
            MechanicId::Jewelry(piece.display_id),
            specs.iter().copied(),
        );
    }
}

/// What putting on `new` in place of `old` changes, stat by stat: each bonus `new` grants less
/// what `old` did, the ones that cancel out left off.
pub(super) fn stat_deltas(new: &Piece, old: &Piece) -> Vec<Bonus> {
    let mut out: Vec<Bonus> = Vec::new();
    let mut add = |b: &Bonus, sign: f32| match out
        .iter_mut()
        .find(|o| o.stat == b.stat && o.school == b.school)
    {
        Some(o) => o.value += sign * b.value,
        None => out.push(Bonus {
            value: sign * b.value,
            ..*b
        }),
    };
    for b in new.bonuses {
        add(b, 1.0);
    }
    for b in old.bonuses {
        add(b, -1.0);
    }
    out.retain(|b| b.value.abs() > 1e-6);
    out
}

/// What one hero wears this run, and the jewelry choices it has still to make. Empty as each run
/// begins, since every run starts a fresh [`HeroState`].
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Jewelry {
    slots: [Option<&'static Piece>; SLOTS.len()],
    /// Jewelry choices dealt to this hero ([`deal`]) and not yet made.
    pub picks: u32,
}

impl Jewelry {
    /// The piece slot `i` of [`SLOTS`] holds.
    pub(super) fn slot(&self, i: usize) -> Option<&'static Piece> {
        self.slots.get(i).copied().flatten()
    }

    /// Every piece worn, in slot order.
    pub(super) fn worn(&self) -> impl Iterator<Item = &'static Piece> + '_ {
        self.slots.iter().flatten().copied()
    }

    /// Whether `piece` is worn in any slot.
    pub(super) fn wears(&self, piece: &Piece) -> bool {
        self.worn().any(|p| p.key == piece.key)
    }

    /// The slots of `kind`, as indices into [`SLOTS`].
    pub(super) fn slots_of(kind: Kind) -> impl Iterator<Item = usize> {
        SLOTS
            .iter()
            .enumerate()
            .filter(move |(_, k)| **k == kind)
            .map(|(i, _)| i)
    }

    /// The first empty slot of `kind`.
    pub(super) fn free(&self, kind: Kind) -> Option<usize> {
        Self::slots_of(kind).find(|&i| self.slots[i].is_none())
    }

    /// What the worn pieces grant, summed.
    pub(super) fn bonuses(&self) -> Bonuses {
        let mut out = Bonuses::default();
        for p in self.worn() {
            out.add_all(p.bonuses);
        }
        out
    }
}

/// Why a piece could not go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refused {
    /// Every slot of its kind is taken: name one to replace.
    Full,
    /// The slot named is not of its kind.
    WrongSlot,
    /// The hero wears it already.
    AlreadyWorn,
}

/// `piece` goes on `hero`: into `replace` when named (a slot of its kind, whatever it holds), else
/// into a free slot of its kind. Its bonuses join the hero's and the replaced piece's leave them;
/// health follows max health, healed by a raise and never left above a cut. Returns the piece
/// replaced.
pub(super) fn wear(
    hero: &mut HeroState,
    piece: &'static Piece,
    replace: Option<usize>,
) -> Result<Option<&'static Piece>, Refused> {
    if hero.jewelry.wears(piece) {
        return Err(Refused::AlreadyWorn);
    }
    let slot = match replace {
        Some(slot) if SLOTS.get(slot) == Some(&piece.kind) => slot,
        Some(_) => return Err(Refused::WrongSlot),
        None => hero.jewelry.free(piece.kind).ok_or(Refused::Full)?,
    };
    let before = hero.stats().max_hp;
    let old = hero.jewelry.slots[slot].replace(piece);
    if let Some(old) = old {
        mechanics::revoke(hero, MechanicId::Jewelry(old.display_id));
    }
    for b in old.iter().flat_map(|o| o.bonuses) {
        hero.bonuses.add(&Bonus {
            value: -b.value,
            ..*b
        });
    }
    hero.bonuses.add_all(piece.bonuses);
    grant_darkmoon(hero, piece);
    // A piece of a school counts toward its set.
    super::synergy::sync_sets(hero);
    let after = hero.stats().max_hp;
    if hero.alive() && after > before {
        hero.hp += after - before;
    }
    hero.hp = hero.hp.min(after);
    Ok(old)
}

/// How many pieces an offer shows.
pub(super) const OFFER_SIZE: usize = 3;
/// How much more each rarity tier above Uncommon weighs for every run level past the first.
const LEVEL_TILT: f32 = 0.05;
/// What taking gold instead of a piece pays, in coins.
const GOLD_COINS: u32 = 10;

/// What one piece of `rarity` weighs in an offer to a hero with `stats` at run level `level`: the
/// rarity's own weight, raised for each tier above Uncommon by the hero's luck (as the level-up
/// hand weighs its rarer cards, [`Stats::luck_weight`]) and by [`LEVEL_TILT`] a level.
pub(super) fn weight(rarity: Rarity, stats: &Stats, level: u32) -> f32 {
    let base = match rarity {
        Rarity::Common => 0.0,
        Rarity::Uncommon => 1.0,
        Rarity::Rare => 0.5,
        Rarity::Epic => 0.2,
        Rarity::Legendary => 0.06,
    };
    let tier = (rarity as i32 - Rarity::Uncommon as i32).max(0);
    let tilt = (1.0 + LEVEL_TILT * level.saturating_sub(1) as f32).powi(tier);
    base * stats.luck_weight(tier) * tilt
}

/// Up to [`OFFER_SIZE`] pieces of `catalog` (an account-aware pool, [`pool`]) for a hero wearing
/// `worn`, by [`weight`], never one it wears and none twice. Repeated entries are intentional high
/// weight; after a pick all siblings are removed, so a hand never contains the same trinket twice.
pub(super) fn roll_pool(
    catalog: &[&'static Piece],
    worn: &Jewelry,
    stats: &Stats,
    level: u32,
    rng: &mut Rng,
) -> Vec<&'static Piece> {
    let mut pool: Vec<_> = catalog
        .iter()
        .copied()
        .filter(|p| !worn.wears(p))
        .map(|p| (p, weight(p.rarity, stats, level)))
        .filter(|(_, weight)| *weight > 0.0)
        .collect();
    let mut out = Vec::with_capacity(OFFER_SIZE);
    while out.len() < OFFER_SIZE && !pool.is_empty() {
        let mut pick = rng.f32() * pool.iter().map(|(_, weight)| weight).sum::<f32>();
        let mut index = pool.len() - 1;
        for (i, (_, weight)) in pool.iter().enumerate() {
            if pick < *weight {
                index = i;
                break;
            }
            pick -= weight;
        }
        let piece = pool.swap_remove(index).0;
        pool.retain(|(other, _)| other.key != piece.key);
        out.push(piece);
    }
    out
}

/// The run's banked jewelry choices become every hero's to make, one each, and the bank empties.
pub(super) fn deal<H: DerefMut<Target = HeroState>>(
    run: &mut Run,
    heroes: impl IntoIterator<Item = H>,
) {
    for mut hero in heroes {
        hero.jewelry.picks += run.pending_jewelry;
    }
    run.pending_jewelry = 0;
}

/// `piece` on `hero` as a co-op hand gives it, with no slot to pick: into a free slot of its kind,
/// else in place of the plainest piece of that kind.
pub(super) fn put_on(hero: &mut HeroState, piece: &'static Piece) {
    let replace = match hero.jewelry.free(piece.kind) {
        Some(_) => None,
        None => Jewelry::slots_of(piece.kind)
            .min_by_key(|&i| hero.jewelry.slot(i).map_or(-1, |p| p.rarity as i32)),
    };
    let _ = wear(hero, piece, replace);
}

/// A hero no one picks for (the test aid's extras) takes a random piece from each offer it is
/// dealt, replacing a random one of its kind when its slots are full; with nothing left to offer
/// it takes a random level-up card instead.
pub(super) fn choose_at_random(
    hero: &mut HeroState,
    catalog: &[&'static Piece],
    level: u32,
    rng: &mut Rng,
) {
    while hero.jewelry.picks > 0 {
        hero.jewelry.picks -= 1;
        let offer = roll_pool(catalog, &hero.jewelry, &hero.stats(), level, rng);
        if offer.is_empty() {
            hero.picks += 1;
            party::pick_at_random(hero, rng);
            continue;
        }
        let piece = offer[rng.index(offer.len())];
        let replace = match hero.jewelry.free(piece.kind) {
            Some(_) => None,
            None => {
                let slots: Vec<usize> = Jewelry::slots_of(piece.kind).collect();
                Some(slots[rng.index(slots.len())])
            }
        };
        let _ = wear(hero, piece, replace);
    }
}

/// The local hero's offer on screen.
#[derive(Resource, Default, Debug)]
pub(super) struct JewelOffer {
    pieces: Vec<&'static Piece>,
    pool: Vec<&'static Piece>,
    /// The piece picked while its slots are full, by its index in `pieces`: the hero is choosing
    /// which worn piece it replaces.
    replacing: Option<usize>,
}

impl JewelOffer {
    pub(super) fn pieces(&self) -> &[&'static Piece] {
        &self.pieces
    }

    /// The piece waiting for a slot to replace, while one is.
    pub(super) fn replacing(&self) -> Option<&'static Piece> {
        self.pieces.get(self.replacing?).copied()
    }
}

/// Deal the local hero's next offer from the choices it has left: true while one is up. With
/// nothing in `catalog` left to offer, its remaining choices become level-up hands instead
/// ([`HeroState::picks`]), which [`resume`] then opens.
pub(super) fn next_offer(
    hero: &mut HeroState,
    offer: &mut JewelOffer,
    catalog: &[&'static Piece],
    level: u32,
    rng: &mut Rng,
) -> bool {
    offer.replacing = None;
    offer.pieces.clear();
    if hero.jewelry.picks == 0 {
        return false;
    }
    offer.pieces = roll_pool(catalog, &hero.jewelry, &hero.stats(), level, rng);
    if offer.pieces.is_empty() {
        hero.picks += hero.jewelry.picks;
        hero.jewelry.picks = 0;
        return false;
    }
    true
}

/// Where play goes once the local hero's jewelry choices are made: to its level-up hands if it
/// holds any, else back to the run.
pub(super) fn resume(hero: &HeroState) -> Phase {
    if hero.picks > 0 {
        Phase::LevelUp
    } else {
        Phase::Playing
    }
}

/// Something done with the offer on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Act {
    /// Take piece `i` of the offer.
    Take(usize),
    /// Put the piece waiting for a slot into slot `i` of [`SLOTS`].
    Replace(usize),
    /// Gold instead of a piece.
    Gold,
    /// Take the offer's rarest piece apart into Enchanting's materials instead of a piece
    /// (docs/PROFESSIONS.md, 5.2).
    Disenchant,
    /// Back from choosing a slot to replace.
    Back,
}

/// A choice made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Done {
    /// The piece went on, over the one it replaced.
    Took(&'static Piece, Option<&'static Piece>),
    /// Gold instead, this much, into the party's purse.
    Gold(u32),
    /// The offer's rarest piece disenchanted: its materials go into the bags.
    Disenchanted(&'static Piece),
}

/// The piece a Disenchant takes apart: the offer's rarest.
pub(super) fn to_disenchant(offer: &JewelOffer) -> Option<&'static Piece> {
    offer.pieces.iter().copied().max_by_key(|p| p.rarity as u8)
}

/// A piece's rarity as a 1.12 item quality (2 Uncommon to 5 Legendary), which a disenchant's
/// yield follows.
pub(super) fn quality(piece: &Piece) -> u32 {
    piece.rarity as u32 + 1
}

/// What taking gold instead pays: [`GOLD_COINS`] coins at what a coin is worth now under the
/// hero's gold multiplier ([`treasure::worth`]), as the level-up hand's Skip pays.
pub(super) fn gold_instead(run: &Run, hero: &HeroState) -> u32 {
    treasure::worth(GOLD_COINS, run.elapsed, hero.stats().gold, 0.5)
}

/// `act` on the local hero's offer. Taking a piece whose slots are full waits for a slot to
/// replace (the offer's [`JewelOffer::replacing`]); the choice is made when this returns one.
pub(super) fn act(
    act: Act,
    hero: &mut HeroState,
    run: &mut Run,
    offer: &mut JewelOffer,
) -> Option<Done> {
    match act {
        Act::Take(i) => {
            if offer.replacing.is_some() {
                return None;
            }
            let piece = *offer.pieces.get(i)?;
            match wear(hero, piece, None) {
                Ok(old) => Some(Done::Took(piece, old)),
                Err(Refused::Full) => {
                    offer.replacing = Some(i);
                    None
                }
                Err(_) => None,
            }
        }
        Act::Replace(slot) => {
            let piece = offer.replacing()?;
            let old = wear(hero, piece, Some(slot)).ok()?;
            offer.replacing = None;
            Some(Done::Took(piece, old))
        }
        Act::Gold => {
            if offer.replacing.is_some() {
                return None;
            }
            let gold = gold_instead(run, hero);
            run.gold += gold;
            Some(Done::Gold(gold))
        }
        Act::Disenchant => {
            if offer.replacing.is_some() {
                return None;
            }
            to_disenchant(offer).map(Done::Disenchanted)
        }
        Act::Back => {
            offer.replacing = None;
            None
        }
    }
}

/// A choice made: its banner, then the next offer, or where play goes on ([`resume`]).
pub(super) fn close(
    done: Done,
    hero: &mut HeroState,
    run: &mut Run,
    offer: &mut JewelOffer,
    rng: &mut Rng,
) -> Option<Phase> {
    run.banner = Some((
        match done {
            Done::Took(piece, Some(old)) => format!("{} replaces {}", piece.name, old.name),
            Done::Took(piece, None) => format!("Equipped {}", piece.name),
            Done::Gold(gold) => format!("Took the gold: +{gold} gold"),
            Done::Disenchanted(piece) => format!("Disenchanted {}", piece.name),
        },
        2.5,
    ));
    hero.jewelry.picks = hero.jewelry.picks.saturating_sub(1);
    if hero.jewelry.picks == 0 {
        offer.pieces.clear();
        offer.replacing = None;
        return Some(resume(hero));
    }
    // The account's pool when the picker dealt one, else the catalog; either way nothing left to
    // offer turns the remaining choices into level-up hands ([`next_offer`]).
    let pool = if offer.pool.is_empty() {
        pieces().iter().collect()
    } else {
        offer.pool.clone()
    };
    let has_offer = next_offer(hero, offer, &pool, run.level, rng);
    (!has_offer).then(|| resume(hero))
}

/// While playing, a banked jewelry choice opens the picker, once the level-up hands are made and
/// unless a transition is already queued this frame (the run lost or won).
fn open_jewelry(run: Res<Run>, mut next: ResMut<NextState<Phase>>) {
    if run.pending_jewelry > 0 && run.pending_levels == 0 && matches!(*next, NextState::Unchanged) {
        next.set(Phase::Jewelry);
    }
}

/// The picker opens: the banked choices are dealt to every hero ([`deal`]), the bots standing
/// choose at random, and the local hero's first offer is dealt if it is up. A downed hero's
/// choices wait for its revive ([`party::open_banked_picks`]); another player's are theirs to make
/// on their own screen.
fn deal_jewelry(
    mut run: ResMut<Run>,
    mut heroes: Query<(&mut HeroState, Has<LocalHero>, Has<party::Bot>)>,
    mut rng: ResMut<Rng>,
    mut offer: ResMut<JewelOffer>,
    mut next: ResMut<NextState<Phase>>,
    profile: Option<Res<Profile>>,
) {
    deal(&mut run, heroes.iter_mut().map(|(hero, ..)| hero));
    let level = run.level;
    for (mut hero, _, bot) in &mut heroes {
        if party::auto_picks(&hero, bot) {
            let pool = pool(profile.as_deref());
            choose_at_random(&mut hero, &pool, level, &mut rng);
        }
    }
    match heroes
        .iter_mut()
        .find(|(hero, local, ..)| *local && hero.alive())
    {
        Some((mut hero, ..)) => {
            let pool = pool(profile.as_deref());
            offer.pool = pool.clone();
            if !next_offer(&mut hero, &mut offer, &pool, level, &mut rng) {
                next.set(resume(&hero));
            }
        }
        None => {
            offer.pieces.clear();
            offer.replacing = None;
            next.set(Phase::Playing);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::bonus::StatKey;
    use super::super::data;
    use super::*;

    const RARITIES: [Rarity; 4] = [
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Epic,
        Rarity::Legendary,
    ];

    fn close_to(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn by_key(key: &str) -> &'static Piece {
        piece(key).unwrap_or_else(|| panic!("no piece {key}"))
    }

    /// [`roll_pool`] over a slice of the catalog.
    fn roll(
        catalog: &'static [Piece],
        worn: &Jewelry,
        stats: &Stats,
        level: u32,
        rng: &mut Rng,
    ) -> Vec<&'static Piece> {
        let pool: Vec<&'static Piece> = catalog.iter().collect();
        roll_pool(&pool, worn, stats, level, rng)
    }

    #[test]
    fn darkmoon_pool_only_contains_completed_decks() {
        let mut profile = Profile::default();
        assert!(darkmoon_pool(&profile).is_empty());
        for rank in [
            "ace", "two", "three", "four", "five", "six", "seven", "eight",
        ] {
            profile.faire.cards.insert(format!("{rank}_of_beasts"));
        }
        let pool = darkmoon_pool(&profile);
        assert_eq!(pool.len(), 4, "completed deck has high offer weight");
        assert!(pool.iter().all(|piece| piece.key == "darkmoon_blue_dragon"));
    }

    #[test]
    fn every_darkmoon_trinket_grants_its_mechanic() {
        for piece in catalog::DARKMOON {
            let mut hero = HeroState::new(0, &data::CLASSES[0], 1, Bonuses::default());
            grant_darkmoon(&mut hero, piece);
            assert!(mechanics::has(&hero, MechanicId::Jewelry(piece.display_id)));
            // Its own mechanic, never the generic proc its rarity would otherwise carry.
            let own = darkmoon_specs(piece.key);
            assert!(!own.is_empty(), "{}", piece.key);
            assert!(
                std::ptr::eq(mechanic_specs(piece), own),
                "{} lost its real mechanic",
                piece.key
            );
        }
    }

    #[test]
    fn a_replacement_shows_what_it_changes_stat_by_stat() {
        // Hand of Justice (+7% damage, +3% crit, +2% armor) over Blackhand's Breadth (+5% crit,
        // +10% crit damage).
        let deltas = stat_deltas(by_key("hand_of_justice"), by_key("blackhands_breadth"));
        let of = |stat| {
            deltas
                .iter()
                .find(|b| b.stat == stat)
                .map(|b| (b.value * 100.0).round())
        };
        assert_eq!(of(StatKey::Damage), Some(7.0));
        assert_eq!(of(StatKey::CritChance), Some(-2.0));
        assert_eq!(of(StatKey::CritDamage), Some(-10.0));
        assert_eq!(of(StatKey::Armor), Some(2.0));
        // A piece over itself changes nothing.
        let same = by_key("naglering");
        assert!(stat_deltas(same, same).is_empty());
    }

    #[test]
    fn proc_text_says_the_chance_the_proc_rolls() {
        let procs = pieces().iter().filter_map(|p| p.proc);
        for key in procs.chain(catalog::DARKMOON.iter().map(|p| p.key)) {
            let text = proc_text(key);
            let specs = darkmoon_specs(key);
            let specs = if specs.is_empty() {
                proc_specs(proc_kind(key))
            } else {
                specs
            };
            for spec in specs {
                let chance = match spec.trigger {
                    Trigger::OnHit { chance, .. } | Trigger::OnKill { chance, .. } => chance,
                    _ => continue,
                };
                let said = format!("{}% on", (chance * 100.0).round());
                assert!(text.contains(&said), "{key}: {text:?} should say {said:?}");
            }
        }
        assert_eq!(proc_text("band_of_rumination"), "10% on hit: strike again");
        assert_ne!(proc_text("diamond_flask"), proc_text("hand_of_justice"));
    }

    #[test]
    fn a_replaced_piece_takes_its_proc_with_it() {
        let mut hero = HeroState::new(0, &data::CLASSES[0], 1, Bonuses::default());
        let first = by_key("hand_of_justice");
        let second = by_key("blackhands_breadth");
        let third = by_key("diamond_flask");
        wear(&mut hero, first, None).unwrap();
        wear(&mut hero, second, None).unwrap();
        let slot = Jewelry::slots_of(Kind::Trinket)
            .find(|&i| hero.jewelry.slot(i) == Some(first))
            .unwrap();
        assert_eq!(wear(&mut hero, third, Some(slot)), Ok(Some(first)));
        assert!(!mechanics::has(
            &hero,
            MechanicId::Jewelry(first.display_id)
        ));
        assert!(mechanics::has(
            &hero,
            MechanicId::Jewelry(second.display_id)
        ));
        assert!(mechanics::has(&hero, MechanicId::Jewelry(third.display_id)));
        // Putting the first back over and over never stacks its proc.
        for _ in 0..3 {
            let slot = Jewelry::slots_of(Kind::Trinket)
                .find(|&i| hero.jewelry.slot(i) != Some(second))
                .unwrap();
            let _ = wear(&mut hero, first, Some(slot));
            let _ = wear(&mut hero, third, Some(slot));
        }
        let worn: Vec<_> = hero.jewelry.worn().map(|p| p.display_id).collect();
        let stale = hero.mechanics.clone().revoke_where(|id| match id {
            MechanicId::Jewelry(d) => !worn.contains(&d),
            _ => false,
        });
        assert_eq!(stale, 0, "a replaced piece's proc lingers");
        let jewelry = hero
            .mechanics
            .clone()
            .revoke_where(|id| matches!(id, MechanicId::Jewelry(_)));
        assert_eq!(jewelry, 2, "one proc for each of the two worn pieces");
    }

    #[test]
    fn only_a_darkmoon_trinket_may_be_pinned_by_another_player() {
        assert!(darkmoon_piece("darkmoon_heroism").is_some());
        assert!(darkmoon_piece("band_of_accuria").is_none());
        assert!(darkmoon_piece("no_such_piece").is_none());
    }

    #[test]
    fn pinned_darkmoon_trinket_equips_at_run_start() {
        let mut profile = Profile::default();
        for rank in [
            "ace", "two", "three", "four", "five", "six", "seven", "eight",
        ] {
            profile.faire.cards.insert(format!("{rank}_of_beasts"));
        }
        profile.faire.wardrobe.pinned_trinket = Some("darkmoon_blue_dragon".into());
        let mut hero = HeroState::new(0, &data::CLASSES[0], 1, Bonuses::default());
        assert!(equip_pinned(&profile, &mut hero));
        assert!(hero.jewelry.wears(piece("darkmoon_blue_dragon").unwrap()));
    }

    /// A rough measure of a piece's power: each bonus over what one "unit" of its stat is worth.
    fn budget(p: &Piece) -> f32 {
        p.bonuses
            .iter()
            .map(|b| {
                let unit = match b.stat {
                    StatKey::Damage => 0.05,
                    StatKey::CritChance => 0.02,
                    StatKey::CritDamage => 0.10,
                    StatKey::Cooldown => 0.04,
                    StatKey::Area => 0.05,
                    StatKey::MoveSpeed => 0.04,
                    StatKey::Pickup => 0.10,
                    StatKey::Armor => 0.03,
                    StatKey::Experience => 0.05,
                    StatKey::Gold => 0.06,
                    StatKey::Luck => 0.06,
                    StatKey::Lifesteal => 0.01,
                    StatKey::MaxHealth => 12.0,
                    StatKey::Regen => 0.4,
                    StatKey::Targets | StatKey::Revives => 1.0 / 3.0,
                };
                b.value / unit
            })
            .sum()
    }

    #[test]
    fn the_catalog_holds_fifteen_of_each_kind() {
        assert_eq!(pieces().len(), 45);
        for kind in Kind::ALL {
            let of: Vec<&Piece> = pieces().iter().filter(|p| p.kind == kind).collect();
            assert_eq!(of.len(), 15, "{kind:?}");
            for rarity in RARITIES {
                assert!(of.iter().any(|p| p.rarity == rarity), "{kind:?} {rarity:?}");
            }
            // Uncommon first, Legendary last.
            assert!(of
                .windows(2)
                .all(|w| (w[0].rarity as u8) <= (w[1].rarity as u8)));
        }
        let mut keys = HashSet::new();
        let mut names = HashSet::new();
        for p in pieces() {
            assert!(keys.insert(p.key), "key {} twice", p.key);
            assert!(names.insert(p.name), "name {} twice", p.name);
            assert!(p
                .key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'));
            assert!(!p.icon.is_empty() && p.display_id > 0, "{}", p.name);
            assert_ne!(p.rarity, Rarity::Common, "{}", p.name);
            assert_eq!(
                p.proc.is_some(),
                matches!(p.rarity, Rarity::Rare | Rarity::Epic | Rarity::Legendary),
                "{}",
                p.name
            );
            assert_eq!(piece(p.key), Some(p));
        }
        assert!(piece("ashbringer").is_none());
    }

    #[test]
    fn every_piece_grants_two_or_three_bonuses_sized_to_its_rarity() {
        // Each rarity's budget band, in stat units: a rarer piece always grants more.
        let band = |r: Rarity| match r {
            Rarity::Uncommon => (2.0, 3.0),
            Rarity::Rare => (3.0, 4.5),
            Rarity::Epic => (4.5, 6.5),
            _ => (6.5, 9.0),
        };
        for p in pieces() {
            assert!(
                (2..=3).contains(&p.bonuses.len()),
                "{}: {} bonuses",
                p.name,
                p.bonuses.len()
            );
            let (lo, hi) = band(p.rarity);
            let b = budget(p);
            assert!(b >= lo && b < hi, "{} ({:?}): budget {b}", p.name, p.rarity);
            for (i, a) in p.bonuses.iter().enumerate() {
                assert!(a.value > 0.0 && a.school.is_none(), "{}", p.name);
                assert!(
                    p.bonuses[i + 1..].iter().all(|b| b.stat != a.stat),
                    "{}: {:?} twice",
                    p.name,
                    a.stat
                );
            }
        }
    }

    #[test]
    fn slots_are_two_rings_two_trinkets_and_a_necklace() {
        let count = |k| Jewelry::slots_of(k).count();
        assert_eq!(
            (
                count(Kind::Ring),
                count(Kind::Trinket),
                count(Kind::Necklace)
            ),
            (2, 2, 1)
        );
        let j = Jewelry::default();
        assert_eq!(j.free(Kind::Ring), Some(0));
        assert_eq!(j.free(Kind::Trinket), Some(2));
        assert_eq!(j.free(Kind::Necklace), Some(4));
        assert_eq!(j.worn().count(), 0);
        // A fresh hero, and every run's, wears nothing.
        assert_eq!(HeroState::default().jewelry, Jewelry::default());
    }

    #[test]
    fn slot_replacement_rules() {
        let mut hero = HeroState::default();
        let (a, b, c) = (
            by_key("quick_strike_ring"),
            by_key("naglering"),
            by_key("band_of_accuria"),
        );
        // Free slots fill in order.
        assert_eq!(wear(&mut hero, a, None), Ok(None));
        assert_eq!(wear(&mut hero, b, None), Ok(None));
        assert_eq!(
            (hero.jewelry.slot(0), hero.jewelry.slot(1)),
            (Some(a), Some(b))
        );
        // A piece worn already cannot go on again, even over another.
        assert_eq!(wear(&mut hero, a, Some(1)), Err(Refused::AlreadyWorn));
        // Full: a slot to replace must be named, and it must be a ring's.
        assert_eq!(wear(&mut hero, c, None), Err(Refused::Full));
        assert_eq!(wear(&mut hero, c, Some(2)), Err(Refused::WrongSlot));
        assert_eq!(wear(&mut hero, c, Some(9)), Err(Refused::WrongSlot));
        assert_eq!(hero.jewelry.worn().count(), 2, "nothing changed");
        assert_eq!(wear(&mut hero, c, Some(1)), Ok(Some(b)));
        assert_eq!(
            (hero.jewelry.slot(0), hero.jewelry.slot(1)),
            (Some(a), Some(c))
        );
        // The other kinds are untouched by the rings, and fill their own slots.
        let neck = by_key("mark_of_fordring");
        assert_eq!(wear(&mut hero, neck, None), Ok(None));
        assert_eq!(hero.jewelry.slot(4), Some(neck));
        let other = by_key("onyxia_tooth_pendant");
        assert_eq!(wear(&mut hero, other, None), Err(Refused::Full));
        assert_eq!(wear(&mut hero, other, Some(4)), Ok(Some(neck)));
        // What it wears is exactly what its bonuses hold.
        let mut want = Bonuses::default();
        for p in [a, c, other] {
            want.add_all(p.bonuses);
        }
        assert_eq!(hero.jewelry.bonuses(), want);
    }

    #[test]
    fn jewelry_bonuses_reach_the_stats() {
        let mut hero = HeroState::new(
            party::LOCAL_PLAYER,
            &data::CLASSES[6],
            7,
            Bonuses::default(),
        );
        let base = hero.stats();
        let full = hero.hp;
        // +1 target, +10% damage, +4% crit.
        wear(&mut hero, by_key("band_of_unnatural_forces"), None).unwrap();
        let s = hero.stats();
        assert_eq!(s.targets, base.targets + 1);
        assert!(close_to(s.might, base.might + 0.10));
        assert!(close_to(s.crit, base.crit + 0.04));
        // An extra life, from a trinket.
        wear(&mut hero, by_key("mark_of_the_champion"), None).unwrap();
        assert_eq!(hero.stats().revives, 1);
        // Max health raised: the hero is healed by the raise.
        wear(&mut hero, by_key("diamond_flask"), None).unwrap();
        let s = hero.stats();
        assert!(close_to(s.max_hp, base.max_hp + 24.0));
        assert!(close_to(s.regen, base.regen + 0.8));
        assert!(close_to(hero.hp, full + 24.0));
        // Replaced, its bonuses leave: health is cut back to the new max.
        let old = wear(&mut hero, by_key("talisman_of_ephemeral_power"), Some(3)).unwrap();
        assert_eq!(old.map(|p| p.key), Some("diamond_flask"));
        let s = hero.stats();
        assert!(close_to(s.max_hp, base.max_hp));
        assert!(close_to(s.regen, base.regen));
        assert!(hero.hp <= s.max_hp + 1e-4);
        assert!(close_to(s.might, base.might + 0.10 + 0.10 + 0.15));
        assert!(close_to(s.haste, base.haste + 0.07));
        // The bonuses hold the worn pieces' and nothing else.
        assert_eq!(hero.bonuses, {
            let mut b = Bonuses::default();
            b.merge(&hero.jewelry.bonuses());
            b
        });
    }

    #[test]
    fn luck_and_level_weigh_rarer_pieces_up() {
        let plain = HeroState::default().stats();
        let lucky = {
            let mut h = HeroState::default();
            h.bonuses.add(&Bonus::new(StatKey::Luck, 0.5));
            h.stats()
        };
        // At the start, unlucky: the commoner the heavier.
        for w in RARITIES.windows(2) {
            assert!(weight(w[0], &plain, 1) > weight(w[1], &plain, 1));
        }
        assert_eq!(weight(Rarity::Uncommon, &plain, 1), 1.0);
        assert_eq!(
            weight(Rarity::Uncommon, &lucky, 30),
            1.0,
            "Uncommon is the yardstick"
        );
        assert_eq!(weight(Rarity::Common, &lucky, 30), 0.0, "no Common jewelry");
        for r in &RARITIES[1..] {
            assert!(weight(*r, &lucky, 1) > weight(*r, &plain, 1), "{r:?}");
            assert!(weight(*r, &plain, 20) > weight(*r, &plain, 1), "{r:?}");
        }
        // Rolled: the share of Epics and Legendaries grows with luck and level.
        let share = |stats: &Stats, level: u32| {
            let mut rng = Rng(0x5EED);
            let worn = Jewelry::default();
            let mut rare = 0;
            for _ in 0..2000 {
                for p in roll(pieces(), &worn, stats, level, &mut rng) {
                    rare += usize::from((p.rarity as u8) >= (Rarity::Epic as u8));
                }
            }
            rare as f32 / (2000 * OFFER_SIZE) as f32
        };
        let early = share(&plain, 1);
        let late = share(&plain, 25);
        let lucky_late = share(&lucky, 25);
        assert!(early < 0.15, "{early}");
        assert!(late > early * 1.5, "{early} -> {late}");
        assert!(lucky_late > late * 1.3, "{late} -> {lucky_late}");
    }

    #[test]
    fn offers_never_repeat_or_hold_what_is_worn() {
        let mut rng = Rng(41);
        let mut hero = HeroState::default();
        for key in [
            "quick_strike_ring",
            "naglering",
            "hand_of_justice",
            "mark_of_fordring",
        ] {
            wear(&mut hero, by_key(key), None).unwrap();
        }
        let stats = hero.stats();
        for n in 0..500 {
            let offer = roll(pieces(), &hero.jewelry, &stats, 1 + n % 30, &mut rng);
            assert_eq!(offer.len(), OFFER_SIZE);
            for (i, p) in offer.iter().enumerate() {
                assert!(!hero.jewelry.wears(p), "{} is worn", p.name);
                assert!(
                    offer[i + 1..].iter().all(|q| q.key != p.key),
                    "{} twice",
                    p.name
                );
            }
        }
        // With only two pieces left to offer, two are offered.
        let tail = &pieces()[..6];
        let worn = Jewelry {
            slots: [Some(&tail[0]), Some(&tail[1]), None, None, None],
            ..Jewelry::default()
        };
        let offer = roll(&pieces()[..4], &worn, &stats, 1, &mut rng);
        assert_eq!(offer.len(), 2);
    }

    #[test]
    fn with_nothing_left_to_offer_the_choice_is_a_level_up_hand() {
        let mut rng = Rng(3);
        let mut hero = HeroState::default();
        let mut offer = JewelOffer::default();
        hero.jewelry.picks = 2;
        // Nothing to offer: both choices become hands, and the picker gives way to them.
        assert!(!next_offer(&mut hero, &mut offer, &[], 1, &mut rng));
        assert!(offer.pieces().is_empty());
        assert_eq!((hero.jewelry.picks, hero.picks), (0, 2));
        assert_eq!(resume(&hero), Phase::LevelUp);
        // With the real catalog an offer is always up, and no hand comes of it.
        let mut hero = HeroState::default();
        hero.jewelry.picks = 1;
        let catalog: Vec<_> = pieces().iter().collect();
        assert!(next_offer(&mut hero, &mut offer, &catalog, 1, &mut rng));
        assert_eq!(offer.pieces().len(), OFFER_SIZE);
        assert_eq!(hero.picks, 0);
        // A hero no one picks for falls back the same way, onto a random card.
        let mut extra = HeroState::new(1, &data::CLASSES[2], 1, Bonuses::default());
        extra.jewelry.picks = 3;
        choose_at_random(&mut extra, &catalog, 5, &mut rng);
        assert_eq!(extra.jewelry.picks, 0);
        // Three pieces taken; a second necklace replaces the first.
        assert!((1..=3).contains(&extra.jewelry.worn().count()));
        // Its numbers are what it wears, whatever was replaced on the way.
        let mut fresh = HeroState::new(1, &data::CLASSES[2], 1, Bonuses::default());
        fresh.bonuses.merge(&extra.jewelry.bonuses());
        let (a, b) = (extra.stats(), fresh.stats());
        assert!(close_to(a.might, b.might) && close_to(a.max_hp, b.max_hp));
        assert!(close_to(a.crit, b.crit) && close_to(a.haste, b.haste));
        assert_eq!((a.targets, a.revives), (b.targets, b.revives));
    }

    #[test]
    fn taking_a_piece_into_full_slots_asks_which_to_replace() {
        let mut rng = Rng(9);
        let mut run = Run::default();
        let mut hero = HeroState::default();
        wear(&mut hero, by_key("naglering"), None).unwrap();
        wear(&mut hero, by_key("blood_ring"), None).unwrap();
        hero.jewelry.picks = 2;
        let mut offer = JewelOffer {
            pool: pool(None),
            pieces: vec![by_key("band_of_accuria"), by_key("arena_master")],
            replacing: None,
        };
        // The ring waits for a slot; gold and other cards wait for it.
        assert_eq!(act(Act::Take(0), &mut hero, &mut run, &mut offer), None);
        assert_eq!(offer.replacing().map(|p| p.key), Some("band_of_accuria"));
        assert_eq!(act(Act::Gold, &mut hero, &mut run, &mut offer), None);
        assert_eq!(act(Act::Take(1), &mut hero, &mut run, &mut offer), None);
        // Back, and in again; a trinket slot will not take a ring.
        assert_eq!(act(Act::Back, &mut hero, &mut run, &mut offer), None);
        assert!(offer.replacing().is_none());
        act(Act::Take(0), &mut hero, &mut run, &mut offer);
        assert_eq!(act(Act::Replace(2), &mut hero, &mut run, &mut offer), None);
        let done = act(Act::Replace(1), &mut hero, &mut run, &mut offer).unwrap();
        assert_eq!(
            done,
            Done::Took(by_key("band_of_accuria"), Some(by_key("blood_ring")))
        );
        assert_eq!(hero.jewelry.slot(1).map(|p| p.key), Some("band_of_accuria"));
        // The choice closes: the next offer is dealt while one is left.
        assert_eq!(close(done, &mut hero, &mut run, &mut offer, &mut rng), None);
        assert_eq!(hero.jewelry.picks, 1);
        assert_eq!(offer.pieces().len(), OFFER_SIZE);
        assert!(run
            .banner
            .as_ref()
            .unwrap()
            .0
            .contains("replaces Blood Ring"));
        // Gold instead: about ten coins' worth, into the purse; then play goes on.
        let pays = gold_instead(&run, &hero);
        assert_eq!(pays, 10);
        let done = act(Act::Gold, &mut hero, &mut run, &mut offer).unwrap();
        assert_eq!((done, run.gold), (Done::Gold(pays), pays));
        assert_eq!(
            close(done, &mut hero, &mut run, &mut offer, &mut rng),
            Some(Phase::Playing)
        );
        assert_eq!(hero.jewelry.picks, 0);
        assert!(offer.pieces().is_empty());
    }

    /// Disenchant takes the offer's rarest piece apart, wears nothing and pays no gold.
    #[test]
    fn disenchant_takes_the_rarest_piece_apart() {
        let mut rng = Rng(17);
        let mut run = Run::default();
        let mut hero = HeroState::default();
        hero.jewelry.picks = 1;
        let pieces: Vec<&'static Piece> = pieces().iter().take(40).collect();
        let rarest = pieces
            .iter()
            .copied()
            .max_by_key(|p| p.rarity as u8)
            .unwrap();
        let mut offer = JewelOffer {
            pool: pool(None),
            pieces: pieces.clone(),
            replacing: None,
        };
        let done = act(Act::Disenchant, &mut hero, &mut run, &mut offer).unwrap();
        assert_eq!(done, Done::Disenchanted(rarest));
        assert_eq!(run.gold, 0);
        assert!(quality(rarest) >= 2, "jewelry is Uncommon or better");
        assert_eq!(
            close(done, &mut hero, &mut run, &mut offer, &mut rng),
            Some(Phase::Playing)
        );
        assert!(run.banner.as_ref().unwrap().0.starts_with("Disenchanted"));
    }

    #[test]
    fn last_choice_from_a_profile_pool_resumes_playing() {
        let mut rng = Rng(17);
        let mut run = Run::default();
        let mut hero = HeroState::default();
        hero.jewelry.picks = 1;
        let mut offer = JewelOffer {
            pool: pool(None),
            pieces: vec![by_key("naglering")],
            replacing: None,
        };
        let done = act(Act::Gold, &mut hero, &mut run, &mut offer).unwrap();
        assert_eq!(
            close(done, &mut hero, &mut run, &mut offer, &mut rng),
            Some(Phase::Playing)
        );
        assert!(offer.pieces().is_empty());
        assert!(offer.replacing().is_none());
    }

    #[test]
    fn an_exhausted_profile_pool_turns_the_choices_left_into_hands() {
        let mut rng = Rng(23);
        let mut run = Run::default();
        let mut hero = HeroState::default();
        let only = by_key("naglering");
        hero.jewelry.picks = 3;
        let mut offer = JewelOffer {
            pool: vec![only],
            pieces: vec![only],
            replacing: None,
        };
        // Taking the pool's only piece leaves it nothing to offer: the two choices left become
        // level-up hands instead of vanishing.
        let done = act(Act::Take(0), &mut hero, &mut run, &mut offer).unwrap();
        assert_eq!(
            close(done, &mut hero, &mut run, &mut offer, &mut rng),
            Some(Phase::LevelUp)
        );
        assert_eq!((hero.jewelry.picks, hero.picks), (0, 2));
        assert!(offer.pieces().is_empty());
    }

    #[test]
    fn banked_choices_are_dealt_to_every_hero() {
        let mut run = Run {
            pending_jewelry: 2,
            ..Run::default()
        };
        let mut heroes: Vec<HeroState> = (0..3)
            .map(|p| HeroState {
                player: p,
                ..HeroState::default()
            })
            .collect();
        deal(&mut run, heroes.iter_mut());
        assert!(heroes.iter().all(|h| h.jewelry.picks == 2));
        assert_eq!(run.pending_jewelry, 0);
    }

    #[test]
    fn a_banked_choice_opens_the_picker_after_the_hands() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<NextState<Phase>>();
        world.insert_resource(Run {
            pending_jewelry: 1,
            pending_levels: 1,
            ..Run::default()
        });
        world.run_system_once(open_jewelry).unwrap();
        assert!(matches!(
            *world.resource::<NextState<Phase>>(),
            NextState::Unchanged
        ));
        world.resource_mut::<Run>().pending_levels = 0;
        world.run_system_once(open_jewelry).unwrap();
        assert!(matches!(
            *world.resource::<NextState<Phase>>(),
            NextState::Pending(Phase::Jewelry)
        ));
        // A transition already queued (the run lost) stands.
        world
            .resource_mut::<NextState<Phase>>()
            .set(Phase::GameOver);
        world.run_system_once(open_jewelry).unwrap();
        assert!(matches!(
            *world.resource::<NextState<Phase>>(),
            NextState::Pending(Phase::GameOver)
        ));
    }

    #[test]
    fn the_picker_pauses_deals_and_resumes() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<Phase>()
            .init_resource::<Time<Virtual>>()
            .init_resource::<Run>()
            .insert_resource(Rng(77))
            .init_resource::<JewelOffer>()
            .add_systems(OnEnter(Phase::Playing), super::super::unpause);
        phases(&mut app);
        let me = app
            .world_mut()
            .spawn((HeroState::default(), LocalHero))
            .id();
        let extra = app
            .world_mut()
            .spawn((
                HeroState {
                    player: 1,
                    ..HeroState::default()
                },
                party::Bot,
            ))
            .id();
        app.world_mut().resource_mut::<Run>().pending_jewelry = 1;
        app.world_mut()
            .resource_mut::<NextState<Phase>>()
            .set(Phase::Jewelry);
        app.update();
        assert_eq!(
            *app.world().resource::<State<Phase>>().get(),
            Phase::Jewelry
        );
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
        assert_eq!(
            app.world().resource::<JewelOffer>().pieces().len(),
            OFFER_SIZE
        );
        assert_eq!(app.world().get::<HeroState>(me).unwrap().jewelry.picks, 1);
        // The bot chose on its own.
        let extra = app.world().get::<HeroState>(extra).unwrap();
        assert_eq!((extra.jewelry.picks, extra.jewelry.worn().count()), (0, 1));
        assert_eq!(app.world().resource::<Run>().pending_jewelry, 0);
        app.world_mut()
            .resource_mut::<NextState<Phase>>()
            .set(Phase::Playing);
        app.update();
        assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    }

    /// The looks are real: every display id is an `ItemDisplayInfo.dbc` row whose inventory icon
    /// is the piece's, and every icon ships as `Interface\Icons\<name>.blp`.
    #[test]
    fn displays_and_icons_exist_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut bad = Vec::new();
        for p in pieces() {
            let path = format!("Interface\\Icons\\{}", p.icon);
            match cat.get(p.display_id) {
                None => bad.push(format!("{}: no display {}", p.name, p.display_id)),
                Some(d)
                    if !d
                        .icon
                        .as_deref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(&path)) =>
                {
                    bad.push(format!(
                        "{}: display {} shows {:?}",
                        p.name, p.display_id, d.icon
                    ))
                }
                Some(_) => {}
            }
            if !chain.contains(&format!("{path}.blp")) {
                bad.push(format!("{}: no {path}.blp", p.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
