--!strict
-- BugBomb (BN5 chip 0x029): a bomb (bomb.luau) lobbed three panels ahead that bursts a second later and gives what it
-- hits a NaviCust bug: BN6's throw (@bn6/lib/bombs/throw, the original's action 0x12; BN5's the same code) with BN5's
-- held bomb and BugBomb.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../../lib/bombs")
local bug_bomb = require("./bomb")
local common = require("@common/bugbomb/bomb")
local throw = require("@bn6/lib/bombs/throw")

return define.chip {
    id = "bn5:bugbomb",
    codes = { "A", "R", "T", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    action = throw.action { held = bombs.held, held_anim = 4, thrower = common.thrower(bug_bomb.kind) },
}
