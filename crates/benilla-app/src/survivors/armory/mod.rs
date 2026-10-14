//! The Armory: a full-screen panel over the pick screen for what a hero keeps between runs, opened
//! by the menu's Armory button and closed by Done or Esc. While it is open the pick screen is
//! hidden, so its clicks and keys (Enter World above all) cannot reach the menu behind it.
//!
//! Laid out on the survivors' 768-unit canvas in the glue screens' art: the title and the class
//! line top left, the tab bar under them (`UI-Character-*Tab`, turned to hang from the top), and
//! the tab content area below, which [`columns`] splits into a framed left column, the center and
//! a framed Details column. Done sits bottom right.
//!
//! [`sync_armory`] builds the chrome on open and despawns it on close. Switching tabs despawns and
//! rebuilds only the content area, by calling the tab's own builder.
//!
//! # Adding content to a tab
//!
//! Each tab lives in its own file (`character.rs`, `collection.rs`, `drop_tables.rs`, `codex.rs`,
//! `talents.rs`, `achievements.rs`, `rewards.rs`) behind one entry point:
//!
//! ```ignore
//! pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx)
//! ```
//!
//! `parent` is the content area, a flex row. Replace the placeholder's `coming_soon` call with
//! your own nodes; nothing in this file needs to change.
//!
//! - [`columns`] gives the standard left / center / Details split, [`panel`] one framed box, and
//!   [`text`] / [`para`] the outlined glue strings. Glue builders here take a unit scale of `1.0`:
//!   `UiScale` already maps the canvas to the window.
//! - [`TabCtx`] carries the glue art, the font and the pick, and [`TabCtx::res`] reads any other
//!   resource (a save profile, the spell catalog) straight from the world.
//! - Put an [`ArmoryTip`] on a node with an [`Interaction`] (a `Button`) and it gets a
//!   `GameTooltip`-style hover box. A [`Hilight`] child lights while the row is hovered.
//! - A tab whose data changes while it is open calls [`ArmoryState::refresh`] to be rebuilt.
//!
//! `WOW_SURVIVORS_ARMORY=<tab>` opens the Armory on that tab once the menu is up, and with
//! `WOW_SURVIVORS_SHOTS=<dir>` saves `<dir>/armory-<tab>.png` 1.5 s later; the directory must
//! already exist.

mod achievements;
mod character;
mod codex;
mod collection;
mod drop_tables;
mod rewards;
mod talents;

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::ui::FocusPolicy;
use bevy::ui_render::ui_material::MaterialNode;
use bevy::window::PrimaryWindow;

use super::data::ClassDef;
use super::hud::{self, HudFont, MenuRoot};
use super::menu::{self, MenuBuilt};
use super::LocalHero;
use super::{Phase, Selection};
use crate::glue::art::{tc_rect, GlueArt, GOLD, NAME_EDGE, REALM_TAB_TC};
use crate::glue::backdrop::{backdrop_border, tiled_bg_node};
use crate::glue::widgets::{
    glue_button, outlined_text, overlay, FallbackFace, GlueBtnKind, GlueDisabled, GlueText, Hilight,
};
use crate::glue::GlueClicks;

/// The pick screen's zone-frame tints (`CharacterSelectCharacterFrame`), for every Armory panel.
const FRAME_BORDER: Color = Color::srgb(0.8, 0.8, 0.8);
const FRAME_FILL: Color = Color::srgba(0.09, 0.09, 0.09, 0.85);
/// `TOOLTIP_DEFAULT_BACKGROUND_COLOR`, the `GameTooltip` fill.
const TIP_FILL: Color = Color::srgb(0.09, 0.09, 0.19);
/// The subtitle's parchment white.
const SUBTITLE: Color = Color::srgb(0.9, 0.85, 0.7);

