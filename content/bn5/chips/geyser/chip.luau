--!strict
-- Geyser (BN5 chip 0x02d): an aqua bomb lobbed at the panel three ahead: on solid ground a splash of 10 damage; in a
-- hole, a geyser that hits its panel and sends water onto the eight around it: BN5's throw (lib/bombs, action 0x12
-- subtype 5) and its Geyser (chips/geyser/geyser, attack objects #0x42 and #0x43).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../../lib/bombs/bombs")
local geyser = require("./geyser")

return define.chip {
    id = "bn5:geyser",
    codes = { "G", "M", "S" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 200,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, thrower = geyser.thrower(0) },
}
