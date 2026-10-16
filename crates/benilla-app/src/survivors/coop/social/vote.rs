//! The fashion vote, and the party's panel on the results screen. After a co-op run everyone sees
//! each hero's card (class, name, title, mount, hat and tabard) and votes for the best-dressed,
//! never themselves. Once everyone has voted, or after [`VOTE_SECS`], the host counts: the most
//! votes win (a tie shares it), and each winner's own machine pays it [`FASHION_TICKETS`] tickets
//! and notes "Fashionista" on the results. The same panel lists how the side bets paid.

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy_replicon::prelude::*;

use super::super::super::hud::{HudFont, NoteKind, ResultsNotes};
use super::super::super::profile::Profile;
use super::super::super::Phase;
use super::super::host::Peer;
use super::super::Session;
use super::bets::{payout_line, Settled};
use super::lobby::HostBook;
use super::wire::{Card, Heard, Say, Speech, VoteResult};
use super::{class_icon, label, name_of, player_of, to_all, Art, CLASS_ICONS, TICKET};
use crate::glue::art::GOLD;
use crate::glue::GlueClicks;
use crate::sound::SoundCue;

/// Seconds the vote stays open.
pub(super) const VOTE_SECS: f32 = 30.0;
/// The best-dressed's prize.
pub(super) const FASHION_TICKETS: u32 = 20;
/// The vote opening, and the winner's cheer.
pub(super) const VOTE_SOUND: &str = "igMainMenuOptionCheckBoxOn";
pub(super) const FASHIONISTA_SOUND: &str = "QUESTCOMPLETED";

/// Count `votes` (voter, for) among `players`: a vote for oneself or for anyone not voting here
/// does not count, and a voter's last vote is its vote. The most votes win, ties all of them; no
/// votes, no winner.
pub(super) fn tally(votes: &[(u8, u8)], players: &[u8]) -> VoteResult {
    let mut last: BTreeMap<u8, u8> = BTreeMap::new();
    for &(voter, choice) in votes {
        if voter != choice && players.contains(&voter) && players.contains(&choice) {
            last.insert(voter, choice);
        }
    }
    let mut count: BTreeMap<u8, u32> = players.iter().map(|&p| (p, 0)).collect();
    for choice in last.values() {
        *count.entry(*choice).or_default() += 1;
    }
    let best = count.values().copied().max().unwrap_or(0);
    VoteResult {
        winners: if best == 0 {
            Vec::new()
        } else {
            count
                .iter()
                .filter(|(_, &n)| n == best)
                .map(|(&p, _)| p)
                .collect()
        },
        tally: count.into_iter().collect(),
    }
}

/// The vote under way, on the host.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Vote {
    players: Vec<u8>,
    votes: Vec<(u8, u8)>,
    left: f32,
}

impl Vote {
    pub(super) fn new(players: &[u8]) -> Self {
        Self {
            players: players.to_vec(),
            votes: Vec::new(),
            left: VOTE_SECS,
        }
    }

    pub(super) fn cast(&mut self, voter: u8, choice: u8) {
        self.votes.push((voter, choice));
    }

    pub(super) fn tick(&mut self, dt: f32) {
        self.left -= dt;
    }

    /// Time is up, or every player has a vote that counts.
    pub(super) fn done(&self) -> bool {
        self.left <= 0.0
            || self.players.iter().all(|p| {
                self.votes
                    .iter()
                    .any(|&(v, c)| v == *p && c != *p && self.players.contains(&c))
            })
    }

    pub(super) fn result(&self) -> VoteResult {
        tally(&self.votes, &self.players)
    }
}

/// The host's vote.
#[derive(Resource, Default)]
struct HostVote(Option<Vote>);

/// The vote as this machine sees it.
#[derive(Resource, Default)]
struct VoteShown {
    open: bool,
    cards: Vec<Card>,
    left: f32,
    mine: Option<u8>,
    result: Option<VoteResult>,
    version: u32,
}

#[derive(Component)]
struct PartyPanel;
#[derive(Component)]
struct PanelBody;
#[derive(Component, Clone, Copy)]
struct VoteButton(u8);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HostVote>()
        .init_resource::<VoteShown>()
        .add_systems(OnEnter(Phase::GameOver), open_vote.run_if(super::hosting))
        .add_systems(
            Update,
            (
                host_vote.run_if(super::hosting),
                hear_vote,
                vote_clicks,
                party_panel,
            )
                .chain(),
        )
        .add_systems(OnEnter(Phase::Menu), close_vote);
}

/// A co-op run is over: the host opens the vote on everyone's card.
fn open_vote(
    session: Res<Session>,
    book: Res<HostBook>,
    mut vote: ResMut<HostVote>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    if session.lobby.len() < 2 {
        return;
    }
    let players: Vec<u8> = session.lobby.iter().map(|m| m.player).collect();
    let cards = session
        .lobby
        .iter()
        .map(|m| {
            book.cards.get(&m.player).cloned().unwrap_or_else(|| Card {
                player: m.player,
                name: m.name.clone(),
                class: m.class,
                level: 1,
                ..Default::default()
            })
        })
        .collect();
    vote.0 = Some(Vote::new(&players));
    out.write(to_all(Heard::VoteOpen {
        cards,
        secs: VOTE_SECS,
    }));
}

