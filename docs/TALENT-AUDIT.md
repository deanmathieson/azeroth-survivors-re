== final message (1m19s on the Mac) ==
# Talent design audit

## Overall read

The repository has strong WoW scaffolding: real `Talent.dbc` names, icons, tiers, spell overlap handling, real 1.12 spell IDs, spec start-spell swaps, visual auras, guardians, party buffs, debuffs, resurrection and threat systems.

The weakness is that only a minority of talents create a memorable run event. Most nodes convert their original identity into generic `damage`, `cooldown`, `armor`, `crit`, `health`, `luck`, or `speed`. That makes the trees authentic in naming but not consistently exciting to spend points in.

Scores are `WoW fidelity / run noticeability / anticipation`, each 1–5.

## Warrior

### Arms — “Sweeping battlefield control that rewards weapon crits”

- Anger Management — current: permanent 4% cooldown reduction. Real 1.12: reduced rage decay and granted periodic rage. Score 2/3/2.
- Sweeping Strikes — current: teaches the spell at rank 2. Real: next five attacks hit an additional nearby target. Score 5/5/5.
- Mortal Strike — current: teaches Mortal Strike; also starts Arms runs with it and grants 10% physical damage plus 10% crit damage through the stat row. Real: weapon strike reducing healing. Score 4/5/5.
- Deep Wounds — current: +7% crit damage per rank. Real: critical strikes caused a bleed. Score 2/3/3.
- Impale — current: +10% crit damage per rank. Real: increased critical-strike damage. Score 4/3/3.
- Bland list: Improved Heroic Strike, Deflection, Improved Rend, Improved Charge, Tactical Mastery, Improved Thunder Clap, Improved Overpower, Two-Handed Weapon Specialization, Axe/Mace/Sword/Polearm Specialization, Improved Hamstring. Upgrade into Rage/weapon moments: Rend leaves a stacking bleed; Charge makes the next strike cleave; Thunder Clap leaves a slowing shockwave; Overpower always crits slowed enemies; Hamstring roots at low health; weapon specializations add a named proc. Existing proc, bleed, CC and area mechanics suffice.

### Fury — “Berserker momentum: faster attacks, rage and execution”

- Piercing Howl — current: +6% area per rank. Real: area slow. Score 2/2/2.
- Death Wish — current: +10% physical damage and -3% armor per rank. Real: damage boost with increased damage taken and fear immunity. Score 4/4/4.
- Bloodthirst — current: teaches Bloodthirst; Fury starts with it; +8% physical damage and 2% lifesteal per rank. Real: attack power-scaled strike that healed the warrior. Score 4/5/5.
- Flurry — current: 2% cooldown reduction per rank. Real: haste after a critical strike. Score 2/3/3.
- Enrage — current: +2% damage per rank. Real: damage increase after suffering a critical hit. Score 2/2/2.
- Bland list: Booming Voice, Cruelty, Improved Demoralizing Shout, Unbridled Wrath, Improved Cleave, Blood Craze, Improved Battle Shout, Dual Wield Specialization, Improved Execute, Improved Slam, Improved Intercept, Improved Berserker Rage. Upgrade Booming Voice to Battle Shout reaching allies; Cleave to chain to a third enemy; Blood Craze to heal nearby allies on proc; Execute to execute enemies below 20%; Flurry to trigger a visible haste burst after crits. Existing aura, chain, heal and execute mechanics suffice.

### Protection — “The immovable tank: shields, retaliation and emergency survival”

- Last Stand — current: at 30% health, +30% maximum health for 12 sec. Real: temporary maximum-health increase. Score 5/5/5.
- Concussion Blow — current: 5% physical damage and 5% luck per rank. Real: stun attack. Score 2/2/3.
- Shield Slam — current: teaches Shield Slam; starts Protection with it; +6% physical damage, +4% armor and +10 health per rank. Real: high-threat shield attack that dispelled one magic effect. Score 4/4/5.
- Holy Shield equivalent via warrior shield nodes is absent; shield talents remain mostly stats.
- Bland list: Shield Specialization, Anticipation, Improved Bloodrage, Toughness, Iron Will, Improved Shield Block, Improved Revenge, Defiance, Improved Sunder Armor, Improved Disarm, Improved Taunt, Improved Shield Wall, Improved Shield Bash, One-Handed Weapon Specialization. Upgrade Shield Block into a timed visible barrier; Revenge into a guaranteed counterattack after blocking; Taunt into a short party-protection window; Sunder Armor into stacking physical vulnerability; Shield Bash into interrupt plus silence. Existing shield, proc, debuff and CC mechanics suffice.

