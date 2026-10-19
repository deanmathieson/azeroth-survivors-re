//! The survivors' small, stock-style Game Menu Options frame.  It deliberately writes the
//! engine's CVar registry rather than carrying a second settings file: CVar observers apply sound
//! and video changes immediately and the registry writes its normal config.toml when quiet.

use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use super::hud::{HudFont, MenuRoot, UiAction};
use super::menu::{MenuBuilt, MenuSlot};
use crate::cvars::Cvars;
use crate::glue::art::{GlueArt, GOLD, INFO_TEXT};
use crate::glue::backdrop::{backdrop_border, tiled_bg_node};
use crate::glue::widgets::{glue_button, outlined_text, GlueBtnKind, GlueText};
use crate::glue::GlueClicks;

const PANEL_W: f32 = 570.0;
const SOUND: &[Setting] = &[
    Setting::slider("Master", "MasterVolume"),
    Setting::slider("Effects", "SoundVolume"),
    Setting::slider("Music", "MusicVolume"),
    Setting::slider("Ambience", "AmbienceVolume"),
    Setting::check("Enable Music", "EnableMusic"),
    Setting::check("Enable Ambience", "EnableAmbience"),
    Setting::check("Sound in Background", "Sound_EnableSoundWhenGameIsInBG"),
];
const VIDEO: &[Setting] = &[
    Setting::check("Vertical Sync", "gxVSync"),
    Setting::windowed(),
    Setting::slider("View Distance", "farclip"),
];
const INTERFACE: &[Setting] = &[
    Setting::slider("Mouse Look Sensitivity", "mousespeed"),
    Setting::check("Invert Mouse Look", "mouseInvertPitch"),
    Setting::check("Quick picks", "survivorsQuickPicks"),
];
const KEYS: &[&str] = &[
    "WASD — move",
    "Space — jump",
    "V — classic / WoW camera",
    "C — character",
    "Tab — target",
    "1 / 2 / 3 — ability strip",
    "M — damage meter",
    "7 / 8 / 9 — toys",
    "Enter — chat",
    "Middle mouse — ping",
    "Esc — game menu",
];

