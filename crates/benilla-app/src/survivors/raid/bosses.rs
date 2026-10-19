//! The Molten Core scripts: Lucifron with his Flamewaker Protectors, Magmadar, Gehennas, Garr and
//! his Firesworn, Shazzrah, Baron Geddon, Sulfuron Harbinger and his Flamewaker Priests, Golemagg
//! the Incinerator with his Core Ragers, Majordomo Executus with his Flamewaker Elites and
//! Healers, and Ragnaros with his Sons of Flame (`docs/RAIDS.md` section 2). Names and spells are the real 1.12 ones (checked against
//! `Spell.dbc` in the tests); the numbers are ours. Damage is a fraction of the struck hero's own
//! maximum health, so a blow lands the same at any level and party size.

use super::super::bonus::School;
use super::{
    After, Blow, Bond, BossAbility as A, Dispel, Does, Erupt, Master, Payload, Pick, Script, Shape,
    Submit, Tone, WardKind, When, Yell,
};

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
pub(crate) static LUCIFRON: Script = Script::of(
    "Lucifron",
    &[
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
);

/// The Protector's name, as the stage roster's add.
pub(crate) const PROTECTOR_NAME: &str = "Flamewaker Protector";

/// Lucifron's two Protectors: a Cleave ahead of them, and (three heroes or more, as RAIDS.md
/// section 7 has it) Dominate Mind on a hero who isn't the tank.
pub(crate) static PROTECTOR: Script = Script::of(
    PROTECTOR_NAME,
    &[
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
);

/// Magmadar: Panic's fear, Lava Bomb's pools building up, a Frenzy for the Hunter, and Magma Spit
/// stacking on his tank. Soft enrage at 2:30 (the pools never cool), hard at 3:30.
pub(crate) static MAGMADAR: Script = Script::of(
    "Magmadar",
    &[
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
);

// ---------------------------------------------------------------------------------------------
// Gehennas, Garr, Shazzrah, Baron Geddon, Sulfuron Harbinger, Golemagg
// ---------------------------------------------------------------------------------------------

// Their 1.12 spells (`Spell.dbc` names in the tests).
pub(crate) const GEHENNAS_CURSE: u32 = 19716;
pub(crate) const RAIN_OF_FIRE: u32 = 19717;
pub(crate) const SHADOW_BOLT: u32 = 19728;
pub(crate) const ANTIMAGIC_PULSE: u32 = 19492;
pub(crate) const MAGMA_SHACKLES: u32 = 19496;
pub(crate) const ERUPTION: u32 = 19497;
/// Garr's Enrage, a stack for each Firesworn that falls.
pub(crate) const GARR_ENRAGE: u32 = 19516;
pub(crate) const MASSIVE_ERUPTION: u32 = 20483;
pub(crate) const ARCANE_EXPLOSION: u32 = 19712;
pub(crate) const SHAZZRAHS_CURSE: u32 = 19713;
pub(crate) const COUNTERSPELL: u32 = 19715;
/// Gate of Shazzrah's blink (23138 is its trigger, with no visual).
pub(crate) const GATE_OF_SHAZZRAH: u32 = 23139;
pub(crate) const LIVING_BOMB: u32 = 20475;
pub(crate) const INFERNO: u32 = 19695;
pub(crate) const IGNITE_MANA: u32 = 19659;
pub(crate) const DARK_MENDING: u32 = 19775;
pub(crate) const INSPIRE: u32 = 19779;
pub(crate) const HAND_OF_RAGNAROS: u32 = 19780;
pub(crate) const FLAME_SPEAR: u32 = 19781;
/// Magma Splash's debuff (13879 is Golemagg's passive that applies it, with no visual).
pub(crate) const MAGMA_SPLASH: u32 = 13880;
pub(crate) const EARTHQUAKE: u32 = 19798;
pub(crate) const MANGLE: u32 = 19820;
pub(crate) const SEPARATION_ANXIETY: u32 = 23492;
pub(crate) const ARMAGEDDON: u32 = 20478;
pub(crate) const SHADOW_WORD_PAIN: u32 = 19776;
pub(crate) const PYROBLAST: u32 = 20228;
pub(crate) const GOLEMAGGS_TRUST: u32 = 20553;
pub(crate) const MAGIC_REFLECTION: u32 = 20619;
pub(crate) const DAMAGE_SHIELD: u32 = 21075;
pub(crate) const TELEPORT: u32 = 20618;
pub(crate) const BLAST_WAVE: u32 = 20229;
pub(crate) const WRATH_OF_RAGNAROS: u32 = 20566;
pub(crate) const ELEMENTAL_FIRE: u32 = 20564;
pub(crate) const MAGMA_BLAST: u32 = 20565;
/// Lava Burst (21158 is the only row of the name, with no visual of its own: its eruption plays
/// as the ring lands).
pub(crate) const LAVA_BURST: u32 = 21158;

// Majordomo's and Ragnaros's spoken lines: `SoundEntries` kits from `Sound\Creature\Executus` and
// `Sound\Creature\Ragnaros` (checked by name in the tests).
/// `A_ExecutusAggro01`: "Reckless mortals...".
pub(crate) const EXECUTUS_AGGRO: u32 = 8035;
/// `A_ExecutusDefeat01`: "Impossible! Stay your attack, mortals... I submit! I submit!".
pub(crate) const EXECUTUS_DEFEAT: u32 = 8038;
/// `A_RagnarosArrival01`: "TOO SOON! YOU HAVE AWAKENED ME TOO SOON, EXECUTUS!".
pub(crate) const RAGNAROS_ARRIVAL: u32 = 8043;
/// `A_RagnarosSpecialAttack01`: "BY FIRE BE PURGED!".
pub(crate) const RAGNAROS_PURGED: u32 = 8046;
/// `A_RagnarosReinforcements01`: "COME FORTH, MY SERVANTS! DEFEND YOUR MASTER!".
pub(crate) const RAGNAROS_SERVANTS: u32 = 8049;
/// `RagnarosDeath`, the death kit of his display.
pub(crate) const RAGNAROS_DEATH: u32 = 7555;

// The creatures' own voices, `CreatureSoundData` through each display (checked in the tests): the
// aggro roar a boss gives as it walks out, and its death cry.
/// The Flamewakers' (salamander) aggro: the `NagaAggro` kit.
pub(crate) const SALAMANDER_AGGRO: u32 = 3438;
/// `NagaDeath`.
pub(crate) const SALAMANDER_DEATH: u32 = 3436;
/// Garr's (rock elemental) aggro: `ElementalEarthAggro`.
pub(crate) const ROCK_AGGRO: u32 = 739;
/// `ElementalEarthDeath`.
pub(crate) const ROCK_DEATH: u32 = 738;
/// Baron Geddon's (fire elemental) aggro: `FireElementalAggro`.
pub(crate) const FIRE_AGGRO: u32 = 1288;
/// `FireElementalDeath`.
pub(crate) const FIRE_DEATH: u32 = 1286;
/// Golemagg's (molten giant) aggro: `MountainGiantAggro`.
pub(crate) const GIANT_AGGRO: u32 = 3685;
/// `MountainGiantDeath`.
pub(crate) const GIANT_DEATH: u32 = 3683;

/// The adds' names, as the stage rosters' adds.
pub(crate) const FLAMEWAKER_NAME: &str = "Flamewaker";
pub(crate) const FIRESWORN_NAME: &str = "Firesworn";
pub(crate) const PRIEST_NAME: &str = "Flamewaker Priest";
pub(crate) const CORE_RAGER_NAME: &str = "Core Rager";
pub(crate) const ELITE_NAME: &str = "Flamewaker Elite";
pub(crate) const HEALER_NAME: &str = "Flamewaker Healer";
pub(crate) const SON_NAME: &str = "Son of Flame";

/// The boss's roar as it walks out.
const fn aggro(kit: u32) -> A {
    A::new("Aggro", 0, When::At(0.0), Pick::Boss, Does::Yell).voice("", kit)
}

const fn berserk(at: f32) -> A {
    A::new(
        "Berserk",
        BERSERK,
        When::At(at),
        Pick::Boss,
        Does::Berserk {
            damage: 3.0,
            speed: 1.5,
        },
    )
    .yell(BERSERK_EMOTE, WARNING)
}

const fn rain_of_fire(pick: Pick, phases: u8) -> A {
    A::new(
        "Rain of Fire",
        RAIN_OF_FIRE,
        every(5.0, 12.0),
        pick,
        Does::Ground {
            shape: Shape::Circle { radius: 8.0 },
            windup: 2.0,
            blow: Blow::hit(0.08, School::Fire).pool(0.08, 6.0),
            tone: Tone::Damage,
        },
    )
    .in_phases(phases)
}

/// Gehennas: his curse quarters the healing his targets receive, Rain of Fire burns under the
/// party, a Shadow Bolt spikes someone off the tank, and two Flamewakers guard him. Soft enrage at
/// 2:00 (five circles of fire), hard at 3:00.
pub(crate) static GEHENNAS: Script = Script {
    death: Some(Yell::voice("", SALAMANDER_DEATH)),
    ..Script::of(
        "Gehennas",
        &[
            aggro(SALAMANDER_AGGRO),
            A::new(
                "Flamewakers",
                0,
                When::At(0.5),
                Pick::Boss,
                Does::Adds {
                    name: FLAMEWAKER_NAME,
                    count: 2,
                    radius: 5.0,
                },
            ),
            // Every 25 s on everyone within 30 yd, for 30 s: healing received -75%. A curse.
            A::new(
                "Gehennas' Curse",
                GEHENNAS_CURSE,
                every(8.0, 25.0),
                Pick::Within(30.0),
                Does::Debuff {
                    dispel: Dispel::Curse,
                    secs: 30.0,
                    payload: Payload::HealingTaken(0.25),
                },
            ),
            // Every 12 s, 8 yd circles under the party's curse count of heroes, burning for 6 s.
            rain_of_fire(Pick::Random(0), 0b01),
            rain_of_fire(Pick::Random(5), 0b10),
            // A small circle under someone off the tank: the healers' spike to catch.
            A::new(
                "Shadow Bolt",
                SHADOW_BOLT,
                every(7.0, 10.0),
                Pick::RandomNonTank(1),
                Does::Ground {
                    shape: Shape::Circle { radius: 3.0 },
                    windup: 2.0,
                    blow: Blow::hit(0.3, School::Shadow),
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
            .yell("Fire rains down around %s!", WARNING),
            berserk(180.0),
        ],
    )
};

/// Garr: his Firesworn (3 to 8 by party size) erupt where they fall, hurting their own kind, and
/// each one's fall enrages him; Antimagic Pulse strips the party's shields near him, Magma Shackles
/// slows it. Soft enrage at 2:30 (he detonates a Firesworn every 15 s), hard at 4:00.
pub(crate) static GARR: Script = Script {
    death: Some(Yell::voice("", ROCK_DEATH)),
    ..Script::of(
        "Garr",
        &[
            aggro(ROCK_AGGRO),
            A::new(
                "Firesworn",
                0,
                When::At(0.5),
                Pick::Boss,
                Does::Adds {
                    name: FIRESWORN_NAME,
                    count: 0,
                    radius: 8.0,
                },
            ),
            // A 20 yd ring round him: shields and timed buffs inside are stripped. Step out.
            A::new(
                "Antimagic Pulse",
                ANTIMAGIC_PULSE,
                every(10.0, 15.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 20.0 },
                    windup: 2.0,
                    blow: Blow::hit(0.0, School::Arcane).then(After::Strip),
                    tone: Tone::Safe,
                },
            ),
            // A 15 yd ring: those inside walk at half pace for 6 s. Magic.
            A::new(
                "Magma Shackles",
                MAGMA_SHACKLES,
                every(6.0, 12.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 15.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.0, School::Fire).then(After::Slow {
                        rate: 0.5,
                        secs: 6.0,
                    }),
                    tone: Tone::Control,
                },
            ),
            A::new(
                "Massive Eruption",
                MASSIVE_ERUPTION,
                every(150.0, 15.0),
                Pick::Boss,
                Does::Detonate {
                    add: FIRESWORN_NAME,
                },
            ),
            berserk(240.0),
        ],
    )
};

/// A Firesworn: as it falls it erupts, a 10 yd fire nova after 1.5 s that also hurts its own kind
/// (two killed side by side chain), and its master Garr gains a stack of Enrage.
pub(crate) static FIRESWORN: Script = Script {
    erupt: Some(Erupt {
        spell: ERUPTION,
        shape: Shape::Circle { radius: 10.0 },
        windup: 1.5,
        blow: Blow::hit(0.25, School::Fire).foes(0.6),
        tone: Tone::Damage,
    }),
    // Dragged more than 40 yd from Garr, a Firesworn's blows double (Separation Anxiety).
    master: Some(Master {
        name: "Garr",
        enrage: Some((GARR_ENRAGE, 0.15)),
        undying: false,
        refuses: None,
        bond: Some(Bond {
            spell: SEPARATION_ANXIETY,
            radius: 40.0,
            mult: 2.0,
            near: false,
        }),
    }),
    ..Script::of(FIRESWORN_NAME, &[])
};

const fn gate_of_shazzrah(every_secs: f32, phases: u8) -> A {
    A::new(
        "Gate of Shazzrah",
        GATE_OF_SHAZZRAH,
        every(15.0, every_secs),
        Pick::RandomNonTank(1),
        Does::Blink {
            windup: 1.5,
            radius: 12.0,
            blow: Blow::hit(0.12, School::Arcane),
            then: ARCANE_EXPLOSION,
        },
    )
    .in_phases(phases)
}

/// Shazzrah: Arcane Explosion round him, a curse that doubles the magic his targets take,
/// Counterspell silencing whoever stands close, and Gate of Shazzrah: a sigil under someone off
/// the tank, then he is there with his taunt dropped, exploding. Soft enrage at 2:00 (he blinks
/// every 12 s), hard at 3:00.
pub(crate) static SHAZZRAH: Script = Script {
    death: Some(Yell::voice("", SALAMANDER_DEATH)),
    ..Script::of(
        "Shazzrah",
        &[
            aggro(SALAMANDER_AGGRO),
            A::new(
                "Arcane Explosion",
                ARCANE_EXPLOSION,
                every(6.0, 6.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 12.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.12, School::Arcane),
                    tone: Tone::Damage,
                },
            ),
            // Every 25 s on everyone within 30 yd, for 20 s: magic damage taken doubled. A curse.
            A::new(
                "Shazzrah's Curse",
                SHAZZRAHS_CURSE,
                every(10.0, 25.0),
                Pick::Within(30.0),
                Does::Debuff {
                    dispel: Dispel::Curse,
                    secs: 20.0,
                    payload: Payload::DamageTaken(2.0),
                },
            ),
            // A 15 yd ring: those inside cast nothing for 2 s. The ranged stay at range.
            A::new(
                "Counterspell",
                COUNTERSPELL,
                every(12.0, 15.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 15.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.0, School::Arcane).then(After::Silence(2.0)),
                    tone: Tone::Debuff,
                },
            ),
            gate_of_shazzrah(20.0, 0b01),
            gate_of_shazzrah(12.0, 0b10),
            A::new(
                "Soft enrage",
                0,
                When::At(120.0),
                Pick::Boss,
                Does::Phase(2),
            )
            .yell("%s blinks faster!", WARNING),
            berserk(180.0),
        ],
    )
};

const fn living_bomb(every_secs: f32, phases: u8) -> A {
    A::new(
        "Living Bomb",
        LIVING_BOMB,
        every(10.0, every_secs),
        Pick::Random(1),
        Does::Debuff {
            dispel: Dispel::None,
            secs: 8.0,
            payload: Payload::Bomb {
                radius: 12.0,
                allies: 0.5,
                carrier: 0.2,
                foes: 0.5,
            },
        },
    )
    .in_phases(phases)
}

/// Baron Geddon: Living Bomb on a hero, a 12 yd ring riding on it for 8 s, then it blows up on
/// everyone near (so its carrier runs from the party, into the horde); Inferno, a pulsing nova
/// round him while he stands rooted; and Ignite Mana slowing cooldowns, magic. Soft enrage at 2:30
/// (the bomb every 10 s), hard at 3:30.
pub(crate) static BARON_GEDDON: Script = Script {
    death: Some(Yell::voice("", FIRE_DEATH)),
    ..Script::of(
        "Baron Geddon",
        &[
            aggro(FIRE_AGGRO),
            living_bomb(15.0, 0b01),
            living_bomb(10.0, 0b10),
            // A 2 s glow, then 8 pulses a second apart in 20 yd, each a quarter harder.
            A::new(
                "Inferno",
                INFERNO,
                every(20.0, 30.0),
                Pick::Boss,
                Does::Inferno {
                    radius: 20.0,
                    windup: 2.0,
                    pulses: 8,
                    every: 1.0,
                    damage: 0.03,
                    growth: 0.25,
                },
            ),
            // Every 20 s on the curse count, for 10 s: cooldowns come round a third slower. Magic.
            A::new(
                "Ignite Mana",
                IGNITE_MANA,
                every(8.0, 20.0),
                Pick::Random(0),
                Does::Debuff {
                    dispel: Dispel::Magic,
                    secs: 10.0,
                    payload: Payload::CooldownRate(0.67),
                },
            ),
            A::new(
                "Soft enrage",
                0,
                When::At(150.0),
                Pick::Boss,
                Does::Phase(2),
            )
            .yell("%s's bombs come faster!", WARNING),
            // At 2% he can't be hurt for 8 s while a 35 yd ring winds up, then it blasts.
            A::new(
                "Armageddon",
                ARMAGEDDON,
                When::Health(0.02),
                Pick::Boss,
                Does::Immune { secs: 8.0 },
            )
            .yell("%s performs one last service for Ragnaros.", WARNING),
            A::new(
                "Armageddon",
                ARMAGEDDON,
                When::Health(0.02),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 35.0 },
                    windup: 8.0,
                    blow: Blow::hit(0.35, School::Fire),
                    tone: Tone::Damage,
                },
            ),
            berserk(210.0),
        ],
    )
};

