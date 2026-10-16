//! The co-op social layer, the WoW way: what a group of friends does besides fighting.
//!
//! - **Need / Greed / Pass** on boss gear, in 1.12's group-loot frame ([`loot`]).
//! - **Chat and `/roll`**, and the voice emotes on the hero ([`chat`]).
//! - **Pings and raid marks** in the world and on the minimap ([`ping`]).
//! - **The ready check** in the lobby, and everyone's card there ([`ready`], [`lobby`]).
//! - **Spectating** a living ally while downed ([`spectate`]).
//! - **Side bets** in Darkmoon Prize Tickets, settled from the run ([`bets`]).
//! - **Faire pulls** told to the lobby ([`chat`]), and the **fashion vote** after a run ([`vote`]).
//!
//! Every player's word goes to the host as one [`wire::Say`], and the host tells everyone, itself
//! included, with [`wire::Heard`]: replicon writes a listen server's own client messages back to
//! it as `FromClient` (from [`ClientId::Server`]) and its `ToClients` to `All` as local messages, so
//! every machine runs the same code on what it hears. Each machine saves only its own profile, so
//! loot, tickets and the vote's prize are granted by the winner's own machine on the host's word.

mod bets;
mod chat;
mod lobby;
mod loot;
mod ping;
mod ready;
mod spectate;
mod vote;
mod wire;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_replicon::prelude::*;

use super::host::Peer;
use super::{Link, Session};
use wire::{Heard, PingInput, Pinged, Say};

pub(super) use lobby::{member_extra, BookShown, LobbyIcon, LobbyMark, SocialSlot};
pub(super) use ready::{begin_check, force_start, ReadyShown};

/// WoW's party chat colour (`ChatTypeInfo["PARTY"]`).
pub(super) const PARTY: Color = Color::srgb(0.667, 0.667, 1.0);
/// A text emote's (`ChatTypeInfo["TEXT_EMOTE"]`).
pub(super) const EMOTE: Color = Color::srgb(1.0, 0.5, 0.25);
/// A system line's (`ChatTypeInfo["SYSTEM"]`).
pub(super) const SYSTEM: Color = Color::srgb(1.0, 1.0, 0.0);
/// A loot line's (`ChatTypeInfo["LOOT"]`).
pub(super) const LOOT: Color = Color::srgb(0.0, 0.667, 0.0);
/// Darkmoon Prize Tickets' parchment gold.
pub(super) const TICKET: Color = Color::srgb(1.0, 0.82, 0.0);

/// The social messages, registered after the rest of the protocol on both ends.
pub(super) fn protocol(app: &mut App) {
    app.add_client_message::<Say>(Channel::Ordered)
        .add_mapped_client_message::<PingInput>(Channel::Ordered)
        .add_server_message::<Heard>(Channel::Ordered)
        .add_mapped_server_message::<Pinged>(Channel::Ordered);
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, apply_art);
    chat::plugin(app);
    loot::plugin(app);
    ping::plugin(app);
    ready::plugin(app);
    lobby::plugin(app);
    bets::plugin(app);
    vote::plugin(app);
    spectate::plugin(app);
}

/// Run condition: in a session, hosting or joined.
pub(super) fn linked(session: Res<Session>) -> bool {
    matches!(session.link, Link::Hosting { .. } | Link::Joined { .. })
}

/// Run condition: hosting.
pub(super) fn hosting(session: Res<Session>) -> bool {
    matches!(session.link, Link::Hosting { .. })
}

/// The session player behind a client message: the host's own (from [`ClientId::Server`]) is 0.
pub(super) fn player_of(id: ClientId, peers: &Query<&Peer>) -> Option<u8> {
    match id {
        ClientId::Server => Some(0),
        ClientId::Client(e) => peers.get(e).ok().map(|p| p.player),
    }
}

/// Player `player`'s name in the lobby.
pub(super) fn name_of(session: &Session, player: u8) -> String {
    session
        .lobby
        .iter()
        .find(|m| m.player == player)
        .map_or_else(|| format!("Player {}", player + 1), |m| m.name.clone())
}

/// `heard` for everyone, the host included.
pub(super) fn to_all(heard: Heard) -> ToClients<Heard> {
    ToClients {
        targets: SendTargets::All,
        message: heard,
    }
}

/// A text node's parts in the HUD's font.
pub(super) fn label(
    font: &Handle<Font>,
    text: impl Into<String>,
    size: f32,
    color: Color,
) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

/// Client art on an image node: the file, its `TexCoords` (left, right, top, bottom) and its vertex
/// colour, (re)applied whenever any of them changes ([`apply_art`]).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(ImageNode, ArtShown)]
pub(super) struct Art {
    pub path: &'static str,
    pub coords: [f32; 4],
    pub tint: Color,
}

impl Art {
    pub(super) const fn whole(path: &'static str) -> Self {
        Self {
            path,
            coords: [0.0, 1.0, 0.0, 1.0],
            tint: Color::WHITE,
        }
    }

    pub(super) const fn crop(path: &'static str, coords: [f32; 4]) -> Self {
        Self {
            path,
            coords,
            tint: Color::WHITE,
        }
    }

    pub(super) const fn tinted(self, tint: Color) -> Self {
        Self { tint, ..self }
    }
}

/// What [`apply_art`] last drew on the node.
#[derive(Component, Default)]
pub(super) struct ArtShown(Option<Art>);

