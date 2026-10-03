--!strict
-- Discord (BN5 chip 0x065): a cut-in chip that sets an instrument in front of the user; while it plays, the other
-- side's navi is confused (lib/instruments/instrument: the shared @common/instruments; BN5's the same code but for the sprite, with its own
-- numbers: 100 HP, `byte_80D4140`'s BN5 counterpart 0x080D5224; it plays 0x55 × 2 ticks and pauses 0x3C, 0x080D5158
-- row 1 and 0x080D53B0).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../../rules/collision")
local effects = require("@common/instruments/effects")
local bn5_instrument = require("../../lib/instruments/instrument")
local instrument = require("@common/instruments/instrument")
local statuses = require("../../rules/status")

local SPRITE = bn5_instrument.sprite

-- Confusion (status 0x23: 4 ticks) on the other navi's panel, learning nothing.
local EFFECT = effects.confuse(statuses.confuse_4, collision.guard_breaking, collision.nothing)

return define.chip {
    id = "bn5:discord",
    codes = { "B", "J", "Q", "*" },
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
    -- BN5's row 1; its identity the actor record 0xDE (0x08014C94: version 5, AI index 0x21).
    dimming = bn5_instrument.hook(instrument.instrument {
        identity = define.identity {
            id = "bn5:discord",
            class = "field_object",
            version = 5,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 0, palette = 2, shadow = true },
        },
        palette = 2,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("bn5:discord"),
        effect = EFFECT,
    }),
}
