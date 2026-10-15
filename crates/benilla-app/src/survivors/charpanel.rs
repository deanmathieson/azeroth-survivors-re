//! The Character panel: C during a run opens a WoW-style "Character" window over the paused
//! battleground, and C or Esc closes it ([`super::hud::keys`] binds them, in runs only). On the
//! left the local hero: its race, class and level, the gear its class wears (the profile's
//! [`GearSave`], each piece in its rarity's colour) and its five jewelry slots. On the right a stat
//! sheet: one row per [`Stats`] number, and one per school a bonus raises, each with its total; a
//! click opens the row onto each source's share.
//!
//! # How the shares are counted
//!
//! The stats fold their sources together in [`HeroState::stats`]: flat health lands before the
//! health percentages scale it, armor and cooldown reduction are capped, targets and extra lives
//! round to whole numbers. So a source's share is not its bonus alone but how far adding it moved
//! the total. [`breakdown`] stacks the sources in a fixed order, base and class, level-up cards,
//! talents, gear, perks, racials, jewelry and run bonuses, and folds the stats again after each
//! through that same [`HeroState::stats`]; a source's share is the step it made. The steps
//! telescope, so the shares always sum to the total, which is the hero's own `stats()`. Where
//! sources interact (a cap reached, a health percentage scaling flat health) the later source in
//! the order takes the difference.
//!
//! Talents, gear, perks and racials are the meta bonuses as the run began ([`RunSources`]); jewelry
//! is what the hero wears now. "Run bonuses" is everything else in the hero's numbers (the
//! `WOW_SURVIVORS_BONUSES` test aid, any in-run effect that adds to the hero's bonuses), found as
//! the last step, from the stacked sources to the hero's actual stats.

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::ui_render::ui_material::MaterialNode;

use super::armory::{self, TabCtx};
use super::bonus::School;
use super::gear::{class_gear, hero_name, GearSave, ItemIcon, Slot};
use super::hud::{self, rgb, HudFont};
use super::jewelry::SLOTS;
use super::meta::MetaParts;
use super::{data, HeroState, LocalHero, Phase, Run, Selection, Stats};
use crate::glue::art::{GlueArt, DIM, GOLD};
use crate::glue::backdrop::{backdrop_border, tiled_bg_node};
use crate::glue::widgets::{outlined_text, overlay, GlueText, Hilight};

/// The key that opens and closes the panel during a run.
pub(super) const KEY: KeyCode = KeyCode::KeyC;

/// Over the HUD (400), under the tooltips; never shown with the level-up cards or the pause menu.
const Z: i32 = 425;
const WINDOW_W: f32 = 820.0;
const WINDOW_H: f32 = 660.0;
const HERO_W: f32 = 330.0;
const STAT_ROW_H: f32 = 18.0;
const SHARE_ROW_H: f32 = 14.0;
/// `GREEN_FONT_COLOR` and `RED_FONT_COLOR`: a share that raises a stat, and one that lowers it.
const UP: Color = Color::srgb(0.1, 1.0, 0.1);
const DOWN: Color = Color::srgb(1.0, 0.1, 0.1);
/// Shares smaller than this are none.
const EPS: f32 = 1e-4;

pub(super) fn plugin(app: &mut App) {
    phases(app);
    app.init_resource::<RunSources>()
        .init_resource::<PanelState>()
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            snapshot_sources,
        )
        .add_systems(Update, (panel_clicks, sync_panel).chain().after(hud::keys));
}

/// The panel pauses the run as the pause menu does; [`super::unpause`] resumes it with play.
fn phases(app: &mut App) {
    app.add_systems(OnEnter(Phase::Character), super::pause);
}

/// The meta bonuses by source as this run began, which the local hero's bonuses were built from.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(super) struct RunSources(pub(super) MetaParts);

/// Which stat row is open onto its shares, and a counter bumped to rebuild the panel.
#[derive(Resource, Default)]
pub(super) struct PanelState {
    open: Option<Line>,
    version: u32,
}

#[derive(Component)]
struct PanelRoot;

/// A stat row's button: a click opens or closes its shares.
#[derive(Component, Clone, Copy)]
struct StatButton(Line);

