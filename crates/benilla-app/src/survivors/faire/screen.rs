//! The Faire's screen: a full-screen glue panel over the pick screen, opened by the menu's Darkmoon
//! Faire button (left of Enter World, mirroring Run Options on its right, with where the Faire is
//! under it) and closed by Done or Esc. While it is open the pick screen is hidden and its keys are
//! off, as under the Armory.
//!
//! Laid out on the survivors' 768-unit canvas in the Armory's kit ([`super::super::armory`]): the
//! title and the Faire's whereabouts top left, the purse top right, the tab bar under the title,
//! and three framed columns below. Four tabs:
//!
//! - **Prize Booth**: the two banners to pull on (the week's featured one and the standing booth),
//!   the picked banner's art and prizes, Pull ×1 and Pull ×10 (greyed with the reason when the
//!   purse is short), the banner's pity bars, and the latest prizes.
//! - **Prize Counter**: Silas Darkmoon's shelves, every prize for Favor, the week's featured ones
//!   20% off, and Buy.
//! - **Collection**: every prize by kind, owned ones in their icons and the rest a "?" in their
//!   rarity, with completion counts.
//! - **Odds & Pity**: the odds table, the rules, the counters and the banner schedule.
//!
//! [`sync_faire`] builds the chrome on open and rebuilds the content whenever [`FaireUi`] asks
//! ([`FaireUi::refresh`]); the pull's cards are their own overlay ([`super::reveal`]).

use bevy::ecs::world::CommandQueue;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::ui_render::ui_material::MaterialNode;

use super::super::armory::{panel, para, text, TabCtx};
use super::super::gear::ItemIcon;
use super::super::hud::{self, HudFont};
use super::super::menu::{self, MenuSlot};
use super::super::profile::Profile;
use super::super::run_options::GREEN;
use super::super::{Phase, Selection};
use super::booth::{self, Banner, BannerKind, Pull, Refusal};
use super::catalog::{catalog, completion, owned, Kind, Prize, Rarity};
use super::reveal::{self, RevealState};
use super::tickets;
use super::DevTickets;
use crate::glue::art::{GlueArt, DIM, GOLD};
use crate::glue::widgets::{
    glue_button, outlined_text, overlay, GlueBtnKind, GlueDisabled, GlueText, Hilight,
    LockHighlight,
};
use crate::glue::GlueClicks;
use crate::sound::GlueSound;

/// Over the pick screen (410), as the Armory (415), with which it never shares a frame.
const Z: i32 = 415;
const MARGIN: f32 = 24.0;
const TITLE_TOP: f32 = 12.0;
const SUBTITLE_TOP: f32 = 46.0;
const TAB_TOP: f32 = 68.0;
const TAB_W: f32 = 150.0;
const TAB_H: f32 = 32.0;
const BODY_TOP: f32 = TAB_TOP + TAB_H + 8.0;
const BODY_BOTTOM: f32 = 66.0;
const SIDE_W: f32 = 250.0;
const GAP: f32 = 12.0;
/// A grid cell, and the Counter's taller one with its price under the icon.
const CELL: f32 = 40.0;
const SHOP_W: f32 = 50.0;
const SHOP_H: f32 = 62.0;

/// The subtitle's parchment white.
pub(super) const PARCHMENT: Color = Color::srgb(0.9, 0.85, 0.7);
/// Faire Favor's colour, and the Faire's own.
pub(super) const FAVOR: Color = Color::srgb(0.78, 0.55, 1.0);
/// `RED_FONT_COLOR`: why a button is greyed.
const REFUSED: Color = Color::srgb(1.0, 0.25, 0.2);
const CELL_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const CELL_EDGE: Color = Color::srgb(0.3, 0.3, 0.3);

/// The purse's icons.
pub(super) const TICKET_ICON: &str = "INV_Misc_Ticket_Darkmoon_01";
pub(super) const FAVOR_ICON: &str = "Spell_Holy_SealOfSalvation";
/// The standing booth's banner.
const STANDARD_NAME: &str = "Darkmoon Prize Booth";
const STANDARD_BLURB: &str =
    "Silas Darkmoon's standing stock: every prize of a rarity at even odds, all year round.";
const STANDARD_TINT: u32 = 0xffc800;

/// The SoundEntries kits the screen plays, as the 1.12 client names them.
pub(super) const SOUND_OPEN: &str = "AuctionWindowOpen";
pub(super) const SOUND_CLOSE: &str = "AuctionWindowClose";
pub(super) const SOUND_TAB: &str = "igCharacterInfoTab";
pub(super) const SOUND_PICK: &str = "igMainMenuOptionCheckBoxOn";
pub(super) const SOUND_PULL: &str = "UnwrapGift";
pub(super) const SOUND_BUY: &str = "LOOTWINDOWCOINSOUND";

/// One of the Faire's tabs, in tab-bar order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub(in crate::survivors) enum FaireTab {
    #[default]
    Booth,
    Counter,
    Collection,
    Odds,
}

impl FaireTab {
    pub(super) const ALL: [Self; 4] = [Self::Booth, Self::Counter, Self::Collection, Self::Odds];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Booth => "Prize Booth",
            Self::Counter => "Prize Counter",
            Self::Collection => "Collection",
            Self::Odds => "Odds & Pity",
        }
    }
}

/// Whether the Faire is up, on which tab, and what its tabs have picked.
#[derive(Resource, Default)]
pub(in crate::survivors) struct FaireUi {
    pub open: bool,
    pub tab: FaireTab,
    /// The banner the booth pulls on.
    pub banner: BannerKind,
    /// The Counter's and the Collection's shelf: one kind, or `None` for all.
    pub shelf: Option<Kind>,
    /// The prize under Details, an index into [`catalog`].
    pub pick: Option<usize>,
    /// The session's latest pulls, newest first.
    pub recent: Vec<Pull>,
    /// The last action's outcome, under the booth's buttons or the Counter's Buy.
    pub message: Option<(String, Color)>,
    /// Bumped by [`Self::refresh`]; the content is rebuilt whenever it moves.
    pub rebuilds: u32,
}

/// How many pulls the booth's side column keeps.
const RECENT: usize = 12;

impl FaireUi {
    pub(super) fn refresh(&mut self) {
        self.rebuilds = self.rebuilds.wrapping_add(1);
    }
}

/// Run condition: the Faire is shut, so the pick screen's own keys and clicks may act.
pub(in crate::survivors) fn closed(ui: Res<FaireUi>) -> bool {
    !ui.open
}

/// The Faire's buttons.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum FaireAction {
    Open,
    Close,
    Tab(FaireTab),
    Banner(BannerKind),
    /// Pull this many: 1 or 10.
    Pull(u32),
    Shelf(Option<Kind>),
    Pick(usize),
    Buy(usize),
}

