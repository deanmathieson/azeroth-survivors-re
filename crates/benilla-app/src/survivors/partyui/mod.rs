//! The party's WoW UI, shown once a run has two heroes or more (a lone hero's screen is as it
//! was): the party frames down the left ([`frames`]), the allies' nameplates over their heads and
//! the arrows at the screen's edge to those off it, with a raid warning when one goes down
//! ([`plates`]), the damage meter ([`meter`], which works alone too) and the results' party table
//! and awards ([`mvp`]).
//!
//! Every one of them reads [`PartyView`] and nothing else: a list of the party's members as the
//! screen shows them. Here [`fill_party_view`] fills it from each hero's [`HeroState`]; a client
//! whose heroes arrive over the wire fills it from those instead, in the same
//! [`FillPartyView`] set, and the UI follows unchanged.
//!
//! The art is the 1.12 client's own: `UI-PartyFrame` and its `UI-StatusBar` bars, the class
//! icons of the character creation screen, the nameplate border, the target frame's skull, the
//! minimap's arrow and the `GameTooltip` backdrop; the colours are `GameFontNormal`'s gold and
//! `RAID_CLASS_COLORS`. Missing art leaves the frames' backgrounds and text, never a white box.

mod frames;
mod meter;
mod mvp;
mod plates;

use bevy::prelude::*;

use super::data::{self, ClassDef};
use super::gear::HeroNames;
use super::hero::Hero;
use super::hud::{self, UiArt};
use super::party::{self, Bot, HeroState, LocalHero};
use super::units::Marks;
use super::{Phase, Run, Selection};

/// `PartyMemberFrame`'s art, the bars' fill and the class icons, `CLASS_ICON_TCOORDS`'s sheet.
pub(super) const PARTY_FRAME: &str = "Interface\\TargetingFrame\\UI-PartyFrame";
pub(super) const STATUS_BAR: &str = "Interface\\TargetingFrame\\UI-StatusBar";
pub(super) const CLASS_ICONS: &str =
    "Interface\\Glues\\CharacterCreate\\UI-CharacterCreate-Classes";
/// The skull a nameplate shows in its level's seat, here for a downed ally.
pub(super) const SKULL: &str = "Interface\\TargetingFrame\\UI-TargetingFrame-Skull";
/// The minimap's player arrow, grey so a tint colours it; it points up.
pub(super) const ARROW: &str = "Interface\\Minimap\\MinimapArrow";
/// The `GameTooltip` backdrop the damage meter sits on.
pub(super) const TOOLTIP_BG: &str = "Interface\\Tooltips\\UI-Tooltip-Background";
pub(super) const TOOLTIP_BORDER: &str = "Interface\\Tooltips\\UI-Tooltip-Border";
/// Every texture the party UI draws, for the data check.
#[cfg(test)]
const TEXTURES: [&str; 9] = [
    PARTY_FRAME,
    STATUS_BAR,
    CLASS_ICONS,
    SKULL,
    ARROW,
    TOOLTIP_BG,
    TOOLTIP_BORDER,
    benilla_ui::script::nameplate::BORDER_TEXTURE,
    benilla_ui::script::nameplate::BAR_FILL_TEXTURE,
];

/// The most allies a frame, plate or arrow is kept for: a party of five less the local hero.
/// Kept apart from [`party::MAX_PARTY`] so the UI holds the largest party co-op allows; it never
/// shows more than the party has.
pub(super) const MAX_ALLIES: usize = 4;

/// `GameFontDisable`'s grey, for a downed member's name.
pub(super) const DISABLED: Color = Color::srgb(0.5, 0.5, 0.5);
/// `GameFontRed`.
pub(super) const RED: Color = Color::srgb(1.0, 0.1, 0.1);
/// A health bar's `BarColor`: unit frames never shade it (`HealthBar_OnValueChanged` without
/// `smooth`).
pub(super) const HEALTH_GREEN: Color = Color::srgb(0.0, 1.0, 0.0);
/// The casting bar's `StatusBarColor`: a revive fills like a cast.
pub(super) const REVIVE_YELLOW: Color = Color::srgb(1.0, 0.7, 0.0);
/// The HUD's own shield overlay.
pub(super) const SHIELD_TINT: Color = Color::srgba(0.85, 0.9, 1.0, 0.75);
/// A status bar's empty track.
pub(super) const TRACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);

