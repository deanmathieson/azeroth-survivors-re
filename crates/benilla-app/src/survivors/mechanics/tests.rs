use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use super::super::abilities::{Hit, PendingHits};
use super::super::bonus::{Bonus, StatKey};
use super::super::cards::{self, Rarity};
use super::super::data::{self, ABILITIES, LEGENDARIES};
use super::super::hud::{self, Choice};
use super::super::{Owned, Rng, Run};
use super::target::{self, Foe};
use super::*;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

fn foe(i: u32, x: f32, z: f32) -> Foe {
    Foe {
        e: Entity::from_raw_u32(i).unwrap(),
        pos: Vec3::new(x, 0.0, z),
        hp: 100.0,
        max_hp: 100.0,
        radius: 0.0,
        boss: false,
        frozen: false,
        burns: 0,
    }
}

fn ids(foes: &[Foe], picked: &[usize]) -> Vec<u32> {
    picked.iter().map(|&i| foes[i].e.index_u32()).collect()
}

fn spec(trigger: Trigger, effect: Effect) -> MechanicSpec {
    MechanicSpec::new(trigger, effect)
}

fn hero_with(key: &str, level: u8) -> HeroState {
    let mut hero = HeroState::default();
    hero.abilities.push(Owned {
        def: data::ability(key),
        level,
        cd: 0.0,
    });
    hero
}

#[test]
fn every_ability_has_one_legendary_and_every_legendary_a_real_ability() {
    assert_eq!(LEGENDARIES.len(), ABILITIES.len());
    for a in ABILITIES {
        let l = data::legendary(a.key).unwrap_or_else(|| panic!("{} has no legendary", a.key));
        assert!(!l.name.is_empty() && !l.text.is_empty(), "{}", a.key);
        assert!(!l.text.contains('\n'), "{}: one line", a.key);
        assert!(
            !l.mechanics.is_empty(),
            "{}: a transformation does something",
            a.key
        );
        assert_ne!(l.visual_spell, 0, "{}", a.key);
    }
    for (i, l) in LEGENDARIES.iter().enumerate() {
        assert!(
            ABILITIES.iter().any(|a| a.key == l.ability),
            "{} transforms no ability",
            l.ability
        );
        assert!(
            LEGENDARIES[i + 1..].iter().all(|o| o.ability != l.ability),
            "{} has two legendaries",
            l.ability
        );
    }
}

#[test]
fn legendary_mechanics_are_valid_and_listen_to_their_own_ability() {
    for l in LEGENDARIES {
        for m in l.mechanics {
            if let Err(why) = m.validate() {
                panic!("{}: {why} in {m:?}", l.ability);
            }
            let of = match m.trigger {
                Trigger::OnCast(of)
                | Trigger::OnHit { of, .. }
                | Trigger::OnCrit { of, .. }
                | Trigger::OnKill { of, .. } => Some(of),
                _ => None,
            };
            if let Some(of) = of {
                assert_eq!(of, Of::Ability(l.ability), "{}: {m:?}", l.ability);
            }
        }
    }
}

#[test]
fn validation_catches_nonsense() {
    let hit = Trigger::OnHit {
        of: Of::Any,
        chance: 1.0,
    };
    let bad = [
        spec(
            Trigger::Periodic { every: 0.0 },
            Effect::Shield {
                amount: 10.0,
                duration: 5.0,
            },
        ),
        spec(
            Trigger::OnHit {
                of: Of::Any,
                chance: 1.5,
            },
            Effect::Boomerang,
        ),
        spec(
            Trigger::OnLowHealth { threshold: 1.0 },
            Effect::ResetCooldowns,
        ),
        spec(
            hit,
            Effect::Nova {
                school: School::Fire,
                radius: -1.0,
                damage: 5.0,
                knockback: 0.0,
                cc: None,
            },
        ),
        spec(
            hit,
            Effect::BurningGround {
                school: School::Fire,
                radius: 3.0,
                dps: 5.0,
                duration: 0.0,
            },
        ),
        spec(
            hit,
            Effect::Chain {
                jumps: 0,
                falloff: 0.5,
            },
        ),
        spec(
            hit,
            Effect::Execute {
                threshold: 0.3,
                mult: 1.0,
            },
        ),
        spec(hit, Effect::Pierce { n: 2 }).visual(0),
        spec(hit, Effect::Pierce { n: 2 }).at(Aim::Spread { radius: 0.0, n: 2 }),
        spec(hit, Effect::Pierce { n: 2 }).cooldown(-1.0),
    ];
    for m in bad {
        assert!(m.validate().is_err(), "{m:?}");
    }
    assert!(spec(hit, Effect::Pierce { n: 2 }).validate().is_ok());
}

