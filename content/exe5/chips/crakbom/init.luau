--!strict
-- CrakBom (EXE5 chip 0x026): a bomb thrown three panels ahead that hits the column where it lands and cracks it: EXE5's
-- throw (lib/bombs, action 0x12 subtype 10, the bomb held in animation 6) and its bomb (chips/crakbom/crakbom, attack
-- object #0x24) of kind 1.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local crakbom = require("@self/crakbom")

-- Its bomb (kind 1): the burst in palette 1, a crack's sound, hit modifier 3, cracking the column.
local BOMB = crakbom.variant {
    burst = new.effect { sprite = crakbom.burst, anim = 0, palette = 1 },
    sound = asset.sound("panel-crack"),
    hit_mod = 3,
    bug = 0,
    cracks = true,
}

local chip: Chip = {
    codes = { "C", "R", "V", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 18,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, held_anim = 6, thrower = crakbom.thrower(BOMB) },
}

return { crakbom = chip }
