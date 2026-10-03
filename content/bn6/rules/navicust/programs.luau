--!strict
-- BN6's NaviCust programs' effects (docs/design/navicust.md): a program is
-- the engine's (`define.navicust_program`, content/bn6/navicust/), its
-- effects the values this module's constructors make, each the work of the
-- original's handler for the program (`navicust_jt_NCPs`); the navicust
-- system (./system.luau) applies a player's programs to the navi's stats as
-- the round is set up. What a program brings when it is bugged is its `bug`
-- (one of `BUGS`), and which programs only one of on the command line works
-- is its `exclusive`.

local programs = {}

export type Effect = NaviCustEffect

local function effect(e: Effect): Effect
    return e
end

-- SuprArmr: no flinching (NaviStats+0x23 = 1).
function programs.super_armor(): Effect
    return effect({ kind = "super_armor" })
end

-- Custom1, Custom2: the custom screen deals `n` more, at most 8 (+0x0A).
function programs.custom(n: number): Effect
    return effect({ kind = "custom", amount = n })
end

-- MegFldr1, MegFldr2: `n` more Mega chips in the folder, at most 10 (+0x0B).
function programs.mega_folder(n: number): Effect
    return effect({ kind = "mega_folder", amount = n })
end

-- GigFldr1: `n` more Giga chips, at most 10 (+0x0C).
function programs.giga_folder(n: number): Effect
    return effect({ kind = "giga_folder", amount = n })
end

-- FstBarr: the navi enters with this barrier (+0x06).
function programs.first_barrier(barrier: Barrier): Effect
    return effect({ kind = "first_barrier", barrier = barrier })
end

-- Shield, Reflect, AntiDmg: B+Back's weapon (+0x07).
function programs.back_special(weapon: Weapon): Effect
    return effect({ kind = "back_special", weapon = weapon })
end

-- FlotShoe, AirShoes, UnderSht (+0x1B, +0x1C, +0x1D).
function programs.float_shoes(): Effect
    return effect({ kind = "float_shoes" })
end

function programs.air_shoes(): Effect
    return effect({ kind = "air_shoes" })
end

function programs.undershirt(): Effect
    return effect({ kind = "undershirt" })
end

-- ChpShufl: the custom screen re-deals (+0x60 = 1); NumbrOpn: it deals ten
-- (+0x61 = 1).
function programs.chip_shuffle(): Effect
    return effect({ kind = "chip_shuffle" })
end

function programs.number_open(): Effect
    return effect({ kind = "number_open" })
end

-- Collect: more chips dropped (+0x26's bit 1).
function programs.collect(): Effect
    return effect({ kind = "collect" })
end

-- What the program does outside battle, to a stat no netbattle reads (the
-- engine doesn't keep it): SneakRun (+0x1E = 1), OilBody, Fish, Battery,
-- Jungle (+0x27 = 2, 4, 8, 0x10), Millions (+0x33 = 1), Humor (+0x25 = 1),
-- Poem (+0x5F = 1), SlipRunr (+0x35 = 1), AutoHeal (+0x36 = 1).
function programs.outside(what: string): Effect
    return effect({ kind = "outside", what = what })
end

-- The buster's levels: `n` more, at most 4 (+0x01, +0x02, +0x03), or 4.
function programs.attack(n: number): Effect
    return effect({ kind = "attack", amount = n })
end

function programs.rapid(n: number): Effect
    return effect({ kind = "rapid", amount = n })
end

function programs.charge(n: number): Effect
    return effect({ kind = "charge", amount = n })
end

function programs.attack_max(): Effect
    return effect({ kind = "attack_max" })
end

function programs.rapid_max(): Effect
    return effect({ kind = "rapid_max" })
end

function programs.charge_max(): Effect
    return effect({ kind = "charge_max" })
end

-- BugStop: no bug (+0x1F = 1, which the bugs' routine reads first:
-- `sub_813C490`).
function programs.bug_stop(): Effect
    return effect({ kind = "bug_stop" })
end

-- Rush, Beat, Tango: a support (+0x0D's bits 0, 1, 2).
function programs.rush(): Effect
    return effect({ kind = "rush" })
end

function programs.beat(): Effect
    return effect({ kind = "beat" })
end

function programs.tango(): Effect
    return effect({ kind = "tango" })
end

-- HP+50 to HP+500: `n` more HP (the HP the programs add, `Unk2004334`,
-- which the maximum adds to the base: `sub_803CED4`).
function programs.hp(n: number): Effect
    return effect({ kind = "hp", amount = n })
end

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
