--!strict
-- LifeSync (EXE4 chip 0x01C): the dimming chips' variant 0x30, a cut-in chip that, outside link battles, sets the HP
-- of the other console's navi to that of the enemy two panels ahead: EXE4's controller (controller.luau, effect object
-- #0x5C, 0x080E8784, which EXE5's reworks) with the shared marker (@exelib/lifesync/marker) in EXE4's look. In a
-- netbattle it only dims.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local lifesync = require("@self/controller")

local chip: Chip = {
    codes = { "N", "Q", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = lifesync.hook,
}

return { lifesync = chip }
