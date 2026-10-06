--!strict
-- Magnum (EXE5 chip 0x05e): a cut-in chip: the user's stand-in moves a sight across the enemy's columns until A or 180
-- ticks, then shoots the sight's column, breaking its panels: EXE5's controller (controller.luau, the dimming
-- handler's subtype 15, effect object #0x30) and gunner (gunner.luau, actor object #2).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local controller = require("@self/controller")

local chip: Chip = {
    codes = { "E", "O", "V" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 130,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    dimming = controller.hook(),
    auto_battle = { class = "at_once" },
}

return { magnum = chip }
