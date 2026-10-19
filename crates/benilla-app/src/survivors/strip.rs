//! Co-op level-ups: with two or more players in a run (this machine's hero and remote ones; bots do
//! not count), a level-up never pauses the world. Each hero banks its hands (and jewelry choices)
//! instead ([`bank_level_ups`]), the HUD shows "Level up! xN (Tab)", and Tab opens the hand as a
//! compact strip at the bottom of the screen while the hero keeps moving and casting
//! ([`HandStrip`]). A hero with its strip open is Focused: it takes half damage, under Divine
//! Protection's glow ([`FOCUS_MARK`]). Should every living player have a strip open at once, the
//! world runs at [`SLOW_MO`] speed until one closes. A boss's death makes camp
//! ([`super::campfire`]), with no spawns while it lasts and the full-size hand. Esc's menu and the
//! Character panel pause nothing either ([`world_runs`]).
//!
//! A run of one player (with or without the test aid's bots) banks its hands the same way under
//! the Quick picks setting, on by default ([`quick_picks`]); with it off, it keeps the paused
//! level-up exactly as it was. A hero holding [`AUTO_BUILD_AT`] or more banked hands is offered
//! Auto-build, which takes them one after another by weight, favouring its own spells
//! ([`party::auto_pick`]), through the same pick a click makes.

use bevy::prelude::*;

use super::cards::{PendingPick, Rays};
use super::hud::{self, HudFont, Offer};
use super::party::{self, Bot, LocalHero, NetRole};
use super::profile::Profile;
use super::{jewelry, HeroState, Phase, Rng, Run};
use crate::cvars::Cvars;

/// The key that opens and hides the hand strip; the toy bar has 7 to 9, F the mobility spell.
pub(super) const STRIP_KEY: KeyCode = KeyCode::Tab;
/// The hand's Skip key (R rerolls, B banishes, 1 to 3 pick).
pub(super) const SKIP_KEY: KeyCode = KeyCode::KeyX;
/// Divine Protection: the glow over a Focused hero.
pub(super) const FOCUS_MARK: u32 = 498;
/// What a Focused hero takes of each blow.
pub(super) const FOCUS_TAKEN: f32 = 0.5;
/// The world's speed while every living player is choosing.
pub(super) const SLOW_MO: f32 = 0.4;
/// The banner while the world is slowed.
const CHOOSING: &str = "Everyone's choosing";
/// The strip's scale against the full-screen level-up, and the room under it to the screen's
/// bottom, in canvas pixels.
const STRIP_SCALE: f32 = 0.6;
const STRIP_BOTTOM: f32 = 118.0;

/// The solo setting that banks level-ups for the strip instead of pausing (Options → Interface).
pub(super) const QUICK_PICKS: &str = "survivorsQuickPicks";
/// Banked hands from which the strip offers Auto-build.
pub(super) const AUTO_BUILD_AT: u32 = 5;

/// Whether Quick picks is on: the setting's value, or its default (on) where it is not set, or
/// with no console variables at all.
pub(super) fn quick_picks(cvars: Option<&Cvars>) -> bool {
    cvars
        .and_then(|c| c.get(QUICK_PICKS))
        .is_none_or(|value| value.trim() != "0")
}

/// Whether hands bank for the strip: always in co-op, and solo under Quick picks.
fn banks(coop: &CoopRun, cvars: Option<&Res<Cvars>>) -> bool {
    coop.0 || quick_picks(cvars.map(|c| &**c))
}

/// Whether this run is a co-op one: two or more players' heroes in it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CoopRun(pub bool);

/// The local player's hand strip, open or hidden.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct HandStrip {
    pub open: bool,
}

/// The "Level up! xN (Tab)" badge over the action bar.
#[derive(Component)]
struct Badge;

/// Auto-build, running: the local hero's banked hands are taken one by one until none is left.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AutoBuild {
    pub on: bool,
}

/// The Auto-build button by the strip.
#[derive(Component)]
struct AutoBuildButton;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<CoopRun>()
        .init_resource::<HandStrip>()
        .init_resource::<AutoBuild>()
        .add_systems(PreUpdate, detect)
        .add_systems(
            Update,
            (
                toggle_strip,
                focus,
                (deal_local, press_auto_build, auto_build)
                    .chain()
                    .before(super::cards::deal_cards),
                slow_mo,
                (spawn_badge, spawn_auto_button),
                (refresh_badge, refresh_auto_button, style_strip)
                    .after(hud::show_panels)
                    .before(crate::glue::GlueVisuals),
            ),
        )
        .add_systems(
            PostUpdate,
            bank_level_ups
                .before(party::wait_for_party)
                .run_if(party::simulates),
        );
}

