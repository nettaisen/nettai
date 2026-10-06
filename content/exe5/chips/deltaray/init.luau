--!strict
-- DeltaRay (EXE5 chip 0x12e): ProtoMan cuts in as DeltaRay and strikes the first enemy navi ahead, a strike more for each A while dimmed, up to three (./navi).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")

local chip: Chip = {
    codes = { "Z" },
    element = "null",
    family = "sword",
    class = "giga",
    rarity = 4,
    mb = 82,
    damage = 220,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    extra_flags = { 4, 16 },
    slot_in_limit = 1,
    navi = navi.summon { palette = 0 },
}

return { deltaray = chip }