#[derive(Component)]
struct FaireRoot;
#[derive(Component)]
struct TabButton(FaireTab);
/// On [`MenuRoot`] once the menu's button is in.
#[derive(Component)]
struct EntryBuilt;
/// The menu's lines under the button: where the Faire is, and the purse.
#[derive(Component)]
struct EntryWhere;
#[derive(Component)]
struct EntryPurse;
/// A grid that scrolls under the wheel.
#[derive(Component)]
struct FaireScroll;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<FaireUi>()
        .add_message::<GlueSound>()
        .add_systems(
            Update,
            (build_entry, refresh_entry)
                .chain()
                .before(crate::glue::GlueVisuals)
                .before(hud::keys),
        )
        .add_systems(
            Update,
            (faire_input, sync_faire, paint_tabs, scroll_shelves)
                .chain()
                // After the menu's own (gated) input, before the pick screen is shown or hidden,
                // so open and close land in one frame, as the Armory's do.
                .after(hud::keys)
                .after(menu::menu_clicks)
                .after(reveal::reveal_input)
                .before(hud::show_panels),
        );
}

// ── The menu's button ───────────────────────────────────────────────────────────────────────────

/// The Darkmoon Faire button and its whereabouts and purse, as one block in the menu action row.
#[allow(clippy::type_complexity)]
fn build_entry(
    mut commands: Commands,
    buttons: Query<(Entity, &MenuSlot), Without<EntryBuilt>>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
) {
    let Some(buttons) = buttons
        .iter()
        .find_map(|(entity, slot)| (*slot == MenuSlot::Faire).then_some(entity))
    else {
        return;
    };
    let f = &font.0;
    let px = Val::Px;
    commands
        .entity(buttons)
        .insert(EntryBuilt)
        .with_children(|ui| {
            ui.spawn(Node {
                width: px(150.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(1.0),
                ..default()
            })
            .with_children(|c| {
                glue_button(
                    c,
                    &art,
                    f,
                    FaireAction::Open,
                    "Darkmoon Faire",
                    150.0,
                    38.0,
                    GlueBtnKind::Small,
                    1.0,
                );
                for (marker, color) in [(true, FAVOR), (false, GOLD)] {
                    let line = outlined_text(
                        c,
                        Node::default(),
                        (),
                        (),
                        GlueText {
                            text: "",
                            size: 12.0,
                            color,
                            wrap: false,
                        },
                        f,
                        1.0,
                    );
                    if marker {
                        c.commands().entity(line).insert(EntryWhere);
                    } else {
                        c.commands().entity(line).insert(EntryPurse);
                    }
                }
            });
        });
}

/// The menu's lines: "The Faire is in Mulgore" and the tickets to spend.
#[allow(clippy::type_complexity)]
fn refresh_entry(
    profile: Option<Res<Profile>>,
    dev: Res<DevTickets>,
    mut lines: ParamSet<(
        Query<&mut Text, With<EntryWhere>>,
        Query<&mut Text, With<EntryPurse>>,
    )>,
) {
    let Some(profile) = profile else { return };
    let runs = profile.account.runs;
    let want = format!("The Faire is in {}", tickets::faire_zone(runs));
    for mut t in &mut lines.p0() {
        if t.0 != want {
            t.0 = want.clone();
        }
    }
    let n = profile.faire.tickets.saturating_add(dev.0);
    let want = format!("{n} Prize Ticket{}", if n == 1 { "" } else { "s" });
    for mut t in &mut lines.p1() {
        if t.0 != want {
            t.0 = want.clone();
        }
    }
}

// ── Input ───────────────────────────────────────────────────────────────────────────────────────

/// The Faire's clicks, and Esc to close it; nothing while a pull's cards are up, which take the
/// clicks and keys themselves.
#[allow(clippy::too_many_arguments)]
pub(super) fn faire_input(
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    actions: Query<(Entity, &FaireAction, Option<&GlueDisabled>)>,
    mut ui: ResMut<FaireUi>,
    mut profile: ResMut<Profile>,
    mut dev: ResMut<DevTickets>,
    mut reveal: ResMut<RevealState>,
    mut sounds: MessageWriter<GlueSound>,
    mut pulled: MessageWriter<super::FairePulled>,
) {
    // Leaving the menu (the auto-runner starting a run) closes it; the reveal closes itself.
    if *phase.get() != Phase::Menu {
        if ui.open {
            ui.open = false;
        }
        return;
    }
    if reveal.0.is_some() {
        return;
    }
    if ui.open && keys.just_pressed(KeyCode::Escape) {
        ui.open = false;
        sounds.write(GlueSound(SOUND_CLOSE));
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _, off)| !off.is_some_and(|d| d.0) && clicks.hit(*e))
        .map(|(_, a, _)| *a)
    else {
        return;
    };
    let runs = profile.account.runs;
    let banner = booth::banner_at(runs);
    match action {
        FaireAction::Open => {
            ui.open = true;
            ui.pick = None;
            ui.message = None;
            ui.refresh();
            sounds.write(GlueSound(SOUND_OPEN));
        }
        FaireAction::Close => {
            ui.open = false;
            sounds.write(GlueSound(SOUND_CLOSE));
        }
        FaireAction::Tab(tab) => {
            if ui.tab != tab {
                ui.tab = tab;
                ui.pick = None;
                ui.message = None;
                ui.refresh();
                sounds.write(GlueSound(SOUND_TAB));
            }
        }
        FaireAction::Banner(kind) => {
            if ui.banner != kind {
                ui.banner = kind;
                ui.message = None;
                ui.refresh();
                sounds.write(GlueSound(SOUND_PICK));
            }
        }
        FaireAction::Shelf(shelf) => {
            if ui.shelf != shelf {
                ui.shelf = shelf;
                ui.refresh();
                sounds.write(GlueSound(SOUND_PICK));
            }
        }
        FaireAction::Pick(i) => {
            if ui.pick != Some(i) {
                ui.pick = Some(i);
                ui.message = None;
                ui.refresh();
                sounds.write(GlueSound(SOUND_PICK));
            }
        }
        FaireAction::Pull(count) => {
            let kind = ui.banner;
            match booth::pull(&mut profile, &mut dev.0, kind, banner, count) {
                Ok(pulls) => {
                    profile.mark_dirty();
                    ui.message = Some((pull_summary(&pulls), PARCHMENT));
                    for p in &pulls {
                        ui.recent.insert(0, *p);
                    }
                    ui.recent.truncate(RECENT);
                    let title = match kind {
                        BannerKind::Featured => banner.name,
                        BannerKind::Standard => STANDARD_NAME,
                    };
                    // A co-op lobby hears of it ([`super::FairePulled`]).
                    pulled.write(super::FairePulled(
                        pulls
                            .iter()
                            .map(|p| (p.prize().name.clone(), p.prize().rarity))
                            .collect(),
                    ));
                    reveal.start(pulls, title);
                    sounds.write(GlueSound(SOUND_PULL));
                }
                Err(refusal) => ui.message = Some((refusal_line(refusal, "ticket"), REFUSED)),
            }
            ui.refresh();
        }
        FaireAction::Buy(i) => {
            let Some(prize) = catalog().get(i) else {
                return;
            };
            match booth::buy(&mut profile, &prize.id, banner) {
                Ok(cost) => {
                    profile.mark_dirty();
                    ui.message = Some((format!("Bought {} for {cost} Favor!", prize.name), GREEN));
                    sounds.write(GlueSound(SOUND_BUY));
                }
                Err(refusal) => ui.message = Some((refusal_line(refusal, "Favor"), REFUSED)),
            }
            ui.refresh();
        }
    }
}

