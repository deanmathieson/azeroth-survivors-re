//! Gear in a run: a slain roster boss leaves a chest ([`treasure::drop_boss_chests`]), and its
//! [`BossDrop`] rolls as the chest opens; new pieces go straight into the class's collection (worn
//! if they beat what the slot wears), and each is announced over the chest and on the run's banner
//! in its rarity's colour, and on the results screen. A piece already collected pays
//! [`DUPLICATE_COINS`] coins of gold instead.

use std::collections::VecDeque;

use benilla_protocol::messages::ObjectFields;
use bevy::prelude::*;

use super::super::data::CLASSES;
use super::super::hud::{BannerText, NoteKind, ResultsNotes, GOLD_TONE};
use super::super::party::{NetRole, Remote};
use super::super::treasure::BossRolls;
use super::super::{treasure, HeroState, LocalHero, Phase, Rng, Run, Selection};
use super::save::Collected;
use super::store::{GearRead, GearStore};
use super::{drops_for, item, piece, BossDrop, Slot};
use crate::combat_text::CombatTextSpawn;
use crate::net::ObjectStore;

/// A duplicate's worth: this many coins at what a coin is worth when it drops, under the gold
/// multiplier, as Skip pays ([`treasure::worth`]).
pub(super) const DUPLICATE_COINS: u32 = 10;
/// Seconds each loot line holds the banner.
const TOAST_SECS: f32 = 3.0;
/// `UNIT_VIRTUAL_ITEM_SLOT_DISPLAY`, the creature weapon field [`super::super::units`] writes.
const FIELD_VIRTUAL_ITEM_DISPLAY: u16 = 37;

/// The odds each slot of a drop falls: the boss's `chance`, scaled by the Run Options rewards
/// multiplier and the hero's luck multiplier (1 + luck bonuses), at most certain.
pub(super) fn drop_odds(chance: f32, reward_mult: f32, luck: f32) -> f32 {
    (chance * reward_mult.max(0.0) * luck.max(0.0)).clamp(0.0, 1.0)
}

/// The slots of `drop` that fall on one kill, each rolled on its own at [`drop_odds`].
pub(super) fn roll(drop: &BossDrop, reward_mult: f32, luck: f32, rng: &mut Rng) -> Vec<Slot> {
    let odds = drop_odds(drop.chance, reward_mult, luck);
    drop.slots
        .iter()
        .copied()
        .filter(|_| rng.f32() < odds)
        .collect()
}

/// Which of battleground `map`'s roster bosses is called `name`.
#[cfg(test)]
pub(super) fn boss_index(map: usize, name: &str) -> Option<usize> {
    super::super::data::roster(map)
        .bosses
        .iter()
        .position(|b| b.name == name)
}

/// The local hero's class, as the profile records it: the pick, else the hero's class by name.
fn hero_class(selection: &Selection, hero: &HeroState) -> usize {
    selection.class.unwrap_or_else(|| {
        CLASSES
            .iter()
            .position(|c| c.name == hero.class.name)
            .unwrap_or(0)
    })
}

/// One boss piece a co-op host put up for the party's roll: Need, Greed or Pass (the co-op
/// social layer runs the roll and tells the winner's machine to keep it, [`WonLoot`]).
#[cfg_attr(not(feature = "coop"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct GroupDrop {
    /// The piece, by [`super::Item::id`]: one class's, picked among the party's.
    pub item: u16,
    /// The chest it came out of.
    pub chest: Entity,
}

/// Boss pieces waiting for a group roll; filled only on a co-op host with other players in the run.
#[derive(Resource, Default, Debug)]
pub(in crate::survivors) struct GroupDrops(pub Vec<GroupDrop>);

/// Pieces this player won at a group roll, waiting to go into this machine's own profile.
#[derive(Resource, Default, Debug)]
pub(in crate::survivors) struct WonLoot(pub Vec<u16>);

/// A hero's class, as an index into [`CLASSES`].
fn class_of(hero: &HeroState) -> usize {
    CLASSES
        .iter()
        .position(|c| c.id == hero.class.id)
        .unwrap_or(0)
}

/// Loot lines waiting for the banner, and the one on it.
#[derive(Resource, Default)]
pub(super) struct LootToasts {
    queue: VecDeque<(String, Color)>,
    shown: Option<(String, Color)>,
}

/// `color` as combat text's 0xAARRGGBB.
fn argb(color: Color) -> u32 {
    let c = color.to_srgba();
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    byte(c.alpha) << 24 | byte(c.red) << 16 | byte(c.green) << 8 | byte(c.blue)
}