## Paladin

### Holy — “A radiant battle-healer whose crits turn into party sustain”

- Consecration — current: +8% area and 3% holy damage per rank; teaches the spell. Real: ground AoE damage. Score 4/4/4.
- Divine Favor — current: +4% crit chance per rank. Real: next heal or offensive spell guaranteed to crit. Score 2/3/3.
- Holy Shock — current: teaches it; Holy starts with it; +10% holy damage and +0.5 regen per rank. Real: instant heal or damage spell. Score 5/5/5.
- Illumination — current: 1% cooldown reduction per rank. Real: mana return after a critical heal. Score 1/2/2.
- Bland list: Divine Strength, Divine Intellect, Spiritual Focus, Improved Seal of Righteousness, Healing Light, Improved Lay on Hands, Unyielding Faith, Illumination, Improved Blessing of Wisdom, Lasting Judgement, Holy Power. Upgrade Divine Favor to guarantee the next damaging spell crit and trigger a heal pulse; Healing Light to heal allies in the spell’s area; Lay on Hands to grant a short shield; Illumination to refund/reset the next ability after a heal crit. Existing crit, heal, shield and cooldown-reset mechanics suffice.

### Protection — “A holy bulwark that turns blocking into damage and group safety”

- Blessing of Kings — current: learners start the passive at rank 2; stat effect is +10 health and 3% damage per rank. Real: 10% all-stat blessing. Score 3/3/4.
- Blessing of Sanctuary — current: permanent 4% armor and 3% holy damage per rank. Real: reduced damage and gained holy damage after blocking. Score 3/3/3.
- Holy Shield — current: +8% holy damage and 5% armor per rank; spec capstone, but no actual shield-proc mechanic. Real: charges that dealt holy damage when blocking. Score 3/2/4.
- Bland list: Improved Devotion Aura, Redoubt, Precision, Guardian’s Favor, Toughness, Improved Righteous Fury, Shield Specialization, Anticipation, Improved Hammer of Justice, Improved Concentration Aura, Reckoning, One-Handed Weapon Specialization. Upgrade Redoubt into a visible block shield after taking hits; Reckoning into a guaranteed retaliation burst; Devotion into an ally aura; Hammer of Justice into a chain-stun on elite enemies. Existing aura, shield, proc and CC mechanics suffice.

### Retribution — “Judgement, holy bursts and revenge against attackers”

- Seal of Command — current: +7% holy damage per rank. Real: chance for weapon strikes to deal extra holy damage. Score 2/3/3.
- Sanctity Aura — current: permanent +10% holy damage. Real: increased holy damage for the party. Score 4/4/4.
- Repentance — current: +8% damage and 10% crit damage; spec capstone. Real: incapacitated a humanoid target. Score 2/3/4.
- Eye for an Eye — current: +1.5% armor per rank. Real: reflected a portion of spell critical damage. Score 1/2/2.
- Bland list: Improved Blessing of Might, Benediction, Improved Judgement, Improved Seal of the Crusader, Deflection, Vindication, Conviction, Pursuit of Justice, Improved Retribution Aura, Two-Handed Weapon Specialization, Vengeance. Upgrade Seal of Command into a visible holy strike proc; Vindication into an enemy damage debuff; Eye for an Eye into spell reflection; Vengeance into stacking damage after crits; Retribution Aura into party reflect. Existing debuff, reflect, proc and aura mechanics suffice.

## Hunter

### Beast Mastery — “The pet is half the build: a relentless companion-led hunt”

- Bestial Swiftness — current: +5% movement speed per rank. Real: pet movement speed. Score 3/3/3.
- Intimidation — current: +3% damage and 5% luck per rank. Real: pet stun. Score 2/3/3.
- Bestial Wrath — current: +10% damage per rank; spec capstone adds a Frostsaber. Real: pet enraged, increased damage and immunity to fear/charm. Score 3/5/5.
- Frenzy — current: 2% cooldown reduction per rank. Real: pet attack-speed stacking after critical hits. Score 2/3/3.
- Bland list: Improved Aspect of the Hawk, Endurance Training, Improved Eyes of the Beast, Improved Aspect of the Monkey, Thick Hide, Improved Revive Pet, Pathfinding, Unleashed Fury, Improved Mend Pet, Ferocity, Spirit Bond, Bestial Discipline. Upgrade Improved Imp-like pet nodes into pet-cast buffs: Spirit Bond heals nearby allies, Mend Pet heals the party, Frenzy stacks attack speed on the Frostsaber, Intimidation makes the pet stun and taunt. Existing guardian, heal, proc and threat mechanics suffice.

