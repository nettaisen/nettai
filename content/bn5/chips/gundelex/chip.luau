--!strict
-- GunDelEX (BN5 chip 0x0db): a wide sun beam that drains the column two panels ahead and the one past it while A is
-- held, more in the sun, for at most 120 ticks: BN5's GunDelSol (objects/gundels, action 0x2F with subtype 3: the
-- gun's attachment row 0x16, palette 9; the firing ticks' fourth, 120; the wide beams).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local gundels = require("../../objects/gundels/gundels")

return define.chip {
    id = "bn5:gundelex",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
    action = gundels.action {
        id = "bn5:gundelex/action",
        gun = gundels.gun(9),
        firing_ticks = 120,
        beam = gundels.beams.wide,
        beam_in_sun = gundels.beams.wide_in_sun,
        wide = true,
    },
}
