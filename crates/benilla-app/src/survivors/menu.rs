//! The run's pick screen, laid out like the 1.12 character create and select glue screens and built
//! from their own widgets and art ([`crate::glue`]): the race, gender and class tower on the left,
//! the zone list in the character-select frame on the right, Enter World at the bottom. Beside the
//! tower, the picked race's traits ([`super::racials`]); beside the list, the zone card. A zone not
//! yet open ([`super::zones`]) is dimmed in the list and cannot be picked or entered.
//!
//! The survivors HUD lays out on the 768-unit canvas through `UiScale`, so every glue builder here
//! takes a unit scale.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use bevy::ui_render::ui_material::MaterialNode;
use bevy::window::PrimaryWindow;

use super::hud::{looks_for, rgb, start_run, HudFont, MenuRoot};
use super::profile::Profile;
use super::run_options::GREEN;
use super::zones::{self, Unlocks};
use super::{data, racials, HeroState, LocalHero, MetaBonuses, Phase, Rng, Run, Selection};
use crate::char_select::race_name;
use crate::glue::art::{class_tc, race_tc, tc_rect, GlueArt, DIM, GOLD, NAME_EDGE};
use crate::glue::backdrop::{backdrop_border, tiled_bg_node};
use crate::glue::widgets::{
    abs, glue_button, icon_button, outlined_text, GlueBtnKind, GlueDisabled, GlueText, Hilight,
    HoverLabel, LockHighlight,
};
use crate::glue::GlueClicks;

/// The create screen's two race columns.
const ALLIANCE: [u8; 4] = [1, 3, 4, 7];
const HORDE: [u8; 4] = [2, 5, 6, 8];

/// `CharacterSelectCharacterFrame`'s border and fill tints.
const FRAME_BORDER: Color = Color::srgb(0.8, 0.8, 0.8);
const FRAME_FILL: Color = Color::srgba(0.09, 0.09, 0.09, 0.85);

/// The tower ends under the class grid and Randomize, where the create screen's appearance dials
/// would start: there is nothing to customise here.
const TOWER_H: f32 = 590.0;

/// The bottom bar across the screen, on the 768-unit canvas: the utility buttons left and right,
/// Enter World and Run Options in the middle. Every utility button is one size.
pub(super) const BAR_H: f32 = 90.0;
pub(super) const BAR_BTN_W: f32 = 116.0;
pub(super) const BAR_BTN_H: f32 = 32.0;
pub(super) const BAR_GAP: f32 = 8.0;
/// How far the utility buttons sit below the bar's top edge, centring them on Enter World.
pub(super) const BAR_BTN_DROP: f32 = 8.0;
const ENTER_W: f32 = 210.0;
const ENTER_H: f32 = 48.0;

/// The zone list's frame, at TOPRIGHT (-5,-15), ending above the bar.
const FRAME_TOP: f32 = 15.0;
const FRAME_H: f32 = 768.0 - BAR_H - FRAME_TOP - 10.0;
/// The scrolled list inside the frame, under its title.
const LIST_TOP: f32 = 40.0;
/// The zone rows: the character-select row, compacted.
const ROW_GAP: f32 = 2.0;
const ROW_H: f32 = 34.0;
const ROW_W: f32 = 216.0;
/// The rows' inset in the list, room for the highlight's glow on their left.
const ROW_INDENT: f32 = 20.0;
/// A section header's colour: `HIGHLIGHT_FONT_COLOR`.
const HEADER: Color = Color::srgb(1.0, 1.0, 1.0);
/// The raid whose stages the list groups under their own header.
const MOLTEN_CORE: &str = "Molten Core";

/// A row's pips: the won marker, then [`zones::PIPS`] difficulty pips, at the end of its name.
const MARKER_W: f32 = 7.0;
const PIP_W: f32 = 5.0;
const PIP_GAP: f32 = 2.0;
const PIP_OFF: Color = Color::srgba(0.12, 0.12, 0.12, 0.9);
const PIP_ON: Color = Color::srgb(1.0, 0.45, 0.1);
/// `RED_FONT_COLOR`: what opens a locked zone.
const LOCKED: Color = Color::srgb(1.0, 0.1, 0.1);

/// The cards: tooltip boxes that size to their text.
const CARD_PAD: f32 = 12.0;
/// The zone card, left of the list (whose border starts 265 in) and level with its top.
const ZONE_CARD_RIGHT: f32 = 272.0;
const ZONE_CARD_TOP: f32 = 19.0;
const ZONE_CARD_W: f32 = 250.0;
/// The race card, right of the tower's banners and level with the race columns.
const RACE_CARD_LEFT: f32 = 262.0;
const RACE_CARD_TOP: f32 = 142.0;
const RACE_CARD_W: f32 = 210.0;
/// The final boss's line, and the foe list.
const FINAL_BOSS: Color = Color::srgb(1.0, 0.5, 0.25);
const FOES: Color = Color::srgb(0.8, 0.8, 0.8);

/// The fixed, top-to-bottom order of the character-select utility stack.
#[cfg(test)]
const SECONDARY_BUTTONS: [&str; 6] = [
    "Customize",
    "Armory",
    "Darkmoon Faire",
    "Multiplayer",
    "Options",
    "Quit",
];

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MenuAction {
    Race(u8),
    Sex(u8),
    Class(usize),
    Map(usize),
    Randomize,
    Customize,
    Enter,
    Quit,
}

/// On [`MenuRoot`] once the glue art is in and the screen is built.
#[derive(Component)]
pub(super) struct MenuBuilt;

/// The bottom bar's left group of utility buttons. Feature modules receive a slot, rather than
/// appending themselves in plugin order.
#[derive(Component)]
pub(super) struct MenuButtons;

#[derive(Component)]
pub(super) struct MenuPrimary;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuSlot {
    Customize,
    Armory,
    Faire,
    Multiplayer,
    Options,
}

/// The menu's slim profile readout.
#[derive(Component)]
struct MenuResources;
#[derive(Component)]
struct MenuResourceText;

/// A race button's face, re-cropped to the picked gender.
#[derive(Component)]
pub(super) struct RaceFace(u8);

#[derive(Component)]
pub(super) struct PickName;

/// A zone row's name and its second line (the level band, or what opens it), by map index.
#[derive(Component)]
pub(super) struct ZoneRowName(usize);
#[derive(Component)]
pub(super) struct ZoneRowLine(usize);

/// The zone list's clipped viewport, under the mouse wheel.
#[derive(Component)]
pub(super) struct ZoneScroll;

/// The zone list's body inside [`ZoneScroll`], moved up by its `top` as it scrolls.
#[derive(Component)]
pub(super) struct ZoneScrollBody;

/// A zone row's pips, shown while the zone is open.
#[derive(Component)]
pub(super) struct ZonePips(usize);