### Marksmanship — “Long-range precision: marked targets explode under critical shots”

- Aimed Shot — current: teaches it and starts the spec with it; +7% physical damage. Real: slow, high-damage ranged shot. Score 4/5/5.
- Scatter Shot — current: +3% armor and +3% speed per rank. Real: short disorient. Score 1/2/3.
- Trueshot Aura — current: +10% damage; spec capstone. Real: party attack-power aura. Score 4/4/5.
- Mortal Shots — current: +5% crit damage per rank. Real: increased critical damage. Score 4/3/3.
- Bland list: Improved Concussive Shot, Efficiency, Improved Hunter’s Mark, Lethal Shots, Improved Arcane Shot, Hawk Eye, Improved Serpent Sting, Barrage, Improved Scorpid Sting, Ranged Weapon Specialization. Upgrade Hunter’s Mark into a party-wide vulnerability target; Barrage into piercing volleys; Concussive Shot into a ricochet slow; Scorpid Sting into a stacking enemy damage reduction. Existing debuff, pierce, area and CC mechanics suffice.

### Survival — “Traps and opportunistic strikes: enemies die for walking into your space”

- Deterrence — current: +5% armor per rank. Real: temporary parry/dodge immunity window. Score 2/3/3.
- Counterattack — current: +4% physical damage and 3% armor per rank. Real: attack after parrying that immobilized the target. Score 2/3/3.
- Wyvern Sting — current: +10% nature damage; spec capstone. Real: incapacitating poison sting. Score 3/3/4.
- Trap Mastery — current: +4% luck per rank. Real: improved trap duration/effect. Score 1/2/2.
- Bland list: Monster Slaying, Humanoid Slaying, Deflection, Entrapment, Savage Strikes, Improved Wing Clip, Clever Traps, Survivalist, Surefooted, Improved Feign Death, Killer Instinct, Lightning Reflexes. Upgrade Monster/Humanoid Slaying into enemy-type marked bonuses; Entrapment into roots from trap explosions; Deterrence into a short immune state; Wing Clip into a spreading slow. Existing marks, roots, immunity and trap/area mechanics suffice.

## Rogue

### Assassination — “Poison, bleeds and lethal finishers that snowball through crowds”

- Cold Blood — current: +5% crit chance per rank. Real: next offensive ability guaranteed critical. Score 2/3/4.
- Vigor — current: +6% damage and 4% cooldown reduction; spec capstone adds Deadly Poison. Real: increased energy pool. Score 2/4/4.
- Relentless Strikes — current: 5% cooldown reduction per rank. Real: combo-point finisher chance to restore energy. Score 1/2/2.
- Seal Fate — current: 2% cooldown reduction per rank. Real: critical finishers granted an extra combo point. Score 1/2/2.
- Bland list: Improved Eviscerate, Remorseless Attacks, Malice, Ruthlessness, Murder, Improved Slice and Dice, Improved Expose Armor, Lethality, Vile Poisons, Improved Poisons, Improved Kidney Shot, Improved Poisons. Upgrade Improved Poisons into stacking poison spread; Cold Blood into guaranteed crit plus poison burst; Ruthlessness into kill-reset cooldown; Murder into bonus damage to marked/bleeding enemies. Existing DoT, proc, reset and debuff mechanics suffice.

### Combat — “A sustained brawler: blades, flurry and chain-cleave”

- Riposte — current: +5% physical damage and 2% armor per rank. Real: counterattack after parry, disarming the enemy. Score 2/3/3.
- Blade Flurry — current: teaches the spell. Real: attacks an additional nearby target and increased attack speed. Score 5/5/5.
- Adrenaline Rush — current: +15% shorter cooldowns through the Combat spec form. Real: greatly increased energy regeneration. Score 3/4/5.
- Bland list: Improved Gouge, Improved Sinister Strike, Lightning Reflexes, Improved Backstab, Deflection, Precision, Endurance, Improved Sprint, Improved Kick, Dagger/Mace/Dual Wield Specialization. Upgrade Gouge into a short stun that makes the next hit cleave; Sprint into a damage trail; Kick into interrupt plus silence; Precision into crits against debuffed targets. Existing CC, area and debuff mechanics suffice.

