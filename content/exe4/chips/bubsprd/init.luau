--!strict
-- BubSprd (EXE4 chip 0x142; its record at 0x080197EC + 0x142 * 0x2C): the Program Advance of three Bubblers, Bub-Vs
-- or BublSides of consecutive codes (the recipe table, the pointer list at 0x080212B0). The spreaders' action 0x1E
-- (./spreader/spreader), its variant 2, the Bubblers': their gun (attachment row 0x0E) fires past the tenth tick,
-- from the ground (height 0), bullet row 7 (its parameter +0x10): a bubble burst over the 3x3 around the first thing
-- it finds.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bubblers = require("./bubbler")
local bullet = require("../objects/bullet")
local regions = require("@exelib/regions")
local spreader = require("./spreader/spreader")

-- The Bubblers' gun (attachment row 0x0E: 0c-1c).
local GUN = attachment.look { sprite = asset.sprite("bubbler"), palette = 0 }
-- Bullet row 7: the bubble burst (effect row 0x25) over the 3x3 around the panel found, its sound 0x9F.
local BURST = bullet.variant {
    region = regions.square,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("bubble-burst"), anim = 0, palette = 0 },
    sound = asset.sound("bubble-burst"),
}

local chip: Chip = {
    action = spreader.action { id = "bubsprd/action", gun = GUN, delay = 10, z = 0, shot = BURST },
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 300,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage" },
    program_advances = {
        { order = 9, code_run = { chip = bubblers.bubbler, count = 3 } },
        { order = 11, code_run = { chip = bubblers["bub-v"], count = 3 } },
        { order = 13, code_run = { chip = bubblers.bublside, count = 3 } },
    },
}

return { bubsprd = chip }
