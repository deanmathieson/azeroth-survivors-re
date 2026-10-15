//! The Armory's Companion tab, the pet's trainer. On the left, the pet: its name (Rename picks the
//! next), its level and experience bar, the training points to spend, and what it does now. In the
//! middle, every skill: its rank pips, what it does now and at its next rank, the next rank's cost
//! in training points, Train and Untrain, and the trainer's free Reset under them. On the right,
//! the looks, two to a row: those still locked dimmed with what unlocks them, the worn one lit;
//! with the Second Companion, a pair of buttons picks which pet a click dresses. The Companion
//! perk itself is bought on the Rewards tab ([`super::rewards`]); the clicks land in
//! [`crate::survivors::rewards`] as [`CompanionAction`]s.

use bevy::prelude::*;

use super::collection::sheen;
use super::{panel, para, text, ArmoryAction, ArmoryTab, ArmoryTip, TabCtx};
use crate::glue::art::{DIM, GOLD};
use crate::glue::widgets::{glue_button, GlueBtnKind, GlueDisabled, LockHighlight};
use crate::survivors::achievements::{catalog, AchievementSave};
use crate::survivors::data;
use crate::survivors::profile::Profile;
use crate::survivors::quests::{for_map, Reward};
use crate::survivors::rewards::looks::{Look, Unlock};
use crate::survivors::rewards::pet::{self, Blocked, PetSkills, Skill, SKILLS};
use crate::survivors::rewards::{
    all_looks, companion_unlocked, offered, slot_look, CompanionAction, CompanionView, RewardsSave,
};

/// The pet's column, as wide as the Armory's own side columns, and the looks', wider for two
/// names to a row.
const SIDE_W: f32 = 250.0;
const LOOKS_W: f32 = 300.0;
const SKILL_H: f32 = 58.0;
const LOOK_H: f32 = 36.0;
const GRID_GAP: f32 = 4.0;
const BUTTON_W: f32 = 62.0;
const BUTTON_H: f32 = 24.0;

/// A description's parchment.
const DESC: Color = Color::srgb(0.85, 0.82, 0.72);
/// `GREEN_FONT_COLOR`, what a skill does.
const GREEN: Color = Color::srgb(0.1, 1.0, 0.1);
const TIP_GOLD: Color = Color::srgb(1.0, 0.82, 0.0);
const ROW_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.45);
const ROW_EDGE: Color = Color::srgb(0.3, 0.28, 0.24);
const PIP_EMPTY: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const PIP_EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const BAR_BACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const BAR_EDGE: Color = Color::srgb(0.45, 0.4, 0.3);
/// The pet experience bar's violet.
const BAR_FILL: Color = Color::srgb(0.45, 0.2, 0.7);

pub(super) fn build(parent: &mut ChildSpawnerCommands, ctx: &TabCtx) {
    let fresh = Profile::default();
    let profile = ctx.res::<Profile>().unwrap_or(&fresh);
    let view = ctx.res::<CompanionView>().copied().unwrap_or_default();
    let save = &profile.rewards;
    let column = |width: f32| Node {
        width: Val::Px(width),
        height: Val::Percent(100.0),
        flex_shrink: 0.0,
        ..default()
    };
    panel(parent, ctx, column(SIDE_W), Some("Companion"), |p| {
        pet_card(p, ctx, save)
    });
    let middle = Node {
        flex_grow: 1.0,
        height: Val::Percent(100.0),
        ..default()
    };
    panel(parent, ctx, middle, Some("Training"), |p| {
        training(p, ctx, save)
    });
    panel(parent, ctx, column(LOOKS_W), Some("Looks"), |p| {
        looks(p, ctx, save, &profile.achievements, view)
    });
}

