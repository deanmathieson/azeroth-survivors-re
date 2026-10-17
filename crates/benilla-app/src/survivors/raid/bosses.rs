//! The Molten Core scripts so far: Lucifron with his Flamewaker Protectors, and Magmadar
//! (`docs/RAIDS.md` section 2). Names and spells are the real 1.12 ones (checked against
//! `Spell.dbc` in the tests); the numbers are ours. Damage is a fraction of the struck hero's own
//! maximum health, so a blow lands the same at any level and party size.

use super::super::bonus::School;
use super::{Blow, BossAbility as A, Dispel, Does, Payload, Pick, Script, Shape, Tone, When};

// The 1.12 spells (`Spell.dbc` names in the tests).
pub(crate) const IMPENDING_DOOM: u32 = 19702;
pub(crate) const LUCIFRONS_CURSE: u32 = 19703;
pub(crate) const SHADOW_SHOCK: u32 = 19460;
pub(crate) const DOMINATE_MIND: u32 = 20604;
pub(crate) const CLEAVE: u32 = 20605;
pub(crate) const PANIC: u32 = 19408;
pub(crate) const LAVA_BOMB: u32 = 19411;
pub(crate) const FRENZY: u32 = 19451;
/// Magma Spit's debuff (19449 is Magmadar's passive that applies it, with no visual).
pub(crate) const MAGMA_SPIT: u32 = 19450;
pub(crate) const BERSERK: u32 = 26662;

/// The raid-warning horn, the 1.12 sound for a boss emote.
const WARNING: &str = "RaidWarning";
/// The cores' generic enrage emote.
const BERSERK_EMOTE: &str = "%s goes into a berserker rage!";

const fn every(first: f32, every: f32) -> When {
    When::Every { first, every }
}

/// Lucifron: curses for the decursers, Impending Doom for the healers, Shadow Shock ahead of him
/// for the tank, and two Protectors to kill first. Soft enrage at 2:00 (the curse takes three),
/// hard at 3:00.
pub(crate) static LUCIFRON: Script = Script {
    name: "Lucifron",
    abilities: &[
        A::new(
            "Flamewaker Protectors",
            0,
            When::At(0.5),
            Pick::Boss,
            Does::Adds {
                name: PROTECTOR_NAME,
                count: 2,
                radius: 5.0,
            },
        ),
        // Every 20 s on the party's curse count: auto-casts at half rate for 20 s. A curse: the
        // Mage's or Druid's pulse takes it off.
        A::new(
            "Lucifron's Curse",
            LUCIFRONS_CURSE,
            every(10.0, 20.0),
            Pick::Random(0),
            Does::Debuff {
                dispel: Dispel::Curse,
                secs: 20.0,
                payload: Payload::CooldownRate(0.5),
            },
        )
        .in_phases(0b01),
        A::new(
            "Lucifron's Curse",
            LUCIFRONS_CURSE,
            every(10.0, 20.0),
            Pick::Random(3),
            Does::Debuff {
                dispel: Dispel::Curse,
                secs: 20.0,
                payload: Payload::CooldownRate(0.5),
            },
        )
        .in_phases(0b10),
        // Every 25 s on everyone within 30 yd: 35% of max health in 10 s unless dispelled. Magic:
        // the doomed huddle on the Priest or Paladin.
        A::new(
            "Impending Doom",
            IMPENDING_DOOM,
            every(18.0, 25.0),
            Pick::Within(30.0),
            Does::Debuff {
                dispel: Dispel::Magic,
                secs: 10.0,
                payload: Payload::Doom(0.35),
            },
        ),
        // A frontal cone at his target: only the tank belongs ahead of him.
        A::new(
            "Shadow Shock",
            SHADOW_SHOCK,
            every(6.0, 9.0),
            Pick::Tank,
            Does::Ground {
                shape: Shape::Cone {
                    radius: 14.0,
                    half_angle: 0.6,
                },
                windup: 2.0,
                blow: Blow::hit(0.25, School::Shadow),
                tone: Tone::Damage,
            },
        ),
        A::new(
            "Soft enrage",
            0,
            When::At(120.0),
            Pick::Boss,
            Does::Phase(2),
        )
        .yell("%s's curses spread!", WARNING),
        A::new(
            "Berserk",
            BERSERK,
            When::At(180.0),
            Pick::Boss,
            Does::Berserk {
                damage: 3.0,
                speed: 1.5,
            },
        )
        .yell(BERSERK_EMOTE, WARNING),
    ],
};

