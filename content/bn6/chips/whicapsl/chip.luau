--!strict
-- WhiCapsl: folded into the damaging chip before it on the custom screen,
-- that chip paralyzes; used alone, a sparkle (or, from a special source,
-- the side's attack bonus with the paralysis flag).

local plus = require("../../lib/instant/plus")

return define.chip {
    id = "whicapsl",
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 1, mb = 30, damage = 0, hit_param = 50, lockout = 20,
    flags = { "library" },
    extra_flags = { "modifier" },
    library = { number = 192, index = 192, sort = 506 },
    slot_in_limit = 3,
    modifier = "paralyze",
    instant = plus.attack_with(plus.PARALYZE),
}
