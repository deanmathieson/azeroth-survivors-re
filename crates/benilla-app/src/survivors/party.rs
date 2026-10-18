//! The party: every hero in a run carries its own [`HeroState`] (who plays it, its class and
//! build, its health and buffs, its level-up tools), while the [`Run`] resource keeps what the
//! heroes share (the clock, the horde, the XP bar and level, gold, caches and bosses). A
//! single-player run is a party of one: player 0, the [`LocalHero`] this machine's input, camera
//! and HUD follow. `WOW_SURVIVORS_PARTY=<n>`, a test aid, fills the party to n heroes with
//! [`Bot`]s ([`fill_party`]) that follow the local hero and pick their cards at random; a
//! [`Remote`] hero is another player's, whose picks arrive through [`apply_pick`] / [`apply_skip`].
//!
//! The co-op rules: a hero at 0 health is downed, not dead, while an ally stands; an ally beside it
//! for [`REVIVE_SECS`] ([`HEALER_REVIVE_SECS`] for a healer) gets it up at half health
//! ([`revive_downed`]), and the run is lost only once every hero is down at once ([`all_down`]).
//! Heals land on no downed hero, and a downed hero's level-up hands wait until it is up. The horde
//! scales with the party ([`scale`]) and rings round one standing hero at a time
//! ([`spawn_anchor`], [`spawn_point`]).
//!
//! Two seams wait for healers and tanks: every heal and shield lands through [`heal`] /
//! [`heal_over_time`] / [`shield`] on the hero [`support_target`] picks (the caster itself, for
//! now), and every enemy picks whom to go for through [`pick_target`], by [`HeroState::threat`]
//! over distance (all threat is 1 for now, so the nearest).

use std::ops::DerefMut;

use bevy::prelude::*;

use super::bonus::Bonuses;
use super::data::{self, ClassDef, PassiveDef};
use super::hero::{self, Hero};
use super::hud::{self, Choice, Offer};
use super::units::{self, flat, yaw_toward, Marks};
use super::{hand, jewelry, mobility, Owned, Phase, Rng, Run, Selection, BASE_HP};
use crate::creature_anim::{move_flags, MovementState};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// The player this machine drives: the only one until co-op.
pub(super) const LOCAL_PLAYER: u8 = 0;
/// The most heroes a party holds.
pub(super) const MAX_PARTY: u8 = 5;
/// How far from the local hero the test aid's extras stand, in yards.
const EXTRA_SPACING: f32 = 3.0;

/// The hero this machine's input, camera and HUD follow: player [`LOCAL_PLAYER`]'s.
#[derive(Component)]
pub(super) struct LocalHero;

/// A hero no one plays: the test aid's extras, which only [`fill_party`] makes. It follows the
/// local hero ([`follow_leader`]) and takes its level-up cards and jewelry at random; the
/// level-up never waits for it.
#[derive(Component)]
pub(super) struct Bot;

/// Another player's hero, driven from their own machine: it picks its own cards on its own screen,
/// handed in through [`apply_pick`] / [`apply_skip`], and the level-up waits for it. Nothing sets
/// it outside online co-op ([`super::coop`]).
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
#[derive(Component)]
pub(super) struct Remote;

/// Who simulates the run on this machine: alone or hosting, this machine does; a co-op client only
/// shows the host's world ([`simulates`]).
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum NetRole {
    #[default]
    Solo,
    Host,
    Client,
}

/// Run condition: this machine runs the horde, the damage, the drops and the clock.
pub(super) fn simulates(role: Res<NetRole>) -> bool {
    *role != NetRole::Client
}

/// One choice at the level-up hand, as a co-op client makes it: sent to the host, which applies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HandDecision {
    Pick(usize),
    Reroll,
    Banish(usize),
    Skip,
}

/// Present on a co-op client only: the cards and the hand's tools post the choices here instead of
/// applying them, and the host deals the next hand.
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
#[derive(Resource, Default)]
pub(super) struct HandOutbox(pub Vec<HandDecision>);

/// How many heroes a run fields: one, unless `WOW_SURVIVORS_PARTY` asks for more.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Party {
    /// Heroes in each run, 1 to [`MAX_PARTY`].
    pub size: u8,
}

impl Default for Party {
    fn default() -> Self {
        Self { size: 1 }
    }
}

impl Party {
    /// `WOW_SURVIVORS_PARTY=<n>` (1 to [`MAX_PARTY`]); unset or malformed is a party of one.
    pub(super) fn from_env() -> Self {
        let text = crate::webenv::var("WOW_SURVIVORS_PARTY");
        let size = size_from(text.as_deref());
        if size > 1 {
            info!("survivors: WOW_SURVIVORS_PARTY: a party of {size}");
        }
        Self { size }
    }
}

/// A party size from the env text: a number clamped to 1..=[`MAX_PARTY`], else 1.
fn size_from(text: Option<&str>) -> u8 {
    text.and_then(|t| t.trim().parse::<u8>().ok())
        .map_or(1, |n| n.clamp(1, MAX_PARTY))
}

