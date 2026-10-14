//! Run Options: difficulty sliders the player sets at the menu (enemy density, health, damage and
//! speed) that raise a rewards multiplier. The options live in [`RunOptions`], saved to
//! `benilla-config/survivors-run-options.toml` (`localStorage` on the page). A run copies the
//! multiplier onto [`Run::reward_mult`] as it begins, and [`scaled`] is the one place an enemy
//! takes the options, at spawn.
//!
//! Test aids: `WOW_SURVIVORS_RUN_OPTIONS="density=3,health=1.5,damage=1,speed=1.1"` sets the
//! options for the session (an unnamed key stays at 100%, and nothing is saved), and
//! `WOW_SURVIVORS_OPEN=run_options` opens the panel once the menu is up, saving
//! `<WOW_SURVIVORS_SHOTS>/run-options.png` [`SHOT_DELAY`] seconds later when that is set.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::ui::FocusPolicy;
use serde::{Deserialize, Serialize};

use super::data::EnemyDef;
use super::hud::{HudFont, MenuRoot};
use super::menu::MenuBuilt;
use super::{Phase, Run};
use crate::glue::art::{GlueArt, GOLD, INFO_TEXT};
use crate::glue::backdrop::{backdrop_border, tiled_bg_node};
use crate::glue::widgets::{glue_button, outlined_text, GlueBtnKind, GlueDisabled, GlueText};
use crate::glue::GlueClicks;

/// The rewards multiplier's ceiling: past it the options only make the run harder.
const REWARD_CAP: f32 = 2.0;
/// `GREEN_FONT_COLOR`, the multiplier wherever it shows.
pub(super) const GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
/// A value raised above 100%.
const RAISED: Color = Color::srgb(1.0, 0.4, 0.3);
/// The panel, and its rows inside the backdrop's padding.
const PANEL_W: f32 = 560.0;
const ROW_W: f32 = 496.0;
/// Seconds from the test aid opening the panel to its screenshot.
const SHOT_DELAY: f32 = 1.5;

/// One of the four options.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Slider {
    Density,
    Health,
    Damage,
    Speed,
}

impl Slider {
    pub(super) const ALL: [Self; 4] = [Self::Density, Self::Health, Self::Damage, Self::Speed];

    /// `(lowest, highest, step)`, as multipliers.
    pub(super) fn range(self) -> (f32, f32, f32) {
        match self {
            Self::Density | Self::Health => (1.0, 5.0, 0.25),
            Self::Damage => (1.0, 3.0, 0.25),
            Self::Speed => (1.0, 1.5, 0.05),
        }
    }

    /// The key in the saved file and in the environment override.
    fn key(self) -> &'static str {
        match self {
            Self::Density => "density",
            Self::Health => "health",
            Self::Damage => "damage",
            Self::Speed => "speed",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Density => "Density",
            Self::Health => "Health",
            Self::Damage => "Damage",
            Self::Speed => "Speed",
        }
    }

    /// The row's one-line description, with the range.
    fn blurb(self) -> String {
        let what = match self {
            Self::Density => "More enemies, each worth less XP",
            Self::Health => "Enemy, elite and boss health",
            Self::Damage => "The damage enemies deal",
            Self::Speed => "Enemy movement speed",
        };
        let (lo, hi, _) = self.range();
        format!("{what} ({} to {})", percent(lo), percent(hi))
    }
}

/// The player's difficulty multipliers, 1.0 = 100%. Always in range and on a step: every write
/// goes through [`RunOptions::set`].
#[derive(Resource, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct RunOptions {
    /// More enemies at once, each worth less experience.
    #[serde(serialize_with = "two_places")]
    pub density: f32,
    /// Enemy, elite and boss health.
    #[serde(serialize_with = "two_places")]
    pub health: f32,
    /// The damage enemies deal.
    #[serde(serialize_with = "two_places")]
    pub damage: f32,
    /// Enemy move speed.
    #[serde(serialize_with = "two_places")]
    pub speed: f32,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            density: 1.0,
            health: 1.0,
            damage: 1.0,
            speed: 1.0,
        }
    }
}

