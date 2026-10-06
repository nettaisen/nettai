--!strict
-- HolyPanl: a cut-in chip that makes the panel in front of the user holy
-- (`byte_80E272C` row 4).

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips/controller")

return define.chip {
    id = "holypanl",
    codes = { "A", "B", "S", "*" },
    element = "null", family = "null", class = "standard",
    rarity = 1, mb = 24, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
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
