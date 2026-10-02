--!strict
-- MiniBomb (BN5 chip 0x01d): a bomb thrown three panels ahead that blasts the panel it lands on: BN6's throw and
-- bomb (@bn6/lib/bombs; BN5's the same code), with BN5's collision types.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bomb = require("@bn6/lib/bombs/bomb")
local collision = require("../../rules/collision")
local regions = require("@bn6/lib/regions")
local throw = require("@bn6/lib/bombs/throw")

return define.chip {
    id = "minibomb",
    codes = { "B", "G", "L", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 50,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = throw.action {
        held = bomb.held,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 0,
            palette = 0,
            anim = 1,
            blast = regions.single,
        }),
    },
}
