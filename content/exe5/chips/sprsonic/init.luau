--!strict
-- SprSonic (EXE5 chip 0x177): a swing that throws a sonic boom through the row (NeoVari's pick: no library chip): EXE5's
-- action 0x2B (../sonicbom/action) and EXE5's sonic boom (lib/swords), hit modifier 1, going through.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local sonicbom = require("./sonicbom/action")

return define.chip {
    id = "sprsonic",
    codes = { "E", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "special",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    action = sonicbom.make("sprsonic/action", 1, true),
}