/// Above the pick screen (410), under the level-up cards (420) it never shares a frame with.
const Z: i32 = 415;
/// The canvas margin round the whole panel.
const MARGIN: f32 = 24.0;
const TITLE_TOP: f32 = 12.0;
const SUBTITLE_TOP: f32 = 46.0;
const TAB_TOP: f32 = 68.0;
const TAB_H: f32 = 32.0;
/// `$parentLeft` / `$parentRight`, 20 wide, round a middle as wide as the label.
const TAB_END: f32 = 20.0;
/// Each next tab's left at the one before's right −15, as `RealmListTabButtonTemplate` chains.
const TAB_OVERLAP: f32 = 15.0;
/// The bottom tabs' front art stands 3 higher and the text 2; hung from the top, both drop.
const TAB_FRONT_DROP: f32 = 3.0;
const TAB_TEXT_DROP: f32 = 2.0;
/// `$parentHighlightTexture`, 10 in from each side.
const TAB_HILIGHT_INSET: f32 = 10.0;
/// The content area: under the tabs, over the Done row.
const BODY_TOP: f32 = TAB_TOP + TAB_H;
const BODY_BOTTOM: f32 = 66.0;
/// The left and Details columns; equal, so the center (and the menu's hero) stays centred.
const SIDE_W: f32 = 250.0;
const GAP: f32 = 12.0;

/// One of the Armory's tabs, in tab-bar order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub(super) enum ArmoryTab {
    #[default]
    Character,
    Collection,
    DropTables,
    Codex,
    Talents,
    Achievements,
    Rewards,
}

impl ArmoryTab {
    pub(super) const ALL: [Self; 7] = [
        Self::Character,
        Self::Collection,
        Self::DropTables,
        Self::Codex,
        Self::Talents,
        Self::Achievements,
        Self::Rewards,
    ];

    /// The tab bar's caption.
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Character => "Character",
            Self::Collection => "Collection",
            Self::DropTables => "Drop Tables",
            Self::Codex => "Codex",
            Self::Talents => "Talents",
            Self::Achievements => "Achievements",
            Self::Rewards => "Rewards",
        }
    }

    /// The tab's name in `WOW_SURVIVORS_ARMORY` and the screenshot's file name.
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Collection => "collection",
            Self::DropTables => "drop-tables",
            Self::Codex => "codex",
            Self::Talents => "talents",
            Self::Achievements => "achievements",
            Self::Rewards => "rewards",
        }
    }

    /// The tab a name means, its key or caption in any case and with any spaces, `_` or `-`.
    pub(super) fn from_name(name: &str) -> Option<Self> {
        let squash = |s: &str| {
            s.chars()
                .filter(|c| !matches!(c, ' ' | '_' | '-'))
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let want = squash(name);
        Self::ALL.into_iter().find(|t| squash(t.key()) == want)
    }

    /// Build the tab's content into the content area.
    fn build(self, parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
        match self {
            Self::Character => character::build(parent, ctx),
            Self::Collection => collection::build(parent, ctx),
            Self::DropTables => drop_tables::build(parent, ctx),
            Self::Codex => codex::build(parent, ctx),
            Self::Talents => talents::build(parent, ctx),
            Self::Achievements => achievements::build(parent, ctx),
            Self::Rewards => rewards::build(parent, ctx),
        }
    }
}

/// Whether the Armory is up, and on which tab; the tab is kept while closed, so it reopens there.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) struct ArmoryState {
    pub open: bool,
    pub tab: ArmoryTab,
    /// Bumped by [`Self::refresh`]; the content is rebuilt whenever it moves.
    pub rebuilds: u32,
}

impl ArmoryState {
    /// Open on `tab`. True when anything changed.
    pub fn open(&mut self, tab: ArmoryTab) -> bool {
        let changed = !self.open || self.tab != tab;
        self.open = true;
        self.tab = tab;
        changed
    }

    /// Close, keeping the tab. True when it was open.
    pub fn close(&mut self) -> bool {
        let was = self.open;
        self.open = false;
        was
    }

    /// Show `tab` while open; closed, nothing changes. True when the tab changed.
    pub fn switch(&mut self, tab: ArmoryTab) -> bool {
        if !self.open || self.tab == tab {
            return false;
        }
        self.tab = tab;
        true
    }

    /// Rebuild the open tab's content, for a tab whose data changed under it.
    pub fn refresh(&mut self) {
        self.rebuilds = self.rebuilds.wrapping_add(1);
    }
}

/// Run condition: the Armory is closed, so the pick screen's own input may run.
pub(super) fn closed(state: Res<ArmoryState>) -> bool {
    !state.open
}

