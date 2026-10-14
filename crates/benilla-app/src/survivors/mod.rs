//! Warcraft Survivors: a top-down survival mode on the benilla world, started by
//! [`crate::run_survivors`]. There is no server: the hero, the enemies and the spells are local
//! entities dressed by the same attach, animation and spell-visual systems a streamed unit gets,
//! on a battleground picked at the menu ([`data::MAPS`]), with the player controller and FrameXML
//! off ([`crate::run_mode::CaptureMode`]). The local hero walks through the world's collision
//! ([`body`]) and, by default, under WoW's own camera and controls ([`wowcam`], [`view`]).

mod abilities;
mod achievements;
mod armory;
mod auto;
mod body;
mod bonus;
mod cards;
mod charpanel;
mod data;
mod enemies;
mod gear;
mod grade;
mod hand;
mod hero;
mod hud;
mod jewelry;
mod menu;
mod meta;
mod minimap;
mod mobility;
mod party;
mod profile;
mod quests;
mod racials;
mod rewards;
mod run_options;
mod talents;
mod treasure;
mod units;
mod view;
mod wowcam;
mod zones;

use benilla_world::schedule::WorldStage;
use bevy::prelude::*;

use bonus::{Bonuses, School, StatKey, ARMOR_CAP};
use data::{AbilityDef, ClassDef, Stat};
use party::{HeroState, LocalHero};

/// Set by [`crate::run_survivors`] before the app builds. wasm32 has no process environment, so
/// the flag lives here; `WOW_SURVIVORS` (`?survivors=1` / `__wenilla_env.survivors` on the web)
/// still turns it on from outside.
pub(crate) static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn active() -> bool {
    ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
        || crate::webenv::var("WOW_SURVIVORS").is_some()
}

pub(crate) struct SurvivorsPlugin;

