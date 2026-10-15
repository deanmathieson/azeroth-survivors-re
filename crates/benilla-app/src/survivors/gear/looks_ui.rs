//! The Armory Character tab's Appearance panel, and the hero's name wherever it shows: a name box
//! with a Random button, then one row per feature the race and sex has, each with its
//! create-screen caption, ← / → arrows and a "3 / 9" readout, and Randomize under them. Every
//! change is saved to the class's [`super::GearSave::looks`] at once, and the menu's hero wears it
//! the same frame it is re-dressed ([`super::outfit::dress_bodies`]).
//!
//! The name box is the glue edit box over the engine's own text input ([`crate::textinput`]):
//! click it to type, letters only, at most 12; Enter or a click elsewhere ends the edit and Esc
//! drops it. While it has the focus, the keys it takes reach nothing else.

use benilla_ui::widget::EditBoxState;
use bevy::ecs::system::SystemParam;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use super::super::armory::{panel, ArmoryState, TabCtx};
use super::super::hud::{HudFont, MenuRoot};
use super::super::menu::MenuBuilt;
use super::super::{Rng, Selection};
use super::looks::{
    feature_counts, feature_labels, has_facial, normalize_name, random_name, worn_features,
    Features, FACIAL, FEATURES, NAME_MAX,
};
use super::store::{class_gear, GearRead, GearStore};
use crate::entities::{CharCreate, Creatures};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{
    dial_arrow, glue_button, glue_edit_box, outlined_text, paint_glue_field, GlueBtnKind,
    GlueFieldPart, GlueText,
};
use crate::glue::GlueClicks;
use crate::glue_strings::GlueStrings;
use crate::textinput;

/// The panel, top left of the Character tab's clear center, beside the hero.
const PANEL_W: f32 = 250.0;
const PANEL_H: f32 = 336.0;
const ROW_H: f32 = 32.0;
/// The name box's frame tints, the pick screen's zone frame.
const BOX_BORDER: Color = Color::srgb(0.8, 0.8, 0.8);
const BOX_FILL: Color = Color::srgba(0.09, 0.09, 0.09, 0.85);
/// `RED_FONT_COLOR`, a name the box will not keep.
const BAD: Color = Color::srgb(1.0, 0.1, 0.1);
/// The hint under the name box, the create screen's rule.
const NAME_RULE: &str = "Letters only, 2 to 12";

/// A click in the Appearance panel.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::survivors) enum LooksAction {
    /// Feature `.0` one step left (-1) or right (1).
    Step(u8, i8),
    Randomize,
    RandomName,
    /// Give the name box the focus.
    EditName,
}

/// A feature's row, hidden for a race that has no facial hair or equivalent.
#[derive(Component)]
pub(super) struct FeatureRow(u8);
#[derive(Component)]
pub(super) struct FeatureLabel(u8);
#[derive(Component)]
pub(super) struct FeatureCount(u8);
/// The name box itself, which takes the click.
#[derive(Component)]
pub(super) struct NameBox;
/// The name box's text runs and caret.
#[derive(Component, Clone)]
pub(super) struct NameField;
#[derive(Component)]
pub(super) struct NameHint;

/// A text that shows the picked class hero's name: the menu's line, the Character tab's title.
#[derive(Component)]
pub(in crate::survivors) struct HeroNameText;

/// The name box's state: the typed text, and whether it has the focus.
#[derive(Resource)]
pub(super) struct NameEdit {
    field: EditBoxState,
    focused: bool,
}

impl Default for NameEdit {
    fn default() -> Self {
        Self {
            field: textinput::field(NAME_MAX, false),
            focused: false,
        }
    }
}

/// Every class hero's saved name, for any screen that shows one.
#[derive(SystemParam)]
pub(in crate::survivors) struct HeroNames<'w> {
    gear: GearRead<'w>,
}

impl HeroNames<'_> {
    /// Class `class`'s hero name, `None` until it has one.
    pub(in crate::survivors) fn name(&self, class: usize) -> Option<&str> {
        self.gear.get(class).looks.name()
    }
}

/// Class `class`'s hero name, read straight from the world: the profile's
/// `classes.<class>.gear.looks.name`, for the Armory's builders and the character panel.
pub(in crate::survivors) fn hero_name(world: &World, class: usize) -> Option<&str> {
    class_gear(world, class).looks.name()
}