/// The Protector's name, as the stage roster's add.
pub(crate) const PROTECTOR_NAME: &str = "Flamewaker Protector";

/// Lucifron's two Protectors: a Cleave ahead of them, and (three heroes or more, as RAIDS.md
/// section 7 has it) Dominate Mind on a hero who isn't the tank.
pub(crate) static PROTECTOR: Script = Script {
    name: PROTECTOR_NAME,
    abilities: &[
        A::new(
            "Cleave",
            CLEAVE,
            every(4.0, 8.0),
            Pick::Tank,
            Does::Ground {
                shape: Shape::Cone {
                    radius: 8.0,
                    half_angle: 1.0,
                },
                windup: 2.0,
                blow: Blow::hit(0.15, School::Physical),
                tone: Tone::Damage,
            },
        ),
        // Held for 6 s, its allies within 6 yd burned each second: they spread away from it. Magic.
        A::new(
            "Dominate Mind",
            DOMINATE_MIND,
            every(15.0, 30.0),
            Pick::RandomNonTank(1),
            Does::Debuff {
                dispel: Dispel::Magic,
                secs: 6.0,
                payload: Payload::Dominate {
                    radius: 6.0,
                    damage: 0.06,
                },
            },
        )
        .min_party(3),
    ],
};

/// Magmadar: Panic's fear, Lava Bomb's pools building up, a Frenzy for the Hunter, and Magma Spit
/// stacking on his tank. Soft enrage at 2:30 (the pools never cool), hard at 3:30.
pub(crate) static MAGMADAR: Script = Script {
    name: "Magmadar",
    abilities: &[
        // A 2 s wind-up with a yellow ring: everyone inside flees from him for 6 s.
        A::new(
            "Panic",
            PANIC,
            every(15.0, 30.0),
            Pick::Boss,
            Does::Ground {
                shape: Shape::Circle { radius: 30.0 },
                windup: 2.0,
                blow: Blow::fear(6.0),
                tone: Tone::Control,
            },
        ),
        // Every 8 s, a 6 yd circle under a random hero, then a pool that burns for 30 s.
        A::new(
            "Lava Bomb",
            LAVA_BOMB,
            every(5.0, 8.0),
            Pick::Random(1),
            Does::Ground {
                shape: Shape::Circle { radius: 6.0 },
                windup: 1.5,
                blow: Blow::hit(0.12, School::Fire).pool(0.06, 30.0),
                tone: Tone::Damage,
            },
        )
        .in_phases(0b01),
        A::new(
            "Lava Bomb",
            LAVA_BOMB,
            every(5.0, 8.0),
            Pick::Random(1),
            Does::Ground {
                shape: Shape::Circle { radius: 6.0 },
                windup: 1.5,
                blow: Blow::hit(0.12, School::Fire).pool(0.06, f32::INFINITY),
                tone: Tone::Damage,
            },
        )
        .in_phases(0b10),
        // Every 15 s for 8 s: 60% more swings. A Hunter's Tranquilizing Shot takes it off.
        A::new(
            "Frenzy",
            FRENZY,
            every(12.0, 15.0),
            Pick::Boss,
            Does::Frenzy {
                secs: 8.0,
                haste: 0.6,
            },
        )
        .yell("%s goes into a killing frenzy!", WARNING),
        // Each swing that reaches his target adds a stack: a burn that falls off once he turns.
        A::new(
            "Magma Spit",
            MAGMA_SPIT,
            every(1.0, 1.0),
            Pick::Tank,
            Does::Debuff {
                dispel: Dispel::None,
                secs: 30.0,
                payload: Payload::Stacks {
                    per: 0.01,
                    warn: 6,
                    decay: 3.0,
                },
            },
        ),
        A::new(
            "Soft enrage",
            0,
            When::At(150.0),
            Pick::Boss,
            Does::Phase(2),
        )
        .yell("%s's lava pools stop cooling!", WARNING),
        A::new(
            "Berserk",
            BERSERK,
            When::At(210.0),
            Pick::Boss,
            Does::Berserk {
                damage: 3.0,
                speed: 1.5,
            },
        )
        .yell(BERSERK_EMOTE, WARNING),
    ],
};

/// The script a boss or add of this name fights by.
pub(crate) fn script_for(name: &str) -> Option<&'static Script> {
    [&LUCIFRON, &PROTECTOR, &MAGMADAR]
        .into_iter()
        .find(|s| s.name == name)
}
