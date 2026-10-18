//! Campfire intermissions: every roster boss's death turns into a short, host-authoritative camp.
//!
//! - **The camp.** [`tick`] starts it on a boss kill ([`PARTY_SECS`] in a party, [`SOLO_SECS`]
//!   alone): the spawns hold and the horde stands still while it lasts. A campfire burns at the
//!   party's centre ([`props`]), with its crackle, and every hero near it shows WoW's Well Rested
//!   "Zzz" ([`rested`], [`RESTED_SPELL`]).
//! - **The goblin merchant.** A real 1.12 goblin stands by the fire; clicking him opens a merchant
//!   window ([`ui`]) selling [`VendorItem`]s from the party's shared purse, the elixirs as timed
//!   buffs through the mechanics toolkit ([`super::mechanics::timed_buff`]).
//! - **Fishing.** A bobber floats at a fishing spot near the fire and dips once every
//!   [`FISH_CYCLE`] seconds; a click while it is under ([`fish_caught`]) lands a fish and a Well
//!   Fed buff, once per player and camp.
//! - **The roll-off.** "Roll for the cache" rolls 1-100 for each player; once everyone has rolled
//!   (or the camp ends) the highest roll wins an unlocked bonus cache at their feet.
//! - **Leaving.** A ready check ("Ready to move on?") ends the camp once every player is ready,
//!   else the timer runs out; then the war horn sounds. Alone, Ready or Enter breaks camp at once.
//!
//! Every interaction is a [`CampAsk`]: the local player's goes out as a [`CampRequest`], applied
//! here on a host or alone ([`apply`]), sent to the host by a co-op client, which validates it the
//! same way. The camp's state ([`Camp`]) travels on the run's replicated record, in each machine's
//! own player numbering.

mod props;
mod ui;

use bevy::prelude::*;

use super::bonus::{Bonus, StatKey};
use super::enemies::Loot;
use super::party::{self, Bot, HeroState, LocalHero, NetRole};
use super::units::flat;
use super::{Rng, Run};
use crate::sound::SoundCue;

#[cfg_attr(not(feature = "coop"), allow(unused_imports))]
pub(in crate::survivors) use props::CampProp;

/// A camp's length alone, and in a party, in seconds.
pub(super) const SOLO_SECS: f32 = 30.0;
pub(super) const PARTY_SECS: f32 = 60.0;
/// The bobber's cycle, in seconds: it dips at [`BITE_AT`] for [`BITE_SECS`].
pub(super) const FISH_CYCLE: f32 = 4.0;
pub(super) const BITE_AT: f32 = 2.8;
pub(super) const BITE_SECS: f32 = 0.9;
/// How near the fire a hero rests, in yards.
pub(super) const RESTED_RANGE: f32 = 10.0;
/// The Well Rested "Zzz" over a resting hero: `Self Visual - Sleep Until Cancelled (DND)`, whose
/// only kit is its state kit, `Sleep_State_Head.mdx`.
pub(super) const RESTED_SPELL: u32 = 16093;
/// The war horn when the camp breaks: the Horn of Echeyakee's blast.
pub(super) const WAR_HORN: &str = "HornofEcheyakee";
/// The campfire's loop, played at the fire every [`CRACKLE_EVERY`] seconds.
pub(super) const CAMPFIRE_CRACKLE: &str = "CampFireSmallLoop";
const CRACKLE_EVERY: f32 = 10.0;
/// The bobber's splash, and the line reeled in.
pub(super) const FISH_BITE: &str = "Fishing Hooked";
pub(super) const FISH_REEL: &str = "Fishing Reel in";
/// Coins on a purchase, and the roll's dice (the group loot's).
pub(super) const COIN_SOUND: &str = "LOOTWINDOWCOINSOUND";
pub(super) const DICE_SOUND: &str = "PutDownGems";
/// How long an elixir or Well Fed lasts, in seconds.
pub(super) const BUFF_SECS: f32 = 120.0;
/// The timed buffs' keys in the mechanics toolkit, far above any granted mechanic's.
const MONGOOSE_KEY: u32 = 0xCA4F_0001;
const FORTITUDE_KEY: u32 = 0xCA4F_0002;
const WELL_FED_KEY: u32 = 0xCA4F_0003;
/// Elixir of the Mongoose (+25 Agility, +2% crit): crit and a little speed.
pub(super) const MONGOOSE: &[Bonus] = &[
    Bonus::new(StatKey::CritChance, 0.04),
    Bonus::new(StatKey::MoveSpeed, 0.05),
];
/// Elixir of Fortitude (Health II, +120 health).
pub(super) const FORTITUDE: &[Bonus] = &[Bonus::new(StatKey::MaxHealth, 30.0)];
/// Well Fed (+12 Stamina and Spirit): a little health and regeneration.
pub(super) const WELL_FED: &[Bonus] = &[
    Bonus::new(StatKey::MaxHealth, 12.0),
    Bonus::new(StatKey::Regen, 1.0),
];
/// What a Healthstone restores, of the hero's health.
const HEALTHSTONE_HEAL: f32 = 0.5;

