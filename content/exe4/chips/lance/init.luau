--!strict
-- Lance (EXE4 chip 0x051): the spawners' action 0x20, its variant 6: three lances in the far column (./lance), its
-- parameters' first byte 0, their first palette.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local lance = require("@self/lance")
local spawners = require("../lib/spawners")

local chip: Chip = {
    codes = { "A", "G", "R", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 42,
    damage = 130,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    action = spawners.action {
        id = "lance/action",
        spawn = function(user: Object, element: number, damage: number)
            lance.thrust(user, element, 0, damage)
        end,
    },
}

return { lance = chip }
