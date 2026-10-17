//! Mounts: `UNIT_FIELD_MOUNTDISPLAYID` drawn as a second creature visual, a child entity with its
//! own `NetEntity` built by the ordinary attach path. The rider's rig roots under the mount's
//! attachment-0 seat (`0x60ce70`) and holds Mount (91), while the mount child walks on the host's
//! movement ([`crate::creature_anim`]).
//!
//! The change handler `0x5ffa50` tears the old seat down, then builds the new, and neither half
//! touches the rider's model. Mounting (`0x607a00`) sets the mount model (`0x613d80`, into
//! `CGUnit+0xdc`) and re-parents the body onto its attachment 0 (`0x712f70`), recomposed each frame
//! from that live bone (`0x718657`..`0x71876f`); dismounting (`0x607ce0`) detaches the body
//! (`0x713020`), destroys the mount model and stands the body, instantly. Here the rig's consumer
//! anchors move between the unit's frame and the seat, re-pointing [`RigPose::joints_root`], so
//! nothing that rides the rider is rebuilt.

use benilla_protocol::EntityKind;
use bevy::prelude::*;

use crate::net::{NetEntity, ObjectStore};
use benilla_world::rig_anim::RigPose;

/// A cosmetic hat requested for this mount child: its ItemDisplayInfo id, and a nudge from the
/// generic head seat ([`mount_hat_seat`]) in the mount model's Bevy space, for a head whose crown
/// sits off the common lift.
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct MountHat {
    pub(crate) display: u32,
    pub(crate) nudge: Vec3,
    /// Per-hat adjustment after its mount's head seat.
    pub(crate) offset: Vec3,
    pub(crate) pitch: f32,
    pub(crate) scale: f32,
}

/// The hat that the mount's ordinary equipment path has applied.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) struct AppliedMountHat(Option<u32>);

/// The unadjusted attach transform remembered at the root, so retrying an asynchronously built
/// hat does not stack its offset or pitch every frame.
#[derive(Component, Clone, Copy, PartialEq)]
pub(super) struct AppliedMountHatFit {
    base: Vec3,
    offset: Vec3,
    pitch: f32,
    scale: f32,
}

type MountHatQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static NetEntity,
        Option<&'static MountHat>,
        Option<&'static AppliedMountHat>,
        Option<&'static mut super::BoneAttach>,
    ),
    With<MountBody>,
>;

/// The mount child, the client's `unit+0xdc` second model.
#[derive(Component)]
pub(crate) struct MountBody {
    /// The mounted unit, whose movement the animation driver reads to walk the mount.
    pub(crate) host: Entity,
}

/// On the unit: its live mount child (the client's `[unit+0xdc]` handle).
#[derive(Component)]
pub(crate) struct MountChild(pub(crate) Entity);

/// On the unit: the mount display its rig is seated on (`0` standing), [`reseat_mounts`]' diff
/// key; it lags the field while a mount model loads.
#[derive(Component)]
pub(super) struct AppliedMount(pub(super) u32);