/// Players among `heroes` (each with whether it is a [`Bot`]): co-op is two or more.
pub(super) fn players<'a>(heroes: impl IntoIterator<Item = (&'a HeroState, bool)>) -> usize {
    heroes.into_iter().filter(|(_, bot)| !bot).count()
}

fn detect(mut coop: ResMut<CoopRun>, heroes: Query<(&HeroState, Has<Bot>)>) {
    coop.set_if_neq(CoopRun(players(heroes.iter()) >= 2));
}

/// Run condition: the world runs. Always while playing; in co-op also under Esc's menu and the
/// Character panel, which pause nothing there.
pub(super) fn world_runs(phase: Res<State<Phase>>, coop: Res<CoopRun>) -> bool {
    match phase.get() {
        Phase::Playing => true,
        Phase::Paused | Phase::Character => coop.0,
        _ => false,
    }
}

/// Whether the hand's cards and tools take input: the paused level-up's, or the open strip's.
pub(super) fn hand_up(phase: Phase, strip: Option<&HandStrip>) -> bool {
    phase == Phase::LevelUp || (phase == Phase::Playing && strip.is_some_and(|s| s.open))
}

/// Hands and jewelry choices a hero holds banked.
pub(super) fn banked(hero: &HeroState) -> u32 {
    hero.picks + hero.jewelry.picks
}

/// The hands a player joining a run under way starts with, to catch up: one per level the party
/// has made.
pub(super) fn catch_up(level: u32) -> u32 {
    level.saturating_sub(1)
}

/// In co-op, a level-up (or a jewelry choice) opens nothing: the run's banked levels and jewelry
/// become every hero's banked hands, bots take theirs at once, and play goes on.
pub(super) fn bank_level_ups(
    coop: Res<CoopRun>,
    cvars: Option<Res<Cvars>>,
    profile: Option<Res<Profile>>,
    mut next: ResMut<NextState<Phase>>,
    mut run: ResMut<Run>,
    mut heroes: Query<(&mut HeroState, Has<Bot>)>,
    mut rng: ResMut<Rng>,
) {
    if !banks(&coop, cvars.as_ref()) {
        return;
    }
    if matches!(
        *next,
        NextState::Pending(Phase::LevelUp | Phase::Jewelry)
            | NextState::PendingIfNeq(Phase::LevelUp | Phase::Jewelry)
    ) {
        next.reset();
    }
    if run.pending_levels == 0 && run.pending_jewelry == 0 {
        return;
    }
    party::deal_hands(&mut run, heroes.iter_mut().map(|(h, _)| h));
    jewelry::deal(&mut run, heroes.iter_mut().map(|(h, _)| h));
    let level = run.level;
    // The bots are this machine's, as the paused picker's are ([`jewelry`]): this profile's pool.
    let pool = jewelry::pool(profile.as_deref());
    for (mut hero, bot) in &mut heroes {
        if party::auto_picks(&hero, bot) {
            jewelry::choose_at_random(&mut hero, &pool, level, &mut rng);
            party::pick_at_random(&mut hero, &mut rng);
        }
    }
}

/// Tab opens and hides the strip while the local hero is up with a hand banked; with none left, or
/// off the run, it shuts.
fn toggle_strip(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    coop: Res<CoopRun>,
    cvars: Option<Res<Cvars>>,
    offer: Res<Offer>,
    local: Query<&HeroState, With<LocalHero>>,
    run: Res<Run>,
    mut strip: ResMut<HandStrip>,
) {
    let holds = local
        .single()
        .is_ok_and(|me| me.alive() && (banked(me) > 0 || offer.len() > 0));
    if !(banks(&coop, cvars.as_ref()) && *phase.get() == Phase::Playing && holds) {
        if strip.open {
            strip.open = false;
        }
        return;
    }
    // A camp is deliberately a full, safe level-up hand rather than the combat strip.
    if run.breather > 0.0 {
        strip.open = true;
        return;
    }
    if keys.just_pressed(STRIP_KEY) {
        strip.open = !strip.open;
    }
}

