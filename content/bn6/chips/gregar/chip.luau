--!strict
-- Gregar: a giga cut-in chip that summons the cyber beast Gregar, which
-- breathes flames over the three columns ahead of its user and drops rocks
-- on the enemies' area (./controller, ./summon, ./flame;
-- docs/engine/beast-chips.md). Its use is the Japanese ROMs': the US ROMs
-- have its record and a null routine (`off_802CCB4` entry 34). Its
-- parameter byte (100) is the rocks' damage; its own damage the flames'.
-- (The record's extra flag byte 0x10 is read by no battle routine of the
-- US ROMs.)

local controller = require("./controller")

-- The palette of its picture on the custom screen, which no ROM holds:
-- the Japanese record points at EWRAM 0x02000B10, where a gift received
-- over the link (an e-Reader card's, Card e+) puts it (`0x0813006C` in
-- the Japanese ROMs copies the card's palette to 0x02000AF0 + 32 * slot;
-- slot 1 is the console's own beast), and the save keeps it there (save
-- offset 0xB10). These are the card's colours
-- as Tango's netplay save for EXE6 Gregar has them (tango-gamesupport-bn6,
-- saves/g_jp.raw, offset 0xB10). The picture is the Japanese Gregar ROM's
-- (docs/engine/jp-differences.md §4.4).
local ART_PALETTE = {
    0x0000, 0x77FF, 0x479E, 0x1F3F, 0x0A7D, 0x0D77, 0x04F4, 0x0051,
    0x1089, 0x18A3, 0x4D5F, 0x3787, 0x7F90, 0x5ACC, 0x3609, 0x2126,
}

return define.chip {
    id = "bn6:gregar",
    art_palette = ART_PALETTE,
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 16 },
    library = { number = 312, index = 12, sort = 82 },
    slot_in_limit = 1,
    dimming = controller.hook { rock_damage = 100 },
}
