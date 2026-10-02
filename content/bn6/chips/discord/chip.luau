--!strict
-- Discord: a cut-in chip that sets an instrument in front of the user;
-- while it plays, the other side's navi is confused (lib/instruments).

local collision = require("../../rules/collision")
local effects = require("../../lib/instruments/effects")
local instrument = require("../../lib/instruments/instrument")
local instruments = require("../../lib/instruments/controller")
local statuses = require("../../rules/status")

-- Confusion (status 0x23: 4 ticks) on the other navi's panel, learning
-- nothing (`sub_80D4392`).
local confuse = effects.confuse(statuses.confuse_4, collision.guard_breaking, collision.nothing)

return define.chip {
    id = "discord",
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
