--!strict
-- MiniBomb (EXE4 chip 0x01D; its record at 0x080197EC + 0x1D * 0x2C): the throw, action 0x09's variant 0: the navi
-- holds the bomb (attachment row 4, its animation 0: 0x080EB4EC) and throws bomb variant 0 (its parameter +0x10: the
-- row 0x0A05 0000 of 0x080CE088, thrown, hitting navis with modifier 0, in palette 0, animation 1, 0x080CE0E4),
-- blasting the panel where it lands (region 0x01, 0x080CE270) with the explosion (effect row 0, 0x080CE277).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")

local minibomb: Chip = {
    action = bombs.throw {
        id = "minibomb/action",
        held = bombs.held,
        thrower = bombs.thrower(bombs.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 0,
            palette = 0,
            anim = 1,
            blast = regions.single,
        }),
    },
    codes = { "B", "L", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 50,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { minibomb = minibomb }
