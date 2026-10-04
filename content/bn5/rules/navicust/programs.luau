--!strict
-- BN5's NaviCust programs' effects (docs/design/bn5-map.md §15.13): a program is the engine's
-- (`define.navicust_program`, content/bn5/navicust/), its effects the values the shared constructors make
-- (@exelib/navicust/effects, BN6's too), each the work of the original's handler for the program (the table at
-- 0x0813FB44); the navicust system (./system.luau) applies a player's programs to the navi's stats as the round is set
-- up. What a program brings when it is bugged is its `bug` (one of `BUGS`), and which programs only one of on the
-- command line works is its `exclusive`.

local effects = require("@exelib/navicust/effects")

local programs = {}

export type Effect = NaviCustEffect

programs.super_armor = effects.super_armor
programs.custom = effects.custom
programs.mega_folder = effects.mega_folder
programs.giga_folder = effects.giga_folder
programs.first_barrier = effects.first_barrier
programs.back_special = effects.back_special
programs.float_shoes = effects.float_shoes
programs.air_shoes = effects.air_shoes
programs.undershirt = effects.undershirt
programs.collect = effects.collect
programs.outside = effects.outside
programs.attack = effects.attack
programs.rapid = effects.rapid
programs.charge = effects.charge
programs.attack_max = effects.attack_max
programs.rapid_max = effects.rapid_max
programs.charge_max = effects.charge_max
programs.bug_stop = effects.bug_stop
programs.rush = effects.rush
programs.beat = effects.beat
programs.tango = effects.tango
programs.soul_time = effects.soul_time
programs.hp = effects.hp

-- The bugs, in the order the bugs' routine runs them (0x08140008, its table 0x08140054 by bug group and level): a
-- program's `bug` names the one it brings, the colors' count brings `status` or `status-strong`. Each writes the navi's
-- stats by its level, 1 to 3 (./system.luau).
programs.BUGS = {
    -- 1: steps go astray (+0x31 = 1).
    "step",
    -- 2: the emotion swings (+0x24 = 1).
    "emotion",
    -- 3: a step cracks the panel behind (+0x12 = 3), at level 2, 4 or 8 (+0x13).
    "panel",
    -- 4: the custom screen hurts 20, 40 or 80 more (+0x54).
    "custom",
    -- 5: more random encounters (+0x28 = 1).
    "encounter",
    -- 6: Zenny for chips (+0x26 = 1).
    "result",
    -- 7: the buster fires blanks (+0x14 = 4) and charged shots (+0x15 = 2), at every level.
    "buster",
    -- 8: no supports (+0x0D = 0xFF).
    "support",
    -- 9: a hit gives the HP bug (+0x16 = 3).
    "hp",
    -- 10: HubBatc's: the HP programs give half (the maximum the base and half of them, the HP no more than it), which
    -- the HP's routine then undoes (0x0803C13C sets the maximum and the HP whole).
    "hub",
    -- 11: a status at the battle's start (+0x1A = 9): five colors.
    "status",
    -- 12: a stronger one (+0x1A = 10): six or more.
    "status-strong",
}

return programs
