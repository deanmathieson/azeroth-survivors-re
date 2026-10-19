//! The bags' side of the stock FrameXML: the feed that mirrors [`Bags`] into the VM through the
//! stock container path (`ui_items::feed::apply_container_source`, so `BAG_UPDATE` and the per-bag
//! diff are the stock ones), the drains that apply the bag gestures' intents to [`Bags`] (the
//! stock net drains stand down under [`crate::run_mode::SurvivorsUi`]), and the keys: B, Esc and
//! the keyboard stand-down.
//!
//! The systems are thin: the work is in plain functions over `&mut UiScript`, which the tests drive
//! without an app.

use std::collections::HashMap;

use bevy::prelude::*;

use benilla_ui::script::{
    ContainerSlot, ContainerState, InvSlotView, InventorySlots, ScriptValue, UiScript,
};

use super::inventory::{BagError, Bags, BACKPACK};
use super::items;
use crate::ui_items::feed::{apply_container_source, FeedMemory, SlotGuids};
use crate::ui_script::{PlayerUiHover, UiKeyboardCapture, VmMemo};

/// What the feed last did, per VM: a fresh VM gets every template and bag again.
#[derive(Default)]
pub(crate) struct BagFeed {
    memory: FeedMemory,
    /// The [`Bags::generation`] last pushed; `None` before the first push.
    pushed: Option<u64>,
    templates: bool,
    /// The purse last pushed.
    money: Option<u64>,
}

/// `Update`, after the input pass: apply this frame's bag gestures, then push what changed, so the
/// paint this frame shows the result.
pub(super) fn sync_bags(
    script: Option<NonSendMut<UiScript>>,
    mut bags: ResMut<Bags>,
    purse: Res<super::econ::Purse>,
    mut feed: Local<VmMemo<BagFeed>>,
) {
    let Some(mut script) = script else {
        return;
    };
    let feed = feed.get(&script);
    sync(
        &mut script,
        bags.bypass_change_detection(),
        purse.copper(),
        feed,
    );
}

/// [`sync_bags`]' body: `money` is the purse, in copper.
pub(crate) fn sync(script: &mut UiScript, bags: &mut Bags, money: u64, feed: &mut BagFeed) {
    if !feed.templates {
        for def in items::ITEMS {
            script.set_item_template(def.entry, items::template_view(def));
        }
        // The camp windows' addons, loaded with the curated interface, hear their load here: the
        // trainer's sets its filters on it.
        for (name, _) in crate::ui_script::SURVIVORS_UI_ADDONS {
            script.fire_event("ADDON_LOADED", vec![ScriptValue::Str((*name).to_string())]);
        }
        feed.templates = true;
    }
    // The purse, on the Backpack's money line and every window's: `PLAYER_MONEY` repaints them.
    if feed.money != Some(money) {
        script.set_money(money);
        script.fire_event("PLAYER_MONEY", vec![]);
        feed.money = Some(money);
    }
    apply_intents(script, bags);
    // Every authored template is in the store, so an ask is for an id survivors does not have.
    let unknown = script.take_item_stat_asks();
    if !unknown.is_empty() {
        debug!("survivors: tooltip asked for unauthored items {unknown:?}");
    }
    if feed.pushed != Some(bags.generation()) {
        push(script, bags, &mut feed.memory);
        feed.pushed = Some(bags.generation());
    }
}