/// What the goblin sells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum VendorItem {
    Reroll,
    Banish,
    Healthstone,
    ElixirOfTheMongoose,
    ElixirOfFortitude,
}

impl VendorItem {
    pub(super) const ALL: [Self; 5] = [
        Self::Reroll,
        Self::Banish,
        Self::Healthstone,
        Self::ElixirOfTheMongoose,
        Self::ElixirOfFortitude,
    ];

    pub(super) const fn price(self) -> u32 {
        match self {
            Self::Reroll => 25,
            Self::Banish => 25,
            Self::Healthstone => 35,
            Self::ElixirOfTheMongoose | Self::ElixirOfFortitude => 50,
        }
    }

    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Reroll => "Reroll Token",
            Self::Banish => "Banish Token",
            Self::Healthstone => "Healthstone",
            Self::ElixirOfTheMongoose => "Elixir of the Mongoose",
            Self::ElixirOfFortitude => "Elixir of Fortitude",
        }
    }

    /// Its icon: the Healthstone's and the elixirs' own spells' (`Healthstone`, `Elixir of the
    /// Mongoose`, `Health II`), and a token for the reroll.
    pub(super) const fn icon(self) -> &'static str {
        match self {
            Self::Reroll => "Interface\\Icons\\INV_Misc_Token_ArgentDawn",
            Self::Banish => "Interface\\Icons\\Spell_Shadow_AnimateDead",
            Self::Healthstone => "Interface\\Icons\\INV_Stone_04",
            Self::ElixirOfTheMongoose => "Interface\\Icons\\INV_Potion_32",
            Self::ElixirOfFortitude => "Interface\\Icons\\INV_Potion_44",
        }
    }

    pub(super) const fn blurb(self) -> &'static str {
        match self {
            Self::Reroll => "One more reroll of a hand",
            Self::Banish => "Remove one offer from future hands",
            Self::Healthstone => "Restores half your health",
            Self::ElixirOfTheMongoose => "+4% crit, +5% speed, 2 min",
            Self::ElixirOfFortitude => "+30 maximum health, 2 min",
        }
    }

    /// Its place on the wire.
    #[cfg_attr(not(feature = "coop"), allow(dead_code))]
    pub(in crate::survivors) fn index(self) -> u8 {
        Self::ALL.iter().position(|&i| i == self).unwrap_or(0) as u8
    }

    #[cfg_attr(not(feature = "coop"), allow(dead_code))]
    pub(in crate::survivors) fn from_index(i: u8) -> Option<Self> {
        Self::ALL.get(usize::from(i)).copied()
    }
}

/// One camp interaction a player asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum CampAsk {
    Ready,
    Fish,
    Roll,
    Buy(VendorItem),
}

/// The local player's ask, from the camp's windows and clicks: applied here on a host or alone,
/// sent to the host by a co-op client.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) struct CampRequest(pub(in crate::survivors) CampAsk);

/// What an ask came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::survivors) enum Outcome {
    Readied,
    Caught,
    Missed,
    Rolled(u8),
    Bought(VendorItem),
    Broke,
    Nothing,
}

/// The camp's shared state, the host's, mirrored on co-op clients in their own numbering.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct Camp {
    /// Players ready to move on.
    pub(in crate::survivors) ready: Vec<u8>,
    /// Players who have landed their fish this camp.
    pub(in crate::survivors) fished: Vec<u8>,
    /// Seconds into the bobber's [`FISH_CYCLE`].
    pub(in crate::survivors) fishing_dip: f32,
    /// Each player's roll for the cache, 1 to 100.
    pub(in crate::survivors) roll: Vec<(u8, u8)>,
    pub(in crate::survivors) roll_winner: Option<u8>,
    /// Where the fire burns: the party's centre when the camp began.
    pub(in crate::survivors) fire: Option<Vec3>,
}

/// A personal line under the camp's ready check ("You catch ..."), and its seconds left: kept off
/// the run's banner, which a host shares with everyone.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct CampNote(pub(in crate::survivors) Option<(String, f32)>);

