--!strict
-- DrkCanon (EXE4 chip 0x0BB): a dark chip, which no folder holds and the custom screen offers (rules/dark_chips). The
-- cannons' action 0x0B (lib/cannon), its variant 3: the dark cannon (attachment row 0x30), the cannon's shot (projectile row 1).
-- Its damage is the HP its user has lost, at most 999 (its power 1046: damage formula 46, 0x080195AE), worked out
-- again every tick it waits in the hand (flag 0x80). Its cost: the charged shot becomes weapon routine 0x21
-- (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local cannon = require("../lib/cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")

-- The dark cannon (attachment row 0x30: 0c-01 in palette 3).
local ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 3 }

-- The cannon's shot (projectile row 1, action 0x0B's variants 0 to 3: 0x080EB97A): it staggers (hit modifier 3), with
-- the impact spark.
local SHOT = projectile.variant {
    id = "drkcanon/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.impact,
}

local drkcanon: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = { formula = "hp_lost", cap = 999 },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "dark", "variable_damage" },
    action = cannon.action { id = "drkcanon/action", arm = ARM, shot = SHOT },
}

return { drkcanon = drkcanon }
