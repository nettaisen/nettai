--!strict
-- SumnBlk1, SumnBlk2, SumnBlk3: cut-in chips that call a navi out of the
-- hole in front of the user; it comes up in front of one of the other
-- side's navis and claws its column (controller.luau). Without a free
-- hole in front, nothing happens.

local controller = require("@self/controller")
local lockon = require("../rules/lockon")

local chips: { [string]: Chip } = {
    sumnblk1 = {
        codes = { "E", "I", "P" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 30, damage = 160, hit_param = 158, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        beast = { rush = false, lockon = lockon.cannon },
        dimming = controller.hook(0),
    },
    sumnblk2 = {
        codes = { "H", "O", "V" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 40, damage = 200, hit_param = 158, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        beast = { rush = false, lockon = lockon.cannon },
        dimming = controller.hook(1),
    },
    sumnblk3 = {
        codes = { "W", "Y", "Z" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 46, damage = 260, hit_param = 158, lockout = 0,
        flags = { "dimming", "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        beast = { rush = false, lockon = lockon.cannon },
        dimming = controller.hook(2),
    },
}

return chips
