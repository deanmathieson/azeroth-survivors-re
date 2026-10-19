//! Item uses (docs/PROFESSIONS.md, 2.5): a right-click on a bag slot is the stock
//! `UseContainerItem`, taken here before the stock applier (which would send `CMSG_USE_ITEM` to a
//! server that is not there). P1 wires the Minor Healing Potion: it restores 15% of the hero's
//! health, consumes one, and starts the shared potion cooldown, whose sweep the bag slots show.
//! A co-op client's hero lives on the host, so its heal goes there as a `UseItem`. Every other
//! item does nothing yet (P2's satchel and effects). With the goblin's window open, a right-click
//! sells the stack instead, as in 1.12.

use bevy::prelude::*;

use benilla_ui::script::UiScript;

use super::super::party::{self, HeroState, LocalHero, NetRole};
use super::gather::{CastLine, UiLines};
use super::inventory::Bags;
use super::items;

/// The shared potion cooldown, in seconds (5.2).
pub(crate) const POTION_COOLDOWN: f32 = 60.0;
/// The share of the hero's health a Minor Healing Potion restores.
pub(crate) const POTION_HEAL: f32 = 0.15;

/// A co-op client's uses for the host to apply to its hero, by item entry.
#[derive(Resource, Default)]
pub(crate) struct UseOutbox(pub(crate) Vec<u32>);

/// What a use did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Used {
    /// A potion went down: heal the hero.
    Potion,
    /// The cooldown is still running.
    NotReady,
    /// Nothing to use there, or nothing that does anything yet.
    Nothing,
}

/// One `UseContainerItem(bag, slot)` against `bags` at `now` on the `GetTime` clock.
pub(crate) fn use_item(bags: &mut Bags, bag: i64, slot: u32, now: f64) -> Used {
    let Some(stack) = bags.get(bag, slot) else {
        return Used::Nothing;
    };
    if stack.item != items::MINOR_HEALING_POTION {
        return Used::Nothing;
    }
    if bags
        .potion_cooldown()
        .is_some_and(|start| now < start + f64::from(POTION_COOLDOWN))
    {
        return Used::NotReady;
    }
    if bags.destroy(bag, slot, 1).is_err() {
        return Used::Nothing;
    }
    bags.start_potion_cooldown(now);
    Used::Potion
}

/// Heal the hero a potion's share.
pub(in crate::survivors) fn drink(hero: &mut HeroState) {
    let amount = hero.stats().max_hp * POTION_HEAL;
    party::heal(hero, amount);
}

/// `Update`, after the input pass and before the stock script-call applier: this frame's bag
/// right-clicks.
pub(super) fn use_items(
    script: Option<NonSendMut<UiScript>>,
    role: Res<NetRole>,
    mut wallet: super::econ::Wallet,
    mut outbox: ResMut<UseOutbox>,
    mut ui: ResMut<UiLines>,
    mut hero: Query<&mut HeroState, With<LocalHero>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let now = script.now();
    let super::econ::Wallet { bags, purse, shop } = &mut wallet;
    for (bag, slot) in script.take_container_uses() {
        if shop.open {
            if let Err(e) = super::econ::sell(bags, purse, shop, bag, slot) {
                let (key, fallback) = e.global_string();
                ui.0.push(CastLine::Refused(key, fallback));
            }
            continue;
        }
        match use_item(bags, bag, slot, now) {
            Used::Potion if *role == NetRole::Client => outbox.0.push(items::MINOR_HEALING_POTION),
            Used::Potion => {
                if let Ok(mut hero) = hero.single_mut() {
                    drink(&mut hero);
                }
            }
            Used::NotReady => ui.0.push(CastLine::NotReady),
            Used::Nothing => debug!("survivors: nothing to use at {bag}/{slot}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survivors_effects_a_potion_heals_once_per_cooldown() {
        let mut bags = Bags::default();
        bags.add(items::MINOR_HEALING_POTION, 2);
        bags.add(items::COPPER_ORE, 1);
        assert_eq!(use_item(&mut bags, 0, 1, 100.0), Used::Potion);
        assert_eq!(bags.get(0, 1).map(|s| s.count), Some(1), "one consumed");
        assert_eq!(bags.potion_cooldown(), Some(100.0));
        assert_eq!(use_item(&mut bags, 0, 1, 159.0), Used::NotReady);
        assert_eq!(use_item(&mut bags, 0, 2, 200.0), Used::Nothing, "ore");
        assert_eq!(use_item(&mut bags, 0, 1, 160.0), Used::Potion);
        assert_eq!(bags.get(0, 1), None, "the last one");
        assert_eq!(use_item(&mut bags, 0, 1, 300.0), Used::Nothing, "empty");
    }

    /// The right-click through the engine's own `UseContainerItem` lands here.
    #[test]
    fn survivors_effects_the_bag_right_click_reaches_the_drain() {
        let mut s = UiScript::new().unwrap();
        let mut bags = Bags::default();
        bags.add(items::MINOR_HEALING_POTION, 1);
        let mut feed = super::super::frames::BagFeed::default();
        super::super::frames::sync(&mut s, &mut bags, 0, &mut feed);
        s.run("UseContainerItem(0, 1)").unwrap();
        let uses = s.take_container_uses();
        assert_eq!(uses, vec![(0, 1)]);
        assert_eq!(use_item(&mut bags, 0, 1, s.now()), Used::Potion);
    }
}
