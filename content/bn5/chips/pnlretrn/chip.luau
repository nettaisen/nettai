--!strict
-- PnlRetrn (BN5 chip 0x07a): every panel of the user's area back to normal: BN6's panel changer
-- (@bn6/objects/panel-changer, @bn6/lib/panel-chips; BN5's the same code).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local panel_changer = require("@bn6/objects/panel-changer/panel_changer")
local panel_chips = require("@bn6/lib/panel-chips/controller")

return define.chip {
    id = "pnlretrn",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 14,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
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
