//! A pull's reveal: its prizes come up face down as cards (one big card for a single pull, two rows
//! of five for ten) and flip over one by one, each edged and glowing in its rarity's colour and
//! badged "NEW!" or with the Favor its duplicate paid, to the client's own 1.12 interface sounds.
//! An Epic holds the beat a moment longer; a Legendary flashes the screen gold and orange to the
//! level-up sting. A click or Space flips the rest at once; once all are up, another closes it.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use super::super::gear::ItemIcon;
use super::super::hud::{self, HudFont};
use super::super::run_options::GREEN;
use super::super::Phase;
use super::booth::Pull;
use super::catalog::Rarity;
use super::screen::{icon_node, FAVOR, PARCHMENT};
use crate::glue::art::{DIM, GOLD};
use crate::glue::GlueClicks;
use crate::sound::GlueSound;

/// Over the Faire (415), under the level-up cards (420) it never shares a frame with.
const Z: i32 = 418;
/// Seconds before the first card turns, a card's turn, and the beat between cards.
const FIRST_FLIP: f32 = 0.45;
const FLIP_SECS: f32 = 0.34;
const BEAT: f32 = 0.24;
/// The longer beat after an Epic or better, and after a Legendary.
const EPIC_BEAT: f32 = 0.3;
const LEGENDARY_BEAT: f32 = 0.7;
/// The Legendary flash's seconds.
const FLASH_SECS: f32 = 1.1;

/// The card back's art.
pub(super) const CARD_BACK: &str = "INV_Misc_Ticket_Tarot_Stack_01";
/// The SoundEntries kits a card plays as it turns: a page for a new Common or Uncommon, coins for
/// a duplicate, the quest chimes for a Rare and an Epic, and the level-up sting for a Legendary.
const SOUND_TURN: &str = "igSpellBokPageTurn";
const SOUND_FAVOR: &str = "LOOTWINDOWCOINSOUND";
const SOUND_RARE: &str = "QUESTADDED";
const SOUND_EPIC: &str = "QUESTCOMPLETED";
const SOUND_LEGENDARY: &str = "LEVELUPSOUND";
#[cfg(test)]
pub(super) const SOUNDS: [&str; 5] = [
    SOUND_TURN,
    SOUND_FAVOR,
    SOUND_RARE,
    SOUND_EPIC,
    SOUND_LEGENDARY,
];

/// The pull on show, if any.
#[derive(Resource, Default)]
pub(super) struct RevealState(pub(super) Option<Reveal>);

pub(super) struct Reveal {
    pulls: Vec<Pull>,
    title: &'static str,
    /// Seconds since the reveal began, real time.
    t: f32,
    /// When each card begins to turn.
    starts: Vec<f32>,
    /// Whether each card is face up (its sound played).
    shown: Vec<bool>,
    /// Flipped all at once: the turned cards play one sound between them.
    skipped: bool,
    /// Seconds into the Legendary flash.
    flash: Option<f32>,
    /// Which reveal the overlay was built for.
    serial: u32,
}

impl Reveal {
    /// The schedule: each card turns a beat after the one before, longer after the rare ones.
    fn new(pulls: Vec<Pull>, title: &'static str, serial: u32) -> Self {
        let mut at = FIRST_FLIP;
        let starts = pulls
            .iter()
            .map(|p| {
                let start = at;
                at += FLIP_SECS + BEAT;
                at += match p.prize().rarity {
                    Rarity::Legendary => LEGENDARY_BEAT,
                    Rarity::Epic => EPIC_BEAT,
                    _ => 0.0,
                };
                start
            })
            .collect();
        Self {
            shown: vec![false; pulls.len()],
            pulls,
            title,
            t: 0.0,
            starts,
            skipped: false,
            flash: None,
            serial,
        }
    }

    fn done(&self) -> bool {
        self.shown.iter().all(|&s| s)
    }

    /// Turn every card now.
    fn skip(&mut self) {
        self.skipped = true;
        for s in &mut self.starts {
            *s = s.min(self.t - FLIP_SECS);
        }
    }
}

