//! Spectating while downed. A downed player in a co-op run can watch the living: **Tab** or **→**
//! moves the camera to the next ally standing, **←** to the one before, with "Spectating <Name>"
//! at the top of the screen. When the watched ally falls the camera moves on, and when the player
//! is revived it snaps back to their own hero ([`Spectate`]).

use bevy::prelude::*;

use super::super::super::hero::{Hero, Spectate};
use super::super::super::hud::HudFont;
use super::super::super::{HeroState, LocalHero, Phase};
use super::super::wire::NetHero;
use super::super::Session;
use super::chat::Chat;
use super::label;

/// The ally `step` places on from `current` among the `living` (in party order), wrapping round;
/// with `current` gone, the first. `None` with nobody standing.
pub(super) fn cycle(current: Option<Entity>, living: &[Entity], step: i32) -> Option<Entity> {
    if living.is_empty() {
        return None;
    }
    let Some(i) = current.and_then(|c| living.iter().position(|&e| e == c)) else {
        return Some(if step < 0 {
            living[living.len() - 1]
        } else {
            living[0]
        });
    };
    let n = living.len() as i32;
    Some(living[(i as i32 + step).rem_euclid(n) as usize])
}

/// The ally still watched: `current` while it stands, else the next one standing.
pub(super) fn keep(current: Option<Entity>, living: &[Entity]) -> Option<Entity> {
    match current {
        Some(c) if living.contains(&c) => Some(c),
        Some(_) => cycle(None, living, 1),
        None => None,
    }
}

#[derive(Component)]
struct SpectateText;

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (spectate, spectate_text).chain())
        .add_systems(OnEnter(Phase::Menu), stop);
}

fn stop(mut watch: ResMut<Spectate>) {
    if watch.0.is_some() {
        watch.0 = None;
    }
}

/// Tab and the arrows, while this player is down in a co-op run, move the camera along the allies
/// standing; up again, it comes home.
#[allow(clippy::type_complexity)]
fn spectate(
    keys: Res<ButtonInput<KeyCode>>,
    phase: Res<State<Phase>>,
    session: Res<Session>,
    chat: Res<Chat>,
    local: Query<&HeroState, With<LocalHero>>,
    allies: Query<(Entity, &HeroState, Option<&NetHero>), (With<Hero>, Without<LocalHero>)>,
    mut watch: ResMut<Spectate>,
) {
    let downed = session.in_run
        && *phase.get() == Phase::Playing
        && local.single().is_ok_and(|me| !me.alive());
    if !downed {
        if watch.0.is_some() {
            watch.0 = None;
        }
        return;
    }
    let mut living: Vec<(u8, Entity)> = allies
        .iter()
        .filter(|(_, h, _)| h.alive())
        .map(|(e, h, n)| (n.map_or(h.player, |n| n.player), e))
        .collect();
    living.sort_by_key(|l| l.0);
    let living: Vec<Entity> = living.into_iter().map(|l| l.1).collect();
    let mut want = keep(watch.0, &living);
    if !chat.typing() {
        if keys.any_just_pressed([KeyCode::Tab, KeyCode::ArrowRight]) {
            want = cycle(want, &living, 1);
        } else if keys.just_pressed(KeyCode::ArrowLeft) {
            want = cycle(want, &living, -1);
        }
    }
    if watch.0 != want {
        watch.0 = want;
    }
}

/// "Spectating <Name>" while watching; the hint while down with allies standing.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn spectate_text(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    phase: Res<State<Phase>>,
    session: Res<Session>,
    watch: Res<Spectate>,
    local: Query<&HeroState, With<LocalHero>>,
    names: Query<&NetHero>,
    allies: Query<&HeroState, (With<Hero>, Without<LocalHero>)>,
    mut texts: Query<(&mut Text, &mut Visibility), With<SpectateText>>,
) {
    let Some(font) = font else { return };
    if texts.is_empty() {
        commands.spawn((
            SpectateText,
            label(&font.0, "", 20.0, Color::srgb(1.0, 0.82, 0.0)),
            TextShadow::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(96.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-160.0)),
                width: Val::Px(320.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Center),
            GlobalZIndex(300),
            Visibility::Hidden,
        ));
        return;
    }
    let downed = session.in_run
        && *phase.get() == Phase::Playing
        && local.single().is_ok_and(|me| !me.alive());
    let line = match watch.0 {
        Some(e) => Some(format!(
            "Spectating {}",
            names.get(e).map_or("an ally", |n| n.name.as_str())
        )),
        None if downed && allies.iter().any(HeroState::alive) => {
            Some("Downed: Tab or \u{2190} \u{2192} to watch an ally".to_string())
        }
        None => None,
    };
    for (mut t, mut v) in &mut texts {
        match &line {
            Some(l) => {
                if t.0 != *l {
                    t.0 = l.clone();
                }
                v.set_if_neq(Visibility::Inherited);
            }
            None => {
                v.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_cycles_through_the_living() {
        let mut world = World::new();
        let [a, b, c] = [(); 3].map(|_| world.spawn_empty().id());
        let living = [a, b, c];
        assert_eq!(cycle(None, &living, 1), Some(a));
        assert_eq!(cycle(None, &living, -1), Some(c));
        assert_eq!(cycle(Some(a), &living, 1), Some(b));
        assert_eq!(cycle(Some(c), &living, 1), Some(a), "round the end");
        assert_eq!(cycle(Some(a), &living, -1), Some(c), "and back");
        assert_eq!(cycle(Some(a), &[], 1), None, "nobody standing");
    }

    #[test]
    fn a_fallen_ally_hands_the_camera_on() {
        let mut world = World::new();
        let [a, b, c] = [(); 3].map(|_| world.spawn_empty().id());
        assert_eq!(keep(Some(b), &[a, b, c]), Some(b), "still up");
        assert_eq!(keep(Some(b), &[a, c]), Some(a), "b fell");
        assert_eq!(keep(Some(b), &[]), None, "everyone fell");
        assert_eq!(keep(None, &[a]), None, "not watching");
    }
}
