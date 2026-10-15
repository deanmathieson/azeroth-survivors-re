//! The mode's screens, in bevy_ui since FrameXML is off: the class and hero pick, the in-run HUD,
//! the level-up cards, pause, and the results.

use std::collections::HashMap;

use benilla_ui::script::nameplate;
use benilla_world::view::WorldCamera;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};
use bevy::window::PrimaryWindow;

use super::data::{self, AbilityDef, Effect, Kind, PassiveDef, Stat};
use super::hero::{self, Hero};
use super::minimap::{self, MiniMap};
use super::party;
use super::profile::{LastRunNews, Profile};
use super::units::Marks;
use super::{
    HeroState, LocalHero, MetaBonuses, Overcharge, Owned, Phase, Rng, Run, Selection, Stats,
};

const GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
/// The mobility button's edge, an action button's 36 units.
const MOBILITY_SIZE: f32 = 40.0;
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const BUTTON: Color = Color::srgba(0.18, 0.06, 0.04, 0.95);
const BUTTON_HOT: Color = Color::srgba(0.36, 0.12, 0.06, 0.98);
const DIM: Color = Color::srgb(0.6, 0.6, 0.6);
/// The results screen's news line.
const NEWS_GREEN: Color = Color::srgb(0.25, 0.9, 0.35);
pub(super) const DIM_GREY: Color = DIM;
pub(super) const GOLD_TONE: Color = GOLD;
pub(super) const EDGE_TONE: Color = EDGE;

#[derive(Component)]
pub(super) struct MenuRoot;
#[derive(Component)]
pub(super) struct HudRoot;
#[derive(Component)]
pub(super) struct LevelUpRoot;
#[derive(Component)]
pub(super) struct PauseRoot;
#[derive(Component)]
pub(super) struct OverRoot;
#[derive(Component)]
pub(super) struct XpBar;
#[derive(Component)]
pub(super) struct XpFill;
#[derive(Component)]
pub(super) struct XpText;
#[derive(Component)]
pub(super) struct LevelText;
#[derive(Component)]
pub(super) struct TimerText;
#[derive(Component)]
pub(super) struct KillText;
#[derive(Component)]
pub(super) struct HpFrame;
#[derive(Component)]
pub(super) struct HpBar;
#[derive(Component)]
pub(super) struct HpLevelSeat;
#[derive(Component)]
pub(super) struct HpLevel;
#[derive(Component)]
pub(super) struct HpFill;
#[derive(Component)]
pub(super) struct ShieldFill;
#[derive(Component)]
pub(super) struct BannerText;
#[derive(Component)]
pub(super) struct AbilityRow;
#[derive(Component)]
pub(super) struct PassiveRow;
#[derive(Component)]
pub(super) struct ToyRow;
#[derive(Component)]
pub(super) struct OfferRow;
/// The line under LEVEL UP: which level this is.
#[derive(Component)]
pub(super) struct LevelUpSub;

/// A radial cooldown sweep over one ability icon: `sweep` is the remaining fraction
/// (1.0 = just cast, 0.0 = ready), drawn by `ui_sweep.wgsl`.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct SweepUiMaterial {
    #[uniform(0)]
    pub(super) sweep: f32,
}

impl UiMaterial for SweepUiMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://benilla_app/shaders/ui_sweep.wgsl".into()
    }
}

/// The cooldown overlay on one ability slot, driven every frame.
#[derive(Component)]
pub(super) struct AbilitySweep {
    handle: Handle<SweepUiMaterial>,
    index: usize,
}

/// The class mobility's action button, bottom centre over the XP bar; shown once it is learned.
#[derive(Component)]
pub(super) struct MobilitySlot;
#[derive(Component)]
pub(super) struct MobilityIcon;
#[derive(Component)]
pub(super) struct MobilitySweep(Handle<SweepUiMaterial>);
#[derive(Component)]
pub(super) struct OverTitle;
#[derive(Component)]
pub(super) struct OverStats;
#[derive(Component)]
pub(super) struct OverLine;
#[derive(Component)]
pub(super) struct OverNews;
/// The results screen's column of [`ResultsNotes`] lines, and one line in it.
#[derive(Component)]
pub(super) struct OverNotes;
#[derive(Component)]
pub(super) struct OverNote(usize);
#[derive(Component)]
pub(super) struct OverStat(usize);

/// The most [`ResultsNotes`] the results screen shows.
pub(super) const MAX_NOTES: usize = 4;

/// How a results note reads, by its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoteKind {
    /// A plain line, in parchment.
    // Nothing pushes a plain note yet: achievements and loot colour theirs.
    #[allow(dead_code)]
    Info,
    /// Good news, green like the news line.
    Good,
    /// An item, in its quality's colour (Epic purple).
    Loot,
    Achievement,
}

impl NoteKind {
    pub(super) fn color(self) -> Color {
        match self {
            Self::Info => Color::srgb(0.85, 0.78, 0.6),
            Self::Good => NEWS_GREEN,
            Self::Loot => super::cards::EPIC,
            Self::Achievement => GOLD,
        }
    }
}

/// The run's lines for the results screen, in the order pushed: features push during the run (a
/// drop) or on [`super::profile::RunEnded`] (an achievement). Cleared as each run starts; the
/// first [`MAX_NOTES`] show, under the news line.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(super) struct ResultsNotes(pub Vec<(String, NoteKind)>);

impl ResultsNotes {
    pub(super) fn push(&mut self, text: impl Into<String>, kind: NoteKind) {
        self.0.push((text.into(), kind));
    }
}

/// [`ResultsNotes`], emptied as each run begins; [`refresh_notes`] shows them.
pub(super) fn notes_plugin(app: &mut App) {
    app.init_resource::<ResultsNotes>().add_systems(
        OnTransition {
            exited: Phase::Menu,
            entered: Phase::Playing,
        },
        clear_notes,
    );
}

/// A run begins: the last run's notes go.
fn clear_notes(mut notes: ResMut<ResultsNotes>) {
    notes.0.clear();
}

/// An icon slot filled from the spell's `SpellIcon` once the client data is up.
#[derive(Component)]
pub(super) struct SpellIcon(pub(super) u32);
#[derive(Component)]
pub(super) struct IconDone;

/// A FrameXML `Texture` region: a client art file, its `TexCoords` (left, right, top, bottom) and
/// vertex colour, drawn once the archives are readable.
#[derive(Component)]
pub(super) struct UiArt {
    path: &'static str,
    coords: [f32; 4],
    tint: Color,
}

impl UiArt {
    pub(super) fn crop(path: &'static str, coords: [f32; 4]) -> Self {
        Self {
            path,
            coords,
            tint: Color::WHITE,
        }
    }

    pub(super) fn tinted(path: &'static str, tint: Color) -> Self {
        Self {
            path,
            coords: [0.0, 1.0, 0.0, 1.0],
            tint,
        }
    }
}

#[derive(Component)]
pub(super) struct ArtDone;

/// What an icon slot's tooltip describes: an index into the run's abilities or passives.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TipFor {
    Ability(usize),
    Passive(usize),
    Mobility,
}

/// The one `GameTooltip`, and the column its lines go in.
#[derive(Component)]
pub(super) struct GameTooltip;
#[derive(Component)]
pub(super) struct TipBody;

#[derive(Component, Clone, Copy, PartialEq)]
pub(super) enum UiAction {
    Resume,
    /// From the results: the next run at once, on the same pick ([`Replay`]).
    PlayAgain,
    MainMenu,
    Quit,
}

/// "Play again" on the results: the menu the run goes back through starts the next one at once, on
/// the same pick, as Enter World would.
#[derive(Resource, Default)]
pub(super) struct Replay(pub(super) bool);

/// The colour a button returns to when not hovered.
#[derive(Component)]
pub(super) struct BaseColor(Color);

#[derive(Clone, Copy)]
pub(super) enum Choice {
    NewAbility(&'static AbilityDef),
    Upgrade(usize),
    NewPassive(&'static PassiveDef),
    UpPassive(usize),
    Overcharge(Overcharge),
    Heal,
}

/// The level-up cards on offer.
#[derive(Resource, Default)]
pub(super) struct Offer {
    choices: Vec<Choice>,
    /// The one slot a banish dealt again; the rest of the hand stays where it landed.
    redealt: Option<usize>,
}

impl Offer {
    pub(super) fn len(&self) -> usize {
        self.choices.len()
    }

    pub(super) fn choices(&self) -> &[Choice] {
        &self.choices
    }

    pub(super) fn redealt(&self) -> Option<usize> {
        self.redealt
    }

    /// A whole new hand.
    pub(super) fn deal(&mut self, choices: Vec<Choice>) {
        self.choices = choices;
        self.redealt = None;
    }

    /// Slot `i` dealt again with `choice`, or dropped when nothing is left to offer.
    pub(super) fn redeal(&mut self, i: usize, choice: Option<Choice>) {
        match choice {
            Some(c) if i < self.choices.len() => {
                self.choices[i] = c;
                self.redealt = Some(i);
            }
            _ => {
                if i < self.choices.len() {
                    self.choices.remove(i);
                }
                if self.choices.is_empty() {
                    self.choices.push(Choice::Heal);
                }
                self.redealt = None;
            }
        }
    }
}

#[derive(Resource)]
pub(super) struct HudFont(pub(super) Handle<Font>);

pub(super) fn rgb(hex: u32) -> Color {
    Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn text(font: &Handle<Font>, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
        TextShadow {
            offset: Vec2::splat(1.5),
            color: Color::BLACK,
        },
    )
}

pub(super) fn text_bundle(
    font: &Handle<Font>,
    s: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    text(font, s, size, color)
}

pub(super) fn centered_bundle(
    font: &Handle<Font>,
    s: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    centered(font, s, size, color)
}

fn centered(font: &Handle<Font>, s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        text(font, s, size, color),
        TextLayout::new_with_justify(Justify::Center),
    )
}

fn button(action: UiAction, base: Color) -> impl Bundle {
    edged_button(action, base, EDGE)
}

fn edged_button(action: UiAction, base: Color, edge: Color) -> impl Bundle {
    (
        Button,
        action,
        BaseColor(base),
        BackgroundColor(base),
        BorderColor::all(edge),
    )
}

fn icon(spell: u32, size: f32) -> impl Bundle {
    (
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            ..default()
        },
        ImageNode {
            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            ..default()
        },
        SpellIcon(spell),
    )
}

fn spell_name(spells: Option<&crate::ui_action::Spells>, spell: u32, fallback: &str) -> String {
    spells
        .and_then(|s| s.catalog.get(spell))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| fallback.replace('_', " "))
}

