--!strict
-- Atk+10: folded into the chip before it on the custom screen, 10 more
-- damage; used alone, a sparkle (or, from a special source, the side's
-- attack bonus).

local plus = require("../lib/instant/plus")

local chip: Chip = {
    codes = { "*" },
    element = "null", family = "plus", class = "standard",
    rarity = 0, mb = 4, damage = 10, hit_param = 0, lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { "modifier", "free_slot_in" },
    slot_in_limit = 1,
    modifier = "attack_plus",
    instant = plus.attack,
}

return { ["atk-10"] = chip }
