//! The Survivors character-create overlay.  Unlike the Armory panel this is a transaction: it
//! previews changes on the menu hero and puts the saved body and looks back on Cancel.

use std::collections::BTreeMap;

use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::input::ButtonState;
use bevy::prelude::*;

use super::super::hud::{HudFont, MenuRoot};
use super::super::menu::{all_looks, find, set};
use super::super::{data, Rng, Selection};
use super::looks::{
    feature_labels, normalize_name, random_name, Body, HeroLooks, FEATURES, NAME_MAX,
};
use super::looks_ui::current;
use super::store::GearStore;
use crate::char_select::race_name;
use crate::entities::{CharCreate, Creatures};
use crate::glue::art::{class_tc, race_tc, tc_rect, GlueArt, GOLD};
use crate::glue::widgets::{
    dial_arrow, glue_button, icon_button, outlined_text, GlueBtnKind, GlueText,
};
use crate::glue::GlueClicks;
use crate::glue_strings::GlueStrings;

const ALLIANCE: [u8; 4] = [1, 3, 4, 7];
const HORDE: [u8; 4] = [2, 5, 6, 8];

#[derive(Component)]
pub(in crate::survivors) struct CreateRoot;
#[derive(Component, Clone, Copy)]
pub(in crate::survivors) enum Action {
    Race(u8),
    Sex(u8),
    Class(usize),
    Step(u8, i8),
    Random,
    EditName,
    RandomName,
    Accept,
    Cancel,
    Rotate(i8),
    Model,
}
#[derive(Component)]
pub(in crate::survivors) struct Readout(u8);
#[derive(Component)]
pub(in crate::survivors) struct Caption(u8);
#[derive(Component)]
pub(in crate::survivors) struct Name;

/// Snapshotting makes the screen safe to open from a live menu: no profile change survives
/// Cancel, including a body switch which causes the menu preview to be respawned.
#[derive(Resource, Default)]
pub(in crate::survivors) struct CreateSession {
    open: bool,
    selection: Option<Selection>,
    looks: BTreeMap<usize, HeroLooks>,
    name: String,
    editing_name: bool,
}
pub(in crate::survivors) fn open(session: &mut CreateSession, selection: &Selection) {
    if !session.open {
        session.open = true;
        session.selection = Some(selection.clone());
    }
}

pub(super) fn build(
    mut commands: Commands,
    session: Res<CreateSession>,
    roots: Query<Entity, With<MenuRoot>>,
    existing: Query<(), With<CreateRoot>>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
) {
    if !session.open || !existing.is_empty() {
        return;
    }
    let Ok(root) = roots.single() else {
        return;
    };
    commands.entity(root).with_children(|ui| {
        ui.spawn((
            CreateRoot,
            GlobalZIndex(50),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.02, 0.35)),
        ))
        .with_children(|screen| {
            // This transparent pane owns left drags; the existing menu preview remains the
            // actual large, centred 3-D hero behind it.
            screen.spawn((
                Action::Model,
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(245.0),
                    right: Val::Px(285.0),
                    top: Val::Px(80.0),
                    bottom: Val::Px(90.0),
                    ..default()
                },
            ));
            column(screen, &art, &font.0, true);
            details(screen, &art, &font.0);
            bottom(screen, &art, &font.0);
        });
    });
}

fn column(p: &mut ChildSpawnerCommands, art: &GlueArt, font: &Handle<Font>, left: bool) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(26.0),
            top: Val::Px(70.0),
            width: Val::Px(208.0),
            height: Val::Px(590.0),
            padding: UiRect::all(Val::Px(12.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.03, 0.02, 0.88)),
    ))
    .with_children(|c| {
        outlined_text(
            c,
            Node::default(),
            (),
            (),
            GlueText {
                text: "CHOOSE YOUR RACE",
                size: 14.0,
                color: GOLD,
                wrap: false,
            },
            font,
            1.0,
        );
        for race in ALLIANCE.into_iter().chain(HORDE) {
            let face = art.races.as_ref().and_then(|(sheet, size)| {
                race_tc(race, 0).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
            });
            icon_button(
                c,
                font,
                Action::Race(race),
                None::<()>,
                face,
                None::<()>,
                race_name(race),
                art,
                1.0,
            );
        }
        outlined_text(
            c,
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            (),
            (),
            GlueText {
                text: "GENDER",
                size: 14.0,
                color: GOLD,
                wrap: false,
            },
            font,
            1.0,
        );
        c.spawn((
            Action::EditName,
            Button,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(5.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.85)),
        ))
        .with_children(|g| {
            for (sex, label) in [(0, "Male"), (1, "Female")] {
                glue_button(
                    g,
                    art,
                    font,
                    Action::Sex(sex),
                    label,
                    82.0,
                    28.0,
                    GlueBtnKind::Small,
                    1.0,
                );
            }
        });
        outlined_text(
            c,
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            (),
            (),
            GlueText {
                text: "CLASS",
                size: 14.0,
                color: GOLD,
                wrap: false,
            },
            font,
            1.0,
        );
        c.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(3.0),
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|g| {
            for (i, class) in data::CLASSES.iter().enumerate() {
                let face = art.classes.as_ref().and_then(|(sheet, size)| {
                    class_tc(class.id).map(|tc| (sheet.clone(), tc_rect(*size, tc)))
                });
                icon_button(
                    g,
                    font,
                    Action::Class(i),
                    None::<()>,
                    face,
                    None::<()>,
                    class.name,
                    art,
                    1.0,
                );
            }
        });
    });
    let _ = left;
}

