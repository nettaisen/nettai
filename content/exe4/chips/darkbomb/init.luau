--!strict
-- DarkBomb (EXE4 chip 0x0BD): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- throw, action 0x09's variant 2: the navi holds the dark bomb (attachment row 0x11) and throws bomb variant 6 (its
-- parameter +0x10): thrown, hitting navis with modifier 3, in palette 3, blasting the 3x3 around where it lands
-- (region 0x0F) with the dark explosion (effect row 0x2B). Its cost: MegaMan steps left with no key held
-- (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local regions = require("@exelib/regions")

-- The dark bomb in hand (attachment row 0x11: the bomb, 0c-02, in palette 3), and its explosion (effect row 0x2B: the
-- explosion in palette 1).
local HELD = attachment.look { sprite = asset.sprite("bomb"), palette = 3 }
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 }

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
        held = HELD,
        thrower = bombs.thrower(bombs.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 3,
            palette = 3,
            anim = 1,
            blast = regions.square,
            explosion = EXPLOSION,
        }),
    },
}

return { darkbomb = darkbomb }