/// One hero's own share of a run, on its entity. [`Run`] holds what the party shares.
#[derive(Component)]
pub(super) struct HeroState {
    /// Which player this is; [`LOCAL_PLAYER`] is this machine's.
    pub player: u8,
    /// The class and race picked: each player's own, as the hero's look is.
    pub class: &'static ClassDef,
    pub race: u8,
    /// Health: each hero takes its own blows.
    pub hp: f32,
    /// The build each hero picks for itself at level-ups.
    pub abilities: Vec<Owned>,
    pub passives: Vec<(&'static PassiveDef, u8)>,
    /// Uncapped bonuses picked after every ability and passive slot is mastered.
    pub overcharges: [u32; 6],
    /// The permanent bonuses this hero started with: its player's [`super::MetaBonuses`] and the
    /// env test aid.
    pub(super) bonuses: Bonuses,
    /// Extra lives spent so far: each hero spends its own.
    pub revives_used: u32,
    /// The timed buffs the hero's own spells put on it: shield, immunity, thorns, a heal.
    pub shield: f32,
    pub shield_t: f32,
    pub immune_t: f32,
    pub thorns: f32,
    pub thorns_t: f32,
    pub thorns_spell: u32,
    pub heal_rate: f32,
    pub heal_t: f32,
    /// The class mobility's cooldown and running effect.
    pub(super) mobile: mobility::MobilityState,
    /// Seconds since this hero fell, while the death plays out.
    pub dead_t: Option<f32>,
    /// How far a downed hero's revive has got, 0 to 1: an ally beside it fills it
    /// ([`revive_downed`]).
    pub revive_t: f32,
    /// Focused: in co-op, the hero's hand is open, and it takes half damage
    /// ([`super::strip::FOCUS_TAKEN`]).
    pub focused: bool,
    /// Level-up hands this hero has still to pick from, dealt from the run's banked levels
    /// ([`deal_hands`]).
    pub picks: u32,
    /// The level-up hand's tools left, and the choices banished, for this hero's own hands
    /// ([`hand`]).
    pub rerolls: u32,
    pub banishes: u32,
    pub banished: Vec<hand::Struck>,
    /// How hard the horde wants this hero, against distance ([`pick_target`]): 1, or more for a
    /// tank in its stance or form in a party ([`super::synergy`]).
    pub threat: f32,
    /// Damage this hero's spells dealt; [`Run::damage_done`] is the party's total.
    pub damage_done: f64,
    /// Health this hero's own spells, regeneration and leech restored (pickups and the
    /// healthstone are no one's), and the damage that got through to its health.
    pub healing_done: f64,
    pub damage_taken: f64,
    /// Downed allies this hero got up ([`revive_downed`] credits the one who did), the gold it
    /// picked up off the ground (into the party's purse), and how often it went down.
    pub revives_given: u32,
    pub gold_collected: u32,
    pub times_downed: u32,
    /// Gold this hero brought into the party's purse: the coins it picked up and the hands it
    /// skipped (the co-op side bets' "Most gold").
    pub gold: u32,
    /// The rings, trinkets and necklace this hero found this run, and its jewelry choices left
    /// ([`jewelry`]); what they grant is already in `bonuses`.
    pub(super) jewelry: jewelry::Jewelry,
    /// What legendaries, talents and jewelry have granted this hero ([`super::mechanics`]).
    pub mechanics: super::mechanics::Mechanics,
    /// Its part in a party's play ([`super::synergy`]): the group buffs on it, its resurrection,
    /// soulstone and ankh, and its tank stance and taunt.
    pub(super) synergy: super::synergy::HeroSynergy,
    /// Its part in a raid boss's fight ([`super::raid`]): its debuffs, fear, and the forced moves
    /// the host sent it.
    pub(super) raid: super::raid::HeroRaid,
}

impl Default for HeroState {
    fn default() -> Self {
        Self {
            player: LOCAL_PLAYER,
            class: &data::CLASSES[0],
            race: 0,
            hp: BASE_HP,
            abilities: Vec::new(),
            passives: Vec::new(),
            overcharges: [0; 6],
            bonuses: Bonuses::default(),
            revives_used: 0,
            shield: 0.0,
            shield_t: 0.0,
            immune_t: 0.0,
            thorns: 0.0,
            thorns_t: 0.0,
            thorns_spell: 0,
            heal_rate: 0.0,
            heal_t: 0.0,
            mobile: mobility::MobilityState::default(),
            dead_t: None,
            revive_t: 0.0,
            focused: false,
            picks: 0,
            rerolls: hand::REROLLS_PER_RUN,
            banishes: hand::BANISHES_PER_RUN,
            banished: Vec::new(),
            threat: 1.0,
            damage_done: 0.0,
            healing_done: 0.0,
            damage_taken: 0.0,
            revives_given: 0,
            gold_collected: 0,
            times_downed: 0,
            gold: 0,
            jewelry: jewelry::Jewelry::default(),
            mechanics: Default::default(),
            synergy: Default::default(),
            raid: Default::default(),
        }
    }
}

impl HeroState {
    /// A hero as a run begins: its class's first spell, at full health under `bonuses`.
    pub(super) fn new(player: u8, class: &'static ClassDef, race: u8, bonuses: Bonuses) -> Self {
        let mut hero = Self {
            player,
            class,
            race,
            bonuses,
            ..Self::default()
        };
        hero.abilities.push(Owned {
            def: data::ability(class.start),
            level: 1,
            cd: 0.5,
        });
        hero.hp = hero.stats().max_hp;
        hero
    }

    /// Up and fighting: a fallen hero takes no part.
    pub(super) fn alive(&self) -> bool {
        self.dead_t.is_none()
    }
}

/// The run's banked levels become hands: every hero gets one pick for each, and the bank empties.
/// A downed hero's wait in its [`HeroState::picks`] until it is up again ([`open_banked_picks`]).
pub(super) fn deal_hands<H: DerefMut<Target = HeroState>>(
    run: &mut Run,
    heroes: impl IntoIterator<Item = H>,
) {
    for mut hero in heroes {
        hero.picks += run.pending_levels;
    }
    run.pending_levels = 0;
}

/// A hero no one picks for (a [`Bot`]) takes a random card from each hand it holds, so its hands
/// drain.
pub(super) fn pick_at_random(hero: &mut HeroState, rng: &mut Rng) {
    while hero.picks > 0 {
        let hand = hud::choices_for(hero, rng);
        let choice = hand[rng.index(hand.len())];
        hud::take(hero, choice);
        hero.picks -= 1;
    }
}

/// Whether a hero takes its cards at random the moment they are dealt: a [`Bot`] that is up. A
/// downed bot banks them, as anyone does.
pub(super) fn auto_picks(hero: &HeroState, bot: bool) -> bool {
    bot && hero.alive()
}

/// Whether a hero holds the level-up open: a player's (not a [`Bot`]'s), up, with a hand still to
/// pick from. A downed player's hands wait for its revive and hold nothing.
pub(super) fn holds_level_up(hero: &HeroState, bot: bool) -> bool {
    !bot && hero.alive() && hero.picks > 0
}

/// Whether every player standing has picked, of `heroes` (each with whether it is a [`Bot`]): the
/// level-up may close.
pub(super) fn party_picked<'a>(heroes: impl IntoIterator<Item = (&'a HeroState, bool)>) -> bool {
    !heroes
        .into_iter()
        .any(|(hero, bot)| holds_level_up(hero, bot))
}

/// A [`Remote`] player's pick, made on their own screen: `choice` joins `hero`'s build and one of
/// its hands is spent. False, and nothing taken, for a hero downed or with no hand to pick from.
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
pub(super) fn apply_pick(hero: &mut HeroState, choice: Choice) -> bool {
    if !hero.alive() {
        return false;
    }
    // A piece of jewelry spends a jewelry choice; any other card a level-up hand.
    let bank = match choice {
        Choice::Jewel(_) => &mut hero.jewelry.picks,
        _ => &mut hero.picks,
    };
    if *bank == 0 {
        return false;
    }
    *bank -= 1;
    hud::take(hero, choice);
    true
}

/// A [`Remote`] player's Skip: one hand passed up for the gold [`hand::skip_gold`] pays under the
/// skipper's own multiplier, into the party's purse. `None` for a hero downed or with no hand.
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
pub(super) fn apply_skip(hero: &mut HeroState, run: &mut Run, held: &[Choice]) -> Option<u32> {
    if !hero.alive() || held.is_empty() {
        return None;
    }
    let gold = hand::skip_gold(run, hero);
    run.gold += gold;
    hero.gold += gold;
    hero.gold_collected += gold;
    if held.iter().any(|choice| matches!(choice, Choice::Jewel(_))) {
        hero.jewelry.picks -= 1;
    } else {
        hero.picks -= 1;
    }
    Some(gold)
}

