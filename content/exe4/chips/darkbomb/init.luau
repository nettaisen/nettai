--!strict
-- DarkBomb (EXE4 chip 0x0BD): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- throw, action 0x09's variant 2: the navi holds the dark bomb (attachment row 0x11) and throws bomb variant 6 (its
-- parameter +0x10): thrown, hitting navis with modifier 3, in palette 3, blasting the 3x3 around where it lands
-- (region 0x0F) with the dark explosion (effect row 0x2B). Its cost: MegaMan steps left with no key held
-- (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")

local darkbomb: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 200,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "dark" },
    action = bombs.throw {
        id = "darkbomb/action",
        held = bombs.held_dark,
        thrower = bombs.thrower(bombs.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 3,
            palette = 3,
            anim = 1,
            blast = regions.square,
            explosion = bombs.effects.dark_explosion,
        }),
    },
}

return { darkbomb = darkbomb }
