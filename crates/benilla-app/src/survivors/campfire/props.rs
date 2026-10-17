//! The camp in the world: the campfire, the goblin merchant, the fishing spot and its bobber.
//!
//! Every machine builds its own from the camp's replicated anchor ([`Camp::fire`]), so the props
//! never travel (the host keeps [`CampProp`]s out of replication) and the bobber can dip smoothly
//! on each screen from the shared [`Camp::fishing_dip`]. A click on the bobber strikes; a click
//! on the goblin opens his merchant window ([`super::ui`]).

use benilla_world::world_point::WorldPoint;
use bevy::prelude::*;

use super::super::hero::AimPoint;
use super::super::party::LocalHero;
use super::super::units::{self, flat, yaw_toward, UnitSpec};
use super::super::{Phase, Run};
use super::ui::MerchantWindow;
use super::{fish_caught, Camp, CampAsk, CampNote, CampRequest, BITE_AT, BITE_SECS, FISH_BITE};
use crate::sound::SoundCue;

/// `GameObjectDisplayInfo` rows: Elwynn's campfire, the fishing bobber and a school of fish.
pub(in crate::survivors) const CAMPFIRE_DISPLAY: u32 = 192;
pub(in crate::survivors) const BOBBER_DISPLAY: u32 = 668;
pub(in crate::survivors) const FISH_SCHOOL_DISPLAY: u32 = 6291;
/// A goblin NPC's `CreatureDisplayInfo` row, a dressed `GoblinMale.mdx` (Short John Mithril's).
pub(in crate::survivors) const GOBLIN_DISPLAY: u32 = 14561;
/// The goblin greets whoever opens his window.
pub(super) const GOBLIN_GREETING: &str = "GoblinMaleGruffVendorNPCGreetin";
/// Where the merchant and the fishing spot stand from the fire, in yards.
const VENDOR_OFFSET: Vec3 = Vec3::new(4.5, 0.0, 2.5);
const FISHING_OFFSET: Vec3 = Vec3::new(-5.0, 0.0, 3.5);
/// The bobber is drawn larger than life, to be seen and clicked from the run's camera.
const BOBBER_SCALE: f32 = 2.5;
/// How far below its float the bobber sinks while a fish bites, in yards.
const DIP_DEPTH: f32 = 0.35;
/// How near the cursor's ground point must come to the bobber or the goblin for a click, in yards.
const CLICK_REACH: f32 = 2.0;
/// The props' serials: their own, far above the run's, the same on every machine.
const SERIAL: u64 = 0x00CA_4F00_0000;

/// A camp object, built on each machine and never replicated.
#[derive(Component)]
pub(in crate::survivors) struct CampProp;

/// The bobber, and the water line it floats at.
#[derive(Component)]
struct Bobber {
    float: Vec3,
}

/// Where this camp's merchant and bobber stand, on the ground.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct CampSpots {
    pub(super) vendor: Option<Vec3>,
    pub(super) bobber: Option<Vec3>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<CampSpots>()
        .add_systems(Update, (place_props, bob, clicks).chain());
}

/// The merchant's and the fishing spot's places round the fire at `fire`.
pub(super) fn layout(fire: Vec3) -> (Vec3, Vec3) {
    (fire + VENDOR_OFFSET, fire + FISHING_OFFSET)
}

/// How far below its float the bobber hangs at `dip` seconds into its cycle: a gentle bob, and a
/// plunge while a fish bites.
pub(super) fn sink(dip: f32) -> f32 {
    if fish_caught(dip) {
        let t = (dip.rem_euclid(super::FISH_CYCLE) - BITE_AT) / BITE_SECS;
        DIP_DEPTH * (0.7 + 0.3 * (t * std::f32::consts::TAU * 3.0).sin().abs())
    } else {
        0.04 * (dip * std::f32::consts::TAU * 0.75).sin()
    }
}

/// Whether the cursor's ground point `aim` picks the thing at `at`.
pub(super) fn picks(aim: Option<Vec3>, at: Option<Vec3>) -> bool {
    matches!((aim, at), (Some(a), Some(b)) if flat(a - b).length() <= CLICK_REACH)
}

