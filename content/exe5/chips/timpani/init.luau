--!strict
-- Timpani (EXE5 chip 0x066): a cut-in chip that sets an instrument in front of the user; while it plays, everything
-- on a solid panel of the other side is held in place and the screen shakes (lib/instruments/instrument: the shared @exelib/instruments; EXE5's the
-- same code but for the sprite, with its own numbers: 100 HP, `byte_80D4140`'s EXE5 counterpart 0x080D5224; it plays
-- 0x55 × 2 ticks and pauses 0x3C, 0x080D5158 row 2 and 0x080D53B0).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local exe5_instrument = require("../lib/instruments/instrument")
local instrument = require("@exelib/instruments/instrument")
local statuses = require("../rules/status")

local SPRITE = exe5_instrument.sprite

-- Held in place (status 0x43: 4 ticks) over the solid panels, and a camera shake.
local EFFECT = effects.drum(statuses.immobilize_4, collision.guard_breaking, collision.hits_navis)

return define.chip {
    id = "timpani",
    codes = { "A", "L", "V", "*" },
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
    -- EXE5's row 2; its identity the actor record 0xDF (0x08014C94: version 0, AI index 0x21).
    dimming = exe5_instrument.hook(instrument.instrument {
        identity = define.identity {
            class = "field_object",
            version = 0,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 0, palette = 4, shadow = true },
        },
        palette = 4,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("timpani"),
        effect = EFFECT,
    }),
}