/// A run begins: its sources are kept for the breakdown, and the panel opens with every row shut.
fn snapshot_sources(
    parts: Res<MetaParts>,
    mut sources: ResMut<RunSources>,
    mut state: ResMut<PanelState>,
) {
    sources.0 = parts.clone();
    state.open = None;
}

/// Where a hero's numbers come from, in the order [`breakdown`] stacks them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Source {
    Base,
    Cards,
    Talents,
    Gear,
    Perks,
    Racial,
    Jewelry,
    Run,
}

impl Source {
    pub(super) const ALL: [Source; 8] = [
        Source::Base,
        Source::Cards,
        Source::Talents,
        Source::Gear,
        Source::Perks,
        Source::Racial,
        Source::Jewelry,
        Source::Run,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Source::Base => "Base & class",
            Source::Cards => "Level-up cards",
            Source::Talents => "Talents",
            Source::Gear => "Gear",
            Source::Perks => "Perks",
            Source::Racial => "Racials",
            Source::Jewelry => "Jewelry",
            Source::Run => "Run bonuses",
        }
    }
}

/// One row of the stat sheet: a [`Stats`] number, or one school's damage bonus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Line {
    Health,
    Damage,
    Cooldowns,
    Crit,
    CritDamage,
    Area,
    Targets,
    Speed,
    Regen,
    Pickup,
    Armor,
    Reduction,
    Experience,
    Gold,
    Luck,
    Lifesteal,
    Revives,
    School(School),
}

/// How a line's number reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Flat,
    Percent,
    Whole,
    Yards,
    YardsPerSec,
    PerSec,
}

const SCHOOLS: [School; 7] = [
    School::Physical,
    School::Holy,
    School::Fire,
    School::Nature,
    School::Frost,
    School::Shadow,
    School::Arcane,
];

impl Line {
    /// Every [`Stats`] number, in the sheet's order.
    pub(super) const STATS: [Line; 17] = [
        Line::Health,
        Line::Damage,
        Line::Cooldowns,
        Line::Crit,
        Line::CritDamage,
        Line::Area,
        Line::Targets,
        Line::Speed,
        Line::Regen,
        Line::Pickup,
        Line::Armor,
        Line::Reduction,
        Line::Experience,
        Line::Gold,
        Line::Luck,
        Line::Lifesteal,
        Line::Revives,
    ];

    pub(super) fn label(self) -> String {
        let s = match self {
            Line::Health => "Health",
            Line::Damage => "Damage",
            Line::Cooldowns => "Cooldown reduction",
            Line::Crit => "Critical strike",
            Line::CritDamage => "Critical damage",
            Line::Area => "Area of effect",
            Line::Targets => "Extra targets",
            Line::Speed => "Movement speed",
            Line::Regen => "Health per second",
            Line::Pickup => "Pickup range",
            Line::Armor => "Armor",
            Line::Reduction => "Damage reduction",
            Line::Experience => "Experience",
            Line::Gold => "Gold",
            Line::Luck => "Luck",
            Line::Lifesteal => "Lifesteal",
            Line::Revives => "Extra lives",
            Line::School(school) => return format!("{} damage", school.label()),
        };
        s.to_string()
    }

    fn unit(self) -> Unit {
        match self {
            Line::Health | Line::Armor => Unit::Flat,
            Line::Targets | Line::Revives => Unit::Whole,
            Line::Speed => Unit::YardsPerSec,
            Line::Regen => Unit::PerSec,
            Line::Pickup => Unit::Yards,
            _ => Unit::Percent,
        }
    }

    /// This line's number for `hero`, whose stats are `s`.
    fn of(self, s: &Stats, hero: &HeroState) -> f32 {
        match self {
            Line::Health => s.max_hp,
            Line::Damage => s.might,
            Line::Cooldowns => s.haste,
            Line::Crit => s.crit,
            Line::CritDamage => s.crit_damage,
            Line::Area => s.area,
            Line::Targets => s.targets as f32,
            Line::Speed => s.speed,
            Line::Regen => s.regen,
            Line::Pickup => s.magnet,
            Line::Armor => s.armor,
            Line::Reduction => s.armor_frac,
            Line::Experience => s.growth,
            Line::Gold => s.gold,
            Line::Luck => s.luck,
            Line::Lifesteal => s.lifesteal,
            Line::Revives => s.revives as f32,
            Line::School(school) => hero.school_mult(school) - 1.0,
        }
    }

