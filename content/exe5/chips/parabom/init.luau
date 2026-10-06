--!strict
-- ParaBom (EXE5 chip 0x027): a bomb thrown three panels ahead that hits the column where it lands and paralyzes: EXE5's
-- throw (lib/bombs, action 0x12 subtype 10, the bomb held in animation 6) and its bomb (chips/crakbom/crakbom, attack
-- object #0x24) of kind 2.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local crakbom = require("./crakbom/crakbom")
local statuses = require("../rules/status")

-- Its bomb (kind 2): the burst in palette 2, the poison panel's sound, paralysis (status 0x10).
local BOMB = crakbom.variant {
    burst = new.effect { sprite = crakbom.burst, anim = 0, palette = 2 },
    sound = asset.sound("panel-poison"),
    hit_mod = 0,
    status = statuses.paralyze_90,
    bug = 0,
    cracks = false,
}

local chip: Chip = {
    codes = { "H", "N", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 34,
    damage = 90,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = bombs.held, held_anim = 6, thrower = crakbom.thrower(BOMB) },
}

return { parabom = chip }
