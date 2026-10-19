//! The satchel bar (docs/PROFESSIONS.md, 5.3): three slots on keys 4, 5 and 6, on the HUD under
//! the toy bar, each naming an item to use out of the bags mid-run. It is a bar, not a second
//! container: a use takes from the bags, as a right-click would ([`effects::use_stack`]), under
//! the same cooldowns, and in co-op the host applies it.
//!
//! - **Filling it:** pick an item up in the stock bags and click a slot: the slot takes it and the
//!   item drops back where it was (the small bridge from the FrameXML cursor to a Bevy target).
//!   Shift-click a slot to empty it.
//! - **Each slot** shows the item's icon, its count across the bags, its key, and the seconds left
//!   on its cooldown.
//! - **Saved** as `[professions] satchel`.

use bevy::prelude::*;

use benilla_ui::script::UiScript;

use super::super::gear::ItemIcon;
use super::super::hud::{self, HudFont};
use super::effects::{self, Pack, Stage, Taker, UseCtx, Used};
use super::inventory::Bags;
use super::items;
use super::skills::Skills;

/// The bar's keys, in slot order.
pub(crate) const KEYS: [KeyCode; 3] = [KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6];
/// The bar's slots.
pub(crate) const SLOTS: usize = 3;

const SLOT_PX: f32 = 40.0;
const FACE: Color = Color::srgba(0.18, 0.06, 0.04, 0.95);
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const GOLD: Color = Color::srgb(1.0, 0.82, 0.0);

/// The three slots' items, 0 for an empty slot.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Satchel {
    slots: [u32; SLOTS],
    /// Bumped on every change: the save's and the bar's gate.
    generation: u64,
}

impl Satchel {
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Slot `i`'s item, 0 when empty.
    pub(crate) fn item(&self, i: usize) -> u32 {
        self.slots.get(i).copied().unwrap_or(0)
    }

    /// Put `item` in slot `i`, if it is something to use; it leaves any other slot it was in.
    pub(crate) fn set(&mut self, i: usize, item: u32) -> bool {
        if i >= SLOTS || !effects::usable(item) || items::template(item).is_none() {
            return false;
        }
        for s in &mut self.slots {
            if *s == item {
                *s = 0;
            }
        }
        self.slots[i] = item;
        self.generation = self.generation.wrapping_add(1);
        true
    }

    /// Empty slot `i`.
    pub(crate) fn clear(&mut self, i: usize) {
        if let Some(s) = self.slots.get_mut(i).filter(|s| **s != 0) {
            *s = 0;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub(crate) fn to_save(&self) -> Vec<u32> {
        self.slots.to_vec()
    }

    /// From the save: an item this build does not know, or cannot use, leaves its slot empty.
    pub(crate) fn from_save(save: &[u32]) -> Self {
        let mut satchel = Self::default();
        for (i, &item) in save.iter().take(SLOTS).enumerate() {
            if item != 0 && !satchel.set(i, item) {
                warn!("survivors: the saved satchel holds {item}, which this build skips");
            }
        }
        satchel.generation = 0;
        satchel
    }
}

/// One press of slot `i`: its item used out of the bags, as a right-click on its first stack.
pub(in crate::survivors) fn use_slot(
    bags: &mut Bags,
    skills: &mut Skills,
    satchel: &Satchel,
    i: usize,
    ctx: UseCtx,
) -> Used {
    let item = satchel.item(i);
    if item == 0 {
        return Used::Nothing;
    }
    match effects::find(bags, item) {
        Some((bag, slot)) => effects::use_stack(bags, skills, bag, slot, ctx),
        None => Used::Refused("ERR_ITEM_NOT_FOUND", "You have none left."),
    }
}

/// The HUD row the bar is built in, under the toy bar.
#[derive(Component)]
pub(in crate::survivors) struct SatchelRow;
/// A slot's button.
#[derive(Component, Clone, Copy)]
pub(crate) struct SatchelButton(pub(crate) usize);
/// A slot's count and cooldown lines.
#[derive(Component, Clone, Copy)]
pub(crate) struct SatchelCount(pub(crate) usize);
#[derive(Component, Clone, Copy)]
pub(crate) struct SatchelCooldown(pub(crate) usize);

/// A slot's click: with an item on the stock cursor it takes that item (and the item drops back
/// into its bag); shift-clicked it empties; otherwise it uses its item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Click {
    Take(u32),
    Clear,
    Use,
}

fn click(cursor: Option<u32>, shift: bool) -> Click {
    match (cursor, shift) {
        (Some(item), _) => Click::Take(item),
        (None, true) => Click::Clear,
        (None, false) => Click::Use,
    }
}

/// `Update`, after the input pass: the keys 4-6 (a focused edit box has already taken them) and
/// the slots' clicks, while a run is playing.
pub(super) fn satchel_input(
    script: Option<NonSendMut<UiScript>>,
    keys: Res<ButtonInput<KeyCode>>,
    stage: Stage,
    mut pack: Pack,
    mut taker: Taker,
    mut satchel: ResMut<Satchel>,
    buttons: Query<(&Interaction, &SatchelButton), Changed<Interaction>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let ctx = stage.ctx(script.now());
    if !ctx.in_run {
        return;
    }
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let mut presses: Vec<(usize, Click)> = KEYS
        .iter()
        .enumerate()
        .filter(|(_, k)| keys.just_pressed(**k))
        .map(|(i, _)| (i, Click::Use))
        .collect();
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            let held = script.cursor_item().map(|c| c.item_id);
            presses.push((button.0, click(held, shift)));
        }
    }
    for (i, what) in presses {
        match what {
            Click::Take(item) => {
                if satchel.set(i, item) {
                    script.clear_cursor_payload();
                }
            }
            Click::Clear => satchel.clear(i),
            Click::Use => {
                let (bags, skills) = pack.bags_and_skills();
                let used = use_slot(bags, skills, &satchel, i, ctx);
                taker.after(bags, used, ctx, stage.role(), stage.class());
            }
        }
    }
}

