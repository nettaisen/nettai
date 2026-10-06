--!strict
-- Timpani: a cut-in chip that sets a drum in front of the user; while it
-- plays, everything on a solid panel of the other side is held in place
-- and the screen shakes (lib/instruments).

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local instrument = require("../lib/instruments/instrument")
local instruments = require("../lib/instruments/controller")
local statuses = require("../rules/status")

-- Held in place (status 0x43: 4 ticks) over the solid panels, and a
-- camera shake (`sub_80D43C0`).
local drum = effects.drum(statuses.immobilize_4, collision.guard_breaking, collision.hits_navis)

local chip: Chip = {
    codes = { "I", "O", "T", "*" },
    element = "null", family = "summon", class = "standard",
    rarity = 1, mb = 20, damage = 0, hit_param = 0, lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 3,
    -- `byte_80D4078` row 2.
    dimming = instruments.hook(instrument.instrument {
        identity = new.identity {
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

return { timpani = chip }