const fn inspire(when: When) -> A {
    A::new(
        "Inspire",
        INSPIRE,
        when,
        Pick::Boss,
        Does::Inspire {
            secs: f32::INFINITY,
            haste: 0.5,
            range: 40.0,
        },
    )
}

/// Sulfuron Harbinger: four Flamewaker Priests heal him (kill them first, or stun them off their
/// cast), Flame Spear marks a hero's spot, Hand of Ragnaros stuns whoever stands close, and below
/// half health Inspire hastens them all. Soft enrage at 2:30 (Inspire regardless), hard at 3:30.
pub(crate) static SULFURON: Script = Script {
    death: Some(Yell::voice("", SALAMANDER_DEATH)),
    ..Script::of(
        "Sulfuron Harbinger",
        &[
            aggro(SALAMANDER_AGGRO),
            A::new(
                "Flamewaker Priests",
                0,
                When::At(0.5),
                Pick::Boss,
                Does::Adds {
                    name: PRIEST_NAME,
                    count: 4,
                    radius: 6.0,
                },
            ),
            A::new(
                "Flame Spear",
                FLAME_SPEAR,
                every(8.0, 12.0),
                Pick::Random(1),
                Does::Ground {
                    shape: Shape::Circle { radius: 6.0 },
                    windup: 2.0,
                    blow: Blow::hit(0.2, School::Fire),
                    tone: Tone::Split,
                },
            ),
            A::new(
                "Hand of Ragnaros",
                HAND_OF_RAGNAROS,
                every(14.0, 20.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 12.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.08, School::Fire).then(After::Stun(2.0)),
                    tone: Tone::Control,
                },
            ),
            inspire(When::Health(0.5)),
            inspire(When::At(150.0)),
            berserk(210.0),
        ],
    )
};

