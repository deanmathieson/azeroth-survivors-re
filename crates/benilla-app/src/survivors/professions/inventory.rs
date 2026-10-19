//! The player's bags, the source of truth the stock `ContainerFrame` mirrors: the Backpack
//! (container 0, 16 slots) and the four worn bags (containers 1-4, sized by the bag item), in the
//! 1.12 client's numbering with 1-based slots. Every rule lives here and the UI only shows the
//! result: stack limits; move, swap, merge and split; autostore; destroy; bag families.

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::items;

/// The Backpack's container id.
pub(crate) const BACKPACK: i64 = 0;
/// The Backpack's slots, as in 1.12.
pub(crate) const BACKPACK_SLOTS: u32 = 16;
/// The worn bags: containers 1 to 4.
pub(crate) const WORN_BAGS: usize = 4;

/// One occupied slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stack {
    /// The 1.12 item entry ([`items::template`]).
    pub(crate) item: u32,
    pub(crate) count: u32,
}

/// Why an operation was refused, each with the stock `GlobalStrings.lua` key the client shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BagError {
    /// No such container or slot, or nothing in the source slot.
    NotFound,
    /// The split count is zero or more than the stack holds.
    BadCount,
    /// The destination is a different item, or the merge would pass the stack limit.
    SplitFailed,
    /// A specialty bag refuses the item.
    WrongBag,
    /// The target bag has no room.
    BagFull,
}

impl BagError {
    /// The `GlobalStrings.lua` key of the line `UIErrorsFrame` would print.
    pub(crate) fn global_string(self) -> &'static str {
        match self {
            BagError::NotFound => "ERR_ITEM_NOT_FOUND",
            BagError::BadCount | BagError::SplitFailed => "ERR_SPLIT_FAILED",
            BagError::WrongBag => "ERR_ITEM_DOESNT_GO_INTO_BAG",
            BagError::BagFull => "ERR_BAG_FULL",
        }
    }
}

/// The bags' save section (docs/PROFESSIONS.md, 8): `[inventory]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct InventorySave {
    /// The worn bag entries in containers 1-4; 0 is empty.
    pub(crate) bags: [u32; WORN_BAGS],
    pub(crate) items: Vec<SlotSave>,
}

/// One saved slot.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SlotSave {
    /// 0 the Backpack, 1-4 the worn bags.
    pub(crate) bag: i64,
    pub(crate) slot: u32,
    pub(crate) item: u32,
    pub(crate) count: u32,
}

/// The containers and their contents. `generation` moves on every change, so the feed pushes
/// only when something did.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct Bags {
    /// The worn bags' entries, containers 1-4; 0 is an empty bag slot.
    worn: [u32; WORN_BAGS],
    /// Occupied slots by `(container, 1-based slot)`.
    slots: BTreeMap<(i64, u32), Stack>,
    generation: u64,
    /// The shared potion cooldown's start on the `GetTime` clock, while one runs; not saved.
    potion_cooldown: Option<f64>,
}

/// The same contents, whatever the change counter says.
impl PartialEq for Bags {
    fn eq(&self, other: &Self) -> bool {
        self.worn == other.worn
            && self.slots == other.slots
            && self.potion_cooldown == other.potion_cooldown
    }
}

impl Bags {
    /// Bags with these worn bags and nothing in them; an entry that is not a bag is left out.
    pub(crate) fn with_worn(worn: [u32; WORN_BAGS]) -> Self {
        let mut bags = Self::default();
        for (i, entry) in worn.into_iter().enumerate() {
            if items::template(entry).is_some_and(|t| t.bag_slots > 0) {
                bags.worn[i] = entry;
            }
        }
        bags
    }

