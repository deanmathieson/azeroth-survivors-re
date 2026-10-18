//! The authored item templates. WoW keeps item templates on the server, and survivors has none, so
//! each row here is what `SMSG_ITEM_QUERY_SINGLE_RESPONSE` would carry for a real 1.12 entry: its
//! name, quality, class and icon, with our own numbers (stack sizes, prices, effects).
//!
//! **What is verified.** Each icon ships in the client (`Interface\Icons\<icon>.blp`), and each
//! item's display id (held by the test until something draws the item in the world) is an
//! `ItemDisplayInfo.dbc` row with that icon: `tests::displays_and_icons_exist_in_the_client`,
//! data-gated. **Not verified:** the entries and names, and which of the rows sharing an icon the
//! cores' `item_template` names. The 1.12 client ships no item names and the repo has no
//! `Item.dbc` parser, so those are from memory and want a check against a core's `item_template`
//! (docs/PROFESSIONS.md, ground rules).

use benilla_ui::script::ItemTemplateView;

/// Copper Ore. Entry unverified (no `Item.dbc` parser in the repo).
pub(crate) const COPPER_ORE: u32 = 2770;
/// Tin Ore. Entry unverified.
pub(crate) const TIN_ORE: u32 = 2771;
/// Copper Bar. Entry unverified.
pub(crate) const COPPER_BAR: u32 = 2840;
/// Peacebloom. Entry unverified.
pub(crate) const PEACEBLOOM: u32 = 2447;
/// Silverleaf. Entry unverified.
pub(crate) const SILVERLEAF: u32 = 765;
/// Linen Cloth. Entry unverified.
pub(crate) const LINEN_CLOTH: u32 = 2589;
/// Minor Healing Potion. Entry unverified.
pub(crate) const MINOR_HEALING_POTION: u32 = 118;
/// Linen Bag. Entry unverified.
pub(crate) const LINEN_BAG: u32 = 4238;
/// Helm of Valor, a duplicate boss drop as a Soulbound bag item (section 3.4). Entry unverified;
/// display and icon are the gear catalog's (`gear/catalog.rs`, id 120).
pub(crate) const HELM_OF_VALOR: u32 = 16731;

/// One authored template.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ItemDef {
    pub(crate) entry: u32,
    pub(crate) name: &'static str,
    /// 0 Poor … 5 Legendary.
    pub(crate) quality: u32,
    /// `ItemClass.dbc`: 0 Consumable, 1 Container, 4 Armor, 7 Trade Goods.
    pub(crate) class: u32,
    pub(crate) subclass: u32,
    /// 0 none, 1 Head, 18 Bag.
    pub(crate) inventory_type: u32,
    /// Under `Interface\Icons\`.
    pub(crate) icon: &'static str,
    /// Most per slot.
    pub(crate) stack: u32,
    /// A bag's slots; 0 for anything else.
    pub(crate) bag_slots: u32,
    /// `BagFamily` bits: a specialty bag's, or the families an item may go into; 0 general.
    pub(crate) family: u32,
    /// Copper per item.
    pub(crate) sell_price: u32,
    /// 1 binds on pickup, 2 on equip.
    pub(crate) bonding: u32,
    /// The green `Use:` or `Equip:` line: `(trigger, text)`, trigger 0 use, 1 equip.
    pub(crate) effect: Option<(u32, &'static str)>,
    pub(crate) armor: u32,
}

const fn goods(
    entry: u32,
    name: &'static str,
    quality: u32,
    icon: &'static str,
    stack: u32,
    sell_price: u32,
) -> ItemDef {
    ItemDef {
        entry,
        name,
        quality,
        class: 7,
        subclass: 0,
        inventory_type: 0,
        icon,
        stack,
        bag_slots: 0,
        family: 0,
        sell_price,
        bonding: 0,
        effect: None,
        armor: 0,
    }
}