/// An opened boss chest's drop, for each hero it rolls for under that hero's luck: each new piece
/// is collected (and maybe worn) at once, shown over the chest, announced and noted for the
/// results; each duplicate pays gold into the party's purse. A chest opened in the run rolls for
/// the heroes standing, and one the run's end opened for every hero.
///
/// In co-op, a host with other players in the run puts every piece that falls up for the party's
/// roll instead ([`GroupDrops`]): each is one class's, picked at random among the heroes', and the
/// winner's own machine keeps it. Solo, only the local player's hero rolls and keeps.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn roll_loot(
    mut rolls: ResMut<BossRolls>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    selection: Res<Selection>,
    local: Query<&HeroState, With<LocalHero>>,
    mut store: GearStore,
    mut notes: ResMut<ResultsNotes>,
    mut toasts: ResMut<LootToasts>,
    mut shown: MessageWriter<CombatTextSpawn>,
    party: (
        Option<Res<NetRole>>,
        Query<&HeroState>,
        Query<(), With<Remote>>,
        Option<ResMut<GroupDrops>>,
    ),
) {
    if rolls.0.is_empty() {
        return;
    }
    let (role, heroes, remotes, mut group) = party;
    let co_op = role.is_some_and(|r| *r == NetRole::Host) && !remotes.is_empty();
    for open in std::mem::take(&mut rolls.0) {
        let Some(drop) = drops_for(open.map, open.boss) else {
            continue;
        };
        let Ok(me) = local.single() else {
            continue;
        };
        if let (true, Some(group)) = (co_op, group.as_deref_mut()) {
            let classes: Vec<usize> = heroes.iter().map(class_of).collect();
            for slot in roll(drop, run.reward_mult, me.stats().luck, &mut rng) {
                let pick = rng.index(classes.len().max(1));
                let class = classes.get(pick).copied().unwrap_or(0);
                if let Some(p) = piece(class, slot, drop.rarity) {
                    group.0.push(GroupDrop {
                        item: p.id,
                        chest: open.chest,
                    });
                }
            }
            continue;
        }
        if open.living_only && !me.alive() {
            continue;
        }
        let class = hero_class(&selection, me);
        let stats = me.stats();
        for slot in roll(drop, run.reward_mult, stats.luck, &mut rng) {
            let Some(p) = piece(class, slot, drop.rarity) else {
                continue;
            };
            let got = if store.get(class).has(p.id) {
                Collected::Duplicate
            } else {
                store
                    .edit(class, |g| g.collect(p))
                    .unwrap_or(Collected::Duplicate)
            };
            match got {
                Collected::New { equipped } => {
                    let line = format!("Loot: {}", p.name);
                    notes.push(
                        if equipped {
                            format!("{line} (equipped)")
                        } else {
                            line.clone()
                        },
                        NoteKind::Loot,
                    );
                    shown.write(CombatTextSpawn {
                        anchor: open.chest,
                        text: line.clone(),
                        category: 0,
                        color: Some(argb(p.rarity.color())),
                    });
                    toasts.queue.push_back((line, p.rarity.color()));
                }
                Collected::Duplicate => {
                    let gold = treasure::worth(DUPLICATE_COINS, run.elapsed, stats.gold, 0.5);
                    run.gold += gold;
                    toasts
                        .queue
                        .push_back((format!("{} again: +{gold} gold", p.name), GOLD_TONE));
                }
            }
        }
    }
}

/// Pieces this player won at a co-op roll go into its own profile, under the piece's class (a Greed
/// win of another class's piece fills that class's collection): noted for the results and
/// announced; one already collected is only announced.
pub(super) fn grant_won(
    mut won: ResMut<WonLoot>,
    mut store: GearStore,
    mut notes: ResMut<ResultsNotes>,
    mut toasts: ResMut<LootToasts>,
) {
    for id in std::mem::take(&mut won.0) {
        let Some(p) = item(id) else { continue };
        let got = if store.get(p.class).has(p.id) {
            Collected::Duplicate
        } else {
            store
                .edit(p.class, |g| g.collect(p))
                .unwrap_or(Collected::Duplicate)
        };
        match got {
            Collected::New { equipped } => {
                let line = format!("Won: {}", p.name);
                notes.push(
                    if equipped {
                        format!("Loot: {} (equipped)", p.name)
                    } else {
                        format!("Loot: {}", p.name)
                    },
                    NoteKind::Loot,
                );
                toasts.queue.push_back((line, p.rarity.color()));
            }
            Collected::Duplicate => toasts.queue.push_back((
                format!("{} again: already in your collection", p.name),
                GOLD_TONE,
            )),
        }
    }
}

