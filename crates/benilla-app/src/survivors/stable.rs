//! Stable collection and the small bridge from its saved cosmetic pick to a unit mount field.

use bevy::prelude::*;

use super::armory::ArmoryState;
use super::bonus::{Bonus, Bonuses, StatKey};
use super::faire::{catalog, Kind, Prize};
use super::hero;
use super::meta::MetaParts;
use super::profile::Profile;
use super::LocalHero;
use crate::glue::GlueClicks;
use crate::net::ObjectStore;

/// A Stable tile click. `None` is the explicit no-hat tile.
#[derive(Component, Clone, Debug)]
pub(in crate::survivors) enum StableAction {
    Mount(String),
    Hat(Option<String>),
}

/// CreatureDisplayInfo rows for the 1.12 riding models. The prize names deliberately remain the
/// save keys: several rewards (Faire and quest chains) unlock the same entry.
pub(in crate::survivors) fn mount_display(key: &str) -> Option<u32> {
    Some(match key {
        "brown_horse" => 2404,
        "chestnut_mare" => 2405,
        "pinto" => 2409,
        "gray_kodo" => 12246,
        "brown_ram" => 2785,
        "striped_frostsaber" => 6444,
        "red_mechanostrider" => 9473,
        "red_skeletal_horse" => 10670,
        "horn_of_the_dire_wolf" => 247,
        "horn_of_the_frostwolf_howler" => 14776,
        "stormpike_battle_charger" => 14777,
        "black_war_tiger" => 14330,
        "swift_palomino" => 2408,
        "swift_white_mechanostrider" => 10666,
        "reins_of_the_winterspring_frostsaber" => 6080,
        "swift_stormsaber" => 14632,
        "horn_of_the_black_war_wolf" => 14334,
        "deathcharger_s_reins" => 10718,
        "swift_razzashi_raptor" => 15289,
        "swift_zulian_tiger" => 15290,
        _ => return None,
    })
}

/// Where a mount's head wants its hat off the engine's common seat, in the riding model's own
/// units (Bevy space, +Y up, -Z forward), tuned in game: the kodo's crown stands far above and
/// ahead of its head bone, the ram's poll is already the crown.
fn hat_nudge(key: Option<&str>) -> Vec3 {
    match key {
        Some("gray_kodo") => Vec3::new(0.0, 0.45, -0.3),
        Some("brown_ram") => Vec3::new(0.0, -0.25, 0.0),
        _ => Vec3::ZERO,
    }
}

/// The hat's real `ItemDisplayInfo` model, resolved through its client icon.
fn hat_display(key: &str, displays: &crate::entities::ItemDisplays) -> Option<u32> {
    prizes(Kind::MountHat)
        .find(|prize| prize.key == key)
        .and_then(|prize| displays.display_for_icon(prize.icon))
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (clicks, apply_mount_bonus, apply_mount_field).chain(),
    );
}

fn clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &StableAction)>,
    mut profile: ResMut<Profile>,
    mut armory: ResMut<ArmoryState>,
) {
    if !armory.open {
        return;
    }
    let Some(action) = actions.iter().find(|(e, _)| clicks.hit(*e)).map(|(_, a)| a) else {
        return;
    };
    match action {
        StableAction::Mount(key) if profile.quests.mounts.contains(key) => {
            profile.stable.mount =
                (profile.stable.mount.as_deref() != Some(key)).then(|| key.clone());
        }
        StableAction::Hat(key)
            if key
                .as_ref()
                .is_none_or(|k| profile.quests.mount_hats.contains(k)) =>
        {
            profile.stable.mount_hat = key.clone();
        }
        _ => return,
    }
    profile.mark_dirty();
    armory.refresh();
}

/// A selected, owned mount is a persistent meta source. Clearing a stale selection keeps old
/// saves harmless and prevents an unowned key from granting speed.
fn apply_mount_bonus(profile: Res<Profile>, mut parts: ResMut<MetaParts>) {
    let mut want = Bonuses::default();
    if profile
        .stable
        .mount
        .as_deref()
        .is_some_and(|key| profile.quests.mounts.contains(key) && mount_display(key).is_some())
    {
        want.add(&Bonus::new(StatKey::MoveSpeed, 0.10));
    }
    if parts.mount != want {
        parts.mount = want;
    }
}

/// Keep the currently shown local hero mounted in both the menu preview and a run. The engine
/// detects this ordinary update field and builds/reseats its mount child itself.
fn apply_mount_field(
    mut commands: Commands,
    profile: Res<Profile>,
    displays: Option<Res<crate::entities::ItemDisplays>>,
    mut heroes: Query<
        (
            &mut ObjectStore,
            Option<&crate::entities::mount::MountChild>,
        ),
        With<LocalHero>,
    >,
) {
    let display = profile
        .stable
        .mount
        .as_deref()
        .filter(|key| profile.quests.mounts.contains(*key))
        .and_then(mount_display)
        .unwrap_or(0);
    let hat = profile
        .stable
        .mount_hat
        .as_deref()
        .filter(|key| profile.quests.mount_hats.contains(*key))
        .and_then(|key| {
            displays
                .as_deref()
                .and_then(|displays| hat_display(key, displays))
        });
    for (mut store, mount) in &mut heroes {
        if store.0.unit_mount_display_id() != display {
            hero::set_mount(&mut store, display);
        }
        if let Some(mount) = mount {
            let mut child = commands.entity(mount.0);
            if let Some(hat) = hat {
                child.insert(crate::entities::mount::MountHat {
                    display: hat,
                    nudge: hat_nudge(profile.stable.mount.as_deref()),
                });
            } else {
                child.remove::<crate::entities::mount::MountHat>();
            }
        }
    }
}

