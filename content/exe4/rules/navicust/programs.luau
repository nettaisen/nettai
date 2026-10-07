--!strict
-- EXE4's NaviCust programs' effects (docs/design/exe4-map.md §8): a program is the engine's (content/exe4/navicust/,
-- tools/exe4/gen_navicust.py), its effects the values these constructors make, each the work of the original's handler
-- for the program (the table at 0x08041A50, by the part id >> 2); rules/navicust (./init.luau) applies a player's
-- programs to the navi's stats as the round is set up. Most are EXE5's and EXE6's kinds (@exelib/navicust/effects);
-- EXE4's own is BustPack's weapon level. What a program brings when it is bugged is its `bug` (one of `BUGS`).

local effects = require("@exelib/navicust/effects")

local programs = {}

export type Effect = NaviCustEffect

-- SprArmr (+0x01 = 1).
programs.super_armor = effects.super_armor
-- Custom1, Custom2 (+0x12, at most 8).
programs.custom = effects.custom
-- MegFldr1, MegFldr2 (+0x13, at most 10); GigFldr1 (+0x14, at most 10).
programs.mega_folder = effects.mega_folder
programs.giga_folder = effects.giga_folder
-- FstBarr (+0x21 = 1: the barrier the navi enters with).
programs.first_barrier = effects.first_barrier
-- Shield, Reflect, AntiMagc (+0x0C = 0x25, 0x26, 0x27: B+Left's weapon).
programs.back_special = effects.back_special
-- FlotShoe, AirShoes, UnderSht (+0x02, +0x03, +0x04 = 1).
programs.float_shoes = effects.float_shoes
programs.air_shoes = effects.air_shoes
programs.undershirt = effects.undershirt
-- What no netbattle reads: SneakRun (+0x16 = 1), OilBody, Fish, Battery, Jungle (+0x17 = 2, 4, 8, 0x10), Collect
-- (+0x19's bit 1), Humor (+0x1C = 1), SoulClen (+0x1E = 1), HubBatc's own part (event flag 0x0C49).
programs.outside = effects.outside
-- Attack+1, Speed+1, Charge+1 (+0x05, +0x06, +0x07, at most 4); AttckMAX, SpeedMAX, ChargMAX (4).
programs.attack = effects.attack
programs.rapid = effects.rapid
programs.charge = effects.charge
programs.attack_max = effects.attack_max
programs.rapid_max = effects.rapid_max
programs.charge_max = effects.charge_max
-- BugStop (+0x1D = 1, which the bugs' routine reads first: 0x08042A94).
programs.bug_stop = effects.bug_stop
-- Rush, Beat, Tango (+0x18 = 1, 2, 4: one support, the last placed's).
programs.rush = effects.rush
programs.beat = effects.beat
programs.tango = effects.tango
-- HP+50 to HP+500: `n` more HP (the HP the programs add, the toolkit's +0x6C, which the maximum adds to the base:
-- 0x08042FD0).
programs.hp = effects.hp

-- BustPack's weapon level: `n` more, at most 2 (+0x0B, which the charged shot reads: the navi's AI data +8, 0x0800D8D2).
function programs.weapon_level(n: number): Effect
    return { kind = "weapon_level", amount = n }
end

-- The bugs, in the order the bugs' routines run them (0x08042A94 and 0x08042A58, their table 0x08042B18 by bug group
-- and level): a program's `bug` names the one it brings, the colors' count brings `move` and `custom`. Each writes the
-- navi's stats by its level, 1 to 3 (./init.luau). (Group 8, the weapon's: +0x0A = 0x1F, 0x20, 0x21, comes from
-- WeapLV+1 alone, which no save can hold: no part of it has a color. Group 11's routines do nothing.)
programs.BUGS = {
    -- 1: confused at the start (+0x0D = 0xFF: 720 ticks, 0x0800D8B0), or steps of its own, right or left (0x10, 0x20).
    "move",
    -- 2: HP drains a point every 10, 6 or 3 ticks (+0x0E).
    "hp",
    -- 3: the custom screen drains a point every 6, 4 or 3 ticks (+0x0F).
    "custom",
    -- 4: a step leaves the panel cracked, broken or poisoned (+0x1B = 3, 1, 4).
    "panel",
    -- 5: more random encounters (+0x15 = 1).
    "encounter",
    -- 6: the buster fires blanks, 4, 8 or 10 of 16 (+0x08).
    "buster",
    -- 7: the support corrupted (+0x18 = 0xFF).
    "support",
    -- 9: Zenny for chips (+0x19 = 1).
    "result",
    -- 10: HubBatc's: the maximum HP half the base (at least 1) and the HP programs (0x08042EF4).
    "hub",
}

return programs
