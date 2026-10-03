--!strict
-- SideBub1, SideBub2 and SideBub3: the sidebub series (each chip's own comment above its definition).
-- Their records are written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local sidebub = require("./sidebub")

-- SideBub1 (BN5 chip 0x0a0): the navi sends two bubbles to the column ahead, in the rows above and below its own,
-- drifting forward and bouncing between the top and bottom rows; a popped bubble bursts into an aqua shot: BN5's
-- own (chips/sidebub/sidebub).
local sidebub1 = define.chip {
    id = "bn5:sidebub1",
    codes = { "D", "Q", "S" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 25,
    damage = 90,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = sidebub.action,
}

-- SideBub2 (BN5 chip 0x0a1): the navi sends two bubbles to the column ahead, in the rows above and below its own,
-- drifting forward and bouncing between the top and bottom rows; a popped bubble bursts into an aqua shot: BN5's
-- own (chips/sidebub/sidebub).
local sidebub2 = define.chip {
    id = "bn5:sidebub2",
    codes = { "H", "L", "W" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 35,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = sidebub.action,
}

-- SideBub3 (BN5 chip 0x0a2): the navi sends two bubbles to the column ahead, in the rows above and below its own,
-- drifting forward and bouncing between the top and bottom rows; a popped bubble bursts into an aqua shot: BN5's
-- own (chips/sidebub/sidebub).
local sidebub3 = define.chip {
    id = "bn5:sidebub3",
    codes = { "E", "N", "P" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 45,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = sidebub.action,
}

return { sidebub1 = sidebub1, sidebub2 = sidebub2, sidebub3 = sidebub3 }