/// The saved file's numbers as written by hand (`1.15`, not the f32's `1.149999976158142`).
fn two_places<S: serde::Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64((f64::from(*v) * 100.0).round() / 100.0)
}

/// `v` clamped to the option's range and snapped to its nearest step; NaN is the lowest.
fn snap(slider: Slider, v: f32) -> f32 {
    let (lo, hi, step) = slider.range();
    if v.is_nan() {
        return lo;
    }
    let steps = ((v.clamp(lo, hi) - lo) / step).round();
    (lo + steps * step).min(hi)
}

impl RunOptions {
    pub(super) fn get(&self, slider: Slider) -> f32 {
        match slider {
            Slider::Density => self.density,
            Slider::Health => self.health,
            Slider::Damage => self.damage,
            Slider::Speed => self.speed,
        }
    }

    /// Set an option, clamped to its range and snapped to its step.
    pub(super) fn set(&mut self, slider: Slider, v: f32) {
        let v = snap(slider, v);
        match slider {
            Slider::Density => self.density = v,
            Slider::Health => self.health = v,
            Slider::Damage => self.damage = v,
            Slider::Speed => self.speed = v,
        }
    }

    /// One step up (`1.0`) or down (`-1.0`).
    pub(super) fn step(&mut self, slider: Slider, dir: f32) {
        self.set(slider, self.get(slider) + dir * slider.range().2);
    }

    pub(super) fn at_min(&self, slider: Slider) -> bool {
        let (lo, _, step) = slider.range();
        self.get(slider) <= lo + step / 2.0
    }

    pub(super) fn at_max(&self, slider: Slider) -> bool {
        let (_, hi, step) = slider.range();
        self.get(slider) >= hi - step / 2.0
    }

    /// Every value back in range and on its step: a hand-edited file may hold anything.
    pub(super) fn sanitized(mut self) -> Self {
        for s in Slider::ALL {
            self.set(s, self.get(s));
        }
        self
    }

    /// `1 + 0.10·(density−1) + 0.15·(health−1) + 0.20·(damage−1) + 0.60·(speed−1)`, at most
    /// [`REWARD_CAP`]. Speed pays most: it is the hardest to outplay.
    pub(super) fn reward_mult(&self) -> f32 {
        let raw = 1.0
            + 0.10 * (self.density - 1.0)
            + 0.15 * (self.health - 1.0)
            + 0.20 * (self.damage - 1.0)
            + 0.60 * (self.speed - 1.0);
        raw.min(REWARD_CAP)
    }

    /// A crowd of `n` at 100% density, at this density.
    pub(super) fn crowd(&self, n: usize) -> usize {
        (n as f32 * self.density).round() as usize
    }

    fn to_toml(self) -> String {
        format!(
            "# Warcraft Survivors run options (Run Options on the menu), 1.0 = 100%.\n{}",
            toml::to_string(&self).unwrap_or_default()
        )
    }

    fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str::<Self>(text).map(Self::sanitized)
    }
}

/// The experience the density split still owes, in `1/(4·density)` parts of a point: whole
/// integers, so the total over many spawns is exact, never a float's drift.
#[derive(Default, Debug)]
pub(super) struct XpCarry(u32);

impl XpCarry {
    /// One spawn's share of `xp` at `density`, `xp / density` on average. The density sits on a
    /// quarter step, so `4·density` parts make a point.
    pub(super) fn share(&mut self, xp: u32, density: f32) -> u32 {
        let parts = (density * 4.0).round().max(4.0) as u32;
        let owed = self.0 + xp.saturating_mul(4);
        self.0 = owed % parts;
        owed / parts
    }
}