#[test]
fn periodic_fires_once_a_period_and_once_for_a_long_frame() {
    let mut clock = 0.0;
    let fired: Vec<bool> = (0..8).map(|_| periodic(&mut clock, 2.0, 0.5)).collect();
    assert_eq!(
        fired,
        [false, false, false, true, false, false, false, true]
    );
    // A five-second hitch fires once, not twice, and keeps its place in the period.
    let mut clock = 0.0;
    assert!(periodic(&mut clock, 2.0, 5.0));
    assert!(clock <= 2.0);
}

#[test]
fn low_health_fires_once_per_fall() {
    let mut armed = true;
    let seen: Vec<bool> = [0.9, 0.3, 0.2, 0.1, 0.6, 0.25]
        .into_iter()
        .map(|frac| low_health(&mut armed, frac, 0.35))
        .collect();
    assert_eq!(seen, [false, true, false, false, false, true]);
}

#[test]
fn chance_rolls_are_seeded_and_fair() {
    let mut rng = Rng(42);
    assert!((0..100).all(|_| chance(&mut rng, 1.0)));
    assert!((0..100).all(|_| !chance(&mut rng, 0.0)));
    let hits = (0..10_000).filter(|_| chance(&mut rng, 0.3)).count();
    assert!((2_800..3_200).contains(&hits), "{hits}");
    // The same seed, the same rolls.
    let roll = |seed| {
        let mut rng = Rng(seed);
        (0..32).map(|_| chance(&mut rng, 0.5)).collect::<Vec<_>>()
    };
    assert_eq!(roll(9), roll(9));
}

#[test]
fn triggers_match_their_ability_and_their_kind_of_proc() {
    let frostbolt = data::ability("frostbolt");
    let fireball = data::ability("fireball");
    let hit = ProcKind::Hit {
        crit: false,
        killed: false,
    };
    let crit = ProcKind::Hit {
        crit: true,
        killed: false,
    };
    let kill = ProcKind::Hit {
        crit: false,
        killed: true,
    };
    let mine = Of::Ability("frostbolt");
    let on_hit = Trigger::OnHit {
        of: mine,
        chance: 0.4,
    };
    assert_eq!(fires_on(on_hit, hit, frostbolt), Some(0.4));
    assert_eq!(fires_on(on_hit, crit, frostbolt), Some(0.4));
    assert_eq!(fires_on(on_hit, hit, fireball), None);
    assert_eq!(fires_on(on_hit, ProcKind::Cast, frostbolt), None);
    let on_crit = Trigger::OnCrit {
        of: Of::Any,
        chance: 1.0,
    };
    assert_eq!(fires_on(on_crit, hit, fireball), None);
    assert_eq!(fires_on(on_crit, crit, fireball), Some(1.0));
    let on_kill = Trigger::OnKill {
        of: mine,
        chance: 1.0,
    };
    assert_eq!(fires_on(on_kill, crit, frostbolt), None);
    assert_eq!(fires_on(on_kill, kill, frostbolt), Some(1.0));
    let on_cast = Trigger::OnCast(mine);
    assert_eq!(fires_on(on_cast, ProcKind::Cast, frostbolt), Some(1.0));
    assert_eq!(fires_on(on_cast, hit, frostbolt), None);
    assert_eq!(fires_on(Trigger::OnLethal, ProcKind::Cast, frostbolt), None);
}

#[test]
fn a_nova_reaches_everything_in_its_radius() {
    let mut foes = vec![foe(1, 1.0, 0.0), foe(2, 0.0, 3.9), foe(3, 5.0, 0.0)];
    assert_eq!(ids(&foes, &target::around(&foes, Vec3::ZERO, 4.0)), [1, 2]);
    // Half a body inside counts.
    foes[2].radius = 2.4;
    assert_eq!(
        ids(&foes, &target::around(&foes, Vec3::ZERO, 4.0)),
        [1, 2, 3]
    );
}

#[test]
fn a_chain_jumps_to_the_nearest_fresh_enemy_within_reach() {
    let foes = [
        foe(1, 0.0, 0.0),
        foe(2, 4.0, 0.0),
        foe(3, 9.0, 0.0),
        foe(4, 1.0, 1.0),
        foe(5, 40.0, 0.0),
    ];
    let from = Some(foes[0].e);
    // From 1: 4 is nearest, then 2, then 3; 5 is out of reach of everyone.
    let path = target::chain(&foes, from, foes[0].pos, 6, 6.0);
    assert_eq!(ids(&foes, &path), [4, 2, 3]);
    assert_eq!(
        ids(&foes, &target::chain(&foes, from, foes[0].pos, 2, 6.0)),
        [4, 2]
    );
}

