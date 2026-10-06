--!strict
-- BodyGrd, the Program Advance of AntiNavi, AntiSwrd and AntiDmg: a hidden
-- trap that swallows the next hit of 10 damage or more and answers with
-- its counter (the damage intake's `sub_802CEF4`; the counter is the role
-- `actions.body_guard_counter`). Its controller is the trap chips'
-- (lib/traps/controller); it sets no object of its own.

local traps = require("../lib/traps/controller")
local antidmg = require("./antidmg").antidmg
local antinavi = require("./antinavi").antinavi
local antiswrd = require("./antiswrd").antiswrd

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = { { order = 3, sequence = { antinavi, antiswrd, antidmg } } },
    trap = "body_guard",
    dimming = traps.hook(),
}

return { bodygrd = chip }
