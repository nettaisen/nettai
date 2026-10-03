--!strict
-- Anubis (BN5 chip 0x0cc): a statue in front of the user that drains every enemy navi 1 HP each 6 ticks for 3000
-- ticks, with bubbles over the enemy's area: BN6's Anubis with BN5's statue (chips/anubis/anubis).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local anubis = require("./anubis")

return define.chip {
    id = "bn5:anubis",
    codes = { "A" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 4,
    mb = 86,
    damage = 1,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    megaman = "dark",
    dimming = anubis.controller.hook(anubis.variants.anubis),
}
