--!strict
-- Phoenix (BN5 chip 0x139): The phoenix drops 18 fireballs on the field's middle columns, then heals its user's side (./controller,
-- ./phoenix). Past the navi chips' block: AntiNavi lets it through.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("./controller")

return define.chip {
    id = "phoenix",
    codes = { "P" },
    element = "fire",
    family = "fire",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 150,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    slot_in_limit = 1,
    traits = { "not_navi_slot" },
    dimming = controller.hook,
}
