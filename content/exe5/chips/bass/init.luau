--!strict
-- Bass (EXE5 chip 0x12d): Bass cuts in and fires 24 volleys of two shots at the enemy area (./navi).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi = require("@self/navi")
local navi_chips = require("../lib/navi_chips/chip")

local chip: Chip = {
    codes = { "F" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 60,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 4, 16 },
    slot_in_limit = 1,
    -- (Past the navi chips' block: AntiNavi lets it through, and it isn't kept as the last navi chip.)
    navi = navi_chips.make {
        id = "bass/controller",
        summon = navi.summon,
        navi_block = false,
    },
    auto_battle = { class = "at_once" },
}

return { bass = chip }