/// One pip: slot 0 is the won marker, 1 to [`zones::PIPS`] the difficulty.
#[derive(Component)]
pub(super) struct ZonePip {
    map: usize,
    slot: u8,
}

/// A line of the zone card.
#[derive(Component, Clone, Copy)]
pub(super) enum ZoneCardText {
    Name,
    /// The tier and the level band.
    Kind,
    /// The best win, "Not yet won", or what opens the zone.
    Best,
    /// The `n`th boss on the schedule.
    Boss(usize),
    Foes,
}

/// The race card, shown once a race is picked.
#[derive(Component)]
pub(super) struct RaceCard;

#[derive(Component, Clone, Copy)]
pub(super) enum RaceCardText {
    Title,
    /// The `n`th trait's name, and what it grants.
    Name(usize),
    Effect(usize),
}

/// The menu's own refreshes beyond [`refresh_menu_ui`]: the zone rows, the zone card and the race
/// card.
pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            refresh_zone_rows,
            refresh_zone_card,
            refresh_race_card,
            refresh_resources,
            scroll_zone_list,
        )
            .after(menu_clicks)
            .before(crate::glue::GlueVisuals),
    );
}

/// The list's offset after `lines` wheel notches (up positive, as Bevy reports them): a row a
/// notch, held between the top (0) and `max`.
fn scrolled(now: f32, lines: f32, max: f32) -> f32 {
    (now - lines * (ROW_H + ROW_GAP)).clamp(0.0, max.max(0.0))
}

/// Wheel-scroll the zone list while the cursor is over it: a notch a row, held between the top
/// and the last row. The offset is the body's `top`, kept in range as the body's height settles.
#[allow(clippy::type_complexity)]
fn scroll_zone_list(
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    lists: Query<(&ComputedNode, &UiGlobalTransform, &Children), With<ZoneScroll>>,
    mut bodies: Query<(&ComputedNode, &mut Node), With<ZoneScrollBody>>,
) {
    let lines: f32 = wheel
        .read()
        .map(|ev| crate::bindings::wheel_lines(ev.unit, ev.y))
        .sum();
    let cursor = windows
        .single()
        .ok()
        .and_then(Window::physical_cursor_position);
    for (view, transform, children) in &lists {
        let over = cursor.is_some_and(|c| view.contains_point(*transform, c));
        for &child in children {
            let Ok((body, mut node)) = bodies.get_mut(child) else {
                continue;
            };
            let max = ((body.size.y - view.size.y) * view.inverse_scale_factor).max(0.0);
            let now = match node.top {
                Val::Px(top) => -top,
                _ => 0.0,
            };
            let want = scrolled(now, if over { lines } else { 0.0 }, max);
            if want != now {
                node.top = Val::Px(-want);
            }
        }
    }
}

/// The saved pick, `benilla-config/survivors-pick.txt` (`localStorage` on the page):
/// `class race sex map`, restored at the menu so a reload keeps the last hero and zone.
fn pick_path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|h| h.join("survivors-pick.txt"))
}

fn save_pick(sel: &Selection) {
    let Some(text) = pick_text(sel) else {
        return;
    };
    if let Some(path) = pick_path() {
        let _ = crate::local_state::write_atomic(&path, &text);
    }
}

/// The pick as the file holds it, `class race sex map`; `None` before a class and look.
fn pick_text(sel: &Selection) -> Option<String> {
    let class = sel.class?;
    let &(_, race, sex) = sel.look.and_then(|i| sel.looks.get(i))?;
    Some(format!("{class} {race} {sex} {}", sel.map))
}

/// Restore the saved pick once the creature data can resolve its looks; a stale or missing file
/// leaves the default. A zone not yet open, saved or default, gives way to the first open one.
pub(super) fn restore_pick(
    creatures: Option<Res<crate::entities::Creatures>>,
    unlocks: Res<Unlocks>,
    mut selection: ResMut<Selection>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let Some(creatures) = creatures.as_deref() else {
        return;
    };
    *done = true;
    if let Some(text) = pick_path().and_then(|p| crate::local_state::read_to_string(&p).ok()) {
        apply_pick(&text, creatures, &mut selection);
    }
    if selection.class.is_none() {
        default_pick(creatures, &mut selection);
    }
    zones::open_pick(&mut selection, &unlocks);
}

/// A fresh profile's hero: the tower's first race in the first class it has, so the menu always
/// shows one. Not saved: the file keeps what the player picks.
fn default_pick(creatures: &crate::entities::Creatures, selection: &mut Selection) {
    let all = all_looks(creatures);
    for race in ALLIANCE.into_iter().chain(HORDE) {
        for (class, looks) in all.iter().enumerate() {
            let look = find(looks, Some(race), Some(0)).or_else(|| find(looks, Some(race), None));
            if let Some(l) = look {
                set(selection, &all, class, l);
                return;
            }
        }
    }
}

/// The saved `class race sex map`, as far as it still resolves. A file that is not four numbers
/// (a hand edit, a byte-order mark) restores nothing rather than shifting the fields. A look the
/// class no longer offers keeps the race in the other sex, as a class click does.
fn apply_pick(text: &str, creatures: &crate::entities::Creatures, selection: &mut Selection) {
    let fields: Option<Vec<usize>> = text
        .trim_start_matches('\u{feff}')
        .split_whitespace()
        .map(|t| t.parse().ok())
        .collect();
    let Some(&[class, race, sex, map]) = fields.as_deref() else {
        return;
    };
    let (Ok(race), Ok(sex)) = (u8::try_from(race), u8::try_from(sex)) else {
        return;
    };
    if map < data::MAPS.len() {
        selection.map = map;
        selection.stage = map;
        selection.version += 1;
    }
    if class < data::CLASSES.len() {
        let all = all_looks(creatures);
        let looks = &all[class];
        let look = find(looks, Some(race), Some(sex))
            .or_else(|| find(looks, Some(race), None))
            .or_else(|| find(looks, None, Some(sex)))
            .or_else(|| find(looks, None, None));
        if let Some(l) = look {
            set(selection, &all, class, l);
        }
    }
}

/// Every class's looks, `(display, race, sex)`, indexed like [`data::CLASSES`].
pub(super) fn all_looks(creatures: &crate::entities::Creatures) -> Vec<Vec<(u32, u8, u8)>> {
    data::CLASSES
        .iter()
        .map(|c| looks_for(c, creatures))
        .collect()
}

pub(super) fn find(looks: &[(u32, u8, u8)], race: Option<u8>, sex: Option<u8>) -> Option<usize> {
    looks
        .iter()
        .position(|&(_, r, s)| race.is_none_or(|x| x == r) && sex.is_none_or(|x| x == s))
}

pub(super) fn set(sel: &mut Selection, all: &[Vec<(u32, u8, u8)>], class: usize, look: usize) {
    sel.class = Some(class);
    sel.looks = all[class].clone();
    sel.look = Some(look);
    sel.version += 1;
}

