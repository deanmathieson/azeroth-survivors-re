//! Find Minerals and Find Herbs (docs/PROFESSIONS.md, 4.1): one tracking at a time, as in 1.12,
//! cycled by the tracking button on the survivors minimap (off, minerals, herbs). While one is on,
//! the minimap shows each standing vein or herb of its kind as `ObjectIcons.blp`'s gold
//! resource-tracking dot. The choice is saved with the professions (`tracking`).

use bevy::prelude::*;

use super::nodes::Kind;
use super::skills::Line;

/// The tracking in force.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Tracking {
    #[default]
    Off,
    Minerals,
    Herbs,
}

impl Tracking {
    pub(crate) const ALL: [Tracking; 3] = [Tracking::Off, Tracking::Minerals, Tracking::Herbs];

    /// The next one round: off, minerals, herbs, off.
    pub(crate) fn next(self) -> Self {
        match self {
            Tracking::Off => Tracking::Minerals,
            Tracking::Minerals => Tracking::Herbs,
            Tracking::Herbs => Tracking::Off,
        }
    }

    /// The tracking spell: `(id, name, icon, line)`, Find Minerals 2580 or Find Herbs 2383.
    pub(crate) const fn spell(self) -> Option<(u32, &'static str, &'static str, Line)> {
        match self {
            Tracking::Off => None,
            Tracking::Minerals => Some((
                2580,
                "Find Minerals",
                "Interface\\Icons\\Spell_Nature_Earthquake",
                Line::Mining,
            )),
            Tracking::Herbs => Some((
                2383,
                "Find Herbs",
                "Interface\\Icons\\INV_Misc_Flower_02",
                Line::Herbalism,
            )),
        }
    }

    /// Whether a node of `kind` shows.
    pub(crate) fn shows(self, kind: Kind) -> bool {
        matches!(
            (self, kind),
            (Tracking::Minerals, Kind::Vein) | (Tracking::Herbs, Kind::Herb)
        )
    }

    /// The save's word.
    pub(crate) const fn slug(self) -> &'static str {
        match self {
            Tracking::Off => "",
            Tracking::Minerals => "minerals",
            Tracking::Herbs => "herbs",
        }
    }

    /// From the save; anything else is off.
    pub(crate) fn from_slug(slug: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|t| t.slug() == slug && !slug.is_empty())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survivors_tracking_cycles_and_saves() {
        let mut t = Tracking::Off;
        let mut seen = Vec::new();
        for _ in 0..3 {
            t = t.next();
            seen.push(t);
        }
        assert_eq!(
            seen,
            vec![Tracking::Minerals, Tracking::Herbs, Tracking::Off]
        );
        assert!(Tracking::Minerals.shows(Kind::Vein) && !Tracking::Minerals.shows(Kind::Herb));
        assert!(Tracking::Herbs.shows(Kind::Herb) && !Tracking::Herbs.shows(Kind::Pool));
        assert!(!Tracking::Off.shows(Kind::Vein));
        for t in Tracking::ALL {
            assert_eq!(Tracking::from_slug(t.slug()), t);
        }
        assert_eq!(Tracking::from_slug("treasure"), Tracking::Off);
    }
}