/// "3 new prizes, +11 Favor".
fn pull_summary(pulls: &[Pull]) -> String {
    let new = pulls.iter().filter(|p| p.new).count();
    let favor: u32 = pulls.iter().map(|p| p.favor).sum();
    let s = if new == 1 { "" } else { "s" };
    match (new, favor) {
        (_, 0) => format!("Last pull: {new} new prize{s}!"),
        (0, f) => format!("Last pull: all duplicates, +{f} Favor"),
        (n, f) => format!("Last pull: {n} new prize{s}, +{f} Favor"),
    }
}

/// Why the booth or the Counter refused, in `unit`s ("ticket" or "Favor").
fn refusal_line(refusal: Refusal, unit: &str) -> String {
    match refusal {
        Refusal::Short(n) if unit == "ticket" => {
            format!("You need {n} more ticket{}.", if n == 1 { "" } else { "s" })
        }
        Refusal::Short(n) => format!("You need {n} more {unit}."),
        Refusal::Owned => "You already have that prize.".into(),
        Refusal::Unknown => "Silas has never heard of that prize.".into(),
    }
}

/// The open tab's button stays lit.
fn paint_tabs(ui: Res<FaireUi>, mut tabs: Query<(&TabButton, &mut LockHighlight)>) {
    for (tab, mut lock) in &mut tabs {
        let want = tab.0 == ui.tab;
        if lock.0 != want {
            lock.0 = want;
        }
    }
}

/// Wheel-scroll the hovered grid.
fn scroll_shelves(
    mut wheel: MessageReader<MouseWheel>,
    mut grids: Query<(&Interaction, &ComputedNode, &mut ScrollPosition), With<FaireScroll>>,
) {
    for ev in wheel.read() {
        let dy = match ev.unit {
            MouseScrollUnit::Line => ev.y * 36.0,
            MouseScrollUnit::Pixel => ev.y,
        };
        for (interaction, node, mut pos) in &mut grids {
            if *interaction != Interaction::None {
                let max =
                    ((node.content_size.y - node.size.y) * node.inverse_scale_factor).max(0.0);
                pos.0.y = (pos.0.y - dy).clamp(0.0, max);
            }
        }
    }
}

// ── Building ────────────────────────────────────────────────────────────────────────────────────

/// What [`sync_faire`] last built.
struct Built {
    root: Entity,
    content: Entity,
    rebuilds: u32,
}

/// Bring the screen in line with [`FaireUi`]: build it on open, despawn it on close, and rebuild
/// the content on a refresh.
fn sync_faire(world: &mut World, mut built: Local<Option<Built>>) {
    if built
        .as_ref()
        .is_some_and(|b| world.get_entity(b.root).is_err())
    {
        *built = None;
    }
    let (open, rebuilds) = {
        let ui = world.resource::<FaireUi>();
        (ui.open, ui.rebuilds)
    };
    if !open {
        if let Some(b) = built.take() {
            world.entity_mut(b.root).despawn();
        }
        return;
    }
    if built.as_ref().is_some_and(|b| b.rebuilds == rebuilds) {
        return;
    }
    let mut queue = CommandQueue::default();
    let (root, content) = {
        let Some(ctx) = Ctx::new(world) else { return };
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
            .with_children(|c| build_content(c, &ctx));
        (root, content)
    };
    queue.apply(world);
    *built = Some(Built {
        root,
        content,
        rebuilds,
    });
}

/// What the builders read.
struct Ctx<'w> {
    tab: TabCtx<'w>,
    profile: &'w Profile,
    ui: &'w FaireUi,
    /// The test aid's tickets.
    dev: u32,
    /// Runs recorded: the Faire's whereabouts and the banner follow it.
    runs: u32,
    banner: &'static Banner,
}

impl<'w> Ctx<'w> {
    fn new(world: &'w World) -> Option<Self> {
        let profile = world.get_resource::<Profile>()?;
        let runs = profile.account.runs;
        Some(Self {
            tab: TabCtx {
                art: world.get_resource::<GlueArt>()?,
                font: &world.get_resource::<HudFont>()?.0,
                selection: world.get_resource::<Selection>()?,
                world,
            },
            profile,
            ui: world.get_resource::<FaireUi>()?,
            dev: world.get_resource::<DevTickets>().map_or(0, |d| d.0),
            runs,
            banner: booth::banner_at(runs),
        })
    }

    fn tickets(&self) -> u32 {
        self.profile.faire.tickets.saturating_add(self.dev)
    }

    fn favor(&self) -> u32 {
        self.profile.faire.favor
    }
}

/// The Faire's frame: the shade, the title and whereabouts, the tab bar, the empty content area
/// and Done. Returns the root and the content area.
fn spawn_chrome(commands: &mut Commands, ctx: &Ctx) -> (Entity, Entity) {
    let px = Val::Px;
    let f = ctx.tab.font;
    let zone = tickets::faire_zone(ctx.runs);
    let left = tickets::runs_left_here(ctx.runs);
    let next = tickets::faire_zone(ctx.runs + left);
    let subtitle = format!(
        "The Faire is in {zone}: runs there pay double tickets. It moves to {next} in {left} run{}.",
        if left == 1 { "" } else { "s" }
    );
    let mut content = Entity::PLACEHOLDER;
    let mut root = commands.spawn((
        FaireRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        // A dusk over the Faire grounds: deep purple, darker at the edges.
        BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.05, 0.0, 0.09, 0.94), 0.0),
            ColorStop::percent(Color::srgba(0.09, 0.03, 0.15, 0.86), 30.0),
            ColorStop::percent(Color::srgba(0.06, 0.02, 0.11, 0.86), 75.0),
            ColorStop::percent(Color::srgba(0.02, 0.0, 0.04, 0.94), 100.0),
        ])),
        GlobalZIndex(Z),
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
                text: "Darkmoon Faire",
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
                color: PARCHMENT,
                wrap: false,
            },
            f,
            1.0,
        );
        r.spawn(Node {
            position_type: PositionType::Absolute,
            left: px(MARGIN),
            top: px(TAB_TOP),
            flex_direction: FlexDirection::Row,
            column_gap: px(4.0),
            ..default()
        })
        .with_children(|strip| {
            for tab in FaireTab::ALL {
                let b = glue_button(
                    strip,
                    ctx.tab.art,
                    f,
                    FaireAction::Tab(tab),
                    tab.label(),
                    TAB_W,
                    TAB_H,
                    GlueBtnKind::Small,
                    1.0,
                );
                strip
                    .commands()
                    .entity(b)
                    .insert((TabButton(tab), LockHighlight(tab == ctx.ui.tab)));
            }
        });
        content = r
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                FocusPolicy::Pass,
            ))
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
                ctx.tab.art,
                f,
                FaireAction::Close,
                "Done",
                170.0,
                45.0,
                GlueBtnKind::Normal,
                1.0,
            );
        });
    });
    (root.id(), content)
}

