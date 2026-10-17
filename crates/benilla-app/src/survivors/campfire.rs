//! The short, host-authoritative rest between roster bosses. A camp uses the existing spawn hold.

use bevy::prelude::*;

use super::enemies::Loot;
use super::party::HeroState;
use super::Run;

pub(super) const SOLO_SECS: f32 = 30.0;
pub(super) const PARTY_SECS: f32 = 60.0;
/// `GameObjectDisplayInfo`'s Campfire.mdx row in the 1.12 client data.
pub(super) const CAMPFIRE_DISPLAY: u32 = 336;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VendorItem {
    Reroll,
    Healthstone,
    ElixirOfTheMongoose,
    ElixirOfFortitude,
}

impl VendorItem {
    pub(super) const fn price(self) -> u32 {
        match self {
            Self::Reroll => 25,
            Self::Healthstone => 35,
            Self::ElixirOfTheMongoose | Self::ElixirOfFortitude => 50,
        }
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(super) struct Camp {
    pub(super) ready: Vec<u8>,
    pub(super) fishing_dip: f32,
    pub(super) roll: Vec<(u8, u8)>,
}

#[derive(Component)]
struct CampfireObject;

impl Camp {
    pub(super) fn active(&self, run: &Run) -> bool {
        run.breather > 0.0
    }
    pub(super) fn ready(&mut self, player: u8) {
        if !self.ready.contains(&player) {
            self.ready.push(player);
            self.ready.sort_unstable();
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Camp>().add_systems(Update, visuals);
}

/// One client-side object at the party centre makes the shared state tangible; it is not a
/// collider, so the camp never interferes with the arena's navigation.
fn visuals(
    mut commands: Commands,
    mut run: ResMut<Run>,
    camp: Res<Camp>,
    heroes: Query<&Transform, With<HeroState>>,
    fires: Query<Entity, With<CampfireObject>>,
) {
    if camp.active(&run) && fires.is_empty() {
        let mut center = Vec3::ZERO;
        let mut count = 0.0;
        for tf in &heroes {
            center += tf.translation;
            count += 1.0;
        }
        if count > 0.0 {
            center /= count;
            let fire = super::units::spawn_object(
                &mut commands,
                run.next_guid,
                CAMPFIRE_DISPLAY,
                1.0,
                center,
            );
            run.next_guid += 1;
            commands.entity(fire).insert(CampfireObject);
        }
    } else if !camp.active(&run) {
        for fire in &fires {
            commands.entity(fire).despawn();
        }
    }
}

/// The host validates the shared purse before applying an item.
pub(super) fn buy(run: &mut Run, hero: &mut HeroState, item: VendorItem) -> bool {
    if run.gold < item.price() {
        return false;
    }
    run.gold -= item.price();
    match item {
        VendorItem::Reroll => hero.rerolls += 1,
        VendorItem::Healthstone => hero.hp = hero.hp.max(hero.stats().max_hp * 0.55),
        VendorItem::ElixirOfTheMongoose => hero.overcharges[0] += 1,
        VendorItem::ElixirOfFortitude => hero.overcharges[2] += 1,
    }
    true
}

/// A bobber catch is valid only during its dip, centred at 0.5 of its one-second cycle.
pub(super) fn fish_caught(dip: f32) -> bool {
    (dip.rem_euclid(1.0) - 0.5).abs() <= 0.12
}

/// Highest roll wins; a tie deliberately favours the lower player number.
pub(super) fn roll_winner(rolls: &[(u8, u8)]) -> Option<u8> {
    rolls
        .iter()
        .max_by_key(|&&(player, roll)| (roll, u8::MAX - player))
        .map(|v| v.0)
}

/// Boss loot starts camp in solo and co-op. Enter ends solo; humans must all ready in a party.
pub(super) fn tick(
    time: Res<Time>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut run: ResMut<Run>,
    mut camp: ResMut<Camp>,
    heroes: Query<&HeroState>,
    mut loot: MessageReader<Loot>,
) {
    if loot.read().any(|l| l.boss) {
        let humans = heroes.iter().filter(|h| h.player != u8::MAX).count();
        run.breather = if humans <= 1 { SOLO_SECS } else { PARTY_SECS };
        camp.ready.clear();
        camp.roll.clear();
        run.banner = Some(("Campfire — spend picks, fish, or ready up".into(), 4.0));
    }
    if run.breather <= 0.0 {
        return;
    }
    camp.fishing_dip = (camp.fishing_dip + time.delta_secs()).rem_euclid(1.0);
    let humans: Vec<u8> = heroes
        .iter()
        .filter(|h| h.alive() && h.player != u8::MAX)
        .map(|h| h.player)
        .collect();
    let enter = keys.is_some_and(|k| k.just_pressed(KeyCode::Enter));
    if humans.len() > 1 && enter {
        camp.ready(super::party::LOCAL_PLAYER);
    }
    if humans.len() <= 1 && enter {
        run.breather = 0.0;
    } else if humans.len() > 1 && humans.iter().all(|p| camp.ready.contains(p)) {
        run.breather = 0.0;
    } else {
        run.breather = (run.breather - time.delta_secs()).max(0.0);
    }
    if run.breather <= 0.0 {
        run.banner = Some(("The horn sounds — defend yourselves!".into(), 3.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vendor_never_overspends_the_shared_purse() {
        let mut run = Run {
            gold: 24,
            ..default()
        };
        let mut hero = HeroState::default();
        assert!(!buy(&mut run, &mut hero, VendorItem::Reroll));
        run.gold = 25;
        assert!(buy(&mut run, &mut hero, VendorItem::Reroll));
        assert_eq!((run.gold, hero.rerolls), (0, 4));
    }
    #[test]
    fn fishing_only_catches_in_the_dip() {
        assert!(fish_caught(0.5));
        assert!(!fish_caught(0.2));
    }
    #[test]
    fn roll_off_breaks_ties_by_player_number() {
        assert_eq!(roll_winner(&[(2, 81), (1, 81), (3, 12)]), Some(1));
    }
}
