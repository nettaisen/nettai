--!strict
-- ColorPt and DblPoint: the colorpt series (each chip's own comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local colorpt = require("@self/colorpt")

-- ColorPt (EXE4 chip 0x07f): the dimming chips' variant 0x27: the user's front column goes to the other side, panel by
-- panel, for Atk+10 a panel on the chip at the hand's cursor (./colorpt; the record's parameters: its points in
-- palette 0, 10 each).
local colorpt_chip: Chip = {
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 1,
    mb = 25,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    dimming = colorpt.hook { palette = 0, bonus = 10 },
}

-- DblPoint (EXE4 chip 0x0cf): the dimming chips' variant 0x27: the user's front column goes to the other side, panel
-- by panel, for Atk+20 a panel on the chip at the hand's cursor (./colorpt; the record's parameters: its points in
-- palette 4, the record's 1 times 4 as the point's init reads it, 0x080E7D46; 20 each).
local dblpoint: Chip = {
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 3,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    dimming = colorpt.hook { palette = 4, bonus = 20 },
}

return { colorpt = colorpt_chip, dblpoint = dblpoint }