### Subtlety — “Ambush from the shadows, then vanish before the horde can answer”

- Ghostly Strike — current: periodic physical nova. Real: damage strike that increased dodge chance. Score 2/3/3.
- Preparation — current: resets all cooldowns every 60 sec. Real: reset major rogue cooldowns. Score 4/5/4.
- Premeditation — current: periodic nova; spec capstone adds a Shadow Double. Real: granted combo points before an opening attack. Score 2/4/4.
- Bland list: Improved Gouge, Master of Deception, Camouflage, Initiative, Improved Ambush, Setup, Elusiveness, Serrated Blades, Improved Eviscerate, Deadliness. Upgrade Initiative into a first-hit burst/extra projectile; Camouflage into temporary untargetability; Setup into dodge-triggered shadow clones; Serrated Blades into armor-breaking bleeds. Existing cheat-death, clone, proc and debuff mechanics suffice.

## Priest

### Discipline — “Protective support that converts damage into shields and cooldown control”

- Inner Focus — current: permanent 1% cooldown reduction per rank. Real: next spell free and increased critical chance. Score 2/2/3.
- Divine Spirit — current: permanent aura stat effect. Real: spirit buff for the party. Score 4/3/4.
- Power Infusion — current: +10% damage and 10% damage reduction as Inner Fire; spec capstone. Real: temporary haste and spell damage buff, usable on allies. Score 3/4/5.
- Bland list: Improved Power Word: Fortitude, Martyrdom, Inner Focus, Meditation, Mental Agility, Improved Inner Fire, Mental Strength, Focused Power, Enlightenment. Upgrade Power Infusion into an ally-targetable 15-second burst; Inner Focus into the next cast echo/free cast; Meditation into regen for nearby allies; Martyrdom into a shield after taking damage. Existing burst, echo, shield and party-range mechanics suffice.

### Holy — “A healing engine whose holy damage keeps the party alive”

- Holy Nova — current: teaches it and starts Holy with it. Real: instant AoE holy damage and party heal. Score 5/4/5.
- Spirit of Redemption — current: cheat death, 50% health and 5 sec immunity. Real: temporary spirit after death that could heal. Score 3/4/4.
- Lightwell — current: spec capstone; planted guardian heals its owner for 14 every 4 sec within 15 yd. Real: clickable group healing object. Score 4/3/5.
- Bland list: Healing Focus, Improved Renew, Inspiration, Divine Fury, Holy Reach, Blessed Recovery, Holy Specialization, Healing Prayers, Spiritual Guidance, Spiritual Healing. Upgrade Lightwell to heal allies; Inspiration to grant armor after a crit heal; Renew to leave a HoT; Holy Reach to enlarge heal and damage radii. Existing heal pulse, aura and area mechanics suffice.

### Shadow — “A spreading shadow caster whose debuffs make every tick matter”

- Blackout — current: stun proc, 2% per rank on hits. Real: chance for Shadow spells to stun. Score 5/4/4.
- Vampiric Embrace — current: permanent aura effect. Real: Shadow damage healed the party. Score 4/4/5.
- Shadowform — current: permanent +15% damage aura and spec form; starts with Mind Flay. Real: Shadow damage increase and reduced physical damage taken. Score 4/4/5.
- Mind Flay — current: teaches it and starts Shadow with it. Real: channeled Shadow damage and slow. Score 5/5/5.
- Bland list: Spirit Tap, Improved Shadow Word: Pain, Shadow Focus, Improved Psychic Scream, Improved Mind Blast, Shadow Affinity, Improved Vampiric Embrace, Darkness, Shadow Reach, Improved Fade. Upgrade Shadow Word: Pain into stacking Shadow Weaving; Psychic Scream into fear chaining; Vampiric Embrace into ally healing; Spirit Tap into kill-based cooldown acceleration. Existing DoT, fear, aura and proc mechanics suffice.

## Shaman

### Elemental — “Lightning and fire crits chain through the horde”

- Elemental Focus — current: permanent aura effect. Real: clearcasting after a spell critical. Score 3/3/4.
- Elemental Fury — current: permanent crit-damage aura. Real: increased critical strike damage. Score 4/3/4.
- Elemental Mastery — current: permanent effect plus Air Elemental guardian. Real: next spell guaranteed crit and no mana cost. Score 3/4/5.
- Bland list: Convection, Concussion, Call of Flame, Elemental Warding, Reverberation, Call of Thunder, Improved Fire Totems, Eye of the Storm, Lightning Mastery, Elemental Shields. Upgrade Elemental Mastery into the next spell guaranteed crit and chain; Fire Totems into a party fire aura; Eye of the Storm into a damage burst after taking a hit. Existing echo, chain, aura and proc mechanics suffice.

