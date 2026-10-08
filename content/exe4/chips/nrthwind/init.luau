--!strict
-- NrthWind (EXE4 chip 0x08A): a north wind blows over the field for 100 ticks, then strips every barrier off it: the
-- dimming chips' action 0x0C, its variant 0x1C (0x080220E0's), whose controller (effect object #0x39, 0x080E622C:
-- EXE6's and EXE5's code, `sub_80E47D4`: @exelib/nrthwind/controller) blows EXE4's north wind
-- (../../objects/north_wind, WindSoul's change's too) and waits for it. Its spawner (0x080E629C, the chip's `dimming`
-- hook) copies the user's side, not its facing. (Its power, 40, nothing reads.) Its record is written by
-- tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("@exelib/nrthwind/controller")
local north_wind = require("../objects/north_wind")

-- 0x080E622C, spawned by 0x080E629C.
local DIMMING = controller.make { id = "nrthwind/controller", wind = north_wind, side_only = true }

local chip: Chip = {
    codes = { "E", "N", "T", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = DIMMING,
}

return { nrthwind = chip }
