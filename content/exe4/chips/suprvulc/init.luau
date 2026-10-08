--!strict
-- SuprVulc (EXE4 chip 0x0DC; its record at 0x080197EC + 0xDC * 0x2C): the vulcans' action 0x1F (chips/vulcan's), its
-- variant 3: the gun (attachment row 13) fires 12 shots (0x080EC934), bullet row 16 (its parameter +0x10): the panel
-- found and the one behind it, modifier 1, its hit (effect row 0x69: the Vulcans' hit in palette 1) shown once at the
-- bullet.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")
local regions = require("@exelib/regions")
local vulcan = require("./vulcan/vulcan")

local GUN = attachment.look { sprite = asset.sprite("vulcan"), palette = 0 }

local SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 1,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 1 },
    single = true,
}

local suprvulc: Chip = {
    action = vulcan.action { id = "suprvulc/action", gun = GUN, shots = 12, shot = SHOT },
    codes = { "V" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = 10,
    hit_param = 3,
    lockout = 0,
    flags = { "has_damage", "library" },
}

return { suprvulc = suprvulc }