impl RevealState {
    /// Put `pulls` up, face down.
    pub(super) fn start(&mut self, pulls: Vec<Pull>, title: &'static str) {
        let serial = self.0.as_ref().map_or(1, |r| r.serial.wrapping_add(1));
        self.0 = Some(Reveal::new(pulls, title, serial));
    }
}

/// The sound for a card turning up `pull`.
fn sound_for(pull: &Pull) -> &'static str {
    match pull.prize().rarity {
        Rarity::Legendary => SOUND_LEGENDARY,
        Rarity::Epic => SOUND_EPIC,
        Rarity::Rare => SOUND_RARE,
        _ if !pull.new => SOUND_FAVOR,
        _ => SOUND_TURN,
    }
}

#[derive(Component)]
pub(super) struct RevealRoot(u32);
#[derive(Component)]
struct RevealCard(usize);
#[derive(Component)]
struct CardBack(usize);
#[derive(Component)]
struct CardFace(usize);
#[derive(Component)]
struct Flash;
#[derive(Component)]
struct Footer;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<RevealState>().add_systems(
        Update,
        (reveal_input, sync_reveal, animate_reveal)
            .chain()
            .after(hud::keys)
            .before(hud::show_panels),
    );
}

/// A click anywhere or Space (or Esc, or Enter) turns the rest; once all are up, closes the
/// reveal. The key is the reveal's alone: the Faire under it must not close on the same Esc.
pub(super) fn reveal_input(
    clicks: Res<GlueClicks>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    roots: Query<Entity, With<RevealRoot>>,
    mut state: ResMut<RevealState>,
) {
    if *phase.get() != Phase::Menu {
        if state.0.is_some() {
            state.0 = None;
        }
        return;
    }
    let Some(done) = state.0.as_ref().map(Reveal::done) else {
        return;
    };
    let mut pressed = false;
    for key in [KeyCode::Space, KeyCode::Escape, KeyCode::Enter] {
        pressed |= keys.clear_just_pressed(key);
    }
    let clicked = roots.iter().any(|e| clicks.hit(e));
    if !(pressed || clicked) {
        return;
    }
    if done {
        state.0 = None;
    } else if let Some(reveal) = state.0.as_mut() {
        reveal.skip();
    }
}

/// Build the overlay for a new reveal, and take it down when the reveal ends.
fn sync_reveal(
    mut commands: Commands,
    state: Res<RevealState>,
    roots: Query<(Entity, &RevealRoot)>,
    font: Res<HudFont>,
) {
    let want = state.0.as_ref().map(|r| r.serial);
    for (e, root) in &roots {
        if Some(root.0) != want {
            commands.entity(e).despawn();
        }
    }
    let Some(reveal) = state.0.as_ref() else {
        return;
    };
    if roots.iter().any(|(_, r)| r.0 == reveal.serial) {
        return;
    }
    spawn_overlay(&mut commands, reveal, &font.0);
}

/// The dim, the title, the face-down cards, the flash and the hint.
fn spawn_overlay(commands: &mut Commands, reveal: &Reveal, f: &Handle<Font>) {
    let px = Val::Px;
    let big = reveal.pulls.len() == 1;
    let (w, h) = if big { (176.0, 244.0) } else { (118.0, 168.0) };
    commands
        .spawn((
            RevealRoot(reveal.serial),
            Button,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(14.0),
                ..default()
            },
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Color::srgba(0.12, 0.04, 0.2, 0.88), 0.0),
                    ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.92), 100.0),
                ],
            )),
            GlobalZIndex(Z),
            FocusPolicy::Block,
        ))
        .with_children(|root| {
            root.spawn(hud::centered_bundle(f, reveal.title, 26.0, GOLD));
            let rows: Vec<&[Pull]> = reveal.pulls.chunks(5).collect();
            let mut index = 0;
            for row in rows {
                root.spawn((
                    Node {
                        column_gap: px(14.0),
                        ..default()
                    },
                    FocusPolicy::Pass,
                ))
                .with_children(|r| {
                    for pull in row {
                        card(r, f, index, pull, w, h, big);
                        index += 1;
                    }
                });
            }
            root.spawn((
                Footer,
                hud::centered_bundle(f, "Click or press Space to turn them all", 14.0, DIM),
            ));
            root.spawn((
                Flash,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                flash_gradient(0.0),
                Visibility::Hidden,
                FocusPolicy::Pass,
            ));
        });
}

