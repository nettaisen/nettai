--!strict
-- EnergBom (BN5 chip 0x01e): a bomb that leaves an energy burst where it lands (three hits on that panel): BN6's
-- throw, bomb and burst (@common/bombs, @common/energbom/burst; BN5's the same code), with BN5's collision
-- types.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../../lib/bombs")
local burst = require("./burst")
local collision = require("../../rules/collision")

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
    action = bombs.throw {
        held = bombs.held,
        held_anim = 2,
        thrower = bombs.thrower(bombs.variant {
            collision = { is = collision.thrown, hits = collision.hits_navis },
            hit_mod = 1,
            palette = 0,
            anim = 3,
            after = burst.leave,
        }),
    },
}
