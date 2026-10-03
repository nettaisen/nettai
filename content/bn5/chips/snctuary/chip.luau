--!strict
-- Snctuary (BN5 chip 0x0d0): every panel of the user's area turns holy: BN6's panel changer
-- (@bn6/objects/panel-changer, @bn6/lib/panel-chips; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local panel_changer = require("../../objects/panel-changer/panel_changer")
local panel_chips = require("../../lib/panel_chips")

return define.chip {
    id = "bn5:snctuary",
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
}
