//! Item uses (docs/PROFESSIONS.md, 2.5, 5.2, 6.1): a right-click on a bag slot (the stock
//! `UseContainerItem`, taken here before the stock applier would send `CMSG_USE_ITEM` to a server
//! that is not there) or a satchel key ([`super::satchel`]) uses the item:
//!
//! - **Potions** ([`Use::Heal`], [`Use::Buff`]) heal a share of the hero's health or give a short
//!   buff, under one shared [`POTION_COOLDOWN`].
//! - **Elixirs and flasks** ([`Use::Elixir`], [`Use::Flask`]) last the run: one elixir and one
//!   flask at a time, a second replacing the first. They are taken at camp or as the run starts
//!   ([`RUN_START_SECS`]).
//! - **Food** ([`Use::Food`]) is Well Fed, one meal at a time, taken at camp; there it feeds the
//!   whole party.
//! - **Bandages** ([`Use::Bandage`]) heal over [`BANDAGE_SECS`]; each hit takes
//!   [`BANDAGE_HIT_COST`] off what is left, and Recently Bandaged keeps another off for
//!   [`BANDAGE_COOLDOWN`]. They share no cooldown with the potions.
//! - **Recipes** teach their recipe ([`super::recipes::taught_by`]).
//!
//! The bags and the cooldowns are each player's own; the hero lives where the run is simulated,
//! so a co-op client's use goes to the host as a `UseItem` and the host applies it
//! ([`take_effect`]). With the goblin's window open a right-click sells instead, and with the
//! Enchanting window open it disenchants ([`super::disenchant`]), as in 1.12.
//!
//! **Authored:** every number here (6.1's budget: a flask, an elixir and a meal come to about
//! +15% together). The visuals are the real use spells' (`Spell.dbc`, checked by
//! [`tests::consumable_spells_resolve_in_the_client`]).

use bevy::prelude::*;

use benilla_ui::script::UiScript;

use super::super::bonus::{Bonus, School, StatKey};
use super::super::campfire::{Camp, WELL_FED_KEY};
use super::super::party::{self, HeroState, LocalHero, NetRole};
use super::super::units::Fx;
use super::super::Run;
use super::gather::{CastLine, UiLines};
use super::inventory::Bags;
use super::items::*;
use super::recipes;
use super::skills::Skills;

/// The shared potion cooldown, in seconds (5.2).
pub(crate) const POTION_COOLDOWN: f32 = 60.0;
/// Recently Bandaged (11196): no second bandage for this long.
pub(crate) const BANDAGE_COOLDOWN: f32 = 60.0;
/// A bandage's heal runs this long, before hits shorten it.
pub(crate) const BANDAGE_SECS: f32 = 6.0;
/// Each hit takes this much off a running bandage, as 1.12's interrupt, softened (5.2).
pub(crate) const BANDAGE_HIT_COST: f32 = 1.0;
/// An elixir or a flask lasts the run: an hour is longer than any.
pub(crate) const RUN_BUFF_SECS: f32 = 3600.0;
/// Well Fed's 15 minutes (`Spell.dbc` 19705's duration).
pub(crate) const FOOD_SECS: f32 = 900.0;
/// Elixirs and flasks are taken at camp, or this early in a run.
pub(crate) const RUN_START_SECS: f32 = 30.0;
/// The timed buffs' keys in the mechanics toolkit, beside the camp's own; a meal shares Well
/// Fed's, so the fish and a dish never stack.
const ELIXIR_KEY: u32 = 0xCA4F_0101;
const FLASK_KEY: u32 = 0xCA4F_0102;
const POTION_KEY: u32 = 0xCA4F_0103;
const FOOD_KEY: u32 = WELL_FED_KEY;

/// The use spells whose visuals play on the drinker: `Healing Potion`, `Restore Mana`, `Elixir
/// of the Giants`, `Flask of the Titans`, `Food` and `First Aid`.
pub(crate) const POTION_SPELL: u32 = 439;
pub(crate) const MANA_SPELL: u32 = 438;
pub(crate) const ELIXIR_SPELL: u32 = 11405;
pub(crate) const FLASK_SPELL: u32 = 17626;
pub(crate) const FOOD_SPELL: u32 = 433;
pub(crate) const BANDAGE_SPELL: u32 = 746;

/// A cooldown the bags keep: each its own clock, shown on its items' slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    Potion,
    Bandage,
}

impl Category {
    pub(crate) fn secs(self) -> f32 {
        match self {
            Category::Potion => POTION_COOLDOWN,
            Category::Bandage => BANDAGE_COOLDOWN,
        }
    }
}