/// The pick: its class, trainer display, race and sex.
fn pick(selection: &Selection) -> Option<(usize, u32, u8, u8)> {
    let class = selection.class?;
    let &(display, race, sex) = selection.look.and_then(|i| selection.looks.get(i))?;
    Some((class, display, race, sex))
}

/// The features the pick wears now and each one's count.
fn current(
    gear: &super::GearSave,
    trainer_display: u32,
    race: u8,
    sex: u8,
    cc: Option<&CharCreate>,
    creatures: Option<&Creatures>,
) -> (Features, [u8; FEATURES]) {
    let cc = cc.map(|c| &c.0);
    let counts = feature_counts(cc, race, sex);
    let trainer = creatures.and_then(|c| c.npc_appearance(trainer_display));
    let f = worn_features(&gear.looks, race, sex, trainer.as_ref(), Some(counts));
    (f, counts)
}

/// The Appearance panel, built into the Character tab's center column.
pub(in crate::survivors) fn looks_panel(p: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    if pick(ctx.selection).is_none() {
        return;
    }
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        top: Val::Px(0.0),
        width: Val::Px(PANEL_W),
        height: Val::Px(PANEL_H),
        ..default()
    };
    panel(p, ctx, node, Some("Appearance"), |b| {
        b.spawn(Node {
            height: Val::Px(40.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|r| {
            glue_edit_box(
                r,
                ctx.art,
                ctx.font,
                (Button, LooksAction::EditName, NameBox),
                NameField,
                (138.0, 36.0),
                (BOX_BORDER, BOX_FILL),
                (12.0, 6.0, 0.0, 0.0),
                1.0,
            );
            glue_button(
                r,
                ctx.art,
                ctx.font,
                LooksAction::RandomName,
                "Random",
                72.0,
                30.0,
                GlueBtnKind::Small,
                1.0,
            );
        });
        outlined_text(
            b,
            Node {
                flex_shrink: 0.0,
                margin: UiRect::bottom(Val::Px(6.0)),
                ..default()
            },
            (),
            NameHint,
            GlueText {
                text: NAME_RULE,
                size: 10.0,
                color: DIM,
                wrap: false,
            },
            ctx.font,
            1.0,
        );
        for i in 0..FEATURES as u8 {
            feature_row(b, ctx, i);
        }
        b.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
        b.spawn(Node {
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|r| {
            glue_button(
                r,
                ctx.art,
                ctx.font,
                LooksAction::Randomize,
                "Randomize",
                146.0,
                30.0,
                GlueBtnKind::Small,
                1.0,
            );
        });
    });
}

/// One feature: its caption, the ← arrow, "n / count" and the → arrow, as the create screen's
/// dial row with the count between the arrows.
fn feature_row(b: &mut ChildSpawnerCommands, ctx: &TabCtx, i: u8) {
    b.spawn((
        FeatureRow(i),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(ROW_H),
            flex_shrink: 0.0,
            ..default()
        },
    ))
    .with_children(|row| {
        outlined_text(
            row,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                width: Val::Px(96.0),
                height: Val::Px(ROW_H),
                align_items: AlignItems::Center,
                ..default()
            },
            (),
            FeatureLabel(i),
            GlueText {
                text: "",
                size: 12.0,
                color: GOLD,
                wrap: false,
            },
            ctx.font,
            1.0,
        );
        dial_arrow(
            row,
            &ctx.art.arrow_left,
            ctx.font,
            LooksAction::Step(i, -1),
            98.0,
            "<",
            1.0,
        );
        outlined_text(
            row,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(130.0),
                width: Val::Px(52.0),
                height: Val::Px(ROW_H),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            (),
            FeatureCount(i),
            GlueText {
                text: "",
                size: 12.0,
                color: Color::WHITE,
                wrap: false,
            },
            ctx.font,
            1.0,
        );
        dial_arrow(
            row,
            &ctx.art.arrow_right,
            ctx.font,
            LooksAction::Step(i, 1),
            182.0,
            ">",
            1.0,
        );
    });
}

