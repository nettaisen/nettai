--!strict
-- ProtoMan's charged slash (`protoman/charge`): the original's weapon routine 0x1E (0x0800F8CE). WideSwrd loaded as the
-- attack (0x0800FE78: the chip's record's element, lockout and params; no chip is spent), then the damage his level
-- gives (the damage rows' row 0, lib/navi_level) and his own counter byte; it starts the chip's use (EXE5's action 0x13
-- as its subtype 1, chips/wideswrd).

local navi_level = require("../../lib/navi_level")
local wideswrd = require("../../chips/wideswrd").wideswrd

local charge = {}

local HIT_PARAM = 0x94
-- The damage rows' row 0, by his level.
local DAMAGE: { number } = { 60, 60, 70, 70, 80, 80, 80 }

-- 0x0800F8CE
local function setup(navi: Object): Chip
    navi:load_chip_attack(wideswrd)
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    return wideswrd
end

charge.weapon = new.weapon {
    id = "protoman/charge",
    -- The charge table's row 0x1E (0x0801CB98), by Charge 0 to 5: past 4 it reads on into row 0x1F's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 40 },
    setup = setup,
}

return charge
