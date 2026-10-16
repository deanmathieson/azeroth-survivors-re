//! The results for a party: a table of every hero's run (damage and its rate, healing, damage
//! taken, revives, gold, falls) and the awards, each an icon from the client's own set in a gold
//! rim: Top Damage, Healbot, Meat Shield, Necromancer, Gold Goblin and Floor Inspector. A lone
//! hero's results are as they were.

use bevy::prelude::*;

use super::{class_color, class_icon, Member, PartyView};
use crate::survivors::hud::{self, HudFont, OverParty, UiArt};
use crate::survivors::Phase;

/// One of the results' awards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Award {
    TopDamage,
    Healbot,
    MeatShield,
    Necromancer,
    GoldGoblin,
    FloorInspector,
}

impl Award {
    pub(super) const ALL: [Self; 6] = [
        Self::TopDamage,
        Self::Healbot,
        Self::MeatShield,
        Self::Necromancer,
        Self::GoldGoblin,
        Self::FloorInspector,
    ];

    pub(super) fn title(self) -> &'static str {
        match self {
            Self::TopDamage => "Top Damage",
            Self::Healbot => "Healbot",
            Self::MeatShield => "Meat Shield",
            Self::Necromancer => "Necromancer",
            Self::GoldGoblin => "Gold Goblin",
            Self::FloorInspector => "Floor Inspector",
        }
    }

    /// Its icon, an `Interface\Icons` file without the extension.
    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::TopDamage => "Interface\\Icons\\Ability_Warrior_InnerRage",
            Self::Healbot => "Interface\\Icons\\Spell_Holy_FlashHeal",
            Self::MeatShield => "Interface\\Icons\\INV_Shield_06",
            Self::Necromancer => "Interface\\Icons\\Spell_Shadow_RaiseDead",
            Self::GoldGoblin => "Interface\\Icons\\INV_Misc_Coin_02",
            Self::FloorInspector => "Interface\\Icons\\Ability_Rogue_FeignDeath",
        }
    }

    /// What it is won by.
    pub(super) fn score(self, m: &Member) -> f64 {
        match self {
            Self::TopDamage => m.damage_done,
            Self::Healbot => m.healing_done,
            Self::MeatShield => m.damage_taken,
            Self::Necromancer => f64::from(m.revives),
            Self::GoldGoblin => f64::from(m.gold),
            Self::FloorInspector => f64::from(m.downs),
        }
    }

    /// The winning score, as the badge reads it.
    pub(super) fn line(self, v: f64) -> String {
        let n = v.round() as u64;
        match self {
            Self::TopDamage => format!("{} damage", hud::compact(v)),
            Self::Healbot => format!("{} healing", hud::compact(v)),
            Self::MeatShield => format!("{} taken", hud::compact(v)),
            Self::Necromancer => format!("{n} revive{}", if n == 1 { "" } else { "s" }),
            Self::GoldGoblin => format!("{n} gold"),
            Self::FloorInspector => format!("down {n} time{}", if n == 1 { "" } else { "s" }),
        }
    }
}

/// An award won: by whom (indices into the members; a tie shares it) and with what.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Won {
    pub award: Award,
    pub winners: Vec<usize>,
    pub score: f64,
}

/// The awards for a party of `members`: each goes to the highest score, shared on a tie, and to
/// no one when no one scored (no healing done, no one revived). None for a lone hero.
pub(super) fn awards(members: &[Member]) -> Vec<Won> {
    if members.len() < 2 {
        return Vec::new();
    }
    Award::ALL
        .into_iter()
        .filter_map(|award| {
            let best = members
                .iter()
                .map(|m| award.score(m))
                .fold(0.0_f64, f64::max);
            (best > 0.0).then(|| Won {
                award,
                winners: (0..members.len())
                    .filter(|&i| award.score(&members[i]) == best)
                    .collect(),
                score: best,
            })
        })
        .collect()
}

/// The table's heading, and its numbers for a member.
const COLUMNS: [&str; 7] = [
    "Damage", "DPS", "Healing", "Taken", "Revives", "Gold", "Downs",
];

pub(super) fn columns(m: &Member, elapsed: f32) -> [String; 7] {
    [
        hud::compact(m.damage_done),
        format!("{:.1}", super::meter::per_second(m.damage_done, elapsed)),
        hud::compact(m.healing_done),
        hud::compact(m.damage_taken),
        m.revives.to_string(),
        m.gold.to_string(),
        m.downs.to_string(),
    ]
}

/// What the results' party row shows: rebuilt when it changes.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Board {
    members: Vec<Member>,
    elapsed: f32,
}

