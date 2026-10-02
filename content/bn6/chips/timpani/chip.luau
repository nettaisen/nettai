--!strict
-- Timpani: a cut-in chip that sets a drum in front of the user; while it
-- plays, everything on a solid panel of the other side is held in place
-- and the screen shakes (lib/instruments).

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local instrument = require("../../lib/instruments/instrument")
local instruments = require("../../lib/instruments/controller")
local statuses = require("../../rules/status")

-- Held in place (status 0x43: 4 ticks), with the sixth element's damage 0.
local IMMOBILIZE, ELEMENT_NONE = statuses.immobilize_4, 5

-- `sub_80D43C0`: a one-tick hit over the solid panels that immobilizes
-- the other side, and a camera shake.
local function drum(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = ELEMENT_NONE,
        z = 0,
        region = regions.solid,
        -- None.
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.guard_breaking,
        damage = 0,
        stamina = 0,
        hit_mod = 0,
        status = IMMOBILIZE,
    })
    battle.shake_camera(2, 4)
end

return define.chip {
    id = "timpani",
    codes = { "I", "O", "T", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 151, index = 151, sort = 437 },
    slot_in_limit = 3,
    -- `byte_80D4078` row 2.
    dimming = instruments.hook(instrument.instrument {
        identity = define.identity {
            id = "timpani",
            class = "field_object",
            version = 0,
            ai_index = 1,
            look = { sprite = asset.sprite("instrument"), anim = 0, palette = 4, shadow = true },
        },
        palette = 4,
        play_ticks = 0x3C * 2,
        pause_ticks = 0xAA,
        tune = asset.sound("timpani"),
        effect = drum,
    }),
}