/// The players' names for the camp's windows, in this machine's numbering; co-op fills it, and a
/// player missing from it is "You" or "Player N".
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub(in crate::survivors) struct CampNames(pub(in crate::survivors) Vec<(u8, String)>);

impl CampNames {
    pub(in crate::survivors) fn of(&self, player: u8) -> String {
        self.0
            .iter()
            .find(|(p, _)| *p == player)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| {
                if player == party::LOCAL_PLAYER {
                    "You".into()
                } else {
                    format!("Player {}", player + 1)
                }
            })
    }
}

impl Camp {
    pub(in crate::survivors) fn active(&self, run: &Run) -> bool {
        run.breather > 0.0
    }

    /// A new camp around `fire`: everything of the last one forgotten.
    fn start(&mut self, fire: Vec3) {
        *self = Self {
            fire: Some(fire),
            ..default()
        };
    }

    pub(in crate::survivors) fn ready(&mut self, player: u8) {
        if !self.ready.contains(&player) {
            self.ready.push(player);
            self.ready.sort_unstable();
        }
    }

    /// `player` strikes while the bobber is where it is: a catch only in the dip, once per camp.
    pub(in crate::survivors) fn fish(&mut self, player: u8) -> bool {
        if self.fished.contains(&player) || !fish_caught(self.fishing_dip) {
            return false;
        }
        self.fished.push(player);
        self.fished.sort_unstable();
        true
    }

    pub(in crate::survivors) fn has_rolled(&self, player: u8) -> bool {
        self.roll.iter().any(|(who, _)| *who == player)
    }

    /// `player` rolls `value`: their first roll stands.
    pub(in crate::survivors) fn roll(&mut self, player: u8, value: u8) -> bool {
        if self.has_rolled(player) {
            return false;
        }
        self.roll.push((player, value));
        self.roll.sort_unstable();
        true
    }
}

const fn camp_secs(humans: usize) -> f32 {
    if humans <= 1 {
        SOLO_SECS
    } else {
        PARTY_SECS
    }
}

/// The camp breaks when its time is up, or when every standing player is ready.
fn camp_ends(left: f32, humans: &[u8], ready: &[u8]) -> bool {
    left <= 0.0 || (!humans.is_empty() && humans.iter().all(|player| ready.contains(player)))
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Camp>()
        .init_resource::<CampNote>()
        .init_resource::<CampNames>()
        .add_message::<CampRequest>()
        .add_systems(
            Update,
            (enter_breaks_camp, apply_local, feedback, camp_sounds).chain(),
        );
    props::plugin(app);
    ui::plugin(app);
}

/// The vendor's sale, validated against the shared purse: gold out, the item into effect.
pub(in crate::survivors) fn buy(run: &mut Run, hero: &mut HeroState, item: VendorItem) -> bool {
    if run.gold < item.price() {
        return false;
    }
    run.gold -= item.price();
    match item {
        VendorItem::Reroll => hero.rerolls += 1,
        VendorItem::Banish => hero.banishes += 1,
        VendorItem::Healthstone => {
            let amount = hero.stats().max_hp * HEALTHSTONE_HEAL;
            party::heal(hero, amount);
        }
        VendorItem::ElixirOfTheMongoose => {
            super::mechanics::timed_buff(hero, MONGOOSE_KEY, BUFF_SECS, MONGOOSE);
        }
        VendorItem::ElixirOfFortitude => {
            super::mechanics::timed_buff(hero, FORTITUDE_KEY, BUFF_SECS, FORTITUDE);
        }
    }
    true
}

/// One player's ask, on the host or alone: the one place a camp interaction is decided.
pub(in crate::survivors) fn apply(
    camp: &mut Camp,
    run: &mut Run,
    hero: &mut HeroState,
    ask: CampAsk,
    rng: &mut Rng,
) -> Outcome {
    if !camp.active(run) {
        return Outcome::Nothing;
    }
    match ask {
        CampAsk::Ready => {
            camp.ready(hero.player);
            Outcome::Readied
        }
        CampAsk::Fish if camp.fished.contains(&hero.player) => Outcome::Nothing,
        CampAsk::Fish => {
            if camp.fish(hero.player) {
                super::mechanics::timed_buff(hero, WELL_FED_KEY, BUFF_SECS, WELL_FED);
                Outcome::Caught
            } else {
                Outcome::Missed
            }
        }
        CampAsk::Roll => {
            let value = rng.index(100) as u8 + 1;
            if camp.roll(hero.player, value) {
                Outcome::Rolled(value)
            } else {
                Outcome::Nothing
            }
        }
        CampAsk::Buy(item) => {
            if buy(run, hero, item) {
                Outcome::Bought(item)
            } else {
                Outcome::Broke
            }
        }
    }
}

