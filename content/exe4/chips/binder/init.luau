--!strict
-- Binder1, Binder2 and Binder3 (EXE4 chips 0x026 to 0x028; their records at 0x080197EC + n * 0x2C): the throw, action
-- 0x09's variant 3: the navi holds the bomb (attachment row 4, its animation 0: 0x080EB4EC) and throws a binder
-- (./binder, 0x080D2748), which hops along the row hitting each panel it lands on; in palette 0, 2 or 4 (twice their
-- parameter, 0 to 2).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local binder = require("@self/binder")
local bombs = require("../lib/bombs")

local function throw(id: string, palette: number): Action
    return bombs.throw { id = id, held = bombs.held, thrower = binder.thrower(palette) }
end

local binder1: Chip = {
    action = throw("binder1/action", 0),
    codes = { "C", "O", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 60,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local binder2: Chip = {
    action = throw("binder2/action", 2),
    codes = { "A", "T", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 17,
    damage = 80,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local binder3: Chip = {
    action = throw("binder3/action", 4),
    codes = { "E", "I", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 24,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { binder1 = binder1, binder2 = binder2, binder3 = binder3 }
