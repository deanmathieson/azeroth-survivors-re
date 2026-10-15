//! The permanent bonuses from outside a run, by source. Each feature owns exactly one field of
//! [`MetaParts`] and writes it from its own system, never another's; [`sum_parts`] alone writes
//! [`MetaBonuses`], their sum, which [`super::hud::start_run`] copies into each run.

use bevy::prelude::*;

use super::bonus::Bonuses;
use super::MetaBonuses;

/// One [`Bonuses`] per source. A feature replaces its own field whole and touches no other.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(super) struct MetaParts {
    /// Written only by [`super::talents`].
    pub(super) talents: Bonuses,
    /// Written only by [`super::gear`].
    pub(super) gear: Bonuses,
    /// Written only by [`super::rewards`].
    pub(super) perks: Bonuses,
    /// Written only by the race workstream.
    pub(super) race: Bonuses,
    /// Written only by the Stable.
    pub(super) mount: Bonuses,
}

impl MetaParts {
    /// Every part added together.
    pub(super) fn total(&self) -> Bonuses {
        let mut total = Bonuses::default();
        for part in [
            &self.talents,
            &self.gear,
            &self.perks,
            &self.race,
            &self.mount,
        ] {
            total.merge(part);
        }
        total
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<MetaParts>()
        .add_systems(PostUpdate, sum_parts.run_if(resource_changed::<MetaParts>));
}

/// After any write to [`MetaParts`], [`MetaBonuses`] becomes their sum, ready for the next frame's
/// run start.
fn sum_parts(parts: Res<MetaParts>, mut meta: ResMut<MetaBonuses>) {
    meta.0 = parts.total();
}

#[cfg(test)]
mod tests {
    use super::super::bonus::{Bonus, School, StatKey};
    use super::*;

    #[test]
    fn parts_sum_into_meta_bonuses() {
        let mut app = App::new();
        app.init_resource::<MetaBonuses>().add_plugins(plugin);
        app.update();
        assert_eq!(app.world().resource::<MetaBonuses>().0, Bonuses::default());

        {
            let mut parts = app.world_mut().resource_mut::<MetaParts>();
            parts.talents.add(&Bonus::new(StatKey::Damage, 0.1));
            parts.gear.add(&Bonus::new(StatKey::Damage, 0.2));
            parts.perks.add(&Bonus::new(StatKey::Gold, 0.5));
            parts.race.add(&Bonus::school(School::Fire, 0.3));
        }
        app.update();
        let meta = &app.world().resource::<MetaBonuses>().0;
        assert!((meta.get(StatKey::Damage) - 0.3).abs() < 1e-6);
        assert_eq!(meta.get(StatKey::Gold), 0.5);
        assert_eq!(meta.school_damage(School::Fire), 0.3);

        // A feature replacing its own part replaces its share of the sum.
        app.world_mut().resource_mut::<MetaParts>().gear = Bonuses::default();
        app.update();
        let meta = &app.world().resource::<MetaBonuses>().0;
        assert!((meta.get(StatKey::Damage) - 0.1).abs() < 1e-6);
        assert_eq!(meta, &app.world().resource::<MetaParts>().total());
    }
}