/// Build the screen into [`MenuRoot`] once the archives are readable, so the glue art is there.
pub(super) fn build_menu(
    mut commands: Commands,
    roots: Query<Entity, (With<MenuRoot>, Without<MenuBuilt>)>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut art: ResMut<GlueArt>,
    mut images: ResMut<Assets<Image>>,
    mut add_mats: ResMut<Assets<crate::glue::add_material::AddUiMaterial>>,
    font: Res<HudFont>,
) {
    let Ok(root) = roots.single() else { return };
    let Some(mut assets) = assets else { return };
    art.ensure_loaded(&mut assets, &mut images, &mut add_mats);
    let f = &font.0;
    commands.entity(root).insert(MenuBuilt).with_children(|ui| {
        // The outer screen is a column: a slim account bar, then the familiar select screen.
        // The screen's art still owns its detailed tower and zone-row placement.
        resource_bar(ui, f);
        tower(ui, &art, f);
        // `CharacterCreateWoWLogo`, 256×128 at TOPLEFT (3,-7), after the tower so it draws over
        // the border art, as the create screen orders it.
        if let Some(logo) = &art.logo {
            ui.spawn((
                ImageNode::new(logo.clone()),
                abs(1.0, 3.0, 7.0, 256.0, 128.0),
            ));
        }
        zone_frame(ui, &art, f);
        zone_card(ui, &art, f);
        race_card(ui, &art, f);

        // `CharSelectCharacterName`, GlueFontNormalHuge, just above the bar.
        ui.spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(BAR_H + 14.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|c| {
            outlined_text(
                c,
                Node::default(),
                (),
                PickName,
                GlueText {
                    text: "",
                    size: 22.0,
                    color: GOLD,
                    wrap: false,
                },
                f,
                1.0,
            );
        });
        bottom_bar(ui, &art, f);
    });
}

/// A utility button's column in the bar: the button, then any caption lines under it.
fn bar_slot() -> Node {
    Node {
        width: Val::Px(BAR_BTN_W),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A side group of the bar. The two sides share the width left over, so Enter World stays
/// centred between them.
fn bar_side(justify: JustifyContent) -> Node {
    Node {
        flex_grow: 1.0,
        flex_basis: Val::Px(0.0),
        flex_direction: FlexDirection::Row,
        justify_content: justify,
        align_items: AlignItems::FlexStart,
        column_gap: Val::Px(BAR_GAP),
        padding: UiRect::top(Val::Px(BAR_BTN_DROP)),
        ..default()
    }
}

/// The bottom bar, as the character-select screen's: Customize, Armory, the Faire and
/// Multiplayer on the left; Enter World and Run Options in the middle; Options and Quit on the
/// right. The feature modules fill their [`MenuSlot`]s and [`MenuPrimary`].
fn bottom_bar(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        left: px(4.0),
        right: px(4.0),
        bottom: px(2.0),
        height: px(BAR_H),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::FlexStart,
        column_gap: px(24.0),
        padding: UiRect {
            left: px(20.0),
            right: px(20.0),
            top: px(10.0),
            bottom: px(6.0),
        },
        ..default()
    })
    .with_children(|bar| {
        card_backdrop(bar, art);
        bar.spawn((MenuButtons, bar_side(JustifyContent::FlexStart)))
            .with_children(|side| {
                side.spawn((MenuSlot::Customize, bar_slot()))
                    .with_children(|slot| {
                        glue_button(
                            slot,
                            art,
                            f,
                            MenuAction::Customize,
                            "Customize",
                            BAR_BTN_W,
                            BAR_BTN_H,
                            GlueBtnKind::Small,
                            1.0,
                        );
                    });
                side.spawn((MenuSlot::Armory, bar_slot()))
                    .with_children(|slot| super::armory::entry_button(slot, art, f));
                for slot in [MenuSlot::Faire, MenuSlot::Multiplayer] {
                    side.spawn((slot, bar_slot()));
                }
            });
        bar.spawn((
            MenuPrimary,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                column_gap: px(BAR_GAP),
                ..default()
            },
        ))
        .with_children(|primary| {
            glue_button(
                primary,
                art,
                f,
                MenuAction::Enter,
                "Enter World",
                ENTER_W,
                ENTER_H,
                GlueBtnKind::Normal,
                1.0,
            );
        });
        bar.spawn(bar_side(JustifyContent::FlexEnd))
            .with_children(|side| {
                side.spawn((MenuSlot::Options, bar_slot()));
                side.spawn(bar_slot()).with_children(|slot| {
                    glue_button(
                        slot,
                        art,
                        f,
                        MenuAction::Quit,
                        "Quit",
                        BAR_BTN_W,
                        BAR_BTN_H,
                        GlueBtnKind::Small,
                        1.0,
                    );
                });
            });
    });
}

fn resource_bar(ui: &mut ChildSpawnerCommands, f: &Handle<Font>) {
    ui.spawn((
        MenuResources,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
    ))
    .with_children(|bar| {
        let text = outlined_text(
            bar,
            Node::default(),
            (),
            (),
            GlueText {
                text: "",
                size: 14.0,
                color: GOLD,
                wrap: false,
            },
            f,
            1.0,
        );
        bar.commands().entity(text).insert(MenuResourceText);
    });
}

fn refresh_resources(
    profile: Option<Res<Profile>>,
    selection: Res<Selection>,
    mut labels: Query<&mut Text, With<MenuResourceText>>,
) {
    let Some(profile) = profile else { return };
    let points = selection
        .class
        .and_then(|class| super::talents::saved(&profile, class))
        .map_or(0, |save| {
            super::talents::earned_points(save, 0).saturating_sub(save.spent())
        });
    let text = format!(
        "Talent Points: {points}    Tickets: {}    Gold: {}",
        profile.faire.tickets, profile.account.gold_earned
    );
    for mut label in &mut labels {
        label.0 = text.clone();
    }
}

