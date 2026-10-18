//! The jewelry choice on screen, cards-like as the level-up hand deals its own: three pieces, each
//! its icon in a socket, its name in its rarity's colour, its slot and what it grants, and a
//! button for the gold instead. A piece whose slots are full turns the row into the choice of
//! which worn piece it replaces, the new one beside them. Click a card or press 1, 2 or 3; Esc goes
//! back from the replacing.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use super::super::auto::Auto;
use super::super::gear::{rarity_label, ItemIcon};
use super::super::hud::{self, HudFont};
use super::super::profile::Profile;
use super::super::{HeroState, LocalHero, Phase, Rng, Run};
use super::{
    act, close, gold_instead, proc_text, stat_deltas, Act, Done, JewelOffer, Jewelry, Piece,
};

const CARD_W: f32 = 220.0;
const CARD_H: f32 = 300.0;
/// A fresh offer takes no input this long (real seconds), so a double click takes one card only.
const SETTLE_SECS: f32 = 0.25;
/// `GREEN_FONT_COLOR`: what a piece grants.
const GRANTS: Color = Color::srgb(0.1, 1.0, 0.1);
/// `RED_FONT_COLOR`: what a replacement takes away.
const LOSES: Color = Color::srgb(1.0, 0.125, 0.125);
const PARCHMENT: Color = Color::srgb(0.85, 0.78, 0.6);
const FULL: Color = Color::srgb(1.0, 0.55, 0.2);
const FACE: Color = Color::srgba(0.18, 0.06, 0.04, 0.95);
const FACE_HOT: Color = Color::srgba(0.36, 0.12, 0.06, 0.98);
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);

#[derive(Component)]
pub(super) struct JewelRoot;
#[derive(Component)]
pub(super) struct JewelSub;
#[derive(Component)]
pub(super) struct JewelRow;
#[derive(Component)]
pub(super) struct JewelTools;
#[derive(Component)]
pub(super) struct JewelHint;

/// A card, lit in its rarity's colour under the cursor.
#[derive(Component)]
pub(super) struct JewelCard(Color);

/// What a click on a card or button does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct JewelButton(Act);

/// Whether this run's picks are logged in the profile's Codex: not for an auto-runner run the
/// profile does not record ([`super::super::profile::record_finished`]), as its gear is not.
fn records(auto: Option<&Auto>) -> bool {
    auto.is_none() || std::env::var("WOW_SURVIVORS_RECORD").as_deref() == Ok("1")
}

/// The overlay, built once the HUD's font is in; hidden until the picker opens.
pub(super) fn spawn_root(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    built: Query<(), With<JewelRoot>>,
) {
    if !built.is_empty() {
        return;
    }
    let Some(font) = font else { return };
    let f = &font.0;
    commands
        .spawn((
            JewelRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(16.0),
                display: Display::None,
                ..default()
            },
            // The level-up hand's dark vignette.
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Color::srgba(0.04, 0.025, 0.0, 0.72), 0.0),
                    ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.9), 100.0),
                ],
            )),
            GlobalZIndex(420),
            // The build's icons stay hoverable round the cards.
            FocusPolicy::Pass,
        ))
        .with_children(|root| {
            root.spawn(hud::centered_bundle(f, "TREASURE", 52.0, hud::GOLD_TONE));
            root.spawn((JewelSub, hud::centered_bundle(f, "", 16.0, PARCHMENT)));
            root.spawn((
                JewelRow,
                Node {
                    column_gap: Val::Px(22.0),
                    align_items: AlignItems::Center,
                    margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(20.0), Val::Px(12.0)),
                    ..default()
                },
            ));
            root.spawn((
                JewelTools,
                Node {
                    column_gap: Val::Px(14.0),
                    ..default()
                },
            ));
            root.spawn((JewelHint, hud::centered_bundle(f, "", 14.0, hud::DIM_GREY)));
        });
}

