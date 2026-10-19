//! Gold (docs/PROFESSIONS.md, 5.4, 6.2): the profile's purse, banked from each run, and the camp
//! goblin's goods.
//!
//! - **The purse** ([`Purse`]) is 1.12 copper, saved as `[purse]`. The bags' money line (the
//!   Backpack's stock `MoneyFrame`) and the merchant and trainer windows show it.
//! - **Banking:** a run's gold is the party's shared run purse, counted in survivors' gold pieces.
//!   Every player's machine counts what the party collected ([`GoldTally`]: each rise of the run
//!   purse, so the camp's shopping never shrinks it), and when the run is recorded banks
//!   [`BANK_PERCENT`]% of it into its own purse, a gold piece being worth a silver
//!   ([`COPPER_PER_RUN_GOLD`]): a 250-gold run banks 62s 50c.
//! - **The goblin's goods** ([`GOBLIN_WARES`]) and every bag item's sale at its `sell_price`, with a
//!   12-row buyback list in memory ([`Shop`]). The tokens he sold before stay on his Bevy window and
//!   still spend the run purse.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use benilla_ui::script::{ItemStatsHead, MerchantItem, MerchantState};

use super::super::hud::{NoteKind, ResultsNotes, MAX_NOTES};
use super::super::profile::RunEnded;
use super::super::Run;
use super::inventory::{BagError, Bags, Stack};
use super::items::{
    self, ItemDef, COAL, CRYSTAL_VIAL, EMPTY_VIAL, HOT_SPICES, LEADED_VIAL, LINEN_BAG, MILD_SPICES,
    REFRESHING_SPRING_WATER, SOOTHING_SPICES,
};

/// A run's gold piece, in the purse's copper: a silver.
pub(crate) const COPPER_PER_RUN_GOLD: u64 = 100;
/// The share of the party's collected gold each player banks.
pub(crate) const BANK_PERCENT: u64 = 25;
/// The buyback list's rows, as 1.12's.
pub(crate) const BUYBACK_SLOTS: usize = 12;

/// What the goblin sells, `(item, price in copper)`: Smelt Steel's Coal, the potion recipes' three
/// vials, a Linen Bag, and Cooking's spices and water. New wares go on the end, so the rows before
/// keep their places. The prices are ours.
pub(crate) static GOBLIN_WARES: &[(u32, u32)] = &[
    (COAL, 500),
    (EMPTY_VIAL, 4),
    (LINEN_BAG, 400),
    (LEADED_VIAL, 40),
    (CRYSTAL_VIAL, 200),
    (MILD_SPICES, 10),
    (HOT_SPICES, 40),
    (SOOTHING_SPICES, 100),
    (REFRESHING_SPRING_WATER, 5),
];

/// The profile's gold, in copper.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Purse {
    copper: u64,
    /// Bumped on every change: the save's and the feeds' gate.
    generation: u64,
}

impl Purse {
    pub(crate) fn with(copper: u64) -> Self {
        Self {
            copper,
            generation: 0,
        }
    }