/// The purse top right, and the open tab's three columns.
fn build_content(c: &mut ChildSpawnerCommands, ctx: &Ctx) {
    let px = Val::Px;
    c.spawn(Node {
        position_type: PositionType::Absolute,
        right: px(MARGIN),
        top: px(TITLE_TOP + 2.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexEnd,
        row_gap: px(4.0),
        ..default()
    })
    .with_children(|purse| {
        let tickets = ctx.tickets();
        let s = if tickets == 1 { "" } else { "s" };
        let dev = if ctx.dev > 0 {
            format!(" ({} not saved)", ctx.dev)
        } else {
            String::new()
        };
        for (icon, line, color) in [
            (
                TICKET_ICON,
                format!("{tickets} Darkmoon Prize Ticket{s}{dev}"),
                GOLD,
            ),
            (FAVOR_ICON, format!("{} Faire Favor", ctx.favor()), FAVOR),
        ] {
            purse
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    text(row, &ctx.tab, &line, 15.0, color);
                    icon_node(row, icon, 22.0, Color::WHITE);
                });
        }
    });
    c.spawn(Node {
        position_type: PositionType::Absolute,
        left: px(MARGIN),
        right: px(MARGIN),
        top: px(BODY_TOP),
        bottom: px(BODY_BOTTOM),
        flex_direction: FlexDirection::Row,
        column_gap: px(GAP),
        ..default()
    })
    .with_children(|body| match ctx.ui.tab {
        FaireTab::Booth => booth_tab(body, ctx),
        FaireTab::Counter => counter_tab(body, ctx),
        FaireTab::Collection => collection_tab(body, ctx),
        FaireTab::Odds => odds_tab(body, ctx),
    });
}

// ── The kit ─────────────────────────────────────────────────────────────────────────────────────

fn side() -> Node {
    Node {
        width: Val::Px(SIDE_W),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    }
}

fn center() -> Node {
    Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        ..default()
    }
}

fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

/// `0xRRGGBB` as a colour.
pub(super) fn tint(hex: u32) -> Color {
    hud::rgb(hex)
}

/// A `size`-square `Interface\Icons` icon, drawn in `color` once it loads.
pub(super) fn icon_node(p: &mut ChildSpawnerCommands, icon: &'static str, size: f32, color: Color) {
    p.spawn((
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            ..default()
        },
        ImageNode {
            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
            ..default()
        },
        ItemIcon { icon, tint: color },
    ));
}

/// A prize's icon in a socket edged in its rarity's colour.
fn socket(p: &mut ChildSpawnerCommands, prize: &Prize, size: f32, lit: bool) {
    let edge = prize.rarity.color();
    p.spawn((
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(edge),
        BackgroundColor(CELL_FILL),
    ))
    .with_children(|s| {
        let tint = if lit {
            Color::WHITE
        } else {
            Color::srgb(0.35, 0.35, 0.35)
        };
        icon_node(s, prize.icon, size - 2.0, tint);
    });
}

/// A progress bar `w` wide, `frac` full.
fn bar(p: &mut ChildSpawnerCommands, frac: f32, color: Color) {
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(8.0),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.35, 0.35, 0.35)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
    ))
    .with_children(|b| {
        b.spawn((
            Node {
                width: Val::Percent(100.0 * frac.clamp(0.0, 1.0)),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(color),
        ));
    });
}

/// A label left and a value right, on one line.
fn line_pair(p: &mut ChildSpawnerCommands, ctx: &Ctx, left: &str, right: &str, color: Color) {
    p.spawn(Node {
        width: Val::Percent(100.0),
        justify_content: JustifyContent::SpaceBetween,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|r| {
        text(r, &ctx.tab, left, 12.0, color);
        text(r, &ctx.tab, right, 12.0, Color::WHITE);
    });
}

/// A row's hover sheen: the quest log's, else the zone list's.
fn sheen(r: &mut ChildSpawnerCommands, art: &GlueArt) {
    let hi = art.quest_hilight.as_ref().or(art.select_highlight.as_ref());
    if let Some(hi) = hi {
        r.spawn((
            Hilight,
            Visibility::Hidden,
            MaterialNode(hi.clone()),
            overlay(),
        ));
    }
}

/// An icon cell's hover ring.
fn ring(r: &mut ChildSpawnerCommands, art: &GlueArt) {
    if let Some(hi) = &art.mouse_hilight {
        r.spawn((
            Hilight,
            Visibility::Hidden,
            MaterialNode(hi.clone()),
            overlay(),
        ));
    }
}

/// A clickable list row, `selected` drawn lit.
fn list_row(
    p: &mut ChildSpawnerCommands,
    ctx: &Ctx,
    action: FaireAction,
    selected: bool,
    left: &str,
    right: &str,
) {
    p.spawn((
        Button,
        action,
        LockHighlight(selected),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(22.0),
            flex_shrink: 0.0,
            padding: UiRect::horizontal(Val::Px(6.0)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
    ))
    .with_children(|r| {
        sheen(r, ctx.tab.art);
        let color = if selected { Color::WHITE } else { GOLD };
        text(r, &ctx.tab, left, 13.0, color);
        text(r, &ctx.tab, right, 12.0, PARCHMENT);
    });
}

/// The prizes on `shelf`, by kind and then rarest first.
fn shelf_prizes(shelf: Option<Kind>) -> Vec<usize> {
    let mut list: Vec<usize> = (0..catalog().len())
        .filter(|&i| shelf.is_none_or(|k| catalog()[i].kind == k))
        .collect();
    list.sort_by_key(|&i| {
        let p = &catalog()[i];
        (
            Kind::ALL.iter().position(|&k| k == p.kind),
            std::cmp::Reverse(p.rarity),
        )
    });
    list
}

/// A grid that scrolls under the wheel, filling the rest of its column.
fn scroll_grid(p: &mut ChildSpawnerCommands, body: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn((
        FaireScroll,
        Interaction::default(),
        ScrollPosition::default(),
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    ))
    .with_children(body);
}

/// A wrapping row of cells.
fn cells(p: &mut ChildSpawnerCommands, body: impl FnOnce(&mut ChildSpawnerCommands)) {
    p.spawn(Node {
        width: Val::Percent(100.0),
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(5.0),
        row_gap: Val::Px(5.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(body);
}

/// The banner shown for `kind`: its name, blurb, emblem and colour.
fn banner_face(ctx: &Ctx, kind: BannerKind) -> (&'static str, &'static str, &'static str, Color) {
    match kind {
        BannerKind::Featured => (
            ctx.banner.name,
            ctx.banner.blurb,
            ctx.banner.emblem,
            tint(ctx.banner.tint),
        ),
        BannerKind::Standard => (
            STANDARD_NAME,
            STANDARD_BLURB,
            TICKET_ICON,
            tint(STANDARD_TINT),
        ),
    }
}

// ── Prize Booth ─────────────────────────────────────────────────────────────────────────────────

fn booth_tab(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    panel(p, &ctx.tab, side(), Some("Banners"), |b| {
        for kind in BannerKind::ALL {
            banner_card(b, ctx, kind);
            gap(b, 8.0);
        }
        text(b, &ctx.tab, "Coming up", 13.0, GOLD);
        gap(b, 2.0);
        for (n, banner) in booth::upcoming(ctx.runs, 2) {
            let s = if n == 1 { "" } else { "s" };
            para(
                b,
                &ctx.tab,
                &format!("{} in {n} run{s}", banner.name),
                12.0,
                PARCHMENT,
            );
        }
    });
    panel(p, &ctx.tab, center(), None, |b| booth_center(b, ctx));
    panel(p, &ctx.tab, side(), Some("Latest Prizes"), |b| {
        latest(b, ctx)
    });
}

/// A banner to pull on: its emblem, name and pity, lit while picked.
fn banner_card(p: &mut ChildSpawnerCommands, ctx: &Ctx, kind: BannerKind) {
    let (name, _, emblem, color) = banner_face(ctx, kind);
    let picked = ctx.ui.banner == kind;
    let pity = ctx.profile.faire.pity(kind.key());
    let sub = match kind {
        BannerKind::Featured => {
            let left = tickets::runs_left_here(ctx.runs);
            format!(
                "Featured, {left} run{} left",
                if left == 1 { "" } else { "s" }
            )
        }
        BannerKind::Standard => "Standing stock".to_string(),
    };
    p.spawn((
        Button,
        FaireAction::Banner(kind),
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(7.0)),
            row_gap: Val::Px(4.0),
            border: UiRect::all(Val::Px(if picked { 2.0 } else { 1.0 })),
            ..default()
        },
        BorderColor::all(if picked { GOLD } else { CELL_EDGE }),
        BackgroundGradient::from(LinearGradient::to_right(vec![
            ColorStop::percent(color.with_alpha(if picked { 0.35 } else { 0.15 }), 0.0),
            ColorStop::percent(Color::srgba(0.0, 0.0, 0.0, 0.4), 100.0),
        ])),
    ))
    .with_children(|c| {
        sheen(c, ctx.tab.art);
        c.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            icon_node(row, emblem, 32.0, Color::WHITE);
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                ..default()
            })
            .with_children(|col| {
                para(col, &ctx.tab, name, 14.0, color.mix(&Color::WHITE, 0.3));
                text(col, &ctx.tab, &sub, 11.0, PARCHMENT);
            });
        });
        text(
            c,
            &ctx.tab,
            &format!(
                "Epic+ within {} · Legendary within {}",
                booth::EPIC_PITY - pity.since_epic.min(booth::EPIC_PITY - 1),
                booth::LEGENDARY_PITY - pity.since_legendary.min(booth::LEGENDARY_PITY - 1)
            ),
            11.0,
            DIM,
        );
    });
}

