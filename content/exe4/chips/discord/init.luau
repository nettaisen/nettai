--!strict
-- Discord (EXE4 chip 0x065): a cut-in chip that sets an instrument in front of the user; while it plays, the other
-- side's navi is confused (lib/instruments: the shared @exelib/instruments with EXE4's look; its row 1 of 0x080DC32C,
-- its tune 0x12B, its effect 0x080DC5D8: EXE6's `sub_80D4392`).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local effects = require("@exelib/instruments/effects")
local instrument = require("@exelib/instruments/instrument")
local instruments = require("../lib/instruments")
local statuses = require("../rules/status")

-- Confusion (status 0x23: 4 ticks) on the other navi's panel, learning nothing.
local EFFECT = effects.confuse(statuses.confuse_4, collision.guard_breaking, collision.nothing)

local chip: Chip = {
    codes = { "D", "T", "V", "*" },
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
        palette = 2,
        play_ticks = 0x55 * 2,
        pause_ticks = 0x3C,
        tune = asset.sound("discord"),
        effect = EFFECT,
    }),
}

return { discord = chip }
