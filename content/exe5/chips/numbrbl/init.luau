--!strict
-- NumbrBl (EXE5 chip 0x0d3): a cut-in chip: the user warps out, and NumberMan himself sets five balls rolling down the
-- row (./controller, ./numberman); its damage is the last two digits of the user's HP.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "numbrbl",
    codes = { "N", "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 69,
    damage = { formula = "hp_last_digits" },
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 1,
    dimming = controller.hook(2),
}
