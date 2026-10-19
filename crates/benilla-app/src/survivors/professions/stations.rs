//! The camp's stock windows (docs/PROFESSIONS.md, 2.1, 5.4): the goblin's `MerchantFrame`, the
//! gathering trainer's `ClassTrainerFrame` and Smelting's `TradeSkillFrame`, each pushed from the
//! local [`Bags`], [`Purse`] and [`Skills`], each drained here (the stock net drains stand down
//! under [`crate::run_mode::SurvivorsUi`]). Everything is this player's own, so nothing goes on the
//! wire.
//!
//! - **The merchant** opens with the goblin's own window (a click on him, `campfire`), beside it:
//!   his goods for the purse, any bag item sold for its `sell_price` (a right-click on a bag slot,
//!   or a drop on the window), and the buyback tab. Closing either window closes both.
//! - **The trainer** and **Smelting** open from the camp's Train and Smelt buttons. The trainer
//!   sells Journeyman, Expert and Artisan on each gathering line; Smelting casts for 3 s at the
//!   camp's fire, the stock casting bar showing it, takes the ore and makes the bar, and rolls
//!   Mining on the recipe's bands. Create All repeats while the ore lasts.
//! - All three close as the camp ends or a modal survivors screen opens.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use benilla_ui::script::{
    ScriptValue, TrainerService, TrainerServiceCategory, TrainerSkillReq, TrainerState,
    TrainerTooltip, UiScript, UnitState,
};

use super::super::campfire::{Camp, MerchantWindow};
use super::super::{Phase, Rng, Run};
use super::econ::{self, Purse, Shop, ShopError, Wallet};
use super::gather::{CastLine, UiLines};
use super::inventory::Bags;
use super::recipes::{self, SMELT_SECS};
use super::skills::{Line, Skills, TrainError, TRAINER_RANKS};
use crate::ui_script::VmMemo;

/// The 1.12 tradeskill trainer type (`SMSG_TRAINER_LIST`'s 2).
const TRAINER_TYPE_TRADESKILL: u32 = 2;
/// The names the windows' titles read off the `npc` unit.
const MERCHANT_NAME: &str = "Goblin Merchant";
const TRAINER_NAME: &str = "Gathering Trainer";
const TRAINER_GREETING: &str =
    "Ore, herbs, hides and fish: show me what you have gathered, and I will teach you to gather more.";

/// The camp's Train and Smelt buttons ask for their windows here; each press toggles.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CampDesk {
    pub(crate) trainer: bool,
    pub(crate) smelting: bool,
}

/// A Smelting cast in progress, and the Create All repeats left after it.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct Smelter {
    current: Option<(u32, f32)>,
    repeat: u32,
}

impl Smelter {
    pub(crate) fn busy(&self) -> bool {
        self.current.is_some()
    }
}

/// What this VM was last shown, so a window opens, updates and closes once each.
#[derive(Default)]
pub(crate) struct Shown {
    merchant: Option<(u64, u64)>,
    trainer: Option<(u64, u64)>,
    smelting: Option<(u64, u64, u32)>,
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
    pub(crate) smelter: &'a mut Smelter,
    pub(crate) rng: &'a mut Rng,
    pub(crate) lines: &'a mut Vec<CastLine>,
    pub(crate) dt: f32,
}

/// Where the run stands.
#[derive(SystemParam)]
pub(super) struct Where<'w> {
    phase: Res<'w, State<Phase>>,
    run: Res<'w, Run>,
    camp: Res<'w, Camp>,
    time: Res<'w, Time>,
}

/// The crafting half's state.
#[derive(SystemParam)]
pub(super) struct Craft<'w> {
    skills: ResMut<'w, Skills>,
    smelter: ResMut<'w, Smelter>,
    rng: ResMut<'w, Rng>,
    ui: ResMut<'w, UiLines>,
}

/// `Update`, after the input pass and before the bag push: the three windows' intents, casts and
/// pushes.
pub(super) fn run_stations(
    script: Option<NonSendMut<UiScript>>,
    at: Where,
    window: Option<ResMut<MerchantWindow>>,
    mut desk: ResMut<CampDesk>,
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
        smelter,
        rng,
        ui,
    } = &mut craft;
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
            smelter,
            rng,
            lines: &mut ui.0,
            dt: at.time.delta_secs(),
        },
    );
    if let Some(mut w) = window {
        if w.open != merchant_open {
            w.open = merchant_open;
        }
    }
}