    pub(crate) fn copper(&self) -> u64 {
        self.copper
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn earn(&mut self, copper: u64) {
        if copper > 0 {
            self.copper = self.copper.saturating_add(copper);
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Pay `copper`, or refuse and change nothing.
    pub(crate) fn spend(&mut self, copper: u64) -> bool {
        if copper > self.copper {
            return false;
        }
        if copper > 0 {
            self.copper -= copper;
            self.generation = self.generation.wrapping_add(1);
        }
        true
    }
}

/// `[purse]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct PurseSave {
    pub(crate) copper: u64,
}

/// What `collected` run gold banks, in copper.
pub(crate) fn banked(collected: u32) -> u64 {
    u64::from(collected) * COPPER_PER_RUN_GOLD * BANK_PERCENT / 100
}

/// `12g 3s 45c`, as the money frames read: the empty denominations left out, `0c` for nothing.
pub(crate) fn coins(copper: u64) -> String {
    let (g, s, c) = (copper / 10_000, copper / 100 % 100, copper % 100);
    let mut parts = Vec::new();
    if g > 0 {
        parts.push(format!("{g}g"));
    }
    if s > 0 {
        parts.push(format!("{s}s"));
    }
    if c > 0 || parts.is_empty() {
        parts.push(format!("{c}c"));
    }
    parts.join(" ")
}

/// The party's gold collected this run, as this machine saw the run purse rise.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct GoldTally {
    last_gold: u32,
    last_elapsed: f32,
    pub(crate) collected: u32,
}

impl GoldTally {
    /// One look at the run purse: a rise is gold collected, a fall is shopping, and the clock
    /// running back is a new run.
    pub(crate) fn observe(&mut self, gold: u32, elapsed: f32) {
        if elapsed < self.last_elapsed {
            self.collected = 0;
            self.last_gold = gold.min(self.last_gold);
        }
        if gold > self.last_gold {
            self.collected = self.collected.saturating_add(gold - self.last_gold);
        }
        self.last_gold = gold;
        self.last_elapsed = elapsed;
    }
}

/// `Update`: watch the run purse.
pub(super) fn tally_gold(run: Res<Run>, mut tally: ResMut<GoldTally>) {
    tally.observe(run.gold, run.elapsed);
}

/// `Update`, on the frame a run is recorded: bank its share, and say so on the results screen.
pub(super) fn bank_run_gold(
    mut ended: MessageReader<RunEnded>,
    run: Res<Run>,
    mut tally: ResMut<GoldTally>,
    mut purse: ResMut<Purse>,
    notes: Option<ResMut<ResultsNotes>>,
) {
    if ended.read().count() == 0 {
        return;
    }
    // The last frame's pickups too.
    tally.observe(run.gold, run.elapsed);
    let copper = banked(std::mem::take(&mut tally.collected));
    if copper == 0 {
        return;
    }
    purse.earn(copper);
    info!("survivors: banked {} into the purse", coins(copper));
    if let Some(mut notes) = notes {
        let at = notes.0.len().min(MAX_NOTES - 1);
        notes.0.insert(
            at,
            (
                format!("You bank {} of the party's gold.", coins(copper)),
                NoteKind::Good,
            ),
        );
    }
}

/// Why the goblin refused, each with the stock `GlobalStrings.lua` key the client shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShopError {
    NoMoney,
    /// No sell price: the vendor does not want it.
    NotSellable,
    /// No such row, slot or buyback.
    NotFound,
    Bags(BagError),
}

impl ShopError {
    pub(crate) fn global_string(self) -> (&'static str, &'static str) {
        match self {
            ShopError::NoMoney => ("ERR_NOT_ENOUGH_MONEY", "You don't have enough money."),
            ShopError::NotSellable => (
                "ERR_VENDOR_DOESNT_BUY",
                "The merchant doesn't want that item.",
            ),
            ShopError::NotFound => ("ERR_ITEM_NOT_FOUND", "The item was not found."),
            ShopError::Bags(BagError::BagFull) => ("ERR_INV_FULL", "Inventory is full."),
            ShopError::Bags(e) => (e.global_string(), "Item not found."),
        }
    }
}

/// The goblin's open window: whether the stock `MerchantFrame` shows him, and the buyback list,
/// oldest first.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(crate) struct Shop {
    pub(crate) open: bool,
    buyback: Vec<(Stack, u64)>,
    generation: u64,
}