impl Plugin for SurvivorsPlugin {
    fn build(&self, app: &mut App) {
        let party = party::Party::from_env();
        // Present before `OnEnter(InWorld)`, where the UI lifecycle decides not to load FrameXML.
        app.insert_resource(crate::run_mode::CaptureMode)
            .add_plugins(grade::plugin)
            .add_plugins(armory::plugin)
            .add_plugins(run_options::plugin)
            .add_plugins(talents::plugin)
            .add_plugins(gear::plugin)
            .add_plugins(achievements::plugin)
            .add_plugins(rewards::plugin)
            .add_plugins(quests::plugin)
            .add_plugins(meta::plugin)
            .add_plugins(zones::plugin)
            .add_plugins(racials::plugin)
            .add_plugins(jewelry::plugin)
            .add_plugins(charpanel::plugin)
            .add_plugins(menu::plugin)
            .add_plugins(hud::notes_plugin)
            .init_state::<Phase>()
            .init_resource::<Rng>()
            .init_resource::<Selection>()
            .init_resource::<Run>()
            .insert_resource(party)
            .init_resource::<profile::LastRunNews>()
            .init_resource::<MetaBonuses>()
            .init_resource::<hero::CamRig>()
            .init_resource::<hero::AimPoint>()
            .init_resource::<view::View>()
            .init_resource::<wowcam::WowFrame>()
            .init_resource::<abilities::PendingHits>()
            .init_resource::<enemies::Spawner>()
            .init_resource::<hud::Offer>()
            .init_resource::<cards::PendingPick>()
            .init_resource::<hand::HandMode>()
            .add_message::<abilities::Hit>()
            .add_message::<enemies::Loot>()
            .add_message::<profile::RunEnded>()
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<hud::SweepUiMaterial>::default())
            .add_systems(Startup, (profile::load_profile, hud::setup_hud).chain())
            .add_systems(Startup, view::load_view)
            .add_systems(Update, hero::daylight)
            .add_systems(PostUpdate, profile::save_if_dirty)
            // The HUD's hover reaches the world-mouse latch as UI, so a click on it starts no
            // mouselook.
            .add_systems(
                Update,
                wowcam::hud_takes_pointer
                    .in_set(WorldStage::Input)
                    .before(crate::player::camera::latch_world_mouse),
            )
            // After the latch: the look session reads which buttons the world owns.
            .add_systems(
                Update,
                (
                    view::toggle_view,
                    hero::aim_cursor,
                    wowcam::wow_look,
                    mobility::cast_mobility,
                    hero::drive_hero,
                    hero::place_camera,
                    wowcam::wow_camera,
                )
                    .chain()
                    .in_set(WorldStage::Input)
                    .after(crate::player::camera::latch_world_mouse),
            )
            .add_systems(
                Update,
                (
                    enemies::spawn_enemies,
                    enemies::move_enemies,
                    abilities::cast_abilities,
                    abilities::tick_areas,
                    abilities::apply_hits,
                    enemies::tick_marks,
                    enemies::drop_loot,
                    enemies::collect_gems,
                    hero::hero_vitals,
                )
                    .chain()
                    .after(WorldStage::Input)
                    .run_if(in_state(Phase::Playing))
                    // Entering a battleground raises its loading screen: the run waits behind it.
                    .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            )
            .add_systems(
                Update,
                (
                    (treasure::place_guardians, treasure::spawn_elites)
                        .after(enemies::spawn_enemies)
                        .before(enemies::move_enemies),
                    treasure::open_caches
                        .after(enemies::collect_gems)
                        .before(hero::hero_vitals),
                )
                    .run_if(in_state(Phase::Playing))
                    .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            )
            .add_systems(
                Update,
                (enemies::reap_dead, enemies::still_gems).after(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    hand::use_tools
                        .after(cards::choose_card)
                        .before(cards::animate_cards),
                    (hand::refresh_tools, treasure::refresh_gold)
                        .after(hud::refresh_hud)
                        .before(hud::show_panels),
                ),
            )
            .add_systems(
                Update,
                (
                    hero::follow_map,
                    hud::menu_preview,
                    hud::buttons,
                    // The Armory and the Run Options panel, either open over the menu, take its
                    // keys and clicks.
                    hud::keys.run_if(armory::closed).run_if(run_options::closed),
                    hud::refresh_menu,
                    menu::restore_pick,
                    menu::build_menu,
                    menu::menu_clicks
                        .run_if(armory::closed)
                        .run_if(run_options::closed),
                    menu::refresh_menu_ui.before(crate::glue::GlueVisuals),
                    cards::deal_cards,
                    cards::choose_card,
                    cards::animate_cards,
                    hud::refresh_hud,
                    hud::refresh_results,
                    hud::refresh_notes,
                    hud::fill_icons,
                    hud::update_cooldown_sweeps,
                    hud::refresh_mobility,
                    hud::show_panels,
                )
                    .chain()
                    .after(WorldStage::Input),
            )
            .add_systems(
                Update,
                (
                    hud::apply_ui_scale,
                    hud::fill_art,
                    hud::xp_hover,
                    hud::icon_tooltip,
                    hud::hero_plate,
                    minimap::map_buttons,
                    minimap::zone_text,
                    minimap::compose,
                )
                    .chain()
                    .after(hud::show_panels),
            )
            .add_systems(OnEnter(Phase::Playing), unpause)
            .add_systems(OnEnter(Phase::LevelUp), (pause, hud::roll_offer))
            .add_systems(OnEnter(Phase::Paused), pause)
            .add_systems(
                OnEnter(Phase::GameOver),
                (unpause, profile::record_finished).chain(),
            )
            .add_systems(OnEnter(Phase::Menu), (unpause, reset_run));
        if let Some(auto) = auto::from_env() {
            app.insert_resource(auto)
                .add_systems(
                    Update,
                    auto::drive.after(hud::keys).before(hud::refresh_menu),
                )
                .add_systems(Update, auto::plan.before(WorldStage::Input));
        }
        // The test aid's extra heroes join each run once it is under way.
        if party.size > 1 {
            app.add_systems(
                Update,
                party::fill_party
                    .after(WorldStage::Input)
                    .run_if(in_state(Phase::Playing))
                    .run_if(|l: Res<crate::loading_screen::LoadingScreen>| !l.covering()),
            );
        }
    }
}

