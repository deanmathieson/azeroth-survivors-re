//! The level-up picker, as a survivors game deals it: three tall cards that flip up out of the
//! deck one after another, tilt toward the cursor, lift under it and burst on the pick. Each card
//! wears its rarity in WoW's item-quality colours, so what is better reads at a glance: a fresh
//! spell by its power among the class's. A rank-up rolls its quality as it is dealt
//! ([`roll_rarity`]): Common for one rank, Rare for two, Epic for three, the rarer ones likelier
//! with luck; a hand holding a Rare or Epic roll lands with a quality glow and a chime. The top
//! rank is Legendary: it transforms the spell ([`data::LEGENDARIES`]), and its card burns
//! gold-orange and says into what.

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::sound::SoundCue;

use super::data::{self, Kind};
use super::hand::{self, HandMode};
use super::hud::{self, Choice, HudFont, Offer, SpellIcon};
use super::{party, HeroState, LocalHero, Phase, Rng, Run, Stats};

/// WoW's item-quality colours (`ITEM_QUALITY_COLORS`).
pub(super) const COMMON: Color = Color::srgb(1.0, 1.0, 1.0);
pub(super) const UNCOMMON: Color = Color::srgb(0.118, 1.0, 0.0);
pub(super) const RARE: Color = Color::srgb(0.0, 0.439, 0.867);
pub(super) const EPIC: Color = Color::srgb(0.639, 0.208, 0.933);
pub(super) const LEGENDARY: Color = Color::srgb(1.0, 0.502, 0.0);

const CARD_W: f32 = 236.0;
const CARD_H: f32 = 330.0;
/// Seconds between one card landing and the next.
const DEAL_STAGGER: f32 = 0.09;
const DEAL_SECS: f32 = 0.38;
/// A hand chained straight onto the last (stacked level-ups) deals this quickly.
const CHAIN_DEAL_SECS: f32 = 0.2;
/// A hand dealt within this many real seconds of the last one closing is chained onto it.
const CHAIN_GAP: f32 = 0.6;

/// How long a fresh hand's cards take to land: quicker for one chained onto the hand before it
/// (dealt over it, or within [`CHAIN_GAP`] of its closing), the 2nd and later of a stack.
fn deal_secs(had_hand: bool, closed_at: Option<f32>, now: f32) -> f32 {
    let chained = had_hand || closed_at.is_some_and(|t| now - t < CHAIN_GAP);
    if chained {
        CHAIN_DEAL_SECS
    } else {
        DEAL_SECS
    }
}

/// The chimes a hand plays as it lands holding a rolled Rare or Epic, as the Darkmoon Faire's
/// reveal rings its Rare and Epic cards: the quest-accepted and quest-complete kits.
pub(super) const ROLL_RARE_SOUND: &str = "QUESTADDED";
pub(super) const ROLL_EPIC_SOUND: &str = "QUESTCOMPLETED";

/// A rank-up card's qualities and their base weights: one card in four rolls Rare or better.
const ROLLS: [(Rarity, f32); 3] = [
    (Rarity::Common, 75.0),
    (Rarity::Rare, 20.0),
    (Rarity::Epic, 5.0),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    /// Ranks granted by a card of this quality.
    pub(super) const fn ranks(self) -> u8 {
        match self {
            Self::Common | Self::Uncommon => 1,
            Self::Rare => 2,
            Self::Epic | Self::Legendary => 3,
        }
    }

    pub(super) fn color(self) -> Color {
        match self {
            Self::Common => COMMON,
            Self::Uncommon => UNCOMMON,
            Self::Rare => RARE,
            Self::Epic => EPIC,
            Self::Legendary => LEGENDARY,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }

    /// Its place on the wire.
    #[cfg_attr(not(feature = "coop"), allow(dead_code))]
    pub(super) const fn index(self) -> u8 {
        self as u8
    }

    #[cfg_attr(not(feature = "coop"), allow(dead_code))]
    pub(super) const fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Self::Common),
            1 => Some(Self::Uncommon),
            2 => Some(Self::Rare),
            3 => Some(Self::Epic),
            4 => Some(Self::Legendary),
            _ => None,
        }
    }

    /// Whether this is `other`'s quality or better.
    pub(super) const fn at_least(self, other: Self) -> bool {
        self as u8 >= other as u8
    }
}