/// One card: the back (Darkmoon's tarot stack) and, hidden until it turns, the face.
#[allow(clippy::too_many_arguments)]
fn card(
    r: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    index: usize,
    pull: &Pull,
    w: f32,
    h: f32,
    big: bool,
) {
    let px = Val::Px;
    let prize = pull.prize();
    let rc = prize.rarity.color();
    let fill = |n: f32| Node {
        position_type: PositionType::Absolute,
        left: px(0.0),
        right: px(0.0),
        top: px(0.0),
        bottom: px(0.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        padding: UiRect::all(px(n)),
        row_gap: px(4.0),
        border_radius: BorderRadius::all(px(6.0)),
        ..default()
    };
    r.spawn((
        RevealCard(index),
        Node {
            width: px(w),
            height: px(h),
            border: UiRect::all(px(2.0)),
            border_radius: BorderRadius::all(px(8.0)),
            ..default()
        },
        BorderColor::all(GOLD.mix(&Color::BLACK, 0.3)),
        BoxShadow::new(rc.with_alpha(0.0), px(0.0), px(0.0), px(0.0), px(22.0)),
        UiTransform::default(),
        FocusPolicy::Pass,
    ))
    .with_children(|c| {
        // The back.
        c.spawn((
            CardBack(index),
            fill(8.0),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(Color::srgb(0.25, 0.08, 0.38), 0.0),
                ColorStop::percent(Color::srgb(0.08, 0.02, 0.14), 100.0),
            ])),
            FocusPolicy::Pass,
        ))
        .with_children(|b| {
            icon_node(b, CARD_BACK, if big { 72.0 } else { 48.0 }, Color::WHITE);
            b.spawn(hud::centered_bundle(f, "Darkmoon Faire", 11.0, FAVOR));
        });
        // The face.
        let size = if big { 72.0 } else { 46.0 };
        c.spawn((
            CardFace(index),
            fill(if big { 10.0 } else { 6.0 }),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                ColorStop::percent(rc.mix(&Color::BLACK, 0.5).with_alpha(0.98), 0.0),
                ColorStop::percent(Color::srgba(0.05, 0.04, 0.035, 0.97), 55.0),
                ColorStop::percent(Color::srgba(0.02, 0.015, 0.01, 0.98), 100.0),
            ])),
            Visibility::Hidden,
            FocusPolicy::Pass,
        ))
        .with_children(|face| {
            face.spawn(hud::centered_bundle(
                f,
                prize.rarity.label(),
                if big { 13.0 } else { 10.0 },
                rc,
            ));
            face.spawn((
                Node {
                    width: px(size),
                    height: px(size),
                    border: UiRect::all(px(2.0)),
                    border_radius: BorderRadius::all(px(5.0)),
                    ..default()
                },
                BorderColor::all(rc.mix(&GOLD, 0.4)),
                BoxShadow::new(rc.with_alpha(0.6), px(0.0), px(0.0), px(2.0), px(12.0)),
            ))
            .with_children(|s| {
                s.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        border_radius: BorderRadius::all(px(3.0)),
                        ..default()
                    },
                    ImageNode {
                        color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                        ..default()
                    },
                    ItemIcon {
                        icon: prize.icon,
                        tint: Color::WHITE,
                    },
                ));
            });
            face.spawn(hud::centered_bundle(
                f,
                prize.name.clone(),
                if big { 16.0 } else { 12.0 },
                rc.mix(&Color::WHITE, 0.25),
            ));
            face.spawn(hud::centered_bundle(
                f,
                prize.kind.label(),
                if big { 12.0 } else { 10.0 },
                PARCHMENT,
            ));
            let (line, color) = if pull.new {
                ("NEW!".to_string(), GREEN)
            } else {
                (format!("+{} Favor", pull.favor), FAVOR)
            };
            face.spawn(hud::centered_bundle(
                f,
                line,
                if big { 15.0 } else { 12.0 },
                color,
            ));
            if pull.featured {
                face.spawn(hud::centered_bundle(f, "Featured", 10.0, GOLD));
            }
        });
    });
}