/// What a tab's builder draws with: the glue art set, the font, the pick, and the world for
/// anything else.
pub(super) struct TabCtx<'w> {
    /// The glue screens' art (buttons, borders, highlights, the class and race icon sheets).
    pub art: &'w GlueArt,
    /// The survivors' glue font.
    pub font: &'w Handle<Font>,
    /// The class, look and zone picked at the menu.
    pub selection: &'w Selection,
    /// Read-only, for the resources a tab needs that the fields above do not carry.
    pub world: &'w World,
}

impl TabCtx<'_> {
    /// The picked class.
    pub fn class(&self) -> Option<&'static ClassDef> {
        self.selection.class_def()
    }

    /// The picked look's `(race, sex)`, sex 0 male and 1 female.
    pub fn race_sex(&self) -> Option<(u8, u8)> {
        let sel = self.selection;
        sel.look
            .and_then(|i| sel.looks.get(i))
            .map(|&(_, race, sex)| (race, sex))
    }

    /// Any resource, `None` while it is not in the world.
    pub fn res<T: Resource>(&self) -> Option<&T> {
        self.world.get_resource::<T>()
    }
}

/// The Armory's buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ArmoryAction {
    Open,
    Close,
    Tab(ArmoryTab),
}

/// Hover text for a node in a tab, shown in a `GameTooltip`-style box beside it. The node needs an
/// [`Interaction`]: make it a `Button`.
#[derive(Component, Clone, PartialEq, Debug)]
pub(super) struct ArmoryTip {
    /// The white header line.
    pub title: String,
    /// The lines under it, wrapped, each in its own colour.
    pub lines: Vec<(String, Color)>,
}

#[derive(Component)]
struct ArmoryRoot;
/// A tab's button.
#[derive(Component)]
struct TabOf(ArmoryTab);
/// One piece of a tab's art: the front (selected) set or the back one.
#[derive(Component)]
struct TabArt {
    tab: ArmoryTab,
    front: bool,
}
#[derive(Component)]
struct TabLabel(ArmoryTab);
#[derive(Component)]
struct TipFrame;
#[derive(Component)]
struct TipLines;

/// Which of the content area's three columns a [`columns`] fill is building.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Column {
    Left,
    Center,
    Details,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ArmoryState>().add_systems(
        Update,
        (armory_input, sync_armory, paint_tabs, armory_tooltip)
            .chain()
            // After the menu's own input has had its (gated) turn, and before the pick screen is
            // shown or hidden, so open and close land in one frame.
            .after(hud::keys)
            .after(menu::menu_clicks)
            .before(hud::show_panels),
    );
    if let Some(aid) = DevAid::from_env() {
        app.insert_resource(aid)
            .add_systems(Update, dev_aid.before(armory_input));
    }
}

/// The menu's Armory button, under the race and class tower, centred on it as Randomize is.
pub(super) fn entry_button(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        // The tower: 206 wide at (28, 74), 590 tall; Randomize is 146 wide at its x 30.
        left: Val::Px(28.0 + 30.0),
        top: Val::Px(74.0 + 590.0 + 14.0),
        ..default()
    })
    .with_children(|b| {
        glue_button(
            b,
            art,
            f,
            ArmoryAction::Open,
            "Armory",
            146.0,
            30.0,
            GlueBtnKind::Small,
            1.0,
        );
    });
}

/// The Armory's clicks, and Esc to close it; Esc does nothing else while it is open.
fn armory_input(
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    actions: Query<(Entity, &ArmoryAction)>,
    mut state: ResMut<ArmoryState>,
) {
    if *phase.get() != Phase::Menu {
        return;
    }
    if state.open && keys.just_pressed(KeyCode::Escape) {
        state.close();
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
        ArmoryAction::Open => {
            let tab = state.tab;
            state.open(tab);
        }
        ArmoryAction::Close => {
            state.close();
        }
        ArmoryAction::Tab(tab) => {
            state.switch(tab);
        }
    }
}

/// What [`sync_armory`] last built: the root, its content area, and what the content shows.
struct Built {
    root: Entity,
    content: Entity,
    tab: ArmoryTab,
    rebuilds: u32,
}

