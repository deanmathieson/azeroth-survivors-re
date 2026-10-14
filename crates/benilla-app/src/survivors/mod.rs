//! Warcraft Survivors: a top-down survival mode on the benilla world, started by
//! [`crate::run_survivors`]. There is no server: the hero, the enemies and the spells are local
//! entities dressed by the same attach, animation and spell-visual systems a streamed unit gets,
//! on a battleground picked at the menu ([`data::MAPS`]), with the player controller and FrameXML
//! off ([`crate::run_mode::CaptureMode`]).

mod abilities;
mod armory;
mod auto;
mod bonus;
mod cards;
mod data;
mod enemies;
mod grade;
mod hand;
mod hero;
mod hud;
mod menu;
mod minimap;
mod mobility;
mod profile;
mod treasure;
mod units;

use benilla_world::schedule::WorldStage;
use bevy::prelude::*;

use bonus::{Bonuses, School, StatKey, ARMOR_CAP};
use data::{AbilityDef, ClassDef, PassiveDef, Stat};

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
        // Present before `OnEnter(InWorld)`, where the UI lifecycle decides not to load FrameXML.
        app.insert_resource(crate::run_mode::CaptureMode)
            .add_plugins(grade::plugin)
            .add_plugins(armory::plugin)
            .init_state::<Phase>()
            .init_resource::<Rng>()
            .init_resource::<Selection>()
            .init_resource::<Run>()
            .init_resource::<profile::LastRunNews>()
            .init_resource::<MetaBonuses>()
            .init_resource::<hero::CamRig>()
            .init_resource::<hero::AimPoint>()
            .init_resource::<abilities::PendingHits>()
            .init_resource::<enemies::Spawner>()
            .init_resource::<hud::Offer>()
            .init_resource::<cards::PendingPick>()
            .init_resource::<hand::HandMode>()
            .add_message::<abilities::Hit>()
            .add_message::<enemies::Loot>()
            .add_plugins(bevy::ui_render::UiMaterialPlugin::<hud::SweepUiMaterial>::default())
            .add_systems(Startup, (profile::load_profile, hud::setup_hud).chain())
            .add_systems(Update, hero::daylight)
            .add_systems(
                Update,
                (
                    hero::aim_cursor,
                    mobility::cast_mobility,
                    hero::drive_hero,
                    hero::place_camera,
                )
                    .chain()
                    .in_set(WorldStage::Input),
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
                    // The Armory over the menu takes the keys and clicks while it is open.
                    hud::keys.run_if(armory::closed),
                    hud::refresh_menu,
                    menu::restore_pick,
                    menu::build_menu,
                    menu::menu_clicks.run_if(armory::closed),
                    menu::refresh_menu_ui.before(crate::glue::GlueVisuals),
                    cards::deal_cards,
                    cards::choose_card,
                    cards::animate_cards,
                    hud::refresh_hud,
                    hud::refresh_results,
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

/// The hero's derived numbers: the base, the class, the level-up passives and the run's
/// [`Bonuses`] ([`Run::stats`]). Within a stat a passive and a bonus add: +10% and +10% make
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
    /// The gold multiplier. Unread until gold exists: its consumer is the per-coin value.
    #[allow(dead_code)]
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
/// as it starts.
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

/// One run's state.
#[derive(Resource)]
pub(super) struct Run {
    pub class: &'static ClassDef,
    pub race: u8,
    pub elapsed: f32,
    pub kills: u32,
    pub level: u32,
    pub xp: f32,
    pub hp: f32,
    pub abilities: Vec<Owned>,
    pub passives: Vec<(&'static PassiveDef, u8)>,
    /// The permanent bonuses this run started with: [`MetaBonuses`] and the env test aid.
    bonuses: Bonuses,
    /// Extra lives spent so far.
    pub revives_used: u32,
    pub shield: f32,
    pub shield_t: f32,
    pub immune_t: f32,
    pub thorns: f32,
    pub thorns_t: f32,
    pub thorns_spell: u32,
    pub heal_rate: f32,
    pub heal_t: f32,
    /// The class mobility's cooldown and running effect.
    mobile: mobility::MobilityState,
    pub pending_levels: u32,
    pub bosses: u32,
    pub rings: u32,
    pub won: bool,
    /// A GameOver transition records this run once, even if a later transition repeats it.
    pub recorded: bool,
    /// Seconds since the hero fell, while the death plays out.
    pub dead_t: Option<f32>,
    pub next_guid: u64,
    pub damage_done: f64,
    /// Bumped whenever abilities or passives change, so the HUD rebuilds the icon rows once.
    pub version: u32,
    /// A centre-screen announcement and its seconds left.
    pub banner: Option<(String, f32)>,
    /// Gold picked up this run ([`treasure`]).
    pub gold: u32,
    /// Elites and guardians slain, and treasure caches opened.
    pub elites: u32,
    pub caches: u32,
    /// When the next elite walks out of the horde, in run seconds.
    pub next_elite: f32,
    /// Whether this run's guardians stand at their posts yet.
    pub guarded: bool,
    /// The level-up hand's tools left, and the choices banished for the run ([`hand`]).
    pub rerolls: u32,
    pub banishes: u32,
    pub banished: Vec<hand::Struck>,
}

impl Default for Run {
    fn default() -> Self {
        Self {
            class: &data::CLASSES[0],
            race: 0,
            elapsed: 0.0,
            kills: 0,
            level: 1,
            xp: 0.0,
            hp: BASE_HP,
            abilities: Vec::new(),
            passives: Vec::new(),
            bonuses: Bonuses::default(),
            revives_used: 0,
            shield: 0.0,
            shield_t: 0.0,
            immune_t: 0.0,
            thorns: 0.0,
            thorns_t: 0.0,
            thorns_spell: 0,
            heal_rate: 0.0,
            heal_t: 0.0,
            mobile: mobility::MobilityState::default(),
            pending_levels: 0,
            bosses: 0,
            rings: 0,
            won: false,
            recorded: false,
            dead_t: None,
            next_guid: 1,
            damage_done: 0.0,
            version: 0,
            banner: None,
            gold: 0,
            elites: 0,
            caches: 0,
            next_elite: treasure::ELITE_FIRST_SECS,
            guarded: false,
            rerolls: hand::REROLLS_PER_RUN,
            banishes: hand::BANISHES_PER_RUN,
            banished: Vec::new(),
        }
    }
}

impl Run {
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
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn run_with(bonuses: &[Bonus]) -> Run {
        let mut run = Run::default();
        run.bonuses.add_all(bonuses);
        run
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
        let s = Run::default().stats();
        assert!(close(s.might, 1.0) && close(s.area, 1.0) && close(s.growth, 1.0));
        assert!(close(s.haste, 0.0) && close(s.armor_frac, 0.0) && close(s.lifesteal, 0.0));
        assert!(close(s.crit, 0.05) && close(s.crit_damage, BASE_CRIT_DAMAGE));
        assert!(close(s.gold, 1.0) && close(s.luck, 1.0));
        assert_eq!((s.targets, s.revives), (0, 0));
    }

    #[test]
    fn each_stat_key_reaches_its_stat() {
        let base = Run::default().stats();
        let one = |stat, v| run_with(&[Bonus::new(stat, v)]).stats();
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
        let fire = run_with(&[Bonus::school(School::Fire, 0.3)]).stats();
        assert!(close(fire.might, 1.0));
    }

    #[test]
    fn passives_and_bonuses_add_within_a_stat() {
        let mut run = run_with(&[
            Bonus::new(StatKey::MoveSpeed, 0.1),
            Bonus::new(StatKey::Damage, 0.1),
            Bonus::new(StatKey::MaxHealth, 20.0),
        ]);
        run.passives = vec![
            (passive("pack"), 1),
            (passive("might"), 1),
            (passive("fortitude"), 1),
        ];
        let base = Run::default().stats();
        let s = run.stats();
        assert!(close(s.speed, base.speed * 1.2));
        assert!(close(s.might, 1.2));
        // Flat health lands before the percentage scales it.
        assert!(close(s.max_hp, (base.max_hp + 20.0) * 1.2));
    }

    #[test]
    fn armor_is_capped_with_the_flat_block_counted() {
        let s = run_with(&[Bonus::new(StatKey::Armor, 0.9)]).stats();
        assert_eq!(s.armor_frac, ARMOR_CAP);
        assert!(close(s.after_armor(100.0), 40.0));
        let mut run = run_with(&[Bonus::new(StatKey::Armor, 0.5)]);
        run.passives = vec![(passive("wild"), 5)];
        let s = run.stats();
        assert!(close(s.armor, 5.0));
        // 5 flat and half of 100 remove 55.
        assert!(close(s.after_armor(100.0), 45.0));
        // 5 flat and half of 20 would remove 15; the cap allows 12.
        assert!(close(s.after_armor(20.0), 8.0));
        assert!(close(s.after_armor(1.5), 1.0));
        // The flat block alone, under the cap, is unchanged.
        run.bonuses = Bonuses::default();
        assert!(close(run.stats().after_armor(20.0), 15.0));
    }

    #[test]
    fn cooldown_bonus_is_capped_and_stacks_with_haste() {
        let s = run_with(&[Bonus::new(StatKey::Cooldown, 0.8)]).stats();
        assert_eq!(s.haste, COOLDOWN_CAP);
        let mut run = run_with(&[Bonus::new(StatKey::Cooldown, 0.8)]);
        run.passives = vec![(passive("intellect"), 5)];
        let s = run.stats();
        assert!(close(s.haste, COOLDOWN_CAP + 0.4));
        // The ability floor still keeps 35% of a cooldown.
        let o = owned("frostbolt", 1);
        assert!(close(o.cooldown(&s), o.def.cooldown * 0.35));
    }

    #[test]
    fn targets_add_to_ability_counts() {
        let s = run_with(&[Bonus::new(StatKey::Targets, 2.0)]).stats();
        // Three missiles, one more at rank 3, two from the bonus.
        assert_eq!(owned("arcane_missiles", 3).count(&s), 6);
        assert_eq!(
            owned("arcane_missiles", 3).count(&Run::default().stats()),
            4
        );
    }

    #[test]
    fn school_damage_multiplies_its_school_on_top_of_might() {
        let run = run_with(&[
            Bonus::school(School::Fire, 0.3),
            Bonus::new(StatKey::Damage, 0.5),
        ]);
        assert!(close(run.school_mult(School::Fire), 1.3));
        assert!(close(run.school_mult(School::Frost), 1.0));
        assert!(close(
            run.stats().might * run.school_mult(School::Fire),
            1.95
        ));
    }

    #[test]
    fn lifesteal_heals_its_share_up_to_max_health() {
        let mut s = run_with(&[Bonus::new(StatKey::Lifesteal, 0.1)]).stats();
        let max = s.max_hp;
        assert!(close(s.leech(50.0, 100.0), 60.0));
        assert!(close(s.leech(max - 3.0, 100.0), max));
        assert!(close(s.leech(50.0, 0.0), 50.0));
        s.lifesteal = 0.0;
        assert!(close(s.leech(50.0, 100.0), 50.0));
    }

    #[test]
    fn revives_are_spent_one_at_a_time() {
        let mut run = run_with(&[Bonus::new(StatKey::Revives, 1.0)]);
        run.hp = -5.0;
        assert!(run.revive());
        assert_eq!(run.revives_used, 1);
        assert!(close(run.hp, run.stats().max_hp * REVIVE_HP));
        assert!(close(run.immune_t, REVIVE_IMMUNE_SECS));
        // Untouchable while the revive's immunity lasts.
        assert_eq!(hero::hurt_hero(&mut run, 500.0), 0.0);
        run.hp = 0.0;
        assert!(!run.revive());
        assert_eq!(run.revives_used, 1);
        assert!(!Run::default().revive());
    }

    #[test]
    fn luck_weighs_rarer_tiers_up() {
        let plain = Run::default().stats();
        for tier in 0..5 {
            assert_eq!(plain.luck_weight(tier), 1.0);
        }
        let lucky = run_with(&[Bonus::new(StatKey::Luck, 0.5)]).stats();
        assert_eq!(lucky.luck_weight(0), 1.0);
        assert!(close(lucky.luck_weight(2), 2.25));
        assert!(lucky.luck_weight(4) > lucky.luck_weight(3));
    }
}