/// The Legendary flash at `alpha`: gold at the heart, orange out to the edges.
fn flash_gradient(alpha: f32) -> BackgroundGradient {
    BackgroundGradient::from(RadialGradient::new(
        UiPosition::CENTER,
        RadialGradientShape::FarthestCorner,
        vec![
            ColorStop::percent(Color::srgba(1.0, 0.85, 0.35, 0.9 * alpha), 0.0),
            ColorStop::percent(Color::srgba(1.0, 0.5, 0.0, 0.55 * alpha), 45.0),
            ColorStop::percent(Color::srgba(0.6, 0.2, 0.0, 0.0), 100.0),
        ],
    ))
}

/// Turn the cards on the schedule, sound each as it comes up, glow the rare ones, and flash for a
/// Legendary. Real time: the menu's clock may be anything.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn animate_reveal(
    time: Res<Time<Real>>,
    mut state: ResMut<RevealState>,
    mut cards: Query<(
        &RevealCard,
        &mut UiTransform,
        &mut BoxShadow,
        &mut BorderColor,
    )>,
    mut backs: Query<(&CardBack, &mut Visibility), Without<CardFace>>,
    mut faces: Query<(&CardFace, &mut Visibility), Without<CardBack>>,
    mut flash: Query<
        (&mut BackgroundGradient, &mut Visibility),
        (With<Flash>, Without<CardBack>, Without<CardFace>),
    >,
    mut footer: Query<&mut Text, With<Footer>>,
    mut sounds: MessageWriter<GlueSound>,
) {
    let Some(reveal) = state.0.as_mut() else {
        return;
    };
    let dt = time.delta_secs().min(0.05);
    reveal.t += dt;
    let t = reveal.t;
    // Which cards come up this frame, and the best of them for a skip's one sound.
    let mut best: Option<usize> = None;
    let mut legendary = false;
    for (i, pull) in reveal.pulls.iter().enumerate() {
        let u = (t - reveal.starts[i]) / FLIP_SECS;
        if u >= 0.5 && !reveal.shown[i] {
            reveal.shown[i] = true;
            legendary |= pull.prize().rarity == Rarity::Legendary;
            if reveal.skipped {
                if best.is_none_or(|b| reveal.pulls[b].prize().rarity < pull.prize().rarity) {
                    best = Some(i);
                }
            } else {
                sounds.write(GlueSound(sound_for(pull)));
            }
        }
    }
    if let Some(b) = best {
        sounds.write(GlueSound(sound_for(&reveal.pulls[b])));
    }
    if legendary {
        reveal.flash = Some(0.0);
    }
    for (card, mut tf, mut shadow, mut border) in &mut cards {
        let i = card.0;
        let Some(pull) = reveal.pulls.get(i) else {
            continue;
        };
        let u = ((t - reveal.starts[i]) / FLIP_SECS).clamp(0.0, 1.0);
        // The turn: the card narrows to its edge and opens on the other side, lifting a little.
        let turn = (u * std::f32::consts::PI).cos().abs().max(0.02);
        let lift = 1.0 + 0.08 * (u * std::f32::consts::PI).sin();
        let want = Vec2::new(turn * lift, lift);
        if tf.scale != want {
            tf.scale = want;
        }
        let rarity = pull.prize().rarity;
        let rc = rarity.color();
        if reveal.shown[i] {
            if border.top != rc {
                *border = BorderColor::all(rc);
            }
            // The rare ones keep breathing.
            let base = match rarity {
                Rarity::Legendary => 0.95,
                Rarity::Epic => 0.8,
                Rarity::Rare => 0.6,
                Rarity::Uncommon => 0.35,
                Rarity::Common => 0.15,
            };
            let pulse = if rarity >= Rarity::Epic {
                0.15 * (t * 4.0).sin()
            } else {
                0.0
            };
            if let Some(s) = shadow.0.first_mut() {
                s.color = rc.with_alpha((base + pulse).clamp(0.0, 1.0));
                s.blur_radius = Val::Px(if rarity >= Rarity::Epic { 30.0 } else { 20.0 });
            }
        }
    }
    for (back, mut vis) in &mut backs {
        let up = reveal.shown.get(back.0).copied().unwrap_or(false);
        let want = if up {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (face, mut vis) in &mut faces {
        let up = reveal.shown.get(face.0).copied().unwrap_or(false);
        let want = if up {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    if let Some(ft) = reveal.flash.as_mut() {
        *ft += dt;
        let k = (*ft / FLASH_SECS).clamp(0.0, 1.0);
        let alpha = (1.0 - k) * (1.0 - k);
        for (mut g, mut vis) in &mut flash {
            *g = flash_gradient(alpha);
            *vis = if k < 1.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if k >= 1.0 {
            reveal.flash = None;
        }
    }
    let want = if reveal.done() {
        "Click or press Space to continue"
    } else {
        "Click or press Space to turn them all"
    };
    for mut text in &mut footer {
        if text.0 != want {
            text.0 = want.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{catalog, Rarity};
    use super::*;

    fn pull_of(rarity: Rarity, new: bool) -> Pull {
        let prize = catalog().iter().position(|p| p.rarity == rarity).unwrap();
        Pull {
            prize,
            new,
            favor: if new { 0 } else { rarity.favor() },
            featured: false,
        }
    }

    #[test]
    fn cards_turn_in_order_and_a_skip_turns_the_rest() {
        let pulls: Vec<Pull> = [
            Rarity::Common,
            Rarity::Legendary,
            Rarity::Uncommon,
            Rarity::Epic,
            Rarity::Rare,
        ]
        .into_iter()
        .map(|r| pull_of(r, true))
        .collect();
        let mut r = Reveal::new(pulls, "Scourge Week", 1);
        assert!(r.starts.windows(2).all(|w| w[0] < w[1]));
        // The Legendary holds the beat longer than a Common.
        let gap = |i: usize| r.starts[i + 1] - r.starts[i];
        assert!(gap(1) > gap(0) + LEGENDARY_BEAT - 1e-4);
        assert!(gap(3) > gap(2));
        assert!(!r.done());
        r.t = 1.0;
        r.skip();
        assert!(r.skipped);
        assert!(r.starts.iter().all(|&s| s <= r.t - FLIP_SECS));
    }

    #[test]
    fn each_card_sounds_its_rarity() {
        assert_eq!(sound_for(&pull_of(Rarity::Common, true)), SOUND_TURN);
        assert_eq!(sound_for(&pull_of(Rarity::Common, false)), SOUND_FAVOR);
        assert_eq!(sound_for(&pull_of(Rarity::Rare, false)), SOUND_RARE);
        assert_eq!(sound_for(&pull_of(Rarity::Epic, true)), SOUND_EPIC);
        assert_eq!(
            sound_for(&pull_of(Rarity::Legendary, true)),
            SOUND_LEGENDARY
        );
        let mut state = RevealState::default();
        state.start(vec![pull_of(Rarity::Rare, true)], "x");
        let first = state.0.as_ref().unwrap().serial;
        state.start(vec![pull_of(Rarity::Rare, true)], "x");
        assert_ne!(
            state.0.as_ref().unwrap().serial,
            first,
            "a new pull, a new overlay"
        );
    }
}
