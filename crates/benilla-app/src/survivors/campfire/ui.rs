//! The camp's windows: the goblin's merchant window on the left, and on the right the ready check
//! ("Ready to move on?"), in the lobby's `UI-ReadyCheckFrame` with each player's mark, the
//! "Roll for the cache" button with the rolls, and the camp's personal note.

use bevy::prelude::*;

use super::super::hud::{HudFont, UiArt};
use super::super::party::{Bot, HeroState, LocalHero, NetRole, LOCAL_PLAYER};
use super::super::{Phase, Run};
use super::props::CampSpots;
use super::{Camp, CampAsk, CampNames, CampNote, CampRequest, VendorItem, COIN_SOUND};
use crate::glue::art::GlueArt;
use crate::glue::widgets::{glue_button, GlueBtnKind, GlueDisabled};
use crate::glue::GlueClicks;
use crate::sound::SoundCue;

/// The ready check's frame, cropped as `ReadyCheckFrame` draws it (the lobby's, [`READY_FRAME`]).
pub(in crate::survivors) const READY_FRAME: &str = "Interface\\RaidFrame\\UI-ReadyCheckFrame";
const READY_FRAME_TC: [f32; 4] = [0.0, 0.630_859_4, 0.0, 0.757_812_5];
/// The lobby's marks: the check box's tick for ready, the pocket watch for waiting.
pub(in crate::survivors) const READY_ICON: &str = "Interface\\Buttons\\UI-CheckBox-Check";
pub(in crate::survivors) const WAITING_ICON: &str = "Interface\\Icons\\INV_Misc_PocketWatch_01";
const WAITING_TC: [f32; 4] = [0.07, 0.93, 0.07, 0.93];
/// The ready check's portrait: a campfire's.
pub(in crate::survivors) const FIRE_ICON: &str = "Interface\\Icons\\Spell_Fire_Fire";
/// An item slot's frame, and the gold coin of `MoneyFrame`'s icons.
pub(in crate::survivors) const SLOT_FRAME: &str = "Interface\\Buttons\\UI-Quickslot2";
pub(in crate::survivors) const MONEY_ICONS: &str = "Interface\\MoneyFrame\\UI-MoneyIcons";
const GOLD_TC: [f32; 4] = [0.0, 0.25, 0.0, 1.0];
/// The fishing line's icon in the hint.
pub(in crate::survivors) const FISHING_ICON: &str = "Interface\\Icons\\Trade_Fishing";
/// How far the hero may wander from the goblin before his window shuts, in yards.
const MERCHANT_LEAVE: f32 = 10.0;

const GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
const GREY: Color = Color::srgb(0.75, 0.75, 0.75);
const SYSTEM: Color = Color::srgb(1.0, 1.0, 0.0);
const PANEL: Color = Color::srgba(0.0, 0.0, 0.0, 0.82);
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const ROW_HOVER: Color = Color::srgba(1.0, 0.82, 0.0, 0.18);

/// Whether the goblin's window is open.
#[derive(Resource, Default)]
pub(super) struct MerchantWindow {
    pub(super) open: bool,
}

#[derive(Component)]
struct MerchantRoot;
#[derive(Component)]
struct PurseText;
#[derive(Component, Clone, Copy)]
struct MerchantRow(VendorItem);
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum CampButton {
    Ready,
    Roll,
    CloseMerchant,
}
#[derive(Component)]
struct CampRoot;
#[derive(Component)]
struct ReadyTitle;
#[derive(Component)]
struct ReadyMarks;
#[derive(Component)]
struct RollText;
#[derive(Component)]
struct NoteText;
#[derive(Component)]
struct HintText;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MerchantWindow>().add_systems(
        Update,
        (
            build,
            camp_clicks,
            merchant_clicks,
            refresh_merchant,
            refresh_camp,
        )
            .chain(),
    );
}

fn label(font: &Handle<Font>, text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow::default(),
    )
}

fn art(path: &'static str, coords: [f32; 4], size: f32) -> impl Bundle {
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        ImageNode {
            color: Color::NONE,
            ..default()
        },
        UiArt::crop(path, coords),
    )
}

/// The ready check's line for `left` seconds of camp.
pub(super) fn ready_title(left: f32) -> String {
    format!("Ready to move on? ({}s)", left.ceil().max(0.0) as u32)
}

