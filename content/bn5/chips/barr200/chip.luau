--!strict
-- Barr200 (BN5 chip 0x089): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:barr200",
    codes = { "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 47,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
