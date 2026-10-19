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
    for script in SCRIPTS {
        for a in script.abilities {
            let warned = match a.does {
                Does::Ground {
                    shape,
                    windup,
                    blow,
                    ..
                } => Some((shape, windup, blow)),
                Does::Blink { windup, blow, .. } => Some((
                    Shape::Circle {
                        radius: SIGIL_RADIUS,
                    },
                    windup,
                    blow,
                )),
                _ => None,
            };
            if let Some((shape, windup, blow)) = warned {
                assert!(
                    windup >= shape.min_windup(),
                    "{}: {} winds up {windup}s",
                    script.name,
                    a.name
                );
                if blow.split {
                    // A stack to share: 2.5 s to gather, and two soakers make it fair.
                    assert!(windup >= 2.5, "{}: {}", script.name, a.name);
                    assert!(blow.damage / 2.0 <= 0.4, "{}: {}", script.name, a.name);
                } else {
                    assert!(blow.damage <= 0.35, "{}: {}", script.name, a.name);
                }
            }
            // The melee check's punishment warns as a circle and is half a life at most.
            if let Does::MeleeCheck {
                radius,
                windup,
                blow,
                grace,
                ..
            } = a.does
            {
                assert!(windup >= Shape::Circle { radius }.min_windup());
                assert!(blow.damage <= 0.5 && grace >= 2.0, "{}", script.name);
            }
            // A blink's blow warns as it arrives; an Inferno glows before its first pulse.
            let circle = Shape::Circle { radius: 1.0 }.min_windup();
            if let Does::Inferno { windup, .. } = a.does {
                assert!(windup >= circle, "{}", script.name);
            }
            assert!(BLINK_BLOW_WINDUP >= circle);
        }
        if let Some(e) = script.erupt {
            assert!(e.windup >= e.shape.min_windup(), "{}", script.name);
            assert!(e.blow.damage <= 0.35, "{}", script.name);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The brain: timers, thresholds, phases, party size
// ---------------------------------------------------------------------------------------------

static TEST_SCRIPT: Script = Script {
    name: "Test Dummy",
    death: None,
    erupt: None,
    master: None,
    aegis: 0.0,
    submits: None,
    rooted: false,
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
    world.init_resource::<Messages<Hit>>();
    world
}

fn hero_in(world: &mut World, state: HeroState, at: Vec3) -> Entity {
    world
        .spawn((Transform::from_translation(at), state, Hero::default()))
        .id()
}

/// A boss standing at the origin, as the pools' source.
fn boss_in(world: &mut World) -> Entity {
    let en = enemy_of(&data::roster(0).bosses[0], true);
    world.spawn((en, Transform::default())).id()
}

/// A test enemy of `def` with 1000 health, a boss or not.
fn enemy_of(def: &'static data::EnemyDef, boss: bool) -> Enemy {
    Enemy {
        def,
        hp: 1000.0,
        max_hp: 1000.0,
        speed: 0.0,
        damage: 10.0,
        radius: 1.0,
        xp: 1,
        boss,
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
    }
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
            foes: 0.0,
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
    for script in SCRIPTS {
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
        (GEHENNAS_CURSE, "Gehennas' Curse"),
        (RAIN_OF_FIRE, "Rain of Fire"),
        (SHADOW_BOLT, "Shadow Bolt"),
        (ANTIMAGIC_PULSE, "Antimagic Pulse"),
        (MAGMA_SHACKLES, "Magma Shackles"),
        (ERUPTION, "Eruption"),
        (GARR_ENRAGE, "Enrage"),
        (MASSIVE_ERUPTION, "Massive Eruption"),
        (ARCANE_EXPLOSION, "Arcane Explosion"),
        (SHAZZRAHS_CURSE, "Shazzrah's Curse"),
        (COUNTERSPELL, "Counterspell"),
        (GATE_OF_SHAZZRAH, "Gate of Shazzrah"),
        (LIVING_BOMB, "Living Bomb"),
        (INFERNO, "Inferno"),
        (IGNITE_MANA, "Ignite Mana"),
        (DARK_MENDING, "Dark Mending"),
        (INSPIRE, "Inspire"),
        (HAND_OF_RAGNAROS, "Hand of Ragnaros"),
        (FLAME_SPEAR, "Flame Spear"),
        (MAGMA_SPLASH, "Magma Splash"),
        (EARTHQUAKE, "Earthquake"),
        (MANGLE, "Mangle"),
        (SEPARATION_ANXIETY, "Separation Anxiety"),
        (ARMAGEDDON, "Armageddon"),
        (SHADOW_WORD_PAIN, "Shadow Word: Pain"),
        (PYROBLAST, "Pyroblast"),
        (GOLEMAGGS_TRUST, "Golemagg's Trust"),
        (MAGIC_REFLECTION, "Magic Reflection"),
        (DAMAGE_SHIELD, "Damage Shield"),
        (TELEPORT, "Teleport"),
        (BLAST_WAVE, "Blast Wave"),
        (WRATH_OF_RAGNAROS, "Wrath of Ragnaros"),
        (ELEMENTAL_FIRE, "Elemental Fire"),
        (MAGMA_BLAST, "Magma Blast"),
        (LAVA_BURST, "Lava Burst"),
        (RAGNAROS_EMERGE, "Ragnaros Emerge"),
    ];
    let mut bad = Vec::new();
    for (id, name) in named {
        match spells.get(id) {
            Some(d) if d.name == name => {
                // Lava Burst's only row has no visual of its own.
                let bare = d.visual == 0 || visuals.stages(d.visual).is_none();
                if bare && id != LAVA_BURST {
                    bad.push(format!("{id} {name} has no visual"));
                }
            }
            Some(d) => bad.push(format!("{id} is {:?}, not {name}", d.name)),
            None => bad.push(format!("{id} ({name}) is not in Spell.dbc")),
        }
    }
    // Every scripted spell is one of those, under its own name; and so is every spell a blink
    // lands, a fall sets off or a master gains.
    for script in SCRIPTS {
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
            if let Does::Blink { then, .. } = a.does {
                if !named.iter().any(|&(id, _)| id == then) {
                    bad.push(format!("{}: {then} is unchecked", script.name));
                }
            }
        }
        let erupt = script.erupt.map(|e| e.spell);
        let enrage = script.master.and_then(|m| m.enrage).map(|(spell, _)| spell);
        let bond = script.master.and_then(|m| m.bond).map(|b| b.spell);
        for spell in erupt.into_iter().chain(enrage).chain(bond) {
            if !named.iter().any(|&(id, _)| id == spell) {
                bad.push(format!("{}: {spell} is unchecked", script.name));
            }
        }
    }
    // Curses and magic are what the 1.12 spells are: Dispel Type 2 curse, 1 magic.
    for (id, kind) in [
        (LUCIFRONS_CURSE, 2),
        (IMPENDING_DOOM, 1),
        (DOMINATE_MIND, 1),
        (GEHENNAS_CURSE, 2),
        (SHAZZRAHS_CURSE, 2),
        (MAGMA_SHACKLES, 1),
        (IGNITE_MANA, 1),
        (SHADOW_WORD_PAIN, 1),
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
    // Majordomo's and Ragnaros's spoken lines are their own kits.
    for (kit, name) in [
        (EXECUTUS_AGGRO, "A_ExecutusAggro01"),
        (EXECUTUS_DEFEAT, "A_ExecutusDefeat01"),
        (RAGNAROS_ARRIVAL, "A_RagnarosArrival01"),
        (RAGNAROS_PURGED, "A_RagnarosSpecialAttack01"),
        (RAGNAROS_SERVANTS, "A_RagnarosReinforcements01"),
        (RAGNAROS_DEATH, "RagnarosDeath"),
    ] {
        if kits.get(kit).map(|k| k.name.as_str()) != Some(name) {
            bad.push(format!("kit {kit} is not {name}"));
        }
    }
    for sound in SOUNDS {
        if kits.by_name(sound).is_none() {
            bad.push(format!("sound {sound} is not a SoundEntries kit"));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// A stage's final boss.
fn final_boss(stage: &str) -> &'static data::EnemyDef {
    data::roster(stage_of(stage)).bosses.last().unwrap()
}

/// A Molten Core add, by name.
fn add_named(name: &str) -> &'static data::EnemyDef {
    data::roster(stage_of("Molten Core: Lucifron's Cavern"))
        .adds
        .iter()
        .find(|d| d.name == name)
        .unwrap()
}

/// The bosses wear their 1.12 creature displays: Lucifron and his Protectors the Flamewaker
/// salamander in black, Magmadar the core hound, Gehennas the salamander in dark red, Garr the
/// black rock elemental and his Firesworn a small one, Shazzrah the salamander in blue, Baron
/// Geddon the fire elemental, Sulfuron the salamander in red and his Priests in dark red, Golemagg
/// the molten giant and his Core Ragers core hounds.
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
        (
            final_boss("Molten Core: Gehennas' Cavern"),
            "salamander",
            "darkred",
        ),
        (add_named(FLAMEWAKER_NAME), "salamander", "skinred"),
        (
            final_boss("Molten Core: Garr's Cavern"),
            "elementalearth",
            "rockelementalskinblack",
        ),
        (
            add_named(FIRESWORN_NAME),
            "elementalearth",
            "rockelementalskinblack",
        ),
        (
            final_boss("Molten Core: Shazzrah's Cavern"),
            "salamander",
            "blue",
        ),
        (
            final_boss("Molten Core: Baron Geddon's Cavern"),
            "fireelemental",
            "fireelementalskin",
        ),
        (
            final_boss("Molten Core: Sulfuron's Cavern"),
            "salamander",
            "skinred",
        ),
        (add_named(PRIEST_NAME), "salamander", "darkred"),
        (
            final_boss("Molten Core: Golemagg's Ridge"),
            "mountaingiant",
            "moltengiant",
        ),
        (add_named(CORE_RAGER_NAME), "felbeast", "redblack"),
        (
            final_boss("Molten Core: Majordomo's Camp"),
            "salamander",
            "black",
        ),
        (add_named(ELITE_NAME), "salamander", "skinred"),
        (add_named(HEALER_NAME), "salamander", "skinred"),
        (
            final_boss("Molten Core: Ragnaros' Lair"),
            "ragnaros",
            "ragnarosskin",
        ),
        (add_named(SON_NAME), "fireelemental", "fireelementalskin"),
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
    let mut rooted = IntoSystem::into_system(place_rooted);
    rooted.initialize(&mut world);
    let mut blinks = IntoSystem::into_system(land_blinks);
    blinks.initialize(&mut world);
    let mut bombs = IntoSystem::into_system(follow_bombs);
    bombs.initialize(&mut world);
    let mut guard = IntoSystem::into_system(guard_undying);
    guard.initialize(&mut world);
    let mut deaths = IntoSystem::into_system(on_deaths);
    deaths.initialize(&mut world);
}

// ---------------------------------------------------------------------------------------------
// Raid stages: the last boss after the trash, and its death wins the run
// ---------------------------------------------------------------------------------------------

fn stage_of(name: &str) -> usize {
    data::MAPS.iter().position(|m| m.name == name).unwrap()
}

/// The roster places a stage sends out over its first `secs`, and when.
fn walk_outs(stage: usize, secs: f32) -> Vec<(usize, f32)> {
    let map = &data::MAPS[stage];
    let count = data::roster(stage).bosses.len();
    let mut out = 0u32;
    let mut seen = Vec::new();
    let mut t = 0.0;
    while t <= secs {
        while let Some(i) = data::next_boss(map, t, out, count) {
            seen.push((i, t));
            out = i as u32 + 1;
        }
        t += 0.25;
    }
    seen
}

#[test]
fn only_the_molten_core_boss_stages_are_raids() {
    let raids: Vec<&str> = data::MAPS
        .iter()
        .filter(|m| m.raid)
        .map(|m| m.name)
        .collect();
    assert_eq!(
        raids,
        [
            "Molten Core: Lucifron's Cavern",
            "Molten Core: Magmadar's Lair",
            "Molten Core: Majordomo's Camp",
            "Molten Core: Ragnaros' Lair",
            "Molten Core: Gehennas' Cavern",
            "Molten Core: Garr's Cavern",
            "Molten Core: Shazzrah's Cavern",
            "Molten Core: Baron Geddon's Cavern",
            "Molten Core: Sulfuron's Cavern",
            "Molten Core: Golemagg's Ridge",
        ]
    );
    // Each new boss closes its own stage's roster.
    for (stage, boss) in [
        ("Molten Core: Gehennas' Cavern", "Gehennas"),
        ("Molten Core: Garr's Cavern", "Garr"),
        ("Molten Core: Shazzrah's Cavern", "Shazzrah"),
        ("Molten Core: Baron Geddon's Cavern", "Baron Geddon"),
        ("Molten Core: Sulfuron's Cavern", "Sulfuron Harbinger"),
        ("Molten Core: Golemagg's Ridge", "Golemagg the Incinerator"),
        ("Molten Core: Majordomo's Camp", "Majordomo Executus"),
        ("Molten Core: Ragnaros' Lair", "Ragnaros"),
    ] {
        assert_eq!(final_boss(stage).name, boss);
        assert!(script_for(boss).is_some(), "{boss} has a script");
        let outs = walk_outs(stage_of(stage), 900.0);
        assert_eq!(outs.last().map(|o| o.1), Some(data::RAID_BOSS_SECS));
    }
}

#[test]
fn a_raid_stages_last_boss_walks_out_at_six_minutes() {
    let stage = stage_of("Molten Core: Lucifron's Cavern");
    let last = data::roster(stage).bosses.len() - 1;
    let outs = walk_outs(stage, 900.0);
    // Two of the trash bosses on the clock, then Lucifron at 6:00, then nothing more.
    assert_eq!(outs, [(0, 135.0), (1, 270.0), (last, 360.0)]);
    assert_eq!(data::roster(stage).bosses[last].name, "Lucifron");
}

#[test]
fn a_normal_stage_keeps_its_boss_every_two_and_a_quarter_minutes_and_its_bell() {
    for stage in (0..data::MAPS.len()).filter(|&s| !data::MAPS[s].raid) {
        let count = data::roster(stage).bosses.len();
        let outs = walk_outs(stage, 900.0);
        let want: Vec<(usize, f32)> = (0..count.min(6))
            .map(|i| (i, (i as f32 + 1.0) * data::BOSS_EVERY_SECS))
            .collect();
        assert_eq!(outs, want, "{}", data::MAPS[stage].name);
        let map = &data::MAPS[stage];
        assert!(!data::won(map, data::VICTORY_SECS - 0.5, true));
        assert!(data::won(map, data::VICTORY_SECS, false));
    }
    const { assert!(data::RAID_BOSS_SECS < data::VICTORY_SECS) };
}

/// A world [`hero::hero_vitals`] and the final-boss watch run in: one standing hero on `stage`.
fn vitals_world(stage: usize, now: f32) -> (World, Entity) {
    let mut world = raid_world(0.1, now, 1);
    world.insert_resource(super::super::Selection {
        map: stage,
        stage,
        ..Default::default()
    });
    world.init_resource::<super::super::campfire::Camp>();
    world.init_resource::<NextState<super::super::Phase>>();
    world.init_resource::<Messages<crate::net::FieldChanged>>();
    world.init_resource::<Messages<enemies::Loot>>();
    let hero = world
        .spawn((
            Transform::default(),
            Hero::default(),
            hero_of(8),
            crate::net::Guid(7),
            crate::net::ObjectStore::default(),
            Marks::buffs(),
        ))
        .id();
    (world, hero)
}

fn game_over(world: &World) -> bool {
    matches!(
        world.resource::<NextState<super::super::Phase>>(),
        NextState::Pending(super::super::Phase::GameOver)
    )
}

#[test]
fn a_raid_stage_is_not_won_at_fifteen_minutes_while_its_boss_lives() {
    let stage = stage_of("Molten Core: Magmadar's Lair");
    let (mut world, _) = vitals_world(stage, 1200.0);
    world.run_system_once(hero::hero_vitals).unwrap();
    assert!(!world.resource::<Run>().won && !game_over(&world));

    // Elsewhere the bell still rings at 15:00.
    let (mut barrens, _) = vitals_world(0, data::VICTORY_SECS);
    barrens.run_system_once(hero::hero_vitals).unwrap();
    assert!(barrens.resource::<Run>().won && game_over(&barrens));
}

#[test]
fn killing_a_raid_stages_last_boss_wins_the_run() {
    let stage = stage_of("Molten Core: Lucifron's Cavern");
    let roster = data::roster(stage);
    let (mut world, _) = vitals_world(stage, 400.0);
    // A trash boss falling wins nothing.
    world.write_message(enemies::Loot {
        pos: Vec3::ZERO,
        xp: 10,
        boss: true,
        elite: false,
        def: &roster.bosses[0],
    });
    world.run_system_once(note_final_boss).unwrap();
    world.run_system_once(hero::hero_vitals).unwrap();
    assert!(!world.resource::<Run>().won && !game_over(&world));
    world.write_message(enemies::Loot {
        pos: Vec3::ZERO,
        xp: 200,
        boss: true,
        elite: false,
        def: roster.bosses.last().unwrap(),
    });
    world.run_system_once(note_final_boss).unwrap();
    world.run_system_once(hero::hero_vitals).unwrap();
    assert!(world.resource::<Run>().won, "Lucifron's death wins the run");
    assert!(game_over(&world));
}

#[test]
fn a_wiped_raid_is_lost_as_anywhere() {
    let stage = stage_of("Molten Core: Lucifron's Cavern");
    let (mut world, hero) = vitals_world(stage, 400.0);
    world.get_mut::<HeroState>(hero).unwrap().dead_t = Some(10.0);
    world.run_system_once(hero::hero_vitals).unwrap();
    assert!(!world.resource::<Run>().won);
}

// ---------------------------------------------------------------------------------------------
// Gehennas, Garr, Shazzrah, Baron Geddon, Sulfuron Harbinger, Golemagg
// ---------------------------------------------------------------------------------------------

/// An enemy of `def` at `at`, with 1000 health.
fn foe_at(world: &mut World, def: &'static data::EnemyDef, at: Vec3, boss: bool) -> Entity {
    world
        .spawn((enemy_of(def, boss), Transform::from_translation(at)))
        .id()
}

/// A scripted foe of `def` at `at`, with its brain and marks.
fn scripted_at(world: &mut World, def: &'static data::EnemyDef, at: Vec3) -> Entity {
    let e = foe_at(world, def, at, false);
    let script = script_for(def.name).unwrap();
    world
        .entity_mut(e)
        .insert((RaidBoss::new(script), Marks::debuffs()));
    e
}

fn drain_hits(world: &mut World) -> Vec<Hit> {
    world.resource_mut::<Messages<Hit>>().drain().collect()
}

fn drain_yells(world: &mut World) -> Vec<Yelled> {
    world.resource_mut::<Messages<Yelled>>().drain().collect()
}

#[test]
fn every_new_boss_roars_as_it_walks_out_and_cries_as_it_falls() {
    for (script, aggro, death) in [
        (&GEHENNAS, SALAMANDER_AGGRO, SALAMANDER_DEATH),
        (&GARR, ROCK_AGGRO, ROCK_DEATH),
        (&SHAZZRAH, SALAMANDER_AGGRO, SALAMANDER_DEATH),
        (&BARON_GEDDON, FIRE_AGGRO, FIRE_DEATH),
        (&SULFURON, SALAMANDER_AGGRO, SALAMANDER_DEATH),
        (&GOLEMAGG, GIANT_AGGRO, GIANT_DEATH),
    ] {
        let mut brain = RaidBoss::new(script);
        let first: Vec<&BossAbility> = brain.due(0.0625, 1.0, 1);
        let roar = first
            .iter()
            .find_map(|a| a.yell.filter(|y| y.kit != 0))
            .unwrap_or_else(|| panic!("{} roars on its first beat", script.name));
        let y = Yelled::of(&roar, script.name);
        assert_eq!((y.kit, y.text.as_str()), (aggro, ""), "{}", script.name);
        assert_eq!(script.death.map(|d| d.kit), Some(death), "{}", script.name);
    }
}

#[test]
fn gehennas_curses_rains_fire_bolts_and_enrages_on_schedule() {
    let fired = schedule(&GEHENNAS, 185.0, 5, |_| 1.0);
    assert_eq!(times(&fired, "Flamewakers"), [0.5]);
    assert_eq!(
        times(&fired, "Gehennas' Curse"),
        [8.0, 33.0, 58.0, 83.0, 108.0, 133.0, 158.0, 183.0]
    );
    assert_eq!(times(&fired, "Rain of Fire")[..3], [5.0, 17.0, 29.0]);
    assert_eq!(times(&fired, "Shadow Bolt")[..3], [7.0, 17.0, 27.0]);
    assert_eq!(times(&fired, "Soft enrage"), [120.0]);
    assert_eq!(times(&fired, "Berserk"), [180.0]);
    // Before the soft enrage the rain falls on the curse count, after it on five.
    let mut brain = RaidBoss::new(&GEHENNAS);
    let mut picks = Vec::new();
    for _ in 0..(185.0 / 0.0625) as usize {
        for a in brain.due(0.0625, 1.0, 5) {
            if a.name == "Rain of Fire" {
                picks.push((brain.clock, a.pick));
            }
        }
    }
    assert!(picks
        .iter()
        .all(|&(t, pick)| (t < 120.0) == (pick == Pick::Random(0))));
    assert!(picks.iter().any(|&(_, pick)| pick == Pick::Random(5)));
    // The Flamewakers are plain adds: the horde's Flamewakers share their name.
    assert!(script_for(FLAMEWAKER_NAME).is_none());
}

#[test]
fn gehennas_curse_quarters_the_healing_a_hero_receives_until_decursed() {
    let mut hero = hero_of(8);
    let max = hero.stats().max_hp;
    hero.hp = max - 100.0;
    hero.raid.add(Debuff::new(
        GEHENNAS_CURSE,
        Dispel::Curse,
        30.0,
        Payload::HealingTaken(0.25),
    ));
    let healed = crate::survivors::party::heal(&mut hero, 40.0);
    assert!(close(healed, 10.0), "{healed}");
    assert!(hero.raid.dispel(Dispel::Curse).is_some());
    let healed = crate::survivors::party::heal(&mut hero, 40.0);
    assert!(close(healed, 40.0), "{healed}");
}

#[test]
fn garr_calls_firesworn_by_party_size_pulses_shackles_and_detonates_them() {
    assert_eq!([1, 2, 3, 4, 5].map(add_count), [3, 4, 5, 6, 8]);
    assert_eq!(add_count(9), 8);
    let row = GARR
        .abilities
        .iter()
        .find(|a| a.name == "Firesworn")
        .unwrap();
    assert!(matches!(row.does, Does::Adds { count: 0, .. }));
    let fired = schedule(&GARR, 245.0, 5, |_| 1.0);
    assert_eq!(times(&fired, "Firesworn"), [0.5]);
    assert_eq!(times(&fired, "Antimagic Pulse")[..3], [10.0, 25.0, 40.0]);
    assert_eq!(times(&fired, "Magma Shackles")[..3], [6.0, 18.0, 30.0]);
    assert_eq!(
        times(&fired, "Massive Eruption"),
        [150.0, 165.0, 180.0, 195.0, 210.0, 225.0, 240.0]
    );
    assert_eq!(times(&fired, "Berserk"), [240.0]);
}

#[test]
fn magma_shackles_halve_the_pace_until_they_run_out_or_are_dispelled() {
    let mut hero = hero_of(8);
    let shackles = After::Slow {
        rate: 0.5,
        secs: 6.0,
    };
    after_blow(&mut hero, MAGMA_SHACKLES, shackles);
    assert!(close(hero.raid.move_rate(), 0.5));
    run_ticks(&mut hero.raid, 5.875, false);
    assert!(close(hero.raid.move_rate(), 0.5));
    run_ticks(&mut hero.raid, 0.25, false);
    assert!(close(hero.raid.move_rate(), 1.0), "ran out");
    after_blow(&mut hero, MAGMA_SHACKLES, shackles);
    assert!(hero.raid.dispel(Dispel::Magic).is_some());
    assert!(close(hero.raid.move_rate(), 1.0), "dispelled");
}

#[test]
fn antimagic_pulse_strips_shields_and_timed_buffs() {
    let mut hero = hero_of(5);
    hero.shield = 80.0;
    hero.shield_t = 6.0;
    hero.thorns_t = 4.0;
    hero.heal_t = 3.0;
    after_blow(&mut hero, ANTIMAGIC_PULSE, After::Strip);
    assert_eq!(
        (hero.shield, hero.shield_t, hero.thorns_t, hero.heal_t),
        (0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn an_eruption_hurts_the_other_firesworn_inside_it_but_not_bosses() {
    let mut world = raid_world(0.1, 2.0, 1);
    let firesworn = add_named(FIRESWORN_NAME);
    let garr = final_boss("Molten Core: Garr's Cavern");
    let source = foe_at(&mut world, garr, Vec3::NEG_X * 30.0, true);
    let near = foe_at(&mut world, firesworn, Vec3::X * 4.0, false);
    foe_at(&mut world, firesworn, Vec3::X * 20.0, false);
    foe_at(&mut world, garr, Vec3::X * 2.0, true);
    let hero = hero_in(&mut world, hero_of(8), Vec3::Z * 5.0);
    let full = hp(&world, hero);
    let erupt = FIRESWORN.erupt.unwrap();
    world.spawn((
        telegraph(erupt.shape, Vec3::ZERO, 2.0),
        Pending {
            source,
            spell: erupt.spell,
            blow: erupt.blow,
        },
    ));
    world.run_system_once(land_telegraphs).unwrap();
    let hits = drain_hits(&mut world);
    assert_eq!(hits.len(), 1, "only the Firesworn inside");
    assert_eq!(hits[0].target, near);
    assert!(close(hits[0].damage, 600.0));
    assert!(hp(&world, hero) < full, "and the hero inside");
}

#[test]
fn a_firesworns_fall_erupts_where_it_fell_and_enrages_garr() {
    let mut world = raid_world(0.1, 50.0, 3);
    world.init_resource::<Messages<enemies::Loot>>();
    world.init_resource::<Messages<Yelled>>();
    let garr = scripted_at(
        &mut world,
        final_boss("Molten Core: Garr's Cavern"),
        Vec3::ZERO,
    );
    let before = world.get::<Enemy>(garr).unwrap().damage;
    let at = Vec3::new(8.0, 0.0, 3.0);
    world.write_message(enemies::Loot {
        pos: at,
        xp: 1,
        boss: false,
        elite: true,
        def: add_named(FIRESWORN_NAME),
    });
    world.run_system_once(on_deaths).unwrap();
    let mut q = world.query::<(&Telegraph, &Pending)>();
    let (t, pending) = q.single(&world).unwrap();
    assert_eq!(t.origin(), at);
    assert!(close(t.land - t.start, 1.5));
    assert_eq!((pending.spell, pending.source), (ERUPTION, garr));
    assert!(pending.blow.foes > 0.0);
    assert!(close(
        world.get::<Enemy>(garr).unwrap().damage,
        before * 1.15
    ));
    assert!(world
        .get::<Marks>(garr)
        .unwrap()
        .spells()
        .any(|s| s == GARR_ENRAGE));

    // Garr's own fall: his death cry.
    world.write_message(enemies::Loot {
        pos: Vec3::ZERO,
        xp: 1,
        boss: true,
        elite: false,
        def: final_boss("Molten Core: Garr's Cavern"),
    });
    world.run_system_once(on_deaths).unwrap();
    let yells = drain_yells(&mut world);
    assert_eq!(yells.last().map(|y| y.kit), Some(ROCK_DEATH));
}

#[test]
fn shazzrah_explodes_curses_counterspells_and_blinks_faster_after_two_minutes() {
    let fired = schedule(&SHAZZRAH, 175.0, 3, |_| 1.0);
    assert_eq!(times(&fired, "Arcane Explosion")[..3], [6.0, 12.0, 18.0]);
    assert_eq!(times(&fired, "Shazzrah's Curse")[..2], [10.0, 35.0]);
    assert_eq!(times(&fired, "Counterspell")[..2], [12.0, 27.0]);
    assert_eq!(
        times(&fired, "Gate of Shazzrah"),
        [15.0, 35.0, 55.0, 75.0, 95.0, 115.0, 123.0, 135.0, 147.0, 159.0, 171.0]
    );
    assert_eq!(times(&fired, "Soft enrage"), [120.0]);
}

#[test]
fn shazzrahs_curse_doubles_the_magic_a_hero_takes_not_the_physical() {
    let curse = || {
        Debuff::new(
            SHAZZRAHS_CURSE,
            Dispel::Curse,
            20.0,
            Payload::DamageTaken(2.0),
        )
    };
    let mut cursed = hero_of(8);
    cursed.raid.add(curse());
    let mut fresh = hero_of(8);
    let magic = hurt_school(&mut cursed, 0.1, School::Arcane);
    assert!(close(magic, hurt_share(&mut fresh, 0.2)), "{magic}");
    let mut cursed = hero_of(8);
    cursed.raid.add(curse());
    let mut fresh = hero_of(8);
    let physical = hurt_school(&mut cursed, 0.1, School::Physical);
    assert!(close(physical, hurt_share(&mut fresh, 0.1)), "{physical}");
}

#[test]
fn counterspell_silences_and_a_stun_holds_for_their_seconds() {
    let mut hero = hero_of(8);
    after_blow(&mut hero, COUNTERSPELL, After::Silence(2.0));
    assert!(!hero.raid.can_cast() && hero.raid.silenced());
    run_ticks(&mut hero.raid, 1.875, false);
    assert!(!hero.raid.can_cast());
    run_ticks(&mut hero.raid, 0.25, false);
    assert!(hero.raid.can_cast(), "the silence ran out");

    after_blow(&mut hero, HAND_OF_RAGNAROS, After::Stun(2.0));
    assert!(!hero.raid.can_cast());
    assert_eq!(hero.raid.last_move.map(|m| m.kind), Some(MoveKind::Hold));
    let mut walk = Walk::default();
    assert_eq!(walk.step(&hero.raid, 0.1), Some(Vec3::ZERO), "held");
}

#[test]
fn a_gate_of_shazzrah_puts_him_on_the_sigil_drops_his_taunt_and_explodes() {
    let mut world = raid_world(0.1, 2.0, 2);
    let boss = foe_at(
        &mut world,
        final_boss("Molten Core: Shazzrah's Cavern"),
        Vec3::ZERO,
        true,
    );
    let tank = world.spawn_empty().id();
    world.get_mut::<Enemy>(boss).unwrap().taunted = Some((tank, 99.0));
    let sigil_at = Vec3::new(20.0, 0.0, -5.0);
    let blow = Blow::hit(0.12, School::Arcane);
    let sigil = world
        .spawn((
            telegraph(
                Shape::Circle {
                    radius: SIGIL_RADIUS,
                },
                sigil_at,
                2.5,
            ),
            BlinkTo {
                boss,
                spell: GATE_OF_SHAZZRAH,
                radius: 12.0,
                blow,
                then: ARCANE_EXPLOSION,
            },
        ))
        .id();
    world.run_system_once(land_blinks).unwrap();
    assert!(world.get_entity(sigil).is_ok(), "still winding up");
    world.resource_mut::<Run>().elapsed = 2.5;
    world.run_system_once(land_blinks).unwrap();
    assert!(world.get_entity(sigil).is_err());
    assert_eq!(world.get::<Transform>(boss).unwrap().translation, sigil_at);
    assert_eq!(world.get::<Enemy>(boss).unwrap().taunted, None);
    let mut q = world.query::<(&Telegraph, &Pending)>();
    let (t, pending) = q.single(&world).unwrap();
    assert_eq!(t.origin(), sigil_at);
    assert_eq!(t.shape, Shape::Circle { radius: 12.0 });
    assert!(close(t.land - t.start, BLINK_BLOW_WINDUP));
    assert_eq!((pending.spell, pending.blow), (ARCANE_EXPLOSION, blow));
}

#[test]
fn baron_geddon_bombs_infernos_ignites_and_bombs_faster_after_two_and_a_half_minutes() {
    let fired = schedule(&BARON_GEDDON, 175.0, 5, |_| 1.0);
    assert_eq!(
        times(&fired, "Living Bomb"),
        [10.0, 25.0, 40.0, 55.0, 70.0, 85.0, 100.0, 115.0, 130.0, 145.0, 160.0, 170.0]
    );
    assert_eq!(times(&fired, "Inferno")[..3], [20.0, 50.0, 80.0]);
    assert_eq!(times(&fired, "Ignite Mana")[..2], [8.0, 28.0]);
    assert_eq!(times(&fired, "Soft enrage"), [150.0]);
    // His last service, at 2% of his health.
    let low = schedule(
        &BARON_GEDDON,
        20.0,
        1,
        |t| if t < 12.0 { 0.5 } else { 0.01 },
    );
    // The channel and its ring, together.
    assert_eq!(times(&low, "Armageddon"), [12.0, 12.0]);
    let bomb = BARON_GEDDON
        .abilities
        .iter()
        .find(|a| a.name == "Living Bomb")
        .unwrap();
    assert!(matches!(
        bomb.does,
        Does::Debuff {
            dispel: Dispel::None,
            secs: 8.0,
            payload: Payload::Bomb { radius: 12.0, .. },
        }
    ));
}

#[test]
fn an_inferno_glows_then_pulses_harder_each_beat() {
    let beats = inferno_beats(2.0, 8, 1.0, 0.03, 0.25);
    assert_eq!(beats.len(), 8);
    assert_eq!((beats[0].0, beats[0].1), (0.0, 2.0), "the glow warns first");
    assert_eq!((beats[1].0, beats[1].1), (2.0, 3.0));
    assert_eq!((beats[7].0, beats[7].1), (8.0, 9.0));
    assert!(close(beats[0].2, 0.03) && close(beats[7].2, 0.03 * 2.75));
    // Each pulse's warning ends as the next begins: one ring at a time.
    for w in beats.windows(2) {
        assert!(close(w[0].1, w[1].0));
    }
}

#[test]
fn a_living_bombs_ring_follows_its_carrier_and_goes_with_the_bomb() {
    let mut world = raid_world(0.1, 1.0, 3);
    let mut carrier = hero_of(8);
    carrier.raid.add(Debuff::new(
        LIVING_BOMB,
        Dispel::None,
        8.0,
        Payload::Bomb {
            radius: 12.0,
            allies: 0.5,
            carrier: 0.2,
            foes: 0.5,
        },
    ));
    assert_eq!(carrier.raid.bomb(), Some((8.0, 12.0)));
    let hero = hero_in(&mut world, carrier, Vec3::ZERO);
    let ring = world
        .spawn((
            telegraph(Shape::Circle { radius: 12.0 }, Vec3::ZERO, 9.0),
            Carried {
                hero,
                spell: LIVING_BOMB,
            },
        ))
        .id();
    world.get_mut::<Transform>(hero).unwrap().translation = Vec3::X * 10.0;
    world.run_system_once(follow_bombs).unwrap();
    assert_eq!(
        world.get::<Telegraph>(ring).unwrap().origin(),
        Vec3::X * 10.0
    );
    world.get_mut::<HeroState>(hero).unwrap().raid.clear();
    world.run_system_once(follow_bombs).unwrap();
    assert!(world.get_entity(ring).is_err(), "gone with the bomb");
}

#[test]
fn a_living_bomb_blasts_the_horde_round_its_carrier_but_not_bosses() {
    let mut world = raid_world(0.25, 0.0, 1);
    let mut carrier = hero_of(8);
    carrier.raid.add(Debuff::new(
        LIVING_BOMB,
        Dispel::None,
        0.25,
        Payload::Bomb {
            radius: 12.0,
            allies: 0.5,
            carrier: 0.2,
            foes: 0.5,
        },
    ));
    let carrier = hero_in(&mut world, carrier, Vec3::ZERO);
    let def = &data::roster(0).enemies[0];
    let near = foe_at(&mut world, def, Vec3::X * 6.0, false);
    foe_at(&mut world, def, Vec3::X * 20.0, false);
    foe_at(&mut world, def, Vec3::X * 3.0, true);
    world.run_system_once(tick_heroes).unwrap();
    let hits = drain_hits(&mut world);
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].target, hits[0].source), (near, carrier));
    assert!(close(hits[0].damage, 500.0));
}

#[test]
fn a_bot_carrying_the_bomb_runs_away_from_its_party() {
    let leader = Vec3::new(5.0, 0.0, 5.0);
    let spot = bomb_escape(leader + Vec3::X * 2.0, leader, 12.0);
    assert!(close(spot.distance(leader), 12.0 + BOMB_MARGIN));
    assert!(spot.x > leader.x, "straight away");
    // Standing on its leader it still picks a way out.
    let spot = bomb_escape(leader, leader, 12.0);
    assert!(close(spot.distance(leader), 12.0 + BOMB_MARGIN));
}

#[test]
fn sulfuron_calls_priests_spears_stuns_and_inspires_at_half_health_and_two_and_a_half_minutes() {
    let fired = schedule(&SULFURON, 215.0, 5, |t| if t < 60.0 { 1.0 } else { 0.4 });
    assert_eq!(times(&fired, "Flamewaker Priests"), [0.5]);
    assert_eq!(times(&fired, "Flame Spear")[..2], [8.0, 20.0]);
    assert_eq!(times(&fired, "Hand of Ragnaros")[..2], [14.0, 34.0]);
    assert_eq!(times(&fired, "Inspire"), [60.0, 150.0]);
    assert_eq!(times(&fired, "Berserk"), [210.0]);
    let priest = schedule(&FLAMEWAKER_PRIEST, 30.0, 5, |_| 1.0);
    assert_eq!(times(&priest, "Dark Mending"), [5.0, 14.0, 23.0]);
}

#[test]
fn dark_mending_heals_the_most_hurt_in_reach_unless_a_stun_breaks_it() {
    let mut world = World::new();
    let [a, b, c, d] = [(); 4].map(|_| world.spawn_empty().id());
    let allies = [
        (a, Vec3::ZERO, 1.0),
        (b, Vec3::X * 10.0, 0.4),
        (c, Vec3::X * 50.0, 0.2),
        (d, Vec3::X * 20.0, 0.7),
    ];
    assert_eq!(mend_target(Vec3::ZERO, 40.0, &allies), Some(b));
    assert_eq!(
        mend_target(Vec3::ZERO, 40.0, &allies[..1]),
        None,
        "none hurt"
    );
    let cast = Casting {
        spell: DARK_MENDING,
        done: 12.5,
        share: 0.15,
        range: 40.0,
        ring: a,
    };
    assert_eq!(cast_state(&cast, 11.0, false), CastState::Casting);
    assert_eq!(cast_state(&cast, 11.0, true), CastState::Interrupted);
    assert_eq!(cast_state(&cast, 12.5, false), CastState::Done);
}

#[test]
fn golemagg_splashes_his_tank_and_quakes_below_a_tenth() {
    let fired = schedule(&GOLEMAGG, 130.0, 5, |t| if t < 100.0 { 0.5 } else { 0.05 });
    assert_eq!(times(&fired, "Core Ragers"), [0.5]);
    assert_eq!(times(&fired, "Magma Splash").len(), 130);
    assert_eq!(times(&fired, "Soft enrage"), [100.0]);
    assert_eq!(times(&fired, "Earthquake")[..3], [102.0, 108.0, 114.0]);
    let full = schedule(&GOLEMAGG, 245.0, 5, |_| 1.0);
    assert!(times(&full, "Earthquake").is_empty());
    assert_eq!(times(&full, "Berserk"), [240.0]);
}

#[test]
fn a_core_rager_refuses_to_die_while_golemagg_stands_and_falls_with_him() {
    let mut world = raid_world(0.1, 10.0, 2);
    world.init_resource::<Messages<Yelled>>();
    let golemagg = scripted_at(
        &mut world,
        final_boss("Molten Core: Golemagg's Ridge"),
        Vec3::ZERO,
    );
    let rager = scripted_at(&mut world, add_named(CORE_RAGER_NAME), Vec3::X * 25.0);
    world.run_system_once(guard_undying).unwrap();
    world.flush();
    assert!(world.get::<Undying>(rager).is_some(), "tied to its master");
    // Below half it heals to full, with its emote.
    world.get_mut::<Enemy>(rager).unwrap().hp = 300.0;
    world.run_system_once(guard_undying).unwrap();
    assert!(close(world.get::<Enemy>(rager).unwrap().hp, 1000.0));
    let yells = drain_yells(&mut world);
    assert_eq!(
        yells.last().map(|y| y.text.as_str()),
        Some("Core Rager refuses to die while its master is in trouble.")
    );
    assert_eq!(undying_heal(600.0, 1000.0), None);
    assert_eq!(undying_heal(400.0, 1000.0), Some(1000.0));
    // Golemagg falls: the tie is cut and a killing blow lands.
    world.get_mut::<Enemy>(golemagg).unwrap().dead = true;
    world.run_system_once(guard_undying).unwrap();
    world.flush();
    assert!(world.get::<Undying>(rager).is_none());
    let hits = drain_hits(&mut world);
    assert!(hits.iter().any(|h| h.target == rager && h.damage > 1000.0));
}

#[test]
fn the_new_adds_are_scripted_and_tied_to_their_masters() {
    for (name, master) in [
        (FIRESWORN_NAME, "Garr"),
        (CORE_RAGER_NAME, "Golemagg the Incinerator"),
    ] {
        let script = script_for(name).unwrap();
        assert_eq!(script.master.map(|m| m.name), Some(master));
        assert!(script_for(master).is_some());
    }
    assert!(FIRESWORN.erupt.is_some() && FIRESWORN.master.unwrap().enrage.is_some());
    assert!(CORE_RAGER.master.unwrap().undying);
    assert_eq!(script_for(PRIEST_NAME).map(|s| s.name), Some(PRIEST_NAME));
}

/// Each new boss's aggro roar and death cry are its display's own `CreatureSoundData` kits.
#[test]
fn the_new_bosses_voices_are_their_creatures_own() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = benilla_formats::open_chain(&data).expect("open chain");
    let voices = benilla_formats::load_creature_voice_catalog(&mut chain).expect("voices");
    let kits = benilla_formats::load_sound_kit_catalog(&mut chain).expect("SoundEntries.dbc");
    let mut bad = Vec::new();
    for (stage, script) in [
        ("Molten Core: Gehennas' Cavern", &GEHENNAS),
        ("Molten Core: Garr's Cavern", &GARR),
        ("Molten Core: Shazzrah's Cavern", &SHAZZRAH),
        ("Molten Core: Baron Geddon's Cavern", &BARON_GEDDON),
        ("Molten Core: Sulfuron's Cavern", &SULFURON),
        ("Molten Core: Golemagg's Ridge", &GOLEMAGG),
    ] {
        let def = final_boss(stage);
        let aggro = script
            .abilities
            .iter()
            .find_map(|a| a.yell.filter(|y| y.kit != 0))
            .map(|y| y.kit);
        let death = script.death.map(|y| y.kit);
        match voices.for_display(def.display) {
            Some(v) => {
                if aggro != Some(v.aggro) || death != Some(v.death) {
                    bad.push(format!(
                        "{}: aggro {aggro:?} death {death:?}, the display's are {} {}",
                        def.name, v.aggro, v.death
                    ));
                }
            }
            None => bad.push(format!("{} display {} has no voice", def.name, def.display)),
        }
        for kit in aggro.into_iter().chain(death) {
            if kits.get(kit).is_none() {
                bad.push(format!("{}: kit {kit} is not in SoundEntries", def.name));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

// ---------------------------------------------------------------------------------------------
// Majordomo Executus, Ragnaros, and the side mechanics
// ---------------------------------------------------------------------------------------------

fn ally(world: &mut World, name: &'static str, pos: Vec3) -> Ally {
    Ally {
        e: world.spawn_empty().id(),
        pos,
        hp: 1000.0,
        max_hp: 1000.0,
        name,
    }
}

#[test]
fn majordomo_wards_his_adds_in_turn_and_teleports_his_tank() {
    let fired = schedule(&MAJORDOMO, 245.0, 5, |_| 1.0);
    assert_eq!(times(&fired, "Flamewaker Elites"), [0.5]);
    assert_eq!(times(&fired, "Flamewaker Healers"), [0.5]);
    assert_eq!(
        times(&fired, "Magic Reflection"),
        [15.0, 75.0, 135.0, 195.0]
    );
    assert_eq!(times(&fired, "Damage Shield"), [45.0, 105.0, 165.0, 225.0]);
    assert_eq!(times(&fired, "Teleport")[..3], [20.0, 40.0, 60.0]);
    assert_eq!(times(&fired, "Berserk"), [240.0]);
    let adds: Vec<&str> = MAJORDOMO
        .abilities
        .iter()
        .filter_map(|a| match a.does {
            Does::Adds { name, count, .. } => Some((name, count)),
            _ => None,
        })
        .map(|(name, count)| {
            assert_eq!(count, 4);
            name
        })
        .collect();
    assert_eq!(adds, [ELITE_NAME, HEALER_NAME]);
}

#[test]
fn a_ward_answers_only_the_blows_it_is_for_and_never_more_than_its_cap() {
    let reflect = Ward {
        kind: WardKind::Reflect,
        spell: MAGIC_REFLECTION,
        until: 10.0,
    };
    let thorns = Ward {
        kind: WardKind::Thorns,
        spell: DAMAGE_SHIELD,
        ..reflect
    };
    assert!(close(
        reflect.answer(5.0, Some(School::Fire), 100.0, 10_000.0),
        50.0
    ));
    assert_eq!(
        reflect.answer(5.0, Some(School::Physical), 100.0, 10_000.0),
        0.0
    );
    assert_eq!(reflect.answer(5.0, None, 100.0, 10_000.0), 0.0);
    assert!(close(thorns.answer(5.0, None, 100.0, 10_000.0), 50.0));
    assert!(close(
        thorns.answer(5.0, Some(School::Physical), 100.0, 10_000.0),
        50.0
    ));
    assert_eq!(
        thorns.answer(5.0, Some(School::Arcane), 100.0, 10_000.0),
        0.0
    );
    // At most 4% of the striker's health a blow, and nothing once it has lapsed.
    assert!(close(
        reflect.answer(5.0, Some(School::Fire), 10_000.0, 1000.0),
        40.0
    ));
    assert_eq!(
        reflect.answer(10.0, Some(School::Fire), 100.0, 10_000.0),
        0.0
    );
}

#[test]
fn majordomo_holds_a_fifth_of_his_health_and_yields_once_his_adds_fall() {
    let mut world = World::new();
    let camp = final_boss("Molten Core: Majordomo's Camp");
    let mut brain = RaidBoss::new(&MAJORDOMO);
    let mut enemy = enemy_of(camp, true);
    enemy.hp = 50.0;
    let elite = ally(&mut world, ELITE_NAME, Vec3::X * 5.0);
    let me = ally(&mut world, camp.name, Vec3::ZERO);
    brain.clock = 1.0;
    let ties = tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me]);
    assert!(close(enemy.hp, 200.0), "the Aegis of Ragnaros");
    assert_eq!(ties.shield, Some(true), "he can't die");
    assert_eq!(ties.submitted, None, "too soon to count his adds gone");
    brain.clock = 30.0;
    let ties = tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me, elite]);
    assert_eq!(ties.submitted, None, "an Elite still stands");
    let ties = tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me]);
    let line = ties.submitted.expect("he yields");
    assert_eq!(line.kit, EXECUTUS_DEFEAT);
    assert!(line.text.contains("I submit!"));
    assert_eq!(
        tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me]).submitted,
        None,
        "once"
    );
    assert!(enemy.stun_t > 1000.0, "he fights no more");
}

