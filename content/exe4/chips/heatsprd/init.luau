--!strict
-- HeatSprd (EXE4 chip 0x141; its record at 0x080197EC + 0x141 * 0x2C): the Program Advance of HeatShot, Heat-V and
-- HeatSide, or three of one of them of consecutive codes (the recipe table, the pointer list at 0x080212B0). The
-- spreaders' action 0x1E (./spreader/spreader), its variant 1, the HeatShots': their gun (attachment row 6) fires
-- past the tenth tick, at height 0x10, bullet row 0x0B (its parameter +0x10): a fire burst over the 3x3 around the
-- first thing it finds.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")
local heatshots = require("./heatshot")
local regions = require("@exelib/regions")
local spreader = require("./spreader/spreader")

-- The HeatShots' gun (attachment row 6: 0c-0d).
local GUN = attachment.look { sprite = asset.sprite("heatshot"), palette = 0 }
-- Bullet row 0x0B: the fire burst (effect row 0x29) over the 3x3 around the panel found, its sound 0xBC.
local BURST = bullet.variant {
    region = regions.square,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("heat-burst"), anim = 0, palette = 0 },
    sound = asset.sound("heat-burst"),
}

local chip: Chip = {
    action = spreader.action { id = "heatsprd/action", gun = GUN, delay = 10, z = 0x10, shot = BURST },
    codes = { "A", "B", "C", "*" },
    element = "fire",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 4, sequence = { heatshots.heatshot, heatshots["heat-v"], heatshots.heatside } },
        { order = 8, code_run = { chip = heatshots.heatshot, count = 3 } },
        { order = 10, code_run = { chip = heatshots["heat-v"], count = 3 } },
        { order = 12, code_run = { chip = heatshots.heatside, count = 3 } },
    },
}

return { heatsprd = chip }