/// The local hero is Focused while its strip is open.
fn focus(
    strip: Res<HandStrip>,
    coop: Res<CoopRun>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
) {
    // Solo quick picks use the strip but do not gain co-op's damage reduction.
    let want = coop.0 && strip.open;
    if let Ok(mut me) = local.single_mut() {
        if me.focused != want {
            me.focused = want;
        }
    }
}

/// The local hero's next banked hand, dealt here (a co-op client's comes from its host) while
/// playing, the camp's full-size hand too; a hero gone down puts its hand away. Jewelry rolls from
/// this player's own pool, its completed decks' trinkets with the catalog.
#[allow(clippy::too_many_arguments)]
fn deal_local(
    coop: Res<CoopRun>,
    cvars: Option<Res<Cvars>>,
    profile: Option<Res<Profile>>,
    phase: Res<State<Phase>>,
    role: Res<NetRole>,
    pending: Res<PendingPick>,
    run: Res<Run>,
    mut offer: ResMut<Offer>,
    mut rng: ResMut<Rng>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
) {
    if !banks(&coop, cvars.as_ref())
        || *phase.get() != Phase::Playing
        || *role == NetRole::Client
        || pending.busy()
    {
        return;
    }
    let Ok(mut me) = local.single_mut() else {
        return;
    };
    if !me.alive() {
        if offer.len() > 0 {
            offer.deal(Vec::new());
        }
        return;
    }
    if offer.len() == 0 {
        let pool = jewelry::pool(profile.as_deref());
        if let Some(hand) = party::next_hand(&mut me, &pool, run.level, &mut rng) {
            offer.deal(hand);
        }
    }
}

/// The Auto-build button pressed: the local hero's banked hands start resolving.
fn press_auto_build(
    presses: Query<&Interaction, (Changed<Interaction>, With<AutoBuildButton>)>,
    mut auto: ResMut<AutoBuild>,
) {
    if presses.iter().any(|i| *i == Interaction::Pressed) && !auto.on {
        auto.on = true;
    }
}

/// Auto-build at work: each hand dealt to the local hero is taken at once, by weight favouring
/// its own spells ([`party::auto_pick`]), the way a click takes it: applied here, or posted to the
/// host by a co-op client, which deals the next. It stops with nothing left banked, off the run,
/// or with the hero down.
#[allow(clippy::too_many_arguments)]
fn auto_build(
    phase: Res<State<Phase>>,
    pending: Res<PendingPick>,
    mut auto: ResMut<AutoBuild>,
    mut run: ResMut<Run>,
    mut offer: ResMut<Offer>,
    mut rng: ResMut<Rng>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut outbox: Option<ResMut<party::HandOutbox>>,
) {
    if !auto.on {
        return;
    }
    let Ok(mut me) = local.single_mut() else {
        auto.on = false;
        return;
    };
    if *phase.get() != Phase::Playing || !me.alive() || (banked(&me) == 0 && offer.len() == 0) {
        auto.on = false;
        return;
    }
    if pending.busy() || offer.len() == 0 {
        return;
    }
    let Some(i) = party::auto_pick(offer.choices(), &me, &mut rng) else {
        return;
    };
    match outbox.as_deref_mut() {
        Some(outbox) => outbox.0.push(party::HandDecision::Pick(i)),
        None => {
            if let Some(&choice) = offer.choices().get(i) {
                if party::apply_pick(&mut me, choice) {
                    run.version += 1;
                }
            }
        }
    }
    offer.deal(Vec::new());
}

/// Whether every living player has a hand open: the world slows. A party with none standing is
/// not choosing.
pub(super) fn all_choosing<'a>(heroes: impl IntoIterator<Item = (&'a HeroState, bool)>) -> bool {
    let mut any = false;
    for (hero, bot) in heroes {
        if bot || !hero.alive() {
            continue;
        }
        if !hero.focused {
            return false;
        }
        any = true;
    }
    any
}

