--!strict
-- VarSwrd (BN5 chip 0x037): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:varswrd",
    codes = { "B", "C", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
