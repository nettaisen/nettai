--!strict
-- Jealousy (EXE4 chip 0x0D4): the dimming chips' variant 0x1A, a hit on every panel the other side stands on once for
-- each chip the other side's navi holds, 10 ticks apart: EXE6's leftover controller (effect object #0x36, 0x080E5EF4:
-- @exelib/jealousy/controller) in EXE4's look: the chips counted (0x080E5FDC: the other side's local navi when that
-- side is the console's, `battle_networkInvert`, else the most any of its listed actors holds; in a netbattle each is
-- the other side's navi, its hand from the cursor, 0x080E6014), its hits' row 0x1A at navis with the heavy spark; no
-- operation battle.
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local controller = require("@exelib/jealousy/controller")
local sparks = require("../lib/sparks")

-- 0x080E5FDC: the chips the other side's navi holds (none without one).
local function count(me: Object): number
    local navi = battle.player(bit32.bxor(me.alliance, 1))
    if not navi then
        return 0
    end
    return battle.hand_left(navi.alliance)
end

local chip: Chip = {
    codes = { "J" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 35,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller.make {
        id = "jealousy/controller",
        count = count,
        is = collision.collision_1a,
        hits = collision.hits_navis,
        spark = sparks.heavy,
    },
}

return { jealousy = chip }
