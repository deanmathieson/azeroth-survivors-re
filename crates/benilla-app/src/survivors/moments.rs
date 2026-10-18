//! The run's big moments, on this machine's screen: a spell transforming into its legendary (a
//! hitstop alone, a flash in the legendary orange, a banner and Zul'Gurub's gong; in co-op the
//! allies read a line in the chat instead, [`super::coop`]), a Paragon reached (a gold banner, and
//! the gold aura its hero wears, [`PARAGON_AURA`]), the XP bar flashing gold as a level comes,
//! and a micro-event's named rare in the silver dragon frame of 1.12's rare target
//! ([`RARE_FRAME`]). The world's side of a level, the knockback nova and its golden ring, is the
//! host's ([`super::enemies`]).

use bevy::prelude::*;

use crate::sound::SoundCue;

use super::cards::LEGENDARY;
use super::data::{self, LegendaryDef};
use super::enemies::{Enemy, RareMob};
use super::hud::{self, HudFont, UiArt};
use super::strip::CoopRun;
use super::{HeroState, LocalHero, Run};

/// The world's speed through a transformation's hitstop, and how long it holds, in real seconds.
pub(super) const HITSTOP_SPEED: f32 = 0.05;
const HITSTOP_SECS: f32 = 0.5;
/// A transformation's sound: Zul'Gurub's troll gong.
pub(super) const TRANSFORM_SOUND: &str = "G_GongTroll01";
/// A Paragon reached: the quest-complete fanfare.
pub(super) const PARAGON_SOUND: &str = "QUESTCOMPLETED";
/// Every this many Overcharges taken, in all, a Paragon.
pub(super) const PARAGON_EVERY: u32 = 5;
/// What each Paragon adds to all damage.
pub(super) const PARAGON_DAMAGE: f32 = 0.10;
/// A Paragon's gold aura: Blessing of Protection's golden state.
pub(super) const PARAGON_AURA: u32 = 1022;
/// The golden ring a level throws out round each hero: Holy Nova's.
pub(super) const LEVEL_RING: u32 = 15237;
/// The rare target frame's silver dragon.
pub(super) const RARE_FRAME: &str = "Interface\\TargetingFrame\\UI-TargetingFrame-Rare";
/// The real spells the run's moments play, for the data test ([`super::mechanics`]).
#[cfg(test)]
pub(super) const SPELLS: [(&str, u32); 3] = [
    ("Paragon aura", PARAGON_AURA),
    ("level ring", LEVEL_RING),
    ("shrine blessing", super::enemies::SHRINE_SPELL),
];
/// The sound kits the run's moments ring, for the data test.
#[cfg(test)]
pub(super) const SOUNDS: [&str; 4] = [
    TRANSFORM_SOUND,
    PARAGON_SOUND,
    super::cards::ROLL_RARE_SOUND,
    super::cards::ROLL_EPIC_SOUND,
];

const FLASH_SECS: f32 = 0.6;
const BANNER_SECS: f32 = 3.0;
const XP_FLASH_SECS: f32 = 0.6;
const PARAGON_GOLD: Color = Color::srgb(1.0, 0.82, 0.2);

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Hitstop>()
        .init_resource::<Moment>()
        .add_systems(
            Update,
            (
                spawn_moment_ui,
                legendary_moment,
                paragon_moment,
                xp_flash,
                show_moments,
                show_rare_frame,
            )
                .chain(),
        );
}

/// A hitstop running: the world at [`HITSTOP_SPEED`] for its real seconds left
/// ([`super::strip::slow_mo`] sets the speed).
#[derive(Resource, Default)]
pub(super) struct Hitstop {
    pub left: f32,
}

/// What the screen shows of a moment: a flash over everything and a big banner, each with its
/// colour and the real seconds it has left; and the XP bar's flash.
#[derive(Resource, Default)]
pub(super) struct Moment {
    flash: Option<(Color, f32)>,
    banner: Option<(String, Color, f32)>,
    xp: f32,
}

impl Moment {
    pub(super) fn flash(&mut self, color: Color) {
        self.flash = Some((color, FLASH_SECS));
    }

    pub(super) fn banner(&mut self, text: impl Into<String>, color: Color) {
        self.banner = Some((text.into(), color, BANNER_SECS));
    }
}

