//! The camp's stock windows (docs/PROFESSIONS.md, 2.1, 5.1, 5.4): the goblin's `MerchantFrame`, the
//! trainer's `ClassTrainerFrame`, the crafting books' `TradeSkillFrame` and Enchanting's
//! `CraftFrame`, each pushed from the local [`Bags`], [`Purse`] and [`Skills`], each drained here
//! (the stock net drains stand down under [`crate::run_mode::SurvivorsUi`]). Everything is this
//! player's own, so nothing goes on the wire.
//!
//! - **The merchant** opens with the goblin's own window (a click on him, `campfire`), beside it:
//!   his goods for the purse, any bag item sold for its `sell_price` (a right-click on a bag slot,
//!   or a drop on the window), and the buyback tab. Closing either window closes both.
//! - **The trainer** opens from the camp's Train button. It sells Journeyman, Expert and Artisan
//!   on every line, and the recipes its books teach for gold ([`recipes::Learn::Trainer`]).
//! - **The books** open from the camp's Smelt, Alchemy, Cook, First Aid and Enchant buttons, one at
//!   a time ([`CampDesk::book`]). A recipe casts for its own time at the camp's fire (the stock
//!   casting bar showing it), takes its reagents, makes its product and rolls its line on its
//!   bands; Create All repeats while the reagents last. An enchant goes on the piece the picked
//!   class wears in its slot ([`GearHook`]).
//! - All of them close as the camp ends or a modal survivors screen opens.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use benilla_ui::script::{
    ScriptValue, TrainerService, TrainerServiceCategory, TrainerSkillReq, TrainerState,
    TrainerTooltip, UiScript, UnitState,
};

use super::super::campfire::{Camp, MerchantWindow};
use super::super::gear::{GearStore, Slot};
use super::super::{Phase, Rng, Run, Selection};
use super::econ::{self, Purse, Shop, ShopError, Wallet};
use super::gather::{CastLine, UiLines};
use super::inventory::Bags;
use super::recipes::{self, Enchant, Learn, Recipe};
use super::skills::{Line, Skills, TrainError, TRAINER_RANKS};
use crate::ui_script::VmMemo;

/// The 1.12 tradeskill trainer type (`SMSG_TRAINER_LIST`'s 2).
const TRAINER_TYPE_TRADESKILL: u32 = 2;
/// The names the windows' titles read off the `npc` unit.
const MERCHANT_NAME: &str = "Goblin Merchant";
const TRAINER_NAME: &str = "Profession Trainer";
const TRAINER_GREETING: &str =
    "Gather it, brew it, cook it, bind it, enchant it, forge it, sew it or blow it up: show me what you can do, and I will teach you more.";

/// The camp's Train and book buttons ask for their windows here; each press toggles.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CampDesk {
    pub(crate) trainer: bool,
    /// The open book: Enchanting's is the craft window, the others the tradeskill window.
    pub(crate) book: Option<Line>,
}

impl CampDesk {
    /// A book button: opens `line`'s book in place of any other, or closes it.
    pub(crate) fn toggle(&mut self, line: Line) {
        self.book = if self.book == Some(line) {
            None
        } else {
            Some(line)
        };
    }
}

/// A recipe cast in progress, and the Create All repeats left after it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct Crafter {
    current: Option<(u32, f32)>,
    repeat: u32,
}

impl Crafter {
    pub(crate) fn busy(&self) -> bool {
        self.current.is_some()
    }
}

/// What an enchant reaches: the gear the picked class wears.
pub(in crate::survivors) trait GearHook {
    /// Whether a piece is worn in `slot`.
    fn wears(&self, slot: Slot) -> bool;
    /// Put `e` on the piece worn in its slot, over whatever enchant it had.
    fn enchant(&mut self, e: &'static Enchant);
}

/// What this VM was last shown, so a window opens, updates and closes once each.
#[derive(Default)]
pub(crate) struct Shown {
    merchant: Option<(u64, u64)>,
    trainer: Option<(u64, u64)>,
    book: Option<(Line, u64, u64, u32)>,
    craft: Option<(u64, u64)>,
}

/// One frame's inputs, borrowed from the app or a test.
pub(crate) struct Frame<'a> {
    /// A camp is on and the run is playing.
    pub(crate) at_camp: bool,
    /// The goblin's Bevy window is open.
    pub(crate) merchant: &'a mut bool,
    pub(crate) desk: &'a mut CampDesk,
    pub(crate) bags: &'a mut Bags,
    pub(crate) purse: &'a mut Purse,
    pub(crate) shop: &'a mut Shop,
    pub(crate) skills: &'a mut Skills,
    pub(crate) crafter: &'a mut Crafter,
    pub(crate) rng: &'a mut Rng,
    pub(crate) lines: &'a mut Vec<CastLine>,
    pub(in crate::survivors) gear: &'a mut dyn GearHook,
    pub(crate) dt: f32,
}

/// Where the run stands.
#[derive(SystemParam)]
pub(super) struct Where<'w> {
    phase: Res<'w, State<Phase>>,
    run: Res<'w, Run>,
    camp: Res<'w, Camp>,
    time: Res<'w, Time>,
    selection: Res<'w, Selection>,
}

/// The crafting half's state.
#[derive(SystemParam)]
pub(super) struct Craft<'w> {
    skills: ResMut<'w, Skills>,
    crafter: ResMut<'w, Crafter>,
    rng: ResMut<'w, Rng>,
    ui: ResMut<'w, UiLines>,
}

