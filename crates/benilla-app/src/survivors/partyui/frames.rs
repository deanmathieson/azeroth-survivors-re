//! The party frames: `PartyMemberFrame1` to `3` down the left side, one for each ally (the local
//! hero's own health stays where the HUD has it, under its feet), laid out as
//! `PartyFrameTemplates.xml` lays them: the class icon in the portrait's ring, the name over the
//! health bar with its numbers and the shield over it, the buffs in the debuff row under the bars,
//! and the frame below the last one's pet frame. A downed ally's portrait greys as
//! `PartyMemberHealthCheck` greys the dead, turns the ghost's blue while an ally gets it up, and
//! the mana bar's slot fills with the revive; a portrait under a fifth of its health blinks red.

use bevy::prelude::*;

use super::{
    blink, class_color, class_icon, in_run, Member, PartyView, DISABLED, HEALTH_GREEN, MAX_ALLIES,
    PARTY_FRAME, REVIVE_YELLOW, SHIELD_TINT, STATUS_BAR, TRACK,
};
use crate::survivors::hud::{self, ArtDone, HudFont, IconDone, SpellIcon, UiArt};
use crate::survivors::Phase;

/// `PartyMemberFrameTemplate`'s size.
const FRAME_W: f32 = 128.0;
const FRAME_H: f32 = 53.0;
/// `PartyMemberFrame1` sits at (10, -128) on `UIParent`; the build's icon rows hold the top left
/// here, so the column starts under them.
const LEFT: f32 = 10.0;
const TOP: f32 = 150.0;
/// One frame to the next: the pet frame under each (TOPLEFT (20, -47), 26 tall) and the next
/// frame's anchor 10 under that.
const STRIDE: f32 = 47.0 + 26.0 + 10.0;
/// The portrait, 37 square at (7, -6), and the frame's art, 128×64 at (0, -2).
const PORTRAIT_X: f32 = 7.0;
const PORTRAIT_Y: f32 = 6.0;
const PORTRAIT: f32 = 37.0;
const ART_Y: f32 = 2.0;
/// The health and mana bars, 70×8 at (47, -12) and (47, -21).
const BAR_X: f32 = 47.0;
const HEALTH_Y: f32 = 12.0;
const MANA_Y: f32 = 21.0;
const BAR_W: f32 = 70.0;
const BAR_H: f32 = 8.0;
/// The name's BOTTOMLEFT at (50, 43) from the frame's; the level right-aligned over the bars'
/// right end.
const NAME_X: f32 = 50.0;
const NAME_BOTTOM: f32 = 43.0;
const LEVEL_RIGHT: f32 = FRAME_W - BAR_X - BAR_W;
/// `PartyBuffButtonTemplate`: 15 square from (48, -32), 2 apart, four to a frame.
const BUFF_X: f32 = 48.0;
const BUFF_Y: f32 = 32.0;
const BUFF: f32 = 15.0;
const BUFF_GAP: f32 = 2.0;
const MAX_BUFFS: usize = 4;
/// "Choosing" under the buffs, in the pet frame's room.
const CHOOSING_Y: f32 = BUFF_Y + BUFF + 1.0;
/// Over the level-up's shade, so who is still choosing reads through it; under the Character
/// panel (425), pause and the results.
const Z: i32 = 422;

/// `UnitIsDead`'s and `UnitIsGhost`'s portrait tints.
const DEAD_TINT: Color = Color::srgb(0.35, 0.35, 0.35);
const GHOST_TINT: Color = Color::srgb(0.2, 0.2, 0.75);

#[derive(Component)]
pub(super) struct FramesRoot;

/// One region of frame `slot`.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct FramePart {
    slot: usize,
    part: Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Slot,
    /// The class whose icon it shows, once one is set.
    Portrait(Option<u8>),
    HealthFill,
    ShieldFill,
    /// The mana bar's slot, shown while the ally is down, and the revive filling it.
    ReviveTrack,
    ReviveFill,
    Buff(usize),
}

