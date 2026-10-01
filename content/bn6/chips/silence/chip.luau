--!strict
-- Silence: a cut-in chip that sets an instrument in front of the user;
-- while it plays, the other side is blinded (lib/instruments).

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local instrument = require("../../lib/instruments/instrument")
local instruments = require("../../lib/instruments/controller")

-- Blinded (status 0x33: 4 ticks), with the sixth element's damage 0.
local BLIND, ELEMENT_NONE = 0x33, 5

-- `sub_80D43E8`: a one-tick hit over the whole field that blinds the
-- other side.
local function hush(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = ELEMENT_NONE,
        z = 0,
        region = regions.field,
        -- None.
        hit_effect = 0xFF,
        target = collision.hits_navis,
        self_type = collision.guard_breaking,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = BLIND,
    })
end

return define.chip {
    id = "silence",
    name = "Silence",
    codes = { "B", "R", "W", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 152, index = 152, sort = 407 },
    slot_in_limit = 3,
    -- `byte_80D4078` row 3.
    dimming = instruments.hook(instrument.instrument {
        identity = 0xE0,
        palette = 6,
        play_ticks = 0x78 * 2,
        pause_ticks = 0x1E,
        tune = asset.sound("silence"),
        effect = hush,
    }),
}