### Enhancement — “Weapon imbues, wolves and melee elemental bursts”

- Two-Handed Axes and Maces — current: permanent aura effect. Real: allowed two-handed weapons. Score 3/1/2.
- Parry — current: permanent defensive aura. Real: granted parry chance. Score 4/2/2.
- Stormstrike — current: teaches it, starts Enhancement with it. Real: two weapon strikes and increased Nature damage taken. Score 5/5/5.
- Totemic Mastery — current: aura/stat effect. Real: increased totem radius. Score 2/2/3.
- Bland list: Ancestral Knowledge, Shield Specialization, Thundering Strikes, Improved Ghost Wolf, Improved Lightning Shield, Flurry, Toughness, Improved Weapon Totems, Weapon Mastery, Dual Wield, Stormstrike-adjacent stat rows. Upgrade Totemic Mastery to party-radius totems; Flurry to haste after crits; Lightning Shield to retaliatory lightning; Weapon Totems to ally weapon buffs. Existing aura, reflect, proc and totem mechanics suffice.

### Restoration — “Totems, chain healing and mana-like endurance for the party”

- Nature’s Swiftness — current: permanent cooldown/stat effect. Real: next Nature spell instant. Score 3/2/4.
- Mana Tide Totem — current: periodic Nature nova; spec starts with Earth Shock and summons Healing Stream Totem. Real: party mana regeneration totem. Score 3/3/5.
- Bland list: Improved Healing Wave, Tidal Focus, Improved Reincarnation, Ancestral Healing, Totemic Focus, Nature’s Guidance, Healing Way, Restorative Totems, Tidal Mastery, Purification. Upgrade Mana Tide into a party cooldown-reduction/regen pulse; Ancestral Healing into ally damage reduction; Healing Way into stacking heal amplification; Reincarnation into a self-revive with co-op rescue. Existing party buff, revive, heal and aura mechanics suffice.

## Mage

### Arcane — “Mana-like tempo: missiles echo, spells surge and cooldowns collapse”

- Arcane Concentration — current: every cast echoes once. Real: chance for clearcasting/free spell. Score 3/5/5.
- Presence of Mind — current: one extra cast every 30 sec. Real: next spell instant. Score 3/4/4.
- Arcane Power — current: every 60 sec, +30% damage and 30% shorter cooldowns for 15 sec; Mana Fiend spec guardian. Real: +30% spell damage with increased mana cost. Score 4/5/5.
- Bland list: Arcane Subtlety, Arcane Focus, Improved Arcane Missiles, Wand Specialization, Magic Absorption, Arcane Concentration-adjacent rows, Improved Counterspell, Arcane Meditation, Arcane Mind, Improved Mana Shield. Upgrade Arcane Missiles into a split/echo barrage; Counterspell into a silence that makes the target vulnerable; Mana Shield into an absorb that detonates; Magic Absorption into spell-shielded allies. Existing echo, split, shield and CC mechanics suffice.

### Fire — “Big crits that snowball through Ignite and burning ground”

- Impact — current: 2% chance per rank on hit to stun for 1.5 sec. Real: chance for Fire spells to stun. Score 4/4/4.
- Ignite — current: every critical strike applies 8 Fire damage/sec for 4 sec. Real: critical Fire spells caused a stacking DoT. Score 5/5/5.
- Combustion — current: every 60 sec, all blows crit for 12 sec; Fire Elemental guardian and Pyroblast start. Real: stacking critical chance until three critical spells. Score 4/5/5.
- Pyroblast — current: teaches the spell at rank 2. Real: long cast, large Fire hit with DoT. Score 5/5/5.
- Bland list: Improved Fireball, Impact-adjacent Fire Power, Flame Throwing, Improved Scorch, Improved Fire Blast, Incinerate, Improved Flamestrike, Critical Mass, Fire Power, Blazing Speed. Upgrade Improved Fireball to leave a 2-sec Ignite burn; Fire Blast to spread Ignite; Flamestrike to leave Burning Ground; Critical Mass to make burning enemies take more crit damage. Existing DoT, ground, area and vulnerability mechanics suffice.

### Frost — “Control first: chill, freeze, shatter”