/// `CharacterCreateConfigurationFrame`, 206×600 at TOPLEFT (28,-74): the race columns under the
/// faction banners, the gender pair, the class grid, the class text and Randomize.
fn tower(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        left: px(28.0),
        top: px(74.0),
        width: px(206.0),
        height: px(TOWER_H),
        ..default()
    })
    .with_children(|tower| {
        if let Some(bg) = &art.tower_bg {
            tower.spawn((
                ImageNode::new(bg.clone()),
                abs(1.0, -3.0, 0.0, 218.0, TOWER_H + 6.0),
            ));
        }
        // The create tower's top piece, then its foot piece cut to end where the class grid does.
        if let Some((border, size)) = &art.tower_border {
            for (top, height, tc) in [
                (0.0, 236.0, [0.0, 0.875, 0.0, 0.9375]),
                (
                    236.0,
                    TOWER_H - 236.0 + 6.0,
                    [
                        0.0,
                        0.875,
                        1.0 - (TOWER_H - 236.0 + 6.0) / 256.0 * 0.8203125 / (210.0 / 256.0),
                        1.0,
                    ],
                ),
            ] {
                tower.spawn((
                    ImageNode {
                        image: border.clone(),
                        rect: Some(tc_rect(*size, tc)),
                        ..default()
                    },
                    abs(1.0, -9.0, top, 224.0, height),
                ));
            }
        }
        if let Some(banners) = &art.banners {
            tower.spawn((
                ImageNode::new(banners.clone()),
                abs(1.0, -27.0, 60.0, 256.0, 259.0),
            ));
        }
        for (label, center) in [("Alliance", 51.0), ("Horde", 151.0)] {
            outlined_text(
                tower,
                Node {
                    justify_content: JustifyContent::Center,
                    ..abs(1.0, center - 60.0, 40.0, 120.0, 17.0)
                },
                (),
                (),
                GlueText {
                    text: label,
                    size: 15.0,
                    color: GOLD,
                    wrap: false,
                },
                f,
                1.0,
            );
        }

        for (faction, left) in [(ALLIANCE, 33.0), (HORDE, 127.0)] {
            tower
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(68.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5.0),
                    ..default()
                })
                .with_children(|col| {
                    for race in faction {
                        let face = art.races.as_ref().and_then(|(sheet, size)| {
                            race_tc(race, 0).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
                        });
                        icon_button(
                            col,
                            f,
                            MenuAction::Race(race),
                            Some(RaceFace(race)),
                            face,
                            None::<()>,
                            race_name(race),
                            art,
                            1.0,
                        );
                    }
                });
        }

        tower
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(53.0),
                top: px(303.0),
                flex_direction: FlexDirection::Row,
                column_gap: px(5.0),
                ..default()
            })
            .with_children(|g| {
                for (sex, label) in [(0u8, "Male"), (1u8, "Female")] {
                    let face = art.gender.as_ref().map(|(sheet, size)| {
                        let half = if sex == 0 {
                            [0.0, 0.5, 0.0, 1.0]
                        } else {
                            [0.5, 1.0, 0.0, 1.0]
                        };
                        (sheet.clone(), tc_rect(*size, half))
                    });
                    icon_button(
                        g,
                        f,
                        MenuAction::Sex(sex),
                        None::<()>,
                        face,
                        None::<()>,
                        label,
                        art,
                        1.0,
                    );
                }
            });

        tower
            .spawn(Node {
                position_type: PositionType::Absolute,
                // The gender pair's centre line: 53 + (2·48 + 5)/2 ≈ 103.
                left: px(103.0 - (3.0 * 48.0 + 2.0 * 4.0) / 2.0),
                width: px(3.0 * 48.0 + 2.0 * 4.0),
                top: px(368.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: px(4.0),
                row_gap: px(4.0),
                ..default()
            })
            .with_children(|grid| {
                for (i, class) in data::CLASSES.iter().enumerate() {
                    let face = art.classes.as_ref().and_then(|(sheet, size)| {
                        class_tc(class.id).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
                    });
                    icon_button(
                        grid,
                        f,
                        MenuAction::Class(i),
                        None::<()>,
                        face,
                        None::<()>,
                        class.name,
                        art,
                        1.0,
                    );
                }
            });

        tower
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(30.0),
                top: px(TOWER_H - 46.0),
                ..default()
            })
            .with_children(|r| {
                glue_button(
                    r,
                    art,
                    f,
                    MenuAction::Randomize,
                    "Randomize",
                    146.0,
                    30.0,
                    GlueBtnKind::Small,
                    1.0,
                );
            });
    });
}

/// "Level 1-10", or "Level 5" for a band of one.
fn levels(map: usize) -> String {
    match zones::level_band(map) {
        (lo, hi) if lo == hi => format!("Level {lo}"),
        (lo, hi) => format!("Level {lo}-{hi}"),
    }
}

/// A zone row's second line: its level band and continent, or what opens it.
fn row_line(map: usize, unlocks: &Unlocks) -> String {
    if unlocks.is_open(map) {
        format!("{}  {}", levels(map), data::MAPS[map].zone)
    } else {
        zones::lock_line(map)
    }
}

/// The zone list's sections, in order: a header and the zones under it, each section lowest
/// level first ([`zones::list_order`]), the Molten Core stages in raid order.
fn list_sections() -> Vec<(&'static str, Vec<usize>)> {
    let order = zones::list_order();
    let of = |want: zones::Tier, raid: Option<bool>| -> Vec<usize> {
        order
            .iter()
            .copied()
            .filter(|&i| {
                zones::map_tier(i) == want
                    && raid.is_none_or(|mc| (data::MAPS[i].zone == MOLTEN_CORE) == mc)
            })
            .collect()
    };
    let mut core = of(zones::Tier::Raid, Some(true));
    core.sort_unstable();
    [
        ("Starting Zones", of(zones::Tier::Starting, None)),
        ("Frontier", of(zones::Tier::Frontier, None)),
        ("Elite", of(zones::Tier::Elite, None)),
        ("Raid", of(zones::Tier::Raid, Some(false))),
        ("Raid: Molten Core", core),
    ]
    .into_iter()
    .filter(|(_, maps)| !maps.is_empty())
    .collect()
}

/// A row's name: a Molten Core stage drops the raid's name, which its header carries.
fn row_name(map: usize) -> &'static str {
    let name = data::MAPS[map].name;
    name.strip_prefix(MOLTEN_CORE)
        .and_then(|n| n.strip_prefix(": "))
        .unwrap_or(name)
}

