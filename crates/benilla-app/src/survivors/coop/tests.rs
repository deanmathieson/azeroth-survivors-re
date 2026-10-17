//! Co-op's rules without a window: the per-player hand flow and the authority checks on their own,
//! then a host App and a client App in one process over real UDP on the loopback, with the
//! replication and messages the game uses.

use std::net::{Ipv4Addr, SocketAddr};

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::RepliconRenetPlugins;

use super::super::bonus::Bonuses;
use super::super::hand;
use super::super::hero;
use super::super::hero::Hero;
use super::super::hud::Choice;
use super::super::party::{self, HandDecision, LOCAL_PLAYER};
use super::super::partyui::PartyView;
use super::super::stable::CosmeticHat;
use super::super::talents::HeroSpec;
use super::super::units::{self, Look, UnitSpec};
use super::super::{data, HeroState, LocalHero, Phase, Rng, Run, Selection};
use super::client::{apply_build, fill_party_view, local_player, playable};
use super::host::{self, decide, owns, Owner, Peer};
use super::wire::*;
use super::{net, PlayerName, Session};
use crate::creature_anim::MovementState;
use crate::net::ObjectStore;

fn hero_of(player: u8, class: usize) -> HeroState {
    let mut h = HeroState::new(player, &data::CLASSES[class], 1, Bonuses::default());
    h.picks = 2;
    h
}

fn ranks(h: &HeroState) -> u32 {
    h.abilities.iter().map(|o| u32::from(o.level)).sum::<u32>()
        + h.passives.iter().map(|(_, l)| u32::from(*l)).sum::<u32>()
}

/// A hand only offers what its own hero can take: a class's new spells come from its own pool.
fn fits(c: Choice, h: &HeroState) -> bool {
    match c {
        Choice::NewAbility(def) => h.class.pool.contains(&def.key),
        Choice::Upgrade(k) => k < h.abilities.len(),
        Choice::UpPassive(k) => k < h.passives.len(),
        Choice::NewPassive(_) | Choice::Heal | Choice::Overcharge(_) | Choice::Jewel(_) => true,
    }
}

#[test]
fn each_player_is_dealt_and_picks_from_their_own_hand() {
    let mut run = Run::default();
    let mut rng = Rng(7);
    // Two players of different classes, each holding two picks.
    let mut mage = hero_of(1, 0);
    let mut priest = hero_of(2, 3);
    assert_ne!(mage.class.id, priest.class.id);
    let mut mage_hand = Some(super::super::hud::choices_for(&mage, &mut rng));
    let mut priest_hand = Some(super::super::hud::choices_for(&priest, &mut rng));
    assert!(mage_hand.as_ref().unwrap().iter().all(|&c| fits(c, &mage)));
    assert!(priest_hand
        .as_ref()
        .unwrap()
        .iter()
        .all(|&c| fits(c, &priest)));
    let before = (ranks(&mage), ranks(&priest));

    // The mage picks: only the mage's build grows, and its second hand comes at once.
    let next = decide(
        &mut mage,
        &mut mage_hand,
        HandDecision::Pick(0),
        &mut run,
        &mut rng,
    )
    .expect("a pick from a held hand");
    assert_eq!(ranks(&mage), before.0 + 1);
    assert_eq!(ranks(&priest), before.1);
    assert_eq!(mage.picks, 1);
    let wire = |v: &[Choice]| v.iter().map(|&c| WireChoice::of(c)).collect::<Vec<_>>();
    assert_eq!(mage_hand.as_deref().map(wire), Some(wire(&next)));
    assert!(!next.is_empty());

    // A slot never dealt changes nothing.
    assert!(decide(
        &mut mage,
        &mut mage_hand,
        HandDecision::Pick(9),
        &mut run,
        &mut rng
    )
    .is_none());

    // Its last pick closes its picker with an empty hand.
    let last = decide(
        &mut mage,
        &mut mage_hand,
        HandDecision::Pick(1),
        &mut run,
        &mut rng,
    )
    .unwrap();
    assert!(last.is_empty() && mage_hand.is_none() && mage.picks == 0);
    // No hand held: nothing to decide.
    assert!(decide(
        &mut mage,
        &mut mage_hand,
        HandDecision::Skip,
        &mut run,
        &mut rng
    )
    .is_none());

    // The priest's tools are the priest's own: a reroll spends one, a banish strikes the card.
    let rerolls = priest.rerolls;
    let rolled = decide(
        &mut priest,
        &mut priest_hand,
        HandDecision::Reroll,
        &mut run,
        &mut rng,
    )
    .unwrap();
    assert_eq!(priest.rerolls, rerolls - 1);
    assert_eq!(mage.rerolls, hand::REROLLS_PER_RUN);
    assert!(rolled.iter().all(|&c| fits(c, &priest)));
    if let Some(struck) = hand::struck(rolled[0], &priest) {
        decide(
            &mut priest,
            &mut priest_hand,
            HandDecision::Banish(0),
            &mut run,
            &mut rng,
        )
        .unwrap();
        assert_eq!(priest.banishes, hand::BANISHES_PER_RUN - 1);
        assert!(priest.banished.contains(&struck));
        assert!(mage.banished.is_empty());
    }
    // A skip pays the party's purse and spends the pick.
    let gold = run.gold;
    let after_skip = decide(
        &mut priest,
        &mut priest_hand,
        HandDecision::Skip,
        &mut run,
        &mut rng,
    )
    .unwrap();
    assert!(run.gold > gold);
    assert_eq!(priest.picks, 1);
    assert!(!after_skip.is_empty());
}