- Improved Blizzard — current: chill proc. Real: stronger Blizzard slow. Score 3/3/3.
- Cold Snap — current: periodic cooldown reset every 60 sec. Real: reset Frost cooldowns. Score 5/4/4.
- Ice Block — current: lethal cheat death, 30% health and 3 sec immunity. Real: immunity and could cancel harmful effects. Score 5/5/5.
- Ice Barrier — current: 45-damage shield every 30 sec for 20 sec; Frost capstone and Water Elemental. Real: damage absorb shield. Score 5/5/5.
- Bland list: Improved Frostbolt, Frostbite, Improved Frost Nova, Permafrost, Piercing Ice, Icy Shards, Improved Cone of Cold, Arctic Reach, Frost Channeling, Shatter, Ice Floes, Winter’s Chill. Upgrade Improved Frost Nova so frozen enemies shatter for bonus damage; Shatter to guaranteed crit frozen enemies; Frostbite to spreading freeze; Blizzard to leave a slowing field. Existing CC, crit, area and ground mechanics suffice.

## Warlock

### Affliction — “Layer curses and DoTs until the whole screen is dying”

- Amplify Curse — current: periodic Shadow nova. Real: increased the effect of the next curse. Score 2/3/3.
- Siphon Life — current: periodic heal/damage effect. Real: DoT that healed the caster. Score 4/4/4.
- Dark Pact — current: periodic cooldown reset/nova-style effect; Affliction capstone and Corruption start. Real: drained mana from the pet. Score 2/3/4.
- Bland list: Improved Corruption, Suppression, Improved Curse of Agony, Improved Life Tap, Soul Siphon, Improved Drain Life, Nightfall, Grim Reach, Empowered Corruption, Shadow Embrace. Upgrade Corruption into a stacking DoT that spreads on death; Nightfall into instant Shadow Bolt/extra cast; Soul Siphon into bonus damage per afflicted enemy; Amplify Curse into a party-wide curse burst. Existing DoT, chain, echo and debuff mechanics suffice.

### Demonology — “Trade safety for a frightening demon engine”

- Fel Domination — current: permanent cooldown reduction aura. Real: reduced summon time and mana cost. Score 2/2/3.
- Demonic Sacrifice — current: permanent aura effect. Real: sacrificed the pet for a school-specific damage buff. Score 4/3/4.
- Soul Link — current: permanent damage reduction/damage-sharing style aura; Doomguard capstone. Real: shared damage with the demon. Score 5/4/5.
- Bland list: Improved Healthstone, Improved Voidwalker, Improved Succubus, Fel Intellect, Master Demonologist, Unholy Power, Demonic Embrace, Improved Imp, Improved Health Funnel, Demonic Knowledge. Upgrade Improved Healthstone to heal nearby allies; Improved Imp to cast Fire Shield on allies; Voidwalker to taunt and shield the party; Master Demonologist to change the guardian’s behavior. Existing guardian, shield, aura and threat mechanics suffice.

### Destruction — “Explosive Fire and Shadow bursts with dangerous finishers”

- Shadowburn — current: teaches it at rank 2. Real: instant Shadow damage on a dying target, consuming a shard. Score 4/4/5.
- Ruin — current: permanent crit-damage aura. Real: increased critical strike damage of Destruction spells. Score 4/3/3.
- Conflagrate — current: periodic Fire nova; Destruction capstone and Infernal guardian. Real: consumed Immolate for a burst. Score 4/4/5.
- Bland list: Improved Shadow Bolt, Cataclysm, Bane, Aftermath, Improved Firebolt, Devastation, Improved Immolate, Emberstorm, Shadow and Flame, Backlash. Upgrade Improved Immolate so Conflagrate consumes/spreads it; Aftermath into stun from Fire hits; Backlash into a reactive cast; Devastation into crits spreading burning ground. Existing DoT, CC, proc and ground mechanics suffice.

## Druid

### Balance — “Solar/lunar spellcasting with a huge Moonkin identity”

- Omen of Clarity — current: permanent aura effect. Real: chance for free next spell/attack after a hit. Score 3/2/4.
- Nature’s Grace — current: permanent cooldown effect. Real: spell haste after a critical spell. Score 3/3/4.
- Moonkin Form — current: +15% damage and visible Moonkin form; Starfire start. Real: spell crit aura, armor and shapeshift. Score 4/5/5.
- Bland list: Improved Wrath, Nature’s Grasp, Improved Entangling Roots, Natural Weapons, Improved Moonfire, Vengeance, Moonglow, Improved Starfire, Brambles, Lunar Guidance. Upgrade Moonfire into a burn that makes Starfire crit; Nature’s Grasp into roots on attackers; Vengeance into an eclipse-like crit burst; Omen into a visible free-cast proc. Existing DoT, root, proc and echo mechanics suffice.

