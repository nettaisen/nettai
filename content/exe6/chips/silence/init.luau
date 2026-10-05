--!strict
-- Silence: a cut-in chip that sets an instrument in front of the user;
-- while it plays, the other side is blinded (lib/instruments).

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local instrument = require("../lib/instruments/instrument")
local instruments = require("../lib/instruments/controller")
local statuses = require("../rules/status")

-- Blinded (status 0x33: 4 ticks) over the whole field (`sub_80D43E8`).
local hush = effects.hush(statuses.blind_4, collision.guard_breaking, collision.hits_navis)

return define.chip {
    id = "silence",
    codes = { "B", "R", "W", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 152, index = 152, sort = 407 },
    slot_in_limit = 3,
    -- `byte_80D4078` row 3.
    dimming = instruments.hook(instrument.instrument {
        identity = define.identity {
            id = "silence",
            class = "field_object",
            version = 1,
            ai_index = 1,
            look = { sprite = asset.sprite("instrument"), anim = 0, palette = 6, shadow = true },
        },
        palette = 6,
        play_ticks = 0x78 * 2,
        pause_ticks = 0x1E,
        tune = asset.sound("silence"),
        effect = hush,
    }),
}