/// The hero's name over the pick screen's race and class line, added once the menu is built.
pub(super) fn add_menu_name(
    mut commands: Commands,
    roots: Query<Entity, (With<MenuRoot>, Added<MenuBuilt>)>,
    font: Res<HudFont>,
) {
    for root in &roots {
        commands.entity(root).with_children(|ui| {
            // Over `CharSelectCharacterName`'s line at BOTTOM (0,100).
            ui.spawn(Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(126.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|c| {
                outlined_text(
                    c,
                    Node::default(),
                    (),
                    HeroNameText,
                    GlueText {
                        text: "",
                        size: 26.0,
                        color: Color::WHITE,
                        wrap: false,
                    },
                    &font.0,
                    1.0,
                );
            });
        });
    }
}

/// The panel's clicks: a feature step, Randomize, a random name, or the name box taking the
/// focus. A click anywhere else ends an edit.
#[allow(clippy::too_many_arguments)]
pub(super) fn looks_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &LooksAction)>,
    boxes: Query<&Interaction, With<NameBox>>,
    mouse: Res<ButtonInput<MouseButton>>,
    armory: Res<ArmoryState>,
    selection: Res<Selection>,
    cc: Option<Res<CharCreate>>,
    creatures: Option<Res<Creatures>>,
    mut store: GearStore,
    mut rng: ResMut<Rng>,
    mut edit: ResMut<NameEdit>,
) {
    if !armory.open {
        edit.focused = false;
        return;
    }
    if edit.focused
        && mouse.just_pressed(MouseButton::Left)
        && boxes.iter().all(|i| *i == Interaction::None)
    {
        edit.focused = false;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    let Some((class, trainer, race, sex)) = pick(&selection) else {
        return;
    };
    let (now, counts) = current(
        store.get(class),
        trainer,
        race,
        sex,
        cc.as_deref(),
        creatures.as_deref(),
    );
    match action {
        LooksAction::EditName if !edit.focused => {
            let saved = store.get(class).looks.name.clone();
            edit.field.set_text(&saved);
            edit.focused = true;
        }
        LooksAction::EditName => {}
        LooksAction::RandomName => {
            let name = random_name(race, sex, &mut rng);
            edit.field.set_text(&name);
            edit.focused = false;
            store.edit(class, |g| g.looks.name = name);
        }
        LooksAction::Step(i, dir) => {
            let next = now.step(usize::from(i), dir, counts);
            if next != now {
                store.edit(class, |g| g.looks.set_features(race, sex, next));
            }
        }
        LooksAction::Randomize => {
            let mut next = Features::random(counts, &mut rng);
            if !has_facial(cc.as_deref().map(|c| &c.0), race, sex) {
                next.facial_hair = now.facial_hair;
            }
            store.edit(class, |g| g.looks.set_features(race, sex, next));
        }
    }
}

/// Typing into the focused name box, ahead of every other key reader: letters through the
/// engine's text input, Enter to end the edit, Esc to drop it. Each key the box takes is taken
/// from the frame's presses, so the camera's and the menu's keys do not also fire; a name that is
/// valid is saved as it is typed.
#[allow(clippy::too_many_arguments)]
pub(super) fn type_name(
    mut keyboard: MessageReader<KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut edit: ResMut<NameEdit>,
    armory: Option<Res<ArmoryState>>,
    selection: Res<Selection>,
    mut store: GearStore,
    mut clipboard: Option<NonSendMut<textinput::HostClipboard>>,
    layout: Option<Res<crate::bindings::LayoutNames>>,
    raw_handle: Query<&bevy::window::RawHandleWrapper, With<bevy::window::PrimaryWindow>>,
    time: Res<Time>,
) {
    if !edit.focused || !armory.is_some_and(|a| a.open) {
        keyboard.clear();
        return;
    }
    let Some((class, ..)) = pick(&selection) else {
        keyboard.clear();
        return;
    };
    textinput::tick_caret(&mut edit.field, true, time.delta_secs());
    let mods = textinput::mods_now(&keys);
    let wl = textinput::wayland_display(raw_handle.iter().next());
    let before = edit.field.text.clone();
    for ev in keyboard.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match ev.key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                edit.focused = false;
            }
            KeyCode::Escape => {
                let saved = store.get(class).looks.name.clone();
                edit.field.set_text(&saved);
                edit.focused = false;
            }
            _ => {
                let fed = match (clipboard.as_deref_mut(), layout.as_deref()) {
                    (Some(clip), Some(layout)) => textinput::feed_key(
                        &mut edit.field,
                        ev,
                        mods,
                        layout,
                        clip,
                        wl,
                        textinput::CharFilter::Letters,
                    ),
                    // No host input: plain letters only.
                    _ => match ev
                        .text
                        .as_deref()
                        .filter(|t| t.chars().all(|c| c.is_ascii_alphabetic()))
                    {
                        Some(t) => {
                            edit.field.insert(t);
                            textinput::FieldKey::Consumed
                        }
                        None => textinput::FieldKey::Passthrough,
                    },
                };
                if fed == textinput::FieldKey::Passthrough {
                    continue;
                }
            }
        }
        keys.clear_just_pressed(ev.key_code);
        if !edit.focused {
            break;
        }
    }
    if edit.field.text != before {
        if let Some(name) = normalize_name(&edit.field.text) {
            if store.get(class).looks.name != name {
                store.edit(class, |g| g.looks.name = name);
            }
        }
    }
    if !edit.focused {
        keyboard.clear();
    }
}