#[test]
fn ragnaros_wraths_burns_bursts_and_submerges_at_three_minutes() {
    assert!(RAGNAROS.rooted);
    let fired = schedule(&RAGNAROS, 185.0, 5, |_| 1.0);
    assert_eq!(
        times(&fired, "Wrath of Ragnaros"),
        [25.0, 50.0, 75.0, 100.0, 125.0, 150.0, 175.0]
    );
    assert_eq!(times(&fired, "Elemental Fire")[..2], [8.0, 20.0]);
    assert_eq!(times(&fired, "Lava Burst")[..2], [6.0, 15.0]);
    assert_eq!(times(&fired, "Submerge"), [180.0]);
    let wrath = RAGNAROS
        .abilities
        .iter()
        .find(|a| a.name == "Wrath of Ragnaros")
        .unwrap();
    let y = Yelled::of(wrath.yell.as_ref().unwrap(), "Ragnaros");
    assert_eq!(
        (y.text.as_str(), y.kit),
        ("BY FIRE BE PURGED!", RAGNAROS_PURGED)
    );
    assert!(matches!(
        wrath.does,
        Does::Ground {
            shape: Shape::Circle { radius: MELEE_RING },
            ..
        }
    ));
    let aggro = Yelled::of(RAGNAROS.abilities[0].yell.as_ref().unwrap(), "Ragnaros");
    assert!(aggro.text.starts_with("TOO SOON!") && aggro.kit == RAGNAROS_ARRIVAL);
}

