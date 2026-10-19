//! The host's half: it takes clients into the lobby, spawns their heroes in its run, applies their
//! walks, level-up choices and self-resurrections, and replicates its world to them: every body's
//! look and place, heroes' vitals and builds, enemies' health and deaths, the run, the spell
//! visuals, and at [`super::SLOW_HZ`] the auras, group buffs and per-hero numbers the party UI
//! shows.

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::server::server_tick::ServerTick;

use super::super::campfire::{self, Camp, CampAsk, CampProp, VendorItem};
use super::super::data;
use super::super::enemies::{self, Enemy, Gem};
use super::super::gear::GearRead;
use super::super::hand;
use super::super::hero::{self, Hero};
use super::super::hud::{Choice, Offer};
use super::super::jewelry;
use super::super::party::{self, HandDecision};
use super::super::partyui::MemberName;
use super::super::professions;
use super::super::profile::Profile;
use super::super::raid::{Telegraph, Yelled};
use super::super::roulette::RouletteWon;
use super::super::run_options::RunOptions;
use super::super::synergy;
use super::super::talents::{self, HeroSpec};
use super::super::units::Marks;
use super::super::{
    env_bonuses, HeroState, LocalHero, MetaBonuses, Phase, Rng, Run, RunEntity, Selection,
};
use super::wire::*;
use super::{Link, Pace, PlayerName, Session};
use crate::combat_text::CombatTextSpawn;
use crate::creature_anim::{CastEvent, CastEventKind, MovementState, SpellGoTargets, SwingMessage};

/// A connected client that said [`Hello`]: its player and pick, and its hero once spawned.
#[derive(Component, Clone, Debug)]
pub(super) struct Peer {
    pub(super) player: u8,
    pub(super) name: String,
    pub(super) class: u8,
    pub(super) race: u8,
    pub(super) sex: u8,
    pub(super) trainer: u32,
    pub(super) looks: NetLooks,
    pub(super) bonuses: super::super::bonus::Bonuses,
    pub(super) talents: Vec<(u32, u8)>,
    pub(super) mount_display: u32,
    pub(super) mount_hat: Option<super::super::stable::CosmeticHat>,
    pub(super) pinned_trinket: Option<String>,
    pub(super) hero: Option<Entity>,
}

/// On a remote hero: the connected client whose player drives it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Owner(pub(super) Entity);

/// On a remote hero: the hand its player holds now, rolled here, and the number of the last hand
/// sent ([`Hand::seq`]): a pick made at an older one is stale.
#[derive(Component, Default)]
pub(super) struct HeldHand(pub(super) Option<Vec<Choice>>, pub(super) u32);

impl HeldHand {
    /// The next hand's number, taken as it is sent.
    fn next_seq(&mut self) -> u32 {
        self.1 = self.1.wrapping_add(1);
        self.1
    }
}

/// The replicated entity carrying [`NetRun`].
#[derive(Component)]
pub(super) struct RunCarrier;

pub(super) fn hosting(session: Res<Session>) -> bool {
    matches!(session.link, Link::Hosting { .. })
}

fn hosting_run(session: Res<Session>) -> bool {
    matches!(session.link, Link::Hosting { .. }) && session.in_run
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        PreUpdate,
        (
            greet,
            update_cosmetics,
            apply_moves,
            apply_focus,
            apply_self_res,
            camp_actions,
            hand_actions,
            gather_actions,
            use_items,
        )
            .after(ServerSystems::Receive)
            .run_if(hosting),
    )
    .add_systems(
        Update,
        (
            spawn_remote_heroes
                .after(hero::drive_hero)
                .run_if(in_state(Phase::Playing))
                .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            drop_departed,
            serve_hands,
            (tag, tag_telegraphs).run_if(hosting_run),
            lobby,
        )
            .run_if(hosting),
    )
    .add_systems(
        PostUpdate,
        (
            (
                sync_positions,
                sync_heroes,
                sync_enemies,
                sync_run,
                sync_slow,
                relay_fx,
                relay_yells,
                relay_roulettes,
                send_remote_yields,
            )
                .run_if(hosting_run),
            tick,
        )
            .chain()
            .before(ServerSystems::Send)
            .run_if(hosting),
    )
    .add_systems(
        OnTransition {
            exited: Phase::Menu,
            entered: Phase::Playing,
        },
        begin_run.run_if(hosting),
    )
    .add_systems(OnEnter(Phase::GameOver), end_run.run_if(hosting))
    .add_systems(OnEnter(Phase::Menu), to_lobby.run_if(hosting));
}

/// The lowest player number not taken: the host is 0, its clients 1 to 3.
pub(super) fn free_player(taken: &[u8]) -> Option<u8> {
    (1..party::MAX_PARTY).find(|p| !taken.contains(p))
}

/// Whether `sender` is the client driving the hero `owner` marks: a client moves and picks for
/// its own hero only, and the host's own messages own no remote hero.
pub(super) fn owns(owner: Owner, sender: ClientId) -> bool {
    sender == ClientId::Client(owner.0)
}

