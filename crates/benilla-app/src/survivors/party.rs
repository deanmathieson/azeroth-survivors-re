//! The party: every hero in a run carries its own [`HeroState`] (who plays it, its class and
//! build, its health and buffs, its level-up tools), while the [`Run`] resource keeps what the
//! heroes share (the clock, the horde, the XP bar and level, gold, caches and bosses). A
//! single-player run is a party of one: player 0, the [`LocalHero`] this machine's input, camera
//! and HUD follow. `WOW_SURVIVORS_PARTY=<n>`, a test aid, fills the party to n heroes
//! ([`fill_party`]).
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
use super::hud;
use super::{hand, jewelry, mobility, Owned, Rng, Run, Selection, BASE_HP};

/// The player this machine drives: the only one until co-op.
pub(super) const LOCAL_PLAYER: u8 = 0;
/// The most heroes a party holds.
pub(super) const MAX_PARTY: u8 = 4;
/// How far from the local hero the test aid's extras stand, in yards.
const EXTRA_SPACING: f32 = 3.0;

/// The hero this machine's input, camera and HUD follow: player [`LOCAL_PLAYER`]'s.
#[derive(Component)]
pub(super) struct LocalHero;

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
    /// Level-up hands this hero has still to pick from, dealt from the run's banked levels
    /// ([`deal_hands`]).
    pub picks: u32,
    /// The level-up hand's tools left, and the choices banished, for this hero's own hands
    /// ([`hand`]).
    pub rerolls: u32,
    pub banishes: u32,
    pub banished: Vec<hand::Struck>,
    /// How hard the horde wants this hero, against distance ([`pick_target`]): 1 for everyone
    /// until tank stances raise it.
    pub threat: f32,
    /// Damage this hero's spells dealt; [`Run::damage_done`] is the party's total.
    pub damage_done: f64,
    /// Health this hero's own spells, regeneration and leech restored (pickups and the
    /// healthstone are no one's), and the damage that got through to its health.
    pub healing_done: f64,
    pub damage_taken: f64,
    /// The rings, trinkets and necklace this hero found this run, and its jewelry choices left
    /// ([`jewelry`]); what they grant is already in `bonuses`.
    pub(super) jewelry: jewelry::Jewelry,
    /// What legendaries, talents and jewelry have granted this hero ([`super::mechanics`]).
    pub mechanics: super::mechanics::Mechanics,
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
            picks: 0,
            rerolls: hand::REROLLS_PER_RUN,
            banishes: hand::BANISHES_PER_RUN,
            banished: Vec::new(),
            threat: 1.0,
            damage_done: 0.0,
            healing_done: 0.0,
            damage_taken: 0.0,
            jewelry: jewelry::Jewelry::default(),
            mechanics: Default::default(),
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
pub(super) fn deal_hands<H: DerefMut<Target = HeroState>>(
    run: &mut Run,
    heroes: impl IntoIterator<Item = H>,
) {
    for mut hero in heroes {
        hero.picks += run.pending_levels;
    }
    run.pending_levels = 0;
}

/// A hero no one picks for (the test aid's extras) takes a random card from each hand it holds,
/// so the party's hands drain.
pub(super) fn pick_at_random(hero: &mut HeroState, rng: &mut Rng) {
    while hero.picks > 0 {
        let hand = hud::choices_for(hero, rng);
        let choice = hand[rng.index(hand.len())];
        hud::take(hero, choice);
        hero.picks -= 1;
    }
}

/// Who a heal or shield of `caster`'s lands on (a healing pickup's, the hero who took it): for now
/// always the caster itself. The seam where a healer's spells will pick the most injured ally in
/// reach.
pub(super) fn support_target(caster: Entity) -> Entity {
    caster
}

/// `amount` of health for `target`, never past its max health; returns what landed, for the
/// healer's [`HeroState::healing_done`]. An area heal calls it for every hero inside the area.
pub(super) fn heal(target: &mut HeroState, amount: f32) -> f32 {
    let before = target.hp;
    target.hp = (target.hp + amount).min(target.stats().max_hp);
    (target.hp - before).max(0.0)
}

/// A heal over time on `target`, `per_sec` for `secs`, replacing any it had; it ticks in
/// [`hero::hero_vitals`].
pub(super) fn heal_over_time(target: &mut HeroState, per_sec: f32, secs: f32) {
    target.heal_rate = per_sec;
    target.heal_t = secs;
}

/// A shield on `target` soaking `amount` for `secs`, replacing any it had.
pub(super) fn shield(target: &mut HeroState, amount: f32, secs: f32) {
    target.shield = amount;
    target.shield_t = secs;
}

/// The hero an enemy at `at` goes for, of `heroes` (each with its place and threat): the most
/// threat over ground-plane distance. With every threat 1 that is the nearest; tank stances will
/// raise a hero's, and a taunt forces the target outright (`Enemy::taunted`).
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
/// run lacks joins it beside the local hero, in the same class and look. They stand still (no
/// input reaches them yet) and fight, level and pick on their own [`HeroState`]s.
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
    let extras = party.size.saturating_sub(1).max(1);
    for player in 1..party.size {
        if heroes.iter().any(|h| h.player == player) {
            continue;
        }
        let a = f32::from(player - 1) / f32::from(extras) * std::f32::consts::TAU;
        let at = tf.translation + Vec3::new(a.cos(), 0.0, a.sin()) * EXTRA_SPACING;
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
        hero::spawn_hero(&mut commands, state, display, sex, serial, pos);
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
}