/// The next hand a co-op hero holds of its banked ones: a jewelry choice first, rolled from the
/// whole catalog (one with nothing left to offer becomes a level-up hand), then a level-up hand.
/// `None` with none banked, or the hero down.
pub(super) fn next_hand(hero: &mut HeroState, level: u32, rng: &mut Rng) -> Option<Vec<Choice>> {
    if !hero.alive() {
        return None;
    }
    while hero.jewelry.picks > 0 {
        let pool: Vec<_> = jewelry::pieces().iter().collect();
        let pieces = jewelry::roll_pool(&pool, &hero.jewelry, &hero.stats(), level, rng);
        if !pieces.is_empty() {
            return Some(pieces.into_iter().map(Choice::Jewel).collect());
        }
        hero.jewelry.picks -= 1;
        hero.picks += 1;
    }
    (hero.picks > 0).then(|| hud::choices_for(hero, rng))
}

/// The level-up waits for the party: while a player standing still holds a hand, play does not
/// resume when the local hero is done (the local screen shows the wait), and once the last of
/// them has picked, it does. Runs after every picker of the frame.
pub(super) fn wait_for_party(
    coop: Option<Res<super::strip::CoopRun>>,
    phase: Res<State<Phase>>,
    heroes: Query<(&HeroState, Has<Bot>)>,
    offer: Res<Offer>,
    mut next: ResMut<NextState<Phase>>,
) {
    // In co-op nothing waits: every hero's hands stay banked ([`super::strip`]).
    if coop.is_some_and(|c| c.0) {
        if matches!(*phase.get(), Phase::LevelUp | Phase::Jewelry)
            && matches!(*next, NextState::Unchanged)
        {
            next.set(Phase::Playing);
        }
        return;
    }
    if *phase.get() != Phase::LevelUp {
        return;
    }
    let done = party_picked(heroes.iter());
    match *next {
        NextState::Pending(Phase::Playing) | NextState::PendingIfNeq(Phase::Playing) if !done => {
            next.reset();
        }
        NextState::Unchanged if done && offer.len() == 0 => next.set(Phase::Playing),
        _ => {}
    }
}

/// While playing, a hero up again with hands banked while it was down gets them: a [`Bot`] picks
/// at once, and a player's opens the level-up (the local hero's banked jewelry, the jewelry
/// choice), unless a transition is already queued this frame.
pub(super) fn open_banked_picks(
    run: Res<Run>,
    mut heroes: Query<(&mut HeroState, Has<Bot>, Has<LocalHero>)>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
) {
    let mut open = None;
    for (mut hero, bot, local) in &mut heroes {
        if !hero.alive() {
            continue;
        }
        if bot {
            if hero.picks > 0 || hero.jewelry.picks > 0 {
                let pool: Vec<_> = jewelry::pieces().iter().collect();
                jewelry::choose_at_random(&mut hero, &pool, run.level, &mut rng);
                pick_at_random(&mut hero, &mut rng);
            }
        } else if local && hero.jewelry.picks > 0 {
            open = Some(Phase::Jewelry);
        } else if hero.picks > 0 && open.is_none() {
            open = Some(Phase::LevelUp);
        }
    }
    if let Some(phase) = open {
        if matches!(*next, NextState::Unchanged) && run.pending_jewelry == 0 {
            next.set(phase);
        }
    }
}

/// The level-up's subtitle: the local hero's call to choose, or, with its hands done (or it down),
/// the wait for the rest of the party.
pub(super) fn level_up_sub(
    phase: Res<State<Phase>>,
    offer: Res<Offer>,
    mut subs: Query<&mut Text, With<hud::LevelUpSub>>,
) {
    if *phase.get() != Phase::LevelUp {
        return;
    }
    let want = if offer.len() == 0 {
        "Waiting for your party to choose"
    } else {
        "Choose your power"
    };
    for mut t in &mut subs {
        if **t != want {
            **t = want.into();
        }
    }
}

/// Who a heal or shield of `caster`'s lands on (a healing pickup's, the hero who took it): for now
/// always the caster itself. The seam where a healer's spells will pick the most injured ally in
/// reach.
pub(super) fn support_target(caster: Entity) -> Entity {
    caster
}

/// `amount` of health for `target`, never past its max health; returns what landed, for the
/// healer's [`HeroState::healing_done`]. An area heal calls it for every hero inside the area. A
/// downed hero takes none: only a revive gets it up.
pub(super) fn heal(target: &mut HeroState, amount: f32) -> f32 {
    if !target.alive() {
        return 0.0;
    }
    let before = target.hp;
    target.hp = (target.hp + amount).min(target.stats().max_hp);
    (target.hp - before).max(0.0)
}

/// A heal over time on `target`, `per_sec` for `secs`, replacing any it had; it ticks in
/// [`hero::hero_vitals`]. Nothing for a downed hero.
pub(super) fn heal_over_time(target: &mut HeroState, per_sec: f32, secs: f32) {
    if !target.alive() {
        return;
    }
    target.heal_rate = per_sec;
    target.heal_t = secs;
}

/// A shield on `target` soaking `amount` for `secs`, replacing any it had. Nothing for a downed
/// hero.
pub(super) fn shield(target: &mut HeroState, amount: f32, secs: f32) {
    if !target.alive() {
        return;
    }
    target.shield = amount;
    target.shield_t = secs;
}

/// The hero an enemy at `at` goes for, of `heroes` (each with its place and threat): the most
/// threat over ground-plane distance. With every threat 1 that is the nearest; a tank's stance
/// raises its own, and a taunt forces the target outright (`Enemy::taunted`).
pub(super) fn pick_target(
    at: Vec3,
    heroes: impl IntoIterator<Item = (Entity, Vec3, f32)>,
) -> Option<Entity> {
    heroes
        .into_iter()
        .map(|(e, pos, threat)| (e, threat / hero::reach(at, pos).max(1e-3)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e)
}

/// Whether the whole party is down at once, which loses the run; a party of none is not.
pub(super) fn all_down<'a>(heroes: impl IntoIterator<Item = &'a HeroState>) -> bool {
    let mut any = false;
    for hero in heroes {
        if hero.alive() {
            return false;
        }
        any = true;
    }
    any
}

/// How near a downed hero an ally stands to revive it, in yards.
pub(super) const REVIVE_REACH: f32 = 3.0;
/// How long an ally stands beside a downed hero to get it up, in seconds; a healer is quicker.
pub(super) const REVIVE_SECS: f32 = 3.0;
pub(super) const HEALER_REVIVE_SECS: f32 = 1.5;
/// The share of its max health a revived hero gets up with.
pub(super) const REVIVED_HP: f32 = 0.5;
/// The glow over a downed hero while an ally can revive it: Divine Intervention's.
pub(super) const REVIVE_MARK: u32 = 19752;

/// The healing classes, which revive a downed ally twice as fast: Priest, Paladin, Druid, Shaman.
pub(super) fn is_healer(class: &ClassDef) -> bool {
    matches!(class.id, 2 | 5 | 7 | 11)
}

/// How long `reviver` stands beside a downed ally to get it up.
pub(super) fn revive_secs(reviver: &ClassDef) -> f32 {
    if is_healer(reviver) {
        HEALER_REVIVE_SECS
    } else {
        REVIVE_SECS
    }
}

