--!strict
-- PnlRetrn (EXE4 chip 0x07A): every panel of the user's area back to normal: the panel chips' controller
-- (lib/panel_chips, the dimming chips' variant 0x0B) with the panel changer's row 0: the user's own area, shown as
-- normal for 64 ticks, flickering, with the tick and change sounds.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips")

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 14,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = panel_chips.hook(panel_changer.change {
        panel = "normal",
        panels = "all",
        flicker = true,
        ticks = 0x40,
        sides = panel_changer.OWN_AREA,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}

return { pnlretrn = chip }