#[test]
fn submerged_ragnaros_is_untouchable_until_his_sons_fall_then_opens_with_wrath() {
    let mut world = World::new();
    let lair = final_boss("Molten Core: Ragnaros' Lair");
    let me = ally(&mut world, lair.name, Vec3::ZERO);
    let son = ally(&mut world, SON_NAME, Vec3::X * 10.0);
    let mut brain = RaidBoss::new(&RAGNAROS);
    let mut enemy = enemy_of(lair, true);
    for _ in 0..(180.0 / 0.0625) as usize {
        brain.due(0.0625, 1.0, 5);
    }
    brain.submerge(800.0, 90.0, SON_NAME, "Wrath of Ragnaros");
    assert_eq!(brain.phase, 2);
    let mut fired = Vec::new();
    for _ in 0..(10.0 / 0.0625) as usize {
        fired.extend(brain.due(0.0625, 1.0, 5).iter().map(|a| a.name));
    }
    assert!(!fired.contains(&"Wrath of Ragnaros"), "idle below");
    enemy.hp = 100.0;
    let ties = tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me, son]);
    assert!(close(enemy.hp, 800.0), "untouchable");
    assert_eq!(ties.shield, Some(true));
    assert!(!ties.emerged && enemy.stun_t > 0.0, "his Sons still stand");
    let ties = tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me]);
    assert!(ties.emerged && brain.phase == 1);
    assert_eq!(ties.shield, Some(false), "touchable again");
    let next: Vec<&str> = brain.due(0.0625, 1.0, 5).iter().map(|a| a.name).collect();
    assert!(
        next.contains(&"Wrath of Ragnaros"),
        "opens with Wrath: {next:?}"
    );

    // His time up, he rises even with Sons standing.
    let mut brain = RaidBoss::new(&RAGNAROS);
    brain.submerge(800.0, 90.0, SON_NAME, "Wrath of Ragnaros");
    brain.clock = 91.0;
    assert!(tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me, son]).emerged);
}

