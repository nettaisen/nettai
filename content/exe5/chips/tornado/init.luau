--!strict
-- Tornado and Static: the tornado series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local tornado = require("@self/tornado")

-- Tornado (EXE5 chip 0x008): action 0x22, EXE6's blow with EXE5's fan and tornado (chips/tornado/tornado): a plain tornado
-- two panels ahead.
local tornado_chip: Chip = {
    codes = { "D", "L", "R" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 22,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = tornado.single,
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

-- Static (EXE5 chip 0x01b): action 0x22, EXE6's blow with EXE5's fan and tornado (chips/tornado/tornado): static tornadoes two
-- panels ahead and around, more the more kinds of NaviCust bug the navi has, paralyzing by that level.
local static: Chip = {
    codes = { "N", "S", "V" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "dark",
    action = tornado.static,
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

return { tornado = tornado_chip, static = static }
