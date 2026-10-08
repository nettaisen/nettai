--!strict
-- Blinder (EXE4 chip 0x081): the dimming chips' variant 0x0E, a flash that blinds every enemy navi: its controller
-- (effect object #0x23, 0x080E48EC: @exelib/blinder/controller, EXE6's code) in EXE4's look: SparkMan's flash's sound
-- (0xBA), the blinding hit's row 0x34 at navis (0x080E49D0), blindness for 480 ticks (status 0x3000).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/blinder/controller")
local statuses = require("../rules/status")

local chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 9,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = controller.make {
        id = "blinder/controller",
        sound = asset.sound("spark-flash"),
        is = collision.collision_34,
        hits = collision.hits_navis,
        status = statuses.blind_480,
    },
}

return { blinder = chip }