/// The picked banner's art, the purse, Pull ×1 and ×10, and the pity bars.
fn booth_center(b: &mut ChildSpawnerCommands, ctx: &Ctx) {
    let px = Val::Px;
    let kind = ctx.ui.banner;
    let (name, blurb, emblem, color) = banner_face(ctx, kind);
    // The banner: its colour washed down the box, the emblem, the name and its prizes.
    b.spawn((
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(10.0)),
            row_gap: px(6.0),
            border: UiRect::all(px(2.0)),
            border_radius: BorderRadius::all(px(6.0)),
            ..default()
        },
        BorderColor::all(color.mix(&GOLD, 0.35)),
        BackgroundGradient::from(LinearGradient::to_bottom(vec![
            ColorStop::percent(color.with_alpha(0.45), 0.0),
            ColorStop::percent(color.mix(&Color::BLACK, 0.8).with_alpha(0.6), 100.0),
        ])),
        BoxShadow::new(color.with_alpha(0.35), px(0.0), px(0.0), px(1.0), px(14.0)),
    ))
    .with_children(|art| {
        art.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: px(10.0),
            ..default()
        })
        .with_children(|row| {
            icon_node(row, emblem, 52.0, Color::WHITE);
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(2.0),
                ..default()
            })
            .with_children(|col| {
                text(col, &ctx.tab, name, 22.0, color.mix(&Color::WHITE, 0.35));
                let line = match kind {
                    BannerKind::Featured => "Its prizes share half of their rarity's odds",
                    BannerKind::Standard => "No favourites: every prize at even odds",
                };
                text(col, &ctx.tab, line, 12.0, PARCHMENT);
            });
        });
        para(art, &ctx.tab, blurb, 12.0, Color::WHITE);
        let (label, shown): (&str, Vec<usize>) = match kind {
            BannerKind::Featured => (
                "Featured prizes",
                ctx.banner
                    .featured
                    .iter()
                    .filter_map(|id| super::catalog::index_of(id))
                    .collect(),
            ),
            BannerKind::Standard => (
                "Legendary prizes",
                (0..catalog().len())
                    .filter(|&i| catalog()[i].rarity == Rarity::Legendary)
                    .collect(),
            ),
        };
        text(art, &ctx.tab, label, 12.0, GOLD);
        cells(art, |row| {
            for i in shown {
                prize_cell(row, ctx, i, CellMode::Banner);
            }
        });
    });
    gap(b, 10.0);
    // The purse and the two pulls.
    let tickets = ctx.tickets();
    b.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: px(8.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        icon_node(row, TICKET_ICON, 26.0, Color::WHITE);
        let s = if tickets == 1 { "" } else { "s" };
        text(
            row,
            &ctx.tab,
            &format!("You have {tickets} Darkmoon Prize Ticket{s}"),
            15.0,
            Color::WHITE,
        );
    });
    gap(b, 6.0);
    b.spawn(Node {
        column_gap: px(13.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        for (count, caption, hint) in [
            (1, "Pull ×1 · 10 tickets", "One prize"),
            (10, "Pull ×10 · 90 tickets", "A Rare or better guaranteed"),
        ] {
            let cost = booth::cost(count);
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2.0),
                ..default()
            })
            .with_children(|col| {
                let short = cost.saturating_sub(tickets);
                let btn = glue_button(
                    col,
                    ctx.tab.art,
                    ctx.tab.font,
                    FaireAction::Pull(count),
                    caption,
                    200.0,
                    40.0,
                    GlueBtnKind::Dialog,
                    1.0,
                );
                col.commands().entity(btn).insert(GlueDisabled(short > 0));
                if short > 0 {
                    let s = if short == 1 { "" } else { "s" };
                    text(
                        col,
                        &ctx.tab,
                        &format!("Need {short} more ticket{s}"),
                        11.0,
                        REFUSED,
                    );
                } else {
                    text(col, &ctx.tab, hint, 11.0, GREEN);
                }
            });
        }
    });
    gap(b, 10.0);
    // This banner's pity.
    let pity = ctx.profile.faire.pity(kind.key());
    text(b, &ctx.tab, "Pity on this banner", 14.0, GOLD);
    gap(b, 4.0);
    pity_bars(b, ctx, pity);
    if let Some((line, color)) = &ctx.ui.message {
        gap(b, 8.0);
        para(b, &ctx.tab, line, 13.0, *color);
    }
}

/// The two pity counters as bars, with how many pulls until each is sure.
fn pity_bars(b: &mut ChildSpawnerCommands, ctx: &Ctx, pity: super::save::Pity) {
    for (label, since, of, color) in [
        (
            "Epic or better",
            pity.since_epic,
            booth::EPIC_PITY,
            Rarity::Epic.color(),
        ),
        (
            "Legendary",
            pity.since_legendary,
            booth::LEGENDARY_PITY,
            Rarity::Legendary.color(),
        ),
    ] {
        let since = since.min(of - 1);
        line_pair(
            b,
            ctx,
            &format!("{label}: sure within {} pulls", of - since),
            &format!("{since} / {of}"),
            color,
        );
        gap(b, 2.0);
        bar(b, since as f32 / of as f32, color);
        gap(b, 6.0);
    }
}

