--!strict
-- Hole (EXE4 chip 0x052): the panel in front of the user becomes a hole: the panel chips' controller (lib/panel_chips,
-- the dimming chips' variant 0x0B) with the panel changer's row 9: the panel in front, whatever its side, shown as a
-- hole for 64 ticks without flickering, the hole's start sound and no other.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips")

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 23,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = panel_chips.hook(panel_changer.change {
        panel = "hole",
        panels = "front",
        flicker = false,
        ticks = 0x40,
        sides = panel_changer.ANY,
        start_sound = panel_changer.START_SOUND,
    }),
}

return { hole = chip }
