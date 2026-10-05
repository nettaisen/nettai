--!strict
-- BugBomb (EXE5 chip 0x029): a bomb (bomb.luau) lobbed three panels ahead that bursts a second later and gives what it
-- hits a NaviCust bug: EXE5's throw (lib/bombs: the shared throw, @exelib/bombs/throw, EXE6's action 0x12; EXE5's the same
-- code) with EXE5's held bomb and BugBomb.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local bug_bomb = require("@self/bomb")
local common = require("@exelib/bugbomb/bomb")

return define.chip {
    id = "bugbomb",
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
    action = bombs.throw { held = bombs.held, held_anim = 4, thrower = common.thrower(bug_bomb.kind) },
}
