--!strict
-- WindSoul (EXE4 chip 0x163, its record at 0x0801d4f0), no folder's: Double Soul's chip of soul 3, which OK puts
-- first in the hand for it (0x0801F0A0: the UNITE button's `hand_chip`, rules/souls/custom). With it next in the hand
-- MegaMan changes into the soul as the fight runs: its use is the soul's change (its action 0x0D: rules/souls/change),
-- whose end takes the hand on past it (0x080EBC92).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local change = require("../../../../rules/souls/change")

local windsoul: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = {},
    -- Next in the hand it shows no icon over MegaMan (0x0801502E: chips 0x160 to 0x16F).
    traits = { "no_icon_when_next" },
    action = change.make {
        id = "windsoul",
    },
}

return { windsoul = windsoul }