/// One text of frame `slot`.
#[derive(Component, Clone, Copy, Debug)]
pub(super) struct FrameLabel {
    slot: usize,
    line: Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Line {
    Name,
    Level,
    Health,
    Revive,
    Choosing,
}

/// The health bar's text: `TextStatusBar`'s "current / max", and `DEAD` at none.
pub(super) fn health_text(m: &Member) -> String {
    if m.downed {
        "Dead".to_string()
    } else {
        format!("{} / {}", m.hp.ceil() as u32, m.max_hp.round() as u32)
    }
}

/// The portrait's tint, `t` seconds in: `PartyMemberHealthCheck`'s grey for the dead and blue
/// for a ghost (here, a downed ally an ally is getting up), and red blinking under a fifth of
/// its health.
pub(super) fn portrait_tint(m: &Member, t: f32) -> Color {
    let health = m.health();
    if m.downed {
        if m.revive > 0.0 {
            GHOST_TINT
        } else {
            DEAD_TINT
        }
    } else if health > 0.0 && health <= 0.2 {
        Color::srgba(1.0, 0.0, 0.0, blink(t))
    } else {
        Color::WHITE
    }
}

fn absolute(left: f32, top: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(top),
        width: Val::Px(width),
        height: Val::Px(height),
        ..default()
    }
}

fn unpainted() -> ImageNode {
    ImageNode {
        color: Color::NONE,
        ..default()
    }
}

/// A label `size` units high, its node `node`.
fn label(f: &Handle<Font>, size: f32, color: Color, node: Node) -> impl Bundle {
    (
        hud::text_bundle(f, "", size, color),
        TextLayout::new(Justify::Left, LineBreak::NoWrap),
        node,
    )
}

/// A bar's fill: the status bar art, tinted.
fn fill(part: FramePart, tint: Color) -> impl Bundle {
    (
        part,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(0.0),
            height: Val::Percent(100.0),
            ..default()
        },
        unpainted(),
        UiArt::tinted(STATUS_BAR, tint),
    )
}

/// A bar's text, centred on it.
fn bar_text(f: &Handle<Font>, slot: usize, line: Line) -> impl Bundle {
    (
        FrameLabel { slot, line },
        hud::text_bundle(f, "", 8.0, Color::WHITE),
        TextLayout::new(Justify::Center, LineBreak::NoWrap),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(-2.0),
            width: Val::Percent(100.0),
            ..default()
        },
    )
}

