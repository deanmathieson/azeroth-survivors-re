//! What the social layer sends. Every player's word goes to the host as one [`Say`] (the host's
//! own too: replicon hands a listen server's client messages straight back to it), and the host
//! answers everyone, itself included, with [`Heard`]. Pings carry an entity, so they travel on
//! their own mapped pair ([`PingInput`], [`Pinged`]).

use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::super::wire::Ent;
use super::bets::Wager;
use super::loot::RollPick;
use super::ping::PingKind;
use super::ready::Mark;

/// One player's lobby card: who they are and what they wear, for the lobby, the bets and the
/// fashion vote. The host fills in `player`, `name` and `class` from its own record of the client.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct Card {
    pub player: u8,
    pub name: String,
    /// An index into [`super::super::super::data::CLASSES`].
    pub class: u8,
    /// The class's level as its talents make it: 1, then 10 to 60.
    pub level: u32,
    pub title: Option<String>,
    pub mount: Option<String>,
    pub hat: Option<String>,
    pub tabard: Option<String>,
}

/// What a player says or does, to the host.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) enum Speech {
    /// A line of party chat.
    Text(String),
    /// `/roll [max]`: the host rolls 1 to `max`.
    Roll { max: u32 },
    /// A voice emote, by its place in [`super::chat::EMOTES`].
    Emote(u8),
    /// A Darkmoon Faire pull: each prize's name and rarity (0 Common to 4 Legendary).
    Pulled(Vec<(String, u8)>),
    /// A choice at the group roll `roll`.
    Loot { roll: u32, pick: RollPick },
    /// The answer to the ready check.
    Ready(bool),
    /// This player's side bet, or none.
    Bet(Option<Wager>),
    /// The fashion vote, for this player.
    Vote(u8),
    /// This player's lobby card.
    Card(Card),
}

/// A player's [`Speech`], client to host.
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Say(pub(in crate::survivors) Speech);

/// What a chat line says, which each machine words and colours for itself.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) enum Body {
    Said(String),
    Rolled {
        value: u32,
        max: u32,
    },
    Emoted(u8),
    /// Won a group roll for this piece of gear.
    Won {
        item: u16,
        pick: RollPick,
        value: u32,
    },
    /// Everyone passed on this piece.
    AllPassed {
        item: u16,
    },
    /// Pulled this Darkmoon Faire prize, of this rarity.
    Pulled {
        prize: String,
        rarity: u8,
    },
    /// A quick ping.
    Pinged(PingKind),
    /// A plain system line.
    Note(String),
}

/// A chat line: who (`player`, `name`) and what.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) struct Line {
    pub player: u8,
    pub name: String,
    pub body: Body,
}

/// The ready check as everyone sees it.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct ReadyView {
    pub open: bool,
    /// Who started it.
    pub by: String,
    pub marks: Vec<(u8, Mark)>,
    pub left: f32,
}

/// The lobby's extras: everyone's card and side bet.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct Book {
    pub cards: Vec<Card>,
    pub bets: Vec<(u8, Wager)>,
}

/// One side bet, settled.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct Payout {
    pub player: u8,
    pub wager: Wager,
    /// Tickets back: the share of the pool, the stake refunded, or nothing.
    pub paid: u32,
    pub won: bool,
    /// No bet on its market won: every stake on it comes back.
    pub refunded: bool,
}

/// The fashion vote's count, and who won it (ties share it).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::survivors) struct VoteResult {
    pub tally: Vec<(u8, u32)>,
    pub winners: Vec<u8>,
}

/// What the host tells everyone.
#[derive(Message, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(in crate::survivors) enum Heard {
    Line(Line),
    /// A player's hero plays a voice emote ([`super::chat::EMOTES`]).
    Emote {
        player: u8,
        emote: u8,
    },
    /// A group roll opens on this piece for `secs`.
    RollStart {
        id: u32,
        item: u16,
        secs: f32,
    },
    RollEnd {
        id: u32,
    },
    /// To the winner alone: keep this piece.
    Grant {
        item: u16,
    },
    Ready(ReadyView),
    Book(Book),
    Settled(Vec<Payout>),
    /// The fashion vote opens, for these players, for `secs`.
    VoteOpen {
        cards: Vec<Card>,
        secs: f32,
    },
    VoteResult(VoteResult),
}

/// A ping or raid mark this player put down, at a point or on a unit.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct PingInput {
    pub kind: PingKind,
    pub at: [f32; 3],
    pub target: Option<Ent>,
}