#[derive(Clone, Copy)]
struct Setting {
    label: &'static str,
    cvar: &'static str,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Slider,
    Check,
    Windowed,
}
impl Setting {
    const fn slider(label: &'static str, cvar: &'static str) -> Self {
        Self {
            label,
            cvar,
            kind: Kind::Slider,
        }
    }
    const fn check(label: &'static str, cvar: &'static str) -> Self {
        Self {
            label,
            cvar,
            kind: Kind::Check,
        }
    }
    const fn windowed() -> Self {
        Self {
            label: "Windowed Mode",
            cvar: "gxWindow",
            kind: Kind::Windowed,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Sound,
    Video,
    Interface,
    Keys,
}
impl Page {
    const ALL: [Self; 4] = [Self::Sound, Self::Video, Self::Interface, Self::Keys];
    fn label(self) -> &'static str {
        match self {
            Self::Sound => "Sound",
            Self::Video => "Video",
            Self::Interface => "Interface",
            Self::Keys => "Key Bindings",
        }
    }
}
#[derive(Resource, Default)]
struct State {
    open: bool,
    page: Option<Page>,
}
#[derive(Component)]
struct Built;
#[derive(Component)]
struct Modal;
#[derive(Component, Clone, Copy)]
enum Action {
    Open,
    Close,
    Page(Page),
    Less(&'static str),
    More(&'static str),
    Toggle(&'static str),
}
#[derive(Component)]
struct Body;

type MenuBuildRoot = (With<MenuRoot>, With<MenuBuilt>, Without<Built>);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<State>().add_systems(
        Update,
        (build, input, redraw).chain().before(super::hud::keys),
    );
}

fn build(
    mut commands: Commands,
    roots: Query<Entity, MenuBuildRoot>,
    buttons: Query<(Entity, &MenuSlot)>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
) {
    let (Ok(root), Some(buttons)) = (
        roots.single(),
        buttons
            .iter()
            .find_map(|(entity, slot)| (*slot == MenuSlot::Options).then_some(entity)),
    ) else {
        return;
    };
    commands
        .entity(root)
        .insert(Built)
        .with_children(|p| modal(p, &art, &font.0));
    commands.entity(buttons).with_children(|p| {
        glue_button(
            p,
            &art,
            &font.0,
            Action::Open,
            "Options",
            super::menu::BAR_BTN_W,
            super::menu::BAR_BTN_H,
            GlueBtnKind::Small,
            1.0,
        );
    });
}

fn modal(p: &mut ChildSpawnerCommands, art: &GlueArt, font: &Handle<Font>) {
    p.spawn((
        Modal,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
        FocusPolicy::Block,
        ZIndex(430),
    ))
    .with_children(|dim| {
        dim.spawn(Node {
            width: Val::Px(PANEL_W),
            min_height: Val::Px(430.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(26.0)),
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|panel| {
            if let (Some(bg), Some(border)) = (&art.dialog_bg, &art.dialog_border) {
                panel.spawn((
                    tiled_bg_node(bg.clone(), 32.0, 1.0, Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(11.0),
                        right: Val::Px(12.0),
                        top: Val::Px(12.0),
                        bottom: Val::Px(11.0),
                        ..default()
                    },
                ));
                backdrop_border(panel, border, 32.0, Color::WHITE);
            }
            outlined_text(
                panel,
                Node::default(),
                (),
                (),
                GlueText {
                    text: "Options",
                    size: 22.0,
                    color: GOLD,
                    wrap: false,
                },
                font,
                1.0,
            );
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|tabs| {
                    for page in Page::ALL {
                        glue_button(
                            tabs,
                            art,
                            font,
                            Action::Page(page),
                            page.label(),
                            116.0,
                            30.0,
                            GlueBtnKind::Small,
                            1.0,
                        );
                    }
                });
            panel.spawn((
                Body,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(5.0),
                    padding: UiRect::vertical(Val::Px(10.0)),
                    ..default()
                },
            ));
            glue_button(
                panel,
                art,
                font,
                Action::Close,
                "Okay",
                150.0,
                34.0,
                GlueBtnKind::Normal,
                1.0,
            );
        });
    });
}

fn input(
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    hud: Query<(Entity, &UiAction)>,
    actions: Query<(Entity, &Action)>,
    mut state: ResMut<State>,
    mut cvars: ResMut<Cvars>,
) {
    if !state.open
        && hud
            .iter()
            .any(|(e, a)| *a == UiAction::Options && clicks.hit(e))
    {
        state.open = true;
        state.page = Some(Page::Sound);
        return;
    }
    if state.open && keys.just_pressed(KeyCode::Escape) {
        state.open = false;
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    match action {
        Action::Open => {
            state.open = true;
            state.page = Some(Page::Sound);
        }
        Action::Close => state.open = false,
        Action::Page(p) => state.page = Some(p),
        Action::Toggle(name) => {
            let on = cvars.get(name).is_some_and(|v| v != "0");
            // `gxWindow` is inverted from the user-facing checkbox: zero is fullscreen.
            cvars.set(
                name,
                if name == "gxWindow" {
                    if on {
                        "0"
                    } else {
                        "1"
                    }
                } else if on {
                    "0"
                } else {
                    "1"
                },
            );
        }
        Action::Less(name) | Action::More(name) => {
            let now = cvars
                .get(name)
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            let step = if name == "farclip" { 25.0 } else { 0.05 };
            let delta = if matches!(action, Action::Less(_)) {
                -step
            } else {
                step
            };
            cvars.set(name, &format_value(name, now + delta));
        }
    }
}
fn format_value(name: &str, value: f32) -> String {
    if name == "farclip" {
        value.clamp(50.0, 777.0).round().to_string()
    } else {
        format!("{:.2}", value.clamp(0.0, 1.0))
    }
}

fn redraw(
    mut commands: Commands,
    state: Res<State>,
    mut modals: Query<&mut Node, With<Modal>>,
    bodies: Query<Entity, With<Body>>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
    cvars: Res<Cvars>,
) {
    let Ok(mut node) = modals.single_mut() else {
        return;
    };
    let display = if state.open {
        Display::Flex
    } else {
        Display::None
    };
    if node.display != display {
        node.display = display;
    }
    if !state.is_changed() && !cvars.is_changed() {
        return;
    }
    let Ok(body) = bodies.single() else { return };
    commands.entity(body).despawn_related::<Children>();
    commands
        .entity(body)
        .with_children(|p| match state.page.unwrap_or(Page::Sound) {
            Page::Keys => {
                for key in KEYS {
                    outlined_text(
                        p,
                        Node::default(),
                        (),
                        (),
                        GlueText {
                            text: key,
                            size: 15.0,
                            color: Color::WHITE,
                            wrap: false,
                        },
                        &font.0,
                        1.0,
                    );
                }
            }
            page => {
                for setting in settings(page) {
                    row(p, &art, &font.0, setting, &cvars);
                }
            }
        });
}
fn settings(page: Page) -> &'static [Setting] {
    match page {
        Page::Sound => SOUND,
        Page::Video => VIDEO,
        Page::Interface => INTERFACE,
        Page::Keys => &[],
    }
}
fn row(
    p: &mut ChildSpawnerCommands,
    art: &GlueArt,
    font: &Handle<Font>,
    setting: &Setting,
    cvars: &Cvars,
) {
    p.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Px(34.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|r| {
        outlined_text(
            r,
            Node {
                width: Val::Px(210.0),
                ..default()
            },
            (),
            (),
            GlueText {
                text: setting.label,
                size: 14.0,
                color: GOLD,
                wrap: false,
            },
            font,
            1.0,
        );
        match setting.kind {
            Kind::Slider => {
                glue_button(
                    r,
                    art,
                    font,
                    Action::Less(setting.cvar),
                    "<",
                    34.0,
                    26.0,
                    GlueBtnKind::Small,
                    1.0,
                );
                let value = slider_text(setting.cvar, cvars);
                outlined_text(
                    r,
                    Node {
                        width: Val::Px(90.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    (),
                    (),
                    GlueText {
                        text: &value,
                        size: 14.0,
                        color: INFO_TEXT,
                        wrap: false,
                    },
                    font,
                    1.0,
                );
                glue_button(
                    r,
                    art,
                    font,
                    Action::More(setting.cvar),
                    ">",
                    34.0,
                    26.0,
                    GlueBtnKind::Small,
                    1.0,
                );
            }
            Kind::Check | Kind::Windowed => {
                let raw = cvars.get(setting.cvar).is_some_and(|v| v != "0");
                let on = if setting.kind == Kind::Windowed {
                    !raw
                } else {
                    raw
                };
                glue_button(
                    r,
                    art,
                    font,
                    Action::Toggle(setting.cvar),
                    if on { "[x]" } else { "[ ]" },
                    58.0,
                    26.0,
                    GlueBtnKind::Small,
                    1.0,
                );
            }
        }
    });
}
fn slider_text(name: &str, cvars: &Cvars) -> String {
    let value = cvars
        .get(name)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or_default();
    if name == "farclip" {
        format!("{} yd", value.round())
    } else {
        format!("{}%", (value.clamp(0.0, 1.0) * 100.0).round())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sliders_clamp_to_their_cvar_ranges() {
        assert_eq!(format_value("MusicVolume", 2.0), "1.00");
        assert_eq!(format_value("MusicVolume", -2.0), "0.00");
        assert_eq!(format_value("farclip", 1.0), "50");
    }
    #[test]
    fn panel_starts_closed() {
        assert!(!State::default().open);
    }
}