/// The picked class's gear, as an enchant reaches it.
struct WornGear<'a, 'w> {
    store: &'a mut GearStore<'w>,
    class: Option<usize>,
}

impl GearHook for WornGear<'_, '_> {
    fn wears(&self, slot: Slot) -> bool {
        self.class
            .is_some_and(|c| self.store.get(c).worn(slot).is_some())
    }

    fn enchant(&mut self, e: &'static Enchant) {
        let Some(class) = self.class else {
            return;
        };
        let Some(piece) = self.store.get(class).worn(e.slot).map(|p| p.id) else {
            return;
        };
        self.store.edit(class, |g| g.enchant(piece, e.id));
    }
}

/// `Update`, after the input pass and before the bag push: the windows' intents, casts and
/// pushes.
pub(super) fn run_stations(
    script: Option<NonSendMut<UiScript>>,
    at: Where,
    window: Option<ResMut<MerchantWindow>>,
    (mut desk, mut store): (ResMut<CampDesk>, GearStore),
    mut wallet: Wallet,
    mut craft: Craft,
    mut memo: Local<VmMemo<Shown>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let shown = memo.get(&script);
    let at_camp = *at.phase.get() == Phase::Playing && at.camp.active(&at.run);
    let mut merchant_open = window.as_ref().is_some_and(|w| w.open);
    let Craft {
        skills,
        crafter,
        rng,
        ui,
    } = &mut craft;
    let mut gear = WornGear {
        store: &mut store,
        class: at.selection.class,
    };
    drive(
        &mut script,
        shown,
        Frame {
            at_camp,
            merchant: &mut merchant_open,
            desk: &mut desk,
            bags: &mut wallet.bags,
            purse: &mut wallet.purse,
            shop: &mut wallet.shop,
            skills,
            crafter,
            rng,
            lines: &mut ui.0,
            gear: &mut gear,
            dt: at.time.delta_secs(),
        },
    );
    if let Some(mut w) = window {
        if w.open != merchant_open {
            w.open = merchant_open;
        }
    }
}

/// One frame of the windows.
pub(crate) fn drive(script: &mut UiScript, shown: &mut Shown, f: Frame) {
    let Frame {
        at_camp,
        merchant,
        desk,
        bags,
        purse,
        shop,
        skills,
        crafter,
        rng,
        lines,
        gear,
        dt,
    } = f;
    if !at_camp {
        *desk = CampDesk::default();
    }
    merchant_window(
        script,
        &mut shown.merchant,
        at_camp,
        merchant,
        Till {
            bags,
            purse,
            shop,
            lines,
        },
    );
    trainer_window(
        script,
        &mut shown.trainer,
        at_camp,
        desk,
        purse,
        skills,
        lines,
    );
    let mut ctx = Crafting {
        bags,
        skills,
        crafter,
        rng,
        lines,
        gear,
    };
    ctx.drain(script, shown.book.is_some(), shown.craft.is_some());
    ctx.tick(at_camp, dt);
    book_window(script, &mut shown.book, at_camp, desk, &ctx);
    craft_window(script, &mut shown.craft, at_camp, desk, &ctx);
}

/// A refusal as the stock error line.
fn refuse(lines: &mut Vec<CastLine>, e: ShopError) {
    let (key, fallback) = e.global_string();
    lines.push(CastLine::Refused(key, fallback));
}

/// What a sale or purchase reads and writes.
struct Till<'a> {
    bags: &'a mut Bags,
    purse: &'a mut Purse,
    shop: &'a mut Shop,
    lines: &'a mut Vec<CastLine>,
}

/// The goblin's `MerchantFrame`.
fn merchant_window(
    script: &mut UiScript,
    shown: &mut Option<(u64, u64)>,
    at_camp: bool,
    open: &mut bool,
    till: Till,
) {
    let Till {
        bags,
        purse,
        shop,
        lines,
    } = till;
    if script.take_merchant_close() {
        *open = false;
    }
    let buys = script.take_merchant_buys();
    let sells = script.take_merchant_cursor_sells();
    let slot_buys = script.take_merchant_slot_buys();
    let backs = script.take_merchant_buybacks();
    if shown.is_some() {
        for (row, quantity) in buys {
            if let Err(e) = econ::buy(bags, purse, row, quantity) {
                refuse(lines, e);
            }
        }
        for (bag, slot) in sells {
            if let Err(e) = econ::sell(bags, purse, shop, bag, slot) {
                refuse(lines, e);
            }
        }
        for (_, _, item) in slot_buys {
            if let Err(e) = econ::buy_entry(bags, purse, item) {
                refuse(lines, e);
            }
        }
        for index in backs {
            if let Err(e) = econ::buyback(bags, purse, shop, index) {
                refuse(lines, e);
            }
        }
    }
    let want = *open && at_camp;
    shop.open = want;
    let key = (shop.generation(), purse.generation());
    match (*shown, want) {
        (None, true) => {
            seat_npc(script, MERCHANT_NAME);
            script.set_merchant(Some(econ::merchant_state(shop)));
            script.fire_event("MERCHANT_SHOW", vec![]);
            *shown = Some(key);
        }
        (Some(was), true) if was != key => {
            script.set_merchant(Some(econ::merchant_state(shop)));
            script.fire_event("MERCHANT_UPDATE", vec![]);
            *shown = Some(key);
        }
        (Some(_), false) => {
            script.set_merchant(None);
            // Its OnHide queues the close the stock frame's own button would: spent here.
            script.fire_event("MERCHANT_CLOSED", vec![]);
            let _ = script.take_merchant_close();
            *open = false;
            *shown = None;
        }
        _ => {}
    }
}