/// A Flamewaker Priest: Dark Mending, a 2.5 s cast that heals the most hurt of Sulfuron and his
/// Priests 15%. A stun or fear on the Priest breaks it.
pub(crate) static FLAMEWAKER_PRIEST: Script = Script::of(
    PRIEST_NAME,
    &[
        A::new(
            "Dark Mending",
            DARK_MENDING,
            every(5.0, 9.0),
            Pick::Boss,
            Does::Mend {
                share: 0.15,
                cast: 2.5,
                range: 40.0,
            },
        ),
        // A hero burns 2% a second for 12 s. Magic.
        A::new(
            "Shadow Word: Pain",
            SHADOW_WORD_PAIN,
            every(7.0, 12.0),
            Pick::Random(1),
            Does::Debuff {
                dispel: Dispel::Magic,
                secs: 12.0,
                payload: Payload::Burn(0.02),
            },
        ),
    ],
);

/// Golemagg the Incinerator: Magma Splash stacking on his tank, two Core Ragers that can't die
/// while he stands, and below 10% an Earthquake round him every 6 s. Hard enrage at 4:00.
pub(crate) static GOLEMAGG: Script = Script {
    death: Some(Yell::voice("", GIANT_DEATH)),
    ..Script::of(
        "Golemagg the Incinerator",
        &[
            aggro(GIANT_AGGRO),
            A::new(
                "Core Ragers",
                0,
                When::At(0.5),
                Pick::Boss,
                Does::Adds {
                    name: CORE_RAGER_NAME,
                    count: 2,
                    radius: 6.0,
                },
            ),
            A::new(
                "Magma Splash",
                MAGMA_SPLASH,
                every(1.0, 1.0),
                Pick::Tank,
                Does::Debuff {
                    dispel: Dispel::None,
                    secs: 30.0,
                    payload: Payload::Stacks {
                        per: 0.012,
                        warn: 6,
                        decay: 3.0,
                    },
                },
            ),
            A::new(
                "Soft enrage",
                0,
                When::Health(0.1),
                Pick::Boss,
                Does::Phase(2),
            )
            .yell("The ground trembles beneath %s!", WARNING),
            // An orange 8 yd circle on a hero off the tank: 80% shared among those inside.
            A::new(
                "Pyroblast",
                PYROBLAST,
                every(12.0, 12.0),
                Pick::RandomNonTank(1),
                Does::Ground {
                    shape: Shape::Circle { radius: 8.0 },
                    windup: 3.0,
                    blow: Blow::hit(0.8, School::Fire).split(),
                    tone: Tone::Split,
                },
            ),
            A::new(
                "Earthquake",
                EARTHQUAKE,
                every(6.0, 6.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle { radius: 15.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.2, School::Physical),
                    tone: Tone::Damage,
                },
            )
            .in_phases(0b10),
            berserk(240.0),
        ],
    )
};