/// Loot lines take the run's banner in turn, each once it is free, in the piece's rarity colour;
/// any other announcement keeps the banner's gold.
pub(super) fn show_toasts(
    phase: Res<State<Phase>>,
    mut run: ResMut<Run>,
    mut toasts: ResMut<LootToasts>,
    mut banner: Query<(&Text, &mut TextColor), With<BannerText>>,
) {
    let on_banner = |run: &Run, line: &str| run.banner.as_ref().is_some_and(|(b, _)| b == line);
    // Expired, or another announcement took the banner.
    if toasts
        .shown
        .as_ref()
        .is_some_and(|(line, _)| !on_banner(&run, line))
    {
        toasts.shown = None;
    }
    if toasts.shown.is_none() && run.banner.is_none() && *phase.get() == Phase::Playing {
        if let Some((line, color)) = toasts.queue.pop_front() {
            run.banner = Some((line.clone(), TOAST_SECS));
            toasts.shown = Some((line, color));
        }
    }
    for (text, mut tint) in &mut banner {
        let want = match &toasts.shown {
            Some((line, color)) if **text == *line => *color,
            _ => GOLD_TONE,
        };
        if tint.0 != want {
            tint.0 = want;
        }
    }
}

/// Back at the menu, lines the last run never showed are dropped.
pub(super) fn clear_toasts(mut toasts: ResMut<LootToasts>) {
    toasts.queue.clear();
    toasts.shown = None;
}

