--!strict
-- DeltaRay (EXE4 chip 0x12E): a navi chip in all but its navi flag (AntiNavi lets it through), the dimming chips'
-- action 0x0C, its variant 0x32: the shared navi chip controller (objects/navi_chip_controller, the chip's third
-- parameter 0xC) brings DeltaRay (./navi), who strikes an enemy navi at the delta's corners, once more each time his
-- user presses A during a strike.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("../objects/navi_chip_controller")

local chip: Chip = {
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "giga",
    rarity = 4,
    mb = 82,
    damage = 260,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    -- The shared controller's DeltaRay (the chip's third parameter).
    dimming = controller.hook_for(0, 0xC),
}

return { deltaray = chip }
