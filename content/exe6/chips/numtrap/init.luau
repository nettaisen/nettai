--!strict
-- NumTrap: a special chip with the plus chips' effect (a sparkle, or,
-- from a special source, its damage into the side's attack bonus).

local plus = require("../lib/instant/plus")

local chip: Chip = {
    element = "null", family = "null", class = "special",
    rarity = 0, mb = 99, damage = 10, hit_param = 0, lockout = 0,
    slot_in_limit = 1,
    instant = plus.attack,
}

return { numtrap = chip }