/// A ping or raid mark, from the host to everyone.
#[derive(Message, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct Pinged {
    pub player: u8,
    pub kind: PingKind,
    pub at: [f32; 3],
    pub target: Option<Ent>,
}

impl MapEntities for PingInput {
    fn map_entities<E: EntityMapper>(&mut self, m: &mut E) {
        if let Some(t) = &mut self.target {
            t.0 = m.get_mapped(t.0);
        }
    }
}

impl MapEntities for Pinged {
    fn map_entities<E: EntityMapper>(&mut self, m: &mut E) {
        if let Some(t) = &mut self.target {
            t.0 = m.get_mapped(t.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round<T: Serialize + for<'de> Deserialize<'de>>(v: &T) -> T {
        let mut buf = [0u8; 4096];
        let bytes = bevy_replicon::postcard::to_slice(v, &mut buf).expect("serializes");
        bevy_replicon::postcard::from_bytes(bytes).expect("deserializes")
    }

    #[test]
    fn every_social_message_survives_the_wire() {
        let card = Card {
            player: 2,
            name: "Thrall".into(),
            class: 5,
            level: 23,
            title: Some("Carnie".into()),
            mount: Some("Swift Zulian Tiger".into()),
            hat: None,
            tabard: Some("Stormwind Tabard".into()),
        };
        let wager = Wager {
            market: 1,
            on: 2,
            stake: 25,
        };
        let says = [
            Speech::Text("lol".into()),
            Speech::Roll { max: 100 },
            Speech::Emote(3),
            Speech::Pulled(vec![("Swift Zulian Tiger".into(), 4)]),
            Speech::Loot {
                roll: 7,
                pick: RollPick::Need,
            },
            Speech::Ready(true),
            Speech::Bet(Some(wager)),
            Speech::Bet(None),
            Speech::Vote(1),
            Speech::Card(card.clone()),
        ];
        for s in says {
            let say = Say(s);
            assert_eq!(round(&say), say);
        }
        let heard = [
            Heard::Line(Line {
                player: 1,
                name: "Jaina".into(),
                body: Body::Rolled {
                    value: 42,
                    max: 100,
                },
            }),
            Heard::Line(Line {
                player: 0,
                name: "Host".into(),
                body: Body::Won {
                    item: 1234,
                    pick: RollPick::Greed,
                    value: 87,
                },
            }),
            Heard::Line(Line {
                player: 0,
                name: String::new(),
                body: Body::Pinged(PingKind::Mark(8)),
            }),
            Heard::Emote {
                player: 3,
                emote: 0,
            },
            Heard::RollStart {
                id: 1,
                item: 99,
                secs: 20.0,
            },
            Heard::RollEnd { id: 1 },
            Heard::Grant { item: 99 },
            Heard::Ready(ReadyView {
                open: true,
                by: "Host".into(),
                marks: vec![(0, Mark::Ready), (1, Mark::Waiting), (2, Mark::NotReady)],
                left: 28.5,
            }),
            Heard::Book(Book {
                cards: vec![card.clone()],
                bets: vec![(2, wager)],
            }),
            Heard::Settled(vec![Payout {
                player: 2,
                wager,
                paid: 50,
                won: true,
                refunded: false,
            }]),
            Heard::VoteOpen {
                cards: vec![card],
                secs: 30.0,
            },
            Heard::VoteResult(VoteResult {
                tally: vec![(1, 2), (2, 2)],
                winners: vec![1, 2],
            }),
        ];
        for h in heard {
            assert_eq!(round(&h), h);
        }
        let e = Ent(Entity::from_raw_u32(5).unwrap());
        let ping = PingInput {
            kind: PingKind::Help,
            at: [1.0, 2.0, 3.0],
            target: Some(e),
        };
        assert_eq!(round(&ping), ping);
        let pinged = Pinged {
            player: 1,
            kind: PingKind::Mark(1),
            at: [0.0; 3],
            target: None,
        };
        assert_eq!(round(&pinged), pinged);
    }

    #[test]
    fn ping_targets_map_to_the_other_side() {
        struct Shift;
        impl EntityMapper for Shift {
            fn get_mapped(&mut self, e: Entity) -> Entity {
                Entity::from_raw_u32(e.index_u32() + 100).unwrap()
            }
            fn set_mapped(&mut self, _: Entity, _: Entity) {}
        }
        let mut p = Pinged {
            player: 0,
            kind: PingKind::Mark(8),
            at: [0.0; 3],
            target: Some(Ent(Entity::from_raw_u32(4).unwrap())),
        };
        p.map_entities(&mut Shift);
        assert_eq!(p.target.unwrap().0, Entity::from_raw_u32(104).unwrap());
    }
}
