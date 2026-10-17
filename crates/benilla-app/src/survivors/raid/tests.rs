//! The boss mechanics library and the first two Molten Core scripts: each block's timers and
//! damage, the bomb's proximity, dispels, fear, enrage, each boss's schedule, and (against the
//! client's data, skipped without it) every spell, display and sound they name.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::bosses::*;
use super::*;
use crate::survivors::bonus::Bonuses;
use crate::survivors::data;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn class_index(id: u8) -> usize {
    data::CLASSES.iter().position(|c| c.id == id).unwrap()
}

fn hero_of(class_id: u8) -> HeroState {
    HeroState::new(
        0,
        &data::CLASSES[class_index(class_id)],
        1,
        Bonuses::default(),
    )
}

// ---------------------------------------------------------------------------------------------
// Shapes
// ---------------------------------------------------------------------------------------------

#[test]
fn a_yaw_faces_the_way_it_was_taken_toward() {
    for d in [Vec3::X, Vec3::NEG_Z, Vec3::new(1.0, 0.0, 1.0).normalize()] {
        assert!(forward(yaw_toward(d)).distance(d) < 1e-4, "{d}");
    }
}

#[test]
fn each_shape_holds_what_lies_inside_it() {
    let o = Vec3::new(10.0, 5.0, -3.0);
    let east = yaw_toward(Vec3::X);
    let circle = Shape::Circle { radius: 6.0 };
    assert!(circle.contains(o, 0.0, o + Vec3::new(4.0, 9.0, 4.0)));
    assert!(!circle.contains(o, 0.0, o + Vec3::new(5.0, 0.0, 5.0)));

    let cone = Shape::Cone {
        radius: 10.0,
        half_angle: 0.5,
    };
    assert!(cone.contains(o, east, o + Vec3::new(8.0, 0.0, 1.0)));
    assert!(
        !cone.contains(o, east, o + Vec3::new(-8.0, 0.0, 0.0)),
        "behind"
    );
    assert!(
        !cone.contains(o, east, o + Vec3::new(4.0, 0.0, 6.0)),
        "wide"
    );
    assert!(
        !cone.contains(o, east, o + Vec3::new(11.0, 0.0, 0.0)),
        "far"
    );

    let line = Shape::Line {
        length: 20.0,
        width: 4.0,
    };
    assert!(line.contains(o, east, o + Vec3::new(15.0, 0.0, 1.5)));
    assert!(!line.contains(o, east, o + Vec3::new(15.0, 0.0, 2.5)));
    assert!(!line.contains(o, east, o + Vec3::new(-1.0, 0.0, 0.0)));

    let donut = Shape::Donut {
        inner: 5.0,
        outer: 12.0,
    };
    assert!(
        !donut.contains(o, 0.0, o + Vec3::X * 2.0),
        "the safe middle"
    );
    assert!(donut.contains(o, 0.0, o + Vec3::X * 8.0));
    assert!(!donut.contains(o, 0.0, o + Vec3::X * 13.0));
}