/// An enemy's base numbers under the options, before the run clock's own growth.
pub(super) struct Scaled {
    pub hp: f32,
    pub damage: f32,
    pub speed: f32,
    pub xp: u32,
}

/// The one place the options touch an enemy. A boss keeps its whole experience: density adds no
/// bosses.
pub(super) fn scaled(def: &EnemyDef, opts: &RunOptions, boss: bool, carry: &mut XpCarry) -> Scaled {
    Scaled {
        hp: def.hp * opts.health,
        damage: def.damage * opts.damage,
        speed: def.speed * opts.speed,
        xp: if boss {
            def.xp
        } else {
            carry.share(def.xp, opts.density)
        },
    }
}

/// `density=3,health=1.5,damage=1,speed=1.1` over the defaults (an unnamed key stays at 100%),
/// and a line for each entry it could not read.
pub(super) fn parse_env(spec: &str) -> (RunOptions, Vec<String>) {
    let mut opts = RunOptions::default();
    let mut bad = Vec::new();
    for item in spec
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let Some((key, value)) = item.split_once('=') else {
            bad.push(format!("`{item}` is not key=value"));
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let Some(slider) = Slider::ALL
            .into_iter()
            .find(|s| s.key().eq_ignore_ascii_case(key))
        else {
            bad.push(format!("unknown option `{key}`"));
            continue;
        };
        match value.parse::<f32>() {
            Ok(v) if v.is_finite() => opts.set(slider, v),
            _ => bad.push(format!("`{value}` is not a number")),
        }
    }
    (opts, bad)
}

fn percent(v: f32) -> String {
    format!("{}%", (v * 100.0).round() as i32)
}

/// The multiplier as the menu and the results show it.
fn rewards_label(m: f32) -> String {
    format!("Rewards ×{m:.2}")
}

/// The panel's state, and the test aids'.
#[derive(Resource, Default)]
pub(super) struct OptionsPanel {
    open: bool,
    /// Off when the environment set the options: a test run never overwrites the saved file.
    persist: bool,
    /// `WOW_SURVIVORS_OPEN=run_options`: open once the menu is up.
    open_once: bool,
    shot_dir: Option<PathBuf>,
    /// Seconds since the test aid opened the panel, until its screenshot.
    shot_t: Option<f32>,
}

/// Run condition: the panel is shut, so the menu's own keys and clicks may act.
pub(super) fn closed(panel: Res<OptionsPanel>) -> bool {
    !panel.open
}

/// What a Run Options widget does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum OptionsAction {
    Open,
    Less(Slider),
    More(Slider),
    Reset,
    Done,
}

/// On [`MenuRoot`] once the button and the panel are in.
#[derive(Component)]
struct OptionsBuilt;
/// The panel's full-screen dim, which holds the panel and swallows clicks meant for the menu.
#[derive(Component)]
struct OptionsModal;
#[derive(Component)]
struct ValueText(Slider);
#[derive(Component)]
struct ModalRewards;
#[derive(Component)]
struct MenuRewards;
/// The results screen's rewards line, spawned with the results.
#[derive(Component)]
pub(super) struct ResultsRewards;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<RunOptions>()
        .init_resource::<OptionsPanel>()
        .add_systems(Startup, load_options)
        .add_systems(
            OnTransition {
                exited: Phase::Menu,
                entered: Phase::Playing,
            },
            stamp_run,
        )
        .add_systems(
            Update,
            (
                build_ui,
                open_from_env,
                options_input,
                refresh_ui.before(crate::glue::GlueVisuals),
                refresh_results,
            )
                .chain()
                .before(super::hud::keys),
        );
}

/// `benilla-config/survivors-run-options.toml`.
fn options_path() -> Option<PathBuf> {
    crate::local_state::home().map(|h| h.join("survivors-run-options.toml"))
}

