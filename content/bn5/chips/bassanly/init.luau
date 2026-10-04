--!strict
-- BassAnly (BN5 chip 0x132): Bass cuts in as BassAnly and fires four dark balls up and down the slant, seeking enemy navis (./navi).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("@self/navi")

return define.chip {
    id = "bassanly",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 160,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
    -- (Past the navi chips' block: AntiNavi lets it through, and it isn't kept as the last navi chip.)
    traits = { "not_navi_slot" },
    navi = navi.summon,
}