/// Draw each changed [`Art`], once the archives are readable; a file missing from them leaves the
/// node clear.
fn apply_art(
    mut nodes: Query<(&Art, &mut ImageNode, &mut ArtShown)>,
    assets: Option<ResMut<benilla_assets::WorldAssets>>,
    mut images: ResMut<Assets<Image>>,
    mut cache: Local<HashMap<&'static str, Option<Handle<Image>>>>,
) {
    let Some(mut assets) = assets else { return };
    for (art, mut img, mut shown) in &mut nodes {
        if shown.0 == Some(*art) {
            continue;
        }
        shown.0 = Some(*art);
        let handle = cache
            .entry(art.path)
            .or_insert_with(|| {
                let h = assets.sprite_texture(&format!("{}.blp", art.path), &mut images);
                if h.is_none() {
                    warn!("survivors: co-op: no art {}", art.path);
                }
                h
            })
            .clone();
        let Some(h) = handle else {
            img.color = Color::NONE;
            continue;
        };
        let [l, r, t, b] = art.coords;
        img.rect = images
            .get(&h)
            .map(Image::size_f32)
            .map(|s| Rect::new(l * s.x, t * s.y, r * s.x, b * s.y));
        img.image = h;
        img.color = art.tint;
    }
}

/// `UI-CharacterCreate-Classes`, the class icons the character screens show.
pub(super) const CLASS_ICONS: &str =
    "Interface\\Glues\\CharacterCreate\\UI-CharacterCreate-Classes";

/// Class `class`'s cell of [`CLASS_ICONS`], `CLASS_ICON_TCOORDS` in `CharacterCreate.lua`.
pub(super) fn class_icon(class: usize) -> [f32; 4] {
    let name = super::super::data::CLASSES
        .get(class)
        .map_or("Warrior", |c| c.name);
    match name {
        "Warrior" => [0.0, 0.25, 0.0, 0.25],
        "Mage" => [0.25, 0.496_093_75, 0.0, 0.25],
        "Rogue" => [0.496_093_75, 0.742_187_5, 0.0, 0.25],
        "Druid" => [0.742_187_5, 0.988_281_25, 0.0, 0.25],
        "Hunter" => [0.0, 0.25, 0.25, 0.5],
        "Shaman" => [0.25, 0.496_093_75, 0.25, 0.5],
        "Priest" => [0.496_093_75, 0.742_187_5, 0.25, 0.5],
        "Warlock" => [0.742_187_5, 0.988_281_25, 0.25, 0.5],
        _ => [0.0, 0.25, 0.5, 0.75],
    }
}

/// Every texture and sound kit the social layer names, for the data-gated test.
#[cfg(test)]
fn every_art() -> Vec<&'static str> {
    let mut out = vec![
        CLASS_ICONS,
        super::super::minimap::RAID_ICONS,
        chat::INPUT_BORDER,
        ready::FRAME,
        ready::READY_ICON,
        ready::NOT_READY_ICON,
        ready::WAITING_ICON,
        ping::PING_ART,
        loot::TIMER_BAR,
    ];
    out.extend(loot::BUTTONS.iter().flat_map(|b| [b.0, b.1]));
    out
}

#[cfg(test)]
fn every_kit() -> Vec<&'static str> {
    let mut out = vec![
        loot::DICE,
        chat::FANFARE_EPIC,
        chat::FANFARE_LEGENDARY,
        ready::READY_SOUND,
        bets::COIN,
        vote::VOTE_SOUND,
        vote::FASHIONISTA_SOUND,
    ];
    out.extend(ping::KINDS.iter().map(|k| ping::sound(*k)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_class_has_its_icon_cell() {
        let cells: Vec<_> = (0..super::super::super::data::CLASSES.len())
            .map(class_icon)
            .collect();
        for (i, a) in cells.iter().enumerate() {
            assert!(a[0] < a[1] && a[2] < a[3]);
            for b in &cells[i + 1..] {
                assert_ne!(a, b, "two classes share a cell");
            }
        }
    }

    /// Every texture the social layer draws, and every `SoundEntries` kit it plays, is in the
    /// client data; and each slash emote resolves to an `EmotesText` row with an animation.
    #[test]
    fn every_texture_sound_and_emote_is_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).unwrap();
        for path in every_art() {
            let file = format!("{path}.blp");
            assert!(chain.contains(&file), "{file}");
        }
        let kits = benilla_formats::load_sound_kit_catalog(&mut chain).unwrap();
        for kit in every_kit() {
            assert!(kits.by_name(kit).is_some(), "{kit}");
        }
        let emotes = benilla_formats::load_emote_sound_catalog(&mut chain).unwrap();
        for (command, row, _) in chat::EMOTES {
            let text = emotes
                .text_id(row)
                .unwrap_or_else(|| panic!("/{command}: no EmotesText row {row}"));
            let anim = emotes.text_emote(text).and_then(|e| emotes.anim(e));
            assert!(anim.is_some(), "/{command} plays no animation");
        }
        // The voiced ones speak in every race's voice: a Human man's and an Orc woman's cheer.
        let cheer = emotes.text_id("CHEER").unwrap();
        assert!(emotes.voice(cheer, 1, 0).is_some());
        assert!(emotes.voice(cheer, 2, 1).is_some());
    }
}