#[derive(Component)]
struct FlashNode;
#[derive(Component)]
struct BannerRoot;
#[derive(Component)]
struct BannerLine;
#[derive(Component)]
struct XpFlash;
#[derive(Component)]
struct RareRoot;
#[derive(Component)]
struct RareName;
#[derive(Component)]
struct RareHealth;

/// The abilities `hero` has transformed: each at its top rank, with a legendary.
pub(super) fn transformed(hero: &HeroState) -> Vec<&'static LegendaryDef> {
    hero.abilities
        .iter()
        .filter(|o| o.level >= data::MAX_ABILITY_LEVEL)
        .filter_map(|o| data::legendary(o.def.key))
        .collect()
}

/// The keys of the legendaries in `now` that `before` did not hold.
pub(super) fn fresh<'a>(
    before: &[&'static str],
    now: &'a [&'static LegendaryDef],
) -> Vec<&'a LegendaryDef> {
    now.iter()
        .filter(|l| !before.contains(&l.ability))
        .copied()
        .collect()
}

/// The banner a transformation shows: "<Spell> becomes <Legendary>!".
pub(super) fn transform_line(
    spells: Option<&crate::ui_action::Spells>,
    legend: &LegendaryDef,
) -> String {
    let def = data::ability(legend.ability);
    format!(
        "{} becomes {}!",
        hud::spell_name(spells, def.spell, def.key),
        legend.name
    )
}

/// How many Paragons `hero` has reached: one for every [`PARAGON_EVERY`] Overcharges taken.
pub(super) fn paragons(hero: &HeroState) -> u32 {
    hero.overcharges.iter().sum::<u32>() / PARAGON_EVERY
}

fn spawn_moment_ui(
    mut commands: Commands,
    font: Option<Res<HudFont>>,
    roots: Query<(), With<FlashNode>>,
    bars: Query<(Entity, Option<&Children>), With<hud::XpBar>>,
    flashes: Query<(), With<XpFlash>>,
) {
    let Some(font) = font else { return };
    if roots.is_empty() {
        commands.spawn((
            FlashNode,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(430),
            bevy::ui::FocusPolicy::Pass,
        ));
        commands
            .spawn((
                BannerRoot,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    top: Val::Percent(24.0),
                    justify_content: JustifyContent::Center,
                    display: Display::None,
                    ..default()
                },
                GlobalZIndex(431),
                bevy::ui::FocusPolicy::Pass,
            ))
            .with_children(|b| {
                b.spawn((
                    BannerLine,
                    hud::centered_bundle(&font.0, "", 34.0, LEGENDARY),
                ));
            });
    }
    if roots.is_empty() {
        spawn_rare_frame(&mut commands, &font.0);
    }
    for (bar, children) in &bars {
        let has = children.is_some_and(|c| c.into_iter().any(|&e| flashes.contains(e)));
        if !has {
            commands.entity(bar).with_children(|b| {
                b.spawn((
                    XpFlash,
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    bevy::ui::FocusPolicy::Pass,
                ));
            });
        }
    }
}

/// The rare's frame: 1.12's rare target frame, its silver dragon on the right, with the rare's
/// name and health; hidden until one walks the field.
fn spawn_rare_frame(commands: &mut Commands, font: &Handle<Font>) {
    commands
        .spawn((
            RareRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Px(150.0),
                margin: UiRect::left(Val::Px(-116.0)),
                width: Val::Px(232.0),
                height: Val::Px(100.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(405),
            bevy::ui::FocusPolicy::Pass,
        ))
        .with_children(|f| {
            f.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ImageNode {
                    color: Color::NONE,
                    ..default()
                },
                // `TargetFrameTexture`'s own crop of the 256×128 art.
                UiArt::crop(RARE_FRAME, [0.09375, 1.0, 0.0, 0.781_25]),
            ));
            f.spawn((
                RareName,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(8.0),
                    top: Val::Px(22.0),
                    width: Val::Px(150.0),
                    ..default()
                },
                hud::centered_bundle(font, "", 13.0, Color::WHITE),
            ));
            f.spawn((
                RareHealth,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(8.0),
                    top: Val::Px(44.0),
                    width: Val::Px(150.0),
                    ..default()
                },
                hud::centered_bundle(font, "", 12.0, Color::srgb(0.75, 0.75, 0.8)),
            ));
        });
}