#[test]
fn magma_blast_comes_only_after_three_seconds_with_nobody_in_melee() {
    let mut alone = 0.0;
    for _ in 0..11 {
        assert!(!melee_check(&mut alone, false, 0.25, 3.0));
    }
    assert!(melee_check(&mut alone, false, 0.25, 3.0), "3 s alone");
    assert_eq!(alone, 0.0, "and the count starts over");
    for _ in 0..8 {
        melee_check(&mut alone, false, 0.25, 3.0);
    }
    assert!(!melee_check(&mut alone, true, 0.25, 3.0));
    assert_eq!(alone, 0.0, "someone stepped in");
    let ring = melee_ring(Vec3::X, 5.0);
    assert_eq!(ring.shape, Shape::Circle { radius: MELEE_RING });
    // Standing from the start, never gone.
    assert!(ring.until.is_infinite() && ring.land <= 5.0);
}

#[test]
fn ragnaros_drops_his_hoard_and_his_fall_clears_the_core() {
    let mut world = raid_world(0.1, 400.0, 1);
    world.init_resource::<Messages<enemies::Loot>>();
    world.init_resource::<Messages<Yelled>>();
    world.init_resource::<crate::survivors::hud::ResultsNotes>();
    for _ in 0..20 {
        world.write_message(enemies::Loot {
            pos: Vec3::ZERO,
            xp: 1,
            boss: true,
            elite: false,
            def: final_boss("Molten Core: Ragnaros' Lair"),
        });
    }
    world.run_system_once(on_deaths).unwrap();
    let yells = drain_yells(&mut world);
    assert!(yells
        .iter()
        .all(|y| y.kit == RAGNAROS_DEATH && y.text.contains("Molten Core is cleared")));
    let notes = &world.resource::<crate::survivors::hud::ResultsNotes>().0;
    assert!(!notes.is_empty(), "twenty falls drop something");
    for (text, _) in notes {
        assert!(
            RAGNAROS_LOOT
                .iter()
                .any(|(id, name, _)| text.contains(name) && text.contains(&id.to_string())),
            "{text}"
        );
    }
    assert_eq!(
        RAGNAROS_LOOT.iter().map(|l| l.1).collect::<Vec<_>>(),
        [
            "Sulfuron Ingot",
            "Eye of Sulfuras",
            "Bonereaver's Edge",
            "Band of Accuria"
        ]
    );
}