/// The left column: name, level, experience, training points, and what the pet does now.
fn pet_card(p: &mut ChildSpawnerCommands, ctx: &TabCtx, save: &RewardsSave) {
    let level = pet::level(save.pet_xp);
    p.spawn(Node {
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|r| {
        text(r, ctx, pet::name(save), 20.0, Color::WHITE);
        glue_button(
            r,
            ctx.art,
            ctx.font,
            CompanionAction::Rename,
            "Rename",
            BUTTON_W + 10.0,
            BUTTON_H,
            GlueBtnKind::Small,
            1.0,
        );
    });
    gap(p, 6.0);
    let shown = if level >= pet::MAX_LEVEL {
        format!("{level} (top)")
    } else {
        format!("{level} of {}", pet::MAX_LEVEL)
    };
    line(p, ctx, "Level", &shown);
    let (frac, label) = match pet::progress(save.pet_xp) {
        Some((into, need)) => (into as f32 / need as f32, format!("{into} / {need} XP")),
        None => (1.0, "Top level".to_string()),
    };
    bar(p, ctx, frac, &label);
    gap(p, 12.0);
    text(p, ctx, "Training points", 13.0, GOLD);
    text(
        p,
        ctx,
        &pet::available(save).to_string(),
        28.0,
        Color::WHITE,
    );
    line(p, ctx, "Earned", &pet::training_points(level).to_string());
    line(p, ctx, "Spent", &pet::spent(save).to_string());
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    para(p, ctx, &does(PetSkills::of(save)), 11.0, DESC);
    gap(p, 6.0);
    let [a, b, c, d] = pet::BONUS_TP_LEVELS;
    para(
        p,
        ctx,
        &format!(
            "It earns {} XP for every item it fetches and {} for every run. Each level brings a \
             training point, and levels {a}, {b}, {c} and {d} one more.",
            pet::XP_PER_ITEM,
            pet::RUN_XP
        ),
        11.0,
        DIM,
    );
    if !companion_unlocked(save) {
        gap(p, 8.0);
        para(
            p,
            ctx,
            "Buy the Companion perk on the Rewards tab and it comes along on every run.",
            12.0,
            GOLD,
        );
        gap(p, 4.0);
        p.spawn(Node {
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|b| {
            glue_button(
                b,
                ctx.art,
                ctx.font,
                ArmoryAction::Tab(ArmoryTab::Rewards),
                "Rewards",
                120.0,
                BUTTON_H + 4.0,
                GlueBtnKind::Small,
                1.0,
            );
        });
    }
}

/// What the companion does with `skills`, in a sentence or two.
fn does(skills: PetSkills) -> String {
    let every = pet::fetch_every(skills.swiftness);
    let reach = pet::fetch_reach(skills.wide_patrol);
    let (who, fetches) = if skills.second {
        ("Your two companions follow", "each fetches")
    } else {
        ("Your companion follows", "it fetches")
    };
    let what = match pet::items_per_trip(skills.saddlebags) {
        1 => "the nearest gem or coin that lies".to_string(),
        n => format!("up to {n} gems or coins that lie"),
    };
    format!(
        "{who} you on every run. Every {every:.1} seconds {fetches} {what} past your pickup \
         range, within {reach} yards."
    )
}

/// The middle column: every skill, then the free reset.
fn training(p: &mut ChildSpawnerCommands, ctx: &TabCtx, save: &RewardsSave) {
    for (i, skill) in SKILLS.iter().enumerate() {
        skill_row(p, ctx, i, skill, save);
    }
    p.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });
    p.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|r| {
        let reset = glue_button(
            r,
            ctx.art,
            ctx.font,
            CompanionAction::Reset,
            "Reset training",
            170.0,
            32.0,
            GlueBtnKind::Small,
            1.0,
        );
        if pet::spent(save) == 0 {
            r.commands().entity(reset).insert(GlueDisabled(true));
        }
        text(
            r,
            ctx,
            "Free at the trainer: every point comes back.",
            11.0,
            DIM,
        );
    });
}

/// One skill: name and pips, what it does, its effect now and next; the cost, Train and Untrain.
fn skill_row(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    i: usize,
    skill: &Skill,
    save: &RewardsSave,
) {
    let rank = skill.rank(save);
    let next = skill.next_cost(save);
    let effect = match (skill.effect_at(rank), next.and(skill.effect_at(rank + 1))) {
        (None, Some(next)) => format!("Next: {next}"),
        (Some(now), Some(next)) => format!("Now: {now}   Next: {next}"),
        (Some(now), None) => format!("Now: {now} (top rank)"),
        (None, None) => String::new(),
    };
    let check = pet::can_train(save, skill);
    let (label, color) = match check {
        Ok(cost) => (format!("Cost: {cost} TP"), Color::WHITE),
        Err(Blocked::TopRank) => ("Top rank".to_string(), DIM),
        Err(Blocked::Level(level)) => (format!("Pet level {level}"), DIM),
        Err(Blocked::Points(cost)) => (format!("Cost: {cost} TP"), DIM),
        Err(Blocked::NoCompanion) => (format!("Cost: {} TP", next.unwrap_or(0)), DIM),
    };
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(SKILL_H),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            margin: UiRect::bottom(Val::Px(4.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(ROW_FILL),
        BorderColor::all(ROW_EDGE),
    ))
    .with_children(|r| {
        // No content-sized minimum, so a long effect line is clipped, never pushing out the buttons.
        r.spawn(Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            overflow: Overflow::clip(),
            ..default()
        })
        .with_children(|t| {
            t.spawn(Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|n| {
                text(n, ctx, skill.name, 14.0, GOLD);
                pips(n, rank, skill.max_rank());
            });
            text(t, ctx, skill.desc, 11.0, DESC);
            text(t, ctx, &effect, 11.0, GREEN);
        });
        r.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(3.0),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|a| {
            text(a, ctx, &label, 12.0, color);
            a.spawn(Node {
                column_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|b| {
                let train = glue_button(
                    b,
                    ctx.art,
                    ctx.font,
                    CompanionAction::Train(i),
                    "Train",
                    BUTTON_W,
                    BUTTON_H,
                    GlueBtnKind::Small,
                    1.0,
                );
                if check.is_err() {
                    b.commands().entity(train).insert(GlueDisabled(true));
                }
                let untrain = glue_button(
                    b,
                    ctx.art,
                    ctx.font,
                    CompanionAction::Untrain(i),
                    "Untrain",
                    BUTTON_W,
                    BUTTON_H,
                    GlueBtnKind::Small,
                    1.0,
                );
                if rank == 0 {
                    b.commands().entity(untrain).insert(GlueDisabled(true));
                }
            });
        });
    });
}