    /// The P0 test inventory: a Linen Bag worn in slot 1, and a little of every authored item.
    pub(crate) fn test_inventory() -> Self {
        let mut bags = Self::with_worn([items::LINEN_BAG, 0, 0, 0]);
        for &(item, count) in items::TEST_INVENTORY {
            let left = bags.add(item, count);
            debug_assert_eq!(left, 0, "the test inventory fits");
        }
        bags
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// The potion cooldown's start, if one was started.
    pub(crate) fn potion_cooldown(&self) -> Option<f64> {
        self.potion_cooldown
    }

    /// Start the potion cooldown at `now` (`GetTime`), so the bag slots show its sweep.
    pub(crate) fn start_potion_cooldown(&mut self, now: f64) {
        self.potion_cooldown = Some(now);
        self.touch();
    }

    /// The bags as the profile saves them: worn bags and every occupied slot.
    pub(crate) fn to_save(&self) -> InventorySave {
        InventorySave {
            bags: self.worn,
            items: self
                .slots
                .iter()
                .map(|(&(bag, slot), s)| SlotSave {
                    bag,
                    slot,
                    item: s.item,
                    count: s.count,
                })
                .collect(),
        }
    }

    /// From the profile. An unknown item, a slot past its bag's size, an overfull stack and a
    /// second item in one slot are skipped with a warning; they never fail the load.
    pub(crate) fn from_save(save: &InventorySave) -> Self {
        let mut bags = Self::with_worn(save.bags);
        for s in &save.items {
            let fits = items::template(s.item).is_some()
                && s.slot >= 1
                && s.slot <= bags.size(s.bag)
                && s.count >= 1
                && s.count <= Self::max_stack(s.item)
                && bags.get(s.bag, s.slot).is_none();
            if fits {
                bags.slots.insert(
                    (s.bag, s.slot),
                    Stack {
                        item: s.item,
                        count: s.count,
                    },
                );
            } else {
                warn!("survivors: the saved bags hold {s:?}, which this build skips");
            }
        }
        bags
    }

    /// The worn bag entry in container `bag` (1-4), 0 when empty or out of range.
    pub(crate) fn worn(&self, bag: i64) -> u32 {
        usize::try_from(bag - 1)
            .ok()
            .and_then(|i| self.worn.get(i))
            .copied()
            .unwrap_or(0)
    }

    /// Container `bag`'s slot count: 16 for the Backpack, the bag item's for a worn bag, else 0.
    pub(crate) fn size(&self, bag: i64) -> u32 {
        match bag {
            BACKPACK => BACKPACK_SLOTS,
            1..=4 => items::template(self.worn(bag)).map_or(0, |t| t.bag_slots),
            _ => 0,
        }
    }

    /// The containers that exist, in order.
    pub(crate) fn containers(&self) -> impl Iterator<Item = i64> + '_ {
        (BACKPACK..=WORN_BAGS as i64).filter(|&b| self.size(b) > 0)
    }

    pub(crate) fn get(&self, bag: i64, slot: u32) -> Option<Stack> {
        self.slots.get(&(bag, slot)).copied()
    }