/// One member of the party as the screen shows it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Member {
    /// The hero's entity: where its body stands, and the key the warnings follow it by.
    pub entity: Entity,
    /// Its player slot, which orders the frames and the tables.
    pub player: u8,
    pub name: String,
    /// `ChrClasses` and `ChrRaces` ids.
    pub class: u8,
    pub race: u8,
    pub level: u32,
    pub hp: f32,
    pub max_hp: f32,
    /// What its shield soaks still; 0 for none.
    pub shield: f32,
    pub downed: bool,
    /// How far an ally has got it up, 0 to 1, while it is down.
    pub revive: f32,
    /// The spells of the buffs on it, in their slots' order.
    pub buffs: Vec<u32>,
    /// Its feet, in the world.
    pub position: Vec3,
    /// This machine's hero.
    pub is_local: bool,
    /// Picking its cards: a level-up hand open on its own screen.
    pub choosing: bool,
    pub damage_done: f64,
    pub healing_done: f64,
    pub damage_taken: f64,
    /// Allies it got up, gold it picked up, and how often it went down.
    pub revives: u32,
    pub gold: u32,
    pub downs: u32,
}

impl Member {
    /// A hero's member, its name the player's slot ("Player 2") and its level 1 until the caller
    /// knows better; `bot` for a hero no one plays, which is never seen choosing.
    pub(super) fn from_hero(entity: Entity, hero: &HeroState, local: bool, bot: bool) -> Self {
        let down = !hero.alive();
        Self {
            entity,
            player: hero.player,
            name: default_name(hero.player),
            class: hero.class.id,
            race: hero.race,
            level: 1,
            hp: hero.hp.max(0.0),
            max_hp: hero.stats().max_hp,
            shield: if hero.shield_t > 0.0 {
                hero.shield.max(0.0)
            } else {
                0.0
            },
            downed: down,
            revive: if down {
                hero.revive_t.clamp(0.0, 1.0)
            } else {
                0.0
            },
            buffs: Vec::new(),
            position: Vec3::ZERO,
            is_local: local,
            choosing: party::holds_level_up(hero, bot),
            damage_done: hero.damage_done,
            healing_done: hero.healing_done,
            damage_taken: hero.damage_taken,
            revives: hero.revives_given,
            gold: hero.gold_collected,
            downs: hero.times_downed,
        }
    }

    /// Health as a share of max health, 0 to 1.
    pub(super) fn health(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.hp / self.max_hp).clamp(0.0, 1.0)
        }
    }

    /// The shield as a share of max health, 0 to 1.
    pub(super) fn shield_share(&self) -> f32 {
        if self.max_hp <= 0.0 {
            0.0
        } else {
            (self.shield / self.max_hp).clamp(0.0, 1.0)
        }
    }
}

/// The name a member without one of its own goes by.
pub(super) fn default_name(player: u8) -> String {
    format!("Player {}", u32::from(player) + 1)
}

/// The party as the screen shows it: the one thing the party UI reads.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(super) struct PartyView {
    /// In player order.
    pub members: Vec<Member>,
    /// The run's clock, in seconds, for the per-second numbers.
    pub elapsed: f32,
}

impl PartyView {
    /// A party of two or more: the party UI is up. A lone hero's screen stays as it was.
    pub(super) fn coop(&self) -> bool {
        self.members.len() >= 2
    }

    /// The local hero's member.
    pub(super) fn local(&self) -> Option<&Member> {
        self.members.iter().find(|m| m.is_local)
    }

