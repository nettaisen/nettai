--!strict
-- DrkLance (EXE5 chip 0x0bf): a dark chip: Lance's three lances (instant chip effect 4) with its Param1 1, in their
-- second palette with the bug 0x16 (chips/lance/lance). Its use costs a hit status (rules/light_dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local exe5_lance = require("./lance/lance")
local lance = require("@exelib/lance")
local sparks = require("../lib/sparks")
local dark_costs = require("../lib/dark_costs")

-- Its hit (0x080CF904's row 1): EXE5's row 0x25 against attacks, hit modifier 0x10, the wood spark, with bug 0x16
-- (argument 2).
local HIT = lance.hit {
    is = collision.attack_480,
    hits = collision.attack,
    spark = sparks.wood,
    hit_mod = 0x10,
    bug = 0x16,
    bug_arg = 2,
}

-- Its cost: the hit status bug (2).
local DARK_COST = dark_costs.cost(function(stats: Navi)
    stats.hit_status = 2
end)

local chip: Chip = {
    codes = { "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 400,
    hit_param = 158,
    lockout = 20,
    flags = { "has_damage", "dark", "variable_damage" },
    slot_in_limit = 1,
    instant = lance.instant_of(exe5_lance.kind, 1, HIT),
    -- What its use costs its user (rules/light_dark).
    dark_cost = DARK_COST,
    auto_battle = { class = "at_once" },
}

return { drklance = chip }
