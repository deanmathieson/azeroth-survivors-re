//! The campfire in a session: the camp's windows name everyone as the lobby does, and the
//! roll-off for the bonus cache is told in the party chat, "[Name] rolls 73 (1-100)", as `/roll`
//! is. Every machine tells it from the replicated camp ([`Camp`]), so the host sends no line.

use bevy::prelude::*;

use super::super::super::campfire::{Camp, CampNames};
use super::super::super::party::LOCAL_PLAYER;
use super::super::client::local_player;
use super::super::{Link, PlayerName, Session};
use super::chat::{spans, Chat};
use super::linked;
use super::wire::{Body, Line};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (camp_names, camp_rolls.run_if(linked)).chain());
}

/// The lobby's names in this machine's numbering, this player's own included.
pub(super) fn names_of(lobby: &[(u8, String)], me: u8, mine: &str) -> Vec<(u8, String)> {
    let mut out: Vec<(u8, String)> = lobby
        .iter()
        .map(|(p, n)| (local_player(*p, me), n.clone()))
        .collect();
    if !out.iter().any(|(p, _)| *p == LOCAL_PLAYER) && !mine.is_empty() {
        out.push((LOCAL_PLAYER, mine.to_string()));
    }
    out.sort();
    out
}

/// The camp's names follow the session's lobby; alone there are none.
fn camp_names(session: Res<Session>, name: Res<PlayerName>, mut names: ResMut<CampNames>) {
    let want = if matches!(session.link, Link::Hosting { .. } | Link::Joined { .. }) {
        let lobby: Vec<(u8, String)> = session
            .lobby
            .iter()
            .map(|m| (m.player, m.name.clone()))
            .collect();
        names_of(&lobby, session.me, &name.0)
    } else {
        Vec::new()
    };
    if names.0 != want {
        names.0 = want;
    }
}

/// The chat's lines for `rolls` not yet told, and the winner once there is one.
pub(super) fn roll_lines(
    rolls: &[(u8, u8)],
    told: &[(u8, u8)],
    winner: Option<u8>,
    winner_told: bool,
    names: &CampNames,
) -> Vec<Line> {
    let mut out: Vec<Line> = rolls
        .iter()
        .filter(|r| !told.contains(r))
        .map(|&(player, value)| Line {
            player,
            name: names.of(player),
            body: Body::Rolled {
                value: u32::from(value),
                max: 100,
            },
        })
        .collect();
    if let (Some(w), false) = (winner, winner_told) {
        out.push(Line {
            player: w,
            name: names.of(w),
            body: Body::Note(format!("{} wins the bonus cache.", names.of(w))),
        });
    }
    out
}

/// Each roll as it lands, and the winner, into the party chat.
fn camp_rolls(
    camp: Res<Camp>,
    names: Res<CampNames>,
    mut chat: ResMut<Chat>,
    mut told: Local<(Vec<(u8, u8)>, bool)>,
) {
    if camp.roll.is_empty() && camp.roll_winner.is_none() {
        *told = (Vec::new(), false);
        return;
    }
    let lines = roll_lines(&camp.roll, &told.0, camp.roll_winner, told.1, &names);
    for line in &lines {
        chat.push(spans(line));
    }
    told.0.clone_from(&camp.roll);
    told.1 = camp.roll_winner.is_some();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_names_everyone_in_its_own_numbering() {
        let lobby = vec![(0, "Host".to_string()), (2, "Me".to_string())];
        // Player 2's machine: it is 0 here, the host is 2.
        assert_eq!(
            names_of(&lobby, 2, "Me"),
            vec![(0, "Me".to_string()), (2, "Host".to_string())]
        );
        // The host, missing from its own lobby, still names itself.
        assert_eq!(
            names_of(&[(1, "Jaina".to_string())], 0, "Thrall"),
            vec![(0, "Thrall".to_string()), (1, "Jaina".to_string())]
        );
    }

    #[test]
    fn each_roll_and_the_winner_are_told_once() {
        let names = CampNames(vec![(0, "Thrall".into()), (1, "Jaina".into())]);
        let lines = roll_lines(&[(1, 73)], &[], None, false, &names);
        assert_eq!(lines.len(), 1);
        assert_eq!(spans(&lines[0])[0].0, "[Jaina] rolls 73 (1-100)");
        let lines = roll_lines(&[(0, 12), (1, 73)], &[(1, 73)], Some(1), false, &names);
        let text: Vec<String> = lines.iter().map(|l| spans(l)[0].0.clone()).collect();
        assert_eq!(
            text,
            vec![
                "[Thrall] rolls 12 (1-100)".to_string(),
                "Jaina wins the bonus cache.".to_string()
            ]
        );
        assert!(roll_lines(
            &[(0, 12), (1, 73)],
            &[(0, 12), (1, 73)],
            Some(1),
            true,
            &names
        )
        .is_empty());
    }
}
