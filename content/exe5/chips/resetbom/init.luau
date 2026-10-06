--!strict
-- ResetBom (EXE5 chip 0x028): a bomb thrown three panels ahead that hits the column where it lands and takes away
-- MegaMan's body programs (bug 0xFB): EXE5's throw (lib/bombs, action 0x12 subtype 10, the bomb held in animation 6)
-- and its bomb (chips/crakbom/crakbom, attack object #0x24) of kind 3.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local crakbom = require("./crakbom/crakbom")

-- Its bomb (kind 3): the burst in palette 3, the poison panel's sound, hit modifier 3, bug 0xFB.
local BOMB = crakbom.variant {
    burst = new.effect { sprite = crakbom.burst, anim = 0, palette = 3 },
    sound = asset.sound("panel-poison"),
    hit_mod = 3,
    bug = 0xFB,
    cracks = false,
}

local chip: Chip = {
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
    action = bombs.throw { held = bombs.held, held_anim = 6, thrower = crakbom.thrower(BOMB) },
    auto_battle = { class = "into_reach", range = "column_three_ahead" },
}

return { resetbom = chip }