/// The local hero holds its class's worn main-hand piece, else the class's own weapon: the
/// creature weapon slot the attach already dresses from (`UNIT_VIRTUAL_ITEM_SLOT_DISPLAY`). The
/// armour slots are worn on the body itself ([`super::outfit`]). The gear is the local player's.
pub(super) fn dress_hero(
    selection: Res<Selection>,
    gear: GearRead,
    mut heroes: Query<&mut ObjectStore, With<LocalHero>>,
) {
    let Some(class) = selection.class else { return };
    let Some(&(slot, default, ..)) = CLASSES
        .get(class)
        .and_then(|c| c.weapons.iter().find(|w| w.0 == 0))
    else {
        return;
    };
    let want = gear
        .get(class)
        .worn(Slot::MainHand)
        .map_or(default, |i| i.display_id);
    for mut store in &mut heroes {
        if store.0.unit_virtual_item_display(0) != Some(want) {
            store.0.merge(ObjectFields::from_pairs(&[(
                FIELD_VIRTUAL_ITEM_DISPLAY + slot,
                want,
            )]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::data;
    use super::super::super::profile::Profile;
    use super::super::{class_gear, Rarity};
    use super::*;

    const EASY: BossDrop = BossDrop {
        boss: "Test",
        slots: &[Slot::Head, Slot::Chest, Slot::Legs],
        rarity: Rarity::Common,
        chance: 1.0,
    };
    const NEVER: BossDrop = BossDrop {
        chance: 0.0,
        ..EASY
    };
    const HALF: BossDrop = BossDrop {
        chance: 0.25,
        ..EASY
    };

    #[test]
    fn certain_and_impossible_drops() {
        let mut rng = Rng(0x1234_5678);
        for _ in 0..200 {
            assert_eq!(roll(&EASY, 1.0, 1.0, &mut rng).len(), 3);
            assert!(roll(&NEVER, 1.0, 1.0, &mut rng).is_empty());
            // Nothing scales zero up.
            assert!(roll(&NEVER, 3.0, 2.0, &mut rng).is_empty());
        }
    }

    #[test]
    fn rewards_and_luck_raise_the_odds_up_to_certain() {
        assert_eq!(drop_odds(0.4, 1.0, 1.0), 0.4);
        assert!((drop_odds(0.4, 1.5, 1.0) - 0.6).abs() < 1e-6);
        assert!((drop_odds(0.4, 1.0, 1.25) - 0.5).abs() < 1e-6);
        assert_eq!(drop_odds(0.6, 2.0, 1.5), 1.0, "capped");
        assert_eq!(drop_odds(0.6, -1.0, 1.0), 0.0);
        // Seeded, the better odds drop more.
        let count = |mult: f32, luck: f32| {
            let mut rng = Rng(42);
            (0..2000)
                .map(|_| roll(&HALF, mult, luck, &mut rng).len())
                .sum::<usize>()
        };
        let base = count(1.0, 1.0);
        assert!((1300..1700).contains(&base), "{base} of 6000 at 25%");
        assert!(count(2.0, 1.0) > base + 1000);
        assert!(count(1.0, 1.5) > base + 500);
        assert_eq!(count(4.0, 1.0), 6000, "capped at certain");
    }

    #[test]
    fn every_drop_row_names_a_roster_boss_once() {
        for (m, map) in data::MAPS.iter().enumerate() {
            let bosses = &data::roster(m).bosses;
            for (b, boss) in bosses.iter().enumerate() {
                assert_eq!(
                    boss_index(m, boss.name),
                    Some(b),
                    "{} {}",
                    map.name,
                    boss.name
                );
            }
        }
        assert_eq!(boss_index(0, "Nobody"), None);
    }

    /// A boss chest through the systems: new pieces are collected, worn, noted and announced in
    /// their colour over the chest; a repeat pays gold.
    #[test]
    fn a_boss_chest_collects_and_announces() {
        use super::super::super::treasure::BossChestOpened;
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<BossRolls>();
        world.init_resource::<Messages<CombatTextSpawn>>();
        world.init_resource::<ResultsNotes>();
        world.init_resource::<LootToasts>();
        world.insert_resource(Profile::default());
        // Elwynn Forest's last boss, a certain drop under a high rewards multiplier.
        let map = data::MAPS
            .iter()
            .position(|m| m.name == "Elwynn Forest")
            .unwrap();
        let roster = data::roster(map);
        let last = roster.bosses.len() - 1;
        let drop = drops_for(map, last).unwrap();
        world.insert_resource(Selection {
            class: Some(2),
            map,
            ..default()
        });
        world.insert_resource(Run {
            reward_mult: 10.0,
            elapsed: 300.0,
            ..default()
        });
        world.insert_resource(Rng(7));
        world.spawn((HeroState::default(), LocalHero));
        let chest = world.spawn_empty().id();
        let kill = |world: &mut World| {
            world.resource_mut::<BossRolls>().0.push(BossChestOpened {
                map,
                boss: last,
                chest,
                living_only: true,
            });
            world.run_system_once(roll_loot).unwrap();
            assert!(world.resource::<BossRolls>().0.is_empty(), "rolled once");
        };
        kill(&mut world);
        let gear = class_gear(&world, 2);
        assert_eq!(gear.collected.len(), drop.slots.len());
        for &slot in drop.slots {
            let p = piece(2, slot, drop.rarity).unwrap();
            assert!(gear.has(p.id) && gear.wears(p.id), "{}", p.name);
        }
        let notes = &world.resource::<ResultsNotes>().0;
        assert_eq!(notes.len(), drop.slots.len());
        assert!(notes
            .iter()
            .all(|(s, k)| s.starts_with("Loot: ") && *k == NoteKind::Loot));
        let toasts = world.resource::<LootToasts>();
        assert_eq!(toasts.queue.len(), drop.slots.len());
        assert!(toasts.queue.iter().all(|(_, c)| *c == drop.rarity.color()));
        assert_eq!(world.resource::<Run>().gold, 0);
        // Each find floats over the chest in its colour.
        let over: Vec<_> = world
            .resource::<Messages<CombatTextSpawn>>()
            .iter_current_update_messages()
            .map(|t| (t.anchor, t.text.starts_with("Loot: "), t.color))
            .collect();
        assert_eq!(over.len(), drop.slots.len());
        assert!(over
            .iter()
            .all(|&(a, l, c)| a == chest && l && c == Some(argb(drop.rarity.color()))));
        // Nothing queued, nothing rolled.
        world.run_system_once(roll_loot).unwrap();
        assert_eq!(world.resource::<ResultsNotes>().0.len(), drop.slots.len());

        // The same kill again: duplicates, paid in gold, nothing new noted.
        kill(&mut world);
        let each = treasure::worth(DUPLICATE_COINS, 300.0, 1.0, 0.5);
        assert_eq!(world.resource::<Run>().gold, each * drop.slots.len() as u32);
        assert_eq!(world.resource::<ResultsNotes>().0.len(), drop.slots.len());
        assert_eq!(
            class_gear(&world, 2).collected.len(),
            drop.slots.len(),
            "nothing new"
        );
    }
}
