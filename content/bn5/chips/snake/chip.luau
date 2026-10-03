--!strict
-- Snake (BN5 chip 0x060): a cut-in chip whose snakes leap out of the holes on the user's side at the nearest enemy,
-- three at a time (chips/snake: BN6's controller with BN5's nest and snakes).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local snake = require("./snake")

return define.chip {
    id = "bn5:snake",
    codes = { "C", "M", "Z" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 30,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = snake.hook,
}