/// Bring the screen in line with [`ArmoryState`]: build the Armory on open, despawn it on close,
/// and rebuild only the content area on a tab switch or a refresh. Leaving the menu (the
/// auto-runner starting a run) closes it.
fn sync_armory(world: &mut World, mut built: Local<Option<Built>>) {
    let in_menu = *world.resource::<State<Phase>>().get() == Phase::Menu;
    if !in_menu && world.resource::<ArmoryState>().open {
        world.resource_mut::<ArmoryState>().close();
    }
    let state = *world.resource::<ArmoryState>();
    if built
        .as_ref()
        .is_some_and(|b| world.get_entity(b.root).is_err())
    {
        *built = None;
    }
    if !state.open {
        if let Some(b) = built.take() {
            world.entity_mut(b.root).despawn();
        }
        return;
    }
    if built
        .as_ref()
        .is_some_and(|b| b.tab == state.tab && b.rebuilds == state.rebuilds)
    {
        return;
    }
    let (Some(art), Some(font)) = (
        world.get_resource::<GlueArt>(),
        world.get_resource::<HudFont>(),
    ) else {
        return;
    };
    let mut queue = CommandQueue::default();
    let (root, content) = {
        let ctx = TabCtx {
            art,
            font: &font.0,
            selection: world.resource::<Selection>(),
            world,
        };
        let mut commands = Commands::new(&mut queue, world);
        let (root, content) = match built.as_ref() {
            Some(b) => {
                commands.entity(b.content).despawn_related::<Children>();
                (b.root, b.content)
            }
            None => spawn_chrome(&mut commands, &ctx),
        };
        commands
            .entity(content)
            .with_children(|p| state.tab.build(p, &ctx));
        (root, content)
    };
    queue.apply(world);
    *built = Some(Built {
        root,
        content,
        tab: state.tab,
        rebuilds: state.rebuilds,
    });
}

/// The Armory's frame: the shade, the title and class line, the tab bar, the empty content area,
/// Done, and the hidden tooltip. Returns the root and the content area.
fn spawn_chrome(commands: &mut Commands, ctx: &TabCtx) -> (Entity, Entity) {
    let px = Val::Px;
    let f = ctx.font;
    let subtitle = match ctx.class() {
        Some(c) => format!(
            "|cff{:06x}{}|r: gear and looks are kept per class",
            c.color, c.name
        ),
        None => "Gear and looks are kept per class".to_string(),
    };
    let mut content = Entity::PLACEHOLDER;
    let mut root = commands.spawn((
        ArmoryRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        // Dark behind the title and Done, clear across the middle where the hero stands.
        BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.85), 0.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.4), 14.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.1), 30.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.1), 80.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.75), 100.0),
        ])),
        GlobalZIndex(Z),
        // Nothing under the Armory takes the cursor, the hidden pick screen least of all.
        FocusPolicy::Block,
    ));
    root.with_children(|r| {
        outlined_text(
            r,
            Node {
                position_type: PositionType::Absolute,
                left: px(MARGIN + 4.0),
                top: px(TITLE_TOP),
                ..default()
            },
            (),
            (),
            GlueText {
                text: "Armory",
                size: 28.0,
                color: GOLD,
                wrap: false,
            },
            f,
            1.0,
        );
        outlined_text(
            r,
            Node {
                position_type: PositionType::Absolute,
                left: px(MARGIN + 6.0),
                top: px(SUBTITLE_TOP),
                ..default()
            },
            (),
            (),
            GlueText {
                text: &subtitle,
                size: 13.0,
                color: SUBTITLE,
                wrap: false,
            },
            f,
            1.0,
        );
        tab_bar(r, ctx);
        content = r
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(MARGIN),
                right: px(MARGIN),
                top: px(BODY_TOP),
                bottom: px(BODY_BOTTOM),
                flex_direction: FlexDirection::Row,
                column_gap: px(GAP),
                ..default()
            })
            .id();
        r.spawn(Node {
            position_type: PositionType::Absolute,
            right: px(MARGIN),
            bottom: px(12.0),
            ..default()
        })
        .with_children(|b| {
            glue_button(
                b,
                ctx.art,
                f,
                ArmoryAction::Close,
                "Done",
                170.0,
                45.0,
                GlueBtnKind::Normal,
                1.0,
            );
        });
        tooltip(r, ctx.art);
    });
    (root.id(), content)
}