/// One frame of the three windows.
pub(crate) fn drive(script: &mut UiScript, shown: &mut Shown, f: Frame) {
    let Frame {
        at_camp,
        merchant,
        desk,
        bags,
        purse,
        shop,
        skills,
        smelter,
        rng,
        lines,
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
    let ctx = Smelting {
        bags,
        skills,
        smelter,
        rng,
        lines,
    };
    smelting_window(script, &mut shown.smelting, at_camp, desk, ctx, dt);
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

/// The trainer's list: Apprentice to Artisan on each gathering line, grouped by line. A known rank
/// is "used", the next one "available" once the skill reaches it, the rest "unavailable".
pub(crate) fn trainer_state(skills: &Skills) -> TrainerState {
    let mut services = Vec::new();
    for line in Line::ALL {
        let r = skills.rank(line);
        for (step, &spell) in line.rank_spells().iter().enumerate() {
            let rank = TRAINER_RANKS[step];
            let category = if step <= r.step() {
                TrainerServiceCategory::Used
            } else if skills.can_train(line, step).is_ok() {
                TrainerServiceCategory::Available
            } else {
                TrainerServiceCategory::Unavailable
            };
            services.push(TrainerService {
                spell_id: spell,
                name: Some(line.spell_name().to_string()),
                subtext: Some(rank.name.to_string()),
                texture: Some(line.icon().to_string()),
                description: format!("Allows {} to reach {} skill.", line.name(), rank.cap),
                cost: rank.cost,
                prof_first_rank: false,
                category,
                level_req: 0,
                skill_req: (rank.needs > 0).then(|| TrainerSkillReq {
                    name: line.name().to_string(),
                    rank: u32::from(rank.needs),
                    met: r.rank >= rank.needs,
                }),
                ability_reqs: Vec::new(),
                is_trade_skill: true,
                group_key: line.skill_line(),
                group_name: line.name().to_string(),
                tooltip: TrainerTooltip::Spell {
                    spell_id: spell,
                    alt_caster: false,
                },
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

/// One `BuyTrainerService`: the rank spell's line and step, paid from the purse.
pub(crate) fn train(
    skills: &mut Skills,
    purse: &mut Purse,
    spell: u32,
) -> Result<(Line, usize), TrainError> {
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
    Ok((line, step))
}

/// The gathering trainer's `ClassTrainerFrame`.
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
                Ok((line, step)) => lines.push(CastLine::Learned(line, step)),
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

/// What a Smelting cast reads and writes.
struct Smelting<'a> {
    bags: &'a mut Bags,
    skills: &'a mut Skills,
    smelter: &'a mut Smelter,
    rng: &'a mut Rng,
    lines: &'a mut Vec<CastLine>,
}

impl Smelting<'_> {
    /// Start a cast of `spell`, if it is known and the bags can feed it.
    fn start(&mut self, spell: u32) -> bool {
        let Some(r) = recipes::recipe(spell) else {
            return false;
        };
        if !recipes::known(r, self.skills.rank(Line::Mining).rank)
            || recipes::available(self.bags, r) == 0
        {
            return false;
        }
        self.smelter.current = Some((spell, SMELT_SECS));
        self.lines.push(CastLine::Cast(r.name, SMELT_SECS));
        true
    }

    /// The cast's clock: an interrupt off the camp, the bar on the finish, then the next repeat.
    fn tick(&mut self, at_camp: bool, dt: f32) {
        let Some((spell, left)) = self.smelter.current else {
            return;
        };
        if !at_camp {
            self.smelter.current = None;
            self.smelter.repeat = 0;
            self.lines.push(CastLine::Interrupted);
            return;
        }
        let left = left - dt;
        if left > 0.0 {
            self.smelter.current = Some((spell, left));
            return;
        }
        self.smelter.current = None;
        self.lines.push(CastLine::Stop);
        let Some(r) = recipes::recipe(spell) else {
            return;
        };
        match recipes::smelt_once(self.bags, r) {
            Ok(()) => {
                self.lines.push(CastLine::Created(r.product, r.made));
                let roll = self.rng.index(100) as u32;
                if let Some(rank) =
                    self.skills
                        .crafted(Line::Mining, r.trivial.0, r.trivial.1, roll)
                {
                    self.lines.push(CastLine::SkillUp(Line::Mining, rank));
                }
            }
            Err(e) => {
                self.smelter.repeat = 0;
                refuse(self.lines, ShopError::Bags(e));
                return;
            }
        }
        if self.smelter.repeat > 0 {
            self.smelter.repeat -= 1;
            if !self.start(spell) {
                self.smelter.repeat = 0;
            }
        }
    }
}

/// Smelting's `TradeSkillFrame`.
fn smelting_window(
    script: &mut UiScript,
    shown: &mut Option<(u64, u64, u32)>,
    at_camp: bool,
    desk: &mut CampDesk,
    mut s: Smelting,
    dt: f32,
) {
    if script.take_trade_skill_close() {
        desk.smelting = false;
    }
    for (spell, count) in script.take_trade_skill_dos() {
        if shown.is_some() && !s.smelter.busy() && s.start(spell) {
            s.smelter.repeat = count.saturating_sub(1);
        }
    }
    s.tick(at_camp, dt);
    let touched = script.take_trade_skill_touched();
    let want = desk.smelting && at_camp;
    let key = (s.bags.generation(), s.skills.generation(), s.smelter.repeat);
    let state = || recipes::trade_skill_state(s.bags, s.skills, s.smelter.repeat);
    match (*shown, want) {
        (None, true) => {
            script.set_trade_skill(Some(state()));
            script.fire_event("TRADE_SKILL_SHOW", vec![]);
            *shown = Some(key);
        }
        (Some(was), true) if was != key => {
            script.set_trade_skill(Some(state()));
            script.fire_event("TRADE_SKILL_UPDATE", vec![]);
            *shown = Some(key);
        }
        (Some(_), true) if touched => script.fire_event("TRADE_SKILL_UPDATE", vec![]),
        (Some(_), false) => {
            script.set_trade_skill(None);
            script.fire_event("TRADE_SKILL_CLOSE", vec![]);
            let _ = script.take_trade_skill_close();
            desk.smelting = false;
            *shown = None;
        }
        _ => {}
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::survivors::professions::items::{COPPER_BAR, COPPER_ORE};
    use crate::survivors::professions::skills::Rank;

    /// Everything a frame borrows, owned.
    #[derive(Default)]
    pub(crate) struct World {
        pub(crate) merchant: bool,
        pub(crate) desk: CampDesk,
        pub(crate) bags: Bags,
        pub(crate) purse: Purse,
        pub(crate) shop: Shop,
        pub(crate) skills: Skills,
        pub(crate) smelter: Smelter,
        pub(crate) rng: Rng,
        pub(crate) lines: Vec<CastLine>,
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
                    smelter: &mut self.smelter,
                    rng: &mut self.rng,
                    lines: &mut self.lines,
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
               'TRADE_SKILL_UPDATE', 'TRADE_SKILL_CLOSE'}) do f:RegisterEvent(e) end \
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
        assert_eq!(s.eval::<i64>("return GetMerchantNumItems()").unwrap(), 3);
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

    /// The trainer sells Journeyman once the skill and the purse allow it.
    #[test]
    fn survivors_stations_the_trainer_sells_ranks() {
        let mut s = UiScript::new().unwrap();
        events(&mut s);
        let mut w = World {
            skills: Skills::from_save(&std::collections::BTreeMap::from([(
                "mining".to_string(),
                Rank { rank: 60, max: 75 },
            )])),
            ..Default::default()
        };
        w.desk.trainer = true;
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRAINER_SHOW"]);
        let state = trainer_state(&w.skills);
        let journeyman = state
            .services
            .iter()
            .find(|sv| sv.spell_id == 2576)
            .unwrap();
        assert_eq!(journeyman.category, TrainerServiceCategory::Available);
        let herb = state
            .services
            .iter()
            .find(|sv| sv.spell_id == 2368)
            .unwrap();
        assert_eq!(herb.category, TrainerServiceCategory::Unavailable);
        assert_eq!(
            train(&mut w.skills, &mut w.purse, 2576),
            Err(TrainError::NoMoney)
        );
        w.purse.earn(12_000);
        assert_eq!(
            train(&mut w.skills, &mut w.purse, 2576),
            Ok((Line::Mining, 1))
        );
        assert_eq!(w.purse.copper(), 2_000);
        assert_eq!(w.skills.rank(Line::Mining).max, 150);
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
        w.desk.smelting = true;
        w.frame(&mut s, true, 0.016);
        assert_eq!(seen(&s), vec!["TRADE_SKILL_SHOW"]);
        assert_eq!(
            s.eval::<i64>("return GetNumTradeSkills()").unwrap(),
            2,
            "a header and Smelt Copper"
        );
        s.run("DoTradeSkill(2, 5)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(w.smelter.busy());
        assert_eq!(w.lines, vec![CastLine::Cast("Smelt Copper", SMELT_SECS)]);
        w.lines.clear();
        w.frame(&mut s, true, SMELT_SECS);
        assert_eq!(w.bags.count(COPPER_BAR), 1);
        assert!(w.lines.contains(&CastLine::Created(COPPER_BAR, 1)));
        assert!(w.smelter.busy(), "the second cast");
        w.frame(&mut s, true, SMELT_SECS);
        assert_eq!((w.bags.count(COPPER_ORE), w.bags.count(COPPER_BAR)), (0, 2));
        assert!(!w.smelter.busy(), "out of ore");
        assert!(w.skills.rank(Line::Mining).rank > 1, "orange: always rises");
        // An interrupted cast makes nothing.
        w.bags.add(COPPER_ORE, 1);
        s.run("DoTradeSkill(2, 1)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(w.smelter.busy());
        w.frame(&mut s, false, SMELT_SECS);
        assert!(!w.smelter.busy());
        assert_eq!(w.bags.count(COPPER_ORE), 1);
        assert!(w.lines.contains(&CastLine::Interrupted));
    }
}