/// What a key does: 1, 2 and 3 take a card or, while replacing, name the slot; Esc goes back.
fn key_act(keys: &ButtonInput<KeyCode>, offer: &JewelOffer) -> Option<Act> {
    let digits = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3];
    let n = digits.iter().position(|k| keys.just_pressed(*k));
    match offer.replacing() {
        Some(piece) => {
            if keys.just_pressed(KeyCode::Escape) {
                return Some(Act::Back);
            }
            n.and_then(|n| Jewelry::slots_of(piece.kind).nth(n))
                .map(Act::Replace)
        }
        None if keys.just_pressed(KeyCode::Escape) => Some(Act::Gold),
        None => n.map(Act::Take),
    }
}

/// A piece taken is logged in the Codex; gold changes nothing there.
fn log_taken(done: Done, profile: Option<&mut ResMut<Profile>>, auto: Option<&Auto>) {
    let (Done::Took(piece, _), Some(profile)) = (done, profile) else {
        return;
    };
    if records(auto) {
        profile.codex.take(piece.key);
        profile.mark_dirty();
    }
}

/// Clicks on the cards and buttons, and the keys.
#[allow(clippy::too_many_arguments)]
pub(super) fn jewel_input(
    phase: Res<State<Phase>>,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    presses: Query<(&Interaction, &JewelButton), Changed<Interaction>>,
    mut offer: ResMut<JewelOffer>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
    mut profile: Option<ResMut<Profile>>,
    auto: Option<Res<Auto>>,
    mut settle: Local<f32>,
) {
    if *phase.get() != Phase::Jewelry {
        return;
    }
    if offer.is_changed() {
        *settle = 0.0;
    }
    *settle += time.delta_secs();
    if *settle < SETTLE_SECS {
        return;
    }
    let pressed = presses
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, b)| b.0);
    let Some(a) = pressed.or_else(|| key_act(&keys, &offer)) else {
        return;
    };
    let Ok(mut me) = local.single_mut() else {
        return;
    };
    let Some(done) = act(a, &mut me, &mut run, &mut offer) else {
        return;
    };
    log_taken(done, profile.as_mut(), auto.as_deref());
    if let Some(phase) = close(done, &mut me, &mut run, &mut offer, &mut rng) {
        next.set(phase);
    }
}

/// The auto-runner takes a random piece a second after each offer, over a random worn one when
/// its slots are full.
#[allow(clippy::too_many_arguments)]
pub(super) fn auto_pick(
    phase: Res<State<Phase>>,
    time: Res<Time<Real>>,
    mut offer: ResMut<JewelOffer>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
    mut profile: Option<ResMut<Profile>>,
    auto: Option<Res<Auto>>,
    mut wait: Local<f32>,
) {
    if *phase.get() != Phase::Jewelry || offer.pieces().is_empty() {
        *wait = 0.0;
        return;
    }
    *wait += time.delta_secs();
    if *wait < 1.0 {
        return;
    }
    *wait = 0.0;
    let Ok(mut me) = local.single_mut() else {
        return;
    };
    let i = rng.index(offer.pieces().len());
    let mut done = act(Act::Take(i), &mut me, &mut run, &mut offer);
    if let (None, Some(piece)) = (done, offer.replacing()) {
        let slots: Vec<usize> = Jewelry::slots_of(piece.kind).collect();
        let slot = slots[rng.index(slots.len())];
        done = act(Act::Replace(slot), &mut me, &mut run, &mut offer);
    }
    let Some(done) = done else { return };
    log_taken(done, profile.as_mut(), auto.as_deref());
    if let Some(phase) = close(done, &mut me, &mut run, &mut offer, &mut rng) {
        next.set(phase);
    }
}

