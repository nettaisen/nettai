--!strict
-- ComingRd: a cut-in chip that lays roads toward the user over the other
-- side's panels of its row (`byte_80E272C` row 0x11; on side 1 row 0x12,
-- whose roads point the other way: `sub_80E28C8` swaps them).

local panel_changer = require("../objects/panel_changer")
local panel_chips = require("../lib/panel_chips/controller")

local chip: Chip = {
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 0, mb = 21, damage = 40, hit_param = 30, lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    dimming = panel_chips.hook(panel_changer.change {
        panel = "road_left",
        side1_panel = "road_right",
        panels = "row",
        flicker = true,
        ticks = 0x40,
        sides = panel_chips.OTHER_AREA,
        tick_sound = panel_changer.TICK_SOUND,
        change_sound = panel_changer.CHANGE_SOUND,
    }),
}

return { comingrd = chip }