/// The host takes the votes and counts them once everyone has voted or the time is up.
fn host_vote(
    time: Res<Time<Real>>,
    mut vote: ResMut<HostVote>,
    mut said: MessageReader<FromClient<Say>>,
    peers: Query<&Peer>,
    mut out: MessageWriter<ToClients<Heard>>,
) {
    for m in said.read() {
        let Speech::Vote(choice) = m.message.0 else {
            continue;
        };
        let (Some(voter), Some(v)) = (player_of(m.client_id, &peers), vote.0.as_mut()) else {
            continue;
        };
        v.cast(voter, choice);
    }
    let Some(v) = vote.0.as_mut() else { return };
    v.tick(time.delta_secs());
    if v.done() {
        let result = v.result();
        vote.0 = None;
        out.write(to_all(Heard::VoteResult(result)));
    }
}

/// The vote opens and is counted here as the host says; the best-dressed's own machine pays it.
fn hear_vote(
    mut heard: MessageReader<Heard>,
    time: Res<Time<Real>>,
    session: Res<Session>,
    mut shown: ResMut<VoteShown>,
    mut profile: ResMut<Profile>,
    mut notes: ResMut<ResultsNotes>,
    mut cues: MessageWriter<SoundCue>,
) {
    if shown.open {
        shown.left = (shown.left - time.delta_secs()).max(0.0);
    }
    for h in heard.read() {
        match h {
            Heard::VoteOpen { cards, secs } => {
                *shown = VoteShown {
                    open: true,
                    cards: cards.clone(),
                    left: *secs,
                    mine: None,
                    result: None,
                    version: shown.version.wrapping_add(1),
                };
                cues.write(SoundCue::flat(VOTE_SOUND));
            }
            Heard::VoteResult(result) => {
                shown.open = false;
                shown.result = Some(result.clone());
                shown.version = shown.version.wrapping_add(1);
                if result.winners.contains(&session.me) {
                    let faire = &mut profile.faire;
                    faire.tickets = faire.tickets.saturating_add(FASHION_TICKETS);
                    faire.tickets_earned = faire
                        .tickets_earned
                        .saturating_add(u64::from(FASHION_TICKETS));
                    profile.mark_dirty();
                    notes.push(
                        format!("Fashionista! +{FASHION_TICKETS} Darkmoon Prize Tickets"),
                        NoteKind::Good,
                    );
                    cues.write(SoundCue::flat(FASHIONISTA_SOUND));
                }
            }
            _ => {}
        }
    }
}

/// A vote for another hero: once, to the host.
fn vote_clicks(
    clicks: Res<GlueClicks>,
    buttons: Query<(Entity, &VoteButton)>,
    session: Res<Session>,
    mut shown: ResMut<VoteShown>,
    mut say: MessageWriter<Say>,
) {
    let Some(b) = buttons
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, b)| *b)
    else {
        return;
    };
    if !shown.open || shown.mine.is_some() || b.0 == session.me {
        return;
    }
    shown.mine = Some(b.0);
    shown.version = shown.version.wrapping_add(1);
    say.write(Say(Speech::Vote(b.0)));
}

/// The party's panel on the results screen: the fashion vote's cards and count, and the side bets'
/// payouts. Rebuilt whenever either changes.
#[allow(clippy::too_many_arguments)]
fn party_panel(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    font: Option<Res<HudFont>>,
    session: Res<Session>,
    shown: Res<VoteShown>,
    settled: Res<Settled>,
    mut panels: Query<&mut Visibility, With<PartyPanel>>,
    bodies: Query<Entity, With<PanelBody>>,
    mut built: Local<Option<(u32, bool, u32)>>,
) {
    let Some(font) = font else { return };
    if panels.is_empty() {
        commands
            .spawn((
                PartyPanel,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Percent(16.0),
                    width: Val::Px(330.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.02, 0.05, 0.88)),
                BorderColor::all(Color::srgb(0.75, 0.6, 0.25)),
                GlobalZIndex(305),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    PanelBody,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ));
            });
        return;
    }
    let any = shown.open || shown.result.is_some() || settled.0.is_some();
    let on = *phase.get() == Phase::GameOver && any;
    for mut v in &mut panels {
        v.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    let seconds = shown.left.ceil() as u32;
    let key = (shown.version, settled.0.is_some(), seconds);
    if !on || *built == Some(key) {
        return;
    }
    *built = Some(key);
    let f = &font.0;
    for body in &bodies {
        commands.entity(body).despawn_related::<Children>();
        commands.entity(body).with_children(|b| {
            if !shown.cards.is_empty() {
                let title = if shown.open {
                    format!("Fashion vote: who is best dressed? ({seconds}s)")
                } else {
                    "Fashion vote".to_string()
                };
                b.spawn(label(f, title, 15.0, GOLD));
                for card in &shown.cards {
                    fashion_row(b, f, card, &session, &shown);
                }
                if let Some(result) = &shown.result {
                    let line = if result.winners.is_empty() {
                        "No votes: everyone looked fabulous.".to_string()
                    } else {
                        let names: Vec<String> = result
                            .winners
                            .iter()
                            .map(|&p| name_of(&session, p))
                            .collect();
                        format!(
                            "Fashionista: {}! +{FASHION_TICKETS} tickets",
                            names.join(" and ")
                        )
                    };
                    b.spawn(label(f, line, 14.0, TICKET));
                }
            }
            if let Some(payouts) = &settled.0 {
                b.spawn(Node {
                    height: Val::Px(6.0),
                    ..default()
                });
                b.spawn(label(f, "Side bets", 15.0, GOLD));
                for p in payouts {
                    let line = payout_line(
                        &name_of(&session, p.player),
                        &name_of(&session, p.wager.on),
                        p,
                    );
                    let color = if p.won {
                        Color::srgb(0.1, 1.0, 0.1)
                    } else if p.refunded {
                        Color::WHITE
                    } else {
                        Color::srgb(0.6, 0.6, 0.6)
                    };
                    b.spawn(label(f, line, 12.0, color));
                }
            }
        });
    }
}