/// The zone list in `CharacterSelectCharacterFrame` (260 wide at TOPRIGHT (-5,-15)), ending
/// above the bar: one character-row per zone under its section's header, in a body the wheel
/// scrolls. [`refresh_zone_rows`] keeps the locks and pips.
fn zone_frame(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        right: px(5.0),
        top: px(FRAME_TOP),
        width: px(260.0),
        height: px(FRAME_H),
        ..default()
    })
    .with_children(|frame| {
        if let (Some(bg), Some(border)) = (&art.tooltip_bg, &art.name_border) {
            frame.spawn((
                tiled_bg_node(bg.clone(), NAME_EDGE, 1.0, FRAME_FILL),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(10.0),
                    right: px(5.0),
                    top: px(4.0),
                    bottom: px(9.0),
                    ..default()
                },
            ));
            backdrop_border(frame, border, NAME_EDGE, FRAME_BORDER);
        } else {
            frame.spawn((BackgroundColor(FRAME_FILL), crate::glue::widgets::overlay()));
        }
        outlined_text(
            frame,
            Node {
                position_type: PositionType::Absolute,
                top: px(12.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            (),
            (),
            GlueText {
                text: "Choose a Zone",
                size: 18.0,
                color: DIM,
                wrap: false,
            },
            f,
            1.0,
        );
        frame
            .spawn((
                ZoneScroll,
                // Hovered between rows too, so the wheel scrolls the list, not the camera.
                Interaction::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(12.0),
                    right: px(8.0),
                    top: px(LIST_TOP),
                    bottom: px(14.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .with_children(|view| {
                view.spawn((
                    ZoneScrollBody,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 0.0,
                        top: px(0.0),
                        padding: UiRect {
                            left: px(ROW_INDENT),
                            right: px(4.0),
                            top: px(2.0),
                            bottom: px(8.0),
                        },
                        ..default()
                    },
                ))
                .with_children(|list| {
                    for (n, (title, maps)) in list_sections().into_iter().enumerate() {
                        section_header(list, f, title, n == 0);
                        for i in maps {
                            zone_row(list, art, f, i);
                        }
                    }
                });
            });
    });
}

/// A section's title over a rule, spanning the row.
fn section_header(list: &mut ChildSpawnerCommands, f: &Handle<Font>, title: &str, first: bool) {
    let px = Val::Px;
    list.spawn(Node {
        width: px(ROW_W),
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        margin: UiRect {
            top: px(if first { 0.0 } else { 8.0 }),
            bottom: px(6.0),
            ..default()
        },
        ..default()
    })
    .with_children(|h| {
        outlined_text(
            h,
            Node::default(),
            (),
            (),
            GlueText {
                text: title,
                size: 13.0,
                color: HEADER,
                wrap: false,
            },
            f,
            1.0,
        );
        h.spawn((
            Node {
                width: Val::Percent(100.0),
                height: px(1.0),
                margin: UiRect::top(px(2.0)),
                ..default()
            },
            BackgroundColor(GOLD.with_alpha(0.45)),
        ));
    });
}

/// One zone's row: its name, its level band or what opens it, its pips.
fn zone_row(list: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>, i: usize) {
    let px = Val::Px;
    let map = &data::MAPS[i];
    list.spawn((
        MenuAction::Map(i),
        Button,
        LockHighlight::default(),
        GlueDisabled::default(),
        Node {
            width: px(ROW_W),
            height: px(ROW_H),
            flex_shrink: 0.0,
            margin: UiRect::bottom(px(ROW_GAP)),
            ..default()
        },
    ))
    .with_children(|b| {
        if let Some(hi) = &art.select_highlight {
            b.spawn((
                Hilight,
                Visibility::Hidden,
                MaterialNode(hi.clone()),
                abs(1.0, -ROW_INDENT, -2.0, ROW_W + ROW_INDENT, ROW_H + 4.0),
            ));
        }
        outlined_text(
            b,
            abs(1.0, 0.0, 2.0, ROW_W, 16.0),
            (),
            ZoneRowName(i),
            GlueText {
                text: row_name(i),
                size: 15.0,
                color: GOLD,
                wrap: false,
            },
            f,
            1.0,
        );
        outlined_text(
            b,
            abs(1.0, 0.0, 19.0, ROW_W, 14.0),
            (),
            ZoneRowLine(i),
            GlueText {
                text: &format!("{}  {}", levels(i), map.zone),
                size: 12.0,
                color: Color::WHITE,
                wrap: false,
            },
            f,
            1.0,
        );
        pip_track(b, i);
    });
}

/// A zone row's pips, right-aligned on its name line: the won marker, lit gold once the zone is
/// won at all, then [`zones::PIPS`] lit for the best win's rewards multiplier ([`zones::pips`]:
/// one per +0.125 over ×1.00, all eight at the ×2.00 cap).
fn pip_track(b: &mut ChildSpawnerCommands, map: usize) {
    let px = Val::Px;
    let pips = f32::from(zones::PIPS);
    let width = MARKER_W + 2.0 * PIP_GAP + pips * (PIP_W + PIP_GAP) - PIP_GAP;
    b.spawn((
        ZonePips(map),
        Node {
            column_gap: px(PIP_GAP),
            align_items: AlignItems::Center,
            ..abs(1.0, ROW_W - width, 5.0, width, MARKER_W)
        },
    ))
    .with_children(|track| {
        for slot in 0..=zones::PIPS {
            let size = if slot == 0 { MARKER_W } else { PIP_W };
            track.spawn((
                ZonePip { map, slot },
                Node {
                    width: px(size),
                    height: px(size),
                    border: UiRect::all(px(1.0)),
                    margin: UiRect::right(px(if slot == 0 { PIP_GAP } else { 0.0 })),
                    ..default()
                },
                BackgroundColor(PIP_OFF),
                BorderColor::all(Color::BLACK),
            ));
        }
    });
}

/// A tooltip box's backdrop, as the zone list's frame is drawn: the tiled fill inside the border.
fn card_backdrop(card: &mut ChildSpawnerCommands, art: &GlueArt) {
    let px = Val::Px;
    if let (Some(bg), Some(border)) = (&art.tooltip_bg, &art.name_border) {
        card.spawn((
            tiled_bg_node(bg.clone(), NAME_EDGE, 1.0, FRAME_FILL),
            Node {
                position_type: PositionType::Absolute,
                left: px(4.0),
                right: px(4.0),
                top: px(4.0),
                bottom: px(4.0),
                ..default()
            },
        ));
        backdrop_border(card, border, NAME_EDGE, FRAME_BORDER);
    } else {
        card.spawn((BackgroundColor(FRAME_FILL), crate::glue::widgets::overlay()));
    }
}

/// A box of lines `width` wide that grows to its text, placed by `node`'s offsets, with its
/// backdrop under the `lines`.
fn card(
    ui: &mut ChildSpawnerCommands,
    art: &GlueArt,
    node: Node,
    width: f32,
    extra: impl Bundle,
    lines: impl FnOnce(&mut ChildSpawnerCommands),
) {
    ui.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(CARD_PAD)),
            ..node
        },
        extra,
    ))
    .with_children(|c| {
        card_backdrop(c, art);
        lines(c);
    });
}

/// One wrapped line of a card, `gap` below the line before.
#[allow(clippy::too_many_arguments)]
fn card_line(
    card: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    marker: impl Bundle,
    text: &str,
    size: f32,
    color: Color,
    width: f32,
    gap: f32,
) {
    outlined_text(
        card,
        Node {
            width: Val::Px(width),
            margin: UiRect::top(Val::Px(gap)),
            ..default()
        },
        (),
        marker,
        GlueText {
            text,
            size,
            color,
            wrap: true,
        },
        f,
        1.0,
    );
}

