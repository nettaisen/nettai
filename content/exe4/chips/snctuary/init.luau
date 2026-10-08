--!strict
-- Snctuary (EXE4 chip 0x0D0): every panel of the user's area turns holy: the panel chips' controller
-- (lib/panel_chips, the dimming chips' variant 0x0B) with the panel changer's row 7: the user's own area, shown as
-- holy for 64 ticks, flickering, with the tick and change sounds.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips")

local chip: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 62,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = panel_chips.hook(panel_changer.change {
        panel = "holy",
        panels = "all",
        flicker = true,
        ticks = 0x40,
        sides = panel_changer.OWN_AREA,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}

return { snctuary = chip }