/// The saved options; a missing file is the defaults, and a broken one too, with a warning.
fn load() -> RunOptions {
    let Some(path) = options_path() else {
        return RunOptions::default();
    };
    let text = match crate::local_state::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return RunOptions::default(),
        Err(e) => {
            warn!(
                "survivors: cannot read {}: {e}; run options at 100%",
                path.display()
            );
            return RunOptions::default();
        }
    };
    RunOptions::from_toml(&text).unwrap_or_else(|e| {
        warn!(
            "survivors: {} is malformed ({e}); run options at 100%",
            path.display()
        );
        RunOptions::default()
    })
}

fn save(opts: &RunOptions) {
    let Some(path) = options_path() else { return };
    if let Err(e) = crate::local_state::write_atomic(&path, &opts.to_toml()) {
        warn!("survivors: cannot save {}: {e}", path.display());
    }
}

/// The saved options, or the environment's for this session, and the test aids.
fn load_options(mut opts: ResMut<RunOptions>, mut panel: ResMut<OptionsPanel>) {
    *opts = load();
    panel.persist = true;
    if let Some(spec) = crate::webenv::var("WOW_SURVIVORS_RUN_OPTIONS") {
        let (over, bad) = parse_env(&spec);
        for b in bad {
            warn!("survivors: WOW_SURVIVORS_RUN_OPTIONS: {b}");
        }
        info!("survivors: run options from the environment, not saved: {over:?}");
        *opts = over;
        panel.persist = false;
    }
    let open_once = crate::webenv::var("WOW_SURVIVORS_OPEN")
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("run_options"));
    panel.open_once = open_once;
    panel.shot_dir = crate::webenv::var("WOW_SURVIVORS_SHOTS")
        .filter(|_| open_once)
        .map(PathBuf::from);
}

/// A run begins: it keeps the multiplier it was started under.
fn stamp_run(opts: Res<RunOptions>, mut run: ResMut<Run>) {
    run.reward_mult = opts.reward_mult();
}

/// The menu's Run Options button and the panel, into [`MenuRoot`] once the menu is built.
#[allow(clippy::type_complexity)]
fn build_ui(
    mut commands: Commands,
    roots: Query<Entity, (With<MenuRoot>, With<MenuBuilt>, Without<OptionsBuilt>)>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
) {
    let Ok(root) = roots.single() else { return };
    let f = &font.0;
    commands
        .entity(root)
        .insert(OptionsBuilt)
        .with_children(|ui| {
            menu_button(ui, &art, f);
            modal(ui, &art, f);
        });
}

/// Right of Enter World (200×60 at BOTTOM (0,30)), on its centre line, the multiplier under it.
fn menu_button(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn(Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(50.0),
        bottom: px(33.0),
        margin: UiRect::left(px(116.0)),
        width: px(150.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(1.0),
        ..default()
    })
    .with_children(|c| {
        glue_button(
            c,
            art,
            f,
            OptionsAction::Open,
            "Run Options",
            150.0,
            38.0,
            GlueBtnKind::Small,
            1.0,
        );
        outlined_text(
            c,
            Node::default(),
            (),
            MenuRewards,
            GlueText {
                text: &rewards_label(1.0),
                size: 12.0,
                color: GREEN,
                wrap: false,
            },
            f,
            1.0,
        );
    });
}