    /// `v` as this line reads it; `signed` puts a sign on it, as a share or a school bonus reads.
    fn show(self, v: f32, signed: bool) -> String {
        let signed = signed || matches!(self, Line::School(_));
        let size = if signed { v.abs() } else { v };
        let body = match self.unit() {
            Unit::Flat => num(size),
            Unit::Percent => format!("{}%", num(size * 100.0)),
            Unit::Whole => format!("{}", size.round() as i64),
            Unit::Yards => format!("{} yd", num(size)),
            Unit::YardsPerSec => format!("{} yd/s", num(size)),
            Unit::PerSec => format!("{} / sec", num(size)),
        };
        match (signed, v < 0.0) {
            (false, _) => body,
            (true, true) => format!("-{body}"),
            (true, false) => format!("+{body}"),
        }
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

/// One stat row: its total, and each [`Source`]'s share in [`Source::ALL`] order. The shares sum
/// to the total: the first is the base, every other the step its source made.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Row {
    pub line: Line,
    pub total: f32,
    pub shares: [f32; 8],
}

/// `hero`'s stat sheet, every line split by source as the module doc counts it, the meta-progression
/// sources as `parts` holds them. School lines are left out where no source raises the school.
pub(super) fn breakdown(hero: &HeroState, parts: &MetaParts) -> Vec<Row> {
    let lines: Vec<Line> = Line::STATS
        .into_iter()
        .chain(SCHOOLS.map(Line::School))
        .collect();
    let numbers = |h: &HeroState| {
        let s = h.stats();
        lines.iter().map(|l| l.of(&s, h)).collect::<Vec<f32>>()
    };
    // The hero rebuilt a source at a time: the bare class, its cards, then each bonus source.
    let mut layer = HeroState {
        class: hero.class,
        race: hero.race,
        ..HeroState::default()
    };
    let mut steps = vec![numbers(&layer)];
    layer.passives = hero.passives.clone();
    layer.overcharges = hero.overcharges;
    steps.push(numbers(&layer));
    let jewelry = hero.jewelry.bonuses();
    for part in [
        &parts.talents,
        &parts.gear,
        &parts.perks,
        &parts.race,
        &jewelry,
    ] {
        layer.bonuses.merge(part);
        steps.push(numbers(&layer));
    }
    // The rest, up to the hero's own numbers.
    steps.push(numbers(hero));
    lines
        .iter()
        .enumerate()
        .map(|(i, &line)| {
            let mut shares = [0.0; 8];
            shares[0] = steps[0][i];
            for (k, share) in shares.iter_mut().enumerate().skip(1) {
                *share = steps[k][i] - steps[k - 1][i];
            }
            Row {
                line,
                total: steps[steps.len() - 1][i],
                shares,
            }
        })
        .filter(|r| {
            !matches!(r.line, Line::School(_))
                || r.total.abs() > EPS
                || r.shares.iter().any(|v| v.abs() > EPS)
        })
        .collect()
}

/// A click on a stat row opens it onto its shares, or shuts it; one row is open at a time.
fn panel_clicks(
    phase: Res<State<Phase>>,
    presses: Query<(&Interaction, &StatButton), Changed<Interaction>>,
    mut state: ResMut<PanelState>,
) {
    if *phase.get() != Phase::Character {
        return;
    }
    let Some(line) = presses
        .iter()
        .find(|(i, _)| **i == Interaction::Pressed)
        .map(|(_, b)| b.0)
    else {
        return;
    };
    state.open = (state.open != Some(line)).then_some(line);
    state.version = state.version.wrapping_add(1);
}

/// Build the panel as it opens and rebuild it as a row opens or shuts; despawn it as it closes.
fn sync_panel(world: &mut World, mut built: Local<Option<(Entity, u32)>>) {
    if built.is_some_and(|(root, _)| world.get_entity(root).is_err()) {
        *built = None;
    }
    let on = *world.resource::<State<Phase>>().get() == Phase::Character;
    if !on {
        if let Some((root, _)) = built.take() {
            world.entity_mut(root).despawn();
        }
        return;
    }
    let version = world.resource::<PanelState>().version;
    if built.is_some_and(|(_, v)| v == version) {
        return;
    }
    let mut heroes = world.query_filtered::<&HeroState, With<LocalHero>>();
    let mut queue = CommandQueue::default();
    let root = {
        let (Some(art), Some(font)) = (
            world.get_resource::<GlueArt>(),
            world.get_resource::<HudFont>(),
        ) else {
            return;
        };
        let Ok(hero) = heroes.single(world) else {
            return;
        };
        let ctx = TabCtx {
            art,
            font: &font.0,
            selection: world.resource::<Selection>(),
            world,
        };
        let class = data::CLASSES
            .iter()
            .position(|c| c.id == hero.class.id)
            .unwrap_or(0);
        let sheet = Sheet {
            hero,
            class,
            level: world.resource::<Run>().level,
            gear: class_gear(world, class),
            rows: breakdown(hero, &world.resource::<RunSources>().0),
            open: world.resource::<PanelState>().open,
        };
        let mut commands = Commands::new(&mut queue, world);
        if let Some((old, _)) = *built {
            commands.entity(old).despawn();
        }
        spawn_panel(&mut commands, &ctx, &sheet)
    };
    queue.apply(world);
    *built = Some((root, version));
}

/// What the panel shows.
struct Sheet<'a> {
    hero: &'a HeroState,
    class: usize,
    level: u32,
    gear: &'a GearSave,
    rows: Vec<Row>,
    open: Option<Line>,
}

