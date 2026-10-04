--!strict
-- The invalid chip (BN5 chip 0x185, past the named ones, no name): what a chip use BN5's light and dark rule
-- refuses becomes (0x080100EC: rules/light-dark), and what an illegal pick counts as. Its own use is action 0x1A's
-- effect 26, BN6's `sub_80104E0` (0x0800E240, the same code): a sparkle by the navi (lib/plus: @exelib/instant/sparkle). A
-- refused use runs effect 3 instead (0x0800E198, BN5's plus chip), which with the parameters the refusal zeroes
-- shows the same sparkle.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local plus = require("../../lib/plus")

return define.chip {
    id = "invalid",
    codes = {},
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = {},
    slot_in_limit = 1,
    instant = plus.sparkle_alone,
}
