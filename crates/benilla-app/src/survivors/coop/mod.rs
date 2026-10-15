//! Online co-op: friends play one run over the internet, host-authoritative, on `bevy_replicon` with
//! renet2's UDP transport ([`net`]).
//!
//! - The host simulates everything that decides the run: the horde, damage, drops, XP, gold and
//!   bosses ([`host`]). Its world replicates to the clients as lean components ([`wire`]): each
//!   unit's [`Look`](super::units::Look) once and a quantised [`wire::NetPos`]; heroes add their
//!   identity, vitals and build; enemies their kind, health and death; the run its clock and bar.
//! - A client simulates nothing ([`super::party::simulates`]). It builds every body from its look,
//!   interpolates what moves, and plays the spell visuals and numbers the host relays
//!   ([`wire::NetFx`]). It owns its own hero's walk and sends it ~30 times a second; the host trusts
//!   it ([`client`]).
//! - Level-ups: the host rolls each player's hand and sends it to that player; the pick, rerolls,
//!   banishes and skips come back as client messages and the host applies them.
//! - The menu's Multiplayer panel hosts, joins and shows the lobby ([`ui`]).
//!
//! Test aids: `WOW_SURVIVORS_HOST=<port>` hosts and `WOW_SURVIVORS_JOIN=<address>` joins once the
//! menu is up; `WOW_SURVIVORS_NAME=<name>` names this player in the lobby.

mod client;
mod host;
mod net;
#[cfg(test)]
mod tests;
mod ui;
mod wire;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RepliconRenetPlugins;

use super::party::{HandOutbox, NetRole};
use super::units::Look;
use wire::*;

pub(super) use ui::closed;

/// Replication and message rounds a second, each way.
const RATE_HZ: f32 = 30.0;

/// This machine's link to a co-op session, and what the lobby shows.
#[derive(Resource, Default)]
pub(super) struct Session {
    pub(super) link: Link,
    /// This machine's player number: 0 hosting, the host's [`Welcome`] once joined.
    pub(super) me: u8,
    /// Everyone connected, the host first.
    pub(super) lobby: Vec<Member>,
    /// What the panel says about the link: an error, or why a session ended.
    pub(super) status: String,
    /// A run of the session is under way.
    pub(super) in_run: bool,
    /// Bumped by every join, so the link watch starts over.
    pub(super) attempt: u32,
}

/// Where this machine stands with a session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Link {
    #[default]
    Off,
    /// Hosting on this UDP port.
    Hosting {
        port: u16,
    },
    /// Connecting to a host at this address, until its [`Welcome`].
    Joining {
        addr: String,
    },
    Joined {
        addr: String,
    },
}

impl Link {
    pub(super) fn role(&self) -> NetRole {
        match self {
            Link::Off => NetRole::Solo,
            Link::Hosting { .. } => NetRole::Host,
            Link::Joining { .. } | Link::Joined { .. } => NetRole::Client,
        }
    }
}

/// This player's name in lobbies: `WOW_SURVIVORS_NAME`, else the account's user name.
#[derive(Resource)]
pub(super) struct PlayerName(pub(super) String);

impl Default for PlayerName {
    fn default() -> Self {
        let name = crate::webenv::var("WOW_SURVIVORS_NAME")
            .or_else(|| std::env::var("USERNAME").ok())
            .or_else(|| std::env::var("USER").ok())
            .map(|n| n.trim().chars().take(16).collect::<String>())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Adventurer".into());
        Self(name)
    }
}

/// The real-time pacing of replication and of a client's walk updates.
#[derive(Resource)]
pub(super) struct Pace {
    pub(super) interval: f32,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            interval: 1.0 / RATE_HZ,
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        RepliconPlugins.set(ServerPlugin {
            // Ticked by `host::tick` at `RATE_HZ`, on the wall clock: a paused run still syncs.
            tick_schedule: None,
            ..default()
        }),
        RepliconRenetPlugins,
    ));
    protocol(app);
    app.init_resource::<Session>()
        .init_resource::<PlayerName>()
        .init_resource::<Pace>()
        .add_systems(PreUpdate, sync_role);
    host::plugin(app);
    client::plugin(app);
    ui::plugin(app);
}

/// The replication rules and messages, registered in one order on both ends (replicon checks the
/// two match at the handshake).
pub(super) fn protocol(app: &mut App) {
    app.replicate::<Look>()
        .replicate::<NetPos>()
        .replicate::<NetHero>()
        .replicate::<NetVitals>()
        .replicate::<NetBuild>()
        .replicate::<NetEnemy>()
        .replicate::<NetLife>()
        .replicate::<NetRun>()
        .add_client_message::<Hello>(Channel::Ordered)
        .add_client_message::<MoveInput>(Channel::Unreliable)
        .add_client_message::<HandAction>(Channel::Ordered)
        .add_server_message::<Welcome>(Channel::Ordered)
        .add_server_message::<Lobby>(Channel::Ordered)
        .add_server_message::<StartRun>(Channel::Ordered)
        .add_server_message::<RunPhase>(Channel::Ordered)
        .add_server_message::<Hand>(Channel::Ordered)
        .add_mapped_server_message::<NetFx>(Channel::Unreliable);
}

/// The run's [`NetRole`] follows the link, and a client gets the hand outbox its cards post to.
fn sync_role(
    mut commands: Commands,
    session: Res<Session>,
    mut role: ResMut<NetRole>,
    outbox: Option<Res<HandOutbox>>,
) {
    let want = session.link.role();
    if *role != want {
        *role = want;
    }
    match (want == NetRole::Client, outbox.is_some()) {
        (true, false) => commands.init_resource::<HandOutbox>(),
        (false, true) => commands.remove_resource::<HandOutbox>(),
        _ => {}
    }
}
