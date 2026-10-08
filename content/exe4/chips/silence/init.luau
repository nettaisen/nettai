--!strict
-- Silence (EXE4 chip 0x007): a cut-in chip that sets an instrument in front of the user; while it plays, the other
-- side is blinded (lib/instruments: the shared @exelib/instruments with EXE4's look; its row 3 of 0x080DC32C, its tune
-- 0x12D, its effect 0x080DC62C: EXE6's `sub_80D43E8`).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local instrument = require("@exelib/instruments/instrument")
local instruments = require("../lib/instruments")
local statuses = require("../rules/status")

-- Blindness (status 0x33: 4 ticks) over the whole field.
local EFFECT = effects.hush(statuses.blind_4, collision.guard_breaking, collision.hits_navis)

local chip: Chip = {
    codes = { "C", "M", "R", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 20,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = instruments.hook(instrument.instrument {
        palette = 6,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("silence"),
        effect = EFFECT,
    }),
}

return { silence = chip }