pub(super) fn setup_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut sweeps_init: ResMut<Assets<SweepUiMaterial>>,
) {
    let font = crate::char_select::wow_font(&asset_server);
    let number_font: Handle<Font> = asset_server.load("mpq://Fonts/ARIALN.ttf");
    commands.insert_resource(HudFont(font.clone()));
    let f = &font;
    let map = MiniMap::new(&mut images);

    // --- The in-run HUD. ---
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(400),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            // `MainMenuExpBar`, 1024×13 at the bottom centre. The reference stacks it on the
            // 40-unit action bar; with no action bar here it rests on the screen's edge.
            root.spawn((
                XpBar,
                Interaction::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(0.0),
                    margin: UiRect::left(Val::Px(-512.0)),
                    width: Val::Px(1024.0),
                    height: Val::Px(13.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    XpFill,
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    UiArt::tinted(
                        "Interface\\TargetingFrame\\UI-StatusBar",
                        Color::srgb(0.58, 0.0, 0.55),
                    ),
                ));
                // `MainMenuXPBarTexture0`..`3`: the segment rules, 256×10 on the bar's top.
                for (i, top) in [0.792_968_75, 0.542_968_75, 0.292_968_75, 0.042_968_75]
                    .into_iter()
                    .enumerate()
                {
                    bar.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(256.0 * i as f32),
                            top: Val::Px(0.0),
                            width: Val::Px(256.0),
                            height: Val::Px(10.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        UiArt::crop(
                            "Interface\\MainMenuBar\\UI-MainMenuBar-Dwarf",
                            [0.0, 1.0, top, top + 0.039_062_5],
                        ),
                    ));
                }
                // `MainMenuBarExpText`, `TextStatusBarText` centred one unit up, shown on hover.
                bar.spawn(Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    top: Val::Px(-1.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|n| {
                    n.spawn((
                        XpText,
                        Text::new(""),
                        TextFont {
                            font: number_font.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow {
                            offset: Vec2::new(1.0, 1.0),
                            color: Color::BLACK,
                        },
                        TextLayout::new(Justify::Center, LineBreak::NoWrap),
                        Visibility::Hidden,
                    ));
                });
            });
            minimap::spawn_cluster(root, f, &map);
            // The mobility button: a 36-unit action slot in `UI-Quickslot2`, its key over it.
            root.spawn((
                MobilitySlot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(24.0),
                    margin: UiRect::left(Val::Px(-MOBILITY_SIZE / 2.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(2.0),
                    display: Display::None,
                    ..default()
                },
            ))
            .with_children(|col| {
                col.spawn(centered(f, "F", 18.0, Color::WHITE));
                col.spawn((
                    Node {
                        width: Val::Px(MOBILITY_SIZE),
                        height: Val::Px(MOBILITY_SIZE),
                        ..default()
                    },
                    Interaction::default(),
                    TipFor::Mobility,
                ))
                .with_children(|slot| {
                    // `UI-Quickslot2` is the empty slot's 64×64 frame, the button 36 inside it.
                    slot.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-MOBILITY_SIZE * 0.42),
                            top: Val::Px(-MOBILITY_SIZE * 0.42),
                            width: Val::Px(MOBILITY_SIZE * 1.84),
                            height: Val::Px(MOBILITY_SIZE * 1.84),
                            ..default()
                        },
                        ImageNode {
                            color: Color::NONE,
                            ..default()
                        },
                        UiArt::crop("Interface\\Buttons\\UI-Quickslot2", [0.0, 1.0, 0.0, 1.0]),
                    ));
                    slot.spawn((
                        MobilityIcon,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                            ..default()
                        },
                    ));
                    let handle = sweeps_init.add(SweepUiMaterial { sweep: 0.0 });
                    slot.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        MaterialNode(handle.clone()),
                        MobilitySweep(handle),
                    ));
                });
            });
            root.spawn(Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|row| {
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    width: Val::Px(300.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((
                        AbilityRow,
                        Node {
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));
                    col.spawn((
                        PassiveRow,
                        Node {
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));
                    col.spawn((
                        ToyRow,
                        Node {
                            column_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));
                });
                // The clock, with the level and the kill count on one line under it.
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(0.0),
                    ..default()
                })
                .with_children(|col| {
                    col.spawn((TimerText, centered(f, "00:00", 34.0, Color::WHITE)));
                    col.spawn(Node {
                        column_gap: Val::Px(14.0),
                        ..default()
                    })
                    .with_children(|line| {
                        line.spawn((LevelText, text(f, "Level 1", 14.0, GOLD)));
                        line.spawn((KillText, text(f, "Kills 0", 14.0, Color::WHITE)));
                        line.spawn((super::treasure::GoldText, text(f, "Gold 0", 14.0, GOLD)));
                    });
                });
                // The minimap cluster's column.
                row.spawn(Node {
                    width: Val::Px(300.0),
                    ..default()
                });
            });
            root.spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(22.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|n| {
                n.spawn((BannerText, centered(f, "", 30.0, GOLD)));
            });
            // The hero's health as a nameplate (`CGNamePlateFrame`): the bar under the border that
            // caps it, the level in the border's seat; [`hero_plate`] sizes and seats it.
            let absolute = || Node {
                position_type: PositionType::Absolute,
                ..default()
            };
            let unpainted = || ImageNode {
                color: Color::NONE,
                ..default()
            };
            let [r, g, b, _] = crate::vplates::PLATE_FRIENDLY;
            root.spawn((HpFrame, absolute(), Visibility::Hidden))
                .with_children(|plate| {
                    plate.spawn((HpBar, absolute())).with_children(|bar| {
                        bar.spawn((
                            HpFill,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            unpainted(),
                            UiArt::tinted(nameplate::BAR_FILL_TEXTURE, Color::srgb(r, g, b)),
                        ));
                        bar.spawn((
                            ShieldFill,
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..absolute()
                            },
                            unpainted(),
                            UiArt::tinted(
                                nameplate::BAR_FILL_TEXTURE,
                                Color::srgba(0.85, 0.9, 1.0, 0.75),
                            ),
                        ));
                    });
                    plate.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..absolute()
                        },
                        unpainted(),
                        UiArt::crop(nameplate::BORDER_TEXTURE, [0.0, 1.0, 0.0, 1.0]),
                    ));
                    let [r, g, b, _] = crate::vplates::CON_YELLOW;
                    plate
                        .spawn((
                            HpLevelSeat,
                            Node {
                                width: Val::Px(0.0),
                                height: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                overflow: Overflow::visible(),
                                ..absolute()
                            },
                        ))
                        .with_children(|seat| {
                            seat.spawn((
                                HpLevel,
                                Text::new("1"),
                                TextFont {
                                    font: f.clone(),
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(r, g, b)),
                                TextShadow {
                                    offset: Vec2::new(1.0, 1.0),
                                    color: Color::BLACK,
                                },
                                TextLayout::new(Justify::Center, LineBreak::NoWrap),
                            ));
                        });
                });
        });
    commands.insert_resource(map);

    // --- The run's pick screen; [`super::menu`] builds into it once the glue art is in. ---
    commands.spawn((
        MenuRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        GlobalZIndex(410),
        bevy::ui::FocusPolicy::Pass,
    ));

    // --- Level up. ---
    commands
        .spawn((
            LevelUpRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            // A dark vignette closing in on the cards.
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Color::srgba(0.04, 0.025, 0.0, 0.72), 0.0),
                    ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.9), 100.0),
                ],
            )),
            GlobalZIndex(420),
            Visibility::Hidden,
            // The build's icons stay hoverable under the cards.
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            // The slow golden burst behind everything.
            root.spawn((
                super::cards::Rays,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(1400.0),
                    height: Val::Px(1400.0),
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    margin: UiRect::new(
                        Val::Px(-700.0),
                        Val::Px(0.0),
                        Val::Px(-700.0),
                        Val::Px(0.0),
                    ),
                    ..default()
                },
                BackgroundGradient(vec![
                    Gradient::Conic(ConicGradient::new(
                        UiPosition::CENTER,
                        (0..24)
                            .map(|i| {
                                let a = i as f32 / 24.0 * std::f32::consts::TAU;
                                let on = i % 2 == 0;
                                AngularColorStop::new(
                                    if on {
                                        Color::srgba(1.0, 0.8, 0.3, 0.07)
                                    } else {
                                        Color::srgba(1.0, 0.8, 0.3, 0.0)
                                    },
                                    a,
                                )
                            })
                            .collect(),
                    )),
                    Gradient::Radial(RadialGradient::new(
                        UiPosition::CENTER,
                        RadialGradientShape::ClosestSide,
                        vec![
                            ColorStop::percent(Color::srgba(1.0, 0.75, 0.25, 0.22), 0.0),
                            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.0), 100.0),
                        ],
                    )),
                ]),
                UiTransform::default(),
                bevy::ui::FocusPolicy::Pass,
            ));
            root.spawn(centered(f, "LEVEL UP", 52.0, GOLD));
            root.spawn((
                LevelUpSub,
                centered(f, "Choose your power", 16.0, Color::srgb(0.85, 0.78, 0.6)),
            ));
            root.spawn((
                OfferRow,
                Node {
                    column_gap: Val::Px(22.0),
                    // Room for a hovered card's lift.
                    margin: UiRect::new(Val::ZERO, Val::ZERO, Val::Px(28.0), Val::Px(16.0)),
                    ..default()
                },
            ));
            super::hand::spawn_tools(root, f);
            root.spawn(centered(f, "Click a card, or press 1, 2 or 3", 14.0, DIM));
        });

    // --- Pause. ---
    commands
        .spawn((
            PauseRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            GlobalZIndex(430),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            root.spawn(centered(f, "Paused", 46.0, GOLD));
            for (label, action) in [
                ("Resume", UiAction::Resume),
                ("Abandon run", UiAction::MainMenu),
                ("Quit game", UiAction::Quit),
            ] {
                root.spawn((
                    button(action, BUTTON),
                    Node {
                        width: Val::Px(240.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    b.spawn(text(f, label, 20.0, Color::WHITE));
                });
            }
        });

    // --- Results. ---
    commands
        .spawn((
            OverRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundGradient::from(RadialGradient::new(
                UiPosition::CENTER,
                RadialGradientShape::FarthestCorner,
                vec![
                    ColorStop::percent(Color::srgba(0.07, 0.02, 0.0, 0.8), 0.0),
                    ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.94), 100.0),
                ],
            )),
            GlobalZIndex(440),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((OverTitle, centered(f, "", 64.0, GOLD)));
            root.spawn((
                OverLine,
                centered(f, "", 18.0, Color::srgb(0.85, 0.78, 0.6)),
            ));
            root.spawn((OverNews, centered(f, "", 18.0, NEWS_GREEN)));
            // The run's notes, tight under the news; hidden lines leave the layout.
            root.spawn((
                OverNotes,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(2.0),
                    display: Display::None,
                    ..default()
                },
            ))
            .with_children(|notes| {
                for i in 0..MAX_NOTES {
                    notes.spawn((
                        OverNote(i),
                        centered(f, "", 16.0, Color::WHITE),
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ));
                }
            });
            // The run in numbers, one tile each.
            root.spawn(Node {
                column_gap: Val::Px(12.0),
                margin: UiRect::vertical(Val::Px(14.0)),
                ..default()
            })
            .with_children(|row| {
                for (i, label) in [
                    "Survived", "Level", "Kills", "Damage", "Bosses", "Gold", "Elites", "Caches",
                ]
                .into_iter()
                .enumerate()
                {
                    row.spawn((
                        Node {
                            width: Val::Px(112.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(12.0)),
                            row_gap: Val::Px(4.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundGradient::from(LinearGradient::to_bottom(vec![
                            ColorStop::percent(Color::srgba(0.2, 0.14, 0.05, 0.92), 0.0),
                            ColorStop::percent(Color::srgba(0.04, 0.03, 0.02, 0.94), 100.0),
                        ])),
                        BorderColor::all(EDGE),
                    ))
                    .with_children(|tile| {
                        tile.spawn((OverStat(i), centered(f, "", 26.0, Color::WHITE)));
                        tile.spawn(centered(f, label, 12.0, DIM));
                    });
                }
            });
            root.spawn((
                super::run_options::ResultsRewards,
                centered(f, "", 18.0, super::run_options::GREEN),
            ));
            root.spawn((OverStats, centered(f, "", 15.0, DIM)));
            root.spawn((
                edged_button(UiAction::PlayAgain, BUTTON, GOLD),
                Node {
                    width: Val::Px(260.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(11.0)),
                    margin: UiRect::top(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BoxShadow::new(
                    Color::srgba(1.0, 0.7, 0.2, 0.35),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                    Val::Px(16.0),
                ),
            ))
            .with_children(|b| {
                b.spawn(text(f, "Play again", 22.0, GOLD));
            });
            root.spawn((
                button(UiAction::MainMenu, BUTTON),
                Node {
                    width: Val::Px(260.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
            ))
            .with_children(|b| {
                b.spawn(text(f, "Main menu", 20.0, Color::WHITE));
            });
            root.spawn((
                button(UiAction::Quit, Color::NONE),
                Node {
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                    ..default()
                },
            ))
            .with_children(|b| {
                b.spawn(text(f, "Quit game", 15.0, DIM));
            });
        });
}

/// The menu's hero is the run's hero: respawned whenever the pick changes, dressed in the class's
/// gear and looks ([`super::gear::Dresser`]), which re-dress it in place as they change.
#[allow(clippy::too_many_arguments)]
pub(super) fn menu_preview(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    selection: Res<Selection>,
    mut run: ResMut<Run>,
    heroes: Query<Entity, With<Hero>>,
    next: Res<NextState<Phase>>,
    collide: Option<benilla_world::collision::WorldCollision>,
    world: benilla_world::world_point::WorldPoint,
    mut dresser: super::gear::Dresser,
    mut shown: Local<Option<(usize, u32, usize, u32)>>,
) {
    if *phase.get() != Phase::Menu {
        return;
    }
    // Replay has already reset the local hero and queued Playing on this entry frame.
    if matches!(
        *next,
        NextState::Pending(Phase::Playing) | NextState::PendingIfNeq(Phase::Playing)
    ) {
        return;
    }
    let want = selection
        .class
        .zip(selection.look.and_then(|i| selection.looks.get(i)).copied());
    let key = want.map(|(c, l)| (c, l.0, selection.stage, selection.version));
    if key == *shown && (key.is_none() || !heroes.is_empty()) {
        return;
    }
    for e in &heroes {
        commands.entity(e).despawn();
    }
    *shown = key;
    let Some((class, (trainer, race, sex))) = want else {
        return;
    };
    let display = dresser.display(class, (trainer, race, sex));
    let pos = super::enemies::ground_on_collision(
        collide.as_ref(),
        &world,
        hero::arena(selection.stage_def()),
        selection.stage_def().floor,
    );
    let serial = run.guid();
    // The local player's hero, out of a run until one begins on it ([`start_run`]).
    let state = HeroState {
        class: &data::CLASSES[class],
        race,
        ..HeroState::default()
    };
    hero::spawn_hero(&mut commands, state, display, sex, serial, pos);
}

/// A run begins on the menu's pick: the shared state starts over, and the local hero (the menu's,
/// when there is one) becomes the pick, with the player's [`MetaBonuses`].
pub(super) fn start_run(
    run: &mut Run,
    hero: Option<&mut HeroState>,
    selection: &mut Selection,
    meta: &MetaBonuses,
) -> bool {
    let (Some(class), Some(&(_, race, _))) = (
        selection.class_def(),
        selection.look.and_then(|i| selection.looks.get(i)),
    ) else {
        return false;
    };
    // The world goes to the battleground picked; until now it stood in the last one.
    selection.stage = selection.map;
    let next_guid = run.next_guid;
    *run = Run {
        next_guid,
        ..Run::default()
    };
    if let Some(hero) = hero {
        let mut bonuses = meta.0.clone();
        bonuses.merge(&super::env_bonuses());
        *hero = HeroState::new(hero.player, class, race, bonuses);
    }
    run.version += 1;
    true
}

/// The local hero's parts a replay resets.
type ReplayHero = (
    &'static mut HeroState,
    Option<&'static mut Hero>,
    Option<&'static mut Marks>,
);

/// A results replay enters the menu only long enough to reset its run entities. The local hero
/// stays through that reset, then becomes the same menu pick immediately, so the menu never
/// presents a frame between results and the next run.
pub(super) fn start_replay(
    mut replay: ResMut<Replay>,
    mut run: ResMut<Run>,
    mut local: Query<ReplayHero, With<LocalHero>>,
    mut selection: ResMut<Selection>,
    meta: Res<MetaBonuses>,
    unlocks: Res<super::zones::Unlocks>,
    mut next: ResMut<NextState<Phase>>,
) {
    if !std::mem::take(&mut replay.0) {
        return;
    }
    if unlocks.is_open(selection.map)
        && local.single_mut().is_ok_and(|(mut hero, motion, marks)| {
            if let Some(mut motion) = motion {
                *motion = Hero::default();
            }
            if let Some(mut marks) = marks {
                *marks = Marks::buffs();
            }
            start_run(&mut run, Some(&mut hero), &mut selection, &meta)
        })
    {
        next.set(Phase::Playing);
    }
}

/// Everything a hand can deal `hero` now, with its weight; its banished choices are gone for the
/// run.
pub(super) fn offer_pool(hero: &HeroState) -> Vec<(Choice, f32)> {
    let mut pool: Vec<(Choice, f32)> = Vec::new();
    for (i, o) in hero.abilities.iter().enumerate() {
        if o.level < data::MAX_ABILITY_LEVEL {
            pool.push((Choice::Upgrade(i), 1.2));
        }
    }
    if hero.abilities.len() < data::MAX_ABILITIES {
        for key in hero.class.pool {
            if !hero.abilities.iter().any(|o| o.def.key == *key) {
                pool.push((Choice::NewAbility(data::ability(key)), 1.0));
            }
        }
    }
    for (i, (_, level)) in hero.passives.iter().enumerate() {
        if *level < data::MAX_PASSIVE_LEVEL {
            pool.push((Choice::UpPassive(i), 0.8));
        }
    }
    if hero.passives.len() < data::MAX_PASSIVES {
        for p in data::PASSIVES {
            if !hero.passives.iter().any(|(q, _)| q.key == p.key) {
                pool.push((Choice::NewPassive(p), 0.5));
            }
        }
    }
    pool.retain(|&(c, _)| !super::hand::is_banished(c, hero));
    // Luck tilts the draw toward the rarer cards.
    let stats = hero.stats();
    for (choice, w) in &mut pool {
        *w *= stats.luck_weight(super::cards::rarity(*choice, hero) as i32);
    }
    pool
}

/// Up to `n` choices from `pool` by weight, none twice.
pub(super) fn draw(mut pool: Vec<(Choice, f32)>, n: usize, rng: &mut Rng) -> Vec<Choice> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n && !pool.is_empty() {
        let total: f32 = pool.iter().map(|(_, w)| w).sum();
        let mut pick = rng.f32() * total;
        let mut idx = pool.len() - 1;
        for (i, (_, w)) in pool.iter().enumerate() {
            if pick < *w {
                idx = i;
                break;
            }
            pick -= w;
        }
        out.push(pool.swap_remove(idx).0);
    }
    out
}

/// A level-up hand for `hero`: three choices, or repeatable Overcharges and a healthstone once
/// everything is mastered.
pub(super) fn choices_for(hero: &HeroState, rng: &mut Rng) -> Vec<Choice> {
    let mut out = draw(offer_pool(hero), 3, rng);
    if out.is_empty() {
        out = draw(
            Overcharge::ALL
                .into_iter()
                .map(|kind| (Choice::Overcharge(kind), 1.0))
                .collect(),
            2,
            rng,
        );
        out.push(Choice::Heal);
    }
    out
}

/// The picker opens: the banked levels become a hand for every hero ([`party::deal_hands`]), the
/// heroes no one here picks for take theirs at random, and the local hero's first hand is dealt.
pub(super) fn roll_offer(
    mut run: ResMut<Run>,
    mut heroes: Query<(&mut HeroState, Has<LocalHero>)>,
    mut rng: ResMut<Rng>,
    mut offer: ResMut<Offer>,
    mut next: ResMut<NextState<Phase>>,
) {
    party::deal_hands(&mut run, heroes.iter_mut().map(|(hero, _)| hero));
    for (mut hero, local) in &mut heroes {
        if !local {
            party::pick_at_random(&mut hero, &mut rng);
        }
    }
    match heroes.iter().find(|(_, local)| *local) {
        Some((hero, _)) => offer.deal(choices_for(hero, &mut rng)),
        None => {
            offer.deal(Vec::new());
            next.set(Phase::Playing);
        }
    }
}

/// `choice` joins `hero`'s build; a top rank transforms its ability for the rest of the run.
pub(super) fn take(hero: &mut HeroState, choice: Choice) {
    match choice {
        Choice::NewAbility(def) => hero.abilities.push(Owned {
            def,
            level: 1,
            cd: 0.3,
        }),
        Choice::Upgrade(k) => {
            hero.abilities[k].level += 1;
            super::mechanics::sync_legendaries(hero);
        }
        Choice::NewPassive(p) => hero.passives.push((p, 1)),
        Choice::UpPassive(k) => hero.passives[k].1 += 1,
        Choice::Overcharge(kind) => hero.overcharges[kind.index()] += 1,
        Choice::Heal => hero.hp = hero.stats().max_hp,
    }
}

/// The local hero takes card `i` of the hand on offer.
pub(super) fn pick(
    i: usize,
    hero: &mut HeroState,
    run: &mut Run,
    offer: &mut Offer,
    rng: &mut Rng,
    next: &mut NextState<Phase>,
) {
    let Some(&choice) = offer.choices.get(i) else {
        return;
    };
    take(hero, choice);
    run.version += 1;
    close_hand(hero, offer, rng, next);
}

/// A hand done with, taken or passed up: another the hero holds deals next, else play resumes.
/// The other heroes' hands are already drawn by then ([`roll_offer`]).
pub(super) fn close_hand(
    hero: &mut HeroState,
    offer: &mut Offer,
    rng: &mut Rng,
    next: &mut NextState<Phase>,
) {
    hero.picks = hero.picks.saturating_sub(1);
    if hero.picks > 0 {
        offer.deal(choices_for(hero, rng));
    } else {
        offer.deal(Vec::new());
        next.set(Phase::Playing);
    }
}

pub(super) fn looks_for(
    class: &data::ClassDef,
    creatures: &crate::entities::Creatures,
) -> Vec<(u32, u8, u8)> {
    let mut out: Vec<(u32, u8, u8)> = Vec::new();
    for &d in class.looks {
        if let Some((race, sex)) = creatures.display_race_sex(d) {
            if !out.iter().any(|&(_, r, s)| r == race && s == sex) {
                out.push((d, race, sex));
            }
        }
    }
    out.sort_by_key(|&(_, r, s)| (r, s));
    out
}

#[allow(clippy::too_many_arguments)]
pub(super) fn buttons(
    mut presses: Query<
        (&Interaction, &UiAction, &BaseColor, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    mut next: ResMut<NextState<Phase>>,
    mut exit: MessageWriter<AppExit>,
    phase: Res<State<Phase>>,
    mut replay: ResMut<Replay>,
) {
    for (interaction, action, base, mut bg) in &mut presses {
        match interaction {
            Interaction::Hovered => bg.0 = BUTTON_HOT,
            Interaction::None => bg.0 = base.0,
            Interaction::Pressed => {
                bg.0 = BUTTON_HOT;
                match (*action, *phase.get()) {
                    (UiAction::Resume, Phase::Paused) => next.set(Phase::Playing),
                    (UiAction::PlayAgain, Phase::GameOver) => {
                        replay.0 = true;
                        next.set(Phase::Menu);
                    }
                    (UiAction::MainMenu, Phase::Paused | Phase::GameOver) => next.set(Phase::Menu),
                    (UiAction::Quit, Phase::Paused | Phase::GameOver) => {
                        exit.write(AppExit::Success);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(super) fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    mut selection: ResMut<Selection>,
    mut run: ResMut<Run>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    meta: Res<MetaBonuses>,
    unlocks: Res<super::zones::Unlocks>,
    mut next: ResMut<NextState<Phase>>,
    mut replay: ResMut<Replay>,
) {
    match phase.get() {
        Phase::Menu => {
            // `start_replay` has already made this run on the Menu entry frame. The Enter which
            // requested it is still just pressed, but must not begin it a second time.
            if matches!(
                *next,
                NextState::Pending(Phase::Playing) | NextState::PendingIfNeq(Phase::Playing)
            ) {
                return;
            }
            // As Enter World, never into a zone not yet open.
            if keys.just_pressed(KeyCode::Enter)
                && unlocks.is_open(selection.map)
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
        Phase::Playing => {
            if keys.just_pressed(KeyCode::Escape) {
                next.set(Phase::Paused);
            } else if keys.just_pressed(super::charpanel::KEY) {
                next.set(Phase::Character);
            }
        }
        Phase::Paused => {
            if keys.just_pressed(KeyCode::Escape) {
                next.set(Phase::Playing);
            }
        }
        // The Character panel closes on its own key, or Esc.
        Phase::Character => {
            if keys.any_just_pressed([KeyCode::Escape, super::charpanel::KEY]) {
                next.set(Phase::Playing);
            }
        }
        // The jewelry choice takes its own keys ([`super::jewelry`]).
        Phase::LevelUp | Phase::Jewelry => {}
        Phase::GameOver => {
            if keys.just_pressed(KeyCode::Enter) {
                replay.0 = true;
                next.set(Phase::Menu);
            } else if keys.just_pressed(KeyCode::Escape) {
                next.set(Phase::Menu);
            }
        }
    }
}

/// The class's looks once the creature data is in, defaulting to the first. The auto-runner picks
/// a class with no look; the menu itself always sets both.
pub(super) fn refresh_menu(
    mut selection: ResMut<Selection>,
    creatures: Option<Res<crate::entities::Creatures>>,
) {
    if let (Some(class), Some(creatures)) = (selection.class_def(), creatures.as_deref()) {
        if selection.looks.is_empty() {
            let looks = looks_for(class, creatures);
            if !looks.is_empty() {
                selection.looks = looks;
                selection.look = Some(0);
                selection.version += 1;
            }
        }
    }
}

/// What a level-up card shows.
pub(super) struct CardText {
    pub spell: u32,
    pub name: String,
    pub tag: String,
    pub tag_color: Color,
    pub desc: String,
    /// `(rank now, top rank)`, for the pips; `None` for the healthstone.
    pub ranks: Option<(u8, u8)>,
    /// What the pick transforms its ability into: the top rank's legendary.
    pub legendary: Option<&'static data::LegendaryDef>,
}

pub(super) fn describe_card(
    choice: Choice,
    hero: &HeroState,
    spells: Option<&crate::ui_action::Spells>,
) -> CardText {
    let new = Color::srgb(0.3, 1.0, 0.3);
    match choice {
        Choice::NewAbility(def) => {
            let stats = hero.stats();
            let c = cast_at(def, 1, &stats);
            CardText {
                spell: def.spell,
                name: spell_name(spells, def.spell, def.key),
                tag: "New spell".into(),
                tag_color: new,
                desc: format!("{}.", ability_body(def, &c)),
                ranks: Some((0, data::MAX_ABILITY_LEVEL)),
                legendary: None,
            }
        }
        Choice::Upgrade(k) => {
            let o = &hero.abilities[k];
            let next = o.level + 1;
            let stats = hero.stats();
            let (a, b) = (
                cast_at(o.def, o.level, &stats),
                cast_at(o.def, next, &stats),
            );
            // What actually changes, as before → after.
            let mut lines = Vec::new();
            let (da, db) = if a.dmg > 0.0 {
                (a.dmg, b.dmg)
            } else {
                (a.dot, b.dot)
            };
            if db > da {
                lines.push(format!("Damage {} » {}", num(da), num(db)));
            }
            if b.cooldown < a.cooldown - 0.01 {
                lines.push(format!(
                    "Cooldown {}s » {}s",
                    num(a.cooldown),
                    num(b.cooldown)
                ));
            }
            if b.count > a.count {
                lines.push(format!("Targets {} » {}", a.count, b.count));
            }
            if b.radius > a.radius + 0.05 {
                lines.push(format!("Area {} » {} yd", num(a.radius), num(b.radius)));
            }
            if b.duration > a.duration + 0.05 {
                lines.push(format!("Lasts {}s » {}s", num(a.duration), num(b.duration)));
            }
            CardText {
                spell: o.def.spell,
                name: spell_name(spells, o.def.spell, o.def.key),
                tag: format!("Rank {} » {next}", o.level),
                tag_color: Color::WHITE,
                desc: lines.join("\n"),
                ranks: Some((o.level, data::MAX_ABILITY_LEVEL)),
                legendary: data::legendary(o.def.key).filter(|_| next == data::MAX_ABILITY_LEVEL),
            }
        }
        Choice::NewPassive(p) => CardText {
            spell: p.spell,
            name: p.name.into(),
            tag: "New talent".into(),
            tag_color: new,
            desc: passive_text(p.stat, p.per_level),
            ranks: Some((0, data::MAX_PASSIVE_LEVEL)),
            legendary: None,
        },
        Choice::UpPassive(k) => {
            let (p, level) = hero.passives[k];
            let fmt = |v: f32| match p.stat {
                Stat::Regen | Stat::Armor => num(v),
                _ => pct(v),
            };
            CardText {
                spell: p.spell,
                name: p.name.into(),
                tag: format!("Rank {level} » {}", level + 1),
                tag_color: Color::WHITE,
                desc: format!(
                    "{}\n{} » {}",
                    stat_label(p.stat),
                    fmt(p.per_level * f32::from(level)),
                    fmt(p.per_level * f32::from(level + 1)),
                ),
                ranks: Some((level, data::MAX_PASSIVE_LEVEL)),
                legendary: None,
            }
        }
        Choice::Overcharge(kind) => {
            let stack = hero.overcharges[kind.index()];
            let (name, desc) = match kind {
                Overcharge::Damage => ("Overcharge: Damage", "Increases all damage by 4%."),
                Overcharge::Haste => ("Overcharge: Haste", "Reduces cooldowns by 3%."),
                Overcharge::MaxHp => ("Overcharge: Vitality", "Increases maximum health by 5%."),
                Overcharge::Speed => ("Overcharge: Speed", "Increases movement speed by 4%."),
                Overcharge::Magnet => ("Overcharge: Magnet", "Increases pickup radius by 10%."),
                Overcharge::Gold => ("Overcharge: Gold", "Increases gold found by 10%."),
            };
            CardText {
                spell: 0,
                name: name.into(),
                tag: format!("Stack {stack} » {}", stack + 1),
                tag_color: super::cards::EPIC,
                desc: desc.into(),
                ranks: None,
                legendary: None,
            }
        }
        Choice::Heal => CardText {
            spell: 5720,
            name: "Healthstone".into(),
            tag: "Restore".into(),
            tag_color: Color::WHITE,
            desc: "Everything is mastered. Restores all health.".into(),
            ranks: None,
            legendary: None,
        },
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn refresh_hud(
    mut commands: Commands,
    time: Res<Time>,
    phase: Res<State<Phase>>,
    mut run: ResMut<Run>,
    local: Query<&HeroState, With<LocalHero>>,
    selection: Res<Selection>,
    names: super::gear::HeroNames,
    profile: Option<Res<Profile>>,
    font: Res<HudFont>,
    mut texts: ParamSet<(
        Query<&mut Text, With<LevelText>>,
        Query<&mut Text, With<TimerText>>,
        Query<&mut Text, With<KillText>>,
        Query<&mut Text, With<BannerText>>,
        Query<&mut Text, With<OverTitle>>,
        Query<&mut Text, With<OverStats>>,
        Query<&mut Text, With<XpText>>,
    )>,
    mut xp_fill: Query<&mut Node, With<XpFill>>,
    rows: Query<(Entity, Has<AbilityRow>), Or<(With<AbilityRow>, With<PassiveRow>)>>,
    toy_rows: Query<Entity, With<ToyRow>>,
    toy_bar: Res<super::toys::ToyBar>,
    mut sweeps: ResMut<Assets<SweepUiMaterial>>,
    mut last_version: Local<Option<u32>>,
) {
    let need = data::xp_to_next(run.level);
    if let Ok(mut n) = xp_fill.single_mut() {
        n.width = Val::Percent((run.xp / need * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut t) = texts.p6().single_mut() {
        let xp = format!("XP {} / {}", run.xp as u32, need as u32);
        if **t != xp {
            **t = xp;
        }
    }
    if let Ok(mut t) = texts.p0().single_mut() {
        **t = format!("Level {}", run.level);
    }
    let secs = run.elapsed as u32;
    if let Ok(mut t) = texts.p1().single_mut() {
        **t = format!("{:02}:{:02}", secs / 60, secs % 60);
    }
    let kills = run.kills;
    if let Ok(mut t) = texts.p2().single_mut() {
        **t = format!("Kills {kills}");
    }
    let banner = match run.banner.as_mut() {
        Some((msg, t)) => {
            *t -= time.delta_secs();
            if *t > 0.0 {
                msg.clone()
            } else {
                String::new()
            }
        }
        None => String::new(),
    };
    if banner.is_empty() {
        run.banner = None;
    }
    // The picker owns the screen: an announcement waits until it closes.
    let banner = if *phase.get() == Phase::LevelUp {
        String::new()
    } else {
        banner
    };
    if let Ok(mut t) = texts.p3().single_mut() {
        if **t != banner {
            **t = banner;
        }
    }
    let title = if run.won {
        "Victory!"
    } else {
        "You have fallen"
    }
    .to_string();
    if let Ok(mut t) = texts.p4().single_mut() {
        **t = title;
    }
    // The local hero's, as this screen and the icon rows below are.
    let fallback;
    let me = match local.single() {
        Ok(hero) => hero,
        Err(_) => {
            fallback = HeroState::default();
            &fallback
        }
    };
    // "Thrall the Orc Shaman in Durotar" once the hero has a name.
    let summary = format!(
        "{}   —   Enter to play again · Esc for the main menu",
        super::gear::results_title(
            selection.class.and_then(|c| names.name(c)),
            profile
                .as_deref()
                .and_then(super::faire::catalog::equipped_title),
            crate::char_select::race_name(me.race),
            me.class.name,
            selection.map_def().name,
        ),
    );
    if let Ok(mut t) = texts.p5().single_mut() {
        if **t != summary {
            **t = summary;
        }
    }

    // The ability and passive icons, rebuilt when the build changes.
    if *last_version == Some(run.version) {
        return;
    }
    *last_version = Some(run.version);
    let f = &font.0;
    for (row, is_abilities) in &rows {
        commands.entity(row).despawn_related::<Children>();
        commands.entity(row).with_children(|row| {
            let entries: Vec<(u32, u8, f32, TipFor)> = if is_abilities {
                me.abilities
                    .iter()
                    .enumerate()
                    .map(|(i, o)| (o.def.spell, o.level, 40.0, TipFor::Ability(i)))
                    .collect()
            } else {
                me.passives
                    .iter()
                    .enumerate()
                    .map(|(i, (p, l))| (p.spell, *l, 30.0, TipFor::Passive(i)))
                    .collect()
            };
            for (spell, level, size, tip) in entries {
                let ability = match &tip {
                    TipFor::Ability(i) => Some(*i),
                    _ => None,
                };
                // A transformed ability wears the legendary rim for the rest of the run.
                let legendary = ability.is_some() && level >= data::MAX_ABILITY_LEVEL;
                let (rim, edge) = if legendary {
                    (2.0, super::cards::LEGENDARY)
                } else {
                    (1.0, EDGE)
                };
                row.spawn((
                    Node {
                        width: Val::Px(size),
                        height: Val::Px(size),
                        border: UiRect::all(Val::Px(rim)),
                        ..default()
                    },
                    BorderColor::all(edge),
                    Interaction::default(),
                    tip,
                ))
                .with_children(|slot| {
                    slot.spawn(icon(spell, size - 2.0 * rim));
                    if legendary {
                        slot.spawn(legendary_corner());
                    }
                    // The cooldown sweep sits over the icon, under the level text.
                    if let Some(i) = ability {
                        let handle = sweeps.add(SweepUiMaterial { sweep: 0.0 });
                        slot.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            MaterialNode(handle.clone()),
                            AbilitySweep { handle, index: i },
                        ));
                    }
                    slot.spawn(Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(2.0),
                        bottom: Val::Px(0.0),
                        ..default()
                    })
                    .with_children(|n| {
                        n.spawn(text(f, level.to_string(), 12.0, Color::WHITE));
                    });
                });
            }
        });
    }
    for row in &toy_rows {
        commands.entity(row).despawn_related::<Children>();
        commands.entity(row).with_children(|row| {
            for (i, slot) in toy_bar.slots().iter().enumerate() {
                row.spawn((
                    Button,
                    super::toys::ToyButton(i),
                    BaseColor(BUTTON),
                    BackgroundColor(BUTTON),
                    BorderColor::all(EDGE),
                    Node {
                        width: Val::Px(52.0),
                        height: Val::Px(40.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|button| {
                    button.spawn(centered(
                        f,
                        format!("{} {}", i + 7, slot.name()),
                        10.0,
                        Color::WHITE,
                    ));
                    button.spawn((
                        super::toys::ToyCooldownText(i),
                        centered(f, "Ready", 10.0, GOLD),
                    ));
                });
            }
        });
    }
}

/// The gold corner on a transformed ability's icon, over its top left.
fn legendary_corner() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(14.0),
            height: Val::Px(14.0),
            ..default()
        },
        BackgroundGradient::from(LinearGradient::to_bottom_right(vec![
            ColorStop::percent(GOLD, 0.0),
            ColorStop::percent(super::cards::LEGENDARY, 48.0),
            ColorStop::percent(super::cards::LEGENDARY.with_alpha(0.0), 52.0),
        ])),
        bevy::ui::FocusPolicy::Pass,
    )
}

/// Fill icon slots from `SpellIcon.dbc` once the client data and the asset reader are up.
pub(super) fn fill_icons(
    mut commands: Commands,
    mut slots: Query<(Entity, &mut ImageNode, &SpellIcon), Without<IconDone>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: Local<HashMap<u32, Handle<Image>>>,
) {
    let (Some(spells), Some(assets)) = (spells.as_deref(), assets.as_deref_mut()) else {
        return;
    };
    for (e, mut img, icon) in &mut slots {
        let handle = match cache.get(&icon.0) {
            Some(h) => Some(h.clone()),
            None => {
                let path = spells
                    .catalog
                    .get(icon.0)
                    .and_then(|d| d.icon.clone())
                    .unwrap_or_else(|| "Interface\\Icons\\INV_Misc_QuestionMark".into());
                let h = assets.sprite_texture(&format!("{path}.blp"), &mut images);
                if let Some(h) = &h {
                    cache.insert(icon.0, h.clone());
                }
                h
            }
        };
        if let Some(h) = handle {
            img.image = h;
            img.color = Color::WHITE;
        }
        commands.entity(e).insert(IconDone);
    }
}

/// The mobility button: shown once the local hero has learned it, its icon the class's spell, its
/// sweep the cooldown.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_mobility(
    run: Res<Run>,
    local: Query<&HeroState, With<LocalHero>>,
    mut commands: Commands,
    mut slot: Query<&mut Node, With<MobilitySlot>>,
    icon: Query<(Entity, Option<&SpellIcon>), With<MobilityIcon>>,
    sweep: Query<&MobilitySweep>,
    mut mats: ResMut<Assets<SweepUiMaterial>>,
) {
    let me = local.single().ok();
    let def = me.and_then(|hero| hero.mobility(run.level));
    if let Ok(mut node) = slot.single_mut() {
        let display = if def.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let (Some(me), Some(def)) = (me, def) else {
        return;
    };
    // A new spell re-arms the icon fill ([`fill_icons`]).
    if let Ok((e, have)) = icon.single() {
        if have.is_none_or(|s| s.0 != def.spell) {
            commands
                .entity(e)
                .remove::<IconDone>()
                .insert(SpellIcon(def.spell));
        }
    }
    if let Ok(s) = sweep.single() {
        let f = (me.mobile.cd / def.cooldown.max(0.01)).clamp(0.0, 1.0);
        if let Some(m) = mats.get_mut(&s.0) {
            if m.sweep != f {
                m.sweep = f;
            }
        }
    }
}

/// Drive each of the local hero's ability icons' cooldown sweep from its remaining cooldown.
pub(super) fn update_cooldown_sweeps(
    local: Query<&HeroState, With<LocalHero>>,
    mut mats: ResMut<Assets<SweepUiMaterial>>,
    sweeps: Query<&AbilitySweep>,
) {
    let Ok(me) = local.single() else { return };
    let stats = me.stats();
    for s in &sweeps {
        let Some(o) = me.abilities.get(s.index) else {
            continue;
        };
        let total = o.cooldown(&stats).max(0.01);
        let f = (o.cd / total).clamp(0.0, 1.0);
        if let Some(m) = mats.get_mut(&s.handle) {
            m.sweep = f;
        }
    }
}

/// Draw each [`UiArt`] region once its file can be read.
pub(super) fn fill_art(
    mut commands: Commands,
    mut regions: Query<(Entity, &mut ImageNode, &UiArt), Without<ArtDone>>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut assets) = assets else { return };
    for (e, mut img, art) in &mut regions {
        commands.entity(e).insert(ArtDone);
        let Some(h) = assets.sprite_texture(art.path, &mut images) else {
            warn!("survivors: no UI art {}", art.path);
            continue;
        };
        let [l, r, t, b] = art.coords;
        if let Some(size) = images.get(&h).map(Image::size_f32) {
            img.rect = Some(Rect::new(l * size.x, t * size.y, r * size.x, b * size.y));
        }
        img.image = h;
        img.color = art.tint;
    }
}

/// The reference lays its UI out on a screen 768 units tall at any resolution (`useUiScale` off).
pub(super) fn apply_ui_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
) {
    let Ok(window) = windows.single() else { return };
    let want = window.height() / 768.0;
    if want > 0.0 && (scale.0 - want).abs() > 1e-4 {
        scale.0 = want;
    }
}

/// Between the hero's feet and the top of its plate's bar, in gx units.
const PLATE_FEET_GAP: f32 = 0.004;
/// A couple of screen pixels lower than the gap alone sets it.
const PLATE_NUDGE: f32 = 2.0;

/// The local hero's nameplate under its feet, sized from the screen diagonal as [`crate::vplates`]
/// sizes a plate, outside the UI scale. The border's top half seats a name the hero has none of,
/// so the plate rises until its bar sits just under the feet.
#[allow(clippy::type_complexity)]
pub(super) fn hero_plate(
    run: Res<Run>,
    ui_scale: Res<UiScale>,
    hero: Query<(&Transform, &HeroState), With<LocalHero>>,
    cams: Query<(&Camera, &Transform), With<WorldCamera>>,
    mut nodes: ParamSet<(
        Query<(&mut Node, &mut Visibility), With<HpFrame>>,
        Query<&mut Node, With<HpBar>>,
        Query<&mut Node, With<HpFill>>,
        Query<&mut Node, With<ShieldFill>>,
        Query<&mut Node, With<HpLevelSeat>>,
    )>,
    mut level: Query<(&mut Text, &mut TextFont), With<HpLevel>>,
) {
    use crate::vplates::{
        gx_px, plate_basis, text_px, BAR_H, BAR_OFF_X, BAR_OFF_Y, BAR_W, LEVEL_H, LEVEL_OFF_X,
        LEVEL_OFF_Y, PLATE_H, PLATE_W,
    };
    let seat = hero.single().ok().and_then(|(h, me)| {
        let (cam, tf) = cams.single().ok()?;
        let viewport = cam.logical_viewport_size()?;
        let feet = cam
            .world_to_viewport(&GlobalTransform::from(*tf), h.translation)
            .ok()?;
        Some((feet, viewport, me))
    });
    let mut frames = nodes.p0();
    let Ok((mut frame, mut vis)) = frames.single_mut() else {
        return;
    };
    let Some((feet, viewport, me)) = seat else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Inherited;
    let s = ui_scale.0;
    let basis = plate_basis(viewport);
    let gx = |v: f32| gx_px(v, basis) / s;
    let (pw, ph) = (gx(PLATE_W), gx(PLATE_H));
    let bar_top = ph - gx(BAR_OFF_Y) - gx(BAR_H);
    frame.width = Val::Px(pw);
    frame.height = Val::Px(ph);
    frame.left = Val::Px(feet.x / s - pw / 2.0);
    frame.top = Val::Px(feet.y / s + gx(PLATE_FEET_GAP) - bar_top + PLATE_NUDGE / s);
    if let Ok(mut bar) = nodes.p1().single_mut() {
        bar.left = Val::Px(gx(BAR_OFF_X));
        bar.top = Val::Px(bar_top);
        bar.width = Val::Px(gx(BAR_W));
        bar.height = Val::Px(gx(BAR_H));
    }
    let stats = me.stats();
    let hp = (me.hp / stats.max_hp).clamp(0.0, 1.0);
    let shield = if me.shield_t > 0.0 {
        (me.shield / stats.max_hp).clamp(0.0, 1.0)
    } else {
        0.0
    };
    if let Ok(mut n) = nodes.p2().single_mut() {
        n.width = Val::Percent(hp * 100.0);
    }
    if let Ok(mut n) = nodes.p3().single_mut() {
        n.width = Val::Percent(shield * 100.0);
    }
    if let Ok(mut seat) = nodes.p4().single_mut() {
        seat.left = Val::Px(pw - gx(LEVEL_OFF_X));
        seat.top = Val::Px(ph - gx(LEVEL_OFF_Y));
    }
    if let Ok((mut text, mut font)) = level.single_mut() {
        let n = run.level.to_string();
        if text.0 != n {
            text.0 = n;
        }
        let size = text_px(LEVEL_H, basis) / s;
        if font.font_size != size {
            font.font_size = size;
        }
    }
}

/// `MainMenuExpBar`'s `OnEnter` and `OnLeave`: the numbers show while the cursor is on the bar.
pub(super) fn xp_hover(
    bars: Query<&Interaction, (With<XpBar>, Changed<Interaction>)>,
    mut texts: Query<&mut Visibility, With<XpText>>,
) {
    for interaction in &bars {
        let want = if *interaction == Interaction::None {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        for mut v in &mut texts {
            *v = want;
        }
    }
}

// ── The build icons' `GameTooltip` ──────────────────────────────────────────────────────────────

/// `TOOLTIP_DEFAULT_BACKGROUND_COLOR`, the `GameTooltip` backdrop tint (`GameTooltip.lua`).
const TIP_FILL: Color = Color::srgb(0.09, 0.09, 0.19);
/// The grey a spell tooltip prints its rank in, right of the name.
const TIP_GREY: Color = Color::srgb(0.5, 0.5, 0.5);

/// One tooltip line: the left text, an optional right-aligned one, the colour and font size
/// (`GameTooltipHeaderText` 14, `GameTooltipText` 12).
struct TipLine {
    left: String,
    right: Option<(String, Color)>,
    color: Color,
    size: f32,
}

impl TipLine {
    fn new(left: impl Into<String>, color: Color) -> Self {
        Self {
            left: left.into(),
            right: None,
            color,
            size: 12.0,
        }
    }

    fn right(mut self, text: impl Into<String>, color: Color) -> Self {
        self.right = Some((text.into(), color));
        self
    }

    fn header(mut self) -> Self {
        self.size = 14.0;
        self
    }
}

/// The results screen's tiles and its line under the title.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_results(
    run: Res<Run>,
    selection: Res<Selection>,
    news: Res<LastRunNews>,
    mut over_texts: ParamSet<(
        Query<(&OverStat, &mut Text)>,
        Query<&mut Text, With<OverLine>>,
        Query<&mut Text, With<OverNews>>,
    )>,
) {
    let secs = run.elapsed as u32;
    let stats_line = if run.won {
        selection.map_def().victory
    } else {
        "The horde was too much this time."
    };
    let values = [
        format!("{:02}:{:02}", secs / 60, secs % 60),
        run.level.to_string(),
        run.kills.to_string(),
        compact(run.damage_done),
        run.bosses.to_string(),
        run.gold.to_string(),
        run.elites.to_string(),
        run.caches.to_string(),
    ];
    for (stat, mut t) in &mut over_texts.p0() {
        if let Some(v) = values.get(stat.0) {
            if **t != *v {
                **t = v.clone();
            }
        }
    }
    if let Ok(mut t) = over_texts.p1().single_mut() {
        let summary = format!(
            "{stats_line}  ·  Run {} in {} · best {}",
            news.zone_runs,
            selection.map_def().name,
            clock(news.best_seconds),
        );
        if **t != summary {
            **t = summary;
        }
    }
    if let Ok(mut t) = over_texts.p2().single_mut() {
        let note = results_note(&news);
        if **t != note {
            **t = note;
        }
    }
}

/// The results screen's [`ResultsNotes`] lines: the first [`MAX_NOTES`], each in its kind's
/// colour; the column leaves the layout while there are none.
#[allow(clippy::type_complexity)]
pub(super) fn refresh_notes(
    notes: Res<ResultsNotes>,
    mut column: Query<&mut Node, (With<OverNotes>, Without<OverNote>)>,
    mut lines: Query<(&OverNote, &mut Text, &mut TextColor, &mut Node), Without<OverNotes>>,
) {
    let display = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut column {
        let want = display(!notes.0.is_empty());
        if node.display != want {
            node.display = want;
        }
    }
    for (line, mut text, mut color, mut node) in &mut lines {
        let note = notes.0.get(line.0).filter(|_| line.0 < MAX_NOTES);
        let want = display(note.is_some());
        if node.display != want {
            node.display = want;
        }
        let Some((s, kind)) = note else { continue };
        if **text != *s {
            **text = s.clone();
        }
        let want = kind.color();
        if color.0 != want {
            color.0 = want;
        }
    }
}

fn results_note(news: &LastRunNews) -> String {
    if news.first_zone_win {
        format!("First win in {}!", news.zone)
    } else if news.first_class_win {
        format!("First win as a {}!", news.class)
    } else if news.new_best_survival {
        "New best time!".to_string()
    } else {
        String::new()
    }
}

fn clock(seconds: f32) -> String {
    let seconds = seconds as u32;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// 12.3k, 4.5M.
fn compact(v: f64) -> String {
    if v >= 1e6 {
        format!("{:.1}M", v / 1e6)
    } else if v >= 1e4 {
        format!("{:.1}k", v / 1e3)
    } else {
        format!("{}", v.round() as u64)
    }
}

/// A number as the tooltips print it: whole when near enough, else to one decimal.
fn num(v: f32) -> String {
    if (v - v.round()).abs() < 0.05 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn pct(v: f32) -> String {
    format!("{}%", num(v * 100.0))
}

/// The numbers an ability casts with at a level, as [`super::abilities::cast_abilities`] takes
/// them from [`Owned`].
struct Cast {
    dmg: f32,
    dot: f32,
    count: u32,
    radius: f32,
    duration: f32,
    cooldown: f32,
}

fn cast_at(def: &'static AbilityDef, level: u8, stats: &Stats) -> Cast {
    let o = Owned {
        def,
        level,
        cd: 0.0,
    };
    let mult = o.damage_mult() * stats.might;
    Cast {
        dmg: def.damage * mult,
        dot: def.dot * mult,
        count: o.count(stats),
        radius: o.radius(stats),
        duration: o.duration(),
        cooldown: o.cooldown(stats),
    }
}

fn effect_text(effect: Effect) -> Option<String> {
    Some(match effect {
        Effect::None => return None,
        Effect::Slow(m, t) => format!("slowing them by {} for {} sec", pct(1.0 - m), num(t)),
        Effect::Root(t) => format!("freezing them in place for {} sec", num(t)),
        Effect::Stun(t) => format!("stunning them for {} sec", num(t)),
        Effect::Fear(t) => format!("sending them fleeing for {} sec", num(t)),
        Effect::Weaken(m, t) => format!(
            "cutting their damage by {} for {} sec",
            pct(1.0 - m),
            num(t)
        ),
        Effect::Drain(f) => format!("healing you for {} of the damage", pct(f)),
        Effect::Execute(f) => format!("killing them outright below {} health", pct(f)),
    })
}

/// What an ability does at these numbers, in the voice of a spell description.
fn ability_body(def: &AbilityDef, c: &Cast) -> String {
    let d = num(c.dmg);
    let burst = def.duration <= 0.0;
    let mut s = match def.kind {
        Kind::Bolt | Kind::Strike | Kind::Melee => {
            let toughest = matches!(def.key, "pyroblast" | "aimed_shot" | "exorcism" | "starfire");
            let melee = def.kind == Kind::Melee;
            let who = match (toughest, melee, c.count) {
                (true, _, 1) => "the toughest enemy in range".to_string(),
                (true, _, n) => format!("the {n} toughest enemies in range"),
                (false, true, 1) => "the closest enemy".into(),
                (false, true, n) => format!("the {n} closest enemies"),
                (false, false, 1) => "the nearest enemy".into(),
                (false, false, n) => format!("the {n} nearest enemies"),
            };
            let mut s = format!("{} {who} for {d} damage", if melee { "Strikes" } else { "Hits" });
            if c.radius > 0.0 && !melee {
                s += &format!(", and everything within {} yd of the hit", num(c.radius));
            }
            s
        }
        Kind::Nova => format!("Deals {d} damage to every enemy within {} yd of you", num(c.radius)),
        Kind::Cone => format!("Deals {d} damage to every enemy in a {} yd cone toward the closest", num(c.radius)),
        Kind::Ground | Kind::GroundSelf => {
            let at = if def.kind == Kind::GroundSelf { "around you" } else { "on the thickest pack" };
            if burst {
                format!("Strikes a {} yd area {at} for {d} damage", num(c.radius))
            } else {
                format!(
                    "Covers a {} yd area {at} for {} sec, dealing {d} damage every second",
                    num(c.radius),
                    num(c.duration)
                )
            }
        }
        Kind::Chain => format!(
            "Strikes the nearest enemy for {d} damage, then jumps to {} more within {} yd of each other, 15% weaker with every jump",
            c.count,
            num(c.radius)
        ),
        Kind::Dot => format!(
            "Afflicts {} enemies in range with {} damage every second for {} sec",
            c.count,
            num(c.dot),
            num(c.duration)
        ),
        Kind::Aura => format!(
            "Every {} sec, deals {d} damage to every enemy within {} yd of you",
            num(c.cooldown),
            num(c.radius)
        ),
        Kind::Orbit => format!(
            "Every {} sec, zaps {} within {} yd of you for {d} damage",
            num(c.cooldown),
            if c.count == 1 { "an enemy".to_string() } else { format!("{} enemies", c.count) },
            num(c.radius)
        ),
        Kind::Shield => format!("Absorbs the next {d} damage you take within {} sec", num(c.duration)),
        Kind::Immune => format!(
            "When an enemy closes in, makes you immune to all damage for {} sec",
            num(c.duration)
        ),
        Kind::Thorns => format!(
            "When enemies close in, deals {d} damage to every attacker for {} sec",
            num(c.duration)
        ),
        Kind::Heal => format!("Below 85% health, heals you for {d} over {} sec", num(c.duration)),
        Kind::Totem if def.key == "magma_totem" => format!(
            "Drops a totem for {} sec that burns every enemy within {} yd for {d} damage every 2 sec",
            num(c.duration),
            num(c.radius)
        ),
        Kind::Totem => format!(
            "Drops a totem for {} sec that hits the nearest enemy within {} yd for {d} damage every second",
            num(c.duration),
            num(def.range)
        ),
    };
    // Areas that tick carry no over-time part; a burst leaves a 5 second one.
    let ground = matches!(def.kind, Kind::Ground | Kind::GroundSelf);
    if c.dot > 0.0 && def.kind != Kind::Dot && (!ground || burst) {
        let secs = if ground { 5.0 } else { c.duration };
        s += &format!(
            ", then {} damage every second for {} sec",
            num(c.dot),
            num(secs)
        );
    }
    if let Some(e) = effect_text(def.effect) {
        s += ", ";
        s += &e;
    }
    s.push('.');
    s
}

/// What the next rank changes, at the current stats.
fn next_rank(def: &AbilityDef, now: &Cast, next: &Cast) -> String {
    let what = match def.kind {
        Kind::Shield => "absorbed",
        Kind::Heal => "healing",
        _ => "damage",
    };
    let mut gains = Vec::new();
    if next.dmg > 0.0 && def.kind != Kind::Dot && def.kind != Kind::Immune {
        gains.push(format!("{} {what}", num(next.dmg)));
    }
    if next.dot > 0.0 {
        gains.push(format!("{} damage per second", num(next.dot)));
    }
    if next.count != now.count
        && matches!(
            def.kind,
            Kind::Bolt | Kind::Strike | Kind::Melee | Kind::Dot | Kind::Chain | Kind::Orbit
        )
    {
        gains.push(match def.kind {
            Kind::Chain => format!("{} jumps", next.count),
            _ => format!("{} targets", next.count),
        });
    }
    if next.radius > 0.0 {
        gains.push(format!("{} yd", num(next.radius)));
    }
    if next.duration > 0.0 {
        gains.push(format!("{} sec", num(next.duration)));
    }
    gains.push(format!("{} sec cooldown", num(next.cooldown)));
    gains.join(", ")
}

fn ability_tip(
    o: &Owned,
    stats: &Stats,
    spells: Option<&crate::ui_action::Spells>,
) -> Vec<TipLine> {
    let def = o.def;
    let now = cast_at(def, o.level, stats);
    let mut lines = vec![
        TipLine::new(spell_name(spells, def.spell, def.key), Color::WHITE)
            .header()
            .right(format!("Rank {}", o.level), TIP_GREY),
    ];
    let range = match def.kind {
        Kind::Melee => Some("Melee Range".to_string()),
        Kind::Bolt | Kind::Strike | Kind::Chain | Kind::Dot | Kind::Ground | Kind::Totem => {
            Some(format!("{} yd range", num(def.range)))
        }
        _ => None,
    };
    let mut cast = TipLine::new("Casts itself", Color::WHITE);
    if let Some(range) = range {
        cast = cast.right(range, Color::WHITE);
    }
    lines.push(cast);
    lines.push(
        TipLine::new("Instant", Color::WHITE)
            .right(format!("{} sec cooldown", num(now.cooldown)), Color::WHITE),
    );
    lines.push(TipLine::new(ability_body(def, &now), GOLD));
    if !matches!(
        def.kind,
        Kind::Shield | Kind::Immune | Kind::Heal | Kind::Dot
    ) {
        lines.push(TipLine::new(
            format!(
                "{} chance to critically strike for double damage",
                pct(stats.crit)
            ),
            DIM,
        ));
    }
    if o.level < data::MAX_ABILITY_LEVEL {
        let next = cast_at(def, o.level + 1, stats);
        lines.push(TipLine::new("Next rank:", Color::WHITE));
        lines.push(TipLine::new(next_rank(def, &now, &next), GOLD));
        if o.level + 1 == data::MAX_ABILITY_LEVEL {
            if let Some(l) = data::legendary(def.key) {
                lines.push(TipLine::new(
                    format!("Transforms: {}", l.name),
                    super::cards::LEGENDARY,
                ));
            }
        }
    } else if let Some(l) = data::legendary(def.key) {
        lines.push(TipLine::new(
            format!("Legendary: {}", l.name),
            super::cards::LEGENDARY,
        ));
        lines.push(TipLine::new(l.text, GOLD));
    } else {
        lines.push(TipLine::new("Highest rank", DIM));
    }
    lines
}

/// What a passive's bonus `v` does, in the voice of a buff.
/// A stat's short name, for the card's before » after line.
fn stat_label(stat: Stat) -> &'static str {
    match stat {
        Stat::Might => "All damage",
        Stat::MaxHp => "Maximum health",
        Stat::Speed => "Movement speed",
        Stat::Regen => "Health per second",
        Stat::Haste => "Recharge speed",
        Stat::Area => "Area size",
        Stat::Magnet => "Pickup range",
        Stat::Growth => "Experience gained",
        Stat::Armor => "Damage blocked per hit",
        Stat::Crit => "Critical strike chance",
    }
}

fn passive_text(stat: Stat, v: f32) -> String {
    match stat {
        Stat::Might => format!("Increases all damage you deal by {}.", pct(v)),
        Stat::MaxHp => format!("Increases your maximum health by {}.", pct(v)),
        Stat::Speed => format!("Increases your movement speed by {}.", pct(v)),
        Stat::Regen => format!("Restores {} health every second.", num(v)),
        Stat::Haste => format!("Your abilities recharge {} faster.", pct(v)),
        Stat::Area => format!("Increases the size of your areas of effect by {}.", pct(v)),
        Stat::Magnet => format!("Increases your pickup range by {}.", pct(v)),
        Stat::Growth => format!("Increases the experience you gain by {}.", pct(v)),
        Stat::Armor => format!("Reduces the damage of every hit you take by {}.", num(v)),
        Stat::Crit => format!(
            "Increases your chance to critically strike for double damage by {}.",
            pct(v)
        ),
    }
}

/// The hero's total for the stat a passive raises, every source counted.
fn stat_total(stat: Stat, s: &Stats) -> String {
    match stat {
        Stat::Might => format!("Damage dealt: {}", pct(s.might)),
        Stat::MaxHp => format!("Maximum health: {}", num(s.max_hp)),
        Stat::Speed => format!("Movement speed: {} yd per second", num(s.speed)),
        Stat::Regen => format!("Health restored: {} per second", num(s.regen)),
        Stat::Haste => format!("Cooldowns: {} shorter", pct(s.haste.min(0.65))),
        Stat::Area => format!("Area of effect: {}", pct(s.area)),
        Stat::Magnet => format!("Pickup range: {} yd", num(s.magnet)),
        Stat::Growth => format!("Experience gained: {}", pct(s.growth)),
        Stat::Armor => format!("Damage blocked per hit: {}", num(s.armor)),
        Stat::Crit => format!("Critical strike chance: {}", pct(s.crit)),
    }
}

fn passive_tip(p: &PassiveDef, level: u8, stats: &Stats) -> Vec<TipLine> {
    let mut lines = vec![
        TipLine::new(p.name, Color::WHITE)
            .header()
            .right(format!("Rank {level}"), TIP_GREY),
        TipLine::new(passive_text(p.stat, p.per_level * f32::from(level)), GOLD),
        TipLine::new(stat_total(p.stat, stats), Color::WHITE),
    ];
    if level < data::MAX_PASSIVE_LEVEL {
        lines.push(TipLine::new("Next rank:", Color::WHITE));
        lines.push(TipLine::new(
            passive_text(p.stat, p.per_level * f32::from(level + 1)),
            GOLD,
        ));
    } else {
        lines.push(TipLine::new("Highest rank", DIM));
    }
    lines
}

/// The `GameTooltip` frame: `UI-Tooltip-Background` tiled under `UI-Tooltip-Border`, 16-unit edges
/// and 5-unit insets, its lines 10 units in. Returns the frame and its line column.
fn spawn_tooltip(
    commands: &mut Commands,
    assets: &mut benilla_assets::WorldAssets,
    images: &mut Assets<Image>,
    ui_scale: f32,
) -> (Entity, Entity) {
    use crate::glue::backdrop::{backdrop_border, backdrop_edges, tiled_bg_node};
    let bg = assets.sprite_texture("Interface\\Tooltips\\UI-Tooltip-Background", images);
    let edges = backdrop_edges(assets, "Interface\\Tooltips\\UI-Tooltip-Border", images);
    let mut body = Entity::PLACEHOLDER;
    let mut root = commands.spawn((
        GameTooltip,
        Node {
            position_type: PositionType::Absolute,
            max_width: Val::Px(300.0),
            ..default()
        },
        GlobalZIndex(1000),
        Visibility::Hidden,
        bevy::ui::FocusPolicy::Pass,
    ));
    if bg.is_none() || edges.is_none() {
        root.insert(BackgroundColor(TIP_FILL.with_alpha(0.95)));
    }
    root.with_children(|t| {
        if let Some(bg) = bg {
            t.spawn((
                tiled_bg_node(bg, 16.0, ui_scale, TIP_FILL),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(5.0),
                    right: Val::Px(5.0),
                    top: Val::Px(5.0),
                    bottom: Val::Px(5.0),
                    ..default()
                },
            ));
        }
        if let Some(edges) = &edges {
            backdrop_border(t, edges, 16.0, Color::WHITE);
        }
        body = t
            .spawn((
                TipBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
            ))
            .id();
    });
    (root.id(), body)
}

/// Hovering a build icon raises its `GameTooltip` under the icon, every number from the current
/// rank and stats, and what the next rank brings.
#[allow(clippy::too_many_arguments)]
pub(super) fn icon_tooltip(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    local: Query<&HeroState, With<LocalHero>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    font: Res<HudFont>,
    ui_scale: Res<UiScale>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    slots: Query<(
        &Interaction,
        &TipFor,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut frames: Query<(&mut Node, &mut Visibility), With<GameTooltip>>,
    mut built: Local<Option<(Entity, Entity)>>,
    mut shown: Local<Option<(TipFor, u32)>>,
) {
    let Some((frame, body)) = *built else {
        if let Some(assets) = assets.as_deref_mut() {
            *built = Some(spawn_tooltip(
                &mut commands,
                assets,
                &mut images,
                ui_scale.0,
            ));
        }
        return;
    };
    let Ok((mut node, mut vis)) = frames.get_mut(frame) else {
        return;
    };
    let hovered = slots
        .iter()
        .find(|(i, ..)| **i != Interaction::None)
        .filter(|_| *phase.get() != Phase::Menu);
    let Some((_, &tip, computed, at)) = hovered else {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
        *shown = None;
        return;
    };
    // Physical pixels to the 768-unit canvas the `Val`s are on.
    let inv = computed.inverse_scale_factor;
    let (left, top) = (
        Val::Px((at.translation.x - computed.size().x / 2.0) * inv),
        Val::Px((at.translation.y + computed.size().y / 2.0) * inv + 4.0),
    );
    if node.left != left || node.top != top {
        node.left = left;
        node.top = top;
    }
    if *shown == Some((tip, run.version)) {
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
        return;
    }
    let Ok(me) = local.single() else { return };
    let stats = me.stats();
    let lines = match tip {
        TipFor::Ability(i) => me
            .abilities
            .get(i)
            .map(|o| ability_tip(o, &stats, spells.as_deref())),
        TipFor::Passive(i) => me
            .passives
            .get(i)
            .map(|&(p, level)| passive_tip(p, level, &stats)),
        TipFor::Mobility => me.mobility(run.level).map(|m| {
            vec![
                TipLine::new(spell_name(spells.as_deref(), m.spell, m.name), Color::WHITE).header(),
                TipLine::new("Press F", Color::WHITE)
                    .right(format!("{} sec cooldown", num(m.cooldown)), Color::WHITE),
                TipLine::new(m.desc, GOLD),
            ]
        }),
    };
    let Some(lines) = lines else { return };
    *shown = Some((tip, run.version));
    // Shown from the next frame, once the new lines have laid out.
    *vis = Visibility::Hidden;
    let f = &font.0;
    commands.entity(body).despawn_related::<Children>();
    commands.entity(body).with_children(|b| {
        for line in lines {
            let face = |s: String, size: f32, color: Color| {
                (
                    Text::new(s),
                    TextFont {
                        font: f.clone(),
                        font_size: size,
                        ..default()
                    },
                    TextColor(color),
                    TextShadow {
                        offset: Vec2::splat(1.0),
                        color: Color::BLACK,
                    },
                )
            };
            b.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(16.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn(face(line.left, line.size, line.color));
                if let Some((text, color)) = line.right {
                    row.spawn((
                        face(text, line.size, color),
                        TextLayout::new(Justify::Right, LineBreak::NoWrap),
                    ));
                }
            });
        }
    });
}

#[allow(clippy::type_complexity)]
pub(super) fn show_panels(
    phase: Res<State<Phase>>,
    armory: Res<super::armory::ArmoryState>,
    faire: Res<super::faire::FaireUi>,
    mut roots: ParamSet<(
        Query<(&mut Visibility, &mut Node), With<MenuRoot>>,
        Query<(&mut Visibility, &mut Node), With<HudRoot>>,
        Query<(&mut Visibility, &mut Node), With<LevelUpRoot>>,
        Query<(&mut Visibility, &mut Node), With<PauseRoot>>,
        Query<(&mut Visibility, &mut Node), With<OverRoot>>,
    )>,
    next: Res<NextState<Phase>>,
) {
    let p = *phase.get();
    // A run starting from the menu this frame: the screens hold until it is under way, so "Play
    // again" keeps the results up through its one menu frame rather than flashing the menu.
    if p == Phase::Menu
        && matches!(
            *next,
            NextState::Pending(Phase::Playing) | NextState::PendingIfNeq(Phase::Playing)
        )
    {
        return;
    }
    // A hidden panel leaves the layout too: a merely invisible button still takes clicks.
    let set = |(mut v, mut node): (Mut<Visibility>, Mut<Node>), on: bool| {
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    };
    for root in &mut roots.p0() {
        // The Armory or the Faire covers the pick screen, which leaves the layout so it takes no
        // clicks.
        set(root, p == Phase::Menu && !armory.open && !faire.open);
    }
    for root in &mut roots.p1() {
        set(root, p != Phase::Menu && p != Phase::GameOver);
    }
    for root in &mut roots.p2() {
        set(root, p == Phase::LevelUp);
    }
    for root in &mut roots.p3() {
        set(root, p == Phase::Paused);
    }
    for root in &mut roots.p4() {
        set(root, p == Phase::GameOver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_note_prioritizes_first_zone_win() {
        let news = LastRunNews {
            zone: "Elwynn".to_string(),
            class: "Warrior".to_string(),
            first_zone_win: true,
            first_class_win: true,
            new_best_survival: true,
            ..default()
        };

        assert_eq!(results_note(&news), "First win in Elwynn!");
    }

    #[test]
    fn results_notes_clear_when_a_run_starts() {
        fn go(app: &mut App, phase: Phase) {
            app.world_mut()
                .resource_mut::<NextState<Phase>>()
                .set(phase);
            app.update();
        }
        let mut app = App::new();
        app.add_plugins((bevy::state::app::StatesPlugin, notes_plugin))
            .init_state::<Phase>();
        app.update();
        app.world_mut()
            .resource_mut::<ResultsNotes>()
            .push("Last run's drop", NoteKind::Loot);
        go(&mut app, Phase::Playing);
        assert!(app.world().resource::<ResultsNotes>().0.is_empty());
        // Pushed during the run, kept through the results and back at the menu.
        app.world_mut()
            .resource_mut::<ResultsNotes>()
            .push("Drop", NoteKind::Loot);
        go(&mut app, Phase::GameOver);
        app.world_mut()
            .resource_mut::<ResultsNotes>()
            .push("Achievement", NoteKind::Achievement);
        go(&mut app, Phase::Menu);
        assert_eq!(
            app.world().resource::<ResultsNotes>().0,
            [
                ("Drop".to_string(), NoteKind::Loot),
                ("Achievement".to_string(), NoteKind::Achievement),
            ]
        );
        go(&mut app, Phase::Playing);
        assert!(app.world().resource::<ResultsNotes>().0.is_empty());
    }

    #[test]
    fn note_kinds_have_distinct_colours() {
        let kinds = [
            NoteKind::Info,
            NoteKind::Good,
            NoteKind::Loot,
            NoteKind::Achievement,
        ];
        for (i, a) in kinds.iter().enumerate() {
            for b in &kinds[i + 1..] {
                assert_ne!(a.color(), b.color(), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn mastered_build_hands_offer_overcharges_and_a_healthstone() {
        let mut hero = HeroState::default();
        hero.abilities = hero.class.pool[..data::MAX_ABILITIES]
            .iter()
            .map(|key| Owned {
                def: data::ability(key),
                level: data::MAX_ABILITY_LEVEL,
                cd: 0.0,
            })
            .collect();
        hero.passives = data::PASSIVES[..data::MAX_PASSIVES]
            .iter()
            .map(|passive| (passive, data::MAX_PASSIVE_LEVEL))
            .collect();
        let mut rng = Rng::default();
        let hand = choices_for(&hero, &mut rng);
        assert_eq!(hand.len(), 3);
        assert!(hand.iter().any(|choice| matches!(choice, Choice::Heal)));
        assert_eq!(
            hand.iter()
                .filter(|choice| matches!(choice, Choice::Overcharge(_)))
                .count(),
            2
        );
    }

    /// The results and their ways out: the keys and buttons, the record on GameOver and the reset
    /// on Menu, with a local hero standing in for the menu's (`menu_preview` needs the world).
    fn results_app() -> App {
        use super::super::{abilities, enemies, profile, zones};
        // Unsaved changes, so the record never reloads the save on disk.
        let mut saved = profile::Profile::default();
        saved.mark_dirty();
        let mut app = App::new();
        app.add_plugins((bevy::state::app::StatesPlugin, notes_plugin))
            .init_state::<Phase>()
            .insert_resource(saved)
            .insert_resource(Selection {
                class: Some(2),
                looks: vec![(1, 1, 0)],
                look: Some(0),
                ..default()
            })
            .insert_resource(zones::Unlocks::from_profile(
                &profile::Profile::default(),
                true,
            ))
            .init_resource::<Run>()
            .init_resource::<MetaBonuses>()
            .init_resource::<Replay>()
            .init_resource::<LastRunNews>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<abilities::PendingHits>()
            .init_resource::<enemies::Spawner>()
            .add_message::<profile::RunEnded>()
            .add_message::<AppExit>()
            .add_systems(Update, (buttons, keys).chain())
            .add_systems(OnEnter(Phase::GameOver), profile::record_finished)
            .add_systems(
                OnEnter(Phase::Menu),
                (super::super::reset_run, start_replay).chain(),
            );
        app.world_mut().spawn((HeroState::default(), LocalHero));
        app.update();
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
    }

    fn click(app: &mut App, action: UiAction) {
        let button = app
            .world_mut()
            .spawn((
                Interaction::Pressed,
                action,
                BaseColor(BUTTON),
                BackgroundColor(BUTTON),
            ))
            .id();
        app.update();
        app.world_mut().despawn(button);
    }

    fn phase(app: &App) -> Phase {
        *app.world().resource::<State<Phase>>().get()
    }

    fn local(world: &mut World) -> Entity {
        world
            .query_filtered::<Entity, With<LocalHero>>()
            .single(world)
            .unwrap()
    }

    fn hero(app: &mut App) -> &HeroState {
        let e = local(app.world_mut());
        app.world().get::<HeroState>(e).unwrap()
    }

    fn runs(app: &App) -> u32 {
        app.world()
            .resource::<super::super::profile::Profile>()
            .account
            .runs
    }

    /// From the menu, Enter World and a run lost: kills on the clock, the hero down, a note
    /// pushed, and the results up.
    fn lose_a_run(app: &mut App) {
        assert_eq!(phase(app), Phase::Menu);
        press(app, KeyCode::Enter);
        app.update();
        assert_eq!(phase(app), Phase::Playing);
        let world = app.world_mut();
        let mut run = world.resource_mut::<Run>();
        (run.kills, run.elapsed) = (9, 100.0);
        world
            .resource_mut::<ResultsNotes>()
            .push("Drop", NoteKind::Loot);
        let e = local(world);
        let mut me = world.get_mut::<HeroState>(e).unwrap();
        (me.hp, me.dead_t, me.damage_done, me.rerolls) = (0.0, Some(3.5), 500.0, 0);
        world
            .resource_mut::<NextState<Phase>>()
            .set(Phase::GameOver);
        app.update();
        assert_eq!(phase(app), Phase::GameOver);
        assert!(app.world().resource::<Run>().recorded);
    }

    #[test]
    fn play_again_starts_a_fresh_run_on_the_same_pick() {
        let mut app = results_app();
        lose_a_run(&mut app);
        assert_eq!(runs(&app), 1);

        // Enter on the results: its invisible Menu transition resets and begins the next run.
        press(&mut app, KeyCode::Enter);
        app.update();
        assert_eq!(phase(&app), Phase::Menu);
        assert!(
            !app.world().resource::<Replay>().0,
            "taken on the menu frame"
        );
        assert!(matches!(
            *app.world().resource::<NextState<Phase>>(),
            NextState::Pending(Phase::Playing)
        ));
        app.update();
        assert_eq!(phase(&app), Phase::Playing);
        assert_eq!(runs(&app), 1, "the lost run is recorded once");
        let run = app.world().resource::<Run>();
        assert_eq!((run.kills, run.elapsed, run.recorded), (0, 0.0, false));
        assert_eq!(run.version, 1, "begun by start_run");
        let selection = app.world().resource::<Selection>();
        assert_eq!(
            (selection.class, selection.look, selection.map),
            (Some(2), Some(0), 0)
        );
        // Through the menu's own Menu → Playing transition, which clears the notes.
        assert!(app.world().resource::<ResultsNotes>().0.is_empty());
        let me = hero(&mut app);
        assert_eq!((me.class.name, me.race), (data::CLASSES[2].name, 1));
        assert_eq!(me.dead_t, None);
        assert_eq!(me.hp, me.stats().max_hp);
        assert_eq!((me.damage_done, me.rerolls), (0.0, hand_rerolls()));

        // The play-again button does the same, and that run is recorded in its turn.
        app.world_mut()
            .resource_mut::<NextState<Phase>>()
            .set(Phase::GameOver);
        app.update();
        assert_eq!(runs(&app), 2);
        click(&mut app, UiAction::PlayAgain);
        app.update();
        app.update();
        assert_eq!(phase(&app), Phase::Playing);
        assert_eq!(runs(&app), 2);
        assert_eq!(app.world().resource::<Run>().version, 1);
    }

    fn hand_rerolls() -> u32 {
        HeroState::default().rerolls
    }

    #[test]
    fn main_menu_returns_without_starting_a_run() {
        let mut app = results_app();
        lose_a_run(&mut app);
        click(&mut app, UiAction::MainMenu);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(phase(&app), Phase::Menu);
        assert_eq!(runs(&app), 1, "recorded once, on GameOver");
        let run = app.world().resource::<Run>();
        assert_eq!((run.kills, run.version), (0, 0), "reset, and no run begun");
        assert!(!app.world().resource::<Replay>().0);
        assert!(hero(&mut app).dead_t.is_some(), "no run began on the hero");
        // The notes stay for the menu until a run begins.
        assert!(!app.world().resource::<ResultsNotes>().0.is_empty());

        // Esc on the results goes the same way.
        lose_a_run(&mut app);
        assert_eq!(runs(&app), 2);
        press(&mut app, KeyCode::Escape);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(phase(&app), Phase::Menu);
        assert_eq!(runs(&app), 2);
        assert_eq!(app.world().resource::<Run>().version, 0);
    }
}