pub(in crate::survivors) fn prizes(kind: Kind) -> impl Iterator<Item = &'static Prize> {
    catalog().iter().filter(move |p| p.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_round_trips_and_mount_bonus_needs_an_owned_mount() {
        let mut profile = Profile::default();
        profile.stable.mount = Some("brown_horse".into());
        let text = toml::to_string(&profile).unwrap();
        assert_eq!(
            toml::from_str::<Profile>(&text).unwrap().stable,
            profile.stable
        );

        let mut app = App::new();
        app.insert_resource(profile)
            .init_resource::<MetaParts>()
            .add_systems(Update, apply_mount_bonus);
        app.update();
        assert_eq!(
            app.world()
                .resource::<MetaParts>()
                .mount
                .get(StatKey::MoveSpeed),
            0.0
        );
        app.world_mut()
            .resource_mut::<Profile>()
            .quests
            .mounts
            .insert("brown_horse".into());
        app.update();
        assert!(
            (app.world()
                .resource::<MetaParts>()
                .mount
                .get(StatKey::MoveSpeed)
                - 0.1)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn local_hero_receives_the_selected_mount_field() {
        let mut profile = Profile::default();
        profile.quests.mounts.insert("brown_horse".into());
        profile.stable.mount = Some("brown_horse".into());
        let mut app = App::new();
        let hero = app
            .world_mut()
            .spawn((LocalHero, ObjectStore::default()))
            .id();
        app.insert_resource(profile)
            .add_systems(Update, apply_mount_field);
        app.update();
        assert_eq!(
            app.world()
                .entity(hero)
                .get::<ObjectStore>()
                .unwrap()
                .0
                .unit_mount_display_id(),
            2404
        );
    }

    #[test]
    fn catalog_mounts_and_hats_resolve_to_client_models() {
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let items = benilla_formats::load_item_display_catalog(&mut chain).expect("items");
        for prize in prizes(Kind::Mount) {
            let display = mount_display(&prize.key).expect("catalog mount has a display");
            assert!(
                creatures
                    .model(display)
                    .is_some_and(|model| !model.model_path.is_empty()),
                "{} ({display}) has no creature model",
                prize.name
            );
        }
        let mut missing = Vec::new();
        for prize in prizes(Kind::MountHat) {
            let icon = format!("Interface\\Icons\\{}", prize.icon);
            if !items.iter().any(|(_, display)| {
                display.icon.as_deref() == Some(&icon) && display.model[0].is_some()
            }) {
                missing.push(format!("{} ({icon})", prize.name));
            }
        }
        assert!(missing.is_empty(), "no ItemDisplayInfo model: {missing:?}");
    }

    /// Every catalog mount wears a hat: each hat's human helm file is in the client, and each
    /// riding model, none of which authors the helm point, resolves a head seat forward of its
    /// saddle, where the hat hangs at a real size.
    #[test]
    fn every_catalog_mount_seats_a_hat() {
        use crate::entities::mount::{mount_hat_seat, HAT_SCALE};
        use benilla_assets::coords::wow_to_bevy;

        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).expect("open chain");
        let creatures = benilla_formats::load_creature_catalog(&mut chain).expect("creatures");
        let items = benilla_formats::load_item_display_catalog(&mut chain).expect("items");
        let displays = crate::entities::ItemDisplays::icons_for_tests(items);
        for hat in prizes(Kind::MountHat) {
            let display = hat_display(&hat.key, &displays).expect("hat display");
            let model = displays
                .catalog
                .get(display)
                .and_then(|d| d.model[0].clone());
            let file = format!(
                "Item\\ObjectComponents\\Head\\{}",
                crate::entities::helm_file(&model.expect("helm model"), 1, 0)
            );
            assert!(chain.contains(&file), "{}: no {file}", hat.name);
        }
        for prize in prizes(Kind::Mount) {
            let display = mount_display(&prize.key).expect("catalog mount has a display");
            let model = creatures.model(display).expect("creature model");
            let path = benilla_assets::m2_url(&model.model_path).replace("mpq://", "");
            let bytes = chain.read_file(&path).expect("riding model");
            let skeleton = benilla_formats::parse_m2_skeleton(&bytes).expect("skeleton");
            let attachments = benilla_formats::parse_m2_attachments(&bytes).expect("attachments");
            let pivots: Vec<Vec3> = skeleton
                .bones
                .iter()
                .map(|b| wow_to_bevy(b.pivot))
                .collect();
            let head = skeleton
                .bones
                .iter()
                .position(|b| b.key_bone == 6)
                .map(|b| b as u16);
            let point = |id: u16| {
                attachments.iter().find(|a| a.id == id).map(|a| {
                    let pivot = pivots[a.bone as usize];
                    (a.bone, wow_to_bevy(a.position) - pivot)
                })
            };
            let (bone, offset) = mount_hat_seat(head, point)
                .unwrap_or_else(|| panic!("{} ({path}) has no hat seat", prize.name));
            let offset = offset + hat_nudge(Some(&prize.key));
            let hat = Transform::from_translation(offset).with_scale(Vec3::splat(HAT_SCALE));
            let at = pivots[bone as usize] + hat.translation;
            let (saddle_bone, saddle) = point(0).expect("a saddle");
            let saddle = pivots[saddle_bone as usize] + saddle;
            assert!(
                at.is_finite() && at != Vec3::ZERO && hat.scale.min_element() > 0.0,
                "{}: hat at {at:?}",
                prize.name
            );
            // Bevy's forward is -Z: the head leads the saddle.
            assert!(
                at.z < saddle.z,
                "{} ({path}): hat {at:?} is not ahead of the saddle {saddle:?}",
                prize.name
            );
        }
    }
}
