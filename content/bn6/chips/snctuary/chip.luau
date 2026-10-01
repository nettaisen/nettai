--!strict
-- Snctuary: a cut-in chip that makes the user's whole area holy
-- (`byte_80E272C` row 5).

local panel_changer = require("../../objects/panel-changer/panel_changer")
local panel_chips = require("../../lib/panel-chips/controller")

return define.chip {
    id = "snctuary",
    name = "Snctuary",
    codes = { "Z" },
    element = "null", family = "null", class = "standard",
    rarity = 4, mb = 62, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 171, index = 171, sort = 413 },
    slot_in_limit = 1,
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
