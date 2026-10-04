--!strict
-- ChaosLrd (BN5 chip 0x138): a navi chip (the summon table's entry 29) bringing Bass and the dark beast behind him to
-- his user's back column, who throws a chaos strike at the middle of the other side's area (./navi: BN5's actor
-- object #0x51).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs. An e-Reader card's chip:
-- its name, description and picture palette are the save's (bn5-map.md §6.4), the palette here from Tango's netplay
-- saves, as a gift chip's in BN6.

local navi = require("./navi")

return define.chip {
    id = "chaoslrd",
    codes = { "X" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 99,
    damage = 500,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    slot_in_limit = 1,
    art_palette = { 0x0000, 0x773A, 0x5AB9, 0x657F, 0x587A, 0x0173, 0x127A, 0x1D9B, 0x0099, 0x0031, 0x0D0D, 0x18C8, 0x1C25, 0x556F, 0x34C6, 0x76A6 },
    navi = navi.summon,
}