/// A Core Rager: Mangle ahead of it, and it can't die while Golemagg stands.
pub(crate) static CORE_RAGER: Script = Script {
    master: Some(Master {
        name: "Golemagg the Incinerator",
        enrage: None,
        undying: true,
        refuses: Some(Yell {
            text: "%s refuses to die while its master is in trouble.",
            sound: WARNING,
            kit: 0,
        }),
        // Within 20 yd of Golemagg its blows double: keep them apart.
        bond: Some(Bond {
            spell: GOLEMAGGS_TRUST,
            radius: 20.0,
            mult: 2.0,
            near: true,
        }),
    }),
    ..Script::of(
        CORE_RAGER_NAME,
        &[A::new(
            "Mangle",
            MANGLE,
            every(5.0, 8.0),
            Pick::Tank,
            Does::Ground {
                shape: Shape::Cone {
                    radius: 7.0,
                    half_angle: 0.8,
                },
                windup: 2.0,
                blow: Blow::hit(0.12, School::Physical),
                tone: Tone::Damage,
            },
        )],
    )
};

const fn ward(name: &'static str, spell: u32, kind: WardKind, first: f32, text: &'static str) -> A {
    A::new(
        name,
        spell,
        every(first, 60.0),
        Pick::Boss,
        Does::Ward {
            kind,
            secs: 10.0,
            range: 60.0,
        },
    )
    .yell(text, WARNING)
}