/// The `npc` unit the windows title themselves from.
fn seat_npc(script: &mut UiScript, name: &str) {
    script.set_unit(
        "npc",
        Some(UnitState {
            exists: true,
            name: Some(name.to_string()),
            ..Default::default()
        }),
    );
}

/// A trainer service's category: known is "used", what the player can take now "available", the
/// rest "unavailable".
fn category(known: bool, ready: bool) -> TrainerServiceCategory {
    if known {
        TrainerServiceCategory::Used
    } else if ready {
        TrainerServiceCategory::Available
    } else {
        TrainerServiceCategory::Unavailable
    }
}

/// The trainer's list, grouped by line: Apprentice to Artisan on each, then the recipes the line's
/// book teaches for gold. A known rank is "used", the next one "available" once the skill reaches
/// it, the rest "unavailable"; a recipe is "available" at its skill.
pub(crate) fn trainer_state(skills: &Skills) -> TrainerState {
    let mut services = Vec::new();
    for line in Line::ALL {
        let r = skills.rank(line);
        let service = |spell: u32, name: String, subtext: String, cost: u32| TrainerService {
            spell_id: spell,
            name: Some(name),
            subtext: Some(subtext),
            texture: Some(line.icon().to_string()),
            description: String::new(),
            cost,
            prof_first_rank: false,
            category: TrainerServiceCategory::Unavailable,
            level_req: 0,
            skill_req: None,
            ability_reqs: Vec::new(),
            is_trade_skill: true,
            group_key: line.skill_line(),
            group_name: line.name().to_string(),
            tooltip: TrainerTooltip::Spell {
                spell_id: spell,
                alt_caster: false,
            },
        };
        for (step, &spell) in line.rank_spells().iter().enumerate() {
            let rank = TRAINER_RANKS[step];
            services.push(TrainerService {
                description: format!("Allows {} to reach {} skill.", line.name(), rank.cap),
                category: category(step <= r.step(), skills.can_train(line, step).is_ok()),
                skill_req: (rank.needs > 0).then(|| TrainerSkillReq {
                    name: line.name().to_string(),
                    rank: u32::from(rank.needs),
                    met: r.rank >= rank.needs,
                }),
                ..service(
                    spell,
                    line.spell_name().to_string(),
                    rank.name.to_string(),
                    rank.cost,
                )
            });
        }
        if line == Line::Engineering {
            // The specialisations, at Engineering 200, one or the other.
            for spec in recipes::Spec::ALL {
                let held = skills.learned(spec.spell());
                services.push(TrainerService {
                    texture: Some(spec.icon().to_string()),
                    description: format!(
                        "Makes you a {}, with the schematics only {} know. You may only be one: \
                         Gnomish or Goblin.",
                        spec.name(),
                        if spec == recipes::Spec::Gnomish {
                            "gnomes"
                        } else {
                            "goblins"
                        }
                    ),
                    category: category(held, recipes::spec_open(skills)),
                    skill_req: Some(TrainerSkillReq {
                        name: line.name().to_string(),
                        rank: u32::from(recipes::SPEC_SKILL),
                        met: r.rank >= recipes::SPEC_SKILL,
                    }),
                    ..service(
                        spec.spell(),
                        spec.name().to_string(),
                        String::new(),
                        recipes::SPEC_COST,
                    )
                });
            }
        }
        for rec in recipes::book(line) {
            let Learn::Trainer(cost) = rec.learn else {
                continue;
            };
            let product = super::items::template(rec.product);
            services.push(TrainerService {
                texture: product
                    .map(super::items::icon_path)
                    .or_else(|| Some(recipes::ENCHANT_ICON.to_string())),
                description: format!("Teaches you how to make {}.", rec.name),
                category: category(recipes::known(rec, skills), recipes::learnable(rec, skills)),
                skill_req: Some(TrainerSkillReq {
                    name: line.name().to_string(),
                    rank: u32::from(rec.req),
                    met: r.rank >= rec.req,
                }),
                ..service(rec.spell, rec.name.to_string(), String::new(), cost)
            });
        }
    }
    TrainerState {
        services,
        greeting: TRAINER_GREETING.to_string(),
        trainer_type: TRAINER_TYPE_TRADESKILL,
        groups: Vec::new(),
    }
}

/// What a `BuyTrainerService` bought.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Trained {
    /// A rank: the line and the step.
    Rank(Line, usize),
    /// A recipe.
    Recipe(&'static Recipe),
    /// An Engineering specialisation.
    Spec(recipes::Spec),
}

