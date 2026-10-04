--!strict
-- Atk+10: folded into the chip before it on the custom screen, 10 more
-- damage; used alone, a sparkle (or, from a special source, the side's
-- attack bonus).

local plus = require("../../lib/instant/plus")

return define.chip {
    id = "atk-10",
    codes = { "*" },
    element = "null", family = "plus", class = "standard",
    rarity = 0, mb = 4, damage = 10, hit_param = 0, lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { "modifier", "free_slot_in" },
    library = { number = 196, index = 196, sort = 21 },
    slot_in_limit = 1,
    modifier = "attack_plus",
    instant = plus.attack,
}