/// The session's latest prizes, newest first, and the Favor they paid.
fn latest(b: &mut ChildSpawnerCommands, ctx: &Ctx) {
    b.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        icon_node(row, FAVOR_ICON, 22.0, Color::WHITE);
        text(
            row,
            &ctx.tab,
            &format!("{} Faire Favor", ctx.favor()),
            15.0,
            FAVOR,
        );
    });
    para(
        b,
        &ctx.tab,
        "A prize you have already turns into Favor; Silas Darkmoon takes it at the Prize Counter.",
        11.0,
        DIM,
    );
    gap(b, 8.0);
    if ctx.ui.recent.is_empty() {
        para(
            b,
            &ctx.tab,
            "Pull at the booth: your prizes land here.",
            12.0,
            DIM,
        );
        return;
    }
    for pull in &ctx.ui.recent {
        let prize = pull.prize();
        b.spawn(Node {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            flex_shrink: 0.0,
            margin: UiRect::bottom(Val::Px(3.0)),
            ..default()
        })
        .with_children(|row| {
            socket(row, prize, 22.0, true);
            row.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|n| {
                text(n, &ctx.tab, &prize.name, 12.0, prize.rarity.color());
            });
            if pull.new {
                text(row, &ctx.tab, "NEW!", 11.0, GREEN);
            } else {
                text(row, &ctx.tab, &format!("+{}", pull.favor), 11.0, FAVOR);
            }
        });
    }
}

// ── Prize Counter ───────────────────────────────────────────────────────────────────────────────

fn counter_tab(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    panel(p, &ctx.tab, side(), Some("Silas Darkmoon"), |b| {
        para(
            b,
            &ctx.tab,
            "\"Any prize at my counter, friend, for Faire Favor. This week's featured prizes come \
             20% cheaper.\"",
            12.0,
            PARCHMENT,
        );
        gap(b, 6.0);
        b.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            icon_node(row, FAVOR_ICON, 26.0, Color::WHITE);
            text(
                row,
                &ctx.tab,
                &format!("{} Faire Favor", ctx.favor()),
                17.0,
                FAVOR,
            );
        });
        gap(b, 8.0);
        shelves(b, ctx);
    });
    panel(
        p,
        &ctx.tab,
        center(),
        Some(shelf_title(ctx.ui.shelf)),
        |b| {
            scroll_grid(b, |g| {
                cells(g, |row| {
                    for i in shelf_prizes(ctx.ui.shelf) {
                        prize_cell(row, ctx, i, CellMode::Counter);
                    }
                });
            });
        },
    );
    panel(p, &ctx.tab, side(), Some("Details"), |b| {
        details(b, ctx, true)
    });
}

fn shelf_title(shelf: Option<Kind>) -> &'static str {
    shelf.map_or("Every Prize", Kind::plural)
}

/// The shelf list: every prize, then each kind, with what the account has of it.
fn shelves(b: &mut ChildSpawnerCommands, ctx: &Ctx) {
    let (have, of) = completion(ctx.profile, |_| true);
    list_row(
        b,
        ctx,
        FaireAction::Shelf(None),
        ctx.ui.shelf.is_none(),
        "Every Prize",
        &format!("{have} / {of}"),
    );
    for kind in Kind::ALL {
        let (have, of) = completion(ctx.profile, |p| p.kind == kind);
        list_row(
            b,
            ctx,
            FaireAction::Shelf(Some(kind)),
            ctx.ui.shelf == Some(kind),
            kind.plural(),
            &format!("{have} / {of}"),
        );
    }
}

// ── Collection ──────────────────────────────────────────────────────────────────────────────────

fn collection_tab(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    panel(p, &ctx.tab, side(), Some("Collection"), |b| {
        let (have, of) = completion(ctx.profile, |_| true);
        b.spawn(Node {
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|c| {
            text(c, &ctx.tab, &format!("{have} / {of}"), 22.0, Color::WHITE);
        });
        gap(b, 4.0);
        bar(b, have as f32 / of.max(1) as f32, GOLD);
        gap(b, 10.0);
        shelves(b, ctx);
        gap(b, 10.0);
        text(b, &ctx.tab, "By rarity", 13.0, GOLD);
        gap(b, 2.0);
        for r in Rarity::ALL {
            let (have, of) = completion(ctx.profile, |p| p.rarity == r);
            line_pair(b, ctx, r.label(), &format!("{have} / {of}"), r.color());
        }
    });
    panel(
        p,
        &ctx.tab,
        center(),
        Some(shelf_title(ctx.ui.shelf)),
        |b| {
            scroll_grid(b, |g| match ctx.ui.shelf {
                Some(kind) => cells(g, |row| {
                    for i in shelf_prizes(Some(kind)) {
                        prize_cell(row, ctx, i, CellMode::Collection);
                    }
                }),
                None => {
                    for kind in Kind::ALL {
                        let (have, of) = completion(ctx.profile, |p| p.kind == kind);
                        text(
                            g,
                            &ctx.tab,
                            &format!("{}  {have} / {of}", kind.plural()),
                            14.0,
                            GOLD,
                        );
                        cells(g, |row| {
                            for i in shelf_prizes(Some(kind)) {
                                prize_cell(row, ctx, i, CellMode::Collection);
                            }
                        });
                        gap(g, 4.0);
                    }
                }
            });
        },
    );
    panel(p, &ctx.tab, side(), Some("Details"), |b| {
        details(b, ctx, false)
    });
}

/// Where a prize cell stands, which sets what it shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CellMode {
    /// On a banner: always its icon, unowned ones dimmed.
    Banner,
    /// At the Counter: its icon and its price, or "Owned".
    Counter,
    /// In the Collection: its icon once owned, else a "?" in its rarity.
    Collection,
}