#[test]
fn a_split_fans_to_the_nearest_others_within_its_spread() {
    let foes = [
        foe(1, 0.0, 0.0),
        foe(2, 2.0, 0.0),
        foe(3, -3.0, 0.0),
        foe(4, 0.0, 6.0),
        foe(5, 0.0, 20.0),
    ];
    let shards = target::nearest_n(&foes, Vec3::ZERO, 8.0, 3, Some(foes[0].e));
    assert_eq!(ids(&foes, &shards), [2, 3, 4]);
    let two = target::nearest_n(&foes, Vec3::ZERO, 8.0, 2, Some(foes[0].e));
    assert_eq!(ids(&foes, &two), [2, 3]);
}

#[test]
fn a_pierce_strikes_only_the_line_behind_its_target() {
    // The hero at the origin shoots the enemy 5 yards along +x.
    let foes = [
        foe(1, 5.0, 0.0),
        foe(2, 9.0, 0.5),
        foe(3, 12.0, -1.0),
        foe(4, 3.0, 0.0),
        foe(5, 9.0, 4.0),
        foe(6, 30.0, 0.0),
    ];
    let line = target::pierce(
        &foes,
        Vec3::ZERO,
        foes[0].pos,
        Some(foes[0].e),
        5,
        1.6,
        14.0,
    );
    // Not the one in front (4), off the line (5) or past the reach (6).
    assert_eq!(ids(&foes, &line), [2, 3]);
    let one = target::pierce(
        &foes,
        Vec3::ZERO,
        foes[0].pos,
        Some(foes[0].e),
        1,
        1.6,
        14.0,
    );
    assert_eq!(ids(&foes, &one), [2]);
}

#[test]
fn a_boomerang_strikes_what_lies_between_its_target_and_the_hero() {
    let foes = [
        foe(1, 10.0, 0.0),
        foe(2, 6.0, 0.5),
        foe(3, 2.0, -0.5),
        foe(4, -3.0, 0.0),
        foe(5, 5.0, 5.0),
    ];
    let back = target::boomerang(&foes, foes[0].pos, Vec3::ZERO, Some(foes[0].e), 1.6);
    assert_eq!(ids(&foes, &back), [2, 3], "in the order it meets them");
}

#[test]
fn the_densest_pack_wins_the_aim() {
    let foes = [
        foe(1, 10.0, 0.0),
        foe(2, -10.0, 0.0),
        foe(3, -11.0, 1.0),
        foe(4, -9.0, -1.0),
        foe(5, 60.0, 0.0),
        foe(6, 61.0, 0.0),
        foe(7, 62.0, 0.0),
        foe(8, 60.0, 1.0),
    ];
    let at = target::densest(&foes, Vec3::ZERO, 20.0, 5.0).unwrap();
    assert!(
        at.x < -8.0,
        "the pack of three in range, not the four out of it: {at}"
    );
    assert_eq!(target::densest(&foes, Vec3::ZERO, 2.0, 5.0), None);
}

#[test]
fn execute_adds_the_blow_over_below_its_threshold_only() {
    assert!(close(execute_bonus(20.0, 100.0, 10.0, 0.3, 3.0), 20.0));
    assert!(close(execute_bonus(30.0, 100.0, 10.0, 0.3, 3.0), 20.0));
    assert_eq!(execute_bonus(31.0, 100.0, 10.0, 0.3, 3.0), 0.0);
    assert_eq!(
        execute_bonus(0.0, 100.0, 10.0, 0.3, 3.0),
        0.0,
        "already dead"
    );
    assert_eq!(execute_bonus(20.0, 100.0, 0.0, 0.3, 3.0), 0.0, "no blow");
}

#[test]
fn cheat_death_stands_the_hero_up_untouchable() {
    let mut hero = HeroState {
        hp: -12.0,
        ..HeroState::default()
    };
    let max = hero.stats().max_hp;
    cheat_death(&mut hero, 2.0, 0.3);
    assert!(close(hero.hp, max * 0.3));
    assert!(close(hero.immune_t, 2.0));
    // Never lower than it stands.
    hero.hp = max * 0.8;
    cheat_death(&mut hero, 1.0, 0.3);
    assert!(close(hero.hp, max * 0.8));
    assert!(close(hero.immune_t, 2.0));
}