/// Every piece an offer shows is logged as seen in the Codex.
pub(super) fn log_seen(
    offer: Res<JewelOffer>,
    mut profile: Option<ResMut<Profile>>,
    auto: Option<Res<Auto>>,
) {
    if !offer.is_changed() || offer.pieces().is_empty() || !records(auto.as_deref()) {
        return;
    }
    let Some(profile) = profile.as_mut() else {
        return;
    };
    // Read first, so a profile that already knows them is not marked changed.
    if offer.pieces().iter().all(|p| profile.codex.saw(p.key)) {
        return;
    }
    for p in offer.pieces() {
        profile.codex.see(p.key);
    }
    profile.mark_dirty();
}

/// The overlay shows while the picker is open, rebuilt for each offer and each step of replacing.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_offer(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    offer: Res<JewelOffer>,
    local: Query<&HeroState, With<LocalHero>>,
    run: Res<Run>,
    font: Option<Res<HudFont>>,
    mut roots: Query<&mut Node, With<JewelRoot>>,
    rows: Query<Entity, With<JewelRow>>,
    tools: Query<Entity, With<JewelTools>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<JewelSub>>,
        Query<&mut Text, With<JewelHint>>,
    )>,
    mut shown: Local<bool>,
) {
    let on = *phase.get() == Phase::Jewelry;
    for mut node in &mut roots {
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    if !on {
        *shown = false;
        return;
    }
    if *shown && !offer.is_changed() {
        return;
    }
    let (Some(font), Ok(row), Ok(tools), Ok(me)) =
        (font, rows.single(), tools.single(), local.single())
    else {
        return;
    };
    *shown = true;
    let f = &font.0;
    commands.entity(row).despawn_related::<Children>();
    commands.entity(tools).despawn_related::<Children>();
    let (sub, hint) = match offer.replacing() {
        Some(piece) => {
            let kind = piece.kind;
            let worn: Vec<(usize, &'static Piece)> = Jewelry::slots_of(kind)
                .filter_map(|s| me.jewelry.slot(s).map(|p| (s, p)))
                .collect();
            commands.entity(row).with_children(|r| {
                card(r, f, piece, "New".into(), None, None, None);
                r.spawn(hud::centered_bundle(f, "»", 40.0, hud::GOLD_TONE));
                for (n, &(slot, old)) in worn.iter().enumerate() {
                    card(
                        r,
                        f,
                        old,
                        format!("{}  Replace", n + 1),
                        Some(Act::Replace(slot)),
                        None,
                        Some(piece),
                    );
                }
            });
            commands.entity(tools).with_children(|t| {
                tool(t, f, "Back (Esc)".into(), Act::Back);
            });
            let keys = if worn.len() > 1 { "1 or 2" } else { "1" };
            (
                format!(
                    "Your {} are full: which does {} replace?",
                    kind.plural().to_lowercase(),
                    piece.name
                ),
                format!(
                    "Click a {} to replace it, or press {keys}",
                    kind.label().to_lowercase()
                ),
            )
        }
        None => {
            commands.entity(row).with_children(|r| {
                for (i, &piece) in offer.pieces().iter().enumerate() {
                    let note = match me.jewelry.free(piece.kind) {
                        Some(_) => None,
                        None => Some((
                            format!("{} slots full: replaces one", piece.kind.label()),
                            FULL,
                        )),
                    };
                    card(
                        r,
                        f,
                        piece,
                        format!("{}", i + 1),
                        Some(Act::Take(i)),
                        note,
                        None,
                    );
                }
            });
            let gold = gold_instead(&run, me);
            commands.entity(tools).with_children(|t| {
                tool(t, f, format!("Take gold instead: +{gold} gold"), Act::Gold);
            });
            let keys = match offer.pieces().len() {
                1 => "1",
                2 => "1 or 2",
                _ => "1, 2 or 3",
            };
            (
                "Choose a piece of jewelry".to_string(),
                format!("Click a piece, or press {keys}"),
            )
        }
    };
    if let Ok(mut t) = texts.p0().single_mut() {
        t.0 = sub;
    }
    if let Ok(mut t) = texts.p1().single_mut() {
        t.0 = hint;
    }
}

/// One piece as a card: the key and rarity along the top, the icon in a socket, the name in its
/// rarity's colour, its slot, a note when there is one, and what it grants. A card with `act` is a
/// button. A worn piece the `replaced_by` piece would replace shows, under what it grants, what
/// the swap changes ([`stat_deltas`]): gains in green, losses in red.
#[allow(clippy::too_many_arguments)]
fn card(
    row: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    piece: &'static Piece,
    top: String,
    act: Option<Act>,
    note: Option<(String, Color)>,
    replaced_by: Option<&'static Piece>,
) {
    let rc = piece.rarity.color();
    let mut c = row.spawn((
        Node {
            width: Val::Px(CARD_W),
            // A replacement's comparison may stretch it.
            min_height: Val::Px(CARD_H),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(16.0), Val::Px(12.0)),
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
            rc.with_alpha(0.25),
            Val::Px(0.0),
            Val::Px(6.0),
            Val::Px(2.0),
            Val::Px(18.0),
        ),
        JewelCard(rc),
    ));
    if let Some(a) = act {
        c.insert((Button, Interaction::default(), JewelButton(a)));
    }
    c.with_children(|c| {
        c.spawn(Node {
            width: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|t| {
            t.spawn(hud::text_bundle(f, top, 14.0, hud::DIM_GREY));
            t.spawn(hud::text_bundle(f, rarity_label(piece.rarity), 13.0, rc));
        });
        c.spawn((
            Node {
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                margin: UiRect::vertical(Val::Px(4.0)),
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
                ItemIcon {
                    icon: piece.icon,
                    tint: Color::WHITE,
                },
            ));
        });
        c.spawn(hud::centered_bundle(
            f,
            piece.name,
            19.0,
            rc.mix(&Color::WHITE, 0.25),
        ));
        c.spawn(hud::centered_bundle(
            f,
            piece.kind.label(),
            14.0,
            Color::WHITE,
        ));
        if let Some((text, color)) = note {
            c.spawn(hud::centered_bundle(f, text, 12.0, color));
        }
        if let Some(proc) = piece.proc {
            c.spawn(hud::centered_bundle(
                f,
                proc_text(proc),
                12.0,
                hud::GOLD_TONE,
            ));
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
        for b in piece.bonuses {
            c.spawn(hud::centered_bundle(f, b.describe(), 14.0, GRANTS));
        }
        if let Some(new) = replaced_by {
            c.spawn(hud::centered_bundle(f, "If replaced:", 12.0, PARCHMENT));
            for b in stat_deltas(new, piece) {
                let color = if b.value > 0.0 { GRANTS } else { LOSES };
                c.spawn(hud::centered_bundle(f, b.describe(), 13.0, color));
            }
        }
    });
}

/// A button under the cards.
fn tool(t: &mut ChildSpawnerCommands, f: &Handle<Font>, label: String, act: Act) {
    t.spawn((
        JewelButton(act),
        Button,
        Node {
            min_width: Val::Px(200.0),
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
        b.spawn(hud::centered_bundle(f, label, 16.0, Color::WHITE));
    });
}

/// Cards glow and buttons light under the cursor.
#[allow(clippy::type_complexity)]
pub(super) fn paint_buttons(
    mut cards: Query<(&Interaction, &JewelCard, &mut BoxShadow), Changed<Interaction>>,
    mut tools: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<JewelButton>, Without<JewelCard>),
    >,
) {
    for (interaction, card, mut shadow) in &mut cards {
        let hot = *interaction != Interaction::None;
        if let Some(sh) = shadow.0.first_mut() {
            sh.color = card.0.with_alpha(if hot { 0.8 } else { 0.25 });
            sh.blur_radius = Val::Px(if hot { 34.0 } else { 18.0 });
            sh.spread_radius = Val::Px(if hot { 5.0 } else { 2.0 });
        }
    }
    for (interaction, mut bg) in &mut tools {
        bg.0 = if *interaction == Interaction::None {
            FACE
        } else {
            FACE_HOT
        };
    }
}