/// The screen the mode is on.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) enum Phase {
    #[default]
    Menu,
    Playing,
    LevelUp,
    Paused,
    GameOver,
    /// The jewelry choice a cache or a quest turn-in opens, paused ([`jewelry`]).
    Jewelry,
    /// The Character panel over the paused run, opened by C ([`charpanel`]).
    Character,
}

/// Everything spawned for a run, despawned when the next begins.
#[derive(Component)]
pub(super) struct RunEntity;

/// The class, look and battleground picked at the menu; the battleground holds for the run.
#[derive(Resource, Default)]
pub(super) struct Selection {
    pub class: Option<usize>,
    /// `(display, race, sex)`, one per race and sex the class's trainers come in.
    pub looks: Vec<(u32, u8, u8)>,
    pub look: Option<usize>,
    /// An index into [`data::MAPS`].
    pub map: usize,
    /// The battleground the world is standing in: the last one entered (or the saved pick at
    /// boot). Picking another at the menu changes only `map`, so the menu hero stays put and no
    /// loading screen rises until Enter World.
    pub stage: usize,
    /// Bumped on every change, so the menu rebuilds once.
    pub version: u32,
}

impl Selection {
    pub fn class_def(&self) -> Option<&'static ClassDef> {
        self.class.map(|i| &data::CLASSES[i])
    }

    pub fn map_def(&self) -> &'static data::MapDef {
        &data::MAPS[self.map.min(data::MAPS.len() - 1)]
    }

    pub fn stage_def(&self) -> &'static data::MapDef {
        &data::MAPS[self.stage.min(data::MAPS.len() - 1)]
    }
}

/// An ability the hero has, with its own cooldown.
pub(super) struct Owned {
    pub def: &'static AbilityDef,
    pub level: u8,
    pub cd: f32,
}

impl Owned {
    pub fn damage_mult(&self) -> f32 {
        1.0 + 0.3 * f32::from(self.level - 1)
    }
    pub fn cooldown(&self, stats: &Stats) -> f32 {
        let base = self.def.cooldown * 0.93f32.powi(i32::from(self.level) - 1);
        (base * (1.0 - stats.haste).max(0.35)).max(0.2)
    }
    /// Targets, projectiles or jumps, whichever the kind counts; the hero's extra targets add on.
    pub fn count(&self, stats: &Stats) -> u32 {
        let extra = [3u8, 5, 7].iter().filter(|&&l| self.level >= l).count() as u32;
        self.def.count + extra + stats.targets
    }
    pub fn radius(&self, stats: &Stats) -> f32 {
        self.def.radius * (1.0 + 0.08 * f32::from(self.level - 1)) * stats.area
    }
    pub fn duration(&self) -> f32 {
        self.def.duration * (1.0 + 0.1 * f32::from(self.level - 1))
    }
}

/// A hero's derived numbers: the base, the class, the level-up passives and the run's
/// [`Bonuses`] ([`HeroState::stats`]). Within a stat a passive and a bonus add: +10% and +10% make
/// +20%, not +21%. Flat bonuses (health, regen, targets, revives) land on the base before any
/// percentage scales it.
#[derive(Clone, Copy)]
pub(super) struct Stats {
    /// The damage multiplier.
    pub might: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub regen: f32,
    /// Fraction of cooldowns removed; [`Owned::cooldown`] keeps at least 35% of one.
    pub haste: f32,
    pub area: f32,
    pub magnet: f32,
    pub growth: f32,
    /// Damage blocked per hit, flat, from the passive.
    pub armor: f32,
    /// Fraction of each hit removed, from bonuses; see [`Stats::after_armor`] for the cap.
    pub armor_frac: f32,
    pub crit: f32,
    /// What a critical strike multiplies damage by.
    pub crit_damage: f32,
    /// Extra targets, projectiles or jumps for every ability that counts them.
    pub targets: u32,
    /// The gold multiplier on every coin's worth ([`treasure::purse`], [`treasure::cache_gold`]).
    pub gold: f32,
    /// The luck multiplier: each rarity tier up weighs this much more in level-up offers.
    pub luck: f32,
    /// Fraction of damage dealt returned as health.
    pub lifesteal: f32,
    /// Extra lives for the run.
    pub revives: u32,
}