#[test]
fn grant_replaces_by_id_and_revoke_takes_back_only_its_own() {
    let hit = Trigger::OnHit {
        of: Of::Any,
        chance: 1.0,
    };
    let periodic = Trigger::Periodic { every: 3.0 };
    let pierce = spec(hit, Effect::Pierce { n: 1 });
    let shield = spec(
        periodic,
        Effect::Shield {
            amount: 5.0,
            duration: 3.0,
        },
    );
    let mut hero = HeroState::default();
    assert_eq!(hero.mechanics.specs().count(), 0);
    assert!(!hero.mechanics.listens());
    grant(&mut hero, MechanicId::Talent(16757), [pierce, shield]);
    grant(&mut hero, MechanicId::Jewelry(3), [shield]);
    assert_eq!(hero.mechanics.specs().count(), 3);
    assert!(hero.mechanics.listens(), "the pierce waits on hits");
    // Granting an id again replaces what it held.
    grant(&mut hero, MechanicId::Talent(16757), [shield]);
    assert_eq!(hero.mechanics.specs().count(), 2);
    assert!(!hero.mechanics.listens());
    assert!(has(&hero, MechanicId::Talent(16757)));
    assert_eq!(revoke(&mut hero, MechanicId::Talent(16757)), 1);
    assert!(!has(&hero, MechanicId::Talent(16757)));
    assert_eq!(revoke(&mut hero, MechanicId::Talent(16757)), 0);
    // A whole source at once.
    grant(&mut hero, MechanicId::Jewelry(4), [pierce, pierce]);
    let gone = hero
        .mechanics
        .revoke_where(|id| matches!(id, MechanicId::Jewelry(_)));
    assert_eq!(gone, 3);
    assert_eq!(hero.mechanics.specs().count(), 0);
}

#[test]
fn the_top_rank_grants_its_legendary_once_for_the_run() {
    let mut hero = hero_with("frostbolt", data::MAX_ABILITY_LEVEL - 2);
    hud::take(&mut hero, Choice::Upgrade(0));
    let id = MechanicId::Legendary("frostbolt");
    assert!(
        has(&hero, id),
        "a multi-rank card transforms at the top rank"
    );
    let legend = data::legendary("frostbolt").unwrap();
    assert_eq!(hero.mechanics.specs().count(), legend.mechanics.len());
    assert!(hero
        .mechanics
        .specs()
        .all(|(i, s)| i == id && legend.mechanics.contains(s)));
    assert_eq!(hero.mechanics.flourish, [legend.visual_spell]);
    assert_eq!(sync_legendaries(&mut hero), 0, "once");
    // Reached any other way, the top rank still transforms.
    let mut other = hero_with("fireball", data::MAX_ABILITY_LEVEL);
    assert_eq!(sync_legendaries(&mut other), 1);
    assert!(has(&other, MechanicId::Legendary("fireball")));
    // A run starts with none.
    assert_eq!(HeroState::default().mechanics.specs().count(), 0);
}

#[test]
fn echoes_come_due_one_gap_apart() {
    let mut m = Mechanics::default();
    m.echo("whirlwind", 1.0, 2);
    assert!(!m.take_echo("whirlwind", 0.9));
    assert!(!m.take_echo("cleave", 1.0));
    assert!(m.take_echo("whirlwind", 1.0));
    assert!(!m.take_echo("whirlwind", 1.0 + ECHO_GAP * 0.5));
    assert!(m.take_echo("whirlwind", 1.0 + ECHO_GAP));
    assert!(!m.take_echo("whirlwind", 10.0), "spent");
}

#[test]
fn the_legendary_card_is_flagged_and_says_into_what() {
    let mut hero = hero_with("frostbolt", data::MAX_ABILITY_LEVEL - 1);
    let card = hud::describe_card(Choice::Upgrade(0), &hero, None);
    let legend = card.legendary.expect("the top rank's card transforms");
    assert_eq!((legend.ability, legend.name), ("frostbolt", "Shatterbolt"));
    assert!(!legend.text.is_empty());
    assert_eq!(cards::rarity(Choice::Upgrade(0), &hero), Rarity::Legendary);
    hero.abilities[0].level = 3;
    assert!(hud::describe_card(Choice::Upgrade(0), &hero, None)
        .legendary
        .is_none());
    assert_ne!(cards::rarity(Choice::Upgrade(0), &hero), Rarity::Legendary);
    let fresh = hud::describe_card(Choice::NewAbility(data::ability("fireball")), &hero, None);
    assert!(fresh.legendary.is_none());
}

/// A world the mechanics' tick runs in, a second passing, with no horde.
fn tick_world() -> World {
    let mut world = World::new();
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs(1));
    world.insert_resource(time);
    world.init_resource::<Run>();
    world.insert_resource(Rng(11));
    world.init_resource::<Procs>();
    world.init_resource::<PendingHits>();
    world.init_resource::<Messages<Hit>>();
    world.init_resource::<Messages<crate::entities::dest_fx::GroundBurst>>();
    world.init_resource::<crate::creature_anim::PlaySeq>();
    world.init_resource::<Messages<crate::creature_anim::CastEvent>>();
    world.init_resource::<Messages<crate::creature_anim::SpellGoTargets>>();
    world.init_resource::<Messages<crate::creature_anim::SwingMessage>>();
    world.init_resource::<Messages<crate::combat_text::CombatTextSpawn>>();
    world
}