/// What using an item does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Use {
    /// Restores this share of the hero's health; the potion cooldown.
    Heal(f32),
    /// These bonuses for this long; the potion cooldown.
    Buff {
        secs: f32,
        bonuses: &'static [Bonus],
    },
    /// The run's elixir: these bonuses until the run ends, replacing any other elixir.
    Elixir(&'static [Bonus]),
    /// The run's flask, the same.
    Flask(&'static [Bonus]),
    /// Well Fed: these bonuses for [`FOOD_SECS`], replacing any other meal.
    Food(&'static [Bonus]),
    /// Heals this share of the hero's health over [`BANDAGE_SECS`].
    Bandage(f32),
}

impl Use {
    /// The cooldown it starts, if any.
    pub(crate) fn category(self) -> Option<Category> {
        match self {
            Use::Heal(_) | Use::Buff { .. } => Some(Category::Potion),
            Use::Bandage(_) => Some(Category::Bandage),
            Use::Elixir(_) | Use::Flask(_) | Use::Food(_) => None,
        }
    }

    /// Whether it is taken only at camp or as the run starts.
    pub(crate) fn at_camp(self) -> bool {
        matches!(self, Use::Elixir(_) | Use::Flask(_) | Use::Food(_))
    }
}

/// A usable item: its entry, what it does, and the use spell whose visual plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) struct Consumable {
    pub(in crate::survivors) item: u32,
    pub(in crate::survivors) kind: Use,
    pub(in crate::survivors) spell: u32,
}

const fn heal(item: u32, share: f32) -> Consumable {
    Consumable {
        item,
        kind: Use::Heal(share),
        spell: POTION_SPELL,
    }
}

const fn buff(item: u32, secs: f32, bonuses: &'static [Bonus]) -> Consumable {
    Consumable {
        item,
        kind: Use::Buff { secs, bonuses },
        spell: POTION_SPELL,
    }
}

const fn mana(item: u32, secs: f32, bonuses: &'static [Bonus]) -> Consumable {
    Consumable {
        spell: MANA_SPELL,
        ..buff(item, secs, bonuses)
    }
}

const fn elixir(item: u32, bonuses: &'static [Bonus]) -> Consumable {
    Consumable {
        item,
        kind: Use::Elixir(bonuses),
        spell: ELIXIR_SPELL,
    }
}

const fn flask(item: u32, bonuses: &'static [Bonus]) -> Consumable {
    Consumable {
        item,
        kind: Use::Flask(bonuses),
        spell: FLASK_SPELL,
    }
}

const fn food(item: u32, bonuses: &'static [Bonus]) -> Consumable {
    Consumable {
        item,
        kind: Use::Food(bonuses),
        spell: FOOD_SPELL,
    }
}

const fn bandage(item: u32, share: f32) -> Consumable {
    Consumable {
        item,
        kind: Use::Bandage(share),
        spell: BANDAGE_SPELL,
    }
}

const fn b(stat: StatKey, value: f32) -> Bonus {
    Bonus::new(stat, value)
}

/// Every usable item. The numbers are ours, sized to 6.1: the best flask, elixir and meal come to
/// about +15% output together, and a potion is recovery or a burst, never a run's power.
pub(in crate::survivors) static CONSUMABLES: &[Consumable] = &[
    // Healing potions: 15% to 40% of max health.
    heal(MINOR_HEALING_POTION, 0.15),
    heal(MINOR_REJUVENATION_POTION, 0.12),
    heal(GREATER_HEALING_POTION, 0.25),
    heal(SUPERIOR_HEALING_POTION, 0.32),
    heal(MAJOR_HEALING_POTION, 0.40),
    // Mana potions: survivors has no mana, so a short spell of shorter cooldowns.
    mana(MINOR_MANA_POTION, 10.0, &[b(StatKey::Cooldown, 0.10)]),
    mana(LESSER_MANA_POTION, 10.0, &[b(StatKey::Cooldown, 0.12)]),
    mana(MANA_POTION, 12.0, &[b(StatKey::Cooldown, 0.15)]),
    mana(GREATER_MANA_POTION, 12.0, &[b(StatKey::Cooldown, 0.18)]),
    mana(MAJOR_MANA_POTION, 15.0, &[b(StatKey::Cooldown, 0.25)]),
    // Combat potions.
    buff(
        MINOR_MAGIC_RESISTANCE_POTION,
        15.0,
        &[b(StatKey::Armor, 0.10)],
    ),
    buff(FREE_ACTION_POTION, 15.0, &[b(StatKey::MoveSpeed, 0.25)]),
    buff(MIGHTY_TROLLS_BLOOD_POTION, 30.0, &[b(StatKey::Regen, 4.0)]),
    buff(MIGHTY_RAGE_POTION, 20.0, &[b(StatKey::Damage, 0.25)]),
    buff(GREATER_STONESHIELD_POTION, 20.0, &[b(StatKey::Armor, 0.25)]),
    // Elixirs: one at a time, for the run.
    elixir(ELIXIR_OF_MINOR_DEFENSE, &[b(StatKey::Armor, 0.02)]),
    elixir(ELIXIR_OF_DEFENSE, &[b(StatKey::Armor, 0.03)]),
    elixir(ELIXIR_OF_GREATER_DEFENSE, &[b(StatKey::Armor, 0.05)]),
    elixir(ELIXIR_OF_SUPERIOR_DEFENSE, &[b(StatKey::Armor, 0.08)]),
    elixir(ELIXIR_OF_FORTITUDE, &[b(StatKey::MaxHealth, 20.0)]),
    elixir(ELIXIR_OF_AGILITY, &[b(StatKey::CritChance, 0.02)]),
    elixir(ELIXIR_OF_GREATER_AGILITY, &[b(StatKey::CritChance, 0.03)]),
    elixir(ELIXIR_OF_GIANTS, &[b(StatKey::Damage, 0.05)]),
    elixir(
        ELIXIR_OF_BRUTE_FORCE,
        &[b(StatKey::Damage, 0.03), b(StatKey::MaxHealth, 15.0)],
    ),
    elixir(
        ELIXIR_OF_THE_MONGOOSE,
        &[b(StatKey::CritChance, 0.04), b(StatKey::MoveSpeed, 0.05)],
    ),
    elixir(ELIXIR_OF_THE_SAGES, &[b(StatKey::Cooldown, 0.05)]),
    elixir(
        GREATER_ARCANE_ELIXIR,
        &[Bonus::school(School::Arcane, 0.08)],
    ),
    // Flasks: one, for the run.
    flask(FLASK_OF_THE_TITANS, &[b(StatKey::MaxHealth, 36.0)]),
    flask(FLASK_OF_SUPREME_POWER, &[b(StatKey::Damage, 0.06)]),
    flask(FLASK_OF_DISTILLED_WISDOM, &[b(StatKey::Cooldown, 0.06)]),
    // Food: Well Fed, one meal at a time.
    food(BRILLIANT_SMALLFISH, &[b(StatKey::Regen, 0.5)]),
    food(
        SPICED_WOLF_MEAT,
        &[b(StatKey::MaxHealth, 6.0), b(StatKey::Regen, 0.5)],
    ),
    food(COYOTE_STEAK, &[b(StatKey::MaxHealth, 8.0)]),
    food(LONGJAW_MUD_SNAPPER, &[b(StatKey::Regen, 1.0)]),
    food(SMOKED_BEAR_MEAT, &[b(StatKey::MaxHealth, 10.0)]),
    food(SMOKED_SAGEFISH, &[b(StatKey::Regen, 1.5)]),
    food(SAVORY_DEVIATE_DELIGHT, &[b(StatKey::MoveSpeed, 0.08)]),
    food(
        BIG_BEAR_STEAK,
        &[b(StatKey::MaxHealth, 12.0), b(StatKey::Regen, 0.5)],
    ),
    food(
        HOT_LION_CHOPS,
        &[b(StatKey::MaxHealth, 14.0), b(StatKey::Regen, 0.5)],
    ),
    food(
        ROAST_RAPTOR,
        &[b(StatKey::MaxHealth, 16.0), b(StatKey::Regen, 1.0)],
    ),
    food(
        HOT_WOLF_RIBS,
        &[b(StatKey::Damage, 0.02), b(StatKey::MaxHealth, 10.0)],
    ),
    food(SPOTTED_YELLOWTAIL, &[b(StatKey::Regen, 2.0)]),
    food(MONSTER_OMELET, &[b(StatKey::MaxHealth, 24.0)]),
    food(
        TENDER_WOLF_STEAK,
        &[b(StatKey::MaxHealth, 18.0), b(StatKey::Regen, 1.0)],
    ),
    food(GRILLED_SQUID, &[b(StatKey::CritChance, 0.03)]),
    food(NIGHTFIN_SOUP, &[b(StatKey::Cooldown, 0.04)]),
    // Bandages: 20% to 60% over six seconds.
    bandage(LINEN_BANDAGE, 0.20),
    bandage(HEAVY_LINEN_BANDAGE, 0.24),
    bandage(WOOL_BANDAGE, 0.28),
    bandage(HEAVY_WOOL_BANDAGE, 0.32),
    bandage(SILK_BANDAGE, 0.36),
    bandage(HEAVY_SILK_BANDAGE, 0.40),
    bandage(MAGEWEAVE_BANDAGE, 0.45),
    bandage(HEAVY_MAGEWEAVE_BANDAGE, 0.50),
    bandage(RUNECLOTH_BANDAGE, 0.55),
    bandage(HEAVY_RUNECLOTH_BANDAGE, 0.60),
];

/// What `item` does when used, if anything.
pub(in crate::survivors) fn consumable(item: u32) -> Option<&'static Consumable> {
    CONSUMABLES.iter().find(|c| c.item == item)
}

