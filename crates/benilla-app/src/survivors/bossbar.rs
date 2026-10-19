//! The boss frame: top centre under the clock, a roster boss's name, a wide health bar with its
//! health and percentage, the skull a far higher level shows, and "Final boss" on the roster's
//! last; a raid's finale (Ragnaros) gets a huge one. Up to [`MAX_FRAMES`] stack, the most recent
//! bosses; each fades out over [`FADE_SECS`]
//! once its boss falls. Elites and guardians keep their marks and get no frame. It stands clear
//! of the minimap and quest column (top right) and hides under the level-up cards.

use std::collections::HashMap;

use bevy::prelude::*;

use super::data;
use super::enemies::Enemy;
use super::hud::{self, ArtDone, HudFont, UiArt};
use super::{Phase, Selection};

/// Frames shown at once; more bosses alive show the most recent.
pub(super) const MAX_FRAMES: usize = 2;
/// Seconds a fallen boss's frame takes to fade.
pub(super) const FADE_SECS: f32 = 1.5;
/// The bar's size, in UI units.
const BAR_WIDTH: f32 = 400.0;
const BAR_HEIGHT: f32 = 20.0;
/// A finale's bar and name ([`HUGE_BOSSES`]).
const HUGE_WIDTH: f32 = 680.0;
const HUGE_HEIGHT: f32 = 34.0;
const NAME_SIZE: f32 = 17.0;
const HUGE_NAME_SIZE: f32 = 28.0;
/// The bosses whose frame is huge: a raid's finale.
const HUGE_BOSSES: &[&str] = &["Ragnaros"];
/// Below the clock and its line of counts.
const TOP: f32 = 78.0;
const BAR_RED: Color = Color::srgb(0.85, 0.08, 0.05);
const FINAL_TONE: Color = Color::srgb(1.0, 0.45, 0.1);
const BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);

/// A roster boss on the field, as the frame reads it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BossInfo {
    /// Its place in the battleground's roster, which is also the order bosses walk out in.
    pub order: usize,
    pub name: &'static str,
    pub hp: f32,
    pub max_hp: f32,
    /// Seconds since it fell; `None` while it stands.
    pub dead_for: Option<f32>,
}

/// One frame's contents.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BossFrame {
    pub name: &'static str,
    pub final_boss: bool,
    /// A raid finale's huge frame ([`HUGE_BOSSES`]).
    pub huge: bool,
    /// The bar's fill, 0 to 1.
    pub fill: f32,
    /// "current / max (percent%)".
    pub label: String,
    pub alpha: f32,
}

/// Health as a whole percentage: never 0 while standing, nor 100 once scratched.
pub(super) fn percent(hp: f32, max_hp: f32) -> u32 {
    if max_hp <= 0.0 || hp <= 0.0 {
        return 0;
    }
    let p = (hp / max_hp * 100.0).clamp(0.0, 100.0);
    if p >= 100.0 {
        100
    } else {
        (p.floor() as u32).clamp(1, 99)
    }
}

/// The frames for `bosses` on a roster of `roster_len`: standing bosses and those still fading,
/// the [`MAX_FRAMES`] most recent, in the order they walked out.
pub(super) fn frames(bosses: &[BossInfo], roster_len: usize) -> Vec<BossFrame> {
    let mut shown: Vec<&BossInfo> = bosses
        .iter()
        .filter(|b| b.dead_for.is_none_or(|t| t < FADE_SECS))
        .collect();
    shown.sort_by_key(|b| b.order);
    let skip = shown.len().saturating_sub(MAX_FRAMES);
    shown
        .into_iter()
        .skip(skip)
        .map(|b| {
            let hp = if b.dead_for.is_some() {
                0.0
            } else {
                b.hp.max(0.0)
            };
            let max = b.max_hp.max(1.0);
            let percent = percent(hp, max);
            BossFrame {
                name: b.name,
                final_boss: b.order + 1 == roster_len,
                huge: HUGE_BOSSES.contains(&b.name),
                fill: (hp / max).clamp(0.0, 1.0),
                label: format!("{} / {} ({percent}%)", hp.ceil() as u64, max.round() as u64),
                alpha: b
                    .dead_for
                    .map_or(1.0, |t| (1.0 - t / FADE_SECS).clamp(0.0, 1.0)),
            }
        })
        .collect()
}

#[derive(Component)]
struct BossBarRoot;
/// Frame `i`'s own root.
#[derive(Component)]
struct BossSlot(usize);
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Line {
    Name,
    Tag,
    Health,
}
#[derive(Component)]
struct BossText(usize, Line);
#[derive(Component)]
struct BossBack(usize);
#[derive(Component)]
struct BossFill(usize);
#[derive(Component)]
struct BossSkull(usize);

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup.after(hud::setup_hud))
        .add_systems(Update, refresh.after(hud::show_panels));
}

