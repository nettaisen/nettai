--!strict
-- EnergBom and MegEnBom (EXE4 chips 0x01E and 0x01F; their records at 0x080197EC + n * 0x2C): the throw, action 0x09's
-- variant 1: the navi holds the bomb (attachment row 4, its animation 2: 0x080EB4EC) and throws bomb variant 2 (their
-- parameter +0x10: the row 0x0A05 0100 of 0x080CE088, thrown, hitting navis with modifier 1, in palette 0, animation
-- 3, 0x080CE0E4), which blasts nothing where it lands (0x080CE270's 0) and leaves an energy burst there the tick after
-- (0x080CE2A0: ./burst, three hits on that panel).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local bombs = require("../lib/bombs")
local burst = require("@self/burst")
local collision = require("../rules/collision")

local THROWER = bombs.thrower(bombs.variant {
    collision = { is = collision.thrown, hits = collision.hits_navis },
    hit_mod = 1,
    palette = 0,
    anim = 3,
    after = burst.leave,
})

local function throw(id: string): Action
    return bombs.throw { id = id, held = bombs.held, held_anim = 2, thrower = THROWER }
end

local energbom: Chip = {
    action = throw("energbom/action"),
    codes = { "E", "N", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 11,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local megenbom: Chip = {
    action = throw("megenbom/action"),
    codes = { "D", "J", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 30,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { energbom = energbom, megenbom = megenbom }
