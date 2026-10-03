--!strict
-- BugBomb: a bomb (bomb.luau) lobbed three panels ahead that bursts a
-- second later and gives what it hits a NaviCust bug.

local bomb = require("../../lib/bombs/bomb")
local bug_bomb = require("./bomb")
local throw = require("../../lib/bombs/throw")

return define.chip {
    id = "bn6:bugbomb",
    codes = { "G", "S", "V" },
    element = "null", family = "null", class = "standard",
    rarity = 2, mb = 24, damage = 0, hit_param = 0, lockout = 0,
    flags = { "standard_library", "library" },
    library = { number = 66, index = 66, sort = 45 },
    slot_in_limit = 3,
    action = throw.action { held = bomb.held, held_anim = 4, thrower = bug_bomb.thrower() },
}