/// Apply the gestures' queued intents to `bags`, each refusal shown as the stock error line.
pub(super) fn apply_intents(script: &mut UiScript, bags: &mut Bags) {
    let mut refused = Vec::new();
    for mv in script.take_container_moves() {
        if let Err(e) = bags.move_item(
            (mv.src_bag, mv.src_slot),
            (mv.dst_bag, mv.dst_slot),
            mv.count,
        ) {
            debug!("survivors: move {mv:?} refused: {e:?}");
            refused.push(e);
        }
    }
    for st in script.take_bag_autostores() {
        if let Err(e) = bags.autostore((st.src_bag, st.src_slot), st.dst_bag, st.count) {
            debug!("survivors: autostore {st:?} refused: {e:?}");
            refused.push(e);
        }
    }
    for (bag, slot, count) in script.take_container_destroys() {
        if let Err(e) = bags.destroy(bag, slot, count) {
            debug!("survivors: destroy {bag}/{slot} refused: {e:?}");
            refused.push(e);
        }
    }
    // Queues with no survivors meaning yet; taken so nothing else sends them anywhere.
    let equips = script.take_container_autoequips();
    let repairs = script.take_container_repairs();
    let wraps = script.take_container_wraps();
    if !(equips.is_empty() && repairs.is_empty() && wraps.is_empty()) {
        debug!("survivors: ignored equips {equips:?}, repairs {repairs:?}, wraps {wraps:?}");
    }
    for e in refused {
        show_error(script, e);
    }
}

/// The refusal's stock line through `UI_ERROR_MESSAGE`, which `UIErrorsFrame` prints.
fn show_error(script: &mut UiScript, e: BagError) {
    let key = e.global_string();
    let text = script
        .lua()
        .globals()
        .get::<String>(key)
        .unwrap_or_else(|_| key.to_string());
    script.fire_event("UI_ERROR_MESSAGE", vec![ScriptValue::Str(text)]);
}

/// The bags as the stock container snapshots, keyed by container id.
pub(super) fn snapshot(bags: &Bags) -> HashMap<i64, ContainerState> {
    // A cooldown's sweep, `(start ms on GetTime, duration ms, enabled)`, on its items' slots.
    let cooldown = |item: u32| {
        let cat = super::effects::consumable(item)?.kind.category()?;
        bags.cooldown(cat)
            .map(|start| ((start * 1000.0) as i64, (cat.secs() * 1000.0) as u32, true))
    };
    bags.containers()
        .map(|bag| {
            let name = if bag == BACKPACK {
                "Backpack".to_string()
            } else {
                items::template(bags.worn(bag)).map_or_else(String::new, |d| d.name.to_string())
            };
            let slots = bags
                .contents(bag)
                .filter_map(|(slot, stack)| {
                    let def = items::template(stack.item)?;
                    Some((
                        slot,
                        ContainerSlot {
                            texture: Some(items::icon_path(def)),
                            count: stack.count,
                            quality: Some(def.quality),
                            item_id: def.entry,
                            link: Some(items::link(def)),
                            // A Bind on Pickup item in the bags is already bound.
                            already_bound: def.bonding == 1,
                            cooldown: cooldown(def.entry),
                            ..Default::default()
                        },
                    ))
                })
                .collect();
            (
                bag,
                ContainerState {
                    name: Some(name),
                    num_slots: bags.size(bag),
                    slots,
                },
            )
        })
        .collect()
}

/// The worn bags as the doll's slots 20-23, which the bag windows' portraits read.
fn bag_slots(bags: &Bags) -> InventorySlots {
    let mut inv: InventorySlots = Default::default();
    for bag in 1..=4i64 {
        if let Some(def) = items::template(bags.worn(bag)) {
            inv[19 + bag as usize] = Some(InvSlotView {
                item_id: def.entry,
                icon: Some(items::icon_path(def)),
                count: 1,
                quality: def.quality as i32,
                name: Some(def.name.to_string()),
                link: Some(items::link(def)),
                ..Default::default()
            });
        }
    }
    inv
}

/// Push the bags through the stock diff: changed bags, then their `BAG_UPDATE`s.
fn push(script: &mut UiScript, bags: &Bags, memory: &mut FeedMemory) {
    // A synthetic guid per worn bag, so swapping one closes its window as the stock watcher does.
    let guids = SlotGuids {
        bags: std::array::from_fn(|i| {
            let entry = bags.worn(i as i64 + 1);
            if entry == 0 {
                0
            } else {
                0x4000_0000_0000_0000 | ((i as u64 + 1) << 32) | u64::from(entry)
            }
        }),
        ..SlotGuids::default()
    };
    script.set_inventory_slots(bag_slots(bags));
    apply_container_source(
        script,
        memory,
        Some(snapshot(bags)),
        guids,
        Vec::new(),
        Vec::new(),
    );
}