/// The zone card, left of the list: the zone hovered (else the one picked), its tier, its best
/// win or what opens it, its bosses with their times and its horde. [`refresh_zone_card`] fills
/// it.
fn zone_card(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let w = ZONE_CARD_W - 2.0 * CARD_PAD;
    let bosses = (0..data::MAPS.len())
        .map(|i| zones::boss_schedule(i).len())
        .max()
        .unwrap_or(0);
    let node = Node {
        right: Val::Px(ZONE_CARD_RIGHT),
        top: Val::Px(ZONE_CARD_TOP),
        ..default()
    };
    card(ui, art, node, ZONE_CARD_W, (), |c| {
        card_line(c, f, ZoneCardText::Name, "", 16.0, GOLD, w, 0.0);
        card_line(c, f, ZoneCardText::Kind, "", 12.0, Color::WHITE, w, 2.0);
        card_line(c, f, ZoneCardText::Best, "", 12.0, GREEN, w, 2.0);
        card_line(c, f, (), "Bosses", 13.0, GOLD, w, 8.0);
        for slot in 0..bosses {
            card_line(
                c,
                f,
                ZoneCardText::Boss(slot),
                "",
                12.0,
                Color::WHITE,
                w,
                1.0,
            );
        }
        card_line(c, f, (), "Foes", 13.0, GOLD, w, 8.0);
        card_line(c, f, ZoneCardText::Foes, "", 11.0, FOES, w, 1.0);
    });
}

/// The race card, right of the tower: the picked race's two traits and what each grants.
/// [`refresh_race_card`] fills it and hides it until a race is picked.
fn race_card(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let w = RACE_CARD_W - 2.0 * CARD_PAD;
    let node = Node {
        left: Val::Px(RACE_CARD_LEFT),
        top: Val::Px(RACE_CARD_TOP),
        ..default()
    };
    card(
        ui,
        art,
        node,
        RACE_CARD_W,
        (RaceCard, Visibility::Hidden),
        |c| {
            card_line(c, f, RaceCardText::Title, "", 15.0, GOLD, w, 0.0);
            for slot in 0..2 {
                card_line(
                    c,
                    f,
                    RaceCardText::Name(slot),
                    "",
                    12.0,
                    Color::WHITE,
                    w,
                    6.0,
                );
                card_line(c, f, RaceCardText::Effect(slot), "", 12.0, GREEN, w, 1.0);
            }
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn menu_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &MenuAction)>,
    phase: Res<State<Phase>>,
    creatures: Option<Res<crate::entities::Creatures>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    meta: Res<MetaBonuses>,
    unlocks: Res<Unlocks>,
    mut rng: ResMut<Rng>,
    mut next: ResMut<NextState<Phase>>,
    profile: Option<Res<super::profile::Profile>>,
    mut dice: Option<ResMut<super::gear::LooksDice>>,
    mut create: ResMut<super::gear::looks_create::CreateSession>,
    mut exit: MessageWriter<AppExit>,
) {
    if *phase.get() != Phase::Menu {
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    if action == MenuAction::Quit {
        exit.write(AppExit::Success);
        return;
    }
    let Some(creatures) = creatures.as_deref() else {
        return;
    };
    let all = all_looks(creatures);
    let cur = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, r, s)| (r, s));
    let (race, sex) = (cur.map(|c| c.0), cur.map_or(0, |c| c.1));
    match action {
        // A locked zone takes no pick: its row and the zone card say what opens it.
        MenuAction::Map(i) if !unlocks.is_open(i) => return,
        MenuAction::Map(i) => {
            if selection.map != i {
                selection.map = i;
                selection.version += 1;
            }
        }
        MenuAction::Enter => {
            if unlocks.is_open(selection.map)
                && start_run(
                    &mut run,
                    local.single_mut().ok().as_deref_mut(),
                    &mut selection,
                    &meta,
                )
            {
                next.set(Phase::Playing);
            }
        }
        MenuAction::Quit => return,
        MenuAction::Randomize => {
            let picks: Vec<(usize, usize)> = all
                .iter()
                .enumerate()
                .flat_map(|(c, looks)| (0..looks.len()).map(move |l| (c, l)))
                .collect();
            if !picks.is_empty() {
                let (c, l) = picks[rng.index(picks.len())];
                set(&mut selection, &all, c, l);
                // The create screen's dice roll the features too.
                if let Some(dice) = dice.as_deref_mut() {
                    dice.0 = true;
                }
            }
        }
        MenuAction::Customize => {
            super::gear::looks_create::open(&mut create, &selection);
            return;
        }
        // The class comes back as its hero was last picked, else keeps the race and gender
        // where it can.
        MenuAction::Class(c) => {
            let looks = &all[c];
            let saved = profile
                .as_deref()
                .and_then(|p| super::gear::saved_body(p, c));
            let l = saved
                .and_then(|(r, s)| find(looks, Some(r), Some(s)))
                .or_else(|| find(looks, race, Some(sex)))
                .or_else(|| find(looks, race, None))
                .or_else(|| find(looks, None, Some(sex)))
                .or_else(|| find(looks, None, None));
            if let Some(l) = l {
                set(&mut selection, &all, c, l);
            }
        }
        // The race keeps the class where it can, else takes the first class it has.
        MenuAction::Race(r) => {
            let classes = selection.class.into_iter().chain(0..all.len());
            for c in classes {
                let l = find(&all[c], Some(r), Some(sex)).or_else(|| find(&all[c], Some(r), None));
                if let Some(l) = l {
                    set(&mut selection, &all, c, l);
                    break;
                }
            }
        }
        MenuAction::Sex(s) => {
            if let Some(c) = selection.class {
                if let Some(l) = find(&all[c], race, Some(s)) {
                    set(&mut selection, &all, c, l);
                }
            }
        }
    }
    save_pick(&selection);
}

