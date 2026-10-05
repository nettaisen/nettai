--!strict
-- The NaviCust programs' effects EXE5's and EXE6's share (docs/design/navicust.md): a program is the engine's
-- (`define.navicust_program`), its effects the values these constructors make, each the work of the original's handler
-- for the program (EXE6's `navicust_jt_NCPs`, EXE5's table at 0x0813FB44, the same NaviStats bytes); ./compile applies a
-- player's programs to the navi's stats as the round is set up. A game's rules/navicust/programs.luau hands them to its
-- programs, with the bugs its programs bring.

local effects = {}

export type Effect = NaviCustEffect

local function effect(e: Effect): Effect
    return e
end

-- SprArmr: no flinching (NaviStats+0x23 = 1).
function effects.super_armor(): Effect
    return effect({ kind = "super_armor" })
end

-- Custom1, Custom2: the custom screen deals `n` more, at most 8 (+0x0A).
function effects.custom(n: number): Effect
    return effect({ kind = "custom", amount = n })
end

-- MegFldr1, MegFldr2: `n` more Mega chips in the folder, at most 10 (+0x0B).
function effects.mega_folder(n: number): Effect
    return effect({ kind = "mega_folder", amount = n })
end

-- GigFldr1: `n` more Giga chips, at most 10 (+0x0C).
function effects.giga_folder(n: number): Effect
    return effect({ kind = "giga_folder", amount = n })
end

-- FstBarr: the navi enters with this barrier (+0x06).
function effects.first_barrier(barrier: Barrier): Effect
    return effect({ kind = "first_barrier", barrier = barrier })
end

-- Shield, Reflect, AntiDmg: B+Back's weapon (+0x07).
function effects.back_special(weapon: Weapon): Effect
    return effect({ kind = "back_special", weapon = weapon })
end

-- FlotShoe, AirShoes, UnderSht (+0x1B, +0x1C, +0x1D).
function effects.float_shoes(): Effect
    return effect({ kind = "float_shoes" })
end

function effects.air_shoes(): Effect
    return effect({ kind = "air_shoes" })
end

function effects.undershirt(): Effect
    return effect({ kind = "undershirt" })
end

-- EXE6's ChpShufl: the custom screen re-deals (+0x60 = 1); NumbrOpn: it deals ten (+0x61 = 1).
function effects.chip_shuffle(): Effect
    return effect({ kind = "chip_shuffle" })
end

function effects.number_open(): Effect
    return effect({ kind = "number_open" })
end

-- Collect: more chips dropped (+0x26's bit 1).
function effects.collect(): Effect
    return effect({ kind = "collect" })
end

-- What the program does outside battle, to a stat no netbattle reads (the engine doesn't keep it): SneakRun (+0x1E =
-- 1), OilBody, Fish, Battery, Jungle (+0x27 = 2, 4, 8, 0x10), Millions (+0x33 = 1), Humor (+0x25 = 1); EXE6's Poem
-- (+0x5F = 1), SlipRunr (+0x35 = 1), AutoHeal (+0x36 = 1); EXE5's Chivalry (+0x34 = 1), AutoRun (+0x35 = 1), AutoHeal
-- (+0x36 = 1), SoulClen (+0x20 = 1, which the light/dark value's change after a battle reads), MegaVirs and GigaVirs
-- (+0x37, +0x38 = 1), HubBatc's own part (event flag 0x10B1).
function effects.outside(what: string): Effect
    return effect({ kind = "outside", what = what })
end

-- The buster's levels: `n` more, at most 4 (+0x01, +0x02, +0x03), or 4.
function effects.attack(n: number): Effect
    return effect({ kind = "attack", amount = n })
end

function effects.rapid(n: number): Effect
    return effect({ kind = "rapid", amount = n })
end

function effects.charge(n: number): Effect
    return effect({ kind = "charge", amount = n })
end

function effects.attack_max(): Effect
    return effect({ kind = "attack_max" })
end

function effects.rapid_max(): Effect
    return effect({ kind = "rapid_max" })
end

function effects.charge_max(): Effect
    return effect({ kind = "charge_max" })
end

-- BugStop: no bug (+0x1F = 1, which the bugs' routine reads first: EXE6's `sub_813C490`, EXE5's 0x0813FA48).
function effects.bug_stop(): Effect
    return effect({ kind = "bug_stop" })
end

-- Rush, Beat, Tango: a support (+0x0D's bits 0, 1, 2).
function effects.rush(): Effect
    return effect({ kind = "rush" })
end

function effects.beat(): Effect
    return effect({ kind = "beat" })
end

function effects.tango(): Effect
    return effect({ kind = "tango" })
end

-- EXE5's SoulT+1: a soul's `n` more turns, at most 6 (+0x32, which the custom screen adds to a soul's three).
function effects.soul_time(n: number): Effect
    return effect({ kind = "soul_time", amount = n })
end

-- HP+50 to HP+500: `n` more HP (the HP the programs add, EXE6's `Unk2004334`, which the maximum adds to the base:
-- `sub_803CED4`).
function effects.hp(n: number): Effect
    return effect({ kind = "hp", amount = n })
end

return effects
