--!strict
-- Snctuary (EXE5 chip 0x0d0): every panel of the user's area turns holy: EXE6's panel changer
-- (@exelib/panel_changer, @exelib/panel_chips; EXE5's the same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips")

local chip: Chip = {
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 62,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    megaman = "light",
    dimming = panel_chips.hook(panel_changer.change {
        panel = "holy",
        panels = "all",
        flicker = true,
        ticks = 0x40,
        sides = panel_chips.OWN_AREA,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
    auto_battle = { class = "at_once" },
}

return { snctuary = chip }
