//! The damage meter, a Recount-style window on the `GameTooltip` backdrop at the bottom right:
//! one bar per member in its class's colour, ranked by damage done, healing done or damage
//! taken (a click on the header goes round them), with the run's total and its per-second rate.
//! [`KEY`] opens and closes it; alone, it shows the lone hero.

use bevy::prelude::*;

use super::{class_color, in_run, Member, PartyView, MAX_ALLIES, STATUS_BAR, TOOLTIP_BG};
use super::{TOOLTIP_BORDER, TRACK};
use crate::survivors::hud::{self, ArtDone, HudFont, UiArt};
use crate::survivors::Phase;

/// Opens and closes the meter. Free in the mode: the world map it opens in the client does not
/// exist here, and the dev chord's mute (with Ctrl and Shift held) is left alone.
pub(super) const KEY: KeyCode = KeyCode::KeyM;
/// One bar for each member of the largest party.
const ROWS: usize = MAX_ALLIES + 1;
const WIDTH: f32 = 236.0;
const ROW_H: f32 = 15.0;
/// Above the XP bar, clear of the quest column.
const RIGHT: f32 = 12.0;
const BOTTOM: f32 = 22.0;
/// The `GameTooltip` backdrop's fill tint.
const TIP_FILL: Color = Color::srgb(0.09, 0.09, 0.19);
const Z: i32 = 406;

/// What the meter ranks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Mode {
    #[default]
    Damage,
    Healing,
    Taken,
}

impl Mode {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Damage => Self::Healing,
            Self::Healing => Self::Taken,
            Self::Taken => Self::Damage,
        }
    }

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Damage => "Damage Done",
            Self::Healing => "Healing Done",
            Self::Taken => "Damage Taken",
        }
    }

    /// The per-second column's heading.
    pub(super) fn rate(self) -> &'static str {
        match self {
            Self::Damage => "DPS",
            Self::Healing => "HPS",
            Self::Taken => "DTPS",
        }
    }

    pub(super) fn value(self, m: &Member) -> f64 {
        match self {
            Self::Damage => m.damage_done,
            Self::Healing => m.healing_done,
            Self::Taken => m.damage_taken,
        }
    }
}

/// Whether the meter is open, and what it ranks.
#[derive(Resource, Default)]
pub(super) struct Meter {
    pub open: bool,
    pub mode: Mode,
}

/// One bar of the meter.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MeterRow {
    pub name: String,
    pub class: u8,
    pub value: f64,
    pub per_sec: f64,
    /// The bar's length, against the leader's.
    pub fill: f32,
}

/// `value` over a run `elapsed` seconds long, per second; the first second counts as a whole
/// one, so an opening hit reads as itself and not a spike.
pub(super) fn per_second(value: f64, elapsed: f32) -> f64 {
    value / f64::from(elapsed.max(1.0))
}

/// The meter's bars for `view` under `mode`: the most first, a tie in player order, each bar
/// against the leader's.
pub(super) fn ranking(view: &PartyView, mode: Mode) -> Vec<MeterRow> {
    let mut members: Vec<&Member> = view.members.iter().collect();
    members.sort_by(|a, b| {
        mode.value(b)
            .total_cmp(&mode.value(a))
            .then(a.player.cmp(&b.player))
    });
    let top = members.first().map_or(0.0, |m| mode.value(m));
    members
        .into_iter()
        .map(|m| {
            let value = mode.value(m);
            MeterRow {
                name: m.name.clone(),
                class: m.class,
                value,
                per_sec: per_second(value, view.elapsed),
                fill: if top > 0.0 {
                    (value / top).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                },
            }
        })
        .collect()
}

/// A row's right-hand text: the total and the rate, "12.3k (45.6)".
pub(super) fn amount(row: &MeterRow) -> String {
    let rate = if row.per_sec >= 1e4 {
        hud::compact(row.per_sec)
    } else {
        format!("{:.1}", row.per_sec)
    };
    format!("{} ({rate})", hud::compact(row.value))
}

#[derive(Component)]
pub(super) struct MeterRoot;
#[derive(Component)]
pub(super) struct MeterHeader;
#[derive(Component)]
pub(super) struct MeterTitle;
#[derive(Component)]
pub(super) struct MeterRate;
#[derive(Component)]
pub(super) struct MeterSlot(usize);
#[derive(Component)]
pub(super) struct MeterBar(usize);
#[derive(Component, Clone, Copy)]
pub(super) struct MeterText(usize, bool);

