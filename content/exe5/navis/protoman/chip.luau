--!strict
-- ProtoMan's own chip, StepSwrd (EXE5 chip 0x191), which his custom screen offers until it is picked (rules: the
-- navi's `own_chip`): WideSwrd's slash (EXE5's action 0x13 as its subtype 1, chips/wideswrd) from two panels ahead. The
-- step is the chip's own routine's doing (its record's +0x1F: routine 2 of 0x0800EC80, which the chip's use calls at
-- 0x080100CE, once the attack is loaded): it sets the attack's charged byte, by which EXE5's swords step (lib/swords).
-- Its damage is by his level (the chips' damage formula 50: the damage rows' row 1, lib/navi_level).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local wideswrd = require("../../chips/wideswrd")

return define.chip {
    id = "stepswrd",
    codes = { "B" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 8,
    damage = navi_level.row({ 80, 80, 100, 100, 120, 120, 120 }),
    hit_param = 148,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = wideswrd.action,
    -- 0x0800EC68's routine 2.
    setup = function(navi: Object)
        navi.charged = 1
    end,
}