/// Every authored item. Stack sizes are 1.12's as remembered (ore 10, bars, herbs and cloth 20,
/// potions 5); prices and effects are ours.
pub(crate) static ITEMS: &[ItemDef] = &[
    goods(COPPER_ORE, "Copper Ore", 1, "INV_Ore_Copper_01", 10, 5),
    goods(TIN_ORE, "Tin Ore", 1, "INV_Ore_Tin_01", 10, 25),
    goods(COPPER_BAR, "Copper Bar", 1, "INV_Ingot_02", 20, 10),
    goods(PEACEBLOOM, "Peacebloom", 1, "INV_Misc_Flower_02", 20, 5),
    goods(SILVERLEAF, "Silverleaf", 1, "INV_Misc_Herb_10", 20, 5),
    goods(LINEN_CLOTH, "Linen Cloth", 1, "INV_Fabric_Linen_01", 20, 13),
    ItemDef {
        entry: MINOR_HEALING_POTION,
        name: "Minor Healing Potion",
        quality: 1,
        class: 0,
        subclass: 0,
        inventory_type: 0,
        icon: "INV_Potion_49",
        stack: 5,
        bag_slots: 0,
        family: 0,
        sell_price: 5,
        bonding: 0,
        effect: Some((0, "Restores 15% of your health.")),
        armor: 0,
    },
    ItemDef {
        entry: LINEN_BAG,
        name: "Linen Bag",
        quality: 1,
        class: 1,
        subclass: 0,
        inventory_type: 18,
        icon: "INV_Misc_Bag_10",
        stack: 1,
        bag_slots: 6,
        family: 0,
        sell_price: 100,
        bonding: 0,
        effect: None,
        armor: 0,
    },
    ItemDef {
        entry: HELM_OF_VALOR,
        name: "Helm of Valor",
        quality: 3,
        class: 4,
        subclass: 4,
        inventory_type: 1,
        icon: "INV_Helmet_02",
        stack: 1,
        bag_slots: 0,
        family: 0,
        sell_price: 9_870,
        bonding: 1,
        effect: Some((1, "Increases damage done by 3%.")),
        armor: 548,
    },
];

/// The P0 test inventory, `(entry, count)`: ore, a bar, herbs, cloth, potions, a spare bag and a
/// helm.
pub(crate) static TEST_INVENTORY: &[(u32, u32)] = &[
    (COPPER_ORE, 14),
    (TIN_ORE, 6),
    (COPPER_BAR, 8),
    (PEACEBLOOM, 9),
    (SILVERLEAF, 17),
    (LINEN_CLOTH, 20),
    (MINOR_HEALING_POTION, 4),
    (LINEN_BAG, 1),
    (HELM_OF_VALOR, 1),
];

pub(crate) fn template(entry: u32) -> Option<&'static ItemDef> {
    ITEMS.iter().find(|d| d.entry == entry)
}

/// `Interface\Icons\<icon>`, the path a slot and a template carry.
pub(crate) fn icon_path(def: &ItemDef) -> String {
    format!("Interface\\Icons\\{}", def.icon)
}

/// `ITEM_QUALITY_COLORS`' hex for `quality`, the colour a link wears.
fn quality_hex(quality: u32) -> &'static str {
    match quality {
        0 => "ff9d9d9d",
        1 => "ffffffff",
        2 => "ff1eff00",
        3 => "ff0070dd",
        4 => "ffa335ee",
        _ => "ffff8000",
    }
}

/// The 1.12 chat link, `|cff…|Hitem:<entry>:0:0:0|h[Name]|h|r`.
pub(crate) fn link(def: &ItemDef) -> String {
    format!(
        "|c{}|Hitem:{}:0:0:0|h[{}]|h|r",
        quality_hex(def.quality),
        def.entry,
        def.name
    )
}

/// `ItemClass.dbc`'s name for the classes authored here.
fn class_name(class: u32) -> &'static str {
    match class {
        0 => "Consumable",
        1 => "Container",
        4 => "Armor",
        _ => "Trade Goods",
    }
}

/// `ItemSubClass.dbc`'s display name for the subclasses authored here.
fn subclass_name(class: u32, subclass: u32) -> &'static str {
    match (class, subclass) {
        (1, _) => "Bag",
        (4, 4) => "Plate",
        (0, _) => "Consumable",
        _ => "Trade Goods",
    }
}