#[test]
fn golemaggs_pyroblast_is_shared_by_those_who_stack_under_it() {
    assert!(close(split_share(0.8, true, 2, false), 0.4));
    assert!(
        close(split_share(0.8, true, 1, true), 0.4),
        "the Companion soaks"
    );
    assert!(close(split_share(0.8, true, 0, false), 0.8));
    assert!(close(split_share(0.8, false, 4, false), 0.8));
    let mut world = raid_world(0.1, 3.0, 2);
    let source = world.spawn_empty().id();
    let a = hero_in(&mut world, hero_of(8), Vec3::X);
    let b = hero_in(&mut world, hero_of(8), Vec3::NEG_X);
    let full = hp(&world, a);
    world.spawn((
        telegraph(Shape::Circle { radius: 8.0 }, Vec3::ZERO, 3.0),
        Pending {
            source,
            spell: PYROBLAST,
            blow: Blow::hit(0.8, School::Fire).split(),
        },
    ));
    world.run_system_once(land_telegraphs).unwrap();
    let want = hurt_share(&mut hero_of(8), 0.4);
    assert!((full - hp(&world, a) - want).abs() < 1.0);
    assert!((full - hp(&world, b) - want).abs() < 1.0);
}

#[test]
fn shadow_word_pain_burns_until_it_runs_out_or_is_dispelled() {
    let pain = || Debuff::new(SHADOW_WORD_PAIN, Dispel::Magic, 12.0, Payload::Burn(0.02));
    let mut raid = HeroRaid::default();
    raid.add(pain());
    let total: f32 = run_ticks(&mut raid, 12.0, false)
        .iter()
        .map(|e| match e {
            RaidEvent::Hit { damage, .. } => *damage,
            _ => 0.0,
        })
        .sum();
    assert!(close(total, 0.24), "{total}");
    assert!(raid.debuffs.is_empty());
    raid.add(pain());
    assert!(raid.dispel(Dispel::Magic).is_some());
    assert!(run_ticks(&mut raid, 2.0, false).is_empty());
    let priest = FLAMEWAKER_PRIEST
        .abilities
        .iter()
        .find(|a| a.name == "Shadow Word: Pain")
        .unwrap();
    assert!(matches!(
        priest.does,
        Does::Debuff {
            dispel: Dispel::Magic,
            payload: Payload::Burn(_),
            ..
        }
    ));
}