/// The shade, the window and its two columns.
fn spawn_panel(commands: &mut Commands, ctx: &TabCtx, sheet: &Sheet) -> Entity {
    let px = Val::Px;
    commands
        .spawn((
            PanelRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            GlobalZIndex(Z),
            FocusPolicy::Block,
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: px(WINDOW_W),
                height: px(WINDOW_H),
                flex_direction: FlexDirection::Column,
                padding: UiRect::new(px(22.0), px(22.0), px(18.0), px(20.0)),
                row_gap: px(6.0),
                ..default()
            })
            .with_children(|w| {
                // `UI-DialogBox-Border` over its tiled background, as the glue dialogs draw it.
                match (&ctx.art.dialog_bg, &ctx.art.dialog_border) {
                    (Some(bg), Some(border)) => {
                        w.spawn((
                            tiled_bg_node(bg.clone(), 32.0, 1.0, Color::WHITE),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(11.0),
                                right: px(12.0),
                                top: px(12.0),
                                bottom: px(11.0),
                                ..default()
                            },
                        ));
                        backdrop_border(w, border, 32.0, Color::WHITE);
                    }
                    _ => {
                        w.spawn((
                            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.96)),
                            BorderColor::all(Color::srgb(0.55, 0.45, 0.25)),
                            Node {
                                border: UiRect::all(px(2.0)),
                                ..overlay()
                            },
                        ));
                    }
                }
                w.spawn(Node {
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(px(10.0)),
                    ..default()
                })
                .with_children(|t| {
                    armory::text(t, ctx, "Character", 20.0, GOLD);
                    armory::text(t, ctx, "C or Esc to close", 11.0, DIM);
                });
                w.spawn(Node {
                    flex_grow: 1.0,
                    column_gap: px(10.0),
                    ..default()
                })
                .with_children(|body| {
                    let side = Node {
                        width: px(HERO_W),
                        height: Val::Percent(100.0),
                        flex_shrink: 0.0,
                        ..default()
                    };
                    armory::panel(body, ctx, side, Some("Hero"), |p| {
                        hero_column(p, ctx, sheet)
                    });
                    let main = Node {
                        flex_grow: 1.0,
                        height: Val::Percent(100.0),
                        ..default()
                    };
                    armory::panel(body, ctx, main, Some("Stats"), |p| {
                        stat_column(p, ctx, sheet)
                    });
                });
            });
        })
        .id()
}

