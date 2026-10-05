--!strict
-- DarkThnd (EXE5 chip 0x0c1): a dark chip: Thunder's shot (@exelib/thunder) with EXE5's big ball (chips/thunder/ball)
-- that homes in for 12 panels, paralyzes and carries bug 0x19 (its record's parameters); its use costs the custom
-- screen's HP drain (rules/light-dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local statuses = require("../rules/status")
local thunder = require("./thunder/ball")

return define.chip {
    id = "darkthnd",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = thunder.shoot {
        id = "darkthnd/action",
        ball = { panels = 0x0C, status = statuses.paralyze_90, bug = 0x19 },
    },
}
