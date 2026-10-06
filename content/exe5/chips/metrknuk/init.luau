--!strict
-- MetrKnuk (EXE5 chip 0x130): a giga cut-in chip that rains 16 fists around the enemies, cracking the panels (./controller).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "M" },
    element = "null",
    family = "break",
    class = "giga",
    rarity = 4,
    mb = 90,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 4, 16 },
    slot_in_limit = 1,
    megaman = "light",
    dimming = controller.hook,
    auto_battle = { class = "at_once" },
}

return { metrknuk = chip }