/// The Run Options as they travel.
pub(super) fn options_of(opts: &RunOptions) -> [f32; 4] {
    [opts.density, opts.health, opts.damage, opts.speed]
}

fn start_of(selection: &Selection, opts: &RunOptions) -> StartRun {
    StartRun {
        stage: selection.stage.min(255) as u8,
        options: options_of(opts),
    }
}

/// A client says who it is: it gets a player number, the lobby, and, mid-run, the run.
#[allow(clippy::too_many_arguments)]
pub(super) fn greet(
    mut commands: Commands,
    mut hellos: MessageReader<FromClient<Hello>>,
    peers: Query<(Entity, &Peer)>,
    mut session: ResMut<Session>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    mut welcome: MessageWriter<ToClients<Welcome>>,
    mut start: MessageWriter<ToClients<StartRun>>,
    build: Option<Res<crate::BuildId>>,
    name: Res<PlayerName>,
    mut refuse: MessageWriter<ToClients<Refused>>,
    mut cut: MessageWriter<DisconnectRequest>,
) {
    let ours = super::build_tag(build.as_deref());
    let mut taken: Vec<(Entity, u8)> = peers.iter().map(|(e, p)| (e, p.player)).collect();
    for hello in hellos.read() {
        let ClientId::Client(client) = hello.client_id else {
            continue;
        };
        // Another build would desync: turned away, told why, on both screens.
        if hello.message.build != ours {
            let reason = super::mismatch(&hello.message.name);
            warn!(
                "survivors: co-op: {reason} (theirs {}, ours {ours})",
                hello.message.build
            );
            refuse.write(ToClients {
                targets: SendTargets::Single(hello.client_id),
                message: Refused {
                    reason: reason.clone(),
                },
            });
            cut.write(DisconnectRequest { client });
            session.status = reason;
            continue;
        }
        let player = match taken.iter().find(|(e, _)| *e == client) {
            Some(&(_, p)) => p,
            None => {
                let numbers: Vec<u8> = taken.iter().map(|t| t.1).collect();
                let Some(p) = free_player(&numbers) else {
                    warn!("survivors: co-op: the party is full; turning a client away");
                    commands.entity(client).try_despawn();
                    continue;
                };
                p
            }
        };
        taken.push((client, player));
        let h = &hello.message;
        info!(
            "survivors: co-op: {} joined as player {}",
            h.name,
            player + 1
        );
        let old_hero = peers.get(client).ok().and_then(|(_, p)| p.hero);
        if let Some(hero) = old_hero {
            commands.entity(hero).try_insert(h.looks.clone());
        }
        commands.entity(client).try_insert(Peer {
            player,
            name: h.name.chars().take(24).collect(),
            class: h.class,
            race: h.race,
            sex: h.sex,
            trainer: h.trainer,
            looks: h.looks.clone(),
            bonuses: h.bonuses.clone(),
            talents: h.talents.clone(),
            mount_display: h.mount_display,
            mount_hat: h.mount_hat,
            pinned_trinket: h.pinned_trinket.clone(),
            // A repeated hello (sent until welcomed) keeps the hero it has.
            hero: old_hero,
        });
        let to = SendTargets::Single(hello.client_id);
        welcome.write(ToClients {
            targets: to,
            message: Welcome {
                player,
                host: name.0.clone(),
                build: ours.clone(),
            },
        });
        if session.in_run {
            start.write(ToClients {
                targets: to,
                message: start_of(&selection, &opts),
            });
        }
        // The newcomer needs the lobby even if nothing else changed.
        session.lobby.clear();
    }
}

/// Keep the host's authoritative peer and already-spawned hero in step with a lobby Stable
/// change.  Updating `NetHero` is what makes Replicon fan this cosmetic choice out to everyone.
pub(super) fn update_cosmetics(
    mut cosmetics: MessageReader<FromClient<Cosmetics>>,
    mut peers: Query<&mut Peer>,
    mut heroes: Query<&mut NetHero>,
) {
    for update in cosmetics.read() {
        let ClientId::Client(client) = update.client_id else {
            continue;
        };
        let Ok(mut peer) = peers.get_mut(client) else {
            continue;
        };
        peer.mount_display = update.message.mount_display;
        peer.mount_hat = update.message.mount_hat;
        if let Some(mut hero) = peer.hero.and_then(|hero| heroes.get_mut(hero).ok()) {
            hero.mount_display = update.message.mount_display;
            hero.mount_hat = update.message.mount_hat;
        }
    }
}

/// The lobby, sent to every client whenever it changes: the host, then the clients by number.
fn lobby(
    mut session: ResMut<Session>,
    peers: Query<&Peer>,
    selection: Res<Selection>,
    name: Res<PlayerName>,
    mut out: MessageWriter<ToClients<Lobby>>,
) {
    let race = selection
        .look
        .and_then(|i| selection.looks.get(i))
        .map_or(0, |l| l.1);
    let mut players = vec![Member {
        player: 0,
        name: name.0.clone(),
        class: selection.class.unwrap_or(0) as u8,
        race,
    }];
    let mut peers: Vec<&Peer> = peers.iter().collect();
    peers.sort_by_key(|p| p.player);
    players.extend(peers.iter().map(|p| Member {
        player: p.player,
        name: p.name.clone(),
        class: p.class,
        race: p.race,
    }));
    if session.lobby != players {
        session.lobby = players.clone();
        out.write(ToClients {
            targets: SendTargets::CLIENTS_ONLY,
            message: Lobby { players },
        });
    }
}

