--!strict
-- AirHoc (BN5 chip 0x005): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "airhoc",
    codes = { "Q", "R", "S" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 40,
    hit_param = 5,
    lockout = 50,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
