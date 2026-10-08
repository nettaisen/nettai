--!strict
-- YoYo (EXE5 chip 0x052): a yoyo that rolls two panels ahead, spins twice there and rolls back (EXE5's yoyo,
-- chips/yoyo/yoyo; the throw EXE6's, action 0x4A).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local yoyo = require("@self/yoyo")

local chip: Chip = {
    codes = { "D", "E", "F" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = yoyo.action(yoyo.spec { panels = 2, rounds = 2 }),
    auto_battle = { class = "before_target" },
}

return { yoyo = chip }