/// Each client's walk moves its own hero, trusted: friends only.
pub(super) fn apply_moves(
    mut moves: MessageReader<FromClient<MoveInput>>,
    mut heroes: Query<(&Owner, &mut Transform, &mut MovementState), With<party::Remote>>,
) {
    for m in moves.read() {
        let at = Vec3::from_array(m.message.at);
        if !at.is_finite() || !m.message.yaw.is_finite() || !m.message.pace.is_finite() {
            continue;
        }
        for (owner, mut tf, mut mv) in &mut heroes {
            if !owns(*owner, m.client_id) {
                continue;
            }
            tf.translation = at;
            tf.rotation = Quat::from_rotation_y(m.message.yaw);
            if mv.speed != m.message.pace || mv.flags != m.message.flags {
                mv.speed = m.message.pace;
                mv.flags = m.message.flags;
            }
        }
    }
}

/// Each client's hand strip open or shut: its hero is Focused while it is, trusted.
pub(super) fn apply_focus(
    mut msgs: MessageReader<FromClient<FocusInput>>,
    mut heroes: Query<(&Owner, &mut HeroState), With<party::Remote>>,
) {
    for m in msgs.read() {
        for (owner, mut hero) in &mut heroes {
            if owns(*owner, m.client_id) && hero.focused != m.message.0 {
                hero.focused = m.message.0;
            }
        }
    }
}

/// Each client's self-resurrection key, on its own hero while it is down: the soulstone or ankh
/// it holds gets it up ([`synergy`]), as the host's own key does for the host's hero.
pub(super) fn apply_self_res(
    mut msgs: MessageReader<FromClient<SelfRes>>,
    mut heroes: Query<(&Owner, &mut HeroState), With<party::Remote>>,
) {
    for m in msgs.read() {
        for (owner, mut hero) in &mut heroes {
            if owns(*owner, m.client_id) && !hero.alive() {
                hero.synergy.res.asked = true;
            }
        }
    }
}

/// Camp requests are only meaningful while the authoritative timer is live, and only on the
/// requesting client's own hero, so a client cannot spend the common purse for another. Each is
/// decided as the host's own are ([`campfire::apply`]); the host rolls the dice.
pub(super) fn camp_actions(
    mut msgs: MessageReader<FromClient<CampAction>>,
    mut camp: ResMut<Camp>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut heroes: Query<(&Owner, &mut HeroState), With<party::Remote>>,
) {
    for msg in msgs.read() {
        if !camp.active(&run) {
            continue;
        }
        let ask = match msg.message {
            CampAction::Ready => CampAsk::Ready,
            CampAction::Fish => CampAsk::Fish,
            CampAction::Roll => CampAsk::Roll,
            CampAction::Buy(i) => match VendorItem::from_index(i) {
                Some(item) => CampAsk::Buy(item),
                None => continue,
            },
        };
        let Some((_, mut hero)) = heroes
            .iter_mut()
            .find(|(owner, _)| owns(**owner, msg.client_id))
        else {
            continue;
        };
        campfire::apply(&mut camp, &mut run, &mut hero, ask, &mut rng);
    }
}

/// A client finished a gather: its own hero's player rolls its own yield on the node, once, as
/// the host's own gathers do ([`professions::gather::harvest`]), and the yield goes back to it
/// alone. A depleted node, a repeat or too low a rank sends nothing.
pub(super) fn gather_actions(
    mut msgs: MessageReader<FromClient<GatherAction>>,
    heroes: Query<(&Owner, &HeroState), With<party::Remote>>,
    party_size: Query<(), With<HeroState>>,
    mut nodes: Query<(
        &mut professions::gather::NetNode,
        &mut professions::gather::NodeLife,
    )>,
    mut rng: ResMut<Rng>,
    mut out: MessageWriter<ToClients<Yield>>,
) {
    let party = party_size.iter().count();
    for msg in msgs.read() {
        let Some(player) = heroes
            .iter()
            .find(|(owner, _)| owns(**owner, msg.client_id))
            .map(|(_, hero)| hero.player)
        else {
            continue;
        };
        let Some((mut node, mut life)) = nodes
            .iter_mut()
            .find(|(node, _)| node.id == msg.message.node)
        else {
            continue;
        };
        if let Some(y) = professions::gather::harvest(
            &mut node,
            &mut life,
            player,
            msg.message.rank,
            party,
            &mut rng,
        ) {
            out.write(ToClients {
                targets: SendTargets::Single(msg.client_id),
                message: Yield(y),
            });
        }
    }
}