    /// The allies the party frames, plates and arrows are for, in player order: the party less
    /// the local hero, at most [`MAX_ALLIES`], and none at all for a lone hero.
    pub(super) fn allies(&self) -> Vec<&Member> {
        if !self.coop() {
            return Vec::new();
        }
        self.members
            .iter()
            .filter(|m| !m.is_local)
            .take(MAX_ALLIES)
            .collect()
    }
}

/// Where [`PartyView`] is filled: the UI's systems run after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FillPartyView;

/// Whether the in-run party UI is up in `phase`: over a run (playing, paused, the level-up and
/// the panels over it), never at the menu nor on the results.
pub(super) fn in_run(phase: Phase) -> bool {
    !matches!(phase, Phase::Menu | Phase::GameOver)
}

/// A class by its `ChrClasses` id.
pub(super) fn class_def(id: u8) -> Option<&'static ClassDef> {
    data::CLASSES.iter().find(|c| c.id == id)
}

/// A class's `RAID_CLASS_COLORS` colour; white for one unknown.
pub(super) fn class_color(id: u8) -> Color {
    class_def(id).map_or(Color::WHITE, |c| hud::rgb(c.color))
}

/// A class's icon on the character creation sheet, `CLASS_ICON_TCOORDS`' cell.
pub(super) fn class_icon(id: u8) -> UiArt {
    let tc = crate::glue::art::class_tc(id).unwrap_or([0.0, 0.25, 0.0, 0.25]);
    UiArt::crop(CLASS_ICONS, tc)
}

/// `PartyMemberFrame_UpdateMemberHealth`'s blink, `t` seconds in: up from half to full over half
/// a second, then back down.
pub(super) fn blink(t: f32) -> f32 {
    let c = t.rem_euclid(1.0);
    if c < 0.5 {
        (127.0 + c * 256.0) / 255.0
    } else {
        (255.0 - (c - 0.5) * 256.0) / 255.0
    }
}

/// Fill [`PartyView`] from the heroes' own state: every hero's numbers, the shared level and
/// clock, the local hero's name, and the buffs its aura slots show.
#[allow(clippy::type_complexity)] // one Bevy system's full input set
pub(super) fn fill_party_view(
    run: Res<Run>,
    selection: Res<Selection>,
    names: HeroNames,
    heroes: Query<
        (
            Entity,
            &HeroState,
            &Transform,
            Option<&Marks>,
            Has<LocalHero>,
            Has<Bot>,
        ),
        With<Hero>,
    >,
    mut view: ResMut<PartyView>,
) {
    let own = selection.class.and_then(|c| names.name(c));
    let members = members_of(
        heroes
            .iter()
            .map(|(e, hero, tf, marks, local, bot)| HeroRead {
                entity: e,
                hero,
                position: tf.translation,
                buffs: marks.map(|m| m.spells().collect()).unwrap_or_default(),
                local,
                bot,
            }),
        run.level,
        own,
    );
    let want = PartyView {
        members,
        elapsed: run.elapsed,
    };
    if *view != want {
        *view = want;
    }
}

/// One hero as [`fill_party_view`] reads it.
pub(super) struct HeroRead<'a> {
    pub entity: Entity,
    pub hero: &'a HeroState,
    pub position: Vec3,
    pub buffs: Vec<u32>,
    pub local: bool,
    pub bot: bool,
}

/// The members for `heroes`, in player order: at the party's `level`, the local hero by its own
/// name where it has one, and a downed hero showing no buffs (only its revive).
pub(super) fn members_of<'a>(
    heroes: impl IntoIterator<Item = HeroRead<'a>>,
    level: u32,
    own_name: Option<&str>,
) -> Vec<Member> {
    let mut members: Vec<Member> = heroes
        .into_iter()
        .map(|read| {
            let mut m = Member::from_hero(read.entity, read.hero, read.local, read.bot);
            m.level = level;
            m.position = read.position;
            if !m.downed {
                m.buffs = read.buffs;
            }
            if let (true, Some(name)) = (read.local, own_name) {
                m.name = name.to_string();
            }
            m
        })
        .collect();
    members.sort_by_key(|m| m.player);
    members
}