#[test]
fn players_are_numbered_and_only_drive_their_own_heroes() {
    assert_eq!(host::free_player(&[]), Some(1));
    assert_eq!(host::free_player(&[1, 3]), Some(2));
    assert_eq!(
        host::free_player(&[1, 2, 3, 4]),
        None,
        "a party of five is full"
    );

    let mut world = World::new();
    world.init_resource::<Messages<FromClient<MoveInput>>>();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    assert!(owns(Owner(a), ClientId::Client(a)));
    assert!(!owns(Owner(a), ClientId::Client(b)));
    assert!(!owns(Owner(a), ClientId::Server));
    let spawn = |world: &mut World, owner: Entity| {
        world
            .spawn((
                party::Remote,
                Owner(owner),
                Transform::default(),
                MovementState::default(),
            ))
            .id()
    };
    let (ha, hb) = (spawn(&mut world, a), spawn(&mut world, b));
    let host_hero = world
        .spawn((Transform::default(), MovementState::default()))
        .id();
    let to = |at: [f32; 3]| MoveInput {
        at,
        yaw: 1.0,
        pace: 7.0,
        flags: 1,
    };
    world.write_message(FromClient {
        client_id: ClientId::Client(a),
        message: to([5.0, 1.0, -2.0]),
    });
    // The host's own messages move no remote hero, and a bad float moves nothing.
    world.write_message(FromClient {
        client_id: ClientId::Server,
        message: to([9.0, 9.0, 9.0]),
    });
    world.write_message(FromClient {
        client_id: ClientId::Client(b),
        message: to([f32::NAN, 0.0, 0.0]),
    });
    world.run_system_once(host::apply_moves).unwrap();
    let at = |world: &World, e: Entity| world.get::<Transform>(e).unwrap().translation;
    assert_eq!(at(&world, ha), Vec3::new(5.0, 1.0, -2.0));
    assert_eq!(world.get::<MovementState>(ha).unwrap().speed, 7.0);
    assert_eq!(at(&world, hb), Vec3::ZERO);
    assert_eq!(at(&world, host_hero), Vec3::ZERO);
}

#[test]
fn camp_asks_are_host_validated_on_the_askers_own_hero() {
    use super::super::campfire::Camp;
    let mut world = World::new();
    world.init_resource::<Messages<FromClient<CampAction>>>();
    world.insert_resource(Rng(7));
    world.insert_resource(Run {
        breather: 30.0,
        gold: 60,
        ..Run::default()
    });
    world.init_resource::<Camp>();
    let a = world.spawn_empty().id();
    let b = world.spawn_empty().id();
    let ha = world.spawn((party::Remote, Owner(a), hero_of(1, 0))).id();
    let hb = world.spawn((party::Remote, Owner(b), hero_of(2, 0))).id();
    let rerolls = world.get::<HeroState>(ha).unwrap().rerolls;
    let ask = |world: &mut World, from: Entity, message: CampAction| {
        world.write_message(FromClient {
            client_id: ClientId::Client(from),
            message,
        });
    };
    // A purchase lands on the buyer's own hero, out of the shared purse; an unknown item and one
    // the purse cannot pay buy nothing.
    ask(&mut world, a, CampAction::Buy(0));
    ask(&mut world, b, CampAction::Buy(9));
    ask(&mut world, b, CampAction::Buy(3));
    ask(&mut world, b, CampAction::Ready);
    ask(&mut world, a, CampAction::Roll);
    // Off the dip, a strike catches nothing.
    ask(&mut world, a, CampAction::Fish);
    world.run_system_once(host::camp_actions).unwrap();
    assert_eq!(world.get::<HeroState>(ha).unwrap().rerolls, rerolls + 1);
    assert_eq!(world.get::<HeroState>(hb).unwrap().rerolls, rerolls);
    assert_eq!(world.resource::<Run>().gold, 60 - 25);
    let camp = world.resource::<Camp>().clone();
    assert_eq!(camp.ready, vec![2]);
    assert!(camp.fished.is_empty());
    assert_eq!(camp.roll.len(), 1);
    assert_eq!(camp.roll[0].0, 1, "the host rolled for player 1");
    // Once the camp is over the host hears no more.
    world.resource_mut::<Run>().breather = 0.0;
    ask(&mut world, a, CampAction::Buy(1));
    world.run_system_once(host::camp_actions).unwrap();
    assert_eq!(world.resource::<Run>().gold, 35);
}