/// A client drank from its own bags: its hero takes the effect here, where it lives.
pub(super) fn use_items(
    mut msgs: MessageReader<FromClient<UseItem>>,
    mut heroes: Query<(&Owner, &mut HeroState), With<party::Remote>>,
) {
    for msg in msgs.read() {
        if msg.message.item != professions::items::MINOR_HEALING_POTION {
            continue;
        }
        if let Some((_, mut hero)) = heroes
            .iter_mut()
            .find(|(owner, _)| owns(**owner, msg.client_id))
        {
            professions::effects::drink(&mut hero);
        }
    }
}

/// The yields rolled here for other players (a skinned kill), each to its player's client.
fn send_remote_yields(
    remote: Option<ResMut<professions::gather::RemoteYields>>,
    heroes: Query<(&Owner, &HeroState), With<party::Remote>>,
    mut out: MessageWriter<ToClients<Yield>>,
) {
    let Some(mut remote) = remote else {
        return;
    };
    for (player, y) in std::mem::take(&mut remote.0) {
        if let Some((owner, _)) = heroes.iter().find(|(_, hero)| hero.player == player) {
            out.write(ToClients {
                targets: SendTargets::Single(ClientId::Client(owner.0)),
                message: Yield(y),
            });
        }
    }
}

/// What a hero's player's HUD and cards read of it.
pub(super) fn build_of(hero: &HeroState) -> NetBuild {
    NetBuild {
        abilities: hero
            .abilities
            .iter()
            .map(|o| (o.def.key.to_string(), o.level))
            .collect(),
        passives: hero
            .passives
            .iter()
            .map(|(p, l)| (p.key.to_string(), *l))
            .collect(),
        bonuses: hero.bonuses.clone(),
        picks: hero.picks,
        rerolls: hero.rerolls,
        banishes: hero.banishes,
        banished: hero.banished.iter().map(|&s| WireStruck::of(s)).collect(),
        overcharges: hero.overcharges,
        jewels: hero.jewelry.picks,
        empower: hero.empower,
    }
}

/// The hand message for `choices` dealt to `hero`, with the build its slots index into and its
/// number `seq`.
pub(super) fn hand_for(hero: &HeroState, choices: &[Choice], seq: u32) -> Hand {
    Hand {
        choices: choices.iter().map(|&c| WireChoice::of(c)).collect(),
        build: build_of(hero),
        seq,
    }
}

/// One choice a remote player made at the hand it holds (`held`), applied to its hero: the hand
/// it holds next, which the caller sends (an empty one when it holds none), or `None` when the
/// choice changed nothing (no hand held, a slot not dealt, a tool used up).
pub(super) fn decide(
    hero: &mut HeroState,
    held: &mut Option<Vec<Choice>>,
    decision: HandDecision,
    run: &mut Run,
    rng: &mut Rng,
) -> Option<Vec<Choice>> {
    let hand = held.as_mut()?;
    match decision {
        HandDecision::Pick(i) => {
            let &choice = hand.get(i)?;
            if !party::apply_pick(hero, choice) {
                return None;
            }
            run.version += 1;
            *held = None;
        }
        HandDecision::Skip => {
            party::apply_skip(hero, run, hand)?;
            *held = None;
        }
        // A jewelry hand is not rerolled.
        HandDecision::Reroll if hand.iter().any(|c| matches!(c, Choice::Jewel(_))) => return None,
        HandDecision::Reroll => {
            let mut offer = Offer::default();
            if !hand::reroll(hero, &mut offer, rng) {
                return None;
            }
            *hand = offer.choices().to_vec();
            return Some(hand.clone());
        }
        HandDecision::Banish(i) => {
            let mut offer = Offer::default();
            offer.deal(hand.clone());
            if !hand::banish(i, hero, &mut offer, rng) {
                return None;
            }
            *hand = offer.choices().to_vec();
            return Some(hand.clone());
        }
    }
    // A hand closed: the next is dealt by `serve_hands`; with none left the client is told.
    // Another player's jewelry rolls from the catalog: its own completed decks are its profile's.
    Some(
        match party::next_hand(hero, &jewelry::pool(None), run.level, rng) {
            Some(next) => {
                *held = Some(next.clone());
                next
            }
            None => Vec::new(),
        },
    )
}

/// Each remote player's picks, rerolls, banishes and skips, on its own hero only, at the hand it
/// holds now: one made at a hand since dealt over (a reroll's, the next one's) is dropped.
pub(super) fn hand_actions(
    mut actions: MessageReader<FromClient<HandAction>>,
    mut heroes: Query<(&Owner, &mut HeroState, &mut HeldHand), With<party::Remote>>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut out: MessageWriter<ToClients<Hand>>,
) {
    for a in actions.read() {
        for (owner, mut hero, mut held) in &mut heroes {
            if !owns(*owner, a.client_id) || a.message.seq != held.1 {
                continue;
            }
            let next = decide(
                &mut hero,
                &mut held.0,
                a.message.decision.decision(),
                &mut run,
                &mut rng,
            );
            if let Some(next) = next {
                let seq = held.next_seq();
                out.write(ToClients {
                    targets: SendTargets::Single(a.client_id),
                    message: hand_for(&hero, &next, seq),
                });
            }
        }
    }
}