pub(super) const BASE_HP: f32 = 120.0;
pub(super) const BASE_SPEED: f32 = 7.0;
pub(super) const BASE_MAGNET: f32 = 4.5;
/// A critical strike's multiplier before bonuses.
pub(super) const BASE_CRIT_DAMAGE: f32 = 2.0;
/// The share of max health an extra life comes back with.
pub(super) const REVIVE_HP: f32 = 0.5;
/// Seconds an extra life keeps the hero untouchable.
pub(super) const REVIVE_IMMUNE_SECS: f32 = 2.0;

impl Stats {
    /// What gets through of a `raw` hit: the flat block and the fraction together remove at most
    /// [`ARMOR_CAP`] of it, and at least 1 always lands.
    pub fn after_armor(&self, raw: f32) -> f32 {
        let blocked = (self.armor + raw * self.armor_frac).min(raw * ARMOR_CAP);
        (raw - blocked).max(1.0)
    }

    /// Health after lifesteal returns its share of `dealt`, never above max health.
    pub fn leech(&self, hp: f32, dealt: f32) -> f32 {
        if self.lifesteal <= 0.0 || dealt <= 0.0 {
            return hp;
        }
        (hp + dealt * self.lifesteal).min(self.max_hp).max(hp)
    }

    /// How much more an offer of rarity `tier` (0 common … 4 legendary) weighs under luck.
    pub fn luck_weight(&self, tier: i32) -> f32 {
        self.luck.max(0.0).powi(tier)
    }
}

/// Bonuses from outside the run (talents, gear and perks, as they land), copied into each run
/// as it starts. Written only by [`meta`], as the sum of its [`meta::MetaParts`].
#[derive(Resource, Default)]
struct MetaBonuses(pub Bonuses);

/// `WOW_SURVIVORS_BONUSES`, a test aid: `damage=0.5,crit_chance=0.2,fire=0.3,targets=1` adds
/// those bonuses to every run ([`bonus::parse`] reads it); a malformed entry is skipped.
fn env_bonuses() -> Bonuses {
    let Some(text) = crate::webenv::var("WOW_SURVIVORS_BONUSES") else {
        return Bonuses::default();
    };
    let (bonuses, skipped) = bonus::parse(&text);
    for entry in skipped {
        warn!("survivors: WOW_SURVIVORS_BONUSES: skipping {entry:?} (want stat_or_school=number)");
    }
    bonuses
}

/// A whole count from a bonus total: never negative, rounded off float dust.
fn whole(v: f32) -> u32 {
    v.max(0.0).round() as u32
}