/// `Update`, in the input pass after the key dispatch: the bag keys and the stand-down.
pub(super) fn bag_keys(
    script: Option<NonSendMut<UiScript>>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    capture: Res<UiKeyboardCapture>,
    phase: Res<State<super::super::Phase>>,
    strip: Option<Res<super::super::strip::HandStrip>>,
    skills: Res<super::skills::Skills>,
) {
    let Some(mut script) = script else {
        return;
    };
    let playing = *phase.get() == super::super::Phase::Playing;
    let hand = super::super::strip::hand_up(*phase.get(), strip.as_deref());
    handle_keys(&mut script, &mut keys, &capture, playing && !hand, &skills);
}

/// The keys, in order:
/// - **stand-down:** a focused edit box takes every key, and a keyboard-enabled frame (the stack
///   split) the keys it ate, so no survivors key sees them this frame;
/// - **Esc** closes the stock windows first (popups, menus, bags), and only an Esc that closed
///   nothing reaches the survivors menu;
/// - **B** toggles every bag (stock `OpenAllBags`) while a run is playing and no hand is up (B
///   banishes then);
/// - **K** toggles the skills readout, the same.
pub(super) fn handle_keys(
    script: &mut UiScript,
    keys: &mut ButtonInput<KeyCode>,
    capture: &UiKeyboardCapture,
    bags_allowed: bool,
    skills: &super::skills::Skills,
) {
    if capture.typing {
        let pressed: Vec<KeyCode> = keys.get_just_pressed().copied().collect();
        for key in pressed {
            keys.clear_just_pressed(key);
        }
        return;
    }
    for key in &capture.consumed {
        keys.clear_just_pressed(*key);
    }
    if keys.just_pressed(KeyCode::Escape) && escape(script) {
        keys.clear_just_pressed(KeyCode::Escape);
    }
    if bags_allowed && keys.just_pressed(KeyCode::KeyB) {
        crate::ui_script::run_or_warn(script, "OpenAllBags()");
        keys.clear_just_pressed(KeyCode::KeyB);
    }
    if bags_allowed && keys.just_pressed(KeyCode::KeyK) {
        crate::ui_script::run_or_warn(script, &skills_readout(skills));
        keys.clear_just_pressed(KeyCode::KeyK);
    }
}

/// K's skills readout: P1's stand-in for the stock `SkillFrame`, which lives in
/// `CharacterFrame` and its pages, far outside the curated load. The stock `GameTooltip`, held at
/// the screen's centre, lists the professions and then the secondary skill, each as rank / cap; K
/// or Esc hides it.
pub(crate) fn skills_readout(skills: &super::skills::Skills) -> String {
    use super::skills::Line;
    let mut lua = String::from(
        "if SurvivorsSkills_Shown and GameTooltip:IsShown() then            GameTooltip:Hide() SurvivorsSkills_Shown = nil return end          GameTooltip:SetOwner(UIParent, 'ANCHOR_NONE') GameTooltip:ClearAllPoints()          GameTooltip:SetPoint('CENTER', UIParent, 'CENTER', 0, 120)          GameTooltip:SetText(SKILLS or 'Skills', 1, 1, 1) ",
    );
    for (secondary, header) in [
        (false, "TRADE_SKILLS or 'Professions'"),
        (true, "'Secondary Skills'"),
    ] {
        lua.push_str(&format!("GameTooltip:AddLine({header}, 1, 0.82, 0) "));
        for line in Line::ALL.iter().filter(|l| l.secondary() == secondary) {
            let r = skills.rank(*line);
            lua.push_str(&format!(
                "GameTooltip:AddDoubleLine('{}', '{}/{}', 1, 1, 1, 1, 1, 1) ",
                line.name(),
                r.rank,
                r.max
            ));
        }
    }
    lua.push_str("GameTooltip:Show() SurvivorsSkills_Shown = 1");
    lua
}