### Feral — “Fast shapeshifting melee: bleed, pounce and pack leader”

- Feral Charge — current: periodic Nature/physical nova. Real: charge to an enemy and interrupt/catch it. Score 2/3/3.
- Faerie Fire (Feral) — current: periodic debuff-style nova. Real: armor reduction and prevented stealth. Score 3/3/3.
- Leader of the Pack — current: Cat Form, +10% speed and 5% crit; spec capstone. Real: party crit aura. Score 4/5/5.
- Bland list: Ferocity, Feral Aggression, Feral Instinct, Brutal Impact, Thick Hide, Feral Swiftness, Sharpened Claws, Blood Frenzy, Primal Fury, Predatory Strikes, Bloodthirsty. Upgrade Leader of the Pack into a party crit aura; Feral Charge into a gap-closing stun; Predatory Strikes into kill-based speed/crit; Blood Frenzy into bleed spread. Existing aura, CC, proc and DoT mechanics suffice.

### Restoration — “Regrowth, roots and treants: the forest keeps everyone standing”

- Nature’s Swiftness — current: permanent cooldown effect. Real: next Nature spell instant. Score 3/2/4.
- Swiftmend — current: heals self every 20 sec for 32; Restoration capstone adds three Treants. Real: consumed a HoT for an instant heal. Score 3/4/5.
- Insect Swarm — current: teaches it and starts Restoration with it. Real: Nature DoT reducing enemy chance to hit. Score 5/4/5.
- Bland list: Improved Mark of the Wild, Furor, Naturalist, Nature’s Focus, Natural Shapeshifter, Intensity, Subtlety, Gift of Nature, Improved Tranquility, Improved Regrowth, Living Spirit, Tree of Life-era rows absent in 1.12. Upgrade Improved Mark of the Wild to a stronger party buff; Tranquility to a party heal pulse; Regrowth to a HoT plus instant heal; Treants to heal allies or root enemies. Existing buff, heal, guardian and root mechanics suffice.

## 10/20/30 milestones

The code explicitly assigns three landmarks per tree and gold-frames headline mechanics, but the moment quality is uneven.

- Strong moments: Fire `Impact / Ignite / Combustion`; Frost `Cold Snap / Ice Barrier`; Arms `Anger Management / Sweeping Strikes / Mortal Strike`; Holy Priest `Holy Nova / Spirit of Redemption / Lightwell`; Shadow Priest `Mind Flay / Vampiric Embrace / Shadowform`; Enhancement `Totemic Mastery / Nature’s Swiftness / Stormstrike`.
- Weak moments: Protection Warrior `Last Stand / Concussion Blow / Shield Slam` because Concussion Blow is only stats; Holy Paladin `Consecration / Divine Favor / Holy Shock` because Divine Favor is only crit; Protection Paladin `Blessing of Kings / Blessing of Sanctuary / Holy Shield` because the first two are mostly permanent stats; Survival `Deterrence / Counterattack / Wyvern Sting`; Elemental `Elemental Focus / Elemental Fury / Elemental Mastery`; Restoration Shaman `Totemic Mastery / Nature’s Swiftness / Mana Tide`.
- Visuals and spell visuals exist for most bespoke mechanics. Sound is generally inherited from the talent spell visual, but there is no clearly defined milestone-specific “you just unlocked this” presentation. Add a short level-up fanfare, banner pulse and combat-text line for each 10/20/30 unlock.

## 31-point specs

The start-spell swap is excellent: it changes minute one and makes the spec legible immediately.

Strongest specs: Fire Mage, Frost Mage, Arms Warrior, Fury Warrior, Holy Priest, Shadow Priest, Enhancement Shaman, Feral Druid, Beast Mastery Hunter.

Weak or underpowered summons/forms:

- Holy Priest Lightwell only heals its owner, despite the real spell being a group object.
- Restoration Shaman Healing Stream Totem only heals its owner.
- Restoration Druid’s three Treants are melee-only and have no healing/root identity.
- Demonology’s Doomguard is melee-only and does not express the demonology fantasy strongly enough.
- Warlock Infernal is melee-only and lacks its iconic area fire.
- Hunter Beast Mastery’s Frostsaber is visually strong but mechanically generic.
- Subtlety’s Shadow Double is a generic melee guardian rather than an ambush/vanish companion.
- Arcane’s Mana Fiend and Fire’s Elemental are useful but generic bolt turrets.