/// The panel, centred over a dim that takes every click: the `GlueDialog` backdrop, a title, a
/// line on what the options do, a row per option, the multiplier, Reset and Done.
fn modal(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn((
        OptionsModal,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        FocusPolicy::Block,
        // Over the menu's own widgets, whichever order they were built in.
        ZIndex(100),
        Visibility::Hidden,
    ))
    .with_children(|dim| {
        dim.spawn(Node {
            width: px(PANEL_W),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(32.0), px(26.0)),
            row_gap: px(6.0),
            ..default()
        })
        .with_children(|p| {
            // `GlueDialog`'s backdrop: the background tiled at 32 inside (11,12,12,11) under the
            // 32-edge border.
            if let (Some(bg), Some(border)) = (&art.dialog_bg, &art.dialog_border) {
                p.spawn((
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
                backdrop_border(p, border, 32.0, Color::WHITE);
            } else {
                p.spawn((
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.95)),
                    crate::glue::widgets::overlay(),
                ));
            }
            let line = |p: &mut ChildSpawnerCommands, text: &str, size: f32, color: Color| {
                outlined_text(
                    p,
                    Node::default(),
                    (),
                    (),
                    GlueText {
                        text,
                        size,
                        color,
                        wrap: false,
                    },
                    f,
                    1.0,
                );
            };
            line(p, "Run Options", 20.0, GOLD);
            line(
                p,
                "Make the horde harder, and the run pays more for it.",
                13.0,
                INFO_TEXT,
            );
            p.spawn(Node {
                flex_direction: FlexDirection::Column,
                margin: UiRect::vertical(px(8.0)),
                row_gap: px(4.0),
                ..default()
            })
            .with_children(|rows| {
                for s in Slider::ALL {
                    option_row(rows, art, f, s);
                }
            });
            outlined_text(
                p,
                Node::default(),
                (),
                ModalRewards,
                GlueText {
                    text: "",
                    size: 16.0,
                    color: GREEN,
                    wrap: false,
                },
                f,
                1.0,
            );
            // `GlueDialogButtonTemplate` 200×40, the pair with the dialog's 13 gap.
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(13.0),
                margin: UiRect::top(px(10.0)),
                ..default()
            })
            .with_children(|row| {
                for (action, caption) in [
                    (OptionsAction::Reset, "Reset"),
                    (OptionsAction::Done, "Done"),
                ] {
                    glue_button(
                        row,
                        art,
                        f,
                        action,
                        caption,
                        200.0,
                        40.0,
                        GlueBtnKind::Dialog,
                        1.0,
                    );
                }
            });
        });
    });
}