/// The survivors Esc arbitration (`SurvivorsUi_Escape`, the shim's): whether it closed something.
fn escape(script: &UiScript) -> bool {
    match script.eval::<Option<bool>>("return SurvivorsUi_Escape and SurvivorsUi_Escape() and true")
    {
        Ok(closed) => closed.unwrap_or(false),
        Err(e) => {
            warn!("survivors: SurvivorsUi_Escape failed: {e}");
            false
        }
    }
}

/// `PreUpdate`, after Bevy's UI focus: while the pointer is on a stock frame (the hover the input
/// pass found last frame), no survivors HUD node is pressed or hovered, so a click on a bag slot
/// never also clicks the HUD button under it. The wowcam half needs nothing here: the same hover
/// already makes `PointerOverUi`, which keeps the click from starting a mouselook.
pub(super) fn bags_take_hud_pointer(hover: Res<PlayerUiHover>, mut nodes: Query<&mut Interaction>) {
    if hover.0.is_none() {
        return;
    }
    for mut interaction in &mut nodes {
        interaction.set_if_neq(Interaction::None);
    }
}

/// `OnExit(Playing)`: a modal survivors screen (the level-up, the jewelry choice, the results, the
/// menu, the pause) closes every stock window as it opens.
pub(super) fn close_windows(script: Option<NonSend<UiScript>>) {
    if let Some(script) = script {
        crate::ui_script::run_or_warn(
            &script,
            "if CloseAllWindows then CloseAllWindows() end \
             if StackSplitFrame and StackSplitFrame:IsShown() then StackSplitFrame:Hide() end",
        );
    }
}

/// `WOW_SURVIVORS_OPEN=bags` (the screenshot aid): open every bag and hover the first item once a
/// run is playing, then save `<WOW_SURVIVORS_SHOTS>/bags.png` a moment later when that is set.
#[derive(Resource)]
pub(super) struct OpenBagsAid {
    pub(super) done: bool,
    pub(super) shot_dir: Option<std::path::PathBuf>,
    pub(super) shot_t: Option<f32>,
}

/// Seconds from the bags opening to the screenshot: the windows fade in over a few frames.
const SHOT_DELAY: f32 = 1.5;