/// What the party shares in one run: the clock, the horde, the XP bar and level, the spoils and
/// the announcements. Each hero's own share is its [`HeroState`].
#[derive(Resource)]
pub(super) struct Run {
    /// The clock: one horde, one run.
    pub elapsed: f32,
    pub kills: u32,
    /// The XP bar and level: every hero's pickups fill the one bar, and the party levels together.
    pub level: u32,
    pub xp: f32,
    /// Hands banked by level-ups and caches, not yet dealt: [`party::deal_hands`] gives every hero
    /// one pick for each as the picker opens.
    pub pending_levels: u32,
    /// The horde's schedule: bosses and closing rings come on the shared clock.
    pub bosses: u32,
    pub rings: u32,
    pub won: bool,
    /// A GameOver transition records this run once, even if a later transition repeats it.
    pub recorded: bool,
    /// Serials for every unit and object the run spawns, whoever's they are.
    pub next_guid: u64,
    /// Damage dealt by the whole party; each hero's share is [`HeroState::damage_done`].
    pub damage_done: f64,
    /// Bumped whenever the local hero's abilities or passives change, so the HUD rebuilds the icon
    /// rows once.
    pub version: u32,
    /// A centre-screen announcement and its seconds left.
    pub banner: Option<(String, f32)>,
    /// Gold picked up this run ([`treasure`]): one purse for the party.
    pub gold: u32,
    /// Elites and guardians slain, and treasure caches opened.
    pub elites: u32,
    pub caches: u32,
    /// When the next elite walks out of the horde, in run seconds.
    pub next_elite: f32,
    /// Whether this run's guardians stand at their posts yet.
    pub guarded: bool,
    /// The Run Options rewards multiplier the run began under, for whatever pays out.
    pub reward_mult: f32,
    /// In-run quests turned in ([`quests`]).
    pub quests: u32,
    /// Whether the zone's last boss fell, for the quest chains' second step.
    pub final_boss: bool,
    /// Jewelry choices banked by caches and quest turn-ins, not yet dealt: [`jewelry::deal`] gives
    /// every hero one for each as the picker opens.
    pub pending_jewelry: u32,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            kills: 0,
            level: 1,
            xp: 0.0,
            pending_levels: 0,
            bosses: 0,
            rings: 0,
            won: false,
            recorded: false,
            next_guid: 1,
            damage_done: 0.0,
            version: 0,
            banner: None,
            gold: 0,
            elites: 0,
            caches: 0,
            next_elite: treasure::ELITE_FIRST_SECS,
            guarded: false,
            reward_mult: 1.0,
            quests: 0,
            final_boss: false,
            pending_jewelry: 0,
        }
    }
}

impl HeroState {
    /// The base, the class, the passives and the bonuses folded together (see [`Stats`]).
    pub fn stats(&self) -> Stats {
        let b = &self.bonuses;
        let mut s = Stats {
            might: 1.0 + b.get(StatKey::Damage),
            max_hp: BASE_HP + b.get(StatKey::MaxHealth),
            speed: BASE_SPEED,
            regen: b.get(StatKey::Regen),
            haste: b.cooldown(),
            area: 1.0 + b.get(StatKey::Area),
            magnet: BASE_MAGNET,
            growth: 1.0 + b.get(StatKey::Experience),
            armor: 0.0,
            armor_frac: b.armor(),
            crit: 0.05 + b.get(StatKey::CritChance),
            crit_damage: BASE_CRIT_DAMAGE + b.get(StatKey::CritDamage),
            targets: whole(b.get(StatKey::Targets)),
            gold: 1.0 + b.get(StatKey::Gold),
            luck: 1.0 + b.get(StatKey::Luck),
            lifesteal: b.get(StatKey::Lifesteal).max(0.0),
            revives: whole(b.get(StatKey::Revives)),
        };
        // Percentages that scale a base sum before they apply, so a passive and a bonus add.
        let mut hp_pct = 0.0;
        let mut speed_pct = b.get(StatKey::MoveSpeed);
        let mut magnet_pct = b.get(StatKey::Pickup);
        // Tanky classes start tougher, cloth starts quicker.
        match self.class.id {
            1 | 2 => s.max_hp += 40.0,
            7 | 11 | 3 | 4 => s.max_hp += 15.0,
            _ => s.speed += 0.5,
        }
        for &(p, level) in &self.passives {
            let v = p.per_level * f32::from(level);
            match p.stat {
                Stat::Might => s.might += v,
                Stat::MaxHp => hp_pct += v,
                Stat::Speed => speed_pct += v,
                Stat::Regen => s.regen += v,
                Stat::Haste => s.haste += v,
                Stat::Area => s.area += v,
                Stat::Magnet => magnet_pct += v,
                Stat::Growth => s.growth += v,
                Stat::Armor => s.armor += v,
                Stat::Crit => s.crit += v,
            }
        }
        s.max_hp *= 1.0 + hp_pct;
        s.speed *= 1.0 + speed_pct;
        s.magnet *= 1.0 + magnet_pct;
        s
    }

