--!strict
-- MoonBld3 (BN5 chip 0x03b): a spin with a blade that sweeps the eight panels around the navi (BN5's MoonBld,
-- objects/moonbld: action 0x53, BN6's swing with the shared moon blade; a charged one steps first).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local moonbld = require("../../objects/moonbld/moonbld")

return define.chip {
    id = "bn5:moonbld3",
    codes = { "C", "L", "T" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 49,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = moonbld.action,
}
