--!strict
-- ProtoSoul's charged shot (`protosoul/charged-shot`): weapon routine 0x3E (0x0800D148): WideSwrd's attack (chip
-- 0x031, loaded as 0x0800D406 loads a chip) and WideSwrd's use (the swords' action 0x0A, its variant 1).

local wideswrd = require("../../../../chips/sword").wideswrd

local charge = {}

-- 0x0800D148.
local function setup(navi: Object): Chip
    navi:load_chip_attack(wideswrd)
    return wideswrd
end

charge.weapon = new.weapon {
    id = "protosoul/charged-shot",
    -- The charge table's row 0x3E (0x080180B8 by AIData +0x0F), by Charge 0 to 4; Charge 5 reads on into row 0x3F's
    -- first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 60 },
    setup = setup,
}

return charge