/// One prize in a grid; a click shows it under Details.
fn prize_cell(row: &mut ChildSpawnerCommands, ctx: &Ctx, index: usize, mode: CellMode) {
    let prize = &catalog()[index];
    let have = owned(ctx.profile, prize);
    let picked = ctx.ui.pick == Some(index) && mode != CellMode::Banner;
    let rc = prize.rarity.color();
    let edge = if picked {
        GOLD
    } else if have || mode != CellMode::Collection {
        rc
    } else {
        rc.mix(&Color::BLACK, 0.55)
    };
    let (w, h) = match mode {
        CellMode::Counter => (SHOP_W, SHOP_H),
        CellMode::Banner => (34.0, 34.0),
        CellMode::Collection => (CELL, CELL),
    };
    let size = w.min(CELL);
    let mut cell = row.spawn((
        Node {
            width: Val::Px(w),
            height: Val::Px(h),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            ..default()
        },
        FocusPolicy::Pass,
    ));
    if mode != CellMode::Banner {
        cell.insert((Button, FaireAction::Pick(index)));
    }
    cell.with_children(|c| {
        c.spawn((
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(if picked { 2.0 } else { 1.0 })),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderColor::all(edge),
            BackgroundColor(CELL_FILL),
            BoxShadow::new(
                rc.with_alpha(if have && prize.rarity >= Rarity::Epic {
                    0.45
                } else {
                    0.0
                }),
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(0.0),
                Val::Px(8.0),
            ),
            FocusPolicy::Pass,
        ))
        .with_children(|s| {
            match mode {
                CellMode::Collection if !have => {
                    text(s, &ctx.tab, "?", 18.0, rc.mix(&Color::BLACK, 0.2));
                }
                _ => {
                    let lit = have || mode == CellMode::Banner;
                    let tint = if lit && !(mode == CellMode::Counter && have) {
                        Color::WHITE
                    } else {
                        Color::srgb(0.38, 0.38, 0.38)
                    };
                    s.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                            ..default()
                        },
                        ItemIcon {
                            icon: prize.icon,
                            tint,
                        },
                        FocusPolicy::Pass,
                    ));
                }
            }
            if mode != CellMode::Banner {
                ring(s, ctx.tab.art);
            }
        });
        if mode == CellMode::Counter {
            if have {
                text(c, &ctx.tab, "Owned", 10.0, GREEN);
            } else {
                let cost = booth::price(prize, ctx.banner);
                let color = if cost < prize.rarity.price() {
                    GREEN
                } else {
                    GOLD
                };
                text(c, &ctx.tab, &cost.to_string(), 11.0, color);
            }
        }
    });
}

/// The picked prize in full: what it is, whether the account has it, its odds, its price, and at
/// the Counter a Buy button.
fn details(b: &mut ChildSpawnerCommands, ctx: &Ctx, buying: bool) {
    let Some(prize) = ctx.ui.pick.and_then(|i| catalog().get(i)) else {
        let hint = if buying {
            "Click a prize to see its price."
        } else {
            "Click a prize to see what it is and where it comes from."
        };
        para(b, &ctx.tab, hint, 13.0, DIM);
        return;
    };
    let have = owned(ctx.profile, prize);
    let rc = prize.rarity.color();
    b.spawn(Node {
        column_gap: Val::Px(8.0),
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(6.0)),
        ..default()
    })
    .with_children(|h| {
        socket(h, prize, 44.0, have);
        h.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        })
        .with_children(|col| {
            para(col, &ctx.tab, &prize.name, 15.0, rc);
            text(
                col,
                &ctx.tab,
                &format!("{} {}", prize.rarity.label(), prize.kind.label()),
                12.0,
                rc.mix(&Color::WHITE, 0.3),
            );
        });
    });
    let (status, color) = if have {
        ("Collected", GREEN)
    } else {
        ("Not yet collected", DIM)
    };
    text(b, &ctx.tab, status, 13.0, color);
    gap(b, 4.0);
    let featured = ctx.banner.features(prize);
    if featured {
        para(
            b,
            &ctx.tab,
            &format!(
                "Featured in {}: half of its rarity's pulls go to the featured prizes.",
                ctx.banner.name
            ),
            12.0,
            GREEN,
        );
        gap(b, 4.0);
    }
    let pool = catalog()
        .iter()
        .filter(|p| p.rarity == prize.rarity)
        .count();
    para(
        b,
        &ctx.tab,
        &format!(
            "Prize Booth: a {} pull ({} of pulls) is one of {pool} {} prizes.",
            prize.rarity.label(),
            prize.rarity.odds_label(),
            prize.rarity.label()
        ),
        12.0,
        PARCHMENT,
    );
    gap(b, 4.0);
    let full = prize.rarity.price();
    let cost = booth::price(prize, ctx.banner);
    let price = if cost < full {
        format!("Prize Counter: {cost} Favor (20% off, was {full})")
    } else {
        format!("Prize Counter: {cost} Favor")
    };
    para(b, &ctx.tab, &price, 12.0, FAVOR);
    gap(b, 4.0);
    para(b, &ctx.tab, kind_note(prize.kind), 11.0, DIM);
    if !buying {
        return;
    }
    gap(b, 10.0);
    let short = cost.saturating_sub(ctx.favor());
    b.spawn(Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: Val::Px(3.0),
        width: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|col| {
        let btn = glue_button(
            col,
            ctx.tab.art,
            ctx.tab.font,
            FaireAction::Buy(ctx.ui.pick.unwrap_or_default()),
            &format!("Buy · {cost} Favor"),
            200.0,
            40.0,
            GlueBtnKind::Dialog,
            1.0,
        );
        let off = have || short > 0;
        col.commands().entity(btn).insert(GlueDisabled(off));
        if have {
            text(col, &ctx.tab, "You have it already", 11.0, REFUSED);
        } else if short > 0 {
            text(
                col,
                &ctx.tab,
                &format!("Need {short} more Favor"),
                11.0,
                REFUSED,
            );
        }
        if let Some((line, color)) = &ctx.ui.message {
            para(col, &ctx.tab, line, 12.0, *color);
        }
    });
}

/// What having a prize of `kind` does today.
fn kind_note(kind: Kind) -> &'static str {
    match kind {
        Kind::CompanionLook => {
            "Dress your companion in it on the Armory's Companion tab, once the Companion perk is bought."
        }
        Kind::Mount | Kind::MountHat => "Equip in the Armory > Stable.",
        Kind::Tabard | Kind::Title => "Used in the Armory Wardrobe.",
        Kind::Toy => "Used in the Wardrobe Toy bar during runs.",
        Kind::DarkmoonCard => "Used in Deck > trinket when all eight cards are collected.",
    }
}

// ── Odds & Pity ─────────────────────────────────────────────────────────────────────────────────