/// Whether `item` does anything when used: a consumable or a recipe.
pub(crate) fn usable(item: u32) -> bool {
    consumable(item).is_some() || recipes::taught_by(item).is_some()
}

/// "+5% damage and +20 max health", as a tooltip sentence reads a list of bonuses.
fn describe(bonuses: &[Bonus]) -> String {
    let parts: Vec<String> = bonuses.iter().map(Bonus::describe).collect();
    match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The tooltip's green `Use:` line for `item` (the stock tooltip adds the "Use: "), from what it
/// does, so the words never drift from the numbers.
pub(crate) fn use_line(item: u32) -> Option<String> {
    if let Some(r) = recipes::taught_by(item) {
        return Some(format!(
            "Teaches you how to make {} ({} {}).",
            r.name,
            r.line.name(),
            r.req
        ));
    }
    let c = consumable(item)?;
    let pct = |share: f32| (share * 100.0).round() as u32;
    Some(match c.kind {
        Use::Heal(share) => format!("Restores {}% of your health.", pct(share)),
        Use::Buff { secs, bonuses } => format!("{} for {secs:.0} sec.", describe(bonuses)),
        Use::Elixir(bonuses) => format!(
            "{} for the rest of the run. One elixir at a time.",
            describe(bonuses)
        ),
        Use::Flask(bonuses) => format!(
            "{} for the rest of the run, even through a fall. One flask at a time.",
            describe(bonuses)
        ),
        Use::Food(bonuses) => format!(
            "Well Fed: {} for {} min. Eaten at camp, it feeds your whole party.",
            describe(bonuses),
            (FOOD_SECS / 60.0) as u32
        ),
        Use::Bandage(share) => format!(
            "Heals {}% of your health over {BANDAGE_SECS:.0} sec. Each hit you take shortens it.",
            pct(share)
        ),
    })
}

/// Where a use happens.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct UseCtx {
    /// `GetTime`'s clock, for the cooldowns.
    pub(crate) now: f64,
    /// A run is playing, so there is a hero to take it.
    pub(crate) in_run: bool,
    /// A camp is on.
    pub(crate) at_camp: bool,
    /// The run's clock, in seconds.
    pub(crate) run_secs: f32,
}