/// One frame of a downed hero's revive, `progress` from 0 to 1: it fills at the quickest of the
/// allies in reach (`revivers`, their classes), and with none in reach it starts over. True once
/// it is full, the progress spent.
pub(super) fn tick_revive<'a>(
    progress: &mut f32,
    dt: f32,
    revivers: impl IntoIterator<Item = &'a ClassDef>,
) -> bool {
    let quickest = revivers.into_iter().map(revive_secs).reduce(f32::min);
    let Some(secs) = quickest else {
        *progress = 0.0;
        return false;
    };
    *progress += dt / secs;
    if *progress >= 1.0 - 1e-4 {
        *progress = 0.0;
        return true;
    }
    false
}

/// A downed hero gets up at [`REVIVED_HP`] of its max health.
pub(super) fn get_up(hero: &mut HeroState) {
    hero.dead_t = None;
    hero.revive_t = 0.0;
    hero.hp = hero.stats().max_hp * REVIVED_HP;
}

/// Down, not dead, while the party has a hero standing: a downed hero glows for its allies, and
/// one standing within [`REVIVE_REACH`] long enough gets it up ([`tick_revive`]). A lone hero has
/// no one to revive it: down is dead, as it always was.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn revive_downed(
    time: Res<Time>,
    mut heroes: Query<
        (
            Entity,
            &Guid,
            &Transform,
            &mut ObjectStore,
            &mut Marks,
            &mut HeroState,
        ),
        With<Hero>,
    >,
    mut fields: MessageWriter<FieldChanged>,
) {
    let dt = time.delta_secs();
    if heroes.iter().count() < 2 {
        return;
    }
    let standing: Vec<(Entity, Vec3, &'static ClassDef)> = heroes
        .iter()
        .filter(|(.., h)| h.alive())
        .map(|(e, _, tf, .., h)| (e, tf.translation, h.class))
        .collect();
    if standing.is_empty() {
        return;
    }
    let mut credited = Vec::new();
    for (entity, guid, tf, mut store, mut marks, mut hero) in &mut heroes {
        if hero.alive() {
            continue;
        }
        let at = tf.translation;
        let revivers: Vec<(Entity, Vec3, &'static ClassDef)> = standing
            .iter()
            .copied()
            .filter(|(_, pos, _)| hero::reach(at, *pos) <= REVIVE_REACH)
            .collect();
        let r = &mut *hero;
        if tick_revive(
            &mut r.revive_t,
            dt,
            revivers.iter().map(|(.., class)| *class),
        ) {
            get_up(r);
            units::raise_unit(&mut store);
            marks.clear();
            credited.extend(credited_reviver(at, &revivers));
        } else {
            marks.set(&[REVIVE_MARK]);
        }
        marks.flush(entity, guid.0, &mut store, &mut fields);
    }
    for e in credited {
        if let Ok((.., mut hero)) = heroes.get_mut(e) {
            hero.revives_given += 1;
        }
    }
}

/// Who a revive at `at` is credited to, of the allies in reach (`revivers`, each with its place
/// and class): the quickest reviver, the nearest of those on a tie.
pub(super) fn credited_reviver(at: Vec3, revivers: &[(Entity, Vec3, &ClassDef)]) -> Option<Entity> {
    revivers
        .iter()
        .min_by(|a, b| {
            revive_secs(a.2)
                .total_cmp(&revive_secs(b.2))
                .then(hero::reach(at, a.1).total_cmp(&hero::reach(at, b.1)))
        })
        .map(|r| r.0)
}

/// What the horde's health and numbers are multiplied by for a party of `heroes` (bots count):
/// x1.0 / 1.6 / 2.2 / 2.8 / 3.4 for one to five.
pub(super) fn scale(heroes: u8) -> f32 {
    match heroes {
        0 | 1 => 1.0,
        2 => 1.6,
        3 => 2.2,
        4 => 2.8,
        _ => 3.4,
    }
}

/// `n` spawns made for a party of `heroes` ([`scale`]); a party of one's is `n`.
pub(super) fn crowd(n: usize, heroes: u8) -> usize {
    if heroes <= 1 {
        return n;
    }
    (n as f32 * scale(heroes)).round() as usize
}

/// The run's party size, kept on [`Run::party`] for the horde's [`scale`].
pub(super) fn count_party(mut run: ResMut<Run>, heroes: Query<(), With<HeroState>>) {
    let n = heroes.iter().count().clamp(1, usize::from(MAX_PARTY)) as u8;
    if run.party != n {
        run.party = n;
    }
}

/// How near any hero a spawn may appear, in yards.
pub(super) const SPAWN_CLEAR: f32 = 25.0;
/// How many turns round its ring a spawn tries for a clear spot.
const SPAWN_TRIES: u32 = 8;

/// The standing hero the next spawn rings round, the heroes taking turns: `turn` counts the
/// spawns so far. `None` with no one standing.
pub(super) fn spawn_anchor(standing: &[Vec3], turn: &mut u32) -> Option<Vec3> {
    if standing.is_empty() {
        return None;
    }
    let at = standing[*turn as usize % standing.len()];
    *turn = turn.wrapping_add(1);
    Some(at)
}

/// The battleground a run is leashed to: its centre and the leash's radius, in yards.
#[derive(Clone, Copy, Debug)]
pub(super) struct Leash {
    pub centre: Vec3,
    pub radius: f32,
}

impl Leash {
    /// `map`'s leash.
    pub(super) fn of(map: &data::MapDef) -> Self {
        Self {
            centre: hero::arena(map),
            // A yard inside the hero's own leash, so nothing spawns on its very edge.
            radius: (map.leash - 1.0).max(0.0),
        }
    }

    /// `p` pulled inside the leash on the ground plane, its height kept.
    pub(super) fn hold(self, p: Vec3) -> Vec3 {
        let off = flat(p - self.centre);
        if off.length() <= self.radius {
            return p;
        }
        let held = self.centre + off.normalize() * self.radius;
        Vec3::new(held.x, p.y, held.z)
    }
}

/// The nearest of `heroes` to `p` on the ground plane, in yards; infinite for none.
fn clearance(p: Vec3, heroes: &[Vec3]) -> f32 {
    heroes
        .iter()
        .map(|&h| hero::reach(p, h))
        .fold(f32::INFINITY, f32::min)
}

/// Where a spawn `r` yards round `anchor` at angle `a` appears, inside `leash`: the first of a few
/// turns on round the ring that lands at least [`SPAWN_CLEAR`] from every one of `heroes`, else the
/// turn furthest from them.
pub(super) fn spawn_point(anchor: Vec3, a: f32, r: f32, heroes: &[Vec3], leash: Leash) -> Vec3 {
    let mut best = (f32::NEG_INFINITY, anchor);
    for i in 0..SPAWN_TRIES {
        let a = a + i as f32 / SPAWN_TRIES as f32 * std::f32::consts::TAU;
        let p = leash.hold(anchor + Vec3::new(a.cos(), 0.0, a.sin()) * r);
        let clear = clearance(p, heroes);
        if clear >= SPAWN_CLEAR {
            return p;
        }
        if clear > best.0 {
            best = (clear, p);
        }
    }
    best.1
}

/// How far a bot lets its spot beside the local hero get before it follows, in yards.
const FOLLOW_SLACK: f32 = 2.0;

/// Where `player`'s bot stands beside a leader at `leader`, of `extras` bots: spread round it,
/// [`EXTRA_SPACING`] out.
fn follow_spot(leader: Vec3, player: u8, extras: u8) -> Vec3 {
    let a = f32::from(player.saturating_sub(1)) / f32::from(extras.max(1)) * std::f32::consts::TAU;
    leader + Vec3::new(a.cos(), 0.0, a.sin()) * EXTRA_SPACING
}

/// The bots follow the local hero loosely: once their spot beside it is more than
/// [`FOLLOW_SLACK`] off they run to it at their own speed; a downed bot lies where it fell.
#[allow(clippy::type_complexity)]
pub(super) fn follow_leader(
    time: Res<Time>,
    party: Res<Party>,
    selection: Res<Selection>,
    collide: Option<benilla_world::collision::WorldCollision>,
    world: benilla_world::world_point::WorldPoint,
    leader: Query<&Transform, (With<LocalHero>, Without<Bot>)>,
    mut bots: Query<
        (
            &mut Transform,
            &mut MovementState,
            &HeroState,
            Option<&mut super::raid::Walk>,
        ),
        With<Bot>,
    >,
) {
    let dt = time.delta_secs();
    let Ok(lead) = leader.single() else { return };
    let extras = party.size.saturating_sub(1);
    for (mut tf, mut mv, hero, walk) in &mut bots {
        let mut speed = 0.0;
        // A raid boss's fear, knockback or mind control moves the bot for it.
        let forced = walk
            .and_then(|mut w| w.step(&hero.raid, dt))
            .filter(|_| hero.alive());
        if let Some(v) = forced {
            speed = v.length();
            if speed > 0.0 {
                tf.translation = super::enemies::ground_on_collision(
                    collide.as_ref(),
                    &world,
                    tf.translation + v * dt,
                    selection.stage_def().floor,
                );
                tf.rotation = Quat::from_rotation_y(yaw_toward(v));
            }
        } else if hero.alive() && dt > 0.0 {
            let to = flat(follow_spot(lead.translation, hero.player, extras) - tf.translation);
            let gap = to.length();
            if gap > FOLLOW_SLACK {
                speed = hero.stats().speed * hero.haste_mult();
                let dir = to / gap;
                let step = dir * (speed * dt).min(gap);
                let next = super::enemies::ground_on_collision(
                    collide.as_ref(),
                    &world,
                    tf.translation + step,
                    selection.stage_def().floor,
                );
                tf.translation = next;
                tf.rotation = Quat::from_rotation_y(yaw_toward(dir));
            }
        }
        let flags = if speed > 0.0 { move_flags::FORWARD } else { 0 };
        if mv.speed != speed || mv.flags != flags {
            mv.speed = speed;
            mv.flags = flags;
        }
    }
}

/// Of `heroes` (a key and a position each), the one nearest `at` on the ground plane, the
/// measure every range in the mode uses ([`hero::reach`]).
pub(super) fn nearest<T>(
    heroes: impl IntoIterator<Item = (T, Vec3)>,
    at: Vec3,
) -> Option<(T, Vec3)> {
    heroes
        .into_iter()
        .map(|(key, pos)| (key, pos, hero::reach(at, pos)))
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(key, pos, _)| (key, pos))
}

/// The middle of `positions`, which the horde gathers round; `None` for none. One hero's is
/// exactly its own position.
pub(super) fn centre(positions: impl IntoIterator<Item = Vec3>) -> Option<Vec3> {
    let (sum, n) = positions
        .into_iter()
        .fold((Vec3::ZERO, 0u32), |(sum, n), p| (sum + p, n + 1));
    (n > 0).then(|| sum / n as f32)
}

/// The test aid's extras: once a run is under way, every player [`Party::size`] asks for and the
/// run lacks joins it beside the local hero as a [`Bot`], in the same class and look. They follow
/// the local hero ([`follow_leader`]) and fight, level and pick on their own [`HeroState`]s.
#[allow(clippy::too_many_arguments)]
pub(super) fn fill_party(
    mut commands: Commands,
    party: Res<Party>,
    selection: Res<Selection>,
    mut run: ResMut<Run>,
    local: Query<(&Transform, &HeroState), With<LocalHero>>,
    heroes: Query<&HeroState, With<Hero>>,
    collide: Option<benilla_world::collision::WorldCollision>,
    world: benilla_world::world_point::WorldPoint,
    mut dresser: super::gear::Dresser,
) {
    let Ok((tf, me)) = local.single() else { return };
    let (Some(class), Some(&look)) = (
        selection.class,
        selection.look.and_then(|i| selection.looks.get(i)),
    ) else {
        return;
    };
    let sex = look.2;
    let extras = party.size.saturating_sub(1);
    for player in 1..party.size {
        if heroes.iter().any(|h| h.player == player) {
            continue;
        }
        let at = follow_spot(tf.translation, player, extras);
        let pos = super::enemies::ground_on_collision(
            collide.as_ref(),
            &world,
            at,
            selection.stage_def().floor,
        );
        let state = HeroState::new(player, me.class, me.race, super::env_bonuses());
        let serial = run.guid();
        // The local hero's dress: the extras share its class and look.
        let display = dresser.display(class, look);
        let bot = hero::spawn_hero(&mut commands, state, display, sex, serial, pos);
        commands.entity(bot).insert(Bot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn down(mut hero: HeroState) -> HeroState {
        hero.dead_t = Some(0.0);
        hero
    }

    #[test]
    fn unset_or_malformed_env_is_a_party_of_one() {
        assert_eq!(size_from(None), 1);
        assert_eq!(Party::default().size, 1);
        assert_eq!(size_from(Some("")), 1);
        assert_eq!(size_from(Some("two")), 1);
        assert_eq!(size_from(Some("0")), 1);
        assert_eq!(size_from(Some(" 3 ")), 3);
        assert_eq!(size_from(Some("9")), MAX_PARTY);
    }

    #[test]
    fn the_nearest_hero_is_the_target() {
        let heroes = [
            ("far", Vec3::new(20.0, 0.0, 0.0)),
            // Height does not count: a hero on the hill above is still the nearest.
            ("near, uphill", Vec3::new(0.0, 30.0, 4.0)),
            ("mid", Vec3::new(-6.0, 0.0, 0.0)),
        ];
        assert_eq!(
            nearest(heroes, Vec3::ZERO).map(|h| h.0),
            Some("near, uphill")
        );
        assert_eq!(
            nearest(heroes, Vec3::new(15.0, 0.0, 0.0)).map(|h| h.0),
            Some("far")
        );
        assert_eq!(nearest::<u8>([], Vec3::ZERO), None);
        // The middle of one hero is the hero.
        let p = Vec3::new(3.25, -1.5, 7.0);
        assert_eq!(centre([p]), Some(p));
        assert_eq!(centre([]), None);
        assert_eq!(
            centre([Vec3::ZERO, Vec3::new(4.0, 0.0, 2.0)]),
            Some(Vec3::new(2.0, 0.0, 1.0))
        );
    }

    #[test]
    fn enemies_go_for_the_nearest_hero_until_threat_says_otherwise() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let heroes = [
            (e(1), Vec3::new(10.0, 0.0, 0.0), 1.0),
            (e(2), Vec3::new(0.0, 20.0, -3.0), 1.0),
            (e(3), Vec3::new(-6.0, 0.0, 0.0), 1.0),
        ];
        assert_eq!(pick_target(Vec3::ZERO, heroes), Some(e(2)));
        assert_eq!(pick_target(Vec3::new(9.0, 0.0, 0.0), heroes), Some(e(1)));
        assert_eq!(pick_target(Vec3::ZERO, []), None);
        // A tank's threat outweighs being a little further off.
        let mut tanked = heroes;
        tanked[0].2 = 4.0;
        assert_eq!(pick_target(Vec3::ZERO, tanked), Some(e(1)));
        // Standing on the enemy is no division by zero.
        assert_eq!(pick_target(heroes[2].1, heroes), Some(e(3)));
    }

    #[test]
    fn heals_and_shields_land_on_the_target_and_count_what_landed() {
        let mut hero = HeroState::default();
        let max = hero.stats().max_hp;
        hero.hp = max - 10.0;
        assert_eq!(heal(&mut hero, 4.0), 4.0);
        assert_eq!(hero.hp, max - 6.0);
        // Overhealing lands only the gap.
        assert_eq!(heal(&mut hero, 50.0), 6.0);
        assert_eq!(hero.hp, max);
        heal_over_time(&mut hero, 5.0, 3.0);
        shield(&mut hero, 40.0, 6.0);
        assert_eq!((hero.heal_rate, hero.heal_t), (5.0, 3.0));
        assert_eq!((hero.shield, hero.shield_t), (40.0, 6.0));
        // Today every heal is the caster's own.
        let me = Entity::from_raw_u32(7).unwrap();
        assert_eq!(support_target(me), me);
    }

    #[test]
    fn one_shared_level_up_queues_one_pick_per_hero() {
        let mut run = Run {
            pending_levels: 1,
            ..Run::default()
        };
        let mut heroes: Vec<HeroState> = (0..3)
            .map(|p| HeroState {
                player: p,
                ..HeroState::default()
            })
            .collect();
        deal_hands(&mut run, heroes.iter_mut());
        assert!(heroes.iter().all(|h| h.picks == 1));
        assert_eq!(run.pending_levels, 0, "the bank is dealt out");
        // A cache and two levels later, everyone holds three more.
        run.pending_levels = 3;
        deal_hands(&mut run, heroes.iter_mut());
        assert!(heroes.iter().all(|h| h.picks == 4));
    }

    #[test]
    fn a_hero_with_no_picker_drains_its_hands_at_random() {
        let mut hero = HeroState::new(1, &data::CLASSES[2], 1, Bonuses::default());
        hero.picks = 3;
        let mut rng = Rng(17);
        pick_at_random(&mut hero, &mut rng);
        assert_eq!(hero.picks, 0);
        let ranks: u32 = hero
            .abilities
            .iter()
            .map(|o| u32::from(o.level))
            .sum::<u32>()
            + hero
                .passives
                .iter()
                .map(|(_, l)| u32::from(*l))
                .sum::<u32>();
        // The start spell's rank, and one rank for each card taken.
        assert_eq!(ranks, 1 + 3);
    }

    #[test]
    fn the_run_is_lost_only_when_every_hero_is_down() {
        let up = HeroState::default;
        assert!(!all_down(&[up()]));
        assert!(all_down(&[down(up())]));
        assert!(!all_down(&[down(up()), up(), down(up())]));
        assert!(all_down(&[down(up()), down(up()), down(up())]));
        assert!(!all_down(&[]), "no party, no loss");
    }

    #[test]
    fn a_fresh_hero_starts_on_its_class_spell_at_full_health() {
        for class in data::CLASSES {
            let hero = HeroState::new(LOCAL_PLAYER, class, 1, Bonuses::default());
            assert_eq!(hero.abilities.len(), 1);
            assert_eq!(hero.abilities[0].def.key, class.start);
            assert_eq!(hero.hp, hero.stats().max_hp);
            assert_eq!((hero.picks, hero.dead_t), (0, None));
        }
    }

    fn class(name: &str) -> &'static ClassDef {
        data::CLASSES.iter().find(|c| c.name == name).unwrap()
    }

    fn pending(world: &World) -> Option<Phase> {
        match world.resource::<NextState<Phase>>() {
            NextState::Pending(p) | NextState::PendingIfNeq(p) => Some(*p),
            NextState::Unchanged => None,
        }
    }

    /// A world the level-up systems run in, one banked level, with a local hero, a remote player
    /// and a bot.
    fn picking_world() -> (World, [Entity; 3]) {
        let mut world = World::new();
        world.insert_resource(Run {
            pending_levels: 1,
            ..Run::default()
        });
        world.insert_resource(Rng(11));
        world.init_resource::<Offer>();
        world.init_resource::<NextState<Phase>>();
        world.insert_resource(State::new(Phase::LevelUp));
        let hero = |player| HeroState::new(player, &data::CLASSES[2], 1, Bonuses::default());
        let me = world.spawn((hero(0), LocalHero)).id();
        let remote = world.spawn((hero(1), Remote)).id();
        let bot = world.spawn((hero(2), Bot)).id();
        (world, [me, remote, bot])
    }

    fn picks(world: &World, e: Entity) -> u32 {
        world.get::<HeroState>(e).unwrap().picks
    }

    #[test]
    fn bots_pick_at_random_and_players_pick_for_themselves() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, [me, remote, bot]) = picking_world();
        world.run_system_once(hud::roll_offer).unwrap();
        assert_eq!(picks(&world, bot), 0, "the bot took its card");
        // The players hold theirs: the local one on screen, the remote one on its own screen.
        assert_eq!((picks(&world, me), picks(&world, remote)), (1, 1));
        assert_eq!(world.resource::<Offer>().len(), 3);
        assert_eq!(pending(&world), None, "the level-up stays open");
        // A downed bot banks its hand, as anyone does.
        let mut hero = HeroState::default();
        assert!(auto_picks(&hero, true));
        assert!(!auto_picks(&hero, false), "a player is never picked for");
        hero.dead_t = Some(0.0);
        assert!(!auto_picks(&hero, true));
    }

    #[test]
    fn the_level_up_waits_for_every_player_standing() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, [me, remote, _]) = picking_world();
        world.run_system_once(hud::roll_offer).unwrap();
        // The local hero picks: its own hand closes and play would resume, but the remote player
        // still holds a hand.
        world
            .run_system_once(
                |mut run: ResMut<Run>,
                 mut local: Query<&mut HeroState, With<LocalHero>>,
                 mut offer: ResMut<Offer>,
                 mut rng: ResMut<Rng>,
                 mut next: ResMut<NextState<Phase>>| {
                    let mut hero = local.single_mut().unwrap();
                    hud::pick(0, &mut hero, &mut run, &mut offer, &mut rng, &mut next);
                },
            )
            .unwrap();
        assert_eq!(picks(&world, me), 0);
        assert_eq!(pending(&world), Some(Phase::Playing));
        world.run_system_once(wait_for_party).unwrap();
        assert_eq!(pending(&world), None, "held for the remote player");
        world.run_system_once(wait_for_party).unwrap();
        assert_eq!(pending(&world), None);
        // Its pick arrives: play resumes.
        let choice = {
            let hero = world.get::<HeroState>(remote).unwrap();
            hud::choices_for(hero, &mut Rng(5))[0]
        };
        let mut hero = world.get_mut::<HeroState>(remote).unwrap();
        assert!(apply_pick(&mut hero, choice));
        assert!(!apply_pick(&mut hero, choice), "no hand left to pick from");
        world.run_system_once(wait_for_party).unwrap();
        assert_eq!(pending(&world), Some(Phase::Playing));
        // A downed player holds nothing: its hands wait for its revive.
        let mut down = HeroState {
            picks: 2,
            ..HeroState::default()
        };
        assert!(holds_level_up(&down, false));
        assert!(!holds_level_up(&down, true), "nor does a bot");
        down.dead_t = Some(0.0);
        assert!(!holds_level_up(&down, false));
        assert!(party_picked([(&down, false)]));
        assert!(apply_skip(&mut down, &mut Run::default(), &[Choice::Heal]).is_none());
    }

    #[test]
    fn a_remote_skip_pays_under_the_skippers_multiplier() {
        let mut run = Run::default();
        let mut plain = HeroState {
            picks: 1,
            ..HeroState::default()
        };
        let mut rich = HeroState {
            picks: 1,
            ..HeroState::default()
        };
        rich.bonuses.add(&super::super::bonus::Bonus::new(
            super::super::bonus::StatKey::Gold,
            1.0,
        ));
        let a = apply_skip(&mut plain, &mut run, &[Choice::Heal]).unwrap();
        let b = apply_skip(&mut rich, &mut run, &[Choice::Heal]).unwrap();
        assert_eq!(b, 2 * a);
        assert_eq!(run.gold, a + b);
        assert_eq!((plain.picks, rich.picks), (0, 0));
    }

    #[test]
    fn a_local_hero_down_at_the_level_up_is_offered_nothing() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, [me, remote, _]) = picking_world();
        world.get_mut::<HeroState>(me).unwrap().dead_t = Some(0.0);
        world.get_mut::<HeroState>(remote).unwrap().dead_t = Some(0.0);
        world.run_system_once(hud::roll_offer).unwrap();
        assert_eq!(world.resource::<Offer>().len(), 0);
        assert_eq!(picks(&world, me), 1, "banked");
        assert_eq!(
            pending(&world),
            Some(Phase::Playing),
            "no one standing to wait for"
        );
    }

    #[test]
    fn heals_and_shields_skip_the_downed() {
        let mut hero = down(HeroState::default());
        hero.hp = 0.0;
        assert_eq!(heal(&mut hero, 50.0), 0.0);
        heal_over_time(&mut hero, 5.0, 3.0);
        shield(&mut hero, 40.0, 6.0);
        assert_eq!((hero.hp, hero.heal_t, hero.shield_t), (0.0, 0.0, 0.0));
        // The healthstone neither.
        hud::take(&mut hero, Choice::Heal);
        assert_eq!(hero.hp, 0.0);
        // Up again, they land.
        get_up(&mut hero);
        let max = hero.stats().max_hp;
        assert_eq!(hero.hp, max * REVIVED_HP);
        assert!(heal(&mut hero, 10.0) > 0.0);
    }

    #[test]
    fn a_healer_revives_twice_as_fast() {
        let (warrior, priest) = (class("Warrior"), class("Priest"));
        for name in ["Priest", "Paladin", "Druid", "Shaman"] {
            assert!(is_healer(class(name)), "{name}");
        }
        assert!(!is_healer(warrior) && !is_healer(class("Mage")));
        let frames = |revivers: &[&'static ClassDef]| {
            let mut progress = 0.0;
            (1..=100)
                .find(|_| tick_revive(&mut progress, 0.1, revivers.iter().copied()))
                .unwrap()
        };
        assert_eq!(frames(&[warrior]), 30, "3 s beside it");
        assert_eq!(frames(&[priest]), 15, "1.5 s for a healer");
        assert_eq!(frames(&[warrior, priest]), 15, "the quickest counts");
        // Stepping away starts it over.
        let mut progress = 0.0;
        assert!(!tick_revive(&mut progress, 2.0, [warrior]));
        assert!(!tick_revive(&mut progress, 0.1, []));
        assert_eq!(progress, 0.0);
        assert!(!tick_revive(&mut progress, 2.0, [warrior]));
    }

    /// A world [`revive_downed`] and [`open_banked_picks`] run in, a second passing each run, with
    /// heroes of these classes at these places.
    fn revive_world(heroes: &[(&str, f32)]) -> (World, Vec<Entity>) {
        let mut world = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(1));
        world.insert_resource(time);
        world.init_resource::<Run>();
        world.insert_resource(Rng(3));
        world.init_resource::<NextState<Phase>>();
        world.init_resource::<Messages<FieldChanged>>();
        let heroes = heroes
            .iter()
            .enumerate()
            .map(|(i, &(name, x))| {
                let state = HeroState::new(i as u8, class(name), 1, Bonuses::default());
                let store = ObjectStore(Default::default());
                let body = (Guid(i as u64 + 1), Transform::from_xyz(x, 0.0, 0.0), store);
                let mut e = world.spawn((Hero::default(), body, Marks::buffs(), state));
                if i == 0 {
                    e.insert(LocalHero);
                }
                e.id()
            })
            .collect();
        (world, heroes)
    }

    #[test]
    fn an_ally_beside_a_downed_hero_gets_it_up_with_its_banked_hands() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, heroes) = revive_world(&[("Warrior", 0.0), ("Mage", 2.0), ("Mage", 40.0)]);
        {
            let mut me = world.get_mut::<HeroState>(heroes[0]).unwrap();
            me.dead_t = Some(0.0);
            me.hp = 0.0;
            // Two levels came while it was down.
            me.picks = 2;
        }
        let me = |world: &World| {
            let h = world.get::<HeroState>(heroes[0]).unwrap();
            (h.dead_t, h.hp, h.stats().max_hp)
        };
        world.run_system_once(revive_downed).unwrap();
        world.run_system_once(revive_downed).unwrap();
        assert!(me(&world).0.is_some(), "still down after 2 s");
        world.run_system_once(open_banked_picks).unwrap();
        assert_eq!(pending(&world), None, "no hand for the downed");
        world.run_system_once(revive_downed).unwrap();
        let (dead, hp, max) = me(&world);
        assert_eq!(dead, None, "up after 3 s");
        assert_eq!(hp, max * REVIVED_HP);
        assert_eq!(world.get::<HeroState>(heroes[0]).unwrap().picks, 2);
        world.run_system_once(open_banked_picks).unwrap();
        assert_eq!(pending(&world), Some(Phase::LevelUp), "its hands, now");
    }

    #[test]
    fn a_revived_bot_takes_its_banked_hands_at_once() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, heroes) = revive_world(&[("Priest", 0.0), ("Mage", 1.0)]);
        world.entity_mut(heroes[1]).insert(Bot);
        world.get_mut::<HeroState>(heroes[1]).unwrap().picks = 2;
        world.run_system_once(open_banked_picks).unwrap();
        assert_eq!(world.get::<HeroState>(heroes[1]).unwrap().picks, 0);
        assert_eq!(pending(&world), None, "a bot opens no level-up");
    }

    #[test]
    fn no_one_revives_a_lone_hero_or_a_wiped_party() {
        use bevy::ecs::system::RunSystemOnce;
        // Alone, down is dead.
        let (mut world, heroes) = revive_world(&[("Priest", 0.0)]);
        world.get_mut::<HeroState>(heroes[0]).unwrap().dead_t = Some(0.0);
        for _ in 0..5 {
            world.run_system_once(revive_downed).unwrap();
        }
        assert!(world.get::<HeroState>(heroes[0]).unwrap().dead_t.is_some());
        // Every hero down: the downed revive no one.
        let (mut world, heroes) = revive_world(&[("Priest", 0.0), ("Priest", 1.0)]);
        for &h in &heroes {
            world.get_mut::<HeroState>(h).unwrap().dead_t = Some(0.0);
        }
        for _ in 0..5 {
            world.run_system_once(revive_downed).unwrap();
        }
        assert!(all_down(
            heroes.iter().map(|&h| world.get::<HeroState>(h).unwrap())
        ));
        // An ally out of reach does not either.
        let (mut world, heroes) = revive_world(&[("Priest", 0.0), ("Priest", 3.5)]);
        world.get_mut::<HeroState>(heroes[0]).unwrap().dead_t = Some(0.0);
        for _ in 0..5 {
            world.run_system_once(revive_downed).unwrap();
        }
        assert!(world.get::<HeroState>(heroes[0]).unwrap().dead_t.is_some());
    }

    #[test]
    fn the_horde_scales_with_the_party() {
        let table: Vec<f32> = (1..=5).map(scale).collect();
        assert_eq!(table, [1.0, 1.6, 2.2, 2.8, 3.4]);
        assert_eq!(scale(0), 1.0);
        for n in [0, 1, 7, 28, 220] {
            assert_eq!(crowd(n, 1), n, "a party of one is unchanged");
        }
        assert_eq!(crowd(100, 2), 160);
        assert_eq!(crowd(100, 3), 220);
        assert_eq!(crowd(100, 4), 280);
        assert_eq!(crowd(100, 5), 340);
    }

    #[test]
    fn spawns_ring_round_each_hero_in_turn_and_clear_of_them_all() {
        let heroes = [
            Vec3::ZERO,
            Vec3::new(30.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 30.0),
        ];
        let mut turn = 0;
        let anchors: Vec<Vec3> = (0..6)
            .map(|_| spawn_anchor(&heroes, &mut turn).unwrap())
            .collect();
        assert_eq!(anchors[..3], heroes);
        assert_eq!(anchors[3..], heroes);
        assert_eq!(spawn_anchor(&[], &mut turn), None);
        let wide = Leash {
            centre: Vec3::ZERO,
            radius: 500.0,
        };
        for &anchor in &heroes {
            for i in 0..24 {
                let a = i as f32 / 24.0 * std::f32::consts::TAU;
                for r in [26.0, 30.0, 34.0, 42.0] {
                    let p = spawn_point(anchor, a, r, &heroes, wide);
                    for h in heroes {
                        assert!(
                            hero::reach(p, h) >= SPAWN_CLEAR,
                            "{p} is {} from {h}",
                            hero::reach(p, h)
                        );
                    }
                }
            }
        }
        // A lone hero's ring is where it always was.
        let a = 0.7;
        let p = spawn_point(Vec3::ZERO, a, 30.0, &[Vec3::ZERO], wide);
        assert!(p.distance(Vec3::new(a.cos(), 0.0, a.sin()) * 30.0) < 1e-4);
        // Inside the leash, whatever the ring asks.
        let small = Leash {
            centre: Vec3::ZERO,
            radius: 40.0,
        };
        let hero = Vec3::new(-30.0, 0.0, 0.0);
        for i in 0..24 {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            let p = spawn_point(hero, a, 34.0, &[hero], small);
            assert!(flat(p).length() <= 40.0 + 1e-3, "{p} past the leash");
            assert!(hero::reach(p, hero) >= SPAWN_CLEAR, "{p} on the hero");
        }
    }

    #[test]
    fn a_revive_is_credited_to_the_quickest_reviver_then_the_nearest() {
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let (warrior, priest) = (class("Warrior"), class("Priest"));
        let at = Vec3::ZERO;
        // A healer revives quicker, so it gets the credit though it stands further off.
        let revivers = [
            (e(1), Vec3::new(1.0, 0.0, 0.0), warrior),
            (e(2), Vec3::new(2.5, 0.0, 0.0), priest),
        ];
        assert_eq!(credited_reviver(at, &revivers), Some(e(2)));
        // Two of a kind: the nearer.
        let revivers = [
            (e(1), Vec3::new(2.0, 0.0, 0.0), warrior),
            (e(3), Vec3::new(0.0, 0.0, 1.0), warrior),
        ];
        assert_eq!(credited_reviver(at, &revivers), Some(e(3)));
        assert_eq!(credited_reviver(at, &[]), None);
    }

    #[test]
    fn the_reviver_is_counted_as_the_downed_hero_gets_up() {
        use bevy::ecs::system::RunSystemOnce;
        let (mut world, heroes) = revive_world(&[("Warrior", 0.0), ("Priest", 1.0), ("Mage", 2.0)]);
        world.get_mut::<HeroState>(heroes[0]).unwrap().dead_t = Some(0.0);
        // The priest is in reach and quickest: 1.5 s.
        world.run_system_once(revive_downed).unwrap();
        world.run_system_once(revive_downed).unwrap();
        let given =
            |world: &World, i: usize| world.get::<HeroState>(heroes[i]).unwrap().revives_given;
        assert!(world.get::<HeroState>(heroes[0]).unwrap().alive());
        assert_eq!((given(&world, 1), given(&world, 2)), (1, 0));
    }

    #[test]
    fn no_run_begins_with_no_hero() {
        let mut selection = Selection {
            class: Some(0),
            looks: vec![(0, 1, 0)],
            look: Some(0),
            ..Default::default()
        };
        let mut run = Run {
            kills: 3,
            ..Run::default()
        };
        let meta = super::super::MetaBonuses::default();
        assert!(!hud::start_run(&mut run, None, &mut selection, &meta));
        assert_eq!((run.kills, run.version), (3, 0), "nothing started over");
        let mut hero = HeroState::default();
        assert!(hud::start_run(
            &mut run,
            Some(&mut hero),
            &mut selection,
            &meta
        ));
        assert_eq!(run.kills, 0);
    }
}
