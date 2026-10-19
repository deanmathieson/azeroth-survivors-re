//! Survivors' curated FrameXML (docs/PROFESSIONS.md, 2.1-2.2): under
//! [`crate::run_mode::SurvivorsUi`] the world entry loads only the `FrameXML.toc` rows the bags,
//! the item tooltip and the camp's windows (the merchant, the trainer, the tradeskill window)
//! need, in the toc's own order, then the layer files that adapt them, then the two
//! LoadOnDemand addons those windows live in ([`ADDONS`]), loaded eagerly as chain files. No
//! `Bindings.xml` (so no stock key does anything), no action bars, unit frames, chat, minimap or
//! buff frame (so nothing fights the survivors HUD), no third-party addons.
//!
//! What those rows reach in files left out is stood in by [`STAND_INS`], run right after
//! `UIParent.xml` as the test kits do (`tests/common/mod.rs`): no-op callees, hidden stand-in
//! frames for the bag bar, the chat box and the windows the bag code indexes unguarded, and a
//! survivors `UIParent_ManageFramePositions` that only seats the bags.

use bevy::prelude::*;

use benilla_ui::script::UiScript;

use super::addons::Addon;

/// The `FrameXML.toc` rows survivors loads, by file name; the toc decides the order.
pub(super) const ROWS: &[&str] = &[
    "GlobalStrings.lua",
    "Fonts.xml",
    "BasicControls.xml",
    "UIParent.xml",
    "MoneyFrame.lua",
    "MoneyFrame.xml",
    "GameTooltip.xml",
    "UIMenu.xml",
    "UIDropDownMenu.xml",
    "UIPanelTemplates.lua",
    "UIPanelTemplates.xml",
    "UIErrorsFrame.xml",
    // The delete confirm (`DELETE_ITEM`).
    "StaticPopup.xml",
    // The shift-click stack split.
    "StackSplitFrame.xml",
    // `CooldownFrame_SetTimer`, which every bag slot's update calls.
    "Cooldown.xml",
    "ItemButtonTemplate.xml",
    "ContainerFrame.xml",
    // The camp goblin's window.
    "MerchantFrame.xml",
    // The list and detail templates the trainer and tradeskill windows inherit.
    "ClassTrainerFrameTemplates.xml",
    // The gathers' casting bar (`SPELLCAST_START` and the rest).
    "CastingBarFrame.xml",
];

/// Template files no toc row names (the full load reaches them through another row's
/// `<Include>`), each loaded off the chain right after the row it follows: `(row, file)`. The
/// merchant's tabs inherit `CharacterFrameTabButtonTemplate`, and the tradeskill window's reagent
/// slots `QuestItemTemplate`.
pub(super) const EXTRAS: &[(&str, &str)] = &[
    (
        "UIPanelTemplates.xml",
        r"Interface\FrameXML\CharacterFrameTemplates.xml",
    ),
    (
        "ItemButtonTemplate.xml",
        r"Interface\FrameXML\QuestFrameTemplates.xml",
    ),
];

/// The layer files survivors loads, from `assets/ui`, in the layer's order: the scroll parts the
/// windows' lists use, and `OpenBackpack` over every bag.
pub(super) const LAYER: &[&str] = &["ScrollTemplates.xml", "ContainerFrameAdapters.xml"];

/// The LoadOnDemand addons the camp's windows live in, `(name, file)`: loaded with the rest rather
/// than on demand (the curated load has no addon registry for `LoadAddOn`). Their
/// `ADDON_LOADED`s fire with the bags' first push (`survivors::professions::frames`), which holds
/// the VM mutably; the stand-ins make the stock `*_LoadUI` callers no-ops.
pub(crate) const ADDONS: &[(&str, &str)] = &[
    (
        "Blizzard_TrainerUI",
        r"Interface\AddOns\Blizzard_TrainerUI\Blizzard_TrainerUI.xml",
    ),
    (
        "Blizzard_TradeSkillUI",
        r"Interface\AddOns\Blizzard_TradeSkillUI\Blizzard_TradeSkillUI.xml",
    ),
];

