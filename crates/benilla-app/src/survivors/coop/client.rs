//! The client's half: it shows the host's run and simulates none of it. Every replicated body is
//! built from its look, everything but this player's own hero eases toward the host's place, and
//! the host's spell visuals and numbers play here. This player's hero walks here, and its place
//! goes to the host ~30 times a second; level-up hands come from the host and the choices go back,
//! as does the self-resurrection key. The party UI's view is filled here from the replicated
//! heroes ([`fill_party_view`]), as the host fills its own from its heroes.

use bevy::prelude::*;
use bevy_replicon::prelude::{ClientState, ClientSystems};

use super::super::campfire::{Camp, CampAsk, CampRequest};
use super::super::data;
use super::super::enemies::{self, Enemy};
use super::super::gear::{wear, Dresser, GearRead, HeroNames, Slot};
use super::super::hero::Hero;
use super::super::hud::{Choice, Offer};
use super::super::party::{self, HandOutbox};
use super::super::partyui::{self, FillPartyView, HeroRead, PartyView};
use super::super::profile::Profile;
use super::super::raid::Yelled;
use super::super::run_options::RunOptions;
use super::super::synergy::{self, PartyBuff};
use super::super::talents;
use super::super::units::{self, Look, Marks};
use super::super::{HeroState, LocalHero, MetaBonuses, Owned, Phase, Run, Selection};
use super::host::own_bonuses;
use super::wire::*;
use super::{Link, Pace, PlayerName, Session};
use crate::combat_text::CombatTextSpawn;
use crate::creature_anim::{
    CastEvent, CastEventKind, MovementState, PlaySeq, SpellGoTargets, SwingMessage,
};
use crate::net::{FieldChanged, Guid, ObjectStore};

/// A gap past which a body cuts to the host's place instead of easing there, in yards.
const SNAP: f32 = 12.0;
/// The ease toward the host's place (s): about a replication round and a half.
const EASE_TAU: f32 = 0.05;

/// A start the host sent: the guard on solo starts lets it through.
#[derive(Resource, Default)]
pub(super) struct HostStart(bool);

pub(super) fn joined(session: Res<Session>) -> bool {
    matches!(session.link, Link::Joined { .. })
}

fn linked(session: Res<Session>) -> bool {
    matches!(session.link, Link::Joining { .. } | Link::Joined { .. })
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<HostStart>()
        .add_systems(
            PreUpdate,
            (
                watch_link,
                (welcome, lobby, start, run_phase).run_if(joined_or_joining),
                (
                    materialize,
                    redress,
                    mirror_heroes,
                    mirror_auras,
                    enemy_life,
                    copy_run,
                    receive_hands,
                    play_fx,
                    receive_yells,
                )
                    .chain()
                    .run_if(joined),
            )
                .chain()
                .after(ClientSystems::Receive)
                .run_if(linked),
        )
        .add_systems(
            Update,
            (
                interpolate,
                hold_solo_starts
                    .after(super::super::hud::keys)
                    .after(super::super::menu::menu_clicks),
                fill_party_view.in_set(FillPartyView),
            )
                .run_if(linked),
        )
        .add_systems(OnEnter(Phase::Menu), quit_run.run_if(joined))
        .add_systems(
            PostUpdate,
            (
                send_cosmetics,
                send_move,
                send_hand,
                send_focus,
                send_self_res,
                send_camp,
            )
                .before(ClientSystems::Send)
                .run_if(joined),
        );
}

/// A replicated look can arrive after the body. Replacing only the display keeps the existing
/// mirrored hero and its interpolation state intact.
#[allow(clippy::type_complexity)]
fn redress(
    mut dresser: Dresser,
    mut heroes: Query<
        (
            &NetHero,
            &NetLooks,
            &mut crate::net::NetEntity,
            &mut ObjectStore,
        ),
        (Changed<NetLooks>, With<bevy_replicon::prelude::Remote>),
    >,
) {
    for (hero, looks, mut net, mut store) in &mut heroes {
        let display = dresser.display_remote(
            usize::from(hero.class).min(data::CLASSES.len() - 1),
            (hero.trainer, hero.race, hero.sex),
            looks,
        );
        wear(display, &mut net, &mut store);
    }
}