/// What a use came to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::survivors) enum Used {
    /// One went down: the hero takes it.
    Took(&'static Consumable),
    /// A recipe was read.
    Learned(&'static recipes::Recipe),
    /// Its cooldown is still running.
    NotReady,
    /// Refused, with the stock `GlobalStrings.lua` key and its words.
    Refused(&'static str, &'static str),
    /// Nothing to use there, or nothing that does anything.
    Nothing,
}

/// One use of the stack at `(bag, slot)`: the item out of the bags and its cooldown started, or
/// the recipe learned, or the reason not.
pub(in crate::survivors) fn use_stack(
    bags: &mut Bags,
    skills: &mut Skills,
    bag: i64,
    slot: u32,
    ctx: UseCtx,
) -> Used {
    let Some(stack) = bags.get(bag, slot) else {
        return Used::Nothing;
    };
    if let Some(r) = recipes::taught_by(stack.item) {
        if recipes::known(r, skills) {
            return Used::Refused("ERR_LEARN_SPELL_ALREADY_KNOWN", "You already know that.");
        }
        if !recipes::learnable(r, skills) {
            return Used::Refused(
                "ERR_LEARN_RECIPE_SKILL_TOO_LOW",
                "Your skill is not high enough.",
            );
        }
        if bags.destroy(bag, slot, 1).is_err() {
            return Used::Nothing;
        }
        skills.learn(r.spell);
        return Used::Learned(r);
    }
    let Some(c) = consumable(stack.item) else {
        return Used::Nothing;
    };
    if !ctx.in_run {
        return Used::Refused("ERR_USE_CANT_IMMUNE", "You can't do that right now.");
    }
    if c.kind.at_camp() && !ctx.at_camp && ctx.run_secs > RUN_START_SECS {
        return Used::Refused(
            "SPELL_FAILED_NOT_HERE",
            "Only at camp, or as the run starts.",
        );
    }
    let category = c.kind.category();
    if category.is_some_and(|cat| bags.cooldown_running(cat, ctx.now)) {
        return Used::NotReady;
    }
    if bags.destroy(bag, slot, 1).is_err() {
        return Used::Nothing;
    }
    if let Some(cat) = category {
        bags.start_cooldown(cat, ctx.now);
    }
    Used::Took(c)
}

/// The first stack of `item` in the bags, `(bag, slot)`.
pub(crate) fn find(bags: &Bags, item: u32) -> Option<(i64, u32)> {
    bags.containers().find_map(|bag| {
        bags.contents(bag)
            .find(|(_, s)| s.item == item)
            .map(|(slot, _)| (bag, slot))
    })
}

/// A bandage running on a hero, where the run is simulated: its heal a second, the seconds left,
/// and the hero's damage taken when last looked at, so a hit shortens it.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bandaging {
    pub(crate) per_sec: f32,
    pub(crate) left: f32,
    seen_taken: f64,
}

impl Bandaging {
    /// A bandage healing `share` of `hero`'s health over [`BANDAGE_SECS`].
    pub(in crate::survivors) fn on(hero: &HeroState, share: f32) -> Self {
        Self {
            per_sec: hero.stats().max_hp * share / BANDAGE_SECS,
            left: BANDAGE_SECS,
            seen_taken: hero.damage_taken,
        }
    }

