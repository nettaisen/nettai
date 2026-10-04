--!strict
-- LavaSeed (BN5 chip 0x02b): a seed thrown three panels ahead that turns the panels around where it lands to lava
-- (only its own if it hit something): BN5's throw (lib/bombs, action 0x12) and its seed (lib/bombs/seed, attack object
-- #0x4A).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../../lib/bombs/bombs")
local seed = require("../../lib/bombs/seed")

return define.chip {
    id = "lavaseed",
    codes = { "H", "K", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 60,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    action = bombs.throw {
        held = seed.held(seed.variants.lava),
        held_anim = 8,
        thrower = seed.thrower(seed.variants.lava),
    },
}
