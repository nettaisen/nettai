--!strict
-- GunDelS3 (BN5 chip 0x022): a sun beam that drains the column two panels ahead while A is held, more in the sun, for
-- at most 120 ticks: BN5's GunDelSol (objects/gundels; BN6's code but for holding A, its collision row and its firing
-- ticks).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local gundels = require("../../objects/gundels/gundels")

return define.chip {
    id = "bn5:gundels3",
    codes = { "D", "M", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 35,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundels3/action",
        gun = gundels.gun(6),
        firing_ticks = 120,
        beam = gundels.beams.plain,
        beam_in_sun = gundels.beams.plain_in_sun,
    },
}
