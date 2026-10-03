--!strict
-- GunDelS2 (BN5 chip 0x021): a sun beam that drains the column two panels ahead while A is held, more in the sun, for
-- at most 90 ticks: BN5's GunDelSol (objects/gundels; BN6's code but for holding A, its collision row and its firing
-- ticks).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local gundels = require("../../objects/gundels/gundels")

return define.chip {
    id = "bn5:gundels2",
    codes = { "G", "U", "Y" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundels2/action",
        gun = gundels.gun(3),
        firing_ticks = 90,
        beam = gundels.beams.plain,
        beam_in_sun = gundels.beams.plain_in_sun,
    },
}
