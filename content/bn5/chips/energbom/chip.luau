--!strict
-- EnergBom (BN5 chip 0x01e): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "energbom",
    codes = { "E", "P", "W" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 11,
    damage = 40,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