/// Majordomo Executus: he can't be brought below a fifth of his health (Aegis of Ragnaros); his
/// four Flamewaker Elites and four Healers must fall, and then he yields. Every 30 s for 10 s his
/// adds take a ward in turn, Magic Reflection (casters out) then Damage Shield (melee out); every
/// 20 s he sends his target onto hot coals. Hard enrage at 4:00.
pub(crate) static MAJORDOMO: Script = Script {
    aegis: 0.2,
    submits: Some(Submit {
        adds: &[ELITE_NAME, HEALER_NAME],
        line: Yell::voice(
            "Impossible! Stay your attack, mortals... I submit! I submit!",
            EXECUTUS_DEFEAT,
        ),
    }),
    ..Script::of(
        "Majordomo Executus",
        &[
            A::new("Aggro", 0, When::At(0.0), Pick::Boss, Does::Yell).voice(
                "Reckless mortals! None may challenge the Sons of the Living Flame!",
                EXECUTUS_AGGRO,
            ),
            A::new(
                "Flamewaker Elites",
                0,
                When::At(0.5),
                Pick::Boss,
                Does::Adds {
                    name: ELITE_NAME,
                    count: 4,
                    radius: 6.0,
                },
            ),
            A::new(
                "Flamewaker Healers",
                0,
                When::At(0.5),
                Pick::Boss,
                Does::Adds {
                    name: HEALER_NAME,
                    count: 4,
                    radius: 9.0,
                },
            ),
            ward(
                "Magic Reflection",
                MAGIC_REFLECTION,
                WardKind::Reflect,
                15.0,
                "Magic Reflection shields %s's adds! Casters out!",
            ),
            ward(
                "Damage Shield",
                DAMAGE_SHIELD,
                WardKind::Thorns,
                45.0,
                "Damage Shield burns on %s's adds! Melee out!",
            ),
            A::new(
                "Teleport",
                TELEPORT,
                every(20.0, 20.0),
                Pick::Tank,
                Does::Teleport {
                    yards: 25.0,
                    dps: 0.05,
                    secs: 10.0,
                },
            ),
            berserk(240.0),
        ],
    )
};