/// One `BuyTrainerService`: a rank spell's line and step, a recipe or a specialisation, paid from
/// the purse.
pub(crate) fn train(
    skills: &mut Skills,
    purse: &mut Purse,
    spell: u32,
) -> Result<Trained, TrainError> {
    if let Some(spec) = recipes::Spec::of_spell(spell) {
        if skills.learned(spec.spell()) || skills.learned(spec.rival().spell()) {
            return Err(TrainError::NotNext);
        }
        if !recipes::spec_open(skills) {
            return Err(TrainError::LowSkill);
        }
        if !purse.spend(u64::from(recipes::SPEC_COST)) {
            return Err(TrainError::NoMoney);
        }
        skills.learn(spec.spell());
        return Ok(Trained::Spec(spec));
    }
    if let Some(rec) = recipes::recipe(spell) {
        let Learn::Trainer(cost) = rec.learn else {
            return Err(TrainError::NotNext);
        };
        if recipes::known(rec, skills) {
            return Err(TrainError::NotNext);
        }
        if !recipes::learnable(rec, skills) {
            return Err(TrainError::LowSkill);
        }
        if !purse.spend(u64::from(cost)) {
            return Err(TrainError::NoMoney);
        }
        skills.learn(rec.spell);
        return Ok(Trained::Recipe(rec));
    }
    let (line, step) = Line::ALL
        .iter()
        .find_map(|&l| {
            l.rank_spells()
                .iter()
                .position(|&s| s == spell)
                .map(|st| (l, st))
        })
        .ok_or(TrainError::NotNext)?;
    skills.can_train(line, step)?;
    if !purse.spend(u64::from(TRAINER_RANKS[step].cost)) {
        return Err(TrainError::NoMoney);
    }
    skills.train(line, step)?;
    Ok(Trained::Rank(line, step))
}

/// The trainer's `ClassTrainerFrame`.
fn trainer_window(
    script: &mut UiScript,
    shown: &mut Option<(u64, u64)>,
    at_camp: bool,
    desk: &mut CampDesk,
    purse: &mut Purse,
    skills: &mut Skills,
    lines: &mut Vec<CastLine>,
) {
    if script.take_trainer_close() {
        desk.trainer = false;
    }
    let buys = script.take_trainer_buys();
    if shown.is_some() {
        for spell in buys {
            match train(skills, purse, spell) {
                Ok(Trained::Rank(line, step)) => lines.push(CastLine::Learned(line, step)),
                Ok(Trained::Recipe(rec)) => lines.push(CastLine::LearnedRecipe(rec.name)),
                Ok(Trained::Spec(spec)) => lines.push(CastLine::LearnedRecipe(spec.name())),
                Err(TrainError::NoMoney) => refuse(lines, ShopError::NoMoney),
                Err(e) => debug!("survivors: trainer refused {spell}: {e:?}"),
            }
        }
    }
    let want = desk.trainer && at_camp;
    let key = (skills.generation(), purse.generation());
    match (*shown, want) {
        (None, true) => {
            seat_npc(script, TRAINER_NAME);
            script.reset_trainer_list_state(TRAINER_TYPE_TRADESKILL);
            script.set_trainer(Some(trainer_state(skills)));
            script.fire_event(
                "TRAINER_SHOW",
                vec![ScriptValue::Str(TRAINER_NAME.to_string())],
            );
            *shown = Some(key);
        }
        (Some(was), true) if was != key => {
            script.set_trainer(Some(trainer_state(skills)));
            script.fire_event(
                "TRAINER_UPDATE",
                vec![ScriptValue::Str(TRAINER_NAME.to_string())],
            );
            *shown = Some(key);
        }
        (Some(_), false) => {
            script.set_trainer(None);
            script.fire_event("TRAINER_CLOSED", vec![]);
            let _ = script.take_trainer_close();
            desk.trainer = false;
            *shown = None;
        }
        _ => {}
    }
}

/// What a recipe cast reads and writes.
struct Crafting<'a> {
    bags: &'a mut Bags,
    skills: &'a mut Skills,
    crafter: &'a mut Crafter,
    rng: &'a mut Rng,
    lines: &'a mut Vec<CastLine>,
    gear: &'a mut dyn GearHook,
}

impl Crafting<'_> {
    /// This frame's `DoTradeSkill`s and `DoCraft`s, each a cast if its window is up and no cast
    /// runs.
    fn drain(&mut self, script: &mut UiScript, book_open: bool, craft_open: bool) {
        for (spell, count) in script.take_trade_skill_dos() {
            if book_open && !self.crafter.busy() && self.start(spell) {
                self.crafter.repeat = count.saturating_sub(1);
            }
        }
        for spell in script.take_craft_dos() {
            if craft_open && !self.crafter.busy() {
                self.start(spell);
            }
        }
    }

    /// Start a cast of `spell`, if it is known, the bags can feed it and, for an enchant, a piece
    /// is worn to take it.
    fn start(&mut self, spell: u32) -> bool {
        let Some(r) = recipes::recipe(spell) else {
            return false;
        };
        if !recipes::known(r, self.skills) || recipes::available(self.bags, r) == 0 {
            return false;
        }
        if let Some(e) = r.enchant {
            if !self.gear.wears(e.slot) {
                self.lines.push(CastLine::Refused(
                    "SPELL_FAILED_EQUIPPED_ITEM",
                    "You must wear a piece in that slot to enchant it.",
                ));
                return false;
            }
        }
        self.crafter.current = Some((spell, r.cast_secs()));
        self.lines.push(CastLine::Cast(r.name, r.cast_secs()));
        true
    }

    /// The cast's clock: an interrupt off the camp, the product (or the enchant) on the finish,
    /// then the next repeat.
    fn tick(&mut self, at_camp: bool, dt: f32) {
        let Some((spell, left)) = self.crafter.current else {
            return;
        };
        if !at_camp {
            self.crafter.current = None;
            self.crafter.repeat = 0;
            self.lines.push(CastLine::Interrupted);
            return;
        }
        let left = left - dt;
        if left > 0.0 {
            self.crafter.current = Some((spell, left));
            return;
        }
        self.crafter.current = None;
        self.lines.push(CastLine::Stop);
        let Some(r) = recipes::recipe(spell) else {
            return;
        };
        match recipes::craft_once(self.bags, r) {
            Ok(()) => {
                match r.enchant {
                    Some(e) => {
                        self.gear.enchant(e);
                        self.lines.push(CastLine::Enchanted(e.name));
                    }
                    None => self.lines.push(CastLine::Created(r.product, r.made)),
                }
                let roll = self.rng.index(100) as u32;
                if let Some(rank) = self.skills.crafted(r.line, r.trivial.0, r.trivial.1, roll) {
                    self.lines.push(CastLine::SkillUp(r.line, rank));
                }
            }
            Err(e) => {
                self.crafter.repeat = 0;
                refuse(self.lines, ShopError::Bags(e));
                return;
            }
        }
        if self.crafter.repeat > 0 {
            self.crafter.repeat -= 1;
            if !self.start(spell) {
                self.crafter.repeat = 0;
            }
        }
    }
}

