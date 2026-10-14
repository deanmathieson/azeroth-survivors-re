//! The quest tracker, `QuestWatchFrame`-style under the minimap: the accepted quest's name in gold,
//! its objective, and the time left, or who to go back to once it is done. Hidden with no quest
//! accepted; it rides the in-run HUD, so it shows and hides with it.

use bevy::prelude::*;

use super::super::hud::{self, HudFont, HudRoot};
use super::field::{QuestRun, Stage};
use super::objective::{clock, Status, Tracker};

/// Under the 192-unit minimap cluster at the top right.
const TOP: f32 = 200.0;
const RIGHT: f32 = 14.0;
const WIDTH: f32 = 240.0;
/// `GameFontNormal`'s gold, and the time line's red when it is nearly out.
const TITLE: Color = hud::GOLD_TONE;
const OBJECTIVE: Color = Color::WHITE;
const HURRY: Color = Color::srgb(1.0, 0.25, 0.2);
const DONE: Color = Color::srgb(0.25, 0.9, 0.35);
/// Seconds left at which the time line turns red.
const HURRY_SECS: f32 = 15.0;

#[derive(Component)]
pub(super) struct TrackerRoot;

/// One of the tracker's three lines.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum TrackerLine {
    Title,
    Objective,
    Status,
}

/// What the tracker shows for an accepted quest: its three lines and the last one's colour.
pub(super) fn lines(name: &str, giver: &str, tracker: &Tracker) -> [(String, Color); 3] {
    let title = (name.to_string(), TITLE);
    let objective = (format!("- {}", tracker.objective()), OBJECTIVE);
    let status = match tracker.status {
        Status::Active => {
            let color = if tracker.left <= HURRY_SECS {
                HURRY
            } else {
                OBJECTIVE
            };
            (format!("Time left: {}", clock(tracker.left)), color)
        }
        Status::Complete => (format!("Complete: return to {giver}"), DONE),
        Status::Failed => ("Failed".to_string(), HURRY),
    };
    [title, objective, status]
}

/// Builds the tracker into the in-run HUD once the HUD is up.
pub(super) fn spawn_tracker(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    huds: Query<Entity, With<HudRoot>>,
    built: Query<(), With<TrackerRoot>>,
) {
    if !built.is_empty() {
        return;
    }
    let (Some(font), Ok(hud)) = (font, huds.single()) else {
        return;
    };
    let f = &font.0;
    commands.entity(hud).with_children(|root| {
        root.spawn((
            TrackerRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(RIGHT),
                top: Val::Px(TOP),
                width: Val::Px(WIDTH),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|col| {
            col.spawn((TrackerLine::Title, hud::text_bundle(f, "", 15.0, TITLE)));
            col.spawn((
                TrackerLine::Objective,
                hud::text_bundle(f, "", 13.0, OBJECTIVE),
            ));
            col.spawn((
                TrackerLine::Status,
                hud::text_bundle(f, "", 13.0, OBJECTIVE),
            ));
        });
    });
}

/// Fills the tracker from the quest under way, and hides it with none accepted.
pub(super) fn refresh_tracker(
    quests: Res<QuestRun>,
    mut roots: Query<&mut Node, With<TrackerRoot>>,
    mut texts: Query<(&TrackerLine, &mut Text, &mut TextColor)>,
) {
    let shown = quests
        .active
        .as_ref()
        .filter(|a| a.stage == Stage::Accepted)
        .map(|a| lines(a.quest.name, a.giver_name, &a.tracker));
    let display = if shown.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut roots {
        if node.display != display {
            node.display = display;
        }
    }
    let Some(shown) = shown else { return };
    for (line, mut text, mut color) in &mut texts {
        let (s, c) = &shown[*line as usize];
        if text.0 != *s {
            text.0.clone_from(s);
        }
        if color.0 != *c {
            color.0 = *c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{quest, Goal};
    use super::*;

    #[test]
    fn the_tracker_reads_like_the_quest_log() {
        let (_, def) = quest("elwynn_fargodeep_mine").unwrap();
        let mut t = Tracker::new(def);
        let [title, objective, status] = lines(def.name, "Marshal Dughan", &t);
        assert_eq!(title, ("The Fargodeep Mine".to_string(), TITLE));
        assert_eq!(objective.0, "- Kobold Tunneler slain: 0/10");
        assert_eq!(status, ("Time left: 2:00".to_string(), OBJECTIVE));
        t.tick(110.0, false);
        assert_eq!(
            lines(def.name, "Marshal Dughan", &t)[2],
            ("Time left: 0:10".into(), HURRY)
        );
        for _ in 0..10 {
            t.kill("Kobold Tunneler", false);
        }
        assert_eq!(
            lines(def.name, "Marshal Dughan", &t)[2],
            ("Complete: return to Marshal Dughan".into(), DONE)
        );
        assert!(matches!(t.goal, Goal::Kill { .. }));
    }
}