/// The frames, hidden until a party fills them.
pub(super) fn setup(mut commands: Commands, font: Res<HudFont>) {
    let f = &font.0;
    commands
        .spawn((
            FramesRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(LEFT),
                top: Val::Px(TOP),
                width: Val::Px(FRAME_W),
                height: Val::Px(STRIDE * MAX_ALLIES as f32),
                ..default()
            },
            GlobalZIndex(Z),
            Visibility::Hidden,
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|root| {
            for slot in 0..MAX_ALLIES {
                let part = |part| FramePart { slot, part };
                root.spawn((
                    part(Part::Slot),
                    Node {
                        display: Display::None,
                        ..absolute(0.0, STRIDE * slot as f32, FRAME_W, FRAME_H)
                    },
                ))
                .with_children(|frame| {
                    frame.spawn((
                        part(Part::Portrait(None)),
                        Node {
                            border_radius: BorderRadius::all(Val::Percent(50.0)),
                            ..absolute(PORTRAIT_X, PORTRAIT_Y, PORTRAIT, PORTRAIT)
                        },
                        BackgroundColor(TRACK),
                        unpainted(),
                    ));
                    frame.spawn((
                        absolute(0.0, ART_Y, 128.0, 64.0),
                        unpainted(),
                        UiArt::crop(PARTY_FRAME, [0.0, 1.0, 0.0, 1.0]),
                    ));
                    frame
                        .spawn((
                            absolute(BAR_X, HEALTH_Y, BAR_W, BAR_H),
                            BackgroundColor(TRACK),
                        ))
                        .with_children(|bar| {
                            bar.spawn(fill(part(Part::HealthFill), HEALTH_GREEN));
                            bar.spawn(fill(part(Part::ShieldFill), SHIELD_TINT));
                            bar.spawn(bar_text(f, slot, Line::Health));
                        });
                    frame
                        .spawn((
                            part(Part::ReviveTrack),
                            Node {
                                display: Display::None,
                                ..absolute(BAR_X, MANA_Y, BAR_W, BAR_H)
                            },
                            BackgroundColor(TRACK),
                        ))
                        .with_children(|bar| {
                            bar.spawn(fill(part(Part::ReviveFill), REVIVE_YELLOW));
                            bar.spawn(bar_text(f, slot, Line::Revive));
                        });
                    frame.spawn((
                        FrameLabel {
                            slot,
                            line: Line::Name,
                        },
                        label(
                            f,
                            10.0,
                            Color::WHITE,
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(NAME_X),
                                bottom: Val::Px(NAME_BOTTOM),
                                max_width: Val::Px(BAR_W - 16.0),
                                overflow: Overflow::clip_x(),
                                ..default()
                            },
                        ),
                    ));
                    frame.spawn((
                        FrameLabel {
                            slot,
                            line: Line::Level,
                        },
                        label(
                            f,
                            10.0,
                            hud::GOLD_TONE,
                            Node {
                                position_type: PositionType::Absolute,
                                right: Val::Px(LEVEL_RIGHT),
                                bottom: Val::Px(NAME_BOTTOM),
                                ..default()
                            },
                        ),
                    ));
                    for i in 0..MAX_BUFFS {
                        let left = BUFF_X + i as f32 * (BUFF + BUFF_GAP);
                        frame.spawn((
                            part(Part::Buff(i)),
                            Node {
                                display: Display::None,
                                ..absolute(left, BUFF_Y, BUFF, BUFF)
                            },
                            unpainted(),
                        ));
                    }
                    frame.spawn((
                        FrameLabel {
                            slot,
                            line: Line::Choosing,
                        },
                        label(
                            f,
                            9.0,
                            hud::GOLD_TONE,
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(NAME_X),
                                top: Val::Px(CHOOSING_Y),
                                display: Display::None,
                                ..default()
                            },
                        ),
                    ));
                });
            }
        });
}

fn display(on: bool) -> Display {
    if on {
        Display::Flex
    } else {
        Display::None
    }
}

