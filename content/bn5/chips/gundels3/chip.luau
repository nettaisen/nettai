--!strict
-- GunDelS3 (BN5 chip 0x022): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "gundels3",
    codes = { "D", "M", "O" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 35,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "light",
}
