--!strict
-- TimeBom+ (BN5 chip 0x15b): the Program Advance of TimeBom1, TimeBom2 and TimeBom3 in that order: TimeBom1-3's bomb
-- with 300 HP, its own look and a harder shake (objects/timebom: BN6's TimeBom with BN5's placement).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local timebom = require("../../objects/timebom/timebom")
local timebom1 = require("../timebom1/chip")
local timebom2 = require("../timebom2/chip")
local timebom3 = require("../timebom3/chip")

return define.chip {
    id = "bn5:timebom",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 600,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    -- BN5's recipes are in the order of its full table (0x08027FC8; the netbattles' 0x0802801C is that table but
    -- for its first 21, which only a battle with each side keeping its own gauge tries): its 24th, the
    -- netbattles' third.
    program_advances = { { order = 23, sequence = { timebom1, timebom2, timebom3 } } },
    dimming = timebom.controller.hook(timebom.variants.big),
}