fn setup(mut commands: Commands, font: Res<HudFont>) {
    let f = &font.0;
    commands
        .spawn((
            BossBarRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(TOP),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            // Over the HUD, under the level-up cards, pause and results.
            GlobalZIndex(405),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            for i in 0..MAX_FRAMES {
                root.spawn((
                    BossSlot(i),
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(2.0),
                        display: Display::None,
                        ..default()
                    },
                    Visibility::Hidden,
                ))
                .with_children(|slot| {
                    // The name, the skull a far higher level shows, and the final boss's tag.
                    slot.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|head| {
                        head.spawn((
                            BossSkull(i),
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            ImageNode {
                                color: Color::NONE,
                                ..default()
                            },
                            UiArt::crop(
                                "Interface\\TargetingFrame\\UI-TargetingFrame-Skull",
                                [0.0, 1.0, 0.0, 1.0],
                            ),
                        ));
                        head.spawn((
                            BossText(i, Line::Name),
                            hud::text_bundle(f, "", NAME_SIZE, hud::GOLD_TONE),
                        ));
                        head.spawn((
                            BossText(i, Line::Tag),
                            hud::text_bundle(f, "", 13.0, FINAL_TONE),
                        ));
                    });
                    slot.spawn((
                        BossBack(i),
                        Node {
                            width: Val::Px(BAR_WIDTH),
                            height: Val::Px(BAR_HEIGHT),
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(BACKDROP),
                        BorderColor::all(hud::EDGE_TONE),
                    ))
                    .with_children(|bar| {
                        bar.spawn((
                            BossFill(i),
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            ImageNode {
                                color: Color::NONE,
                                ..default()
                            },
                            UiArt::tinted("Interface\\TargetingFrame\\UI-StatusBar", BAR_RED),
                        ));
                        bar.spawn((
                            BossText(i, Line::Health),
                            hud::centered_bundle(f, "", 13.0, Color::WHITE),
                        ));
                    });
                });
            }
        });
}

/// Seconds since each boss fell, kept while its corpse stands.
#[derive(Default)]
struct Fallen(HashMap<Entity, f32>);

