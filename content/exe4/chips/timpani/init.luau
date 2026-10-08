--!strict
-- Timpani (EXE4 chip 0x066): a cut-in chip that sets an instrument in front of the user; while it plays, the other
-- side can't move (lib/instruments: the shared @exelib/instruments with EXE4's look; its row 2 of 0x080DC32C, its tune
-- 0x12C, its effect 0x080DC604: EXE6's `sub_80D43C0`).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local instrument = require("@exelib/instruments/instrument")
local instruments = require("../lib/instruments")
local statuses = require("../rules/status")

-- Immobilization (status 0x43: 4 ticks) over the solid panels, and a camera shake.
local EFFECT = effects.drum(statuses.immobilize_4, collision.guard_breaking, collision.hits_navis)

local chip: Chip = {
    codes = { "P", "T", "Z", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = instruments.hook(instrument.instrument {
        palette = 4,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("timpani"),
        effect = EFFECT,
    }),
}

return { timpani = chip }
