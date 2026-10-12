--!strict
-- JunkSoul's charged shot (`junksoul/charged-shot`): weapon routine 0x61 (0x0800CE4C): Poltrgst's attack (chip 0x17D,
-- loaded as 0x0800D406 loads a chip: 100 damage) with the counter byte 0x94, and Poltrgst's use (the dimming chips'
-- action 0x0C: chips/poltrgst): MegaMan's stand-in raises the poltergeist, every obstacle on the field thrown at the
-- other side.

local poltrgst = require("../../../../chips/poltrgst").poltrgst

local charge = {}

-- 0x0800CE4C.
local function setup(navi: Object): Chip
    navi:load_chip_attack(poltrgst)
    navi.hit_param = 0x94
    return poltrgst
end

charge.weapon = new.weapon {
    id = "junksoul/charged-shot",
    -- The charge table's row 0x61 (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x62's
    -- first entry.
    charge_ticks = { 80, 80, 80, 80, 80, 60 },
    setup = setup,
}

return charge
