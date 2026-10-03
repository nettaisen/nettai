--!strict
-- PnlRetrn: a cut-in chip that turns the user's area back to normal
-- panels (`byte_80E272C` row 0).

local panel_changer = require("../../objects/panel-changer/panel_changer")
local panel_chips = require("../../lib/panel-chips/controller")

return define.chip {
    id = "bn6:pnlretrn",
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 1, mb = 14, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    library = { number = 168, index = 168, sort = 326 },
    slot_in_limit = 1,
    dimming = panel_chips.hook(panel_changer.change {
        panel = "normal",
        panels = "all",
        flicker = true,
        ticks = 0x40,
        sides = panel_chips.OWN_AREA,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}
