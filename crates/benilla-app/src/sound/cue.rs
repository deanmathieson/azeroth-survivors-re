//! Game cues for a mode built on the client: a `SoundEntries` kit by name, played once on the SFX
//! slider, 2D or at a point. The survivors mode sounds its level-ups, coins, chests and boss
//! arrivals with the client's own entries this way (`LEVELUPSOUND`, `LOOTWINDOWCOINSOUND`, ...).
//! The kit's own gates, the volume CVars and the unfocused-window mute all apply as to any kit.

use bevy::prelude::*;

use benilla_assets::WorldAssets;
use benilla_world::schedule::WorldStage;

use super::kit::{play_kit_ext, KitRef, PlayExtras, SoundCategory, SoundKits, Volume};
use super::{AudioListener, SoundConfig, SoundOutput};

/// Play `kit` once: 2D with no `at`, else positioned there; `volume` multiplies the kit's own.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub(crate) struct SoundCue {
    pub(crate) kit: &'static str,
    pub(crate) at: Option<Vec3>,
    pub(crate) volume: f32,
}

impl SoundCue {
    /// A 2D cue at the kit's own level.
    pub(crate) fn flat(kit: &'static str) -> Self {
        Self {
            kit,
            at: None,
            volume: 1.0,
        }
    }
}

fn play_cues(
    mut cues: MessageReader<SoundCue>,
    kits: Option<ResMut<SoundKits>>,
    assets: Option<Res<WorldAssets>>,
    mut out: NonSendMut<SoundOutput>,
    config: Res<SoundConfig>,
    listener: Res<AudioListener>,
) {
    if cues.is_empty() {
        return;
    }
    let (Some(mut kits), Some(assets)) = (kits, assets) else {
        cues.clear();
        return;
    };
    for cue in cues.read() {
        if let Err(e) = play_kit_ext(
            &mut kits,
            &assets,
            &mut out,
            &config,
            listener.pos,
            KitRef::Name(cue.kit),
            cue.at,
            SoundCategory::Sfx,
            PlayExtras {
                volume_mult: Volume(cue.volume),
                ..default()
            },
        ) {
            debug!("sound cue {}: {e:#}", cue.kit);
        }
    }
}

/// Registration hook for [`super::SoundPlugin`].
pub(super) fn plugin(app: &mut App) {
    app.add_message::<SoundCue>()
        .add_systems(Update, play_cues.in_set(WorldStage::Present));
}
