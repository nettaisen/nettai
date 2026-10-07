--!strict
-- FullCust (EXE4 chip 0x0D1; its record at 0x080197EC + 0xD1 * 0x2C): the spawners' action 0x20 (lib/spawners), its
-- variant 7 (0x080084BC, EXE5's FullCust's 0x080096C4 but for the test): the custom gauge fills. (For a navi whose
-- NaviStats +0x26 is 2 it sets the side's own gauge full instead, the side's chip block +0x28: only a story's other
-- navi has that, 0x08041100, never a netbattle's.)
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local spawners = require("../lib/spawners")

-- 0x080084BC: 0x08016810(0x4000).
local function fill(_user: Object, _element: number, _damage: number)
    battle.fill_custom_gauge()
end

local chip: Chip = {
    action = spawners.action { id = "fullcust/action", spawn = fill },
    codes = { "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 45,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
}

return { fullcust = chip }
