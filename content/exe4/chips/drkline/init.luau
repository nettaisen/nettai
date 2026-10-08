--!strict
-- DrkLine (EXE4 chip 0x0D8): the user's row becomes holes: the panel chips' controller (lib/panel_chips, the dimming
-- chips' variant 0x0B) with the panel changer's row 10: the panels of the user's row, whatever their side, shown as
-- holes for 64 ticks without flickering, the hole's start sound and no other.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips")

local chip: Chip = {
    codes = { "L", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 59,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = panel_chips.hook(panel_changer.change {
        panel = "hole",
        panels = "row",
        flicker = false,
        ticks = 0x40,
        sides = panel_changer.ANY,
        start_sound = panel_changer.START_SOUND,
    }),
}

return { drkline = chip }