/// The panel's and the names' texts, from the pick, its looks and the tables: each feature's
/// caption and "n / count", the facial row hidden where the race has none, and the name in the
/// box (with its caret while typing) and wherever [`HeroNameText`] shows it.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_looks(
    selection: Res<Selection>,
    gear: GearRead,
    cc: Option<Res<CharCreate>>,
    creatures: Option<Res<Creatures>>,
    strings: Option<Res<GlueStrings>>,
    mut edit: ResMut<NameEdit>,
    fresh: Query<(), Or<(Added<FeatureRow>, Added<HeroNameText>, Added<NameField>)>>,
    mut rows: Query<(&FeatureRow, &mut Node)>,
    mut texts: ParamSet<(
        Query<(&FeatureLabel, &mut Text)>,
        Query<(&FeatureCount, &mut Text)>,
        Query<&mut Text, With<HeroNameText>>,
        Query<&mut TextColor, With<NameHint>>,
        Query<(&GlueFieldPart, Option<&mut Text>, &mut Visibility), With<NameField>>,
    )>,
) {
    let changed = selection.is_changed() || gear.is_changed() || !fresh.is_empty();
    let Some((class, trainer, race, sex)) = pick(&selection) else {
        if changed {
            for mut t in &mut texts.p2() {
                if !t.0.is_empty() {
                    t.0.clear();
                }
            }
        }
        return;
    };
    let saved = gear.get(class).looks.name().unwrap_or_default();
    // Out of an edit, the box shows the saved name.
    if !edit.focused && edit.field.text != saved {
        edit.field.set_text(saved);
    }
    if changed || edit.is_changed() {
        // Red while the typed name is one the box will not keep.
        let want = if edit.focused && normalize_name(&edit.field.text).is_none() {
            BAD
        } else {
            DIM
        };
        for mut color in &mut texts.p3() {
            if color.0 != want {
                color.0 = want;
            }
        }
        paint_glue_field(&edit.field, edit.focused, texts.p4().iter_mut());
    }
    if !changed {
        return;
    }
    for mut t in &mut texts.p2() {
        if t.0 != saved {
            t.0 = saved.to_string();
        }
    }
    let cat = cc.as_deref().map(|c| &c.0);
    let (now, counts) = current(
        gear.get(class),
        trainer,
        race,
        sex,
        cc.as_deref(),
        creatures.as_deref(),
    );
    let labels = feature_labels(strings.as_deref(), cat, race, sex);
    let facial = has_facial(cat, race, sex);
    for (row, mut node) in &mut rows {
        let show = usize::from(row.0) != FACIAL || facial;
        let want = if show { Display::Flex } else { Display::None };
        if node.display != want {
            node.display = want;
        }
    }
    for (label, mut t) in &mut texts.p0() {
        let want = &labels[usize::from(label.0).min(FEATURES - 1)];
        if t.0 != *want {
            t.0.clone_from(want);
        }
    }
    let values = now.to_array();
    for (count, mut t) in &mut texts.p1() {
        let i = usize::from(count.0).min(FEATURES - 1);
        let want = format!("{} / {}", values[i] + 1, counts[i]);
        if t.0 != want {
            t.0 = want;
        }
    }
}
