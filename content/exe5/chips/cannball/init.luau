--!strict
-- CannBall (EXE5 chip 0x02c): a cannonball lobbed at the panel three ahead that hits and breaks the panel where it
-- lands: EXE5's throw (lib/bombs, action 0x12 subtype 4) and its cannonball (chips/cannball/cannball, attack object #0x35) of
-- kind 0.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local bombs = require("../lib/bombs")
local cannball = require("@self/cannball")

local chip: Chip = {
    codes = { "C", "T", "Z", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 140,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = bombs.throw { held = cannball.held, thrower = cannball.thrower() },
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

return { cannball = chip }
