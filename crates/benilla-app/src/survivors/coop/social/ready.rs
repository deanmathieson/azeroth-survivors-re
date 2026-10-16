//! The ready check. In the lobby the host's Start is a **Ready Check**: everyone else gets 1.12's
//! ready-check pop-up (`UI-ReadyCheckFrame`, "<Host> has initiated a ready check. Are you
//! ready?") with the `ReadyCheck` sound, and the lobby marks each player ready, not ready or
//! waiting. The run starts as soon as everyone is ready; the host may start it anyway; after
//! [`READY_SECS`] whoever has not answered counts as not ready and the check ends.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::super::hud::HudFont;
use super::super::super::Phase;
use super::super::host::Peer;
use super::super::{Link, PlayerName, Session};
use super::wire::{Body, Heard, Line, ReadyView, Say, Speech};
use super::{label, name_of, player_of, to_all, Art};
use crate::glue::art::GlueArt;
use crate::glue::widgets::{glue_button, GlueBtnKind};
use crate::glue::GlueClicks;
use crate::sound::SoundCue;

/// Seconds the check waits for answers.
pub(super) const READY_SECS: f32 = 30.0;
/// The pop-up's frame, cropped as `ReadyCheckFrame` draws it.
pub(super) const FRAME: &str = "Interface\\RaidFrame\\UI-ReadyCheckFrame";
const FRAME_TC: [f32; 4] = [0.0, 0.630_859_4, 0.0, 0.757_812_5];
/// The lobby's marks: 1.12 has no ready-check icons, so the check box's tick, the group loot's red
/// cross and the pocket watch stand in.
pub(super) const READY_ICON: &str = "Interface\\Buttons\\UI-CheckBox-Check";
pub(super) const NOT_READY_ICON: &str = "Interface\\Buttons\\UI-GroupLoot-Pass-Up";
pub(super) const WAITING_ICON: &str = "Interface\\Icons\\INV_Misc_PocketWatch_01";
/// `PlaySound("ReadyCheck")`.
pub(super) const READY_SOUND: &str = "ReadyCheck";

/// One player's answer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Mark {
    Waiting,
    Ready,
    NotReady,
}

impl Mark {
    /// Its icon in the lobby.
    pub(super) fn art(self) -> Art {
        match self {
            Self::Ready => Art::whole(READY_ICON),
            Self::NotReady => Art::whole(NOT_READY_ICON),
            Self::Waiting => Art::crop(WAITING_ICON, [0.07, 0.93, 0.07, 0.93]),
        }
    }
}

/// A ready check under way, on the host.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReadyCheck {
    pub by: String,
    pub marks: Vec<(u8, Mark)>,
    pub left: f32,
}

impl ReadyCheck {
    /// A check `by` the player `initiator`, who is ready, among `players`.
    pub(super) fn start(by: &str, players: &[u8], initiator: u8) -> Self {
        Self {
            by: by.to_string(),
            marks: players
                .iter()
                .map(|&p| {
                    (
                        p,
                        if p == initiator {
                            Mark::Ready
                        } else {
                            Mark::Waiting
                        },
                    )
                })
                .collect(),
            left: READY_SECS,
        }
    }

    /// `player` answers; whether anything changed.
    pub(super) fn answer(&mut self, player: u8, ready: bool) -> bool {
        let want = if ready { Mark::Ready } else { Mark::NotReady };
        match self.marks.iter_mut().find(|m| m.0 == player) {
            Some(m) if m.1 != want => {
                m.1 = want;
                true
            }
            _ => false,
        }
    }

    /// The party changed: a newcomer waits, the gone are dropped. Whether anything changed.
    pub(super) fn sync(&mut self, players: &[u8]) -> bool {
        let before = self.marks.len();
        self.marks.retain(|m| players.contains(&m.0));
        let mut changed = self.marks.len() != before;
        for &p in players {
            if !self.marks.iter().any(|m| m.0 == p) {
                self.marks.push((p, Mark::Waiting));
                changed = true;
            }
        }
        changed
    }

    pub(super) fn all_ready(&self) -> bool {
        self.marks.iter().all(|m| m.1 == Mark::Ready)
    }

    /// `dt` on; at the end of the time every unanswered player is not ready, and `true` says it
    /// ended now.
    pub(super) fn tick(&mut self, dt: f32) -> bool {
        let was = self.left > 0.0;
        self.left -= dt;
        if was && self.left <= 0.0 {
            for m in &mut self.marks {
                if m.1 == Mark::Waiting {
                    m.1 = Mark::NotReady;
                }
            }
            return true;
        }
        false
    }

    pub(super) fn not_ready(&self) -> Vec<u8> {
        self.marks
            .iter()
            .filter(|m| m.1 != Mark::Ready)
            .map(|m| m.0)
            .collect()
    }

    pub(super) fn view(&self, open: bool) -> ReadyView {
        ReadyView {
            open,
            by: self.by.clone(),
            marks: self.marks.clone(),
            left: self.left.max(0.0),
        }
    }
}