/// What the local player is told of `outcome`.
pub(in crate::survivors) fn note_of(outcome: Outcome) -> Option<String> {
    match outcome {
        Outcome::Missed => Some("The fish got away.".into()),
        Outcome::Bought(item) => Some(format!("You bought {}.", item.name())),
        Outcome::Broke => Some("You don't have enough money.".into()),
        _ => None,
    }
}

/// Alone, Enter breaks camp (in co-op it opens the chat, which takes the key first).
fn enter_breaks_camp(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    run: Res<Run>,
    camp: Res<Camp>,
    phase: Res<State<super::Phase>>,
    mut out: MessageWriter<CampRequest>,
) {
    let Some(keys) = keys else { return };
    if camp.active(&run)
        && *phase.get() == super::Phase::Playing
        && keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter])
    {
        out.write(CampRequest(CampAsk::Ready));
    }
}

/// The local player's asks on a host or alone; a co-op client sends them instead.
#[allow(clippy::too_many_arguments)]
fn apply_local(
    role: Res<NetRole>,
    mut asks: MessageReader<CampRequest>,
    mut camp: ResMut<Camp>,
    mut run: ResMut<Run>,
    mut rng: ResMut<Rng>,
    mut note: ResMut<CampNote>,
    mut heroes: Query<&mut HeroState, With<LocalHero>>,
    mut cues: MessageWriter<SoundCue>,
) {
    if *role == NetRole::Client {
        return;
    }
    for ask in asks.read() {
        let Ok(mut hero) = heroes.single_mut() else {
            return;
        };
        let outcome = apply(&mut camp, &mut run, &mut hero, ask.0, &mut rng);
        if let Outcome::Bought(_) = outcome {
            cues.write(SoundCue::flat(COIN_SOUND));
        }
        if let Some(text) = note_of(outcome) {
            note.0 = Some((text, 3.0));
        }
    }
}

/// The local player's catch and roll as the shared state shows them, the same on every machine.
fn feedback(
    time: Res<Time<Real>>,
    camp: Res<Camp>,
    run: Res<Run>,
    names: Res<CampNames>,
    mut note: ResMut<CampNote>,
    mut seen: Local<(bool, bool, Option<u8>)>,
    mut cues: MessageWriter<SoundCue>,
) {
    if let Some((_, t)) = note.0.as_mut() {
        *t -= time.delta_secs();
        if *t <= 0.0 {
            note.0 = None;
        }
    }
    if !camp.active(&run) {
        *seen = (false, false, None);
        return;
    }
    let me = party::LOCAL_PLAYER;
    let caught = camp.fished.contains(&me);
    if caught && !seen.0 {
        note.0 = Some((
            "You catch a Raw Brilliant Smallfish. You are Well Fed.".into(),
            4.0,
        ));
        cues.write(SoundCue::flat(FISH_REEL));
    }
    let rolled = camp.has_rolled(me);
    if rolled && !seen.1 {
        cues.write(SoundCue::flat(DICE_SOUND));
    }
    if let Some(winner) = camp.roll_winner.filter(|_| seen.2 != camp.roll_winner) {
        let text = if winner == me {
            "You win the roll-off: the bonus cache is yours!".to_string()
        } else {
            format!("{} wins the roll-off.", names.of(winner))
        };
        note.0 = Some((text, 4.0));
    }
    *seen = (caught, rolled, camp.roll_winner);
}

/// The fire's crackle at the camp's start and on through it, and the war horn as it breaks.
fn camp_sounds(
    time: Res<Time<Real>>,
    camp: Res<Camp>,
    run: Res<Run>,
    mut was_active: Local<bool>,
    mut crackle: Local<f32>,
    mut cues: MessageWriter<SoundCue>,
) {
    let active = camp.active(&run);
    if active {
        *crackle -= time.delta_secs();
        if !*was_active || *crackle <= 0.0 {
            *crackle = CRACKLE_EVERY;
            cues.write(SoundCue {
                kit: CAMPFIRE_CRACKLE,
                at: camp.fire,
                volume: 1.0,
            });
        }
    } else if *was_active {
        cues.write(SoundCue::flat(WAR_HORN));
    }
    *was_active = active;
}

/// Whether the bobber at `dip` seconds into its cycle is under: the moment to strike.
pub(super) fn fish_caught(dip: f32) -> bool {
    let t = dip.rem_euclid(FISH_CYCLE);
    (BITE_AT..BITE_AT + BITE_SECS).contains(&t)
}

