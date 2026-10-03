--!strict
-- SuprVulc (BN5 chip 0x0dc): action 0x19, BN6's vulcan with BN5's gun and bullets (objects/vulcan): 12 bullets of
-- row 0x10 (BN6's SuprVulc's).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bullet = require("../../objects/bullet/bullet")
local vulcan = require("../../objects/vulcan/vulcan")

return define.chip {
    id = "bn5:suprvulc",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = 10,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = vulcan.action(12, bullet.variants.super_vulcan),
}
