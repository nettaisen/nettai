--!strict
-- ResetBom (BN5 chip 0x028): a bomb thrown three panels ahead that hits the column where it lands and takes away
-- MegaMan's body programs (bug 0xFB): BN5's throw (lib/bombs, action 0x12 subtype 10, the bomb held in animation 6)
-- and its bomb (chips/crakbom/crakbom, attack object #0x24) of kind 3.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local bombs = require("../lib/bombs")
local crakbom = require("./crakbom/crakbom")

return define.chip {
    id = "resetbom",
    codes = { "D", "I", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, held_anim = 6, thrower = crakbom.thrower(3) },
}
