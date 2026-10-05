--!strict
-- DrkLance (EXE5 chip 0x0bf): a dark chip: Lance's three lances (instant chip effect 4) with its Param1 1, in their
-- second palette with the bug 0x16 (chips/lance/lance). Its use costs a hit status (rules/light-dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local exe5_lance = require("./lance/lance")
local lance = require("@exelib/lance")

return define.chip {
    id = "drklance",
    codes = { "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 400,
    hit_param = 158,
    lockout = 20,
    flags = { "has_damage", "dark", "variable_damage" },
    slot_in_limit = 1,
    instant = lance.instant_of(exe5_lance.kind, 1),
}