/// `Update`: the bar's slots rebuilt when the satchel changes or the HUD's row is new.
pub(super) fn build_satchel(
    mut commands: Commands,
    satchel: Res<Satchel>,
    font: Option<Res<HudFont>>,
    rows: Query<(Entity, Ref<SatchelRow>)>,
    mut built: Local<Option<u64>>,
) {
    let Some(font) = font else {
        return;
    };
    let fresh = rows.iter().any(|(_, r)| r.is_added());
    if !fresh && *built == Some(satchel.generation()) {
        return;
    }
    *built = Some(satchel.generation());
    let f = &font.0;
    for (row, _) in &rows {
        commands.entity(row).despawn_related::<Children>();
        commands.entity(row).with_children(|row| {
            for (i, key) in ["4", "5", "6"].into_iter().enumerate() {
                let item = items::template(satchel.item(i));
                let mut slot = row.spawn((
                    Button,
                    SatchelButton(i),
                    BackgroundColor(FACE),
                    BorderColor::all(EDGE),
                    Node {
                        width: Val::Px(SLOT_PX),
                        height: Val::Px(SLOT_PX),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                ));
                slot.with_children(|s| {
                    if let Some(def) = item {
                        s.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            ImageNode::default(),
                            ItemIcon {
                                icon: def.icon,
                                tint: Color::WHITE,
                            },
                        ));
                    }
                    s.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(2.0),
                            top: Val::Px(0.0),
                            ..default()
                        },
                        hud::text_bundle(f, key, 11.0, Color::WHITE),
                    ));
                    s.spawn((
                        SatchelCount(i),
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(2.0),
                            bottom: Val::Px(0.0),
                            ..default()
                        },
                        hud::text_bundle(f, "", 11.0, Color::WHITE),
                    ));
                    s.spawn((
                        SatchelCooldown(i),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            top: Val::Px(12.0),
                            ..default()
                        },
                        hud::centered_bundle(f, "", 13.0, GOLD),
                    ));
                });
            }
        });
    }
}

/// What slot `i` reads now: its count across the bags (blank when empty), and the seconds left on
/// its cooldown (blank when ready).
pub(crate) fn slot_text(bags: &Bags, satchel: &Satchel, i: usize, now: f64) -> (String, String) {
    let item = satchel.item(i);
    if item == 0 {
        return (String::new(), String::new());
    }
    let count = bags.count(item).to_string();
    let left = effects::cooldown_of(bags, item)
        .and_then(|(start, secs)| {
            let left = start + f64::from(secs) - now;
            (left > 0.0).then(|| format!("{}", left.ceil() as u32))
        })
        .unwrap_or_default();
    (count, left)
}