/// A Flamewaker Elite: Blast Wave throws whoever stands close.
pub(crate) static FLAMEWAKER_ELITE: Script = Script {
    master: Some(MAJORDOMOS),
    ..Script::of(
        ELITE_NAME,
        &[A::new(
            "Blast Wave",
            BLAST_WAVE,
            every(8.0, 12.0),
            Pick::Boss,
            Does::Ground {
                shape: Shape::Circle { radius: 10.0 },
                windup: 1.5,
                blow: Blow::hit(0.1, School::Fire).knocks(10.0),
                tone: Tone::Control,
            },
        )],
    )
};

/// A Flamewaker Healer: Shadow Shock under a hero off the tank, and an interruptible heal on the
/// most hurt of Majordomo's adds.
pub(crate) static FLAMEWAKER_HEALER: Script = Script {
    master: Some(MAJORDOMOS),
    ..Script::of(
        HEALER_NAME,
        &[
            A::new(
                "Shadow Shock",
                SHADOW_SHOCK,
                every(7.0, 10.0),
                Pick::RandomNonTank(1),
                Does::Ground {
                    shape: Shape::Circle { radius: 3.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.15, School::Shadow),
                    tone: Tone::Damage,
                },
            ),
            A::new(
                "Dark Mending",
                DARK_MENDING,
                every(6.0, 10.0),
                Pick::Boss,
                Does::Mend {
                    share: 0.1,
                    cast: 2.5,
                    range: 40.0,
                },
            ),
        ],
    )
};