/// [`KEY`] opens and closes the meter over a run; never with a modifier held.
pub(super) fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    mut meter: ResMut<Meter>,
) {
    let modified = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
    ]);
    if in_run(*phase.get()) && !modified && keys.just_pressed(KEY) {
        meter.open = !meter.open;
    }
}

/// A click on the header goes on to the next ranking.
pub(super) fn cycle(
    headers: Query<&Interaction, (Changed<Interaction>, With<MeterHeader>)>,
    mut meter: ResMut<Meter>,
) {
    for i in &headers {
        if *i == Interaction::Pressed {
            meter.mode = meter.mode.next();
        }
    }
}

/// The window, once the archives are readable for its backdrop.
pub(super) fn build(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    ui_scale: Res<UiScale>,
    mut assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    built: Query<(), With<MeterRoot>>,
) {
    use crate::glue::backdrop::{backdrop_border, backdrop_edges, tiled_bg_node};
    if !built.is_empty() {
        return;
    }
    let (Some(font), Some(assets)) = (font, assets.as_deref_mut()) else {
        return;
    };
    let f = &font.0;
    let bg = assets.sprite_texture(TOOLTIP_BG, &mut images);
    let edges = backdrop_edges(assets, TOOLTIP_BORDER, &mut images);
    let mut root = commands.spawn((
        MeterRoot,
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(RIGHT),
            bottom: Val::Px(BOTTOM),
            width: Val::Px(WIDTH),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(2.0),
            display: Display::None,
            ..default()
        },
        GlobalZIndex(Z),
        // The pointer on it is the UI's: no mouselook starts there.
        Interaction::default(),
        bevy::ui::FocusPolicy::Block,
    ));
    if bg.is_none() || edges.is_none() {
        root.insert(BackgroundColor(TIP_FILL.with_alpha(0.9)));
    }
    root.with_children(|w| {
        if let Some(bg) = bg {
            w.spawn((
                tiled_bg_node(bg, 16.0, ui_scale.0, TIP_FILL),
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
            backdrop_border(w, edges, 16.0, Color::WHITE);
        }
        w.spawn((
            MeterHeader,
            Button,
            Node {
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(2.0)),
                margin: UiRect::bottom(Val::Px(2.0)),
                ..default()
            },
        ))
        .with_children(|h| {
            h.spawn((MeterTitle, hud::text_bundle(f, "", 12.0, hud::GOLD_TONE)));
            h.spawn((MeterRate, hud::text_bundle(f, "", 10.0, hud::DIM_GREY)));
        });
        for i in 0..ROWS {
            w.spawn((
                MeterSlot(i),
                Node {
                    height: Val::Px(ROW_H),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(3.0)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(TRACK),
            ))
            .with_children(|row| {
                row.spawn((
                    MeterBar(i),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        height: Val::Percent(100.0),
                        width: Val::Percent(0.0),
                        ..default()
                    },
                    ImageNode {
                        color: Color::NONE,
                        ..default()
                    },
                    UiArt::tinted(STATUS_BAR, Color::WHITE),
                ));
                for right in [false, true] {
                    row.spawn((
                        MeterText(i, right),
                        hud::text_bundle(f, "", 10.0, Color::WHITE),
                        TextLayout::new(Justify::Left, LineBreak::NoWrap),
                    ));
                }
            });
        }
    });
}

