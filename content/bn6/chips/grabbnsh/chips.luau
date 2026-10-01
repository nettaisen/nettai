--!strict
-- GrabBnsh and GrabRvng: the user's side takes back the panels the other
-- side stole, and for each, a hand drops on every enemy navi's panel
-- (docs/engine/dimming-chip-effects.md §3).

local controller = require("./controller")

local grabbnsh = define.chip {
    id = "grabbnsh",
    name = "GrabBnsh",
    codes = { "B", "M", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 166, index = 166, sort = 220 },
    slot_in_limit = 1,
    dimming = controller.hook,
}

local grabrvng = define.chip {
    id = "grabrvng",
    name = "GrabRvng",
    codes = { "I", "Q", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 50,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    library = { number = 167, index = 167, sort = 221 },
    slot_in_limit = 1,
    dimming = controller.hook,
}

return { grabbnsh = grabbnsh, grabrvng = grabrvng }