/// One hero in the vote: its class icon, name and title, what it wears, and its Vote button (none
/// for this player's own, nor once this player has voted) or its count.
fn fashion_row(
    b: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    card: &Card,
    session: &Session,
    shown: &VoteShown,
) {
    let worn: Vec<String> = [&card.mount, &card.hat, &card.tabard]
        .into_iter()
        .flatten()
        .cloned()
        .collect();
    let worn = if worn.is_empty() {
        "Nothing fancy".to_string()
    } else {
        worn.join(", ")
    };
    let name = match &card.title {
        Some(t) => super::super::super::faire::catalog::titled_name(&card.name, Some(t)),
        None => card.name.clone(),
    };
    let votes = shown
        .result
        .as_ref()
        .and_then(|r| r.tally.iter().find(|t| t.0 == card.player).map(|t| t.1));
    b.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|r| {
        r.spawn((
            Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                ..default()
            },
            Art::crop(CLASS_ICONS, class_icon(usize::from(card.class))),
        ));
        r.spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|c| {
            c.spawn(label(f, name, 13.0, Color::WHITE));
            c.spawn(label(f, worn, 11.0, Color::srgb(0.75, 0.75, 0.75)));
        });
        if let Some(n) = votes {
            let s = if n == 1 { "" } else { "s" };
            r.spawn(label(f, format!("{n} vote{s}"), 12.0, TICKET));
        } else if shown.mine == Some(card.player) {
            r.spawn(label(f, "Your vote", 12.0, TICKET));
        } else if shown.open && shown.mine.is_none() && card.player != session.me {
            r.spawn((
                VoteButton(card.player),
                Button,
                Interaction::default(),
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.25, 0.05, 0.05, 0.9)),
                BorderColor::all(GOLD),
            ))
            .with_children(|v| {
                v.spawn(label(f, "Vote", 12.0, GOLD));
            });
        }
    });
}

fn close_vote(mut vote: ResMut<HostVote>, mut shown: ResMut<VoteShown>) {
    vote.0 = None;
    let version = shown.version.wrapping_add(1);
    *shown = VoteShown {
        version,
        ..Default::default()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_votes_win_and_a_tie_shares_it() {
        let players = [0, 1, 2, 3];
        let r = tally(&[(0, 1), (1, 2), (2, 1), (3, 2)], &players);
        assert_eq!(r.winners, vec![1, 2], "two each: a tie");
        assert_eq!(r.tally, vec![(0, 0), (1, 2), (2, 2), (3, 0)]);
        let r = tally(&[(0, 1), (1, 2), (2, 1), (3, 1)], &players);
        assert_eq!(r.winners, vec![1]);
    }

    #[test]
    fn no_vote_for_oneself_and_the_last_vote_counts() {
        let players = [0, 1, 2];
        let r = tally(&[(0, 0), (1, 1), (2, 2), (5, 1), (0, 9)], &players);
        assert!(r.winners.is_empty(), "nothing counted, nobody wins");
        let r = tally(&[(0, 1), (0, 2), (1, 2)], &players);
        assert_eq!(r.winners, vec![2], "0 changed its mind");
        assert_eq!(r.tally, vec![(0, 0), (1, 0), (2, 2)]);
    }

    #[test]
    fn the_vote_closes_on_the_last_vote_or_the_clock() {
        let mut v = Vote::new(&[0, 1, 2]);
        v.cast(0, 1);
        v.cast(1, 1);
        v.cast(2, 0);
        assert!(!v.done(), "1's vote for itself does not count");
        v.cast(1, 2);
        assert!(v.done());
        let mut slow = Vote::new(&[0, 1]);
        slow.cast(0, 1);
        slow.tick(VOTE_SECS - 1.0);
        assert!(!slow.done());
        slow.tick(1.0);
        assert!(slow.done());
        assert_eq!(slow.result().winners, vec![1]);
    }
}
