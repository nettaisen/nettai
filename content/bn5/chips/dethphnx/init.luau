--!strict
-- DethPhnx (BN5 chip 0x13a): DeathPhoenix sets 12 fires over the enemy area, then brings the last navi chip's navi again (./controller,
-- ./phoenix). Past the navi chips' block: AntiNavi lets it through.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local controller = require("@self/controller")

return define.chip {
    id = "dethphnx",
    codes = { "D" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 93,
    damage = 150,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    traits = { "not_navi_slot" },
    dimming = controller.hook,
}