/// A remote hero with picks banked and no hand out gets one, rolled here and sent to its player.
pub(super) fn serve_hands(
    mut heroes: Query<(&Owner, &mut HeroState, &mut HeldHand), With<party::Remote>>,
    run: Res<Run>,
    mut rng: ResMut<Rng>,
    mut out: MessageWriter<ToClients<Hand>>,
) {
    let pool = jewelry::pool(None);
    for (owner, mut hero, mut held) in &mut heroes {
        // A player going down puts its hand away; it is dealt again once revived.
        if !hero.alive() {
            if held.0.take().is_some() {
                let seq = held.next_seq();
                out.write(ToClients {
                    targets: SendTargets::Single(ClientId::Client(owner.0)),
                    message: hand_for(&hero, &[], seq),
                });
            }
            continue;
        }
        if held.0.is_some() {
            continue;
        }
        // The player's next banked hand (a jewelry choice first), ready for its strip.
        let Some(choices) = party::next_hand(&mut hero, &pool, run.level, &mut rng) else {
            continue;
        };
        let seq = held.next_seq();
        out.write(ToClients {
            targets: SendTargets::Single(ClientId::Client(owner.0)),
            message: hand_for(&hero, &choices, seq),
        });
        held.0 = Some(choices);
    }
}

/// Once the host's own hero stands in the run, every client's hero joins it, beside it, on that
/// player's pick and bonuses; a new run respawns them.
fn spawn_remote_heroes(
    mut commands: Commands,
    mut run: ResMut<Run>,
    local: Query<&Transform, With<LocalHero>>,
    mut peers: Query<(Entity, &mut Peer)>,
    heroes: Query<(), With<Hero>>,
    world: benilla_world::world_point::WorldPoint,
) {
    let Ok(tf) = local.single() else { return };
    for (client, mut peer) in &mut peers {
        if peer.hero.is_some_and(|h| heroes.contains(h)) {
            continue;
        }
        let a = f32::from(peer.player) / f32::from(party::MAX_PARTY) * std::f32::consts::TAU;
        let at = tf.translation + Vec3::new(a.cos(), 0.0, a.sin()) * 3.0;
        let pos = enemies::ground(&world, at, tf.translation.y);
        let e = spawn_peer_hero(&mut commands, &mut run, client, &peer, pos);
        peer.hero = Some(e);
    }
}

/// `peer`'s hero at `pos`, owned by `client`: in its trainer's look (its dressed look is its own
/// machine's), on its bonuses.
pub(super) fn spawn_peer_hero(
    commands: &mut Commands,
    run: &mut Run,
    client: Entity,
    peer: &Peer,
    pos: Vec3,
) -> Entity {
    let class = usize::from(peer.class).min(data::CLASSES.len() - 1);
    let mut state = HeroState::new(
        peer.player,
        &data::CLASSES[class],
        peer.race,
        peer.bonuses.clone(),
    );
    let spec = talents::grant_learned(&mut state, &peer.talents);
    // Joining a run under way, it banks a hand for every level the party has made.
    state.picks = super::super::strip::catch_up(run.level);
    // Caches already opened paid every existing player a jewelry choice.
    state.jewelry.picks = run.caches;
    // Its deck is its own profile's to have completed; what the host holds it to is that the key
    // names a Darkmoon trinket, never any other piece of the catalog.
    if let Some(piece) = peer
        .pinned_trinket
        .as_deref()
        .and_then(jewelry::darkmoon_piece)
    {
        jewelry::put_on(&mut state, piece);
    }
    let e = hero::spawn_hero(commands, state, peer.trainer, peer.sex, run.guid(), pos);
    let mut remote = commands.entity(e);
    remote.insert((
        party::Remote,
        Owner(client),
        HeldHand::default(),
        NetHero {
            player: peer.player,
            name: peer.name.clone(),
            class: class as u8,
            race: peer.race,
            sex: peer.sex,
            trainer: peer.trainer,
            mount_display: peer.mount_display,
            mount_hat: peer.mount_hat,
        },
        peer.looks.clone(),
        MemberName(peer.name.clone()),
    ));
    if let Some(spec) = spec {
        remote.insert(HeroSpec(spec));
    }
    e
}

/// A client gone: its hero leaves the run.
fn drop_departed(
    mut commands: Commands,
    heroes: Query<(Entity, &Owner)>,
    peers: Query<(), With<Peer>>,
) {
    for (e, owner) in &heroes {
        if !peers.contains(owner.0) {
            info!("survivors: co-op: a player left; their hero goes");
            commands.entity(e).try_despawn();
        }
    }
}

/// Where `def` stands in the stage's roster: `(boss, index)`.
pub(super) fn roster_index(stage: usize, def: &data::EnemyDef) -> (bool, u16) {
    let roster = data::roster(stage);
    let named = |list: &[data::EnemyDef]| list.iter().position(|d| d.name == def.name);
    if let Some((boss, i)) = roster.index_of(def) {
        return (boss, i as u16);
    }
    // A def from outside this roster: its namesake, else the roster's first.
    match named(&roster.bosses) {
        Some(i) => (true, i as u16),
        None => (false, named(&roster.enemies).unwrap_or(0) as u16),
    }
}