#[test]
fn a_client_sees_the_camp_in_its_own_numbering() {
    use super::client::camp_of;
    let net = NetRun {
        camp_left: 20.0,
        camp_ready: vec![0, 2],
        camp_fished: vec![2],
        fishing_dip: 3.0,
        camp_rolls: vec![(0, 40), (2, 90)],
        camp_roll_winner: Some(2),
        camp_fire: Some([1.0, 2.0, 3.0]),
        ..NetRun::default()
    };
    // Player 2's machine: it is 0 here, the host 2.
    let camp = camp_of(&net, 2);
    assert_eq!(camp.ready, vec![0, 2]);
    assert_eq!(camp.fished, vec![LOCAL_PLAYER]);
    assert_eq!(camp.roll, vec![(0, 90), (2, 40)]);
    assert_eq!(camp.roll_winner, Some(LOCAL_PLAYER));
    assert_eq!(camp.fire, Some(Vec3::new(1.0, 2.0, 3.0)));
    assert_eq!(camp.fishing_dip, 3.0);
    // The host's own numbering is the session's.
    assert_eq!(camp_of(&net, 0).roll_winner, Some(2));
}

#[test]
fn every_machine_sees_itself_as_player_zero() {
    // Player 2's machine: it is 0 here, the host takes its 2, the others keep theirs.
    assert_eq!(local_player(2, 2), LOCAL_PLAYER);
    assert_eq!(local_player(0, 2), 2);
    assert_eq!(local_player(1, 2), 1);
    // The host's own numbering is the session's.
    assert_eq!(local_player(0, 0), 0);
    assert_eq!(local_player(3, 0), 3);
}

#[test]
fn a_build_and_its_hand_arrive_together() {
    let mut host_hero = hero_of(1, 0);
    super::super::hud::take(&mut host_hero, Choice::NewPassive(&data::PASSIVES[0]));
    host_hero.abilities[0].level = 3;
    let hand = host::hand_for(
        &host_hero,
        &[Choice::Upgrade(0), Choice::UpPassive(0), Choice::Heal],
    );
    // A mirror that has not caught up would offer slots it lacks.
    let mut mirror = hero_of(1, 0);
    // Upgrade(0) (the start spell) and Heal are there; UpPassive(0) is not yet.
    assert_eq!(playable(&hand, &mirror).len(), 2);
    apply_build(&mut mirror, &hand.build);
    assert_eq!(playable(&hand, &mirror).len(), 3);
    assert_eq!(mirror.abilities[0].level, 3);
    assert_eq!(mirror.passives.len(), 1);
    assert_eq!(mirror.picks, host_hero.picks);
}

#[test]
fn every_enemy_kind_is_found_again_in_the_roster() {
    for stage in 0..data::MAPS.len() {
        let roster = data::roster(stage);
        for (i, def) in roster.enemies.iter().enumerate() {
            let (boss, idx) = host::roster_index(stage, def);
            assert!(!boss);
            assert_eq!(
                roster.enemies[usize::from(idx)].name,
                roster.enemies[i].name
            );
        }
        for (i, def) in roster.bosses.iter().enumerate() {
            assert_eq!(host::roster_index(stage, def), (true, i as u16));
        }
    }
}

/// A headless app with replicon over renet2, ticking every update, and the game's protocol.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        RepliconPlugins.set(ServerPlugin::new(PostUpdate)),
        RepliconRenetPlugins,
    ));
    super::protocol(&mut app);
    app.init_resource::<Session>()
        .init_resource::<PlayerName>()
        .init_resource::<Selection>()
        .init_resource::<Run>()
        .insert_resource(Rng(11));
    app.finish();
    app
}

