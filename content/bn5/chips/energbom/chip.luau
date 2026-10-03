--!strict
-- EnergBom (BN5 chip 0x01e): a bomb that leaves an energy burst where it lands (three hits on that panel): BN6's
-- throw, bomb and burst (@bn6/lib/bombs, @bn6/chips/energbom/burst; BN5's the same code), with BN5's collision
-- types.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bomb = require("@bn6/lib/bombs/bomb")
local burst = require("@bn6/chips/energbom/burst")
local collision = require("../../rules/collision")
local throw = require("@bn6/lib/bombs/throw")

return define.chip {
    id = "bn5:energbom",
    codes = { "E", "P", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 11,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = throw.action {
        held = bomb.held,
        held_anim = 2,
        thrower = bomb.thrower(bomb.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 1,
            palette = 0,
            anim = 3,
            after = burst.leave,
        }),
    },
}