/// The world at [`SLOW_MO`] while every living player chooses, with the banner; a co-op client
/// follows its host's. A transformation's hitstop holds it at [`super::moments::HITSTOP_SPEED`].
pub(super) fn slow_mo(
    role: Res<NetRole>,
    coop: Res<CoopRun>,
    heroes: Query<(&HeroState, Has<Bot>)>,
    hitstop: Option<Res<super::moments::Hitstop>>,
    mut run: ResMut<Run>,
    mut time: ResMut<Time<Virtual>>,
) {
    let slow = if *role == NetRole::Client {
        run.slow
    } else {
        let slow = coop.0 && run.breather <= 0.0 && all_choosing(heroes.iter());
        if run.slow != slow {
            run.slow = slow;
        }
        if slow && run.banner.as_ref().is_none_or(|b| b.0 == CHOOSING) {
            run.banner = Some((CHOOSING.into(), 1.0));
        }
        slow
    };
    let speed = if hitstop.is_some_and(|h| h.left > 0.0) {
        super::moments::HITSTOP_SPEED
    } else if slow {
        SLOW_MO
    } else {
        1.0
    };
    if time.relative_speed() != speed {
        time.set_relative_speed(speed);
    }
}

/// Whether the horde may grow: not while the party camps.
pub(super) fn may_spawn(run: &Run) -> bool {
    run.breather <= 0.0
}

fn spawn_badge(mut commands: Commands, font: Option<Res<HudFont>>, badges: Query<(), With<Badge>>) {
    let Some(font) = font else { return };
    if !badges.is_empty() {
        return;
    }
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                bottom: Val::Px(STRIP_BOTTOM + 18.0),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(415),
            bevy::ui::FocusPolicy::Pass,
            Badge,
        ))
        .with_children(|b| {
            b.spawn(hud::centered_bundle(&font.0, "", 18.0, hud::GOLD_TONE));
        });
}

/// The badge: shown while the local hero has hands banked and its strip hidden.
fn refresh_badge(
    phase: Res<State<Phase>>,
    coop: Res<CoopRun>,
    cvars: Option<Res<Cvars>>,
    strip: Res<HandStrip>,
    local: Query<&HeroState, With<LocalHero>>,
    mut badges: Query<(&mut Node, &Children), With<Badge>>,
    mut texts: Query<&mut Text>,
) {
    let n = local
        .single()
        .ok()
        .filter(|me| me.alive())
        .map_or(0, banked);
    let on = banks(&coop, cvars.as_ref()) && *phase.get() == Phase::Playing && !strip.open && n > 0;
    for (mut node, children) in &mut badges {
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        let want = format!("Level up! \u{00d7}{n} (Tab)");
        for &child in children {
            if let Ok(mut t) = texts.get_mut(child) {
                if t.0 != want {
                    t.0 = want.clone();
                }
            }
        }
    }
}

/// The Auto-build button's width.
const AUTO_BUTTON_W: f32 = 200.0;

fn spawn_auto_button(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    buttons: Query<(), With<AutoBuildButton>>,
) {
    let Some(font) = font else { return };
    if !buttons.is_empty() {
        return;
    }
    commands
        .spawn((
            AutoBuildButton,
            Button,
            // Centred above the strip's badge: the bottom-right corner is the stock bags'.
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                width: Val::Px(AUTO_BUTTON_W),
                margin: UiRect::left(Val::Px(-AUTO_BUTTON_W / 2.0)),
                justify_content: JustifyContent::Center,
                bottom: Val::Px(STRIP_BOTTOM + 52.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(7.0)),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.09, 0.03, 0.92)),
            BorderColor::all(hud::GOLD_TONE),
            GlobalZIndex(416),
        ))
        .with_children(|b| {
            b.spawn(hud::centered_bundle(&font.0, "", 16.0, hud::GOLD_TONE));
        });
}

/// The Auto-build button: shown while the local hero holds [`AUTO_BUILD_AT`] or more banked
/// hands in a run that banks them, and while it works through them.
fn refresh_auto_button(
    phase: Res<State<Phase>>,
    coop: Res<CoopRun>,
    cvars: Option<Res<Cvars>>,
    auto: Res<AutoBuild>,
    local: Query<&HeroState, With<LocalHero>>,
    mut buttons: Query<(&mut Node, &Children), With<AutoBuildButton>>,
    mut texts: Query<&mut Text>,
) {
    let n = local
        .single()
        .ok()
        .filter(|me| me.alive())
        .map_or(0, banked);
    let on = banks(&coop, cvars.as_ref())
        && *phase.get() == Phase::Playing
        && (auto.on || n >= AUTO_BUILD_AT);
    let want = if auto.on {
        format!("Auto-building\u{2026} {n} left")
    } else {
        format!("Auto-build {n} hands")
    };
    for (mut node, children) in &mut buttons {
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
        for &child in children {
            if let Ok(mut t) = texts.get_mut(child) {
                if t.0 != want {
                    t.0 = want.clone();
                }
            }
        }
    }
}

