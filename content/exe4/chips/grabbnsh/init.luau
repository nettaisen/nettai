--!strict
-- GrabRvng and GrabBnsh (EXE4 chips 0x076 and 0x077; their records at 0x080197EC + n * 0x2C): the dimming chips'
-- action 0x0C, its variant 0x0D (0x080E4814): the user's side counts the panels of its home the other side holds, and
-- a hand drops on every enemy navi for each (./controller: @exelib/grabbnsh with EXE4's look and taking back).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local controller = require("@self/controller")

local grabrvng: Chip = {
    dimming = controller.hook,
    codes = { "D", "J", "N" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 48,
    damage = 50,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
}

local grabbnsh: Chip = {
    dimming = controller.hook,
    codes = { "E", "K", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 30,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
}

return { grabrvng = grabrvng, grabbnsh = grabbnsh }
