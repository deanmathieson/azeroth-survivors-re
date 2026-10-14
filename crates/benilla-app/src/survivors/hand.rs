//! The level-up hand's tools, under the cards: Reroll deals a new hand, Skip passes it up for gold,
//! and Banish strikes one choice from the offers for the rest of the run and deals its slot again.
//! Rerolls and banishes are counted per hero and run; R and B are their keys, and an armed Banish
//! takes the next card clicked (or 1, 2, 3) instead of picking it. The hand on screen is the local
//! hero's.

use bevy::prelude::*;

use super::cards::PendingPick;
use super::hud::{self, Choice, Offer};
use super::{treasure, HeroState, LocalHero, Phase, Rng, Run};

/// Rerolls and banishes a run starts with.
pub(super) const REROLLS_PER_RUN: u32 = 2;
pub(super) const BANISHES_PER_RUN: u32 = 1;

/// The armed Banish's colour, on its button and on the cards' glow.
pub(super) const BANISH_RED: Color = Color::srgb(0.9, 0.15, 0.1);
const FACE: Color = Color::srgba(0.18, 0.06, 0.04, 0.95);
const FACE_HOT: Color = Color::srgba(0.36, 0.12, 0.06, 0.98);
const FACE_ARMED: Color = Color::srgba(0.45, 0.04, 0.02, 0.98);
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const DIM: Color = Color::srgb(0.5, 0.5, 0.5);

/// A choice struck from the offers: the spell or the talent, whichever rank it would bring.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Struck {
    Ability(&'static str),
    Passive(&'static str),
}

/// What banishing `choice` (from `hero`'s hand) strikes; the healthstone, offered when nothing else
/// is, cannot be.
pub(super) fn struck(choice: Choice, hero: &HeroState) -> Option<Struck> {
    match choice {
        Choice::NewAbility(def) => Some(Struck::Ability(def.key)),
        Choice::Upgrade(k) => hero.abilities.get(k).map(|o| Struck::Ability(o.def.key)),
        Choice::NewPassive(p) => Some(Struck::Passive(p.key)),
        Choice::UpPassive(k) => hero.passives.get(k).map(|(p, _)| Struck::Passive(p.key)),
        Choice::Heal => None,
    }
}

pub(super) fn is_banished(choice: Choice, hero: &HeroState) -> bool {
    struck(choice, hero).is_some_and(|k| hero.banished.contains(&k))
}

/// What passing up a hand pays, in coins.
const SKIP_COINS: u32 = 10;

/// The gold for passing up a hand: [`SKIP_COINS`] coins at what a coin is worth now
/// ([`treasure::coin_value`]) under the hero's gold multiplier, rounded to the nearest coin and
/// never less than one. The button's label and the payout both read it.
pub(super) fn skip_gold(run: &Run, hero: &HeroState) -> u32 {
    treasure::worth(SKIP_COINS, run.elapsed, hero.stats().gold, 0.5)
}

/// The Skip button's label, with what it pays.
fn skip_label(run: &Run, hero: &HeroState) -> String {
    format!("Skip: +{} gold", skip_gold(run, hero))
}

/// Deal a whole new hand, if a reroll is left.
pub(super) fn reroll(hero: &mut HeroState, offer: &mut Offer, rng: &mut Rng) -> bool {
    if hero.rerolls == 0 {
        return false;
    }
    hero.rerolls -= 1;
    offer.deal(hud::choices_for(hero, rng));
    true
}

/// Whether the card in `slot` can be banished now.
pub(super) fn strikable(slot: usize, hero: &HeroState, offer: &Offer) -> bool {
    hero.banishes > 0
        && offer
            .choices()
            .get(slot)
            .is_some_and(|&c| struck(c, hero).is_some())
}

fn can_banish(hero: &HeroState, offer: &Offer) -> bool {
    (0..offer.len()).any(|i| strikable(i, hero, offer))
}

/// Strike the card in `slot` for the run and deal that slot again from what is left, never a card
/// already in the hand.
pub(super) fn banish(slot: usize, hero: &mut HeroState, offer: &mut Offer, rng: &mut Rng) -> bool {
    if !strikable(slot, hero, offer) {
        return false;
    }
    let Some(key) = struck(offer.choices()[slot], hero) else {
        return false;
    };
    hero.banishes -= 1;
    hero.banished.push(key);
    let held: Vec<Struck> = offer
        .choices()
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != slot)
        .filter_map(|(_, &c)| struck(c, hero))
        .collect();
    let mut pool = hud::offer_pool(hero);
    pool.retain(|&(c, _)| struck(c, hero).is_none_or(|k| !held.contains(&k)));
    offer.redeal(slot, hud::draw(pool, 1, rng).pop());
    true
}