/// This workstream's systems, registered by [`super::SurvivorsPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.init_resource::<PartyView>()
        .init_resource::<meter::Meter>()
        .init_resource::<plates::RaidWarnings>()
        .add_systems(
            Startup,
            (frames::setup, plates::setup).after(hud::setup_hud),
        )
        .add_systems(
            Update,
            fill_party_view
                .in_set(FillPartyView)
                .after(party::open_banked_picks),
        )
        .add_systems(
            Update,
            (
                frames::refresh,
                plates::refresh_plates,
                plates::refresh_arrows,
                plates::raid_warnings,
                (meter::toggle, meter::cycle, meter::build, meter::refresh).chain(),
                mvp::refresh,
            )
                .after(FillPartyView)
                .after(hud::show_panels),
        );
}

#[cfg(test)]
mod tests {
    use super::super::bonus::Bonuses;
    use super::*;

    fn class(name: &str) -> &'static ClassDef {
        data::CLASSES.iter().find(|c| c.name == name).unwrap()
    }

    pub(super) fn member(player: u8, class_name: &str) -> Member {
        let hero = HeroState::new(player, class(class_name), 2, Bonuses::default());
        let e = Entity::from_raw_u32(u32::from(player) + 1).unwrap();
        Member::from_hero(e, &hero, player == 0, false)
    }

    pub(super) fn view_of(members: Vec<Member>) -> PartyView {
        PartyView {
            members,
            elapsed: 60.0,
        }
    }

    #[test]
    fn the_class_colours_are_raid_class_colors() {
        for (name, hex) in [
            ("Warrior", 0xC79C6E),
            ("Paladin", 0xF58CBA),
            ("Hunter", 0xABD473),
            ("Rogue", 0xFFF569),
            ("Priest", 0xFFFFFF),
            ("Shaman", 0x0070DE),
            ("Mage", 0x69CCF0),
            ("Warlock", 0x9482C9),
            ("Druid", 0xFF7D0A),
        ] {
            assert_eq!(class_color(class(name).id), hud::rgb(hex), "{name}");
            assert!(
                crate::glue::art::class_tc(class(name).id).is_some(),
                "{name}"
            );
        }
        assert_eq!(class_color(42), Color::WHITE);
    }

    #[test]
    fn the_view_is_each_hero_in_player_order() {
        let mut me = HeroState::new(0, class("Priest"), 5, Bonuses::default());
        (me.damage_done, me.healing_done, me.damage_taken) = (1200.0, 340.0, 90.0);
        (me.revives_given, me.gold_collected, me.times_downed) = (2, 55, 1);
        me.shield = 30.0;
        me.shield_t = 4.0;
        me.hp = 40.0;
        me.picks = 1;
        let mut ally = HeroState::new(1, class("Mage"), 7, Bonuses::default());
        ally.dead_t = Some(1.0);
        ally.hp = 0.0;
        ally.revive_t = 0.4;
        // A spent shield is no shield.
        ally.shield = 25.0;
        let mut bot = HeroState::new(2, class("Rogue"), 8, Bonuses::default());
        bot.picks = 3;
        let e = |i: u32| Entity::from_raw_u32(i).unwrap();
        let read = |entity, hero, local, bot, buffs: &[u32]| HeroRead {
            entity,
            hero,
            position: Vec3::splat(f32::from(u8::from(local))),
            buffs: buffs.to_vec(),
            local,
            bot,
        };
        let members = members_of(
            [
                read(e(9), &bot, false, true, &[]),
                read(e(8), &ally, false, false, &[19752]),
                read(e(7), &me, true, false, &[17, 139]),
            ],
            12,
            Some("Anduin"),
        );
        let players: Vec<u8> = members.iter().map(|m| m.player).collect();
        assert_eq!(players, [0, 1, 2]);
        let [me, ally, bot] = [&members[0], &members[1], &members[2]];
        assert_eq!(
            (me.entity, me.name.as_str(), me.class, me.race),
            (e(7), "Anduin", 5, 5)
        );
        assert!(me.is_local && !ally.is_local && !bot.is_local);
        assert_eq!(me.level, 12);
        assert_eq!(
            me.max_hp,
            HeroState::new(0, class("Priest"), 5, Bonuses::default())
                .stats()
                .max_hp
        );
        assert_eq!((me.hp, me.shield, me.downed), (40.0, 30.0, false));
        assert_eq!(me.buffs, [17, 139]);
        assert_eq!(me.position, Vec3::ONE);
        assert_eq!(
            (me.damage_done, me.healing_done, me.damage_taken),
            (1200.0, 340.0, 90.0)
        );
        assert_eq!((me.revives, me.gold, me.downs), (2, 55, 1));
        assert!(me.choosing, "a player with a hand open");
        // The ally: its slot's name, down with its revive under way and no buffs shown.
        assert_eq!(ally.name, "Player 2");
        assert_eq!((ally.hp, ally.shield, ally.downed), (0.0, 0.0, true));
        assert!((ally.revive - 0.4).abs() < 1e-6);
        assert!(ally.buffs.is_empty());
        assert!(!ally.choosing, "the downed choose nothing");
        // A bot never holds the level-up.
        assert!(!bot.choosing);
        assert_eq!(bot.name, "Player 3");
    }

    #[test]
    fn the_party_ui_is_for_a_party_and_the_frames_for_allies() {
        let solo = view_of(vec![member(0, "Warrior")]);
        assert!(!solo.coop());
        assert!(
            solo.allies().is_empty(),
            "alone, no frames, plates or arrows"
        );
        assert_eq!(solo.local().map(|m| m.player), Some(0));
        let party = view_of(vec![
            member(0, "Warrior"),
            member(1, "Priest"),
            member(2, "Mage"),
            member(3, "Druid"),
        ]);
        assert!(party.coop());
        let allies: Vec<u8> = party.allies().iter().map(|m| m.player).collect();
        assert_eq!(allies, [1, 2, 3], "everyone but the local hero");
        // In a run only: never at the menu nor over the results.
        for phase in [
            Phase::Playing,
            Phase::Paused,
            Phase::LevelUp,
            Phase::Jewelry,
            Phase::Character,
        ] {
            assert!(in_run(phase), "{phase:?}");
        }
        assert!(!in_run(Phase::Menu) && !in_run(Phase::GameOver));
    }

    #[test]
    fn health_and_shield_are_shares_of_max_health() {
        let mut m = member(1, "Warrior");
        (m.hp, m.max_hp, m.shield) = (40.0, 160.0, 400.0);
        assert_eq!(m.health(), 0.25);
        assert_eq!(m.shield_share(), 1.0, "capped at a full bar");
        m.max_hp = 0.0;
        assert_eq!((m.health(), m.shield_share()), (0.0, 0.0));
    }

    #[test]
    fn the_blink_runs_half_to_full_and_back() {
        assert!((blink(0.0) - 127.0 / 255.0).abs() < 1e-6);
        assert!(blink(0.49) > 0.98);
        assert!(blink(0.25) > blink(0.0) && blink(0.75) < blink(0.5));
        assert!((blink(1.0) - blink(0.0)).abs() < 1e-6);
    }

    #[test]
    fn the_ui_art_is_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let chain = benilla_formats::open_chain(&data).expect("open chain");
        let icons = mvp::Award::ALL.map(|a| a.icon());
        for path in TEXTURES.into_iter().chain(icons) {
            let file = format!("{path}.blp");
            assert!(chain.contains(&file), "{file}");
        }
    }

    #[test]
    fn the_raid_warning_sounds_in_the_client_data() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let kits = benilla_formats::load_sound_kit_catalog(&mut chain).expect("sound kits");
        assert!(kits.by_name(plates::RAID_WARNING_SOUND).is_some());
    }
}