/// The window's rows from [`PartyView`], while it is open over a run.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn refresh(
    phase: Res<State<Phase>>,
    view: Res<PartyView>,
    meter: Res<Meter>,
    mut root: Query<&mut Node, (With<MeterRoot>, Without<MeterSlot>, Without<MeterBar>)>,
    mut slots: Query<(&MeterSlot, &mut Node), (Without<MeterRoot>, Without<MeterBar>)>,
    mut bars: Query<
        (&MeterBar, &mut Node, &mut ImageNode, Has<ArtDone>),
        (Without<MeterRoot>, Without<MeterSlot>),
    >,
    mut texts: ParamSet<(
        Query<&mut Text, With<MeterTitle>>,
        Query<&mut Text, With<MeterRate>>,
        Query<(&MeterText, &mut Text, &mut TextColor)>,
    )>,
) {
    let open = meter.open && in_run(*phase.get());
    let display = |on: bool| if on { Display::Flex } else { Display::None };
    for mut node in &mut root {
        if node.display != display(open) {
            node.display = display(open);
        }
    }
    if !open {
        return;
    }
    let rows = ranking(&view, meter.mode);
    for mut t in &mut texts.p0() {
        if **t != meter.mode.title() {
            **t = meter.mode.title().to_string();
        }
    }
    for mut t in &mut texts.p1() {
        if **t != meter.mode.rate() {
            **t = meter.mode.rate().to_string();
        }
    }
    for (slot, mut node) in &mut slots {
        if node.display != display(slot.0 < rows.len()) {
            node.display = display(slot.0 < rows.len());
        }
    }
    for (bar, mut node, mut img, ready) in &mut bars {
        let Some(row) = rows.get(bar.0) else { continue };
        let w = Val::Percent(row.fill * 100.0);
        if node.width != w {
            node.width = w;
        }
        let tint = class_color(row.class).with_alpha(0.8);
        if ready && img.color != tint {
            img.color = tint;
        }
    }
    for (text, mut t, mut color) in &mut texts.p2() {
        let Some(row) = rows.get(text.0) else {
            continue;
        };
        let want = if text.1 {
            amount(row)
        } else {
            format!("{}. {}", text.0 + 1, row.name)
        };
        if **t != want {
            **t = want;
        }
        if color.0 != Color::WHITE {
            color.0 = Color::WHITE;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{member, view_of};
    use super::*;

    #[test]
    fn the_meter_ranks_by_the_mode_with_ties_in_player_order() {
        let mut a = member(0, "Warrior");
        let mut b = member(1, "Priest");
        let mut c = member(2, "Mage");
        (a.damage_done, b.damage_done, c.damage_done) = (600.0, 150.0, 1200.0);
        (a.healing_done, b.healing_done, c.healing_done) = (0.0, 900.0, 0.0);
        (a.damage_taken, b.damage_taken, c.damage_taken) = (300.0, 300.0, 50.0);
        let view = view_of(vec![a, b, c]);
        let names =
            |mode| -> Vec<String> { ranking(&view, mode).into_iter().map(|r| r.name).collect() };
        assert_eq!(names(Mode::Damage), ["Player 3", "Player 1", "Player 2"]);
        assert_eq!(names(Mode::Healing)[0], "Player 2");
        // A tie: player order.
        assert_eq!(names(Mode::Taken), ["Player 1", "Player 2", "Player 3"]);
        let damage = ranking(&view, Mode::Damage);
        assert_eq!(damage[0].fill, 1.0);
        assert_eq!(damage[1].fill, 0.5);
        // A minute in: per second.
        assert_eq!(damage[0].per_sec, 20.0);
        assert_eq!(amount(&damage[0]), "1200 (20.0)");
        // No healing at all: no bar for anyone.
        assert!(ranking(&view, Mode::Healing)[1..]
            .iter()
            .all(|r| r.fill == 0.0));
        // The header goes round.
        assert_eq!(Mode::Damage.next().next().next(), Mode::Damage);
        assert_eq!(
            [Mode::Damage, Mode::Healing, Mode::Taken].map(Mode::rate),
            ["DPS", "HPS", "DTPS"]
        );
    }

    #[test]
    fn per_second_is_over_the_run_and_never_spikes_at_its_start() {
        assert_eq!(per_second(600.0, 60.0), 10.0);
        assert_eq!(per_second(5.0, 0.25), 5.0);
        assert_eq!(per_second(0.0, 0.0), 0.0);
        assert_eq!(per_second(1.5e6, 300.0), 5000.0);
    }

    #[test]
    fn alone_the_meter_shows_the_lone_hero() {
        let mut me = member(0, "Rogue");
        me.damage_done = 42.0;
        let rows = ranking(&view_of(vec![me]), Mode::Damage);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].name.as_str(), rows[0].fill), ("Player 1", 1.0));
    }

    #[test]
    fn m_opens_and_closes_it_over_a_run_only() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<Meter>();
        world.insert_resource(State::new(Phase::Playing));
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KEY);
        world.insert_resource(keys);
        world.run_system_once(toggle).unwrap();
        assert!(world.resource::<Meter>().open);
        // With the dev chord's modifiers held, it is not the meter's.
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ControlLeft);
        keys.press(KEY);
        world.insert_resource(keys);
        world.run_system_once(toggle).unwrap();
        assert!(world.resource::<Meter>().open);
        // At the menu it does nothing.
        world.insert_resource(State::new(Phase::Menu));
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KEY);
        world.insert_resource(keys);
        world.run_system_once(toggle).unwrap();
        assert!(world.resource::<Meter>().open);
    }
}
