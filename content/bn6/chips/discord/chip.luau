--!strict
-- Discord: a cut-in chip that sets an instrument in front of the user;
-- while it plays, the other side's navi is confused (lib/instruments).

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local instrument = require("../../lib/instruments/instrument")
local instruments = require("../../lib/instruments/controller")
local statuses = require("../../rules/status")

-- Confusion (status 0x23: 4 ticks), with the sixth element's damage 0.
local CONFUSE, ELEMENT_NONE = statuses.confuse_4, 5

-- `sub_80D4392`: a one-tick hit on the other navi's panel that confuses
-- it, and learns nothing.
local function confuse(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = ELEMENT_NONE,
        z = 0,
        region = regions.other_navi[me.alliance],
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
    description = "Confuses \nenemies\nw/music",
    codes = { "A", "G", "S", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 150, index = 150, sort = 121 },
    slot_in_limit = 3,
    -- `byte_80D4078` row 1.
    dimming = instruments.hook(instrument.instrument {
        identity = define.identity {
            id = "discord",
            class = "field_object",
            version = 5,
            ai_index = 1,
            look = { sprite = asset.sprite("instrument"), anim = 0, palette = 2, shadow = true },
        },
        palette = 2,
        play_ticks = 0x37 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("discord"),
        effect = confuse,
    }),
}