impl Shop {
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    fn touch(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

/// The bags, the purse and the shop together: every sale and purchase changes all three.
#[derive(SystemParam)]
pub(crate) struct Wallet<'w> {
    pub(crate) bags: ResMut<'w, Bags>,
    pub(crate) purse: ResMut<'w, Purse>,
    pub(crate) shop: ResMut<'w, Shop>,
}

/// Sell the stack at `(bag, slot)` whole, at its `sell_price` each, onto the buyback list.
pub(crate) fn sell(
    bags: &mut Bags,
    purse: &mut Purse,
    shop: &mut Shop,
    bag: i64,
    slot: u32,
) -> Result<u64, ShopError> {
    let stack = bags.get(bag, slot).ok_or(ShopError::NotFound)?;
    let def = items::template(stack.item).ok_or(ShopError::NotFound)?;
    if def.sell_price == 0 {
        return Err(ShopError::NotSellable);
    }
    let paid = u64::from(def.sell_price) * u64::from(stack.count);
    bags.destroy(bag, slot, 0).map_err(ShopError::Bags)?;
    purse.earn(paid);
    shop.buyback.push((stack, paid));
    if shop.buyback.len() > BUYBACK_SLOTS {
        shop.buyback.remove(0);
    }
    shop.touch();
    Ok(paid)
}

/// `BuyMerchantItem(row, quantity)`: `quantity` of the 1-based row's item.
pub(crate) fn buy(
    bags: &mut Bags,
    purse: &mut Purse,
    row: u32,
    quantity: u32,
) -> Result<(), ShopError> {
    let &(item, price) = usize::try_from(row)
        .ok()
        .and_then(|r| r.checked_sub(1))
        .and_then(|r| GOBLIN_WARES.get(r))
        .ok_or(ShopError::NotFound)?;
    let quantity = quantity.max(1);
    let cost = u64::from(price) * u64::from(quantity);
    if cost > purse.copper() {
        return Err(ShopError::NoMoney);
    }
    bags.exchange(&[], &[(item, quantity)])
        .map_err(ShopError::Bags)?;
    purse.spend(cost);
    Ok(())
}

/// A held merchant row dropped on a bag slot (`CMSG_BUY_ITEM_IN_SLOT`): one of `item`, stored as
/// the bags store it.
pub(crate) fn buy_entry(bags: &mut Bags, purse: &mut Purse, item: u32) -> Result<(), ShopError> {
    let row = GOBLIN_WARES
        .iter()
        .position(|&(i, _)| i == item)
        .ok_or(ShopError::NotFound)?;
    buy(bags, purse, row as u32 + 1, 1)
}

/// `BuybackItem(index)`: the 1-based buyback row back, for what it sold for.
pub(crate) fn buyback(
    bags: &mut Bags,
    purse: &mut Purse,
    shop: &mut Shop,
    index: u32,
) -> Result<(), ShopError> {
    let at = usize::try_from(index)
        .ok()
        .and_then(|i| i.checked_sub(1))
        .filter(|&i| i < shop.buyback.len())
        .ok_or(ShopError::NotFound)?;
    let (stack, price) = shop.buyback[at];
    if price > purse.copper() {
        return Err(ShopError::NoMoney);
    }
    bags.exchange(&[], &[(stack.item, stack.count)])
        .map_err(ShopError::Bags)?;
    purse.spend(price);
    shop.buyback.remove(at);
    shop.touch();
    Ok(())
}

/// One merchant row for `def`.
fn row(def: &ItemDef, price: u64, quantity: u32) -> MerchantItem {
    MerchantItem {
        name: Some(def.name.to_string()),
        texture: Some(items::icon_path(def)),
        price: u32::try_from(price).unwrap_or(u32::MAX),
        quantity,
        num_available: -1,
        item_id: def.entry,
        stats: Some(ItemStatsHead {
            quality: def.quality,
            inventory_type: def.inventory_type,
            class: def.class,
            subclass: def.subclass,
            armor: def.armor,
            sell_price: def.sell_price,
            ..Default::default()
        }),
        link: Some(items::link(def)),
        max_stack: Some(def.stack),
    }
}

/// The stock `MerchantFrame`'s snapshot: the goblin's goods and the buyback list.
pub(crate) fn merchant_state(shop: &Shop) -> MerchantState {
    MerchantState {
        items: GOBLIN_WARES
            .iter()
            .filter_map(|&(item, price)| Some(row(items::template(item)?, u64::from(price), 1)))
            .collect(),
        buyback: shop
            .buyback
            .iter()
            .filter_map(|(stack, price)| {
                let mut r = row(items::template(stack.item)?, *price, stack.count);
                // 1.12 has no `GetBuybackItemLink`.
                r.link = None;
                Some(r)
            })
            .collect(),
        can_repair: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::professions::items::{COPPER_ORE, HELM_OF_VALOR};

    #[test]
    fn survivors_econ_a_run_banks_a_quarter_in_silver() {
        assert_eq!(banked(250), 6_250, "250 gold banks 62s 50c");
        assert_eq!(coins(6_250), "62s 50c");
        assert_eq!(coins(120_345), "12g 3s 45c");
        assert_eq!(coins(0), "0c");
        assert_eq!(coins(10_000), "1g");
    }

    /// The tally counts every rise and ignores the camp's spending; a new run starts it again.
    #[test]
    fn survivors_econ_the_tally_counts_collected_not_left() {
        let mut t = GoldTally::default();
        t.observe(0, 0.0);
        t.observe(40, 10.0);
        t.observe(15, 20.0); // a reroll and a banish bought at camp
        t.observe(55, 30.0);
        assert_eq!(t.collected, 80);
        // The run resets: the clock and the purse go back to nothing.
        t.observe(0, 0.0);
        assert_eq!(t.collected, 0);
        t.observe(7, 1.0);
        assert_eq!(t.collected, 7);
    }

    #[test]
    fn survivors_econ_the_purse_pays_or_refuses() {
        let mut p = Purse::with(100);
        assert!(!p.spend(101));
        assert_eq!(p.copper(), 100);
        assert!(p.spend(40));
        p.earn(5);
        assert_eq!(p.copper(), 65);
        assert_eq!(p.generation(), 2);
    }

    /// Sell a stack, buy it back, buy goods; every refusal changes nothing.
    #[test]
    fn survivors_econ_sell_buy_and_buy_back() {
        let mut bags = Bags::default();
        let mut purse = Purse::default();
        let mut shop = Shop::default();
        bags.add(COPPER_ORE, 7);
        bags.add(HELM_OF_VALOR, 1);
        let paid = sell(&mut bags, &mut purse, &mut shop, 0, 1).unwrap();
        assert_eq!(paid, 7 * 5, "Copper Ore's 5c each");
        assert_eq!(bags.count(COPPER_ORE), 0);
        assert_eq!(purse.copper(), 35);
        let state = merchant_state(&shop);
        assert_eq!(state.items.len(), GOBLIN_WARES.len());
        assert_eq!(state.buyback.len(), 1);
        assert_eq!((state.buyback[0].price, state.buyback[0].quantity), (35, 7));
        // Coal at 5s: refused, then bought once the helm is sold.
        assert_eq!(buy(&mut bags, &mut purse, 1, 1), Err(ShopError::NoMoney));
        sell(&mut bags, &mut purse, &mut shop, 0, 2).unwrap();
        assert_eq!(purse.copper(), 35 + 9_870);
        buy(&mut bags, &mut purse, 1, 2).unwrap();
        assert_eq!(bags.count(COAL), 2);
        assert_eq!(purse.copper(), 35 + 9_870 - 1_000);
        // The ore back for what it fetched.
        buyback(&mut bags, &mut purse, &mut shop, 1).unwrap();
        assert_eq!(bags.count(COPPER_ORE), 7);
        assert_eq!(purse.copper(), 9_870 - 1_000);
        assert_eq!(merchant_state(&shop).buyback.len(), 1, "the helm is left");
        assert_eq!(
            buyback(&mut bags, &mut purse, &mut shop, 5),
            Err(ShopError::NotFound)
        );
        assert_eq!(buy(&mut bags, &mut purse, 99, 1), Err(ShopError::NotFound));
        buy_entry(&mut bags, &mut purse, EMPTY_VIAL).unwrap();
        assert_eq!(bags.count(EMPTY_VIAL), 1);
    }

    #[test]
    fn survivors_econ_the_buyback_keeps_twelve() {
        let mut bags = Bags::default();
        let mut purse = Purse::default();
        let mut shop = Shop::default();
        for _ in 0..15 {
            bags.add(COPPER_ORE, 1);
            sell(&mut bags, &mut purse, &mut shop, 0, 1).unwrap();
        }
        assert_eq!(merchant_state(&shop).buyback.len(), BUYBACK_SLOTS);
        assert_eq!(purse.copper(), 15 * 5);
    }

    #[test]
    fn survivors_econ_goods_are_authored() {
        for &(item, price) in GOBLIN_WARES {
            let def = items::template(item).expect("authored");
            assert!(
                u64::from(price) >= u64::from(def.sell_price),
                "{} sells back for no more than it costs",
                def.name
            );
        }
    }
}