/// A roll as the chat says it: "[Name] rolls 73 (1-100)", or "You roll" alone.
pub(super) fn roll_line(names: &CampNames, player: u8, value: u8) -> String {
    if player == LOCAL_PLAYER && !names.0.iter().any(|(p, _)| *p == player) {
        format!("You roll {value} (1-100)")
    } else {
        format!("[{}] rolls {value} (1-100)", names.of(player))
    }
}

/// The players the camp waits on and whether each is ready, in player order.
pub(super) fn marks(players: &[u8], ready: &[u8]) -> Vec<(u8, bool)> {
    let mut out: Vec<(u8, bool)> = players.iter().map(|&p| (p, ready.contains(&p))).collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Both windows, built once (hidden) as soon as the HUD's font and the glue art are up.
fn build(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    glue: Option<Res<GlueArt>>,
    built: Query<(), With<CampRoot>>,
) {
    let (Some(font), Some(glue)) = (font, glue) else {
        return;
    };
    if !built.is_empty() {
        return;
    }
    let f = &font.0;
    let px = Val::Px;
    commands
        .spawn((
            MerchantRoot,
            Node {
                position_type: PositionType::Absolute,
                left: px(16.0),
                top: Val::Percent(36.0),
                width: px(290.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(8.0)),
                row_gap: px(4.0),
                border: UiRect::all(px(2.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(EDGE),
            GlobalZIndex(330),
        ))
        .with_children(|w| {
            w.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|t| {
                t.spawn(label(f, "Goblin Merchant", 16.0, GOLD));
                glue_button(
                    t,
                    &glue,
                    f,
                    CampButton::CloseMerchant,
                    "Close",
                    64.0,
                    22.0,
                    GlueBtnKind::Small,
                    1.0,
                );
            });
            for item in VendorItem::ALL {
                w.spawn((
                    MerchantRow(item),
                    Button,
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(8.0),
                        padding: UiRect::all(px(3.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|r| {
                    r.spawn(Node {
                        width: px(40.0),
                        height: px(40.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|slot| {
                        slot.spawn(art(item.icon(), [0.0, 1.0, 0.0, 1.0], 34.0));
                        slot.spawn((
                            art(SLOT_FRAME, [0.0, 1.0, 0.0, 1.0], 64.0),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(-12.0),
                                top: px(-12.0),
                                width: px(64.0),
                                height: px(64.0),
                                ..default()
                            },
                        ));
                    });
                    r.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        ..default()
                    })
                    .with_children(|c| {
                        c.spawn(label(f, item.name(), 13.0, Color::WHITE));
                        c.spawn(label(f, item.blurb(), 11.0, GREY));
                    });
                    r.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(3.0),
                        ..default()
                    })
                    .with_children(|p| {
                        p.spawn(label(f, item.price().to_string(), 13.0, Color::WHITE));
                        p.spawn(art(MONEY_ICONS, GOLD_TC, 13.0));
                    });
                });
            }
            w.spawn((PurseText, label(f, "", 12.0, GOLD)));
        });
    commands
        .spawn((
            CampRoot,
            Node {
                position_type: PositionType::Absolute,
                right: px(16.0),
                top: Val::Percent(34.0),
                width: px(323.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(6.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(330),
        ))
        .with_children(|root| {
            // The lobby's ready-check pop-up, its portrait the fire's.
            root.spawn(Node {
                width: px(323.0),
                min_height: px(97.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::new(px(70.0), px(14.0), px(8.0), px(8.0)),
                row_gap: px(5.0),
                ..default()
            })
            .with_children(|p| {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0.0),
                        top: px(0.0),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    UiArt::crop(READY_FRAME, READY_FRAME_TC),
                ));
                p.spawn((
                    art(FIRE_ICON, [0.07, 0.93, 0.07, 0.93], 50.0),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(10.0),
                        top: px(9.0),
                        width: px(50.0),
                        height: px(50.0),
                        ..default()
                    },
                ));
                p.spawn((ReadyTitle, label(f, "", 13.0, Color::WHITE)));
                p.spawn((
                    ReadyMarks,
                    Node {
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        column_gap: px(8.0),
                        row_gap: px(2.0),
                        ..default()
                    },
                ));
                glue_button(
                    p,
                    &glue,
                    f,
                    CampButton::Ready,
                    "Ready",
                    100.0,
                    26.0,
                    GlueBtnKind::Dialog,
                    1.0,
                );
            });
            glue_button(
                root,
                &glue,
                f,
                CampButton::Roll,
                "Roll for the cache",
                190.0,
                30.0,
                GlueBtnKind::Dialog,
                1.0,
            );
            root.spawn((RollText, label(f, "", 12.0, SYSTEM)));
            root.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: px(4.0),
                ..default()
            })
            .with_children(|h| {
                h.spawn(art(FISHING_ICON, [0.07, 0.93, 0.07, 0.93], 16.0));
                h.spawn((HintText, label(f, "", 12.0, GREY)));
            });
            root.spawn((NoteText, label(f, "", 13.0, GOLD)));
        });
}

/// Ready and the roll go out as the local player's asks.
fn camp_clicks(
    clicks: Res<GlueClicks>,
    buttons: Query<(Entity, &CampButton, Option<&GlueDisabled>)>,
    mut window: ResMut<MerchantWindow>,
    mut out: MessageWriter<CampRequest>,
) {
    for (e, button, disabled) in &buttons {
        if !clicks.hit(e) || disabled.is_some_and(|d| d.0) {
            continue;
        }
        match button {
            CampButton::Ready => {
                out.write(CampRequest(CampAsk::Ready));
            }
            CampButton::Roll => {
                out.write(CampRequest(CampAsk::Roll));
            }
            CampButton::CloseMerchant => window.open = false,
        }
    }
}

/// A row clicked buys its item: told at once when the purse cannot pay; a co-op client hears its
/// coins as it asks, the host's own purchase as it lands ([`super::apply_local`]).
#[allow(clippy::too_many_arguments)]
fn merchant_clicks(
    clicks: Res<GlueClicks>,
    rows: Query<(Entity, &MerchantRow)>,
    window: Res<MerchantWindow>,
    run: Res<Run>,
    role: Res<NetRole>,
    mut note: ResMut<CampNote>,
    mut out: MessageWriter<CampRequest>,
    mut cues: MessageWriter<SoundCue>,
) {
    if !window.open {
        return;
    }
    for (e, row) in &rows {
        if !clicks.hit(e) {
            continue;
        }
        let item = row.0;
        if run.gold < item.price() {
            note.0 = super::note_of(super::Outcome::Broke).map(|t| (t, 3.0));
            continue;
        }
        out.write(CampRequest(CampAsk::Buy(item)));
        if *role == NetRole::Client {
            cues.write(SoundCue::flat(COIN_SOUND));
            note.0 = super::note_of(super::Outcome::Bought(item)).map(|t| (t, 3.0));
        }
    }
}

/// The merchant window shows while it is open in a camp, and shuts as the hero walks away.
#[allow(clippy::type_complexity)]
fn refresh_merchant(
    phase: Res<State<Phase>>,
    run: Res<Run>,
    camp: Res<Camp>,
    spots: Res<CampSpots>,
    hero: Query<&Transform, With<LocalHero>>,
    mut window: ResMut<MerchantWindow>,
    mut roots: Query<&mut Node, With<MerchantRoot>>,
    mut purse: Query<&mut Text, With<PurseText>>,
    mut rows: Query<(&Interaction, &mut BackgroundColor), With<MerchantRow>>,
) {
    let away = match (hero.single().ok(), spots.vendor) {
        (Some(tf), Some(at)) => {
            super::super::units::flat(tf.translation - at).length() > MERCHANT_LEAVE
        }
        _ => true,
    };
    if window.open && (!camp.active(&run) || away) {
        window.open = false;
    }
    let shown = window.open && *phase.get() == Phase::Playing;
    let display = if shown { Display::Flex } else { Display::None };
    for mut node in &mut roots {
        if node.display != display {
            node.display = display;
        }
    }
    if !shown {
        return;
    }
    let line = format!("Party purse: {} gold", run.gold);
    for mut t in &mut purse {
        if t.0 != line {
            t.0 = line.clone();
        }
    }
    for (interaction, mut bg) in &mut rows {
        let want = if *interaction == Interaction::None {
            Color::NONE
        } else {
            ROW_HOVER
        };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}

/// The ready check, the rolls, the hint and the note, while a camp lasts.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn refresh_camp(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    camp: Res<Camp>,
    names: Res<CampNames>,
    note: Res<CampNote>,
    heroes: Query<&HeroState, Without<Bot>>,
    mut roots: Query<&mut Node, With<CampRoot>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<ReadyTitle>>,
        Query<&mut Text, With<RollText>>,
        Query<&mut Text, With<NoteText>>,
        Query<&mut Text, With<HintText>>,
    )>,
    marks_box: Query<(Entity, Option<&Children>), With<ReadyMarks>>,
    mut buttons: Query<(&CampButton, &mut GlueDisabled)>,
    mut shown_marks: Local<Vec<(u8, bool, String)>>,
) {
    let shown = camp.active(&run) && *phase.get() == Phase::Playing;
    let display = if shown { Display::Flex } else { Display::None };
    for mut node in &mut roots {
        if node.display != display {
            node.display = display;
        }
    }
    if !shown {
        shown_marks.clear();
        return;
    }
    let me = LOCAL_PLAYER;
    let title = ready_title(run.breather);
    for mut t in &mut texts.p0() {
        if t.0 != title {
            t.0 = title.clone();
        }
    }
    let rolls: Vec<String> = camp
        .roll
        .iter()
        .map(|&(p, v)| roll_line(&names, p, v))
        .collect();
    let rolls = rolls.join("\n");
    for mut t in &mut texts.p1() {
        if t.0 != rolls {
            t.0 = rolls.clone();
        }
    }
    let line = note.0.as_ref().map(|n| n.0.clone()).unwrap_or_default();
    for mut t in &mut texts.p2() {
        if t.0 != line {
            t.0 = line.clone();
        }
    }
    let hint = if camp.fished.contains(&me) {
        "Well Fed. The goblin by the fire is selling.".to_string()
    } else {
        "Click the bobber when it dips to fish.".to_string()
    };
    for mut t in &mut texts.p3() {
        if t.0 != hint {
            t.0 = hint.clone();
        }
    }
    for (button, mut disabled) in &mut buttons {
        let want = match button {
            CampButton::Ready => camp.ready.contains(&me),
            CampButton::Roll => camp.has_rolled(me),
            CampButton::CloseMerchant => false,
        };
        if disabled.0 != want {
            disabled.0 = want;
        }
    }
    // Each player's mark, rebuilt only when one changes.
    let players: Vec<u8> = heroes.iter().map(|h| h.player).collect();
    let want: Vec<(u8, bool, String)> = marks(&players, &camp.ready)
        .into_iter()
        .map(|(p, ready)| (p, ready, names.of(p)))
        .collect();
    let (Some(font), Ok((holder, children))) = (font, marks_box.single()) else {
        return;
    };
    if *shown_marks == want {
        return;
    }
    if let Some(children) = children {
        for c in children.iter() {
            commands.entity(c).despawn();
        }
    }
    commands.entity(holder).with_children(|row| {
        for (_, ready, name) in &want {
            row.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(3.0),
                ..default()
            })
            .with_children(|chip| {
                if *ready {
                    chip.spawn(art(READY_ICON, [0.0, 1.0, 0.0, 1.0], 16.0));
                } else {
                    chip.spawn(art(WAITING_ICON, WAITING_TC, 14.0));
                }
                let color = if *ready { Color::WHITE } else { GREY };
                chip.spawn(label(&font.0, name.clone(), 12.0, color));
            });
        }
    });
    *shown_marks = want;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ready_check_counts_down_in_whole_seconds() {
        assert_eq!(ready_title(41.2), "Ready to move on? (42s)");
        assert_eq!(ready_title(0.0), "Ready to move on? (0s)");
    }

    #[test]
    fn the_ready_check_marks_each_player_once() {
        assert_eq!(
            marks(&[2, 0, 1, 2], &[1]),
            vec![(0, false), (1, true), (2, false)]
        );
    }

    #[test]
    fn rolls_read_as_the_chat_says_them() {
        let alone = CampNames::default();
        assert_eq!(roll_line(&alone, LOCAL_PLAYER, 73), "You roll 73 (1-100)");
        let party = CampNames(vec![(0, "Dean".into()), (1, "Thrall".into())]);
        assert_eq!(roll_line(&party, 0, 73), "[Dean] rolls 73 (1-100)");
        assert_eq!(roll_line(&party, 1, 5), "[Thrall] rolls 5 (1-100)");
    }
}