/// Take no card: a purse of gold for the party instead, and the hand closes as a pick closes it.
pub(super) fn skip(
    run: &mut Run,
    hero: &mut HeroState,
    offer: &mut Offer,
    rng: &mut Rng,
    next: &mut NextState<Phase>,
) -> u32 {
    let gold = skip_gold(run, hero);
    run.gold += gold;
    run.banner = Some((format!("Skipped: +{gold} gold"), 2.5));
    hud::close_hand(hero, offer, rng, next);
    gold
}

/// Banish waits for a card once armed.
#[derive(Resource, Default)]
pub(super) struct HandMode {
    /// The next card picked is struck instead of taken.
    pub banishing: bool,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tool {
    Reroll,
    Skip,
    Banish,
}

#[derive(Component)]
pub(super) struct ToolLabel(Tool);

/// The line under the tools that asks for a card while Banish is armed.
#[derive(Component)]
pub(super) struct ToolHint;

/// The tool buttons in a row, and the banish prompt under them.
pub(super) fn spawn_tools(root: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    root.spawn(Node {
        column_gap: Val::Px(14.0),
        ..default()
    })
    .with_children(|row| {
        for tool in [Tool::Reroll, Tool::Skip, Tool::Banish] {
            row.spawn((
                tool,
                Button,
                Node {
                    min_width: Val::Px(170.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(7.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(FACE),
                BorderColor::all(EDGE),
            ))
            .with_children(|b| {
                b.spawn((
                    ToolLabel(tool),
                    hud::centered_bundle(font, "", 16.0, Color::WHITE),
                ));
            });
        }
    });
    root.spawn((ToolHint, hud::centered_bundle(font, "", 15.0, BANISH_RED)));
}

/// R, B and the tool buttons; nothing while a picked card is bursting.
#[allow(clippy::too_many_arguments)]
pub(super) fn use_tools(
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    presses: Query<(&Interaction, &Tool), Changed<Interaction>>,
    pending: Res<PendingPick>,
    mut mode: ResMut<HandMode>,
    mut run: ResMut<Run>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut offer: ResMut<Offer>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
) {
    if *phase.get() != Phase::LevelUp {
        if mode.banishing {
            mode.banishing = false;
        }
        return;
    }
    if pending.busy() {
        return;
    }
    let Ok(mut me) = local.single_mut() else {
        return;
    };
    let mut used = presses
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, &t)| t);
    if keys.just_pressed(KeyCode::KeyR) {
        used = Some(Tool::Reroll);
    }
    if keys.just_pressed(KeyCode::KeyB) {
        used = Some(Tool::Banish);
    }
    // Checked first: touching the offer mutably re-deals the cards.
    match used {
        Some(Tool::Reroll) if me.rerolls > 0 => {
            mode.banishing = false;
            reroll(&mut me, &mut offer, &mut rng);
        }
        Some(Tool::Skip) => {
            mode.banishing = false;
            skip(&mut run, &mut me, &mut offer, &mut rng, &mut next);
        }
        Some(Tool::Banish) => {
            let arm = !mode.banishing && can_banish(&me, &offer);
            mode.banishing = arm;
        }
        _ => {}
    }
}

/// The buttons' labels with what is left, their faces lit under the cursor or while armed, and
/// the banish prompt.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_tools(
    phase: Res<State<Phase>>,
    run: Res<Run>,
    local: Query<&HeroState, With<LocalHero>>,
    offer: Res<Offer>,
    mode: Res<HandMode>,
    mut buttons: Query<(&Tool, &Interaction, &mut BackgroundColor, &mut BorderColor)>,
    mut labels: Query<(&ToolLabel, &mut Text, &mut TextColor)>,
    mut hints: Query<&mut Text, (With<ToolHint>, Without<ToolLabel>)>,
) {
    if *phase.get() != Phase::LevelUp {
        return;
    }
    let Ok(me) = local.single() else { return };
    let ready = |tool: Tool| match tool {
        Tool::Reroll => me.rerolls > 0,
        Tool::Skip => true,
        Tool::Banish => mode.banishing || can_banish(me, &offer),
    };
    for (&tool, interaction, mut bg, mut edge) in &mut buttons {
        let armed = tool == Tool::Banish && mode.banishing;
        let face = if armed {
            FACE_ARMED
        } else if ready(tool) && *interaction != Interaction::None {
            FACE_HOT
        } else {
            FACE
        };
        let rim = if armed { BANISH_RED } else { EDGE };
        if bg.0 != face {
            bg.0 = face;
        }
        if edge.top != rim {
            *edge = BorderColor::all(rim);
        }
    }
    for (label, mut text, mut color) in &mut labels {
        let want = match label.0 {
            Tool::Reroll => format!("Reroll (R): {} left", me.rerolls),
            Tool::Skip => skip_label(&run, me),
            Tool::Banish if mode.banishing => "Cancel banish (B)".to_string(),
            Tool::Banish => format!("Banish (B): {} left", me.banishes),
        };
        if text.0 != want {
            text.0 = want;
        }
        let c = if ready(label.0) { Color::WHITE } else { DIM };
        if color.0 != c {
            color.0 = c;
        }
    }
    let hint = if mode.banishing {
        "Banish which card for the rest of the run? Click it, or press 1, 2 or 3"
    } else {
        ""
    };
    for mut t in &mut hints {
        if t.0 != hint {
            t.0 = hint.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::bonus::{Bonus, StatKey};
    use super::super::{data, Owned};
    use super::*;

    fn fresh_hero() -> HeroState {
        let mut hero = HeroState::default();
        let start = data::ability(hero.class.start);
        hero.abilities.push(Owned {
            def: start,
            level: 1,
            cd: 0.0,
        });
        hero.picks = 1;
        hero
    }

    fn keys(offer: &Offer, hero: &HeroState) -> Vec<Option<Struck>> {
        offer.choices().iter().map(|&c| struck(c, hero)).collect()
    }

    #[test]
    fn rerolls_count_down_and_stop_at_zero() {
        let (mut hero, mut offer, mut rng) = (fresh_hero(), Offer::default(), Rng(7));
        offer.deal(hud::choices_for(&hero, &mut rng));
        assert_eq!(hero.rerolls, REROLLS_PER_RUN);
        for left in (0..REROLLS_PER_RUN).rev() {
            assert!(reroll(&mut hero, &mut offer, &mut rng));
            assert_eq!(hero.rerolls, left);
            assert_eq!(offer.len(), 3);
        }
        let before = keys(&offer, &hero);
        assert!(!reroll(&mut hero, &mut offer, &mut rng));
        assert_eq!(hero.rerolls, 0);
        assert_eq!(keys(&offer, &hero), before, "no reroll left, no new hand");
    }

    #[test]
    fn banished_choices_are_never_offered_again() {
        let (mut hero, mut offer, mut rng) = (fresh_hero(), Offer::default(), Rng(11));
        let mut run = Run::default();
        offer.deal(hud::choices_for(&hero, &mut rng));
        let gone = struck(offer.choices()[1], &hero).expect("a strikable card");
        assert!(banish(1, &mut hero, &mut offer, &mut rng));
        assert_eq!(hero.banishes, BANISHES_PER_RUN - 1);
        assert_eq!(hero.banished, vec![gone]);
        assert_eq!(offer.len(), 3, "the slot is dealt again");
        assert_eq!(offer.redealt(), Some(1));
        let hand = keys(&offer, &hero);
        assert!(!hand.contains(&Some(gone)));
        for (i, k) in hand.iter().enumerate() {
            assert!(!hand[i + 1..].contains(k), "no card twice: {hand:?}");
        }
        // Spent: the next banish does nothing.
        let before = keys(&offer, &hero);
        assert!(!banish(0, &mut hero, &mut offer, &mut rng));
        assert_eq!(hero.banishes, 0);
        assert_eq!(keys(&offer, &hero), before);
        // Nor does any later hand, whatever the build grows into.
        for n in 0..400 {
            let hand = hud::choices_for(&hero, &mut rng);
            assert!(hand.iter().all(|&c| !is_banished(c, &hero)), "deal {n}");
            if n % 40 == 0 {
                offer.deal(hand);
                let mut next = NextState::default();
                hud::pick(0, &mut hero, &mut run, &mut offer, &mut rng, &mut next);
            }
        }
    }

    #[test]
    fn banishing_an_owned_spell_ends_its_ranks() {
        let (mut hero, mut offer, mut rng) = (fresh_hero(), Offer::default(), Rng(3));
        hero.banishes = 1;
        offer.deal(vec![Choice::Upgrade(0), Choice::Heal]);
        assert!(!strikable(1, &hero, &offer), "the healthstone stays");
        assert!(banish(0, &mut hero, &mut offer, &mut rng));
        let start = hero.abilities[0].def.key;
        assert_eq!(hero.banished, vec![Struck::Ability(start)]);
        assert!(hud::offer_pool(&hero)
            .iter()
            .all(|&(c, _)| !matches!(c, Choice::Upgrade(0))));
    }

    #[test]
    fn skipping_pays_and_closes_the_hand() {
        let (mut hero, mut offer, mut rng) = (fresh_hero(), Offer::default(), Rng(5));
        let mut run = Run {
            level: 4,
            ..Run::default()
        };
        hero.picks = 2;
        offer.deal(hud::choices_for(&hero, &mut rng));
        let mut next = NextState::default();
        let label = skip_label(&run, &hero);
        let paid = skip(&mut run, &mut hero, &mut offer, &mut rng, &mut next);
        assert_eq!(paid, SKIP_COINS);
        assert_eq!(label, format!("Skip: +{paid} gold"), "it pays what it says");
        assert_eq!((run.gold, hero.picks), (paid, 1));
        assert_ne!(offer.len(), 0, "another banked level deals another hand");
        assert!(matches!(next, NextState::Unchanged));
        skip(&mut run, &mut hero, &mut offer, &mut rng, &mut next);
        assert_eq!((run.gold, hero.picks), (2 * paid, 0));
        assert_eq!(offer.len(), 0);
        assert!(matches!(next, NextState::Pending(Phase::Playing)));
        assert_eq!(hero.abilities.len(), 1, "no card was taken");
    }

    #[test]
    fn skip_gold_follows_the_coin_economy() {
        let pays = |run: &mut Run, hero: &mut HeroState| {
            let label = skip_label(run, hero);
            let (mut offer, mut rng) = (Offer::default(), Rng(9));
            let mut next = NextState::default();
            run.gold = 0;
            hero.picks = 1;
            let paid = skip(run, hero, &mut offer, &mut rng, &mut next);
            assert_eq!(paid, skip_gold(run, hero));
            assert_eq!(run.gold, paid);
            assert_eq!(label, format!("Skip: +{paid} gold"));
            paid
        };
        let (mut run, mut hero) = (Run::default(), fresh_hero());
        // The hero's level has no say: only the clock and the gold multiplier.
        run.level = 174;
        assert_eq!(pays(&mut run, &mut hero), SKIP_COINS);
        run.elapsed = data::VICTORY_SECS / 2.0;
        assert_eq!(pays(&mut run, &mut hero), 2 * SKIP_COINS);
        run.elapsed = data::VICTORY_SECS;
        assert_eq!(pays(&mut run, &mut hero), 3 * SKIP_COINS);
        let mut last = 0;
        for s in 0..=900 {
            run.elapsed = s as f32;
            let g = skip_gold(&run, &hero);
            assert!(g >= last, "a skip never pays less later");
            last = g;
        }
        // A Gold bonus multiplies it as it does every coin.
        hero.bonuses.add(&Bonus::new(StatKey::Gold, 0.5));
        assert_eq!(pays(&mut run, &mut hero), 45);
        run.elapsed = 0.0;
        assert_eq!(pays(&mut run, &mut hero), 15);
        // Never less than one, whatever the multiplier.
        hero.bonuses.add(&Bonus::new(StatKey::Gold, -3.0));
        assert_eq!(pays(&mut run, &mut hero), 1);
    }
}