/// A rank-up card's quality, rolled as it is dealt: Common (one rank) most often, Rare (two) and
/// Epic (three) rarer, each step up weighted again by `stats`' luck ([`Stats::luck_weight`]).
pub(super) fn roll_rarity(stats: &Stats, rng: &mut Rng) -> Rarity {
    let weighed = ROLLS.map(|(r, w)| (r, w * stats.luck_weight(roll_tier(r))));
    let total: f32 = weighed.iter().map(|(_, w)| w).sum();
    let mut pick = rng.f32() * total;
    for (r, w) in weighed {
        if pick < w {
            return r;
        }
        pick -= w;
    }
    Rarity::Epic
}

/// How many steps above Common a rolled quality stands, for the luck weight.
const fn roll_tier(r: Rarity) -> i32 {
    match r {
        Rarity::Common | Rarity::Uncommon => 0,
        Rarity::Rare => 1,
        Rarity::Epic | Rarity::Legendary => 2,
    }
}

/// The quality a rank-up card rolled, as `hero` would take it: at least Rare while it holds an
/// Empower token ([`HeroState::empower`]).
fn rolled(rolled: Rarity, hero: &HeroState) -> Rarity {
    if hero.empower > 0 && !rolled.at_least(Rarity::Rare) {
        Rarity::Rare
    } else {
        rolled
    }
}

/// The quality a dealt card rolled, Rare or better: what makes a hand land with a jolt. A new spell
/// or talent, an Overcharge or a piece of jewelry rolls nothing.
pub(super) fn jolt(choice: Choice, hero: &HeroState) -> Option<Rarity> {
    match choice {
        Choice::Upgrade(_, r) | Choice::UpPassive(_, r) => {
            Some(rolled(r, hero)).filter(|r| r.at_least(Rarity::Rare))
        }
        _ => None,
    }
}

/// A spell's rough damage per second at rank 1 against a crowd: per-cast damage times the
/// enemies it can reach, over its cooldown. Only for ranking a class's own spells against each
/// other; the defensive kinds have none and sit in the middle.
fn power(def: &data::AbilityDef) -> Option<f32> {
    let area = |r: f32| 1.0 + r * 0.6;
    let reach = match def.kind {
        Kind::Bolt | Kind::Strike => def.count as f32 + def.radius * 0.4,
        Kind::Melee | Kind::Orbit => def.count as f32,
        Kind::Nova | Kind::Cone | Kind::Ground | Kind::GroundSelf | Kind::Aura => area(def.radius),
        Kind::Chain => def.count as f32 + 1.0,
        Kind::Dot => def.count as f32,
        _ => return None,
    };
    let per = if def.damage > 0.0 {
        def.damage
    } else {
        def.dot * def.duration.max(1.0)
    };
    let ticks = if matches!(def.kind, Kind::Ground | Kind::GroundSelf) && def.duration > 0.0 {
        def.duration
    } else {
        1.0
    };
    Some(per * ticks * reach / def.cooldown.max(0.2))
}

/// Where a new spell stands among its class's: the strongest third Epic, the weakest third
/// Uncommon, the rest (and the defensive spells) Rare.
fn spell_tier(def: &data::AbilityDef, hero: &HeroState) -> Rarity {
    let Some(mine) = power(def) else {
        return Rarity::Rare;
    };
    let mut all: Vec<f32> = hero
        .class
        .pool
        .iter()
        .filter_map(|k| power(data::ability(k)))
        .collect();
    all.sort_by(f32::total_cmp);
    let rank = all.iter().filter(|&&p| p < mine).count() as f32 / all.len().max(1) as f32;
    if rank >= 0.66 {
        Rarity::Epic
    } else if rank < 0.33 {
        Rarity::Uncommon
    } else {
        Rarity::Rare
    }
}

