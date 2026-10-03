--!strict
-- GunDelS1 (BN5 chip 0x020): a sun beam that drains the column two panels ahead while A is held, more in the sun, for
-- at most 60 ticks: BN5's GunDelSol (objects/gundels; BN6's code but for holding A, its collision row and its firing
-- ticks).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local gundels = require("../../objects/gundels/gundels")

return define.chip {
    id = "bn5:gundels1",
    codes = { "A", "S", "U", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 15,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundels1/action",
        gun = gundels.gun(0),
        firing_ticks = 60,
        beam = gundels.beams.plain,
        beam_in_sun = gundels.beams.plain_in_sun,
    },
}