/// Name and description on the left, then − value +.
fn option_row(rows: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>, s: Slider) {
    let px = Val::Px;
    rows.spawn(Node {
        width: px(ROW_W),
        height: px(40.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(6.0),
        ..default()
    })
    .with_children(|row| {
        row.spawn(Node {
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            row_gap: px(2.0),
            ..default()
        })
        .with_children(|col| {
            let blurb = s.blurb();
            for (text, size, color) in [(s.name(), 15.0, GOLD), (blurb.as_str(), 12.0, INFO_TEXT)] {
                outlined_text(
                    col,
                    Node::default(),
                    (),
                    (),
                    GlueText {
                        text,
                        size,
                        color,
                        wrap: false,
                    },
                    f,
                    1.0,
                );
            }
        });
        glue_button(
            row,
            art,
            f,
            OptionsAction::Less(s),
            "-",
            34.0,
            32.0,
            GlueBtnKind::Dialog,
            1.0,
        );
        outlined_text(
            row,
            Node {
                width: px(64.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            (),
            ValueText(s),
            GlueText {
                text: "100%",
                size: 16.0,
                color: Color::WHITE,
                wrap: false,
            },
            f,
            1.0,
        );
        glue_button(
            row,
            art,
            f,
            OptionsAction::More(s),
            "+",
            34.0,
            32.0,
            GlueBtnKind::Dialog,
            1.0,
        );
    });
}

/// `WOW_SURVIVORS_OPEN=run_options`: open the panel once the menu is up, and screenshot it.
fn open_from_env(
    mut commands: Commands,
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    loading: Option<Res<crate::loading_screen::LoadingScreen>>,
    built: Query<(), With<OptionsModal>>,
    mut panel: ResMut<OptionsPanel>,
) {
    if panel.open_once
        && *phase.get() == Phase::Menu
        && !built.is_empty()
        && !loading.is_some_and(|l| l.covering())
    {
        panel.open_once = false;
        panel.open = true;
        if panel.shot_dir.is_some() {
            panel.shot_t = Some(0.0);
        }
    }
    let Some(t) = panel.shot_t else { return };
    let t = t + time.delta_secs();
    if t < SHOT_DELAY {
        panel.shot_t = Some(t);
        return;
    }
    panel.shot_t = None;
    if let Some(dir) = panel.shot_dir.take() {
        let path = dir.join("run-options.png");
        info!("survivors: run options screenshot to {}", path.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
}

/// The button opens the panel; inside it, − and + step an option, Reset restores 100%, and Done
/// or Esc close it. A change is saved at once. Off the menu the panel is shut.
fn options_input(
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    actions: Query<(Entity, &OptionsAction, &GlueDisabled)>,
    mut opts: ResMut<RunOptions>,
    mut panel: ResMut<OptionsPanel>,
) {
    if *phase.get() != Phase::Menu {
        if panel.open {
            panel.open = false;
        }
        return;
    }
    if panel.open && keys.just_pressed(KeyCode::Escape) {
        panel.open = false;
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _, off)| !off.0 && clicks.hit(*e))
        .map(|(_, a, _)| *a)
    else {
        return;
    };
    let before = *opts;
    match action {
        OptionsAction::Open => panel.open = true,
        OptionsAction::Done => panel.open = false,
        OptionsAction::Reset => *opts = RunOptions::default(),
        OptionsAction::Less(s) => opts.step(s, -1.0),
        OptionsAction::More(s) => opts.step(s, 1.0),
    }
    if *opts != before && panel.persist {
        save(&opts);
    }
}

/// The panel's visibility, values, limits and multiplier, and the menu's multiplier line.
#[allow(clippy::type_complexity)]
fn refresh_ui(
    opts: Res<RunOptions>,
    panel: Res<OptionsPanel>,
    mut modal: Query<(&mut Visibility, &mut Node), With<OptionsModal>>,
    mut buttons: Query<(&OptionsAction, &mut GlueDisabled)>,
    mut texts: ParamSet<(
        Query<(&ValueText, &mut Text, &mut TextColor)>,
        Query<&mut Text, With<ModalRewards>>,
        Query<&mut Text, With<MenuRewards>>,
    )>,
) {
    // Display too: a merely invisible button still takes clicks.
    let (vis, display) = if panel.open {
        (Visibility::Inherited, Display::Flex)
    } else {
        (Visibility::Hidden, Display::None)
    };
    for (mut v, mut node) in &mut modal {
        if *v != vis {
            *v = vis;
        }
        if node.display != display {
            node.display = display;
        }
    }
    for (action, mut disabled) in &mut buttons {
        let off = match *action {
            OptionsAction::Less(s) => opts.at_min(s),
            OptionsAction::More(s) => opts.at_max(s),
            OptionsAction::Reset => *opts == RunOptions::default(),
            OptionsAction::Open | OptionsAction::Done => false,
        };
        if disabled.0 != off {
            disabled.0 = off;
        }
    }
    for (value, mut t, mut color) in &mut texts.p0() {
        let v = opts.get(value.0);
        let want = percent(v);
        if t.0 != want {
            t.0 = want;
        }
        let want = if opts.at_min(value.0) {
            Color::WHITE
        } else {
            RAISED
        };
        if color.0 != want {
            color.0 = want;
        }
    }
    let m = opts.reward_mult();
    let want = format!("Rewards multiplier ×{m:.2}");
    for mut t in &mut texts.p1() {
        if t.0 != want {
            t.0 = want.clone();
        }
    }
    let want = rewards_label(m);
    for mut t in &mut texts.p2() {
        if t.0 != want {
            t.0 = want.clone();
        }
    }
}

/// The results screen's line: the multiplier the run was played under.
fn refresh_results(run: Res<Run>, mut lines: Query<&mut Text, With<ResultsRewards>>) {
    let want = rewards_label(run.reward_mult);
    for mut t in &mut lines {
        if t.0 != want {
            t.0 = want.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn opts(density: f32, health: f32, damage: f32, speed: f32) -> RunOptions {
        RunOptions {
            density,
            health,
            damage,
            speed,
        }
        .sanitized()
    }

    fn same(a: RunOptions, b: RunOptions) -> bool {
        Slider::ALL.iter().all(|&s| close(a.get(s), b.get(s)))
    }

    #[test]
    fn values_clamp_to_their_range_and_snap_to_their_step() {
        let mut o = RunOptions::default();
        o.set(Slider::Density, 9.0);
        assert!(close(o.density, 5.0));
        o.set(Slider::Density, 0.2);
        assert!(close(o.density, 1.0));
        o.set(Slider::Density, 2.3);
        assert!(close(o.density, 2.25));
        o.set(Slider::Damage, 7.0);
        assert!(close(o.damage, 3.0));
        o.set(Slider::Speed, 1.12);
        assert!(close(o.speed, 1.1));
        o.set(Slider::Speed, 1.13);
        assert!(close(o.speed, 1.15));
        o.set(Slider::Speed, 2.0);
        assert!(close(o.speed, 1.5));
        o.set(Slider::Health, f32::NAN);
        assert!(close(o.health, 1.0), "NaN is the lowest");
        o.set(Slider::Health, f32::INFINITY);
        assert!(close(o.health, 5.0));

        // Sixteen quarter steps from 100% to 500%, then the top holds.
        let mut o = RunOptions::default();
        assert!(o.at_min(Slider::Density) && !o.at_max(Slider::Density));
        for _ in 0..16 {
            o.step(Slider::Density, 1.0);
        }
        assert!(close(o.density, 5.0) && o.at_max(Slider::Density));
        o.step(Slider::Density, 1.0);
        assert!(close(o.density, 5.0));
        // Ten speed steps of 5%, and the bottom holds.
        for _ in 0..10 {
            o.step(Slider::Speed, 1.0);
        }
        assert!(close(o.speed, 1.5) && o.at_max(Slider::Speed));
        for _ in 0..12 {
            o.step(Slider::Speed, -1.0);
        }
        assert!(close(o.speed, 1.0) && o.at_min(Slider::Speed));
        assert_eq!(percent(opts(1.0, 1.0, 1.0, 1.15).speed), "115%");
        assert_eq!(opts(5.0, 1.0, 1.0, 1.0).crowd(220), 1100);
        assert_eq!(RunOptions::default().crowd(220), 220);
    }

    #[test]
    fn rewards_follow_the_formula_and_stop_at_two() {
        assert!(close(RunOptions::default().reward_mult(), 1.0));
        assert!(close(opts(2.0, 1.0, 1.0, 1.0).reward_mult(), 1.10));
        assert!(close(opts(1.0, 2.0, 1.0, 1.0).reward_mult(), 1.15));
        assert!(close(opts(1.0, 1.0, 2.0, 1.0).reward_mult(), 1.20));
        assert!(close(opts(1.0, 1.0, 1.0, 1.5).reward_mult(), 1.30));
        // 1 + 0.2 + 0.075 + 0 + 0.06.
        assert!(close(opts(3.0, 1.5, 1.0, 1.1).reward_mult(), 1.335));
        // Uncapped 1 + 0.4 + 0.6 + 0.4 + 0.3 = 2.7.
        assert!(close(opts(5.0, 5.0, 3.0, 1.5).reward_mult(), 2.0));
        assert!(close(opts(5.0, 5.0, 3.0, 1.0).reward_mult(), 2.0));
        // Just under the cap: 1 + 0.4 + 0.45 = 1.85.
        assert!(close(opts(5.0, 4.0, 1.0, 1.0).reward_mult(), 1.85));
        assert_eq!(rewards_label(1.35), "Rewards ×1.35");
        assert_eq!(rewards_label(1.0), "Rewards ×1.00");
    }

    #[test]
    fn the_density_split_keeps_total_experience_for_whole_densities() {
        for density in 1..=5u32 {
            for xp in [1u32, 2, 3, 5, 8, 20, 50, 140] {
                let mut carry = XpCarry::default();
                let kills = density * 10;
                let total: u32 = (0..kills).map(|_| carry.share(xp, density as f32)).sum();
                assert_eq!(total, xp * 10, "{xp} xp at {density}x");
            }
        }
        // A mixed crowd at 300%: the shares add up to a third of the whole.
        let mut carry = XpCarry::default();
        let crowd = [1u32, 5, 1, 20, 1, 5, 3, 1, 1];
        let total: u32 = crowd.iter().map(|&xp| carry.share(xp, 3.0)).sum();
        assert_eq!(total, crowd.iter().sum::<u32>() / 3);
        // At 100% every kill keeps its own.
        let mut carry = XpCarry::default();
        assert!(crowd.iter().all(|&xp| carry.share(xp, 1.0) == xp));
        // A quarter step: 250% gives 2 of every 5 points.
        let mut carry = XpCarry::default();
        let total: u32 = (0..10).map(|_| carry.share(1, 2.5)).sum();
        assert_eq!(total, 4);
    }

    #[test]
    fn the_scaling_touches_health_damage_speed_and_splits_xp() {
        let def = EnemyDef {
            name: "test",
            display: 1,
            hp: 100.0,
            speed: 5.0,
            damage: 10.0,
            scale: 1.0,
            xp: 3,
            level: 1,
            big: false,
        };
        let o = opts(3.0, 2.0, 1.5, 1.2);
        let mut carry = XpCarry::default();
        let s = scaled(&def, &o, false, &mut carry);
        assert!(close(s.hp, 200.0) && close(s.damage, 15.0) && close(s.speed, 6.0));
        assert_eq!(s.xp, 1);
        assert_eq!(
            scaled(&def, &o, true, &mut carry).xp,
            3,
            "a boss keeps it all"
        );
    }

    #[test]
    fn the_saved_file_round_trips_and_forgives() {
        let o = opts(3.0, 1.5, 2.25, 1.15);
        let text = o.to_toml();
        assert!(text.contains("density = 3.0"), "{text}");
        assert!(text.contains("speed = 1.15"), "{text}");
        let back = RunOptions::from_toml(&text).unwrap();
        assert!(same(back, o), "{back:?} != {o:?}");

        // A whole number, a missing key, an out-of-range value, an unknown key.
        let hand = RunOptions::from_toml("density = 3\nhealth = 40\nmood = \"grim\"\n").unwrap();
        assert!(same(hand, opts(3.0, 5.0, 1.0, 1.0)), "{hand:?}");
        assert!(same(
            RunOptions::from_toml("").unwrap(),
            RunOptions::default()
        ));
        assert!(RunOptions::from_toml("density = \"lots\"\n").is_err());
        assert!(RunOptions::from_toml("[[[ not toml").is_err());
    }

    #[test]
    fn the_environment_override_parses() {
        let (o, bad) = parse_env("density=3,health=1.5,damage=1,speed=1.1");
        assert!(bad.is_empty(), "{bad:?}");
        assert!(same(o, opts(3.0, 1.5, 1.0, 1.1)), "{o:?}");

        let (o, bad) = parse_env(" Density = 2 ; SPEED=2 ,");
        assert!(bad.is_empty(), "{bad:?}");
        assert!(
            same(o, opts(2.0, 1.0, 1.0, 1.5)),
            "clamped, the rest at 100%: {o:?}"
        );

        let (o, bad) = parse_env("bogus=1,health,damage=x,speed=inf");
        assert_eq!(bad.len(), 4, "{bad:?}");
        assert!(same(o, RunOptions::default()));
        assert!(same(parse_env("").0, RunOptions::default()));
    }
}