/// Every durable body of the run, once spawned, replicates: its place, and a hero's or an enemy's
/// own parts. Pickups stay host-only: clients receive the shared XP/gold totals at the slow stats
/// cadence, avoiding a reliable entity lifecycle for every gem vacuumed from a horde. The host's
/// own hero is described from the menu's pick.
#[allow(clippy::type_complexity)]
pub(super) fn tag(
    mut commands: Commands,
    selection: Res<Selection>,
    name: Res<PlayerName>,
    profile: Option<Res<Profile>>,
    gear: GearRead,
    displays: Option<Res<crate::entities::ItemDisplays>>,
    fresh: Query<
        (
            Entity,
            &Transform,
            Option<&MovementState>,
            Option<&Enemy>,
            Option<&HeroState>,
            Option<&Marks>,
            Has<NetHero>,
            Option<&enemies::RareMob>,
        ),
        (
            With<super::super::units::Look>,
            With<RunEntity>,
            Without<Gem>,
            // Each machine builds its own camp round the replicated fire.
            Without<CampProp>,
            Without<Replicated>,
        ),
    >,
) {
    for (e, tf, mv, enemy, hero, marks, described, rare) in &fresh {
        let mut ec = commands.entity(e);
        ec.try_insert((Replicated, pos_of(tf, mv)));
        if hero.is_some() || enemy.is_some() {
            ec.try_insert(auras_of(marks));
        }
        if let Some(hero) = hero {
            ec.try_insert((
                vitals_of(hero),
                build_of(hero),
                stats_of(hero),
                buffs_of(hero),
                NetRaid::of(&hero.raid),
            ));
            if !described {
                let look = selection.look.and_then(|i| selection.looks.get(i));
                let (trainer, race, sex) = look.copied().unwrap_or((0, 0, 0));
                let cosmetics = profile
                    .as_deref()
                    .map(|p| super::super::stable::selected_cosmetics(p, displays.as_deref()))
                    .unwrap_or((0, None));
                ec.try_insert(NetHero {
                    player: party::LOCAL_PLAYER,
                    name: name.0.clone(),
                    class: selection.class.unwrap_or(0) as u8,
                    race,
                    sex,
                    trainer,
                    mount_display: cosmetics.0,
                    mount_hat: cosmetics.1,
                });
                let class = selection.class.unwrap_or(0);
                let saved = gear.get(class);
                ec.try_insert(NetLooks {
                    features: saved.looks.features(race, sex),
                    gear: std::array::from_fn(|i| {
                        saved
                            .worn(super::super::gear::Slot::ALL[i])
                            .map_or(0, |piece| piece.display_id)
                    }),
                });
                ec.try_insert(MemberName(name.0.clone()));
            }
        }
        if let Some(enemy) = enemy {
            let (boss, index) = roster_index(selection.stage, enemy.def);
            ec.try_insert((
                NetEnemy {
                    boss,
                    index,
                    elite: enemy.elite,
                    guardian: enemy.guardian,
                    max_hp: enemy.max_hp,
                    rare: rare.map(|r| r.0),
                },
                life_of(enemy),
            ));
        }
    }
}

fn pos_of(tf: &Transform, mv: Option<&MovementState>) -> NetPos {
    NetPos::new(
        tf.translation,
        hero::yaw_of(tf.rotation),
        mv.map_or(0.0, |m| m.speed),
        mv.map_or(0, |m| m.flags),
    )
}

fn life_of(enemy: &Enemy) -> NetLife {
    NetLife {
        hp: enemy.hp.max(0.0).ceil(),
        dead: enemy.dead,
    }
}

fn sync_positions(mut q: Query<(&Transform, Option<&MovementState>, &mut NetPos)>) {
    for (tf, mv, mut pos) in &mut q {
        pos.set_if_neq(pos_of(tf, mv));
    }
}

fn vitals_of(hero: &HeroState) -> NetVitals {
    NetVitals {
        hp: hero.hp.max(0.0),
        max_hp: hero.stats().max_hp,
        shield: if hero.shield_t > 0.0 {
            hero.shield
        } else {
            0.0
        },
        downed: !hero.alive(),
        revive: hero.revive_t,
        focus: hero.focused,
    }
}

pub(super) fn sync_heroes(
    mut q: Query<(
        &HeroState,
        &mut NetVitals,
        &mut NetBuild,
        Option<&mut NetRaid>,
    )>,
) {
    for (hero, mut vitals, mut build, raid) in &mut q {
        vitals.set_if_neq(vitals_of(hero));
        build.set_if_neq(build_of(hero));
        if let Some(mut raid) = raid {
            raid.set_if_neq(NetRaid::of(&hero.raid));
        }
    }
}

/// A raid boss's telegraphs replicate as they are laid, so every screen draws the same warning
/// at the same time ([`super::super::raid`]).
pub(super) fn tag_telegraphs(
    mut commands: Commands,
    fresh: Query<Entity, (With<Telegraph>, Without<Replicated>)>,
) {
    for e in &fresh {
        commands.entity(e).try_insert(Replicated);
    }
}

