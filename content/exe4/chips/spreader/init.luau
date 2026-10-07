--!strict
-- Spreader (EXE4 chip 0x012; its record at 0x080197EC + 0x12 * 0x2C): the spreaders' action 0x1E (./spreader), its
-- variant 0: the gun (attachment row 0x0A) fires past the first tick, at height 0x10, bullet row 3 (its parameter
-- +0x10): a burst over the 3x3 around the first thing it finds.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")
local regions = require("@exelib/regions")
local spreader = require("@self/spreader")

-- Its gun (attachment row 0x0A: 0c-19).
local GUN = attachment.look { sprite = asset.sprite("spreader"), palette = 0 }
-- Bullet row 3: a burst (effect row 0x27) over the 3x3 around the panel found; sound 0x8D.
local BURST = bullet.variant {
    region = regions.square,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 },
    sound = asset.sound("spreader"),
}

local chip: Chip = {
    action = spreader.action { id = "spreader/action", gun = GUN, delay = 1, z = 0x10, shot = BURST },
    codes = { "L", "M", "N", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 30,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { spreader = chip }