/// The tab bar: `UI-Character-ActiveTab` / `-InActiveTab` (as `RealmListTabButtonTemplate` draws
/// them) flipped to hang from the top, each tab its two 20-wide ends round its label, chained 15
/// into the one before. [`paint_tabs`] picks each tab's art set.
fn tab_bar(r: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let px = Val::Px;
    let art = ctx.art;
    let has_art = art.tab_active.is_some() || art.tab_inactive.is_some();
    r.spawn(Node {
        position_type: PositionType::Absolute,
        left: px(MARGIN),
        top: px(TAB_TOP),
        height: px(TAB_H),
        flex_direction: FlexDirection::Row,
        ..default()
    })
    .with_children(|strip| {
        for (i, tab) in ArmoryTab::ALL.into_iter().enumerate() {
            let mut b = strip.spawn((
                ArmoryAction::Tab(tab),
                TabOf(tab),
                Button,
                GlueDisabled(false),
                Node {
                    height: px(TAB_H),
                    flex_shrink: 0.0,
                    margin: UiRect::left(px(if i == 0 { 0.0 } else { -TAB_OVERLAP })),
                    padding: UiRect::horizontal(px(TAB_END)),
                    align_items: AlignItems::Center,
                    ..default()
                },
            ));
            if !has_art {
                b.insert((FallbackFace, BackgroundColor(crate::glue::art::BTN_BG)));
            }
            b.with_children(|t| {
                for (front, sheet) in [(false, &art.tab_inactive), (true, &art.tab_active)] {
                    let Some((tex, size)) = sheet else { continue };
                    for (piece, tc) in REALM_TAB_TC.iter().enumerate() {
                        let (left, right, width) = match piece {
                            0 => (px(0.0), Val::Auto, px(TAB_END)),
                            1 => (px(TAB_END), px(TAB_END), Val::Auto),
                            _ => (Val::Auto, px(0.0), px(TAB_END)),
                        };
                        t.spawn((
                            TabArt { tab, front },
                            Visibility::Hidden,
                            ImageNode {
                                image: tex.clone(),
                                rect: Some(tc_rect(*size, *tc)),
                                flip_y: true,
                                ..default()
                            },
                            Node {
                                position_type: PositionType::Absolute,
                                left,
                                right,
                                width,
                                top: px(if front { TAB_FRONT_DROP } else { 0.0 }),
                                height: px(TAB_H),
                                ..default()
                            },
                        ));
                    }
                }
                if let Some(hi) = &art.tab_highlight {
                    t.spawn((
                        Hilight,
                        Visibility::Hidden,
                        MaterialNode(hi.clone()),
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(TAB_HILIGHT_INSET),
                            right: px(TAB_HILIGHT_INSET),
                            top: px(TAB_TEXT_DROP),
                            height: px(TAB_H),
                            ..default()
                        },
                    ));
                }
                outlined_text(
                    t,
                    Node {
                        top: px(TAB_TEXT_DROP),
                        ..default()
                    },
                    (),
                    TabLabel(tab),
                    GlueText {
                        text: tab.label(),
                        size: 12.0, // GlueFontNormalSmall
                        color: GOLD,
                        wrap: false,
                    },
                    ctx.font,
                    1.0,
                );
            });
        }
    });
}

/// The `GameTooltip` box [`armory_tooltip`] fills: `UI-Tooltip-Background` under
/// `UI-Tooltip-Border`, 16-unit edges, 5-unit insets.
fn tooltip(r: &mut ChildSpawnerCommands, art: &GlueArt) {
    let px = Val::Px;
    let mut frame = r.spawn((
        TipFrame,
        Node {
            position_type: PositionType::Absolute,
            max_width: px(280.0),
            ..default()
        },
        GlobalZIndex(1000),
        Visibility::Hidden,
        FocusPolicy::Pass,
    ));
    let framed = art.tooltip_bg.is_some() && art.tooltip_border.is_some();
    if !framed {
        frame.insert(BackgroundColor(TIP_FILL.with_alpha(0.95)));
    }
    frame.with_children(|t| {
        if let (Some(bg), Some(edges)) = (&art.tooltip_bg, &art.tooltip_border) {
            t.spawn((
                tiled_bg_node(bg.clone(), 16.0, 1.0, TIP_FILL),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(5.0),
                    right: px(5.0),
                    top: px(5.0),
                    bottom: px(5.0),
                    ..default()
                },
            ));
            backdrop_border(t, edges, 16.0, Color::WHITE);
        }
        t.spawn((
            TipLines,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3.0),
                padding: UiRect::all(px(10.0)),
                ..default()
            },
        ));
    });
}