/// The level-up screen as the strip: scaled down to the screen's bottom, with no vignette, burst,
/// title or hint, and the hand's keys under the cards.
#[allow(clippy::type_complexity)]
fn style_strip(
    phase: Res<State<Phase>>,
    strip: Res<HandStrip>,
    run: Res<Run>,
    mut roots: Query<
        (&mut Node, &mut BackgroundGradient, &mut UiTransform),
        With<hud::LevelUpRoot>,
    >,
    mut parts: Query<
        &mut Visibility,
        (
            Or<(With<Rays>, With<hud::LevelUpTitle>, With<hud::LevelUpHint>)>,
            Without<hud::LevelUpRoot>,
        ),
    >,
    mut subs: Query<&mut Text, With<hud::LevelUpSub>>,
    mut vignette: Local<Option<BackgroundGradient>>,
) {
    let compact = *phase.get() == Phase::Playing && strip.open && run.breather <= 0.0;
    for (mut node, mut bg, mut tf) in &mut roots {
        let (justify, pad, scale, lift) = if compact {
            (
                JustifyContent::FlexEnd,
                STRIP_BOTTOM / STRIP_SCALE,
                STRIP_SCALE,
                // Scaled about the screen's centre, the strip's bottom edge comes back down by
                // this share of the screen: (1 - scale) / 2.
                Val::Percent((1.0 - STRIP_SCALE) * 50.0),
            )
        } else {
            (JustifyContent::Center, 0.0, 1.0, Val::ZERO)
        };
        if node.justify_content != justify {
            node.justify_content = justify;
            node.padding.bottom = Val::Px(pad);
        }
        if tf.scale != Vec2::splat(scale) {
            tf.scale = Vec2::splat(scale);
            tf.translation = Val2::new(Val::ZERO, lift);
        }
        match (compact, bg.0.is_empty()) {
            (true, false) => *vignette = Some(std::mem::take(&mut *bg)),
            (false, true) => {
                if let Some(saved) = vignette.take() {
                    *bg = saved;
                }
            }
            _ => {}
        }
    }
    let shown = if compact {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut parts {
        if *v != shown {
            *v = shown;
        }
    }
    if compact {
        let want = "1, 2, 3 pick   R reroll   B banish   X skip   Tab hide";
        for mut t in &mut subs {
            if **t != want {
                **t = want.into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::Bonuses;
    use super::super::data;
    use super::super::hud::Choice;
    use super::*;

    fn hero(player: u8) -> HeroState {
        HeroState::new(player, &data::CLASSES[0], 1, Bonuses::default())
    }

    fn world(coop: bool) -> World {
        let mut world = World::new();
        world.insert_resource(CoopRun(coop));
        world.init_resource::<Run>();
        world.insert_resource(Rng(5));
        world.init_resource::<NextState<Phase>>();
        world.insert_resource(State::new(Phase::Playing));
        world.init_resource::<Time<Virtual>>();
        world.insert_resource(NetRole::Solo);
        world
    }

    #[test]
    fn two_players_bank_their_level_ups_and_play_goes_on() {
        let mut w = world(true);
        let a = w.spawn(hero(0)).id();
        let b = w.spawn((hero(1), party::Remote)).id();
        let bot = w.spawn((hero(2), Bot)).id();
        {
            let mut run = w.resource_mut::<Run>();
            run.pending_levels = 2;
            run.pending_jewelry = 1;
        }
        w.resource_mut::<NextState<Phase>>().set(Phase::LevelUp);
        w.run_system_once(bank_level_ups).unwrap();
        assert!(matches!(
            *w.resource::<NextState<Phase>>(),
            NextState::Unchanged
        ));
        for e in [a, b] {
            let h = w.get::<HeroState>(e).unwrap();
            assert_eq!((h.picks, h.jewelry.picks), (2, 1), "banked");
            assert_eq!(banked(h), 3);
        }
        let bot = w.get::<HeroState>(bot).unwrap();
        assert_eq!(
            (bot.picks, bot.jewelry.picks),
            (0, 0),
            "a bot picks at once"
        );
        let run = w.resource::<Run>();
        assert_eq!((run.pending_levels, run.pending_jewelry), (0, 0));
        // Esc's menu pauses nothing in co-op.
        w.run_system_once(super::super::pause).unwrap();
        assert!(!w.resource::<Time<Virtual>>().is_paused());
    }

    #[test]
    fn a_lone_player_keeps_the_paused_level_up_with_quick_picks_off() {
        let mut w = world(false);
        let mut cvars = Cvars::default();
        let _ = cvars.set(QUICK_PICKS, "0");
        w.insert_resource(cvars);
        w.spawn(hero(0));
        w.spawn((hero(1), Bot));
        w.run_system_once(detect).unwrap();
        assert_eq!(
            *w.resource::<CoopRun>(),
            CoopRun(false),
            "bots are no players"
        );
        w.resource_mut::<Run>().pending_levels = 1;
        w.resource_mut::<NextState<Phase>>().set(Phase::LevelUp);
        w.run_system_once(bank_level_ups).unwrap();
        assert!(matches!(
            *w.resource::<NextState<Phase>>(),
            NextState::Pending(Phase::LevelUp)
        ));
        assert_eq!(w.resource::<Run>().pending_levels, 1, "dealt by the picker");
        w.run_system_once(super::super::pause).unwrap();
        assert!(w.resource::<Time<Virtual>>().is_paused());
        // Two players make it co-op.
        w.spawn((hero(2), party::Remote));
        w.run_system_once(detect).unwrap();
        assert_eq!(*w.resource::<CoopRun>(), CoopRun(true));
    }

    #[test]
    fn quick_picks_is_on_by_default_and_banks_a_lone_players_level_ups() {
        assert!(quick_picks(None), "no console variables: the default");
        let mut cvars = Cvars::default();
        assert!(quick_picks(Some(&cvars)), "the registered default");
        let _ = cvars.set(QUICK_PICKS, "0");
        assert!(!quick_picks(Some(&cvars)));
        // Solo, with no console variables in the world, the level-up banks and play goes on.
        let mut w = world(false);
        let me = w.spawn((hero(0), LocalHero)).id();
        w.resource_mut::<Run>().pending_levels = 2;
        w.resource_mut::<NextState<Phase>>().set(Phase::LevelUp);
        w.run_system_once(bank_level_ups).unwrap();
        assert!(matches!(
            *w.resource::<NextState<Phase>>(),
            NextState::Unchanged
        ));
        assert_eq!(w.get::<HeroState>(me).unwrap().picks, 2);
    }

    fn strip_world(phase: Phase) -> World {
        let mut w = world(false);
        w.insert_resource(State::new(phase));
        w.init_resource::<PendingPick>();
        w.init_resource::<Offer>();
        w.init_resource::<AutoBuild>();
        w.init_resource::<HandStrip>();
        w.init_resource::<ButtonInput<KeyCode>>();
        w
    }

    #[test]
    fn hands_are_dealt_only_while_playing_and_at_camp() {
        let mut w = strip_world(Phase::GameOver);
        let me = w.spawn((hero(0), LocalHero)).id();
        w.get_mut::<HeroState>(me).unwrap().picks = 2;
        // The run is over (its hand cleared): a banked pick deals nothing into the results.
        w.run_system_once(deal_local).unwrap();
        assert_eq!(
            w.resource::<Offer>().len(),
            0,
            "a stale hand outlives its run"
        );
        // At camp the strip is the full-size hand, and it deals.
        w.insert_resource(State::new(Phase::Playing));
        w.resource_mut::<Run>().breather = 20.0;
        w.run_system_once(deal_local).unwrap();
        assert_eq!(w.resource::<Offer>().len(), 3, "camp spends banked picks");
        w.run_system_once(toggle_strip).unwrap();
        assert!(w.resource::<HandStrip>().open);
    }

    #[test]
    fn auto_build_resolves_every_banked_hand_favouring_owned_spells() {
        let mut w = strip_world(Phase::Playing);
        let me = w.spawn((hero(0), LocalHero)).id();
        w.get_mut::<HeroState>(me).unwrap().picks = 8;
        w.resource_mut::<AutoBuild>().on = true;
        for _ in 0..40 {
            w.run_system_once(deal_local).unwrap();
            w.run_system_once(auto_build).unwrap();
        }
        let h = w.get::<HeroState>(me).unwrap();
        assert_eq!(banked(h), 0, "every hand taken");
        assert_eq!(w.resource::<Offer>().len(), 0);
        assert!(!w.resource::<AutoBuild>().on, "it stops once done");
        let ranks: u32 = h.abilities.iter().map(|o| u32::from(o.level)).sum::<u32>()
            + h.passives.iter().map(|(_, l)| u32::from(*l)).sum::<u32>()
            + h.overcharges.iter().sum::<u32>()
            + h.abilities.len() as u32;
        assert!(ranks > 8, "eight picks grew the build: {ranks}");
    }

    #[test]
    fn a_late_joiner_catches_up_and_banked_jewelry_deals_first() {
        assert_eq!(catch_up(1), 0);
        assert_eq!(catch_up(7), 6);
        let mut me = hero(0);
        me.picks = 1;
        me.jewelry.picks = 1;
        let mut rng = Rng(9);
        let pool = jewelry::pool(None);
        let hand = party::next_hand(&mut me, &pool, 3, &mut rng).unwrap();
        assert!(hand.iter().all(|c| matches!(c, Choice::Jewel(_))));
        assert!(party::apply_pick(&mut me, hand[0]));
        assert_eq!(
            (me.picks, me.jewelry.picks),
            (1, 0),
            "a piece spends the jewelry"
        );
        assert_eq!(me.jewelry.worn().count(), 1);
        let hand = party::next_hand(&mut me, &pool, 3, &mut rng).unwrap();
        assert!(!hand.iter().any(|c| matches!(c, Choice::Jewel(_))));
        assert!(party::apply_pick(&mut me, hand[0]));
        assert_eq!(banked(&me), 0);
        assert!(party::next_hand(&mut me, &pool, 3, &mut rng).is_none());
        // Down, nothing is dealt.
        me.picks = 2;
        me.dead_t = Some(0.0);
        assert!(party::next_hand(&mut me, &pool, 3, &mut rng).is_none());
    }

    #[test]
    fn the_world_slows_only_while_every_living_player_chooses() {
        let mut w = world(true);
        let a = w.spawn(hero(0)).id();
        let b = w.spawn((hero(1), party::Remote)).id();
        w.spawn((hero(2), Bot));
        let speed = |w: &mut World| {
            w.run_system_once(slow_mo).unwrap();
            w.resource::<Time<Virtual>>().relative_speed()
        };
        w.get_mut::<HeroState>(a).unwrap().focused = true;
        assert_eq!(speed(&mut w), 1.0, "one of two choosing");
        w.get_mut::<HeroState>(b).unwrap().focused = true;
        assert_eq!(speed(&mut w), SLOW_MO, "both: the bot does not count");
        assert!(w.resource::<Run>().slow);
        assert_eq!(
            w.resource::<Run>().banner.as_ref().map(|b| b.0.as_str()),
            Some(CHOOSING)
        );
        // A downed player is not waited for; one closing its hand ends it.
        w.get_mut::<HeroState>(b).unwrap().dead_t = Some(0.0);
        assert_eq!(speed(&mut w), SLOW_MO);
        w.get_mut::<HeroState>(a).unwrap().focused = false;
        assert_eq!(speed(&mut w), 1.0);
        // Never solo.
        let mut w = world(false);
        let a = w.spawn(hero(0)).id();
        w.get_mut::<HeroState>(a).unwrap().focused = true;
        assert_eq!(speed(&mut w), 1.0);
    }

    #[test]
    fn focus_halves_the_blows() {
        let mut plain_hero = hero(0);
        let mut me = hero(0);
        me.focused = true;
        let plain = super::super::hero::hurt_hero(&mut plain_hero, 40.0);
        let focused = super::super::hero::hurt_hero(&mut me, 40.0);
        assert!(focused > 0.0);
        assert!(
            (focused - plain * FOCUS_TAKEN).abs() < 1e-4,
            "{focused} vs {plain}"
        );
    }
}