/// A skill's ranks as pips, the trained ones gold.
fn pips(p: &mut ChildSpawnerCommands, rank: u32, max: u32) {
    p.spawn(Node {
        column_gap: Val::Px(3.0),
        align_items: AlignItems::Center,
        ..default()
    })
    .with_children(|row| {
        for i in 0..max {
            let fill = if i < rank { GOLD } else { PIP_EMPTY };
            row.spawn((
                Node {
                    width: Val::Px(9.0),
                    height: Val::Px(9.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(fill),
                BorderColor::all(PIP_EDGE),
            ));
        }
    });
}

/// The right column: which pet a click dresses, with the Second Companion, then every look.
fn looks(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    save: &RewardsSave,
    earned: &AchievementSave,
    view: CompanionView,
) {
    let second = PetSkills::of(save).second;
    let dressing = if second { view.dressing } else { 0 };
    if second {
        p.spawn(Node {
            column_gap: Val::Px(GRID_GAP),
            flex_shrink: 0.0,
            margin: UiRect::bottom(Val::Px(6.0)),
            ..default()
        })
        .with_children(|r| {
            for (slot, label) in [(0, "First pet"), (1, "Second pet")] {
                let picked = dressing == slot;
                cell(r, ctx, CompanionAction::Dressing(slot), picked, None, |c| {
                    let color = if picked { Color::WHITE } else { GOLD };
                    text(c, ctx, label, 12.0, color);
                    text(c, ctx, slot_look(save, slot).name, 10.0, DESC);
                });
            }
        });
    }
    p.spawn(Node {
        width: Val::Percent(100.0),
        display: Display::Grid,
        grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
        column_gap: Val::Px(GRID_GAP),
        row_gap: Val::Px(GRID_GAP),
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|grid| {
        let worn = [slot_look(save, 0).key, slot_look(save, 1).key];
        // Indexed over every look, so a look keeps its place once unlocked.
        for (i, look) in all_looks().enumerate() {
            let open = offered(look, save, earned);
            let picked = look.key == worn[dressing as usize];
            let (status, status_color) = if !open {
                (short_source(look.unlock), DIM)
            } else if look.key == worn[0] && second && look.key == worn[1] {
                ("Worn by both".to_string(), GREEN)
            } else if look.key == worn[0] {
                (
                    if second { "Worn by the first" } else { "Worn" }.to_string(),
                    GREEN,
                )
            } else if second && look.key == worn[1] {
                ("Worn by the second".to_string(), GREEN)
            } else {
                (String::new(), DIM)
            };
            let tip = look_tip(look, open, earned);
            cell(
                grid,
                ctx,
                CompanionAction::Look(i),
                picked,
                Some(tip),
                |c| {
                    let color = match (open, picked) {
                        (false, _) => DIM,
                        (true, true) => Color::WHITE,
                        (true, false) => GOLD,
                    };
                    text(c, ctx, look.name, 12.0, color);
                    text(c, ctx, &status, 10.0, status_color);
                },
            );
        }
    });
}

/// A framed, clickable cell of the looks column, lit while `picked`, with its hover box if it has
/// one: `body` fills it.
fn cell(
    p: &mut ChildSpawnerCommands,
    ctx: &TabCtx,
    action: CompanionAction,
    picked: bool,
    tip: Option<ArmoryTip>,
    body: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let mut node = p.spawn((
        Button,
        action,
        LockHighlight(picked),
        Node {
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            height: Val::Px(LOOK_H),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            padding: UiRect::horizontal(Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(ROW_FILL),
        BorderColor::all(ROW_EDGE),
    ));
    if let Some(tip) = tip {
        node.insert(tip);
    }
    node.with_children(|c| {
        sheen(c, ctx);
        body(c);
    });
}

/// A locked look's source, short enough for its cell.
fn short_source(unlock: Unlock) -> String {
    match unlock {
        Unlock::Start => String::new(),
        Unlock::PetLevel(level) => format!("Pet level {level}"),
        Unlock::Achievement(_) => "Achievement".into(),
        Unlock::Quest => "Quest chain".into(),
        Unlock::Faire => "Darkmoon Faire".into(),
    }
}

/// A look's hover box: what unlocks it, and whether it is open to wear.
fn look_tip(look: &Look, open: bool, earned: &AchievementSave) -> ArmoryTip {
    let mut lines = vec![(source(look), TIP_GOLD)];
    if let Unlock::Achievement(id) = look.unlock {
        if let Some(a) = catalog().iter().find(|a| a.id == id) {
            lines.push((a.desc.clone(), DESC));
            if earned.has(id) {
                lines.push(("Earned".into(), GREEN));
            }
        }
    }
    lines.push(if open {
        ("Click to dress your companion".into(), GREEN)
    } else {
        ("Locked".into(), DIM)
    });
    ArmoryTip {
        title: look.name.to_string(),
        lines,
    }
}

/// What unlocks `look`, in full.
fn source(look: &Look) -> String {
    match look.unlock {
        Unlock::Start => "Yours from the start".into(),
        Unlock::PetLevel(level) => format!("Unlocks at pet level {level}"),
        Unlock::Achievement(id) => {
            let name = catalog()
                .iter()
                .find(|a| a.id == id)
                .map_or(id, |a| a.name.as_str());
            format!("Achievement reward: {name}")
        }
        Unlock::Quest => {
            let chain = (0..data::MAPS.len())
                .filter_map(for_map)
                .find(|z| z.chain.reward == Reward::CompanionLook(look.display));
            match chain {
                Some(z) => format!("Quest chain reward: {} ({})", z.chain.name, z.zone),
                None => "Quest chain reward".into(),
            }
        }
        Unlock::Faire => "Darkmoon Faire prize".into(),
    }
}

/// The pet's experience into its level, as a bar with its label over it.
fn bar(p: &mut ChildSpawnerCommands, ctx: &TabCtx, frac: f32, label: &str) {
    p.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(16.0),
            flex_shrink: 0.0,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(BAR_BACK),
        BorderColor::all(BAR_EDGE),
    ))
    .with_children(|b| {
        b.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(frac.clamp(0.0, 1.0) * 100.0),
                ..default()
            },
            BackgroundColor(BAR_FILL),
        ));
        text(b, ctx, label, 11.0, Color::WHITE);
    });
}

