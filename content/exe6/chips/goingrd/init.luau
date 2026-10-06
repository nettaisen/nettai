--!strict
-- GoingRd: a cut-in chip that lays roads away from the user over the
-- other side's panels of its row (`byte_80E272C` row 0x12; on side 1 row
-- 0x11, whose roads point the other way: `sub_80E28C8` swaps them).

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips/controller")

return define.chip {
    id = "goingrd",
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 21, damage = 40, hit_param = 30, lockout = 0,
    flags = { "dimming", "library" },
    library = { number = 173, index = 173, sort = 216 },
    slot_in_limit = 1,
    dimming = panel_chips.hook(panel_changer.change {
        panel = "road_right",
        side1_panel = "road_left",
        panels = "row",
        flicker = true,
        ticks = 0x40,
        sides = panel_chips.OTHER_AREA,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}