fn details(p: &mut ChildSpawnerCommands, art: &GlueArt, font: &Handle<Font>) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(28.0),
            top: Val::Px(130.0),
            width: Val::Px(255.0),
            padding: UiRect::all(Val::Px(14.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.04, 0.03, 0.02, 0.88)),
    ))
    .with_children(|c| {
        outlined_text(
            c,
            Node::default(),
            (),
            (),
            GlueText {
                text: "CUSTOMIZE",
                size: 18.0,
                color: GOLD,
                wrap: false,
            },
            font,
            1.0,
        );
        c.spawn((
            Action::EditName,
            Button,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(5.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.85)),
        ))
        .with_children(|n| {
            outlined_text(
                n,
                Node {
                    width: Val::Px(150.0),
                    ..default()
                },
                (),
                Name,
                GlueText {
                    text: "",
                    size: 14.0,
                    color: Color::WHITE,
                    wrap: false,
                },
                font,
                1.0,
            );
            glue_button(
                n,
                art,
                font,
                Action::RandomName,
                "Random",
                70.0,
                28.0,
                GlueBtnKind::Small,
                1.0,
            );
        });
        for i in 0..FEATURES as u8 {
            row(c, art, font, i);
        }
    });
}

fn row(p: &mut ChildSpawnerCommands, art: &GlueArt, font: &Handle<Font>, i: u8) {
    p.spawn(Node {
        height: Val::Px(29.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|r| {
        outlined_text(
            r,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(112.0),
                ..default()
            },
            (),
            Caption(i),
            GlueText {
                text: "",
                size: 12.0,
                color: GOLD,
                wrap: false,
            },
            font,
            1.0,
        );
        dial_arrow(
            r,
            &art.arrow_left,
            font,
            Action::Step(i, -1),
            115.0,
            "<",
            1.0,
        );
        outlined_text(
            r,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(146.0),
                width: Val::Px(55.0),
                ..default()
            },
            (),
            Readout(i),
            GlueText {
                text: "",
                size: 12.0,
                color: Color::WHITE,
                wrap: false,
            },
            font,
            1.0,
        );
        dial_arrow(
            r,
            &art.arrow_right,
            font,
            Action::Step(i, 1),
            202.0,
            ">",
            1.0,
        );
    });
}

fn bottom(p: &mut ChildSpawnerCommands, art: &GlueArt, font: &Handle<Font>) {
    p.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(24.0),
        width: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(12.0),
        ..default()
    })
    .with_children(|b| {
        glue_button(
            b,
            art,
            font,
            Action::Random,
            "Randomize",
            145.0,
            34.0,
            GlueBtnKind::Small,
            1.0,
        );
        glue_button(
            b,
            art,
            font,
            Action::Rotate(-1),
            "Rotate Left",
            110.0,
            34.0,
            GlueBtnKind::Small,
            1.0,
        );
        glue_button(
            b,
            art,
            font,
            Action::Rotate(1),
            "Rotate Right",
            110.0,
            34.0,
            GlueBtnKind::Small,
            1.0,
        );
        glue_button(
            b,
            art,
            font,
            Action::Accept,
            "Accept",
            120.0,
            38.0,
            GlueBtnKind::Normal,
            1.0,
        );
        glue_button(
            b,
            art,
            font,
            Action::Cancel,
            "Cancel",
            120.0,
            38.0,
            GlueBtnKind::Normal,
            1.0,
        );
    });
}

