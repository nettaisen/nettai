--!strict
-- ElcReel1, ElcReel2 and ElcReel3: the elcreel series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local elcreel = require("@self/elcreel")

-- ElcReel1 (EXE5 chip 0x0a3): the navi launches a reel along its row; what it hits, it hooks with every navi around
-- it, and shocks them: EXE5's own (chips/elcreel/elcreel).
local elcreel1 = define.chip {
    id = "elcreel1",
    codes = { "C", "V", "Z" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = elcreel.action,
}

-- ElcReel2 (EXE5 chip 0x0a4): the navi launches a reel along its row; what it hits, it hooks with every navi around
-- it, and shocks them: EXE5's own (chips/elcreel/elcreel).
local elcreel2 = define.chip {
    id = "elcreel2",
    codes = { "B", "Q", "Y" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 46,
    damage = 100,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = elcreel.action,
}

-- ElcReel3 (EXE5 chip 0x0a5): the navi launches a reel along its row; what it hits, it hooks with every navi around
-- it, and shocks them: EXE5's own (chips/elcreel/elcreel).
local elcreel3 = define.chip {
    id = "elcreel3",
    codes = { "E", "L", "R" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 60,
    damage = 120,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = elcreel.action,
}

return { elcreel1 = elcreel1, elcreel2 = elcreel2, elcreel3 = elcreel3 }