/// The aid's system.
pub(super) fn open_bags_aid(
    mut commands: Commands,
    script: Option<NonSendMut<UiScript>>,
    time: Res<Time<Real>>,
    phase: Res<State<super::super::Phase>>,
    loading: Option<Res<crate::loading_screen::LoadingScreen>>,
    mut aid: ResMut<OpenBagsAid>,
) {
    let Some(mut script) = script else {
        return;
    };
    if !aid.done
        && *phase.get() == super::super::Phase::Playing
        && !loading.is_some_and(|l| l.covering())
    {
        aid.done = true;
        open_and_hover(&mut script);
        if aid.shot_dir.is_some() {
            aid.shot_t = Some(0.0);
        }
    }
    let Some(t) = aid.shot_t else { return };
    let t = t + time.delta_secs();
    if t < SHOT_DELAY {
        aid.shot_t = Some(t);
        return;
    }
    aid.shot_t = None;
    if let Some(dir) = &aid.shot_dir {
        let path = dir.join("bags.png");
        info!("survivors: bags screenshot -> {}", path.display());
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
}

/// Open every bag and put the first item's tooltip up, as a hover would.
pub(super) fn open_and_hover(script: &mut UiScript) {
    crate::ui_script::run_or_warn(
        script,
        "OpenAllBags(1) \
         local i = IsBagOpen(0) \
         if i then \
           local w = 'ContainerFrame'..i \
           for j = 1, MAX_CONTAINER_ITEMS do \
             local b = getglobal(w..'Item'..j) \
             if b and b:IsShown() and b:GetID() == 1 then \
               this = b ContainerFrameItemButton_OnEnter() break \
             end \
           end \
         end",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survivors::professions::items::{COPPER_ORE, LINEN_BAG, LINEN_CLOTH};
    use crate::survivors::professions::skills::Skills;

    fn vm() -> UiScript {
        UiScript::new().expect("a VM")
    }

    #[test]
    fn survivors_frames_snapshot_mirrors_the_bags() {
        let bags = Bags::test_inventory();
        let snap = snapshot(&bags);
        assert_eq!(snap.len(), 2, "the Backpack and the worn Linen Bag");
        assert_eq!(snap[&0].num_slots, 16);
        assert_eq!(snap[&0].name.as_deref(), Some("Backpack"));
        assert_eq!(snap[&1].name.as_deref(), Some("Linen Bag"));
        assert_eq!(snap[&1].num_slots, 6);
        let ore = snap[&0]
            .slots
            .values()
            .find(|s| s.item_id == COPPER_ORE)
            .expect("ore in the backpack");
        assert_eq!(
            ore.texture.as_deref(),
            Some("Interface\\Icons\\INV_Ore_Copper_01")
        );
        assert!(ore.link.as_deref().unwrap().contains("[Copper Ore]"));
    }

    /// The feed pushes into the engine's container store, and only when the bags changed.
    #[test]
    fn survivors_frames_feed_pushes_bags_and_templates() {
        let mut s = vm();
        let mut bags = Bags::test_inventory();
        let mut feed = BagFeed::default();
        sync(&mut s, &mut bags, 12_345, &mut feed);
        assert_eq!(
            s.eval::<i64>("return GetMoney()").unwrap(),
            12_345,
            "the purse"
        );
        assert_eq!(s.eval::<i64>("return GetContainerNumSlots(0)").unwrap(), 16);
        assert_eq!(s.eval::<i64>("return GetContainerNumSlots(1)").unwrap(), 6);
        assert_eq!(
            s.eval::<String>("return GetBagName(1)").unwrap(),
            "Linen Bag"
        );
        let (tex, count): (String, i64) = s
            .eval("local t, c = GetContainerItemInfo(0, 1) return t, c")
            .unwrap();
        assert!(tex.starts_with("Interface\\Icons\\"), "{tex}");
        assert!(count >= 1);
        // `GetItemInfo` answers from the pushed templates.
        assert_eq!(
            s.eval::<String>(&format!("return (GetItemInfo({LINEN_BAG}))"))
                .unwrap(),
            "Linen Bag"
        );
        assert_eq!(feed.pushed, Some(bags.generation()));
    }

    /// The stock gesture: pick up the backpack's first stack, put it down in the Linen Bag. The
    /// queued move lands in [`Bags`], and the next push shows it.
    #[test]
    fn survivors_frames_a_drag_between_slots_moves_the_item_in_bags() {
        let mut s = vm();
        let mut bags = Bags::test_inventory();
        let mut feed = BagFeed::default();
        sync(&mut s, &mut bags, 0, &mut feed);
        let first = bags
            .get(0, 1)
            .expect("the backpack's first slot holds something");
        assert!(
            bags.get(1, 6).is_none(),
            "the Linen Bag's last slot starts empty"
        );

        s.run("PickupContainerItem(0, 1) PickupContainerItem(1, 6)")
            .unwrap();
        sync(&mut s, &mut bags, 0, &mut feed);
        assert_eq!(bags.get(0, 1), None, "the source is empty");
        assert_eq!(bags.get(1, 6), Some(first), "the stack is in the bag");
        let id: i64 = s
            .eval("return (GetContainerItemInfo(1, 6)) and 1 or 0")
            .unwrap();
        assert_eq!(id, 1, "and the VM was told");
        assert!(
            s.take_container_moves().is_empty(),
            "the move was drained exactly once"
        );
    }

    /// A split (`SplitContainerItem`, the stack-split frame's verb) places part of a stack.
    #[test]
    fn survivors_frames_a_split_lands_in_bags() {
        let mut s = vm();
        let mut bags = Bags::with_worn([LINEN_BAG, 0, 0, 0]);
        bags.add(LINEN_CLOTH, 20);
        let mut feed = BagFeed::default();
        sync(&mut s, &mut bags, 0, &mut feed);
        s.run("SplitContainerItem(0, 1, 7) PickupContainerItem(0, 4)")
            .unwrap();
        sync(&mut s, &mut bags, 0, &mut feed);
        assert_eq!(bags.get(0, 1).map(|st| st.count), Some(13));
        assert_eq!(bags.get(0, 4).map(|st| st.count), Some(7));
    }

    /// `DeleteCursorItem`, the stock delete popup's accept, removes the stack.
    #[test]
    fn survivors_frames_a_destroy_removes_the_stack() {
        let mut s = vm();
        let mut bags = Bags::with_worn([0, 0, 0, 0]);
        bags.add(COPPER_ORE, 3);
        let mut feed = BagFeed::default();
        sync(&mut s, &mut bags, 0, &mut feed);
        s.run("PickupContainerItem(0, 1) DeleteCursorItem()")
            .unwrap();
        sync(&mut s, &mut bags, 0, &mut feed);
        assert_eq!(bags.get(0, 1), None);
    }

    /// A press on a HUD node under a stock frame never reaches the HUD.
    #[test]
    fn survivors_frames_a_bag_over_the_hud_takes_its_clicks() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.insert_resource(PlayerUiHover(None));
        let button = world.spawn(Interaction::Pressed).id();
        world.run_system_once(bags_take_hud_pointer).unwrap();
        assert_eq!(
            *world.get::<Interaction>(button).unwrap(),
            Interaction::Pressed,
            "off the stock frames the HUD keeps its press"
        );
        world.insert_resource(PlayerUiHover(Some(7)));
        world.run_system_once(bags_take_hud_pointer).unwrap();
        assert_eq!(
            *world.get::<Interaction>(button).unwrap(),
            Interaction::None,
            "over a bag the press is the bag's"
        );
    }

    /// B toggles the bags, Esc closes them first, and a focused box takes the keys.
    #[test]
    fn survivors_frames_keys_b_esc_and_the_stand_down() {
        let mut s = vm();
        // The stock verbs, stood in: the real ones need the client's FrameXML (the data-gated
        // test in `ui_script::survivors_ui` drives those).
        s.run(
            "OPEN = 0 function OpenAllBags() OPEN = 1 - OPEN end \
             function SurvivorsUi_Escape() if OPEN == 1 then OPEN = 0 return 1 end end",
        )
        .unwrap();
        let mut keys = ButtonInput::<KeyCode>::default();
        let capture = UiKeyboardCapture::default();

        keys.press(KeyCode::KeyB);
        handle_keys(&mut s, &mut keys, &capture, true, &Skills::default());
        assert_eq!(s.eval::<i64>("return OPEN").unwrap(), 1, "B opens");
        assert!(!keys.just_pressed(KeyCode::KeyB), "and B is spent");

        keys.reset_all();
        keys.press(KeyCode::Escape);
        handle_keys(&mut s, &mut keys, &capture, true, &Skills::default());
        assert_eq!(s.eval::<i64>("return OPEN").unwrap(), 0, "Esc closes");
        assert!(
            !keys.just_pressed(KeyCode::Escape),
            "and the survivors menu never sees it"
        );

        keys.reset_all();
        keys.press(KeyCode::Escape);
        handle_keys(&mut s, &mut keys, &capture, true, &Skills::default());
        assert!(
            keys.just_pressed(KeyCode::Escape),
            "with nothing open, Esc reaches the survivors menu"
        );

        // While the hand is up (or off the playing screen), B is the hand's.
        keys.reset_all();
        keys.press(KeyCode::KeyB);
        handle_keys(&mut s, &mut keys, &capture, false, &Skills::default());
        assert_eq!(s.eval::<i64>("return OPEN").unwrap(), 0);
        assert!(keys.just_pressed(KeyCode::KeyB), "B is left for the banish");

        // A focused edit box takes every key.
        keys.reset_all();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::KeyB);
        let typing = UiKeyboardCapture {
            typing: true,
            ..Default::default()
        };
        handle_keys(&mut s, &mut keys, &typing, true, &Skills::default());
        assert_eq!(s.eval::<i64>("return OPEN").unwrap(), 0);
        assert!(!keys.just_pressed(KeyCode::KeyW) && !keys.just_pressed(KeyCode::KeyB));
    }
}
