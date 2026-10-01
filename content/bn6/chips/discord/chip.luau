--!strict
-- Discord: a cut-in chip that sets an instrument in front of the user;
-- while it plays, the other side's navi is confused (lib/instruments).

local collision = require("../../rules/collision")
local instrument = require("../../lib/instruments/instrument")
local instruments = require("../../lib/instruments/controller")

-- The other side's player navi's panels (whole-field regions 0x85 for side
-- 0, 0x84 for side 1).
local OTHER_NAVI = {
    [0] = define.region { field = { require = 0x0020_0000, forbid = 0 } },
    [1] = define.region { field = { require = 0x0040_0000, forbid = 0 } },
}
-- Confusion (status 0x23: 4 ticks), with the sixth element's damage 0.
local CONFUSE, ELEMENT_NONE = 0x23, 5

-- `sub_80D4392`: a one-tick hit on the other navi's panel that confuses
-- it, and learns nothing.
local function confuse(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = ELEMENT_NONE,
        z = 0,
        region = OTHER_NAVI[me.alliance],
        -- None.
        hit_effect = 0xFF,
        target = collision.nothing,
        self_type = collision.guard_breaking,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = CONFUSE,
    })
end

return define.chip {
    id = "discord",
    name = "Discord",
    codes = { "A", "G", "S", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 150, index = 150, sort = 121 },
    slot_in_limit = 3,
    -- `byte_80D4078` row 1.
    dimming = instruments.hook(instrument.instrument {
        identity = 0xDE,
        palette = 2,
        play_ticks = 0x37 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("discord"),
        effect = confuse,
    }),
}
