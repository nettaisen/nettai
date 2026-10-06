--!strict
-- Navi+20: folded into the navi chip before it on the custom screen, 20
-- more damage; used alone, a sparkle (or, from a special source, the
-- side's navi bonus).

local plus = require("../lib/instant/plus")

return define.chip {
    id = "navi-20",
    codes = { "*" },
    element = "null", family = "plus", class = "standard",
    rarity = 2, mb = 36, damage = 20, hit_param = 0, lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { "modifier" },
    slot_in_limit = 1,
    modifier = "navi_plus",
    instant = plus.navi,
}