/// A boss's yells, for the clients to show and sound.
pub(super) fn relay_yells(
    mut yells: MessageReader<Yelled>,
    mut out: MessageWriter<ToClients<NetYell>>,
) {
    for y in yells.read() {
        out.write(ToClients {
            targets: SendTargets::CLIENTS_ONLY,
            message: NetYell {
                text: y.text.clone(),
                sound: y.sound,
            },
        });
    }
}

/// A boss chest's prizes for another player's hero, to its player's screen.
pub(super) fn relay_roulettes(
    mut won: MessageReader<RouletteWon>,
    heroes: Query<&Owner, With<party::Remote>>,
    mut out: MessageWriter<ToClients<NetRoulette>>,
) {
    for w in won.read() {
        let Ok(owner) = heroes.get(w.hero) else {
            continue;
        };
        out.write(ToClients {
            targets: SendTargets::Single(ClientId::Client(owner.0)),
            message: NetRoulette {
                prizes: w
                    .prizes
                    .iter()
                    .map(|p| (p.text.clone(), p.rarity.index()))
                    .collect(),
            },
        });
    }
}

/// The auras `marks` shows, in slot order.
fn auras_of(marks: Option<&Marks>) -> NetAuras {
    NetAuras(marks.map(|m| m.spells().collect()).unwrap_or_default())
}

pub(super) fn stats_of(hero: &HeroState) -> NetStats {
    NetStats {
        damage_done: hero.damage_done,
        healing_done: hero.healing_done,
        damage_taken: hero.damage_taken,
        revives: hero.revives_given,
        gold: hero.gold_collected,
        downs: hero.times_downed,
    }
}

/// The group buffs on `hero`, by their place in [`synergy::BUFFS`].
pub(super) fn buffs_of(hero: &HeroState) -> NetBuffs {
    NetBuffs(
        hero.synergy
            .buffs
            .iter()
            .filter_map(|b| {
                let i = synergy::BUFFS.iter().position(|d| std::ptr::eq(d, b.def))?;
                Some((i as u8, b.strength))
            })
            .collect(),
    )
}

/// The party UI's slower parts, at [`super::SLOW_HZ`] on the wall clock: every hero's and enemy's
/// auras, the heroes' group buffs and their numbers so far. On the results, every frame: the
/// clients' awards are the host's final numbers.
#[allow(clippy::type_complexity)]
pub(super) fn sync_slow(
    time: Res<Time<Real>>,
    phase: Res<State<Phase>>,
    mut acc: Local<f32>,
    mut heroes: Query<(&HeroState, &mut NetStats, &mut NetBuffs)>,
    mut auras: Query<(Option<&Marks>, &mut NetAuras)>,
) {
    let interval = 1.0 / super::SLOW_HZ;
    *acc += time.delta_secs();
    if *acc < interval && *phase.get() != Phase::GameOver {
        return;
    }
    *acc = (*acc - interval).clamp(0.0, interval);
    for (hero, mut stats, mut buffs) in &mut heroes {
        stats.set_if_neq(stats_of(hero));
        buffs.set_if_neq(buffs_of(hero));
    }
    for (marks, mut net) in &mut auras {
        net.set_if_neq(auras_of(marks));
    }
}

fn sync_enemies(mut q: Query<(&Enemy, &mut NetLife)>) {
    for (enemy, mut life) in &mut q {
        life.set_if_neq(life_of(enemy));
    }
}

pub(super) fn run_of(run: &Run, camp: &Camp) -> NetRun {
    NetRun {
        elapsed: run.elapsed,
        kills: run.kills,
        level: run.level,
        xp: run.xp,
        gold: run.gold,
        elites: run.elites,
        caches: run.caches,
        bosses: run.bosses,
        events: run.events,
        won: run.won,
        banner: run.banner.as_ref().map(|b| b.0.clone()),
        slow: run.slow,
        camp_left: run.breather,
        camp_ready: camp.ready.clone(),
        camp_fished: camp.fished.clone(),
        fishing_dip: camp.fishing_dip,
        camp_rolls: camp.roll.clone(),
        camp_roll_winner: camp.roll_winner,
        camp_fire: camp.fire.map(|f| f.to_array()),
    }
}

fn sync_run(run: Res<Run>, camp: Res<Camp>, mut q: Query<&mut NetRun>) {
    for mut net in &mut q {
        net.set_if_neq(run_of(&run, &camp));
    }
}