/// The screen's per-frame state: the locked highlights, hover labels, which races and classes
/// the pick allows, the gender on the race faces, the name line, the zones not yet open and
/// Enter World.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_menu_ui(
    selection: Res<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
    art: Res<GlueArt>,
    unlocks: Res<Unlocks>,
    mut buttons: Query<(
        &MenuAction,
        &Interaction,
        &Children,
        Option<&mut LockHighlight>,
        Option<&mut GlueDisabled>,
        &mut Node,
        &mut Visibility,
    )>,
    mut labels: Query<&mut Visibility, (With<HoverLabel>, Without<MenuAction>)>,
    mut faces: Query<(&RaceFace, &mut ImageNode)>,
    mut name: Query<(&mut Text, &mut TextColor), With<PickName>>,
    mut cache: Local<(Option<u32>, bool, Vec<Vec<(u32, u8, u8)>>)>,
) {
    let have = creatures.is_some();
    if cache.0 != Some(selection.version) || cache.1 != have {
        *cache = (
            Some(selection.version),
            have,
            creatures.as_deref().map(all_looks).unwrap_or_default(),
        );
    }
    let all = &cache.2;
    let cur = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map(|&(_, r, s)| (r, s));

    for (action, interaction, children, lock, disabled, mut node, mut vis) in &mut buttons {
        let selected = match *action {
            MenuAction::Race(r) => cur.is_some_and(|c| c.0 == r),
            MenuAction::Sex(s) => cur.is_some_and(|c| c.1 == s),
            MenuAction::Class(i) => selection.class == Some(i),
            MenuAction::Map(i) => selection.map == i,
            MenuAction::Randomize
            | MenuAction::Customize
            | MenuAction::Enter
            | MenuAction::Quit => false,
        };
        if let Some(mut lock) = lock {
            if lock.0 != selected {
                lock.0 = selected;
            }
        }
        if let Some(mut disabled) = disabled {
            // A disabled zone row never lights, on hover or picked.
            let off = match *action {
                MenuAction::Enter => cur.is_none() || !unlocks.is_open(selection.map),
                MenuAction::Map(i) => !unlocks.is_open(i),
                _ => false,
            };
            if disabled.0 != off {
                disabled.0 = off;
            }
        }
        match *action {
            // As on the create screen, a class the race cannot be leaves the grid.
            MenuAction::Class(i) => {
                let ok = cur.is_none_or(|(r, _)| {
                    all.get(i).is_some_and(|l| find(l, Some(r), None).is_some())
                });
                let display = if ok { Display::Flex } else { Display::None };
                if node.display != display {
                    node.display = display;
                }
            }
            MenuAction::Race(r) => {
                let ok = all.is_empty() || all.iter().any(|l| find(l, Some(r), None).is_some());
                let want = if ok {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *vis != want {
                    *vis = want;
                }
            }
            _ => {}
        }
        let show = selected || *interaction != Interaction::None || art.races.is_none();
        for child in children {
            if let Ok(mut label) = labels.get_mut(*child) {
                let want = if show {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *label != want {
                    *label = want;
                }
            }
        }
    }

    if let (Some((_, size)), Some((_, sex))) = (&art.races, cur) {
        for (face, mut img) in &mut faces {
            if let Some(tc) = race_tc(face.0, sex) {
                let rect = Some(tc_rect(*size, tc));
                if img.rect != rect {
                    img.rect = rect;
                }
            }
        }
    }

    let class = selection.class_def();
    if let Ok((mut t, mut color)) = name.single_mut() {
        let want = match (class, cur) {
            (Some(c), Some((r, _))) => format!("{} {}", race_name(r), c.name),
            _ => String::new(),
        };
        if t.0 != want {
            t.0 = want;
        }
        // In the class's own colour, as the game's raid frames and chat show it.
        let want = class.map_or(GOLD, |c| rgb(c.color));
        if color.0 != want {
            color.0 = want;
        }
    }
}

/// The zone rows' locks and pips: a locked zone's name dims, its second line says what opens it
/// and its pips hide. Only when the standings change, or the rows are new.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_zone_rows(
    unlocks: Res<Unlocks>,
    added: Query<(), Added<ZoneRowLine>>,
    mut names: Query<(&ZoneRowName, &mut TextColor)>,
    mut lines: Query<(&ZoneRowLine, &mut Text, &mut TextColor), Without<ZoneRowName>>,
    mut tracks: Query<(&ZonePips, &mut Visibility)>,
    mut pips: Query<(&ZonePip, &mut BackgroundColor)>,
) {
    if !unlocks.is_changed() && added.is_empty() {
        return;
    }
    for (name, mut color) in &mut names {
        let want = if unlocks.is_open(name.0) { GOLD } else { DIM };
        if color.0 != want {
            color.0 = want;
        }
    }
    for (line, mut text, mut color) in &mut lines {
        let want = row_line(line.0, &unlocks);
        if text.0 != want {
            text.0 = want;
        }
        let want = if unlocks.is_open(line.0) {
            Color::WHITE
        } else {
            LOCKED
        };
        if color.0 != want {
            color.0 = want;
        }
    }
    for (track, mut vis) in &mut tracks {
        let want = if unlocks.is_open(track.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (pip, mut bg) in &mut pips {
        let standing = unlocks.standing(pip.map);
        let want = match pip.slot {
            0 if standing.wins > 0 => GOLD,
            s if s > 0 && s <= zones::pips(standing.best_win_mult) => PIP_ON,
            _ => PIP_OFF,
        };
        if bg.0 != want {
            bg.0 = want;
        }
    }
}

/// The zone card follows the zone hovered in the list, else the one picked.
pub(super) fn refresh_zone_card(
    selection: Res<Selection>,
    unlocks: Res<Unlocks>,
    rows: Query<(&MenuAction, &Interaction)>,
    added: Query<(), Added<ZoneCardText>>,
    mut texts: Query<(&ZoneCardText, &mut Text, &mut TextColor)>,
    mut shown: Local<Option<usize>>,
) {
    let hovered = rows.iter().find_map(|(action, interaction)| match action {
        MenuAction::Map(i) if *interaction != Interaction::None => Some(*i),
        _ => None,
    });
    let map = hovered.unwrap_or(selection.map).min(data::MAPS.len() - 1);
    if *shown == Some(map) && !unlocks.is_changed() && added.is_empty() {
        return;
    }
    *shown = Some(map);
    let tier = zones::map_tier(map);
    let standing = unlocks.standing(map);
    let bosses = zones::boss_schedule(map);
    let foes = zones::foe_names(map).join(", ");
    for (line, mut text, mut color) in &mut texts {
        let (want, tint) = match *line {
            ZoneCardText::Name => (data::MAPS[map].name.to_string(), GOLD),
            ZoneCardText::Kind => (format!("{} · {}", tier.label(), levels(map)), Color::WHITE),
            ZoneCardText::Best => match standing.best_win() {
                _ if !standing.open => (zones::lock_line(map), LOCKED),
                Some(best) => (best, GREEN),
                None => ("Not yet won".to_string(), DIM),
            },
            ZoneCardText::Boss(n) => match bosses.get(n) {
                Some(b) if b.last => (
                    format!("{}  {}  (final)", zones::clock(b.at), b.name),
                    FINAL_BOSS,
                ),
                Some(b) => (format!("{}  {}", zones::clock(b.at), b.name), Color::WHITE),
                None => (String::new(), Color::WHITE),
            },
            ZoneCardText::Foes => (foes.clone(), FOES),
        };
        if text.0 != want {
            text.0 = want;
        }
        if color.0 != tint {
            color.0 = tint;
        }
    }
}

/// The race card shows the picked race's traits, and hides with no race picked.
pub(super) fn refresh_race_card(
    selection: Res<Selection>,
    added: Query<(), Added<RaceCardText>>,
    mut cards: Query<&mut Visibility, With<RaceCard>>,
    mut texts: Query<(&RaceCardText, &mut Text)>,
    mut shown: Local<Option<Option<u8>>>,
) {
    let race = racials::picked_race(&selection);
    if *shown == Some(race) && added.is_empty() {
        return;
    }
    *shown = Some(race);
    let traits = race.and_then(racials::traits);
    for mut vis in &mut cards {
        let want = if traits.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    let Some(traits) = traits else {
        return;
    };
    for (line, mut text) in &mut texts {
        let want = match *line {
            RaceCardText::Title => format!("{} Racials", race_name(traits.race)),
            RaceCardText::Name(n) => traits.traits[n].name.to_string(),
            RaceCardText::Effect(n) => traits.traits[n].effect(),
        };
        if text.0 != want {
            text.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_select_actions_have_one_fixed_stack_order() {
        assert_eq!(
            SECONDARY_BUTTONS,
            [
                "Customize",
                "Armory",
                "Darkmoon Faire",
                "Multiplayer",
                "Options",
                "Quit",
            ]
        );
        assert_eq!(SECONDARY_BUTTONS.len(), 6);
    }

    /// A wheel notch down scrolls one row, and the list stops at both ends.
    #[test]
    fn a_wheel_notch_scrolls_one_row_and_stops_at_the_ends() {
        let row = ROW_H + ROW_GAP;
        assert_eq!(scrolled(0.0, -1.0, 500.0), row);
        assert_eq!(scrolled(row, 1.0, 500.0), 0.0);
        assert_eq!(scrolled(0.0, 3.0, 500.0), 0.0, "no scrolling above the top");
        assert_eq!(
            scrolled(480.0, -40.0, 500.0),
            500.0,
            "no scrolling past the end"
        );
        assert_eq!(
            scrolled(0.0, -1.0, -20.0),
            0.0,
            "a list that fits never scrolls"
        );
        // A shrinking list pulls the offset back in range.
        assert_eq!(scrolled(300.0, 0.0, 100.0), 100.0);
    }

    /// Every zone is listed once, under its tier's header, in the fixed section order; the Molten
    /// Core stages keep the raid's order under their own header.
    #[test]
    fn the_zone_list_has_every_zone_once_under_its_section() {
        let sections = list_sections();
        let titles: Vec<&str> = sections.iter().map(|(t, _)| *t).collect();
        let order = [
            "Starting Zones",
            "Frontier",
            "Elite",
            "Raid",
            "Raid: Molten Core",
        ];
        let mut at = 0;
        for t in &titles {
            let i = order.iter().position(|o| o == t).expect("a known header");
            assert!(i >= at, "{titles:?} out of order");
            at = i;
        }
        let mut seen: Vec<usize> = sections.iter().flat_map(|(_, m)| m.clone()).collect();
        let listed = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), listed, "a zone listed twice");
        assert_eq!(listed, data::MAPS.len(), "a zone missing from the list");
        for (title, maps) in &sections {
            for &m in maps {
                let core = data::MAPS[m].zone == MOLTEN_CORE;
                assert_eq!(
                    *title == "Raid: Molten Core",
                    core,
                    "{}",
                    data::MAPS[m].name
                );
                if !core {
                    assert_eq!(
                        *title == "Raid",
                        zones::map_tier(m) == zones::Tier::Raid,
                        "{}",
                        data::MAPS[m].name
                    );
                }
            }
            if *title == "Raid: Molten Core" {
                assert!(maps.windows(2).all(|w| w[0] < w[1]), "raid order");
                assert!(maps.iter().all(|&m| !row_name(m).starts_with(MOLTEN_CORE)));
            }
        }
    }

    /// The body directory each race's looks must come from, by `ChrRaces` id - 1.
    const RACE_DIRS: [&str; 8] = [
        "human", "orc", "dwarf", "nightelf", "scourge", "tauren", "gnome", "troll",
    ];

    /// The client's creature displays, raw and as the game holds them; `None` without the data.
    fn client_creatures() -> Option<(benilla_formats::CreatureCatalog, crate::entities::Creatures)>
    {
        let data = benilla_formats::wow_data()?;
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let raw = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let held = crate::entities::Creatures::from_catalog(
            benilla_formats::load_creature_catalog(&mut chain).expect("creatures"),
        );
        Some((raw, held))
    }

    /// Every class × race × sex the menu offers saves and restores to the same look, and that
    /// look's body is the race and sex it claims, so a restored Troll never wears a Tauren.
    #[test]
    fn every_offered_pick_round_trips_through_the_file() {
        let Some((catalog, creatures)) = client_creatures() else {
            eprintln!("skipping: no WoW data");
            return;
        };
        let all = all_looks(&creatures);
        let mut wrong = Vec::new();
        for (class, looks) in all.iter().enumerate() {
            assert!(
                !looks.is_empty(),
                "{} has no look",
                data::CLASSES[class].name
            );
            for (look, &(display, race, sex)) in looks.iter().enumerate() {
                let body = catalog
                    .model(display)
                    .map(|m| m.model_path.to_ascii_lowercase())
                    .unwrap_or_default();
                let want = format!(
                    "character\\{}\\{}\\",
                    RACE_DIRS[race as usize - 1],
                    if sex == 1 { "female" } else { "male" }
                );
                if !body.starts_with(&want) {
                    wrong.push(format!("display {display} ({race}, {sex}) is {body}"));
                }
                for map in [0, data::MAPS.len() - 1] {
                    let mut sel = Selection::default();
                    set(&mut sel, &all, class, look);
                    sel.map = map;
                    let text = pick_text(&sel).expect("a full pick");
                    let mut back = Selection::default();
                    apply_pick(&text, &creatures, &mut back);
                    let got = back.look.and_then(|l| back.looks.get(l)).copied();
                    if back.class != Some(class)
                        || got != Some((display, race, sex))
                        || back.map != map
                    {
                        wrong.push(format!("{text:?} restored {:?} {got:?}", back.class));
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    /// A pick whose exact look is gone keeps its class, and its race where the class has it; a
    /// file that is not four numbers restores nothing.
    #[test]
    fn a_stale_or_garbled_pick_keeps_what_it_can() {
        let Some((_, creatures)) = client_creatures() else {
            eprintln!("skipping: no WoW data");
            return;
        };
        let all = all_looks(&creatures);
        for (class, looks) in all.iter().enumerate() {
            for race in 1..=8u8 {
                for sex in 0..=1u8 {
                    let mut back = Selection::default();
                    apply_pick(&format!("{class} {race} {sex} 0"), &creatures, &mut back);
                    assert_eq!(back.class, Some(class), "{class} {race} {sex}");
                    let (_, got_race, _) = back.looks[back.look.expect("a look")];
                    if looks.iter().any(|&(_, r, _)| r == race) {
                        assert_eq!(got_race, race, "class {class} race {race} sex {sex}");
                    }
                }
            }
        }
        let mut back = Selection::default();
        apply_pick("\u{feff}0 8 0 1", &creatures, &mut back);
        assert_eq!(
            (back.class, back.map),
            (Some(0), 1),
            "a byte-order mark is not a field"
        );
        for junk in ["0 8 x 0 1", "0 8 0", "", "0 8 0 1 2", "0 999 0 1"] {
            let mut back = Selection::default();
            apply_pick(junk, &creatures, &mut back);
            assert_eq!(back.class, None, "{junk:?}");
        }
    }
}