/// While joined, publish a changed Stable choice once.  `Hello` only exists to establish a
/// session; a lobby player can change cosmetics long after it has stopped being retried.
fn send_cosmetics(
    profile: Option<Res<Profile>>,
    displays: Option<Res<crate::entities::ItemDisplays>>,
    mut sent: Local<Option<(u32, Option<super::super::stable::CosmeticHat>)>>,
    mut out: MessageWriter<Cosmetics>,
) {
    let cosmetics = profile
        .as_deref()
        .map(|profile| super::super::stable::selected_cosmetics(profile, displays.as_deref()))
        .unwrap_or((0, None));
    if sent.as_ref() == Some(&cosmetics) {
        return;
    }
    *sent = Some(cosmetics);
    out.write(Cosmetics {
        mount_display: cosmetics.0,
        mount_hat: cosmetics.1,
    });
}

fn joined_or_joining(session: Res<Session>) -> bool {
    matches!(session.link, Link::Joining { .. } | Link::Joined { .. })
}

/// Connected: say hello, again each second until welcomed (a hello may beat the handshake).
/// Disconnected after that, or never connected: the session is over.
#[allow(clippy::too_many_arguments)]
fn watch_link(
    mut commands: Commands,
    time: Res<Time<Real>>,
    state: Res<State<ClientState>>,
    mut seen: Local<(u32, Option<ClientState>, f32)>,
    session: Res<Session>,
    selection: Res<Selection>,
    meta: Res<MetaBonuses>,
    profile: Option<Res<Profile>>,
    displays: Option<Res<crate::entities::ItemDisplays>>,
    gear: GearRead,
    name: Res<PlayerName>,
    mut hello: MessageWriter<Hello>,
    build: Option<Res<crate::BuildId>>,
) {
    if seen.0 != session.attempt {
        *seen = (session.attempt, None, 0.0);
    }
    let now = *state.get();
    let before = seen.1.replace(now);
    // Keep the lobby identity fresh too: changing the character screen while waiting for the
    // host must change the body the host and the other clients render.
    if now == ClientState::Connected
        && matches!(session.link, Link::Joining { .. } | Link::Joined { .. })
    {
        seen.2 -= time.delta_secs();
        if before != Some(now) || seen.2 <= 0.0 {
            seen.2 = 1.0;
            let look = selection.look.and_then(|i| selection.looks.get(i));
            let (trainer, race, sex) = look.copied().unwrap_or((0, 0, 0));
            let cosmetics = profile
                .as_deref()
                .map(|p| super::super::stable::selected_cosmetics(p, displays.as_deref()))
                .unwrap_or((0, None));
            let class = selection.class.unwrap_or(0);
            let saved = gear.get(class);
            hello.write(Hello {
                name: name.0.clone(),
                class: class as u8,
                race,
                sex,
                trainer,
                looks: NetLooks {
                    features: saved.looks.features(race, sex),
                    gear: std::array::from_fn(|i| {
                        saved.worn(Slot::ALL[i]).map_or(0, |p| p.display_id)
                    }),
                },
                bonuses: own_bonuses(&meta),
                talents: selection
                    .class
                    .and_then(|class| profile.as_deref().and_then(|p| talents::saved(p, class)))
                    .map_or_else(Vec::new, |saved| saved.ranks.clone()),
                mount_display: cosmetics.0,
                mount_hat: cosmetics.1,
                pinned_trinket: profile
                    .as_deref()
                    .and_then(|p| p.faire.wardrobe.pinned_trinket.clone()),
                build: super::build_tag(build.as_deref()),
            });
        }
    }
    if now == ClientState::Disconnected && before.is_some_and(|b| b != now) {
        let why = if matches!(session.link, Link::Joined { .. }) {
            "The connection to the host was lost"
        } else {
            "Could not reach the host"
        };
        let why = why.to_string();
        commands.queue(move |world: &mut World| leave(world, why));
    }
}