/// The host's check, and what it last told everyone.
#[derive(Resource, Default)]
pub(super) struct HostReady {
    check: Option<ReadyCheck>,
    /// Start the run once the frame's systems are done.
    go: bool,
    sent: Option<ReadyView>,
}

/// The check as this machine last heard it, and whether this player has answered.
#[derive(Resource, Default)]
pub(in crate::survivors) struct ReadyShown {
    pub(in crate::survivors) view: ReadyView,
    answered: bool,
}

impl ReadyShown {
    /// Player `player`'s mark in the check under way, if one is.
    pub(super) fn mark(&self, player: u8) -> Option<Mark> {
        self.view
            .open
            .then(|| self.view.marks.iter().find(|m| m.0 == player).map(|m| m.1))
            .flatten()
    }
}

#[derive(Component)]
struct Popup;
#[derive(Component)]
struct PopupText;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum ReadyAction {
    Ready,
    NotReady,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HostReady>()
        .init_resource::<ReadyShown>()
        .add_systems(
            Update,
            (
                host_ready.run_if(super::hosting),
                hear_ready,
                popup,
                popup_clicks,
            )
                .chain(),
        );
}

/// The host's Ready Check: everyone is asked, the host ready already. Alone, the run starts at once.
pub(in crate::survivors) fn begin_check(world: &mut World) {
    let session = world.resource::<Session>();
    if !matches!(session.link, Link::Hosting { .. }) || session.in_run {
        return;
    }
    let mut players: Vec<u8> = session.lobby.iter().map(|m| m.player).collect();
    if !players.contains(&0) {
        players.insert(0, 0);
    }
    let by = world.resource::<PlayerName>().0.clone();
    let check = ReadyCheck::start(&by, &players, 0);
    let mut host = world.resource_mut::<HostReady>();
    if check.all_ready() {
        host.check = None;
        host.go = true;
    } else {
        host.check = Some(check);
    }
}

/// The host starts the run whoever is ready.
pub(in crate::survivors) fn force_start(world: &mut World) {
    let mut host = world.resource_mut::<HostReady>();
    host.check = None;
    host.go = true;
}

/// The host keeps its check: answers in, the party's comings and goings, the clock; it tells
/// everyone each change, and starts the run once everyone is ready.
#[allow(clippy::too_many_arguments)]
fn host_ready(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut host: ResMut<HostReady>,
    mut said: MessageReader<FromClient<Say>>,
    peers: Query<&Peer>,
    session: Res<Session>,
    phase: Res<State<Phase>>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    if session.in_run || *phase.get() != Phase::Menu {
        host.check = None;
        host.go = false;
    }
    let players: Vec<u8> = session.lobby.iter().map(|m| m.player).collect();
    for m in said.read() {
        let Speech::Ready(ready) = m.message.0 else {
            continue;
        };
        let Some(player) = player_of(m.client_id, &peers) else {
            continue;
        };
        if let Some(check) = host.check.as_mut() {
            check.answer(player, ready);
        }
    }
    let mut ended = None;
    if let Some(check) = host.check.as_mut() {
        if !players.is_empty() {
            check.sync(&players);
        }
        if check.tick(time.delta_secs()) {
            ended = Some(check.not_ready());
        } else if check.all_ready() {
            host.go = true;
        }
    }
    if let Some(not_ready) = ended {
        let names: Vec<String> = not_ready.iter().map(|&p| name_of(&session, p)).collect();
        out.write(to_all(Heard::Line(Line {
            player: 0,
            name: String::new(),
            body: Body::Note(format!("Ready check over. Not ready: {}", names.join(", "))),
        })));
        host.check = None;
    }
    if host.go {
        host.check = None;
    }
    let view = match &host.check {
        Some(c) => Some(c.view(true)),
        None => host.sent.as_ref().filter(|v| v.open).map(|v| ReadyView {
            open: false,
            ..v.clone()
        }),
    };
    // Each change goes out, and the clock every whole second.
    if let Some(view) = view {
        let stale = host.sent.as_ref().is_none_or(|s| {
            s.open != view.open || s.marks != view.marks || s.left.ceil() != view.left.ceil()
        });
        if stale {
            host.sent = Some(view.clone());
            out.write(to_all(Heard::Ready(view)));
        }
    }
    if host.go {
        host.go = false;
        if !session.in_run {
            out.write(to_all(Heard::Line(Line {
                player: 0,
                name: String::new(),
                body: Body::Note("Everyone is ready. Here we go!".into()),
            })));
            commands.queue(|world: &mut World| {
                if let Err(e) = world.run_system_cached(super::super::ui::start_now) {
                    warn!("survivors: co-op: cannot start the run: {e}");
                }
            });
        }
    }
}

/// The check as the host tells it: a new one rings the ready-check bell.
fn hear_ready(
    mut heard: MessageReader<Heard>,
    mut shown: ResMut<ReadyShown>,
    session: Res<Session>,
    mut cues: MessageWriter<SoundCue>,
) {
    for h in heard.read() {
        let Heard::Ready(view) = h else { continue };
        if view.open && !shown.view.open {
            shown.answered = false;
            cues.write(SoundCue::flat(READY_SOUND));
        }
        shown.view = view.clone();
    }
    if session.link == Link::Off && shown.view.open {
        *shown = ReadyShown::default();
    }
}