#[test]
fn every_scripted_telegraph_warns_long_enough_to_read() {
    for script in [&LUCIFRON, &PROTECTOR, &MAGMADAR] {
        for a in script.abilities {
            if let Does::Ground {
                shape,
                windup,
                blow,
                ..
            } = a.does
            {
                assert!(
                    windup >= shape.min_windup(),
                    "{}: {} winds up {windup}s",
                    script.name,
                    a.name
                );
                assert!(blow.damage <= 0.35, "{}: {}", script.name, a.name);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The brain: timers, thresholds, phases, party size
// ---------------------------------------------------------------------------------------------

static TEST_SCRIPT: Script = Script {
    name: "Test Dummy",
    abilities: &[
        BossAbility::new(
            "Tick",
            1,
            When::Every {
                first: 2.0,
                every: 5.0,
            },
            Pick::Boss,
            Does::Yell,
        ),
        BossAbility::new("Once", 2, When::At(4.0), Pick::Boss, Does::Yell),
        BossAbility::new("Half", 3, When::Health(0.5), Pick::Boss, Does::Phase(2)),
        BossAbility::new(
            "Phase two only",
            4,
            When::Every {
                first: 1.0,
                every: 1.0,
            },
            Pick::Boss,
            Does::Yell,
        )
        .in_phases(0b10),
        BossAbility::new("Crowd", 5, When::At(1.0), Pick::Boss, Does::Yell).min_party(3),
    ],
};

/// When each ability fired over `secs`, with health given by the clock.
fn schedule(
    script: &'static Script,
    secs: f32,
    party: u8,
    health: impl Fn(f32) -> f32,
) -> Vec<(u32, &'static str, f32)> {
    let mut brain = RaidBoss::new(script);
    let mut out = Vec::new();
    // A power of two, so the clock lands on every whole second exactly.
    let dt = 0.0625;
    let mut t = 0.0;
    while t < secs {
        t += dt;
        for a in brain.due(dt, health(t), party) {
            out.push((a.spell, a.name, brain.clock));
        }
    }
    out
}

fn times(fired: &[(u32, &str, f32)], name: &str) -> Vec<f32> {
    fired
        .iter()
        .filter(|f| f.1 == name)
        .map(|f| (f.2 * 10.0).round() / 10.0)
        .collect()
}

#[test]
fn abilities_fire_on_their_timers_thresholds_and_phases() {
    let fired = schedule(&TEST_SCRIPT, 13.0, 1, |t| if t < 9.0 { 1.0 } else { 0.4 });
    assert_eq!(times(&fired, "Tick"), [2.0, 7.0, 12.0]);
    assert_eq!(times(&fired, "Once"), [4.0]);
    // The health threshold, once; phase two's own row runs only from then.
    assert_eq!(times(&fired, "Half"), [9.0]);
    assert_eq!(
        times(&fired, "Phase two only"),
        [9.0, 10.0, 11.0, 12.0, 13.0]
    );
    // Off below its party size.
    assert!(times(&fired, "Crowd").is_empty());
    let crowd = schedule(&TEST_SCRIPT, 2.0, 3, |_| 1.0);
    assert_eq!(times(&crowd, "Crowd"), [1.0]);
}

#[test]
fn lucifron_curses_dooms_shocks_and_enrages_on_schedule() {
    let fired = schedule(&LUCIFRON, 185.0, 5, |_| 1.0);
    assert_eq!(times(&fired, "Flamewaker Protectors"), [0.5]);
    assert_eq!(
        times(&fired, "Impending Doom"),
        [18.0, 43.0, 68.0, 93.0, 118.0, 143.0, 168.0]
    );
    let shocks = times(&fired, "Shadow Shock");
    assert_eq!(&shocks[..3], [6.0, 15.0, 24.0]);
    // One curse row a beat: the party's count before the soft enrage, three after it.
    assert_eq!(
        times(&fired, "Lucifron's Curse"),
        [10.0, 30.0, 50.0, 70.0, 90.0, 110.0, 130.0, 150.0, 170.0]
    );
    assert_eq!(times(&fired, "Soft enrage"), [120.0]);
    assert_eq!(times(&fired, "Berserk"), [180.0]);
    let mut brain = RaidBoss::new(&LUCIFRON);
    let mut picks = Vec::new();
    for _ in 0..(185.0 / 0.0625) as usize {
        for a in brain.due(0.0625, 1.0, 5) {
            if a.name == "Lucifron's Curse" {
                picks.push((brain.phase, a.pick));
            }
        }
    }
    assert!(picks
        .iter()
        .all(|&(phase, pick)| (phase == 1) == (pick == Pick::Random(0))));
    assert!(picks.iter().any(|&(_, pick)| pick == Pick::Random(3)));
}

#[test]
fn protectors_cleave_and_dominate_only_a_party_of_three() {
    let five = schedule(&PROTECTOR, 61.0, 5, |_| 1.0);
    assert_eq!(times(&five, "Cleave")[..3], [4.0, 12.0, 20.0]);
    assert_eq!(times(&five, "Dominate Mind"), [15.0, 45.0]);
    let duo = schedule(&PROTECTOR, 61.0, 2, |_| 1.0);
    assert!(times(&duo, "Dominate Mind").is_empty());
}

#[test]
fn magmadar_panics_bombs_frenzies_and_his_pools_stop_cooling() {
    let fired = schedule(&MAGMADAR, 215.0, 1, |_| 1.0);
    assert_eq!(
        times(&fired, "Panic"),
        [15.0, 45.0, 75.0, 105.0, 135.0, 165.0, 195.0]
    );
    assert_eq!(times(&fired, "Frenzy")[..3], [12.0, 27.0, 42.0]);
    assert_eq!(times(&fired, "Lava Bomb")[..3], [5.0, 13.0, 21.0]);
    assert_eq!(times(&fired, "Magma Spit").len(), 215);
    assert_eq!(times(&fired, "Soft enrage"), [150.0]);
    assert_eq!(times(&fired, "Berserk"), [210.0]);
    // Before the soft enrage the pools cool after 30 s; after it they burn for good.
    let mut brain = RaidBoss::new(&MAGMADAR);
    let mut pools = Vec::new();
    for _ in 0..(215.0 / 0.0625) as usize {
        for a in brain.due(0.0625, 1.0, 1) {
            if let Does::Ground { blow, .. } = a.does {
                if let Some(pool) = blow.pool {
                    pools.push((brain.clock, pool.secs));
                }
            }
        }
    }
    assert!(pools.iter().all(|&(t, secs)| (t < 150.0) == (secs == 30.0)));
    assert!(pools.iter().any(|&(_, secs)| secs.is_infinite()));
}

#[test]
fn the_scripts_are_found_by_their_bosses_names() {
    assert_eq!(script_for("Lucifron").map(|s| s.name), Some("Lucifron"));
    assert_eq!(script_for("Magmadar").map(|s| s.name), Some("Magmadar"));
    assert_eq!(
        script_for("Flamewaker Protector").map(|s| s.name),
        Some(PROTECTOR_NAME)
    );
    assert!(script_for("Hogger").is_none());
}

// ---------------------------------------------------------------------------------------------
// Picks and the tank
// ---------------------------------------------------------------------------------------------

fn seen_at(world: &mut World, positions: &[Vec3]) -> Vec<Seen> {
    positions
        .iter()
        .map(|&pos| Seen {
            e: world.spawn_empty().id(),
            pos,
            up: true,
            threat: 1.0,
        })
        .collect()
}

#[test]
fn picks_choose_the_tank_the_near_and_never_the_down() {
    let mut world = World::new();
    let mut seen = seen_at(
        &mut world,
        &[Vec3::X * 2.0, Vec3::X * 10.0, Vec3::X * 40.0, Vec3::X * 5.0],
    );
    seen[3].up = false;
    let mut rng = Rng(5);
    let tank = boss_target(Vec3::ZERO, None, 0.0, &seen);
    assert_eq!(tank, Some(0), "the nearest standing");
    // A taunt holds the boss on its taunter while it lasts.
    let taunted = Some((seen[1].e, 10.0));
    assert_eq!(boss_target(Vec3::ZERO, taunted, 5.0, &seen), Some(1));
    assert_eq!(boss_target(Vec3::ZERO, taunted, 11.0, &seen), Some(0));

    assert_eq!(pick(Pick::Tank, Vec3::ZERO, tank, &seen, 4, &mut rng), [0]);
    let mut near = pick(Pick::Within(30.0), Vec3::ZERO, tank, &seen, 4, &mut rng);
    near.sort();
    assert_eq!(near, [0, 1]);
    for _ in 0..20 {
        let others = pick(Pick::RandomNonTank(1), Vec3::ZERO, tank, &seen, 4, &mut rng);
        assert!(others.len() == 1 && others[0] != 0 && others[0] != 3);
        let curse = pick(Pick::Random(0), Vec3::ZERO, tank, &seen, 4, &mut rng);
        assert_eq!(curse.len(), curse_targets(4));
        assert!(!curse.contains(&3));
    }
    assert_eq!(
        (curse_targets(1), curse_targets(3), curse_targets(5)),
        (1, 2, 3)
    );
}

// ---------------------------------------------------------------------------------------------
// Hero debuffs: doom, curses, stacks, mind control, forced moves
// ---------------------------------------------------------------------------------------------

fn run_ticks(raid: &mut HeroRaid, secs: f32, solo: bool) -> Vec<RaidEvent> {
    let mut out = Vec::new();
    let mut t = 0.0;
    // A power of two, so the timers land on their marks exactly.
    while t < secs - 1e-4 {
        out.extend(raid.tick(0.125, solo));
        t += 0.125;
    }
    out
}

#[test]
fn impending_doom_counts_down_and_lands_unless_dispelled() {
    let doom = || Debuff::new(IMPENDING_DOOM, Dispel::Magic, 10.0, Payload::Doom(0.35));
    let mut raid = HeroRaid::default();
    raid.add(doom());
    let early = run_ticks(&mut raid, 9.5, false);
    assert!(early.iter().all(|e| matches!(e, RaidEvent::Count(_))));
    assert!(early.contains(&RaidEvent::Count(1)));
    let late = run_ticks(&mut raid, 0.625, false);
    assert!(late.contains(&RaidEvent::Hit {
        spell: IMPENDING_DOOM,
        damage: 0.35
    }));
    assert!(raid.debuffs.is_empty());

    // Dispelled: nothing lands. A decurse cannot take magic.
    raid.add(doom());
    assert!(raid.dispel(Dispel::Curse).is_none());
    assert_eq!(raid.urgency(Dispel::Magic), Some(10.0));
    assert!(raid.dispel(Dispel::Magic).is_some());
    assert!(run_ticks(&mut raid, 11.0, false).is_empty());
}

#[test]
fn a_curse_slows_the_cooldowns_until_it_runs_out() {
    let mut raid = HeroRaid::default();
    assert_eq!(raid.cooldown_rate(), 1.0);
    raid.add(Debuff::new(
        LUCIFRONS_CURSE,
        Dispel::Curse,
        20.0,
        Payload::CooldownRate(0.5),
    ));
    assert_eq!(raid.cooldown_rate(), 0.5);
    run_ticks(&mut raid, 19.0, false);
    assert_eq!(raid.cooldown_rate(), 0.5);
    run_ticks(&mut raid, 1.125, false);
    assert_eq!(raid.cooldown_rate(), 1.0);
}

#[test]
fn tank_swap_stacks_burn_warn_and_fall_off() {
    let spit = || {
        Debuff::new(
            MAGMA_SPIT,
            Dispel::None,
            30.0,
            Payload::Stacks {
                per: 0.01,
                warn: 6,
                decay: 3.0,
            },
        )
    };
    let mut raid = HeroRaid::default();
    for _ in 0..5 {
        raid.add(spit());
    }
    assert_eq!(raid.stacks(MAGMA_SPIT), 5);
    // Five stacks burn 5% a second, in half-second ticks.
    let burns = run_ticks(&mut raid, 1.0, false);
    let total: f32 = burns
        .iter()
        .map(|e| match e {
            RaidEvent::Hit { damage, .. } => *damage,
            _ => 0.0,
        })
        .sum();
    assert!(close(total, 0.05), "{total}");
    raid.add(spit());
    assert_eq!(raid.stacks(MAGMA_SPIT), 6);
    let warned = run_ticks(&mut raid, 0.5, false);
    assert_eq!(warned.iter().filter(|e| **e == RaidEvent::Swap).count(), 1);
    // Off the boss, one falls off every 3 s.
    run_ticks(&mut raid, 3.0, false);
    assert_eq!(raid.stacks(MAGMA_SPIT), 5);
    run_ticks(&mut raid, 15.5, false);
    assert_eq!(raid.stacks(MAGMA_SPIT), 0);
    assert!(raid.debuffs.is_empty());
    // Solo, the stacks fall off at their warning.
    for _ in 0..6 {
        raid.add(spit());
    }
    run_ticks(&mut raid, 0.125, true);
    assert_eq!(raid.stacks(MAGMA_SPIT), 0);
}

#[test]
fn mind_control_holds_the_hero_and_burns_its_neighbours_until_dispelled() {
    let mut raid = HeroRaid::default();
    raid.add(Debuff::new(
        DOMINATE_MIND,
        Dispel::Magic,
        6.0,
        Payload::Dominate {
            radius: 6.0,
            damage: 0.06,
        },
    ));
    assert!(!raid.can_cast() && raid.dominated());
    assert_eq!(raid.last_move.map(|m| m.kind), Some(MoveKind::Hold));
    let mut walk = Walk::default();
    assert_eq!(walk.step(&raid, 0.1), Some(Vec3::ZERO));
    let pulses = run_ticks(&mut raid, 2.05, false);
    assert_eq!(
        pulses
            .iter()
            .filter(|e| matches!(e, RaidEvent::Splash { .. }))
            .count(),
        2
    );
    assert!(raid.forced_left > 3.5);
    // The walker holds the hero, then lets go soon after the dispel.
    raid.dispel(Dispel::Magic).unwrap();
    raid.tick(0.1, false);
    assert!(raid.can_cast() && raid.forced_left == 0.0);
    let mut held = 0;
    while walk.step(&raid, 0.1).is_some() {
        held += 1;
        assert!(held < 10, "the hold outlived its dispel");
    }
}

#[test]
fn fear_runs_the_hero_away_for_its_seconds() {
    let mut raid = HeroRaid::default();
    raid.fear(Vec3::new(3.0, 4.0, 0.0), 6.0);
    assert!(!raid.can_cast());
    let mut walk = Walk::default();
    let mut walked = Vec3::ZERO;
    let mut secs = 0.0;
    let dt = 0.05;
    while let Some(v) = walk.step(&raid, dt) {
        walked += v * dt;
        secs += dt;
        raid.tick(dt, false);
        assert_eq!(walk.kind(), Some(MoveKind::Flee));
    }
    assert!((secs - 6.0).abs() < 0.1, "{secs}");
    assert!(close(walked.normalize().x, 1.0), "straight away: {walked}");
    assert!((walked.length() - FLEE_SPEED * 6.0).abs() < 1.0);
    raid.tick(dt, false);
    assert!(raid.can_cast());

    // A knockback throws the hero its yards, quickly.
    raid.knock(Vec3::NEG_Z, 10.0);
    let mut thrown = Vec3::ZERO;
    while let Some(v) = walk.step(&raid, 0.01) {
        thrown += v * 0.01;
    }
    assert!(
        (thrown.length() - 10.0).abs() < 0.5 && thrown.z < 0.0,
        "{thrown}"
    );
}

#[test]
fn fear_is_shrugged_off_by_warriors_undead_and_a_tremor_totem() {
    let at = Vec3::ZERO;
    assert!(fear_immune(1, 1, at, &[]));
    assert!(fear_immune(8, 5, at, &[]));
    assert!(!fear_immune(8, 1, at, &[]));
    assert!(fear_immune(8, 1, at, &[Vec3::X * 15.0]));
    assert!(!fear_immune(8, 1, at, &[Vec3::X * 25.0]));
}

#[test]
fn healer_classes_dispel_the_most_urgent_ally_in_range() {
    assert_eq!(dispels_of(8), Some((Dispel::Curse, REMOVE_LESSER_CURSE)));
    assert_eq!(dispels_of(11), Some((Dispel::Curse, REMOVE_CURSE)));
    assert_eq!(dispels_of(5), Some((Dispel::Magic, DISPEL_MAGIC)));
    assert_eq!(dispels_of(2), Some((Dispel::Magic, CLEANSE)));
    assert_eq!(dispels_of(1), None);
    let afflicted = [
        (Vec3::X * 5.0, Some(6.0)),
        (Vec3::X * 8.0, Some(2.0)),
        (Vec3::X * 30.0, Some(0.5)),
        (Vec3::X * 3.0, None),
    ];
    assert_eq!(dispel_target(Vec3::ZERO, &afflicted), Some(1));
    assert_eq!(dispel_target(Vec3::X * 40.0, &afflicted[..2]), None);
}

// ---------------------------------------------------------------------------------------------
// The host's systems in a world
// ---------------------------------------------------------------------------------------------

/// A world the raid's hero systems run in, `dt` a frame, with the run's clock at `now`.
fn raid_world(dt: f32, now: f32, party: u8) -> World {
    let mut world = World::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs_f32(dt));
    world.insert_resource(time);
    world.insert_resource(Run {
        elapsed: now,
        party,
        ..Run::default()
    });
    world.insert_resource(Rng(3));
    world.init_resource::<crate::creature_anim::PlaySeq>();
    world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
    world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
    world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
    world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
    world
}

fn hero_in(world: &mut World, state: HeroState, at: Vec3) -> Entity {
    world
        .spawn((Transform::from_translation(at), state, Hero::default()))
        .id()
}

/// A boss standing at the origin, as the pools' source.
fn boss_in(world: &mut World) -> Entity {
    let en = Enemy {
        def: &data::roster(0).bosses[0],
        hp: 1000.0,
        max_hp: 1000.0,
        speed: 0.0,
        damage: 0.0,
        radius: 1.0,
        xp: 1,
        boss: true,
        elite: false,
        guardian: false,
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
    };
    world.spawn((en, Transform::default())).id()
}

fn hp(world: &World, e: Entity) -> f32 {
    world.get::<HeroState>(e).unwrap().hp
}

#[test]
fn a_living_bomb_hurts_the_allies_near_its_carrier_not_those_who_ran() {
    let mut world = raid_world(0.25, 0.0, 3);
    let mut carrier = hero_of(8);
    carrier.raid.add(Debuff::new(
        20475,
        Dispel::None,
        8.0,
        Payload::Bomb {
            radius: 12.0,
            allies: 0.6,
            carrier: 0.25,
        },
    ));
    let carrier = hero_in(&mut world, carrier, Vec3::ZERO);
    let near = hero_in(&mut world, hero_of(8), Vec3::X * 6.0);
    let far = hero_in(&mut world, hero_of(8), Vec3::X * 20.0);
    let full = hp(&world, near);
    // The fuse burns: nothing before 8 s.
    for _ in 0..31 {
        world.run_system_once(tick_heroes).unwrap();
    }
    assert_eq!(hp(&world, near), full);
    world.run_system_once(tick_heroes).unwrap();
    let max = world.get::<HeroState>(near).unwrap().stats().max_hp;
    assert!(
        (full - hp(&world, near) - max * 0.6).abs() < 2.0,
        "the ally in the ring takes 60%"
    );
    assert!(
        (full - hp(&world, carrier) - max * 0.25).abs() < 2.0,
        "the carrier takes 25%"
    );
    assert_eq!(hp(&world, far), full, "the ally who ran takes nothing");
    assert!(world
        .get::<HeroState>(carrier)
        .unwrap()
        .raid
        .debuffs
        .is_empty());
}

#[test]
fn a_priest_in_range_dispels_doom_before_it_lands() {
    let mut world = raid_world(0.25, 0.0, 2);
    let doom = || Debuff::new(IMPENDING_DOOM, Dispel::Magic, 10.0, Payload::Doom(0.35));
    let mut near = hero_of(8);
    near.raid.add(doom());
    let mut far = hero_of(8);
    far.raid.add(doom());
    hero_in(&mut world, hero_of(5), Vec3::ZERO);
    let near = hero_in(&mut world, near, Vec3::X * 10.0);
    let far = hero_in(&mut world, far, Vec3::X * 40.0);
    let full = hp(&world, near);
    for _ in 0..44 {
        world.run_system_once(tick_heroes).unwrap();
    }
    assert!(world
        .get::<HeroState>(near)
        .unwrap()
        .raid
        .debuffs
        .is_empty());
    assert_eq!(hp(&world, near), full, "dispelled in time");
    assert!(
        hp(&world, far) < full,
        "out of the priest's reach, the doom landed"
    );
}

#[test]
fn a_mage_decurses_but_a_warrior_cannot() {
    for (class, cleared) in [(8u8, true), (1, false)] {
        let mut world = raid_world(0.25, 0.0, 2);
        let mut cursed = hero_of(3);
        cursed.raid.add(Debuff::new(
            LUCIFRONS_CURSE,
            Dispel::Curse,
            20.0,
            Payload::CooldownRate(0.5),
        ));
        hero_in(&mut world, hero_of(class), Vec3::ZERO);
        let cursed = hero_in(&mut world, cursed, Vec3::X * 5.0);
        world.run_system_once(tick_heroes).unwrap();
        let rate = world.get::<HeroState>(cursed).unwrap().raid.cooldown_rate();
        assert_eq!(rate == 1.0, cleared, "class {class}");
    }
}

fn telegraph(shape: Shape, at: Vec3, land: f32) -> Telegraph {
    Telegraph {
        shape,
        at: at.to_array(),
        yaw: yaw_toward(Vec3::X),
        tone: Tone::Damage,
        start: 0.0,
        land,
        until: land,
    }
}

#[test]
fn a_telegraph_waits_out_its_windup_then_hits_only_who_stayed_inside() {
    let mut world = raid_world(0.1, 1.0, 2);
    let source = world.spawn_empty().id();
    let inside = hero_in(&mut world, hero_of(8), Vec3::X * 3.0);
    let outside = hero_in(&mut world, hero_of(8), Vec3::X * 9.0);
    let full = hp(&world, inside);
    let t = world
        .spawn((
            telegraph(Shape::Circle { radius: 6.0 }, Vec3::ZERO, 2.0),
            Pending {
                source,
                spell: SHADOW_SHOCK,
                blow: Blow::hit(0.25, School::Shadow),
            },
        ))
        .id();
    world.run_system_once(land_telegraphs).unwrap();
    assert_eq!(hp(&world, inside), full, "still winding up");
    world.resource_mut::<Run>().elapsed = 2.0;
    world.run_system_once(land_telegraphs).unwrap();
    let max = world.get::<HeroState>(inside).unwrap().stats().max_hp;
    assert!((full - hp(&world, inside) - max * 0.25).abs() < 2.0);
    assert_eq!(hp(&world, outside), full);
    assert!(world.get_entity(t).is_err(), "a plain blow's warning goes");
}

#[test]
fn panic_fears_who_stands_in_its_ring_and_a_bomb_leaves_a_burning_pool() {
    let mut world = raid_world(0.5, 2.0, 2);
    let source = boss_in(&mut world);
    let mage = hero_in(&mut world, hero_of(8), Vec3::X * 10.0);
    let warrior = hero_in(&mut world, hero_of(1), Vec3::NEG_X * 10.0);
    world.spawn((
        telegraph(Shape::Circle { radius: 30.0 }, Vec3::ZERO, 2.0),
        Pending {
            source,
            spell: PANIC,
            blow: Blow::fear(6.0),
        },
    ));
    world.run_system_once(land_telegraphs).unwrap();
    let feared = &world.get::<HeroState>(mage).unwrap().raid;
    assert!(close(feared.fear_t, 6.0));
    assert_eq!(feared.last_move.map(|m| m.dir), Some([1.0, 0.0]), "away");
    assert_eq!(world.get::<HeroState>(warrior).unwrap().raid.fear_t, 0.0);

    // A Lava Bomb: the circle lands, then a pool burns who stands in it, and cools.
    let pool = world
        .spawn((
            telegraph(Shape::Circle { radius: 6.0 }, Vec3::X * 10.0, 2.0),
            Pending {
                source,
                spell: LAVA_BOMB,
                blow: Blow::hit(0.12, School::Fire).pool(0.06, 30.0),
            },
        ))
        .id();
    world.run_system_once(land_telegraphs).unwrap();
    assert!(world.get::<Pool>(pool).is_some() && world.get::<Pending>(pool).is_none());
    assert!(close(world.get::<Telegraph>(pool).unwrap().until, 32.0));
    let before = hp(&world, mage);
    world.run_system_once(burn_pools).unwrap();
    let max = world.get::<HeroState>(mage).unwrap().stats().max_hp;
    assert!((before - hp(&world, mage) - max * 0.06 * TICK).abs() < 1.5);
    let warrior_hp = hp(&world, warrior);
    world.resource_mut::<Run>().elapsed = 32.5;
    world.run_system_once(burn_pools).unwrap();
    assert!(world.get_entity(pool).is_err(), "cooled");
    assert_eq!(hp(&world, warrior), warrior_hp);

    // A pool that never cools still goes with the boss that left it.
    let lasting = world
        .spawn((
            telegraph(Shape::Circle { radius: 6.0 }, Vec3::ZERO, 32.0),
            Pending {
                source,
                spell: LAVA_BOMB,
                blow: Blow::hit(0.12, School::Fire).pool(0.06, f32::INFINITY),
            },
        ))
        .id();
    world.run_system_once(land_telegraphs).unwrap();
    world.resource_mut::<Run>().elapsed = 1000.0;
    world.run_system_once(burn_pools).unwrap();
    assert!(
        world.get::<Pool>(lasting).is_some(),
        "burns while he stands"
    );
    world.get_mut::<Enemy>(source).unwrap().dead = true;
    world.run_system_once(burn_pools).unwrap();
    assert!(world.get_entity(lasting).is_err(), "gone with him");
}

#[test]
fn a_knockback_throws_who_stands_in_its_ring_away_from_its_centre() {
    let mut world = raid_world(0.1, 3.0, 2);
    let source = world.spawn_empty().id();
    let near = hero_in(&mut world, hero_of(8), Vec3::NEG_Z * 4.0);
    let far = hero_in(&mut world, hero_of(8), Vec3::NEG_Z * 20.0);
    world.spawn((
        telegraph(Shape::Circle { radius: 12.0 }, Vec3::ZERO, 3.0),
        Pending {
            source,
            spell: 0,
            blow: Blow::knock(15.0, 0.1),
        },
    ));
    world.run_system_once(land_telegraphs).unwrap();
    let thrown = world
        .get::<HeroState>(near)
        .unwrap()
        .raid
        .last_move
        .unwrap();
    assert_eq!(thrown.kind, MoveKind::Knock);
    assert_eq!(thrown.dir, [0.0, -1.0], "away from the centre");
    assert!(close(thrown.speed * thrown.secs, 15.0));
    assert!(world
        .get::<HeroState>(far)
        .unwrap()
        .raid
        .last_move
        .is_none());
}

#[test]
fn a_feared_hero_casts_nothing_until_the_fear_runs_out() {
    let mut hero = hero_of(8);
    assert!(hero.raid.can_cast());
    hero.raid.fear(Vec3::X, 2.0);
    assert!(!hero.raid.can_cast());
    hero.raid.tick(2.1, false);
    assert!(hero.raid.can_cast());
}

#[test]
fn the_hard_enrage_fires_once_with_its_emote_and_sound() {
    let fired = schedule(&LUCIFRON, 200.0, 1, |_| 1.0);
    assert_eq!(fired.iter().filter(|f| f.1 == "Berserk").count(), 1);
    let row = LUCIFRON
        .abilities
        .iter()
        .find(|a| a.name == "Berserk")
        .unwrap();
    assert_eq!(
        row.does,
        Does::Berserk {
            damage: 3.0,
            speed: 1.5
        }
    );
    assert_eq!(row.spell, BERSERK);
    let y = Yelled::of(row.yell.as_ref().unwrap(), "Lucifron");
    assert_eq!(y.text, "Lucifron goes into a berserker rage!");
    assert_eq!(SOUNDS[usize::from(y.sound)], "RaidWarning");
}

// ---------------------------------------------------------------------------------------------
// Wiring: the stages' rosters
// ---------------------------------------------------------------------------------------------

#[test]
fn lucifron_and_magmadar_close_their_stages_rosters_with_lucifrons_protectors_as_adds() {
    let stage = |name: &str| data::MAPS.iter().position(|m| m.name == name).unwrap();
    let lucifron = data::roster(stage("Molten Core: Lucifron's Cavern"));
    assert_eq!(lucifron.bosses.last().map(|b| b.name), Some("Lucifron"));
    let magmadar = data::roster(stage("Molten Core: Magmadar's Lair"));
    assert_eq!(magmadar.bosses.last().map(|b| b.name), Some("Magmadar"));
    // The boss frame shows the final boss.
    let info = crate::survivors::bossbar::BossInfo {
        order: lucifron.bosses.len() - 1,
        name: "Lucifron",
        hp: 10.0,
        max_hp: 20.0,
        dead_for: None,
    };
    let frames = crate::survivors::bossbar::frames(&[info], lucifron.bosses.len());
    assert!(frames[0].final_boss && frames[0].name == "Lucifron");
    // Every add a script calls in is on its stage's roster, and numbers back to itself.
    for script in [&LUCIFRON, &PROTECTOR, &MAGMADAR] {
        for a in script.abilities {
            if let Does::Adds { name, .. } = a.does {
                let def = lucifron.adds.iter().find(|d| d.name == name).unwrap();
                let (boss, i) = lucifron.index_of(def).unwrap();
                assert!(!boss);
                assert_eq!(lucifron.kind(boss, i).map(|d| d.name), Some(name));
            }
        }
    }
    // An add is far softer than the boss it serves.
    assert!(lucifron.adds[0].hp < lucifron.bosses.last().unwrap().hp / 5.0);
}

// ---------------------------------------------------------------------------------------------
// Against the client's data
// ---------------------------------------------------------------------------------------------

/// Every spell a script or the library names is the 1.12 spell of that name, with a visual; the
/// pool's visual has a ground area kit; the yells' sound is a `SoundEntries` kit.
#[test]
fn every_raid_spell_and_sound_is_the_real_one() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = benilla_formats::open_chain(&data).expect("open chain");
    let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
    let visuals = benilla_formats::load_spell_visual_catalog(&mut chain).expect("SpellVisual.dbc");
    let named = [
        (IMPENDING_DOOM, "Impending Doom"),
        (LUCIFRONS_CURSE, "Lucifron's Curse"),
        (SHADOW_SHOCK, "Shadow Shock"),
        (DOMINATE_MIND, "Dominate Mind"),
        (CLEAVE, "Cleave"),
        (PANIC, "Panic"),
        (LAVA_BOMB, "Lava Bomb"),
        (FRENZY, "Frenzy"),
        (MAGMA_SPIT, "Magma Spit"),
        (BERSERK, "Berserk"),
        (REMOVE_LESSER_CURSE, "Remove Lesser Curse"),
        (REMOVE_CURSE, "Remove Curse"),
        (DISPEL_MAGIC, "Dispel Magic"),
        (CLEANSE, "Cleanse"),
        (TRANQUILIZING_SHOT, "Tranquilizing Shot"),
        (TREMOR_TOTEM, "Tremor Totem"),
        (POOL_VISUAL, "Flamestrike"),
    ];
    let mut bad = Vec::new();
    for (id, name) in named {
        match spells.get(id) {
            Some(d) if d.name == name => {
                if d.visual == 0 || visuals.stages(d.visual).is_none() {
                    bad.push(format!("{id} {name} has no visual"));
                }
            }
            Some(d) => bad.push(format!("{id} is {:?}, not {name}", d.name)),
            None => bad.push(format!("{id} ({name}) is not in Spell.dbc")),
        }
    }
    // Every scripted spell is one of those, under its own name.
    for script in [&LUCIFRON, &PROTECTOR, &MAGMADAR] {
        for a in script.abilities.iter().filter(|a| a.spell != 0) {
            if !named
                .iter()
                .any(|&(id, name)| id == a.spell && name == a.name)
            {
                bad.push(format!(
                    "{}: {} ({}) is unchecked",
                    script.name, a.name, a.spell
                ));
            }
        }
    }
    // Curses and magic are what the 1.12 spells are: Dispel Type 2 curse, 1 magic.
    for (id, kind) in [
        (LUCIFRONS_CURSE, 2),
        (IMPENDING_DOOM, 1),
        (DOMINATE_MIND, 1),
    ] {
        if spells.get(id).map(|d| d.dispel) != Some(kind) {
            bad.push(format!("{id}: dispel type is not {kind}"));
        }
    }
    let pool = spells
        .get(POOL_VISUAL)
        .and_then(|d| visuals.stages(d.visual));
    if !pool.is_some_and(|s| s.area_gate != 0 && s.area_effect != 0) {
        bad.push(format!("{POOL_VISUAL} has no ground area kit"));
    }
    let kits = benilla_formats::load_sound_kit_catalog(&mut chain).expect("SoundEntries.dbc");
    for sound in SOUNDS {
        if kits.by_name(sound).is_none() {
            bad.push(format!("sound {sound} is not a SoundEntries kit"));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// The bosses wear their 1.12 creature displays: Lucifron and his Protectors the Flamewaker
/// salamander in black, Magmadar the core hound.
#[test]
fn the_bosses_wear_their_creature_displays() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = benilla_formats::open_chain(&data).expect("open chain");
    let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
    let stage = |name: &str| data::MAPS.iter().position(|m| m.name == name).unwrap();
    let lucifron = data::roster(stage("Molten Core: Lucifron's Cavern"));
    let magmadar = data::roster(stage("Molten Core: Magmadar's Lair"));
    let cases = [
        (lucifron.bosses.last().unwrap(), "salamander", "black"),
        (&lucifron.adds[0], "salamander", "black"),
        (magmadar.bosses.last().unwrap(), "felbeast", "redblack"),
    ];
    for (def, model, skin) in cases {
        let m = creatures
            .model(def.display)
            .unwrap_or_else(|| panic!("{} display {} is not a row", def.name, def.display));
        let path = m.model_path.to_ascii_lowercase();
        let tex = m.textures[0]
            .clone()
            .unwrap_or_default()
            .to_ascii_lowercase();
        assert!(
            path.contains(model) && tex.ends_with(skin),
            "{}: {} {:?}",
            def.name,
            m.model_path,
            m.textures
        );
    }
}

/// The host's systems take their parameters without clashing (Bevy checks the access as a system
/// is initialised).
#[test]
fn the_raid_systems_initialise_without_conflicting_access() {
    let mut world = World::new();
    let mut boss = IntoSystem::into_system(run_bosses);
    boss.initialize(&mut world);
    let mut land = IntoSystem::into_system(land_telegraphs);
    land.initialize(&mut world);
    let mut pools = IntoSystem::into_system(burn_pools);
    pools.initialize(&mut world);
    let mut heroes = IntoSystem::into_system(tick_heroes);
    heroes.initialize(&mut world);
    let mut attach = IntoSystem::into_system(attach_scripts);
    attach.initialize(&mut world);
}