fn odds_tab(p: &mut ChildSpawnerCommands, ctx: &Ctx) {
    panel(p, &ctx.tab, side(), Some("Odds"), |b| {
        table_row(b, ctx, ["Rarity", "Odds", "Dupe", "Price"], GOLD);
        for r in Rarity::ALL {
            table_row(
                b,
                ctx,
                [
                    r.label(),
                    &r.odds_label(),
                    &format!("+{}", r.favor()),
                    &r.price().to_string(),
                ],
                r.color(),
            );
        }
        gap(b, 6.0);
        para(
            b,
            &ctx.tab,
            "Dupe: the Favor a duplicate pays. Price: the Prize Counter's, in Favor.",
            11.0,
            DIM,
        );
        gap(b, 10.0);
        text(b, &ctx.tab, "Each prize", 13.0, GOLD);
        gap(b, 2.0);
        for r in Rarity::ALL {
            let n = catalog().iter().filter(|p| p.rarity == r).count();
            let each = f64::from(r.odds()) / 100.0 / n.max(1) as f64;
            line_pair(
                b,
                ctx,
                &format!("{} ({n})", r.label()),
                &format!("{each:.3}% a pull"),
                r.color(),
            );
        }
    });
    panel(p, &ctx.tab, center(), Some("How the Faire Works"), |b| {
        let rules = [
            (
                "Tickets",
                format!(
                    "Every run pays a ticket for each {} gold it ends with, +{} for a win and +{} \
                     more for a zone's first win, +{} for each quest-chain step and +{} for each \
                     achievement it brings. A run in the zone the Faire is visiting pays double. \
                     The Faire moves between {} and {} every {} runs.",
                    tickets::GOLD_PER_TICKET,
                    tickets::WIN_TICKETS,
                    tickets::FIRST_WIN_TICKETS,
                    tickets::CHAIN_STEP_TICKETS,
                    tickets::ACHIEVEMENT_TICKETS,
                    tickets::FAIRE_ZONES[0],
                    tickets::FAIRE_ZONES[1],
                    tickets::RUNS_PER_STAY,
                ),
            ),
            (
                "Pulls",
                format!(
                    "One pull costs {} tickets. Ten at once cost {} and always hold at least one \
                     Rare or better. A pull rolls its rarity at the odds, then one prize of that \
                     rarity at even odds.",
                    booth::PULL_COST,
                    booth::TEN_PULL_COST
                ),
            ),
            (
                "Pity",
                format!(
                    "On each banner type, the {}th pull without an Epic or better is one, and the \
                     {}th without a Legendary is one. A hit resets its count. The featured banners \
                     share one count; the standing booth keeps its own.",
                    booth::EPIC_PITY,
                    booth::LEGENDARY_PITY
                ),
            ),
            (
                "Featured banners",
                format!(
                    "A new themed banner opens every {} runs. Its featured prizes share half of \
                     their rarity's odds; the rest of the rarity shares the other half.",
                    tickets::RUNS_PER_STAY
                ),
            ),
            (
                "Favor",
                format!(
                    "A prize you have already turns into Faire Favor. Silas Darkmoon's Prize \
                     Counter sells any prize for Favor, the current banner's featured prizes {}% \
                     off.",
                    booth::FEATURED_DISCOUNT
                ),
            ),
        ];
        for (head, body) in rules {
            text(b, &ctx.tab, head, 14.0, GOLD);
            gap(b, 2.0);
            para(b, &ctx.tab, &body, 12.0, PARCHMENT);
            gap(b, 8.0);
        }
    });
    panel(p, &ctx.tab, side(), Some("Pity & Banners"), |b| {
        for kind in BannerKind::ALL {
            let (name, ..) = banner_face(ctx, kind);
            para(b, &ctx.tab, name, 13.0, Color::WHITE);
            gap(b, 2.0);
            pity_bars(b, ctx, ctx.profile.faire.pity(kind.key()));
        }
        gap(b, 4.0);
        text(b, &ctx.tab, "Banner schedule", 13.0, GOLD);
        gap(b, 2.0);
        let left = tickets::runs_left_here(ctx.runs);
        line_pair(
            b,
            ctx,
            ctx.banner.name,
            &format!("now, {left} left"),
            tint(ctx.banner.tint),
        );
        for (n, banner) in booth::upcoming(ctx.runs, 3) {
            line_pair(
                b,
                ctx,
                banner.name,
                &format!("in {n} run{}", if n == 1 { "" } else { "s" }),
                tint(banner.tint),
            );
        }
        gap(b, 8.0);
        text(b, &ctx.tab, "The Faire's travels", 13.0, GOLD);
        gap(b, 2.0);
        line_pair(
            b,
            ctx,
            tickets::faire_zone(ctx.runs),
            &format!("now, {left} left"),
            FAVOR,
        );
        let next = ctx.runs + left;
        line_pair(
            b,
            ctx,
            tickets::faire_zone(next),
            &format!("in {left} run{}", if left == 1 { "" } else { "s" }),
            PARCHMENT,
        );
    });
}

/// One row of the odds table: four columns.
fn table_row(p: &mut ChildSpawnerCommands, ctx: &Ctx, cols: [&str; 4], color: Color) {
    p.spawn(Node {
        width: Val::Percent(100.0),
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(2.0)),
        ..default()
    })
    .with_children(|r| {
        for (i, (s, w)) in cols.iter().zip([80.0, 50.0, 45.0, 45.0]).enumerate() {
            r.spawn(Node {
                width: Val::Px(w),
                flex_shrink: 0.0,
                justify_content: if i == 0 {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::FlexEnd
                },
                ..default()
            })
            .with_children(|c| {
                let col = if i == 0 { color } else { Color::WHITE };
                text(c, &ctx.tab, s, 12.0, col);
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelves_hold_their_kind_rarest_first() {
        let mounts = shelf_prizes(Some(Kind::Mount));
        assert!(mounts.iter().all(|&i| catalog()[i].kind == Kind::Mount));
        assert_eq!(catalog()[mounts[0]].rarity, Rarity::Legendary);
        assert_eq!(shelf_prizes(None).len(), catalog().len());
        // Every prize sits on exactly one kind's shelf.
        let by_kind: usize = Kind::ALL.iter().map(|&k| shelf_prizes(Some(k)).len()).sum();
        assert_eq!(by_kind, catalog().len());
    }

    #[test]
    fn summaries_read_well() {
        let p = |new, favor| Pull {
            prize: 0,
            new,
            favor,
            featured: false,
        };
        assert_eq!(pull_summary(&[p(true, 0)]), "Last pull: 1 new prize!");
        assert_eq!(
            pull_summary(&[p(false, 3), p(false, 8)]),
            "Last pull: all duplicates, +11 Favor"
        );
        assert_eq!(
            pull_summary(&[p(true, 0), p(true, 0), p(false, 1)]),
            "Last pull: 2 new prizes, +1 Favor"
        );
        assert_eq!(
            refusal_line(Refusal::Short(1), "ticket"),
            "You need 1 more ticket."
        );
        assert_eq!(
            refusal_line(Refusal::Short(20), "Favor"),
            "You need 20 more Favor."
        );
    }

    #[test]
    fn the_screen_opens_switches_and_closes_with_the_menu() {
        let mut ui = FaireUi::default();
        assert!(
            !ui.open,
            "closed at boot, so the auto-runner never meets it"
        );
        assert_eq!((ui.tab, ui.banner), (FaireTab::Booth, BannerKind::Featured));
        let before = ui.rebuilds;
        ui.refresh();
        assert_ne!(ui.rebuilds, before);
        let labels: Vec<&str> = FaireTab::ALL.iter().map(|t| t.label()).collect();
        assert_eq!(
            labels,
            ["Prize Booth", "Prize Counter", "Collection", "Odds & Pity"]
        );
    }

    #[test]
    fn sounds_are_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let kits = benilla_formats::load_sound_kit_catalog(&mut chain).expect("sound kits");
        for name in [
            SOUND_OPEN,
            SOUND_CLOSE,
            SOUND_TAB,
            SOUND_PICK,
            SOUND_PULL,
            SOUND_BUY,
        ]
        .into_iter()
        .chain(reveal::SOUNDS)
        {
            assert!(kits.by_name(name).is_some(), "{name}");
        }
        for icon in [TICKET_ICON, FAVOR_ICON, reveal::CARD_BACK]
            .into_iter()
            .chain(booth::BANNERS.iter().map(|b| b.emblem))
        {
            let path = format!("Interface\\Icons\\{icon}.blp");
            assert!(chain.contains(&path), "{path}");
        }
    }
}
