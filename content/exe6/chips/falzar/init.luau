--!strict
-- Falzar: a giga cut-in chip that sends the cyber beast Falzar's feathers
-- diving onto the enemies' area, then summons Falzar, whose whirlwind
-- sweeps the three columns ahead of its user (./controller, ./feather,
-- ./summon, ./tornado; docs/engine/beast-chips.md). Its use is the
-- Japanese ROMs': the US ROMs have its record and a null routine
-- (`off_802CCB4` entry 35). (The record's extra flag byte 0x10 is read by
-- no battle routine of the US ROMs.)

local controller = require("@self/controller")

-- The palette of its picture on the custom screen, which no ROM holds:
-- the Japanese record points at EWRAM 0x02000B10, where a gift received
-- over the link (an e-Reader card's, Card e+) puts it (`0x0813006C` in
-- the Japanese ROMs copies the card's palette to 0x02000AF0 + 32 * slot;
-- slot 1 is the console's own beast), and the save keeps it there (save
-- offset 0xB10). These are the card's colors
-- as Tango's netplay save for JP Falzar has them (tango-gamesupport-bn6,
-- saves/f_jp.raw, offset 0xB10). The picture is the Japanese Falzar ROM's
-- (docs/engine/jp-differences.md §4.4).
local ART_PALETTE = {
    0x0000, 0x7BDE, 0x7774, 0x49CC, 0x44EB, 0x7707, 0x6E43, 0x5582,
    0x40C1, 0x1B3E, 0x2659, 0x09B4, 0x76DD, 0x557B, 0x2039, 0x1805,
}

local chip: Chip = {
    art_palette = ART_PALETTE,
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    extra_flags = { 16 },
    slot_in_limit = 1,
    dimming = controller.hook,
}

return { falzar = chip }
