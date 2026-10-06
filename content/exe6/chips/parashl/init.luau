--!strict
-- ParaShl (IronShl's Program Advance): IronShl's throw
-- (chips/ironshl/throw.luau) of three shells down the navi's own back
-- column, which slide four panels, bump and slide back.

local throw = require("./ironshl/throw")
local ironshl_chips = require("./ironshl")

-- Its shells' speed (`byte_80CA7A0`'s third), 16.16 a tick.
local SPEED = 0x8_0000

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 350,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    program_advances = {
        { order = 37, code_run = { chip = ironshl_chips.ironshl1, count = 3 } },
        { order = 38, code_run = { chip = ironshl_chips.ironshl2, count = 3 } },
        { order = 39, code_run = { chip = ironshl_chips.ironshl3, count = 3 } },
    },
    action = throw.action { id = "parashl/action", shell = { palette = 0, speed = SPEED, bumps = 1, para = true } },
}

return { parashl = chip }