/// What a choice is worth to `hero`, by what it brings: a rank-up its rolled quality (at least
/// Rare under an Empower token), Legendary where it reaches the top rank; a new spell by its power
/// among the class's ([`spell_tier`]), a new talent Uncommon.
pub(super) fn rarity(choice: Choice, hero: &HeroState) -> Rarity {
    match choice {
        Choice::NewAbility(def) => spell_tier(def, hero),
        Choice::Upgrade(k, r) => {
            let Some(o) = hero.abilities.get(k) else {
                return Rarity::Common;
            };
            let r = rolled(r, hero);
            if o.level.saturating_add(r.ranks()) >= data::MAX_ABILITY_LEVEL {
                Rarity::Legendary
            } else {
                r
            }
        }
        Choice::NewPassive(_) => Rarity::Uncommon,
        Choice::UpPassive(k, r) => match hero.passives.get(k) {
            None => Rarity::Common,
            Some(_) => rolled(r, hero),
        },
        Choice::Overcharge(_) => Rarity::Epic,
        Choice::Jewel(piece) => piece.rarity,
        Choice::Heal => Rarity::Common,
    }
}

/// One card's motion: dealt in, tilted to the cursor, lifted under it, burst on the pick.
#[derive(Component)]
pub(super) struct Card {
    index: usize,
    /// Seconds since this card's deal began; negative while it waits its turn.
    t: f32,
    hover: f32,
    tilt: Vec2,
    picked: Option<f32>,
    rarity: Rarity,
    /// The rim that breathes round it: a transformation's gold-orange, or a rolled Rare's or
    /// Epic's own colour.
    rim: Option<Color>,
    /// Seconds its deal takes ([`deal_secs`]).
    deal: f32,
}

/// The rarity glow behind the card, which swells under the cursor.
#[derive(Component)]
pub(super) struct CardGlow;

/// The shine that sweeps across a card as it lands.
#[derive(Component)]
pub(super) struct CardShine(usize);

/// The flame licking up a transforming card's foot.
#[derive(Component)]
pub(super) struct CardFlame(usize);

/// A transformation's gold-orange: the legendary colour warmed toward gold.
fn flame_color() -> Color {
    LEGENDARY.mix(&hud::GOLD_TONE, 0.35)
}

/// The radiant burst behind the row, turning slowly.
#[derive(Component)]
pub(super) struct Rays;

/// The pending pick: the card bursts, then the choice applies.
#[derive(Resource, Default)]
pub(super) struct PendingPick(Option<(usize, f32)>);

impl PendingPick {
    /// A picked card is bursting; the hand is spoken for.
    pub(super) fn busy(&self) -> bool {
        self.0.is_some()
    }
}

const PICK_SECS: f32 = 0.28;

/// The chime a freshly dealt hand rings for its best rolled card, if that is Rare or better.
pub(super) fn jolt_sound(hand: &[Choice], hero: &HeroState) -> Option<&'static str> {
    let best = hand
        .iter()
        .filter_map(|&c| jolt(c, hero))
        .max_by_key(|r| *r as u8)?;
    Some(if best.at_least(Rarity::Epic) {
        ROLL_EPIC_SOUND
    } else {
        ROLL_RARE_SOUND
    })
}