#[test]
fn golemaggs_trust_and_separation_anxiety_double_an_adds_blows() {
    let mut world = World::new();
    // A Core Rager near Golemagg hits twice as hard; apart, as before.
    let golemagg = ally(&mut world, "Golemagg the Incinerator", Vec3::ZERO);
    let mut brain = RaidBoss::new(&CORE_RAGER);
    let mut rager = enemy_of(add_named(CORE_RAGER_NAME), false);
    let base = rager.damage;
    let ties = tie_up(&mut brain, &mut rager, Vec3::X * 10.0, &[golemagg]);
    assert_eq!(ties.bond, Some((GOLEMAGGS_TRUST, true)));
    assert!(close(rager.damage, base * 2.0));
    let ties = tie_up(&mut brain, &mut rager, Vec3::X * 30.0, &[golemagg]);
    assert_eq!(ties.bond, Some((GOLEMAGGS_TRUST, false)));
    assert!(close(rager.damage, base));
    // A Firesworn dragged 40 yd from Garr does.
    let garr = ally(&mut world, "Garr", Vec3::ZERO);
    let mut brain = RaidBoss::new(&FIRESWORN);
    let mut firesworn = enemy_of(add_named(FIRESWORN_NAME), false);
    let near = tie_up(&mut brain, &mut firesworn, Vec3::X * 10.0, &[garr]);
    assert_eq!(near.bond, None);
    let far = tie_up(&mut brain, &mut firesworn, Vec3::X * 45.0, &[garr]);
    assert_eq!(far.bond, Some((SEPARATION_ANXIETY, true)));
    assert!(close(firesworn.damage, base * 2.0));
}

