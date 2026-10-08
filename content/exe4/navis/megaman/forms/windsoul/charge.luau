--!strict
-- WindSoul's charged shot (`windsoul/charged-shot`): weapon routine 0x09 (0x0800D204): WindRack's attack (chip 0x035,
-- loaded as 0x0800D406 loads a chip) with 50 damage and counter byte 0x9E, and WindRack's use (its action 0x32).

local windrack = require("../../../../chips/windrack").windrack

local charge = {}

-- 0x0800D204.
local function setup(navi: Object): Chip
    navi:load_chip_attack(windrack)
    navi.attack_damage = 50
    navi.hit_param = 0x9E
    return windrack
end

charge.weapon = new.weapon {
    id = "windsoul/charged-shot",
    -- The charge table's row 0x09 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x0A's
    -- first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 100 },
    setup = setup,
}

return charge
