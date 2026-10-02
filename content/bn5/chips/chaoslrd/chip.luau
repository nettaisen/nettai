--!strict
-- ChaosLrd (BN5 chip 0x138): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- An e-Reader card's chip: its name, description and picture palette are the save's (bn5-map.md
-- §6.4), the palette here from Tango's netplay saves, as a gift chip's in BN6.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

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
}