fn welcome(
    mut commands: Commands,
    mut msgs: MessageReader<Welcome>,
    mut refused: MessageReader<Refused>,
    mut session: ResMut<Session>,
    build: Option<Res<crate::BuildId>>,
) {
    // Turned away by the host, or the host on another build: this end leaves, saying why.
    let welcomes: Vec<Welcome> = msgs.read().cloned().collect();
    let ours = super::build_tag(build.as_deref());
    let why = refused.read().last().map(|r| r.reason.clone()).or_else(|| {
        welcomes
            .iter()
            .find(|w| w.build != ours)
            .map(|w| super::mismatch(&w.host))
    });
    if let Some(why) = why {
        commands.queue(move |world: &mut World| leave(world, why));
        return;
    }
    for w in &welcomes {
        session.me = w.player;
        if let Link::Joining { addr } = &session.link {
            let addr = addr.clone();
            info!("survivors: co-op: joined {addr} as player {}", w.player + 1);
            session.link = Link::Joined { addr };
            session.status.clear();
        }
    }
}

fn lobby(mut msgs: MessageReader<Lobby>, mut session: ResMut<Session>) {
    for l in msgs.read() {
        session.lobby = l.players.clone();
    }
}

/// The host started a run: this machine enters the same stage under the same options, its menu
/// hero gives way to the one the host spawns for it.
#[allow(clippy::too_many_arguments)]
fn start(
    mut commands: Commands,
    mut msgs: MessageReader<StartRun>,
    mut session: ResMut<Session>,
    mut selection: ResMut<Selection>,
    mut opts: ResMut<RunOptions>,
    mut run: ResMut<Run>,
    mut next: ResMut<NextState<Phase>>,
    mut allow: ResMut<HostStart>,
    locals: Query<Entity, (With<Hero>, Without<bevy_replicon::prelude::Remote>)>,
) {
    let Some(s) = msgs.read().last().copied() else {
        return;
    };
    let stage = usize::from(s.stage).min(data::MAPS.len() - 1);
    selection.map = stage;
    selection.stage = stage;
    [opts.density, opts.health, opts.damage, opts.speed] = s.options;
    let next_guid = run.next_guid;
    *run = Run {
        next_guid,
        reward_mult: opts.reward_mult(),
        ..Run::default()
    };
    run.version += 1;
    for e in &locals {
        commands.entity(e).try_despawn();
    }
    session.in_run = true;
    allow.0 = true;
    next.set(Phase::Playing);
}

fn run_phase(
    mut msgs: MessageReader<RunPhase>,
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut run: ResMut<Run>,
    mut session: ResMut<Session>,
) {
    for m in msgs.read() {
        session.in_run = false;
        let at = *phase.get();
        match *m {
            RunPhase::Over { won } => {
                run.won = won;
                if !matches!(at, Phase::Menu | Phase::GameOver) {
                    next.set(Phase::GameOver);
                }
            }
            RunPhase::Lobby => {
                if !matches!(at, Phase::Menu | Phase::GameOver) {
                    session.status = "The host went back to the menu".into();
                    next.set(Phase::Menu);
                }
            }
        }
    }
}

/// On this machine, player `net` of the session is player [`party::LOCAL_PLAYER`] when it is this
/// machine's own; the host's number goes to this machine's, so every machine's own is 0.
pub(super) fn local_player(net: u8, me: u8) -> u8 {
    if net == me {
        party::LOCAL_PLAYER
    } else if net == party::LOCAL_PLAYER {
        me
    } else {
        net
    }
}

