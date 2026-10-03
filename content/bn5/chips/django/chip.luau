--!strict
-- Django (BN5 chip 0x116): Django cuts in, shuts the first enemy navi ahead in his row in his coffin and purifies it
-- under his sun (./navi): a hit a tick for the chip's damage in ticks.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local navi = require("./navi")

return define.chip {
    id = "bn5:django",
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 40,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    megaman = "light",
    -- (In the navi chips' block though the US record lacks the `navi` flag: AntiNavi turns it back, and it is kept
    -- as the last navi chip.)
    traits = { "navi_slot" },
    navi = navi.summon,
}