const NAME_W: f32 = 96.0;
const RACE_W: f32 = 104.0;
const NUM_W: f32 = 54.0;
const BADGE_W: f32 = 96.0;
const BADGE_ICON: f32 = 32.0;
const PARCHMENT: Color = Color::srgb(0.85, 0.78, 0.6);

/// On the results with a party: the table and the awards in the results' party row; out of the
/// layout for a lone hero.
pub(super) fn refresh(
    mut commands: Commands,
    phase: Res<State<Phase>>,
    view: Res<PartyView>,
    font: Option<Res<HudFont>>,
    mut rows: Query<(Entity, &mut Node), With<OverParty>>,
    mut shown: Local<Option<Board>>,
) {
    let on = *phase.get() == Phase::GameOver && view.coop();
    let Ok((row, mut node)) = rows.single_mut() else {
        return;
    };
    let want = if on { Display::Flex } else { Display::None };
    if node.display != want {
        node.display = want;
    }
    if !on {
        *shown = None;
        return;
    }
    let board = Board {
        members: view.members.clone(),
        elapsed: view.elapsed,
    };
    if shown.as_ref() == Some(&board) {
        return;
    }
    let Some(font) = font else { return };
    let f = &font.0;
    commands.entity(row).despawn_related::<Children>();
    commands.entity(row).with_children(|party| {
        // The table.
        party
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(1.0),
                ..default()
            })
            .with_children(|table| {
                let cell = |w: f32| Node {
                    width: Val::Px(w),
                    justify_content: JustifyContent::Center,
                    ..default()
                };
                table
                    .spawn(Node {
                        column_gap: Val::Px(4.0),
                        ..default()
                    })
                    .with_children(|head| {
                        head.spawn(Node {
                            width: Val::Px(14.0 + 4.0 + NAME_W + 4.0 + RACE_W),
                            ..default()
                        });
                        for title in COLUMNS {
                            head.spawn(cell(NUM_W)).with_children(|c| {
                                c.spawn(hud::text_bundle(f, title, 11.0, hud::GOLD_TONE));
                            });
                        }
                    });
                for m in &board.members {
                    table
                        .spawn(Node {
                            column_gap: Val::Px(4.0),
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|line| {
                            line.spawn((
                                Node {
                                    width: Val::Px(14.0),
                                    height: Val::Px(14.0),
                                    ..default()
                                },
                                ImageNode {
                                    color: Color::NONE,
                                    ..default()
                                },
                                class_icon(m.class),
                            ));
                            line.spawn(Node {
                                width: Val::Px(NAME_W),
                                overflow: Overflow::clip_x(),
                                ..default()
                            })
                            .with_children(|c| {
                                c.spawn(hud::text_bundle(
                                    f,
                                    m.name.clone(),
                                    13.0,
                                    class_color(m.class),
                                ));
                            });
                            let class = super::class_def(m.class).map_or("", |c| c.name);
                            let race = crate::char_select::race_name(m.race);
                            line.spawn(Node {
                                width: Val::Px(RACE_W),
                                ..default()
                            })
                            .with_children(|c| {
                                c.spawn(hud::text_bundle(
                                    f,
                                    format!("{race} {class}"),
                                    11.0,
                                    hud::DIM_GREY,
                                ));
                            });
                            for v in columns(m, board.elapsed) {
                                line.spawn(cell(NUM_W)).with_children(|c| {
                                    c.spawn(hud::text_bundle(f, v, 13.0, PARCHMENT));
                                });
                            }
                        });
                }
            });
        // The awards.
        party
            .spawn(Node {
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|badges| {
                for won in awards(&board.members) {
                    badges
                        .spawn(Node {
                            width: Val::Px(BADGE_W),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(1.0),
                            ..default()
                        })
                        .with_children(|badge| {
                            badge
                                .spawn((
                                    Node {
                                        width: Val::Px(BADGE_ICON + 4.0),
                                        height: Val::Px(BADGE_ICON + 4.0),
                                        border: UiRect::all(Val::Px(2.0)),
                                        border_radius: BorderRadius::all(Val::Px(3.0)),
                                        ..default()
                                    },
                                    BorderColor::all(hud::GOLD_TONE),
                                    BackgroundColor(Color::BLACK),
                                    BoxShadow::new(
                                        Color::srgba(1.0, 0.7, 0.2, 0.4),
                                        Val::Px(0.0),
                                        Val::Px(0.0),
                                        Val::Px(0.0),
                                        Val::Px(8.0),
                                    ),
                                ))
                                .with_children(|rim| {
                                    rim.spawn((
                                        Node {
                                            width: Val::Px(BADGE_ICON),
                                            height: Val::Px(BADGE_ICON),
                                            ..default()
                                        },
                                        ImageNode {
                                            color: Color::NONE,
                                            ..default()
                                        },
                                        UiArt::crop(won.award.icon(), [0.0, 1.0, 0.0, 1.0]),
                                    ));
                                });
                            badge.spawn(hud::centered_bundle(
                                f,
                                won.award.title(),
                                12.0,
                                hud::GOLD_TONE,
                            ));
                            badge
                                .spawn(Node {
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    ..default()
                                })
                                .with_children(|names| {
                                    for &i in &won.winners {
                                        let m = &board.members[i];
                                        names.spawn(hud::centered_bundle(
                                            f,
                                            m.name.clone(),
                                            11.0,
                                            class_color(m.class),
                                        ));
                                    }
                                });
                            badge.spawn(hud::centered_bundle(
                                f,
                                won.award.line(won.score),
                                10.0,
                                hud::DIM_GREY,
                            ));
                        });
                }
            });
    });
    *shown = Some(board);
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::tests::{member, view_of};
    use super::*;

    fn party() -> Vec<Member> {
        let mut a = member(0, "Warrior");
        let mut b = member(1, "Priest");
        let mut c = member(2, "Rogue");
        (a.damage_done, b.damage_done, c.damage_done) = (900.0, 200.0, 1400.0);
        (a.healing_done, b.healing_done, c.healing_done) = (10.0, 800.0, 0.0);
        (a.damage_taken, b.damage_taken, c.damage_taken) = (700.0, 300.0, 250.0);
        (a.revives, b.revives, c.revives) = (0, 2, 2);
        (a.gold, b.gold, c.gold) = (40, 15, 90);
        (a.downs, b.downs, c.downs) = (3, 1, 0);
        vec![a, b, c]
    }

    #[test]
    fn each_award_goes_to_the_best_and_a_tie_shares_it() {
        let won = awards(&party());
        let by = |award: Award| won.iter().find(|w| w.award == award).cloned();
        assert_eq!(by(Award::TopDamage).unwrap().winners, [2]);
        assert_eq!(by(Award::Healbot).unwrap().winners, [1]);
        assert_eq!(by(Award::MeatShield).unwrap().winners, [0]);
        let necro = by(Award::Necromancer).unwrap();
        assert_eq!(necro.winners, [1, 2], "a tie shares the award");
        assert_eq!(Award::Necromancer.line(necro.score), "2 revives");
        assert_eq!(by(Award::GoldGoblin).unwrap().winners, [2]);
        let floor = by(Award::FloorInspector).unwrap();
        assert_eq!((floor.winners.as_slice(), floor.score), (&[0][..], 3.0));
        assert_eq!(Award::FloorInspector.line(1.0), "down 1 time");
    }

    #[test]
    fn no_score_no_award_and_none_alone() {
        let mut members = party();
        for m in &mut members {
            (m.healing_done, m.revives) = (0.0, 0);
        }
        let won: Vec<Award> = awards(&members).into_iter().map(|w| w.award).collect();
        assert!(!won.contains(&Award::Healbot) && !won.contains(&Award::Necromancer));
        assert_eq!(won.len(), 4);
        assert!(
            awards(&members[..1]).is_empty(),
            "a lone hero has its run, not awards"
        );
        // A party all tied at the top: everyone shares.
        let mut even = party();
        for m in &mut even {
            m.gold = 10;
        }
        let gold = awards(&even)
            .into_iter()
            .find(|w| w.award == Award::GoldGoblin)
            .unwrap();
        assert_eq!(gold.winners, [0, 1, 2]);
    }

    #[test]
    fn the_table_reads_each_heros_run() {
        let m = &party()[2];
        let cols = columns(m, 70.0);
        assert_eq!(cols, ["1400", "20.0", "0", "250", "2", "90", "0"]);
    }

    fn results_world(view: PartyView) -> World {
        let mut world = World::new();
        world.insert_resource(HudFont(Handle::default()));
        world.insert_resource(State::new(Phase::GameOver));
        world.insert_resource(view);
        world.spawn((
            OverParty,
            Node {
                display: Display::None,
                ..default()
            },
        ));
        world.run_system_once(refresh).unwrap();
        world
    }

    fn row(world: &mut World) -> (Display, usize) {
        let mut q = world.query_filtered::<(&Node, Option<&Children>), With<OverParty>>();
        let (node, children) = q.single(world).unwrap();
        (node.display, children.map_or(0, |c| c.len()))
    }

    #[test]
    fn the_results_show_the_party_row_only_for_a_party() {
        let mut world = results_world(view_of(vec![member(0, "Warrior")]));
        assert_eq!(row(&mut world), (Display::None, 0));
        let mut world = results_world(view_of(party()));
        // The table and the awards.
        assert_eq!(row(&mut world), (Display::Flex, 2));
    }
}