    /// `dt` of it on `hero`: the hits since the last look shorten it first, then it heals for
    /// what is left of `dt`. Whether it still runs.
    pub(in crate::survivors) fn tick(&mut self, hero: &mut HeroState, dt: f32) -> bool {
        if !hero.alive() {
            return false;
        }
        if hero.damage_taken > self.seen_taken {
            self.left -= BANDAGE_HIT_COST;
            self.seen_taken = hero.damage_taken;
        }
        let step = dt.min(self.left.max(0.0));
        if step > 0.0 {
            party::heal(hero, self.per_sec * step);
        }
        self.left -= dt;
        self.left > 0.0
    }
}

/// A taken consumable's effect on the hero who took it, where the run is simulated. Food at camp
/// is the caller's to share ([`feeds_party`]). Its visual plays on the hero.
pub(in crate::survivors) fn take_effect(
    commands: &mut Commands,
    fx: &mut Fx,
    entity: Entity,
    hero: &mut HeroState,
    c: &Consumable,
) {
    use super::super::mechanics::replace_buff;
    match c.kind {
        Use::Heal(share) => {
            let amount = hero.stats().max_hp * share;
            party::heal(hero, amount);
        }
        Use::Buff { secs, bonuses } => replace_buff(hero, POTION_KEY, secs, bonuses),
        Use::Elixir(bonuses) => replace_buff(hero, ELIXIR_KEY, RUN_BUFF_SECS, bonuses),
        Use::Flask(bonuses) => replace_buff(hero, FLASK_KEY, RUN_BUFF_SECS, bonuses),
        Use::Food(bonuses) => replace_buff(hero, FOOD_KEY, FOOD_SECS, bonuses),
        Use::Bandage(share) => {
            commands.entity(entity).insert(Bandaging::on(hero, share));
        }
    }
    fx.cast(entity, c.spell, &[], None);
}

/// Whether `c`, taken now, feeds every hero: a meal at camp.
pub(in crate::survivors) fn feeds_party(c: &Consumable, at_camp: bool) -> bool {
    matches!(c.kind, Use::Food(_)) && at_camp
}

/// A meal's Well Fed on an ally fed at camp, with its visual.
pub(in crate::survivors) fn feed(
    fx: &mut Fx,
    entity: Entity,
    hero: &mut HeroState,
    c: &Consumable,
) {
    if let Use::Food(bonuses) = c.kind {
        if hero.alive() {
            super::super::mechanics::replace_buff(hero, FOOD_KEY, FOOD_SECS, bonuses);
            fx.cast(entity, c.spell, &[], None);
        }
    }
}

/// A co-op client's uses for the host to apply to its hero, by item entry.
#[derive(Resource, Default)]
pub(crate) struct UseOutbox(pub(crate) Vec<u32>);

/// Where the run stands, for a use.
pub(in crate::survivors) fn use_ctx(now: f64, playing: bool, run: &Run, camp: &Camp) -> UseCtx {
    UseCtx {
        now,
        in_run: playing,
        at_camp: playing && camp.active(run),
        run_secs: run.elapsed,
    }
}

/// Where the run stands for a use.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Stage<'w> {
    phase: Res<'w, State<super::super::Phase>>,
    run: Res<'w, Run>,
    camp: Res<'w, Camp>,
    role: Res<'w, NetRole>,
    selection: Res<'w, super::super::Selection>,
}

impl Stage<'_> {
    pub(super) fn ctx(&self, now: f64) -> UseCtx {
        let playing = *self.phase.get() == super::super::Phase::Playing;
        use_ctx(now, playing, &self.run, &self.camp)
    }

    /// The battleground's band, which a disenchant's tier follows.
    pub(super) fn band(&self) -> super::nodes::Band {
        super::nodes::band_of(self.selection.stage_def().name)
    }

    pub(super) fn role(&self) -> NetRole {
        *self.role
    }
}

/// What a use out of the bags reaches: the heroes, their visuals, the co-op outbox and the stock
/// UI's lines.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Taker<'w, 's> {
    commands: Commands<'w, 's>,
    heroes: Query<'w, 's, (Entity, &'static mut HeroState, Has<LocalHero>)>,
    fx: Fx<'w>,
    outbox: ResMut<'w, UseOutbox>,
    pub(super) ui: ResMut<'w, UiLines>,
}

impl Taker<'_, '_> {
    /// What a local use does once it is out of the bags: a learned recipe is announced; a taken
    /// item goes to the host from a co-op client, and otherwise takes effect here, on the local
    /// hero (and, for a meal at camp, every hero).
    pub(super) fn after(&mut self, used: Used, ctx: UseCtx, role: NetRole) {
        match used {
            Used::Took(c) if role == NetRole::Client => self.outbox.0.push(c.item),
            Used::Took(c) => {
                let party = feeds_party(c, ctx.at_camp);
                for (entity, mut hero, local) in self.heroes.iter_mut() {
                    if local {
                        take_effect(&mut self.commands, &mut self.fx, entity, &mut hero, c);
                    } else if party {
                        feed(&mut self.fx, entity, &mut hero, c);
                    }
                }
            }
            Used::Learned(r) => self.ui.0.push(CastLine::LearnedRecipe(r.name)),
            Used::NotReady => self.ui.0.push(CastLine::NotReady),
            Used::Refused(key, text) => self.ui.0.push(CastLine::Refused(key, text)),
            Used::Nothing => {}
        }
    }
}