    /// Every occupied slot of `bag`, by slot.
    pub(crate) fn contents(&self, bag: i64) -> impl Iterator<Item = (u32, Stack)> + '_ {
        self.slots
            .range((bag, 0)..=(bag, u32::MAX))
            .map(|(&(_, slot), &s)| (slot, s))
    }

    fn in_range(&self, bag: i64, slot: u32) -> bool {
        slot >= 1 && slot <= self.size(bag)
    }

    /// Whether container `bag` takes `item`: a general bag takes anything, a specialty bag only its
    /// family.
    fn accepts(&self, bag: i64, item: u32) -> Result<(), BagError> {
        let family = match bag {
            BACKPACK => 0,
            _ => items::template(self.worn(bag)).map_or(0, |t| t.family),
        };
        if family != 0 && items::template(item).is_none_or(|t| t.family & family == 0) {
            return Err(BagError::WrongBag);
        }
        Ok(())
    }

    fn max_stack(item: u32) -> u32 {
        items::template(item).map_or(1, |t| t.stack.max(1))
    }

    fn put(&mut self, bag: i64, slot: u32, stack: Option<Stack>) {
        match stack.filter(|s| s.count > 0) {
            Some(s) => {
                self.slots.insert((bag, slot), s);
            }
            None => {
                self.slots.remove(&(bag, slot));
            }
        }
    }

    fn touch(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// `PickupContainerItem`'s placement: the whole stack (`count: None`) moves to an empty slot,
    /// merges into the same item up to its stack limit (the rest stays behind), or swaps with
    /// another item; a split (`Some(n)`) places `n` onto an empty slot or the same item, and is
    /// refused if that would pass the limit.
    pub(crate) fn move_item(
        &mut self,
        src: (i64, u32),
        dst: (i64, u32),
        count: Option<u32>,
    ) -> Result<(), BagError> {
        if !self.in_range(src.0, src.1) || !self.in_range(dst.0, dst.1) {
            return Err(BagError::NotFound);
        }
        let from = self.get(src.0, src.1).ok_or(BagError::NotFound)?;
        if src == dst {
            return Ok(());
        }
        let to = self.get(dst.0, dst.1);
        let max = Self::max_stack(from.item);
        match count {
            None => match to {
                None => {
                    self.accepts(dst.0, from.item)?;
                    self.put(src.0, src.1, None);
                    self.put(dst.0, dst.1, Some(from));
                }
                Some(t) if t.item == from.item && max > 1 => {
                    let moved = from.count.min(max.saturating_sub(t.count));
                    if moved == 0 {
                        // A full stack onto a full stack: the two trade places, as a swap.
                        return Ok(());
                    }
                    self.put(
                        dst.0,
                        dst.1,
                        Some(Stack {
                            count: t.count + moved,
                            ..t
                        }),
                    );
                    self.put(
                        src.0,
                        src.1,
                        Some(Stack {
                            count: from.count - moved,
                            ..from
                        }),
                    );
                }
                Some(t) => {
                    self.accepts(dst.0, from.item)?;
                    self.accepts(src.0, t.item)?;
                    self.put(src.0, src.1, Some(t));
                    self.put(dst.0, dst.1, Some(from));
                }
            },
            Some(n) => {
                if n == 0 || n > from.count {
                    return Err(BagError::BadCount);
                }
                self.accepts(dst.0, from.item)?;
                let placed = match to {
                    None => n,
                    Some(t) if t.item == from.item && t.count + n <= max => t.count + n,
                    Some(_) => return Err(BagError::SplitFailed),
                };
                self.put(
                    dst.0,
                    dst.1,
                    Some(Stack {
                        item: from.item,
                        count: placed,
                    }),
                );
                self.put(
                    src.0,
                    src.1,
                    Some(Stack {
                        count: from.count - n,
                        ..from
                    }),
                );
            }
        }
        self.touch();
        Ok(())
    }

    /// `PutItemInBag`/`PutItemInBackpack`: the stack at `src` (or `count` of it) into container
    /// `dst_bag`, topping up its stacks of the same item first, then its first empty slot. All or
    /// nothing: with no room the stack stays where it was.
    pub(crate) fn autostore(
        &mut self,
        src: (i64, u32),
        dst_bag: i64,
        count: Option<u32>,
    ) -> Result<(), BagError> {
        let from = self.get(src.0, src.1).ok_or(BagError::NotFound)?;
        let n = count.unwrap_or(from.count);
        if n == 0 || n > from.count {
            return Err(BagError::BadCount);
        }
        if self.size(dst_bag) == 0 {
            return Err(BagError::NotFound);
        }
        if dst_bag == src.0 && count.is_none() {
            return Ok(());
        }
        self.accepts(dst_bag, from.item)?;
        let mut trial = self.clone();
        trial.put(
            src.0,
            src.1,
            Some(Stack {
                count: from.count - n,
                ..from
            }),
        );
        let left = trial.add_into(dst_bag, from.item, n);
        if left > 0 {
            return Err(BagError::BagFull);
        }
        *self = trial;
        self.touch();
        Ok(())
    }

    /// `DeleteCursorItem` after the stock confirm: `count` 0 is the whole stack.
    pub(crate) fn destroy(&mut self, bag: i64, slot: u32, count: u32) -> Result<(), BagError> {
        let from = self.get(bag, slot).ok_or(BagError::NotFound)?;
        let left = if count == 0 {
            0
        } else {
            from.count.saturating_sub(count)
        };
        self.put(
            bag,
            slot,
            Some(Stack {
                count: left,
                ..from
            }),
        );
        self.touch();
        Ok(())
    }

    /// Add `count` of `item` anywhere: same-item stacks first, then empty slots, in container and
    /// slot order. Returns what did not fit ("Inventory is full." for the caller to show).
    pub(crate) fn add(&mut self, item: u32, count: u32) -> u32 {
        let mut left = count;
        let bags: Vec<i64> = self.containers().collect();
        // Top-ups first everywhere, then empty slots, as the client's auto-store does.
        for &bag in &bags {
            left = self.top_up(bag, item, left);
        }
        for &bag in &bags {
            if self.accepts(bag, item).is_ok() {
                left = self.fill_empty(bag, item, left);
            }
        }
        if left != count {
            self.touch();
        }
        left
    }

    /// How many of `item` the bags hold.
    pub(crate) fn count(&self, item: u32) -> u32 {
        self.slots
            .values()
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum()
    }

    /// All or nothing: take every `(item, count)` of `take` out of the bags (the last stacks
    /// first), then add every one of `give`. Refused with [`BagError::NotFound`] when the bags
    /// lack a reagent and [`BagError::BagFull`] when the products do not fit, changing nothing.
    pub(crate) fn exchange(
        &mut self,
        take: &[(u32, u32)],
        give: &[(u32, u32)],
    ) -> Result<(), BagError> {
        let mut trial = self.clone();
        for &(item, count) in take {
            if trial.count(item) < count {
                return Err(BagError::NotFound);
            }
            let mut left = count;
            let held: Vec<(i64, u32)> = trial
                .slots
                .iter()
                .rev()
                .filter(|(_, s)| s.item == item)
                .map(|(&at, _)| at)
                .collect();
            for (bag, slot) in held {
                if left == 0 {
                    break;
                }
                let s = trial.slots[&(bag, slot)];
                let n = left.min(s.count);
                trial.put(
                    bag,
                    slot,
                    Some(Stack {
                        count: s.count - n,
                        ..s
                    }),
                );
                left -= n;
            }
        }
        for &(item, count) in give {
            if trial.add(item, count) > 0 {
                return Err(BagError::BagFull);
            }
        }
        *self = trial;
        self.touch();
        Ok(())
    }

    /// [`Self::add`] limited to one container, without the generation bump.
    fn add_into(&mut self, bag: i64, item: u32, count: u32) -> u32 {
        let left = self.top_up(bag, item, count);
        self.fill_empty(bag, item, left)
    }

    fn top_up(&mut self, bag: i64, item: u32, mut left: u32) -> u32 {
        let max = Self::max_stack(item);
        for slot in 1..=self.size(bag) {
            if left == 0 {
                break;
            }
            if let Some(s) = self
                .get(bag, slot)
                .filter(|s| s.item == item && s.count < max)
            {
                let n = left.min(max - s.count);
                self.put(
                    bag,
                    slot,
                    Some(Stack {
                        count: s.count + n,
                        ..s
                    }),
                );
                left -= n;
            }
        }
        left
    }

    fn fill_empty(&mut self, bag: i64, item: u32, mut left: u32) -> u32 {
        let max = Self::max_stack(item);
        for slot in 1..=self.size(bag) {
            if left == 0 {
                break;
            }
            if self.get(bag, slot).is_none() {
                let n = left.min(max);
                self.put(bag, slot, Some(Stack { item, count: n }));
                left -= n;
            }
        }
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::professions::items::{
        COPPER_ORE, LINEN_BAG, LINEN_CLOTH, MINOR_HEALING_POTION, PEACEBLOOM,
    };

    fn one_bag() -> Bags {
        Bags::with_worn([LINEN_BAG, 0, 0, 0])
    }

    fn at(b: &Bags, bag: i64, slot: u32) -> Option<(u32, u32)> {
        b.get(bag, slot).map(|s| (s.item, s.count))
    }

    /// How many of `item` the bags hold.
    fn count_of(b: &Bags, item: u32) -> u32 {
        b.slots
            .values()
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum()
    }

    #[test]
    fn survivors_bags_sizes_follow_the_worn_bag() {
        let b = one_bag();
        assert_eq!(b.size(0), 16);
        assert_eq!(b.size(1), 6, "a Linen Bag has six slots");
        assert_eq!(b.size(2), 0, "an empty bag slot is no container");
        assert_eq!(b.containers().collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(
            Bags::with_worn([COPPER_ORE, 0, 0, 0]).size(1),
            0,
            "ore is not a bag, so it is never worn"
        );
    }

    #[test]
    fn survivors_bags_add_tops_up_then_fills_and_reports_overflow() {
        let mut b = one_bag();
        // Ore stacks to 10: 25 makes 10, 10, 5.
        assert_eq!(b.add(COPPER_ORE, 25), 0);
        assert_eq!(at(&b, 0, 1), Some((COPPER_ORE, 10)));
        assert_eq!(at(&b, 0, 3), Some((COPPER_ORE, 5)));
        // 7 more tops the 5 up to 10, then starts a new stack.
        assert_eq!(b.add(COPPER_ORE, 7), 0);
        assert_eq!(at(&b, 0, 3), Some((COPPER_ORE, 10)));
        assert_eq!(at(&b, 0, 4), Some((COPPER_ORE, 2)));
        // 22 slots in all, 4 used: 18 more stacks fit, the rest overflows.
        let left = b.add(COPPER_ORE, 8 + 18 * 10 + 3);
        assert_eq!(left, 3, "Inventory is full.");
        assert_eq!(count_of(&b, COPPER_ORE), 22 * 10);
    }

    #[test]
    fn survivors_bags_move_swap_and_merge() {
        let mut b = one_bag();
        b.add(PEACEBLOOM, 12);
        b.add(LINEN_CLOTH, 5);
        // Peacebloom 12 in slot 1, Linen 5 in slot 2.
        let g = b.generation();
        b.move_item((0, 1), (1, 3), None).unwrap();
        assert_eq!(at(&b, 0, 1), None);
        assert_eq!(at(&b, 1, 3), Some((PEACEBLOOM, 12)));
        assert!(b.generation() > g, "a move bumps the generation");
        // A swap with another item.
        b.move_item((0, 2), (1, 3), None).unwrap();
        assert_eq!(at(&b, 1, 3), Some((LINEN_CLOTH, 5)));
        assert_eq!(at(&b, 0, 2), Some((PEACEBLOOM, 12)));
        // A merge tops up to the limit (20) and leaves the rest behind.
        b.add(PEACEBLOOM, 15);
        assert_eq!(at(&b, 0, 2), Some((PEACEBLOOM, 20)));
        assert_eq!(at(&b, 0, 1), Some((PEACEBLOOM, 7)));
        b.move_item((0, 2), (0, 1), None).unwrap();
        assert_eq!(at(&b, 0, 1), Some((PEACEBLOOM, 20)));
        assert_eq!(at(&b, 0, 2), Some((PEACEBLOOM, 7)));
        // Out of range and empty sources are refused, and change nothing.
        let before = b.clone();
        assert_eq!(b.move_item((0, 17), (0, 1), None), Err(BagError::NotFound));
        assert_eq!(b.move_item((1, 7), (0, 1), None), Err(BagError::NotFound));
        assert_eq!(b.move_item((0, 9), (0, 1), None), Err(BagError::NotFound));
        assert_eq!(b, before);
    }

    #[test]
    fn survivors_bags_split_onto_empty_same_and_occupied_slots() {
        let mut b = one_bag();
        b.add(LINEN_CLOTH, 12);
        b.add(MINOR_HEALING_POTION, 3);
        // Linen 12 in slot 1, potions in slot 2.
        b.move_item((0, 1), (0, 5), Some(5)).unwrap();
        assert_eq!(at(&b, 0, 1), Some((LINEN_CLOTH, 7)));
        assert_eq!(at(&b, 0, 5), Some((LINEN_CLOTH, 5)));
        // Onto the same item: merges.
        b.move_item((0, 1), (0, 5), Some(2)).unwrap();
        assert_eq!(at(&b, 0, 5), Some((LINEN_CLOTH, 7)));
        // Onto a different item: refused.
        assert_eq!(
            b.move_item((0, 1), (0, 2), Some(1)),
            Err(BagError::SplitFailed)
        );
        // Past the stack limit (20): refused.
        b.add(LINEN_CLOTH, 15);
        let full = b.clone();
        assert_eq!(at(&full, 0, 1), Some((LINEN_CLOTH, 20)));
        assert_eq!(
            b.move_item((0, 5), (0, 1), Some(1)),
            Err(BagError::SplitFailed)
        );
        assert_eq!(b, full);
        // A bad count.
        assert_eq!(
            b.move_item((0, 5), (0, 9), Some(0)),
            Err(BagError::BadCount)
        );
        assert_eq!(
            b.move_item((0, 5), (0, 9), Some(99)),
            Err(BagError::BadCount)
        );
    }

    #[test]
    fn survivors_bags_autostore_and_destroy() {
        let mut b = one_bag();
        b.add(COPPER_ORE, 4);
        // Into the Linen Bag.
        b.autostore((0, 1), 1, None).unwrap();
        assert_eq!(at(&b, 0, 1), None);
        assert_eq!(at(&b, 1, 1), Some((COPPER_ORE, 4)));
        // A split autostore back into the Backpack.
        b.autostore((1, 1), 0, Some(3)).unwrap();
        assert_eq!(at(&b, 1, 1), Some((COPPER_ORE, 1)));
        assert_eq!(at(&b, 0, 1), Some((COPPER_ORE, 3)));
        // A full bag refuses, and the stack stays put.
        for slot in 2..=6 {
            b.put(
                1,
                slot,
                Some(Stack {
                    item: LINEN_CLOTH,
                    count: 20,
                }),
            );
        }
        b.add(PEACEBLOOM, 1);
        let before = b.clone();
        let peace = b
            .contents(0)
            .find(|(_, s)| s.item == PEACEBLOOM)
            .map(|(slot, _)| slot)
            .unwrap();
        assert_eq!(b.autostore((0, peace), 1, None), Err(BagError::BagFull));
        assert_eq!(b, before);
        // Destroy: a count, then the rest.
        b.destroy(1, 1, 0).unwrap();
        assert_eq!(at(&b, 1, 1), None);
        b.add(LINEN_CLOTH, 10);
        let slot = b
            .contents(0)
            .find(|(_, s)| s.item == LINEN_CLOTH)
            .map(|(slot, _)| slot)
            .unwrap();
        b.destroy(0, slot, 4).unwrap();
        assert_eq!(at(&b, 0, slot), Some((LINEN_CLOTH, 6)));
        assert_eq!(b.destroy(0, 16, 0), Err(BagError::NotFound));
    }

    /// An exchange takes the reagents and adds the products, or changes nothing.
    #[test]
    fn survivors_bags_exchange_is_all_or_nothing() {
        let mut b = Bags::default();
        b.add(COPPER_ORE, 12);
        b.add(LINEN_CLOTH, 1);
        b.exchange(&[(COPPER_ORE, 3)], &[(PEACEBLOOM, 2)]).unwrap();
        assert_eq!(b.count(COPPER_ORE), 9);
        assert_eq!(b.count(PEACEBLOOM), 2);
        // The last stack goes first: 10 + 2 became 9 in the first.
        assert_eq!(at(&b, 0, 1), Some((COPPER_ORE, 9)));
        let before = b.clone();
        assert_eq!(
            b.exchange(&[(COPPER_ORE, 10)], &[]),
            Err(BagError::NotFound)
        );
        assert_eq!(b, before);
        // Fill every slot, then a product that needs a new one does not fit.
        for _ in 0..13 {
            b.add(MINOR_HEALING_POTION, 5);
        }
        assert_eq!(
            b.exchange(&[(LINEN_CLOTH, 1)], &[(COPPER_ORE, 30)]),
            Err(BagError::BagFull)
        );
        assert_eq!(b.count(LINEN_CLOTH), 1, "nothing was taken");
        // The reagent's slot frees room for the product.
        assert_eq!(b.exchange(&[(LINEN_CLOTH, 1)], &[(COPPER_ORE, 2)]), Ok(()));
        assert_eq!(b.count(COPPER_ORE), 11);
    }

    #[test]
    fn survivors_bags_a_bag_item_moves_like_any_item() {
        let mut b = one_bag();
        b.add(LINEN_BAG, 1);
        b.move_item((0, 1), (1, 2), None).unwrap();
        assert_eq!(at(&b, 1, 2), Some((LINEN_BAG, 1)));
    }

    #[test]
    fn survivors_bags_save_round_trips_and_skips_what_does_not_fit() {
        let bags = Bags::test_inventory();
        let save = bags.to_save();
        assert_eq!(Bags::from_save(&save), bags);
        let mut odd = save.clone();
        odd.items.push(SlotSave {
            bag: 0,
            slot: 99,
            item: COPPER_ORE,
            count: 1,
        });
        odd.items.push(SlotSave {
            bag: 0,
            slot: 16,
            item: 424_242,
            count: 1,
        });
        odd.items.push(SlotSave {
            bag: 1,
            slot: 6,
            item: COPPER_ORE,
            count: 11,
        });
        assert_eq!(Bags::from_save(&odd), bags, "the bad rows are skipped");
    }

    #[test]
    fn survivors_bags_the_test_inventory_is_real_items_that_fit() {
        let b = Bags::test_inventory();
        assert_eq!(b.worn(1), LINEN_BAG);
        for &(item, count) in items::TEST_INVENTORY {
            assert!(items::template(item).is_some(), "{item} has a template");
            assert!(count_of(&b, item) >= count, "{item}: {count} stored");
        }
    }
}