/// How long [`pump`] waits on the loopback: generous, since `cargo test` runs every test at once
/// and a loaded machine updates both apps slowly. A healthy session gets anywhere in well under
/// a second.
const PUMP_DEADLINE: std::time::Duration = std::time::Duration::from_secs(60);

/// Both apps a round, a moment apart, until `done` holds; panics once [`PUMP_DEADLINE`] of wall
/// clock has passed without it.
fn pump(host: &mut App, client: &mut App, mut done: impl FnMut(&mut App, &mut App) -> bool) {
    let start = std::time::Instant::now();
    while start.elapsed() < PUMP_DEADLINE {
        client.update();
        host.update();
        if done(host, client) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    panic!(
        "the loopback session never got there in {:?}",
        PUMP_DEADLINE
    );
}

fn drain<M: Message>(app: &mut App) -> Vec<M> {
    app.world_mut()
        .resource_mut::<Messages<M>>()
        .drain()
        .collect()
}

#[test]
fn a_deathcharger_and_tiny_party_hat_reach_the_host_and_client_mirror_over_loopback() {
    let mut host_app = app();
    let mut client_app = app();
    host_app.init_state::<Phase>();
    host_app.add_systems(
        PreUpdate,
        (host::apply_moves, host::hand_actions, host::apply_self_res).after(ServerSystems::Receive),
    );
    host_app.add_systems(
        Update,
        (
            host::serve_hands,
            host::tag,
            host::sync_heroes,
            host::sync_slow,
        ),
    );

    // Host on a free port; join it on 127.0.0.1.
    let channels = host_app.world().resource::<RepliconChannels>();
    let (server, transport, port) = net::host(channels, 0).expect("binds the loopback");
    host_app.insert_resource(server).insert_resource(transport);
    host_app.world_mut().resource_mut::<Session>().in_run = true;
    let to = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    let channels = client_app.world().resource::<RepliconChannels>();
    let (client, transport) = net::join(channels, to).expect("binds a client socket");
    client_app
        .insert_resource(client)
        .insert_resource(transport);
    pump(&mut host_app, &mut client_app, |h, c| {
        *c.world().resource::<State<ClientState>>() == ClientState::Connected
            && h.world_mut()
                .query::<&AuthorizedClient>()
                .iter(h.world())
                .count()
                == 1
    });
    let peer = host_app
        .world_mut()
        .query_filtered::<Entity, With<ConnectedClient>>()
        .single(host_app.world())
        .unwrap();

    // The host spawns its own hero and the client's: both replicate, with who they are.
    let at = Vec3::new(120.0, 30.0, -45.5);
    let world = host_app.world_mut();
    let mut run = Run::default();
    let peer_info = Peer {
        player: 1,
        name: "Jaina".into(),
        class: 3,
        race: 1,
        sex: 1,
        trainer: 3000,
        looks: NetLooks {
            features: Some(super::super::gear::Features::from_array([2, 1, 3, 4, 5])),
            gear: [11, 12, 13, 14, 15, 16, 17, 18, 19],
        },
        bonuses: Bonuses::default(),
        talents: vec![(61, 5), (71, 1)],
        // Deathcharger's display and the Tiny Party Hat's resolved model travel together.
        mount_display: 10718,
        mount_hat: Some(CosmeticHat {
            display: 1234,
            mount_nudge: [0.0, 0.0, 0.0],
            offset: [0.0, -0.08, 0.0],
            pitch: 0.0,
            scale: 1.0,
        }),
        hero: None,
    };
    let (host_hero, theirs) = {
        let mut commands = world.commands();
        let mine = hero::spawn_hero(&mut commands, hero_of(0, 0), 2000, 0, 7, at);
        let theirs = host::spawn_peer_hero(
            &mut commands,
            &mut run,
            peer,
            &peer_info,
            at + Vec3::X * 3.0,
        );
        (mine, theirs)
    };
    world.flush();
    assert_eq!(
        world.get::<HeroSpec>(theirs).map(|s| s.0.title),
        Some("Frost Mage")
    );
    world.get_mut::<HeroState>(theirs).unwrap().picks = 1;
    let heroes = |c: &mut App| {
        c.world_mut()
            .query::<(&NetHero, &NetPos, &NetVitals, &Look)>()
            .iter(c.world())
            .map(|(h, p, v, l)| (h.clone(), *p, *v, l.clone()))
            .collect::<Vec<_>>()
    };
    // The host deals the banked pick as soon as the hero stands, so its hand may land while the
    // session is still getting there: kept from every round, as a message is only held for two
    // updates, and a loaded machine takes many before the hand is looked for.
    let mut hands: Vec<Hand> = Vec::new();
    pump(&mut host_app, &mut client_app, |_, c| {
        hands.extend(drain::<Hand>(c));
        let seen = heroes(c);
        seen.len() == 2 && seen.iter().all(|h| h.2.hp > 0.0)
    });
    let seen = heroes(&mut client_app);
    let mine = seen.iter().find(|h| h.0.player == 0).unwrap();
    assert!(mine.1.pos().distance(at) < 0.01, "{:?}", mine.1.pos());
    assert!(mine.2.hp > 0.0 && mine.2.hp == mine.2.max_hp);
    assert!(matches!(mine.3, Look::Unit { display: 2000, .. }));
    let jaina = seen.iter().find(|h| h.0.player == 1).unwrap();
    assert_eq!((jaina.0.name.as_str(), jaina.0.trainer), ("Jaina", 3000));
    assert_eq!(
        host_app.world().get::<NetLooks>(theirs),
        Some(&peer_info.looks),
        "the host keeps the client's own appearance"
    );
    assert_eq!(jaina.0.mount_display, 10718);
    assert_eq!(
        jaina.0.mount_hat.map(|hat| hat.offset),
        Some([0.0, -0.08, 0.0])
    );
    assert_eq!(
        host_app
            .world()
            .get::<NetHero>(theirs)
            .unwrap()
            .mount_display,
        10718
    );
    assert!(host_app.world().get::<Replicated>(host_hero).is_some());

    // A spec guardian is an ordinary run unit: its Look and NetPos make it to every client,
    // and despawning it on the host removes its client body with the owner/spec.
    let guardian = {
        let mut commands = host_app.world_mut().commands();
        units::spawn_unit(
            &mut commands,
            &UnitSpec {
                serial: 99,
                display: 510,
                scale: 1.0,
                level: 60,
                faction: units::FACTION_FRIENDLY,
                bytes0: 1 << 8,
                weapons: &[],
                sheath: 0,
            },
            at + Vec3::Z * 4.0,
            0.0,
        )
    };
    host_app.world_mut().flush();
    pump(&mut host_app, &mut client_app, |_, c| {
        c.world_mut()
            .query::<(&Look, &NetPos)>()
            .iter(c.world())
            .any(|(look, pos)| {
                matches!(look, Look::Unit { display: 510, .. })
                    && pos.pos().distance(at + Vec3::Z * 4.0) < 0.01
            })
    });
    host_app.world_mut().despawn(guardian);
    pump(&mut host_app, &mut client_app, |_, c| {
        !c.world_mut()
            .query::<&Look>()
            .iter(c.world())
            .any(|look| matches!(look, Look::Unit { display: 510, .. }))
    });

    // The client walks its hero: the host moves the client's hero, and only that one.
    let walked = [130.0, 31.0, -40.0];
    pump(&mut host_app, &mut client_app, |h, c| {
        hands.extend(drain::<Hand>(c));
        c.world_mut().write_message(MoveInput {
            at: walked,
            yaw: 0.5,
            pace: 7.0,
            flags: 1,
        });
        h.world().get::<Transform>(theirs).unwrap().translation == Vec3::from_array(walked)
    });
    assert_eq!(
        host_app
            .world()
            .get::<Transform>(host_hero)
            .unwrap()
            .translation,
        at
    );

    // The host deals the client's banked pick as a hand; the client's pick comes back and lands
    // on its hero, and the picker closes with an empty hand.
    pump(&mut host_app, &mut client_app, |_, c| {
        hands.extend(drain::<Hand>(c));
        !hands.is_empty()
    });
    let hand = hands.remove(0);
    assert!(!hand.choices.is_empty() && hand.build.picks == 1);
    let before = ranks(host_app.world().get::<HeroState>(theirs).unwrap());
    client_app
        .world_mut()
        .write_message(HandAction(WireDecision::Pick(0)));
    let mut closed = false;
    pump(&mut host_app, &mut client_app, |_, c| {
        closed |= drain::<Hand>(c).iter().any(|h| h.choices.is_empty());
        closed
    });
    let after = host_app.world().get::<HeroState>(theirs).unwrap();
    assert_eq!(after.picks, 0);
    assert!(
        ranks(after) == before + 1 || after.hp == after.stats().max_hp,
        "the pick landed"
    );
    assert_eq!(
        host_app.world().get::<HeroState>(host_hero).unwrap().picks,
        2,
        "the host's hero is untouched"
    );

    // The party UI's numbers and buffs reach the client at their slower pace.
    {
        let mut h = host_app.world_mut().get_mut::<HeroState>(theirs).unwrap();
        h.damage_done = 1234.5;
        h.gold_collected = 7;
        h.times_downed = 1;
    }
    let stats = |c: &mut App| {
        c.world_mut()
            .query::<(&NetHero, &NetStats, &NetAuras, &NetBuffs)>()
            .iter(c.world())
            .find(|(h, ..)| h.player == 1)
            .map(|(_, s, ..)| *s)
    };
    pump(&mut host_app, &mut client_app, |_, c| {
        stats(c).is_some_and(|s| s.damage_done == 1234.5)
    });
    let seen = stats(&mut client_app).unwrap();
    assert_eq!((seen.gold, seen.downs, seen.revives), (7, 1, 0));

    // Down, the client's player presses its self-resurrection key: the host is asked to raise its
    // hero, and no other.
    for e in [host_hero, theirs] {
        let mut h = host_app.world_mut().get_mut::<HeroState>(e).unwrap();
        h.dead_t = Some(0.5);
        h.hp = 0.0;
    }
    pump(&mut host_app, &mut client_app, |h, c| {
        c.world_mut().write_message(SelfRes);
        h.world()
            .get::<HeroState>(theirs)
            .unwrap()
            .synergy
            .res
            .asked
    });
    assert!(
        !host_app
            .world()
            .get::<HeroState>(host_hero)
            .unwrap()
            .synergy
            .res
            .asked
    );

    // The client leaves: the host sees the connection go.
    client_app
        .world_mut()
        .resource_mut::<bevy_replicon_renet2::renet2::RenetClient>()
        .disconnect();
    pump(&mut host_app, &mut client_app, |h, _| {
        h.world_mut()
            .query::<&ConnectedClient>()
            .iter(h.world())
            .count()
            == 0
    });
}

#[test]
fn co_op_never_waits_and_remote_players_bank_jewelry_and_catch_up() {
    let mut world = World::new();
    world.insert_resource(Rng(3));
    world.init_resource::<Messages<ToClients<Hand>>>();
    world.init_resource::<super::super::hud::Offer>();
    world.insert_resource(State::new(super::super::Phase::LevelUp));
    world.init_resource::<NextState<super::super::Phase>>();
    world.insert_resource(super::super::strip::CoopRun(true));
    let mut run = Run {
        level: 6,
        ..Run::default()
    };
    let client = world.spawn_empty().id();
    let peer = Peer {
        player: 1,
        name: "Thrall".into(),
        class: 2,
        race: 2,
        sex: 0,
        trainer: 4000,
        looks: NetLooks::default(),
        bonuses: Bonuses::default(),
        talents: Vec::new(),
        mount_display: 0,
        mount_hat: None,
        hero: None,
    };
    // Joining at level 6, it banks five hands; and one jewelry choice from a cache.
    let remote = {
        let mut commands = world.commands();
        host::spawn_peer_hero(&mut commands, &mut run, client, &peer, Vec3::ZERO)
    };
    world.flush();
    world.insert_resource(run);
    assert_eq!(world.get::<HeroState>(remote).unwrap().picks, 5);
    world.get_mut::<HeroState>(remote).unwrap().jewelry.picks = 1;
    // In co-op a level-up holds for no one: play resumes with the hands banked.
    world.run_system_once(party::wait_for_party).unwrap();
    assert!(matches!(
        *world.resource::<NextState<super::super::Phase>>(),
        NextState::Pending(super::super::Phase::Playing)
    ));
    // Its first hand is the jewelry choice, sent like any hand, and picking one spends it.
    world.run_system_once(host::serve_hands).unwrap();
    let held = world
        .get::<host::HeldHand>(remote)
        .unwrap()
        .0
        .clone()
        .unwrap();
    assert!(held.iter().all(|c| matches!(c, Choice::Jewel(_))));
    {
        let mut held = world.get_mut::<host::HeldHand>(remote).unwrap().0.take();
        let (mut run, mut rng) = (Run::default(), Rng(1));
        let mut hero = world.get_mut::<HeroState>(remote).unwrap();
        assert!(
            decide(
                &mut hero,
                &mut held,
                HandDecision::Reroll,
                &mut run,
                &mut rng
            )
            .is_none(),
            "a jewelry hand is not rerolled"
        );
        let next = decide(
            &mut hero,
            &mut held,
            HandDecision::Pick(0),
            &mut run,
            &mut rng,
        )
        .unwrap();
        assert_eq!((hero.jewelry.picks, hero.picks), (0, 5));
        assert_eq!(hero.jewelry.worn().count(), 1);
        assert!(!next.is_empty() && next.iter().all(|c| !matches!(c, Choice::Jewel(_))));
        world.get_mut::<host::HeldHand>(remote).unwrap().0 = held;
    }
    // Going down puts the hand away, and its client is told.
    world.get_mut::<HeroState>(remote).unwrap().dead_t = Some(0.0);
    world.run_system_once(host::serve_hands).unwrap();
    assert!(world.get::<host::HeldHand>(remote).unwrap().0.is_none());
    let sent: Vec<_> = world
        .resource_mut::<Messages<ToClients<Hand>>>()
        .drain()
        .collect();
    assert_eq!(sent.len(), 2);
    assert!(matches!(sent[0].message.choices[0], WireChoice::Jewel(_)));
    assert_eq!(sent[0].message.build.jewels, 1);
    assert!(sent[1].message.choices.is_empty());
}

#[test]
fn a_client_on_another_build_is_turned_away_with_a_reason() {
    let mut world = World::new();
    world.init_resource::<Session>();
    world.init_resource::<Selection>();
    world.init_resource::<super::super::run_options::RunOptions>();
    world.insert_resource(PlayerName("Host".into()));
    world.init_resource::<Messages<FromClient<Hello>>>();
    world.init_resource::<Messages<ToClients<Welcome>>>();
    world.init_resource::<Messages<ToClients<StartRun>>>();
    world.init_resource::<Messages<ToClients<Refused>>>();
    world.init_resource::<Messages<DisconnectRequest>>();
    let ours = super::build_tag(None);
    let hello = |build: &str| Hello {
        name: "Jaina".into(),
        class: 0,
        race: 1,
        sex: 1,
        trainer: 3000,
        looks: NetLooks::default(),
        bonuses: Bonuses::default(),
        talents: vec![(36, 1)],
        mount_display: 2404,
        mount_hat: Some(CosmeticHat {
            display: 1234,
            mount_nudge: [0.0, 0.2, -0.1],
            offset: [0.0; 3],
            pitch: 0.0,
            scale: 1.0,
        }),
        build: build.into(),
    };
    let (stale, fresh) = (world.spawn_empty().id(), world.spawn_empty().id());
    world.write_message(FromClient {
        client_id: ClientId::Client(stale),
        message: hello("p1-0.1.0-deadbee"),
    });
    world.write_message(FromClient {
        client_id: ClientId::Client(fresh),
        message: hello(&ours),
    });
    world.run_system_once(host::greet).unwrap();
    let refused: Vec<_> = world
        .resource_mut::<Messages<ToClients<Refused>>>()
        .drain()
        .collect();
    assert_eq!(refused.len(), 1);
    assert_eq!(
        refused[0].message.reason,
        "Jaina is on a different build; everyone needs the same zip."
    );
    let cut: Vec<_> = world
        .resource_mut::<Messages<DisconnectRequest>>()
        .drain()
        .collect();
    assert_eq!(cut.len(), 1);
    assert_eq!(cut[0].client, stale);
    assert!(world.get::<Peer>(stale).is_none());
    assert_eq!(world.get::<Peer>(fresh).map(|p| p.player), Some(1));
    assert_eq!(
        world.get::<Peer>(fresh).map(|p| p.talents.as_slice()),
        Some(&[(36, 1)][..])
    );
    let welcomed: Vec<_> = world
        .resource_mut::<Messages<ToClients<Welcome>>>()
        .drain()
        .collect();
    assert_eq!(welcomed.len(), 1);
    assert_eq!(welcomed[0].message.build, ours);
    assert_eq!(world.resource::<Session>().status, super::mismatch("Jaina"));
    // A party of five: four clients.
    assert_eq!(host::free_player(&[1, 2, 3]), Some(4));
    assert_eq!(host::free_player(&[1, 2, 3, 4]), None);
    assert_eq!(party::scale(5), 3.4);
}

#[test]
fn a_client_fills_the_party_view_from_the_replicated_heroes() {
    let mut world = World::new();
    world.insert_resource(Run {
        level: 6,
        elapsed: 90.0,
        ..Default::default()
    });
    world.init_resource::<Selection>();
    world.init_resource::<PartyView>();
    let net = |player: u8, name: &str| NetHero {
        player,
        name: name.into(),
        class: 0,
        race: 1,
        sex: 0,
        trainer: 0,
        mount_display: 0,
        mount_hat: None,
    };
    let vitals = |max_hp: f32| NetVitals {
        hp: max_hp,
        max_hp,
        ..Default::default()
    };
    // This machine is player 2: its own mirror is local player 0, the host's is 2.
    let mut mine = hero_of(local_player(2, 2), 0);
    mine.damage_done = 50.0;
    let mut hosts = hero_of(local_player(0, 2), 1);
    hosts.damage_done = 75.0;
    hosts.revives_given = 1;
    let me = world
        .spawn((
            Hero::default(),
            LocalHero,
            mine,
            Transform::default(),
            net(2, "Thrall"),
            vitals(180.0),
        ))
        .id();
    let host = world
        .spawn((
            Hero::default(),
            hosts,
            Transform::default(),
            net(0, "Jaina"),
            vitals(240.0),
        ))
        .id();
    world.run_system_once(fill_party_view).unwrap();
    let view = world.resource::<PartyView>();
    // In the session's order, by the players' names, at the host's max health and numbers.
    let order: Vec<(Entity, u8, &str)> = view
        .members
        .iter()
        .map(|m| (m.entity, m.player, m.name.as_str()))
        .collect();
    assert_eq!(order, [(host, 0, "Jaina"), (me, 2, "Thrall")]);
    assert_eq!(view.members[0].max_hp, 240.0);
    assert_eq!(view.members[0].level, 6);
    assert_eq!(
        (view.members[0].damage_done, view.members[0].revives),
        (75.0, 1)
    );
    assert!(view.members[1].is_local && !view.members[0].is_local);
    assert_eq!(view.elapsed, 90.0);
    assert_eq!(view.allies().len(), 1);
}

#[test]
fn a_client_applies_a_remote_heros_mount_display() {
    let mut world = World::new();
    let remote = world
        .spawn((
            ObjectStore::default(),
            NetHero {
                player: 1,
                name: "Thrall".into(),
                class: 7,
                race: 2,
                sex: 0,
                trainer: 0,
                mount_display: 2404,
                mount_hat: Some(CosmeticHat {
                    display: 1234,
                    mount_nudge: [0.0, 0.2, -0.1],
                    offset: [0.0; 3],
                    pitch: 0.0,
                    scale: 1.0,
                }),
            },
        ))
        .id();
    world
        .run_system_once(super::super::stable::apply_remote_mount_field)
        .unwrap();
    assert_eq!(
        world
            .entity(remote)
            .get::<ObjectStore>()
            .unwrap()
            .0
            .unit_mount_display_id(),
        2404
    );
}

#[test]
fn host_applies_a_clients_cosmetics_to_its_hero_only() {
    let mut world = World::new();
    world.init_resource::<Messages<FromClient<Cosmetics>>>();
    let owner = world.spawn_empty().id();
    let mine = world
        .spawn(NetHero {
            player: 0,
            name: "Host".into(),
            class: 0,
            race: 1,
            sex: 0,
            trainer: 0,
            mount_display: 2404,
            mount_hat: None,
        })
        .id();
    let theirs = world
        .spawn(NetHero {
            player: 1,
            name: "Jaina".into(),
            class: 0,
            race: 1,
            sex: 0,
            trainer: 0,
            mount_display: 0,
            mount_hat: None,
        })
        .id();
    world.entity_mut(owner).insert(Peer {
        player: 1,
        name: "Jaina".into(),
        class: 0,
        race: 1,
        sex: 0,
        trainer: 0,
        looks: NetLooks::default(),
        bonuses: Bonuses::default(),
        talents: Vec::new(),
        mount_display: 0,
        mount_hat: None,
        hero: Some(theirs),
    });
    let hat = CosmeticHat {
        display: 1234,
        mount_nudge: [0.0, 0.2, -0.1],
        offset: [0.0; 3],
        pitch: 0.0,
        scale: 1.0,
    };
    world.write_message(FromClient {
        client_id: ClientId::Client(owner),
        message: Cosmetics {
            mount_display: 10718,
            mount_hat: Some(hat),
        },
    });
    world.run_system_once(host::update_cosmetics).unwrap();
    assert_eq!(world.get::<NetHero>(mine).unwrap().mount_display, 2404);
    assert_eq!(world.get::<NetHero>(theirs).unwrap().mount_display, 10718);
    assert_eq!(world.get::<NetHero>(theirs).unwrap().mount_hat, Some(hat));
}