/// The open book's `TradeSkillFrame`: Smelting, Alchemy, Cooking or First Aid.
fn book_window(
    script: &mut UiScript,
    shown: &mut Option<(Line, u64, u64, u32)>,
    at_camp: bool,
    desk: &mut CampDesk,
    s: &Crafting,
) {
    if script.take_trade_skill_close() && desk.book != Some(Line::Enchanting) {
        desk.book = None;
    }
    let touched = script.take_trade_skill_touched();
    let want = desk
        .book
        .filter(|&l| at_camp && l != Line::Enchanting && !recipes::book(l).is_empty());
    let state = |line: Line| recipes::trade_skill_state(s.bags, s.skills, line, s.crafter.repeat);
    match (*shown, want) {
        (None, Some(line)) => {
            script.set_trade_skill(Some(state(line)));
            script.fire_event("TRADE_SKILL_SHOW", vec![]);
        }
        (Some((was, ..)), Some(line)) if was != line => {
            // Another book: the window shows it in place of the last.
            script.set_trade_skill(Some(state(line)));
            script.fire_event("TRADE_SKILL_SHOW", vec![]);
        }
        (Some(key), Some(line)) if key != book_key(line, s) => {
            script.set_trade_skill(Some(state(line)));
            script.fire_event("TRADE_SKILL_UPDATE", vec![]);
        }
        (Some(_), Some(_)) if touched => script.fire_event("TRADE_SKILL_UPDATE", vec![]),
        (Some(_), None) => {
            script.set_trade_skill(None);
            script.fire_event("TRADE_SKILL_CLOSE", vec![]);
            let _ = script.take_trade_skill_close();
            if desk.book != Some(Line::Enchanting) {
                desk.book = None;
            }
        }
        _ => {}
    }
    *shown = want.map(|line| book_key(line, s));
}

/// What a book's window was last pushed from.
fn book_key(line: Line, s: &Crafting) -> (Line, u64, u64, u32) {
    (
        line,
        s.bags.generation(),
        s.skills.generation(),
        s.crafter.repeat,
    )
}

