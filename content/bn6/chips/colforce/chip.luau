--!strict
-- ColForce: soldiers (force.luau, soldier.luau) drop in behind the other
-- side's navis and objects, column by column, and fire. (Its record's menu
-- classification bits, 0x04 and 0x10, which no battle routine reads, stay
-- out: content-model-v2.md §3.1.)

local force = require("./force")

return define.chip {
    id = "colforce",
    codes = { "Q" },
    element = "null", family = "null", class = "giga",
    rarity = 4, mb = 90, damage = 30, hit_param = 138, lockout = 0,
    flags = { "has_damage" },
    library = { number = 304, index = 4, sort = 104 },
    slot_in_limit = 1,
    instant = force.instant,
}
