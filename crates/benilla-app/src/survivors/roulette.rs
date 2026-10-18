//! A boss chest's roulette, as a survivors game spins one: the chest rolls 1, 3 or 5 prizes (the
//! more, the rarer; luck tilts it, [`spin_count`]), and every hero standing wins that many, each a
//! Rare or better rank-up of one of its spells (one at rank 6 or 7 transforms outright into its
//! legendary), else of a talent, else an Overcharge ([`prize`]). The host decides them all
//! ([`super::treasure::open_caches`]); each player's screen shows its own hero's, one by one, with
//! the loot window's pick-up sound for each ([`RouletteWon`]).

use bevy::prelude::*;

use crate::sound::SoundCue;

use super::cards::Rarity;
use super::data;
use super::hud::{self, HudFont};
use super::{HeroState, LocalHero, Overcharge, Rng, Stats};

/// How many prizes a chest spins, and their base weights: five is the jackpot.
const COUNTS: [(u32, f32); 3] = [(1, 70.0), (3, 25.0), (5, 5.0)];
/// A prize rank-up's qualities and their weights: never under Rare.
const PRIZE_ROLLS: [(Rarity, f32); 2] = [(Rarity::Rare, 80.0), (Rarity::Epic, 20.0)];
/// The loot window's pick-up (`UiLootPickUpItem.wav`), as each prize lands.
pub(super) const PRIZE_SOUND: &str = "ChestBounce";
/// Real seconds between one prize landing and the next, and the panel's linger after the last.
const PRIZE_GAP: f32 = 0.55;
const LINGER: f32 = 2.5;

pub(super) fn plugin(app: &mut App) {
    app.add_message::<RouletteWon>()
        .init_resource::<Roulette>()
        .add_systems(Update, (spawn_panel, receive, play, show).chain());
}

/// One prize, as its hero's screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Prize {
    pub text: String,
    pub rarity: Rarity,
}

/// A boss chest's prizes for one hero, decided by the host: its player's screen spins them.
#[derive(Message, Clone, Debug, PartialEq)]
pub(super) struct RouletteWon {
    pub hero: Entity,
    pub prizes: Vec<Prize>,
}

/// The local hero's roulette on screen: its prizes, how many have landed, and the real seconds
/// since the last did.
#[derive(Resource, Default)]
pub(super) struct Roulette {
    prizes: Vec<Prize>,
    shown: usize,
    t: f32,
}

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct PanelLines;

/// How many prizes a chest spins: 1, 3 or 5, the bigger counts rarer, each step up weighted by
/// `stats`' luck ([`Stats::luck_weight`]).
pub(super) fn spin_count(stats: &Stats, rng: &mut Rng) -> u32 {
    let weighed = COUNTS
        .iter()
        .enumerate()
        .map(|(tier, &(n, w))| (n, w * stats.luck_weight(tier as i32)));
    let total: f32 = weighed.clone().map(|(_, w)| w).sum();
    let mut pick = rng.f32() * total;
    for (n, w) in weighed {
        if pick < w {
            return n;
        }
        pick -= w;
    }
    COUNTS[COUNTS.len() - 1].0
}

/// A prize rank-up's quality: Rare, or with luck Epic.
fn prize_rarity(stats: &Stats, rng: &mut Rng) -> Rarity {
    let rare = PRIZE_ROLLS[0].1;
    let epic = PRIZE_ROLLS[1].1 * stats.luck_weight(1);
    if rng.f32() * (rare + epic) < rare {
        Rarity::Rare
    } else {
        Rarity::Epic
    }
}

/// An Overcharge's short name.
fn overcharge_name(kind: Overcharge) -> &'static str {
    match kind {
        Overcharge::Damage => "Damage",
        Overcharge::Haste => "Haste",
        Overcharge::MaxHp => "Vitality",
        Overcharge::Speed => "Speed",
        Overcharge::Magnet => "Magnet",
        Overcharge::Gold => "Gold",
    }
}

