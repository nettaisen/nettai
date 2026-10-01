--!strict
-- VDoll: a doll (doll.luau) lobbed three panels ahead, that poisons the
-- panel it lands on and, hurt there, curses the other side with the damage
-- it took (curse.luau). The original's action 0x12 with subtype 8.

local bomb = require("../../lib/bombs/bomb")
local doll = require("./doll")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "vdoll",
    name = "VDoll",
    description = "Throws\nVDoll\n3sq ahead",
    codes = { "F", "N", "S", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 3,
    mb = 39,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "standard_library", "library" },
    library = { number = 153, index = 153, sort = 450 },
    slot_in_limit = 3,
    action = throw.action { id = "vdoll/action", held = bomb.held, thrower = doll.thrower() },
}
