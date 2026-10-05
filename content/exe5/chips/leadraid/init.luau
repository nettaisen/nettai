--!strict
-- LeadRaid (EXE5 chip 0x137): a navi chip (the summon table's entry 21) bringing ProtoMan and Colonel, who charge,
-- then strike in turn: ProtoMan a slash ahead of the enemy navi he blinks to, Colonel an X on the first enemy navi in
-- his row (./protoman, ./colonel: EXE5's actor objects #0x20 and #0x22).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs. An e-Reader card's chip:
-- its name, description and picture palette are the save's (exe5-map.md §6.4), the palette here from Tango's netplay
-- saves, as a gift chip's in EXE6.

local protoman = require("@self/protoman")

return define.chip {
    id = "leadraid",
    codes = { "L" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 99,
    damage = 200,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage" },
    slot_in_limit = 1,
    art_palette = { 0x0000, 0x67DE, 0x5AF8, 0x35CF, 0x2507, 0x5E05, 0x4120, 0x28A5, 0x32DF, 0x25DA, 0x14DC, 0x0031, 0x352A, 0x14A5, 0x3F4D, 0x2E66 },
    navi = protoman.summon,
}