/// Run after `UIParent.xml`. Every callee is a plain global write that a real definition would
/// replace, but none of their files load in this mode.
pub(super) const STAND_INS: &str = r#"
-- Callees of UIParent.lua's OnUpdate and OnEvent arms and of ContainerFrame.lua, in files the
-- curated load leaves out: no-ops.
FCF_OnUpdate = FCF_OnUpdate or function() end
FCF_DockUpdate = FCF_DockUpdate or function() end
UnitPopup_OnUpdate = UnitPopup_OnUpdate or function() end
BattlefieldFrame_OnUpdate = BattlefieldFrame_OnUpdate or function() end
MultiActionBar_Update = MultiActionBar_Update or function() end
RaidOptionsFrame_UpdatePartyFrames = RaidOptionsFrame_UpdatePartyFrames or function() end
LocalizeFrames = LocalizeFrames or function() end
UpdateNameplates = UpdateNameplates or function() end
UpdateMicroButtons = UpdateMicroButtons or function() end
UpdateBagButtonHighlight = UpdateBagButtonHighlight or function() end
-- The reference's values, from the files that declare them (MainMenuBarBagButtons.lua,
-- BankFrame.lua).
KEYRING_CONTAINER = KEYRING_CONTAINER or -2
NUM_BAG_SLOTS = NUM_BAG_SLOTS or 4
NUM_BANKBAGSLOTS = NUM_BANKBAGSLOTS or 6

-- The trainer and tradeskill addons are loaded with the rest (`ADDONS`), so UIParent's openers
-- have nothing to load: no `LoadAddOn`, whose failure would raise the stock message box.
function ClassTrainerFrame_LoadUI() end
function TradeSkillFrame_LoadUI() end

-- Frames the bag and window code indexes unguarded, hidden: the bag bar's buttons (the windows
-- light them), the chat box (shift-click links into it when shown), the bank (the column layout
-- measures it) and the four options windows (every window close asks whether one is open).
local function survivors_seat(kind, names)
    for _, name in ipairs(names) do
        if not getglobal(name) then
            local f = CreateFrame(kind, name, UIParent)
            f:Hide()
        end
    end
end
survivors_seat("CheckButton", { "MainMenuBarBackpackButton", "CharacterBag0Slot",
    "CharacterBag1Slot", "CharacterBag2Slot", "CharacterBag3Slot", "KeyRingButton" })
survivors_seat("EditBox", { "ChatFrameEditBox" })
survivors_seat("Frame", { "BankFrame", "GameMenuFrame", "OptionsFrame", "UIOptionsFrame",
    "SoundOptionsFrame" })
survivors_seat("Button", { "OptionsFrameCancel" })
-- `updateContainerFrameAnchors` measures every column against `BankFrame:GetRight()`: a one-unit
-- stand-in on the screen's left edge leaves the whole width to the bags.
BankFrame:ClearAllPoints()
BankFrame:SetPoint("TOPLEFT", UIParent, "TOPLEFT", 0, 0)
BankFrame:SetWidth(1)
BankFrame:SetHeight(1)

