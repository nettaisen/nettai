--!strict
-- BlkBomb: a black bomb (bomb.luau) lobbed three panels ahead, that sits
-- there until a fire hit sets it off over the other side's area.

local black_bomb = require("./bomb")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "blkbomb",
    name = "BlkBomb",
    codes = { "B", "F", "O" },
    element = "fire", family = "fire", class = "standard",
    rarity = 3, mb = 32, damage = 250, hit_param = 18, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 65, index = 65, sort = 42 },
    slot_in_limit = 3,
    action = throw.action { held = black_bomb.held, thrower = black_bomb.thrower() },
}
