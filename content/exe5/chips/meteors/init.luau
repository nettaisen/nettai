--!strict
-- Meteors (EXE5 chip 0x0d2): thirty meteors fall on the enemy's area, one every 10 ticks, cycling through its
-- panels from the far column: the shared Meteors (@exelib/meteors/controller, dimming subtype 18, its meteors' row 1)
-- with EXE5's meteor (chips/meteors).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@exelib/meteors/controller")
local meteors = require("@self/meteors")

local METEORS = controller.make("meteors/controller", meteors.kind)

local chip: Chip = {
    codes = { "R" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 4,
    mb = 73,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    dimming = METEORS,
}

return { meteors = chip }