-- None of the managed frames exist, so the pass only seats the bags: bottom-right, raised by
-- SURVIVORS_CONTAINER_OFFSET_Y (the host sets it above the HUD's bars).
function UIParent_ManageFramePositions()
    CONTAINER_OFFSET_X = SURVIVORS_CONTAINER_OFFSET_X or 0
    CONTAINER_OFFSET_Y = SURVIVORS_CONTAINER_OFFSET_Y or 70
    if updateContainerFrameAnchors then updateContainerFrameAnchors() end
end

-- Esc's first claim, as the stock game menu's order has it: a popup, the stack split, a menu,
-- then every window and bag. True when it closed something; only an Esc that closed nothing
-- reaches the survivors menu.
function SurvivorsUi_Escape()
    if StaticPopup_EscapePressed and StaticPopup_EscapePressed() then return 1 end
    if SurvivorsSkills_Shown and GameTooltip:IsShown() then
        GameTooltip:Hide() SurvivorsSkills_Shown = nil return 1
    end
    if StackSplitFrame and StackSplitFrame:IsShown() then StackSplitFrame:Hide() return 1 end
    if CloseMenus and CloseMenus() then return 1 end
    if CloseAllWindows and CloseAllWindows() then return 1 end
    return nil
end
"#;

/// The file name of a toc row.
fn leaf(row: &str) -> &str {
    row.rsplit(['\\', '/']).next().unwrap_or(row)
}

/// The core cut down to [`ROWS`], in the toc's order.
pub(super) fn curate(mut core: Addon) -> Addon {
    core.toc
        .files
        .retain(|f| ROWS.iter().any(|r| r.eq_ignore_ascii_case(leaf(f))));
    core
}

/// What runs after a core row has loaded: its [`EXTRAS`], and [`STAND_INS`] after `UIParent.xml`.
pub(super) fn after_row(script: &UiScript, row: &str) -> Vec<String> {
    let extras: Vec<String> = EXTRAS
        .iter()
        .filter(|(after, _)| leaf(row).eq_ignore_ascii_case(after))
        .map(|(_, file)| (*file).to_string())
        .collect();
    if !extras.is_empty() {
        return super::reference_ui::addon(Vec::new()).load_files(script, &extras);
    }
    if !leaf(row).eq_ignore_ascii_case("UIParent.xml") {
        return Vec::new();
    }
    match script.run(STAND_INS) {
        Ok(()) => Vec::new(),
        Err(e) => {
            error!("ui_script: the survivors stand-ins failed: {e}");
            vec![format!("survivors stand-ins: {e}")]
        }
    }
}

/// [`LAYER`], after the core, then [`ADDONS`] off the player's chain.
pub(super) fn load_layer(script: &UiScript) -> Vec<String> {
    let layer = Addon::layer();
    let files: Vec<String> = layer
        .toc
        .files
        .iter()
        .filter(|f| LAYER.iter().any(|l| l.eq_ignore_ascii_case(f)))
        .cloned()
        .collect();
    let mut failures = layer.load_files(script, &files);
    if super::reference_ui::core().is_none() {
        // No chain, so no addons either; the missing core is already reported.
        return failures;
    }
    let files: Vec<String> = ADDONS.iter().map(|(_, f)| (*f).to_string()).collect();
    failures.extend(super::reference_ui::addon(Vec::new()).load_files(script, &files));
    failures
}

/// The whole curated load in one call, for a `ReloadUI()` and the tests; the world entry slices
/// the same steps across frames (`lifecycle::run_pending_entry_load`). Returns every failure, a
/// handler's raise during the walk included.
pub(crate) fn load(script: &UiScript) -> Vec<String> {
    script.set_instruction_budget(super::addons::LOAD_INSTRUCTION_BUDGET);
    script.register_cvars(crate::cvars::registered_pairs());
    let mut failures = Vec::new();
    match super::reference_ui::core() {
        Some(core) => {
            let core = curate(core);
            let mut toc = benilla_ui::status::Status::default();
            for row in &core.toc.files {
                failures.extend(core.load_files_into(script, std::slice::from_ref(row), &mut toc));
                failures.extend(after_row(script, row));
            }
            super::manifest::report_core_status(script, toc);
        }
        None => failures.extend(super::manifest::load_core_missing_unbound(script)),
    }
    failures.extend(load_layer(script));
    failures.extend(super::manifest::bootstrap_positions(script));
    failures.extend(
        script
            .errors()
            .into_iter()
            .map(|e| format!("a handler raised during the load walk: {e}")),
    );
    failures
}

#[cfg(test)]
mod tests {
    use benilla_ui::script::{ScriptValue, UiScript};

    use crate::survivors::professions::test_support as bags;

    /// The curated rows are real `FrameXML.toc` rows, so a renamed or missing one shows here.
    #[test]
    fn survivors_ui_rows_are_rows_of_the_players_toc() {
        let _data = benilla_formats::wow_data_or_skip!();
        let core = super::super::reference_ui::core().expect("the player's FrameXML.toc");
        let curated = super::curate(core);
        let mut missing: Vec<&str> = super::ROWS
            .iter()
            .copied()
            .filter(|r| {
                !curated
                    .toc
                    .files
                    .iter()
                    .any(|f| super::leaf(f).eq_ignore_ascii_case(r))
            })
            .collect();
        missing.sort_unstable();
        assert!(missing.is_empty(), "not in FrameXML.toc: {missing:?}");
        assert_eq!(curated.toc.files.len(), super::ROWS.len());
    }

    /// The opt-in's load: every row, the stand-ins and the layer, with no load failure and no
    /// handler raising, and the windows it is for. The time it takes is printed for the doc.
    #[test]
    fn survivors_ui_loads_the_curated_framexml_without_lua_errors() {
        let _data = benilla_formats::wow_data_or_skip!();
        let mut s = UiScript::new().unwrap();
        s.set_screen_size(1365.0, 768.0);
        let t0 = std::time::Instant::now();
        let failures = super::load(&s);
        crate::ui_script::lifecycle::finish_ui_load(&mut s);
        let curated_ms = t0.elapsed().as_secs_f64() * 1000.0;
        assert!(failures.is_empty(), "load failures: {failures:#?}");
        assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
        for frame in [
            "ContainerFrame1",
            "ContainerFrame12",
            "GameTooltip",
            "StaticPopup1",
            "StackSplitFrame",
            "UIErrorsFrame",
            "CastingBarFrame",
            "MerchantFrame",
            "ClassTrainerFrame",
            "TradeSkillFrame",
        ] {
            assert!(
                s.eval::<bool>(&format!("return {frame} ~= nil")).unwrap(),
                "{frame} exists"
            );
        }
        // Nothing the HUD would have to share the screen with, and no stock key command.
        for absent in [
            "MainMenuBar",
            "PlayerFrame",
            "ChatFrame1",
            "Minimap",
            "BuffFrame",
        ] {
            assert!(
                s.eval::<bool>(&format!("return {absent} == nil")).unwrap(),
                "{absent} is not loaded"
            );
        }
        assert_eq!(s.eval::<i64>("return GetNumBindings()").unwrap(), 0);
        assert!(
            !s.eval::<bool>("return MainMenuBarBackpackButton:IsShown() == 1")
                .unwrap(),
            "the bag bar stand-in stays hidden"
        );

        // The full interface on the same machine, for the doc's comparison.
        let full = UiScript::new().unwrap();
        let t1 = std::time::Instant::now();
        let _ = crate::ui_script::load_default_ui(&full);
        let full_ms = t1.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "survivors_ui load: curated {curated_ms:.0} ms ({} rows), full {full_ms:.0} ms",
            super::ROWS.len()
        );
    }

    /// The bags work end to end on the real files: B's verb opens them, a drag queues the move
    /// the survivors drain applies to `Bags`, a tooltip shows, the delete popup and the stack
    /// split work, and Esc closes the bags first. No handler raises along the way.
    #[test]
    fn survivors_ui_bags_open_move_split_destroy_and_close() {
        let _data = benilla_formats::wow_data_or_skip!();
        let mut s = UiScript::new().unwrap();
        s.set_screen_size(1365.0, 768.0);
        assert!(super::load(&s).is_empty());
        crate::ui_script::lifecycle::finish_ui_load(&mut s);
        let mut bags = bags::Fixture::new();
        bags.sync(&mut s);

        // B: the stock `OpenAllBags`, as the key handler runs it.
        bags.press_b(&mut s);
        assert!(
            s.eval::<bool>("return IsBagOpen(0) ~= nil and IsBagOpen(1) ~= nil")
                .unwrap(),
            "B opens the backpack and the Linen Bag"
        );
        let (right, bottom): (f64, f64) = s
            .eval("local f = getglobal('ContainerFrame'..IsBagOpen(0)) return f:GetRight(), f:GetBottom()")
            .unwrap();
        assert!(
            (right - 1365.0).abs() < 1.0 && bottom >= 70.0,
            "the backpack sits bottom-right above the HUD ({right}, {bottom})"
        );

        // Drag the first stack into the Linen Bag's last slot, through the buttons' own OnClick.
        let first = bags.bags.get(0, 1).expect("a first stack");
        click(&s, 0, 1);
        click(&s, 1, 6);
        bags.sync(&mut s);
        assert_eq!(bags.bags.get(0, 1), None);
        assert_eq!(bags.bags.get(1, 6), Some(first), "the move landed in Bags");

        // A hover shows the item's own tooltip.
        s.run(&format!(
            "this = {} ContainerFrameItemButton_OnEnter()",
            button(&s, 1, 6)
        ))
        .unwrap();
        let def = crate::survivors::professions::items::template(first.item).unwrap();
        assert_eq!(
            s.eval::<String>("return GameTooltipTextLeft1:GetText()")
                .unwrap(),
            def.name
        );
        s.run("GameTooltip:Hide()").unwrap();

        // The stack split frame opens off a button and Esc closes it before anything else.
        s.run(&format!(
            "OpenStackSplitFrame(5, {}, 'BOTTOMRIGHT', 'TOPRIGHT')",
            button(&s, 1, 6)
        ))
        .unwrap();
        assert!(s
            .eval::<bool>("return StackSplitFrame:IsShown() == 1")
            .unwrap());
        assert!(bags.escape(&mut s), "Esc closed the split");
        assert!(!s
            .eval::<bool>("return StackSplitFrame:IsShown() == 1")
            .unwrap());

        // The delete popup's accept destroys the held stack.
        s.run("PickupContainerItem(1, 6) StaticPopup_Show('DELETE_ITEM', 'x') StaticPopup1Button1:Click()")
            .unwrap();
        bags.sync(&mut s);
        assert_eq!(bags.bags.get(1, 6), None, "destroyed");

        // Stock events the bags and UIParent listen for raise nothing.
        for (event, args) in [
            ("BAG_UPDATE", vec![ScriptValue::Int(0)]),
            ("ITEM_LOCK_CHANGED", vec![]),
            ("CURSOR_UPDATE", vec![]),
            ("BAG_UPDATE_COOLDOWN", vec![]),
            (
                "UNIT_INVENTORY_CHANGED",
                vec![ScriptValue::Str("player".into())],
            ),
            ("UPDATE_INVENTORY_ALERTS", vec![]),
            ("PLAYER_ENTERING_WORLD", vec![]),
            (
                "UI_ERROR_MESSAGE",
                vec![ScriptValue::Str("Inventory is full.".into())],
            ),
            (
                "UI_INFO_MESSAGE",
                vec![ScriptValue::Str("You receive loot.".into())],
            ),
            (
                "SPELLCAST_START",
                vec![ScriptValue::Str("Mining".into()), ScriptValue::Int(1500)],
            ),
            ("SPELLCAST_DELAYED", vec![ScriptValue::Int(250)]),
        ] {
            s.fire_event(event, args);
        }
        assert!(
            s.eval::<bool>("return CastingBarFrame:IsShown() == 1")
                .unwrap(),
            "a gather's casting bar shows"
        );
        s.fire_event("SPELLCAST_STOP", vec![]);
        // K's skills readout is the stock tooltip, and Esc hides it before anything else.
        s.run(&crate::survivors::professions::frames::skills_readout(
            &crate::survivors::professions::skills::Skills::default(),
        ))
        .unwrap();
        assert!(s.eval::<bool>("return GameTooltip:IsShown() == 1").unwrap());
        assert_eq!(
            s.eval::<String>("return GameTooltipTextLeft3:GetText()")
                .unwrap(),
            "Mining"
        );
        assert!(bags.escape(&mut s), "Esc hid the readout");
        assert!(!s.eval::<bool>("return GameTooltip:IsShown() == 1").unwrap());

        // Esc: the bags close first, then there is nothing left to close.
        assert!(bags.escape(&mut s), "Esc closed the bags");
        assert!(s.eval::<bool>("return IsBagOpen(0) == nil").unwrap());
        assert!(!bags.escape(&mut s), "a second Esc is the survivors menu's");
        assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    }

    /// The camp's windows on the real files, driven as `professions::stations` drives them: the
    /// purse on the Backpack's money line; the goblin's `MerchantFrame` taking a dropped stack and
    /// selling Coal, its buyback tab, and Esc closing it; the trainer's `ClassTrainerFrame` selling
    /// Journeyman Mining; Smelting's `TradeSkillFrame` making two Copper Bars under the casting bar;
    /// and the camp's end closing whatever is open. No handler raises along the way.
    #[test]
    fn survivors_ui_camp_windows_merchant_trainer_and_smelting() {
        use crate::survivors::professions::{
            econ::Purse, frames, gather, inventory::Bags, items, skills, stations,
        };
        let _data = benilla_formats::wow_data_or_skip!();
        let mut s = UiScript::new().unwrap();
        s.set_screen_size(1365.0, 768.0);
        assert!(super::load(&s).is_empty());
        crate::ui_script::lifecycle::finish_ui_load(&mut s);
        let mut w = stations::tests::World {
            bags: Bags::test_inventory(),
            purse: Purse::with(123_456),
            skills: skills::Skills::from_save(&std::collections::BTreeMap::from([(
                "mining".to_string(),
                skills::Rank { rank: 60, max: 75 },
            )])),
            ..Default::default()
        };
        let mut feed = frames::BagFeed::default();
        let mut sync = |s: &mut UiScript, w: &mut stations::tests::World| {
            frames::sync(s, &mut w.bags, w.purse.copper(), &mut feed);
            for line in std::mem::take(&mut w.lines) {
                gather::announce(s, &line);
            }
        };
        sync(&mut s, &mut w);
        let lua_true = |s: &UiScript, expr: &str| {
            s.eval::<bool>(&format!("return ({expr}) and true or false"))
                .unwrap()
        };

        // The purse on the Backpack's money line: 12g 34s 56c.
        s.run("OpenAllBags()").unwrap();
        assert_eq!(
            s.eval::<String>(
                "return getglobal('ContainerFrame'..IsBagOpen(0)..'MoneyFrameGoldButton'):GetText()"
            )
            .unwrap(),
            "12"
        );

        // The goblin: his goods, a dropped stack sold, Coal bought.
        w.merchant = true;
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert!(lua_true(&s, "MerchantFrame:IsVisible()"));
        assert_eq!(
            s.eval::<String>("return MerchantItem1Name:GetText()")
                .unwrap(),
            "Coal"
        );
        s.run("PickupContainerItem(0, 1) PickupMerchantItem(0)")
            .unwrap();
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert_eq!(w.bags.get(0, 1), None, "the first stack is sold");
        assert!(w.purse.copper() > 123_456);
        let purse = w.purse.copper();
        s.run("BuyMerchantItem(1, 1)").unwrap();
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert_eq!(w.bags.count(items::COAL), 1);
        assert_eq!(w.purse.copper(), purse - 500);
        // The buyback tab shows the sale, and back again.
        s.run("MerchantFrameTab2:Click()").unwrap();
        assert_eq!(s.eval::<i64>("return GetNumBuybackItems()").unwrap(), 1);
        s.run("MerchantFrameTab1:Click()").unwrap();
        // Esc closes it, and the goblin's own window with it.
        assert!(bags::escape_closes(&mut s));
        w.frame(&mut s, true, 0.016);
        assert!(!w.merchant && !w.shop.open);
        assert!(lua_true(&s, "not MerchantFrame:IsVisible()"));

        // The trainer: Journeyman Mining, bought off its row.
        w.desk.trainer = true;
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert!(lua_true(&s, "ClassTrainerFrame:IsVisible()"));
        s.run(
            "for i = 1, GetNumTrainerServices() do \
               local name, rank = GetTrainerServiceInfo(i) \
               if name == 'Mining' and rank == 'Journeyman' then BuyTrainerService(i) end \
             end",
        )
        .unwrap();
        let purse = w.purse.copper();
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert_eq!(w.skills.rank(skills::Line::Mining).max, 150);
        assert_eq!(w.purse.copper(), purse - 10_000);
        s.run("HideUIPanel(ClassTrainerFrame)").unwrap();
        w.frame(&mut s, true, 0.016);
        assert!(!w.desk.trainer, "its close closes the desk's");

        // Smelting: two Copper Bars, the casting bar up while they cast.
        w.desk.smelting = true;
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert!(lua_true(&s, "TradeSkillFrame:IsVisible()"));
        let bars = w.bags.count(items::COPPER_BAR);
        s.run(
            "for i = 1, GetNumTradeSkills() do \
               if GetTradeSkillInfo(i) == 'Smelt Copper' then DoTradeSkill(i, 2) end \
             end",
        )
        .unwrap();
        w.frame(&mut s, true, 0.016);
        sync(&mut s, &mut w);
        assert!(lua_true(&s, "CastingBarFrame:IsShown()"));
        for _ in 0..2 {
            w.frame(
                &mut s,
                true,
                crate::survivors::professions::recipes::SMELT_SECS,
            );
            sync(&mut s, &mut w);
        }
        assert_eq!(w.bags.count(items::COPPER_BAR), bars + 2);

        // The camp ends: Smelting closes.
        w.frame(&mut s, false, 0.016);
        sync(&mut s, &mut w);
        assert!(lua_true(&s, "not TradeSkillFrame:IsVisible()"));
        assert!(s.errors().is_empty(), "script errors: {:?}", s.errors());
    }

    /// The `ContainerFrame<n>Item<m>` showing bag `bag` slot `slot`.
    fn button(s: &UiScript, bag: i64, slot: u32) -> String {
        s.eval::<String>(&format!(
            "local w = 'ContainerFrame'..IsBagOpen({bag}) \
             for j = 1, MAX_CONTAINER_ITEMS do \
               local b = getglobal(w..'Item'..j) \
               if b and b:IsShown() and b:GetID() == {slot} then return b:GetName() end \
             end return ''"
        ))
        .unwrap()
    }

    /// A left click on that button, through its own OnClick.
    fn click(s: &UiScript, bag: i64, slot: u32) {
        s.run(&format!(
            "this = {} ContainerFrameItemButton_OnClick('LeftButton')",
            button(s, bag, slot)
        ))
        .unwrap();
    }
}