/// `Update`: each slot's count and cooldown.
pub(super) fn tick_satchel(
    script: Option<NonSend<UiScript>>,
    bags: Res<Bags>,
    satchel: Res<Satchel>,
    mut counts: Query<(&SatchelCount, &mut Text), Without<SatchelCooldown>>,
    mut cooldowns: Query<(&SatchelCooldown, &mut Text), Without<SatchelCount>>,
) {
    let Some(script) = script else {
        return;
    };
    let now = script.now();
    for (c, mut text) in &mut counts {
        let (count, _) = slot_text(&bags, &satchel, c.0, now);
        if **text != count {
            **text = count;
        }
    }
    for (c, mut text) in &mut cooldowns {
        let (_, left) = slot_text(&bags, &satchel, c.0, now);
        if **text != left {
            **text = left;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::professions::effects::Category;
    use crate::survivors::professions::items::{
        COPPER_ORE, ELIXIR_OF_GIANTS, LINEN_BANDAGE, MINOR_HEALING_POTION,
    };

    #[test]
    fn survivors_satchel_slots_take_only_usable_items_and_save() {
        let mut s = Satchel::default();
        assert!(!s.set(0, COPPER_ORE), "ore does nothing");
        assert!(s.set(0, MINOR_HEALING_POTION));
        assert!(s.set(2, LINEN_BANDAGE));
        assert!(!s.set(3, ELIXIR_OF_GIANTS), "three slots");
        // The potion moves rather than doubling.
        assert!(s.set(1, MINOR_HEALING_POTION));
        assert_eq!(s.to_save(), vec![0, MINOR_HEALING_POTION, LINEN_BANDAGE]);
        let g = s.generation();
        s.clear(2);
        assert!(s.generation() > g);
        let back = Satchel::from_save(&[ELIXIR_OF_GIANTS, 424_242, COPPER_ORE, 118]);
        assert_eq!(back.to_save(), vec![ELIXIR_OF_GIANTS, 0, 0]);
        assert_eq!(click(Some(118), true), Click::Take(118));
        assert_eq!(click(None, true), Click::Clear);
        assert_eq!(click(None, false), Click::Use);
    }

    /// A key uses its slot's item from the bags, under the shared cooldown, and the bar shows the
    /// count and the seconds left.
    #[test]
    fn survivors_satchel_a_key_uses_from_the_bags() {
        let mut bags = Bags::default();
        let mut skills = Skills::default();
        bags.add(COPPER_ORE, 1);
        bags.add(MINOR_HEALING_POTION, 2);
        let mut s = Satchel::default();
        s.set(0, MINOR_HEALING_POTION);
        let ctx = UseCtx {
            now: 10.0,
            in_run: true,
            at_camp: false,
            run_secs: 100.0,
        };
        assert!(matches!(
            use_slot(&mut bags, &mut skills, &s, 0, ctx),
            Used::Took(_)
        ));
        assert_eq!(bags.count(MINOR_HEALING_POTION), 1);
        assert_eq!(bags.cooldown(Category::Potion), Some(10.0));
        assert_eq!(slot_text(&bags, &s, 0, 15.5), ("1".into(), "55".into()));
        assert_eq!(use_slot(&mut bags, &mut skills, &s, 0, ctx), Used::NotReady);
        assert_eq!(use_slot(&mut bags, &mut skills, &s, 1, ctx), Used::Nothing);
        let later = UseCtx { now: 80.0, ..ctx };
        assert!(matches!(
            use_slot(&mut bags, &mut skills, &s, 0, later),
            Used::Took(_)
        ));
        assert!(
            matches!(
                use_slot(&mut bags, &mut skills, &s, 0, UseCtx { now: 200.0, ..ctx }),
                Used::Refused(..)
            ),
            "none left"
        );
        assert_eq!(slot_text(&bags, &s, 0, 200.0), ("0".into(), String::new()));
    }
}