/// The bags and what a use or a disenchant changes beside them.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Pack<'w> {
    wallet: super::econ::Wallet<'w>,
    skills: ResMut<'w, Skills>,
    rng: ResMut<'w, super::super::Rng>,
    desk: Res<'w, super::stations::CampDesk>,
}

impl Pack<'_> {
    /// The bags and the skills, which a use changes together.
    pub(super) fn bags_and_skills(&mut self) -> (&mut Bags, &mut Skills) {
        (&mut self.wallet.bags, &mut self.skills)
    }
}

/// `Update`, after the input pass and before the stock script-call applier: this frame's bag
/// right-clicks. The goblin's window open, it sells; the Enchanting window open, it disenchants;
/// otherwise it uses.
pub(super) fn use_items(
    script: Option<NonSendMut<UiScript>>,
    stage: Stage,
    mut pack: Pack,
    mut taker: Taker,
) {
    let Some(mut script) = script else {
        return;
    };
    let ctx = stage.ctx(script.now());
    let Pack {
        wallet,
        skills,
        rng,
        desk,
    } = &mut pack;
    let super::econ::Wallet { bags, purse, shop } = wallet;
    for (bag, slot) in script.take_container_uses() {
        if shop.open {
            if let Err(e) = super::econ::sell(bags, purse, shop, bag, slot) {
                let (key, fallback) = e.global_string();
                taker.ui.0.push(CastLine::Refused(key, fallback));
            }
            continue;
        }
        if desk.book == Some(super::skills::Line::Enchanting) {
            let lines = &mut taker.ui.0;
            super::disenchant::from_bags(bags, skills, (bag, slot), stage.band(), rng, lines);
            continue;
        }
        let used = use_stack(bags, skills, bag, slot, ctx);
        taker.after(used, ctx, stage.role());
    }
}

