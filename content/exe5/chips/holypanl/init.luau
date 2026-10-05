--!strict
-- HolyPanl (EXE5 chip 0x08b): the panels in front of the user turn holy: EXE6's panel changer
-- (@exelib/panel-changer, @exelib/panel-chips; EXE5's the same code).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local panel_changer = require("../objects/panel-changer")
local panel_chips = require("../lib/panel_chips")

return define.chip {
    id = "holypanl",
    codes = { "A", "N", "U", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 24,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    megaman = "light",
    dimming = panel_chips.hook(panel_changer.change {
        panel = "holy",
        panels = "front",
        flicker = true,
        ticks = 0x40,
        sides = panel_chips.ANY,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}