    /// The multiplier on top of might for damage of `school`.
    fn school_mult(&self, school: School) -> f32 {
        (1.0 + self.bonuses.school_damage(school)).max(0.0)
    }

    /// On lethal damage: spends an extra life if one is left, back up at [`REVIVE_HP`] of max
    /// health and untouchable for [`REVIVE_IMMUNE_SECS`]. Whether the hero got up.
    pub fn revive(&mut self) -> bool {
        let stats = self.stats();
        if self.revives_used >= stats.revives {
            return false;
        }
        self.revives_used += 1;
        self.hp = stats.max_hp * REVIVE_HP;
        self.immune_t = self.immune_t.max(REVIVE_IMMUNE_SECS);
        true
    }
}

impl Run {
    pub fn guid(&mut self) -> u64 {
        self.next_guid += 1;
        self.next_guid
    }
}

/// xorshift64*: the mode's dice, seeded from the clock.
#[derive(Resource)]
pub(super) struct Rng(u64);

impl Default for Rng {
    fn default() -> Self {
        let seed = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self(seed | 1)
    }
}

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in `[0, 1)`.
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
    pub fn index(&mut self, len: usize) -> usize {
        (self.next_u64() % len.max(1) as u64) as usize
    }
}

fn pause(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn unpause(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

/// Back at the menu: everything the last run spawned goes, and the run starts over.
fn reset_run(
    mut commands: Commands,
    spawned: Query<Entity, With<RunEntity>>,
    mut run: ResMut<Run>,
    mut pending: ResMut<abilities::PendingHits>,
    mut spawner: ResMut<enemies::Spawner>,
    mut selection: ResMut<Selection>,
) {
    for e in &spawned {
        commands.entity(e).despawn();
    }
    // Guids stay unique across runs: caches keyed by guid may outlive a despawn by a frame.
    let next_guid = run.next_guid;
    *run = Run {
        next_guid,
        ..Run::default()
    };
    pending.0.clear();
    *spawner = enemies::Spawner::default();
    // The preview hero is respawned for the look still picked.
    selection.version += 1;
}

#[cfg(test)]
mod tests {
    use super::bonus::{Bonus, COOLDOWN_CAP};
    use super::data::PassiveDef;
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn hero_with(bonuses: &[Bonus]) -> HeroState {
        let mut hero = HeroState::default();
        hero.bonuses.add_all(bonuses);
        hero
    }

    fn passive(key: &str) -> &'static PassiveDef {
        data::PASSIVES.iter().find(|p| p.key == key).unwrap()
    }

    fn owned(key: &str, level: u8) -> Owned {
        Owned {
            def: data::ability(key),
            level,
            cd: 0.0,
        }
    }

    #[test]
    fn no_bonuses_leave_the_base() {
        let s = HeroState::default().stats();
        assert!(close(s.might, 1.0) && close(s.area, 1.0) && close(s.growth, 1.0));
        assert!(close(s.haste, 0.0) && close(s.armor_frac, 0.0) && close(s.lifesteal, 0.0));
        assert!(close(s.crit, 0.05) && close(s.crit_damage, BASE_CRIT_DAMAGE));
        assert!(close(s.gold, 1.0) && close(s.luck, 1.0));
        assert_eq!((s.targets, s.revives), (0, 0));
    }

    #[test]
    fn each_stat_key_reaches_its_stat() {
        let base = HeroState::default().stats();
        let one = |stat, v| hero_with(&[Bonus::new(stat, v)]).stats();
        assert!(close(
            one(StatKey::MaxHealth, 30.0).max_hp,
            base.max_hp + 30.0
        ));
        assert!(close(one(StatKey::Damage, 0.5).might, 1.5));
        assert!(close(one(StatKey::Cooldown, 0.2).haste, 0.2));
        assert!(close(one(StatKey::CritChance, 0.2).crit, 0.25));
        assert!(close(one(StatKey::CritDamage, 0.5).crit_damage, 2.5));
        assert!(close(one(StatKey::Area, 0.3).area, 1.3));
        assert_eq!(one(StatKey::Targets, 2.0).targets, 2);
        assert!(close(one(StatKey::MoveSpeed, 0.1).speed, base.speed * 1.1));
        assert!(close(one(StatKey::Regen, 1.5).regen, 1.5));
        assert!(close(one(StatKey::Pickup, 0.5).magnet, BASE_MAGNET * 1.5));
        assert!(close(one(StatKey::Armor, 0.2).armor_frac, 0.2));
        assert!(close(one(StatKey::Experience, 0.25).growth, 1.25));
        assert!(close(one(StatKey::Gold, 0.4).gold, 1.4));
        assert!(close(one(StatKey::Luck, 0.5).luck, 1.5));
        assert!(close(one(StatKey::Lifesteal, 0.05).lifesteal, 0.05));
        assert_eq!(one(StatKey::Revives, 1.0).revives, 1);
        // School damage stays off the unscoped multiplier.
        let fire = hero_with(&[Bonus::school(School::Fire, 0.3)]).stats();
        assert!(close(fire.might, 1.0));
    }

    #[test]
    fn passives_and_bonuses_add_within_a_stat() {
        let mut hero = hero_with(&[
            Bonus::new(StatKey::MoveSpeed, 0.1),
            Bonus::new(StatKey::Damage, 0.1),
            Bonus::new(StatKey::MaxHealth, 20.0),
        ]);
        hero.passives = vec![
            (passive("pack"), 1),
            (passive("might"), 1),
            (passive("fortitude"), 1),
        ];
        let base = HeroState::default().stats();
        let s = hero.stats();
        assert!(close(s.speed, base.speed * 1.2));
        assert!(close(s.might, 1.2));
        // Flat health lands before the percentage scales it.
        assert!(close(s.max_hp, (base.max_hp + 20.0) * 1.2));
    }

    #[test]
    fn armor_is_capped_with_the_flat_block_counted() {
        let s = hero_with(&[Bonus::new(StatKey::Armor, 0.9)]).stats();
        assert_eq!(s.armor_frac, ARMOR_CAP);
        assert!(close(s.after_armor(100.0), 40.0));
        let mut hero = hero_with(&[Bonus::new(StatKey::Armor, 0.5)]);
        hero.passives = vec![(passive("wild"), 5)];
        let s = hero.stats();
        assert!(close(s.armor, 5.0));
        // 5 flat and half of 100 remove 55.
        assert!(close(s.after_armor(100.0), 45.0));
        // 5 flat and half of 20 would remove 15; the cap allows 12.
        assert!(close(s.after_armor(20.0), 8.0));
        assert!(close(s.after_armor(1.5), 1.0));
        // The flat block alone, under the cap, is unchanged.
        hero.bonuses = Bonuses::default();
        assert!(close(hero.stats().after_armor(20.0), 15.0));
    }

    #[test]
    fn cooldown_bonus_is_capped_and_stacks_with_haste() {
        let s = hero_with(&[Bonus::new(StatKey::Cooldown, 0.8)]).stats();
        assert_eq!(s.haste, COOLDOWN_CAP);
        let mut hero = hero_with(&[Bonus::new(StatKey::Cooldown, 0.8)]);
        hero.passives = vec![(passive("intellect"), 5)];
        let s = hero.stats();
        assert!(close(s.haste, COOLDOWN_CAP + 0.4));
        // The ability floor still keeps 35% of a cooldown.
        let o = owned("frostbolt", 1);
        assert!(close(o.cooldown(&s), o.def.cooldown * 0.35));
    }

    #[test]
    fn targets_add_to_ability_counts() {
        let s = hero_with(&[Bonus::new(StatKey::Targets, 2.0)]).stats();
        // Three missiles, one more at rank 3, two from the bonus.
        assert_eq!(owned("arcane_missiles", 3).count(&s), 6);
        assert_eq!(
            owned("arcane_missiles", 3).count(&HeroState::default().stats()),
            4
        );
    }

    #[test]
    fn school_damage_multiplies_its_school_on_top_of_might() {
        let hero = hero_with(&[
            Bonus::school(School::Fire, 0.3),
            Bonus::new(StatKey::Damage, 0.5),
        ]);
        assert!(close(hero.school_mult(School::Fire), 1.3));
        assert!(close(hero.school_mult(School::Frost), 1.0));
        assert!(close(
            hero.stats().might * hero.school_mult(School::Fire),
            1.95
        ));
    }

    #[test]
    fn lifesteal_heals_its_share_up_to_max_health() {
        let mut s = hero_with(&[Bonus::new(StatKey::Lifesteal, 0.1)]).stats();
        let max = s.max_hp;
        assert!(close(s.leech(50.0, 100.0), 60.0));
        assert!(close(s.leech(max - 3.0, 100.0), max));
        assert!(close(s.leech(50.0, 0.0), 50.0));
        s.lifesteal = 0.0;
        assert!(close(s.leech(50.0, 100.0), 50.0));
    }

    #[test]
    fn revives_are_spent_one_at_a_time() {
        let mut hero = hero_with(&[Bonus::new(StatKey::Revives, 1.0)]);
        hero.hp = -5.0;
        assert!(hero.revive());
        assert_eq!(hero.revives_used, 1);
        assert!(close(hero.hp, hero.stats().max_hp * REVIVE_HP));
        assert!(close(hero.immune_t, REVIVE_IMMUNE_SECS));
        // Untouchable while the revive's immunity lasts.
        assert_eq!(hero::hurt_hero(&mut hero, 500.0), 0.0);
        hero.hp = 0.0;
        assert!(!hero.revive());
        assert_eq!(hero.revives_used, 1);
        assert!(!HeroState::default().revive());
    }

    #[test]
    fn a_party_of_one_starts_as_the_run_always_did() {
        // No `WOW_SURVIVORS_PARTY`: one hero, the local player's.
        assert_eq!(party::Party::default().size, 1);
        let class = &data::CLASSES[0];
        let mut selection = Selection {
            class: Some(0),
            looks: vec![(0, 1, 0)],
            look: Some(0),
            ..Default::default()
        };
        let mut meta = MetaBonuses::default();
        meta.0.add(&Bonus::new(StatKey::MaxHealth, 20.0));
        let mut run = Run {
            next_guid: 41,
            kills: 9,
            ..Run::default()
        };
        let mut hero = HeroState::default();
        assert!(hud::start_run(
            &mut run,
            Some(&mut hero),
            &mut selection,
            &meta
        ));
        // The shared part starts over, keeping its serials.
        assert_eq!(
            (run.next_guid, run.kills, run.level, run.version),
            (41, 0, 1, 1)
        );
        // The hero's part: the menu's bonuses on a warrior's 40 extra health, the class's first
        // spell, full health and a fresh hand.
        assert_eq!(class.id, 1);
        let s = hero.stats();
        assert_eq!(
            (s.max_hp, s.speed, s.magnet, s.might),
            (BASE_HP + 20.0 + 40.0, BASE_SPEED, BASE_MAGNET, 1.0)
        );
        assert_eq!(hero.hp, s.max_hp);
        assert_eq!((hero.player, hero.race), (party::LOCAL_PLAYER, 1));
        assert_eq!(hero.abilities.len(), 1);
        assert_eq!(hero.abilities[0].def.key, class.start);
        assert_eq!(
            (hero.rerolls, hero.banishes, hero.picks),
            (hand::REROLLS_PER_RUN, hand::BANISHES_PER_RUN, 0)
        );
    }

    #[test]
    fn luck_weighs_rarer_tiers_up() {
        let plain = HeroState::default().stats();
        for tier in 0..5 {
            assert_eq!(plain.luck_weight(tier), 1.0);
        }
        let lucky = hero_with(&[Bonus::new(StatKey::Luck, 0.5)]).stats();
        assert_eq!(lucky.luck_weight(0), 1.0);
        assert!(close(lucky.luck_weight(2), 2.25));
        assert!(lucky.luck_weight(4) > lucky.luck_weight(3));
    }
}