/// Majordomo's adds' tie to him: none of its own.
const MAJORDOMOS: Master = Master {
    name: "Majordomo Executus",
    enrage: None,
    undying: false,
    refuses: None,
    bond: None,
};

/// Ragnaros: he never leaves his lava at the room's centre. Wrath of Ragnaros throws everyone in
/// his melee ring; Elemental Fire burns his tank; Lava Burst erupts under the party; with nobody in
/// his melee ring for 3 s he hurls Magma Blast. At 3:00 (and every 4:30 after) he submerges for up
/// to 90 s while his Sons of Flame (by party size) rise, and comes back with a Wrath. Hard enrage
/// at 10:00.
pub(crate) static RAGNAROS: Script = Script {
    rooted: true,
    death: Some(Yell::voice(
        "Ragnaros is banished to the Elemental Plane! The Molten Core is cleared!",
        RAGNAROS_DEATH,
    )),
    ..Script::of(
        "Ragnaros",
        &[
            A::new("Aggro", 0, When::At(0.0), Pick::Boss, Does::Yell).voice(
                "TOO SOON! YOU HAVE AWAKENED ME TOO SOON, EXECUTUS!",
                RAGNAROS_ARRIVAL,
            ),
            A::new(
                "Wrath of Ragnaros",
                WRATH_OF_RAGNAROS,
                every(25.0, 25.0),
                Pick::Boss,
                Does::Ground {
                    shape: Shape::Circle {
                        radius: super::MELEE_RING,
                    },
                    windup: 2.0,
                    blow: Blow::hit(0.1, School::Fire).knocks(25.0),
                    tone: Tone::Control,
                },
            )
            .voice("BY FIRE BE PURGED!", RAGNAROS_PURGED)
            .in_phases(0b01),
            // His tank burns 3% a second for 8 s.
            A::new(
                "Elemental Fire",
                ELEMENTAL_FIRE,
                every(8.0, 12.0),
                Pick::Tank,
                Does::Debuff {
                    dispel: Dispel::None,
                    secs: 8.0,
                    payload: Payload::Burn(0.03),
                },
            )
            .in_phases(0b01),
            A::new(
                "Lava Burst",
                LAVA_BURST,
                every(6.0, 9.0),
                Pick::Random(0),
                Does::Ground {
                    shape: Shape::Circle { radius: 6.0 },
                    windup: 1.5,
                    blow: Blow::hit(0.15, School::Fire).knocks(8.0),
                    tone: Tone::Damage,
                },
            )
            .in_phases(0b01),
            A::new(
                "Magma Blast",
                MAGMA_BLAST,
                every(0.25, 0.25),
                Pick::Random(1),
                Does::MeleeCheck {
                    range: super::MELEE_RING,
                    grace: 3.0,
                    radius: 4.0,
                    windup: 1.5,
                    blow: Blow::hit(0.5, School::Fire),
                },
            )
            .in_phases(0b01),
            A::new(
                "Submerge",
                0,
                every(180.0, 270.0),
                Pick::Boss,
                Does::Submerge {
                    secs: 90.0,
                    add: SON_NAME,
                    count: 0,
                    opener: "Wrath of Ragnaros",
                },
            )
            .voice(
                "COME FORTH, MY SERVANTS! DEFEND YOUR MASTER!",
                RAGNAROS_SERVANTS,
            ),
            berserk(600.0),
        ],
    )
};

/// A Son of Flame: one of Ragnaros's servants while he is submerged.
pub(crate) static SON_OF_FLAME: Script = Script {
    master: Some(Master {
        name: "Ragnaros",
        enrage: None,
        undying: false,
        refuses: None,
        bond: None,
    }),
    ..Script::of(SON_NAME, &[])
};

/// Every script, bosses and adds.
pub(crate) static SCRIPTS: &[&Script] = &[
    &LUCIFRON,
    &PROTECTOR,
    &MAGMADAR,
    &GEHENNAS,
    &GARR,
    &FIRESWORN,
    &SHAZZRAH,
    &BARON_GEDDON,
    &SULFURON,
    &FLAMEWAKER_PRIEST,
    &GOLEMAGG,
    &CORE_RAGER,
    &MAJORDOMO,
    &FLAMEWAKER_ELITE,
    &FLAMEWAKER_HEALER,
    &RAGNAROS,
    &SON_OF_FLAME,
];

/// The script a boss or add of this name fights by. Gehennas' Flamewakers have none: the horde's
/// Flamewakers share their name.
pub(crate) fn script_for(name: &str) -> Option<&'static Script> {
    SCRIPTS.iter().copied().find(|s| s.name == name)
}
