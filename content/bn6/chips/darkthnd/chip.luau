--!strict
-- DarkThnd: Thunder's shot (chips/thunder) with a big ball that homes in
-- for 12 panels.

local shoot = require("../thunder/shoot")
local lockon = require("../../rules/lockon")
local thunder = require("../thunder/chip")

-- Its hits' status: paralysis.
local PARALYZE = 0x10

return define.chip {
    id = "darkthnd",
    name = "DarkThnd",
    description = "\n\n",
    codes = { "A", "B", "C", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 4,
    mb = 80,
    damage = 200,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage" },
    extra_flags = { 16, 32 },
    beast = { lockon = lockon.firebrn1 },
    library = { number = 0, index = 67, sort = 0 },
    slot_in_limit = 1,
    dark_substitute = thunder,
    hp_bug = 1,
    action = shoot.action { id = "darkthnd/action", ball = { panels = 0x0C, status = PARALYZE } },
}