fn tick(world: &mut World) {
    world.run_system_once(run::tick).unwrap();
}

#[test]
fn cheat_death_turns_a_lethal_blow_away_once_a_cooldown() {
    let mut world = tick_world();
    let mut saved = HeroState {
        hp: -5.0,
        ..HeroState::default()
    };
    let max = saved.stats().max_hp;
    grant(
        &mut saved,
        MechanicId::Other("test"),
        [spec(
            Trigger::OnLethal,
            Effect::CheatDeath {
                immune: 2.0,
                heal_frac: 0.3,
            },
        )
        .cooldown(90.0)],
    );
    let saved = world.spawn((Transform::default(), saved)).id();
    let doomed = world
        .spawn((
            Transform::default(),
            HeroState {
                player: 1,
                hp: -5.0,
                ..HeroState::default()
            },
        ))
        .id();
    tick(&mut world);
    let hero = |world: &World, e| world.get::<HeroState>(e).unwrap().hp;
    assert!(close(hero(&world, saved), max * 0.3));
    assert!(close(world.get::<HeroState>(saved).unwrap().immune_t, 2.0));
    assert_eq!(hero(&world, doomed), -5.0, "no mechanic, no reprieve");
    // Struck down again within the cooldown, it falls.
    world.get_mut::<HeroState>(saved).unwrap().hp = -1.0;
    tick(&mut world);
    assert_eq!(hero(&world, saved), -1.0);
}

#[test]
fn heal_pulse_heals_the_party_in_reach_through_party_heal() {
    let mut world = tick_world();
    let max = HeroState::default().stats().max_hp;
    let mut owner = HeroState {
        hp: max - 50.0,
        ..HeroState::default()
    };
    grant(
        &mut owner,
        MechanicId::Other("test"),
        [spec(
            Trigger::Periodic { every: 0.5 },
            Effect::HealPulse {
                radius: 12.0,
                amount: 10.0,
            },
        )
        .at(Aim::Hero)],
    );
    let owner = world.spawn((Transform::default(), owner)).id();
    let near = world
        .spawn((
            Transform::from_xyz(6.0, 0.0, 0.0),
            HeroState {
                player: 1,
                hp: max - 3.0,
                ..HeroState::default()
            },
        ))
        .id();
    let far = world
        .spawn((
            Transform::from_xyz(30.0, 0.0, 0.0),
            HeroState {
                player: 2,
                hp: max - 30.0,
                ..HeroState::default()
            },
        ))
        .id();
    tick(&mut world);
    let hero = |e| world.get::<HeroState>(e).unwrap();
    assert!(close(hero(owner).hp, max - 40.0));
    assert!(
        close(hero(near).hp, max),
        "capped at max health, as party::heal is"
    );
    assert!(close(hero(far).hp, max - 30.0), "out of reach");
    // The healer is credited with what landed: its own 10 and the ally's 3.
    assert!((hero(owner).healing_done - 13.0).abs() < 1e-4);
    assert_eq!(hero(near).healing_done, 0.0);
}

#[test]
fn a_burst_lifts_the_hero_then_runs_out() {
    let mut world = tick_world();
    let mut hero = HeroState::default();
    static BONUSES: &[Bonus] = &[Bonus::new(StatKey::Damage, 0.5)];
    grant(
        &mut hero,
        MechanicId::Other("test"),
        [spec(
            Trigger::Periodic { every: 1.0 },
            Effect::Burst {
                duration: 2.0,
                bonuses: BONUSES,
            },
        )
        .cooldown(100.0)],
    );
    let hero = world.spawn((Transform::default(), hero)).id();
    let might = |world: &World| world.get::<HeroState>(hero).unwrap().stats().might;
    tick(&mut world);
    assert!(close(might(&world), 1.5));
    tick(&mut world);
    assert!(close(might(&world), 1.5), "a second left");
    tick(&mut world);
    assert!(close(might(&world), 1.0), "run out, and the bonus with it");
}

#[test]
fn flat_numbers_grow_with_the_run_level() {
    assert_eq!(level_scale(1), 1.0);
    assert_eq!(level_scale(0), 1.0);
    assert!(close(level_scale(11), 1.0 + 10.0 * LEVEL_GROWTH));
    assert!(level_scale(40) > level_scale(20));
}

