--!strict
-- ColorPt and DblPoint: the colorpt series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local colorpt = require("@self/colorpt")

-- ColorPt (EXE5 chip 0x07f): the user's front column goes to the other side, panel by panel, for Atk+10 a panel on
-- the chip at the hand's cursor: EXE6's ColorPt with EXE5's points (chips/colorpt/colorpt).
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
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = colorpt.hook { palette = 0, bonus = 10 },
}

-- DblPoint (EXE5 chip 0x0cf): the user's front column goes to the other side, panel by panel, for Atk+20 a panel on
-- the chip at the hand's cursor: EXE6's DblPoint with EXE5's points (chips/colorpt/colorpt).
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
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = colorpt.hook { palette = 1, bonus = 20 },
}

return { colorpt = colorpt_chip, dblpoint = dblpoint }
