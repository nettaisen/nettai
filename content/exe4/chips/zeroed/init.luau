--!strict
-- The zeroed chip (EXE4 chip 0x000, no name): what a zeroed chip field reads (the rules' role `chips.zeroed`), as a
-- charged fire chip's flame in FireSoul leaves the attack's chip (0x0800D638). Its record names the cannons' action
-- 0x0B, its variant 0 (Cannon's: lib/cannon, attachment row 0, projectile row 1), which no use of it reaches.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local cannon_lib = require("../lib/cannon")
local collision = require("../rules/collision")
local projectile = require("../objects/projectile")
local sparks = require("../lib/sparks")

-- Its cannon (attachment row 0: 0c-01 in palette 0) and shot (projectile row 1: it staggers, the impact spark).
local ARM = attachment.look { sprite = asset.sprite("cannon"), palette = 0 }
local SHOT = projectile.variant {
    id = "zeroed/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 3,
    element = "null",
    spark = sparks.impact,
}

local zeroed: Chip = {
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "has_damage" },
    action = cannon_lib.action { id = "zeroed/action", arm = ARM, shot = SHOT },
}

return { zeroed = zeroed }