/// Against the real `Spell.dbc` and `SpellVisual.dbc`; skips without client data. Every visual a
/// legendary names is a real spell with the kit it plays where it plays it.
#[test]
fn legendary_visuals_are_real_spells_with_the_kits_they_play() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = benilla_formats::open_chain(&data).expect("open chain");
    let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
    let visuals = benilla_formats::load_spell_visual_catalog(&mut chain).expect("SpellVisual.dbc");
    let mut bad = Vec::new();
    let stages = |spell: u32| {
        spells
            .get(spell)
            .filter(|d| d.visual != 0)
            .and_then(|d| visuals.stages(d.visual).map(|s| (d.visual, *s)))
    };
    for l in LEGENDARIES {
        if stages(l.visual_spell).is_none() {
            bad.push(format!(
                "{}: visual_spell {} has no visual",
                l.ability, l.visual_spell
            ));
        }
        for m in l.mechanics {
            let Some(spell) = m.visual else { continue };
            let Some((visual, s)) = stages(spell) else {
                bad.push(format!("{}: {spell} has no visual", l.ability));
                continue;
            };
            let on_hero = matches!(m.at, Aim::Hero | Aim::Follow)
                || (m.at == Aim::Trigger
                    && !matches!(
                        m.trigger,
                        Trigger::OnHit { .. } | Trigger::OnCrit { .. } | Trigger::OnKill { .. }
                    ));
            let ok = match m.effect {
                Effect::BurningGround { .. } => s.area_gate != 0 && s.area_effect != 0,
                Effect::Nova { .. } | Effect::HealPulse { .. } if on_hero => {
                    s.cast != 0 || s.impact != 0
                }
                Effect::Nova { .. } | Effect::HealPulse { .. } => {
                    run::point_model(visual, &visuals).is_some()
                }
                Effect::Burst { .. }
                | Effect::Shield { .. }
                | Effect::CheatDeath { .. }
                | Effect::ResetCooldowns
                | Effect::Echo { .. } => s.cast != 0 || s.impact != 0,
                _ => s.impact != 0,
            };
            if !ok {
                bad.push(format!(
                    "{}: {spell} cannot play for {:?}",
                    l.ability, m.effect
                ));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// A sturdy enemy standing still at `x`, with no afflictions but what `edit` puts on it.
fn enemy_at(
    world: &mut World,
    x: f32,
    edit: impl FnOnce(&mut super::super::enemies::Enemy),
) -> Entity {
    let mut en = super::super::enemies::Enemy {
        def: &data::roster(0).enemies[0],
        hp: 1000.0,
        max_hp: 1000.0,
        speed: 0.0,
        damage: 0.0,
        radius: 0.6,
        xp: 1,
        boss: false,
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
    edit(&mut en);
    world.spawn((en, Transform::from_xyz(x, 0.0, 0.0))).id()
}

fn burn(source: Entity, dps: f32, left: f32) -> super::super::enemies::Dot {
    super::super::enemies::Dot {
        dps,
        left,
        tick: 1.0,
        source,
        school: Some(School::Fire),
    }
}

/// Every hit the frame's mechanics wrote.
fn written(world: &mut World) -> Vec<Hit> {
    world.resource_mut::<Messages<Hit>>().drain().collect()
}

/// A crit of `damage` from `owner`'s Fireball on `target`, for the mechanics to match.
fn crit_on(world: &mut World, owner: Entity, target: Entity, damage: f32) {
    world.resource_mut::<Procs>().0.push(Proc {
        owner,
        ability: data::ability("fireball"),
        kind: ProcKind::Hit {
            crit: true,
            killed: false,
        },
        target: Some(target),
        pos: Vec3::ZERO,
        damage,
        effect: data::Effect::None,
    });
}

#[test]
fn next_cast_guarantees_arm_merge_ready_the_slowest_and_are_spent_once() {
    let mut hero = hero_with("fireball", 1);
    hero.abilities.push(Owned {
        def: data::ability("frostbolt"),
        level: 1,
        cd: 3.0,
    });
    hero.abilities[0].cd = 1.0;
    // Presence of Mind: the slowest ability ready now, the next cast a crit.
    let pom = Armed {
        crit: true,
        cooldown: 1.0,
        aura: 12043,
    };
    run::arm(&mut hero, true, pom);
    assert_eq!(
        hero.abilities[1].cd, 0.0,
        "frostbolt, furthest off, is ready"
    );
    assert_eq!(hero.abilities[0].cd, 1.0);
    // Clearcasting on top keeps the crit and halves the cooldown.
    let clearcasting = Armed {
        crit: false,
        cooldown: 0.5,
        aura: 12536,
    };
    run::arm(&mut hero, false, clearcasting);
    assert_eq!(
        hero.mechanics.armed(),
        Some(Armed {
            crit: true,
            cooldown: 0.5,
            aura: 12536
        })
    );
    assert!(hero.mechanics.spend_armed().is_some());
    assert_eq!(hero.mechanics.armed(), None, "one cast spends it");

    // Through the tick: a periodic guarantee arms on its period.
    let mut world = tick_world();
    let mut armed = hero_with("fireball", 1);
    grant(
        &mut armed,
        MechanicId::Other("test"),
        [spec(
            Trigger::Periodic { every: 0.5 },
            Effect::NextCast {
                instant: true,
                crit: true,
                cooldown: 1.0,
                aura: 12043,
            },
        )],
    );
    let e = world.spawn((Transform::default(), armed)).id();
    tick(&mut world);
    assert_eq!(
        world.get::<HeroState>(e).unwrap().mechanics.armed(),
        Some(pom)
    );
}

#[test]
fn ignite_feeds_on_the_fire_burns_on_its_target() {
    let mut world = tick_world();
    let mut mage = HeroState::default();
    grant(
        &mut mage,
        MechanicId::Other("test"),
        [spec(
            Trigger::OnCrit {
                of: Of::Any,
                chance: 1.0,
            },
            Effect::Ignite {
                dps: 8.0,
                secs: 4.0,
                per_burn: 0.15,
            },
        )],
    );
    let mage = world.spawn((Transform::default(), mage)).id();
    let other = Entity::from_raw_u32(999).unwrap();
    let burning = enemy_at(&mut world, 3.0, |en| {
        en.dots = vec![burn(mage, 1.0, 3.0), burn(other, 1.0, 3.0)];
    });
    let bare = enemy_at(&mut world, -3.0, |_| {});
    crit_on(&mut world, mage, burning, 30.0);
    crit_on(&mut world, mage, bare, 30.0);
    tick(&mut world);
    let hits = written(&mut world);
    let dot_on = |e: Entity| hits.iter().find(|h| h.target == e).map(|h| h.dot);
    assert_eq!(hits.len(), 2);
    assert!(close(dot_on(bare).unwrap(), 8.0), "nothing to feed on");
    assert!(
        close(dot_on(burning).unwrap(), 8.0 * 1.3),
        "two burns feed it 15% each"
    );
    assert!(hits.iter().all(|h| h.school == Some(School::Fire)));
}

#[test]
fn shatter_strikes_frozen_targets_again_and_spares_the_rest() {
    let mut world = tick_world();
    let mut mage = HeroState::default();
    grant(
        &mut mage,
        MechanicId::Other("test"),
        [spec(
            Trigger::OnCrit {
                of: Of::Any,
                chance: 1.0,
            },
            Effect::Shatter { bonus: 0.5 },
        )],
    );
    let mage = world.spawn((Transform::default(), mage)).id();
    let rooted = enemy_at(&mut world, 2.0, |en| en.root_t = 2.0);
    let chilled = enemy_at(&mut world, 4.0, |en| {
        en.slow = 0.5;
        en.slow_t = 2.0;
    });
    let slowed = enemy_at(&mut world, 6.0, |en| {
        en.slow = 0.8;
        en.slow_t = 2.0;
    });
    for e in [rooted, chilled, slowed] {
        crit_on(&mut world, mage, e, 40.0);
    }
    tick(&mut world);
    let hits = written(&mut world);
    let on = |e: Entity| hits.iter().filter(|h| h.target == e).count();
    assert_eq!((on(rooted), on(chilled), on(slowed)), (1, 1, 0));
    assert!(hits.iter().all(|h| close(h.damage, 20.0) && h.crit));
}

#[test]
fn conflagrate_consumes_the_burn_for_a_burst_and_spreads_it() {
    let mut world = tick_world();
    let mut lock = HeroState::default();
    grant(
        &mut lock,
        MechanicId::Other("test"),
        [spec(
            Trigger::Periodic { every: 0.5 },
            Effect::ConsumeDot {
                school: School::Fire,
                burst: 1.25,
                spread: 1,
                radius: 8.0,
                secs: 5.0,
            },
        )],
    );
    let lock = world.spawn((Transform::default(), lock)).id();
    let burning = enemy_at(&mut world, 5.0, |en| en.dots = vec![burn(lock, 4.0, 5.0)]);
    let near = enemy_at(&mut world, 9.0, |_| {});
    let nearer = enemy_at(&mut world, 7.0, |_| {});
    let far = enemy_at(&mut world, 30.0, |_| {});
    tick(&mut world);
    assert!(
        world
            .get::<super::super::enemies::Enemy>(burning)
            .unwrap()
            .dots
            .is_empty(),
        "consumed"
    );
    let hits = written(&mut world);
    let burst = hits.iter().find(|h| h.target == burning).unwrap();
    assert!(close(burst.damage, 4.0 * 5.0 * 1.25));
    let spread: Vec<&Hit> = hits.iter().filter(|h| h.dot > 0.0).collect();
    assert_eq!(spread.len(), 1);
    assert_eq!(spread[0].target, nearer, "to the nearest first");
    assert!(close(spread[0].dot, 4.0) && close(spread[0].dot_dur, 5.0));
    assert!(hits.iter().all(|h| h.target != far && h.target != near));

    // Only the hero's own burn of the school is taken.
    let mut dots = vec![burn(lock, 2.0, 3.0), burn(near, 9.0, 3.0)];
    assert_eq!(run::consumed(&mut dots, lock, School::Fire), (6.0, 2.0));
    assert_eq!(dots.len(), 1, "the other hero's burn stays");
    assert_eq!(run::consumed(&mut dots, lock, School::Fire), (0.0, 0.0));
}

#[test]
fn a_shared_heal_turns_its_schools_damage_into_party_healing() {
    let mut world = tick_world();
    let max = HeroState::default().stats().max_hp;
    let mut priest = HeroState {
        hp: max - 50.0,
        ..HeroState::default()
    };
    grant(
        &mut priest,
        MechanicId::Other("test"),
        [spec(
            Trigger::Periodic { every: 0.5 },
            Effect::ShareHeal {
                school: School::Shadow,
                share: 0.5,
                radius: 40.0,
            },
        )],
    );
    priest.mechanics.pool(School::Shadow, 20.0);
    priest.mechanics.pool(School::Fire, 100.0);
    let priest = world.spawn((Transform::default(), priest)).id();
    let hurt = |player: u8| HeroState {
        player,
        hp: max - 50.0,
        ..HeroState::default()
    };
    let ally = world
        .spawn((Transform::from_xyz(20.0, 0.0, 0.0), hurt(1)))
        .id();
    let far = world
        .spawn((Transform::from_xyz(60.0, 0.0, 0.0), hurt(2)))
        .id();
    tick(&mut world);
    let hp = |w: &World, e| w.get::<HeroState>(e).unwrap().hp;
    assert!(
        close(hp(&world, priest), max - 40.0),
        "half the Shadow damage"
    );
    assert!(close(hp(&world, ally), max - 40.0));
    assert!(close(hp(&world, far), max - 50.0), "out of reach");
    let credited = world.get::<HeroState>(priest).unwrap().healing_done;
    assert!((credited - 20.0).abs() < 1e-4);
    // The pool is spent: the next period has nothing to share.
    tick(&mut world);
    assert!(close(hp(&world, ally), max - 40.0));
}

#[test]
fn a_healthstone_heals_the_allies_near_its_taker() {
    let mut world = tick_world();
    let max = HeroState::default().stats().max_hp;
    let mut taker = HeroState {
        hp: 10.0,
        ..HeroState::default()
    };
    hud::take(&mut taker, Choice::Heal);
    assert_eq!(taker.hp, max);
    let taker = world.spawn((Transform::default(), taker)).id();
    let hurt = |player: u8| HeroState {
        player,
        hp: 10.0,
        ..HeroState::default()
    };
    let near = world
        .spawn((Transform::from_xyz(8.0, 0.0, 0.0), hurt(1)))
        .id();
    let far = world
        .spawn((Transform::from_xyz(15.0, 0.0, 0.0), hurt(2)))
        .id();
    tick(&mut world);
    let hp = |w: &World, e| w.get::<HeroState>(e).unwrap().hp;
    let share = max * HEALTHSTONE_SHARE;
    assert!(close(hp(&world, near), 10.0 + share));
    assert!(close(hp(&world, far), 10.0), "past 10 yd");
    let credited = world.get::<HeroState>(taker).unwrap().healing_done;
    assert!((credited - f64::from(share)).abs() < 1e-3);
    // Once: the next tick passes nothing on.
    tick(&mut world);
    assert!(close(hp(&world, near), 10.0 + share));
}

#[test]
fn totem_reach_is_the_best_granted_and_shatter_sees_frost() {
    let mut hero = HeroState::default();
    assert_eq!(hero.mechanics.totem_reach(), 1.0);
    grant(
        &mut hero,
        MechanicId::Other("test"),
        [spec(
            Trigger::Periodic { every: 60.0 },
            Effect::TotemReach { mult: 1.5 },
        )],
    );
    assert_eq!(hero.mechanics.totem_reach(), 1.5);
    let mut world = World::new();
    let rooted = enemy_at(&mut world, 0.0, |en| en.root_t = 1.0);
    let thawed = enemy_at(&mut world, 0.0, |en| {
        en.slow = 0.4;
        en.slow_t = 0.0;
    });
    let en = |e| world.get::<super::super::enemies::Enemy>(e).unwrap();
    assert!(run::frozen(en(rooted)));
    assert!(!run::frozen(en(thawed)), "a chill run out freezes nothing");
}