/// Spawn a unit's mount child (the client's `0x607a00` model creation), built as an ordinary
/// creature. `scale` is the `CreatureDisplayInfo.creatureModelScale` column alone (`0x607a75`), as
/// the unit's `SCALE_X` composes through the hierarchy. `fade_skip` drops the streaming fade, so a
/// cast mount is there at once: our choice, as whether the reference fades a new mount in is
/// untraced.
pub(super) fn spawn_mount_child(
    commands: &mut Commands,
    host: Entity,
    display: u32,
    scale: f32,
    fade_skip: bool,
    // The rider's arrival, so the mount rides the rider's appear ramp.
    arrival: Option<super::Arrival>,
) -> Entity {
    let child = commands
        .spawn((
            NetEntity {
                kind: EntityKind::Unit,
                display_id: Some(display),
                scale,
            },
            MountBody { host },
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    if let Some(arrival) = arrival {
        commands.entity(child).insert(arrival);
    }
    if fade_skip {
        commands.entity(child).insert(super::equipment::Reattached);
    }
    commands.entity(host).add_child(child);
    commands.entity(host).insert(MountChild(child));
    child
}

/// The seat anchor, the rider's model frame while mounted: under the mount's attachment-0 joint,
/// counter-scaled so the rider keeps its size (`0x607b49`, `0x710620`), and a
/// [`benilla_world::rig_anim::RigFrame`] so a re-seat cascades into the rider's palette.
fn spawn_seat_anchor(
    commands: &mut Commands,
    joint: Entity,
    offset: Vec3,
    mount_scale: f32,
    rider: Entity,
) -> Entity {
    let anchor = commands
        .spawn((
            Transform::from_translation(offset)
                .with_scale(Vec3::splat(1.0 / mount_scale.max(0.001))),
            Visibility::default(),
            benilla_world::rig_anim::RigFrame(rider),
        ))
        .id();
    commands.entity(joint).add_child(anchor);
    anchor
}

/// Where the seat leg leaves the rider: [`Seat::Wait`] while the mount model builds,
/// [`Seat::Frame`] to root there, or [`Seat::UnitMatrix`] when the mount authors no attachment 0
/// and the reference leaves the body at the unit matrix (`0x60ce70`'s miss).
pub(super) enum Seat {
    Wait,
    Frame(Entity),
    UnitMatrix,
}

/// The mount children's build state: attached yet, its anchors, and the display it was built
/// with, which the field can move off while the model loads.
pub(super) type MountChildren<'w, 's> = Query<
    'w,
    's,
    (
        Has<super::VisualAttached>,
        Option<&'static super::BoneAttach>,
        &'static NetEntity,
    ),
    With<MountBody>,
>;

/// The seat leg of the first build and the transition: spawn the mount child when missing, drop
/// one built for a display the field has left, and resolve the seat once its rig is up.
pub(super) fn seat_or_spawn_mount(
    commands: &mut Commands,
    children: &MountChildren,
    poses: &mut Query<&mut RigPose>,
    creatures: Option<&super::Creatures>,
    unit: Entity,
    child: Option<Entity>,
    mount_display: u32,
    fade_skip: bool,
    arrival: Option<super::Arrival>,
) -> Seat {
    // `0x607a75`: `SCALE_X` × `creatureModelScale` alone, without `CreatureModelData.modelScale`.
    let mount_scale = creatures
        .and_then(|c| c.catalog.display_scale(mount_display))
        .unwrap_or(1.0);
    let Some(child) = child else {
        spawn_mount_child(
            commands,
            unit,
            mount_display,
            mount_scale,
            fade_skip,
            arrival,
        );
        return Seat::Wait;
    };
    // Built for a display the field has left, or gone: drop it and start over next pass.
    let built = children.get(child).map(|(_, _, n)| n.display_id).ok();
    if built != Some(Some(mount_display)) {
        if let Ok(mut ec) = commands.get_entity(child) {
            ec.despawn();
        }
        commands.entity(unit).remove::<MountChild>();
        return Seat::Wait;
    }
    let Ok((true, Some(bones), _)) = children.get(child) else {
        return Seat::Wait; // the mount child is still building
    };
    // The mount's attachment-0 joint, made on demand in the mount's own `RigPose.anchors`, where
    // the world pass's patch walk finds the seat and cascades the rider.
    let Some((joint, offset)) = bones.points.get(&0).and_then(|&(bone, offset)| {
        poses
            .get_mut(child)
            .ok()
            .and_then(|mut p| p.anchor_for(commands, child, bone))
            .map(|j| (j, offset))
    }) else {
        warn!(
            "MOUNTDISPLAYIDNOMOUNTATTACHMENT: display {mount_display} authors no attachment 0 — \
             rider stays at the unit matrix"
        );
        return Seat::UnitMatrix;
    };
    Seat::Frame(spawn_seat_anchor(
        commands,
        joint,
        offset,
        mount_scale,
        unit,
    ))
}

/// The rider's ground frame: its own entity, or a fresh conform node under it for a model the
/// reference tilts to the terrain (`GlobalModelFlags & 3` of 1 or 3). Mounted, the composite
/// tilts through the mount's node instead.
fn ground_frame(
    commands: &mut Commands,
    unit: Entity,
    net: &NetEntity,
    creatures: Option<&super::Creatures>,
) -> Entity {
    let tilt = net
        .display_id
        .and_then(|d| creatures?.models.get(&d))
        .map_or(0, |dm| dm.terrain_tilt);
    if tilt == 0 {
        return unit;
    }
    let node = commands
        .spawn((
            super::conform::ConformNode { unit, mode: tilt },
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(unit).add_child(node);
    node
}

/// Move a rig's consumer anchors onto `frame` and make it the model frame, as `0x712f70` attaches
/// and `0x713020` detaches; the anchors carry every held item, effect and overhead rider.
/// `pose_dirty` re-seats them and the palette in the same frame's compose.
fn reseat_rig(commands: &mut Commands, rig: &mut RigPose, frame: Entity, conform: bool) {
    if rig.joints_root == frame {
        return;
    }
    let anchors: Vec<Entity> = rig.anchors.iter().map(|&(_, a)| a).collect();
    if !anchors.is_empty() {
        commands.entity(frame).add_children(&anchors);
    }
    let old = std::mem::replace(&mut rig.joints_root, frame);
    rig.pose_dirty = true;
    // A conform node the rig just left is empty, so it goes; a seat anchor dies with its mount.
    if conform {
        if let Ok(mut ec) = commands.get_entity(old) {
            ec.despawn();
        }
    }
}

/// Re-seat a unit's rig when its mount field changes, the client's `0x5ffa50`: tear the old seat
/// down, then build the new, with the rider's own visual untouched. The legs run in separate
/// frames, so a remount stands the rider on its feet while the new mount model loads.
#[allow(clippy::type_complexity)]
pub(super) fn reseat_mounts(
    mut commands: Commands,
    units: Query<
        (
            Entity,
            &NetEntity,
            &ObjectStore,
            Option<&AppliedMount>,
            Option<&MountChild>,
            Option<&super::Arrival>,
        ),
        // A torn-down unit is freed, not re-seated (`0x464920`).
        (
            With<super::VisualAttached>,
            Without<benilla_world::model_fade::DespawnFade>,
        ),
    >,
    // One pose query for both sides, the rider's rig and the mount's, borrowed in sequence below.
    mut poses: Query<&mut RigPose>,
    children: MountChildren,
    conforms: Query<(), With<super::conform::ConformNode>>,
    creatures: Option<Res<super::Creatures>>,
) {
    for (entity, net, store, applied, child, arrival) in &units {
        if !matches!(net.kind, EntityKind::Unit | EntityKind::Player) {
            continue;
        }
        let live = store.0.unit_mount_display_id();
        let applied = applied.map_or(0, |a| a.0);
        // Settled: seated on what the field says, and holding no mount model when it says none.
        // A mount-up whose field zeroes before the model loads leaves an unseated child (`applied`
        // never left 0), which would otherwise stand under the dismounted rider for good.
        if live == applied && (live != 0 || child.is_none()) {
            continue;
        }
        // A model-less unit (the cube) has no rig to seat: stamp the field so it never churns, and
        // sweep any child, which the settled test would otherwise re-fire on every frame.
        let Ok(seated_root) = poses.get(entity).map(|r| r.joints_root) else {
            if let Some(&MountChild(child)) = child {
                if let Ok(mut ec) = commands.get_entity(child) {
                    ec.despawn();
                }
                commands.entity(entity).remove::<MountChild>();
            }
            commands.entity(entity).insert(AppliedMount(live));
            continue;
        };
        let child = child.map(|&MountChild(c)| c);
        // ── Leg 1 · `0x607ce0`: detach the body onto its own frame, destroy the mount model.
        // Taken when seated on a mount the field left or when the field says none; a unit waiting
        // on its first mount model must not take it, or it would tear down its own order.
        if applied != 0 || live == 0 {
            // Only a body on a seat detaches: a rig on its own entity or its conform node stays,
            // or a second conform node would be orphaned beside the first.
            if seated_root != entity && !conforms.contains(seated_root) {
                let ground = ground_frame(&mut commands, entity, net, creatures.as_deref());
                if let Ok(mut rig) = poses.get_mut(entity) {
                    reseat_rig(&mut commands, &mut rig, ground, false);
                }
            }
            if let Some(child) = child {
                if let Ok(mut ec) = commands.get_entity(child) {
                    ec.despawn();
                }
            }
            commands
                .entity(entity)
                .remove::<MountChild>()
                .insert(AppliedMount(0));
            continue;
        }
        // ── Leg 2 · `0x607a00`: build the mount model and attach the body to its seat.
        match seat_or_spawn_mount(
            &mut commands,
            &children,
            &mut poses,
            creatures.as_deref(),
            entity,
            child,
            live,
            true,
            arrival.copied(),
        ) {
            Seat::Wait => {}
            Seat::Frame(anchor) => {
                if let Ok(mut rig) = poses.get_mut(entity) {
                    let leaving_conform = conforms.contains(seated_root);
                    reseat_rig(&mut commands, &mut rig, anchor, leaving_conform);
                }
                commands.entity(entity).insert(AppliedMount(live));
            }
            Seat::UnitMatrix => {
                commands.entity(entity).insert(AppliedMount(live));
            }
        }
    }
}

/// A mount hat's seat above the head bone's pivot, in the mount model's own units (Bevy space:
/// up, and a little forward, -Z): the pivot is the poll, the joint behind the skull, and a helm's
/// origin, the human helm point, sits near its crown with the brim hanging below it.
const HAT_LIFT: Vec3 = Vec3::new(0.0, 0.5, -0.05);

/// From the Breath point (17), on the muzzle, back up under the crown, for a model with no Head
/// key bone; [`HAT_LIFT`] goes on top. Of the catalog mounts only the Stormpike Battle Charger, a
/// ram whose muzzle hangs well below its poll, takes this path; tuned on it in game.
const BREATH_TO_POLL: Vec3 = Vec3::new(0.0, 0.9, 0.1);

/// A mount hat's size against the human head it was modelled for. A riding model is drawn at its
/// mount scale, so its head runs two to three helm widths: a little oversized reads as a hat on
/// purpose, not a stray prop.
pub(crate) const HAT_SCALE: f32 = 2.0;

/// Where a mount hat sits, `(bone, offset from its pivot)` in the model's Bevy space: an authored
/// helm point (11) when a model has one, which no 1.12 riding model does; else just above the
/// Head key bone (6); else above the Breath point (17), on the muzzle's bone; else the Head
/// attachment (20). `None` when a model has none of them.
pub(crate) fn mount_hat_seat(
    head_bone: Option<u16>,
    attachment: impl Fn(u16) -> Option<(u16, Vec3)>,
) -> Option<(u16, Vec3)> {
    const HELM: u16 = super::equipment::attach_id::HELM;
    const BREATH: u16 = 17;
    const HEAD: u16 = 20;
    attachment(HELM)
        .or_else(|| head_bone.map(|bone| (bone, HAT_LIFT)))
        .or_else(|| {
            attachment(BREATH).map(|(bone, offset)| (bone, offset + BREATH_TO_POLL + HAT_LIFT))
        })
        .or_else(|| attachment(HEAD))
}

/// Feed a mount hat through the normal held-item attachment path, seated at
/// [`super::equipment::attach_id::MOUNT_HAT`], which this adds to the mount's [`super::BoneAttach`]
/// once its model is built. A mount with no seat stays bare.
pub(super) fn resolve_mount_hats(
    mut commands: Commands,
    mut mounts: MountHatQuery,
    displays: Option<ResMut<super::equipment::ItemDisplays>>,
    creatures: Option<Res<super::Creatures>>,
    asset_server: Res<AssetServer>,
) {
    let Some(mut displays) = displays else {
        return;
    };
    for (entity, net, hat, applied, bones) in &mut mounts {
        let want = hat.map(|hat| hat.display);
        // The seat first, re-added whenever the mount's attachment table is rebuilt without it:
        // the attach path latches a slot whose point is missing, so the hat waits for its seat.
        if want.is_some() {
            let Some(mut bones) = bones else {
                continue; // the mount model is still building
            };
            // Read before any write: a `Mut` borrowed for `entry` would mark the table changed
            // on every frame.
            let seated = bones
                .points
                .contains_key(&super::equipment::attach_id::MOUNT_HAT);
            if !seated {
                let seat = net
                    .display_id
                    .and_then(|d| creatures.as_deref()?.models.get(&d))
                    .and_then(|dm| {
                        mount_hat_seat(dm.skeleton.head_bone, |id| {
                            dm.attachments
                                .iter()
                                .find(|a| a.id == id)
                                .map(|a| (a.bone, a.offset))
                        })
                    });
                let Some((bone, offset)) = seat else {
                    continue; // no head to wear it: the mount stays bare
                };
                let seat = (bone, offset + hat.map_or(Vec3::ZERO, |hat| hat.nudge));
                debug!(
                    "mount hat seat: mount {entity} display {:?} → bone {} offset {}",
                    net.display_id, seat.0, seat.1
                );
                bones
                    .points
                    .insert(super::equipment::attach_id::MOUNT_HAT, seat);
            }
        }
        if applied.is_some_and(|applied| applied.0 == want) {
            continue;
        }
        if let Some(display) = want {
            super::equipment::ensure_item_model(
                &mut displays,
                display,
                super::equipment::ItemModelKind::Helm { race: 1, sex: 0 },
                &asset_server,
            );
        }
        commands.entity(entity).insert((
            super::equipment::HeldItems::mount_hat(want),
            AppliedMountHat(want),
        ));
    }
}

/// Size each attached mount hat by [`HAT_SCALE`]: its root's transform for a static mesh, and the
/// rider placement for a skinned one, which composes from the mount's rig, not the root.
pub(super) fn size_mount_hats(
    mut commands: Commands,
    mounts: Query<(&super::equipment::HeldAttached, &MountHat), With<MountBody>>,
    mut roots: Query<(
        Entity,
        &mut Transform,
        Option<&benilla_world::rig_rider::RiderScale>,
        Option<&AppliedMountHatFit>,
    )>,
) {
    for (attached, hat) in &mounts {
        let Some(root) = attached.spawned_slots()[super::equipment::HAT_SLOT] else {
            continue;
        };
        let Ok((root, mut tf, scale, applied)) = roots.get_mut(root) else {
            continue;
        };
        let want = AppliedMountHatFit {
            base: applied.map_or(tf.translation, |fit| fit.base),
            offset: hat.offset,
            pitch: hat.pitch,
            scale: hat.scale,
        };
        let visual_scale = HAT_SCALE * want.scale;
        if applied != Some(&want) {
            tf.translation = want.base + want.offset;
            tf.rotation = Quat::from_rotation_x(want.pitch);
            tf.scale = Vec3::splat(visual_scale);
            commands.entity(root).insert(want);
        }
        let rider_scale = benilla_world::rig_rider::RiderScale(visual_scale);
        if scale != Some(&rider_scale) {
            commands.entity(root).try_insert(rider_scale);
        }
    }
}

#[cfg(test)]
mod tests;