/// `Update`, where the run is simulated: the running bandages heal, hits shorten them, and they
/// come off when done or when the hero falls.
pub(super) fn tick_bandages(
    mut commands: Commands,
    time: Res<Time>,
    mut heroes: Query<(Entity, &mut HeroState, &mut Bandaging)>,
) {
    let dt = time.delta_secs();
    for (entity, mut hero, mut bandage) in &mut heroes {
        if !bandage.tick(&mut hero, dt) {
            commands.entity(entity).remove::<Bandaging>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::bonus::Bonuses;
    use super::super::super::data;
    use super::*;

    fn in_run(now: f64) -> UseCtx {
        UseCtx {
            now,
            in_run: true,
            at_camp: false,
            run_secs: 120.0,
        }
    }

    fn hero() -> HeroState {
        HeroState::new(0, &data::CLASSES[0], 1, Bonuses::default())
    }

    #[test]
    fn survivors_effects_a_potion_heals_once_per_cooldown() {
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        bags.add(MINOR_HEALING_POTION, 2);
        bags.add(COPPER_ORE, 1);
        let took = use_stack(&mut bags, &mut skills, 0, 1, in_run(100.0));
        assert!(matches!(took, Used::Took(c) if c.kind == Use::Heal(0.15)));
        assert_eq!(bags.get(0, 1).map(|s| s.count), Some(1), "one consumed");
        assert_eq!(bags.cooldown(Category::Potion), Some(100.0));
        assert_eq!(
            use_stack(&mut bags, &mut skills, 0, 1, in_run(159.0)),
            Used::NotReady
        );
        assert_eq!(
            use_stack(&mut bags, &mut skills, 0, 2, in_run(200.0)),
            Used::Nothing,
            "ore"
        );
        assert!(matches!(
            use_stack(&mut bags, &mut skills, 0, 1, in_run(160.0)),
            Used::Took(_)
        ));
        assert_eq!(bags.get(0, 1), None, "the last one");
        assert_eq!(
            use_stack(&mut bags, &mut skills, 0, 1, in_run(300.0)),
            Used::Nothing,
            "empty"
        );
    }

    /// Potions share one cooldown; a bandage has its own, Recently Bandaged.
    #[test]
    fn survivors_effects_the_potion_cooldown_is_shared_and_bandages_apart() {
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        bags.add(MINOR_HEALING_POTION, 1);
        bags.add(MIGHTY_RAGE_POTION, 1);
        bags.add(LINEN_BANDAGE, 2);
        assert!(matches!(
            use_stack(&mut bags, &mut skills, 0, 1, in_run(10.0)),
            Used::Took(_)
        ));
        let rage = find(&bags, MIGHTY_RAGE_POTION).unwrap();
        assert_eq!(
            use_stack(&mut bags, &mut skills, rage.0, rage.1, in_run(20.0)),
            Used::NotReady,
            "the shared potion cooldown"
        );
        let linen = find(&bags, LINEN_BANDAGE).unwrap();
        assert!(matches!(
            use_stack(&mut bags, &mut skills, linen.0, linen.1, in_run(20.0)),
            Used::Took(c) if c.kind == Use::Bandage(0.20)
        ));
        assert_eq!(
            use_stack(&mut bags, &mut skills, linen.0, linen.1, in_run(79.0)),
            Used::NotReady,
            "Recently Bandaged"
        );
        assert!(matches!(
            use_stack(&mut bags, &mut skills, linen.0, linen.1, in_run(80.5)),
            Used::Took(_)
        ));
    }

    /// Elixirs, flasks and food only at camp or as the run starts, and nothing out of a run.
    #[test]
    fn survivors_effects_elixirs_wait_for_camp() {
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        bags.add(ELIXIR_OF_GIANTS, 2);
        assert!(matches!(
            use_stack(&mut bags, &mut skills, 0, 1, in_run(1.0)),
            Used::Refused(..)
        ));
        let camp = UseCtx {
            at_camp: true,
            ..in_run(1.0)
        };
        assert!(matches!(
            use_stack(&mut bags, &mut skills, 0, 1, camp),
            Used::Took(_)
        ));
        let start = UseCtx {
            run_secs: 5.0,
            ..in_run(1.0)
        };
        assert!(matches!(
            use_stack(&mut bags, &mut skills, 0, 1, start),
            Used::Took(_)
        ));
        bags.add(MINOR_HEALING_POTION, 1);
        assert!(matches!(
            use_stack(&mut bags, &mut skills, 0, 1, UseCtx::default()),
            Used::Refused(..)
        ));
    }

    /// One elixir at a time: a second replaces the first; a flask and a meal sit beside it.
    #[test]
    fn survivors_effects_one_elixir_one_flask_one_meal() {
        use super::super::super::mechanics::replace_buff;
        let mut h = hero();
        let base = h.stats();
        let giants = consumable(ELIXIR_OF_GIANTS).unwrap();
        let fortitude = consumable(ELIXIR_OF_FORTITUDE).unwrap();
        let Use::Elixir(g) = giants.kind else {
            panic!()
        };
        let Use::Elixir(f) = fortitude.kind else {
            panic!()
        };
        replace_buff(&mut h, ELIXIR_KEY, RUN_BUFF_SECS, g);
        assert!(h.stats().might > base.might);
        replace_buff(&mut h, ELIXIR_KEY, RUN_BUFF_SECS, f);
        assert!(
            (h.stats().might - base.might).abs() < 1e-5,
            "Giants is gone"
        );
        assert!(h.stats().max_hp > base.max_hp, "Fortitude is on");
        let Use::Flask(t) = consumable(FLASK_OF_THE_TITANS).unwrap().kind else {
            panic!()
        };
        replace_buff(&mut h, FLASK_KEY, RUN_BUFF_SECS, t);
        let Use::Food(m) = consumable(MONSTER_OMELET).unwrap().kind else {
            panic!()
        };
        replace_buff(&mut h, FOOD_KEY, FOOD_SECS, m);
        let Use::Food(s) = consumable(GRILLED_SQUID).unwrap().kind else {
            panic!()
        };
        replace_buff(&mut h, FOOD_KEY, FOOD_SECS, s);
        assert!(
            (h.stats().max_hp - base.max_hp - 20.0 - 36.0).abs() < 1e-3,
            "the elixir and the flask stand; the squid replaced the omelet"
        );
        assert!(h.stats().crit > base.crit);
    }

    /// A bandage heals over its seconds; each hit takes one off; a fall ends it.
    #[test]
    fn survivors_effects_a_bandage_is_shortened_by_hits() {
        let mut h = hero();
        let max = h.stats().max_hp;
        h.hp = 1.0;
        let mut b = Bandaging::on(&h, 0.30);
        assert!(b.tick(&mut h, 1.0));
        assert!((h.hp - (1.0 + max * 0.05)).abs() < 1e-3, "a sixth a second");
        // A hit: one second gone, then the second's heal.
        h.damage_taken += 5.0;
        assert!(b.tick(&mut h, 1.0));
        assert!((b.left - 3.0).abs() < 1e-5);
        let mut hp = h.hp;
        for _ in 0..3 {
            b.tick(&mut h, 1.0);
            assert!(h.hp > hp);
            hp = h.hp;
        }
        assert!(b.left <= 0.0, "done a second early");
        assert!(!b.tick(&mut h, 1.0));
        assert!((h.hp - hp).abs() < 1e-5, "and heals no more");
    }

    #[test]
    fn survivors_effects_recipes_teach_once() {
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        bags.add(RECIPE_SAVORY_DEVIATE_DELIGHT, 2);
        assert!(
            matches!(
                use_stack(&mut bags, &mut skills, 0, 1, UseCtx::default()),
                Used::Refused(..)
            ),
            "Cooking 85 is needed"
        );
        let mut cook = Skills::from_save(&std::collections::BTreeMap::from([(
            "cooking".to_string(),
            super::super::skills::Rank { rank: 90, max: 150 },
        )]));
        assert!(matches!(
            use_stack(&mut bags, &mut cook, 0, 1, UseCtx::default()),
            Used::Learned(r) if r.spell == 8238
        ));
        assert_eq!(
            bags.count(RECIPE_SAVORY_DEVIATE_DELIGHT),
            1,
            "read and gone"
        );
        let (bag, slot) = find(&bags, RECIPE_SAVORY_DEVIATE_DELIGHT).unwrap();
        assert!(matches!(
            use_stack(&mut bags, &mut cook, bag, slot, UseCtx::default()),
            Used::Refused("ERR_LEARN_SPELL_ALREADY_KNOWN", _)
        ));
    }

    /// Every consumable is authored, and every tooltip line comes from its numbers.
    #[test]
    fn survivors_effects_consumables_are_authored() {
        let mut seen = std::collections::HashSet::new();
        for c in CONSUMABLES {
            assert!(seen.insert(c.item), "{} twice", c.item);
            let def = template(c.item).expect("authored");
            assert_eq!(def.class, 0, "{}: a consumable", def.name);
            assert!(use_line(c.item).is_some());
        }
        for def in ITEMS.iter().filter(|d| d.class == 0) {
            assert!(consumable(def.entry).is_some(), "{} does nothing", def.name);
        }
        assert_eq!(
            use_line(MINOR_HEALING_POTION).as_deref(),
            Some("Restores 15% of your health.")
        );
        assert_eq!(
            use_line(ELIXIR_OF_BRUTE_FORCE).as_deref(),
            Some("+3% damage and +15 max health for the rest of the run. One elixir at a time.")
        );
    }

    /// The power budget (6.1): the best flask, elixir and meal together come to about +15% output,
    /// and no potion lasts past half a minute.
    #[test]
    fn survivors_effects_the_power_budget() {
        // Output as damage plus crit chance (a crit doubles), and cooldowns at their worth.
        let output = |bs: &[Bonus]| -> f32 {
            bs.iter()
                .map(|b| match b.stat {
                    StatKey::Damage => b.value,
                    StatKey::CritChance | StatKey::Cooldown => b.value,
                    _ => 0.0,
                })
                .sum()
        };
        let best = |pick: fn(Use) -> Option<&'static [Bonus]>| -> f32 {
            CONSUMABLES
                .iter()
                .filter_map(|c| pick(c.kind))
                .map(output)
                .fold(0.0, f32::max)
        };
        let elixir = best(|u| match u {
            Use::Elixir(b) => Some(b),
            _ => None,
        });
        let flask = best(|u| match u {
            Use::Flask(b) => Some(b),
            _ => None,
        });
        let meal = best(|u| match u {
            Use::Food(b) => Some(b),
            _ => None,
        });
        let total = elixir + flask + meal;
        assert!(
            (0.10..=0.18).contains(&total),
            "a flask, an elixir and a meal: {total}"
        );
        for c in CONSUMABLES {
            if let Use::Buff { secs, .. } = c.kind {
                assert!(secs <= 30.0, "{}", c.item);
            }
            if let Use::Heal(share) | Use::Bandage(share) = c.kind {
                assert!(share <= 0.60, "{}", c.item);
            }
        }
    }

    /// The right-click through the engine's own `UseContainerItem` lands in the drain.
    #[test]
    fn survivors_effects_the_bag_right_click_reaches_the_drain() {
        let mut s = UiScript::new().unwrap();
        let mut bags = Bags::default();
        bags.add(MINOR_HEALING_POTION, 1);
        let mut feed = super::super::frames::BagFeed::default();
        super::super::frames::sync(&mut s, &mut bags, 0, &mut feed);
        s.run("UseContainerItem(0, 1)").unwrap();
        let uses = s.take_container_uses();
        assert_eq!(uses, vec![(0, 1)]);
        let now = s.now();
        assert!(matches!(
            use_stack(&mut bags, &mut Skills::default(), 0, 1, in_run(now)),
            Used::Took(_)
        ));
    }

    /// Recently Bandaged, the debuff whose 60 s [`BANDAGE_COOLDOWN`] keeps a second bandage off.
    const RECENTLY_BANDAGED: u32 = 11196;

    /// The use spells are real, and named as they should be.
    #[test]
    fn consumable_spells_resolve_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let spells = benilla_formats::load_spell_catalog(&mut chain).expect("Spell.dbc");
        let mut bad = Vec::new();
        for (id, name) in [
            (POTION_SPELL, "Healing Potion"),
            (MANA_SPELL, "Restore Mana"),
            (ELIXIR_SPELL, "Elixir of the Giants"),
            (FLASK_SPELL, "Flask of the Titans"),
            (FOOD_SPELL, "Food"),
            (BANDAGE_SPELL, "First Aid"),
            (RECENTLY_BANDAGED, "Recently Bandaged"),
        ] {
            let found = spells.get(id).map(|s| s.name.as_str());
            if found != Some(name) {
                bad.push(format!("{id}: {found:?}, not {name}"));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