/// One prize for `hero`, applied to it at once: a Rare or better rank-up of one of its spells
/// below the top rank (a spell at rank 6 or 7 with a legendary transforms outright), else of a
/// talent, else an Overcharge.
pub(super) fn prize(
    hero: &mut HeroState,
    spells: Option<&crate::ui_action::Spells>,
    rng: &mut Rng,
) -> Prize {
    let stats = hero.stats();
    let open: Vec<usize> = (0..hero.abilities.len())
        .filter(|&k| hero.abilities[k].level < data::MAX_ABILITY_LEVEL)
        .collect();
    if !open.is_empty() {
        let k = open[rng.index(open.len())];
        let (def, before) = (hero.abilities[k].def, hero.abilities[k].level);
        let name = hud::spell_name(spells, def.spell, def.key);
        let legend = data::legendary(def.key);
        let outright = legend.is_some() && before + 2 >= data::MAX_ABILITY_LEVEL;
        let rarity = prize_rarity(&stats, rng);
        let after = if outright {
            data::MAX_ABILITY_LEVEL
        } else {
            before
                .saturating_add(rarity.ranks())
                .min(data::MAX_ABILITY_LEVEL)
        };
        hero.abilities[k].level = after;
        super::mechanics::sync_legendaries(hero);
        return match legend.filter(|_| after >= data::MAX_ABILITY_LEVEL) {
            Some(l) => Prize {
                text: format!("{name} becomes {}!", l.name),
                rarity: Rarity::Legendary,
            },
            None => Prize {
                text: format!("{name}: rank {before} \u{bb} {after}"),
                rarity,
            },
        };
    }
    let open: Vec<usize> = (0..hero.passives.len())
        .filter(|&k| hero.passives[k].1 < data::MAX_PASSIVE_LEVEL)
        .collect();
    if !open.is_empty() {
        let k = open[rng.index(open.len())];
        let (p, before) = hero.passives[k];
        let rarity = prize_rarity(&stats, rng);
        let after = before
            .saturating_add(rarity.ranks())
            .min(data::MAX_PASSIVE_LEVEL);
        hero.passives[k].1 = after;
        return Prize {
            text: format!("{}: rank {before} \u{bb} {after}", p.name),
            rarity,
        };
    }
    let kind = Overcharge::ALL[rng.index(Overcharge::ALL.len())];
    hero.overcharges[kind.index()] += 1;
    Prize {
        text: format!("Overcharge: {}", overcharge_name(kind)),
        rarity: Rarity::Epic,
    }
}

/// `n` prizes for `hero` ([`prize`]).
pub(super) fn spin(
    hero: &mut HeroState,
    n: u32,
    spells: Option<&crate::ui_action::Spells>,
    rng: &mut Rng,
) -> Vec<Prize> {
    (0..n).map(|_| prize(hero, spells, rng)).collect()
}