/// The pop-up, for a player the check is waiting on.
fn popup(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    art: Res<GlueArt>,
    shown: Res<ReadyShown>,
    session: Res<Session>,
    mut popups: Query<&mut Visibility, With<Popup>>,
    mut text: Query<&mut Text, With<PopupText>>,
) {
    let Some(font) = font else { return };
    if popups.is_empty() {
        spawn_popup(&mut commands, &font.0, &art);
        return;
    }
    let waiting = shown.mark(session.me) == Some(Mark::Waiting) && !shown.answered;
    let want = if waiting {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut popups {
        v.set_if_neq(want);
    }
    let line = format!(
        "{} has initiated a ready check.\nAre you ready? ({}s)",
        shown.view.by,
        shown.view.left.ceil() as u32
    );
    for mut t in &mut text {
        if t.0 != line {
            t.0 = line.clone();
        }
    }
}

fn spawn_popup(commands: &mut Commands, font: &Handle<Font>, art: &GlueArt) {
    let px = Val::Px;
    commands
        .spawn((
            Popup,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(18.0),
                margin: UiRect::left(px(-161.0)),
                width: px(323.0),
                height: px(97.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::new(px(70.0), px(14.0), px(8.0), px(8.0)),
                row_gap: px(6.0),
                ..default()
            },
            GlobalZIndex(420),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.0),
                    top: px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                Art::crop(FRAME, FRAME_TC),
            ));
            p.spawn((
                PopupText,
                label(font, "", 13.0, Color::WHITE),
                TextShadow::default(),
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(8.0),
                ..default()
            })
            .with_children(|r| {
                glue_button(
                    r,
                    art,
                    font,
                    ReadyAction::Ready,
                    "Ready",
                    100.0,
                    26.0,
                    GlueBtnKind::Dialog,
                    1.0,
                );
                glue_button(
                    r,
                    art,
                    font,
                    ReadyAction::NotReady,
                    "Not Ready",
                    100.0,
                    26.0,
                    GlueBtnKind::Dialog,
                    1.0,
                );
            });
        });
}

/// Ready or Not Ready: the answer goes to the host.
fn popup_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &ReadyAction)>,
    mut shown: ResMut<ReadyShown>,
    mut say: MessageWriter<Say>,
) {
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    shown.answered = true;
    say.write(Say(Speech::Ready(action == ReadyAction::Ready)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_run_starts_when_everyone_is_ready() {
        let mut c = ReadyCheck::start("Host", &[0, 1, 2], 0);
        assert_eq!(c.marks[0], (0, Mark::Ready), "the host is ready already");
        assert!(!c.all_ready());
        assert!(c.answer(1, true));
        assert!(!c.answer(1, true), "the same answer twice changes nothing");
        assert!(!c.all_ready());
        assert!(c.answer(2, false));
        assert_eq!(c.not_ready(), vec![2]);
        assert!(c.answer(2, true), "a mind may change");
        assert!(c.all_ready());
        assert!(!c.answer(9, true), "not in the party");
    }

    #[test]
    fn the_party_changing_mid_check_is_followed() {
        let mut c = ReadyCheck::start("Host", &[0, 1], 0);
        c.answer(1, true);
        assert!(c.all_ready());
        assert!(c.sync(&[0, 1, 3]), "a newcomer joins the check");
        assert!(!c.all_ready(), "and is waited on");
        assert!(c.sync(&[0, 1]), "they left again");
        assert!(c.all_ready());
        assert!(!c.sync(&[0, 1]));
    }

    #[test]
    fn time_up_makes_the_silent_not_ready() {
        let mut c = ReadyCheck::start("Host", &[0, 1, 2], 0);
        c.answer(1, true);
        assert!(!c.tick(READY_SECS - 1.0));
        assert!(c.tick(2.0), "the check ends now");
        assert!(!c.tick(1.0), "and only once");
        assert_eq!(c.marks[2], (2, Mark::NotReady));
        assert_eq!(c.not_ready(), vec![2]);
        let v = c.view(true);
        assert!(v.open && v.left == 0.0 && v.by == "Host");
    }

    #[test]
    fn alone_the_check_is_over_at_once() {
        assert!(ReadyCheck::start("Host", &[0], 0).all_ready());
    }

    #[test]
    fn the_shown_check_marks_each_player() {
        let mut shown = ReadyShown::default();
        assert_eq!(shown.mark(1), None, "no check");
        shown.view = ReadyCheck::start("Host", &[0, 1], 0).view(true);
        assert_eq!(shown.mark(1), Some(Mark::Waiting));
        assert_eq!(shown.mark(0), Some(Mark::Ready));
        shown.view.open = false;
        assert_eq!(shown.mark(1), None);
    }
}
