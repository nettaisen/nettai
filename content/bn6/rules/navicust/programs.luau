--!strict
-- BN6's NaviCust programs' effects (docs/design/navicust.md): a program is
-- the engine's (`define.navicust_program`, content/bn6/navicust/), its
-- effects the values the shared constructors make (@exelib/navicust/effects,
-- which BN5's programs share), each the work of the original's handler for
-- the program (`navicust_jt_NCPs`); the navicust system (./system.luau)
-- applies a player's programs to the navi's stats as the round is set up.
-- What a program brings when it is bugged is its `bug` (one of `BUGS`), and
-- which programs only one of on the command line works is its `exclusive`.

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
programs.chip_shuffle = effects.chip_shuffle
programs.number_open = effects.number_open
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
programs.hp = effects.hp

-- The bugs, in the order the bugs' routine runs them (`sub_813CBCC`, its
-- table `byte_813CC18` by bug and level): a program's `bug` names the one
-- it brings, the colors' count brings `status` or `status-strong`. Each
-- writes the navi's stats by its level, 1 to 3.
programs.BUGS = {
    -- 1: steps go astray (+0x31 = 1).
    "step",
    -- 2: the emotion swings (+0x24 = 1).
    "emotion",
    -- 3: a step cracks the panel behind (+0x12 = 3), at level 2 to 4 (+0x13).
    "panel",
    -- 4: from custom screen 4, 3 or 2 on, the hand shrinks (+0x63).
    "custom",
    -- 5: more random encounters (+0x28 = 1).
    "encounter",
    -- 6: Zenny for chips (+0x26 = 1).
    "result",
    -- 7: the buster fires 6, 10 or 13 blanks of 16 (+0x14) and 1, 2 or 3
    -- charged shots (+0x15).
    "buster",
    -- 8: no supports (+0x0D = 0xFF).
    "support",
    -- 9: the HP drains 1, 2 or 3 levels more (+0x18), and a hit gives the HP
    -- bug (+0x16 = 3).
    "hp",
    -- 10: NaviStats+0x62 = 3, 2 or 1, which no battle reads. No program
    -- brings it.
    "unread",
    -- 11: a status at the battle's start (+0x1A = 9): five colors.
    "status",
    -- 12: a stronger one (+0x1A = 10): six or more.
    "status-strong",
}

return programs