## Co-op hooks

Already present:

- Battle Shout, Power Word: Fortitude, Arcane Intellect, Mark of the Wild.
- Devotion and Retribution Aura.
- Blessing of Might, Blessing of Wisdom and Trueshot Aura.
- Aspect of the Pack.
- Shaman Strength of Earth, Windfury and Mana Spring Totems.
- Blood Pact.
- Shared debuffs: Sunder Armor, Expose Armor, Faerie Fire, curses, Hunter’s Mark, Winter’s Chill, Fire Vulnerability, Shadow Weaving and Judgement of the Crusader.
- Resurrection, Redemption, Ancestral Spirit, Rebirth, Soulstone and Reincarnation.
- Tank stance/form threat and taunt.

Missing or underused talent hooks:

- Power Infusion should be explicitly ally-targetable.
- Divine Spirit should be a party-facing talent, not just a self aura.
- Blessing of Sanctuary should protect nearby allies and reward blocking.
- Improved Battle Shout should increase party radius.
- Improved Imp should cast Fire Shield on allies.
- Lightwell and Healing Stream should heal allies.
- Leader of the Pack should grant a party crit aura.
- Vampiric Embrace should heal allies from Shadow damage.
- Improved Healthstone should heal allies.
- Soul Link should visibly share damage with the demon and possibly protect a nearby ally.
- Nature’s Grasp, Innervate-like restoration effects, Totemic Mastery and Vigilance-like tank protection are not represented as talent-driven co-op moments.

## Tooltips

The system has WoW-like `Requires N points in X Talents` and prerequisite wording, but mechanic descriptions are still modern roguelite sentences rather than rank lines.

Bad examples:

- `"Teaches Pyroblast: your runs offer it, and you take it at rank 2."`
- `"For the whole run: +15% damage."`

Other problems:

- No `Rank 1/5`, `Rank 2/5` presentation.
- No real WoW-style “Requires X points in Y” line alongside the effect in the main tooltip.
- “Luck” is used where the underlying talent was a proc chance, making talents like Trap Mastery and Concussion Blow feel arbitrary.
- Spec tooltips say `"Summons a Frostsaber"` or `"Holds Shadowform all run"` but do not state the mechanical fantasy in the same block.
- Talent names are real, but several effects are disconnected enough that the name becomes misleading.

## Ranked TOP 20 fixes

1. Convert every 10/20/30 milestone into a visible, audible, run-changing ability or proc.
2. Make Lightwell and Healing Stream heal allies, not only their owner.
3. Add party-targeted Power Infusion.
4. Make Leader of the Pack a real party crit aura.
5. Give Fireball, Frost Nova and Healthstone their signature WoW behavior.
6. Replace the weakest stat-only capstone mechanics with signature procs.
7. Make Conflagrate consume/spread Immolate.
8. Make Deep Wounds an actual stacking bleed.
9. Make Seal of Command a visible holy strike proc.
10. Give Shield Slam/Concussion Blow real block, stun or dispel behavior.
11. Make Trueshot Aura and Divine Spirit explicitly party-facing.
12. Give Shadowform the missing physical-damage reduction identity.
13. Make Vampiric Embrace heal allies.
14. Make Totemic Mastery enlarge party totem reach and show the radius.
15. Make Doomguard and Infernal cast iconic ranged/area attacks.
16. Make Treants heal or root, not just perform generic melee swings.
17. Add milestone unlock presentation: banner, sound, combat text and card animation.
18. Add rank lines and proper “Requires” blocks to every tooltip.
19. Replace “luck” with named proc text wherever the original talent was a proc.
20. Add a small library of reusable mechanics: stacking bleed, next-cast guarantee, ally target, consume/spread DoT, and party aura.

## Verdict

The project has an unusually good WoW authenticity foundation, especially in names, data, spell visuals, specs and co-op infrastructure. The current talent system is approximately halfway to the stated promise: the capstones and a few iconic talents are genuinely exciting, but many intermediate nodes are still passive accounting.

The highest-value direction is not adding more trees. It is turning existing names into unmistakable gameplay verbs: burn, shatter, spread, taunt, shield, heal allies, empower the next cast, reset after a kill, and summon something that behaves like the original class fantasy.