/// A micro-event's named rare while it stands: its frame, name and health.
#[allow(clippy::type_complexity)]
fn show_rare_frame(
    rares: Query<(&Enemy, &RareMob)>,
    mut roots: Query<&mut Node, With<RareRoot>>,
    mut names: Query<&mut Text, (With<RareName>, Without<RareHealth>)>,
    mut healths: Query<&mut Text, (With<RareHealth>, Without<RareName>)>,
) {
    let rare = rares.iter().find(|(e, _)| !e.dead);
    for mut node in &mut roots {
        let display = if rare.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    let Some((enemy, mob)) = rare else {
        return;
    };
    let name = mob.name();
    for mut t in &mut names {
        if t.0 != name {
            t.0 = name.to_string();
        }
    }
    let health = format!(
        "Rare Elite  {}%",
        super::bossbar::percent(enemy.hp, enemy.max_hp)
    );
    for mut t in &mut healths {
        if t.0 != health {
            t.0 = health.clone();
        }
    }
}

/// The local hero's spell transforming: a flash in the legendary orange, the banner, the gong,
/// and alone a hitstop; a hero seen for the first time (a run joined) announces nothing.
fn legendary_moment(
    coop: Res<CoopRun>,
    local: Query<(Entity, &HeroState), With<LocalHero>>,
    spells: Option<Res<crate::ui_action::Spells>>,
    mut seen: Local<(Option<Entity>, Vec<&'static str>)>,
    mut moment: ResMut<Moment>,
    mut hitstop: ResMut<Hitstop>,
    mut cues: MessageWriter<SoundCue>,
) {
    let Ok((e, hero)) = local.single() else {
        *seen = (None, Vec::new());
        return;
    };
    let now = transformed(hero);
    let first = seen.0 != Some(e);
    let new: Vec<&LegendaryDef> = fresh(&seen.1, &now);
    *seen = (Some(e), now.iter().map(|l| l.ability).collect());
    let Some(legend) = new.first().filter(|_| !first) else {
        return;
    };
    moment.flash(LEGENDARY);
    moment.banner(transform_line(spells.as_deref(), legend), LEGENDARY);
    cues.write(SoundCue::flat(TRANSFORM_SOUND));
    // The world stops only for a lone player: a co-op world is the whole party's.
    if !coop.0 {
        hitstop.left = HITSTOP_SECS;
    }
}

/// The local hero reaching a Paragon: the gold banner and fanfare. Its aura is the hero's own
/// ([`super::hero::hero_vitals`]).
fn paragon_moment(
    local: Query<(Entity, &HeroState), With<LocalHero>>,
    mut seen: Local<(Option<Entity>, u32)>,
    mut moment: ResMut<Moment>,
    mut cues: MessageWriter<SoundCue>,
) {
    let Ok((e, hero)) = local.single() else {
        *seen = (None, 0);
        return;
    };
    let n = paragons(hero);
    let reached = seen.0 == Some(e) && n > seen.1;
    *seen = (Some(e), n);
    if reached {
        let pct = (n as f32 * PARAGON_DAMAGE * 100.0).round();
        moment.banner(format!("Paragon {n}! All damage +{pct}%"), PARAGON_GOLD);
        cues.write(SoundCue::flat(PARAGON_SOUND));
    }
}

/// The XP bar flashes gold as the party levels.
fn xp_flash(run: Res<Run>, mut last: Local<Option<u32>>, mut moment: ResMut<Moment>) {
    if last.is_some_and(|l| run.level > l) {
        moment.xp = XP_FLASH_SECS;
    }
    *last = Some(run.level);
}

/// The moments on screen, fading in real time (a hitstop or a pause stops nothing here).
#[allow(clippy::type_complexity)]
fn show_moments(
    time: Res<Time<Real>>,
    mut moment: ResMut<Moment>,
    mut hitstop: ResMut<Hitstop>,
    mut flashes: Query<(&mut Node, &mut BackgroundColor), (With<FlashNode>, Without<XpFlash>)>,
    mut xp: Query<&mut BackgroundColor, (With<XpFlash>, Without<FlashNode>)>,
    mut banners: Query<&mut Node, (With<BannerRoot>, Without<FlashNode>)>,
    mut lines: Query<(&mut Text, &mut TextColor), With<BannerLine>>,
) {
    let dt = time.delta_secs().min(0.1);
    hitstop.left = (hitstop.left - dt).max(0.0);
    let m = &mut *moment;
    if let Some((_, left)) = m.flash.as_mut() {
        *left -= dt;
    }
    if m.flash.is_some_and(|(_, left)| left <= 0.0) {
        m.flash = None;
    }
    if let Some((_, _, left)) = m.banner.as_mut() {
        *left -= dt;
    }
    if m.banner.as_ref().is_some_and(|b| b.2 <= 0.0) {
        m.banner = None;
    }
    m.xp = (m.xp - dt).max(0.0);
    for (mut node, mut bg) in &mut flashes {
        let (display, color) = match m.flash {
            Some((c, left)) => (
                Display::Flex,
                c.with_alpha(0.45 * (left / FLASH_SECS).clamp(0.0, 1.0)),
            ),
            None => (Display::None, Color::NONE),
        };
        if node.display != display {
            node.display = display;
        }
        bg.0 = color;
    }
    for mut bg in &mut xp {
        let want = PARAGON_GOLD.with_alpha(0.85 * (m.xp / XP_FLASH_SECS).clamp(0.0, 1.0));
        if bg.0 != want {
            bg.0 = want;
        }
    }
    for mut node in &mut banners {
        let display = if m.banner.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    if let Some((text, color, left)) = m.banner.as_ref() {
        // Fades over its last second.
        let alpha = left.clamp(0.0, 1.0);
        for (mut t, mut c) in &mut lines {
            if t.0 != *text {
                t.0 = text.clone();
            }
            c.0 = color.with_alpha(alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{data, Owned};
    use super::*;

    #[test]
    fn a_top_rank_with_a_legendary_is_transformed_once() {
        let mut hero = HeroState::default();
        hero.abilities.push(Owned {
            def: data::ability("frostbolt"),
            level: data::MAX_ABILITY_LEVEL - 1,
            cd: 0.0,
        });
        assert!(transformed(&hero).is_empty());
        hero.abilities[0].level = data::MAX_ABILITY_LEVEL;
        let now = transformed(&hero);
        assert_eq!(now.len(), 1);
        assert_eq!(fresh(&[], &now).len(), 1);
        assert!(fresh(&["frostbolt"], &now).is_empty(), "announced once");
        assert_eq!(
            transform_line(None, now[0]),
            "frostbolt becomes Shatterbolt!"
        );
    }

    #[test]
    fn a_paragon_every_five_overcharges() {
        let mut hero = HeroState {
            overcharges: [1, 1, 1, 1, 0, 0],
            ..HeroState::default()
        };
        assert_eq!(paragons(&hero), 0);
        hero.overcharges[5] = 1;
        assert_eq!(paragons(&hero), 1);
        hero.overcharges[0] = 10;
        assert_eq!(paragons(&hero), 2);
    }

    /// The moments' and micro-events' sound kits, the shrine, the courier and the rare frame are
    /// the client's own; skips without client data.
    #[test]
    fn every_moment_sound_model_and_frame_is_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).unwrap();
        let mut wrong = Vec::new();
        let kits = benilla_formats::load_sound_kit_catalog(&mut chain).unwrap();
        for kit in SOUNDS.into_iter().chain([
            super::super::roulette::PRIZE_SOUND,
            super::super::sfx::RAID_WARNING,
        ]) {
            if kits.by_name(kit).is_none_or(|k| k.files.is_empty()) {
                wrong.push(format!("no sound kit {kit}"));
            }
        }
        let objects = benilla_formats::load_gameobject_catalog(&mut chain).unwrap();
        let shrine = objects
            .model_path(super::super::enemies::SHRINE_DISPLAY)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !shrine.contains("shrine") {
            wrong.push(format!("the shrine is {shrine:?}"));
        }
        let creatures = benilla_formats::load_creature_catalog(&mut chain).unwrap();
        let courier = super::super::enemies::COURIER.display;
        match creatures.model(courier) {
            Some(m) if m.model_path.to_ascii_lowercase().contains("goblin") => {}
            m => wrong.push(format!(
                "the courier {courier} is no goblin: {:?}",
                m.map(|m| m.model_path)
            )),
        }
        let frame = format!("{RARE_FRAME}.blp");
        if !chain.contains(&frame) {
            wrong.push(format!("no {frame}"));
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