#[allow(clippy::type_complexity)] // one Bevy system's full input set
fn refresh(
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    time: Res<Time>,
    enemies: Query<(Entity, &Enemy)>,
    mut fallen: Local<Fallen>,
    mut root: Query<&mut Visibility, (With<BossBarRoot>, Without<BossSlot>)>,
    mut slots: Query<(&BossSlot, &mut Node, &mut Visibility), Without<BossBarRoot>>,
    mut texts: Query<(&BossText, &mut Text, &mut TextColor, &mut TextFont)>,
    mut backs: Query<
        (&BossBack, &mut BackgroundColor, &mut BorderColor, &mut Node),
        (Without<BossFill>, Without<BossSlot>),
    >,
    mut fills: Query<(&BossFill, &mut Node, &mut ImageNode, Has<ArtDone>), Without<BossSlot>>,
    mut skulls: Query<(&BossSkull, &mut ImageNode, Has<ArtDone>), Without<BossFill>>,
) {
    let p = *phase.get();
    // In a run, and never over the level-up cards.
    let on = matches!(p, Phase::Playing | Phase::Paused);
    if let Ok(mut v) = root.single_mut() {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    if !on {
        if p == Phase::Menu {
            fallen.0.clear();
        }
        return;
    }
    let roster = &data::roster(selection.map).bosses;
    let dt = time.delta_secs();
    let mut bosses = Vec::new();
    for (e, enemy) in &enemies {
        if !enemy.boss {
            continue;
        }
        let Some(order) = roster.iter().position(|b| b.name == enemy.def.name) else {
            continue;
        };
        let dead_for = if enemy.dead {
            let t = fallen.0.entry(e).or_insert(0.0);
            *t += dt;
            Some(*t)
        } else {
            None
        };
        bosses.push(BossInfo {
            order,
            name: enemy.def.name,
            hp: enemy.hp,
            max_hp: enemy.max_hp,
            dead_for,
        });
    }
    fallen.0.retain(|e, _| enemies.contains(*e));
    let shown = frames(&bosses, roster.len());

    for (slot, mut node, mut vis) in &mut slots {
        let display = if shown.get(slot.0).is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
            *vis = if display == Display::Flex {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    let alpha = |i: usize| shown.get(i).map_or(0.0, |f| f.alpha);
    for (part, mut text, mut color, mut font) in &mut texts {
        let Some(frame) = shown.get(part.0) else {
            continue;
        };
        if part.1 == Line::Name {
            let size = if frame.huge {
                HUGE_NAME_SIZE
            } else {
                NAME_SIZE
            };
            if font.font_size != size {
                font.font_size = size;
            }
        }
        let (want, tone) = match part.1 {
            Line::Name => (frame.name.to_string(), hud::GOLD_TONE),
            Line::Tag => (
                if frame.final_boss {
                    "Final boss".to_string()
                } else {
                    String::new()
                },
                FINAL_TONE,
            ),
            Line::Health => (frame.label.clone(), Color::WHITE),
        };
        if **text != want {
            **text = want;
        }
        let tone = tone.with_alpha(frame.alpha);
        if color.0 != tone {
            color.0 = tone;
        }
    }
    for (back, mut bg, mut border, mut node) in &mut backs {
        let a = alpha(back.0);
        let huge = shown.get(back.0).is_some_and(|f| f.huge);
        let (w, h) = if huge {
            (HUGE_WIDTH, HUGE_HEIGHT)
        } else {
            (BAR_WIDTH, BAR_HEIGHT)
        };
        if node.width != Val::Px(w) || node.height != Val::Px(h) {
            node.width = Val::Px(w);
            node.height = Val::Px(h);
        }
        let want = BACKDROP.with_alpha(BACKDROP.alpha() * a);
        if bg.0 != want {
            bg.0 = want;
        }
        let edge = BorderColor::all(hud::EDGE_TONE.with_alpha(a));
        if *border != edge {
            *border = edge;
        }
    }
    for (fill, mut node, mut img, ready) in &mut fills {
        let Some(frame) = shown.get(fill.0) else {
            continue;
        };
        let width = Val::Percent(frame.fill * 100.0);
        if node.width != width {
            node.width = width;
        }
        if ready {
            let want = BAR_RED.with_alpha(frame.alpha);
            if img.color != want {
                img.color = want;
            }
        }
    }
    for (skull, mut img, ready) in &mut skulls {
        if ready {
            let want = Color::WHITE.with_alpha(alpha(skull.0));
            if img.color != want {
                img.color = want;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 5] = [
        "Hogger",
        "Gath'Ilzogg",
        "Morgaine",
        "Mor'Ladim",
        "Van Cleef",
    ];

    fn boss(order: usize, hp: f32, dead_for: Option<f32>) -> BossInfo {
        BossInfo {
            order,
            name: NAMES[order],
            hp,
            max_hp: 1000.0,
            dead_for,
        }
    }

    #[test]
    fn ragnaros_gets_the_huge_frame() {
        let rag = BossInfo {
            order: 5,
            name: "Ragnaros",
            hp: 10.0,
            max_hp: 20.0,
            dead_for: None,
        };
        let shown = frames(&[rag, boss(0, 900.0, None)], 6);
        assert!(shown
            .iter()
            .any(|f| f.name == "Ragnaros" && f.huge && f.final_boss));
        assert!(shown.iter().any(|f| f.name == NAMES[0] && !f.huge));
    }

    #[test]
    fn percent_is_whole_and_honest_at_the_ends() {
        assert_eq!(percent(1000.0, 1000.0), 100);
        assert_eq!(percent(999.0, 1000.0), 99, "scratched is not full");
        assert_eq!(percent(500.0, 1000.0), 50);
        assert_eq!(percent(1.0, 1000.0), 1, "standing is never 0%");
        assert_eq!(percent(0.0, 1000.0), 0);
        assert_eq!(percent(-5.0, 1000.0), 0);
        assert_eq!(percent(10.0, 0.0), 0);
    }

    #[test]
    fn frames_show_the_two_most_recent_bosses_in_order() {
        let shown = frames(
            &[
                boss(2, 400.0, None),
                boss(0, 900.0, None),
                boss(1, 1000.0, None),
            ],
            5,
        );
        let names: Vec<_> = shown.iter().map(|f| f.name).collect();
        assert_eq!(names, [NAMES[1], NAMES[2]]);
        assert_eq!(shown[0].label, "1000 / 1000 (100%)");
        assert_eq!(shown[1].label, "400 / 1000 (40%)");
        assert!((shown[1].fill - 0.4).abs() < 1e-6);
        assert!(shown.iter().all(|f| f.alpha == 1.0 && !f.final_boss));
    }

    #[test]
    fn the_last_roster_boss_is_tagged_final() {
        let shown = frames(&[boss(4, 1000.0, None)], 5);
        assert!(shown[0].final_boss);
        assert!(!frames(&[boss(3, 1000.0, None)], 5)[0].final_boss);
    }

    #[test]
    fn a_fallen_boss_fades_then_goes() {
        let shown = frames(&[boss(0, 120.0, Some(FADE_SECS / 2.0))], 3);
        assert_eq!(shown.len(), 1);
        assert!((shown[0].alpha - 0.5).abs() < 1e-6);
        assert_eq!(shown[0].fill, 0.0);
        assert_eq!(shown[0].label, "0 / 1000 (0%)");
        assert!(frames(&[boss(0, 0.0, Some(FADE_SECS))], 3).is_empty());
        assert!(frames(&[], 3).is_empty());
    }
}