/// The fire, the goblin, the fish and the bobber up for a camp, gone with it.
fn place_props(
    mut commands: Commands,
    run: Res<Run>,
    camp: Res<Camp>,
    world: WorldPoint,
    props: Query<Entity, With<CampProp>>,
    mut spots: ResMut<CampSpots>,
    mut built: Local<Option<Vec3>>,
) {
    let want = camp.fire.filter(|_| camp.active(&run));
    if *built == want {
        return;
    }
    for e in &props {
        commands.entity(e).despawn();
    }
    *built = want;
    *spots = CampSpots::default();
    let Some(fire) = want else { return };
    let ground = |p: Vec3| super::super::enemies::ground(&world, p, fire.y);
    let (vendor, fishing) = layout(fire);
    let (fire, vendor, fishing) = (ground(fire), ground(vendor), ground(fishing));
    let flame = units::spawn_object(&mut commands, SERIAL + 1, CAMPFIRE_DISPLAY, 1.0, fire);
    commands.entity(flame).insert(CampProp);
    let goblin = units::spawn_unit(
        &mut commands,
        &UnitSpec {
            serial: SERIAL + 2,
            display: GOBLIN_DISPLAY,
            scale: 1.0,
            level: 60,
            faction: units::FACTION_FRIENDLY,
            bytes0: 1 << 8,
            weapons: &[],
            sheath: 0,
        },
        vendor,
        yaw_toward(fire - vendor),
    );
    commands.entity(goblin).insert(CampProp);
    let school = units::spawn_object(&mut commands, SERIAL + 3, FISH_SCHOOL_DISPLAY, 1.0, fishing);
    commands.entity(school).insert(CampProp);
    let bobber = units::spawn_object(
        &mut commands,
        SERIAL + 4,
        BOBBER_DISPLAY,
        BOBBER_SCALE,
        fishing,
    );
    commands
        .entity(bobber)
        .insert((CampProp, Bobber { float: fishing }));
    *spots = CampSpots {
        vendor: Some(vendor),
        bobber: Some(fishing),
    };
}

/// The bobber rides the water, and plunges with a splash when a fish bites.
fn bob(
    camp: Res<Camp>,
    mut bobbers: Query<(&Bobber, &mut Transform)>,
    mut under: Local<bool>,
    mut cues: MessageWriter<SoundCue>,
) {
    for (bobber, mut tf) in &mut bobbers {
        tf.translation = bobber.float - Vec3::Y * sink(camp.fishing_dip);
        let now = fish_caught(camp.fishing_dip);
        if now && !*under {
            cues.write(SoundCue {
                kit: FISH_BITE,
                at: Some(bobber.float),
                volume: 1.0,
            });
        }
        *under = now;
    }
}

/// A click in the world during camp: on the bobber it strikes (told at once if the fish is not
/// biting, so a client never waits on the host to hear it), on the goblin it opens his window.
#[allow(clippy::too_many_arguments)]
fn clicks(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    aim: Res<AimPoint>,
    phase: Res<State<Phase>>,
    run: Res<Run>,
    camp: Res<Camp>,
    spots: Res<CampSpots>,
    ui: Query<&Interaction>,
    local: Query<(), With<LocalHero>>,
    mut window: ResMut<MerchantWindow>,
    mut note: ResMut<CampNote>,
    mut out: MessageWriter<CampRequest>,
    mut cues: MessageWriter<SoundCue>,
) {
    let Some(mouse) = mouse else { return };
    if !camp.active(&run)
        || *phase.get() != Phase::Playing
        || local.is_empty()
        || !mouse.any_just_pressed([MouseButton::Left, MouseButton::Right])
        || ui.iter().any(|i| *i != Interaction::None)
    {
        return;
    }
    if picks(aim.0, spots.bobber) {
        let me = super::super::party::LOCAL_PLAYER;
        if camp.fished.contains(&me) {
            note.0 = Some(("You have already caught your fish this camp.".into(), 3.0));
        } else if !fish_caught(camp.fishing_dip) {
            note.0 = Some(("The fish got away.".into(), 3.0));
        } else {
            out.write(CampRequest(CampAsk::Fish));
        }
    } else if picks(aim.0, spots.vendor) && !window.open {
        window.open = true;
        cues.write(SoundCue {
            kit: GOBLIN_GREETING,
            at: spots.vendor,
            volume: 1.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::FISH_CYCLE;
    use super::*;

    #[test]
    fn the_bobber_plunges_only_while_a_fish_bites() {
        for i in 0..400 {
            let dip = i as f32 * FISH_CYCLE / 400.0;
            let depth = sink(dip);
            if fish_caught(dip) {
                assert!(depth >= DIP_DEPTH * 0.69, "{dip}: {depth}");
            } else {
                assert!(depth.abs() <= 0.05, "{dip}: {depth}");
            }
        }
    }

    #[test]
    fn the_camp_is_laid_out_round_the_fire() {
        let fire = Vec3::new(100.0, 5.0, -40.0);
        let (vendor, fishing) = layout(fire);
        for spot in [vendor, fishing] {
            let d = flat(spot - fire).length();
            assert!((3.0..super::super::RESTED_RANGE).contains(&d), "{d}");
        }
        assert!(flat(vendor - fishing).length() > 2.0 * CLICK_REACH);
    }

    #[test]
    fn a_click_picks_what_lies_under_the_cursor() {
        let at = Some(Vec3::new(3.0, 0.0, 3.0));
        assert!(picks(Some(Vec3::new(4.0, 9.0, 3.5)), at));
        assert!(!picks(Some(Vec3::new(6.0, 0.0, 3.0)), at));
        assert!(!picks(None, at));
        assert!(!picks(Some(Vec3::ZERO), None));
    }
}