/// A label and its value on one line.
fn line(p: &mut ChildSpawnerCommands, ctx: &TabCtx, label: &str, value: &str) {
    p.spawn(Node {
        height: Val::Px(20.0),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|r| {
        text(r, ctx, label, 12.0, GOLD);
        text(r, ctx, value, 12.0, Color::WHITE);
    });
}

/// Empty room, `h` tall.
fn gap(p: &mut ChildSpawnerCommands, h: f32) {
    p.spawn(Node {
        height: Val::Px(h),
        flex_shrink: 0.0,
        ..default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::rewards::look_for;

    #[test]
    fn the_tab_words_the_pet_and_its_looks() {
        let mut save = RewardsSave::default();
        assert_eq!(
            does(PetSkills::of(&save)),
            "Your companion follows you on every run. Every 2.0 seconds it fetches the nearest \
             gem or coin that lies past your pickup range, within 25 yards."
        );
        save.pet_skills.insert("saddlebags".into(), 1);
        save.pet_skills.insert("swiftness".into(), 1);
        save.pet_skills.insert("second_companion".into(), 1);
        assert_eq!(
            does(PetSkills::of(&save)),
            "Your two companions follow you on every run. Every 1.7 seconds each fetches up to 2 \
             gems or coins that lie past your pickup range, within 25 yards."
        );
        assert_eq!(source(look_for("frog")), "Yours from the start");
        assert_eq!(source(look_for("siamese")), "Unlocks at pet level 3");
        assert_eq!(
            source(look_for("mini_diablo")),
            "Achievement reward: Nemesis"
        );
        assert!(
            source(look_for("black_kingsnake")).starts_with("Quest chain reward: "),
            "{}",
            source(look_for("black_kingsnake"))
        );
        let tip = look_tip(look_for("zergling"), false, &AchievementSave::default());
        assert_eq!(tip.title, "Zergling");
        assert_eq!(tip.lines.last().unwrap().0, "Locked");
    }
}