/// The cards, rebuilt for each offer: the local hero's hand. A new hand holding a rolled Rare or
/// Epic rings its chime ([`jolt_sound`]); one chained onto the last deals quicker ([`deal_secs`]).
#[allow(clippy::too_many_arguments)]
pub(super) fn deal_cards(
    mut commands: Commands,
    time: Res<Time<Real>>,
    offer: Res<Offer>,
    local: Query<&HeroState, With<LocalHero>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    font: Res<HudFont>,
    rows: Query<Entity, With<hud::OfferRow>>,
    mut cues: MessageWriter<SoundCue>,
    mut last: Local<(bool, Option<f32>)>,
) {
    if !offer.is_changed() {
        return;
    }
    // Whether a hand was up before this one, and when the last one closed.
    let now = time.elapsed_secs();
    let secs = deal_secs(last.0, last.1, now);
    if offer.len() == 0 {
        if last.0 {
            last.1 = Some(now);
        }
        last.0 = false;
    } else {
        last.0 = true;
    }
    let Ok(row) = rows.single() else { return };
    commands.entity(row).despawn_related::<Children>();
    let Ok(me) = local.single() else { return };
    if offer.redealt().is_none() {
        if let Some(kit) = jolt_sound(offer.choices(), me) {
            cues.write(SoundCue::flat(kit));
        }
    }
    let f = &font.0;
    commands.entity(row).with_children(|row| {
        for (i, &choice) in offer.choices().iter().enumerate() {
            let card = hud::describe_card(choice, me, spells.as_deref());
            let rarity = rarity(choice, me);
            let rc = rarity.color();
            let legend = card.legendary;
            let rim = match (legend, jolt(choice, me)) {
                (Some(_), _) => Some(flame_color()),
                (None, Some(r)) => Some(r.color()),
                (None, None) => None,
            };
            // After a banish only its slot is dealt again; the rest lie where they landed.
            let t = match offer.redealt() {
                Some(r) if r != i => secs + 1.0,
                Some(_) => 0.0,
                None => -(i as f32) * DEAL_STAGGER * secs / DEAL_SECS,
            };
            let mut face = row.spawn((
                Card {
                    index: i,
                    t,
                    hover: 0.0,
                    tilt: Vec2::ZERO,
                    picked: None,
                    rarity,
                    rim,
                    deal: secs,
                },
                Button,
                Interaction::default(),
                RelativeCursorPosition::default(),
                Node {
                    width: Val::Px(CARD_W),
                    // A transformation's lines may stretch it; the row stretches the rest to match.
                    min_height: Val::Px(CARD_H),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(
                        Val::Px(16.0),
                        Val::Px(16.0),
                        Val::Px(18.0),
                        Val::Px(14.0),
                    ),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(10.0)),
                    ..default()
                },
                // A dark card face lit from the top in the rarity's colour.
                BackgroundGradient::from(LinearGradient::to_bottom(vec![
                    ColorStop::percent(rc.mix(&Color::BLACK, 0.55).with_alpha(0.98), 0.0),
                    ColorStop::percent(Color::srgba(0.05, 0.04, 0.035, 0.97), 42.0),
                    ColorStop::percent(Color::srgba(0.02, 0.015, 0.01, 0.98), 100.0),
                ])),
                BorderColor::all(rc),
                BoxShadow::new(
                    rc.with_alpha(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(26.0),
                ),
                UiTransform {
                    translation: Val2::px(0.0, 260.0),
                    scale: Vec2::splat(0.6),
                    rotation: Rot2::degrees(-14.0),
                },
                Visibility::Hidden,
            ));
            // The transformation's gold rim, or a rolled Rare's or Epic's quality glow, pulsing in
            // `animate_cards`.
            if let Some(rim) = rim {
                face.insert(Outline::new(
                    Val::Px(2.0),
                    Val::Px(2.0),
                    rim.with_alpha(0.6),
                ));
            }
            face.with_children(|c| {
                c.spawn((
                    CardGlow,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        height: Val::Px(150.0),
                        ..default()
                    },
                    BackgroundGradient::from(RadialGradient::new(
                        UiPosition::new(Vec2::new(0.0, -0.1), Val::Auto, Val::Auto),
                        RadialGradientShape::FarthestSide,
                        vec![
                            ColorStop::percent(rc.with_alpha(0.45), 0.0),
                            ColorStop::percent(rc.with_alpha(0.0), 100.0),
                        ],
                    )),
                    bevy::ui::FocusPolicy::Pass,
                ));
                if legend.is_some() {
                    c.spawn((
                        CardFlame(i),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            right: Val::Px(0.0),
                            bottom: Val::Px(0.0),
                            height: Val::Px(90.0),
                            border_radius: BorderRadius::bottom(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundGradient::from(LinearGradient::to_top(vec![
                            ColorStop::percent(flame_color().with_alpha(0.55), 0.0),
                            ColorStop::percent(LEGENDARY.with_alpha(0.22), 45.0),
                            ColorStop::percent(LEGENDARY.with_alpha(0.0), 100.0),
                        ])),
                        bevy::ui::FocusPolicy::Pass,
                    ));
                }
                // The key and the rarity on one line along the top.
                c.spawn(Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|top| {
                    top.spawn(hud::text_bundle(
                        f,
                        format!("{}", i + 1),
                        15.0,
                        hud::DIM_GREY,
                    ));
                    top.spawn(hud::text_bundle(f, rarity.label(), 13.0, rc));
                });
                // The icon in a gold-rimmed socket.
                c.spawn((
                    Node {
                        width: Val::Px(74.0),
                        height: Val::Px(74.0),
                        margin: UiRect::vertical(Val::Px(6.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BorderColor::all(rc.mix(&hud::GOLD_TONE, 0.4)),
                    BoxShadow::new(
                        rc.with_alpha(0.55),
                        Val::Px(0.0),
                        Val::Px(0.0),
                        Val::Px(2.0),
                        Val::Px(14.0),
                    ),
                ))
                .with_children(|s| {
                    s.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        ImageNode {
                            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                            ..default()
                        },
                        SpellIcon(card.spell),
                    ));
                });
                c.spawn(hud::centered_bundle(
                    f,
                    card.name,
                    21.0,
                    rc.mix(&Color::WHITE, 0.25),
                ));
                c.spawn(hud::centered_bundle(f, card.tag, 14.0, card.tag_color));
                // The pips: rank now, and the one this pick fills.
                if let Some((have, max)) = card.ranks {
                    c.spawn(Node {
                        column_gap: Val::Px(4.0),
                        margin: UiRect::vertical(Val::Px(2.0)),
                        ..default()
                    })
                    .with_children(|pips| {
                        for r in 0..max {
                            let color = if r < have {
                                hud::GOLD_TONE
                            } else if r == have {
                                rc
                            } else {
                                Color::srgba(1.0, 1.0, 1.0, 0.12)
                            };
                            pips.spawn((
                                Node {
                                    width: Val::Px(14.0),
                                    height: Val::Px(6.0),
                                    border_radius: BorderRadius::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(color),
                            ));
                        }
                    });
                }
                c.spawn((
                    Node {
                        width: Val::Percent(80.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundGradient::from(LinearGradient::to_right(vec![
                        ColorStop::percent(rc.with_alpha(0.0), 0.0),
                        ColorStop::percent(rc.with_alpha(0.8), 50.0),
                        ColorStop::percent(rc.with_alpha(0.0), 100.0),
                    ])),
                ));
                c.spawn(hud::centered_bundle(
                    f,
                    card.desc,
                    14.0,
                    Color::srgb(0.92, 0.9, 0.84),
                ));
                if let Some(l) = legend {
                    transforms(c, f, l);
                }
                // The landing shine, a bright band that crosses once.
                c.spawn((
                    CardShine(i),
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(-40.0),
                        bottom: Val::Px(-40.0),
                        width: Val::Px(70.0),
                        left: Val::Px(-120.0),
                        ..default()
                    },
                    BackgroundGradient::from(LinearGradient::to_right(vec![
                        ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.0), 0.0),
                        ColorStop::percent(Color::srgba(1.0, 0.97, 0.85, 0.28), 50.0),
                        ColorStop::percent(Color::srgba(1.0, 1.0, 1.0, 0.0), 100.0),
                    ])),
                    UiTransform::from_rotation(Rot2::degrees(18.0)),
                    bevy::ui::FocusPolicy::Pass,
                ));
            });
        }
    });
}

/// The "Transforms:" panel under a legendary card's stats: the transformation's icon and name,
/// and its line.
fn transforms(c: &mut ChildSpawnerCommands, f: &Handle<Font>, l: &data::LegendaryDef) {
    let hot = flame_color();
    c.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            margin: UiRect::top(Val::Px(4.0)),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BorderColor::all(hot.with_alpha(0.7)),
        BackgroundColor(LEGENDARY.with_alpha(0.12)),
    ))
    .with_children(|t| {
        t.spawn(Node {
            column_gap: Val::Px(6.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|head| {
            head.spawn((
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    border_radius: BorderRadius::all(Val::Px(3.0)),
                    ..default()
                },
                ImageNode {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                    ..default()
                },
                SpellIcon(l.visual_spell),
            ));
            head.spawn(hud::text_bundle(
                f,
                format!("Transforms: {}", l.name),
                15.0,
                hot.mix(&Color::WHITE, 0.15),
            ));
        });
        t.spawn(hud::centered_bundle(
            f,
            l.text,
            13.0,
            Color::srgb(1.0, 0.86, 0.6),
        ));
    });
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Deal, tilt, lift and burst: real time, since the game clock is paused under the picker.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn animate_cards(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    mut cards: Query<(
        &mut Card,
        &Interaction,
        &RelativeCursorPosition,
        &mut UiTransform,
        &mut BoxShadow,
        &mut Visibility,
        Option<&mut Outline>,
    )>,
    mut shines: Query<(&CardShine, &mut Node)>,
    mut flames: Query<(&CardFlame, &mut Node), Without<CardShine>>,
    mut rays: Query<&mut UiTransform, (With<Rays>, Without<Card>)>,
    mut pending: ResMut<PendingPick>,
    mut run: ResMut<Run>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut offer: ResMut<Offer>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
    mode: Res<HandMode>,
    mut outbox: Option<ResMut<party::HandOutbox>>,
    strip: Option<Res<super::strip::HandStrip>>,
) {
    if !super::strip::hand_up(*phase.get(), strip.as_deref()) {
        pending.0 = None;
        return;
    }
    // The co-op strip takes its pick off the banked hands; the paused level-up deals on.
    let banked = *phase.get() == Phase::Playing;
    let dt = time.delta_secs().min(0.05);
    let now = time.elapsed_secs();
    for mut r in &mut rays {
        r.rotation = Rot2::radians(now * 0.12);
        r.scale = Vec2::splat(1.0 + 0.03 * (now * 1.7).sin());
    }
    let chosen = pending.0.map(|(i, _)| i);
    let mut shine_at = [None::<(f32, f32)>; 4];
    for (mut card, interaction, cursor, mut tf, mut shadow, mut vis, outline) in &mut cards {
        let card = &mut *card;
        card.t += dt;
        if card.t < 0.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        let deal = (card.t / card.deal).clamp(0.0, 1.0);
        let e = ease_out_back(deal);
        let hot = *interaction != Interaction::None && chosen.is_none() && deal >= 1.0;
        let k = 1.0 - (-dt * 14.0).exp();
        let hv = card.hover;
        card.hover = hv + ((if hot { 1.0 } else { 0.0 }) - hv) * k;
        let want_tilt = if hot {
            cursor
                .normalized
                .unwrap_or_default()
                .clamp(Vec2::splat(-0.5), Vec2::splat(0.5))
        } else {
            Vec2::ZERO
        };
        let tl = card.tilt;
        card.tilt = tl + (want_tilt - tl) * k;
        let h = card.hover;
        // The pick: the chosen card swells and rises, the others drop away.
        let (pick_scale, pick_y, pick_fade) = match (chosen, card.picked) {
            (Some(c), _) if c == card.index => {
                let p = card.picked.get_or_insert(0.0);
                *p += dt;
                let u = (*p / PICK_SECS).clamp(0.0, 1.0);
                (1.0 + 0.18 * u, -30.0 * u, 0.0)
            }
            (Some(_), _) => {
                let p = card.picked.get_or_insert(0.0);
                *p += dt;
                let u = (*p / PICK_SECS).clamp(0.0, 1.0);
                (1.0 - 0.15 * u, 140.0 * u * u, u)
            }
            _ => (1.0, 0.0, 0.0),
        };
        // A perspective tilt, faked: the card turns about the cursor's side and leans with it.
        let lean = card.tilt.x * 7.0;
        let squash = 1.0 - card.tilt.x.abs() * 0.08;
        let bob = (now * 2.2 + card.index as f32 * 1.3).sin() * 1.5 * (1.0 - h);
        tf.translation = Val2::px(
            0.0,
            (1.0 - e) * 260.0 - 18.0 * h + card.tilt.y * 8.0 + bob + pick_y,
        );
        let s = (0.6 + 0.4 * e) * (1.0 + 0.06 * h) * pick_scale;
        tf.scale = Vec2::new(s * squash, s * (1.0 + card.tilt.y.abs() * 0.02));
        tf.rotation = Rot2::degrees((1.0 - e) * -14.0 + lean);
        // An armed Banish lights every card red.
        let rc = if mode.banishing {
            hand::BANISH_RED
        } else {
            card.rarity.color()
        };
        if let Some(sh) = shadow.0.first_mut() {
            sh.color = rc.with_alpha((0.25 + 0.6 * h) * (1.0 - pick_fade));
            sh.blur_radius = Val::Px(18.0 + 22.0 * h);
            sh.spread_radius = Val::Px(2.0 + 4.0 * h);
            sh.y_offset = Val::Px(6.0 + 10.0 * h);
        }
        // A transformation's rim breathes, as does a rolled Rare's or Epic's.
        if let (Some(mut outline), Some(rim)) = (outline, card.rim) {
            let glow = 0.45 + 0.4 * (now * 3.1 + card.index as f32).sin().abs();
            outline.color = rim.with_alpha(glow * (1.0 - pick_fade));
        }
        if pick_fade >= 1.0 {
            *vis = Visibility::Hidden;
        }
        if card.index < shine_at.len() {
            shine_at[card.index] = Some((card.t, card.deal));
        }
    }
    // The flame flickers on two beats.
    for (flame, mut node) in &mut flames {
        let k = flame.0 as f32;
        node.height =
            Val::Px(84.0 + 9.0 * (now * 6.3 + k).sin() + 5.0 * (now * 11.7 + 2.0 * k).sin());
    }
    for (shine, mut node) in &mut shines {
        let Some((t, deal)) = shine_at.get(shine.0).copied().flatten() else {
            continue;
        };
        let u = ((t - deal * 0.6) / 0.5).clamp(0.0, 1.0);
        node.left = Val::Px(-120.0 + u * (CARD_W + 200.0));
    }
    if let Some((i, t)) = pending.0.as_mut() {
        *t += dt;
        if *t >= PICK_SECS {
            let i = *i;
            pending.0 = None;
            if let Some(outbox) = outbox.as_deref_mut() {
                // A co-op client: the host applies the pick and deals what comes next.
                outbox.0.push(party::HandDecision::Pick(i));
                offer.deal(Vec::new());
            } else if let Ok(mut me) = local.single_mut() {
                if banked {
                    if let Some(&choice) = offer.choices().get(i) {
                        if party::apply_pick(&mut me, choice) {
                            run.version += 1;
                        }
                    }
                    offer.deal(Vec::new());
                } else {
                    hud::pick(i, &mut me, &mut run, &mut offer, &mut rng, &mut next);
                }
            }
        }
    }
}

/// A click on a card, or 1/2/3: start its burst; the choice applies when it finishes. With Banish
/// armed the card is struck instead ([`hand::banish`]).
#[allow(clippy::too_many_arguments)]
pub(super) fn choose_card(
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    cards: Query<(&Card, &Interaction), Changed<Interaction>>,
    mut offer: ResMut<Offer>,
    mut pending: ResMut<PendingPick>,
    mut mode: ResMut<HandMode>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut rng: ResMut<Rng>,
    mut outbox: Option<ResMut<party::HandOutbox>>,
    strip: Option<Res<super::strip::HandStrip>>,
) {
    if !super::strip::hand_up(*phase.get(), strip.as_deref()) || pending.0.is_some() {
        return;
    }
    let mut pick = cards
        .iter()
        .find(|(c, i)| **i == Interaction::Pressed && c.t >= c.deal)
        .map(|(c, _)| c.index);
    for (i, key) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
        .into_iter()
        .enumerate()
    {
        if keys.just_pressed(key) {
            pick = Some(i);
        }
    }
    let Some(i) = pick.filter(|&i| i < offer.len()) else {
        return;
    };
    if !mode.banishing {
        pending.0 = Some((i, 0.0));
        return;
    }
    let Ok(mut me) = local.single_mut() else {
        return;
    };
    if hand::strikable(i, &me, &offer) {
        match outbox.as_deref_mut() {
            Some(outbox) => outbox.0.push(party::HandDecision::Banish(i)),
            None => {
                hand::banish(i, &mut me, &mut offer, &mut rng);
            }
        }
        mode.banishing = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stacked_hand_deals_quicker_than_the_first() {
        // Nothing before it: the full deal.
        assert_eq!(deal_secs(false, None, 10.0), DEAL_SECS);
        // Dealt over the hand before, or just after it closed: chained.
        assert_eq!(deal_secs(true, None, 10.0), CHAIN_DEAL_SECS);
        assert_eq!(deal_secs(false, Some(9.8), 10.0), CHAIN_DEAL_SECS);
        // A hand long after the last is a fresh one.
        assert_eq!(deal_secs(false, Some(2.0), 10.0), DEAL_SECS);
    }
}
