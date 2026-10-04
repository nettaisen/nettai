--!strict
-- RockCube (BN5 chip 0x063): a dimming that sets a rock cube rising on the panel in front of the user (./cube, the
-- dimming handler's subtype 20).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local cube = require("./cube")

return define.chip {
    id = "rockcube",
    codes = { "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 6,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
    dimming = cube.hook,
}
