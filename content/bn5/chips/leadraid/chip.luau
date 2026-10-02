--!strict
-- LeadRaid (BN5 chip 0x137): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- An e-Reader card's chip: its name, description and picture palette are the save's (bn5-map.md
-- §6.4), the palette here from Tango's netplay saves, as a gift chip's in BN6.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

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
}