#[test]
fn armageddon_makes_geddon_untouchable_while_its_ring_winds_up() {
    let low = schedule(
        &BARON_GEDDON,
        20.0,
        1,
        |t| if t < 12.0 { 0.5 } else { 0.01 },
    );
    assert_eq!(
        times(&low, "Armageddon"),
        [12.0, 12.0],
        "the channel and its ring"
    );
    let ring = BARON_GEDDON
        .abilities
        .iter()
        .find(|a| matches!(a.does, Does::Ground { .. }) && a.name == "Armageddon")
        .unwrap();
    assert!(matches!(
        ring.does,
        Does::Ground {
            shape: Shape::Circle { radius: 35.0 },
            windup: 8.0,
            ..
        }
    ));
    let mut world = World::new();
    let geddon = final_boss("Molten Core: Baron Geddon's Cavern");
    let me = ally(&mut world, geddon.name, Vec3::ZERO);
    let mut brain = RaidBoss::new(&BARON_GEDDON);
    let mut enemy = enemy_of(geddon, true);
    brain.clock = 100.0;
    brain.frozen = Some((20.0, 108.0));
    enemy.hp = 1.0;
    assert_eq!(
        tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me]).shield,
        Some(true)
    );
    assert!(close(enemy.hp, 20.0));
    brain.clock = 108.0;
    assert_eq!(
        tie_up(&mut brain, &mut enemy, Vec3::ZERO, &[me]).shield,
        Some(false)
    );
}
