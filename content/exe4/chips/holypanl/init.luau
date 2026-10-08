--!strict
-- HolyPanl (EXE4 chip 0x08B): the panel in front of the user turns holy: the panel chips' controller
-- (lib/panel_chips, the dimming chips' variant 0x0B) with the panel changer's row 6: the panel in front, whatever its
-- side, shown as holy for 64 ticks, flickering, with the tick and change sounds.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips")

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = panel_chips.hook(panel_changer.change {
        panel = "holy",
        panels = "front",
        flicker = true,
        ticks = 0x40,
        sides = panel_changer.ANY,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}

return { holypanl = chip }