/// Each ally's frame from [`PartyView`], every frame.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn refresh(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    view: Res<PartyView>,
    time: Res<Time<Real>>,
    mut root: Query<&mut Visibility, With<FramesRoot>>,
    mut parts: Query<
        (
            Entity,
            &mut FramePart,
            &mut Node,
            Option<&mut ImageNode>,
            Has<ArtDone>,
            Option<&SpellIcon>,
        ),
        Without<FrameLabel>,
    >,
    mut labels: Query<(&FrameLabel, &mut Text, &mut TextColor, &mut Node), Without<FramePart>>,
) {
    let show = view.coop() && in_run(*phase.get());
    if let Ok(mut v) = root.single_mut() {
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *v != want {
            *v = want;
        }
    }
    let allies = view.allies();
    let ally = |slot: usize| allies.get(slot).copied().filter(|_| show);
    let t = time.elapsed_secs();
    for (e, mut part, mut node, img, ready, icon) in &mut parts {
        let m = ally(part.slot);
        let want_display = match part.part {
            Part::Slot => Some(display(m.is_some())),
            Part::ReviveTrack => Some(display(m.is_some_and(|m| m.downed))),
            Part::Buff(i) => {
                let spell = m.and_then(|m| m.buffs.get(i).copied());
                if let Some(spell) = spell {
                    if icon.is_none_or(|s| s.0 != spell) {
                        commands
                            .entity(e)
                            .remove::<IconDone>()
                            .insert(SpellIcon(spell));
                    }
                }
                Some(display(spell.is_some()))
            }
            _ => None,
        };
        if let Some(want) = want_display {
            if node.display != want {
                node.display = want;
            }
        }
        let Some(m) = m else { continue };
        let (width, tint) = match part.part {
            Part::Portrait(class) => {
                if class != Some(m.class) {
                    part.part = Part::Portrait(Some(m.class));
                    commands
                        .entity(e)
                        .remove::<ArtDone>()
                        .insert(class_icon(m.class));
                    continue;
                }
                (None, Some(portrait_tint(m, t)))
            }
            Part::HealthFill => (
                Some(if m.downed { 0.0 } else { m.health() }),
                Some(HEALTH_GREEN),
            ),
            Part::ShieldFill => (Some(if m.downed { 0.0 } else { m.shield_share() }), None),
            Part::ReviveFill => (Some(m.revive), None),
            _ => (None, None),
        };
        if let Some(w) = width {
            let w = Val::Percent(w * 100.0);
            if node.width != w {
                node.width = w;
            }
        }
        if let (Some(tint), Some(mut img), true) = (tint, img, ready) {
            if img.color != tint {
                img.color = tint;
            }
        }
    }
    for (label, mut text, mut color, mut node) in &mut labels {
        let Some(m) = ally(label.slot) else { continue };
        let (want, tone, on) = match label.line {
            Line::Name => (
                m.name.clone(),
                if m.downed {
                    DISABLED
                } else {
                    class_color(m.class)
                },
                true,
            ),
            Line::Level => (
                m.level.to_string(),
                if m.downed { DISABLED } else { hud::GOLD_TONE },
                true,
            ),
            Line::Health => (health_text(m), Color::WHITE, true),
            Line::Revive => (
                if m.revive > 0.0 { "Reviving" } else { "Downed" }.to_string(),
                Color::WHITE,
                true,
            ),
            Line::Choosing => (
                "Choosing...".to_string(),
                hud::GOLD_TONE.with_alpha(blink(t)),
                m.choosing && !m.downed,
            ),
        };
        if **text != want {
            **text = want;
        }
        if color.0 != tone {
            color.0 = tone;
        }
        let want = display(on);
        if node.display != want {
            node.display = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::tests::{member, view_of};
    use super::*;

    #[test]
    fn a_downed_ally_reads_dead_grey_then_ghost_as_it_is_revived() {
        let mut m = member(1, "Mage");
        (m.hp, m.max_hp) = (75.4, 150.0);
        assert_eq!(health_text(&m), "76 / 150");
        assert_eq!(portrait_tint(&m, 0.0), Color::WHITE);
        // Under a fifth: red, blinking.
        m.hp = 20.0;
        let low = portrait_tint(&m, 0.0);
        assert_eq!(low.with_alpha(1.0), Color::srgb(1.0, 0.0, 0.0));
        assert!(low.alpha() < portrait_tint(&m, 0.45).alpha());
        m.downed = true;
        m.hp = 0.0;
        assert_eq!(health_text(&m), "Dead");
        assert_eq!(portrait_tint(&m, 0.0), DEAD_TINT);
        m.revive = 0.3;
        assert_eq!(portrait_tint(&m, 0.0), GHOST_TINT);
    }

    fn frames_world(view: PartyView, phase: Phase) -> World {
        let mut world = World::new();
        world.insert_resource(HudFont(Handle::default()));
        world.insert_resource(State::new(phase));
        world.init_resource::<Time<Real>>();
        world.insert_resource(view);
        world.run_system_once(setup).unwrap();
        world.run_system_once(refresh).unwrap();
        world
    }

    /// Each frame's slot: whether it is laid out.
    fn shown(world: &mut World) -> Vec<bool> {
        let mut slots: Vec<(usize, bool)> = world
            .query::<(&FramePart, &Node)>()
            .iter(world)
            .filter(|(p, _)| p.part == Part::Slot)
            .map(|(p, n)| (p.slot, n.display != Display::None))
            .collect();
        slots.sort();
        slots.into_iter().map(|(_, on)| on).collect()
    }

    fn root_visible(world: &mut World) -> bool {
        let mut q = world.query_filtered::<&Visibility, With<FramesRoot>>();
        *q.single(world).unwrap() != Visibility::Hidden
    }

    #[test]
    fn frames_show_one_per_ally_only_in_a_party_run() {
        // Alone: nothing, as the screen always was.
        let mut world = frames_world(view_of(vec![member(0, "Warrior")]), Phase::Playing);
        assert_eq!(shown(&mut world), [false, false, false, false]);
        assert!(!root_visible(&mut world));
        // Three heroes: two allies' frames, the local hero's none.
        let party = view_of(vec![
            member(0, "Warrior"),
            member(1, "Priest"),
            member(2, "Rogue"),
        ]);
        let mut world = frames_world(party.clone(), Phase::Playing);
        assert_eq!(shown(&mut world), [true, true, false, false]);
        assert!(root_visible(&mut world));
        // The names, in class colour.
        let names: Vec<(usize, String, Color)> = {
            let mut q = world.query::<(&FrameLabel, &Text, &TextColor)>();
            let mut v: Vec<_> = q
                .iter(&world)
                .filter(|(l, ..)| l.line == Line::Name)
                .map(|(l, t, c)| (l.slot, t.0.clone(), c.0))
                .collect();
            v.sort_by_key(|(slot, ..)| *slot);
            v
        };
        assert_eq!(names[0].1, "Player 2");
        assert_eq!(names[0].2, class_color(5));
        assert_eq!(names[1].1, "Player 3");
        // Not on the results, nor at the menu.
        for phase in [Phase::GameOver, Phase::Menu] {
            let mut world = frames_world(party.clone(), phase);
            assert!(!root_visible(&mut world), "{phase:?}");
            assert_eq!(shown(&mut world), [false, false, false, false]);
        }
    }

    #[test]
    fn a_frame_shows_its_buffs_and_its_revive_while_down() {
        let mut ally = member(1, "Priest");
        ally.buffs = vec![17, 139];
        let mut world = frames_world(
            view_of(vec![member(0, "Warrior"), ally.clone()]),
            Phase::Playing,
        );
        let mut q = world.query::<(&FramePart, &Node, Option<&SpellIcon>)>();
        let mut buffs = [(false, None); MAX_BUFFS];
        for (p, n, icon) in q.iter(&world) {
            if let (0, Part::Buff(i)) = (p.slot, p.part) {
                buffs[i] = (n.display != Display::None, icon.map(|s| s.0));
            }
        }
        assert_eq!(
            buffs,
            [
                (true, Some(17)),
                (true, Some(139)),
                (false, None),
                (false, None)
            ]
        );
        let revive_track = |world: &mut World| {
            let mut q = world.query::<(&FramePart, &Node)>();
            q.iter(world)
                .find(|(p, _)| p.slot == 0 && p.part == Part::ReviveTrack)
                .map(|(_, n)| n.display != Display::None)
                .unwrap()
        };
        assert!(!revive_track(&mut world));
        ally.downed = true;
        ally.revive = 0.5;
        world.insert_resource(view_of(vec![member(0, "Warrior"), ally]));
        world.run_system_once(refresh).unwrap();
        assert!(revive_track(&mut world));
        let mut q = world.query::<(&FramePart, &Node)>();
        let width = q
            .iter(&world)
            .find(|(p, _)| p.slot == 0 && p.part == Part::ReviveFill)
            .map(|(_, n)| n.width)
            .unwrap();
        assert_eq!(width, Val::Percent(50.0));
    }
}