/// Enchanting's `CraftFrame`.
fn craft_window(
    script: &mut UiScript,
    shown: &mut Option<(u64, u64)>,
    at_camp: bool,
    desk: &mut CampDesk,
    s: &Crafting,
) {
    if script.take_craft_close() && desk.book == Some(Line::Enchanting) {
        desk.book = None;
    }
    let want = at_camp && desk.book == Some(Line::Enchanting);
    let key = (s.bags.generation(), s.skills.generation());
    match (*shown, want) {
        (None, true) => {
            script.set_craft(Some(recipes::craft_state(s.bags, s.skills)));
            script.fire_event("CRAFT_SHOW", vec![]);
            *shown = Some(key);
        }
        (Some(was), true) if was != key => {
            script.set_craft(Some(recipes::craft_state(s.bags, s.skills)));
            script.fire_event("CRAFT_UPDATE", vec![]);
            *shown = Some(key);
        }
        (Some(_), false) => {
            script.set_craft(None);
            script.fire_event("CRAFT_CLOSE", vec![]);
            let _ = script.take_craft_close();
            if desk.book == Some(Line::Enchanting) {
                desk.book = None;
            }
            *shown = None;
        }
        _ => {}
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::survivors::professions::items::{
        COPPER_BAR, COPPER_ORE, EMPTY_VIAL, LINEN_BANDAGE, LINEN_CLOTH, MINOR_HEALING_POTION,
        PEACEBLOOM, SILVERLEAF, STRANGE_DUST,
    };
    use crate::survivors::professions::skills::Rank;

    /// A picked class's gear for the tests: what is worn, and the enchants put on.
    #[derive(Default)]
    pub(crate) struct TestGear {
        worn: Vec<Slot>,
        enchanted: Vec<(Slot, u32)>,
    }

    impl TestGear {
        /// A piece worn in every slot.
        pub(crate) fn wear_everything(&mut self) {
            self.worn = Slot::ALL.to_vec();
        }

        /// How many enchants went on.
        pub(crate) fn enchants(&self) -> usize {
            self.enchanted.len()
        }
    }

    impl GearHook for TestGear {
        fn wears(&self, slot: Slot) -> bool {
            self.worn.contains(&slot)
        }

        fn enchant(&mut self, e: &'static Enchant) {
            self.enchanted.push((e.slot, e.id));
        }
    }

    /// Everything a frame borrows, owned.
    #[derive(Default)]
    pub(crate) struct World {
        pub(crate) merchant: bool,
        pub(crate) desk: CampDesk,
        pub(crate) bags: Bags,
        pub(crate) purse: Purse,
        pub(crate) shop: Shop,
        pub(crate) skills: Skills,
        pub(crate) crafter: Crafter,
        pub(crate) rng: Rng,
        pub(crate) lines: Vec<CastLine>,
        pub(crate) gear: TestGear,
        pub(crate) shown: Shown,
    }

    impl World {
        pub(crate) fn frame(&mut self, script: &mut UiScript, at_camp: bool, dt: f32) {
            drive(
                script,
                &mut self.shown,
                Frame {
                    at_camp,
                    merchant: &mut self.merchant,
                    desk: &mut self.desk,
                    bags: &mut self.bags,
                    purse: &mut self.purse,
                    shop: &mut self.shop,
                    skills: &mut self.skills,
                    crafter: &mut self.crafter,
                    rng: &mut self.rng,
                    lines: &mut self.lines,
                    gear: &mut self.gear,
                    dt,
                },
            );
        }
    }

    fn events(s: &mut UiScript) {
        s.run(
            "SEEN = {} \
             local f = CreateFrame('Frame') \
             for _, e in ipairs({'MERCHANT_SHOW', 'MERCHANT_UPDATE', 'MERCHANT_CLOSED', \
               'TRAINER_SHOW', 'TRAINER_UPDATE', 'TRAINER_CLOSED', 'TRADE_SKILL_SHOW', \
               'TRADE_SKILL_UPDATE', 'TRADE_SKILL_CLOSE', 'CRAFT_SHOW', 'CRAFT_UPDATE', \
               'CRAFT_CLOSE'}) do f:RegisterEvent(e) end \
             f:SetScript('OnEvent', function() table.insert(SEEN, event) end)",
        )
        .unwrap();
    }

    fn seen(s: &UiScript) -> Vec<String> {
        let v: Vec<String> = s.eval("return SEEN").unwrap();
        s.run("SEEN = {}").unwrap();
        v
    }

    /// The merchant opens with the goblin, sells a right-clicked stack, buys, and closes with the
    /// camp.
    #[test]
    fn survivors_stations_the_merchant_buys_and_sells() {
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World::default();
        w.bags.add(COPPER_ORE, 10);
        let mut feed = crate::survivors::professions::frames::BagFeed::default();
        crate::survivors::professions::frames::sync(&mut s, &mut w.bags, 0, &mut feed);
        w.frame(&mut s, true, 0.016);
        assert!(seen(&s).is_empty(), "nothing opens unasked");
        w.merchant = true;
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["MERCHANT_SHOW"]);
        assert!(w.shop.open);
        assert_eq!(
            s.eval::<i64>("return GetMerchantNumItems()").unwrap(),
            econ::GOBLIN_WARES.len() as i64
        );
        // A cursor sell: the ore goes for 50c.
        s.run("PickupContainerItem(0, 1) PickupMerchantItem(0)")
            .unwrap();
        w.frame(&mut s, true, 0.016);
        assert_eq!(w.bags.count(COPPER_ORE), 0);
        assert_eq!(w.purse.copper(), 50);
        assert_eq!(seen(&s), vec!["MERCHANT_UPDATE"]);
        // Empty Vials at 4c.
        s.run("BuyMerchantItem(2, 3)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert_eq!(w.purse.copper(), 38);
        // Coal at 5s is refused, in the stock words.
        s.run("BuyMerchantItem(1, 1)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert_eq!(w.purse.copper(), 38);
        assert!(w.lines.contains(&CastLine::Refused(
            "ERR_NOT_ENOUGH_MONEY",
            "You don't have enough money."
        )));
        seen(&s);
        // The camp ends: the window closes, and the goblin's with it.
        w.frame(&mut s, false, 0.016);
        assert_eq!(seen(&s), vec!["MERCHANT_CLOSED"]);
        assert!(!w.merchant && !w.shop.open);
        assert_eq!(s.eval::<i64>("return GetMerchantNumItems()").unwrap(), 0);
    }

    /// The trainer sells Journeyman once the skill and the purse allow it, and a recipe at its
    /// skill.
    #[test]
    fn survivors_stations_the_trainer_sells_ranks_and_recipes() {
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World {
            skills: Skills::from_save(&std::collections::BTreeMap::from([
                ("mining".to_string(), Rank { rank: 60, max: 75 }),
                ("alchemy".to_string(), Rank { rank: 85, max: 150 }),
            ])),
            ..Default::default()
        };
        w.desk.trainer = true;
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRAINER_SHOW"]);
        let state = trainer_state(&w.skills);
        let find = |spell: u32| {
            state
                .services
                .iter()
                .find(|sv| sv.spell_id == spell)
                .unwrap()
                .category
        };
        assert_eq!(find(2576), TrainerServiceCategory::Available, "Journeyman");
        assert_eq!(find(2368), TrainerServiceCategory::Unavailable);
        assert_eq!(
            find(3101),
            TrainerServiceCategory::Used,
            "Journeyman Alchemy"
        );
        assert_eq!(find(7836), TrainerServiceCategory::Available, "at 80");
        assert_eq!(find(6624), TrainerServiceCategory::Unavailable, "at 150");
        assert_eq!(
            train(&mut w.skills, &mut w.purse, 2576),
            Err(TrainError::NoMoney)
        );
        w.purse.earn(12_000);
        assert_eq!(
            train(&mut w.skills, &mut w.purse, 2576),
            Ok(Trained::Rank(Line::Mining, 1))
        );
        assert_eq!(w.purse.copper(), 2_000);
        assert_eq!(w.skills.rank(Line::Mining).max, 150);
        assert!(matches!(
            train(&mut w.skills, &mut w.purse, 7836),
            Ok(Trained::Recipe(r)) if r.name == "Blackmouth Oil"
        ));
        assert_eq!(w.purse.copper(), 1_600);
        assert_eq!(
            train(&mut w.skills, &mut w.purse, 7836),
            Err(TrainError::NotNext),
            "known"
        );
        assert_eq!(
            train(&mut w.skills, &mut w.purse, 2330),
            Err(TrainError::NotNext),
            "not the trainer's"
        );
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRAINER_UPDATE"]);
        w.frame(&mut s, false, 0.016);
        assert_eq!(seen(&s), vec!["TRAINER_CLOSED"]);
        assert!(!w.desk.trainer, "the next camp starts closed");
    }

    /// Create All on Smelt Copper: a cast a bar, each 3 s, while the ore lasts.
    #[test]
    fn survivors_stations_smelting_casts_and_repeats() {
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World::default();
        w.bags.add(COPPER_ORE, 2);
        w.desk.toggle(Line::Mining);
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRADE_SKILL_SHOW"]);
        assert_eq!(
            s.eval::<i64>("return GetNumTradeSkills()").unwrap(),
            2,
            "a header and Smelt Copper"
        );
        s.run("DoTradeSkill(2, 5)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(w.crafter.busy());
        assert_eq!(
            w.lines,
            vec![CastLine::Cast("Smelt Copper", recipes::SMELT_SECS)]
        );
        w.lines.clear();
        w.frame(&mut s, true, recipes::SMELT_SECS);
        assert_eq!(w.bags.count(COPPER_BAR), 1);
        assert!(w.lines.contains(&CastLine::Created(COPPER_BAR, 1)));
        assert!(w.crafter.busy(), "the second cast");
        w.frame(&mut s, true, recipes::SMELT_SECS);
        assert_eq!((w.bags.count(COPPER_ORE), w.bags.count(COPPER_BAR)), (0, 2));
        assert!(!w.crafter.busy(), "out of ore");
        assert!(w.skills.rank(Line::Mining).rank > 1, "orange: always rises");
        // An interrupted cast makes nothing.
        w.bags.add(COPPER_ORE, 1);
        s.run("DoTradeSkill(2, 1)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(w.crafter.busy());
        w.frame(&mut s, false, recipes::SMELT_SECS);
        assert!(!w.crafter.busy());
        assert_eq!(w.bags.count(COPPER_ORE), 1);
        assert!(w.lines.contains(&CastLine::Interrupted));
    }

    /// The Alchemy button shows Alchemy's book in the same window; a potion brews; First Aid's
    /// bandage too; the camp's end closes it.
    #[test]
    fn survivors_stations_alchemy_and_first_aid_books() {
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World::default();
        w.bags.add(PEACEBLOOM, 1);
        w.bags.add(SILVERLEAF, 1);
        w.bags.add(EMPTY_VIAL, 1);
        w.bags.add(LINEN_CLOTH, 1);
        w.desk.toggle(Line::Alchemy);
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRADE_SKILL_SHOW"]);
        let row = s
            .eval::<i64>(
                "for i = 1, GetNumTradeSkills() do \
                   if GetTradeSkillInfo(i) == 'Minor Healing Potion' then return i end \
                 end return 0",
            )
            .unwrap();
        assert!(row > 0, "Minor Healing Potion is listed");
        s.run(&format!("DoTradeSkill({row}, 1)")).unwrap();
        w.frame(&mut s, true, 0.016);
        w.frame(&mut s, true, 3.0);
        assert_eq!(w.bags.count(MINOR_HEALING_POTION), 1);
        assert_eq!(w.skills.rank(Line::Alchemy).rank, 2);
        seen(&s);
        // Another book in the same window.
        w.desk.toggle(Line::FirstAid);
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRADE_SKILL_SHOW"]);
        let row = s
            .eval::<i64>(
                "for i = 1, GetNumTradeSkills() do \
                   if GetTradeSkillInfo(i) == 'Linen Bandage' then return i end \
                 end return 0",
            )
            .unwrap();
        s.run(&format!("DoTradeSkill({row}, 1)")).unwrap();
        w.frame(&mut s, true, 0.016);
        w.frame(&mut s, true, 3.0);
        assert_eq!(w.bags.count(LINEN_BANDAGE), 1);
        seen(&s);
        w.frame(&mut s, false, 0.016);
        assert_eq!(seen(&s), vec!["TRADE_SKILL_CLOSE"]);
        assert_eq!(w.desk.book, None);
    }

    /// The smith's book at camp: Copper Bracers from two bars, a bind-on-equip piece of gear in
    /// the bags; then the tailor's, a Linen Bag from bolts and thread.
    #[test]
    fn survivors_stations_the_smith_and_the_tailor_make_gear_and_bags() {
        use crate::survivors::professions::items::{
            BOLT_OF_LINEN_CLOTH, COARSE_THREAD, COPPER_BRACERS, LINEN_BAG,
        };
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World::default();
        w.bags.add(COPPER_BAR, 2);
        w.bags.add(BOLT_OF_LINEN_CLOTH, 3);
        w.bags.add(COARSE_THREAD, 3);
        w.desk.toggle(Line::Blacksmithing);
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRADE_SKILL_SHOW"]);
        let row = s
            .eval::<i64>(
                "for i = 1, GetNumTradeSkills() do \
                   if GetTradeSkillInfo(i) == 'Copper Bracers' then return i end \
                 end return 0",
            )
            .unwrap();
        assert!(row > 0, "Copper Bracers at 1");
        s.run(&format!("DoTradeSkill({row}, 1)")).unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(w.crafter.busy());
        w.frame(&mut s, true, 5.2);
        assert_eq!(w.bags.count(COPPER_BRACERS), 1);
        assert_eq!(
            super::super::items::template(COPPER_BRACERS).map(|d| d.bonding),
            Some(2),
            "binds when equipped"
        );
        w.desk.toggle(Line::Tailoring);
        w.skills = Skills::from_save(&std::collections::BTreeMap::from([(
            "tailoring".to_string(),
            Rank { rank: 60, max: 75 },
        )]));
        w.frame(&mut s, true, 0.016);
        let row = s
            .eval::<i64>(
                "for i = 1, GetNumTradeSkills() do \
                   if GetTradeSkillInfo(i) == 'Linen Bag' then return i end \
                 end return 0",
            )
            .unwrap();
        assert!(row > 0, "Linen Bag at 45");
        s.run(&format!("DoTradeSkill({row}, 1)")).unwrap();
        w.frame(&mut s, true, 0.016);
        w.frame(&mut s, true, 8.1);
        assert_eq!(w.bags.count(LINEN_BAG), 1);
    }

    /// The trainer sells Engineering's specialisations at 200, one of them, never both.
    #[test]
    fn survivors_stations_the_trainer_sells_one_engineering_specialisation() {
        use recipes::Spec;
        let mut w = World {
            skills: Skills::from_save(&std::collections::BTreeMap::from([(
                "engineering".to_string(),
                Rank {
                    rank: 210,
                    max: 225,
                },
            )])),
            ..Default::default()
        };
        let category_of = |skills: &Skills, spell: u32| {
            trainer_state(skills)
                .services
                .into_iter()
                .find(|sv| sv.spell_id == spell)
                .unwrap()
                .category
        };
        assert_eq!(
            category_of(&w.skills, Spec::Goblin.spell()),
            TrainerServiceCategory::Available
        );
        assert_eq!(
            train(&mut w.skills, &mut w.purse, Spec::Gnomish.spell()),
            Err(TrainError::NoMoney)
        );
        w.purse.earn(u64::from(recipes::SPEC_COST));
        assert_eq!(
            train(&mut w.skills, &mut w.purse, Spec::Gnomish.spell()),
            Ok(Trained::Spec(Spec::Gnomish))
        );
        assert_eq!(w.purse.copper(), 0);
        w.purse.earn(u64::from(recipes::SPEC_COST));
        assert_eq!(
            train(&mut w.skills, &mut w.purse, Spec::Goblin.spell()),
            Err(TrainError::NotNext),
            "one or the other"
        );
        assert_eq!(
            category_of(&w.skills, Spec::Gnomish.spell()),
            TrainerServiceCategory::Used
        );
        assert_eq!(
            category_of(&w.skills, Spec::Goblin.spell()),
            TrainerServiceCategory::Unavailable
        );
    }

    /// Enchanting's craft window: an enchant needs a piece worn in its slot, takes its dust, and
    /// goes on that piece.
    #[test]
    fn survivors_stations_enchanting_enchants_the_worn_piece() {
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World::default();
        w.bags.add(STRANGE_DUST, 2);
        w.desk.toggle(Line::Enchanting);
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["CRAFT_SHOW"]);
        let row = s
            .eval::<i64>(
                "for i = 1, GetNumCrafts() do \
                   if GetCraftInfo(i) == 'Enchant Bracer - Minor Health' then return i end \
                 end return 0",
            )
            .unwrap();
        assert!(row > 0, "listed at 1");
        s.run(&format!("DoCraft({row})")).unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(!w.crafter.busy(), "no bracer worn");
        assert!(matches!(w.lines.last(), Some(CastLine::Refused(..))));
        w.gear.worn.push(Slot::Wrist);
        s.run(&format!("DoCraft({row})")).unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(w.crafter.busy());
        w.frame(&mut s, true, 5.0);
        assert_eq!(w.gear.enchanted, vec![(Slot::Wrist, 41)]);
        assert_eq!(w.bags.count(STRANGE_DUST), 1);
        assert!(w.lines.contains(&CastLine::Enchanted("Health +5")));
        seen(&s);
        w.desk.toggle(Line::Enchanting);
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["CRAFT_CLOSE"]);
    }
}
