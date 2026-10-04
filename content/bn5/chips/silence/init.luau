--!strict
-- Silence (BN5 chip 0x007): a cut-in chip that sets an instrument in front of the user; while it plays, the other
-- side is blinded (lib/instruments/instrument: the shared @exelib/instruments; BN5's the same code but for the sprite, with its own numbers: 100
-- HP, `byte_80D4140`'s BN5 counterpart 0x080D5224; it plays 0x55 × 2 ticks and pauses 0x3C, 0x080D5158 row 3 and
-- 0x080D53B0).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local bn5_instrument = require("../lib/instruments/instrument")
local instrument = require("@exelib/instruments/instrument")
local statuses = require("../rules/status")

local SPRITE = bn5_instrument.sprite

-- Blinded (status 0x33: 4 ticks) over the whole field.
local EFFECT = effects.hush(statuses.blind_4, collision.guard_breaking, collision.hits_navis)

return define.chip {
    id = "silence",
    codes = { "C", "J", "N", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 3,
    -- BN5's row 3; its identity the actor record 0xE0 (0x08014C94: version 1, AI index 0x21).
    dimming = bn5_instrument.hook(instrument.instrument {
        identity = define.identity {
            id = "silence",
            class = "field_object",
            version = 1,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 0, palette = 6, shadow = true },
        },
        palette = 6,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("silence"),
        effect = EFFECT,
    }),
}