#[allow(clippy::too_many_arguments)]
pub(super) fn input(
    mut commands: Commands,
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &Action)>,
    mut session: ResMut<CreateSession>,
    mut selection: ResMut<Selection>,
    creatures: Option<Res<Creatures>>,
    cc: Option<Res<CharCreate>>,
    strings: Option<Res<GlueStrings>>,
    mut store: GearStore,
    mut rng: ResMut<Rng>,
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut captions: Query<(&Caption, &mut Text), (Without<Readout>, Without<Name>)>,
    mut reads: Query<(&Readout, &mut Text), (Without<Caption>, Without<Name>)>,
    mut names: Query<&mut Text, (With<Name>, Without<Caption>, Without<Readout>)>,
    roots: Query<Entity, With<CreateRoot>>,
) {
    if !session.open {
        return;
    }
    let Some(creatures) = creatures.as_deref() else {
        return;
    };
    if session.looks.is_empty() {
        if let Some((class, ..)) = super::looks_ui::pick(&selection) {
            session.looks.insert(class, store.get(class).looks.clone());
            session.name = store.get(class).looks.name.clone();
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        cancel(
            &mut commands,
            &mut session,
            &mut selection,
            &mut store,
            &roots,
        );
        return;
    }
    if let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    {
        match action {
            Action::Cancel => {
                cancel(
                    &mut commands,
                    &mut session,
                    &mut selection,
                    &mut store,
                    &roots,
                );
                return;
            }
            Action::Accept => {
                if let Some((class, ..)) = super::looks_ui::pick(&selection) {
                    if let Some(name) = normalize_name(&session.name) {
                        store.edit(class, |g| g.looks.name = name);
                    }
                }
                close(&mut commands, &mut session, &roots);
                return;
            }
            Action::Race(r) => choose(&mut selection, creatures, Some(r), None, None),
            Action::Sex(s) => choose(&mut selection, creatures, None, Some(s), None),
            Action::Class(c) => choose_class(&mut selection, creatures, c),
            Action::Random => {
                let picks: Vec<_> = all_looks(creatures)
                    .iter()
                    .enumerate()
                    .flat_map(|(c, v)| (0..v.len()).map(move |l| (c, l)))
                    .collect();
                if !picks.is_empty() {
                    let (c, l) = picks[rng.index(picks.len())];
                    set(&mut selection, &all_looks(creatures), c, l);
                }
            }
            Action::RandomName => {
                if let Some((_, _, race, sex)) = super::looks_ui::pick(&selection) {
                    session.name = random_name(race, sex, &mut rng);
                }
            }
            Action::EditName => session.editing_name = true,
            Action::Step(i, dir) => step(
                &selection,
                cc.as_deref(),
                creatures,
                &mut store,
                &mut session,
                usize::from(i),
                dir,
            ),
            Action::Rotate(_) | Action::Model => {}
        }
        clamp_body(
            &selection,
            cc.as_deref(),
            creatures,
            &mut store,
            &mut session,
        );
    }
    if session.editing_name {
        for ev in keyboard.read() {
            if ev.state != ButtonState::Pressed {
                continue;
            }
            match ev.key_code {
                KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => {
                    session.editing_name = false
                }
                KeyCode::Backspace => {
                    session.name.pop();
                }
                _ => {
                    if let Some(text) = ev
                        .text
                        .as_deref()
                        .filter(|v| v.bytes().all(|b| b.is_ascii_alphabetic()))
                    {
                        if session.name.len() + text.len() <= NAME_MAX {
                            session.name.push_str(text);
                        }
                    }
                }
            }
        }
    }
    refresh(
        &selection,
        cc.as_deref(),
        creatures,
        strings.as_deref(),
        &store,
        &session,
        &mut captions,
        &mut reads,
        &mut names,
    );
}
fn choose(
    selection: &mut Selection,
    creatures: &Creatures,
    race: Option<u8>,
    sex: Option<u8>,
    class: Option<usize>,
) {
    let all = all_looks(creatures);
    let old = selection.look.and_then(|i| selection.looks.get(i)).copied();
    let r = race.or(old.map(|x| x.1));
    let s = sex.or(old.map(|x| x.2));
    for c in class.into_iter().chain(selection.class).chain(0..all.len()) {
        if let Some(l) = find(&all[c], r, s)
            .or_else(|| find(&all[c], r, None))
            .or_else(|| find(&all[c], None, s))
        {
            set(selection, &all, c, l);
            break;
        }
    }
}

/// Class buttons never coerce the race: an unavailable class is inert, as the stock create
/// screen omits it from that race's class list.
fn choose_class(selection: &mut Selection, creatures: &Creatures, class: usize) {
    let all = all_looks(creatures);
    let Some((_, race, sex)) = selection.look.and_then(|i| selection.looks.get(i)).copied() else {
        return;
    };
    if let Some(look) = find(&all[class], Some(race), Some(sex)) {
        set(selection, &all, class, look);
    }
}

fn step(
    selection: &Selection,
    cc: Option<&CharCreate>,
    creatures: &Creatures,
    store: &mut GearStore,
    session: &mut CreateSession,
    i: usize,
    dir: i8,
) {
    if let Some((class, trainer, race, sex)) = super::looks_ui::pick(selection) {
        snapshot(session, store, class);
        let (now, counts) = current(store.get(class), trainer, race, sex, cc, Some(creatures));
        store.edit(class, |g| {
            g.looks.set_features(race, sex, now.step(i, dir, counts))
        });
    }
}
fn clamp_body(
    selection: &Selection,
    cc: Option<&CharCreate>,
    creatures: &Creatures,
    store: &mut GearStore,
    session: &mut CreateSession,
) {
    if let Some((class, trainer, race, sex)) = super::looks_ui::pick(selection) {
        snapshot(session, store, class);
        let (now, counts) = current(store.get(class), trainer, race, sex, cc, Some(creatures));
        store.edit(class, |g| {
            g.looks.body = Some(Body { race, sex });
            g.looks.set_features(race, sex, now.clamped(counts));
        });
    }
}
fn snapshot(session: &mut CreateSession, store: &GearStore, class: usize) {
    session
        .looks
        .entry(class)
        .or_insert_with(|| store.get(class).looks.clone());
}
fn refresh(
    selection: &Selection,
    cc: Option<&CharCreate>,
    creatures: &Creatures,
    strings: Option<&GlueStrings>,
    store: &GearStore,
    session: &CreateSession,
    captions: &mut Query<(&Caption, &mut Text), (Without<Readout>, Without<Name>)>,
    reads: &mut Query<(&Readout, &mut Text), (Without<Caption>, Without<Name>)>,
    names: &mut Query<&mut Text, (With<Name>, Without<Caption>, Without<Readout>)>,
) {
    let Some((class, trainer, race, sex)) = super::looks_ui::pick(selection) else {
        return;
    };
    let (now, counts) = current(store.get(class), trainer, race, sex, cc, Some(creatures));
    let labels = feature_labels(strings, cc.map(|v| &v.0), race, sex);
    let values = now.to_array();
    for (tag, mut text) in captions {
        text.0 = labels[usize::from(tag.0)].clone();
    }
    for (tag, mut text) in reads {
        let i = usize::from(tag.0);
        text.0 = format!("{} / {}", values[i] + 1, counts[i]);
    }
    for mut text in names {
        text.0 = session.name.clone();
    }
}
fn close(
    commands: &mut Commands,
    session: &mut CreateSession,
    roots: &Query<Entity, With<CreateRoot>>,
) {
    session.open = false;
    session.selection = None;
    session.looks.clear();
    for e in roots {
        commands.entity(e).despawn();
    }
}
fn cancel(
    commands: &mut Commands,
    session: &mut CreateSession,
    selection: &mut Selection,
    store: &mut GearStore,
    roots: &Query<Entity, With<CreateRoot>>,
) {
    if let Some(saved) = session.selection.take() {
        *selection = saved;
    }
    for (class, looks) in std::mem::take(&mut session.looks) {
        store.edit(class, |g| g.looks = looks);
    }
    close(commands, session, roots);
}

pub(super) fn rotate(
    session: Res<CreateSession>,
    actions: Query<(&Action, &Interaction)>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut rig: ResMut<super::super::hero::CamRig>,
) {
    if !session.open {
        return;
    }
    for (action, state) in &actions {
        if *state != Interaction::Pressed {
            continue;
        }
        match action {
            Action::Model => {
                rig.orbit -= crate::glue::drag_yaw(motion.delta.x, window.single().ok())
            }
            Action::Rotate(d) => {
                rig.orbit += f32::from(*d) * crate::glue::ROTATE_RATE * time.delta_secs()
            }
            _ => {}
        }
    }
}