/// A thin spacer.
fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

/// A gold heading.
fn heading(p: &mut ChildSpawnerCommands, ctx: &TabCtx, s: &str) {
    p.spawn(Node {
        flex_shrink: 0.0,
        margin: UiRect::bottom(Val::Px(2.0)),
        ..default()
    })
    .with_children(|h| {
        armory::text(h, ctx, s, 13.0, GOLD);
    });
}

/// Text that wraps in what is left of its row.
fn fill_text(p: &mut ChildSpawnerCommands, ctx: &TabCtx, s: &str, size: f32, color: Color) {
    outlined_text(
        p,
        Node {
            flex_grow: 1.0,
            flex_basis: Val::Px(0.0),
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
    );
}

/// A `size`-square item icon, or an empty dark socket.
fn socket(p: &mut ChildSpawnerCommands, icon: Option<&'static str>, size: f32) {
    let mut s = p.spawn((
        Node {
            width: Val::Px(size),
            height: Val::Px(size),
            flex_shrink: 0.0,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
    ));
    if let Some(icon) = icon {
        s.with_children(|s| {
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
                    icon,
                    tint: Color::WHITE,
                },
            ));
        });
    }
}

/// The hero: who it is, what its class wears, and its jewelry.
fn hero_column(p: &mut ChildSpawnerCommands, ctx: &TabCtx, sheet: &Sheet) {
    let hero = sheet.hero;
    let race = crate::char_select::race_name(hero.race);
    if let Some(name) = hero_name(ctx.world, sheet.class) {
        armory::text(p, ctx, name, 18.0, GOLD);
    }
    armory::text(
        p,
        ctx,
        &format!("{race} {}", hero.class.name),
        18.0,
        rgb(hero.class.color),
    );
    armory::text(
        p,
        ctx,
        &format!("Level {}", sheet.level),
        13.0,
        Color::WHITE,
    );
    gap(p, 8.0);
    heading(p, ctx, "Equipment");
    for slot in Slot::ALL {
        let worn = sheet.gear.worn(slot);
        p.spawn(Node {
            height: Val::Px(20.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|r| {
            r.spawn(Node {
                width: Val::Px(66.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|c| {
                let color = if worn.is_some() { GOLD } else { DIM };
                armory::text(c, ctx, slot.label(), 11.0, color);
            });
            socket(r, worn.map(|i| i.icon), 18.0);
            let (name, color) = worn.map_or(("Empty", DIM), |i| (i.name, i.rarity.color()));
            fill_text(r, ctx, name, 11.0, color);
        });
    }
    gap(p, 8.0);
    heading(p, ctx, "Jewelry");
    for (i, kind) in SLOTS.iter().enumerate() {
        let worn = hero.jewelry.slot(i);
        let n = SLOTS[..i].iter().filter(|k| *k == kind).count() + 1;
        let many = SLOTS.iter().filter(|k| *k == kind).count() > 1;
        let label = if many {
            format!("{} {n}", kind.label())
        } else {
            kind.label().to_string()
        };
        p.spawn(Node {
            min_height: Val::Px(32.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|r| {
            r.spawn(Node {
                width: Val::Px(66.0),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|c| {
                let color = if worn.is_some() { GOLD } else { DIM };
                armory::text(c, ctx, &label, 11.0, color);
            });
            socket(r, worn.map(|p| p.icon), 28.0);
            r.spawn(Node {
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .with_children(|c| match worn {
                Some(piece) => {
                    armory::para(c, ctx, piece.name, 11.0, piece.rarity.color());
                    let grants: Vec<String> = piece.bonuses.iter().map(|b| b.describe()).collect();
                    armory::para(c, ctx, &grants.join(", "), 10.0, UP);
                }
                None => {
                    armory::para(c, ctx, "Empty", 11.0, DIM);
                }
            });
        });
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    armory::para(
        p,
        ctx,
        "Caches and quests offer jewelry. Gear changes between runs, in the Armory.",
        10.0,
        DIM,
    );
}

/// The stat sheet: a row per line, the open one followed by its shares.
fn stat_column(p: &mut ChildSpawnerCommands, ctx: &TabCtx, sheet: &Sheet) {
    let sheen = ctx
        .art
        .quest_hilight
        .as_ref()
        .or(ctx.art.select_highlight.as_ref());
    for row in &sheet.rows {
        let open = sheet.open == Some(row.line);
        p.spawn((
            Button,
            StatButton(row.line),
            Node {
                height: Val::Px(STAT_ROW_H),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(6.0)),
                ..default()
            },
        ))
        .with_children(|r| {
            if let Some(hi) = sheen {
                r.spawn((
                    Hilight,
                    Visibility::Hidden,
                    MaterialNode(hi.clone()),
                    overlay(),
                ));
            }
            let label = format!("{} {}", if open { "-" } else { "+" }, row.line.label());
            armory::text(r, ctx, &label, 12.0, if open { Color::WHITE } else { GOLD });
            armory::text(r, ctx, &row.line.show(row.total, false), 12.0, Color::WHITE);
        });
        if !open {
            continue;
        }
        for (k, (&source, &v)) in Source::ALL.iter().zip(&row.shares).enumerate() {
            let base = k == 0;
            if v.abs() <= EPS && !(base && !matches!(row.line, Line::School(_))) {
                continue;
            }
            let color = match (base, v < 0.0) {
                (true, _) => Color::WHITE,
                (false, true) => DOWN,
                (false, false) => UP,
            };
            p.spawn(Node {
                height: Val::Px(SHARE_ROW_H),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::new(Val::Px(22.0), Val::Px(6.0), Val::ZERO, Val::ZERO),
                ..default()
            })
            .with_children(|r| {
                armory::text(r, ctx, source.label(), 11.0, DIM);
                armory::text(r, ctx, &row.line.show(v, !base), 11.0, color);
            });
        }
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    armory::para(
        p,
        ctx,
        "Click a stat to see where it comes from.",
        10.0,
        DIM,
    );
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::{Bonus, Bonuses, StatKey};
    use super::super::jewelry::{piece, wear};
    use super::super::{zones, MetaBonuses, BASE_HP};
    use super::*;

    fn passive(key: &str) -> &'static data::PassiveDef {
        data::PASSIVES.iter().find(|p| p.key == key).unwrap()
    }

    /// A warrior with cards, every meta source, three pieces of jewelry, and a test-aid bonus.
    fn sample() -> (HeroState, MetaParts) {
        let mut parts = MetaParts::default();
        parts.talents.add_all(&[
            Bonus::new(StatKey::Damage, 0.05),
            Bonus::new(StatKey::MaxHealth, 10.0),
            Bonus::school(School::Fire, 0.10),
            Bonus::new(StatKey::Cooldown, 0.10),
        ]);
        parts.gear.add_all(&[
            Bonus::new(StatKey::Cooldown, 0.25),
            Bonus::new(StatKey::Armor, 0.30),
            Bonus::new(StatKey::Targets, 1.0),
        ]);
        parts.perks.add(&Bonus::new(StatKey::Gold, 0.2));
        parts.race.add(&Bonus::new(StatKey::MaxHealth, 25.0));
        let mut bonuses = parts.total();
        let (env, _) = super::super::bonus::parse("luck=0.3,cooldown=0.2,armor=0.4");
        bonuses.merge(&env);
        let mut hero = HeroState::new(0, &data::CLASSES[0], 2, bonuses);
        hero.passives = vec![
            (passive("fortitude"), 2),
            (passive("pack"), 1),
            (passive("might"), 1),
            (passive("intellect"), 3),
            (passive("wild"), 2),
        ];
        for key in [
            "band_of_accuria",
            "kiss_of_the_spider",
            "mark_of_the_champion",
        ] {
            wear(&mut hero, piece(key).unwrap(), None).unwrap();
        }
        (hero, parts)
    }

    fn row(rows: &[Row], line: Line) -> &Row {
        rows.iter().find(|r| r.line == line).unwrap()
    }

    fn share(rows: &[Row], line: Line, source: Source) -> f32 {
        let k = Source::ALL.iter().position(|&s| s == source).unwrap();
        row(rows, line).shares[k]
    }

    #[test]
    fn the_breakdown_sums_to_the_total() {
        let (hero, parts) = sample();
        let rows = breakdown(&hero, &parts);
        let s = hero.stats();
        for r in &rows {
            let sum: f32 = r.shares.iter().sum();
            assert!(
                (sum - r.total).abs() <= 1e-3 * r.total.abs().max(1.0),
                "{:?}: {:?} sum to {sum}, not {}",
                r.line,
                r.shares,
                r.total
            );
            if let Line::School(school) = r.line {
                assert!((r.total - (hero.school_mult(school) - 1.0)).abs() < 1e-5);
            }
        }
        // The totals are the hero's own numbers.
        assert_eq!(row(&rows, Line::Health).total, s.max_hp);
        assert_eq!(row(&rows, Line::Cooldowns).total, s.haste);
        assert_eq!(row(&rows, Line::Revives).total, s.revives as f32);
        assert_eq!(rows.len(), Line::STATS.len() + 1, "one school raised: Fire");
        // Each source's share where it is plain: the warrior's 40 health on the base, the extra
        // life from the jewelry, the luck from the test aid, the gold from the perks.
        assert_eq!(share(&rows, Line::Health, Source::Base), BASE_HP + 40.0);
        assert_eq!(share(&rows, Line::Revives, Source::Jewelry), 1.0);
        assert_eq!(share(&rows, Line::Targets, Source::Gear), 1.0);
        assert!((share(&rows, Line::Luck, Source::Run) - 0.3).abs() < 1e-5);
        assert!((share(&rows, Line::Gold, Source::Perks) - 0.2).abs() < 1e-5);
        assert!((share(&rows, Line::Damage, Source::Cards) - 0.1).abs() < 1e-5);
        assert!((share(&rows, Line::School(School::Fire), Source::Talents) - 0.1).abs() < 1e-5);
        // Health percentages scale flat health: the racial's 25 counts at the cards' +40%.
        assert!((share(&rows, Line::Health, Source::Racial) - 25.0 * 1.4).abs() < 1e-3);
        // Caps: cooldowns take the cards' 0.24 on top of talents' 0.10, gear's 0.25 and the
        // jewelry's 0.12, and the test aid's 0.20 only up to the bonus cap; armor's fraction caps
        // the same way.
        assert!((share(&rows, Line::Cooldowns, Source::Cards) - 0.24).abs() < 1e-5);
        assert!((share(&rows, Line::Cooldowns, Source::Gear) - 0.25).abs() < 1e-5);
        assert!((share(&rows, Line::Cooldowns, Source::Jewelry) - 0.12).abs() < 1e-5);
        assert!(
            (share(&rows, Line::Cooldowns, Source::Run) - (0.50 - 0.47)).abs() < 1e-5,
            "{:?}",
            row(&rows, Line::Cooldowns)
        );
        assert!((share(&rows, Line::Reduction, Source::Jewelry) - 0.05).abs() < 1e-5);
        assert!((share(&rows, Line::Reduction, Source::Run) - (0.60 - 0.35)).abs() < 1e-5);
    }

    #[test]
    fn a_bare_hero_is_all_base() {
        let hero = HeroState::new(0, &data::CLASSES[6], 7, Bonuses::default());
        let rows = breakdown(&hero, &MetaParts::default());
        assert_eq!(rows.len(), Line::STATS.len(), "no school raised");
        for r in &rows {
            assert_eq!(r.shares[0], r.total, "{:?}", r.line);
            assert!(r.shares[1..].iter().all(|v| v.abs() < EPS), "{:?}", r.line);
        }
    }

    #[test]
    fn lines_read_as_the_sheet_prints_them() {
        assert_eq!(Line::Health.show(160.0, false), "160");
        assert_eq!(Line::Health.show(35.0, true), "+35");
        assert_eq!(Line::Damage.show(1.2, false), "120%");
        assert_eq!(Line::Cooldowns.show(-0.05, true), "-5%");
        assert_eq!(Line::Speed.show(7.5, false), "7.5 yd/s");
        assert_eq!(Line::Regen.show(1.0, false), "1 / sec");
        assert_eq!(Line::Pickup.show(4.5, false), "4.5 yd");
        assert_eq!(Line::Targets.show(1.0, true), "+1");
        assert_eq!(Line::Lifesteal.show(0.005, false), "0.5%");
        assert_eq!(Line::School(School::Fire).show(0.1, false), "+10%");
        assert_eq!(Line::School(School::Shadow).label(), "Shadow damage");
        for (i, a) in Line::STATS.iter().enumerate() {
            assert!(Line::STATS[i + 1..].iter().all(|b| a.label() != b.label()));
        }
    }

    /// Runs [`hud::keys`] once in `phase` with `key` pressed: the phase it asks for, if any.
    fn press(phase: Phase, key: KeyCode) -> Option<Phase> {
        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(key);
        world.insert_resource(keys);
        world.insert_resource(State::new(phase));
        world.init_resource::<NextState<Phase>>();
        world.init_resource::<Selection>();
        world.init_resource::<Run>();
        world.init_resource::<MetaBonuses>();
        world.insert_resource(zones::Unlocks::default());
        world.init_resource::<hud::Replay>();
        world.run_system_once(hud::keys).unwrap();
        match *world.resource::<NextState<Phase>>() {
            NextState::Pending(p) | NextState::PendingIfNeq(p) => Some(p),
            NextState::Unchanged => None,
        }
    }

    #[test]
    fn c_toggles_the_panel_only_in_runs() {
        assert_eq!(press(Phase::Playing, KEY), Some(Phase::Character));
        assert_eq!(press(Phase::Character, KEY), Some(Phase::Playing));
        assert_eq!(
            press(Phase::Character, KeyCode::Escape),
            Some(Phase::Playing)
        );
        // Esc in play is still the pause menu.
        assert_eq!(press(Phase::Playing, KeyCode::Escape), Some(Phase::Paused));
        for phase in [
            Phase::Menu,
            Phase::Paused,
            Phase::LevelUp,
            Phase::Jewelry,
            Phase::GameOver,
        ] {
            assert_eq!(press(phase, KEY), None, "{phase:?}");
        }
    }

    #[test]
    fn the_panel_pauses_the_run() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<Phase>()
            .init_resource::<Time<Virtual>>()
            .add_systems(OnEnter(Phase::Playing), super::super::unpause);
        phases(&mut app);
        let go = |app: &mut App, phase| {
            app.world_mut()
                .resource_mut::<NextState<Phase>>()
                .set(phase);
            app.update();
            assert_eq!(*app.world().resource::<State<Phase>>().get(), phase);
            app.world().resource::<Time<Virtual>>().is_paused()
        };
        assert!(!go(&mut app, Phase::Playing));
        assert!(go(&mut app, Phase::Character));
        assert!(!go(&mut app, Phase::Playing));
    }

    #[test]
    fn a_run_keeps_its_sources() {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<Phase>()
            .init_resource::<MetaParts>()
            .init_resource::<RunSources>()
            .init_resource::<PanelState>()
            .add_systems(
                OnTransition {
                    exited: Phase::Menu,
                    entered: Phase::Playing,
                },
                snapshot_sources,
            );
        app.update();
        app.world_mut()
            .resource_mut::<MetaParts>()
            .gear
            .add(&Bonus::new(StatKey::Damage, 0.2));
        app.world_mut()
            .resource_mut::<NextState<Phase>>()
            .set(Phase::Playing);
        app.update();
        let kept = app.world().resource::<RunSources>().0.clone();
        assert_eq!(kept, *app.world().resource::<MetaParts>());
        // A piece of gear dropped mid-run changes the parts, not the run's sources.
        app.world_mut()
            .resource_mut::<MetaParts>()
            .gear
            .add(&Bonus::new(StatKey::Damage, 0.1));
        app.update();
        assert_eq!(app.world().resource::<RunSources>().0, kept);
    }
}