/// Each tab's look: the front art, white caption and no hover sheen for the open tab, drawn over
/// its neighbours; the back art and a gold caption (white on hover) for the rest.
#[allow(clippy::type_complexity)]
fn paint_tabs(
    state: Res<ArmoryState>,
    mut tabs: Query<(&TabOf, &Interaction, &mut GlueDisabled, &mut ZIndex)>,
    mut arts: Query<(&TabArt, &mut Visibility)>,
    mut labels: Query<(&TabLabel, &mut TextColor)>,
) {
    let mut hovered = [false; ArmoryTab::ALL.len()];
    for (tab, interaction, mut disabled, mut z) in &mut tabs {
        let front = tab.0 == state.tab;
        hovered[tab.0 as usize] = *interaction != Interaction::None;
        if disabled.0 != front {
            disabled.0 = front;
        }
        let want = ZIndex(i32::from(front));
        if *z != want {
            *z = want;
        }
    }
    for (art, mut vis) in &mut arts {
        let want = if art.front == (art.tab == state.tab) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    for (label, mut color) in &mut labels {
        let want = if label.0 == state.tab || hovered[label.0 as usize] {
            Color::WHITE
        } else {
            GOLD
        };
        if color.0 != want {
            color.0 = want;
        }
    }
}

/// Hovering a node with an [`ArmoryTip`] raises the tooltip beside it, on whichever side has the
/// room, kept on the screen.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn armory_tooltip(
    mut commands: Commands,
    font: Res<HudFont>,
    ui_scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    tips: Query<(
        Entity,
        &Interaction,
        &ArmoryTip,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut frames: Query<(&mut Node, &mut Visibility, &ComputedNode), With<TipFrame>>,
    lines: Query<Entity, With<TipLines>>,
    mut shown: Local<Option<Entity>>,
) {
    let (Ok((mut node, mut vis, frame)), Ok(body)) = (frames.single_mut(), lines.single()) else {
        *shown = None;
        return;
    };
    let Some((e, _, tip, computed, at)) = tips.iter().find(|(_, i, ..)| **i != Interaction::None)
    else {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
        *shown = None;
        return;
    };
    if *shown != Some(e) {
        *shown = Some(e);
        // Shown from the next frame, once the new lines have laid out.
        *vis = Visibility::Hidden;
        commands.entity(body).despawn_related::<Children>();
        commands.entity(body).with_children(|b| {
            let face = |s: &str, size: f32, color: Color| {
                (
                    Text::new(s),
                    TextFont {
                        font: font.0.clone(),
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
            b.spawn(face(&tip.title, 14.0, Color::WHITE));
            for (line, color) in &tip.lines {
                b.spawn(face(line, 12.0, *color));
            }
        });
        return;
    }
    // Physical pixels to the 768-unit canvas the `Val`s are on.
    let inv = computed.inverse_scale_factor;
    let (center, half) = (at.translation * inv, computed.size() * inv / 2.0);
    let size = frame.size() * frame.inverse_scale_factor;
    let canvas = windows.single().map_or(Vec2::new(1024.0, 768.0), |w| {
        w.size() / ui_scale.0.max(1e-3)
    });
    let right = center.x + half.x + 8.0;
    let left = if right + size.x <= canvas.x - 4.0 {
        right
    } else {
        center.x - half.x - 8.0 - size.x
    };
    let top = (center.y - half.y).min(canvas.y - size.y - 4.0).max(4.0);
    let (left, top) = (Val::Px(left), Val::Px(top));
    if node.left != left || node.top != top {
        node.left = left;
        node.top = top;
    }
    if *vis != Visibility::Inherited {
        *vis = Visibility::Inherited;
    }
}

// ── The tab builders' kit ───────────────────────────────────────────────────────────────────────

/// One outlined glue string in the flow, as the pick screen draws its labels.
pub(super) fn text(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    s: &str,
    size: f32,
    color: Color,
) -> Entity {
    outlined_text(
        p,
        Node::default(),
        (),
        (),
        GlueText {
            text: s,
            size,
            color,
            wrap: false,
        },
        ctx.font,
        1.0,
    )
}

/// [`text`], wrapped to the width of its column.
pub(super) fn para(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    s: &str,
    size: f32,
    color: Color,
) -> Entity {
    outlined_text(
        p,
        Node {
            width: Val::Percent(100.0),
            ..default()
        },
        (),
        (),
        GlueText {
            text: s,
            size,
            color,
            wrap: true,
        },
        ctx.font,
        1.0,
    )
}

/// A framed box in the pick screen's zone-frame look (`Glue-Tooltip-Border` over the tiled
/// tooltip fill), sized by `node`, with an optional gold title; `body` fills its inner column.
pub(super) fn panel(
    parent: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    node: Node,
    title: Option<&str>,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let px = Val::Px;
    parent.spawn(node).with_children(|frame| {
        if let (Some(bg), Some(border)) = (&ctx.art.tooltip_bg, &ctx.art.name_border) {
            // `Glue-Tooltip-Border`'s insets, as the zone frame and the glue edit boxes draw it.
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
            frame.spawn((BackgroundColor(FRAME_FILL), overlay()));
        }
        if let Some(title) = title {
            outlined_text(
                frame,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(14.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                (),
                (),
                GlueText {
                    text: title,
                    size: 16.0,
                    color: GOLD,
                    wrap: false,
                },
                ctx.font,
                1.0,
            );
        }
        frame
            .spawn(Node {
                position_type: PositionType::Absolute,
                left: px(18.0),
                right: px(13.0),
                top: px(if title.is_some() { 42.0 } else { 14.0 }),
                bottom: px(18.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(body);
    });
}

/// The content area's standard split: a framed left column titled `left_title`, the center
/// (framed, or clear over the menu's 3D scene and its hero when `clear_center`), and the framed
/// Details column. `fill` is called once per column to build its contents.
pub(super) fn columns(
    parent: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    left_title: Option<&str>,
    clear_center: bool,
    mut fill: impl FnMut(Column, &mut ChildSpawnerCommands),
) {
    let side = || Node {
        width: Val::Px(SIDE_W),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    };
    let center = Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    panel(parent, ctx, side(), left_title, |b| fill(Column::Left, b));
    if clear_center {
        parent
            .spawn(center)
            .with_children(|b| fill(Column::Center, b));
    } else {
        panel(parent, ctx, center, None, |b| fill(Column::Center, b));
    }
    panel(parent, ctx, side(), Some("Details"), |b| {
        fill(Column::Details, b)
    });
}

/// A placeholder tab: the three columns, empty but for a centred "Coming soon".
pub(super) fn coming_soon(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    columns(parent, ctx, None, false, |col, p| {
        if col != Column::Center {
            return;
        }
        p.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|c| {
            text(c, ctx, "Coming soon", 18.0, SUBTITLE);
        });
    });
}

// ── `WOW_SURVIVORS_ARMORY` ──────────────────────────────────────────────────────────────────────

/// The screenshot aid: open on a tab once the menu is up, and maybe photograph it.
#[derive(Resource)]
struct DevAid {
    tab: ArmoryTab,
    shots: Option<std::path::PathBuf>,
    /// Seconds the menu has been up, waiting for the hero.
    menu_t: f32,
    /// Seconds the menu's hero has stood there, fading in.
    hero_t: f32,
    /// Seconds since the Armory opened.
    open_t: Option<f32>,
    done: bool,
}

impl DevAid {
    fn from_env() -> Option<Self> {
        let name = crate::webenv::var("WOW_SURVIVORS_ARMORY")?;
        let Some(tab) = ArmoryTab::from_name(&name) else {
            warn!("survivors armory: no tab named {name:?}");
            return None;
        };
        Some(Self {
            tab,
            shots: crate::webenv::var("WOW_SURVIVORS_SHOTS").map(std::path::PathBuf::from),
            menu_t: 0.0,
            hero_t: 0.0,
            open_t: None,
            done: false,
        })
    }
}

/// Seconds after opening before the shot, for the art to settle.
const SHOT_AFTER: f32 = 1.5;
/// The menu's hero fades in after it spawns; the Armory opens once it has.
const HERO_SETTLE: f32 = 2.0;
/// How long the menu waits for its hero before opening anyway (a first launch has no pick).
const HERO_WAIT: f32 = 8.0;

#[allow(clippy::too_many_arguments)]
fn dev_aid(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut aid: ResMut<DevAid>,
    phase: Res<State<Phase>>,
    menu: Query<(), (With<MenuRoot>, With<MenuBuilt>)>,
    heroes: Query<(), With<LocalHero>>,
    loading: Res<crate::loading_screen::LoadingScreen>,
    mut state: ResMut<ArmoryState>,
) {
    if aid.done || *phase.get() != Phase::Menu {
        return;
    }
    let dt = time.delta_secs();
    let Some(t) = aid.open_t else {
        if menu.is_empty() || loading.covering() {
            aid.menu_t = 0.0;
            aid.hero_t = 0.0;
            return;
        }
        aid.menu_t += dt;
        aid.hero_t = if heroes.is_empty() {
            0.0
        } else {
            aid.hero_t + dt
        };
        if aid.hero_t < HERO_SETTLE && aid.menu_t < HERO_WAIT {
            return;
        }
        info!("survivors armory: opening on {}", aid.tab.label());
        state.open(aid.tab);
        aid.open_t = Some(0.0);
        return;
    };
    let t = t + dt;
    aid.open_t = Some(t);
    if t < SHOT_AFTER {
        return;
    }
    aid.done = true;
    if let Some(dir) = &aid.shots {
        let path = dir.join(format!("armory-{}.png", aid.tab.key()));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tab_name_round_trips() {
        for tab in ArmoryTab::ALL {
            assert_eq!(ArmoryTab::from_name(tab.key()), Some(tab), "{tab:?} by key");
            assert_eq!(
                ArmoryTab::from_name(tab.label()),
                Some(tab),
                "{tab:?} by caption"
            );
            assert_eq!(
                ArmoryTab::from_name(&tab.key().to_uppercase()),
                Some(tab),
                "{tab:?} in capitals"
            );
            assert_eq!(ArmoryTab::ALL[tab as usize], tab, "ALL is in tab order");
        }
        for name in ["drop_tables", "DropTables", " drop tables "] {
            assert_eq!(ArmoryTab::from_name(name), Some(ArmoryTab::DropTables));
        }
        for name in ["", "gear", "characters"] {
            assert_eq!(ArmoryTab::from_name(name), None, "{name:?}");
        }
        let mut keys: Vec<_> = ArmoryTab::ALL.iter().map(|t| t.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), ArmoryTab::ALL.len(), "every key is its own");
    }

    #[test]
    fn the_armory_opens_switches_and_closes() {
        let mut s = ArmoryState::default();
        assert!(!s.open, "closed at boot, so the auto-runner never meets it");
        assert_eq!(s.tab, ArmoryTab::Character);

        assert!(!s.switch(ArmoryTab::Codex), "closed, a switch does nothing");
        assert_eq!(s.tab, ArmoryTab::Character);

        assert!(s.open(ArmoryTab::Talents));
        assert_eq!((s.open, s.tab), (true, ArmoryTab::Talents));
        assert!(
            !s.open(ArmoryTab::Talents),
            "opening it again changes nothing"
        );

        assert!(s.switch(ArmoryTab::Rewards));
        assert_eq!(s.tab, ArmoryTab::Rewards);
        assert!(
            !s.switch(ArmoryTab::Rewards),
            "the open tab again is no switch"
        );

        let before = s.rebuilds;
        s.refresh();
        assert_ne!(s.rebuilds, before, "a refresh asks for a rebuild");

        assert!(s.close());
        assert!(!s.open);
        assert_eq!(
            s.tab,
            ArmoryTab::Rewards,
            "the tab is kept for the next open"
        );
        assert!(!s.close(), "closing it again changes nothing");

        let tab = s.tab;
        assert!(s.open(tab));
        assert_eq!(s.tab, ArmoryTab::Rewards, "and it reopens there");
    }
}