fn spawn_panel(mut commands: Commands, font: Option<Res<HudFont>>, panels: Query<(), With<Panel>>) {
    let Some(font) = font else { return };
    if !panels.is_empty() {
        return;
    }
    commands
        .spawn((
            Panel,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(34.0),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(425),
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|p| {
            p.spawn((
                PanelLines,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::axes(Val::Px(28.0), Val::Px(14.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.03, 0.02, 0.88)),
                BorderColor::all(hud::GOLD_TONE),
            ))
            .with_children(|l| {
                l.spawn(hud::centered_bundle(
                    &font.0,
                    "The Boss's Hoard",
                    26.0,
                    hud::GOLD_TONE,
                ));
            });
        });
}

/// The local hero's prizes, as this machine's host (or this machine) decided them, start their
/// spin; a second chest's join the end of one still spinning.
fn receive(
    mut won: MessageReader<RouletteWon>,
    local: Query<Entity, With<LocalHero>>,
    mut roulette: ResMut<Roulette>,
) {
    let me = local.single().ok();
    for w in won.read() {
        if Some(w.hero) != me {
            continue;
        }
        if roulette.prizes.is_empty() {
            roulette.shown = 0;
            roulette.t = PRIZE_GAP;
        }
        roulette.prizes.extend(w.prizes.iter().cloned());
    }
}

/// The prizes land one every [`PRIZE_GAP`] real seconds, each with [`PRIZE_SOUND`], and the panel
/// lingers a while after the last.
fn play(time: Res<Time<Real>>, mut roulette: ResMut<Roulette>, mut cues: MessageWriter<SoundCue>) {
    if roulette.prizes.is_empty() {
        return;
    }
    roulette.t += time.delta_secs().min(0.1);
    if roulette.shown < roulette.prizes.len() {
        if roulette.t >= PRIZE_GAP {
            roulette.t = 0.0;
            roulette.shown += 1;
            cues.write(SoundCue::flat(PRIZE_SOUND));
        }
    } else if roulette.t >= LINGER {
        *roulette = Roulette::default();
    }
}

fn show(
    mut commands: Commands,
    roulette: Res<Roulette>,
    font: Option<Res<HudFont>>,
    mut panels: Query<&mut Node, With<Panel>>,
    lines: Query<(Entity, &Children), With<PanelLines>>,
    mut drawn: Local<usize>,
) {
    let on = !roulette.prizes.is_empty();
    for mut node in &mut panels {
        let display = if on { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    let Some(font) = font else { return };
    let want = if on { roulette.shown } else { 0 };
    if *drawn == want {
        return;
    }
    for (list, children) in &lines {
        // The title stays; the prizes are redrawn.
        for &child in children.into_iter().skip(1) {
            commands.entity(child).try_despawn();
        }
        commands.entity(list).with_children(|l| {
            for p in roulette.prizes.iter().take(want) {
                l.spawn(hud::centered_bundle(
                    &font.0,
                    p.text.clone(),
                    20.0,
                    p.rarity.color(),
                ));
            }
        });
    }
    *drawn = want;
}

#[cfg(test)]
mod tests {
    use super::super::bonus::{Bonus, Bonuses, StatKey};
    use super::super::Owned;
    use super::*;

    fn hero_with(key: &'static str, level: u8) -> HeroState {
        let mut hero = HeroState::new(0, &data::CLASSES[0], 1, Bonuses::default());
        hero.abilities = vec![Owned {
            def: data::ability(key),
            level,
            cd: 0.0,
        }];
        hero
    }

    #[test]
    fn a_chest_spins_one_three_or_five_and_luck_makes_more_likelier() {
        let count = |stats: &Stats| {
            let mut rng = Rng(3);
            let mut seen = [0u32; 6];
            for _ in 0..4000 {
                seen[spin_count(stats, &mut rng) as usize] += 1;
            }
            seen
        };
        let plain = count(&HeroState::default().stats());
        assert_eq!(
            plain[0] + plain[2] + plain[4],
            0,
            "only 1, 3 or 5: {plain:?}"
        );
        assert!(
            plain[1] > plain[3] && plain[3] > plain[5] && plain[5] > 0,
            "{plain:?}"
        );
        let mut lucky = HeroState::default();
        lucky.bonuses.add(&Bonus::new(StatKey::Luck, 1.0));
        let lucky = count(&lucky.stats());
        assert!(
            lucky[5] > plain[5] && lucky[1] < plain[1],
            "{lucky:?} vs {plain:?}"
        );
    }

    #[test]
    fn every_prize_is_rare_or_better() {
        let mut rng = Rng(9);
        let mut hero = hero_with("fireball", 1);
        hero.passives.push((&data::PASSIVES[0], 1));
        for _ in 0..12 {
            let p = prize(&mut hero, None, &mut rng);
            assert!(p.rarity.at_least(Rarity::Rare), "{p:?}");
        }
        // Everything mastered: Overcharges.
        let before: u32 = hero.overcharges.iter().sum();
        let p = prize(&mut hero, None, &mut rng);
        assert!(p.text.starts_with("Overcharge"), "{p:?}");
        assert_eq!(hero.overcharges.iter().sum::<u32>(), before + 1);
    }

    #[test]
    fn a_spell_at_rank_six_or_seven_transforms_outright() {
        for level in [6, 7] {
            let mut hero = hero_with("frostbolt", level);
            let p = prize(&mut hero, None, &mut Rng(1));
            assert_eq!(hero.abilities[0].level, data::MAX_ABILITY_LEVEL);
            assert_eq!(p.rarity, Rarity::Legendary);
            assert_eq!(p.text, "frostbolt becomes Shatterbolt!");
            assert!(super::super::mechanics::has(
                &hero,
                super::super::mechanics::MechanicId::Legendary("frostbolt")
            ));
        }
        // Lower, a Rare's two ranks or an Epic's three.
        let mut hero = hero_with("frostbolt", 2);
        let p = prize(&mut hero, None, &mut Rng(1));
        assert!([4, 5].contains(&hero.abilities[0].level), "{p:?}");
        assert_eq!(spin(&mut hero, 3, None, &mut Rng(2)).len(), 3);
    }
}