/// `build` into the mirror `hero`, keeping the cooldowns of the spells it still has.
pub(super) fn apply_build(hero: &mut HeroState, build: &NetBuild) {
    let old: Vec<(&'static str, f32)> = hero.abilities.iter().map(|o| (o.def.key, o.cd)).collect();
    hero.abilities = build
        .abilities
        .iter()
        .filter_map(|(key, level)| {
            let def = ability(key)?;
            let cd = old.iter().find(|o| o.0 == def.key).map_or(0.0, |o| o.1);
            Some(Owned {
                def,
                level: (*level).max(1),
                cd,
            })
        })
        .collect();
    hero.passives = build
        .passives
        .iter()
        .filter_map(|(key, level)| Some((passive(key)?, (*level).max(1))))
        .collect();
    hero.bonuses = build.bonuses.clone();
    hero.picks = build.picks;
    hero.rerolls = build.rerolls;
    hero.banishes = build.banishes;
    hero.banished = build
        .banished
        .iter()
        .filter_map(WireStruck::struck)
        .collect();
    hero.overcharges = build.overcharges;
    hero.jewelry.picks = build.jewels;
}

/// The hero's numbers so far, into the mirror `hero`: the meter and the awards read them.
pub(super) fn apply_stats(hero: &mut HeroState, s: &NetStats) {
    hero.damage_done = s.damage_done;
    hero.healing_done = s.healing_done;
    hero.damage_taken = s.damage_taken;
    hero.revives_given = s.revives;
    hero.gold_collected = s.gold;
    hero.gold = s.gold;
    hero.times_downed = s.downs;
}

/// The group buffs on the hero, into the mirror `hero`: its stats count them, as on the host.
/// They last until the host says otherwise.
pub(super) fn apply_buffs(hero: &mut HeroState, buffs: &NetBuffs) {
    hero.synergy.buffs = buffs
        .0
        .iter()
        .filter_map(|&(i, strength)| {
            Some(PartyBuff {
                def: synergy::BUFFS.get(usize::from(i))?,
                strength,
                left: f32::INFINITY,
                caster: Entity::PLACEHOLDER,
            })
        })
        .collect();
}

/// Health, shield and down, into the mirror `hero`.
pub(super) fn apply_vitals(hero: &mut HeroState, v: &NetVitals, local: bool) {
    hero.hp = v.hp;
    hero.shield = v.shield;
    hero.shield_t = if v.shield > 0.0 { 1.0 } else { 0.0 };
    hero.revive_t = v.revive;
    // This player's own Focus is its strip's, here; the others' are the host's word.
    if !local {
        hero.focused = v.focus;
    }
    match (v.downed, hero.dead_t) {
        (true, None) => hero.dead_t = Some(0.0),
        (false, Some(_)) => hero.dead_t = None,
        _ => {}
    }
}

/// A replicated body arrives: it is built here from its look, a hero with its mirror state (this
/// player's own dressed in its own gear, and the one input and camera follow), an enemy with its
/// kind from the stage's roster.
#[allow(clippy::type_complexity)]
fn materialize(
    mut commands: Commands,
    session: Res<Session>,
    selection: Res<Selection>,
    fresh: Query<
        (
            Entity,
            &Look,
            Option<&NetPos>,
            Option<&NetHero>,
            Option<&NetLooks>,
            Option<&NetBuild>,
            Option<&NetEnemy>,
        ),
        (Added<Look>, With<bevy_replicon::prelude::Remote>),
    >,
    mut dresser: Dresser,
) {
    for (e, look, pos, hero, looks, build, enemy) in &fresh {
        let pos = pos.copied().unwrap_or_default();
        let mut look = look.clone();
        if let (Some(h), Look::Unit { display, .. }) = (hero, &mut look) {
            let class = usize::from(h.class).min(data::CLASSES.len() - 1);
            *display = looks.map_or(h.trainer, |looks| {
                dresser.display_remote(class, (h.trainer, h.race, h.sex), looks)
            });
        }
        let mut ec = commands.entity(e);
        units::dress(&mut ec, look, pos.pos(), pos.yaw());
        if let Some(h) = hero {
            let class = usize::from(h.class).min(data::CLASSES.len() - 1);
            let build = build.cloned().unwrap_or_default();
            let mut state = HeroState::new(
                local_player(h.player, session.me),
                &data::CLASSES[class],
                h.race,
                build.bonuses.clone(),
            );
            apply_build(&mut state, &build);
            ec.try_insert((Hero::default(), Marks::buffs(), state));
            if h.player == session.me {
                ec.try_insert(LocalHero);
            } else {
                ec.try_insert(party::Remote);
            }
        }
        if let Some(n) = enemy {
            let roster = data::roster(selection.stage);
            let list = if n.boss {
                &roster.bosses
            } else {
                &roster.enemies
            };
            let Some(def) = roster.kind(n.boss, usize::from(n.index)).or(list.first()) else {
                continue;
            };
            // Its shared debuffs and elite mark show as the host's do ([`mirror_auras`]).
            ec.try_insert(Marks::debuffs());
            ec.try_insert(Enemy {
                def,
                hp: n.max_hp,
                max_hp: n.max_hp,
                speed: def.speed,
                damage: 0.0,
                radius: 0.6 * def.scale,
                xp: 0,
                boss: n.boss,
                elite: n.elite,
                guardian: n.guardian,
                dormant: false,
                attack_cd: 0.0,
                slow: 1.0,
                slow_t: 0.0,
                root_t: 0.0,
                stun_t: 0.0,
                fear_t: 0.0,
                weaken: 1.0,
                weaken_t: 0.0,
                dots: Vec::new(),
                dead: false,
                taunted: None,
                debuffs: Default::default(),
            });
        }
    }
}

/// The heroes' vitals, builds, group buffs and numbers into their mirrors. A hero going down
/// plays its death; one revived on the host gets up. Its glows are its auras ([`mirror_auras`]).
#[allow(clippy::type_complexity)]
fn mirror_heroes(
    mut heroes: Query<(
        Ref<NetVitals>,
        Ref<NetBuild>,
        Option<Ref<NetStats>>,
        Option<Ref<NetBuffs>>,
        Option<Ref<NetRaid>>,
        &mut HeroState,
        &mut ObjectStore,
        Has<LocalHero>,
    )>,
    mut run: ResMut<Run>,
) {
    for (vitals, build, stats, buffs, raid, mut hero, mut store, local) in &mut heroes {
        // A raid boss's forced moves: this player's own walk plays them ([`super::super::raid`]).
        if let Some(raid) = raid.filter(|r| r.is_changed()) {
            raid.apply(&mut hero.raid);
        }
        if build.is_changed() {
            apply_build(&mut hero, &build);
            if local {
                run.version += 1;
            }
        }
        if let Some(buffs) = buffs.filter(|b| b.is_changed()) {
            apply_buffs(&mut hero, &buffs);
            if local {
                run.version += 1;
            }
        }
        if let Some(stats) = stats.filter(|s| s.is_changed()) {
            apply_stats(&mut hero, &stats);
        }
        if vitals.is_changed() {
            let was_up = hero.alive();
            apply_vitals(&mut hero, &vitals, local);
            match (was_up, hero.alive()) {
                (true, false) => units::kill_unit(&mut store),
                (false, true) => units::raise_unit(&mut store),
                _ => {}
            }
        }
    }
}

/// A unit's auras as the host shows them, into its buff or debuff slots: a hero's group buffs,
/// stance, soulstone and its revive or Focus glow, an enemy's shared debuffs and elite mark, each
/// with its spell's visual.
fn mirror_auras(
    mut q: Query<(Entity, &Guid, &NetAuras, &mut Marks, &mut ObjectStore), Changed<NetAuras>>,
    mut fields: MessageWriter<FieldChanged>,
) {
    for (entity, guid, auras, mut marks, mut store) in &mut q {
        marks.set(&auras.0);
        marks.flush(entity, guid.0, &mut store, &mut fields);
    }
}

/// The party UI's view on a client ([`partyui`]): the replicated heroes, in the session's player
/// order, each by its player's name and at the host's max health. The default filler reads the
/// heroes' own state, which only the host has.
#[allow(clippy::type_complexity)]
pub(super) fn fill_party_view(
    run: Res<Run>,
    selection: Res<Selection>,
    names: HeroNames,
    heroes: Query<
        (
            Entity,
            &HeroState,
            &Transform,
            Option<&Marks>,
            Has<LocalHero>,
            &NetHero,
            &NetVitals,
        ),
        With<Hero>,
    >,
    mut view: ResMut<PartyView>,
) {
    let own = selection.class.and_then(|c| names.name(c));
    let mut members = partyui::members_of(
        heroes
            .iter()
            .map(|(e, hero, tf, marks, local, net, _)| HeroRead {
                entity: e,
                hero,
                position: tf.translation,
                buffs: marks.map(|m| m.spells().collect()).unwrap_or_default(),
                local,
                bot: false,
                name: Some(net.name.as_str()),
            }),
        run.level,
        own,
    );
    for m in &mut members {
        if let Ok((.., net, vitals)) = heroes.get(m.entity) {
            m.player = net.player;
            m.max_hp = vitals.max_hp;
        }
    }
    members.sort_by_key(|m| m.player);
    let want = PartyView {
        members,
        elapsed: run.elapsed,
    };
    if *view != want {
        *view = want;
    }
}

/// An enemy's health; its death plays here and the corpse fades as the host's does.
fn enemy_life(
    mut commands: Commands,
    mut q: Query<(Entity, &NetLife, &mut Enemy, Option<&mut ObjectStore>), Changed<NetLife>>,
) {
    for (e, life, mut enemy, store) in &mut q {
        enemy.hp = life.hp;
        if life.dead && !enemy.dead {
            enemy.dead = true;
            if let Some(mut store) = store {
                units::kill_unit(&mut store);
            }
            enemies::mark_dying(&mut commands, e);
        }
    }
}

/// The shared run: the clock, the bar and level, gold, and the announcement.
fn copy_run(
    q: Query<&NetRun, Changed<NetRun>>,
    session: Res<Session>,
    mut run: ResMut<Run>,
    mut camp: ResMut<Camp>,
) {
    let Some(net) = q.iter().last() else { return };
    run.elapsed = net.elapsed;
    run.kills = net.kills;
    run.level = net.level;
    run.xp = net.xp;
    run.gold = net.gold;
    run.elites = net.elites;
    run.caches = net.caches;
    run.bosses = net.bosses;
    run.won = net.won;
    run.breather = net.camp_left;
    *camp = camp_of(net, session.me);
    if let Some(text) = &net.banner {
        if run.banner.as_ref().is_none_or(|b| b.0 != *text) {
            run.banner = Some((text.clone(), 3.0));
        }
    }
}

/// The host's boss yells, shown and sounded here as there.
fn receive_yells(mut msgs: MessageReader<NetYell>, mut yells: MessageWriter<Yelled>) {
    for y in msgs.read() {
        yells.write(Yelled {
            text: y.text.clone(),
            sound: y.sound,
        });
    }
}

/// The cards of a hand this machine can show `hero`: the slots its build has.
pub(super) fn playable(hand: &Hand, hero: &HeroState) -> Vec<Choice> {
    hand.choices
        .iter()
        .filter_map(WireChoice::choice)
        .filter(|c| match *c {
            Choice::Upgrade(k) => k < hero.abilities.len(),
            Choice::UpPassive(k) => k < hero.passives.len(),
            _ => true,
        })
        .collect()
}

/// The host dealt this player its next banked hand: it waits in the strip ([`super::super::strip`]).
fn receive_hands(
    mut hands: MessageReader<Hand>,
    mut offer: ResMut<Offer>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut run: ResMut<Run>,
    mut pending: Local<Option<Hand>>,
) {
    if let Some(hand) = hands.read().last().cloned() {
        *pending = Some(hand);
    }
    let Some(hand) = pending.take() else {
        return;
    };
    // Held until this player's hero has replicated.
    let Ok(mut me) = local.single_mut() else {
        *pending = Some(hand);
        return;
    };
    apply_build(&mut me, &hand.build);
    run.version += 1;
    // The hand waits in the strip until this player opens it (Tab); an empty one closes it.
    offer.deal(playable(&hand, &me));
}

/// The host's spell visuals, swings and numbers, played here as a server's would be.
#[allow(clippy::too_many_arguments)]
fn play_fx(
    mut fx: MessageReader<NetFx>,
    alive: Query<()>,
    mut seq: ResMut<PlaySeq>,
    mut casts: MessageWriter<CastEvent>,
    mut gos: MessageWriter<SpellGoTargets>,
    mut swings: MessageWriter<SwingMessage>,
    mut texts: MessageWriter<CombatTextSpawn>,
) {
    for f in fx.read() {
        if !f.entities().iter().all(|&e| alive.contains(e)) {
            continue;
        }
        match f {
            NetFx::Go {
                caster,
                spell,
                hits,
                dest,
            } => {
                let s = seq.next();
                casts.write(CastEvent {
                    entity: caster.0,
                    spell_id: *spell,
                    kind: CastEventKind::Go,
                    seq: s,
                });
                gos.write(SpellGoTargets {
                    caster: caster.0,
                    spell_id: *spell,
                    hits: hits.iter().map(|h| h.0).collect(),
                    misses: Vec::new(),
                    dest: dest.map(Vec3::from_array),
                    ammo_display_id: None,
                    seq: s,
                });
            }
            NetFx::Cast {
                entity,
                spell,
                kind,
            } => {
                let kind = match *kind {
                    WireCast::Start => CastEventKind::Start,
                    WireCast::Go => CastEventKind::Go,
                    WireCast::Fail => CastEventKind::Fail,
                    WireCast::Impact { weapon_visual } => CastEventKind::Impact { weapon_visual },
                    WireCast::GroundImpact { at } => CastEventKind::GroundImpact {
                        pos: Vec3::from_array(at),
                    },
                };
                casts.write(CastEvent {
                    entity: entity.0,
                    spell_id: *spell,
                    kind,
                    seq: seq.next(),
                });
            }
            NetFx::Swing {
                attacker,
                victim,
                hit_info,
                victim_state,
                damage,
            } => {
                swings.write(SwingMessage {
                    attacker: attacker.0,
                    victim: victim.map(|v| v.0),
                    hit_info: *hit_info,
                    victim_state: *victim_state,
                    damage: *damage,
                    displayed: false,
                    seq: seq.next(),
                });
            }
            NetFx::Text {
                anchor,
                text,
                category,
                color,
            } => {
                texts.write(CombatTextSpawn {
                    anchor: anchor.0,
                    text: text.clone(),
                    category: *category,
                    color: *color,
                });
            }
        }
    }
}

/// The share of the gap to the host's place a frame of `dt` closes.
pub(super) fn ease(dt: f32) -> f32 {
    1.0 - (-dt.max(0.0) / EASE_TAU).exp()
}

/// Everything but this player's own hero eases toward the host's place and facing, and walks as
/// the host's does.
#[allow(clippy::type_complexity)]
fn interpolate(
    time: Res<Time<Real>>,
    mut q: Query<
        (&NetPos, &mut Transform, Option<&mut MovementState>),
        (Without<LocalHero>, With<bevy_replicon::prelude::Remote>),
    >,
) {
    let k = ease(time.delta_secs());
    for (net, mut tf, mv) in &mut q {
        let to = net.pos();
        let at = tf.translation;
        if at.distance(to) > SNAP {
            tf.translation = to;
        } else if at != to {
            tf.translation = at.lerp(to, k);
        }
        let turn = Quat::from_rotation_y(net.yaw());
        if tf.rotation != turn {
            tf.rotation = tf.rotation.slerp(turn, k);
        }
        if let Some(mut mv) = mv {
            let (pace, flags) = (net.pace(), net.flags);
            if mv.speed != pace || mv.flags != flags {
                mv.speed = pace;
                mv.flags = flags;
            }
        }
    }
}

/// While joined, only the host starts runs: Enter World, Enter and Play Again wait for it.
fn hold_solo_starts(
    phase: Res<State<Phase>>,
    mut next: ResMut<NextState<Phase>>,
    mut allow: ResMut<HostStart>,
    mut session: ResMut<Session>,
) {
    if allow.0 {
        if *phase.get() == Phase::Playing {
            allow.0 = false;
        }
        return;
    }
    if *phase.get() == Phase::Menu
        && matches!(
            *next,
            NextState::Pending(Phase::Playing) | NextState::PendingIfNeq(Phase::Playing)
        )
    {
        *next = NextState::Unchanged;
        session.status = "Waiting for the host to start the run".into();
    }
}

/// This player's hero, where it walks, at the replication pace.
fn send_move(
    time: Res<Time<Real>>,
    pace: Res<Pace>,
    mut acc: Local<f32>,
    session: Res<Session>,
    hero: Query<(&Transform, &MovementState), With<LocalHero>>,
    mut out: MessageWriter<MoveInput>,
) {
    *acc += time.delta_secs();
    if *acc < pace.interval || !session.in_run {
        return;
    }
    *acc = (*acc - pace.interval).min(pace.interval);
    let Ok((tf, mv)) = hero.single() else { return };
    out.write(MoveInput {
        at: tf.translation.to_array(),
        yaw: super::super::hero::yaw_of(tf.rotation),
        pace: mv.speed,
        flags: mv.flags,
    });
}

/// This player's strip opening or shutting, to the host, which Focuses its hero.
fn send_focus(
    local: Query<&HeroState, With<LocalHero>>,
    mut sent: Local<bool>,
    mut out: MessageWriter<FocusInput>,
) {
    let now = local.single().is_ok_and(|me| me.focused);
    if *sent != now {
        *sent = now;
        out.write(FocusInput(now));
    }
}

/// This player's self-resurrection key while its hero is down, to the host: a soulstone or an
/// ankh gets it up there.
fn send_self_res(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    session: Res<Session>,
    local: Query<&HeroState, With<LocalHero>>,
    mut out: MessageWriter<SelfRes>,
) {
    let pressed = keys.is_some_and(|k| k.just_pressed(synergy::SELF_RES_KEY));
    if pressed && session.in_run && local.single().is_ok_and(|me| !me.alive()) {
        out.write(SelfRes);
    }
}

/// The cards' and tools' choices, to the host.
fn send_hand(mut outbox: ResMut<HandOutbox>, mut out: MessageWriter<HandAction>) {
    for d in outbox.0.drain(..) {
        out.write(HandAction(WireDecision::of(d)));
    }
}

/// This player's camp asks go to the host, which owns timing, catches, rolls and the purse.
fn send_camp(mut asks: MessageReader<CampRequest>, mut out: MessageWriter<CampAction>) {
    for ask in asks.read() {
        out.write(match ask.0 {
            CampAsk::Ready => CampAction::Ready,
            CampAsk::Fish => CampAction::Fish,
            CampAsk::Roll => CampAction::Roll,
            CampAsk::Buy(item) => CampAction::Buy(item.index()),
        });
    }
}

/// The host's camp in this machine's numbering, where this player is [`party::LOCAL_PLAYER`].
pub(super) fn camp_of(net: &NetRun, me: u8) -> Camp {
    let ids = |v: &[u8]| -> Vec<u8> {
        let mut out: Vec<u8> = v.iter().map(|&p| local_player(p, me)).collect();
        out.sort_unstable();
        out
    };
    let mut roll: Vec<(u8, u8)> = net
        .camp_rolls
        .iter()
        .map(|&(p, v)| (local_player(p, me), v))
        .collect();
    roll.sort_unstable();
    Camp {
        ready: ids(&net.camp_ready),
        fished: ids(&net.camp_fished),
        fishing_dip: net.fishing_dip,
        roll,
        roll_winner: net.camp_roll_winner.map(|p| local_player(p, me)),
        fire: net.camp_fire.map(Vec3::from_array),
    }
}

/// Back at the menu while the host's run goes on (the pause menu's Main Menu): this player leaves
/// the session, and its hero the run.
fn quit_run(mut commands: Commands, session: Res<Session>) {
    if session.in_run {
        commands.queue(|world: &mut World| leave(world, "You left the run".into()));
    }
}

/// Leave the session (or lose it): the link goes, every replicated body goes, and this machine is
/// back at its own menu with `why` on the panel.
pub(super) fn leave(world: &mut World, why: String) {
    use bevy_replicon_renet2::netcode::NetcodeClientTransport;
    use bevy_replicon_renet2::renet2::RenetClient;
    if let Some(mut client) = world.remove_resource::<RenetClient>() {
        client.disconnect();
        if let Some(mut transport) = world.remove_resource::<NetcodeClientTransport>() {
            transport.disconnect();
        }
    }
    let mut q = world.query_filtered::<Entity, With<bevy_replicon::prelude::Remote>>();
    let gone: Vec<Entity> = q.iter(world).collect();
    for e in gone {
        world.despawn(e);
    }
    {
        let mut session = world.resource_mut::<Session>();
        session.link = Link::Off;
        session.in_run = false;
        session.lobby.clear();
        session.me = 0;
        session.status = why;
    }
    world.resource_mut::<Selection>().version += 1;
    if *world.resource::<State<Phase>>().get() != Phase::Menu {
        world.resource_mut::<NextState<Phase>>().set(Phase::Menu);
    }
}
