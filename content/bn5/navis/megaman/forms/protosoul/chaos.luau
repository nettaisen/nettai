--!strict
-- ProtoSoul's Chaos Unison weapon (`protosoul/chaos`): the original's weapon routine 6 (0x0800F668), AIData +0x11's
-- in ProtoSoul: DrkSword (chip 0xBC) loaded as the attack (0x0800FE78, BN6's `sub_8010D04`'s place: the chip's
-- record's damage, hit parameter, lockout and element) and the chip's use. An armed full B charge released in the
-- chaos cycle's window fires it; the charge's time is its row's by the chaos level.

local drksword = require("../../../../chips/drksword/chip")

local chaos = {}

-- 0x0800F668
local function setup(navi: Object): Chip
    navi:load_chip_attack(drksword)
    return drksword
end

chaos.weapon = define.weapon {
    id = "bn5:protosoul/chaos",
    -- The charge table's row 6 (0x0801CAA8), by the chaos level (at most 4); level 5 would read on into row 7's
    -- first entry.
    charge_ticks = { 40, 60, 90, 120, 150, 130 },
    setup = setup,
}

return chaos