/// Highest roll wins; a tie deliberately favours the lower player number.
pub(super) fn roll_winner(rolls: &[(u8, u8)]) -> Option<u8> {
    rolls
        .iter()
        .max_by_key(|&&(player, roll)| (roll, u8::MAX - player))
        .map(|v| v.0)
}

/// Whether a hero at `at` rests by the camp's fire.
pub(in crate::survivors) fn rested(camp: &Camp, run: &Run, at: Vec3) -> bool {
    camp.active(run)
        && camp
            .fire
            .is_some_and(|fire| flat(at - fire).length() <= RESTED_RANGE)
}

/// The party's centre: of the standing heroes, else of them all.
fn centre(heroes: &[(Vec3, bool)]) -> Option<Vec3> {
    let mean = |pts: Vec<Vec3>| {
        (!pts.is_empty()).then(|| pts.iter().copied().sum::<Vec3>() / pts.len() as f32)
    };
    mean(heroes.iter().filter(|h| h.1).map(|h| h.0).collect())
        .or_else(|| mean(heroes.iter().map(|h| h.0).collect()))
}

/// Boss loot starts camp, alone or in a party; the camp then runs its bobber, settles its
/// roll-off and breaks on the ready check or the clock.
pub(super) fn tick(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut camp: ResMut<Camp>,
    heroes: Query<(&Transform, &HeroState, Has<Bot>)>,
    mut loot: MessageReader<Loot>,
) {
    if loot.read().any(|l| l.boss) {
        let players = heroes.iter().filter(|h| !h.2).count();
        let spots: Vec<(Vec3, bool)> = heroes
            .iter()
            .map(|(tf, h, _)| (tf.translation, h.alive()))
            .collect();
        if let Some(fire) = centre(&spots) {
            run.breather = camp_secs(players);
            camp.start(fire);
            run.banner = Some((
                "The horde falls back. Make camp: spend your picks, trade, fish, roll!".into(),
                4.0,
            ));
        }
    }
    if run.breather <= 0.0 {
        return;
    }
    camp.fishing_dip = (camp.fishing_dip + time.delta_secs()).rem_euclid(FISH_CYCLE);
    let humans: Vec<u8> = heroes
        .iter()
        .filter(|(_, h, bot)| h.alive() && !bot)
        .map(|(_, h, _)| h.player)
        .collect();
    let ends = camp_ends(run.breather, &humans, &camp.ready);
    let all_rolled = !humans.is_empty() && humans.iter().all(|&p| camp.has_rolled(p));
    if camp.roll_winner.is_none() && !camp.roll.is_empty() && (all_rolled || ends) {
        camp.roll_winner = roll_winner(&camp.roll);
        let at = camp.roll_winner.and_then(|w| {
            heroes
                .iter()
                .find(|(_, h, _)| h.player == w)
                .map(|(tf, ..)| tf.translation)
        });
        if let Some(at) = at {
            let serial = run.guid();
            super::treasure::spawn_cache(&mut commands, serial, at, false);
        }
    }
    if ends {
        run.breather = 0.0;
        run.banner = Some(("The war horn sounds: defend yourselves!".into(), 3.0));
    } else {
        run.breather = (run.breather - time.delta_secs()).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::super::bonus::Bonuses;
    use super::super::data;
    use super::*;

    fn hero(player: u8) -> HeroState {
        HeroState::new(player, &data::CLASSES[0], 1, Bonuses::default())
    }

    fn camping() -> (Camp, Run) {
        let mut camp = Camp::default();
        camp.start(Vec3::ZERO);
        let run = Run {
            breather: PARTY_SECS,
            gold: 200,
            ..default()
        };
        (camp, run)
    }

    #[test]
    fn vendor_never_overspends_the_shared_purse() {
        let mut run = Run {
            gold: 24,
            ..default()
        };
        let mut hero = HeroState::default();
        let rerolls = hero.rerolls;
        assert!(!buy(&mut run, &mut hero, VendorItem::Reroll));
        assert_eq!((run.gold, hero.rerolls), (24, rerolls));
        run.gold = 25;
        assert!(buy(&mut run, &mut hero, VendorItem::Reroll));
        assert_eq!((run.gold, hero.rerolls), (0, rerolls + 1));
    }

    #[test]
    fn elixirs_are_timed_buffs_that_wear_off() {
        let mut run = Run {
            gold: 100,
            ..default()
        };
        let mut me = hero(0);
        let (hp, crit) = (me.stats().max_hp, me.stats().crit);
        assert!(buy(&mut run, &mut me, VendorItem::ElixirOfFortitude));
        assert!(buy(&mut run, &mut me, VendorItem::ElixirOfTheMongoose));
        assert_eq!(run.gold, 0);
        assert!(me.stats().max_hp > hp);
        assert!(me.stats().crit > crit);
        // A second elixir refreshes the first rather than stacking.
        super::super::mechanics::tick_buffs(&mut me, 30.0);
        run.gold = 50;
        assert!(buy(&mut run, &mut me, VendorItem::ElixirOfFortitude));
        assert!((me.stats().max_hp - hp - FORTITUDE[0].value).abs() < 1e-3);
        assert_eq!(
            super::super::mechanics::buff_left(&me, FORTITUDE_KEY),
            Some(BUFF_SECS)
        );
        super::super::mechanics::tick_buffs(&mut me, BUFF_SECS + 1.0);
        assert_eq!(super::super::mechanics::buff_left(&me, FORTITUDE_KEY), None);
        assert!((me.stats().max_hp - hp).abs() < 1e-3);
        assert!((me.stats().crit - crit).abs() < 1e-6);
    }

    #[test]
    fn a_healthstone_heals_half_and_no_more() {
        let mut run = Run {
            gold: 35,
            ..default()
        };
        let mut me = hero(0);
        let max = me.stats().max_hp;
        me.hp = 1.0;
        assert!(buy(&mut run, &mut me, VendorItem::Healthstone));
        assert!((me.hp - (1.0 + max * HEALTHSTONE_HEAL)).abs() < 1e-3);
    }

    #[test]
    fn vendor_items_round_trip_their_wire_index() {
        for item in VendorItem::ALL {
            assert_eq!(VendorItem::from_index(item.index()), Some(item));
            assert!(!item.name().is_empty() && item.price() > 0);
        }
        assert_eq!(VendorItem::from_index(9), None);
    }

    #[test]
    fn fishing_only_catches_in_the_dip() {
        assert!(fish_caught(BITE_AT + 0.1));
        assert!(fish_caught(FISH_CYCLE * 3.0 + BITE_AT + 0.5));
        assert!(!fish_caught(0.5));
        assert!(!fish_caught(BITE_AT + BITE_SECS + 0.01));
    }

    #[test]
    fn a_player_catches_once_per_camp_and_is_well_fed() {
        let (mut camp, mut run) = camping();
        let mut rng = Rng(3);
        let mut me = hero(1);
        let hp = me.stats().max_hp;
        camp.fishing_dip = 0.2;
        assert_eq!(
            apply(&mut camp, &mut run, &mut me, CampAsk::Fish, &mut rng),
            Outcome::Missed
        );
        camp.fishing_dip = BITE_AT + 0.2;
        assert_eq!(
            apply(&mut camp, &mut run, &mut me, CampAsk::Fish, &mut rng),
            Outcome::Caught
        );
        assert!(me.stats().max_hp > hp, "Well Fed");
        assert_eq!(
            apply(&mut camp, &mut run, &mut me, CampAsk::Fish, &mut rng),
            Outcome::Nothing
        );
        assert_eq!(camp.fished, vec![1]);
    }

    #[test]
    fn asks_do_nothing_outside_a_camp() {
        let (mut camp, mut run) = camping();
        run.breather = 0.0;
        let mut me = hero(0);
        let ask = CampAsk::Buy(VendorItem::Reroll);
        assert_eq!(
            apply(&mut camp, &mut run, &mut me, ask, &mut Rng(1)),
            Outcome::Nothing
        );
        assert_eq!(run.gold, 200);
    }

    #[test]
    fn purchases_tell_the_buyer_what_happened() {
        let (mut camp, mut run) = camping();
        let mut me = hero(0);
        let ask = CampAsk::Buy(VendorItem::Healthstone);
        let bought = apply(&mut camp, &mut run, &mut me, ask, &mut Rng(1));
        assert_eq!(bought, Outcome::Bought(VendorItem::Healthstone));
        assert_eq!(note_of(bought).unwrap(), "You bought Healthstone.");
        run.gold = 0;
        let broke = apply(&mut camp, &mut run, &mut me, ask, &mut Rng(1));
        assert_eq!(note_of(broke).unwrap(), "You don't have enough money.");
    }

    #[test]
    fn each_player_rolls_once_from_one_to_a_hundred() {
        let (mut camp, mut run) = camping();
        let mut rng = Rng(11);
        let mut me = hero(2);
        let Outcome::Rolled(v) = apply(&mut camp, &mut run, &mut me, CampAsk::Roll, &mut rng)
        else {
            panic!("no roll");
        };
        assert!((1..=100).contains(&v));
        assert_eq!(
            apply(&mut camp, &mut run, &mut me, CampAsk::Roll, &mut rng),
            Outcome::Nothing
        );
        assert_eq!(camp.roll, vec![(2, v)]);
    }

    #[test]
    fn roll_off_breaks_ties_by_player_number() {
        assert_eq!(roll_winner(&[(2, 81), (1, 81), (3, 12)]), Some(1));
        assert_eq!(roll_winner(&[]), None);
    }

    #[test]
    fn all_ready_or_timeout_ends_the_camp() {
        assert_eq!(camp_secs(1), SOLO_SECS);
        assert_eq!(camp_secs(2), PARTY_SECS);
        assert!(camp_ends(PARTY_SECS, &[0, 1], &[0, 1]));
        assert!(camp_ends(0.0, &[0, 1], &[]));
        assert!(!camp_ends(PARTY_SECS, &[0, 1], &[0]));
        assert!(camp_ends(SOLO_SECS, &[0], &[0]), "alone, Ready breaks camp");
    }

    #[test]
    fn heroes_near_the_fire_rest() {
        let (camp, mut run) = camping();
        assert!(rested(&camp, &run, Vec3::new(6.0, 3.0, 6.0)));
        assert!(!rested(
            &camp,
            &run,
            Vec3::new(RESTED_RANGE + 1.0, 0.0, 0.0)
        ));
        run.breather = 0.0;
        assert!(!rested(&camp, &run, Vec3::ZERO), "no camp, no rest");
    }

    #[test]
    fn names_fall_back_to_you_and_player_numbers() {
        let names = CampNames(vec![(1, "Thrall".into())]);
        assert_eq!(names.of(1), "Thrall");
        assert_eq!(names.of(party::LOCAL_PLAYER), "You");
        assert_eq!(names.of(3), "Player 4");
    }

    /// Every model, sound kit, texture and spell the camp names is in the 1.12 client data, and
    /// each is what it claims to be; every mismatch is listed at once.
    #[test]
    fn every_camp_model_sound_icon_and_spell_is_in_the_client_data() {
        use super::props::{
            BOBBER_DISPLAY, CAMPFIRE_DISPLAY, FISH_SCHOOL_DISPLAY, GOBLIN_DISPLAY, GOBLIN_GREETING,
        };
        use super::ui::{
            FIRE_ICON, FISHING_ICON, MONEY_ICONS, READY_FRAME, READY_ICON, SLOT_FRAME, WAITING_ICON,
        };
        let data = benilla_formats::wow_data_or_skip!();
        let mut chain = benilla_formats::open_chain(&data).unwrap();
        let mut wrong = Vec::new();
        let objects = benilla_formats::load_gameobject_catalog(&mut chain).unwrap();
        for (display, want) in [
            (CAMPFIRE_DISPLAY, "campfire"),
            (BOBBER_DISPLAY, "fishingbobber"),
            (FISH_SCHOOL_DISPLAY, "fishschool"),
        ] {
            let path = objects
                .model_path(display)
                .unwrap_or("")
                .to_ascii_lowercase();
            if !path.contains(want) {
                wrong.push(format!("object {display} is {path:?}, not a {want}"));
            }
        }
        let creatures = benilla_formats::load_creature_catalog(&mut chain).unwrap();
        match creatures.model(GOBLIN_DISPLAY) {
            Some(m)
                if m.model_path.to_ascii_lowercase().contains("goblinmale")
                    && m.npc_appearance.is_some() => {}
            m => wrong.push(format!(
                "creature {GOBLIN_DISPLAY} is no dressed goblin: {:?}",
                m.map(|m| m.model_path)
            )),
        }
        let kits = benilla_formats::load_sound_kit_catalog(&mut chain).unwrap();
        for kit in [
            WAR_HORN,
            CAMPFIRE_CRACKLE,
            FISH_BITE,
            FISH_REEL,
            COIN_SOUND,
            DICE_SOUND,
            GOBLIN_GREETING,
        ] {
            if kits.by_name(kit).is_none_or(|k| k.files.is_empty()) {
                wrong.push(format!("no sound kit {kit}"));
            }
        }
        let mut art = vec![
            READY_FRAME,
            READY_ICON,
            WAITING_ICON,
            FIRE_ICON,
            SLOT_FRAME,
            MONEY_ICONS,
            FISHING_ICON,
        ];
        art.extend(VendorItem::ALL.iter().map(|i| i.icon()));
        for path in art {
            if !chain.contains(&format!("{path}.blp")) {
                wrong.push(format!("no texture {path}"));
            }
        }
        let spells = benilla_formats::load_spell_catalog(&mut chain).unwrap();
        // The vendor's icons are the items' own spells' icons.
        for (spell, item) in [
            (5720, VendorItem::Healthstone),
            (17538, VendorItem::ElixirOfTheMongoose),
            (3593, VendorItem::ElixirOfFortitude),
        ] {
            let icon = spells.get(spell).and_then(|s| s.icon.clone());
            if icon.as_deref() != Some(item.icon()) {
                wrong.push(format!(
                    "{} shows {}, spell {spell} {icon:?}",
                    item.name(),
                    item.icon()
                ));
            }
        }
        let visuals = benilla_formats::load_spell_visual_catalog(&mut chain).unwrap();
        let zzz = spells
            .get(RESTED_SPELL)
            .and_then(|s| visuals.stages(s.visual))
            .and_then(|v| visuals.kit(v.state))
            .map(|kit| {
                kit.effect_slots
                    .iter()
                    .flatten()
                    .filter_map(|e| visuals.effect_path(*e))
                    .any(|p| p.to_ascii_lowercase().contains("sleep_state"))
            });
        if zzz != Some(true) {
            wrong.push(format!("spell {RESTED_SPELL} shows no Zzz state kit"));
        }
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    fn world() -> World {
        let mut w = World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs(1));
        w.insert_resource(time);
        w.init_resource::<Run>();
        w.init_resource::<Camp>();
        w.init_resource::<Messages<Loot>>();
        w
    }

    fn boss_down(w: &mut World) {
        let boss = &data::roster(0).bosses[0];
        w.write_message(Loot {
            pos: Vec3::ZERO,
            xp: 1,
            boss: true,
            elite: false,
            def: boss,
        });
    }

    #[test]
    fn a_boss_kill_makes_camp_at_the_party_centre_and_holds_the_spawns() {
        let mut w = world();
        w.spawn((Transform::from_xyz(2.0, 0.0, 0.0), hero(0)));
        w.spawn((Transform::from_xyz(-2.0, 0.0, 4.0), hero(1)));
        w.spawn((Transform::from_xyz(-30.0, 0.0, 0.0), hero(2), Bot));
        boss_down(&mut w);
        w.run_system_once(tick).unwrap();
        let fire = w.resource::<Camp>().fire.unwrap();
        assert!((fire - Vec3::new(-10.0, 0.0, 4.0 / 3.0)).length() < 1e-3);
        assert_eq!(w.resource::<Run>().breather, PARTY_SECS - 1.0);
        assert!(!super::super::strip::may_spawn(w.resource::<Run>()));
        // Both players ready: the camp breaks; the bot is never waited for.
        w.resource_mut::<Messages<Loot>>().clear();
        w.resource_mut::<Camp>().ready(0);
        w.run_system_once(tick).unwrap();
        assert!(w.resource::<Run>().breather > 0.0);
        w.resource_mut::<Camp>().ready(1);
        w.run_system_once(tick).unwrap();
        assert_eq!(w.resource::<Run>().breather, 0.0);
        assert!(super::super::strip::may_spawn(w.resource::<Run>()));
    }

    #[test]
    fn alone_the_camp_is_short_and_runs_out() {
        let mut w = world();
        w.spawn((Transform::default(), hero(0)));
        boss_down(&mut w);
        w.run_system_once(tick).unwrap();
        assert_eq!(w.resource::<Run>().breather, SOLO_SECS - 1.0);
        w.resource_mut::<Messages<Loot>>().clear();
        for _ in 0..SOLO_SECS as usize {
            w.run_system_once(tick).unwrap();
        }
        assert_eq!(w.resource::<Run>().breather, 0.0);
    }

    #[test]
    fn the_highest_roll_wins_a_bonus_cache_at_their_feet() {
        let mut w = world();
        w.spawn((Transform::default(), hero(0)));
        w.spawn((Transform::from_xyz(5.0, 0.0, 1.0), hero(1)));
        boss_down(&mut w);
        w.run_system_once(tick).unwrap();
        w.resource_mut::<Messages<Loot>>().clear();
        w.resource_mut::<Camp>().roll(0, 40);
        w.run_system_once(tick).unwrap();
        assert_eq!(
            w.resource::<Camp>().roll_winner,
            None,
            "player 2 has not rolled"
        );
        w.resource_mut::<Camp>().roll(1, 73);
        w.run_system_once(tick).unwrap();
        assert_eq!(w.resource::<Camp>().roll_winner, Some(1));
        let mut caches = w.query::<(&Transform, &super::super::treasure::Cache)>();
        let spots: Vec<Vec3> = caches.iter(&w).map(|(tf, _)| tf.translation).collect();
        assert_eq!(spots, vec![Vec3::new(5.0, 0.0, 1.0)]);
    }
}