/// The spell visuals, swings and numbers the run played this frame, for the clients: only those
/// whose units replicate (a GO keeps the hits that do).
#[allow(clippy::too_many_arguments)]
fn relay_fx(
    mut casts: MessageReader<CastEvent>,
    mut gos: MessageReader<SpellGoTargets>,
    mut swings: MessageReader<SwingMessage>,
    mut texts: MessageReader<CombatTextSpawn>,
    replicated: Query<(), With<Replicated>>,
    mut out: MessageWriter<ToClients<NetFx>>,
) {
    let shown = |e: Entity| replicated.contains(e);
    let mut send = |fx: NetFx| {
        out.write(ToClients {
            targets: SendTargets::CLIENTS_ONLY,
            message: fx,
        });
    };
    // A GO's cast edge travels with its targets.
    let mut paired = Vec::new();
    for go in gos.read() {
        paired.push(go.seq);
        if !shown(go.caster) {
            continue;
        }
        send(NetFx::Go {
            caster: Ent(go.caster),
            spell: go.spell_id,
            hits: go
                .hits
                .iter()
                .copied()
                .filter(|&h| shown(h))
                .map(Ent)
                .collect(),
            dest: go.dest.map(|d| d.to_array()),
        });
    }
    for c in casts.read() {
        if !shown(c.entity) || (c.kind == CastEventKind::Go && paired.contains(&c.seq)) {
            continue;
        }
        let kind = match c.kind {
            CastEventKind::Start => WireCast::Start,
            CastEventKind::Go => WireCast::Go,
            CastEventKind::Fail => WireCast::Fail,
            CastEventKind::Impact { weapon_visual } => WireCast::Impact { weapon_visual },
            CastEventKind::GroundImpact { pos } => WireCast::GroundImpact { at: pos.to_array() },
        };
        send(NetFx::Cast {
            entity: Ent(c.entity),
            spell: c.spell_id,
            kind,
        });
    }
    for s in swings.read() {
        if !shown(s.attacker) {
            continue;
        }
        send(NetFx::Swing {
            attacker: Ent(s.attacker),
            victim: s.victim.filter(|&v| shown(v)).map(Ent),
            hit_info: s.hit_info,
            victim_state: s.victim_state,
            damage: s.damage,
        });
    }
    for t in texts.read() {
        if !shown(t.anchor) {
            continue;
        }
        send(NetFx::Text {
            anchor: Ent(t.anchor),
            text: t.text.clone(),
            category: t.category,
            color: t.color,
        });
    }
}

/// Replication rounds at [`Pace`], on the wall clock.
fn tick(
    time: Res<Time<Real>>,
    pace: Res<Pace>,
    mut acc: Local<f32>,
    mut server_tick: ResMut<ServerTick>,
) {
    *acc += time.delta_secs();
    if *acc >= pace.interval {
        *acc = (*acc - pace.interval).min(pace.interval);
        server_tick.increment();
    }
}

/// A run of the session began here: the clients enter it on the same stage and options.
fn begin_run(
    mut commands: Commands,
    mut session: ResMut<Session>,
    selection: Res<Selection>,
    opts: Res<RunOptions>,
    run: Res<Run>,
    mut out: MessageWriter<ToClients<StartRun>>,
) {
    session.in_run = true;
    commands.spawn((
        Replicated,
        RunCarrier,
        run_of(&run, &Camp::default()),
        RunEntity,
    ));
    out.write(ToClients {
        targets: SendTargets::CLIENTS_ONLY,
        message: start_of(&selection, &opts),
    });
}

fn end_run(run: Res<Run>, mut out: MessageWriter<ToClients<RunPhase>>) {
    out.write(ToClients {
        targets: SendTargets::CLIENTS_ONLY,
        message: RunPhase::Over { won: run.won },
    });
}

/// Back at the menu: the clients go back to theirs, and nothing of the run replicates.
fn to_lobby(
    mut commands: Commands,
    mut session: ResMut<Session>,
    replicated: Query<Entity, With<Replicated>>,
    mut out: MessageWriter<ToClients<RunPhase>>,
) {
    session.in_run = false;
    for e in &replicated {
        commands.entity(e).try_remove::<Replicated>();
    }
    out.write(ToClients {
        targets: SendTargets::CLIENTS_ONLY,
        message: RunPhase::Lobby,
    });
}

/// The bonuses this machine's own hero runs on, as [`super::super::hud::start_run`] gives them.
pub(super) fn own_bonuses(meta: &MetaBonuses) -> super::super::bonus::Bonuses {
    let mut b = meta.0.clone();
    b.merge(&env_bonuses());
    b
}

/// Stop hosting: the clients are let go, their heroes leave, and nothing replicates.
pub(super) fn stop(world: &mut World) {
    use bevy_replicon_renet2::netcode::NetcodeServerTransport;
    use bevy_replicon_renet2::renet2::RenetServer;
    if let Some(mut server) = world.remove_resource::<RenetServer>() {
        if let Some(mut transport) = world.remove_resource::<NetcodeServerTransport>() {
            transport.disconnect_all(&mut server);
        }
    }
    let mut q = world.query_filtered::<Entity, Or<(With<Owner>, With<RunCarrier>)>>();
    let gone: Vec<Entity> = q.iter(world).collect();
    for e in gone {
        world.despawn(e);
    }
    let mut q = world.query_filtered::<Entity, With<Replicated>>();
    let tagged: Vec<Entity> = q.iter(world).collect();
    for e in tagged {
        world.entity_mut(e).remove::<Replicated>();
    }
    let mut session = world.resource_mut::<Session>();
    session.link = Link::Off;
    session.in_run = false;
    session.lobby.clear();
}