/// The tooltip view the stock `GameTooltip` renders: what the server's template would say.
pub(crate) fn template_view(def: &ItemDef) -> ItemTemplateView {
    ItemTemplateView {
        name: def.name.to_string(),
        quality: def.quality,
        class: def.class,
        subclass: def.subclass,
        inventory_type: def.inventory_type,
        item_type: Some(class_name(def.class).to_string()),
        item_sub_type: Some(subclass_name(def.class, def.subclass).to_string()),
        sub_class_display: Some(subclass_name(def.class, def.subclass).to_string()),
        // Trade goods, consumables and bags print no type cell.
        hide_subclass: def.class != 4,
        bonding: def.bonding,
        stackable: def.stack,
        container_slots: def.bag_slots,
        armor: def.armor,
        // Every class and race may use them: `<= 0` prints no line.
        allowable_class: -1,
        allowable_race: -1,
        spell_triggers: def
            .effect
            .map(|(trigger, text)| vec![(trigger, 0, text.to_string())])
            .unwrap_or_default(),
        sell_price: def.sell_price,
        icon: Some(icon_path(def)),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survivors_items_entries_are_unique_and_sane() {
        let mut seen = std::collections::HashSet::new();
        for d in ITEMS {
            assert!(seen.insert(d.entry), "{} listed twice", d.entry);
            assert!(d.stack >= 1, "{}: a stack holds one at least", d.name);
            assert_eq!(
                d.bag_slots > 0,
                d.class == 1,
                "{}: only bags have slots",
                d.name
            );
        }
        for &(entry, count) in TEST_INVENTORY {
            let d = template(entry).expect("a test item is authored");
            assert!(count >= 1 && d.stack >= 1);
        }
    }

    #[test]
    fn survivors_items_link_and_view_read_like_the_server() {
        let ore = template(COPPER_ORE).unwrap();
        assert_eq!(link(ore), "|cffffffff|Hitem:2770:0:0:0|h[Copper Ore]|h|r");
        let v = template_view(ore);
        assert_eq!(v.stackable, 10);
        assert_eq!(
            v.icon.as_deref(),
            Some("Interface\\Icons\\INV_Ore_Copper_01")
        );
        let bag = template_view(template(LINEN_BAG).unwrap());
        assert_eq!(bag.container_slots, 6);
        let potion = template_view(template(MINOR_HEALING_POTION).unwrap());
        assert_eq!(potion.spell_triggers[0].0, 0, "a Use: line");
        let helm = template(HELM_OF_VALOR).unwrap();
        assert!(link(helm).starts_with("|cff0070dd"), "a Rare link is blue");
    }

    /// Each item's display, `(entry, ItemDisplayInfo.dbc id)`: a row carrying the item's icon. The
    /// helm's is the gear catalog's (`gear/catalog.rs`, id 120).
    const DISPLAYS: &[(u32, u32)] = &[
        (COPPER_ORE, 4681),
        (TIN_ORE, 7340),
        (COPPER_BAR, 7391),
        (PEACEBLOOM, 7396),
        (SILVERLEAF, 18088),
        (LINEN_CLOTH, 7383),
        (MINOR_HEALING_POTION, 15710),
        (LINEN_BAG, 1183),
        (HELM_OF_VALOR, 31284),
    ];

    /// The looks are real: each display id is an `ItemDisplayInfo.dbc` row whose inventory icon is
    /// the item's, and each icon ships as `Interface\Icons\<name>.blp`. Every mismatch is listed.
    #[test]
    fn displays_and_icons_exist_in_the_client() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open the MPQ chain");
        let cat = benilla_formats::load_item_display_catalog(&mut chain).expect("ItemDisplayInfo");
        let mut bad = Vec::new();
        for d in ITEMS {
            let path = icon_path(d);
            let Some(&(_, display_id)) = DISPLAYS.iter().find(|(e, _)| *e == d.entry) else {
                bad.push(format!("{}: no display listed", d.name));
                continue;
            };
            match cat.get(display_id) {
                None => bad.push(format!("{}: no display {display_id}", d.name)),
                Some(row)
                    if !row
                        .icon
                        .as_deref()
                        .is_some_and(|c| c.eq_ignore_ascii_case(&path)) =>
                {
                    bad.push(format!(
                        "{}: display {display_id} shows {:?}",
                        d.name, row.icon
                    ));
                }
                Some(_) => {}
            }
            if !chain.contains(&format!("{path}.blp")) {
                bad.push(format!("{}: no {path}.blp", d.name));
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}
